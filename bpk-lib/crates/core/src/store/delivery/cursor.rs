use crate::coordinate::Region;
use crate::store::delivery::canal::{Canal, CanalBatch, CanalClosed};
use crate::store::delivery::observation::CheckpointId;
use crate::store::index::{IndexEntry, StoreIndex};
use crate::store::platform::clock::{mono_elapsed, Clock};
use crate::store::platform::fs::StoreFs;
use crate::store::StoreError;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

mod checkpoint;
mod gap;
mod worker;

pub use checkpoint::CursorCheckpoint;
use checkpoint::{cursor_checkpoint_path, CursorDurableBinding};
use gap::GapBuffer;
pub use gap::{CursorGapConfig, GapObservation};
pub use worker::{CursorWorkerAction, CursorWorkerConfig, CursorWorkerHandle};

/// Cursor: pull-based event consumption with ordered replay.
///
/// [`crate::store::Store::cursor_guaranteed`] exposes the process-local surface of this
/// type: ordered, at-least-once pull replay from the in-memory index.
/// Checkpoint-backed cursors are the internal mechanism used by
/// `cursor_worker` and typed reactors when `checkpoint_id` is set.
/// Resume succeeds only if the saved checkpoint loads and its region
/// identity matches the cursor's exact [`Region`].
///
/// Delivery semantics:
///
/// * **Without `checkpoint_id`:** at-least-once within process lifetime.
///   Events delivered before a process crash are re-delivered on next
///   start because the in-memory cursor position is lost.
/// * **With `checkpoint_id`:** at-least-once across restarts. A crash
///   between delivery and checkpoint save causes the latest batch to be
///   re-delivered — callers must treat their handler as idempotent.
///   The checkpoint is also bound to the cursor's exact [`Region`];
///   reusing the same id for a different logical consumer fails closed.
///
/// Neither mode is exactly-once: that guarantee requires coordinating
/// the cursor checkpoint with the downstream side-effect in a single
/// atomic write, which this type does not attempt.
pub struct Cursor {
    region: Region,
    position: u64, // tracks global_sequence — next poll starts after this
    started: bool, // false until first event consumed (global_sequence 0 is valid)
    index: Arc<StoreIndex>,
    gap_buffer: Option<GapBuffer>,
    /// Optional durable-checkpoint id. When set, the cursor was
    /// constructed with a data directory and a checkpoint identifier so
    /// its position can be persisted via `save_checkpoint`.
    durable: Option<CursorDurableBinding>,
    /// Store runtime clock; `pull_batch` measures its deadline window on
    /// [`Clock::now_mono_ns`] so injected clocks govern pull waits.
    clock: Arc<dyn Clock>,
}

impl Cursor {
    pub(crate) fn new(region: Region, index: Arc<StoreIndex>, clock: Arc<dyn Clock>) -> Self {
        Self {
            region,
            position: 0,
            started: false,
            index,
            gap_buffer: None,
            durable: None,
            clock,
        }
    }

    fn new_bound_checkpoint(
        region: Region,
        index: Arc<StoreIndex>,
        data_dir: &Path,
        id: CheckpointId,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            region,
            position: 0,
            started: false,
            index,
            gap_buffer: None,
            durable: Some(CursorDurableBinding {
                data_dir: data_dir.to_path_buf(),
                id,
            }),
            clock,
        }
    }

    /// Construct a cursor bound to a durable checkpoint id. On
    /// construction the persisted position (if any) is loaded and the
    /// cursor resumes from it. Missing checkpoints yield a fresh cursor
    /// at position 0; a corrupt checkpoint fails closed with
    /// [`StoreError::CursorCheckpointCorrupt`].
    pub(crate) fn new_with_checkpoint(
        region: Region,
        index: Arc<StoreIndex>,
        data_dir: &Path,
        id: &CheckpointId,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, StoreError> {
        let mut cursor = Self::new_bound_checkpoint(region, index, data_dir, id.clone(), clock);
        match Self::load_checkpoint(data_dir, id) {
            Ok(Some(ckpt)) => {
                let expected_region = cursor.region.checkpoint_identity();
                if ckpt.region_identity.as_deref() != Some(expected_region.as_str()) {
                    return Err(StoreError::CursorCheckpointRegionMismatch {
                        path: cursor_checkpoint_path(data_dir, id),
                        stored: ckpt.region_identity,
                        expected: expected_region,
                    });
                }
                cursor.position = ckpt.position;
                cursor.started = ckpt.started;
            }
            Ok(None) => {}
            Err(error) => {
                if error.kind() == std::io::ErrorKind::InvalidData {
                    return Err(StoreError::CursorCheckpointCorrupt {
                        path: cursor_checkpoint_path(data_dir, id),
                        reason: error.to_string(),
                    });
                }
                return Err(StoreError::Io(error));
            }
        }
        Ok(cursor)
    }

    /// Poll for the next matching event at or after our current position.
    pub fn poll(&mut self) -> Option<IndexEntry> {
        let hits = self
            .index
            .query_hits_after(&self.region, self.position, self.started, 1);
        if let Some(hit) = hits.into_iter().next() {
            let expected_sequence = if self.started {
                self.position.saturating_add(1)
            } else {
                0
            };
            self.record_gap(expected_sequence, hit.global_sequence);
            self.position = hit.global_sequence;
            self.started = true;
            self.index.upgrade_hit(hit)
        } else {
            None
        }
    }

    /// Poll for up to max matching events.
    pub fn poll_batch(&mut self, max: usize) -> Vec<IndexEntry> {
        let hits = self
            .index
            .query_hits_after(&self.region, self.position, self.started, max);
        if hits.is_empty() {
            return Vec::new();
        }
        self.record_gaps_for_hits(&hits);
        self.started = true;
        self.position = hits[hits.len() - 1].global_sequence;
        hits.into_iter()
            .filter_map(|hit| self.index.upgrade_hit(hit))
            .collect()
    }

    /// The index visibility epoch — snapshot before a poll, pass to
    /// [`Self::park_for_data`] so a publish racing the poll is never missed.
    pub(crate) fn visibility_epoch(&self) -> u64 {
        self.index.sequence.visibility_epoch()
    }

    /// Park until the index publishes a new visible entry past `since_epoch`, or
    /// `timeout` elapses. Replaces a poll-sleep spin; the timeout is the deadline
    /// safety net (a missed wakeup degrades to the timeout, never a hang).
    ///
    /// Returns whether the park ran its full `timeout` without a wakeup — the
    /// real-time lower bound [`Cursor::pull_batch`] accumulates so its deadline
    /// holds even under an injected clock that does not advance while parked.
    pub(crate) fn park_for_data(&self, since_epoch: u64, timeout: Duration) -> bool {
        self.index
            .sequence
            .park_for_visibility_change(since_epoch, timeout)
    }

    /// Configure this cursor's in-memory write-to-deliver gap observation.
    #[must_use]
    pub fn with_gap_config(mut self, config: CursorGapConfig) -> Self {
        self.gap_buffer = match config {
            CursorGapConfig::Disabled => None,
            CursorGapConfig::Enabled { capacity } => Some(GapBuffer::new_nonzero(capacity)),
        };
        self
    }

    /// Drain the currently retained write-to-deliver gaps.
    pub fn take_gaps(&mut self) -> Vec<GapObservation> {
        match self.gap_buffer.as_mut() {
            Some(buffer) => buffer.take_all(),
            None => Vec::new(),
        }
    }

    pub(crate) fn checkpoint(&self) -> (u64, bool) {
        (self.position, self.started)
    }

    pub(crate) fn restore_checkpoint(&mut self, position: u64, started: bool) {
        self.position = position;
        self.started = started;
    }

    /// Persist the cursor's current position to its bound durable
    /// checkpoint. No-op if the cursor was constructed without an id.
    ///
    /// Routes the atomic checkpoint publish through `fs` (the store's configured
    /// [`StoreFs`]) so the crash-sensitive persist is fault-injectable under a
    /// `SimFs`; in production `fs` is [`crate::store::platform::fs::RealFs`].
    pub(crate) fn persist_current(&self, fs: &dyn StoreFs) -> std::io::Result<()> {
        let Some(binding) = &self.durable else {
            return Ok(());
        };
        let ckpt = CursorCheckpoint::from_checkpoint(
            self.position,
            self.started,
            self.region.checkpoint_identity(),
        );
        Self::save_checkpoint_with_fs(&binding.data_dir, &binding.id, &ckpt, fs)
    }

    fn record_gaps_for_hits(&mut self, hits: &[crate::store::index::QueryHit]) {
        let mut expected_sequence = if self.started {
            self.position.saturating_add(1)
        } else {
            0
        };
        for hit in hits {
            self.record_gap(expected_sequence, hit.global_sequence);
            expected_sequence = hit.global_sequence.saturating_add(1);
        }
    }

    fn record_gap(&mut self, expected_sequence: u64, delivered_sequence: u64) {
        let Some(buffer) = self.gap_buffer.as_mut() else {
            return;
        };
        if delivered_sequence <= expected_sequence {
            return;
        }
        let cancelled_visibility = self.index.cancelled_visibility_ranges();
        let lane_ranges = self
            .region
            .lane
            .and_then(|lane| cancelled_visibility.lanes.get(&lane));
        let cancelled_ranges = cancelled_visibility
            .global
            .iter()
            .chain(lane_ranges.into_iter().flatten())
            .copied()
            .filter_map(|(start, end)| {
                let overlap_start = start.max(expected_sequence);
                let overlap_end = end.min(delivered_sequence);
                (overlap_start < overlap_end).then_some((overlap_start, overlap_end))
            })
            .collect::<Vec<_>>();
        if cancelled_ranges.is_empty() {
            return;
        }
        buffer.push(GapObservation {
            expected_sequence,
            delivered_sequence,
            cancelled_ranges,
        });
    }
}

impl Canal for Cursor {
    type Item = IndexEntry;
    type Error = CanalClosed;

    fn pull_batch(
        &mut self,
        max: usize,
        deadline: Duration,
    ) -> Result<CanalBatch<Self::Item>, Self::Error> {
        if max == 0 {
            return Ok(CanalBatch::Empty);
        }
        let started_ns = self.clock.now_mono_ns();
        // Real-time floor accumulated from FULLY timed-out parks. An injected
        // clock may not advance while this thread is parked (a frozen logical
        // clock), which would otherwise re-park for the full deadline forever;
        // a timed-out park proves at least `remaining` real time passed, so
        // `max(logical elapsed, floor)` keeps the deadline a real bound while
        // an ADVANCING injected clock still governs the wait deterministically.
        let mut waited_floor = Duration::ZERO;
        loop {
            // Snapshot the visibility epoch BEFORE polling so a publish racing this
            // poll cannot be lost: if it advances the epoch between here and the park
            // below, the park returns immediately and we re-poll.
            let epoch = self.visibility_epoch();
            let batch = self.poll_batch(max);
            match batch.len() {
                0 => {
                    let elapsed = mono_elapsed(self.clock.as_ref(), started_ns).max(waited_floor);
                    if elapsed >= deadline {
                        return Ok(CanalBatch::Empty);
                    }
                    let remaining = deadline.saturating_sub(elapsed);
                    // Park on the index's visibility edge instead of a 1 ms poll-spin:
                    // wakes promptly when the writer publishes a new visible entry, and
                    // falls back to `remaining` as a deadline safety net.
                    if self.park_for_data(epoch, remaining) {
                        waited_floor = waited_floor.saturating_add(remaining);
                    }
                }
                1 => {
                    let mut batch = batch;
                    return Ok(CanalBatch::One(batch.remove(0)));
                }
                _ => return Ok(CanalBatch::Many(batch)),
            }
        }
    }
}

#[cfg(test)]
mod mutation_kill_tests {
    //! Targeted unit tests for the private cursor delivery seam. They pin the
    //! exact observable behavior of `visibility_epoch`, `park_for_data`, and the
    //! `pull_batch` empty arm so the constant/no-op/deleted-arm mutants of those
    //! items are caught.
    use super::{Canal, CanalBatch, Clock, Cursor, CursorGapConfig, Region, StoreIndex};
    use crate::store::hidden_ranges::CancelledVisibilityRanges;
    use crate::store::platform::clock::SystemClock;
    use std::collections::BTreeMap;
    use std::num::NonZeroUsize;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn fresh_index() -> Arc<StoreIndex> {
        Arc::new(StoreIndex::new())
    }

    fn test_clock() -> Arc<dyn Clock> {
        Arc::new(SystemClock::new())
    }

    #[test]
    fn record_gap_intersects_with_a_half_open_range_excluding_zero_width_overlaps() {
        // Kills cursor.rs:254 `overlap_start < overlap_end` -> `<=`. The gap
        // observer intersects the delivered [expected, delivered) interval with
        // each cancelled visibility range and keeps only NON-empty overlaps
        // (`start < end`). A cancelled range that merely touches the delivered
        // boundary is a zero-width intersection and must be dropped; the `<=`
        // mutant keeps it as a spurious `(k, k)` cancelled range.
        let index = fresh_index();
        // Global cancelled ranges: (2,4) genuinely overlaps [1,5); (5,7) only
        // TOUCHES the delivered boundary at 5 (overlap [5,5) is zero-width).
        index.restore_cancelled_visibility_ranges(CancelledVisibilityRanges {
            global: vec![(2, 4), (5, 7)],
            lanes: BTreeMap::new(),
        });
        let mut cursor = Cursor::new(Region::all(), index, test_clock()).with_gap_config(
            CursorGapConfig::Enabled {
                capacity: NonZeroUsize::new(8).expect("nonzero capacity"),
            },
        );

        // expected=1, delivered=5 => skipped interval [1,5).
        cursor.record_gap(1, 5);
        let gaps = cursor.take_gaps();

        assert_eq!(gaps.len(), 1, "exactly one gap observation is recorded");
        assert_eq!(gaps[0].expected_sequence, 1);
        assert_eq!(gaps[0].delivered_sequence, 5);
        assert_eq!(
            gaps[0].cancelled_ranges,
            vec![(2, 4)],
            "PROPERTY: only the genuine (2,4) overlap is retained; the `<=` mutant \
             also admits the zero-width (5,5) boundary touch, got {:?}",
            gaps[0].cancelled_ranges
        );
    }

    #[test]
    fn pull_batch_blocks_until_the_deadline_before_reporting_empty() {
        // Kills cursor.rs:289 `start.elapsed() >= deadline` -> `<`. On an empty
        // cursor the timeout arm must keep parking until the deadline is reached
        // and only THEN return Empty. Flipping `>=` to `<` returns Empty on the
        // very first idle poll (elapsed ~0 < deadline), collapsing the blocking
        // wait to an instant spin — so a 50 ms pull would return in ~0 ms.
        let index = fresh_index();
        let mut cursor = Cursor::new(Region::all(), index, test_clock());

        let deadline = Duration::from_millis(50);
        let start = Instant::now();
        let result = cursor
            .pull_batch(8, deadline)
            .expect("pull_batch on an empty cursor");
        let elapsed = start.elapsed();

        assert!(
            matches!(result, CanalBatch::Empty),
            "an idle cursor eventually returns Empty"
        );
        assert!(
            elapsed >= Duration::from_millis(20),
            "PROPERTY: pull_batch must block toward its deadline before reporting \
             Empty; the `<` mutant returns instantly, elapsed only {elapsed:?}"
        );
    }

    #[test]
    fn visibility_epoch_advances_with_each_publish() {
        // Constant mutants (`-> 0`, `-> 1`) would make the epoch never change.
        let index = fresh_index();
        let cursor = Cursor::new(Region::all(), Arc::clone(&index), test_clock());

        let before = cursor.visibility_epoch();
        index.reserve_sequences(1);
        index
            .publish(1, "mutation-kill-visibility")
            .expect("publish 1");
        let after_one = cursor.visibility_epoch();
        index.reserve_sequences(1);
        index
            .publish(2, "mutation-kill-visibility")
            .expect("publish 2");
        let after_two = cursor.visibility_epoch();

        let mut failures: Vec<String> = Vec::new();
        if before != 0 {
            failures.push(format!("fresh index epoch must be 0, got {before}"));
        }
        if after_one != 1 {
            failures.push(format!(
                "epoch after one publish must be 1, got {after_one}"
            ));
        }
        if after_two != 2 {
            failures.push(format!(
                "epoch after two publishes must be 2, got {after_two}"
            ));
        }
        assert!(
            failures.is_empty(),
            "visibility_epoch mismatches: {failures:?}"
        );
    }

    #[test]
    fn park_for_data_blocks_until_the_timeout_when_no_publish_races() {
        // The `park_for_data -> ()` no-op mutant returns instantly; the real
        // implementation must block (roughly) for the supplied timeout when the
        // epoch never advances.
        let index = fresh_index();
        let cursor = Cursor::new(Region::all(), index, test_clock());
        let epoch = cursor.visibility_epoch();

        // A 50 ms floor keeps the whole-suite tax low while still proving the
        // park actually blocked: the `park_for_data -> ()` no-op mutant returns
        // in ~0 ms, far below the 20 ms lower bound asserted here.
        let timeout = Duration::from_millis(50);
        let start = Instant::now();
        cursor.park_for_data(epoch, timeout);
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(20),
            "park_for_data must block for ~timeout when no publish advances the epoch; \
             elapsed only {elapsed:?}"
        );
    }

    #[test]
    fn pull_batch_returns_empty_not_many_when_no_data_arrives() {
        // Deleting the `0 =>` arm of `pull_batch` would route an empty poll into
        // the `_ =>` arm and return `CanalBatch::Many(<empty>)` immediately.
        let index = fresh_index();
        let mut cursor = Cursor::new(Region::all(), index, test_clock());

        let result = cursor
            .pull_batch(8, Duration::from_millis(30))
            .expect("pull_batch on empty cursor");

        assert!(
            matches!(result, CanalBatch::Empty),
            "an empty cursor must return CanalBatch::Empty after the deadline, got a non-empty batch"
        );
    }

    /// A clock whose monotonic reading never advances — the frozen
    /// logical-clock regime (an injected sim clock nobody drives during the
    /// pull). PR #169 review finding: before the timed-out-park floor,
    /// `pull_batch` recomputed a zero elapsed forever and re-parked for the
    /// full deadline instead of returning `CanalBatch::Empty`.
    struct FrozenClock;

    impl Clock for FrozenClock {
        fn now_us(&self) -> i64 {
            0
        }

        fn now_wall_ns(&self) -> i64 {
            0
        }

        fn now_mono_ns(&self) -> i64 {
            42
        }

        fn process_boot_ns(&self) -> u64 {
            0
        }
    }

    #[test]
    fn pull_batch_deadline_is_a_real_bound_under_a_frozen_clock() {
        let index = fresh_index();
        let mut cursor = Cursor::new(Region::all(), index, Arc::new(FrozenClock));

        let started = Instant::now();
        let result = cursor
            .pull_batch(8, Duration::from_millis(50))
            .expect("pull_batch on empty cursor");
        let elapsed = started.elapsed();

        assert!(
            matches!(result, CanalBatch::Empty),
            "PROPERTY: an empty pull under a frozen injected clock must still return \
             Empty at the deadline (timed-out parks accumulate a real-time floor); a \
             livelock here hangs the test instead"
        );
        assert!(
            elapsed >= Duration::from_millis(20),
            "the deadline park must still actually block; elapsed only {elapsed:?}"
        );
    }
}

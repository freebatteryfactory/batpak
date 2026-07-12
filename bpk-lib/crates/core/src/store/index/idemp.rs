//! Durable idempotency key → receipt store (Phase 3, 0.8.3 schema-evolution).
//!
//! # Why this exists
//!
//! `AppendOptions::with_idempotency(key)` makes the key the `event_id` and the
//! writer returns the original receipt as a no-op on a duplicate. That dedup
//! was only durable inside an event's *retention window*: once `Retention`
//! compaction evicted the event, its `by_id` index entry vanished and a re-run
//! of the same key RE-APPENDED a duplicate. The bug: idempotency was not a true
//! durable correctness primitive.
//!
//! This module closes the gap with a dedicated sidecar that survives
//! compaction / retention / cold-start INDEPENDENT of event eviction:
//!
//! * an in-memory [`DashMap<u128, IdempEntry>`] capturing exactly the tuple
//!   needed to RECONSTRUCT the original `AppendReceipt` for a no-op return even
//!   when the underlying event has been evicted, and
//! * an on-disk artifact `index.idemp` (magic `FBATID`, version
//!   [`IDEMP_VERSION`], crc32fast CRC, written via `write_file_atomically`)
//!   restored UNCONDITIONALLY and early on open — it is an AUTHORITY the
//!   segment-scan index rebuild must NOT overwrite, and is NEVER reconstructed
//!   from a segment scan (segments may have evicted the events).
//!
//! # Growth bound — the window-priority hybrid
//!
//! A durable key store outlives event retention, so it grows unless bounded.
//! [`IdempotencyRetention`] picks the policy. The **window** is the inviolable
//! correctness guarantee; the **cap** is a soft bound + alarm that may only
//! ever evict keys ALREADY OUTSIDE the window. See [`IdempotencyStore::evict`]
//! for the verbatim eviction rule.
//!
//! justifies: INV-IDEMPOTENCY-DURABLE-WINDOW; this module is the durable
//! sidecar + window-priority hybrid that makes a within-window keyed retry an
//! unconditional no-op regardless of compaction, cold-start, or load.

use crate::store::platform::fs::{write_file_atomically_with_fs, StoreFs};
use crate::store::{EncodedBytes, ExtensionKey, StoreError};
use dashmap::DashMap;
use std::collections::BTreeMap;
use std::path::Path;

use super::entry::{DiskPos, IndexEntry};
use crate::event::EventKind;

/// Magic bytes at the start of every `index.idemp` file.
pub(crate) const IDEMP_MAGIC: &[u8; 6] = b"FBATID";

/// On-disk format version stored in the `index.idemp` header.
/// v1: initial durable idempotency sidecar (bare `Vec<IdempEntry>` body).
/// v2 (GAUNT-IDEMPOTENCY-AUTHORITY, #189): the body additionally binds the
/// image to the store's lineage identity and to a compound history anchor
/// (covered global sequence + event id + chain commitment at that sequence),
/// so a transplanted, stale, or diverged-sibling image is detectable at cold
/// start instead of silently narrowing the dedup contract.
pub const IDEMP_VERSION: u16 = 2;

/// Final filename inside the data directory.
pub(crate) const IDEMP_FILENAME: &str = "index.idemp";

/// Default window guarantee: keep idempotency keys for the most recent
/// `DEFAULT_KEEP_SEQUENCES` committed global sequences. Generous on purpose —
/// it must comfortably outlive a realistic event-retention window so a retry
/// of a recently-committed key is always a no-op. Chosen consistently with the
/// store's generous segment/checkpoint defaults (256 MB segments, etc.).
pub(crate) const DEFAULT_KEEP_SEQUENCES: u64 = 16 * 1024 * 1024;

/// Default soft cap on total durable keys. Generous; the window always wins on
/// a residual pigeonhole (see [`IdempotencyStore::evict`]).
pub(crate) const DEFAULT_MAX_KEYS: u64 = 64 * 1024 * 1024;

/// Growth-bound policy for the durable idempotency store.
///
/// The window is the correctness guarantee; the cap is a soft bound + alarm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdempotencyRetention {
    /// Never evict. The store grows with the keyed-append rate forever. Only
    /// safe for bounded-lifetime stores or callers that compact externally.
    Unbounded,
    /// Keep keys whose `recorded_global_sequence` is within `keep_sequences`
    /// of the current frontier. Older keys are trimmed. This is the
    /// correctness guarantee with no soft cap.
    Window {
        /// How many global sequences back from the frontier to keep.
        keep_sequences: u64,
    },
    /// Window guarantee PLUS a soft cap and alarm. DEFAULT policy.
    ///
    /// The window is inviolable; the cap may only evict keys already OUTSIDE
    /// the window. If within-window keys alone exceed `max_keys` (a real
    /// key-rate spike), the window wins and the store temporarily exceeds
    /// `max_keys` (bounded by rate×window, not unbounded) with a loud
    /// diagnostic; [`OverflowPolicy`] then decides escalation.
    Hybrid {
        /// How many global sequences back from the frontier to keep.
        keep_sequences: u64,
        /// Soft cap on total durable keys.
        max_keys: u64,
    },
}

impl Default for IdempotencyRetention {
    fn default() -> Self {
        Self::Hybrid {
            keep_sequences: DEFAULT_KEEP_SEQUENCES,
            max_keys: DEFAULT_MAX_KEYS,
        }
    }
}

impl IdempotencyRetention {
    /// The window guarantee in global sequences, if any. `Unbounded` returns
    /// `None` (no aging).
    pub(crate) fn keep_sequences(self) -> Option<u64> {
        match self {
            Self::Unbounded => None,
            Self::Window { keep_sequences } | Self::Hybrid { keep_sequences, .. } => {
                Some(keep_sequences)
            }
        }
    }

    /// The soft cap on total keys, if any.
    pub(crate) fn max_keys(self) -> Option<u64> {
        match self {
            Self::Unbounded | Self::Window { .. } => None,
            Self::Hybrid { max_keys, .. } => Some(max_keys),
        }
    }
}

/// What to do when within-window keys alone exceed the soft cap (residual
/// pigeonhole). The window ALWAYS wins on correctness; this only decides
/// escalation behavior for the NEW keyed append that would push over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OverflowPolicy {
    /// Log a loud diagnostic and proceed. The store exceeds `max_keys`
    /// temporarily (bounded by rate×window). DEFAULT.
    #[default]
    Warn,
    /// Refuse the new keyed append with a clear error. Correctness over disk:
    /// the within-window keys already recorded are never evicted, so existing
    /// retries stay no-ops; only genuinely new keys are rejected.
    FailClosed,
    /// Signal backpressure / slow down. batpak's writer is single-threaded and
    /// exposes no clean append-time backpressure channel from this record
    /// path, so this is treated as `FailClosed` and noted in the diagnostic.
    Backpressure,
}

/// Minimal tuple captured per durable keyed append. Captures EXACTLY the fields
/// the writer's no-op path reads from an `IndexEntry` to reconstruct the
/// original `AppendReceipt` (see `store/write/writer/append.rs`):
/// `event_id`, `global_sequence`, `disk_pos`, `event_hash`, `prev_hash`,
/// `coord` (as entity+scope strings), `kind`, and `receipt_extensions`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct IdempEntry {
    /// The idempotency key (== the event_id of the original keyed append).
    pub(crate) key: u128,
    /// Original event id (identical to `key`, kept explicit for receipt build).
    pub(crate) event_id: u128,
    /// Global sequence assigned at the original commit.
    pub(crate) global_sequence: u64,
    /// Original frame location. Preserved verbatim even after the event is
    /// evicted — the reconstructed no-op receipt must always carry the ORIGINAL
    /// position so a within-window retry returns the original receipt unchanged.
    /// Eviction is tracked by the separate [`IdempEntry::event_evicted`] flag,
    /// never by mutating this field.
    pub(crate) disk_pos_segment: u64,
    /// Byte offset of the original frame.
    pub(crate) disk_pos_offset: u64,
    /// Byte length of the original frame.
    pub(crate) disk_pos_length: u32,
    /// Blake3 content hash of the original committed payload.
    pub(crate) content_hash: [u8; 32],
    /// Predecessor hash at the time of the original commit (needed to re-sign
    /// the reconstructed receipt identically).
    pub(crate) prev_hash: [u8; 32],
    /// Coordinate entity string.
    pub(crate) entity: String,
    /// Coordinate scope string.
    pub(crate) scope: String,
    /// Event kind discriminant.
    pub(crate) kind: EventKind,
    /// The global sequence at which this entry was RECORDED into the durable
    /// store. This is the value the window-priority rule ages against — it is
    /// the entry's position on the frontier timeline.
    pub(crate) recorded_global_sequence: u64,
    /// Whether this entry's underlying event frame has been evicted by
    /// retention compaction. Set by [`IdempotencyStore::mark_evicted`]; the
    /// `disk_pos_*` fields are NEVER mutated, so the reconstructed receipt keeps
    /// the original frame position.
    #[serde(default)]
    pub(crate) event_evicted: bool,
    /// Opaque receipt extensions committed with the original event.
    pub(crate) receipt_extensions: BTreeMap<ExtensionKey, EncodedBytes>,
}

impl IdempEntry {
    /// Capture the minimal reconstruction tuple from a freshly committed index
    /// entry plus the frontier sequence at record time.
    pub(crate) fn from_index_entry(entry: &IndexEntry, recorded_global_sequence: u64) -> Self {
        Self {
            key: entry.event_id,
            event_id: entry.event_id,
            global_sequence: entry.global_sequence,
            disk_pos_segment: entry.disk_pos.segment_id(),
            disk_pos_offset: entry.disk_pos.offset(),
            disk_pos_length: entry.disk_pos.length(),
            content_hash: entry.hash_chain.event_hash,
            prev_hash: entry.hash_chain.prev_hash,
            entity: entry.coord.entity().to_owned(),
            scope: entry.coord.scope().to_owned(),
            kind: entry.kind,
            recorded_global_sequence,
            event_evicted: false,
            receipt_extensions: entry.receipt_extensions.clone(),
        }
    }

    /// The recorded `disk_pos` as the typed [`DiskPos`] used by receipts.
    pub(crate) fn disk_pos(&self) -> DiskPos {
        DiskPos::new(
            self.disk_pos_segment,
            self.disk_pos_offset,
            self.disk_pos_length,
        )
    }

    /// Whether this entry's underlying event frame has been evicted (retention
    /// compaction). A no-op receipt for an evicted event still carries the
    /// original reconstruction tuple (including the original `disk_pos`); this
    /// flag lets callers distinguish "frame still live" from "deduplicated
    /// against an evicted event" without a disk probe. Non-test code reads the
    /// `event_evicted` field directly; this accessor is the test-facing reader.
    #[cfg(test)]
    pub(crate) fn is_event_evicted(&self) -> bool {
        self.event_evicted
    }
}

/// Outcome diagnostics from a single eviction pass. Used for tests and loud
/// logging; the store never silently changes a within-window answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EvictionReport {
    /// Keys trimmed because they aged out of the window.
    pub(crate) aged_out: u64,
    /// Keys trimmed by the cap from the OUT-OF-WINDOW region (free win).
    pub(crate) cap_trimmed_out_of_window: u64,
    /// True when within-window keys ALONE exceed the soft cap (residual
    /// pigeonhole). The window wins; the store temporarily exceeds `max_keys`.
    pub(crate) within_window_exceeds_cap: bool,
    /// Total keys remaining after the pass.
    pub(crate) remaining: u64,
}

/// Durable idempotency key → receipt-reconstruction store.
///
/// Lives on [`crate::store::index::StoreIndex`] so it is reachable both at
/// append time (the single writer thread holds `&StoreIndex`) and at
/// compaction / close time (lifecycle holds `store.index`). It is a separate
/// field from `by_id` and is therefore NOT cleared by
/// `replace_contents_from_fresh` — its survival across compaction is structural.
pub(crate) struct IdempotencyStore {
    map: DashMap<u128, IdempEntry>,
    retention: IdempotencyRetention,
    overflow: OverflowPolicy,
}

impl IdempotencyStore {
    /// Construct an empty store with the given policy.
    pub(crate) fn new(retention: IdempotencyRetention, overflow: OverflowPolicy) -> Self {
        Self {
            map: DashMap::new(),
            retention,
            overflow,
        }
    }

    /// Number of durable keys currently held.
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    /// Look up a key. Returns the reconstruction tuple on hit. This is the
    /// map-first authority consulted BEFORE `by_id` in the writer no-op check.
    pub(crate) fn get(&self, key: u128) -> Option<IdempEntry> {
        self.map.get(&key).map(|r| r.value().clone())
    }

    /// Whether a single NEW keyed append should be admitted under the soft cap.
    /// `frontier` lets us age out-of-window keys out before fail-closing, so a
    /// stale entry is trimmed rather than a fresh key refused. Re-recording a
    /// known key is always admitted (no growth).
    pub(crate) fn admit_new_key(&self, key: u128, frontier: u64) -> Result<(), StoreError> {
        if self.map.contains_key(&key) {
            return Ok(());
        }
        self.admit_unique_new_count(1, frontier)
    }

    /// Validate and admit a whole batch of candidate keys as a UNIT. Counts only
    /// keys NOT already present, rejects duplicate new keys WITHIN the batch
    /// (they would otherwise both pass a per-item check and derive duplicate
    /// event ids), and enforces `current + unique_new <= max_keys` atomically so
    /// a set of unique new keys can never collectively slip past a fail-closed
    /// cap. All-or-nothing. `frontier` drives the same age-out as
    /// [`IdempotencyStore::admit_new_key`].
    pub(crate) fn admit_new_keys(
        &self,
        keys: impl Iterator<Item = u128>,
        frontier: u64,
    ) -> Result<(), StoreError> {
        let mut seen_new: std::collections::HashSet<u128> = std::collections::HashSet::new();
        let mut unique_new: u64 = 0;
        for key in keys {
            // Already durable: re-recording is a no-op on growth.
            if self.map.contains_key(&key) {
                continue;
            }
            if !seen_new.insert(key) {
                return Err(StoreError::IdempotencyPartialBatch {
                    reason: "duplicate idempotency key in batch".into(),
                });
            }
            unique_new = unique_new.saturating_add(1);
        }
        self.admit_unique_new_count(unique_new, frontier)
    }

    /// Shared admission core: can `unique_new` genuinely-new keys be admitted
    /// without exceeding the soft cap? Ages out-of-window keys out first, then
    /// escalates per [`OverflowPolicy`]. `Unbounded`/`Window` (no cap) always
    /// admit.
    fn admit_unique_new_count(&self, unique_new: u64, frontier: u64) -> Result<(), StoreError> {
        let Some(max_keys) = self.retention.max_keys() else {
            return Ok(());
        };
        if unique_new == 0 {
            return Ok(());
        }
        let mut len = self.map.len() as u64;
        if len.saturating_add(unique_new) <= max_keys {
            return Ok(());
        }
        // Pressure: age out out-of-window keys before fail-closing so a stale
        // key is trimmed rather than a fresh one refused. justifies:
        // INV-IDEMPOTENCY-DURABLE-WINDOW
        self.evict(frontier);
        len = self.map.len() as u64;
        if len.saturating_add(unique_new) <= max_keys {
            return Ok(());
        }
        match self.overflow {
            OverflowPolicy::Warn => Ok(()),
            OverflowPolicy::FailClosed | OverflowPolicy::Backpressure => {
                let backpressure_note = matches!(self.overflow, OverflowPolicy::Backpressure);
                tracing::warn!(
                    target: "batpak::idemp",
                    len,
                    max_keys,
                    unique_new,
                    backpressure_note,
                    "durable idempotency store at soft cap; refusing new keyed append(s) (fail-closed)"
                );
                Err(StoreError::IdempotencyOverflowFailClosed { len, max_keys })
            }
        }
    }

    /// Record (or overwrite) a durable entry. Idempotent on the key.
    pub(crate) fn record(&self, entry: IdempEntry) {
        self.map.insert(entry.key, entry);
    }

    /// Mark durable entries whose underlying event frame is no longer live as
    /// EVICTED by setting the [`IdempEntry::event_evicted`] flag.
    /// `is_live(event_id)` reports whether the frame still exists in the live
    /// index. Run at the compaction tail so a subsequent no-op for a
    /// deduplicated-against-evicted key is honestly flagged. The whole
    /// reconstruction tuple — including `disk_pos_*` — is left immutable so the
    /// reconstructed receipt always carries the ORIGINAL frame position.
    pub(crate) fn mark_evicted(&self, is_live: impl Fn(u128) -> bool) {
        for mut entry in self.map.iter_mut() {
            if !entry.event_evicted && !is_live(entry.event_id) {
                entry.event_evicted = true;
            }
        }
    }

    /// Replace the whole map from a restored vector (cold-start authority).
    /// Existing contents are cleared first — this is the unconditional restore.
    pub(crate) fn restore(&self, entries: Vec<IdempEntry>) {
        self.map.clear();
        for entry in entries {
            self.map.insert(entry.key, entry);
        }
    }

    /// Snapshot all entries for persistence. Iteration is not linearizable but
    /// flushes always run from a quiesced path (close / compaction tail).
    pub(crate) fn snapshot(&self) -> Vec<IdempEntry> {
        self.map.iter().map(|r| r.value().clone()).collect()
    }

    /// THE window-priority hybrid eviction rule. See module docs and
    /// INV-IDEMPOTENCY-DURABLE-WINDOW.
    ///
    /// `frontier` is the current global-sequence frontier (next-to-assign).
    ///
    /// Invariant proved by `evict`: a key whose `recorded_global_sequence` is
    /// within `keep_sequences` of `frontier` is NEVER removed here — not by
    /// window-aging (it is inside the window) and not by the cap (the cap only
    /// touches the out-of-window region). The cap can never make a
    /// within-window retry re-append.
    pub(crate) fn evict(&self, frontier: u64) -> EvictionReport {
        let mut report = EvictionReport::default();

        // The window floor: an entry is INSIDE the window iff
        // recorded_global_sequence >= window_floor. With no keep_sequences
        // (Unbounded) everything is inside the window (floor 0) and nothing
        // ages out.
        let window_floor = match self.retention.keep_sequences() {
            None => {
                report.remaining = self.map.len() as u64;
                return report;
            }
            Some(keep) => frontier.saturating_sub(keep),
        };

        // ── Step 1: window-aging. Trim keys OLDER than the window. This is the
        // only place a key leaves the window; a within-window key is never a
        // candidate here because its recorded_global_sequence >= window_floor.
        let aged: Vec<u128> = self
            .map
            .iter()
            .filter(|r| r.value().recorded_global_sequence < window_floor)
            .map(|r| *r.key())
            .collect();
        for key in &aged {
            self.map.remove(key);
        }
        report.aged_out = aged.len() as u64;

        // ── Step 2: soft cap. The cap may ONLY evict keys ALREADY OUTSIDE the
        // window — pure free win on the out-of-window tail. It must NEVER cross
        // into within-window territory. After step 1 the only remaining
        // out-of-window keys are those with recorded_global_sequence ==
        // window_floor..(strictly < frontier-keep is already gone); in
        // practice step 1 removes everything strictly below window_floor, so
        // step 2 has nothing extra to trim unless window_floor itself sits on
        // the boundary. We keep the structure explicit so the invariant is
        // legible and future window definitions stay safe.
        if let Some(max_keys) = self.retention.max_keys() {
            let len = self.map.len() as u64;
            if len > max_keys {
                // Count within-window keys. If they ALONE exceed the cap, the
                // window wins (residual pigeonhole): we do NOT trim them.
                let within_window = self
                    .map
                    .iter()
                    .filter(|r| r.value().recorded_global_sequence >= window_floor)
                    .count() as u64;

                if within_window >= max_keys {
                    // Residual pigeonhole: window wins, store exceeds max_keys.
                    report.within_window_exceeds_cap = true;
                    tracing::warn!(
                        target: "batpak::idemp",
                        len,
                        max_keys,
                        within_window,
                        "durable idempotency store exceeds soft cap from within-window keys \
                         alone (key-rate spike); window wins, correctness preserved, store \
                         temporarily over cap (bounded by rate x window)"
                    );
                } else {
                    // Trim only the out-of-window surplus down toward the cap.
                    let trim_target = len.saturating_sub(max_keys);
                    let out_of_window: Vec<u128> = self
                        .map
                        .iter()
                        .filter(|r| r.value().recorded_global_sequence < window_floor)
                        .map(|r| *r.key())
                        .take(usize::try_from(trim_target).unwrap_or(usize::MAX))
                        .collect();
                    for key in &out_of_window {
                        self.map.remove(key);
                    }
                    report.cap_trimmed_out_of_window = out_of_window.len() as u64;
                }
            }
        }

        report.remaining = self.map.len() as u64;
        report
    }

    /// Persist the current map to `index.idemp` atomically as a v2 AUTHORITY
    /// IMAGE. Runs from quiesced paths (close, compaction tail, snapshot,
    /// fork). Format:
    /// `magic(6) | version(2 le) | crc(4 le) | body(msgpack IdempImageV2)`
    /// where the body binds the entries to the store's lineage identity and
    /// the compound history anchor at flush time (#189). CRC covers the body
    /// only (same layout as the checkpoint footer).
    pub(crate) fn flush(
        &self,
        data_dir: &Path,
        fs: &dyn StoreFs,
        lineage: u128,
        anchor: Option<crate::store::store_meta::IdempAuthorityAnchor>,
    ) -> Result<(), StoreError> {
        let entries = self.snapshot();
        let count = entries.len();
        let image = IdempImageV2 {
            lineage,
            anchor,
            entries,
        };
        let body = crate::encoding::to_bytes(&image)
            .map_err(|error| StoreError::ser_msg(&format!("encode idemp store: {error}")))?;
        let crc = crc32fast::hash(&body);
        let final_path = data_dir.join(IDEMP_FILENAME);
        write_file_atomically_with_fs(
            data_dir,
            &final_path,
            "idempotency-store",
            |file| {
                file.write_all(&crate::store::wire_header::encode(
                    IDEMP_MAGIC,
                    IDEMP_VERSION,
                    crc,
                ))
                .map_err(StoreError::Io)?;
                file.write_all(&body).map_err(StoreError::Io)?;
                Ok(())
            },
            fs,
        )?;
        tracing::debug!(
            target: "batpak::idemp",
            count,
            "flushed durable idempotency authority image"
        );
        Ok(())
    }
}

// The on-disk image format + fail-closed admission live in the sibling
// `idemp_image` module (split to keep this file within the size cap); the
// stable crate-internal paths re-export from here.
pub(super) use super::idemp_image::IdempImageV2;
pub(crate) use super::idemp_image::{
    load_idempotency_authority, peek_idemp_version, IdempLoad, IdempVersionWitness,
};

#[cfg(test)]
#[path = "idemp_tests.rs"]
mod tests;

//! StoreFs-level fork crash recovery: classify fork destinations using the
//! same {CommittedPrefix | RolledBack | CanonicalRefusal} oracle as B3.

use super::fs::SimFs;
use super::recovery::{fold, is_canonical_refusal, FNV_OFFSET};
use super::recovery_matrix::Classification;
use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::store::fork_report::ForkOptions;
use crate::store::{Open, Store, StoreConfig, StoreError};
use std::path::Path;
use std::sync::Arc;

/// How a fork destination classifies after a StoreFs-level crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForkFaultOutcome {
    pub classification: Classification,
    pub digest: u64,
    pub dest_event_count: usize,
}

fn classification_token(classification: Classification) -> u64 {
    match classification {
        Classification::CommittedPrefix => 0xC0_01,
        Classification::RolledBack => 0xC0_02,
        Classification::CanonicalRefusal => 0xC0_03,
    }
}

fn outcome_digest(seed: u64, classification: Classification, dest_event_count: usize) -> u64 {
    let mut d = fold(FNV_OFFSET, seed);
    d = fold(d, classification_token(classification));
    fold(d, dest_event_count as u64)
}

/// Classify `dest` after a fault: legal outcomes only.
pub(crate) fn classify_fork_destination(
    dest: &Path,
    fs: &Arc<dyn crate::store::platform::fs::StoreFs>,
) -> Result<(Classification, usize), StoreError> {
    if fs.metadata(dest).is_err() {
        return Ok((Classification::RolledBack, 0));
    }
    match Store::open_read_only(StoreConfig::new(dest).with_fs(Arc::clone(fs))) {
        Ok(store) => {
            let count = store.stats().event_count;
            if count == 0 {
                Ok((Classification::RolledBack, 0))
            } else {
                Ok((Classification::CommittedPrefix, count))
            }
        }
        Err(error) if is_canonical_refusal(&error) => Ok((Classification::CanonicalRefusal, 0)),
        Err(error) => Err(error),
    }
}

/// One seeded fork-under-fault scenario over SimFs (StoreFs-level faults only),
/// the fault layer composed over the production [`RealFs`] backend.
///
/// [`RealFs`]: crate::store::platform::fs::RealFs
pub(crate) fn run_seeded_fork_fault(seed: u64) -> Result<ForkFaultOutcome, String> {
    let inner: Arc<dyn crate::store::platform::fs::StoreFs> =
        Arc::new(crate::store::platform::fs::RealFs);
    run_seeded_fork_fault_over(seed, &inner)
}

/// [`run_seeded_fork_fault`] with the SimFs fault layer composed over an
/// arbitrary `inner` backend, so the SAME seeded scenario drives a store over
/// real files or over a pure in-memory backend ([`MemFs`](crate::store::MemFs)).
pub(crate) fn run_seeded_fork_fault_over(
    seed: u64,
    inner: &Arc<dyn crate::store::platform::fs::StoreFs>,
) -> Result<ForkFaultOutcome, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("seed=0x{seed:X}: tmpdir: {e}"))?;
    let source_dir = dir.path().join("source");
    let dest_dir = dir.path().join("dest");

    let fsync_drop = if seed.is_multiple_of(5) { 4 } else { 0 };
    let sim_fs = Arc::new(SimFs::layered(
        seed ^ 0xF0_0F_00,
        fsync_drop,
        Arc::clone(inner),
    ));
    let config = StoreConfig::new(&source_dir)
        .with_sync_every_n_events(1)
        .with_segment_max_bytes(512)
        .with_fs(Arc::clone(&sim_fs) as Arc<dyn crate::store::platform::fs::StoreFs>);

    let store = Store::<Open>::open(config).map_err(|e| format!("seed=0x{seed:X}: open: {e}"))?;
    let steps = 3 + (seed % 5) as usize;
    let kind = EventKind::custom(0xF, 0x0A);
    for i in 0..steps {
        let coord = Coordinate::new(format!("entity-{i}"), "scope:fork")
            .map_err(|e| format!("seed=0x{seed:X}: coord: {e}"))?;
        let _receipt = store
            .append(&coord, kind, &serde_json::json!({ "n": i }))
            .map_err(|e| format!("seed=0x{seed:X}: append: {e}"))?;
    }
    crate::store::lifecycle::sync(&store).map_err(|e| format!("seed=0x{seed:X}: sync: {e}"))?;

    // Committed source prefix at fork time. A fork is a CoW copy of the synced
    // source, so a CommittedPrefix recovery must reproduce this exact count —
    // not `steps`, which counts only user appends and excludes the store's
    // SYSTEM_INIT lifecycle event.
    let source_committed = store.stats().event_count;

    store
        .fork_with_evidence(&dest_dir, ForkOptions::default())
        .map_err(|e| format!("seed=0x{seed:X}: fork: {e}"))?;

    sim_fs.crash();

    let (classification, dest_event_count) = classify_fork_destination(&dest_dir, inner)
        .map_err(|e| format!("seed=0x{seed:X}: classify: {e}"))?;

    if matches!(classification, Classification::CommittedPrefix)
        && dest_event_count != source_committed
    {
        return Err(format!(
            "seed=0x{seed:X}: fork dest event count {dest_event_count} != source {source_committed}"
        ));
    }

    Ok(ForkFaultOutcome {
        classification,
        digest: outcome_digest(seed, classification, dest_event_count),
        dest_event_count,
    })
}

/// Doc-hidden public mirror for integration tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForkFaultOutcomePublic {
    /// Recovered classification using the B3 oracle.
    pub classification: super::recovery_matrix::RecoveredClass,
    /// Determinism digest for this seed + outcome.
    pub digest: u64,
    /// Visible events in the fork destination when classification is CommittedPrefix.
    pub dest_event_count: usize,
}

/// Run one seeded fork-under-fault scenario (StoreFs-level).
///
/// # Errors
/// Returns a seed-tagged description string when the scenario cannot run or the
/// recovered fork destination classifies as an illegal outcome.
pub fn run_seeded_fork_fault_public(seed: u64) -> Result<ForkFaultOutcomePublic, String> {
    run_seeded_fork_fault(seed).map(|o| ForkFaultOutcomePublic {
        classification: o.classification.into(),
        digest: o.digest,
        dest_event_count: o.dest_event_count,
    })
}

/// [`run_seeded_fork_fault_public`], but with the SimFs fault layer composed
/// over a pure in-memory [`MemFs`](crate::store::MemFs) backend — the SAME
/// seeded crash-recovery scenario proven backend-agnostic, with no host files.
///
/// # Errors
/// As [`run_seeded_fork_fault_public`].
pub fn run_seeded_fork_fault_mem_fs_public(seed: u64) -> Result<ForkFaultOutcomePublic, String> {
    let inner: Arc<dyn crate::store::platform::fs::StoreFs> = Arc::new(crate::store::MemFs::new());
    run_seeded_fork_fault_over(seed, &inner).map(|o| ForkFaultOutcomePublic {
        classification: o.classification.into(),
        digest: o.digest,
        dest_event_count: o.dest_event_count,
    })
}

/// Replay seed helper honoring `BATPAK_SEED`.
pub fn fork_fault_replay_seed(default: u64) -> u64 {
    super::seed_from_env(default)
}

#[cfg(all(test, feature = "dangerous-test-hooks"))]
mod tests {
    use super::*;

    fn real_fs() -> Arc<dyn crate::store::platform::fs::StoreFs> {
        Arc::new(crate::store::platform::fs::RealFs)
    }

    /// A dest whose read-only open fails with a CANONICAL corruption error must
    /// classify as `CanonicalRefusal` — never propagate as a hard `Err`. A dir
    /// holding only a malformed compaction marker fails cold start with
    /// `DataDirMalformed` (one of the canonical-refusal variants). Replacing the
    /// `is_canonical_refusal` match guard with `false` would turn this into an
    /// `Err`, which this test forbids.
    #[test]
    fn classify_maps_a_canonical_corruption_error_to_canonical_refusal() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let dest = dir.path().join("corrupt-store");
        crate::store::platform::fs::create_dir_all(&dest).expect("create dest dir");
        let marker = dest.join(crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME);
        crate::store::platform::fs::write_derivative_file_atomically(
            &dest,
            &marker,
            "malformed compaction marker",
            b"this is not valid json",
        )
        .expect("plant malformed compaction marker");

        let classified = classify_fork_destination(&dest, &real_fs());
        assert!(
            matches!(classified, Ok((Classification::CanonicalRefusal, 0))),
            "a canonical corruption error must map to Ok(CanonicalRefusal), got {classified:?}"
        );
    }

    /// A dest whose read-only open fails with a NON-canonical error (here a
    /// regular file standing in for a data dir) must propagate as `Err` — it is
    /// NOT a canonical refusal. Replacing the match guard with `true` would
    /// swallow every open error as `CanonicalRefusal`, which this test forbids.
    #[test]
    fn classify_propagates_a_non_canonical_open_error_as_err() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let dest_file = dir.path().join("not-a-directory");
        crate::store::platform::fs::create_new_file(&dest_file).expect("create plain file");

        let classified = classify_fork_destination(&dest_file, &real_fs());
        assert!(
            classified.is_err(),
            "a non-canonical open error must propagate as Err, not be swallowed as \
             CanonicalRefusal, got {classified:?}"
        );
    }

    // NOTE: the `seed ^ 0xF0_0F_00` → `seed | 0xF0_0F_00` mutant on line 64 is a
    // GENUINE EQUIVALENT mutant and is intentionally left uncovered here. That
    // expression only seeds SimFs's fsync-drop PRNG, while the returned
    // `ForkFaultOutcome.digest` folds solely `(seed, classification,
    // dest_event_count)`. A CoW fork of a fully-synced source recovers the whole
    // committed prefix on every fault-active seed (verified: 800 multiple-of-5
    // seeds all classify `CommittedPrefix` at `dest_event_count ==
    // source_committed`), and `source_committed` is the live in-memory count
    // (drop-schedule independent). So the sub-seed — and thus XOR vs OR — cannot
    // change the digest. Reported for registry exclusion.
}

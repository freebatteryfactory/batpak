//! GAUNT-IDEMPOTENCY-AUTHORITY (#189/#177), compaction half: compaction plus
//! authority persistence is ONE token-bound, crash-recoverable commit.
//!
//! PROVES: INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION and (CONSUMES plan-06)
//! INV-COMPACTION-NAMESPACE-COMMIT — a failed authority-image publish aborts the
//! compaction in its PRE-COMMIT phase (rolling the disk back before any source
//! frame is destroyed and never swapping the live index), and a store that dies
//! without a clean close reopens into exactly the OLD complete generation (fault
//! before the commit) or exactly the NEW complete generation (fault after it) —
//! never a mixed state — with every keyed retry returning its ORIGINAL receipt.
//! The oracle is an event-census state machine (`classify_generation`), not byte
//! equality: the staged-name namespace choreography relocates source bytes, so a
//! filename/byte oracle would false-fail; content is the only honest witness.
//! CATCHES: deleting a source before the authority image is durable, admitting an
//! authority image not bound to the recovered generation (undead events), a keyed
//! retry double-appending after recovery, and clean-close "healing" masking a
//! crash-recovery bug.
//! SEEDED: `MANIFEST_V1` keyed workload on real `RealFs` stores; a delegating
//! `StoreFs` that fails exactly the `index.idemp` atomic publish once while armed;
//! recovery driven by `drop(store)` (crash), never `close()`.
//!
//! Split from `idempotency_authority_fail_closed.rs` (admission half); the shared
//! crash workload/oracle lives in `compaction_crash/mod.rs`. The 9-boundary
//! namespace walk lives in `compaction_crash_windows.rs`.

#[path = "compaction_crash/mod.rs"]
mod harness;

use batpak::store::segment::CompactionOutcome;
use batpak::store::{
    CopyPreference, CowStrategyUsed, DirEntryInfo, FileStat, ParentDirSyncAdmission, RealFs,
    StagedFile, Store, StoreDirLockGuard, StoreError, StoreFile, StoreFs,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

const IDEMP_FILENAME: &str = "index.idemp";

// ---------------------------------------------------------------------------
// Delegating `StoreFs` that fails exactly the atomic PUBLISH of `index.idemp`
// once while armed, forwarding every other operation to `RealFs`. This targets
// the ERROR path of the authority publish (the pre-commit image write), which is
// distinct from the crash walk and still required (#189): a failed publish must
// abort the compaction before any destructive step.
// ---------------------------------------------------------------------------

/// Delegating [`StoreFs`] that fails the atomic PUBLISH of `index.idemp`
/// exactly once while armed. Every other operation forwards to [`RealFs`].
struct FailIdempPublishFs {
    inner: RealFs,
    arm: Arc<AtomicBool>,
}

struct FailIdempPublishStaged {
    inner: Box<dyn StagedFile>,
    arm: Arc<AtomicBool>,
}

impl StagedFile for FailIdempPublishStaged {
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(buf)
    }

    fn sync_all(&mut self) -> std::io::Result<()> {
        self.inner.sync_all()
    }

    fn persist(
        self: Box<Self>,
        final_path: &Path,
        admission: ParentDirSyncAdmission,
    ) -> std::io::Result<()> {
        let is_idemp = final_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == IDEMP_FILENAME);
        if is_idemp && self.arm.swap(false, Ordering::SeqCst) {
            return Err(std::io::Error::other(
                "injected fault: index.idemp publish failed",
            ));
        }
        self.inner.persist(final_path, admission)
    }
}

impl StoreFs for FailIdempPublishFs {
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<DirEntryInfo>> {
        self.inner.read_dir(path)
    }
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_dir_all(path)
    }
    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        self.inner.create_new_file(path)
    }
    fn open_file(&self, path: &Path) -> std::io::Result<Box<dyn StoreFile>> {
        self.inner.open_file(path)
    }
    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError> {
        self.inner.sync_parent_dir(path)
    }
    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError> {
        self.inner.reject_symlink_leaf(path, purpose)
    }
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.inner.read(path)
    }
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }
    fn symlink_metadata(&self, path: &Path) -> std::io::Result<FileStat> {
        self.inner.symlink_metadata(path)
    }
    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: CopyPreference,
    ) -> std::io::Result<CowStrategyUsed> {
        self.inner.cow_copy_file(from, to, preference)
    }
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        self.inner.copy(from, to)
    }
    fn metadata(&self, path: &Path) -> std::io::Result<FileStat> {
        self.inner.metadata(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.inner.rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)
    }
    fn named_temp_in(&self, dir: &Path) -> std::io::Result<Box<dyn StagedFile>> {
        let inner = self.inner.named_temp_in(dir)?;
        Ok(Box::new(FailIdempPublishStaged {
            inner,
            arm: Arc::clone(&self.arm),
        }))
    }
    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        self.inner.try_lock_store_dir(lock_path)
    }
}

/// The retention config evicts every `EVICT_KIND` frame, so the NEW generation's
/// evict-kind census is empty; a recovered non-empty census that is neither the
/// full OLD set nor empty is a torn/mixed generation the classifier rejects.
fn new_evict_census() -> BTreeSet<u128> {
    BTreeSet::new()
}

#[test]
fn compaction_authority_publish_failure_fails_compaction_and_recovers_old_generation(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#189/#177): a failed authority publish fails the compaction BEFORE
    // any destructive step, and a subsequent CRASH (drop, never close) reopens into
    // the complete ORIGINAL generation — never a mixed state.
    let dir = TempDir::new().expect("tempdir");
    let arm = Arc::new(AtomicBool::new(false));
    let fs = Arc::new(FailIdempPublishFs {
        inner: RealFs,
        arm: Arc::clone(&arm),
    });

    let store = Store::open(harness::config(dir.path()).with_fs(fs)).expect("open on fault fs");
    let pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
    let old = harness::kind_census(&store, harness::EVICT_KIND);
    let survivors = harness::kind_census(&store, harness::SURVIVOR_KIND);

    // Arm the fault: the compaction's authority-image publish fails during the
    // PRE-COMMIT phase. The rollback restores the source set and reports
    // `CompactionOutcome::Failed` (rollback of a live marker is not an `Err`).
    arm.store(true, Ordering::SeqCst);
    let (result, _report) = store
        .compact(&harness::evict_all_evict_kind())
        .expect("a pre-commit publish failure rolls the disk back and reports Failed");
    let CompactionOutcome::Failed { reason } = &result.outcome else {
        return Err(std::io::Error::other(format!(
            "PROPERTY (#189/#177): a failed authority publish must abort the compaction as \
             CompactionOutcome::Failed before any destructive step; got {:?}",
            result.outcome
        ))
        .into());
    };
    assert!(
        reason.contains("pre-commit phase failed"),
        "the Failed reason names the pre-commit rollback: {reason}"
    );
    assert!(
        !arm.load(Ordering::SeqCst),
        "the injected fault actually fired (non-vacuity)"
    );

    // The live store still deduplicates every keyed lane: the index was never
    // swapped, so a keyed retry is a no-op returning the original receipt.
    for (key, receipt) in &pairs {
        harness::assert_retry_is_original(&store, *key, receipt);
    }

    // Crash: drop WITHOUT close (no clean-close healing), then reopen on RealFs.
    drop(store);
    let reopened =
        Store::open(harness::config(dir.path())).expect("reopen after failed compaction");

    let recovered = harness::kind_census(&reopened, harness::EVICT_KIND);
    let generation = harness::classify_generation(&recovered, &old, &new_evict_census())
        .map_err(std::io::Error::other)?;
    if generation != harness::RecoveredGeneration::OldComplete {
        return Err(std::io::Error::other(format!(
            "PROPERTY (#189/#177): a pre-commit publish failure reopens into the complete OLD \
             generation, never a mixed state; got {generation:?}"
        ))
        .into());
    }
    assert_eq!(
        harness::kind_census(&reopened, harness::SURVIVOR_KIND),
        survivors,
        "the retention-surviving lane is intact across recovery"
    );
    for (key, receipt) in &pairs {
        harness::assert_retry_is_original(&reopened, *key, receipt);
    }
    assert_eq!(
        harness::kind_census(&reopened, harness::EVICT_KIND),
        recovered,
        "keyed retries after recovery appended NOTHING (no double-append / invented event)"
    );
    reopened.close().expect("close reopened");
    Ok(())
}

#[test]
fn committed_compaction_without_clean_close_recovers_new_generation_bound_to_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177): after the commit point the replacement generation and its
    // published authority are mutually bound — a crash (drop, never close)
    // immediately after a successful compact reopens into the complete NEW
    // generation and every keyed retry returns its ORIGINAL receipt from the
    // durable authority alone.
    let dir = TempDir::new().expect("tempdir");
    let store = Store::open(harness::config(dir.path())).expect("open");
    let pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
    let old = harness::kind_census(&store, harness::EVICT_KIND);
    let survivors = harness::kind_census(&store, harness::SURVIVOR_KIND);

    let (result, _report) = store
        .compact(&harness::evict_all_evict_kind())
        .expect("compaction");
    let CompactionOutcome::Performed = result.outcome else {
        return Err(std::io::Error::other(format!(
            "PROPERTY (#177): a clean compaction over the sealed set must report Performed; got {:?}",
            result.outcome
        ))
        .into());
    };

    // Crash immediately after the commit — drop WITHOUT close.
    drop(store);
    let reopened =
        Store::open(harness::config(dir.path())).expect("reopen after committed compaction");

    let recovered = harness::kind_census(&reopened, harness::EVICT_KIND);
    let generation = harness::classify_generation(&recovered, &old, &new_evict_census())
        .map_err(std::io::Error::other)?;
    if generation != harness::RecoveredGeneration::NewComplete {
        return Err(std::io::Error::other(format!(
            "PROPERTY (#177): a committed compaction reopens into the complete NEW generation; \
             got {generation:?}"
        ))
        .into());
    }
    assert_eq!(
        harness::kind_census(&reopened, harness::SURVIVOR_KIND),
        survivors,
        "the retention-surviving lane is intact across recovery"
    );
    // The evict-kind frames are gone, so every keyed retry that still returns its
    // ORIGINAL receipt proves the durable authority alone deduplicated it — an
    // image not bound to this generation would have refused the open.
    for (key, receipt) in &pairs {
        harness::assert_retry_is_original(&reopened, *key, receipt);
    }
    assert_eq!(
        harness::kind_census(&reopened, harness::EVICT_KIND),
        recovered,
        "keyed retries re-appended NOTHING (durable-authority dedup, no undead event)"
    );
    reopened.close().expect("close reopened");
    Ok(())
}

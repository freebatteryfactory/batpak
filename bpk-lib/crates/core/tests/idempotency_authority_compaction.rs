//! GAUNT-IDEMPOTENCY-AUTHORITY (#189), compaction half: compaction plus
//! authority persistence is ONE recoverable commit.
//!
//! PROVES: INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION — the new authority
//! image is durably published BEFORE any source segment is deleted, a failed
//! image publish fails the compaction instead of being ignored, and every
//! fault boundary yields a legal old-or-new state in which a keyed retry can
//! never double-append (the state-machine walk Fresh -> Keyed -> Compacting ->
//! AuthorityCommitted -> SourcesRetired, driven at its authority boundary).
//! CATCHES: ignoring the sidecar flush failure, deleting source frames before
//! the image is durable, and clearing destructive state ahead of authority
//! durability.
//! SEEDED: real stores with tiny segments; a delegating `StoreFs` that fails
//! exactly the `index.idemp` publish, arming/disarming deterministically.
//!
//! Split from `idempotency_authority_fail_closed.rs` (admission half) to keep
//! each doctrine-bearing harness within the absolute size cap.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::{EntityIdType, IdempotencyKey};
use batpak::store::{
    AppendOptions, CompactionConfig, CompactionStrategy, CopyPreference, CowStrategyUsed,
    DirEntryInfo, FileStat, ParentDirSyncAdmission, RealFs, StagedFile, Store, StoreConfig,
    StoreDirLockGuard, StoreError, StoreFile, StoreFs,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 5);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:compaction").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

fn append_keyed(store: &Store, key: u128) -> batpak::store::AppendReceipt {
    let payload_tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            KIND,
            &serde_json::json!({ "k": payload_tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed append")
}

fn key_event_count(store: &Store, key: u128) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_kind() == KIND && e.event_id().as_u128() == key)
        .count()
}

/// Retention strategy that evicts EVERY user event of `KIND` (keeps only the
/// batch/system markers), forcing keyed event frames out of the store.
fn evict_all_user_events() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != KIND
        })),
        min_segments: 1,
    }
}

// ---------------------------------------------------------------------------
// Fixtures 4/5: a fault at the compaction authority boundary yields only
// legal old-or-new states. The delegating `StoreFs` fails exactly the
// `index.idemp` publish; the compaction must surface the failure BEFORE any
// source segment is deleted, and the store must remain fully recoverable.
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

fn sealed_segment_count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .expect("read store dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "fbat")
        })
        .count()
}

#[test]
fn compaction_idemp_flush_failure_preserves_old_recoverable_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let arm = Arc::new(AtomicBool::new(false));
    let fs = Arc::new(FailIdempPublishFs {
        inner: RealFs,
        arm: Arc::clone(&arm),
    });
    let key = 0x7000_0000_0000_0000_0000_0000_0000_0007u128;

    let store = Store::open(config(dir.path()).with_fs(fs)).expect("open");
    let first = append_keyed(&store, key);
    for i in 0..8 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "filler": i }))
            .expect("append filler event");
    }
    let segments_before = sealed_segment_count(dir.path());

    // Arm the fault: the compaction's authority-image publish fails, and the
    // compaction MUST surface it before deleting any source segment.
    arm.store(true, Ordering::SeqCst);
    let err = match store.compact(&evict_all_user_events()) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a failed authority publish must fail the compaction — \
                 ignoring it would let destructive deletion proceed on an unproven image",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::Io(_) = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        !arm.load(Ordering::SeqCst),
        "the injected fault actually fired (non-vacuity)"
    );
    assert_eq!(
        sealed_segment_count(dir.path()),
        segments_before,
        "PROPERTY (#189): no source segment was deleted before the authority image was durable \
         — old-or-new holds at the flush boundary"
    );

    // The live store still deduplicates, and a clean close + reopen recovers.
    let replay = append_keyed(&store, key);
    assert_eq!(
        replay.global_sequence, first.global_sequence,
        "retry on the live store after the failed compaction is still a no-op"
    );
    store.close().expect("close");

    let reopened = Store::open(config(dir.path())).expect("reopen after failed compaction");
    let replay = append_keyed(&reopened, key);
    assert_eq!(
        replay.global_sequence, first.global_sequence,
        "PROPERTY (#189): after recovery the retry still returns the original receipt — \
         at no boundary could it double-append"
    );
    assert_eq!(
        key_event_count(&reopened, key),
        0,
        "the retention-evicted frame stays evicted across recovery (roll-forward is a legal \
         new-state outcome) — the retry re-appended NOTHING; the receipt above came from the \
         durable authority alone"
    );
    reopened.close().expect("close reopened");
    Ok(())
}

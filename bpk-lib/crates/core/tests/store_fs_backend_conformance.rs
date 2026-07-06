//! MemFs backend conformance + the #168 runtime falsifier.
//!
//! PROVES the handle-abstracted seam is genuinely implementable off-POSIX:
//! the SAME contract corpus that pins `RealFs` (see `tests/common/mod.rs`,
//! driven for RealFs by `store_fs_public_seam.rs`) passes over the pure
//! in-memory [`MemFs`], and a real `Store` runs its full life cycle —
//! open → append → durable wait → read → receipt verify → close → REOPEN
//! (cold-start rehydration) — without ever touching the host filesystem.
//! This is issue #168's checklist executed natively; the premise it
//! falsified ("SimFs is an in-memory StoreFs") is replaced by an actual one.
//!
//! The corpus itself is also proven to BITE: a deliberately-broken backend
//! (create-new exclusivity violated) must FAIL the shared body — a passing
//! corpus over a divergent backend would be a vacuous conformance claim.

use std::path::Path;
use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{MemFs, StoreDirLockGuard, StoreError, StoreFile, StoreFs};

mod common;

#[derive(serde::Serialize, serde::Deserialize, EventPayload)]
#[batpak(category = 0xF, type_id = 22)]
struct MemProbe {
    value: i64,
}

#[test]
fn mem_fs_upholds_the_documented_backend_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fs = MemFs::new();
    let root = Path::new("/virtual/conformance");
    fs.create_dir_all(root)?;
    common::backend_upholds_the_documented_contract(&fs, root)
}

#[test]
fn store_runs_end_to_end_on_a_purely_in_memory_backend() -> Result<(), Box<dyn std::error::Error>> {
    // The #168 falsifier, natively: no tempdir, no host path — the "data
    // dir" exists only inside the MemFs tree. Clone the backend so the
    // second open sees the same tree (Arc-shared state).
    let fs = MemFs::new();
    let data_dir = Path::new("/virtual/store");

    let config = StoreConfig::new(data_dir)
        .with_fs(Arc::new(fs.clone()))
        .with_sync_every_n_events(1);
    let store = Store::open(config)?;

    let coord = Coordinate::new("entity:mem", "scope:e2e")?;
    let receipt = store.append_typed(&coord, &MemProbe { value: 41 })?;
    let fetched = store.get(receipt.event_id)?;
    assert_eq!(fetched.event.header.event_id, receipt.event_id);
    assert!(
        store.verify_append_receipt(&receipt).is_valid(),
        "a store on a purely in-memory backend must verify its own receipt"
    );
    store.close()?;

    // REOPEN over the same in-memory tree: cold-start rehydration (segment
    // scan / cold-start artifacts / dir lock) must run entirely through the
    // seam and recover the appended event.
    let reopened = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())))?;
    let recovered = reopened.get(receipt.event_id)?;
    assert_eq!(
        recovered.event.header.event_id, receipt.event_id,
        "reopen over the in-memory tree must recover the appended event"
    );
    let second = reopened.append_typed(&coord, &MemProbe { value: 42 })?;
    assert!(
        reopened.verify_append_receipt(&second).is_valid(),
        "appends after an in-memory reopen must keep verifying"
    );
    reopened.close()?;

    Ok(())
}

#[test]
fn in_memory_store_directory_lock_excludes_a_second_open() -> Result<(), Box<dyn std::error::Error>>
{
    // The runtime IS the lock for a virtual backend: while one store holds
    // the in-process registry entry, a second open over the same tree and
    // path must fail with the exact StoreLocked variant.
    let fs = MemFs::new();
    let data_dir = Path::new("/virtual/locked");

    let store = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())))?;
    let second = Store::open(StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone())));
    match second {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: a second open over a live in-memory store must be excluded",
            )
            .into())
        }
        Err(error) => assert!(
            matches!(error, StoreError::StoreLocked { .. }),
            "expected StoreLocked, got {error}"
        ),
    }
    store.close()?;
    Ok(())
}

/// A deliberately-divergent backend: create-new exclusivity is violated
/// (an existing file is silently replaced). Everything else delegates to a
/// conforming MemFs.
struct BrokenExclusivityFs {
    inner: MemFs,
}

impl StoreFs for BrokenExclusivityFs {
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<batpak::store::DirEntryInfo>> {
        self.inner.read_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        // THE PLANTED DIVERGENCE: silently clobber instead of refusing.
        drop(self.inner.remove_file(path));
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

    fn canonicalize(&self, path: &Path) -> std::io::Result<std::path::PathBuf> {
        self.inner.canonicalize(path)
    }

    fn symlink_metadata(&self, path: &Path) -> std::io::Result<batpak::store::FileStat> {
        self.inner.symlink_metadata(path)
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: batpak::store::CopyPreference,
    ) -> std::io::Result<batpak::store::CowStrategyUsed> {
        self.inner.cow_copy_file(from, to, preference)
    }

    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        self.inner.copy(from, to)
    }

    fn metadata(&self, path: &Path) -> std::io::Result<batpak::store::FileStat> {
        self.inner.metadata(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)
    }

    fn named_temp_in(&self, dir: &Path) -> std::io::Result<Box<dyn batpak::store::StagedFile>> {
        self.inner.named_temp_in(dir)
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        self.inner.try_lock_store_dir(lock_path)
    }
}

#[test]
fn conformance_corpus_detects_a_divergent_backend() {
    // RED-fixture discipline for the shared-drive corpus: a backend that
    // violates create-new exclusivity must FAIL the shared body. The corpus
    // reports divergence either as an `Err` or as a failed assertion (a
    // panic) — both count as detection here. If this test ever sees the
    // corpus succeed, the corpus stopped biting.
    let fs = BrokenExclusivityFs {
        inner: MemFs::new(),
    };
    let root = Path::new("/virtual/divergent");
    fs.create_dir_all(root)
        .expect("in-memory dir creation is infallible");
    let verdict = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        common::backend_upholds_the_documented_contract(&fs, root)
    }));
    let detected = match verdict {
        Err(_assertion_panic) => true,
        Ok(outcome) => outcome.is_err(),
    };
    assert!(
        detected,
        "PROPERTY: the conformance corpus must detect a create-new exclusivity violation"
    );
}

/// Issue #171: `Store::diagnostics()` must resolve platform evidence THROUGH the
/// configured backend, never the host filesystem. This covers both failure
/// shapes. First, a `data_dir` that ALSO exists on the host (a real tempdir):
/// the pre-fix mmap probe created a `NamedTempFile` there and reported
/// `FileBacked`. Second, a PURELY virtual `data_dir` (`/virtual/...`, absent on
/// the host): the pre-fix `path_status` used host `metadata`, saw `NotFound`,
/// and reported the existing virtual store as `Unknown`. After the fix
/// `path_status` and the mmap probe both route through `fs`, so a MemFs store
/// reports `ObservedUnsupported` (never `FileBacked`/`Unknown`) and never
/// touches the host.
#[test]
fn diagnostics_over_memfs_reports_virtual_mmap_evidence_not_file_backed() {
    let host_dir = tempfile::tempdir().expect("real host tempdir");
    let data_dirs: [&Path; 2] = [host_dir.path(), Path::new("/virtual/diag-store")];

    for data_dir in data_dirs {
        let fs = MemFs::new();
        let config = StoreConfig::new(data_dir).with_fs(Arc::new(fs.clone()));
        let store = Store::open(config).expect("open MemFs store");

        let diagnostics = store.diagnostics();
        assert_eq!(
            diagnostics.platform_evidence.store_path.mmap_index,
            batpak::store::stats::MmapEvidence::ObservedUnsupported,
            "MemFs diagnostics must report ObservedUnsupported for data_dir {data_dir:?}, \
             never FileBacked (host tempfile probe) or Unknown (host path_status)"
        );

        store.close().expect("close");
    }
}

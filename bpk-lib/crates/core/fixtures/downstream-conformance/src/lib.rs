//! An OUT-OF-TREE `StoreFs` backend, implemented from a standalone workspace
//! using only `batpak`'s public surface, that consumes the published
//! `batpak::store::conformance` corpus (#179 acceptance, contract A12).
//!
//! [`DownstreamBackend`] is a transparent delegate over the public reference
//! [`MemFs`] — its whole purpose is to prove the `StoreFs` trait and its handle
//! types are implementable by a downstream author reaching for the clean
//! external `conformance-harness` feature alone, with no access to the internal
//! `dangerous-test-hooks` fault/poison levers. Delegation (not reimplementation)
//! keeps the fixture about the SEAM's reachability, not about re-proving MemFs.
//!
//! [`DownstreamFactory`] mints a fresh MemFs-backed backend per case (isolation
//! keyed on the stable case id) and declares NO hostile controls, so every
//! Crash-family case resolves to a typed qualification while the byte/namespace
//! contract cases must fully pass.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use batpak::store::conformance::{
    BackendFactory, FreshBackend, Qualification, StoreFsConformanceCase,
};
use batpak::store::{
    CopyPreference, CowStrategyUsed, DirEntryInfo, FileStat, MemFs, StagedFile, StoreDirLockGuard,
    StoreError, StoreFile, StoreFs,
};

/// A delegating out-of-tree [`StoreFs`] backed by the public [`MemFs`].
///
/// Every method forwards to `inner`; the fixture's value is that the forwarding
/// compiles at all — the trait and its handle types (`StoreFile`, `StagedFile`,
/// `StoreDirLockGuard`, `DirEntryInfo`, `FileStat`, `CowStrategyUsed`,
/// `CopyPreference`, `StoreError`) are all nameable and implementable from
/// outside the crate.
#[derive(Clone, Default)]
pub struct DownstreamBackend {
    inner: MemFs,
}

impl DownstreamBackend {
    /// A fresh delegate over an empty in-memory filesystem.
    #[must_use]
    pub fn new() -> Self {
        Self { inner: MemFs::new() }
    }
}

impl StoreFs for DownstreamBackend {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
        self.inner.read_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        self.inner.create_new_file(path)
    }

    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>> {
        self.inner.open_file(path)
    }

    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError> {
        self.inner.sync_parent_dir(path)
    }

    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError> {
        self.inner.reject_symlink_leaf(path, purpose)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileStat> {
        self.inner.symlink_metadata(path)
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: CopyPreference,
    ) -> io::Result<CowStrategyUsed> {
        self.inner.cow_copy_file(from, to, preference)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        self.inner.copy(from, to)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileStat> {
        self.inner.metadata(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    // Forwarded because MemFs overrides the provided default (a virtual directory
    // is a key prefix); the delegate must inherit that behavior, not the seam's
    // contents-only default. `remove_file_if_present` / `remove_dir_all_if_present`
    // keep the trait default here — they resolve through the forwarded primitives.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir_all(path)
    }

    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>> {
        self.inner.named_temp_in(dir)
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        self.inner.try_lock_store_dir(lock_path)
    }
}

/// A [`BackendFactory`] handing the corpus a fresh [`DownstreamBackend`] per case.
///
/// No hostile controls: a MemFs-delegate cannot simulate power loss, so the
/// Crash family qualifies (typed) instead of passing — a skip is never a pass.
pub struct DownstreamFactory;

impl BackendFactory for DownstreamFactory {
    fn backend_name(&self) -> String {
        "downstream-memfs-delegate".to_string()
    }

    fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
        // Per-case isolation keyed on the stable case id (MemFsFactory shape):
        // each case gets its own virtual tree so ordering can never pollute.
        let backend = DownstreamBackend::new();
        let root = PathBuf::from(format!("/conformance/{}", case.id()));
        backend
            .create_dir_all(&root)
            .map_err(|error| Qualification::ControlUnsupported {
                control: "create_dir_all",
                backend: "downstream-memfs-delegate".to_string(),
                reason: format!("could not stage the per-case root {}: {error}", root.display()),
            })?;
        Ok(FreshBackend {
            fs: Arc::new(backend),
            root,
            controls: None,
            keepalive: None,
        })
    }
}

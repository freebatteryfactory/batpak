//! [`StoreFs`] seam + handle types for [`ShadowFs`], the namespace-truth crash
//! simulator (plan-02 F2 / contract R1).
//!
//! The model core (state layout, arming/observation surface, `crash()`) lives
//! in the parent `shadow_fs` module; this child module implements the seam that
//! DRIVES that state: `impl StoreFs for ShadowFs` plus the [`StoreFile`] /
//! [`StagedFile`] / [`StoreDirLockGuard`] handles. Being a descendant module it
//! reaches the parent's private `ShadowState`/`ShadowTree`/`ShadowInode` fields
//! directly; every fault/observation DECISION routes through three parent-owned
//! helper methods so the schedule + mutation-log state has one owner.
//!
//! Two-truths model (see the parent module docs for the full state machine):
//! `visible` is what a running process observes NOW; `durable` is what survives
//! `crash()`. Mutating ops touch `visible` only; `durable` advances ONLY at an
//! honored parent-dir sync (explicit [`StoreFs::sync_parent_dir`] or the
//! persist-implicit one). Per-inode byte durability (`durable_len`) advances
//! only at an honored handle/staged sync. [`MemFs`] is the parity reference for
//! every fail-closed edge (dir collision → `IsADirectory`, missing parent →
//! `NotFound`, file-at-dir-path → `NotADirectory`, same-path non-destructive
//! copy/rename, in-process lock registry).
//!
//! Interface this module CONSUMES from the parent `shadow_fs` (the F1↔F2 seam):
//! - `ShadowFs { state: Arc<Mutex<ShadowState>> }`,
//!   `ShadowState { tree: ShadowTree, sched: ShadowSchedules }`,
//!   `ShadowTree { inodes: BTreeMap<u64, ShadowInode>, visible/durable:
//!   BTreeMap<PathBuf, u64>, dirs/locks: BTreeSet<PathBuf>, next_inode: u64 }`,
//!   `ShadowInode { bytes: Vec<u8>, durable_len: usize }`.
//! - `SimOpKind` (the logged op taxonomy, contract R1).
//! - `impl ShadowState` decision helpers, each of which also appends the op to
//!   the mutation log so one owner keeps its order:
//!   - `strike_mutation(&mut self, SimOpKind, path, to: Option<&Path>) ->
//!     Option<io::Error>` — logs the mutating op; returns the injected error
//!     (nothing applied) when a `arm_op_error`/`arm_op_error_at`/`arm_crash_at_op`
//!     schedule or the freeze-on-fault poison fires; else `None`.
//!   - `record_parent_sync(&mut self, parent) -> ParentSyncOutcome` — logs the
//!     parent-dir-sync event and decides `Applied`/`Dropped`/`Errored`.
//!   - `record_file_sync(&mut self) -> FileSyncOutcome` — logs the handle/staged
//!     sync and decides `Honored`/`Dropped`/`Errored`.
//!
//! [`MemFs`]: crate::store::MemFs

use super::{
    FileSyncOutcome, ParentSyncOutcome, ShadowFs, ShadowInode, ShadowState, ShadowTree, SimOpKind,
};
use crate::store::platform::fs::{
    CowStrategyUsed, DirEntryInfo, FileKind, FileStat, StagedFile, StoreDirLockGuard, StoreFile,
    StoreFs,
};
use crate::store::StoreError;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

fn guard(fs: &ShadowFs) -> MutexGuard<'_, ShadowState> {
    fs.state.lock().unwrap_or_else(PoisonError::into_inner)
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("ShadowFs: no such file or directory: {}", path.display()),
    )
}

fn is_a_directory(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::IsADirectory,
        format!("ShadowFs: is a directory: {}", path.display()),
    )
}

fn not_a_directory(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotADirectory,
        format!("ShadowFs: not a directory: {}", path.display()),
    )
}

/// The parent directory whose name durability a sync of `path` reconciles. A
/// bare/relative leaf's parent is the empty path, matching `parent_must_exist`.
fn parent_of(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new(""))
}

/// MemFs-parity precondition: a publish/create/copy/rename destination's parent
/// must be a known directory, else fail closed `NotFound` — never leave an
/// unreachable name `read_dir` could not enumerate.
fn parent_must_exist(tree: &ShadowTree, path: &Path) -> io::Result<()> {
    match path.parent() {
        None => Ok(()),
        Some(parent) if parent.as_os_str().is_empty() => Ok(()),
        Some(parent) if tree.dirs.contains(parent) => Ok(()),
        Some(parent) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ShadowFs: parent directory missing: {}", parent.display()),
        )),
    }
}

/// Mint a fresh inode holding `bytes` with `durable_len` synced, bind `path` to
/// it in the VISIBLE namespace (name pending durability until a parent sync).
fn insert_visible_file(tree: &mut ShadowTree, path: &Path, bytes: Vec<u8>, durable_len: usize) {
    let id = tree.next_inode;
    tree.next_inode = tree.next_inode.saturating_add(1);
    tree.inodes.insert(id, ShadowInode { bytes, durable_len });
    tree.visible.insert(path.to_path_buf(), id);
}

/// Honored parent-dir sync: make `durable`'s children of `parent` exactly mirror
/// `visible`'s children of `parent` (names only; byte durability stays per-inode
/// via `durable_len`). A removed/renamed-away name drops from `durable` here; a
/// freshly-created name lands in it here.
fn apply_durable_names(tree: &mut ShadowTree, parent: &Path) {
    let children: Vec<(PathBuf, u64)> = tree
        .visible
        .iter()
        .filter(|(p, _)| p.parent() == Some(parent))
        .map(|(p, &id)| (p.clone(), id))
        .collect();
    tree.durable.retain(|p, _| p.parent() != Some(parent));
    for (p, id) in children {
        tree.durable.insert(p, id);
    }
}

/// Open handle over the inode CURRENTLY visible at `path`. Path-keyed (not
/// inode-keyed): a write after the name is unlinked errors `NotFound` — MemFs
/// parity, a documented divergence from POSIX open-inode semantics (hazard H5).
struct ShadowStoreFile {
    path: PathBuf,
    state: Arc<Mutex<ShadowState>>,
}

impl ShadowStoreFile {
    /// Handle/staged sync shared by `sync_data`/`sync_all`: an honored sync
    /// advances the visible inode's durable byte prefix; a dropped one returns
    /// `Ok` without advancing (silently-lying disk); an errored one fails closed.
    fn sync_handle(&mut self) -> io::Result<()> {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match st.record_file_sync() {
            FileSyncOutcome::Dropped => Ok(()),
            FileSyncOutcome::Errored(error) => Err(error),
            FileSyncOutcome::Honored => {
                if let Some(&id) = st.tree.visible.get(&self.path) {
                    if let Some(inode) = st.tree.inodes.get_mut(&id) {
                        inode.durable_len = inode.bytes.len();
                    }
                }
                Ok(())
            }
        }
    }
}

impl StoreFile for ShadowStoreFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(error) = st.strike_mutation(SimOpKind::Write, &self.path, None) {
            return Err(error);
        }
        let Some(&id) = st.tree.visible.get(&self.path) else {
            return Err(not_found(&self.path));
        };
        let Some(inode) = st.tree.inodes.get_mut(&id) else {
            return Err(not_found(&self.path));
        };
        inode.bytes.extend_from_slice(buf);
        Ok(())
    }

    fn sync_data(&mut self) -> io::Result<()> {
        self.sync_handle()
    }

    fn sync_all(&mut self) -> io::Result<()> {
        self.sync_handle()
    }

    fn len(&self) -> io::Result<u64> {
        let st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match st.tree.visible.get(&self.path).and_then(|id| st.tree.inodes.get(id)) {
            Some(inode) => Ok(u64::try_from(inode.bytes.len()).unwrap_or(u64::MAX)),
            None => Err(not_found(&self.path)),
        }
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(inode) = st.tree.visible.get(&self.path).and_then(|id| st.tree.inodes.get(id))
        else {
            return Err(not_found(&self.path));
        };
        let bytes = &inode.bytes;
        let Ok(start) = usize::try_from(offset) else {
            return Ok(0);
        };
        if start >= bytes.len() {
            return Ok(0);
        }
        let n = buf.len().min(bytes.len() - start);
        buf[..n].copy_from_slice(&bytes[start..start + n]);
        Ok(n)
    }

    fn as_std_file(&self) -> Option<&std::fs::File> {
        // Purely virtual: no OS file, so mmap admission denies itself and the
        // byte-identical positioned-read fallback serves every read.
        None
    }
}

/// Staged bytes buffered privately; the atomic publish swaps them into the
/// visible namespace under one lock in [`StagedFile::persist`].
struct ShadowStagedFile {
    buf: Vec<u8>,
    /// Bytes an honored staged sync has made durable; carried onto the published
    /// inode's `durable_len` so a `crash()` truncates the publish to this prefix.
    durable_staged: usize,
    state: Arc<Mutex<ShadowState>>,
}

impl StagedFile for ShadowStagedFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        // Private buffer: not yet a namespace op, so unlogged and unfaultable.
        self.buf.extend_from_slice(buf);
        Ok(())
    }

    fn sync_all(&mut self) -> io::Result<()> {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match st.record_file_sync() {
            FileSyncOutcome::Dropped => Ok(()),
            FileSyncOutcome::Errored(error) => Err(error),
            FileSyncOutcome::Honored => {
                self.durable_staged = self.buf.len();
                Ok(())
            }
        }
    }

    fn persist(
        self: Box<Self>,
        final_path: &Path,
        _admission: crate::store::platform::sync::ParentDirSyncAdmission,
    ) -> io::Result<()> {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        // A faulted publish lands nothing — the store's belief the metadata is
        // durable is falsified, exactly the torn atomic publish the harness needs.
        if let Some(error) = st.strike_mutation(SimOpKind::PersistTemp, final_path, None) {
            return Err(error);
        }
        // MemFs publish edges: never split a path into both a file and a dir,
        // never publish under a missing parent.
        if st.tree.dirs.contains(final_path) {
            return Err(is_a_directory(final_path));
        }
        parent_must_exist(&st.tree, final_path)?;
        // Install the fresh inode into the visible namespace, then run the
        // IMPLICIT parent sync (RealFs protocol): Dropped still returns Ok (name
        // landed, not durable); Errored fails AFTER the visible install — the
        // rename happened but the dir sync did not, the real-world shape.
        insert_visible_file(&mut st.tree, final_path, self.buf, self.durable_staged);
        let parent = parent_of(final_path);
        match st.record_parent_sync(parent) {
            ParentSyncOutcome::Applied => {
                apply_durable_names(&mut st.tree, parent);
                Ok(())
            }
            ParentSyncOutcome::Dropped => Ok(()),
            ParentSyncOutcome::Errored(error) => Err(error),
        }
    }
}

/// Held in-process store-directory lock: the registry entry drops with the guard
/// (a `crash()` also clears the whole registry — process death).
struct ShadowDirLockGuard {
    path: PathBuf,
    state: Arc<Mutex<ShadowState>>,
}

impl StoreDirLockGuard for ShadowDirLockGuard {}

impl Drop for ShadowDirLockGuard {
    fn drop(&mut self) {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        st.tree.locks.remove(&self.path);
    }
}

impl StoreFs for ShadowFs {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
        let st = guard(self);
        if !st.tree.dirs.contains(path) {
            // A file at a directory path fails closed `NotADirectory` (ENOTDIR),
            // never mistaken for an absent/empty directory.
            if st.tree.visible.contains_key(path) {
                return Err(not_a_directory(path));
            }
            return Err(not_found(path));
        }
        let mut entries = Vec::new();
        for file_path in st.tree.visible.keys() {
            if file_path.parent() == Some(path) {
                if let Some(name) = file_path.file_name() {
                    entries.push(DirEntryInfo {
                        name: name.to_os_string(),
                        kind: FileKind::File,
                    });
                }
            }
        }
        for dir_path in &st.tree.dirs {
            if dir_path.parent() == Some(path) {
                if let Some(name) = dir_path.file_name() {
                    entries.push(DirEntryInfo {
                        name: name.to_os_string(),
                        kind: FileKind::Dir,
                    });
                }
            }
        }
        Ok(entries)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        // Directories are durable-on-create (the modeled axis is file-name
        // durability): one `dirs` set, no visible/durable split.
        let mut st = guard(self);
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if st.tree.visible.contains_key(&current) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("ShadowFs: not a directory: {}", current.display()),
                ));
            }
            st.tree.dirs.insert(current.clone());
        }
        Ok(())
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        let mut st = guard(self);
        if let Some(error) = st.strike_mutation(SimOpKind::CreateNew, path, None) {
            return Err(StoreError::Io(error));
        }
        parent_must_exist(&st.tree, path).map_err(StoreError::Io)?;
        if st.tree.visible.contains_key(path) || st.tree.dirs.contains(path) {
            return Err(StoreError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("ShadowFs: file exists: {}", path.display()),
            )));
        }
        insert_visible_file(&mut st.tree, path, Vec::new(), 0);
        Ok(Box::new(ShadowStoreFile {
            path: path.to_path_buf(),
            state: Arc::clone(&self.state),
        }))
    }

    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>> {
        let st = guard(self);
        if st.tree.dirs.contains(path) {
            return Err(is_a_directory(path));
        }
        if !st.tree.visible.contains_key(path) {
            return Err(not_found(path));
        }
        Ok(Box::new(ShadowStoreFile {
            path: path.to_path_buf(),
            state: Arc::clone(&self.state),
        }))
    }

    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError> {
        let mut st = guard(self);
        let parent = parent_of(path);
        match st.record_parent_sync(parent) {
            ParentSyncOutcome::Applied => {
                apply_durable_names(&mut st.tree, parent);
                Ok(())
            }
            ParentSyncOutcome::Dropped => Ok(()),
            ParentSyncOutcome::Errored(error) => Err(StoreError::Io(error)),
        }
    }

    fn reject_symlink_leaf(&self, _path: &Path, _purpose: &str) -> Result<(), StoreError> {
        // The virtual tree cannot contain symlinks; the guard holds vacuously.
        Ok(())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let st = guard(self);
        match st.tree.visible.get(path).and_then(|id| st.tree.inodes.get(id)) {
            Some(inode) => Ok(inode.bytes.clone()),
            None if st.tree.dirs.contains(path) => Err(is_a_directory(path)),
            None => Err(not_found(path)),
        }
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let st = guard(self);
        if st.tree.dirs.contains(path) || st.tree.visible.contains_key(path) {
            Ok(path.to_path_buf())
        } else {
            Err(not_found(path))
        }
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileStat> {
        // No symlinks exist, so the symlink-aware query equals the plain one.
        self.metadata(path)
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        _preference: crate::store::CopyPreference,
    ) -> io::Result<CowStrategyUsed> {
        // No links in a virtual tree: every copy is honestly a deep copy.
        self.copy(from, to)?;
        Ok(CowStrategyUsed::DeepCopy)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        // Same-path copy is non-destructive (corpus C13): report the length,
        // leave the bytes intact.
        if from == to {
            let st = guard(self);
            return match st.tree.visible.get(from).and_then(|id| st.tree.inodes.get(id)) {
                Some(inode) => Ok(u64::try_from(inode.bytes.len()).unwrap_or(u64::MAX)),
                None if st.tree.dirs.contains(from) => Err(is_a_directory(from)),
                None => Err(not_found(from)),
            };
        }
        let mut st = guard(self);
        if st.tree.dirs.contains(to) {
            return Err(is_a_directory(to));
        }
        let bytes = match st.tree.visible.get(from).and_then(|id| st.tree.inodes.get(id)) {
            Some(inode) => inode.bytes.clone(),
            None if st.tree.dirs.contains(from) => return Err(is_a_directory(from)),
            None => return Err(not_found(from)),
        };
        parent_must_exist(&st.tree, to)?;
        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        // A copy is a fresh WRITE: the destination inode starts at durable_len 0
        // (name pending), synced like any file by its caller.
        insert_visible_file(&mut st.tree, to, bytes, 0);
        Ok(len)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileStat> {
        let st = guard(self);
        match st.tree.visible.get(path).and_then(|id| st.tree.inodes.get(id)) {
            Some(inode) => Ok(FileStat {
                len: u64::try_from(inode.bytes.len()).unwrap_or(u64::MAX),
                kind: FileKind::File,
            }),
            None if st.tree.dirs.contains(path) => Ok(FileStat {
                len: 0,
                kind: FileKind::Dir,
            }),
            None => Err(not_found(path)),
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut st = guard(self);
        // Consult the fault schedule FIRST (a rename is an occurrence regardless
        // of validity); a strike applies nothing.
        if let Some(error) = st.strike_mutation(SimOpKind::Rename, from, Some(to)) {
            return Err(error);
        }
        // Same-path rename is a non-destructive POSIX no-op (corpus C14).
        if from == to {
            return if st.tree.visible.contains_key(from) || st.tree.dirs.contains(from) {
                Ok(())
            } else {
                Err(not_found(from))
            };
        }
        // Validate the destination edges BEFORE unbinding `from`, so a refused
        // rename leaves the source intact (MemFs parity).
        if st.tree.dirs.contains(to) {
            return Err(is_a_directory(to));
        }
        parent_must_exist(&st.tree, to)?;
        let Some(id) = st.tree.visible.remove(from) else {
            return Err(not_found(from));
        };
        // POSIX rename replaces an existing file destination; durability of the
        // move waits for the next honored parent sync.
        st.tree.visible.insert(to.to_path_buf(), id);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut st = guard(self);
        if let Some(error) = st.strike_mutation(SimOpKind::RemoveFile, path, None) {
            return Err(error);
        }
        // A directory at `path` fails closed (never `Ok(false)` via
        // `remove_file_if_present`), so a corrupt store is never masked.
        if st.tree.dirs.contains(path) {
            return Err(is_a_directory(path));
        }
        match st.tree.visible.remove(path) {
            // The inode is retained while `durable` still references it — a crash
            // before the removal's parent sync resurrects the durable byte prefix.
            Some(_) => Ok(()),
            None => Err(not_found(path)),
        }
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        // Clear the visible namespace subtree AND the directory entries (like
        // MemFs — the recursive default would leave the `dirs` entry alive and
        // re-appearing in `read_dir(parent)`). Durability of the removal waits
        // for a later parent sync (honest two-truths), matching `remove_file`.
        let mut st = guard(self);
        if !st.tree.dirs.contains(path) {
            return Err(not_found(path));
        }
        st.tree.visible.retain(|file_path, _| !file_path.starts_with(path));
        st.tree.dirs.retain(|dir_path| !dir_path.starts_with(path));
        Ok(())
    }

    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>> {
        let st = guard(self);
        if !st.tree.dirs.contains(dir) {
            return Err(not_found(dir));
        }
        Ok(Box::new(ShadowStagedFile {
            buf: Vec::new(),
            durable_staged: 0,
            state: Arc::clone(&self.state),
        }))
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        let mut st = guard(self);
        if st.tree.locks.contains(lock_path) {
            return Ok(None);
        }
        st.tree.locks.insert(lock_path.to_path_buf());
        Ok(Some(Box::new(ShadowDirLockGuard {
            path: lock_path.to_path_buf(),
            state: Arc::clone(&self.state),
        })))
    }
}

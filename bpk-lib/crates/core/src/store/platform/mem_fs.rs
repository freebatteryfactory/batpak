//! Pure in-memory [`StoreFs`] backend — the reference implementation for
//! non-POSIX embeddings (issues #164/#168).
//!
//! [`MemFs`] holds the entire store — segments, cold-start artifacts,
//! metadata, the keyset image — in an in-process tree. No OS file, no
//! tempfile, no mmap: [`StoreFile::as_std_file`] returns `None`, so every
//! platform-only optimization (sealed-segment mmap, the mmap-index map)
//! denies itself and the byte-identical positioned-read fallbacks serve all
//! reads. The store-directory lock is an in-process registry — for a purely
//! virtual backend the runtime IS the lock.
//!
//! Durability semantics: memory is this backend's durable medium — syncs are
//! honest no-ops and `persist` swaps the staged bytes in atomically under one
//! lock. An embedder whose host offers real durability (a database row, a
//! Durable Object transaction) implements its own backend with this file as
//! the shape reference; a test that wants LOSS semantics layers the
//! fault-injecting simulation filesystem on top (its sync-drop/crash model
//! interposes any inner backend).
//!
//! Path semantics: literal. There are no symlinks (the symlink guard is
//! vacuously satisfied), no hardlinks (copy-on-write requests honestly report
//! [`CowStrategyUsed::DeepCopy`]), and [`StoreFs::canonicalize`] is identity
//! for existing paths. Clones share the same tree (`Arc` state), so a config
//! holder and a test can observe one store's bytes.

use super::fs::{
    CowStrategyUsed, DirEntryInfo, FileKind, FileStat, StagedFile, StoreDirLockGuard, StoreFile,
    StoreFs,
};
use crate::store::StoreError;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Shared in-memory tree: file contents, known directories, held locks.
#[derive(Default)]
struct MemTree {
    files: BTreeMap<PathBuf, Vec<u8>>,
    dirs: BTreeSet<PathBuf>,
    locks: BTreeSet<PathBuf>,
}

/// Pure in-memory [`StoreFs`] backend. See the module docs for semantics.
///
/// # Example
///
/// A store that never touches the host filesystem:
///
/// ```
/// use std::sync::Arc;
/// use batpak::prelude::*;
/// use batpak::store::MemFs;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = StoreConfig::new("/virtual/store").with_fs(Arc::new(MemFs::new()));
/// let store = Store::open(config)?;
/// let receipt = store.append(
///     &Coordinate::new("entity:mem", "scope:demo")?,
///     EventKind::custom(0xF, 0x01),
///     &serde_json::json!({ "purely": "in-memory" }),
/// )?;
/// assert!(store.verify_append_receipt(&receipt).is_valid());
/// store.close()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct MemFs {
    tree: Arc<Mutex<MemTree>>,
}

impl MemFs {
    /// An empty in-memory filesystem.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock_tree(&self) -> std::sync::MutexGuard<'_, MemTree> {
        self.tree.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn not_found(path: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("MemFs: no such file or directory: {}", path.display()),
        )
    }

    fn is_a_directory(path: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::IsADirectory,
            format!("MemFs: is a directory: {}", path.display()),
        )
    }

    fn parent_must_exist(tree: &MemTree, path: &Path) -> io::Result<()> {
        match path.parent() {
            // A bare relative name has no parent to validate.
            None => Ok(()),
            Some(parent) if parent.as_os_str().is_empty() => Ok(()),
            Some(parent) if tree.dirs.contains(parent) => Ok(()),
            Some(parent) => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("MemFs: parent directory missing: {}", parent.display()),
            )),
        }
    }
}

/// Open handle over one in-memory file. Reads and appends go straight to the
/// shared tree (matching the visibility of OS page-cache writes).
struct MemStoreFile {
    path: PathBuf,
    tree: Arc<Mutex<MemTree>>,
}

impl MemStoreFile {
    fn lock_tree(&self) -> std::sync::MutexGuard<'_, MemTree> {
        self.tree.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl StoreFile for MemStoreFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut tree = self.lock_tree();
        match tree.files.get_mut(&self.path) {
            Some(bytes) => {
                bytes.extend_from_slice(buf);
                Ok(())
            }
            None => Err(MemFs::not_found(&self.path)),
        }
    }

    fn sync_data(&mut self) -> io::Result<()> {
        // Memory is the durable medium: the bytes already are where they live.
        Ok(())
    }

    fn sync_all(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn len(&self) -> io::Result<u64> {
        let tree = self.lock_tree();
        let bytes = tree
            .files
            .get(&self.path)
            .ok_or_else(|| MemFs::not_found(&self.path))?;
        Ok(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let tree = self.lock_tree();
        let bytes = tree
            .files
            .get(&self.path)
            .ok_or_else(|| MemFs::not_found(&self.path))?;
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
        // Purely virtual: no OS file exists, so mmap admission is denied and
        // the byte-identical fallbacks serve every read.
        None
    }
}

/// Staged bytes for the atomic publish: buffered privately, swapped into the
/// tree in one locked step on persist.
struct MemStagedFile {
    buf: Vec<u8>,
    tree: Arc<Mutex<MemTree>>,
}

impl StagedFile for MemStagedFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buf.extend_from_slice(buf);
        Ok(())
    }

    fn sync_all(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn persist(
        self: Box<Self>,
        final_path: &Path,
        _admission: crate::store::platform::sync::ParentDirSyncAdmission,
    ) -> io::Result<()> {
        let mut tree = self.tree.lock().unwrap_or_else(PoisonError::into_inner);
        // One locked insert IS the atomic publish: a reader sees the old
        // complete bytes or the new complete bytes, never a mixture, and
        // the name is as durable as the medium the moment it lands.
        tree.files.insert(final_path.to_path_buf(), self.buf);
        Ok(())
    }
}

/// Held in-process lock: the registry entry drops with the guard.
struct MemDirLockGuard {
    path: PathBuf,
    tree: Arc<Mutex<MemTree>>,
}

impl StoreDirLockGuard for MemDirLockGuard {}

impl Drop for MemDirLockGuard {
    fn drop(&mut self) {
        let mut tree = self.tree.lock().unwrap_or_else(PoisonError::into_inner);
        tree.locks.remove(&self.path);
    }
}

impl StoreFs for MemFs {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
        let tree = self.lock_tree();
        if !tree.dirs.contains(path) {
            return Err(MemFs::not_found(path));
        }
        let mut entries = Vec::new();
        for file_path in tree.files.keys() {
            if file_path.parent() == Some(path) {
                if let Some(name) = file_path.file_name() {
                    entries.push(DirEntryInfo {
                        name: name.to_os_string(),
                        kind: FileKind::File,
                    });
                }
            }
        }
        for dir_path in &tree.dirs {
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
        let mut tree = self.lock_tree();
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if tree.files.contains_key(&current) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("MemFs: not a directory: {}", current.display()),
                ));
            }
            tree.dirs.insert(current.clone());
        }
        Ok(())
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        let mut tree = self.lock_tree();
        MemFs::parent_must_exist(&tree, path).map_err(StoreError::Io)?;
        if tree.files.contains_key(path) || tree.dirs.contains(path) {
            return Err(StoreError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("MemFs: file exists: {}", path.display()),
            )));
        }
        tree.files.insert(path.to_path_buf(), Vec::new());
        Ok(Box::new(MemStoreFile {
            path: path.to_path_buf(),
            tree: Arc::clone(&self.tree),
        }))
    }

    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>> {
        let tree = self.lock_tree();
        if tree.dirs.contains(path) {
            return Err(MemFs::is_a_directory(path));
        }
        if !tree.files.contains_key(path) {
            return Err(MemFs::not_found(path));
        }
        Ok(Box::new(MemStoreFile {
            path: path.to_path_buf(),
            tree: Arc::clone(&self.tree),
        }))
    }

    fn sync_parent_dir(&self, _path: &Path) -> Result<(), StoreError> {
        // Names are as durable as the medium the moment they land.
        Ok(())
    }

    fn reject_symlink_leaf(&self, _path: &Path, _purpose: &str) -> Result<(), StoreError> {
        // The virtual tree cannot contain symlinks; the guard holds vacuously.
        Ok(())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let tree = self.lock_tree();
        if let Some(bytes) = tree.files.get(path) {
            return Ok(bytes.clone());
        }
        // A directory at a file path fails closed as `IsADirectory` (like
        // RealFs), NOT `NotFound`: loaders treat NotFound as "artifact absent"
        // and would silently ignore a corrupt virtual store.
        if tree.dirs.contains(path) {
            return Err(MemFs::is_a_directory(path));
        }
        Err(MemFs::not_found(path))
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let tree = self.lock_tree();
        if tree.dirs.contains(path) || tree.files.contains_key(path) {
            Ok(path.to_path_buf())
        } else {
            Err(MemFs::not_found(path))
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
        // No links in a virtual tree: every copy is honestly a deep copy
        // (the preference names an optimization, not an obligation).
        self.copy(from, to)?;
        Ok(CowStrategyUsed::DeepCopy)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        let mut tree = self.lock_tree();
        let bytes = tree
            .files
            .get(from)
            .cloned()
            .ok_or_else(|| MemFs::not_found(from))?;
        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        tree.files.insert(to.to_path_buf(), bytes);
        Ok(len)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileStat> {
        let tree = self.lock_tree();
        if let Some(bytes) = tree.files.get(path) {
            return Ok(FileStat {
                len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                kind: FileKind::File,
            });
        }
        if tree.dirs.contains(path) {
            return Ok(FileStat {
                len: 0,
                kind: FileKind::Dir,
            });
        }
        Err(MemFs::not_found(path))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut tree = self.lock_tree();
        let bytes = tree
            .files
            .remove(from)
            .ok_or_else(|| MemFs::not_found(from))?;
        // POSIX rename semantics: an existing destination is replaced.
        tree.files.insert(to.to_path_buf(), bytes);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut tree = self.lock_tree();
        match tree.files.remove(path) {
            Some(_) => Ok(()),
            None => Err(MemFs::not_found(path)),
        }
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut tree = self.lock_tree();
        if !tree.dirs.contains(path) {
            return Err(MemFs::not_found(path));
        }
        // MemFs tracks directories explicitly, so — unlike the recursive
        // default, which only clears contents — remove the directory ENTRY and
        // every nested file/dir too; a lingering `tree.dirs` entry would keep
        // re-appearing in `read_dir(parent)` after an `Ok(true)` removal.
        tree.files
            .retain(|file_path, _| !file_path.starts_with(path));
        tree.dirs.retain(|dir_path| !dir_path.starts_with(path));
        Ok(())
    }

    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>> {
        let tree = self.lock_tree();
        if !tree.dirs.contains(dir) {
            return Err(MemFs::not_found(dir));
        }
        Ok(Box::new(MemStagedFile {
            buf: Vec::new(),
            tree: Arc::clone(&self.tree),
        }))
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        let mut tree = self.lock_tree();
        if tree.locks.contains(lock_path) {
            return Ok(None);
        }
        tree.locks.insert(lock_path.to_path_buf());
        Ok(Some(Box::new(MemDirLockGuard {
            path: lock_path.to_path_buf(),
            tree: Arc::clone(&self.tree),
        })))
    }
}

use crate::store::StoreError;
use std::fs::{File, Metadata, ReadDir};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Which copy strategy a [`StoreFs::cow_copy_file`] call actually used.
///
/// Fork prefers copy-on-write and reports what the filesystem delivered, so
/// fork evidence can record the real cost model instead of the preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CowStrategyUsed {
    /// A filesystem-level reflink (copy-on-write clone) was performed.
    Reflink,
    /// A hardlink to the immutable source file was created.
    Hardlink,
    /// The bytes were physically copied.
    DeepCopy,
}

pub(crate) fn reject_symlink_leaf(path: &Path, purpose: &str) -> Result<(), StoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(StoreError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to write {purpose} through symlink {}",
                path.display()
            ),
        ))),
        Ok(_) | Err(_) => Ok(()),
    }
}

pub(crate) fn reject_cache_symlink_leaf(path: &Path) -> Result<(), StoreError> {
    match reject_symlink_leaf(path, "cache path") {
        Ok(()) => Ok(()),
        Err(StoreError::Io(error)) => Err(StoreError::CacheFailed(Box::new(error))),
        Err(error) => Err(error),
    }
}

/// Atomic write via the production default backend ([`RealFs`]).
///
/// A thin wrapper over [`write_file_atomically_with_fs`] that preserves the
/// standalone (fs-less) test forge (`checkpoint::tests`); every production
/// cold-start artifact write path (checkpoint / mmap-index / idempotency-store /
/// compaction marker) now drives the `_with_fs` variant with the store's
/// configured filesystem so a `SimFs` can tear the atomic publish. Test-only
/// because no production call site remains.
#[cfg(test)]
pub(crate) fn write_file_atomically(
    data_dir: &Path,
    final_path: &Path,
    purpose: &str,
    write: impl FnOnce(&mut dyn io::Write) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    write_file_atomically_with_fs(data_dir, final_path, purpose, write, &RealFs)
}

/// [`write_file_atomically`], routed through the supplied [`StoreFs`] backend.
///
/// A composite of already-routed primitives — the symlink-leaf guard, the
/// staged create ([`StoreFs::named_temp_in`]), the staging sync
/// ([`StagedFile::sync_all`]), and the atomic publish
/// ([`StagedFile::persist`]) — so a fault-injecting backend can drop the
/// staging sync or tear the publish, and a crash harness can observe a
/// cold-start artifact the store believed durable. `RealFs` makes it
/// byte-for-byte the production behavior.
pub(crate) fn write_file_atomically_with_fs(
    data_dir: &Path,
    final_path: &Path,
    purpose: &str,
    write: impl FnOnce(&mut dyn io::Write) -> Result<(), StoreError>,
    fs: &dyn StoreFs,
) -> Result<(), StoreError> {
    fs.reject_symlink_leaf(final_path, purpose)?;
    let mut tmp = fs.named_temp_in(data_dir).map_err(StoreError::Io)?;
    let mut writer = StagedFileWriter(tmp.as_mut());
    write(&mut writer)?;
    tmp.sync_all().map_err(StoreError::Io)?;
    let admission = crate::store::platform::sync::admit_current_parent_dir_sync()?;
    tmp.persist(final_path, admission).map_err(StoreError::Io)?;
    Ok(())
}

/// [`io::Write`] adapter over a [`StagedFile`], so atomic-write callers can
/// stream through writer APIs (`BufWriter`, serializers) while staging.
pub(crate) struct StagedFileWriter<'a>(pub(crate) &'a mut dyn StagedFile);

impl io::Write for StagedFileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn write_derivative_file_atomically(
    data_dir: &Path,
    final_path: &Path,
    purpose: &str,
    bytes: &[u8],
) -> io::Result<()> {
    match reject_symlink_leaf(final_path, purpose) {
        Ok(()) => {}
        Err(StoreError::Io(error)) => return Err(error),
        Err(error) => return Err(io::Error::other(error.to_string())),
    }
    let tmp = named_temp_in(data_dir)?;
    {
        let mut file = io::BufWriter::new(tmp.as_file());
        file.write_all(bytes)?;
        file.into_inner().map_err(|error| error.into_error())?;
    }
    tmp.persist(final_path).map_err(|error| error.error)?;
    Ok(())
}

pub(crate) fn create_new_file(path: &Path) -> Result<File, StoreError> {
    File::create_new(path).map_err(StoreError::Io)
}

pub(crate) fn open_file(path: &Path) -> io::Result<File> {
    File::open(path)
}

pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub(crate) fn read_dir(path: &Path) -> io::Result<ReadDir> {
    std::fs::read_dir(path)
}

pub(crate) fn create_dir_all(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

pub(crate) fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(path)
}

pub(crate) fn metadata(path: &Path) -> io::Result<Metadata> {
    std::fs::metadata(path)
}

pub(crate) fn symlink_metadata(path: &Path) -> io::Result<Metadata> {
    std::fs::symlink_metadata(path)
}

pub(crate) fn remove_file(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

pub(crate) fn remove_file_if_present(path: &Path) -> io::Result<bool> {
    match remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) fn remove_dir_all(path: &Path) -> io::Result<()> {
    std::fs::remove_dir_all(path)
}

pub(crate) fn named_temp_in(dir: &Path) -> io::Result<NamedTempFile> {
    NamedTempFile::new_in(dir)
}

pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)
}

pub(crate) fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    std::fs::copy(from, to)
}

/// Truncate (or extend) the file at `path` to exactly `len` bytes.
///
/// Used by the deterministic-simulation filesystem ([`super::super::sim::fs::SimFs`])
/// to model a crash: each tracked file is truncated to its last durable length,
/// discarding the write-but-unsynced tail. Lives here, under the platform
/// boundary, so the raw file-open + `set_len` target contact stays out of the
/// store-runtime code the structural gate guards.
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) fn truncate_file_to(path: &Path, len: u64) -> io::Result<()> {
    let file = std::fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(len)
}

pub(crate) fn hard_link(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::hard_link(from, to)
}

pub(crate) fn reflink(from: &Path, to: &Path) -> io::Result<()> {
    reject_copy_source(from)?;
    remove_file_if_present(to)?;
    reflink_impl(from, to).inspect_err(|_| {
        drop(remove_file_if_present(to));
    })
}

pub(crate) fn cow_copy_file(
    from: &Path,
    to: &Path,
    preference: crate::store::CopyPreference,
) -> io::Result<CowStrategyUsed> {
    use crate::store::CopyPreference;
    let use_reflink = matches!(preference, CopyPreference::ReflinkThenHardlink);
    let use_hardlink = matches!(
        preference,
        CopyPreference::ReflinkThenHardlink | CopyPreference::HardlinkOnly
    );

    reject_copy_source(from)?;
    remove_file_if_present(to)?;

    if use_reflink {
        match reflink(from, to) {
            Ok(()) => return Ok(CowStrategyUsed::Reflink),
            Err(error) => {
                tracing::debug!(
                    source = %from.display(),
                    destination = %to.display(),
                    error = %error,
                    "reflink failed; falling back to next fork copy rung"
                );
                remove_file_if_present(to)?;
            }
        }
    }

    if use_hardlink {
        match hard_link(from, to) {
            Ok(()) => return Ok(CowStrategyUsed::Hardlink),
            Err(error) => {
                tracing::debug!(
                    source = %from.display(),
                    destination = %to.display(),
                    error = %error,
                    "hardlink failed; falling back to deep copy"
                );
                remove_file_if_present(to)?;
            }
        }
    }

    copy(from, to)?;
    Ok(CowStrategyUsed::DeepCopy)
}

fn reject_copy_source(path: &Path) -> io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to copy symlink source {}", path.display()),
        ));
    }
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to copy non-file source {}", path.display()),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn reflink_impl(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::fd::AsRawFd;

    const FICLONE: libc::c_ulong = 0x4004_9409;
    let source = File::open(from)?;
    let destination = File::create_new(to)?;
    // SAFETY: `source` and `destination` are live file descriptors opened by
    // this function. `FICLONE` does not retain pointers into Rust memory; it
    // asks the kernel to clone file data from `source` into `destination`.
    let result = unsafe { libc::ioctl(destination.as_raw_fd(), FICLONE, source.as_raw_fd()) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn reflink_impl(from: &Path, to: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "source path contains interior NUL",
        )
    })?;
    let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination path contains interior NUL",
        )
    })?;
    // SAFETY: the C strings are NUL-terminated and live for the duration of
    // the call. `clonefile` does not retain the pointers after returning.
    let result = unsafe { libc::clonefile(from.as_ptr(), to.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn reflink_impl(_from: &Path, _to: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "reflink is not supported on this platform",
    ))
}

pub(crate) use super::handle::{
    file_kind_of, file_stat_of, RealDirLockGuard, RealStagedFile, RealStoreFile,
    StoreFileAppendWriter, StoreFileCursor, StoreFileWriter,
};
pub use super::handle::{
    DirEntryInfo, FileKind, FileStat, PositionedReadError, StagedFile, StoreDirLockGuard, StoreFile,
};

// Platform free-function routing status (release row `STORE-PLATFORM-FS-ROUTING`,
// terminal FAIL-CLOSED boundary). The tail is now EMPTY: every store call site
// that reaches the target filesystem does so through [`StoreFs`]. The
// crash-sensitive atomic-rename / persist cluster (`rename`, `remove_file`,
// `named_temp_in`, `persist_temp_with_parent_sync`) was routed by W3; the
// positioned-read primitive (`read_exact_at`) is now routed too, so a `SimFs` can
// fault the active-segment frame read that, as a free fn, was unfaultable. The
// free fn itself STAYS here — it holds the `read_at` / `#[cfg(unix)]` /
// `FileExt` contact the `platform_boundary` gate confines to `platform/` —
// and `RealFs::read_exact_at` delegates straight to it. Asserted empty by
// `store_fs_fail_closed_boundary_lists_unrouted_tail_ops`.

/// Filesystem seam for production, deterministic simulation, and embeddings.
///
/// Boundary: this is the narrow trait through which store code reaches the
/// target filesystem — no store call site touches `platform::fs::*` free
/// functions directly (release row `STORE-PLATFORM-FS-ROUTING`). Production
/// routes through [`RealFs`], whose every method is a byte-for-byte delegate
/// to those free functions; the fault-injecting `SimFs` test backend
/// interposes the same seam for deterministic crash/fault proofs. Promoted
/// from `pub(crate)` for 0.10.0 (following the [`Spawn`] seam's precedent) so
/// embeddings can supply their own backend via
/// [`StoreConfig::with_fs`](crate::store::StoreConfig::with_fs).
///
/// `Send + Sync` so it can live behind `Arc<dyn StoreFs>` on `StoreConfig` and
/// be shared across threads, mirroring the [`Spawn`] seam.
///
/// # Durability contract (what implementations must uphold)
///
/// The store's crash-integrity proofs assume exactly these guarantees; an
/// implementation that weakens any of them silently voids the store's
/// durability claims:
///
/// - **Sync means durable.** A minted handle's [`StoreFile::sync_data`] /
///   [`StoreFile::sync_all`] must not return `Ok` before the handle's
///   contents survive a crash (the per-event / per-rotation durability
///   boundary). [`StoreFs::sync_parent_dir`] must not return `Ok` before the
///   freshly-created file's *name* survives a crash.
/// - **The stage/persist pair is a single atomic publish.**
///   [`StoreFs::named_temp_in`] stages a [`StagedFile`] the caller writes
///   and syncs; [`StagedFile::persist`] must install it at the final path
///   such that a crash at any point leaves either the OLD complete file (or
///   absence) or the NEW complete file — never a torn mixture. This is what
///   the store's atomic-write helper and the keyset flush-before-ack fence
///   rely on.
/// - **Create-new is exclusive.** [`StoreFs::create_new_file`] must fail if
///   the path already exists (`std::fs::File::create_new` semantics), so a
///   segment can never silently truncate a predecessor.
/// - **The lock excludes cooperating owners.**
///   [`StoreFs::try_lock_store_dir`] must return `Ok(None)` (not a fresh
///   lock) while another live guard holds the same store directory, to the
///   same degree the platform's advisory lock does.
/// - **Symlink rejection fails closed.** [`StoreFs::reject_symlink_leaf`]
///   must refuse a symlink leaf; treating one as writable re-opens the
///   redirected-write hazard the guard exists to close.
/// - **`rename`/`remove_file` are the compaction swap and reclaim points.**
///   They must be atomic with respect to crashes to the same degree the
///   platform rename is.
///
/// # Scope: handle-abstracted (0.10.0)
///
/// The seam traffics in abstract handles — [`StoreFile`], [`StagedFile`],
/// [`StoreDirLockGuard`] — not concrete `std::fs::File` /
/// `tempfile::NamedTempFile`, so ANY backend can implement it: alternate
/// native volumes, fault injection (a `SimFs`-style fault layer), a pure
/// in-memory tree ([`MemFs`]), or a non-POSIX host (wasm, Durable-Object
/// storage). Platform-only optimizations (sealed-segment mmap, the mmap
/// index) admit themselves through [`StoreFile::as_std_file`] feeding the
/// existing evidence/admission machinery; a virtual handle returns `None`
/// and every read takes the byte-identical positioned-read fallback instead.
///
/// # Conformance
///
/// The executable form of this contract is the `batpak::store::conformance`
/// corpus (feature `conformance-harness`): a backend author runs
/// `run_all(&your_factory)` and publishes the machine-readable report. The
/// crash-family cases require a [`HostileControls`] backend; a backend without
/// them yields typed qualifications, never silent passes. [`RealFs`], [`MemFs`],
/// and the namespace-truth `ShadowFs` are gated by the same corpus body in-tree,
/// so this doc block and the corpus cannot drift apart.
///
/// [`MemFs`]: crate::store::MemFs
/// [`Spawn`]: crate::store::Spawn
/// [`HostileControls`]: crate::store::conformance::HostileControls
pub trait StoreFs: Send + Sync {
    /// Collect a directory's entries. Owned counterpart of
    /// [`std::fs::read_dir`] (store directories are small).
    ///
    /// # Errors
    /// The underlying directory-read failure.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>>;

    /// Create a directory and all missing parents. Mirrors
    /// [`std::fs::create_dir_all`].
    ///
    /// # Errors
    /// The underlying directory-creation failure.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Create-new the segment/data file at `path`, returning the open handle.
    ///
    /// `path` is the LOGICAL file path the handle backs (e.g. a segment
    /// `.fbat`); a fault-injecting backend bakes it into the handle so its
    /// durable-length model can key off it (a simulated crash truncates the
    /// file to its last synced length). Must be create-new exclusive
    /// (`std::fs::File::create_new` semantics).
    ///
    /// # Errors
    /// [`StoreError::Io`] on the creation failure — including when the path
    /// already exists, which the exclusivity contract requires.
    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError>;

    /// Open the existing file at `path` for reading, returning the handle.
    ///
    /// The read half of [`StoreFs::create_new_file`]: the segment reader's FD
    /// cache, the recovery/compaction scans, and the mmap setup all obtain
    /// their handles here, so a backend serves reads from the same model it
    /// persisted into.
    ///
    /// # Errors
    /// The underlying open failure (`NotFound` when absent).
    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>>;

    /// Fsync the directory entry for `path`'s parent so a freshly-created file's
    /// name is durable.
    ///
    /// # Errors
    /// The underlying directory-sync failure.
    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError>;

    /// Reject symlink leaf paths before writing; a symlink leaf must fail
    /// closed rather than redirect a store write outside the data directory.
    ///
    /// # Errors
    /// [`StoreError::Io`] with `InvalidInput` when `path` is a symlink leaf.
    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError>;

    /// Read the entire file at `path`. Mirrors [`std::fs::read`].
    ///
    /// Routed so a backend that virtualizes or fault-injects storage serves
    /// reads from the same model it persisted into — the whole-file read pair
    /// of the [`StoreFs::named_temp_in`] /
    /// [`StagedFile::persist`] atomic publish (the keyset
    /// load is this method's first consumer).
    ///
    /// # Errors
    /// The underlying read failure.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Canonicalize a path. Mirrors [`std::fs::canonicalize`].
    ///
    /// # Errors
    /// The underlying canonicalization failure.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Symlink-aware metadata. Owned counterpart of
    /// [`std::fs::symlink_metadata`].
    ///
    /// # Errors
    /// The underlying metadata failure.
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileStat>;

    /// Copy-on-write file copy for fork, reporting the strategy actually used.
    ///
    /// # Errors
    /// The failure of the last strategy attempted under `preference`.
    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: crate::store::CopyPreference,
    ) -> io::Result<CowStrategyUsed>;

    /// Deep file copy for snapshot. Mirrors [`std::fs::copy`].
    ///
    /// Same-path (`from == to`) must be non-destructive — like
    /// [`StoreFs::cow_copy_file`], it returns the length (or a typed refusal)
    /// and leaves the file's bytes intact; corrupting or truncating the source
    /// is a contract violation (corpus C13).
    ///
    /// # Errors
    /// The underlying copy failure.
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;

    /// File metadata (follows symlinks). Owned counterpart of
    /// [`std::fs::metadata`].
    ///
    /// # Errors
    /// The underlying metadata failure.
    fn metadata(&self, path: &Path) -> io::Result<FileStat>;

    /// Rename `from` to `to`. Mirrors [`std::fs::rename`].
    ///
    /// Crash-sensitive: the single atomic swap/rollback point of compaction
    /// (relocate the merged source, then restore it on rollback). Routed so a
    /// fault-injecting backend can tear the swap and a crash harness can observe
    /// the half-applied rename.
    ///
    /// Same-path (`from == to`) must be non-destructive — a POSIX no-op `Ok`
    /// leaving the file intact, or a typed refusal; never deletion (corpus
    /// C14).
    ///
    /// # Errors
    /// The underlying rename failure.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Remove the file at `path`. Mirrors [`std::fs::remove_file`].
    ///
    /// Crash-sensitive: the post-swap reclaim of superseded segments after a
    /// successful compaction. Routed so the reclaim becomes fault-injectable.
    ///
    /// # Errors
    /// The underlying removal failure.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Remove the file at `path`, reporting whether it existed (`Ok(false)` when
    /// already absent).
    ///
    /// Provided in terms of [`StoreFs::remove_file`] so a fault-injecting backend
    /// only interposes the one primitive; the not-found tolerance is identical on
    /// every backend.
    ///
    /// # Errors
    /// Any [`StoreFs::remove_file`] failure other than not-found.
    fn remove_file_if_present(&self, path: &Path) -> io::Result<bool> {
        match self.remove_file(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Recursively remove the directory at `path` and its contents.
    ///
    /// The provided default removes the directory's CONTENTS through the seam
    /// ([`StoreFs::read_dir`] + [`StoreFs::remove_file`], recursing into
    /// subdirectories) — sufficient for a virtual backend where a directory is
    /// only a key prefix, so a snapshot/fork cleanup routed through `fs` clears
    /// stale artifacts on any backend. A backend with real directory entries
    /// (e.g. [`RealFs`]) overrides this to also remove the now-empty directory.
    ///
    /// # Errors
    /// The underlying read/remove failure.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        for entry in self.read_dir(path)? {
            let child = path.join(&entry.name);
            if entry.kind == FileKind::Dir {
                self.remove_dir_all(&child)?;
            } else {
                self.remove_file_if_present(&child)?;
            }
        }
        Ok(())
    }

    /// Remove the directory at `path` and its contents, reporting whether it
    /// existed (`Ok(false)` when already absent). Provided in terms of
    /// [`StoreFs::remove_dir_all`].
    ///
    /// # Errors
    /// Any [`StoreFs::remove_dir_all`] failure other than the directory being
    /// absent.
    fn remove_dir_all_if_present(&self, path: &Path) -> io::Result<bool> {
        match self.remove_dir_all(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Stage a uniquely-named temp file in `dir`.
    ///
    /// The staging half of an atomic publish: the caller writes + syncs the
    /// [`StagedFile`], then [`StagedFile::persist`]s it at the final path.
    /// Crash-sensitive: the atomic publish point for the cold-start
    /// artifacts, visibility-range and cursor-checkpoint metadata, and the
    /// crypto-shred keyset. Routed so a fault-injecting backend can drop the
    /// staging sync or tear the publish, and a crash harness can observe a
    /// publish the store believed durable.
    ///
    /// # Errors
    /// The underlying temp-file creation failure.
    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>>;

    /// Try to take the exclusive store-directory lock at `lock_path`.
    ///
    /// Returns `Ok(Some(guard))` when acquired (dropping the guard releases
    /// it) and `Ok(None)` when another live owner holds it. For [`RealFs`]
    /// this is the OS advisory lock (`flock`-class, `File::try_lock`); a
    /// virtual backend keys an in-process registry — its runtime IS the
    /// lock.
    ///
    /// # Errors
    /// [`StoreError`] on open/admission failures other than the lock being
    /// held.
    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError>;
}

/// Production [`StoreFs`]: every method delegates to the existing
/// `platform::fs::*` free functions, so the default build behaves byte-for-byte
/// as it did before the seam was introduced.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFs;

impl StoreFs for RealFs {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
        let mut entries = Vec::new();
        for entry in read_dir(path)? {
            let entry = entry?;
            entries.push(DirEntryInfo {
                name: entry.file_name(),
                kind: file_kind_of(entry.file_type()?),
            });
        }
        Ok(entries)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        create_dir_all(path)
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        Ok(Box::new(RealStoreFile::new(create_new_file(path)?)))
    }

    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>> {
        Ok(Box::new(RealStoreFile::new(open_file(path)?)))
    }

    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError> {
        crate::store::platform::sync::sync_parent_dir(path)
    }

    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError> {
        reject_symlink_leaf(path, purpose)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        read(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        canonicalize(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileStat> {
        Ok(file_stat_of(&symlink_metadata(path)?))
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: crate::store::CopyPreference,
    ) -> io::Result<CowStrategyUsed> {
        cow_copy_file(from, to, preference)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        copy(from, to)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileStat> {
        Ok(file_stat_of(&metadata(path)?))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        remove_file(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        remove_dir_all(path)
    }

    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>> {
        Ok(Box::new(RealStagedFile::new(named_temp_in(dir)?)))
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        let file = crate::store::platform::lock::open_store_lock_file(lock_path)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Box::new(RealDirLockGuard::new(file)))),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(error)) => Err(StoreError::Io(error)),
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "fs_reflink_mutation_tests.rs"]
mod fs_reflink_mutation_tests;

#[cfg(test)]
#[path = "fs_tests.rs"]
mod tests;

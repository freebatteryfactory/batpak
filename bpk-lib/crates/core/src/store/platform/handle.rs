//! Abstract store-file handles for the [`StoreFs`](super::fs::StoreFs) seam.
//!
//! Extracted from `fs.rs` (0.10.0 handle abstraction, ADR-0034) so each
//! platform source file stays under the structural size cap: the
//! `StoreFile` / `StagedFile` / `StoreDirLockGuard` handle traits, the owned
//! metadata types they return, the `io` adapters over a handle, and the
//! `RealFs` handle implementations over `std::fs`.

use std::fs::{File, Metadata};
use std::io;
use std::path::Path;
use tempfile::NamedTempFile;

/// Failure of a positioned read ([`StoreFile::read_exact_at`]).
#[derive(Debug)]
pub enum PositionedReadError {
    /// The underlying positioned read returned an I/O error.
    Io(std::io::Error),
    /// End-of-file was reached before the buffer was filled.
    ShortRead {
        /// Bytes successfully read before the premature end-of-file.
        bytes_read: usize,
    },
}

impl std::fmt::Display for PositionedReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "positioned read failed: {error}"),
            Self::ShortRead { bytes_read } => {
                write!(f, "short read: only {bytes_read} byte(s) read before EOF")
            }
        }
    }
}

impl std::error::Error for PositionedReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::ShortRead { .. } => None,
        }
    }
}

/// What a path points at, as reported by [`StoreFs::metadata`](super::fs::StoreFs::metadata) /
/// [`StoreFs::symlink_metadata`](super::fs::StoreFs::symlink_metadata) and [`DirEntryInfo::kind`].
///
/// Owned (backend-independent) replacement for `std::fs::FileType`, so a
/// virtual backend can answer metadata queries without minting OS types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link (only reported by symlink-aware queries).
    Symlink,
    /// Anything else the host reports (device node, socket, ...).
    Other,
}

/// Owned (backend-independent) metadata record. Replacement for the
/// `std::fs::Metadata` the seam returned before the handle abstraction.
#[derive(Clone, Copy, Debug)]
pub struct FileStat {
    /// File length in bytes (0 for non-files where the host has no length).
    pub len: u64,
    /// What the path points at.
    pub kind: FileKind,
}

/// One directory entry, as reported by [`StoreFs::read_dir`](super::fs::StoreFs::read_dir).
///
/// Owned replacement for `std::fs::DirEntry`: store directories are small
/// (segments + a handful of artifacts), so an eagerly-collected `Vec` costs
/// nothing and frees backends from exposing a lazy OS iterator type.
#[derive(Clone, Debug)]
pub struct DirEntryInfo {
    /// The entry's file name (no path prefix).
    pub name: std::ffi::OsString,
    /// What the entry points at (symlink-aware, like `std::fs::DirEntry`).
    pub kind: FileKind,
}

pub(crate) fn file_kind_of(file_type: std::fs::FileType) -> FileKind {
    if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_dir() {
        FileKind::Dir
    } else if file_type.is_file() {
        FileKind::File
    } else {
        FileKind::Other
    }
}

pub(crate) fn file_stat_of(meta: &Metadata) -> FileStat {
    FileStat {
        len: meta.len(),
        kind: file_kind_of(meta.file_type()),
    }
}

/// An open store file handle minted by a [`StoreFs`](super::fs::StoreFs) backend.
///
/// The abstract replacement for the concrete `std::fs::File` the seam
/// trafficked in before 0.10.0: a backend returns a boxed handle from
/// [`StoreFs::create_new_file`](super::fs::StoreFs::create_new_file) /
/// [`StoreFs::open_file`](super::fs::StoreFs::open_file), and the store
/// writes, syncs, and position-reads through it. A handle knows its own
/// backing path (backends bake it in at mint time), so fault-injecting
/// backends key their durability model per handle without path parameters
/// on every call.
///
/// # Durability contract
///
/// [`StoreFile::sync_data`] / [`StoreFile::sync_all`] must not return `Ok`
/// before the handle's contents survive a crash — they are the per-event /
/// per-rotation durability boundary the store's crash proofs assume.
pub trait StoreFile: Send + Sync {
    /// Append `buf` at the current write position. Mirrors
    /// [`std::io::Write::write_all`].
    ///
    /// # Errors
    /// The underlying write failure.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;

    /// Make the handle's contents durable (`fdatasync` semantics — file
    /// length and data, not necessarily all metadata).
    ///
    /// # Errors
    /// The sync failure; the store treats it as a durability loss and fails
    /// closed.
    fn sync_data(&mut self) -> io::Result<()>;

    /// Make the handle's contents and metadata durable (`fsync` semantics).
    ///
    /// # Errors
    /// The sync failure; the store treats it as a durability loss and fails
    /// closed.
    fn sync_all(&mut self) -> io::Result<()>;

    /// Current length of the backing file in bytes.
    ///
    /// # Errors
    /// The underlying metadata failure.
    fn len(&self) -> io::Result<u64>;

    /// Read up to `buf.len()` bytes at absolute byte `offset`, returning how
    /// many were read (`0` = end of file). Positioned: must not disturb any
    /// sequential cursor another clone of the same OS file shares.
    ///
    /// # Errors
    /// The underlying read failure.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

    /// Read exactly `buf.len()` bytes at absolute byte `offset`. Provided in
    /// terms of [`StoreFile::read_at`] so backends interpose one primitive.
    ///
    /// # Errors
    /// [`PositionedReadError::Io`] on a read failure, or
    /// [`PositionedReadError::ShortRead`] when end-of-file arrives before the
    /// buffer is filled.
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), PositionedReadError> {
        let mut total_read = 0;
        while total_read < buf.len() {
            let n = self
                .read_at(
                    offset.saturating_add(u64::try_from(total_read).unwrap_or(u64::MAX)),
                    &mut buf[total_read..],
                )
                .map_err(PositionedReadError::Io)?;
            if n == 0 {
                return Err(PositionedReadError::ShortRead {
                    bytes_read: total_read,
                });
            }
            total_read = total_read.saturating_add(n);
        }
        Ok(())
    }

    /// Native escape hatch: the underlying `std::fs::File`, when this handle
    /// is backed by a real OS file.
    ///
    /// This is how platform-only optimizations admit themselves without a
    /// second capability mechanism: the sealed-segment mmap and the mmap
    /// index feed this handle into the EXISTING evidence/admission machinery
    /// (`platform::mmap`), and a `None` (virtual backend) simply leaves the
    /// admission denied — the byte-identical positioned-read fallback serves
    /// every read instead.
    fn as_std_file(&self) -> Option<&File>;
}

/// A staged temp file: the staging half of the seam's atomic publish.
///
/// Replaces the concrete [`tempfile::NamedTempFile`] in the seam's
/// signatures. The caller writes and syncs the staged bytes, then
/// [`StagedFile::persist`] installs them at the final path atomically: a
/// crash at any point leaves either the OLD complete file (or its absence)
/// or the NEW complete file — never a torn mixture. This is the protocol the
/// store's atomic-write helper and the keyset flush-before-ack fence rely
/// on. Staging durability is ON the seam (a fault-injecting backend can drop
/// the staging sync as well as tear the publish).
pub trait StagedFile: Send + Sync {
    /// Append `buf` to the staged bytes.
    ///
    /// # Errors
    /// The underlying write failure.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;

    /// Make the staged contents durable before publishing.
    ///
    /// # Errors
    /// The sync failure.
    fn sync_all(&mut self) -> io::Result<()>;

    /// Atomically install the staged bytes at `final_path` and make the name
    /// durable (parent-directory fsync where the platform requires it).
    ///
    /// # Errors
    /// The persist/rename failure; on error the previous complete file (or
    /// its absence) must remain intact at `final_path`.
    fn persist(
        self: Box<Self>,
        final_path: &Path,
        admission: super::sync::ParentDirSyncAdmission,
    ) -> io::Result<()>;
}

/// A held store-directory lock. Dropping the guard releases the lock.
///
/// Minted by [`StoreFs::try_lock_store_dir`](super::fs::StoreFs::try_lock_store_dir). For [`RealFs`](super::fs::RealFs) this owns the
/// OS advisory lock (`flock`-class); for a virtual backend the runtime is
/// the lock (an in-process registry entry).
pub trait StoreDirLockGuard: Send + Sync {}

/// [`io::Write`] adapter over a [`StoreFile`] handle, for store code that
/// streams through writer APIs (`io::copy`, `BufWriter`, serializers).
pub(crate) struct StoreFileWriter<'a>(pub(crate) &'a mut dyn StoreFile);

impl io::Write for StoreFileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Append-only [`io::Write`] + [`io::Seek`] adapter over a [`StoreFile`],
/// for writers that record their position (`stream_position`) while
/// appending — the SIDX footer writer. Real seeks are honestly refused: an
/// append-only store handle has no random-access write cursor.
pub(crate) struct StoreFileAppendWriter<'a> {
    handle: &'a mut dyn StoreFile,
    pos: u64,
}

impl<'a> StoreFileAppendWriter<'a> {
    /// `start_pos` is the handle's current append position (the caller —
    /// the segment — tracks its own written-byte count).
    pub(crate) fn new(handle: &'a mut dyn StoreFile, start_pos: u64) -> Self {
        Self {
            handle,
            pos: start_pos,
        }
    }
}

impl io::Write for StoreFileAppendWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.handle.write_all(buf)?;
        self.pos = self
            .pos
            .saturating_add(u64::try_from(buf.len()).unwrap_or(u64::MAX));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for StoreFileAppendWriter<'_> {
    fn seek(&mut self, seek_from: io::SeekFrom) -> io::Result<u64> {
        match seek_from {
            io::SeekFrom::Current(0) => Ok(self.pos),
            io::SeekFrom::Start(_) | io::SeekFrom::End(_) | io::SeekFrom::Current(_) => {
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "append-only store handle: only stream_position is supported",
                ))
            }
        }
    }
}

/// Sequential [`io::Read`] + [`io::Seek`] adapter over a [`StoreFile`],
/// for the streaming scan/recovery readers (magic/header reads, frame scans,
/// SIDX footers, compaction source copies). Backed by the positioned-read
/// primitive so it never disturbs a shared OS cursor.
pub(crate) struct StoreFileCursor {
    handle: Box<dyn StoreFile>,
    pos: u64,
}

impl StoreFileCursor {
    pub(crate) fn new(handle: Box<dyn StoreFile>) -> Self {
        Self { handle, pos: 0 }
    }

    /// The wrapped handle (for length queries and the native mmap escape).
    pub(crate) fn get_ref(&self) -> &dyn StoreFile {
        self.handle.as_ref()
    }
}

impl io::Read for StoreFileCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.handle.read_at(self.pos, buf)?;
        self.pos = self
            .pos
            .saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
        Ok(n)
    }
}

impl io::Seek for StoreFileCursor {
    fn seek(&mut self, seek_from: io::SeekFrom) -> io::Result<u64> {
        let new_pos = match seek_from {
            io::SeekFrom::Start(offset) => Some(offset),
            io::SeekFrom::End(delta) => {
                let len = self.handle.len()?;
                len.checked_add_signed(delta)
            }
            io::SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
        };
        match new_pos {
            Some(pos) => {
                self.pos = pos;
                Ok(pos)
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek to a negative or overflowing position",
            )),
        }
    }
}

/// [`StoreFile`] over a real OS file — the handle [`RealFs`](super::fs::RealFs) mints.
pub(crate) struct RealStoreFile {
    file: File,
}

impl RealStoreFile {
    pub(crate) fn new(file: File) -> Self {
        Self { file }
    }
}

impl StoreFile for RealStoreFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        io::Write::write_all(&mut self.file, buf)
    }

    fn sync_data(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }

    fn sync_all(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }

    fn len(&self) -> io::Result<u64> {
        Ok(self.file.metadata()?.len())
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        read_at_impl(&mut self.file, offset, buf)
    }

    fn as_std_file(&self) -> Option<&File> {
        Some(&self.file)
    }
}

/// One positioned read against a real file. Holds the `#[cfg(unix)]` /
/// `FileExt` platform contact the boundary gate confines to `platform/`.
/// The non-unix branch seeks, so callers on those targets must not share the
/// OS cursor across concurrent readers (the FD-cache lock upholds this).
pub(crate) fn read_at_impl(file: &mut File, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_at(buf, offset)
    }
    #[cfg(not(unix))]
    {
        use std::io::{Read, Seek, SeekFrom};
        file.seek(SeekFrom::Start(offset))?;
        file.read(buf)
    }
}

/// [`StagedFile`] over a [`tempfile::NamedTempFile`] — what [`RealFs`](super::fs::RealFs)
/// stages. `persist` is byte-for-byte the pre-0.10.0 protocol:
/// defensive contents fsync, atomic rename, parent-directory fsync.
pub(crate) struct RealStagedFile {
    tmp: NamedTempFile,
}

impl RealStagedFile {
    pub(crate) fn new(tmp: NamedTempFile) -> Self {
        Self { tmp }
    }
}

impl StagedFile for RealStagedFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        io::Write::write_all(&mut self.tmp.as_file(), buf)
    }

    fn sync_all(&mut self) -> io::Result<()> {
        self.tmp.as_file().sync_all()
    }

    fn persist(
        self: Box<Self>,
        final_path: &Path,
        admission: super::sync::ParentDirSyncAdmission,
    ) -> io::Result<()> {
        crate::store::platform::sync::persist_temp_with_parent_sync(self.tmp, final_path, admission)
    }
}

/// [`StoreDirLockGuard`] over the OS advisory lock [`RealFs`](super::fs::RealFs) acquires.
pub(crate) struct RealDirLockGuard {
    _file: File,
}

impl RealDirLockGuard {
    pub(crate) fn new(file: File) -> Self {
        Self { _file: file }
    }
}

impl StoreDirLockGuard for RealDirLockGuard {}

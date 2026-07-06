use super::format;
use crate::store::cold_start::{
    validate_watermark_segment, FileLoad, ReservedKindFallbackStats, WatermarkInfo,
    WatermarkValidationError,
};
use crate::store::index::RoutingSummary;
use memmap2::Mmap;
use std::io::Read;
use std::path::Path;

/// The mmap-index backing bytes: a real memory map when the seam handle is
/// backed by an OS file and the platform admits mmap, or an owned whole-file
/// read when it is not (virtual backends). Byte-identical downstream — every
/// consumer slices `&bytes[..]`.
pub(super) enum MmapIndexBytes {
    Mapped(Mmap),
    Owned(Vec<u8>),
}

impl std::ops::Deref for MmapIndexBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            Self::Mapped(mmap) => mmap,
            Self::Owned(bytes) => bytes,
        }
    }
}

pub(super) struct LoadedMmapIndex {
    pub(super) mmap: MmapIndexBytes,
    pub(super) interner_strings: Vec<String>,
    pub(super) routing: RoutingSummary,
    pub(super) entries_offset: usize,
    pub(super) extension_blob_offset: usize,
    pub(super) extension_blob_len: usize,
    pub(super) entry_count: u64,
    pub(super) entry_size: usize,
    pub(super) version: u16,
    pub(super) watermark: WatermarkInfo,
    pub(super) stored_allocator: u64,
    pub(super) cumulative_reserved_kind_fallbacks: ReservedKindFallbackStats,
}

pub(super) fn invalid_load<T>(reason: impl Into<String>) -> FileLoad<T> {
    FileLoad::Invalid {
        reason: reason.into(),
    }
}

fn read_mmap_u32(bytes: Option<&[u8]>, field: &str) -> Result<u32, String> {
    bytes
        .and_then(format::read_le_u32)
        .ok_or_else(|| format!("mmap index {field} is unreadable"))
}

fn read_mmap_u64(bytes: Option<&[u8]>, field: &str) -> Result<u64, String> {
    bytes
        .and_then(format::read_le_u64)
        .ok_or_else(|| format!("mmap index {field} is unreadable"))
}

fn read_mmap_usize(bytes: Option<&[u8]>, field: &str) -> Result<usize, String> {
    let value = read_mmap_u64(bytes, field)?;
    usize::try_from(value).map_err(|_| format!("mmap index {field} is too large"))
}

/// Resolve the mmap-index backing bytes: memory-map the real OS file when the
/// seam handle exposes one (`as_std_file`) and the platform admits mmap, else
/// fall back to an owned whole-file read — byte-identical downstream. The
/// fallback covers a virtual handle (no OS file), a host the mmap capability
/// gate refuses (`Unknown`/`ObservedUnsupported`/`ProbeFailed`), and a mmap
/// syscall that fails on an admitted host. Only an *unreadable* file yields the
/// `FileLoad` reason the caller must propagate; a mappable-but-unmapped file is
/// still a usable index, so it is never downgraded to a rebuild-from-segments.
fn map_or_read_index_bytes(
    std_file: Option<&std::fs::File>,
    fs: &dyn crate::store::platform::fs::StoreFs,
    data_dir: &Path,
    clock: &dyn crate::store::Clock,
    path: &Path,
) -> Result<MmapIndexBytes, String> {
    let mmap_evidence = crate::store::platform::evidence::collect_for_store_path(data_dir, clock)
        .store_path
        .mmap_index;
    map_or_read_index_bytes_with_evidence(std_file, fs, path, mmap_evidence)
}

/// The decision core of [`map_or_read_index_bytes`] with the platform mmap
/// evidence injected, so the owned-read fallback is deterministically testable
/// without pinning a specific host. `admit_mmap_index` is a CAPABILITY gate: it
/// refuses hosts that cannot (or have not been proven to) mmap, never impugning
/// the index bytes. So a refusal — like a virtual handle or a failed mmap
/// syscall — degrades to the owned whole-file read, which downstream re-runs the
/// same magic/version/CRC validation; a corrupt file still fails closed there.
fn map_or_read_index_bytes_with_evidence(
    std_file: Option<&std::fs::File>,
    fs: &dyn crate::store::platform::fs::StoreFs,
    path: &Path,
    mmap_evidence: crate::store::stats::MmapEvidence,
) -> Result<MmapIndexBytes, String> {
    // The owned whole-file read is the universal fallback — the only branch that
    // may legitimately fail (an unreadable file).
    let read_owned = || match fs.read(path) {
        Ok(bytes) => Ok(MmapIndexBytes::Owned(bytes)),
        Err(error) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "failed to read mmap index"
            );
            Err(format!("failed to read mmap index: {error}"))
        }
    };

    let Some(std_file) = std_file else {
        // Virtual handle: no OS file to map — take the owned whole-file read.
        return read_owned();
    };
    let admission = match crate::store::platform::mmap::admit_mmap_index(mmap_evidence) {
        Ok(admission) => admission,
        Err(error) => {
            // Capability gate refused (this host cannot/should-not mmap here):
            // the index bytes are still good, so read them owned rather than
            // forcing a rebuild-from-segments.
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "mmap index admission refused; falling back to owned read"
            );
            return read_owned();
        }
    };
    // SAFETY: Mmap::map requires a valid open file descriptor. The handle
    // remains open for the duration of the mapping. The mmap index file is
    // read-only and not written to while open — external modification would be
    // a usage error, not a correctness concern for safe Rust callers.
    match unsafe { crate::store::platform::mmap::map_mmap_index_file(std_file, admission) } {
        Ok(mmap) => Ok(MmapIndexBytes::Mapped(mmap)),
        Err(error) => {
            // The syscall failed on an admitted host (e.g. a transient ENOMEM):
            // degrade to the owned read rather than discarding a usable index.
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "failed to mmap index file; falling back to owned read"
            );
            read_owned()
        }
    }
}

pub(super) fn load_mmap_index(
    data_dir: &Path,
    clock: &dyn crate::store::Clock,
    fs: &dyn crate::store::platform::fs::StoreFs,
) -> FileLoad<LoadedMmapIndex> {
    let path = data_dir.join(super::MMAP_INDEX_FILENAME);
    let mut file = match fs.open_file(&path) {
        Ok(handle) => crate::store::platform::fs::StoreFileCursor::new(handle),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return FileLoad::Missing,
        Err(error) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "failed to open mmap index"
            );
            return invalid_load(format!("failed to open mmap index: {error}"));
        }
    };

    let mut prefix = [0u8; format::PREFIX_LEN];
    if let Err(error) = file.read_exact(&mut prefix) {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            error = %error,
            "mmap index header is unreadable"
        );
        return invalid_load(format!("mmap index header is unreadable: {error}"));
    }

    if &prefix[..6] != format::MMAP_INDEX_MAGIC {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            "mmap index has wrong magic"
        );
        return invalid_load("mmap index has wrong magic");
    }

    let version = match format::read_le_u16(&prefix[6..8]) {
        Some(version) => version,
        None => return invalid_load("mmap index version is unreadable"),
    };
    // A version STRICTLY NEWER than this binary supports is a canonical typed
    // refusal — NOT a silent rebuild-from-scan. A future writer may have written
    // segments or summaries this reader cannot interpret, so degrading to a scan
    // would risk a silent downgrade instead of a legally reachable state. Older
    // or otherwise-unknown (but not future) versions stay `Invalid`, which the
    // cold-start flow safely rebuilds from the durable segments.
    // justifies: INV-MMAP-SEALED-READS
    if version > format::MMAP_INDEX_VERSION {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            version,
            supported = format::MMAP_INDEX_VERSION,
            "mmap index declares a future format version; refusing canonically"
        );
        return FileLoad::FutureVersion {
            found: version,
            supported: format::MMAP_INDEX_VERSION,
        };
    }
    if !matches!(version, 1..=4) && version != format::MMAP_INDEX_VERSION {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            version,
            expected = format::MMAP_INDEX_VERSION,
            "unsupported mmap index version"
        );
        return invalid_load(format!("unsupported mmap index version {version}"));
    }

    let header_tail_len = format::header_tail_len(version);
    let header_len = format::header_len(version);
    let mut header_tail = vec![0u8; header_tail_len];
    if let Err(error) = file.read_exact(&mut header_tail) {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            error = %error,
            "mmap index header tail is unreadable"
        );
        return invalid_load(format!("mmap index header tail is unreadable: {error}"));
    }

    let expected_crc = match read_mmap_u32(prefix.get(8..12), "crc") {
        Ok(expected_crc) => expected_crc,
        Err(reason) => return invalid_load(reason),
    };
    let mut cursor = 0usize;
    let watermark_segment_id =
        match read_mmap_u64(header_tail.get(cursor..cursor + 8), "watermark segment id") {
            Ok(value) => value,
            Err(reason) => return invalid_load(reason),
        };
    cursor += 8;
    let watermark_offset =
        match read_mmap_u64(header_tail.get(cursor..cursor + 8), "watermark offset") {
            Ok(value) => value,
            Err(reason) => return invalid_load(reason),
        };
    cursor += 8;
    let stored_allocator =
        match read_mmap_u64(header_tail.get(cursor..cursor + 8), "stored allocator") {
            Ok(value) => value,
            Err(reason) => return invalid_load(reason),
        };
    cursor += 8;
    let interner_count = match read_mmap_u32(header_tail.get(cursor..cursor + 4), "interner count")
    {
        Ok(value) => value,
        Err(reason) => return invalid_load(reason),
    };
    cursor += 4;
    let entry_count = match read_mmap_u64(header_tail.get(cursor..cursor + 8), "entry count") {
        Ok(value) => value,
        Err(reason) => return invalid_load(reason),
    };
    cursor += 8;
    let interner_bytes_len = match read_mmap_u64(
        header_tail.get(cursor..cursor + 8),
        "interner section length",
    ) {
        Ok(value) => value,
        Err(reason) => return invalid_load(reason),
    };
    cursor += 8;
    let summary_bytes_len = if version == 1 {
        0usize
    } else {
        match read_mmap_usize(
            header_tail.get(cursor..cursor + 8),
            "summary section length",
        ) {
            Ok(value) => value,
            Err(reason) => return invalid_load(reason),
        }
    };
    if version != 1 {
        cursor += 8;
    }
    let extension_blob_len = if version >= 5 {
        match read_mmap_usize(header_tail.get(cursor..cursor + 8), "extension blob length") {
            Ok(value) => value,
            Err(reason) => return invalid_load(reason),
        }
    } else {
        0
    };

    let metadata_len = match file.get_ref().len() {
        Ok(len) => len,
        Err(error) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "failed to stat mmap index"
            );
            return invalid_load(format!("failed to stat mmap index: {error}"));
        }
    };

    let file_len = match usize::try_from(metadata_len) {
        Ok(len) => len,
        Err(_) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index file is too large for this platform"
            );
            return invalid_load("mmap index file is too large for this platform");
        }
    };
    let interner_bytes_len = match usize::try_from(interner_bytes_len) {
        Ok(len) => len,
        Err(_) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index interner section is too large"
            );
            return invalid_load("mmap index interner section is too large");
        }
    };
    let entry_count_usize = match usize::try_from(entry_count) {
        Ok(count) => count,
        Err(_) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index entry count is too large"
            );
            return invalid_load("mmap index entry count is too large");
        }
    };
    let entry_size = format::entry_size(version);
    let entry_bytes_len = match entry_count_usize.checked_mul(entry_size) {
        Some(len) => len,
        None => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index entry section length overflowed"
            );
            return invalid_load("mmap index entry section length overflowed");
        }
    };
    let summary_offset = match header_len.checked_add(interner_bytes_len) {
        Some(offset) => offset,
        None => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index header offset overflowed"
            );
            return invalid_load("mmap index header offset overflowed");
        }
    };
    let extension_blob_offset = match summary_offset.checked_add(summary_bytes_len) {
        Some(offset) => offset,
        None => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index summary offset overflowed"
            );
            return invalid_load("mmap index summary offset overflowed");
        }
    };
    let entries_offset = match extension_blob_offset.checked_add(extension_blob_len) {
        Some(offset) => offset,
        None => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index extension blob offset overflowed"
            );
            return invalid_load("mmap index extension blob offset overflowed");
        }
    };
    let expected_len = match entries_offset.checked_add(entry_bytes_len) {
        Some(len) => len,
        None => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                "mmap index total length overflowed"
            );
            return invalid_load("mmap index total length overflowed");
        }
    };
    if file_len != expected_len {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            file_len,
            expected_len,
            "mmap index size does not match header"
        );
        return invalid_load(format!(
            "mmap index size does not match header: file {file_len}, expected {expected_len}"
        ));
    }

    match validate_watermark_segment(data_dir, watermark_segment_id, watermark_offset, fs) {
        Ok(()) => {}
        Err(WatermarkValidationError::OffsetPastTail {
            path: watermark_segment_path,
            file_len,
            watermark_offset,
        }) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                watermark_segment = %watermark_segment_path.display(),
                file_len,
                watermark_offset,
                "mmap index watermark points past the segment tail"
            );
            return invalid_load(format!(
                "mmap index watermark {watermark_offset} points past segment tail {}",
                watermark_segment_path.display()
            ));
        }
        Err(WatermarkValidationError::MissingSegment {
            path: watermark_segment_path,
        }) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                watermark_segment = %watermark_segment_path.display(),
                "mmap index watermark segment is missing"
            );
            return invalid_load(format!(
                "mmap index watermark segment is missing: {}",
                watermark_segment_path.display()
            ));
        }
    }

    // The map is a native-only optimization: it needs a real OS file behind
    // the seam handle (the `as_std_file` escape) AND platform admission. A
    // virtual backend (or a host without mmap) takes the owned whole-file
    // read instead — byte-identical downstream, since every consumer slices.
    let mmap =
        match map_or_read_index_bytes(file.get_ref().as_std_file(), fs, data_dir, clock, &path) {
            Ok(bytes) => bytes,
            Err(reason) => return invalid_load(reason),
        };

    let actual_crc = crc32fast::hash(&mmap[12..]);
    if actual_crc != expected_crc {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            expected_crc,
            actual_crc,
            "mmap index CRC mismatch"
        );
        return invalid_load(format!(
            "mmap index CRC mismatch: expected {expected_crc}, actual {actual_crc}"
        ));
    }

    let interner_slice = &mmap[header_len..summary_offset];
    let interner_strings: Vec<String> = match crate::encoding::from_bytes(interner_slice) {
        Ok(strings) => strings,
        Err(error) => {
            tracing::warn!(
                target: "batpak::mmap_index",
                path = %path.display(),
                error = %error,
                "failed to decode mmap index interner snapshot"
            );
            return invalid_load(format!(
                "failed to decode mmap index interner snapshot: {error}"
            ));
        }
    };

    if interner_strings.len() != interner_count as usize {
        tracing::warn!(
            target: "batpak::mmap_index",
            path = %path.display(),
            expected = interner_count,
            actual = interner_strings.len(),
            "mmap index interner count does not match header"
        );
        return invalid_load(format!(
            "mmap index interner count does not match header: expected {interner_count}, actual {}",
            interner_strings.len()
        ));
    }

    let (routing, cumulative_reserved_kind_fallbacks) = if version == 1 {
        (
            RoutingSummary::default(),
            ReservedKindFallbackStats::default(),
        )
    } else {
        let summary_slice = &mmap[summary_offset..extension_blob_offset];
        if version >= 4 {
            match crate::encoding::from_bytes::<format::MmapSummaryDataV4>(summary_slice) {
                Ok(summary) => (summary.routing, summary.reserved_kind_fallbacks),
                Err(error) => {
                    tracing::warn!(
                        target: "batpak::mmap_index",
                        path = %path.display(),
                        error = %error,
                        "failed to decode mmap index summary section"
                    );
                    return invalid_load(format!(
                        "failed to decode mmap index summary section: {error}"
                    ));
                }
            }
        } else {
            match crate::encoding::from_bytes::<format::MmapSummaryDataV2>(summary_slice) {
                Ok(summary) => (summary.routing, ReservedKindFallbackStats::default()),
                Err(error) => {
                    tracing::warn!(
                        target: "batpak::mmap_index",
                        path = %path.display(),
                        error = %error,
                        "failed to decode mmap index summary section"
                    );
                    return invalid_load(format!(
                        "failed to decode mmap index summary section: {error}"
                    ));
                }
            }
        }
    };

    FileLoad::Loaded(LoadedMmapIndex {
        mmap,
        interner_strings,
        routing,
        entries_offset,
        extension_blob_offset,
        extension_blob_len,
        entry_count,
        entry_size,
        version,
        watermark: WatermarkInfo {
            watermark_segment_id,
            watermark_offset,
        },
        stored_allocator,
        cumulative_reserved_kind_fallbacks,
    })
}

#[cfg(test)]
mod tests {
    use super::{map_or_read_index_bytes_with_evidence, MmapIndexBytes};
    use crate::store::platform::fs::{RealFs, StoreFs};
    use crate::store::stats::MmapEvidence;

    // Write a fixture index file THROUGH the platform seam. The structural gate
    // forbids raw `std::fs` machine-contact outside src/store/platform, and
    // routing create/open through `RealFs` exercises the exact handle path the
    // cold-start loader uses (`open_file(..).as_std_file()`).
    fn write_index(fs: &RealFs, path: &std::path::Path, payload: &[u8]) {
        let mut file = fs.create_new_file(path).expect("create index");
        file.write_all(payload).expect("write index bytes");
        // No sync: the test reads these bytes back in-process, where the OS
        // page cache already serves them; durability is not under test here (and
        // routing a real-file sync belongs to src/store/platform).
        drop(file);
    }

    // The mmap capability gate is a CAPABILITY gate, not a trust gate: every
    // evidence value it refuses must degrade to an owned whole-file read of the
    // SAME bytes, never a hard error that would force a rebuild-from-segments.
    // This is the documented "hosts without mmap" fallback, pinned so the
    // `Some(std_file)` branch can never regress back to returning `Err`.
    #[test]
    fn refused_mmap_evidence_falls_back_to_owned_read() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().join("mmap.index");
        let payload = b"owned-fallback-bytes-are-byte-identical-downstream";
        let fs = RealFs;
        write_index(&fs, &path, payload);
        let handle = fs.open_file(&path).expect("open index");

        for refused in [
            MmapEvidence::Unknown,
            MmapEvidence::ObservedUnsupported,
            MmapEvidence::ProbeFailed,
        ] {
            let bytes =
                map_or_read_index_bytes_with_evidence(handle.as_std_file(), &fs, &path, refused)
                    .expect("a refused capability gate must fall back, not error");
            assert!(
                matches!(bytes, MmapIndexBytes::Owned(_)),
                "refused mmap evidence {refused:?} must read owned bytes, not error"
            );
            assert_eq!(&*bytes, payload, "owned fallback bytes must match the file");
        }
    }

    // A FileBacked-admissible host maps the real file (the fast path stays fast).
    #[test]
    fn admitted_evidence_maps_the_real_file() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().join("mmap.index");
        let payload = b"mapped-path-bytes";
        let fs = RealFs;
        write_index(&fs, &path, payload);
        let handle = fs.open_file(&path).expect("open index");

        let bytes = map_or_read_index_bytes_with_evidence(
            handle.as_std_file(),
            &fs,
            &path,
            MmapEvidence::FileBacked,
        )
        .expect("admitted evidence maps the file");
        assert!(
            matches!(bytes, MmapIndexBytes::Mapped(_)),
            "FileBacked evidence must memory-map the real file"
        );
        assert_eq!(&*bytes, payload, "mapped bytes must match the file");
    }

    // A virtual handle (no OS file) always reads owned, whatever the evidence.
    #[test]
    fn virtual_handle_reads_owned_regardless_of_evidence() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().join("mmap.index");
        let payload = b"virtual-owned-bytes";
        let fs = RealFs;
        write_index(&fs, &path, payload);

        let bytes =
            map_or_read_index_bytes_with_evidence(None, &fs, &path, MmapEvidence::FileBacked)
                .expect("a virtual handle reads owned");
        assert!(
            matches!(bytes, MmapIndexBytes::Owned(_)),
            "a virtual handle must read owned bytes"
        );
        assert_eq!(&*bytes, payload, "owned bytes must match the file");
    }
}

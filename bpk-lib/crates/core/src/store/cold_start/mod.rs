pub(crate) mod checkpoint;
pub(crate) mod mmap;
/// Rebuild-path reports and open-index diagnostics.
pub mod rebuild;
pub(crate) mod row;

#[cfg(test)]
pub(crate) use row::raw_to_kind;
pub(crate) use row::{
    kind_to_raw, raw_to_kind_counted, ColdStartIndexRow, ColdStartSource,
    ReservedKindFallbackStats, WatermarkInfo,
};

use crate::store::{file_classification::StoreFileKind, platform, StoreError};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColdStartArtifactKind {
    MmapIndex,
    Checkpoint,
}

#[derive(Debug)]
pub(crate) enum FileLoad<T> {
    Missing,
    Loaded(T),
    Invalid {
        reason: String,
    },
    /// The on-disk artifact declares a format version strictly newer than this
    /// binary supports. This is a CANONICAL TYPED REFUSAL, distinct from
    /// `Invalid` (corrupt/older → safe to rebuild from scan): a future-version
    /// artifact must NOT be silently rebuilt, because a future writer may have
    /// written data this reader cannot interpret. The cold-start flow propagates
    /// this as a hard error rather than swallowing it into a rebuild.
    FutureVersion {
        /// Version stamped on the on-disk file.
        found: u16,
        /// The maximum version this binary understands.
        supported: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ColdStartPolicy {
    try_checkpoint: bool,
    try_mmap_index: bool,
}

impl ColdStartPolicy {
    pub(crate) fn new(enable_checkpoint: bool, enable_mmap_index: bool) -> Self {
        Self {
            try_checkpoint: enable_checkpoint,
            try_mmap_index: enable_mmap_index,
        }
    }

    pub(crate) fn try_checkpoint(self) -> bool {
        self.try_checkpoint
    }

    pub(crate) fn try_mmap_index(self) -> bool {
        self.try_mmap_index
    }

    pub(crate) fn write_target(self) -> Option<ColdStartArtifactKind> {
        if self.try_mmap_index {
            Some(ColdStartArtifactKind::MmapIndex)
        } else if self.try_checkpoint {
            Some(ColdStartArtifactKind::Checkpoint)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub(crate) enum WatermarkValidationError {
    MissingSegment {
        path: PathBuf,
    },
    OffsetPastTail {
        path: PathBuf,
        file_len: u64,
        watermark_offset: u64,
    },
}

pub(crate) fn latest_segment_watermark(data_dir: &Path) -> Result<(u64, u64), StoreError> {
    let mut max: Option<(u64, PathBuf)> = None;
    for entry in platform::fs::read_dir(data_dir).map_err(StoreError::Io)? {
        let entry = entry.map_err(StoreError::Io)?;
        let path = entry.path();
        let segment_id = match StoreFileKind::from_path(&path) {
            StoreFileKind::Segment(segment_id) => segment_id.as_u64(),
            StoreFileKind::MalformedSegment(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "skipping malformed segment filename"
                );
                continue;
            }
            StoreFileKind::VisibilityRanges
            | StoreFileKind::Checkpoint
            | StoreFileKind::MmapIndex
            | StoreFileKind::IdempotencyStore
            | StoreFileKind::PendingCompactionMarker
            | StoreFileKind::CompactSource
            | StoreFileKind::CursorDirectory
            | StoreFileKind::Keyset
            | StoreFileKind::Other => continue,
        };
        if max
            .as_ref()
            .map(|(current, _)| segment_id > *current)
            .unwrap_or(true)
        {
            max = Some((segment_id, path));
        }
    }

    match max {
        Some((segment_id, path)) => {
            let offset = platform::fs::metadata(&path).map_err(StoreError::Io)?.len();
            Ok((segment_id, offset))
        }
        None => Ok((0, 0)),
    }
}

pub(crate) fn validate_watermark_segment(
    data_dir: &Path,
    watermark_segment_id: u64,
    watermark_offset: u64,
) -> Result<(), WatermarkValidationError> {
    let watermark_segment_path = data_dir.join(crate::store::segment::segment_filename(
        watermark_segment_id,
    ));
    match platform::fs::metadata(&watermark_segment_path) {
        Ok(meta) if meta.len() >= watermark_offset => Ok(()),
        Ok(meta) => Err(WatermarkValidationError::OffsetPastTail {
            path: watermark_segment_path,
            file_len: meta.len(),
            watermark_offset,
        }),
        Err(_) => Err(WatermarkValidationError::MissingSegment {
            path: watermark_segment_path,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_segment_watermark_keeps_the_first_directory_entry_on_a_duplicate_id_tie() {
        // "7.fbat" and "000007.fbat" are DIFFERENT files that both parse to
        // segment id 7 (the stem grammar accepts unpadded base-10 on purpose —
        // see `SegmentId::from_stem`). The max scan keeps the FIRST directory
        // entry among equals (`segment_id > *current` is STRICT), so the
        // watermark offset is the byte length of whichever tied file the
        // directory yields first. The oracle below replays that same
        // first-wins rule over the same (stable) read_dir order; the
        // `>` -> `>=` mutant keeps the LAST tied entry instead and reports the
        // other file's length (the tied files differ: 3 vs 9 bytes).
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let write_segment = |name: &str, bytes: &'static [u8]| {
            platform::fs::write_file_atomically(dir.path(), &dir.path().join(name), name, |file| {
                use std::io::Write;
                file.write_all(bytes).map_err(StoreError::Io)
            })
            .expect("write segment fixture through the platform seam");
        };
        write_segment("000001.fbat", b"aaaaa");
        write_segment("7.fbat", b"xyz");
        write_segment("000007.fbat", b"123456789");

        let (segment_id, offset) =
            latest_segment_watermark(dir.path()).expect("watermark scan succeeds");
        assert_eq!(segment_id, 7, "the max id wins regardless of tie order");

        let mut expected: Option<(u64, PathBuf)> = None;
        for entry in platform::fs::read_dir(dir.path()).expect("read dir for the oracle") {
            let entry = entry.expect("oracle dir entry");
            let path = entry.path();
            let StoreFileKind::Segment(id) = StoreFileKind::from_path(&path) else {
                continue;
            };
            let id = id.as_u64();
            if expected
                .as_ref()
                .map(|(current, _)| id > *current)
                .unwrap_or(true)
            {
                expected = Some((id, path));
            }
        }
        let (expected_id, expected_path) = expected.expect("oracle saw the segment files");
        assert_eq!(expected_id, 7, "oracle sanity: the tie sits at the max id");
        let expected_offset = platform::fs::metadata(&expected_path)
            .expect("oracle metadata")
            .len();
        assert_eq!(
            offset, expected_offset,
            "on a duplicate-id tie the FIRST directory entry's length is the watermark offset"
        );
    }
}

use super::CheckpointEntry;
use crate::store::cold_start::{FileLoad, ReservedKindFallbackStats};
use crate::store::index::RoutingSummary;
use crate::store::platform::fs::{write_file_atomically_with_fs, StoreFs};
use crate::store::StoreError;
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Magic bytes at the start of every checkpoint file.
pub(super) const CHECKPOINT_MAGIC: &[u8; 6] = b"FBATCK";

/// Format version stored in the checkpoint header.
/// v6: v5 plus receipt-extension maps in checkpoint entries. The reader accepts
/// the current v5/v6 snapshot; pre-0.10.0 v2/v3/v4 checkpoints are no longer
/// read — an older checkpoint is ignored and the index rebuilds from segments.
pub const CHECKPOINT_VERSION: u16 = 6;

/// Final checkpoint filename inside the data directory.
pub(crate) const CHECKPOINT_FILENAME: &str = "index.ckpt";

const HEADER_LEN: usize = 6 + 2 + 4;

/// Checkpoint format v6: v5 plus receipt-extension maps in entries.
#[derive(Serialize, Deserialize)]
pub(super) struct CheckpointDataV6 {
    pub(super) global_sequence: u64,
    pub(super) watermark_segment_id: u64,
    pub(super) watermark_offset: u64,
    pub(super) interner_strings: Vec<String>,
    pub(super) routing: RoutingSummary,
    #[serde(default)]
    pub(super) reserved_kind_fallbacks: ReservedKindFallbackStats,
    pub(super) entries: Vec<CheckpointEntry>,
}

pub(super) struct LoadedCheckpointFile {
    pub(super) path: PathBuf,
    pub(super) version: u16,
    pub(super) body: Vec<u8>,
}

pub(super) struct DecodedCheckpointData {
    pub(super) entries: Vec<CheckpointEntry>,
    pub(super) interner_strings: Vec<String>,
    pub(super) watermark_segment_id: u64,
    pub(super) watermark_offset: u64,
    pub(super) global_sequence: u64,
    pub(super) routing: RoutingSummary,
    pub(super) cumulative_reserved_kind_fallbacks: ReservedKindFallbackStats,
}

pub(super) fn read_checkpoint_file(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
) -> FileLoad<LoadedCheckpointFile> {
    let path = data_dir.join(CHECKPOINT_FILENAME);

    let raw = match fs.read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return FileLoad::Missing,
        Err(error) => {
            tracing::warn!(
                target: "batpak::checkpoint",
                path = %path.display(),
                error = %error,
                "failed to read checkpoint file"
            );
            return FileLoad::Invalid {
                reason: format!("read failed: {error}"),
            };
        }
    };

    if raw.len() < HEADER_LEN {
        tracing::warn!(
            target: "batpak::checkpoint",
            path = %path.display(),
            len = raw.len(),
            "checkpoint file too short to contain a valid header"
        );
        return FileLoad::Invalid {
            reason: format!("checkpoint file too short: {} bytes", raw.len()),
        };
    }

    if &raw[..6] != CHECKPOINT_MAGIC.as_ref() {
        tracing::warn!(
            target: "batpak::checkpoint",
            path = %path.display(),
            "checkpoint file has wrong magic bytes — ignoring"
        );
        return FileLoad::Invalid {
            reason: "wrong magic bytes".to_owned(),
        };
    }

    let version = u16::from_le_bytes([raw[6], raw[7]]);
    // A future (newer-than-supported) checkpoint is a CANONICAL TYPED REFUSAL,
    // never a silent rebuild-from-scan: a future writer may have written a
    // snapshot this reader cannot interpret. The version field lives OUTSIDE the
    // CRC region (the CRC at bytes 8..12 covers only the body at bytes 12..), so
    // this check fires before the CRC check — a forged version alone trips it.
    // Corrupt or genuinely-older artifacts keep their graceful-rebuild path via
    // `FileLoad::Invalid` below (older versions still decode in
    // `decode_checkpoint_data`). justifies: INV-ONDISK-FORWARD-COMPAT-CANONICAL
    if version > CHECKPOINT_VERSION {
        tracing::warn!(
            target: "batpak::checkpoint",
            path = %path.display(),
            version,
            supported = CHECKPOINT_VERSION,
            "checkpoint declares a future format version — refusing canonically"
        );
        return FileLoad::FutureVersion {
            found: version,
            supported: CHECKPOINT_VERSION,
        };
    }
    let stored_crc = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);
    let body = raw[HEADER_LEN..].to_vec();
    let computed_crc = crc32fast::hash(&body);
    if stored_crc != computed_crc {
        tracing::warn!(
            target: "batpak::checkpoint",
            path = %path.display(),
            stored = stored_crc,
            computed = computed_crc,
            "checkpoint CRC mismatch — file is corrupt, ignoring"
        );
        return FileLoad::Invalid {
            reason: format!("crc mismatch: stored {stored_crc}, computed {computed_crc}"),
        };
    }

    FileLoad::Loaded(LoadedCheckpointFile {
        path,
        version,
        body,
    })
}

pub(super) fn decode_checkpoint_data(
    path: &Path,
    version: u16,
    body: &[u8],
) -> Option<DecodedCheckpointData> {
    match version {
        5 | 6 => {
            let data: CheckpointDataV6 =
                decode_body(path, body, "checkpoint deserialisation failed — ignoring")?;
            Some(DecodedCheckpointData {
                entries: data.entries,
                interner_strings: data.interner_strings,
                watermark_segment_id: data.watermark_segment_id,
                watermark_offset: data.watermark_offset,
                global_sequence: data.global_sequence,
                routing: data.routing,
                cumulative_reserved_kind_fallbacks: data.reserved_kind_fallbacks,
            })
        }
        _ => {
            tracing::warn!(
                target: "batpak::checkpoint",
                path = %path.display(),
                version,
                expected = CHECKPOINT_VERSION,
                "unsupported checkpoint version — ignoring"
            );
            None
        }
    }
}

pub(super) fn decode_checkpoint_snapshot_v6(path: &Path, body: &[u8]) -> Option<CheckpointDataV6> {
    decode_body(
        path,
        body,
        "checkpoint snapshot deserialisation failed — ignoring",
    )
}

pub(super) fn encode_checkpoint_body<T: Serialize + ?Sized>(
    body: &T,
) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    crate::encoding::to_bytes(body)
}

pub(super) fn write_checkpoint_file(
    data_dir: &Path,
    body: &[u8],
    fs: &dyn StoreFs,
) -> Result<(), StoreError> {
    let crc: u32 = crc32fast::hash(body);
    let final_path = data_dir.join(CHECKPOINT_FILENAME);
    write_file_atomically_with_fs(
        data_dir,
        &final_path,
        "checkpoint",
        |file| {
            let mut w = BufWriter::new(file);

            w.write_all(CHECKPOINT_MAGIC)?;
            w.write_all(&CHECKPOINT_VERSION.to_le_bytes())?;
            w.write_all(&crc.to_le_bytes())?;
            w.write_all(body)?;
            w.flush()?;
            Ok(())
        },
        fs,
    )?;
    Ok(())
}

fn decode_body<T: for<'de> Deserialize<'de>>(
    path: &Path,
    body: &[u8],
    message: &'static str,
) -> Option<T> {
    match crate::encoding::from_bytes(body) {
        Ok(data) => Some(data),
        Err(error) => {
            tracing::warn!(
                target: "batpak::checkpoint",
                path = %path.display(),
                error = %error,
                message
            );
            None
        }
    }
}

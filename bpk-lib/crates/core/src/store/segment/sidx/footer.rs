use super::{ENTRY_SIZE, SIDX_MAGIC};
use crate::store::StoreError;
use std::io::{Read, Seek, SeekFrom};

/// Size of the fixed-layout trailer that terminates the SIDX footer:
/// `string_table_offset(8) + entry_count(4) + magic(4)` = 16 bytes.
pub(super) const TRAILER_SIZE: u64 = 16;
const TRAILER_SIZE_USIZE: usize = 16;

/// Size of the CRC32 that sits immediately before the trailer, covering the
/// contiguous `[string_table_bytes ++ entries]` region.
pub(super) const SIDX_CRC_LEN: u64 = 4;
const SIDX_CRC_LEN_USIZE: usize = 4;

pub(super) fn sidx_crc_len_usize() -> usize {
    SIDX_CRC_LEN_USIZE
}

pub(super) struct FooterLayout {
    pub(super) string_table_offset: u64,
    pub(super) string_table_len: u64,
    pub(super) entry_count: usize,
}

pub(super) fn trailer_size_usize() -> usize {
    TRAILER_SIZE_USIZE
}

pub(super) fn read_layout<R: Read + Seek>(
    reader: &mut R,
    segment_id: u64,
) -> Result<Option<FooterLayout>, StoreError> {
    let file_len = reader.seek(SeekFrom::End(0)).map_err(StoreError::Io)?;
    if file_len < TRAILER_SIZE {
        return Ok(None);
    }

    reader
        .seek(SeekFrom::End(-(TRAILER_SIZE as i64)))
        .map_err(StoreError::Io)?;

    let mut trailer = [0u8; 16];
    reader.read_exact(&mut trailer).map_err(StoreError::Io)?;

    if &trailer[12..16] != SIDX_MAGIC {
        return Ok(None);
    }

    let string_table_offset = read_trailer_u64(&trailer[0..8], segment_id)?;
    let entry_count = read_trailer_u32(&trailer[8..12], segment_id)? as usize;
    let entries_block_len = (entry_count as u64)
        .checked_mul(ENTRY_SIZE as u64)
        .ok_or_else(|| StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX entry_count × ENTRY_SIZE overflows u64".into(),
        })?;

    // entries_start = file_len - TRAILER - CRC - entries_block_len. The CRC's 4
    // bytes sit at [entries_start + entries_block_len .. + 4), immediately before
    // the 16-byte trailer; subtracting it here keeps the entries/table geometry
    // byte-identical to the region write_footer hashed.
    let entries_start = file_len
        .checked_sub(TRAILER_SIZE)
        .and_then(|n| n.checked_sub(SIDX_CRC_LEN))
        .and_then(|n| n.checked_sub(entries_block_len))
        .ok_or_else(|| StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX entry block extends before the beginning of the file".into(),
        })?;

    if string_table_offset > entries_start {
        return Err(StoreError::CorruptSegment {
            segment_id,
            detail: format!(
                "SIDX string_table_offset {string_table_offset} is past entries_start {entries_start}"
            ),
        });
    }

    let string_table_len = entries_start
        .checked_sub(string_table_offset)
        .ok_or_else(|| StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX string table length underflows".into(),
        })?;

    // Integrity check: recompute CRC32 over the contiguous covered region
    // [string_table_offset .. entries_start + entries_block_len) and compare it to
    // the 4 stored CRC bytes that sit immediately after the entries block. A
    // mismatch (or an old SDX2-era footer that reached here, which it cannot — the
    // magic gate above already rejected it) means the footer cannot be trusted, so
    // we return Ok(None) to degrade to the CRC-verified frame-scan rebuild. Only an
    // actual IO failure surfaces as StoreError::Io.
    let covered_end = entries_start
        .checked_add(entries_block_len)
        .ok_or_else(|| StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX covered region end overflows u64".into(),
        })?;
    let covered_len = usize::try_from(string_table_len.checked_add(entries_block_len).ok_or_else(
        || StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX covered region length overflows u64".into(),
        },
    )?)
    .map_err(|_| StoreError::CorruptSegment {
        segment_id,
        detail: "SIDX covered region length exceeds usize::MAX".into(),
    })?;

    reader
        .seek(SeekFrom::Start(string_table_offset))
        .map_err(StoreError::Io)?;
    let mut covered = vec![0u8; covered_len];
    reader.read_exact(&mut covered).map_err(StoreError::Io)?;

    reader
        .seek(SeekFrom::Start(covered_end))
        .map_err(StoreError::Io)?;
    let mut stored_crc = [0u8; 4];
    reader.read_exact(&mut stored_crc).map_err(StoreError::Io)?;

    if crc32fast::hash(&covered) != u32::from_le_bytes(stored_crc) {
        return Ok(None);
    }

    Ok(Some(FooterLayout {
        string_table_offset,
        string_table_len,
        entry_count,
    }))
}

fn read_trailer_u64(bytes: &[u8], segment_id: u64) -> Result<u64, StoreError> {
    let bytes: [u8; 8] = bytes.try_into().map_err(|_| StoreError::CorruptFrame {
        segment_id,
        offset: 0,
        reason: "trailer truncated: string_table_offset bytes not readable".into(),
    })?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_trailer_u32(bytes: &[u8], segment_id: u64) -> Result<u32, StoreError> {
    let bytes: [u8; 4] = bytes.try_into().map_err(|_| StoreError::CorruptFrame {
        segment_id,
        offset: 0,
        reason: "trailer truncated: entry_count bytes not readable".into(),
    })?;
    Ok(u32::from_le_bytes(bytes))
}

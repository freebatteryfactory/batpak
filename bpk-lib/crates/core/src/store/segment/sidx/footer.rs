use super::{ENTRY_SIZE, SIDX_MAGIC};
use crate::store::StoreError;
use std::io::{Read, Seek, SeekFrom};

/// Size of the fixed-layout trailer that terminates the SIDX footer:
/// `string_table_offset(8) + entry_count(4) + magic(4)` = 16 bytes.
pub(super) const TRAILER_SIZE: u64 = 16;
const TRAILER_SIZE_USIZE: usize = 16;

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

    let entries_start = file_len
        .checked_sub(TRAILER_SIZE)
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

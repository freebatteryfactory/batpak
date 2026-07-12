use super::{SidxEntry, ENTRY_SIZE, SIDX_MAGIC, SIDX_MAGIC_LEGACY_SDX2};
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
    // STRICT `<`: a file of EXACTLY `TRAILER_SIZE` bytes still contains a full
    // 16-byte trailer and must be parsed (then rejected as corrupt because there
    // is no room for entries/CRC). The `< -> <=` mutant would short-circuit it as
    // "no footer" — see `read_layout_parses_a_trailer_only_file_then_rejects_it`.
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
    // `covered_len` is derived from on-disk geometry. The bounds checks above
    // prove `covered_end <= file_len`, so `covered_len` cannot exceed the file
    // size — but a corrupt/adversarial trailer with `string_table_offset` near 0
    // on a large or sparse segment can still drive it to nearly the whole file.
    // Hash the covered region in fixed-size chunks so memory stays O(chunk)
    // instead of O(covered_len), keeping cold-start reopen from OOM-ing on a
    // forged footer before the CRC can reject it and trigger the frame-scan
    // fallback. The result is byte-identical to hashing the whole span at once.
    let covered_len = string_table_len
        .checked_add(entries_block_len)
        .ok_or_else(|| StoreError::CorruptSegment {
            segment_id,
            detail: "SIDX covered region length overflows u64".into(),
        })?;

    reader
        .seek(SeekFrom::Start(string_table_offset))
        .map_err(StoreError::Io)?;
    let mut hasher = crc32fast::Hasher::new();
    let mut remaining = covered_len;
    let mut chunk = [0u8; 8192];
    while remaining > 0 {
        // `remaining` is clamped to `chunk.len()` (a usize), so the result
        // always fits in usize regardless of how large the corrupt span claims
        // to be — the `min` is computed on the usize side to make that explicit.
        let take = usize::try_from(remaining)
            .unwrap_or(chunk.len())
            .min(chunk.len());
        reader
            .read_exact(&mut chunk[..take])
            .map_err(StoreError::Io)?;
        hasher.update(&chunk[..take]);
        remaining -= take as u64;
    }

    reader
        .seek(SeekFrom::Start(covered_end))
        .map_err(StoreError::Io)?;
    let mut stored_crc = [0u8; 4];
    reader.read_exact(&mut stored_crc).map_err(StoreError::Io)?;

    if hasher.finalize() != u32::from_le_bytes(stored_crc) {
        return Ok(None);
    }

    Ok(Some(FooterLayout {
        string_table_offset,
        string_table_len,
        entry_count,
    }))
}

/// Parse the SIDX entry table from a footer WITHOUT requiring the footer CRC to
/// pass — the untrusted entry-table read.
///
/// This is the manifest-recovery counterpart to [`read_layout`]: it reads the
/// fixed 16-byte trailer geometry (`string_table_offset` + `entry_count`),
/// applies the SAME bounds guards that [`read_layout`] uses (entry block must not
/// extend before the start of the file, `string_table_offset` must not be past
/// `entries_start`), then decodes `entry_count` raw [`SidxEntry`] records via
/// [`SidxEntry::decode_from`] — which is CRC-INDEPENDENT — directly from the
/// entries block. The footer CRC is NEVER verified here, so this works on a
/// CRC-failed SDX3 footer, a legacy un-CRC'd SDX2 footer, or a partially-forged
/// trailer.
///
/// EVERY entry returned is an UNTRUSTED HYPOTHESIS, and there is NO parse- or
/// hash-level step that can upgrade one (GAUNT-SIDX-NO-SELF-AUTH, #192): BLAKE3
/// is public and unkeyed, so a forger who can read or edit the segment can copy
/// a real frame's stored `event_hash` — a matching hash proves nothing about
/// sibling rows, `entry_count`, order, or the footer boundary. The caller
/// (`segment::recovery_manifest`) therefore consumes these rows ONLY as
/// suspicion geometry (offsets claimed at/after the recovered prefix end),
/// routed through the scan tail posture; no row is ever treated as an authority
/// over recovery. Nothing short of a keyed MAC, digital signature, or
/// independently trusted root over the COMPLETE table may change that.
///
/// On ANY geometry/parse failure (absurd `entry_count`, `entry_count × ENTRY_SIZE`
/// overflow, an entries block that runs before the start of the file, a
/// `string_table_offset` past `entries_start`, a magic that is neither `SDX3` nor
/// the legacy `SDX2`, or a short/torn read) this returns ZERO entries
/// (`Ok(Vec::new())`) so the caller cleanly falls back to tail-policy behavior.
/// Only a genuine non-EOF IO failure surfaces as [`StoreError::Io`].
///
/// Both `SDX3` and the legacy `SDX2` magic are accepted because both are
/// recognized as frame-region boundaries by `detect_sidx_boundary`, and the
/// untrusted recovery path handles both provenances identically (the entries
/// block geometry is byte-identical across the two magics).
pub(super) fn read_entries_unauthenticated<R: Read + Seek>(
    reader: &mut R,
    segment_id: u64,
) -> Result<Vec<SidxEntry>, StoreError> {
    let file_len = reader.seek(SeekFrom::End(0)).map_err(StoreError::Io)?;
    if file_len < TRAILER_SIZE {
        return Ok(Vec::new());
    }

    reader
        .seek(SeekFrom::End(-(TRAILER_SIZE as i64)))
        .map_err(StoreError::Io)?;
    let mut trailer = [0u8; 16];
    reader.read_exact(&mut trailer).map_err(StoreError::Io)?;

    // Accept BOTH the current SDX3 and legacy SDX2 magics — the boundary detector
    // recognizes both, and the manifest geometry is identical. A trailer with any
    // other magic carries no parseable entry table → zero entries.
    let magic = &trailer[12..16];
    if magic != SIDX_MAGIC && magic != SIDX_MAGIC_LEGACY_SDX2 {
        return Ok(Vec::new());
    }

    let string_table_offset = u64::from_le_bytes([
        trailer[0], trailer[1], trailer[2], trailer[3], trailer[4], trailer[5], trailer[6],
        trailer[7],
    ]);
    let entry_count = u32::from_le_bytes([trailer[8], trailer[9], trailer[10], trailer[11]]) as u64;

    // Reuse read_layout's geometry guards. ANY failure → zero entries (fall back),
    // never a hard error: a bogus entry_count, an overflowing entries block, or an
    // out-of-range offset is exactly the "no trustworthy signal" case.
    let Some(entries_block_len) = entry_count.checked_mul(ENTRY_SIZE as u64) else {
        return Ok(Vec::new());
    };
    let Some(entries_start) = file_len
        .checked_sub(TRAILER_SIZE)
        .and_then(|n| n.checked_sub(SIDX_CRC_LEN))
        .and_then(|n| n.checked_sub(entries_block_len))
    else {
        return Ok(Vec::new());
    };
    // string_table_offset must sit at/before the entries block start, same as the
    // authenticated layout invariant. A violation means the geometry is garbage.
    if string_table_offset > entries_start {
        return Ok(Vec::new());
    }

    // Decode the entries block in place. The recovery decision consumes only the
    // claimed geometry (frame_offset, frame_length) as suspicion input — so we
    // skip straight to entries_start and read the raw fixed-size records.
    // entity_idx / scope_idx string-table bounds are NOT validated here: every
    // field is an untrusted hypothesis either way, and no field is ever trusted
    // (#192 — rows are suspicion, never authority).
    reader
        .seek(SeekFrom::Start(entries_start))
        .map_err(StoreError::Io)?;

    let count = usize::try_from(entry_count).unwrap_or(usize::MAX);
    let mut entries = Vec::with_capacity(count.min(1024));
    let mut buf = [0u8; ENTRY_SIZE];
    for _ in 0..entry_count {
        if let Err(e) = reader.read_exact(&mut buf) {
            // A torn/short entries block → no usable manifest geometry. Return
            // zero entries so the caller falls back, rather than a partial table
            // feeding partial suspicion. A real non-EOF IO error still propagates.
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(Vec::new());
            }
            return Err(StoreError::Io(e));
        }
        match SidxEntry::decode_from(&buf, segment_id) {
            Ok(entry) => entries.push(entry),
            // decode_from only errors on a wrong buffer length, which cannot happen
            // here (buf is exactly ENTRY_SIZE); treat defensively as "no manifest".
            Err(_) => return Ok(Vec::new()),
        }
    }

    Ok(entries)
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

#[cfg(test)]
mod tests {
    use super::{
        read_entries_unauthenticated, read_layout, sidx_crc_len_usize, trailer_size_usize,
        ENTRY_SIZE, SIDX_MAGIC, SIDX_MAGIC_LEGACY_SDX2,
    };
    use crate::store::StoreError;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    /// A `Read + Seek` over an in-memory footer whose reads at any position BELOW
    /// `fault_below` fail with a NON-EOF IO error (`PermissionDenied`), while reads
    /// AT or ABOVE it (the trailer) succeed. Lets a test drive
    /// `read_entries_unauthenticated` through a valid trailer parse and THEN inject a
    /// real IO failure on the entries read — the exact case the EOF-vs-non-EOF
    /// discriminator must distinguish (torn block → fall back; real IO error →
    /// propagate). A plain `Cursor` cannot reach that branch because the geometry
    /// guards guarantee the entries block is present.
    struct FaultOnEntriesRead {
        data: Vec<u8>,
        pos: u64,
        fault_below: u64,
    }

    impl Read for FaultOnEntriesRead {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pos < self.fault_below {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected: entries read fault",
                ));
            }
            let start = usize::try_from(self.pos)
                .expect("pos fits usize")
                .min(self.data.len());
            let available = &self.data[start..];
            let n = available.len().min(buf.len());
            buf[..n].copy_from_slice(&available[..n]);
            self.pos += n as u64;
            Ok(n)
        }
    }

    impl Seek for FaultOnEntriesRead {
        fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
            let len = self.data.len() as i64;
            let target = match from {
                SeekFrom::Start(n) => n as i64,
                SeekFrom::End(n) => len + n,
                SeekFrom::Current(n) => self.pos as i64 + n,
            };
            if target < 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "seek before start",
                ));
            }
            self.pos =
                u64::try_from(target).expect("seek target is non-negative after the guard above");
            Ok(self.pos)
        }
    }

    /// A minimal footer carrying ONE all-zero entry whose geometry places
    /// `string_table_offset` exactly at `entries_start` (empty string table):
    /// `[entry: ENTRY_SIZE zeros][crc:4][string_table_offset:8 = 0][entry_count:4 = 1][magic:4]`.
    /// `file_len = ENTRY_SIZE + 16` so `entries_start = file_len - 16 - 4 - ENTRY_SIZE == 0`.
    /// The CRC is left zero because `read_entries_unauthenticated` never verifies it.
    fn footer_one_zero_entry(magic: &[u8; 4]) -> Vec<u8> {
        let mut bytes = vec![0u8; ENTRY_SIZE];
        bytes.extend_from_slice(&0u32.to_le_bytes()); // CRC (ignored on the unauthenticated path)
        bytes.extend_from_slice(&0u64.to_le_bytes()); // string_table_offset = 0 == entries_start
        bytes.extend_from_slice(&1u32.to_le_bytes()); // entry_count = 1
        bytes.extend_from_slice(magic);
        bytes
    }

    #[test]
    fn footer_constant_helpers_report_the_exact_on_disk_byte_widths() {
        // These helpers feed the footer geometry (CRC precedes the fixed 16-byte
        // trailer). The `-> 0` / `-> 1` mutants replace the constant body; pinning
        // the exact byte widths convicts every one of them.
        assert_eq!(
            sidx_crc_len_usize(),
            4,
            "the SIDX footer CRC is exactly 4 bytes (a u32 LE)"
        );
        assert_eq!(
            trailer_size_usize(),
            16,
            "the SIDX trailer is exactly 16 bytes (offset8 + count4 + magic4)"
        );
    }

    #[test]
    fn read_entries_unauthenticated_returns_empty_for_a_subtrailer_file() {
        // A file shorter than the 16-byte trailer carries no parseable manifest:
        // the size guard returns ZERO entries as a clean fall-back, WITHOUT ever
        // seeking. The `< -> ==` mutant only short-circuits at file_len == 16, so
        // for a 10-byte file it falls through to `seek(End(-16))`, which underflows
        // past the start of the file and surfaces an IO error instead of Ok(empty).
        let bytes = vec![0xEEu8; 10]; // strictly under TRAILER_SIZE (16)
        let mut cursor = Cursor::new(bytes);
        let parsed = read_entries_unauthenticated(&mut cursor, 7)
            .expect("a sub-trailer file must yield an empty manifest, never an error");
        assert!(
            parsed.is_empty(),
            "a file too small to hold the 16-byte trailer decodes to zero entries"
        );
    }

    #[test]
    fn read_entries_unauthenticated_accepts_the_legacy_sdx2_magic() {
        // The untrusted read accepts BOTH SDX3 and legacy SDX2 (the boundary
        // detector recognizes both, and the entry geometry is byte-identical). The
        // `!= SDX2 -> == SDX2` mutant on the magic gate INVERTS the contract:
        // it would drop a real SDX2 manifest (return zero) while admitting garbage
        // magics. A genuine SDX2 footer's entry table must still parse.
        let bytes = footer_one_zero_entry(SIDX_MAGIC_LEGACY_SDX2);
        let mut cursor = Cursor::new(bytes);
        let parsed = read_entries_unauthenticated(&mut cursor, 7).expect("must not error");
        assert_eq!(
            parsed.len(),
            1,
            "a legacy SDX2 footer's entry table must parse (decode_from is CRC-independent)"
        );
        assert_eq!(
            parsed[0].frame_offset, 0,
            "the decoded (zeroed) entry preserves its frame_offset field"
        );
    }

    #[test]
    fn read_entries_unauthenticated_admits_offset_equal_to_entries_start() {
        // The geometry guard rejects only a `string_table_offset` STRICTLY PAST
        // `entries_start`; `offset == entries_start` (an empty string table) is a
        // legal footer and its entries must still decode. Both the `> -> ==` and
        // `> -> >=` mutants treat `offset == entries_start` as garbage and drop the
        // whole manifest, so a footer built at that exact boundary convicts them.
        let bytes = footer_one_zero_entry(SIDX_MAGIC);
        let mut cursor = Cursor::new(bytes);
        let parsed = read_entries_unauthenticated(&mut cursor, 7).expect("must not error");
        assert_eq!(
            parsed.len(),
            1,
            "offset == entries_start (empty string table) is legal geometry and must parse its entry"
        );
    }

    #[test]
    fn read_entries_unauthenticated_propagates_a_non_eof_io_fault_on_the_entries_read() {
        // A genuine (non-EOF) IO failure while reading the entries block must
        // PROPAGATE as StoreError::Io — only a torn/short (UnexpectedEof) block may
        // degrade to the zero-entries fall-back. The `== UnexpectedEof`→`!=` mutant
        // inverts that discriminator: it would swallow a real IO fault as "no
        // manifest" (and, conversely, surface a benign torn block as a hard error).
        // We parse a valid SDX3 trailer, then fault the entries read.
        let data = footer_one_zero_entry(SIDX_MAGIC); // valid trailer, entries at offset 0
        let fault_below = (data.len() as u64) - 16; // the 16-byte trailer reads cleanly; entries fault
        let mut reader = FaultOnEntriesRead {
            data,
            pos: 0,
            fault_below,
        };

        let err = read_entries_unauthenticated(&mut reader, 7)
            .expect_err("a non-EOF IO fault on the entries read must propagate, not fall back");
        assert!(
            matches!(err, StoreError::Io(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied),
            "a real IO error must surface as StoreError::Io(PermissionDenied), not be swallowed as \
             an empty manifest; got {err:?}"
        );
    }

    #[test]
    fn read_layout_parses_a_trailer_only_file_then_rejects_it() {
        // A file of EXACTLY TRAILER_SIZE (16) bytes carries a full trailer but no
        // room for the entries block + CRC, so real `read_layout` parses the
        // trailer and then fails the geometry check with CorruptSegment. The
        // `< -> <=` mutant treats `file_len == TRAILER_SIZE` as "no footer" and
        // returns Ok(None) — so asserting the result is an Err kills it.
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&0u64.to_le_bytes()); // string_table_offset = 0
        buf.extend_from_slice(&0u32.to_le_bytes()); // entry_count = 0
        buf.extend_from_slice(SIDX_MAGIC); // valid SDX3 magic (total = 16 bytes)
        assert_eq!(
            buf.len(),
            16,
            "trailer-only fixture must be exactly TRAILER_SIZE"
        );

        let is_err = read_layout(&mut Cursor::new(buf), 0).is_err();
        assert!(
            is_err,
            "a magic-bearing file of exactly TRAILER_SIZE must be parsed and rejected as \
             corrupt (no room for entries/CRC), not short-circuited as Ok(None)"
        );
    }
}

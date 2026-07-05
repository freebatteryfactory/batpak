//! SIDX frame-region boundary recovery tests.
//!
//! Extracted from the inline `mod tests` island in `segment/mod.rs` to stay
//! within the inline-test-island budget; these pin the untrusted-offset
//! recovery walker (`crc_valid_frames_end`), the trust-provenance detection
//! (`detect_sidx_boundary`), and the compaction-copy recovery on
//! `append_frames_from_segment` (audit P1s).
//!
//! Invariant under test: for an UNTRUSTED boundary the trailer
//! `string_table_offset` is GARBAGE and must NEVER bound recovery — whether it
//! is too LOW, MID-FRAME, or too HIGH, the walker recovers ALL CRC-valid frames
//! bounded only by `file_len`. A TRUSTED (CRC-authenticated SDX3) offset stays
//! authoritative.

use super::*;
use std::io::Cursor;
use tempfile::TempDir;

/// Production [`RealFs`] behind the [`StoreFs`] seam, for the segment-create
/// calls in these boundary tests (the durability seam is exercised by the sim
/// tests; here we only need a real backing filesystem).
fn realfs() -> std::sync::Arc<dyn crate::store::platform::fs::StoreFs> {
    std::sync::Arc::new(crate::store::platform::fs::RealFs)
}

/// Build an in-memory buffer of `[real CRC-valid frames][CRC-valid SDX3 footer]`.
/// Returns `(bytes, frames_end)` where `frames_end` is the SIDX
/// `string_table_offset` the footer records (= the true end of the frames).
fn frames_then_sdx3_footer(payloads: &[&str]) -> (Vec<u8>, u64) {
    use crate::store::segment::sidx::{kind_to_raw, SidxEntry, SidxEntryCollector};

    let mut bytes = Vec::new();
    let mut collector = SidxEntryCollector::new();
    for (idx, p) in payloads.iter().enumerate() {
        let frame_offset = bytes.len() as u64;
        let frame = frame_encode(&serde_json::json!({ "payload": p })).expect("encode frame");
        let frame_length = u32::try_from(frame.len()).expect("frame length fits u32");
        bytes.extend_from_slice(&frame);
        let entry = SidxEntry {
            event_id: idx as u128 + 1,
            entity_idx: 0,
            scope_idx: 0,
            kind: kind_to_raw(crate::event::EventKind::custom(0x1, 1)),
            wall_ms: 1,
            clock: 1,
            dag_lane: 0,
            dag_depth: 0,
            prev_hash: [0; 32],
            event_hash: [1; 32],
            frame_offset,
            frame_length,
            global_sequence: idx as u64 + 1,
            correlation_id: 1,
            causation_id: 0,
        };
        collector
            .record(entry, "entity:test", "scope:test")
            .expect("intern test strings");
    }
    let frames_end = bytes.len() as u64;
    let mut cursor = Cursor::new(&mut bytes);
    cursor.seek(SeekFrom::End(0)).expect("seek to footer start");
    collector
        .write_footer(&mut cursor, 7)
        .expect("write footer");
    (bytes, frames_end)
}

/// Total on-disk size (8-byte header + payload) of the frame that begins at
/// `offset` in `bytes`, read the same way the scan does. Lint-clean: no `unwrap`
/// and no lossy `as` casts on the offset arithmetic.
fn frame_total_len_at(bytes: &[u8], offset: u64) -> u64 {
    let start = usize::try_from(offset).expect("offset fits usize");
    let header: [u8; 4] = bytes[start..start + 4]
        .try_into()
        .expect("4-byte frame length prefix");
    let payload_len = u64::from(u32::from_be_bytes(header));
    8 + payload_len
}

#[test]
fn detect_sidx_boundary_trusts_a_crc_valid_sdx3_footer() {
    // A real, CRC-valid SDX3 footer authenticates its string_table_offset: the
    // boundary is reported `trusted`, so the scan loops keep the strict
    // (FailClosed-on-bad-frame) policy.
    let (bytes, frames_end) = frames_then_sdx3_footer(&["a", "b", "c"]);
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let boundary = detect_sidx_boundary(&mut cursor, file_len, 7)
        .expect("must not error")
        .expect("a CRC-valid SDX3 footer is a boundary");
    assert_eq!(
        boundary,
        SidxBoundary {
            frames_end,
            trusted: true,
        },
        "PROPERTY: a CRC-valid SDX3 footer must mark the boundary trusted with the true frames_end"
    );
}

#[test]
fn detect_sidx_boundary_distrusts_a_crc_failed_sdx3_footer() {
    // Flip a byte inside the footer's string-table/entries region so the SDX3
    // CRC fails. The trailer (magic + offset) is untouched, so the boundary is
    // still recognized — but it must be flagged UNTRUSTED, because the offset is
    // no longer authenticated and a too-high garbage offset must trigger
    // recover-what-was-found, not a blind FailClosed.
    let (mut bytes, frames_end) = frames_then_sdx3_footer(&["a", "b", "c"]);
    // Corrupt a byte just inside the string table (right after the real frames),
    // which is covered by the footer CRC but is not the trailer geometry.
    let corrupt_at = usize::try_from(frames_end).expect("frames_end fits usize");
    bytes[corrupt_at] ^= 0xFF;
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let boundary = detect_sidx_boundary(&mut cursor, file_len, 7)
        .expect("must not error")
        .expect("a CRC-failed footer is still recognized as a boundary");
    assert_eq!(
        boundary,
        SidxBoundary {
            frames_end,
            trusted: false,
        },
        "PROPERTY: a CRC-failed SDX3 footer must be recognized as a boundary but flagged untrusted"
    );
}

#[test]
fn crc_valid_frames_end_recovers_all_frames_for_a_too_high_hint() {
    // The too-HIGH corrupt-offset case: an untrusted offset pointing INTO the
    // footer region (past the real frames). The hint is GARBAGE and discarded —
    // crc_valid_frames_end walks the CRC-valid frames bounded only by file_len and
    // returns the natural end of the real frames. The footer bytes that follow do
    // not decode as a CRC-valid frame, so the walk stops exactly at the true frame
    // region end and every real frame is recovered.
    let (bytes, real_frames_end) = frames_then_sdx3_footer(&["x", "y", "z"]);
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let recovered = crc_valid_frames_end(&mut cursor, 0, file_len, 7).expect("must not error");
    assert_eq!(
        recovered, real_frames_end,
        "PROPERTY: the walker, bounded by file_len, stops at the true CRC-valid frame end and \
         recovers all frames regardless of the (discarded) hint"
    );
}

#[test]
fn crc_valid_frames_end_recovers_all_frames_for_a_too_low_hint() {
    // The too-LOW case at an exact interior frame boundary. The pre-fix walker
    // used the hint as an UPPER BOUND and would have returned `hint` (dropping the
    // CRC-valid frames at/after it). With the hint discarded and file_len the only
    // bound, the walk recovers ALL frames — no CorruptSegment, no dropped frames.
    let (bytes, real_frames_end) = frames_then_sdx3_footer(&["a", "b", "c"]);
    let file_len = bytes.len() as u64;
    // A too-low hint at the start of the SECOND frame (an exact interior boundary).
    let too_low = frame_total_len_at(&bytes, 0);
    assert!(
        too_low > 0 && too_low < real_frames_end,
        "too-low hint must land on an interior frame boundary"
    );
    let mut cursor = Cursor::new(bytes);
    let recovered = crc_valid_frames_end(&mut cursor, 0, file_len, 7).expect("must not error");
    assert_eq!(
        recovered, real_frames_end,
        "PROPERTY: a too-low untrusted hint must NOT bound recovery; all CRC-valid frames recover"
    );
}

#[test]
fn crc_valid_frames_end_recovers_all_frames_for_a_mid_frame_hint() {
    // The MID-FRAME case (the gap that kept slipping): an untrusted offset landing
    // INSIDE a later CRC-valid frame's header/payload, not at a frame boundary.
    // No frame *begins* at the offset, so the old truncation guard never fired;
    // and the pre-fix walker hit `frame_tail > claimed_end` for the frame that
    // CONTAINS the hint and returned that frame's START — silently dropping that
    // CRC-valid frame and all later ones. With the hint discarded and file_len the
    // only bound, the walk decodes that frame cleanly and recovers EVERY frame.
    let (bytes, real_frames_end) = frames_then_sdx3_footer(&["alpha", "beta", "gamma"]);
    let file_len = bytes.len() as u64;
    // Compute the start of the THIRD frame, then point the hint a few bytes INTO
    // it (mid-header / mid-payload, never a frame boundary).
    let second_start = frame_total_len_at(&bytes, 0);
    let third_start = second_start + frame_total_len_at(&bytes, second_start);
    let mid_frame_hint = third_start + 3; // strictly inside the third frame
    assert!(
        mid_frame_hint > third_start && mid_frame_hint < real_frames_end,
        "hint must land strictly inside a later CRC-valid frame, not at a boundary"
    );
    let mut cursor = Cursor::new(bytes);
    let recovered = crc_valid_frames_end(&mut cursor, 0, file_len, 7).expect("must not error");
    assert_eq!(
        recovered, real_frames_end,
        "PROPERTY: a mid-frame untrusted hint must NOT drop the containing CRC-valid frame; all \
         frames recover (this is the round-4 P1)"
    );
}

#[test]
fn crc_valid_frames_end_stops_at_first_non_frame_byte() {
    // The walk stops at the first byte that does not decode as a CRC-valid frame
    // (footer / corruption), never admitting non-CRC-valid bytes. With an honest
    // footer the natural stop is exactly the true frame end.
    let (bytes, real_frames_end) = frames_then_sdx3_footer(&["p", "q"]);
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let recovered = crc_valid_frames_end(&mut cursor, 0, file_len, 7).expect("must not error");
    assert_eq!(
        recovered, real_frames_end,
        "PROPERTY: the walk stops at the true frame end (footer bytes never decode as a frame)"
    );
}

#[test]
fn crc_valid_frames_end_fails_closed_on_mid_stream_corruption() {
    // ROUND-5 P1 (unit-level): `[valid][valid][CORRUPT][valid][valid][footer]`. The
    // walk reaches the corrupt frame at P (CRC fails -> non-decodable), then the
    // look-ahead resync from P+1 finds the CRC-valid frames that FOLLOW. A
    // non-decodable position with valid frames after it is MID-STREAM corruption,
    // not the true frame end, so recovery must FailClosed (CorruptSegment) — NOT
    // silently truncate to the `[valid][valid]` prefix as the round-4 code did.
    let (mut bytes, frames_end) = frames_then_sdx3_footer(&["a", "b", "c", "d", "e"]);
    // Corrupt the THIRD frame's payload (interior), leaving frames 4/5 CRC-valid.
    let f2_start = frame_total_len_at(&bytes, 0);
    let f3_start = f2_start + frame_total_len_at(&bytes, f2_start);
    let third_payload_byte = usize::try_from(f3_start + 8).expect("offset fits usize");
    assert!(
        (third_payload_byte as u64) < frames_end,
        "corruption target must be inside the frame region"
    );
    bytes[third_payload_byte] ^= 0x01; // break only the third frame's CRC
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let result = crc_valid_frames_end(&mut cursor, 0, file_len, 7);
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY: mid-stream corruption (CRC-valid frames after the first bad frame) must \
         FailClosed with CorruptSegment, not silently recover the prefix; got {result:?}"
    );
}

#[test]
fn crc_valid_frames_end_recovers_prefix_for_torn_last_frame() {
    // ROUND-5 composition (unit-level): a genuinely torn LAST frame with NOTHING
    // CRC-valid after it. The walk recovers `[valid][valid]`, reaches the torn
    // frame at P, the resync look-ahead finds no valid frame between P and EOF, so
    // P is the true frame-region end (torn tail) — recover the prefix, do NOT
    // FailClosed. This proves the fix composes with torn-tail handling.
    let (bytes, _frames_end) = frames_then_sdx3_footer(&["a", "b", "c"]);
    // Find the start of the THIRD (last real) frame, then keep only its header + a
    // single payload byte so it can never decode (truncated) and append NO footer —
    // nothing CRC-valid follows the tear.
    let f2_start = frame_total_len_at(&bytes, 0);
    let f3_start = f2_start + frame_total_len_at(&bytes, f2_start);
    let prefix_end = usize::try_from(f3_start + 8 + 1).expect("offset fits usize");
    let torn = bytes[..prefix_end].to_vec();
    let file_len = torn.len() as u64;
    let mut cursor = Cursor::new(torn);
    let recovered =
        crc_valid_frames_end(&mut cursor, 0, file_len, 7).expect("torn tail must not error");
    assert_eq!(
        recovered, f3_start,
        "PROPERTY: a torn last frame with nothing valid after it recovers the clean prefix \
         (ends at the start of the torn frame), not FailClosed"
    );
}

/// Build a minimal in-memory buffer whose last 16 bytes are a SIDX trailer with
/// the given `string_table_offset`, valid magic, and zero entry_count. The body
/// is zero-filled, so the footer CRC cannot authenticate — the footer is always
/// UNTRUSTED. Mirrors the helper that used to live in `segment/mod.rs`.
fn sidx_trailer_buf(total_len: usize, string_table_offset: u64) -> Vec<u8> {
    assert!(total_len >= 16, "buffer must hold the 16-byte trailer");
    let mut bytes = vec![0u8; total_len];
    let trailer_start = total_len - 16;
    bytes[trailer_start..trailer_start + 8].copy_from_slice(&string_table_offset.to_le_bytes());
    bytes[trailer_start + 8..trailer_start + 12].copy_from_slice(&0u32.to_le_bytes());
    bytes[trailer_start + 12..trailer_start + 16]
        .copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    bytes
}

#[test]
fn detect_sidx_boundary_untrusted_offset_past_trailer_does_not_error() {
    // ROUND-6 P1: a synthetic SDX3 trailer with an offset PAST file_len - 16 but
    // NO CRC-valid footer behind it (zero-filled body, so the footer CRC cannot
    // authenticate). Trust is determined FIRST: the offset is UNTRUSTED, so it is
    // FULLY INERT — it must NOT hard-error as CorruptSegment even though it is out
    // of bounds. The boundary is reported untrusted so callers discard the garbage
    // offset and recover via crc_valid_frames_end. (Pre-round-6 code ran the
    // upper-bound check before trust and returned CorruptSegment here, bricking
    // cold start / compaction.)
    let bytes = sidx_trailer_buf(64, 63); // offset past file_len - 16 = 48
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let result = detect_sidx_boundary(&mut cursor, file_len, 7).expect(
        "PROPERTY: an out-of-bounds UNTRUSTED offset must NOT error — it must downgrade to the \
         CRC-valid-frame recovery scan",
    );
    assert_eq!(
        result,
        Some(SidxBoundary {
            frames_end: 63,
            trusted: false,
        }),
        "PROPERTY: an out-of-bounds offset on an unauthenticated footer is recognized as an \
         untrusted boundary (offset discarded by callers), never a hard error; got {result:?}"
    );
}

#[test]
fn detect_sidx_boundary_synthetic_footer_is_never_trusted() {
    // A synthetic (un-CRC'd) footer at the exact upper bound is recognized as a
    // boundary but flagged untrusted — proving the upper-bound hard-error path is
    // unreachable for any unauthenticated footer (the gate is `trusted`-only).
    let file_len = 64u64;
    let max_offset = file_len - 16;
    let bytes = sidx_trailer_buf(
        usize::try_from(file_len).expect("file_len fits usize"),
        max_offset,
    );
    let mut cursor = Cursor::new(bytes);
    let result = detect_sidx_boundary(&mut cursor, file_len, 7).expect("must not error");
    assert_eq!(
        result.map(|b| b.trusted),
        Some(false),
        "PROPERTY: a synthetic (un-CRC'd) footer is never trusted"
    );
}

#[test]
fn detect_sidx_boundary_untrusted_out_of_bounds_offset_is_inert() {
    // ROUND-6 P1 (unit-level): a CRC-failed SDX3 footer whose `string_table_offset`
    // is forged OUT OF BOUNDS (> file_len - 16). detect_sidx_boundary must
    // determine trust FIRST and treat the out-of-bounds untrusted offset as FULLY
    // INERT — returning an untrusted boundary, NOT a CorruptSegment hard error.
    // (The upper-bound check is gated on `trusted` so it can never fire on an
    // unauthenticated footer.)
    let (mut bytes, _frames_end) = frames_then_sdx3_footer(&["a", "b", "c"]);
    let n = bytes.len() as u64;
    let off_pos = bytes.len() - 16;
    // Forge offset = file_len (strictly past the file_len - 16 upper bound). This
    // also breaks the footer CRC, so trust fails -> untrusted.
    bytes[off_pos..off_pos + 8].copy_from_slice(&n.to_le_bytes());
    let mut cursor = Cursor::new(bytes);
    let boundary = detect_sidx_boundary(&mut cursor, n, 7).expect(
        "PROPERTY: an out-of-bounds offset on an UNTRUSTED footer must NOT hard-error — it must \
         return an untrusted boundary so the caller falls back to crc_valid_frames_end",
    );
    assert_eq!(
        boundary.map(|b| b.trusted),
        Some(false),
        "PROPERTY: an out-of-bounds unauthenticated footer is an untrusted boundary, never a hard \
         error; got {boundary:?}"
    );
}

#[test]
fn crc_valid_frames_end_recovers_all_frames_for_adversarial_untrusted_offsets() {
    // PROPERTY (round-6, unit-level): the untrusted-recovery walker ignores the
    // offset entirely and recovers ALL CRC-valid frames regardless of what
    // adversarial value the (discarded) hint held. The walker is bounded only by
    // `file_len`, so the hint's value cannot change its result. We assert the
    // walker returns the true frame end for an intact frame region; combined with
    // detect_sidx_boundary marking these footers untrusted, no offset value can
    // ever error or drop a frame.
    let (bytes, real_frames_end) = frames_then_sdx3_footer(&["alpha", "beta", "gamma", "delta"]);
    let file_len = bytes.len() as u64;
    // The walker takes `frames_start` and `file_len`, never the offset — so any
    // adversarial offset is irrelevant by construction. Run it and confirm it lands
    // exactly on the true frame end (all four frames recovered).
    let mut cursor = Cursor::new(bytes);
    let recovered = crc_valid_frames_end(&mut cursor, 0, file_len, 7)
        .expect("PROPERTY: the untrusted walker never errors over intact CRC-valid frames");
    assert_eq!(
        recovered, real_frames_end,
        "PROPERTY: the untrusted-recovery walker recovers ALL CRC-valid frames bounded only by \
         file_len, independent of any (discarded) offset hint"
    );
}

#[test]
fn append_frames_from_segment_fails_closed_for_untrusted_too_low_offset_no_corroboration() {
    // Merge-compaction copy path: a sealed source segment whose SIDX trailer has a
    // forged (unauthenticated) string_table_offset pointing BELOW the frame region
    // (offset 0) AND which carries ZERO entries, so the manifest cannot corroborate
    // the recovered frame. The forged offset breaks footer CRC authentication, so
    // the boundary is UNTRUSTED and the offset is (correctly) discarded — but with
    // the round-7 durability feature completed, the compaction copy is STRICT
    // (`fallback_fail_closed = true`) and a NON-EMPTY recovered prefix under an
    // untrusted footer with no corroboration is an unprovable tail: a torn/truncated
    // further committed frame cannot be ruled out, so the copy REFUSES rather than
    // merging a source it cannot prove complete.
    //
    // (Before the FailClosed flag had teeth this recovered-what-was-found; that old
    // expectation predates the completed feature. In production a real sealed source
    // carries a full SIDX entry table that corroborates its frames and still
    // recovers via case b — only a bare/garbage footer with no entry table, as
    // synthesized here, reaches this strict refusal.)
    use std::io::Write as _;

    let dir = TempDir::new().expect("tmpdir");

    // Source segment with one real frame.
    let mut source: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 1, 0, &realfs()).expect("create source");
    let frame =
        frame_encode(&serde_json::json!({"payload": "compaction-recover"})).expect("encode frame");
    source.write_frame(&frame).expect("write frame");
    let source_path = source.path.clone();
    source
        .sync_with_mode(&crate::store::SyncMode::default())
        .expect("sync source");
    drop(source);

    // Append a 16-byte SIDX trailer whose string_table_offset is 0 — well below
    // frames_start (8 + header_len). Valid magic so it is recognized as a boundary,
    // but the forged offset fails CRC auth → UNTRUSTED → discarded.
    let mut trailer = [0u8; 16];
    trailer[0..8].copy_from_slice(&0u64.to_le_bytes());
    trailer[8..12].copy_from_slice(&0u32.to_le_bytes());
    trailer[12..16].copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&source_path)
        .expect("open source for trailer append");
    f.write_all(&trailer).expect("append forged trailer");
    drop(f);

    // Destination segment for the compaction copy.
    let mut dest: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 2, 0, &realfs()).expect("create dest");
    let dest_header_bytes = dest.written_bytes;
    let result = dest.append_frames_from_segment(&source_path);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { detail, .. })
            if detail.contains("unprovable tail")
        ),
        "PROPERTY: the strict compaction copy must FAIL CLOSED on an unprovable non-empty tail \
         under an untrusted footer with no corroborating manifest; got {result:?}"
    );
    assert_eq!(
        dest.written_bytes, dest_header_bytes,
        "a refused compaction copy must not append any source bytes to the destination"
    );
}

#[test]
fn append_frames_from_segment_fails_closed_for_untrusted_out_of_bounds_offset_no_corroboration() {
    // ROUND-6 P1 → ROUND-7 completed (compaction copy): a sealed source segment
    // whose SIDX trailer has a forged (unauthenticated) string_table_offset that is
    // OUT OF BOUNDS (set to file_len, strictly past file_len - 16) AND which carries
    // ZERO entries. The forged offset breaks footer CRC authentication -> UNTRUSTED,
    // and it stays FULLY INERT (never used to bound the copy — that round-6 property
    // is preserved). But the compaction copy is STRICT (`fallback_fail_closed =
    // true`), so with a NON-EMPTY recovered prefix and no corroborating manifest the
    // copy REFUSES the unprovable tail rather than merging a source it cannot prove
    // complete. (This supersedes the round-6 "always recover-what-was-found"
    // expectation now that the FailClosed flag has teeth; a real sealed source with
    // a full entry table still recovers via corroboration, case b.)
    use std::io::Write as _;

    let dir = TempDir::new().expect("tmpdir");

    let mut source: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 1, 0, &realfs()).expect("create source");
    let frame = frame_encode(&serde_json::json!({"payload": "compaction-oob-recover"}))
        .expect("encode frame");
    source.write_frame(&frame).expect("write frame");
    let source_path = source.path.clone();
    source
        .sync_with_mode(&crate::store::SyncMode::default())
        .expect("sync source");
    drop(source);

    // Append a 16-byte SIDX trailer whose string_table_offset is set to a value
    // strictly past file_len - 16 (out of bounds). We do not yet know file_len, so
    // append the trailer with a placeholder offset, then compute file_len and
    // rewrite the offset to file_len (guaranteed > file_len - 16). Valid magic so it
    // is recognized as a boundary; the forged offset fails CRC auth -> UNTRUSTED.
    let mut trailer = [0u8; 16];
    trailer[8..12].copy_from_slice(&0u32.to_le_bytes());
    trailer[12..16].copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    let file_len = {
        use std::io::Seek as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&source_path)
            .expect("open source for trailer append");
        f.write_all(&trailer).expect("append forged trailer");
        // Determine the new file length via a seek-to-end (no fs::metadata, which
        // the structural-check ratchet forbids in store-layer code).
        f.seek(SeekFrom::End(0)).expect("seek to end for file_len")
    };
    // Now set the offset field to file_len (out of bounds) in place.
    {
        use std::io::Seek as _;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&source_path)
            .expect("open source to rewrite offset");
        f.seek(SeekFrom::End(-16)).expect("seek to trailer offset");
        f.write_all(&file_len.to_le_bytes())
            .expect("write out-of-bounds offset");
    }

    let mut dest: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 2, 0, &realfs()).expect("create dest");
    let dest_header_bytes = dest.written_bytes;
    let result = dest.append_frames_from_segment(&source_path);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { detail, .. })
            if detail.contains("unprovable tail")
        ),
        "PROPERTY: the strict compaction copy must FAIL CLOSED on an unprovable non-empty tail \
         under an out-of-bounds untrusted footer with no corroborating manifest; got {result:?}"
    );
    assert_eq!(
        dest.written_bytes, dest_header_bytes,
        "a refused compaction copy must not append any source bytes to the destination"
    );
}

#[test]
fn append_frames_from_segment_accepts_trusted_empty_frame_region() {
    // Compaction copy of a sealed source with a CRC-valid SDX3 footer over an
    // EMPTY frame region: the authenticated `string_table_offset` lands exactly
    // at `frames_start` (8 + header_len) because no frames precede it. The
    // lower-bound guard is `frames_end < frames_start`, so frames_end ==
    // frames_start must stay VALID (no error). A `<` -> `==` mutant would treat
    // this legitimate empty segment as corrupt and reject it, so this asserts the
    // boundary is inclusive of the empty case.
    use std::io::{Seek as _, Write as _};

    let dir = TempDir::new().expect("tmpdir");

    // Empty source segment: header only, zero frames. written_bytes == frames_start.
    let source: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 1, 0, &realfs()).expect("create source");
    let source_path = source.path.clone();
    let frames_start = source.written_bytes;
    drop(source);

    // Append a CRC-valid SDX3 footer for an EMPTY collector. write_footer records
    // string_table_offset = current stream position = frames_start (no frames),
    // and the CRC authenticates that offset -> the boundary is TRUSTED with
    // frames_end == frames_start.
    {
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&source_path)
            .expect("open empty source to append footer");
        f.seek(SeekFrom::End(0)).expect("seek to footer start");
        let position = f.stream_position().expect("footer position");
        assert_eq!(
            position, frames_start,
            "fixture sanity: an empty segment's footer must begin exactly at frames_start"
        );
        crate::store::segment::sidx::SidxEntryCollector::new()
            .write_footer(&mut f, 1)
            .expect("write empty CRC-valid footer");
        f.flush().expect("flush footer");
    }

    // Sanity: the boundary detector must report a TRUSTED footer whose frames_end
    // equals frames_start, which is exactly the input the guard must NOT reject.
    {
        let mut probe = std::fs::OpenOptions::new()
            .read(true)
            .open(&source_path)
            .expect("open source for boundary probe");
        let file_len = probe.seek(SeekFrom::End(0)).expect("file_len");
        let boundary = detect_sidx_boundary(&mut probe, file_len, 1)
            .expect("boundary detect must not error")
            .expect("CRC-valid SDX3 footer is a boundary");
        assert_eq!(
            boundary,
            SidxBoundary {
                frames_end: frames_start,
                trusted: true,
            },
            "fixture sanity: empty trusted footer reports frames_end == frames_start"
        );
    }

    let mut dest: Segment<Active> =
        Segment::create_with_created_ns_on(dir.path(), 2, 0, &realfs()).expect("create dest");
    dest.append_frames_from_segment(&source_path).expect(
        "PROPERTY: a trusted EMPTY frame region (frames_end == frames_start) must copy cleanly — \
         the lower-bound guard `frames_end < frames_start` must NOT reject the inclusive boundary",
    );
}

#[test]
fn try_decode_frame_at_admits_a_frame_ending_exactly_at_file_len() {
    // The frame-tail bound is EXCLUSIVE of nothing: a CRC-valid frame whose
    // tail lands EXACTLY on `file_len` is the common last frame of a segment
    // and must decode; only a tail strictly PAST `file_len` is "no frame
    // here". The `>` -> `==` mutant inverts both sides of this boundary and
    // fails both assertions below.
    let frame = frame_encode(&serde_json::json!({ "payload": "exact-tail" })).expect("encode");
    let file_len = frame.len() as u64;
    let mut source = Cursor::new(frame);

    let admitted =
        try_decode_frame_at(&mut source, 0, file_len).expect("in-memory probe cannot fail seek");
    assert_eq!(
        admitted,
        Some(file_len),
        "a CRC-valid frame whose tail lands exactly on file_len is a real frame"
    );

    // Same bytes, but the authoritative bound now excludes the final byte:
    // the tail extends past `file_len`, so no frame may be admitted — even
    // though the in-memory source still physically holds the whole frame and
    // the read WOULD succeed if the bound were ignored.
    let admitted_past_bound = try_decode_frame_at(&mut source, 0, file_len - 1)
        .expect("in-memory probe cannot fail seek");
    assert_eq!(
        admitted_past_bound, None,
        "a frame tail strictly past file_len must be rejected by the bound, not by a short read"
    );
}

#[test]
fn frame_decode_admits_an_eight_byte_empty_payload_frame() {
    // The minimum valid frame is EXACTLY 8 bytes: [len=0 BE][crc32("")=0 BE] with
    // a zero-length payload. frame_decode must accept it (msgpack = empty slice,
    // consumed = 8). The `buf.len() < 8 -> <= 8` mutant rejects a buffer of exactly
    // 8 bytes as TooShort, so it can never decode the minimal empty-payload frame.
    let mut frame = Vec::with_capacity(8);
    frame.extend_from_slice(&0u32.to_be_bytes()); // len = 0
    frame.extend_from_slice(&0u32.to_be_bytes()); // crc32 of an empty payload = 0
    assert_eq!(
        frame.len(),
        8,
        "fixture must be exactly the 8-byte frame header"
    );

    let (msgpack, consumed) = frame_decode(&frame)
        .expect("an 8-byte [len=0][crc=0] buffer is the minimal valid empty-payload frame");
    assert!(
        msgpack.is_empty(),
        "a zero-length frame decodes to an empty payload slice"
    );
    assert_eq!(consumed, 8, "the whole 8-byte frame is consumed");
}

#[test]
fn detect_sidx_boundary_recognizes_a_file_that_is_exactly_the_trailer() {
    // A file of EXACTLY 16 bytes (TRAILER_LEN) is a bare SIDX trailer and must be
    // recognized as a boundary — an UNTRUSTED one, since 16 bytes leave no room for
    // a CRC-covered footer body to authenticate. The `file_len < 16 -> <= 16` mutant
    // treats file_len == 16 as "no footer" and returns Ok(None), losing the boundary.
    let bytes = sidx_trailer_buf(16, 0);
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let boundary = detect_sidx_boundary(&mut cursor, file_len, 7)
        .expect("a 16-byte trailer file must not error");
    assert_eq!(
        boundary,
        Some(SidxBoundary {
            frames_end: 0,
            trusted: false,
        }),
        "a file that is exactly the 16-byte trailer is an untrusted boundary, not 'no footer'"
    );
}

#[test]
fn try_decode_frame_at_reads_the_header_when_exactly_eight_bytes_remain() {
    // With EXACTLY 8 bytes between `at` and file_len there is room for a frame
    // header but no payload, so try_decode_frame_at seeks + reads the 8-byte header
    // (then returns None, because no non-empty frame fits). The `remaining < 8 ->
    // <= 8` mutant short-circuits at remaining == 8 and returns None WITHOUT
    // consuming the source; pinning the post-call cursor at at+8 convicts it.
    let mut source = Cursor::new(vec![0u8; 8]);
    source
        .seek(SeekFrom::Start(3))
        .expect("park the cursor before the call");
    let decoded =
        try_decode_frame_at(&mut source, 0, 8).expect("in-memory probe cannot fail its seek");
    assert_eq!(
        decoded, None,
        "8 bytes hold only a header, no non-empty frame"
    );
    assert_eq!(
        source.position(),
        8,
        "the header at `at` must be seeked-to and read (cursor at at+8), not short-circuited"
    );
}

#[test]
fn try_decode_frame_at_short_circuits_below_a_header_without_consuming() {
    // With FEWER than 8 bytes remaining there cannot even be a frame header, so the
    // guard returns None as a cheap pre-check WITHOUT seeking or reading. The
    // `remaining < 8 -> == 8` mutant only short-circuits at remaining == 8, so for a
    // 4-byte remainder it falls through, seeks to `at`, and consumes the source.
    // Pinning the parked cursor convicts it.
    let mut source = Cursor::new(vec![0u8; 4]);
    source
        .seek(SeekFrom::Start(2))
        .expect("park the cursor before the call");
    let decoded =
        try_decode_frame_at(&mut source, 0, 4).expect("in-memory probe cannot fail its seek");
    assert_eq!(
        decoded, None,
        "fewer than 8 bytes cannot hold a frame header"
    );
    assert_eq!(
        source.position(),
        2,
        "the sub-header guard must not seek or read — the cursor stays parked"
    );
}

/// A [`StoreFs`] that fails `sync_file_with_mode` with a sentinel error; every
/// other op delegates to [`RealFs`] so segment creation still succeeds. Used to
/// prove `Segment::sync_with_mode` PROPAGATES a backend sync failure rather than
/// swallowing it — the `-> Ok(())` mutant would silently report durability that
/// never happened.
struct SyncFailFs;

impl crate::store::platform::fs::StoreFs for SyncFailFs {
    fn read_dir(&self, path: &std::path::Path) -> std::io::Result<std::fs::ReadDir> {
        crate::store::platform::fs::RealFs.read_dir(path)
    }
    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        crate::store::platform::fs::RealFs.read(path)
    }
    fn create_dir_all(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::store::platform::fs::RealFs.create_dir_all(path)
    }
    fn create_new_file(
        &self,
        path: &std::path::Path,
    ) -> Result<std::fs::File, crate::store::StoreError> {
        crate::store::platform::fs::RealFs.create_new_file(path)
    }
    fn sync_file_with_mode(
        &self,
        _file: &std::fs::File,
        _path: &std::path::Path,
        _mode: &crate::store::SyncMode,
    ) -> Result<(), crate::store::StoreError> {
        Err(crate::store::StoreError::corrupt_segment_with_detail(
            4242,
            "injected: sync_file_with_mode failed",
        ))
    }
    fn sync_file_all(&self, file: &std::fs::File, path: &std::path::Path) -> std::io::Result<()> {
        crate::store::platform::fs::RealFs.sync_file_all(file, path)
    }
    fn sync_parent_dir(&self, path: &std::path::Path) -> Result<(), crate::store::StoreError> {
        crate::store::platform::fs::RealFs.sync_parent_dir(path)
    }
    fn reject_symlink_leaf(
        &self,
        path: &std::path::Path,
        purpose: &str,
    ) -> Result<(), crate::store::StoreError> {
        crate::store::platform::fs::RealFs.reject_symlink_leaf(path, purpose)
    }
    fn canonicalize(&self, path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        crate::store::platform::fs::RealFs.canonicalize(path)
    }
    fn symlink_metadata(&self, path: &std::path::Path) -> std::io::Result<std::fs::Metadata> {
        crate::store::platform::fs::RealFs.symlink_metadata(path)
    }
    fn cow_copy_file(
        &self,
        from: &std::path::Path,
        to: &std::path::Path,
        preference: crate::store::CopyPreference,
    ) -> std::io::Result<crate::store::platform::fs::CowStrategyUsed> {
        crate::store::platform::fs::RealFs.cow_copy_file(from, to, preference)
    }
    fn copy(&self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<u64> {
        crate::store::platform::fs::RealFs.copy(from, to)
    }
    fn metadata(&self, path: &std::path::Path) -> std::io::Result<std::fs::Metadata> {
        crate::store::platform::fs::RealFs.metadata(path)
    }
    fn rename(&self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        crate::store::platform::fs::RealFs.rename(from, to)
    }
    fn remove_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::store::platform::fs::RealFs.remove_file(path)
    }
    fn named_temp_in(&self, dir: &std::path::Path) -> std::io::Result<tempfile::NamedTempFile> {
        crate::store::platform::fs::RealFs.named_temp_in(dir)
    }
    fn persist_temp_with_parent_sync(
        &self,
        named_temp: tempfile::NamedTempFile,
        final_path: &std::path::Path,
        admission: crate::store::platform::sync::ParentDirSyncAdmission,
    ) -> std::io::Result<()> {
        crate::store::platform::fs::RealFs
            .persist_temp_with_parent_sync(named_temp, final_path, admission)
    }
    fn read_exact_at(
        &self,
        file: &mut std::fs::File,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<(), crate::store::platform::fs::PositionedReadError> {
        crate::store::platform::fs::RealFs.read_exact_at(file, offset, buf)
    }
}

#[test]
fn sync_with_mode_propagates_the_backend_sync_failure() {
    // Segment::sync_with_mode routes durability through the StoreFs seam. When the
    // backend's sync_file_with_mode fails, the failure MUST propagate — a segment
    // that swallows a failed fsync would report durability that never happened. The
    // `-> Ok(())` mutant drops the fs call entirely and returns success.
    let dir = TempDir::new().expect("tmpdir");
    let fs: std::sync::Arc<dyn crate::store::platform::fs::StoreFs> =
        std::sync::Arc::new(SyncFailFs);
    let mut segment =
        Segment::create_with_created_ns_on(dir.path(), 9, 0, &fs).expect("create over SyncFailFs");

    let err = segment
        .sync_with_mode(&crate::store::SyncMode::default())
        .expect_err("a failing backend sync must surface as an error, not be swallowed");
    assert!(
        matches!(
            err,
            StoreError::CorruptSegment { segment_id: 4242, ref detail }
            if detail.contains("injected: sync_file_with_mode failed")
        ),
        "sync_with_mode must propagate the exact backend sync failure; got {err:?}"
    );
}

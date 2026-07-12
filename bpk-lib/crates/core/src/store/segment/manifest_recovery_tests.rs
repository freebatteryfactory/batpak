//! Untrusted-footer recovery tests: the SIDX entry table as an UNTRUSTED
//! SUSPICION SIGNAL — never an authority (GAUNT-SIDX-NO-SELF-AUTH, #192).
//!
//! PROVES: INV-UNTRUSTED-MANIFEST-NO-AUTHORITY-ESCALATION.
//!
//! Extracted from `boundary_tests.rs` to keep each inline test file within the
//! structural file-size budget. These pin the untrusted-footer
//! recover-vs-fail-closed decision under THE SAFE LAW: a row corroborated
//! against independently verified frame bytes proves only that row's
//! relationship to that frame — it grants no authority over sibling rows,
//! `entry_count`, order, the footer boundary, or claims about nonexistent
//! trailing frames. BLAKE3 is public and unkeyed, so a stored `event_hash` can
//! be copied by anyone who can read or edit the segment; row hashes therefore
//! contribute NOTHING to the decision.
//!
//! Decision under test (untrusted boundary only):
//! - an EMPTY recovered prefix recovers under any posture (nothing to lose);
//! - permissive (`RecoverTornTail` → false, the default) recovers the CRC-valid
//!   prefix, recording truncation evidence when an untrusted claim (footer
//!   claimed end, or a SIDX row at/after the prefix end) reaches past it;
//! - strict (`FailClosed` → true) refuses ANY non-empty prefix — with a
//!   positive-suspicion detail when an untrusted claim reaches past the prefix,
//!   else as an unprovable tail. Mid-stream corruption still fails closed first.

use super::*;
use crate::event::{EventHeader, EventKind, HashChain};
use crate::store::segment::sidx::{kind_to_raw, read_entries_unauthenticated, SidxEntryCollector};
use std::io::Cursor;

/// Build an in-memory buffer of `[real CRC-valid frames][CRC-valid SDX3 footer]`.
/// Mirror of the helper in `boundary_tests.rs`, used by the mid-stream-corruption
/// regression below. Returns `(bytes, frames_end)`.
fn frames_then_sdx3_footer(payloads: &[&str]) -> (Vec<u8>, u64) {
    use crate::store::segment::sidx::SidxEntry;

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
            kind: kind_to_raw(EventKind::custom(0x1, 1)),
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

/// Total on-disk size of the frame beginning at `offset` in `bytes`.
fn frame_total_len_at(bytes: &[u8], offset: u64) -> u64 {
    let start = usize::try_from(offset).expect("offset fits usize");
    let header: [u8; 4] = bytes[start..start + 4]
        .try_into()
        .expect("4-byte frame length prefix");
    let payload_len = u64::from(u32::from_be_bytes(header));
    8 + payload_len
}

/// Build one real CRC-valid frame whose `FramePayload` carries a `hash_chain`
/// with `event_hash = blake3(payload_bytes)` (the exact writer invariant), plus
/// the matching [`crate::store::segment::sidx::SidxEntry`] (same offset, length,
/// and content event_hash). Returns `(frame_bytes, entry)`.
fn corroboratable_frame(
    frame_offset: u64,
    seq: u64,
    payload: &serde_json::Value,
) -> (Vec<u8>, crate::store::segment::sidx::SidxEntry) {
    let payload_bytes = crate::encoding::to_bytes(payload).expect("encode payload");
    let event_hash = crate::event::hash::compute_hash(&payload_bytes);
    let header = EventHeader::new(
        seq as u128,
        seq as u128,
        None,
        0,
        crate::coordinate::DagPosition::root(),
        u32::try_from(payload_bytes.len()).expect("payload size fits u32"),
        EventKind::custom(0x1, 1),
    );
    let event = crate::event::Event {
        header,
        payload: payload_bytes,
        hash_chain: Some(HashChain {
            prev_hash: [0; 32],
            event_hash,
        }),
    };
    let frame_payload = FramePayload {
        event,
        entity: "entity:test".to_owned(),
        scope: "scope:test".to_owned(),
        receipt_extensions: std::collections::BTreeMap::new(),
    };
    let frame = frame_encode(&frame_payload).expect("encode frame");
    let frame_length = u32::try_from(frame.len()).expect("frame length fits u32");
    let entry = crate::store::segment::sidx::SidxEntry {
        event_id: seq as u128,
        entity_idx: 0,
        scope_idx: 0,
        kind: kind_to_raw(EventKind::custom(0x1, 1)),
        wall_ms: 1,
        clock: 1,
        dag_lane: 0,
        dag_depth: 0,
        prev_hash: [0; 32],
        event_hash,
        frame_offset,
        frame_length,
        global_sequence: seq,
        correlation_id: seq as u128,
        causation_id: 0,
    };
    (frame, entry)
}

/// Append an SDX3 footer recording `entries` to `bytes`, then flip a byte inside
/// the footer's covered region so the footer CRC fails → UNTRUSTED. The 16-byte
/// trailer geometry (offset/count/magic) is left intact, so the entry table stays
/// parseable by `read_entries_unauthenticated` while the footer CRC no longer
/// authenticates the boundary. Returns the footer-body byte offset that was
/// corrupted (= the footer start, the entries' recorded frames_end).
fn append_untrusted_footer(
    bytes: &mut Vec<u8>,
    entries: &[crate::store::segment::sidx::SidxEntry],
) -> u64 {
    let footer_start = bytes.len() as u64;
    let mut collector = SidxEntryCollector::new();
    for entry in entries.iter().cloned() {
        collector
            .record(entry, "entity:test", "scope:test")
            .expect("intern test strings");
    }
    let mut cursor = Cursor::new(&mut *bytes);
    cursor.seek(SeekFrom::End(0)).expect("seek to footer start");
    collector
        .write_footer(&mut cursor, 7)
        .expect("write footer");
    // Corrupt one byte of the footer string-table/entries region (the byte right
    // at footer_start) to break the footer CRC without touching the trailer
    // geometry. This is exactly how `detect_sidx_boundary_distrusts_a_crc_failed_sdx3_footer`
    // produces an untrusted boundary.
    let corrupt_at = usize::try_from(footer_start).expect("footer_start fits usize");
    bytes[corrupt_at] ^= 0xFF;
    footer_start
}

#[test]
fn untrusted_entries_parse_is_crc_independent() {
    // The untrusted entry-table read decodes entries even when the footer CRC has
    // been broken — proving SidxEntry::decode_from is CRC-independent and the
    // table is available as suspicion geometry on a corrupt footer.
    let (mut bytes, e0) = {
        let (f0, e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "a"}));
        (f0, e0)
    };
    let (f1, mut e1) = corroboratable_frame(bytes.len() as u64, 2, &serde_json::json!({"v": "b"}));
    e1.frame_offset = bytes.len() as u64;
    bytes.extend_from_slice(&f1);
    append_untrusted_footer(&mut bytes, &[e0.clone(), e1.clone()]);

    let mut cursor = Cursor::new(bytes);
    let parsed = read_entries_unauthenticated(&mut cursor, 7).expect("must not error");
    assert_eq!(
        parsed.len(),
        2,
        "PROPERTY: entries parse from a CRC-failed footer (decode_from is CRC-independent)"
    );
    assert_eq!(
        parsed[0].event_hash, e0.event_hash,
        "PROPERTY: parsed untrusted entry preserves its fields verbatim (they are hypotheses, \
         reported as-is)"
    );
    assert_eq!(parsed[1].frame_offset, e1.frame_offset);
}

#[test]
fn untrusted_garbage_entry_table_parses_to_zero_entries() {
    // A trailer with absurd entry_count must yield ZERO entries (geometry guard),
    // forcing the fall-back path — never a partial/forged manifest.
    let mut bytes = vec![0xA5u8; 200];
    let trailer_start = bytes.len() - 16;
    bytes[trailer_start..trailer_start + 8].copy_from_slice(&0u64.to_le_bytes());
    // entry_count = u32::MAX → entries_block_len underflows entries_start → zero.
    bytes[trailer_start + 8..trailer_start + 12].copy_from_slice(&u32::MAX.to_le_bytes());
    bytes[trailer_start + 12..trailer_start + 16]
        .copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC);

    let mut cursor = Cursor::new(bytes);
    let parsed = read_entries_unauthenticated(&mut cursor, 7).expect("must not error");
    assert!(
        parsed.is_empty(),
        "PROPERTY: an absurd entry_count yields zero entries (fall back), never a forged manifest"
    );
}

/// Build the torn-last-committed-frame fixture: two intact corroboratable frames,
/// a third committed frame written TORN (header + 1 byte, can never decode), and
/// an untrusted footer whose manifest records all THREE frames. Returns
/// `(bytes, f2_off, e2_frame_length)` where `f2_off` is the torn frame's offset
/// (== the recovery stop P) and `e2_frame_length` is the manifest row's claimed
/// length for it.
fn torn_last_committed_frame_fixture() -> (Vec<u8>, u64, u32) {
    let (f0, mut e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "first"}));
    e0.frame_offset = 0;
    let mut bytes = f0;

    let f1_off = bytes.len() as u64;
    let (f1, mut e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "second"}));
    e1.frame_offset = f1_off;
    bytes.extend_from_slice(&f1);

    // The (committed) third frame: build it fully to compute its real length +
    // event_hash for the entry, but write only its header + a single payload byte
    // so it can NEVER decode (torn). Its recorded offset is the end of frame 1,
    // which is exactly the recovery stop P.
    let f2_off = bytes.len() as u64;
    let (f2_full, mut e2) = corroboratable_frame(f2_off, 3, &serde_json::json!({"v": "third"}));
    e2.frame_offset = f2_off;
    let e2_frame_length = e2.frame_length;
    bytes.extend_from_slice(&f2_full[..9]); // 8-byte header + 1 payload byte = torn

    append_untrusted_footer(&mut bytes, &[e0, e1, e2]);
    (bytes, f2_off, e2_frame_length)
}

#[test]
fn resolve_untrusted_torn_last_frame_strict_refuses_permissive_recovers_with_evidence() {
    // A torn LAST committed frame beneath an untrusted footer whose manifest
    // still lists it. The manifest row at P is an UNTRUSTED CLAIM — real here,
    // but byte-indistinguishable from a forged one — so it is SUSPICION routed
    // through the posture, never proof of loss (#192):
    // - STRICT refuses with the positive-truncation-evidence detail;
    // - PERMISSIVE recovers the CRC-valid prefix and RECORDS the evidence.
    let (bytes, f2_off, e2_frame_length) = torn_last_committed_frame_fixture();
    let file_len = bytes.len() as u64;

    // STASH-VERIFY (the round-7 gap, made explicit): the OLD primitive
    // `crc_valid_frames_end` — the manifest-blind recover-the-prefix walk the three
    // sites used before the manifest path existed — returns Ok(P) for these exact
    // bytes, SILENTLY DROPPING the torn committed frame 2 (P == start of frame 2).
    {
        let mut old_cursor = Cursor::new(bytes.clone());
        let old = crc_valid_frames_end(&mut old_cursor, 0, file_len, 7).expect(
            "OLD primitive recovers the prefix (the gap): torn tail has nothing valid after",
        );
        assert_eq!(
            old, f2_off,
            "STASH-VERIFY: the manifest-blind walk recovers the prefix and silently drops the \
             torn committed frame (recovery stop == start of the torn frame 2)"
        );
    }

    // STRICT (the non-tail sealed-segment posture cold_start passes): refuse, and
    // carry the POSITIVE-suspicion detail (the manifest row at P reaches past it).
    let mut cursor = Cursor::new(bytes.clone());
    let result = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, true);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY (#192): strict posture refuses a torn last committed frame under an untrusted \
         footer; got {result:?}"
    );
    let detail = match result {
        Err(StoreError::CorruptSegment { detail, .. }) => detail,
        _ => String::new(),
    };
    assert!(
        detail.contains("POSITIVE truncation evidence"),
        "the manifest row naming the torn frame is positive (if untrusted) suspicion, so the \
         strict refusal must carry the evidence detail; got detail: {detail}"
    );

    // PERMISSIVE (the default): recover the CRC-valid prefix — a torn tail must
    // not brick the newest segment — while RECORDING the truncation evidence so
    // the caller can surface it. The recorded claimed end is the manifest row's
    // (offset + length), the furthest untrusted claim.
    let mut cursor2 = Cursor::new(bytes);
    let resolved = resolve_untrusted_frames_end(&mut cursor2, 0, file_len, 7, None, false)
        .expect("PROPERTY (#192): the permissive posture recovers the prefix, never refuses");
    assert_eq!(
        resolved.frames_end, f2_off,
        "permissive recovery returns the CRC-valid prefix end (the torn frame is dropped WITH \
         recorded evidence, not silently)"
    );
    let evidence = resolved.truncation_evidence.expect(
        "PROPERTY (#192): permissive recovery under a manifest row at/after P must record \
                 truncation evidence",
    );
    assert_eq!(
        evidence.recovered_prefix_end, f2_off,
        "evidence pins the recovered prefix end"
    );
    assert_eq!(
        evidence.footer_claimed_frames_end,
        f2_off + u64::from(e2_frame_length),
        "evidence carries the furthest untrusted claimed end (the manifest row's offset + length)"
    );
}

#[test]
fn resolve_untrusted_intact_frames_corrupt_footer_permissive_recovers_strict_refuses() {
    // An UNTRUSTED (corrupt) footer but ALL committed frames intact and
    // CRC-valid, with no untrusted claim past the prefix. Under the retired law
    // this recovered EVEN UNDER STRICT ("the manifest agrees the segment is
    // complete") — on forgeable evidence. Under the safe law (#192):
    // - PERMISSIVE recovers all frames (benign corrupt footer, nothing dropped,
    //   no truncation evidence);
    // - STRICT refuses: unauthenticated bytes cannot PROVE the tail complete,
    //   so table agreement is not a recovery ticket. This is the deliberate
    //   availability trade recorded in ADR-0036 — a benignly corrupt footer on
    //   a sealed (non-tail) segment now requires operator remediation.
    let (f0, mut e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "a"}));
    e0.frame_offset = 0;
    let mut bytes = f0;
    let f1_off = bytes.len() as u64;
    let (f1, mut e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "b"}));
    e1.frame_offset = f1_off;
    bytes.extend_from_slice(&f1);
    let f2_off = bytes.len() as u64;
    let (f2, mut e2) = corroboratable_frame(f2_off, 3, &serde_json::json!({"v": "c"}));
    e2.frame_offset = f2_off;
    bytes.extend_from_slice(&f2);
    let frames_end = bytes.len() as u64;

    append_untrusted_footer(&mut bytes, &[e0, e1, e2]);
    let file_len = bytes.len() as u64;

    // PERMISSIVE: recover ALL committed frames, no evidence (no claim past P).
    let mut cursor = Cursor::new(bytes.clone());
    let resolved = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, false)
        .expect("intact frames under a corrupt footer recover under the permissive posture");
    assert_eq!(
        resolved.frames_end, frames_end,
        "PROPERTY: the permissive posture recovers ALL committed frames of a benign \
         corrupt-footer segment"
    );
    assert_eq!(
        resolved.truncation_evidence, None,
        "no untrusted claim reaches past the prefix, so no truncation evidence is recorded"
    );

    // STRICT: refuse as an unprovable tail (the case-(b) removal pin — the exact
    // scenario the retired law recovered on forgeable table agreement).
    let mut cursor2 = Cursor::new(bytes);
    let result = resolve_untrusted_frames_end(&mut cursor2, 0, file_len, 7, None, true);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY (#192): strict posture refuses even a fully-agreeing table — unauthenticated \
         agreement proves nothing; got {result:?}"
    );
    let detail = match result {
        Err(StoreError::CorruptSegment { detail, .. }) => detail,
        _ => String::new(),
    };
    assert!(
        detail.contains("unprovable tail"),
        "with no untrusted claim past the prefix the strict refusal is the absence-only \
         unprovable-tail detail; got detail: {detail}"
    );
}

#[test]
fn resolve_untrusted_empty_manifest_permissive_recovers_prefix() {
    // An UNTRUSTED footer whose entry table is EMPTY (zero rows — the
    // unparseable/no-signal posture). With no untrusted claim past the prefix
    // the default permissive posture recovers the CRC-valid prefix — never a
    // false fail-closed on a benign corrupt-footer store. (The STRICT leg of
    // this exact scenario fails closed as an unprovable tail — see
    // `strict_untrusted_footer_nonempty_prefix_refuses_unprovable_tail`.)
    let (f0, _e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "a"}));
    let mut bytes = f0;
    let f1_off = bytes.len() as u64;
    let (f1, _e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "b"}));
    bytes.extend_from_slice(&f1);
    let frames_end = bytes.len() as u64;

    // Footer records ZERO entries (empty manifest) but is corrupted to UNTRUSTED.
    append_untrusted_footer(&mut bytes, &[]);
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let recovered = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, false)
        .expect("an empty manifest must fall back to prefix recovery under permissive")
        .frames_end;
    assert_eq!(
        recovered, frames_end,
        "PROPERTY: with an empty manifest the default permissive posture recovers the CRC-valid \
         prefix (no false fail-closed)"
    );
}

#[test]
fn resolve_untrusted_recovers_for_garbage_entry_table() {
    // Garbage variant under the PERMISSIVE posture: an UNTRUSTED footer whose
    // entry table is unparseable (absurd entry_count).
    // read_entries_unauthenticated returns zero entries → no rows, no claims.
    // Under the default permissive (`RecoverTornTail` → false) posture this falls
    // back to prefix recovery — never a false fail-closed. (Under STRICT the same
    // garbage manifest fails closed as an unprovable tail; that leg is covered by
    // `strict_untrusted_footer_nonempty_prefix_refuses_unprovable_tail`.)
    let (f0, _e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "x"}));
    let mut bytes = f0;
    let f1_off = bytes.len() as u64;
    let (f1, _e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "y"}));
    bytes.extend_from_slice(&f1);
    let frames_end = bytes.len() as u64;

    // Append a 16-byte trailer with valid magic but an absurd entry_count so the
    // geometry guard yields zero entries. (No CRC region behind it → also breaks
    // any authentication; the boundary is untrusted and the manifest is empty.)
    let mut trailer = [0u8; 16];
    trailer[0..8].copy_from_slice(&frames_end.to_le_bytes());
    trailer[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    trailer[12..16].copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    bytes.extend_from_slice(&trailer);

    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let recovered = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, false)
        .expect("a garbage entry table must fall back to prefix recovery under permissive")
        .frames_end;
    assert_eq!(
        recovered, frames_end,
        "PROPERTY: a garbage/unparseable entry table degrades to prefix recovery under the default \
         permissive posture (no false fail-closed)"
    );
}

#[test]
fn resolve_untrusted_legacy_sdx2_intact_frames_permissive_recovers_strict_refuses() {
    // A legacy SDX2 footer (never CRC-authenticated → always UNTRUSTED) over
    // intact frames. The SDX2 entry table is parseable by
    // read_entries_unauthenticated (it accepts both magics); its rows all sit
    // below the recovered prefix end, so there is no suspicion — but there is no
    // authentication either. Permissive recovers all; strict refuses (#192: an
    // un-CRC'd legacy table cannot prove the tail complete).
    let (f0, mut e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "a"}));
    e0.frame_offset = 0;
    let mut bytes = f0;
    let f1_off = bytes.len() as u64;
    let (f1, mut e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "b"}));
    e1.frame_offset = f1_off;
    bytes.extend_from_slice(&f1);
    let frames_end = bytes.len() as u64;

    // Write a real SDX3 footer, then rewrite the trailing magic to legacy SDX2 so
    // the boundary reads as un-CRC'd/untrusted while the entry geometry is intact.
    append_untrusted_footer(&mut bytes, &[e0, e1]);
    let n = bytes.len();
    bytes[n - 4..n].copy_from_slice(crate::store::segment::sidx::SIDX_MAGIC_LEGACY_SDX2);

    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes.clone());
    let recovered = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, false)
        .expect("legacy SDX2 over intact frames recovers under the permissive posture")
        .frames_end;
    assert_eq!(
        recovered, frames_end,
        "PROPERTY: a legacy SDX2 boundary over intact frames recovers them all under permissive"
    );

    let mut cursor2 = Cursor::new(bytes);
    let result = resolve_untrusted_frames_end(&mut cursor2, 0, file_len, 7, None, true);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY (#192): strict posture refuses a legacy un-CRC'd SDX2 tail — it is exactly as \
         unauthenticated as a corrupt SDX3; got {result:?}"
    );
}

#[test]
fn resolve_untrusted_still_fails_closed_on_mid_stream_corruption() {
    // ROUND-5 invariant preserved THROUGH the manifest path: mid-stream corruption
    // (a CRC-valid frame after the first bad frame) must still FAIL CLOSED before
    // the manifest is even consulted. The walk's look-ahead resync fires inside
    // crc_valid_frames_end_counting exactly as in crc_valid_frames_end.
    let (bytes, frames_end) = frames_then_sdx3_footer(&["a", "b", "c", "d", "e"]);
    let mut bytes = bytes;
    let f2_start = frame_total_len_at(&bytes, 0);
    let f3_start = f2_start + frame_total_len_at(&bytes, f2_start);
    let third_payload_byte = usize::try_from(f3_start + 8).expect("offset fits usize");
    assert!((third_payload_byte as u64) < frames_end);
    bytes[third_payload_byte] ^= 0x01; // break the third frame's CRC (interior)
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let result = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, true);
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY: mid-stream corruption still fails closed via the manifest path; got {result:?}"
    );
}

/// Build two intact CRC-valid frames followed by an UNTRUSTED footer that records
/// ZERO SIDX entries (an empty manifest): a non-empty recovered prefix under an
/// untrusted footer with no rows at all. Returns `(bytes, frames_end)`.
fn two_intact_frames_then_untrusted_empty_footer() -> (Vec<u8>, u64) {
    let (f0, _e0) = corroboratable_frame(0, 1, &serde_json::json!({"v": "a"}));
    let mut bytes = f0;
    let f1_off = bytes.len() as u64;
    let (f1, _e1) = corroboratable_frame(f1_off, 2, &serde_json::json!({"v": "b"}));
    bytes.extend_from_slice(&f1);
    let frames_end = bytes.len() as u64;
    // Empty manifest (zero entries), then flip a footer byte → UNTRUSTED.
    append_untrusted_footer(&mut bytes, &[]);
    (bytes, frames_end)
}

#[test]
fn strict_untrusted_footer_nonempty_prefix_refuses_unprovable_tail() {
    // STRICT (`FailClosed` → true) posture + an UNTRUSTED footer + a NON-EMPTY
    // recovered CRC-valid prefix (two intact frames) + no untrusted claim past
    // the prefix → REFUSE as an unprovable tail. The untrusted footer is exactly
    // the thing that would bound the frame region, so a torn/truncated further
    // committed frame past the prefix cannot be ruled out. The refusal detail
    // must carry the full operator evidence set (#192 ruling): segment
    // identity/filename, the verified boundary, the no-authenticated-evidence
    // statement, and remediation guidance that never says delete-in-place.
    let (bytes, frames_end) = two_intact_frames_then_untrusted_empty_footer();
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let result = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, true);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY: strict posture must REFUSE an unprovable non-empty tail with \
         CorruptSegment{{segment_id:7}}, not recover ({frames_end}); got {result:?}"
    );
    let detail = match result {
        Err(StoreError::CorruptSegment { detail, .. }) => detail,
        _ => String::new(),
    };
    assert!(
        detail.contains("unprovable tail") && detail.contains("no untrusted claim reaches past"),
        "the strict refusal must carry the distinct unprovable-tail detail (not the \
         positive-evidence message); got detail: {detail}"
    );
    assert!(
        detail.contains("000007.fbat")
            && detail.contains("do not delete or edit the segment in place")
            && detail.contains("restore this segment from a backup"),
        "the refusal must carry the operator evidence set: the canonical segment filename and \
         remediation guidance that never suggests deleting in place; got detail: {detail}"
    );
}

#[test]
fn permissive_untrusted_footer_nonempty_prefix_recovers() {
    // The DEFAULT permissive (`RecoverTornTail` → false) posture over the EXACT
    // SAME bytes as the strict leg must RECOVER the full CRC-valid prefix — a
    // benign corrupt-footer store whose frames are intact must still open.
    // resolve returns the recovered frame-region end, which must equal the true
    // frames_end (both committed frames recovered, nothing dropped).
    let (bytes, frames_end) = two_intact_frames_then_untrusted_empty_footer();
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let recovered = resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, None, false)
        .expect("the default permissive posture must recover a benign corrupt-footer store")
        .frames_end;
    assert_eq!(
        recovered, frames_end,
        "PROPERTY: the default posture recovers the entire CRC-valid prefix (both intact frames) — \
         the DEFAULT path is unchanged by the feature"
    );
}

#[test]
fn resolve_out_of_bounds_footer_claim_degrades_to_unprovable_tail_not_evidence() {
    // resolve-level: two intact CRC-valid frames + an UNTRUSTED empty footer (zero
    // entries → no rows). The caller passes a footer-claimed-end that is OUT OF
    // BOUNDS — well past `file_len - 16`, leaving no room for even the 16-byte
    // trailer after it. That is a forged offset, NOT a torn frame region, so `resolve`
    // must BOUND it out and fall back to the absence-only unprovable-tail refusal,
    // never a FALSE evidence-of-truncation. (Pins the `claimed <= file_len - TRAILER_LEN`
    // guard — the exact fix for `append_frames_from_segment` on an out-of-bounds footer.)
    let (bytes, frames_end) = two_intact_frames_then_untrusted_empty_footer();
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);
    let result =
        resolve_untrusted_frames_end(&mut cursor, 0, file_len, 7, Some(frames_end + 64), true);
    assert!(
        matches!(
            &result,
            Err(StoreError::CorruptSegment { segment_id: 7, .. })
        ),
        "PROPERTY: strict posture must REFUSE an out-of-bounds untrusted footer with \
         CorruptSegment{{segment_id:7}}; got {result:?}"
    );
    let detail = match result {
        Err(StoreError::CorruptSegment { detail, .. }) => detail,
        _ => String::new(),
    };
    assert!(
        detail.contains("unprovable tail"),
        "an OUT-OF-BOUNDS footer claim is forgery, not truncation evidence: the strict refusal must \
         carry the absence-only unprovable-tail detail, NOT the POSITIVE-truncation-evidence one; \
         got detail: {detail}"
    );
}

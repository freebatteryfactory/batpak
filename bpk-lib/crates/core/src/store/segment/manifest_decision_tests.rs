//! Decision-level tests for the untrusted-footer recovery law: rows of an
//! UNTRUSTED SIDX table are SUSPICION GEOMETRY, never an authority
//! (GAUNT-SIDX-NO-SELF-AUTH, #192 / ADR-0036).
//!
//! PROVES: INV-UNTRUSTED-MANIFEST-NO-AUTHORITY-ESCALATION.
//!
//! Split from `manifest_recovery_tests.rs` to keep each inline test file within
//! the structural file-size budget: this island drives
//! `decide_untrusted_recovery` DIRECTLY over synthetic entries (no segment
//! bytes), pinning the decision arithmetic — empty-prefix recovery, at-stop row
//! inclusion, max-claim selection, hash-blindness, and the per-posture
//! outcomes. The byte-level `resolve_untrusted_frames_end` integration tests
//! (real frames, real footers, torn tails) stay in
//! `manifest_recovery_tests.rs`.

use super::*;
use crate::event::EventKind;
use crate::store::segment::sidx::kind_to_raw;

/// Build a synthetic SIDX entry for driving `decide_untrusted_recovery` directly.
/// The `event_hash` is a caller-chosen constant precisely because the decision
/// must be BLIND to it (the safe law) — several tests pin that blindness.
fn test_entry(
    frame_offset: u64,
    frame_length: u32,
    event_hash: [u8; 32],
) -> crate::store::segment::sidx::SidxEntry {
    crate::store::segment::sidx::SidxEntry {
        event_id: 1,
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
        global_sequence: 1,
        correlation_id: 1,
        causation_id: 0,
    }
}

#[test]
fn decide_empty_recovered_prefix_recovers_under_any_posture() {
    // An EMPTY recovered prefix has nothing to lose: even under STRICT posture,
    // and even with rows claiming frames past the stop, the decision recovers.
    // Kills a deleted/inverted `recovered_frame_count == 0` guard.
    let manifest = vec![test_entry(0, 64, [9; 32]), test_entry(64, 64, [9; 32])];
    for fallback_fail_closed in [false, true] {
        assert_eq!(
            decide_untrusted_recovery(&manifest, 0, 0, Some(128), fallback_fail_closed),
            UntrustedRecovery::RecoverPrefix(0),
            "PROPERTY: an empty recovered prefix recovers under any posture \
             (fallback_fail_closed = {fallback_fail_closed}); rows cannot conjure data to lose"
        );
    }
}

#[test]
fn decide_rows_past_stop_are_suspicion_regardless_of_hash_content() {
    // THE SAFE LAW, unit-level: the decision is BLIND to row hashes. A trailing
    // row at/after P is suspicion whether its hash copies a real frame's stored
    // hash (forgeable — BLAKE3 is public and unkeyed) or is garbage; a table
    // whose rows all sit below P proves nothing about the tail. Exact-variant
    // pins complement the loose issue fixtures below.
    let count = 2u64; // two recovered 64-byte frames → prefix end at 128
    let p = 128u64;

    // Rows all below P (a "fully agreeing" table): permissive recovers the plain
    // prefix; strict refuses as an unprovable tail — agreement is NOT a recovery
    // ticket (the retired case-(b) recovered here, on forgeable evidence).
    let agreeing = vec![test_entry(0, 64, [7; 32]), test_entry(64, 64, [8; 32])];
    assert_eq!(
        decide_untrusted_recovery(&agreeing, count, p, None, false),
        UntrustedRecovery::RecoverPrefix(p),
        "PROPERTY: an agreeing table under permissive recovers the plain prefix (no evidence)"
    );
    assert_eq!(
        decide_untrusted_recovery(&agreeing, count, p, None, true),
        UntrustedRecovery::FailClosedUnprovableTail,
        "PROPERTY (#192): an agreeing table under STRICT refuses — unauthenticated agreement \
         cannot prove the tail complete (the retired law recovered here)"
    );

    // A trailing row at P: suspicion under both postures, with the claimed end
    // (offset + length) carried as diagnostic evidence.
    let with_copied_hash = vec![test_entry(0, 64, [7; 32]), test_entry(p, 64, [7; 32])];
    let with_garbage_hash = vec![test_entry(0, 64, [7; 32]), test_entry(p, 64, [0xAA; 32])];
    for manifest in [&with_copied_hash, &with_garbage_hash] {
        assert_eq!(
            decide_untrusted_recovery(manifest, count, p, None, false),
            UntrustedRecovery::RecoverPrefixWithTruncationEvidence {
                end: p,
                footer_claimed_end: p + 64,
            },
            "PROPERTY (#192): permissive recovers the prefix and records the trailing row's \
             claimed end as evidence — identically for copied and garbage hashes"
        );
        assert_eq!(
            decide_untrusted_recovery(manifest, count, p, None, true),
            UntrustedRecovery::FailClosedEvidenceOfTruncation {
                footer_claimed_end: p + 64,
            },
            "PROPERTY (#192): strict refuses with positive evidence — identically for copied \
             and garbage hashes (the decision never reads them)"
        );
    }
}

#[test]
fn decide_suspicion_arithmetic_at_stop_inclusion_and_max_claim() {
    // Pins the suspicion geometry arithmetic:
    // - a row exactly AT the stop counts (`>=`, not `>`);
    // - a row BELOW the stop does not count, even when its span crosses it
    //   (rows are matched by offset — the region below P was independently
    //   verified, so a below-P row claims nothing about the tail);
    // - among several claims the FURTHEST claimed end wins (max), across both
    //   manifest rows and the footer's own claimed end;
    // - the claimed end saturates instead of overflowing.
    let count = 2u64;
    let p = 128u64;

    // Row exactly AT P counts (kills `>=` → `>`).
    let at_stop = vec![test_entry(p, 64, [9; 32])];
    assert_eq!(
        decide_untrusted_recovery(&at_stop, count, p, None, true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: p + 64,
        },
        "a row exactly AT the stop is a claim about the unverified tail"
    );

    // Row below P does not count even though offset+length crosses P.
    let below_stop = vec![test_entry(p - 1, 64, [9; 32])];
    assert_eq!(
        decide_untrusted_recovery(&below_stop, count, p, None, true),
        UntrustedRecovery::FailClosedUnprovableTail,
        "a row below the stop claims a frame inside the verified region — not tail suspicion"
    );

    // Max across manifest rows (kills `max` → `min` on the row fold).
    let two_claims = vec![test_entry(p, 8, [9; 32]), test_entry(p, 64, [9; 32])];
    assert_eq!(
        decide_untrusted_recovery(&two_claims, count, p, None, true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: p + 64,
        },
        "the furthest row claim wins"
    );

    // Max across footer claim vs manifest claim, in BOTH directions (kills the
    // combine arm picking a fixed side).
    let row_claim = vec![test_entry(p, 64, [9; 32])]; // claims p + 64
    assert_eq!(
        decide_untrusted_recovery(&row_claim, count, p, Some(p + 100), true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: p + 100,
        },
        "the footer claim wins when it reaches further than the rows"
    );
    let far_row_claim = vec![test_entry(p, 200, [9; 32])]; // claims p + 200
    assert_eq!(
        decide_untrusted_recovery(&far_row_claim, count, p, Some(p + 100), true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: p + 200,
        },
        "the row claim wins when it reaches further than the footer"
    );

    // Saturation: a row at u64::MAX cannot overflow the claimed end.
    let saturating = vec![test_entry(u64::MAX, 64, [9; 32])];
    assert_eq!(
        decide_untrusted_recovery(&saturating, count, p, None, true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: u64::MAX,
        },
        "the claimed end saturates at u64::MAX instead of overflowing"
    );
}

#[test]
fn uncorroborated_trailing_row_exact_outcomes_under_both_postures() {
    // Exact-variant pins of the issue's attack fixture (the loose RED fixtures
    // below accept either suspicion outcome; these pin WHICH one): one recovered
    // 64-byte frame (P = 64), a manifest with an honest-looking row for it plus
    // a trailing row at P. The retired law returned "proven loss" under BOTH
    // postures here; the safe law routes the trailing row as suspicion.
    let manifest = vec![test_entry(0, 64, [7; 32]), test_entry(64, 64, [9; 32])];
    let p = 64u64;
    assert_eq!(
        decide_untrusted_recovery(&manifest, 1, p, None, true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: 128,
        },
        "strict: the trailing row is positive (if untrusted) suspicion — refuse with evidence, \
         never 'proven loss'"
    );
    assert_eq!(
        decide_untrusted_recovery(&manifest, 1, p, None, false),
        UntrustedRecovery::RecoverPrefixWithTruncationEvidence {
            end: p,
            footer_claimed_end: 128,
        },
        "permissive: recover the prefix and record the trailing row's claim as evidence — a \
         forged row must never brick the store"
    );
}

#[test]
fn decide_footer_claimed_past_prefix_yields_truncation_evidence() {
    // NO rows past the stop, but the untrusted footer's OWN claimed frame end
    // lies strictly PAST the recovered prefix end P → a torn/corrupt region may
    // sit between the recovered frames and the footer = POSITIVE (if untrusted)
    // truncation evidence. The strict and permissive postures diverge on what to
    // do with it.
    let below_p_rows = vec![
        test_entry(0, 64, [0xAA; 32]),
        test_entry(64, 64, [0xBB; 32]),
    ];
    let count = 2u64;
    let p = 128u64;

    // STRICT: refuse with the DISTINCT evidence-of-truncation variant (NOT the
    // absence-only FailClosedUnprovableTail). Convicts a swap of the `recovery_stop <
    // claimed` gap guard and a wrong strict-branch variant selection.
    assert_eq!(
        decide_untrusted_recovery(&below_p_rows, count, p, Some(p + 64), true),
        UntrustedRecovery::FailClosedEvidenceOfTruncation {
            footer_claimed_end: p + 64,
        },
        "strict posture with a footer claiming frames past P must fail closed with POSITIVE \
         truncation evidence, not the mere-absence unprovable-tail variant"
    );

    // PERMISSIVE (default): recover the CRC-valid prefix WHILE recording the evidence.
    assert_eq!(
        decide_untrusted_recovery(&below_p_rows, count, p, Some(p + 64), false),
        UntrustedRecovery::RecoverPrefixWithTruncationEvidence {
            end: p,
            footer_claimed_end: p + 64,
        },
        "permissive posture recovers the prefix but records the footer-cross-checked truncation \
         evidence (end = P, footer_claimed_end past P) for the caller"
    );
}

#[test]
fn decide_footer_claimed_at_or_absent_is_not_truncation_evidence() {
    // The gap guard is STRICT (`recovery_stop < claimed`, NOT `<=`): a footer whose
    // claimed end equals P (frames end exactly at the footer = a CLEAN segment) is NOT
    // evidence, and an absent footer hint is not either. Both fall through to the
    // absence-only decisions.
    let below_p_rows = vec![
        test_entry(0, 64, [0xAA; 32]),
        test_entry(64, 64, [0xBB; 32]),
    ];
    let count = 2u64;
    let p = 128u64;

    // footer_claimed_end == P (no gap): strict → the mere-absence unprovable tail, NOT
    // evidence-of-truncation. Convicts a `<`→`<=` mutant on the gap guard.
    assert_eq!(
        decide_untrusted_recovery(&below_p_rows, count, p, Some(p), true),
        UntrustedRecovery::FailClosedUnprovableTail,
        "footer claiming frames end exactly AT P is a clean boundary, not truncation evidence \
         (the gap guard is `<`, not `<=`)"
    );

    // No footer hint at all: strict → the absence-only unprovable-tail refusal.
    assert_eq!(
        decide_untrusted_recovery(&below_p_rows, count, p, None, true),
        UntrustedRecovery::FailClosedUnprovableTail,
        "with no footer-claimed-end hint the strict posture keeps its absence-only \
         unprovable-tail refusal"
    );

    // footer_claimed_end == P under PERMISSIVE: plain prefix recovery, no evidence.
    assert_eq!(
        decide_untrusted_recovery(&below_p_rows, count, p, Some(p), false),
        UntrustedRecovery::RecoverPrefix(p),
        "a clean at-P boundary under the permissive posture recovers the plain prefix with no \
         truncation evidence"
    );
}

// ---------------------------------------------------------------------------
// GAUNT-SIDX-NO-SELF-AUTH (#192): a corroborated row authenticates ONLY itself.
// RED-first fixtures — written against the OLD law (one corroborated row grants
// the whole table authority) and asserting the NEW law, so they FAILED until the
// decision rewrite landed. BLAKE3 is public and unkeyed: a party who can read or
// edit the segment copies a real frame's stored hash, so one honest row must
// never let forged siblings tell the store what reality was. These deliberately
// accept EITHER suspicion outcome per posture (the law fixes the direction, not
// the diagnostic flavor); the exact-variant pins live in
// `uncorroborated_trailing_row_exact_outcomes_under_both_postures`.
// ---------------------------------------------------------------------------

#[test]
fn forged_sibling_row_must_not_prove_loss_one_corroborated_row_authenticates_only_itself() {
    // The issue's exact attack shape: row 0 mirrors a real recovered frame
    // (offset, length, and stored content hash all "match"); row 1 is a
    // FABRICATED trailing-frame claim at/after the recovery stop P. These bytes
    // are INDISTINGUISHABLE from a legitimate torn-last-frame manifest — which
    // is precisely why the sibling row is SUSPICION, never PROOF. The old law
    // upgraded row 0's match into table-level authority and returned
    // FailClosedCorroboratedLoss under BOTH postures (a false refusal /
    // denial-of-service on the permissive default).
    let p = 64u64; // one recovered 64-byte frame
                   // Row 0: honest copy (mirrors the recovered frame). Row 1: forged trailing claim at P.
    let manifest = vec![test_entry(0, 64, [7; 32]), test_entry(64, 64, [9; 32])];

    // PERMISSIVE (the default posture): an unverifiable sibling row is
    // suspicion routed through the tail posture — recover the CRC-valid
    // prefix (optionally carrying truncation evidence), NEVER a hard refusal.
    let permissive = decide_untrusted_recovery(&manifest, 1, p, None, false);
    assert!(
        matches!(
            permissive,
            UntrustedRecovery::RecoverPrefix(64)
                | UntrustedRecovery::RecoverPrefixWithTruncationEvidence { end: 64, .. }
        ),
        "PROPERTY (#192): under the permissive default, a forged/unverifiable sibling \
         row must not brick the store — recover the prefix (with or without recorded \
         evidence); got {permissive:?}"
    );

    // STRICT: the same suspicion refuses — but as a SUSPICION outcome
    // (unprovable tail / positive-if-untrusted evidence), never as the
    // proven-loss claim the old law fabricated from sibling authority.
    let strict = decide_untrusted_recovery(&manifest, 1, p, None, true);
    assert!(
        matches!(
            strict,
            UntrustedRecovery::FailClosedUnprovableTail
                | UntrustedRecovery::FailClosedEvidenceOfTruncation { .. }
        ),
        "PROPERTY (#192): under strict posture the forged-sibling shape refuses as \
         SUSPICION (unprovable/evidence-of-truncation), never as proven loss — a row \
         can only ever vouch for itself; got {strict:?}"
    );
}

#[test]
fn truncated_agreeing_table_must_not_prove_safety_under_strict_posture() {
    // The dual of the forged sibling — the UNREPORTED half found in the full
    // read: `entry_count` is exactly as unauthenticated as the rows. A forger
    // (or plain corruption) that TRUNCATES the table after the last intact
    // frame AND tears the final committed frame produces a manifest that
    // "agrees the recovered region is complete." The old law's case (b) let
    // that agreement recover EVEN UNDER STRICT posture — silently dropping the
    // committed frame that strict posture exists to protect. The new law:
    // table agreement proves nothing; strict refuses any non-empty prefix
    // under an untrusted footer.
    let p = 64u64; // one recovered 64-byte frame
                   // The truncated table: ONLY the row mirroring the surviving frame. Under the
                   // old law that row "anchored" the table and case (b) vouched "nothing
                   // follows P".
    let manifest = vec![test_entry(0, 64, [7; 32])];

    let strict = decide_untrusted_recovery(&manifest, 1, p, None, true);
    assert!(
        matches!(
            strict,
            UntrustedRecovery::FailClosedUnprovableTail
                | UntrustedRecovery::FailClosedEvidenceOfTruncation { .. }
        ),
        "PROPERTY (#192): a truncated-but-agreeing table must not vouch for tail \
         completeness — strict posture refuses the unprovable non-empty tail regardless \
         of manifest agreement; got {strict:?}"
    );

    // Guard against overcorrection (issue non-goal: no blanket refusal under the
    // permissive default): the same shape under permissive still recovers.
    let permissive = decide_untrusted_recovery(&manifest, 1, p, None, false);
    assert!(
        matches!(
            permissive,
            UntrustedRecovery::RecoverPrefix(64)
                | UntrustedRecovery::RecoverPrefixWithTruncationEvidence { end: 64, .. }
        ),
        "PROPERTY (#192): the permissive default still recovers a benign truncated-table \
         store; got {permissive:?}"
    );
}

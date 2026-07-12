//! Untrusted-footer recovery: the SIDX entry table as an UNTRUSTED SUSPICION
//! SIGNAL — never an authority (GAUNT-SIDX-NO-SELF-AUTH, #192).
//!
//! For an UNTRUSTED footer boundary (CRC-failed SDX3, legacy un-CRC'd SDX2, or a
//! forged trailer) the trailer `string_table_offset` is unauthenticated and must
//! never bound recovery. Plain CRC-valid-frame recovery (see
//! [`super::crc_valid_frames_end`]) recovers the prefix and fails closed on
//! mid-stream corruption, but it CANNOT distinguish a torn/corrupt LAST committed
//! frame (followed by the footer) from "intact frames + footer". This module
//! routes that ambiguity through the caller's tail posture, treating every
//! untrusted claim about the tail — the footer's own claimed frame end AND any
//! SIDX row naming a frame at/after the recovered prefix end — as SUSPICION:
//!
//! 1. parse the entry table WITHOUT requiring the footer CRC (every row is an
//!    UNTRUSTED HYPOTHESIS, useful only as geometry/hint input);
//! 2. recover the CRC-valid prefix `[frames_start .. P)`;
//! 3. decide by posture: the default permissive posture recovers the prefix
//!    (recording truncation evidence when an untrusted claim reaches past `P`);
//!    the strict `FailClosed` posture refuses ANY non-empty prefix under an
//!    untrusted footer — truncation of a further committed frame can never be
//!    ruled out from unauthenticated bytes, no matter what the table says.
//!
//! THE SAFE LAW (the doctrine correction this module was rewritten for):
//! a row corroborated against independently verified frame bytes proves only
//! THAT row's relationship to THAT frame. It does not authenticate any other
//! row, the row count, the order, the footer boundary, or claims about
//! nonexistent trailing frames. BLAKE3 is public and unkeyed: a party able to
//! read or edit the segment can copy a real frame's stored `event_hash`, so a
//! matching row is NOT a signature over its siblings. No future optimization
//! may weaken this without a keyed MAC, digital signature, or independently
//! trusted root covering the COMPLETE table. The retired design ("one
//! corroborated row anchors the whole table") let a forged sibling row refuse
//! recovery of an intact store (false loss) and let a truncated-but-agreeing
//! table recover under the STRICT posture while a committed frame was torn
//! (false safety) — both are the same escalation of one row-level match into
//! table-level authority. Because no row-level match can contribute to the
//! decision, this module performs NO hash corroboration at all: rows are
//! consulted only for the offsets they claim (suspicion geometry), and the
//! per-frame hash-extraction walk the retired design ran was deleted with it.
//!
//! SIDX entries cover ONLY committed frames; batch BEGIN/COMMIT markers are real
//! frames but NOT SIDX entries. Rows are matched by offset only for the
//! at/after-`P` suspicion test; contiguity is never assumed, and the scan
//! loops' BatchRecoveryState discard logic is untouched.

use super::{crc_valid_frame_exists_after, sidx, try_decode_frame_at, StoreError};
use std::io::{Read, Seek};

/// The outcome of the untrusted-footer recovery decision.
///
/// `RecoverPrefix(end)` recovers the CRC-valid frame region `[frames_start ..
/// end)`; `RecoverPrefixWithTruncationEvidence` recovers the same prefix but
/// records untrusted-claim truncation evidence for the caller. The two
/// fail-closed reasons surface the SAME [`StoreError::CorruptSegment`] refusal
/// but carry DISTINCT detail strings (see [`resolve_untrusted_frames_end`]):
/// - `FailClosedUnprovableTail` — the strict tail posture refusing a non-empty
///   recovered prefix under an untrusted footer with no positive suspicion
///   signal (fail-closed on ABSENCE of proof that the tail is complete);
/// - `FailClosedEvidenceOfTruncation` — the strict tail posture refusing a
///   prefix when an UNTRUSTED claim (the footer's own claimed frame end, or a
///   SIDX row naming a frame at/after the prefix end) reaches strictly past
///   the recovered prefix (fail-closed on POSITIVE-if-untrusted evidence).
///
/// There is deliberately NO "proven loss" outcome: under an untrusted footer
/// no unauthenticated table row can PROVE a committed frame existed (#192 —
/// the retired `FailClosedCorroboratedLoss` variant claimed exactly that).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UntrustedRecovery {
    /// Recover the CRC-valid prefix that ends at this offset.
    RecoverPrefix(u64),
    /// STRICT (`FailClosed`) posture refusing an UNPROVABLE tail: a non-empty
    /// CRC-valid prefix was recovered beneath an untrusted footer, and nothing
    /// can rule out a torn/truncated further committed frame. Only returned
    /// when the caller passed `fallback_fail_closed` (opted into the strict
    /// tail posture); the default permissive posture recovers the prefix
    /// instead.
    FailClosedUnprovableTail,
    /// Permissive (`RecoverTornTail`) recover of the CRC-valid prefix ending at
    /// `end`, WITH positive truncation evidence: an UNTRUSTED claim — the
    /// footer's own claimed frame end, or a SIDX row naming a committed frame
    /// at/after the prefix end — reaches strictly PAST `end`, so a torn/corrupt
    /// region may sit between the recovered frames and the footer. The default
    /// posture recovers the prefix while recording the evidence for the caller.
    /// `footer_claimed_end` carries the furthest such claimed end regardless of
    /// which untrusted source made the claim; it is DIAGNOSTIC, never authority.
    RecoverPrefixWithTruncationEvidence { end: u64, footer_claimed_end: u64 },
    /// STRICT posture refusing a tail with POSITIVE truncation evidence: an
    /// untrusted claim (footer claimed end, or a SIDX row at/after the prefix
    /// end) reaches strictly PAST the recovered prefix. Distinct from
    /// `FailClosedUnprovableTail`, which refuses on mere ABSENCE of any signal.
    /// Fail-closed-on-SUSPICION: the sources are untrusted, so a forged claim
    /// can trip it — under strict posture that is the correct direction.
    FailClosedEvidenceOfTruncation { footer_claimed_end: u64 },
}

/// Positive, footer-cross-checked evidence that a committed frame was torn between
/// the recovered CRC-valid prefix and the (untrusted) footer. Recorded on the
/// permissive recover path; the strict path fails closed instead (see
/// [`UntrustedRecovery::FailClosedEvidenceOfTruncation`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TruncationEvidence {
    /// The CRC-valid recovered-prefix end `P` (where the frame walk stopped).
    pub(crate) recovered_prefix_end: u64,
    /// The untrusted footer's OWN claimed frame-region end, strictly past `P`.
    pub(crate) footer_claimed_frames_end: u64,
}

/// The resolved frame-region end for an untrusted footer, plus any truncation
/// evidence recorded on the permissive recover path (`None` on the strict path,
/// which fails closed on such evidence rather than recovering).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedFramesEnd {
    pub(crate) frames_end: u64,
    pub(crate) truncation_evidence: Option<TruncationEvidence>,
}

/// Walk the CRC-valid frames from `frames_start`, counting recovered frames AND
/// applying the same mid-stream-corruption fail-closed rule as
/// [`crc_valid_frames_end`]. Returns `(stop_offset, recovered_frame_count)`
/// where `stop_offset` (P) is the first non-decodable position (the recovered
/// prefix end).
///
/// The count exists solely so the decision can distinguish "an EMPTY recovered
/// prefix" (nothing to lose → recover under any posture) from a non-empty one.
/// The retired design additionally re-read every frame here to extract its
/// content `event_hash` for table corroboration; that walk was deleted with the
/// corroboration authority it fed (#192) — frame bytes are decoded once, under
/// their own CRC, and nothing else.
///
/// # Errors
/// Returns [`StoreError::Io`] on seek/read failure, or
/// [`StoreError::CorruptSegment`] on mid-stream corruption (same contract as
/// [`crc_valid_frames_end`]).
pub(crate) fn crc_valid_frames_end_counting<R: Read + Seek>(
    source: &mut R,
    frames_start: u64,
    file_len: u64,
    segment_id: u64,
) -> Result<(u64, u64), StoreError> {
    let mut cursor = frames_start;
    let mut recovered_frame_count: u64 = 0;

    loop {
        if cursor >= file_len {
            return Ok((file_len, recovered_frame_count));
        }
        match try_decode_frame_at(source, cursor, file_len)? {
            Some(frame_size) => {
                recovered_frame_count = recovered_frame_count.saturating_add(1);
                cursor = match cursor.checked_add(frame_size) {
                    Some(next) => next,
                    None => return Ok((cursor, recovered_frame_count)),
                };
            }
            None => {
                let resync_from = match cursor.checked_add(1) {
                    Some(next) => next,
                    None => return Ok((cursor, recovered_frame_count)),
                };
                if crc_valid_frame_exists_after(source, resync_from, file_len)? {
                    return Err(StoreError::corrupt_segment_with_detail(
                        segment_id,
                        format!(
                            "mid-stream corruption: frame at offset {cursor} is non-decodable but a \
                             CRC-valid frame follows before EOF (file_len {file_len}); refusing to \
                             silently truncate to the prefix during untrusted-footer recovery"
                        ),
                    ));
                }
                return Ok((cursor, recovered_frame_count));
            }
        }
    }
}

/// Decide the untrusted-footer recovery outcome from the recovered-prefix shape
/// and the untrusted claims about the tail — the footer's own claimed frame end
/// and the SIDX rows. Rows are SUSPICION, never authority (#192).
///
/// THE SAFE LAW: a row corroborated against independently verified frame bytes
/// proves only that row's relationship to that frame. It does not authenticate
/// any other row, `entry_count`, order, the footer boundary, or claims about
/// nonexistent trailing frames. BLAKE3 is public and unkeyed — a party able to
/// read or edit the segment can copy a real frame's stored `event_hash` — so a
/// matching row must never deputize its siblings. No future optimization may
/// weaken this without a keyed MAC, digital signature, or independently
/// trusted root covering the COMPLETE table.
///
/// Consequently row hashes contribute NOTHING to this decision (the function
/// never reads them): every decision-relevant row (one naming a frame at/after
/// `recovery_stop`) is unverifiable by construction (the CRC walk ended at the
/// stop, so no verified frame exists there to check against), and rows before
/// the stop can vouch only for themselves. The retired design escalated one
/// row-level match into table authority, producing BOTH failure duals:
/// - false LOSS: a forged sibling row "proved" a committed frame was missing,
///   refusing recovery of an intact store under any posture (DoS);
/// - false SAFETY: a truncated-but-corroborating table "agreed the region was
///   complete", recovering under the STRICT posture while a torn committed
///   frame was silently dropped — the exact loss strict posture pays to
///   prevent.
///
/// DECISION (only ever invoked on the UNTRUSTED footer path):
/// - An EMPTY recovered prefix has nothing to lose: recover (any posture).
/// - Suspicion := the bounded footer claimed-end reaches strictly past the
///   stop, OR any row names a committed frame at/after the stop (its claimed
///   end `frame_offset + frame_length` is reported as diagnostic evidence; the
///   value is untrusted either way and never changes the decision direction).
/// - permissive (default `RecoverTornTail` → `false`): recover the CRC-valid
///   prefix — with recorded truncation evidence when suspicion exists. Never a
///   false fail-closed: a benign corrupt-footer store whose frames are intact
///   must still open.
/// - strict (`FailClosed` → `true`): refuse ANY non-empty prefix — as
///   `FailClosedEvidenceOfTruncation` when suspicion is positive, else as
///   `FailClosedUnprovableTail`. Unauthenticated bytes can never PROVE the
///   tail complete, so under strict posture "manifest agreement" is not a
///   recovery ticket.
pub(crate) fn decide_untrusted_recovery(
    entries: &[sidx::SidxEntry],
    recovered_frame_count: u64,
    recovery_stop: u64,
    footer_claimed_frames_end: Option<u64>,
    fallback_fail_closed: bool,
) -> UntrustedRecovery {
    // An EMPTY recovered prefix has no data to lose and recovers regardless of
    // posture (unchanged from the retired design).
    if recovered_frame_count == 0 {
        return UntrustedRecovery::RecoverPrefix(recovery_stop);
    }

    // Suspicion signals — both untrusted, both positive-if-true, both merely
    // DIAGNOSTIC in value:
    // 1. the footer's own claimed frame end past the stop (bounded by the
    //    caller against the file length so a garbage offset cannot manufacture
    //    "positive evidence" — see resolve_untrusted_frames_end);
    // 2. any SIDX row naming a committed frame at/after the stop. The row is
    //    an untrusted hypothesis; whether its stored hash happens to match a
    //    recovered frame elsewhere grants it NO extra weight (the safe law).
    let manifest_claimed_end = entries
        .iter()
        .filter(|entry| entry.frame_offset >= recovery_stop)
        .map(|entry| {
            entry
                .frame_offset
                .saturating_add(u64::from(entry.frame_length))
        })
        .max();
    let footer_claim = footer_claimed_frames_end.filter(|&claimed| recovery_stop < claimed);
    let suspicion = match (footer_claim, manifest_claimed_end) {
        (Some(footer), Some(manifest)) => Some(footer.max(manifest)),
        (Some(footer), None) => Some(footer),
        (None, Some(manifest)) => Some(manifest),
        (None, None) => None,
    };

    if fallback_fail_closed {
        // STRICT: refuse any non-empty prefix under an untrusted footer. The
        // two refusals differ only in their diagnostic detail (positive
        // suspicion vs absence of any signal) — both are refusals.
        return match suspicion {
            Some(claimed) => UntrustedRecovery::FailClosedEvidenceOfTruncation {
                footer_claimed_end: claimed,
            },
            None => UntrustedRecovery::FailClosedUnprovableTail,
        };
    }
    // PERMISSIVE (the default): recover the prefix; record the suspicion as
    // truncation evidence for the caller when it exists.
    match suspicion {
        Some(claimed) => UntrustedRecovery::RecoverPrefixWithTruncationEvidence {
            end: recovery_stop,
            footer_claimed_end: claimed,
        },
        None => UntrustedRecovery::RecoverPrefix(recovery_stop),
    }
}

/// Resolve the frame-region end for an UNTRUSTED footer boundary, treating the
/// SIDX entry table as an untrusted suspicion signal — never an authority
/// (#192; see the module doctrine).
///
/// This is the single entry point the three scan/compaction sites call instead of
/// bare [`crc_valid_frames_end`]. It:
///   1. walks the CRC-valid frames (recovering the prefix, and still failing
///      closed on mid-stream corruption — unchanged round-5/6 behavior);
///   2. parses the untrusted entry table (zero entries on any parse failure);
///   3. decides by tail posture, with untrusted claims past the prefix end
///      (footer claimed-end, SIDX rows) routed as suspicion.
///
/// `fallback_fail_closed` is the caller's [`scan::FrameScanTailPolicy`] reduced to
/// a bool (FailClosed → true): a strict caller refuses ANY non-empty recovered
/// prefix under an untrusted footer (with a suspicion-specific detail when an
/// untrusted claim reaches past the prefix), while the default permissive caller
/// recovers the prefix (recording the suspicion as truncation evidence). It is
/// passed as a bool to keep this module decoupled from the scan layer.
///
/// The strict-refusal detail strings deliberately carry the full operator
/// evidence set (#192 ruling): the segment id AND canonical filename, the last
/// independently verified frame boundary, the fact that nothing decodes past
/// it, the untrusted claim (when one exists), the posture used, the explicit
/// statement that no authenticated evidence proves the missing region empty,
/// and remediation guidance that never suggests deleting the segment in place.
///
/// # Errors
/// Returns [`StoreError::Io`] on read failure, or [`StoreError::CorruptSegment`]
/// for mid-stream corruption (from the walk) or a strict-posture refusal of a
/// non-empty tail (unprovable, or with positive untrusted suspicion).
pub(crate) fn resolve_untrusted_frames_end<R: Read + Seek>(
    source: &mut R,
    frames_start: u64,
    file_len: u64,
    segment_id: u64,
    footer_claimed_frames_end: Option<u64>,
    fallback_fail_closed: bool,
) -> Result<ResolvedFramesEnd, StoreError> {
    // Step 2/4c: parse the untrusted entry table FIRST (it seeks to EOF). Zero
    // entries on any parse failure → pure fall-back.
    let entries = sidx::read_entries_unauthenticated(source, segment_id)?;

    // Step 2/4b: recover the CRC-valid prefix. This is the mid-stream
    // corruption guard; it errors before we ever consult the manifest.
    let (recovery_stop, recovered_frame_count) =
        crc_valid_frames_end_counting(source, frames_start, file_len, segment_id)?;

    // An untrusted footer's claimed frame end is plausible truncation evidence only
    // if it leaves room for the footer trailer after it. A forged/out-of-bounds
    // claim (> file_len - TRAILER_LEN — e.g. == file_len) is garbage, not a torn
    // frame region, so it degrades to the absence-only unprovable-tail decision
    // instead of a FALSE evidence-of-truncation. TRAILER_LEN mirrors the 16-byte
    // SIDX trailer read by `detect_sidx_boundary`.
    const TRAILER_LEN: u64 = 16;
    let bounded_footer_claim = footer_claimed_frames_end
        .filter(|&claimed| claimed <= file_len.saturating_sub(TRAILER_LEN));

    // Step 3/4: decide by posture.
    match decide_untrusted_recovery(
        &entries,
        recovered_frame_count,
        recovery_stop,
        bounded_footer_claim,
        fallback_fail_closed,
    ) {
        UntrustedRecovery::RecoverPrefix(end) => Ok(ResolvedFramesEnd {
            frames_end: end,
            truncation_evidence: None,
        }),
        UntrustedRecovery::RecoverPrefixWithTruncationEvidence {
            end,
            footer_claimed_end,
        } => Ok(ResolvedFramesEnd {
            frames_end: end,
            truncation_evidence: Some(TruncationEvidence {
                recovered_prefix_end: end,
                footer_claimed_frames_end: footer_claimed_end,
            }),
        }),
        UntrustedRecovery::FailClosedUnprovableTail => {
            Err(StoreError::corrupt_segment_with_detail(
                segment_id,
                format!(
                    "untrusted-footer recovery: strict FailClosed posture refuses an unprovable \
                     tail in segment file {segment_id:06}.fbat — a non-empty CRC-valid prefix ends \
                     at byte {recovery_stop} (the last independently verified frame boundary; no \
                     CRC-valid frame decodes past it) beneath an untrusted footer (file_len \
                     {file_len}), and no untrusted claim reaches past that boundary either; \
                     unauthenticated bytes cannot prove the region past the boundary held no \
                     committed frame, so recovery refuses rather than guess; remediation: restore \
                     this segment from a backup, or inspect a COPY of the store directory offline \
                     — do not delete or edit the segment in place; the permissive RecoverTornTail \
                     posture (granted only to the newest segment at cold start) would recover \
                     this prefix"
                ),
            ))
        }
        UntrustedRecovery::FailClosedEvidenceOfTruncation { footer_claimed_end } => {
            Err(StoreError::corrupt_segment_with_detail(
                segment_id,
                format!(
                    "untrusted-footer recovery: strict FailClosed posture refuses a tail with \
                     POSITIVE truncation evidence in segment file {segment_id:06}.fbat — CRC-valid \
                     frames end at byte {recovery_stop} (the last independently verified frame \
                     boundary; no CRC-valid frame decodes past it) but an untrusted claim (the \
                     footer trailer or a SIDX manifest row; unauthenticated either way) says \
                     frames extend to {footer_claimed_end} (file_len {file_len}); a committed \
                     frame may have been torn or dropped in the {gap}-byte gap and no \
                     authenticated evidence proves that region empty; remediation: restore this \
                     segment from a backup, or inspect a COPY of the store directory offline — do \
                     not delete or edit the segment in place",
                    gap = footer_claimed_end.saturating_sub(recovery_stop)
                ),
            ))
        }
    }
}

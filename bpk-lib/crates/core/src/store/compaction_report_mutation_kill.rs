//! PROVES: the compaction-evidence builders carry structural truth — the id
//! bound extraction (first==low / last==high), the live-strategy→shape mapping,
//! the skip-report column zeroing + id sort + sealed count, and the run-report
//! finding ladder (PreSwapRollback on Failed, OutputSegmentHashUnavailable on a
//! Performed run whose merged bytes cannot be hashed) plus its count/hash
//! passthrough. Also that `body_hash` is finding-order independent and non-zero,
//! and `from_body` binds it.
//! CATCHES: `segment_id_bounds` first/last swap + `_ => (None, None)` arm
//! collapse; each `compaction_strategy_shape` match-arm swap; `report_skipped`
//! sealed-count/sort/threshold drift and the `-> 0` reclaim columns;
//! `report_for_run`'s `(Performed, Some|None)` / `_` output-hash arms and
//! `push_failed_finding`'s Failed guard; and `body_hash -> Ok(Default::default())`
//! (all-zero digest) plus the `sort_findings` removal.
//! SEEDED: deterministic — no randomness, no clock, no store.

use super::{
    compaction_id_digest, compaction_strategy_shape, report_for_run, report_skipped,
    segment_id_bounds, CompactionEvidenceReport, CompactionReportBody, CompactionReportFinding,
    CompactionStrategyShape, CompactionStructuralFingerprint, COMPACTION_REPORT_SCHEMA_VERSION,
};
use crate::evidence::content_hash;
use crate::store::segment::{CompactionOutcome, CompactionResult};
use crate::store::{CompactionConfig, CompactionStrategy};
use std::path::{Path, PathBuf};

fn merge_config(min_segments: usize) -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Merge,
        min_segments,
    }
}

fn sealed_pair() -> Vec<(u64, PathBuf)> {
    vec![
        (7u64, PathBuf::from("/d/000007.fbat")),
        (2u64, PathBuf::from("/d/000002.fbat")),
    ]
}

#[test]
fn segment_id_bounds_takes_first_as_low_and_last_as_high() {
    // `report_skipped`/`report_for_run` feed a SORTED id slice, so first==min
    // (low) and last==max (high). Kills the `first`/`last` swap, the
    // `_ => (None, None)` arm collapsing the populated case, and any whole-body
    // `-> (None, None)` / `Default::default()` replacement.
    assert_eq!(segment_id_bounds(&[3, 7, 9]), (Some(3), Some(9)));
    assert_eq!(segment_id_bounds(&[42]), (Some(42), Some(42)));
    assert_eq!(
        segment_id_bounds(&[]),
        (None, None),
        "an empty id set carries no bounds"
    );
}

#[test]
fn compaction_strategy_shape_maps_each_live_strategy_to_its_shape() {
    // The shape is the ONLY strategy identity carried into evidence. Kills each
    // match-arm swap/delete: a Merge that reports Retention (or a constant body)
    // silently mislabels the compaction decision.
    assert_eq!(
        compaction_strategy_shape(&CompactionStrategy::Merge),
        CompactionStrategyShape::Merge
    );
    assert_eq!(
        compaction_strategy_shape(&CompactionStrategy::Retention(Box::new(|_| true))),
        CompactionStrategyShape::Retention
    );
    assert_eq!(
        compaction_strategy_shape(&CompactionStrategy::Tombstone(Box::new(|_| true))),
        CompactionStrategyShape::Tombstone
    );
}

#[test]
fn report_skipped_sorts_ids_counts_sealed_and_zeroes_the_reclaim_columns() {
    // A skip report must: sort the source ids ascending, count every sealed
    // source, echo the threshold + active id, and pin the reclaim columns to
    // zero (nothing was merged or removed). Kills sort removal, count/threshold
    // drift, and the `segments_removed`/`bytes_reclaimed` non-zero mutants.
    let sealed = sealed_pair();
    let config = merge_config(4);
    let body = report_skipped(&config, 11, &sealed).expect("skip report encodes");

    assert_eq!(body.outcome, CompactionOutcome::Skipped);
    assert_eq!(
        body.sealed_segment_count, 2,
        "both sealed sources are counted; kills the `-> 0`/`-> 1` count mutants"
    );
    assert_eq!(
        body.source_segment_ids_sorted,
        vec![2, 7],
        "ids are sorted ascending, not left in scan order; kills the `.sort()` removal"
    );
    assert_eq!(body.input_segment_id_low, Some(2));
    assert_eq!(body.input_segment_id_high, Some(7));
    assert_eq!(body.active_segment_id, 11);
    assert_eq!(
        body.min_segments_threshold, 4,
        "the min-segments threshold is echoed from config"
    );
    assert_eq!(body.merged_segment_id, None);
    assert_eq!(body.segments_removed, 0);
    assert_eq!(body.bytes_reclaimed, 0);
    assert_eq!(body.schema_version, COMPACTION_REPORT_SCHEMA_VERSION);
    assert!(body.output_segment_bytes_hash.is_none());
    assert!(
        body.findings.is_empty(),
        "a pure skip carries no structural findings"
    );
}

#[test]
fn report_for_run_flags_a_failed_run_with_preswap_rollback_and_no_output_hash() {
    // A Failed engine outcome must surface a PreSwapRollback finding carrying the
    // engine reason, and must NOT read any merged bytes (no output hash, no
    // OutputSegmentHashUnavailable). Kills `push_failed_finding`'s Failed guard
    // (a `()` body / deleted push drops the rollback finding) and proves the
    // output-hash match routes a non-Performed outcome to `None`.
    let sealed = sealed_pair();
    let config = merge_config(1);
    let result = CompactionResult {
        outcome: CompactionOutcome::Failed {
            reason: "rebuild aborted".to_string(),
        },
        segments_removed: 0,
        bytes_reclaimed: 0,
    };
    let body = report_for_run(&config, 5, &sealed, Some(2), &result, None)
        .expect("failed run report encodes");

    assert!(
        body.findings.iter().any(|finding| matches!(
            finding,
            CompactionReportFinding::PreSwapRollback { reason } if reason == "rebuild aborted"
        )),
        "a Failed outcome must record PreSwapRollback with the engine reason"
    );
    assert!(
        !body.findings.iter().any(|finding| matches!(
            finding,
            CompactionReportFinding::OutputSegmentHashUnavailable { .. }
        )),
        "a Failed run reads no merged bytes, so no output-hash finding"
    );
    assert_eq!(body.output_segment_bytes_hash, None);
    assert_eq!(
        body.outcome,
        CompactionOutcome::Failed {
            reason: "rebuild aborted".to_string()
        }
    );
}

#[test]
fn report_for_run_hashes_a_readable_merged_segment_and_passes_counts_through() {
    // A Performed run with a readable merged path must hash exactly those bytes,
    // record NO OutputSegmentHashUnavailable / PreSwapRollback finding, and echo
    // the engine `segments_removed`/`bytes_reclaimed`/`merged_segment_id`. Kills
    // the `(Performed, Some(path))` arm collapse and the count passthrough.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let merged = dir.path().join("000002.fbat");
    let merged_bytes = b"merged-segment-evidence-bytes";
    crate::store::platform::fs::write_file_atomically(
        dir.path(),
        &merged,
        "merged segment fixture",
        |file| {
            use std::io::Write;
            file.write_all(merged_bytes)
                .map_err(crate::store::StoreError::Io)
        },
    )
    .expect("write merged fixture through the platform seam");

    let sealed = sealed_pair();
    let config = merge_config(1);
    let result = CompactionResult {
        outcome: CompactionOutcome::Performed,
        segments_removed: 2,
        bytes_reclaimed: 4096,
    };
    let body = report_for_run(&config, 5, &sealed, Some(2), &result, Some(&merged))
        .expect("performed run report encodes");

    assert_eq!(
        body.output_segment_bytes_hash,
        Some(content_hash(merged_bytes)),
        "the output hash is exactly blake3 over the merged segment bytes"
    );
    assert!(
        !body.findings.iter().any(|finding| matches!(
            finding,
            CompactionReportFinding::OutputSegmentHashUnavailable { .. }
                | CompactionReportFinding::PreSwapRollback { .. }
        )),
        "a readable Performed run has neither a rollback nor a hash-unavailable finding"
    );
    assert_eq!(body.segments_removed, 2);
    assert_eq!(body.bytes_reclaimed, 4096);
    assert_eq!(body.merged_segment_id, Some(2));
    assert_eq!(body.source_segment_ids_sorted, vec![2, 7]);
    assert_eq!(body.outcome, CompactionOutcome::Performed);
}

#[test]
fn report_for_run_flags_a_performed_run_whose_merged_bytes_cannot_be_hashed() {
    // A Performed run must still be evidence-complete when the merged bytes are
    // unavailable: `None` path (never materialized for hashing) and an
    // unreadable path (IO error) each record OutputSegmentHashUnavailable with a
    // `None` hash. Kills the `(Performed, None)` arm and the read-error arm.
    let sealed = sealed_pair();
    let config = merge_config(1);
    let result = CompactionResult {
        outcome: CompactionOutcome::Performed,
        segments_removed: 1,
        bytes_reclaimed: 10,
    };

    let none_body = report_for_run(&config, 5, &sealed, Some(2), &result, None)
        .expect("performed/none report encodes");
    assert_eq!(none_body.output_segment_bytes_hash, None);
    assert!(
        none_body.findings.iter().any(|finding| matches!(
            finding,
            CompactionReportFinding::OutputSegmentHashUnavailable { .. }
        )),
        "a Performed run with no merged path records the hash-unavailable finding"
    );

    let missing = Path::new("/nonexistent/batpak/merged/never.fbat");
    let missing_body = report_for_run(&config, 5, &sealed, Some(2), &result, Some(missing))
        .expect("performed/unreadable report encodes");
    assert_eq!(missing_body.output_segment_bytes_hash, None);
    assert!(
        missing_body.findings.iter().any(|finding| matches!(
            finding,
            CompactionReportFinding::OutputSegmentHashUnavailable { .. }
        )),
        "a Performed run whose merged file cannot be read records the hash-unavailable finding"
    );
}

#[test]
fn body_hash_is_finding_order_independent_nonzero_and_bound_by_from_body() {
    // `body_hash` sorts findings before hashing, so two bodies differing only in
    // finding ORDER hash equally, while a body differing in a real column hashes
    // differently and never collapses to the all-zero digest. `from_body` must
    // bind `body_hash` to `body.body_hash()`. Kills the `sort_findings` removal,
    // `body_hash -> Ok(Default::default())`, and a mis-bound `from_body`.
    let finding_a = CompactionReportFinding::PreSwapRollback {
        reason: "a".to_string(),
    };
    let finding_b = CompactionReportFinding::OutputSegmentHashUnavailable {
        reason: "b".to_string(),
    };
    let mut ordered = body_fixture(vec![finding_a.clone(), finding_b.clone()]);
    let reordered = body_fixture(vec![finding_b, finding_a]);

    assert_eq!(
        ordered.body_hash().expect("hash a"),
        reordered.body_hash().expect("hash b"),
        "finding order must not change the body hash (findings are sorted first)"
    );
    assert_ne!(
        ordered.body_hash().expect("hash a"),
        [0u8; 32],
        "a populated body never hashes to the all-zero digest; kills `-> Ok(Default::default())`"
    );

    let bound = ordered.body_hash().expect("hash a");
    ordered.bytes_reclaimed += 1;
    assert_ne!(
        ordered.body_hash().expect("mutated hash"),
        bound,
        "changing a real column changes the body hash"
    );
    ordered.bytes_reclaimed -= 1;

    let report = CompactionEvidenceReport::from_body(ordered.clone()).expect("evidence report");
    assert_eq!(
        report.body_hash,
        ordered.body_hash().expect("hash a"),
        "from_body must bind body_hash to the body's own digest"
    );
}

#[test]
fn compaction_id_digest_is_stable_and_column_sensitive() {
    // `compaction_id` is a stable digest over the structural core. Kills
    // `compaction_id_digest -> Ok(Default::default())` (all-zero) and proves it
    // reacts to a structural column change (the fingerprint is not Clone, so two
    // are built independently through a fixture).
    let digest_nine = compaction_id_digest(&fingerprint_fixture(9)).expect("digest nine");
    let digest_ten = compaction_id_digest(&fingerprint_fixture(10)).expect("digest ten");

    assert_ne!(
        digest_nine, [0u8; 32],
        "a real fingerprint never digests to the all-zero value"
    );
    assert_ne!(
        digest_nine, digest_ten,
        "a changed structural column (active_segment_id) yields a different compaction id"
    );
    assert_eq!(
        digest_nine,
        compaction_id_digest(&fingerprint_fixture(9)).expect("digest nine again"),
        "the digest is a deterministic function of the fingerprint"
    );
}

fn fingerprint_fixture(active_segment_id: u64) -> CompactionStructuralFingerprint {
    CompactionStructuralFingerprint {
        schema_version: COMPACTION_REPORT_SCHEMA_VERSION,
        strategy_shape: CompactionStrategyShape::Merge,
        min_segments_threshold: 2,
        active_segment_id,
        sealed_segment_count: 2,
        source_segment_ids_sorted: vec![2, 7],
        merged_segment_id: Some(2),
        outcome: CompactionOutcome::Performed,
        segments_removed: 2,
        bytes_reclaimed: 4096,
    }
}

fn body_fixture(findings: Vec<CompactionReportFinding>) -> CompactionReportBody {
    CompactionReportBody {
        schema_version: COMPACTION_REPORT_SCHEMA_VERSION,
        compaction_id: [1u8; 32],
        input_segment_id_low: Some(2),
        input_segment_id_high: Some(7),
        strategy_shape: CompactionStrategyShape::Merge,
        min_segments_threshold: 2,
        active_segment_id: 9,
        sealed_segment_count: 2,
        source_segment_ids_sorted: vec![2, 7],
        merged_segment_id: Some(2),
        output_segment_bytes_hash: None,
        outcome: CompactionOutcome::Performed,
        segments_removed: 2,
        bytes_reclaimed: 4096,
        findings,
    }
}

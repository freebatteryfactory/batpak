//! PROVES: the snapshot-evidence builder carries structural truth — copied
//! segment ids are sorted ascending, the fence token / source watermark /
//! presence flags pass through verbatim, the schema version is pinned, and
//! `body_hash` is bound into the envelope. Also that `snapshot_report_body_hash`
//! is finding-order independent, non-zero, and column-sensitive, and that
//! `destination_path_digest` is a deterministic, path-sensitive content hash.
//! CATCHES: the `copied_segment_ids_sorted.sort_unstable()` removal, the
//! presence-flag / watermark / fence passthrough replacements, the
//! `sort_findings` removal, `*_body_hash -> Ok(Default::default())` (all-zero
//! digest), and `destination_path_digest -> Default::default()`.
//! SEEDED: deterministic — no randomness, no clock, no store.

use super::{
    destination_path_digest, snapshot_evidence_report, snapshot_report_body_hash,
    SnapshotFenceTokenRef, SnapshotFinding, SnapshotReportBody, SnapshotReportInput,
    SnapshotWatermarkRef, SNAPSHOT_EVIDENCE_REPORT_SCHEMA_VERSION,
};
use crate::evidence::content_hash;
use std::path::Path;

#[test]
fn snapshot_evidence_report_sorts_ids_and_passes_structural_columns_through() {
    // The report must sort copied ids ascending (not scan order), echo the fence
    // token / source watermark / presence flags exactly, pin the schema version,
    // and bind body_hash. Kills the sort removal and each column passthrough
    // replacement.
    let input = SnapshotReportInput {
        fence_token: 42,
        source_watermark_segment_id: 7,
        source_watermark_offset: 128,
        copied_segment_ids_sorted: vec![9, 2, 5],
        copied_visibility_ranges_present: true,
        copied_pending_compaction_marker_present: false,
        copied_idempotency_store_present: true,
        destination_path_digest: [3u8; 32],
        findings: vec![
            SnapshotFinding::FenceTokenCancelled,
            SnapshotFinding::DestinationCleared { artifact_count: 4 },
        ],
    };
    let report = snapshot_evidence_report(input).expect("snapshot report encodes");

    assert_eq!(
        report.body.copied_segment_ids_sorted,
        vec![2, 5, 9],
        "copied ids are sorted ascending; kills the `sort_unstable` removal"
    );
    assert_eq!(
        report.body.schema_version,
        SNAPSHOT_EVIDENCE_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.body.fence_token.token, 42);
    assert_eq!(report.body.source_watermark.segment_id, 7);
    assert_eq!(report.body.source_watermark.offset, 128);
    assert!(
        report.body.copied_visibility_ranges_present,
        "visibility-ranges presence passes through as true"
    );
    assert!(
        !report.body.copied_pending_compaction_marker_present,
        "pending-compaction presence passes through as false"
    );
    assert!(
        report.body.copied_idempotency_store_present,
        "idempotency-store presence passes through as true"
    );
    assert_eq!(
        report.body_hash,
        report.body.body_hash().expect("body hash"),
        "the envelope body_hash is bound to the body's own digest"
    );
    assert_eq!(report.generated_at_unix_ms, None);
    assert_eq!(report.batpak_version, None);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn snapshot_report_body_hash_is_finding_order_independent_nonzero_and_column_sensitive() {
    // body_hash sorts findings before hashing, so finding ORDER cannot change it,
    // a real column change must, and a populated body never hashes to zero. Kills
    // the `sort_findings` removal and `-> Ok(Default::default())`.
    let cleared = SnapshotFinding::DestinationCleared { artifact_count: 1 };
    let cancelled = SnapshotFinding::FenceTokenCancelled;

    let ordered = body_fixture(vec![cleared.clone(), cancelled.clone()], vec![1, 2]);
    let reordered = body_fixture(vec![cancelled, cleared], vec![1, 2]);
    assert_eq!(
        snapshot_report_body_hash(&ordered).expect("hash ordered"),
        snapshot_report_body_hash(&reordered).expect("hash reordered"),
        "finding order must not change the body hash"
    );
    assert_ne!(
        snapshot_report_body_hash(&ordered).expect("hash ordered"),
        [0u8; 32],
        "a populated body never hashes to the all-zero digest"
    );

    let different_ids = body_fixture(vec![], vec![9, 9]);
    assert_ne!(
        snapshot_report_body_hash(&ordered).expect("hash ordered"),
        snapshot_report_body_hash(&different_ids).expect("hash different"),
        "a changed structural column changes the body hash"
    );
}

#[test]
fn destination_path_digest_is_deterministic_and_path_sensitive() {
    // The digest is exactly blake3 over the destination path bytes. Kills a
    // constant-body `Default::default()` (all-zero) and any path-insensitive
    // replacement.
    let path = Path::new("/var/lib/batpak/snap-dest");
    assert_eq!(
        destination_path_digest(path),
        content_hash(path.as_os_str().as_encoded_bytes()),
        "the digest hashes the raw destination path bytes"
    );
    assert_ne!(
        destination_path_digest(path),
        [0u8; 32],
        "a non-empty path never digests to zero"
    );
    assert_ne!(
        destination_path_digest(Path::new("/a")),
        destination_path_digest(Path::new("/b")),
        "distinct destinations digest differently"
    );
}

fn body_fixture(findings: Vec<SnapshotFinding>, ids: Vec<u64>) -> SnapshotReportBody {
    SnapshotReportBody {
        schema_version: SNAPSHOT_EVIDENCE_REPORT_SCHEMA_VERSION,
        snapshot_id: [1u8; 32],
        fence_token: SnapshotFenceTokenRef { token: 1 },
        source_watermark: SnapshotWatermarkRef {
            segment_id: 1,
            offset: 2,
        },
        copied_segment_ids_sorted: ids,
        copied_visibility_ranges_present: true,
        copied_pending_compaction_marker_present: false,
        copied_idempotency_store_present: true,
        destination_path_digest: [2u8; 32],
        findings,
    }
}

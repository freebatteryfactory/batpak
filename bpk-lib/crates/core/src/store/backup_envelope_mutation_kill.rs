//! Mutation-cure tests for the backup manifest / restore-proof framing math.
//!
//! PROVES: the deterministic backup surface is exactly the contract it claims —
//! segment refs are sorted before hashing, the canonical body hash is real
//! (never a constant) and content-sensitive, the segment index distinguishes an
//! exact-duplicate row from an id-collision with a differing digest, the audit
//! flags an unsupported body schema, and the restore proof separates missing,
//! digest-mismatched, and unexpected segments while a hash-claim mismatch is
//! surfaced verbatim.
//! CATCHES: `sort_backup_segment_refs -> ()`; the body-hash helpers replaced
//! with `Ok(Default::default())` (all-zero digest) or `Ok(vec![])` (empty body
//! bytes, a constant hash); the `prev == seg.bytes_digest` duplicate/inconsistent
//! `==`↔`!=` swap; the audit / restore-proof `schema_version != ...` `!=`↔`==`
//! swap; the restore-proof `obs_digest != exp_digest` `!=`↔`==` swap; the
//! `!expected_map.contains_key(id)` `!`-deletion; and the
//! `computed != claimed_manifest_hash` `!=`↔`==` swap in verification.

use super::*;

/// A fixed 32-byte digest filled with `byte` (distinct bytes give distinct digests).
fn digest(byte: u8) -> SegmentBytesDigest {
    [byte; 32]
}

fn seg(segment_id: u64, digest_byte: u8) -> BackupSegmentRef {
    BackupSegmentRef {
        segment_id,
        bytes_digest: digest(digest_byte),
    }
}

fn manifest(schema_version: u32, segments: Vec<BackupSegmentRef>) -> BackupManifestBody {
    BackupManifestBody {
        schema_version,
        backup_id: digest(0xAA),
        layout_revision: 7,
        tooling_revision: 9,
        segments,
    }
}

const ZERO_DIGEST: SegmentBytesDigest = [0u8; 32];

#[test]
fn sort_backup_segment_refs_orders_ascending_by_id_then_digest() {
    // Kills `sort_backup_segment_refs -> ()`: a no-op body leaves the caller's
    // arbitrary order intact, so any downstream normalized hash would depend on
    // slice order (the module contract says it must not).
    let mut refs = vec![seg(3, 0x30), seg(1, 0x10), seg(2, 0x22), seg(2, 0x11)];
    sort_backup_segment_refs(&mut refs);

    let ids: Vec<u64> = refs.iter().map(|r| r.segment_id).collect();
    assert_eq!(
        ids,
        vec![1, 2, 2, 3],
        "segment ids must be ascending after sort"
    );
    // The two id==2 rows must be ordered by digest (0x11 before 0x22).
    assert_eq!(
        refs[1].bytes_digest,
        digest(0x11),
        "equal-id rows must break ties on bytes_digest ascending"
    );
    assert_eq!(refs[2].bytes_digest, digest(0x22));
}

#[test]
fn backup_manifest_body_hash_is_nonzero_reorder_invariant_and_content_sensitive() {
    // Kills `backup_manifest_body_hash -> Ok(Default::default())` (all-zero
    // digest) via the non-zero assert, and `backup_manifest_body_bytes ->
    // Ok(vec![])` (which would hash the empty body to a single constant) via the
    // content-sensitivity assert. The reorder-invariance pins that the segments
    // are normalized (sorted) before hashing.
    let ordered = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x10), seg(2, 0x20)],
    );
    let reordered = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(2, 0x20), seg(1, 0x10)],
    );
    let changed = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x10), seg(2, 0x21)],
    );

    let h_ordered = backup_manifest_body_hash(&ordered).expect("hash ordered");
    let h_reordered = backup_manifest_body_hash(&reordered).expect("hash reordered");
    let h_changed = backup_manifest_body_hash(&changed).expect("hash changed");

    assert_ne!(
        h_ordered, ZERO_DIGEST,
        "a real manifest body hash must never be the all-zero default digest"
    );
    assert_eq!(
        h_ordered, h_reordered,
        "caller-supplied segment order must not change the normalized body hash"
    );
    assert_ne!(
        h_ordered, h_changed,
        "changing a segment digest MUST change the body hash (empty-body-bytes mutant would collapse both to one constant)"
    );
}

#[test]
fn collect_segment_digest_index_distinguishes_duplicate_from_inconsistent() {
    // Kills the `prev == seg.bytes_digest` `==`↔`!=` swap: an exact-duplicate row
    // (same id, same digest) is a DuplicateSegmentRef, but a same-id row with a
    // DIFFERENT digest is an InconsistentSegmentId. Swapping the comparison flips
    // which finding is emitted. Also kills the whole-body `-> Default::default()`
    // (empty map, no findings) via the non-empty asserts.
    let mut dup_findings = Vec::new();
    let dup_map = collect_segment_digest_index(&[seg(5, 0x55), seg(5, 0x55)], &mut dup_findings);
    assert!(
        dup_findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::DuplicateSegmentRef { segment_id: 5, .. }
        )),
        "two byte-identical rows for id 5 must produce a DuplicateSegmentRef, got {dup_findings:?}"
    );
    assert!(
        !dup_findings
            .iter()
            .any(|f| matches!(f, BackupEnvelopeFinding::InconsistentSegmentId { .. })),
        "identical rows must NOT be reported as an id inconsistency"
    );
    assert_eq!(
        dup_map.get(&5u64),
        Some(&digest(0x55)),
        "the first digest for an id is retained in the index"
    );

    let mut bad_findings = Vec::new();
    let _bad_map = collect_segment_digest_index(&[seg(6, 0x60), seg(6, 0x61)], &mut bad_findings);
    assert!(
        bad_findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::InconsistentSegmentId {
                segment_id: 6,
                first_digest,
                second_digest,
            } if *first_digest == digest(0x60) && *second_digest == digest(0x61)
        )),
        "id 6 with two different digests must produce an InconsistentSegmentId carrying both digests, got {bad_findings:?}"
    );
    assert!(
        !bad_findings
            .iter()
            .any(|f| matches!(f, BackupEnvelopeFinding::DuplicateSegmentRef { .. })),
        "a genuine digest disagreement must NOT be reported as an exact duplicate"
    );
}

#[test]
fn audit_flags_unsupported_schema_and_duplicate_segments() {
    // Kills the audit `schema_version != BACKUP_MANIFEST_BODY_SCHEMA_VERSION`
    // `!=`↔`==` swap (an unsupported version must be flagged), the `collect...`
    // call being dropped (the duplicate must still surface), and the whole-body
    // `-> vec![]`.
    let unsupported = BACKUP_MANIFEST_BODY_SCHEMA_VERSION.wrapping_add(1);
    let findings =
        audit_backup_manifest_segments(&manifest(unsupported, vec![seg(2, 0x20), seg(2, 0x20)]));

    assert!(
        findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::UnsupportedManifestBodySchemaVersion {
                observed,
                expected,
            } if *observed == unsupported && *expected == BACKUP_MANIFEST_BODY_SCHEMA_VERSION
        )),
        "an unsupported body schema version must be flagged, got {findings:?}"
    );
    assert!(
        findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::DuplicateSegmentRef { segment_id: 2, .. }
        )),
        "the duplicate segment must still be reported alongside the schema finding"
    );
}

#[test]
fn audit_accepts_supported_schema_without_a_schema_finding() {
    // Pins the DIRECTION of the `!=` schema check: with the supported version the
    // audit must NOT emit an unsupported-schema finding. The `!=`↔`==` swap would
    // spuriously flag every supported manifest here.
    let findings = audit_backup_manifest_segments(&manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x10), seg(2, 0x20)],
    ));
    assert!(
        !findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::UnsupportedManifestBodySchemaVersion { .. }
        )),
        "a supported schema version must produce no unsupported-schema finding, got {findings:?}"
    );
}

#[test]
fn restore_proof_report_flags_missing_mismatch_and_unexpected() {
    // Kills, in `restore_proof_report_body`:
    //   * the `None =>` arm (MissingExpectedSegment) — id 1 is expected but absent
    //     from the observed set;
    //   * the `obs_digest != exp_digest` `!=`↔`==` swap — id 2 present on both
    //     sides with differing digests must be a SegmentBytesDigestMismatch;
    //   * the `!expected_map.contains_key(id)` `!`-deletion — id 3 observed but not
    //     expected must be an UnexpectedObservedSegment, while ids that ARE expected
    //     (2, 4) must NOT be flagged unexpected;
    //   * a matching row (id 4, equal digest) must produce NONE of those findings.
    // Also asserts manifest_body_hash is the real (non-zero) manifest digest,
    // killing `backup_manifest_body_hash -> Ok(Default::default())` on this path.
    let expected = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x11), seg(2, 0x22), seg(4, 0x44)],
    );
    let observed = vec![seg(2, 0x2F), seg(3, 0x33), seg(4, 0x44)];
    let report = restore_proof_report_body(&expected, &observed).expect("restore proof body");

    assert_ne!(
        report.manifest_body_hash, ZERO_DIGEST,
        "the restore proof must anchor on the real (non-zero) manifest body hash"
    );
    assert!(
        report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::MissingExpectedSegment { segment_id: 1 }
        )),
        "expected id 1 absent from observed must be MissingExpectedSegment, got {:?}",
        report.findings
    );
    assert!(
        report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::SegmentBytesDigestMismatch {
                segment_id: 2,
                expected,
                observed,
            } if *expected == digest(0x22) && *observed == digest(0x2F)
        )),
        "id 2 with differing digests must be a SegmentBytesDigestMismatch carrying both, got {:?}",
        report.findings
    );
    assert!(
        report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::UnexpectedObservedSegment { segment_id: 3 }
        )),
        "observed id 3 not in the manifest must be UnexpectedObservedSegment, got {:?}",
        report.findings
    );
    assert!(
        !report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::UnexpectedObservedSegment { segment_id: 2 | 4 }
        )),
        "expected ids (2, 4) must never be reported as unexpected (the `!contains_key` guard)"
    );
    assert!(
        !report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::MissingExpectedSegment { segment_id: 4 }
                | BackupEnvelopeFinding::SegmentBytesDigestMismatch { segment_id: 4, .. }
        )),
        "the fully-matching id 4 row must produce none of the discrepancy findings"
    );
}

#[test]
fn restore_proof_report_flags_unsupported_schema() {
    // Kills the restore-proof `expected_manifest.schema_version != ...` `!=`↔`==`
    // swap independently of the audit path (this branch lives in
    // `restore_proof_report_body`).
    let unsupported = BACKUP_MANIFEST_BODY_SCHEMA_VERSION.wrapping_add(3);
    let report =
        restore_proof_report_body(&manifest(unsupported, vec![seg(1, 0x10)]), &[seg(1, 0x10)])
            .expect("restore proof body");
    assert!(
        report.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::UnsupportedManifestBodySchemaVersion { observed, .. }
                if *observed == unsupported
        )),
        "an unsupported expected-manifest schema must be flagged, got {:?}",
        report.findings
    );
    assert_eq!(
        report.schema_version, RESTORE_PROOF_REPORT_SCHEMA_VERSION,
        "the report body must stamp its own v1 schema version, not the manifest's"
    );
}

#[test]
fn restore_proof_report_body_hash_is_nonzero_and_findings_order_invariant() {
    // Kills `restore_proof_report_body_hash -> Ok(Default::default())` via the
    // non-zero assert, and confirms the `sorted_findings` / `observed_segments`
    // normalization: the same body with its findings and observed segments
    // permuted must hash identically.
    let expected = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x11), seg(2, 0x22)],
    );
    let observed = vec![seg(2, 0x2F), seg(3, 0x33)];
    let report = restore_proof_report_body(&expected, &observed).expect("restore proof body");

    let h = restore_proof_report_body_hash(&report).expect("hash report");
    assert_ne!(
        h, ZERO_DIGEST,
        "a real restore-proof body hash must never be the all-zero default digest"
    );

    // Permute the findings and observed segments; a normalizing hash is stable.
    let mut permuted = report.clone();
    permuted.findings.reverse();
    permuted.observed_segments_sorted.reverse();
    let h_permuted = restore_proof_report_body_hash(&permuted).expect("hash permuted");
    assert_eq!(
        h, h_permuted,
        "the restore-proof body hash must normalize findings/observed order before hashing"
    );
}

#[test]
fn verify_backup_manifest_envelope_flags_claimed_hash_mismatch_both_polarities() {
    // Kills the `computed != claimed_manifest_hash` `!=`↔`==` swap: a WRONG claimed
    // hash must add a ManifestBodyHashMismatch finding, and the CORRECT claimed
    // hash must not. Asserting both polarities pins the comparison direction.
    let body = manifest(
        BACKUP_MANIFEST_BODY_SCHEMA_VERSION,
        vec![seg(1, 0x10), seg(2, 0x20)],
    );
    let envelope = BackupManifestEnvelope {
        body: body.clone(),
        envelope_schema_version: 1,
        generated_at_wall_ms: None,
        diagnostic_note: None,
        signatures: Vec::new(),
        attestations: Vec::new(),
    };
    let correct = backup_manifest_body_hash(&body).expect("correct hash");

    let matched =
        verify_backup_manifest_envelope(&envelope, correct, |_sig, _body| -> Result<(), String> {
            Ok(())
        })
        .expect("verify with correct claim");
    assert!(
        !matched
            .findings
            .iter()
            .any(|f| matches!(f, BackupEnvelopeFinding::ManifestBodyHashMismatch { .. })),
        "a correct claimed hash must NOT produce a ManifestBodyHashMismatch, got {:?}",
        matched.findings
    );

    let wrong_claim = digest(0xEE);
    let mismatched = verify_backup_manifest_envelope(
        &envelope,
        wrong_claim,
        |_sig, _body| -> Result<(), String> { Ok(()) },
    )
    .expect("verify with wrong claim");
    assert!(
        mismatched.findings.iter().any(|f| matches!(
            f,
            BackupEnvelopeFinding::ManifestBodyHashMismatch { claimed, computed }
                if *claimed == wrong_claim && *computed == correct
        )),
        "a wrong claimed hash must produce a ManifestBodyHashMismatch carrying claimed+computed, got {:?}",
        mismatched.findings
    );
}

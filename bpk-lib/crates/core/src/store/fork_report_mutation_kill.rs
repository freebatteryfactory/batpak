//! PROVES: the fork-evidence surface carries structural truth — `ForkStrategyCounts`
//! increments exactly the matching counter by one, `fork_evidence_report` sorts the
//! shared/deep-copied id sets and passes the fork columns through, `fork_report_body_hash`
//! is finding-order independent + non-zero + column-sensitive, `destination_path_digest`
//! is a deterministic path-sensitive hash, the compat wire round-trips and rejects
//! crc/version corruption, and `ForkOptions::default` excludes caches.
//! CATCHES: each `record_copy` match-arm swap and its `+= 1` arithmetic (`-=`/`*=`),
//! `record_cache_regenerable`/`record_excluded` `+= 1` mutants, the id `sort_unstable`
//! removals, the `sort_findings` removal, `*_body_hash -> Ok(Default::default())`,
//! `destination_path_digest -> Default::default()`, the decode crc `!=`->`==` and
//! version `>`->`<`/`>=` boundary mutants, and `ForkOptions::default` `exclude_caches:
//! true -> false`.
//! SEEDED: deterministic — no randomness, no clock, no store.

use super::{
    decode_fork_evidence_wire, destination_path_digest, encode_fork_evidence_wire,
    fork_evidence_report, fork_report_body_hash, CopyPreference, ForkCopyStrategy, ForkFinding,
    ForkOptions, ForkReportBody, ForkReportInput, ForkStrategyCounts,
    FORK_EVIDENCE_REPORT_SCHEMA_VERSION, FORK_EVIDENCE_WIRE_MAGIC,
};
use crate::evidence::content_hash;
use crate::store::{SnapshotFenceTokenRef, SnapshotWatermarkRef, StoreError};
use std::path::Path;

#[test]
fn record_copy_increments_exactly_the_matching_counter_by_one() {
    // Each strategy increments its OWN counter by exactly one. Distinct final
    // counts (2/1/3) convict both the match-arm mapping swaps and the `+= 1`
    // arithmetic mutants: `-= 1` underflows, `*= 1` stays 0, a mis-mapped arm
    // credits the wrong field.
    let mut counts = ForkStrategyCounts::default();
    counts.record_copy(ForkCopyStrategy::Reflink);
    counts.record_copy(ForkCopyStrategy::Reflink);
    counts.record_copy(ForkCopyStrategy::Hardlink);
    counts.record_copy(ForkCopyStrategy::DeepCopy);
    counts.record_copy(ForkCopyStrategy::DeepCopy);
    counts.record_copy(ForkCopyStrategy::DeepCopy);

    assert_eq!(counts.reflink, 2, "two reflink copies counted");
    assert_eq!(counts.hardlink, 1, "one hardlink copy counted");
    assert_eq!(counts.deep_copy, 3, "three deep copies counted");
    assert_eq!(
        counts.cache_regenerable, 0,
        "record_copy never touches the cache column"
    );
    assert_eq!(
        counts.excluded, 0,
        "record_copy never touches the excluded column"
    );
}

#[test]
fn record_cache_regenerable_and_excluded_each_increment_their_own_column_by_one() {
    // Kills the `cache_regenerable += 1` / `excluded += 1` arithmetic mutants and
    // any cross-wiring between the two counters.
    let mut counts = ForkStrategyCounts::default();
    counts.record_cache_regenerable();
    counts.record_cache_regenerable();
    counts.record_excluded();

    assert_eq!(counts.cache_regenerable, 2);
    assert_eq!(counts.excluded, 1);
    assert_eq!(counts.reflink, 0, "the copy counters stay untouched");
    assert_eq!(counts.hardlink, 0);
    assert_eq!(counts.deep_copy, 0);
}

#[test]
fn fork_evidence_report_sorts_id_sets_and_passes_columns_through() {
    // Both id sets are sorted ascending, and the fence/watermark/active-id/
    // presence/counts columns pass through verbatim, with body_hash bound. Kills
    // the two `sort_unstable` removals and each column passthrough replacement.
    let report = fork_evidence_report(sample_input()).expect("fork report encodes");

    assert_eq!(
        report.body.shared_segment_ids_sorted,
        vec![1, 4, 8],
        "shared ids sorted ascending; kills the shared `sort_unstable` removal"
    );
    assert_eq!(
        report.body.deep_copied_segment_ids_sorted,
        vec![2, 7],
        "deep-copied ids sorted ascending; kills the deep-copy `sort_unstable` removal"
    );
    assert_eq!(report.body.active_segment_id, 12);
    assert_eq!(
        report.body.schema_version,
        FORK_EVIDENCE_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.body.fence_token.token, 5);
    assert_eq!(report.body.source_watermark.segment_id, 3);
    assert_eq!(report.body.source_watermark.offset, 64);
    assert!(report.body.copied_visibility_ranges_present);
    assert!(report.body.copied_pending_compaction_marker_present);
    assert!(!report.body.copied_idempotency_store_present);
    assert_eq!(report.body.strategy_counts.excluded, 3);
    assert_eq!(report.body.strategy_counts.deep_copy, 2);
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
fn fork_report_body_hash_is_finding_order_independent_nonzero_and_column_sensitive() {
    // Kills the `sort_findings` removal, `-> Ok(Default::default())`, and proves a
    // real column change changes the hash.
    let cleared = ForkFinding::DestinationCleared { artifact_count: 1 };
    let cancelled = ForkFinding::FenceTokenCancelled;

    let ordered = body_fixture(vec![cleared.clone(), cancelled.clone()], 12);
    let reordered = body_fixture(vec![cancelled, cleared], 12);
    assert_eq!(
        fork_report_body_hash(&ordered).expect("hash ordered"),
        fork_report_body_hash(&reordered).expect("hash reordered"),
        "finding order must not change the body hash"
    );
    assert_ne!(
        fork_report_body_hash(&ordered).expect("hash ordered"),
        [0u8; 32],
        "a populated body never hashes to the all-zero digest"
    );

    let moved = body_fixture(vec![], 13);
    assert_ne!(
        fork_report_body_hash(&ordered).expect("hash ordered"),
        fork_report_body_hash(&moved).expect("hash moved"),
        "a changed structural column changes the body hash"
    );
}

#[test]
fn destination_path_digest_is_deterministic_and_path_sensitive() {
    let path = Path::new("/var/lib/batpak/fork-dest");
    assert_eq!(
        destination_path_digest(path),
        content_hash(path.as_os_str().as_encoded_bytes()),
        "the digest hashes the raw destination path bytes"
    );
    assert_ne!(destination_path_digest(path), [0u8; 32]);
    assert_ne!(
        destination_path_digest(Path::new("/a")),
        destination_path_digest(Path::new("/b")),
        "distinct destinations digest differently"
    );
}

#[test]
fn fork_evidence_wire_round_trips_and_frames_magic_then_version() {
    // A valid body encodes to `magic | version_le | crc_le | body` and decodes
    // back to an equal body. Kills a wholesale encode/decode replacement and the
    // magic/version framing order.
    let body = fork_evidence_report(sample_input())
        .expect("fork report")
        .body;
    let wire = encode_fork_evidence_wire(&body).expect("encode wire");

    assert_eq!(
        wire.get(..6),
        Some(&FORK_EVIDENCE_WIRE_MAGIC[..]),
        "the wire opens with the fork-evidence magic"
    );
    assert_eq!(
        wire.get(6..8),
        Some(&body.schema_version.to_le_bytes()[..]),
        "the schema version is little-endian at offset 6"
    );

    let decoded = decode_fork_evidence_wire(&wire).expect("decode round-trips");
    assert_eq!(
        decoded, body,
        "a clean round-trip reproduces the body exactly"
    );
}

#[test]
fn fork_evidence_wire_decode_rejects_a_crc_mismatch() {
    // Corrupting a body byte must trip the crc guard `crc32(body) != expected`.
    // The `!=`->`==` mutant would accept the tampered frame (and then fail later
    // with a different error), so we pin the Configuration crc rejection.
    let body = fork_evidence_report(sample_input())
        .expect("fork report")
        .body;
    let mut wire = encode_fork_evidence_wire(&body).expect("encode wire");
    assert!(
        wire.len() > 12,
        "the encoded body occupies bytes past the header"
    );
    wire[12] ^= 0xFF;

    let err =
        decode_fork_evidence_wire(&wire).expect_err("a tampered body must fail the crc check");
    assert!(
        matches!(err, StoreError::Configuration(_)),
        "a crc mismatch is a Configuration error; got {err:?}"
    );
}

#[test]
fn fork_evidence_wire_decode_rejects_a_future_version_but_accepts_the_supported_one() {
    // The version gate is `found > FORK_EVIDENCE_REPORT_SCHEMA_VERSION`. A frame
    // one past the supported version must be rejected as ForkEvidenceFutureVersion;
    // the `>`->`>=` mutant would also reject the supported version (breaking the
    // round-trip test above), and the `>`->`<` mutant would let the future frame
    // slip to the crc/body stage instead. We pin both the reject and the boundary.
    let future = FORK_EVIDENCE_REPORT_SCHEMA_VERSION + 1;
    let mut frame = Vec::new();
    frame.extend_from_slice(FORK_EVIDENCE_WIRE_MAGIC);
    frame.extend_from_slice(&future.to_le_bytes());
    frame.extend_from_slice(&[0u8; 4]); // crc placeholder, unreached by the version gate

    let err = decode_fork_evidence_wire(&frame)
        .expect_err("a version above the supported one must be rejected");
    assert!(
        matches!(
            err,
            StoreError::ForkEvidenceFutureVersion { found, supported }
                if found == future && supported == FORK_EVIDENCE_REPORT_SCHEMA_VERSION
        ),
        "the future-version gate reports the found/supported pair; got {err:?}"
    );
}

#[test]
fn fork_options_default_excludes_caches_and_prefers_reflink_then_hardlink() {
    // Kills `ForkOptions::default`'s `exclude_caches: true -> false` and pins the
    // default copy ladder.
    let opts = ForkOptions::default();
    assert!(
        opts.exclude_caches,
        "the default fork excludes regenerable caches"
    );
    assert_eq!(opts.copy_preference, CopyPreference::ReflinkThenHardlink);
    assert_eq!(opts.copy_preference, CopyPreference::default());
}

fn sample_input() -> ForkReportInput {
    ForkReportInput {
        fence_token: 5,
        source_watermark_segment_id: 3,
        source_watermark_offset: 64,
        active_segment_id: 12,
        shared_segment_ids_sorted: vec![8, 1, 4],
        deep_copied_segment_ids_sorted: vec![7, 2],
        strategy_counts: ForkStrategyCounts {
            reflink: 1,
            hardlink: 0,
            deep_copy: 2,
            cache_regenerable: 0,
            excluded: 3,
        },
        copied_visibility_ranges_present: true,
        copied_pending_compaction_marker_present: true,
        copied_idempotency_store_present: false,
        destination_path_digest: [9u8; 32],
        findings: vec![
            ForkFinding::FenceTokenCancelled,
            ForkFinding::DestinationCleared { artifact_count: 2 },
        ],
    }
}

fn body_fixture(findings: Vec<ForkFinding>, active_segment_id: u64) -> ForkReportBody {
    ForkReportBody {
        schema_version: FORK_EVIDENCE_REPORT_SCHEMA_VERSION,
        fork_id: [1u8; 32],
        fence_token: SnapshotFenceTokenRef { token: 1 },
        source_watermark: SnapshotWatermarkRef {
            segment_id: 1,
            offset: 2,
        },
        active_segment_id,
        shared_segment_ids_sorted: vec![1, 2],
        deep_copied_segment_ids_sorted: vec![3, 4],
        strategy_counts: ForkStrategyCounts::default(),
        copied_visibility_ranges_present: true,
        copied_pending_compaction_marker_present: false,
        copied_idempotency_store_present: true,
        destination_path_digest: [2u8; 32],
        findings,
    }
}

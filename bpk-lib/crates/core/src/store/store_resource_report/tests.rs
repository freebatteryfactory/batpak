use super::{
    restart_policy_shape, store_data_dir_identity_hash, StoreResourceReportError,
    StoreResourceRestartPolicyShape,
};
use crate::store::RestartPolicy;

#[test]
fn data_dir_identity_hash_canonicalizes_existing_path_spellings() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let raw_spelling = dir.path().join(".");
    let canonical =
        crate::store::platform::fs::canonicalize(dir.path()).expect("canonicalize temp dir");

    assert_eq!(
        store_data_dir_identity_hash(&raw_spelling),
        store_data_dir_identity_hash(&canonical)
    );
}

#[test]
fn restart_policy_shape_maps_each_variant_and_preserves_bounded_fields() {
    // PROPERTY: the shape projection maps `Once -> Once` and `Bounded -> Bounded`
    // carrying BOTH fields in the right slots. Kills the match-arm swap
    // (Once↔Bounded) and any field mix-up: `max_restarts` (u32) and `within_ms`
    // (u64) are given distinct values so swapping them, or dropping one to its
    // default, flips this assertion.
    assert_eq!(
        restart_policy_shape(&RestartPolicy::Once),
        StoreResourceRestartPolicyShape::Once,
        "Once must project to the Once shape"
    );
    assert_eq!(
        restart_policy_shape(&RestartPolicy::Bounded {
            max_restarts: 7,
            within_ms: 1234,
        }),
        StoreResourceRestartPolicyShape::Bounded {
            max_restarts: 7,
            within_ms: 1234,
        },
        "Bounded must project both fields into the right slots (no swap, no default)"
    );
}

#[test]
fn data_dir_identity_hash_is_nonzero_and_distinguishes_missing_paths() {
    // Kills `store_data_dir_identity_hash -> Default::default()` (an all-zero
    // digest): a real directory hashes to a non-zero identity. The
    // canonicalization-failed fallback (a MISSING path) must still hash the raw
    // path bytes — so two different missing paths give different, deterministic
    // digests rather than one shared constant.
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let real = store_data_dir_identity_hash(dir.path());
    assert_ne!(
        real, [0u8; 32],
        "an existing data dir must hash to a non-zero identity"
    );

    let missing_a = dir.path().join("does-not-exist-a");
    let missing_b = dir.path().join("does-not-exist-b");
    assert_eq!(
        store_data_dir_identity_hash(&missing_a),
        store_data_dir_identity_hash(&missing_a),
        "the missing-path fallback must be deterministic for one path"
    );
    assert_ne!(
        store_data_dir_identity_hash(&missing_a),
        store_data_dir_identity_hash(&missing_b),
        "distinct missing paths must hash distinctly (raw path-bytes fallback, not a constant)"
    );
}

#[test]
fn store_resource_report_error_display_carries_the_encoding_message() {
    // Kills `StoreResourceReportError::Display -> Ok(())` (empty render) and a
    // wrong format string: the human-readable encoding message must appear.
    let error = StoreResourceReportError::BodyEncoding {
        message: "msgpack: unexpected end of buffer".to_owned(),
    };
    let rendered = error.to_string();
    assert!(
        rendered.contains("msgpack: unexpected end of buffer"),
        "the encoding error message must be surfaced in Display, got {rendered:?}"
    );
    assert!(
        rendered.contains("store resource report body encoding failed"),
        "the Display prefix must name the failing operation, got {rendered:?}"
    );
}

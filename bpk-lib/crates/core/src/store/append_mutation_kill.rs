//! Mutation-kill tests for the append-path value constructors and the
//! signing-downgrade receipt-extension round trip.
//!
//! PROVES: `AppendPositionHint::branch_root` flags a fork at the parent depth;
//! `CausationRef::absolute_typed` erases a typed id to the `Absolute` variant;
//! `CompactionConfig::default` seeds the 2-segment floor; the small
//! `AppendOptions`/`ReceiptExtensionKey` setters wire exactly their own field;
//! and a `SigningDowngradeBody` encodes into, and decodes back out of, a
//! receipt's reserved extension slot.
//! CATCHES: `branch_root: true -> false` and the `depth` assignment; the
//! `Absolute` body replacement; the `min_segments` literal `2`; the setter
//! `-> self` no-ops; the `<prefix>.<field>` composition; and
//! `AppendReceipt::signing_downgrade -> None` plus the cover-build reason /
//! `SIGNING_DOWNGRADE_SCHEMA_VERSION` wiring.

use super::*;

#[test]
fn append_position_hint_branch_root_forks_at_parent_depth() {
    // CATCHES: `branch_root: true -> false`; a dropped `depth: parent_depth`
    // assignment; and a lane<->depth argument swap.
    let forked = AppendPositionHint::branch_root(5, 9);
    assert_eq!(forked.lane, 5, "lane is the first argument");
    assert_eq!(
        forked.depth, 9,
        "branch_root keeps the parent depth verbatim"
    );
    assert!(
        forked.branch_root,
        "PROPERTY: branch_root() must flag a new branch"
    );
    // Dual direction: the plain constructor must NOT set branch_root, so the
    // flag is pinned in both states.
    let flat = AppendPositionHint::new(5, 9);
    assert!(
        !flat.branch_root,
        "PROPERTY: new() must leave branch_root clear"
    );
}

#[test]
fn causation_ref_absolute_typed_erases_to_absolute_variant() {
    // CATCHES: the whole-body replacement of absolute_typed (e.g. -> None).
    let reference = CausationRef::absolute_typed(CausationId::from(0x00C0_FFEE_u128));
    assert_eq!(
        reference,
        CausationRef::Absolute(0x00C0_FFEE),
        "PROPERTY: absolute_typed must wrap the id in the Absolute variant"
    );
}

#[test]
fn compaction_config_default_seeds_two_segment_floor() {
    // CATCHES: the `min_segments: 2` literal replaced (a Default::default body
    // would yield 0, and a `1`/`3` swap flips this too).
    assert_eq!(
        CompactionConfig::default().min_segments,
        2,
        "PROPERTY: the default compaction floor is 2 sealed segments"
    );
}

#[test]
fn append_options_setters_wire_exactly_their_field() {
    // CATCHES: the `-> self` no-op mutants on with_cas / with_flags /
    // with_idempotency.
    let opts = AppendOptions::default()
        .with_cas(11)
        .with_flags(0b0000_0101)
        .with_idempotency(IdempotencyKey::from(0x1234_u128));
    assert_eq!(
        opts.expected_sequence,
        Some(11),
        "with_cas sets expected_sequence"
    );
    assert_eq!(opts.flags, 0b0000_0101, "with_flags sets flags");
    assert_eq!(
        opts.idempotency_key,
        Some(IdempotencyKey::from(0x1234_u128)),
        "with_idempotency sets the key"
    );
}

#[test]
fn receipt_extension_key_composes_prefix_dot_field() {
    // CATCHES: the `<PREFIX>.<field>` composition inside ReceiptExtensionKey::new.
    struct AcmeNamespace;
    impl ReceiptExtensionNamespace for AcmeNamespace {
        const PREFIX: &'static str = "acme";
    }

    let key = ReceiptExtensionKey::<AcmeNamespace>::new("thing").expect("valid composed key");
    assert_eq!(
        key.as_key().as_str(),
        "acme.thing",
        "PROPERTY: the composed key is `<prefix>.<field>`"
    );
}

#[test]
fn signing_downgrade_round_trips_through_the_reserved_extension_slot() {
    // CATCHES: `AppendReceipt::signing_downgrade -> None` (whole body); the
    // cover_build_failed reason wiring; and the SIGNING_DOWNGRADE_SCHEMA_VERSION
    // constant being altered.
    let body = SigningDowngradeBody::cover_build_failed("cover boom");
    let encoded = body.encode_extension().expect("encode downgrade body");
    let mut extensions = BTreeMap::new();
    extensions.insert(signing_downgrade_extension_key(), encoded);

    let receipt = AppendReceipt {
        event_id: EventId::from_u128(1),
        global_sequence: 0,
        disk_pos: DiskPos::new(0, 0, 0),
        content_hash: [0u8; 32],
        key_id: [0u8; 32],
        signature: None,
        extensions,
    };
    let decoded = receipt
        .signing_downgrade()
        .expect("PROPERTY: a present downgrade extension must decode");
    assert_eq!(
        decoded.schema_version, SIGNING_DOWNGRADE_SCHEMA_VERSION,
        "PROPERTY: the decoded body carries the current schema version"
    );
    assert!(
        matches!(
            decoded.reason,
            SigningDowngradeReason::CoverBuildFailed { encoding_error }
                if encoding_error == "cover boom"
        ),
        "PROPERTY: the decoded reason must preserve the cover-build failure text"
    );

    // A receipt with no extensions has no downgrade evidence — the absent path
    // must decode to None, not a spurious body.
    let bare = AppendReceipt {
        event_id: EventId::from_u128(2),
        global_sequence: 1,
        disk_pos: DiskPos::new(0, 0, 0),
        content_hash: [0u8; 32],
        key_id: [0u8; 32],
        signature: None,
        extensions: BTreeMap::new(),
    };
    assert!(
        bare.signing_downgrade().is_none(),
        "PROPERTY: no downgrade extension means signing_downgrade() is None"
    );
}

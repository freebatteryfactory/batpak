//! Unit tests for the in-memory durable idempotency store + v2 image flush
//! (window-priority eviction law, flush/load binding round-trip, format
//! identity pins). Split from `idemp.rs` via `#[path]` to keep the inline
//! test island within the structural size cap.

// These unit tests prove the window-priority eviction rule directly on
// synthetic entries — the correctness-critical eviction property in
// isolation from the store machinery.
use super::*;

fn entry(key: u128, recorded_global_sequence: u64) -> IdempEntry {
    IdempEntry {
        key,
        event_id: key,
        global_sequence: recorded_global_sequence,
        disk_pos_segment: 0,
        disk_pos_offset: 0,
        disk_pos_length: 0,
        content_hash: [0u8; 32],
        prev_hash: [0u8; 32],
        entity: "e".to_owned(),
        scope: "s".to_owned(),
        kind: EventKind::custom(0xB, 1),
        recorded_global_sequence,
        event_evicted: false,
        receipt_extensions: BTreeMap::new(),
    }
}

#[test]
fn cap_never_evicts_within_window_keys_even_under_residual_pigeonhole() {
    // Window keeps last 100 sequences; cap is only 3. Record 10 keys all
    // within the window (recorded at 90..100, frontier 100). The cap MUST
    // NOT evict any of them: within-window keys are inviolable.
    let store = IdempotencyStore::new(
        IdempotencyRetention::Hybrid {
            keep_sequences: 100,
            max_keys: 3,
        },
        OverflowPolicy::Warn,
    );
    for i in 0..10u128 {
        let seq = u64::try_from(i).expect("loop index 0..10 fits u64");
        store.record(entry(i, 90 + seq));
    }
    let report = store.evict(100);
    assert!(report.within_window_exceeds_cap, "residual pigeonhole");
    assert_eq!(report.aged_out, 0);
    assert_eq!(report.cap_trimmed_out_of_window, 0);
    assert_eq!(report.remaining, 10, "all within-window keys survive");
    assert!((0..10u128).all(|i| store.get(i).is_some()));
}

#[test]
fn window_aging_trims_only_out_of_window_keys() {
    let store = IdempotencyStore::new(
        IdempotencyRetention::Window { keep_sequences: 10 },
        OverflowPolicy::Warn,
    );
    // Old keys at sequences 0..5, recent keys at 95..100; frontier 100, so
    // window floor is 90. Old keys age out; recent keys remain.
    for i in 0..5u128 {
        store.record(entry(
            i,
            u64::try_from(i).expect("loop index 0..5 fits u64"),
        ));
    }
    for i in 95..100u128 {
        store.record(entry(
            i,
            u64::try_from(i).expect("loop index 95..100 fits u64"),
        ));
    }
    let report = store.evict(100);
    assert_eq!(report.aged_out, 5, "five out-of-window keys aged out");
    assert_eq!(report.remaining, 5, "five within-window keys remain");
    for i in 0..5u128 {
        assert!(store.get(i).is_none(), "aged-out key {i} is gone");
    }
    assert!((95..100u128).all(|i| store.get(i).is_some()), "kept");
}

#[test]
fn unbounded_never_evicts() {
    let store = IdempotencyStore::new(IdempotencyRetention::Unbounded, OverflowPolicy::Warn);
    for i in 0..50u128 {
        store.record(entry(
            i,
            u64::try_from(i).expect("loop index 0..50 fits u64"),
        ));
    }
    let report = store.evict(1_000_000);
    assert_eq!(report.aged_out, 0);
    assert_eq!(report.cap_trimmed_out_of_window, 0);
    assert_eq!(report.remaining, 50);
}

fn fail_closed(keep_sequences: u64, max_keys: u64) -> IdempotencyStore {
    IdempotencyStore::new(
        IdempotencyRetention::Hybrid {
            keep_sequences,
            max_keys,
        },
        OverflowPolicy::FailClosed,
    )
}

#[test]
fn admit_new_key_fail_closed_refuses_only_new_keys_over_cap() {
    let store = fail_closed(1000, 2);
    store.record(entry(1, 1));
    store.record(entry(2, 2));
    // Re-recording an existing key is admitted; a new key over the cap is not
    // (both held keys are within window at frontier 2).
    assert!(store.admit_new_key(1, 2).is_ok());
    assert!(matches!(
        store.admit_new_key(99, 2),
        Err(StoreError::IdempotencyOverflowFailClosed { .. })
    ));
}

#[test]
fn admit_new_key_ages_out_of_window_before_fail_closing() {
    // Both held keys are OUT of window at frontier 1000 (floor 990). A new
    // key must age them out first and be admitted, not refused.
    let store = fail_closed(10, 2);
    store.record(entry(1, 1));
    store.record(entry(2, 2));
    assert!(
        store.admit_new_key(99, 1000).is_ok(),
        "PROPERTY: out-of-window keys must age out before a fresh key is refused"
    );
    assert_eq!(store.len(), 0, "stale keys were aged out by admission");
}

#[test]
fn admit_new_keys_validates_the_batch_as_a_unit() {
    // (1) Duplicate new keys within one batch are rejected (they would
    // otherwise derive duplicate event ids).
    let dup_store = fail_closed(1000, 1000);
    let dup_err = dup_store
        .admit_new_keys([7u128, 7u128].into_iter(), 0)
        .expect_err("PROPERTY: two identical new keys in a batch must be rejected");
    assert!(
        matches!(dup_err, StoreError::IdempotencyPartialBatch { .. }),
        "duplicate new key must surface IdempotencyPartialBatch, got {dup_err:?}"
    );

    // (2) Cap 3 with 2 within-window keys held: a batch of 2 unique NEW keys
    // reaches 4 > 3 and is rejected as a unit even though each alone passes a
    // per-item len check; a batch that fits (1 new => 3 == cap) is admitted.
    let store = fail_closed(1000, 3);
    store.record(entry(1, 1));
    store.record(entry(2, 2));
    let err = store
        .admit_new_keys([10u128, 11u128].into_iter(), 2)
        .expect_err("PROPERTY: a unique-new batch exceeding the cap must be rejected");
    assert!(
        matches!(err, StoreError::IdempotencyOverflowFailClosed { .. }),
        "over-cap unique batch must surface IdempotencyOverflowFailClosed, got {err:?}"
    );
    assert!(store.admit_new_keys([10u128].into_iter(), 2).is_ok());
}

#[test]
fn mark_evicted_flags_only_dropped_events_and_preserves_tuple() {
    let store = IdempotencyStore::new(IdempotencyRetention::Unbounded, OverflowPolicy::Warn);
    let mut live = entry(1, 1);
    live.disk_pos_segment = 7;
    live.global_sequence = 1;
    let mut dropped = entry(2, 2);
    dropped.disk_pos_segment = 9;
    dropped.global_sequence = 2;
    dropped.content_hash = [0xAB; 32];
    store.record(live);
    store.record(dropped);

    // Event 1 stays live; event 2's frame was dropped.
    store.mark_evicted(|event_id| event_id == 1);

    let one = store.get(1).expect("event 1 was recorded and is live");
    assert!(!one.is_event_evicted() && !one.event_evicted);
    assert_eq!(one.disk_pos_segment, 7);

    let two = store.get(2).expect("event 2 was recorded");
    assert!(two.is_event_evicted() && two.event_evicted);
    // disk_pos is UNCHANGED after eviction: the reconstructed receipt must
    // carry the ORIGINAL frame position. The rest of the tuple is preserved.
    assert_eq!(two.disk_pos_segment, 9, "disk_pos unchanged after eviction");
    assert_eq!(two.global_sequence, 2, "sequence preserved");
    assert_eq!(two.content_hash, [0xAB; 32], "content hash preserved");
}

#[test]
fn flush_then_load_roundtrips_with_binding() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let store = IdempotencyStore::new(IdempotencyRetention::default(), OverflowPolicy::Warn);
    store.record(entry(7, 7));
    store.record(entry(8, 8));
    let lineage = 0xF00D_u128;
    let image_id = 0xA11CE_u128;
    let anchor = crate::store::store_meta::IdempAuthorityAnchor {
        covered_global_sequence: 8,
        event_id_at: 8,
        chain_commitment: [8; 32],
    };
    store
        .flush(
            dir.path(),
            &crate::store::platform::fs::RealFs,
            lineage,
            Some(anchor),
            image_id,
        )
        .expect("flush idempotency store to disk");
    // The FINALIZED authorization token admits the image.
    let finalized = crate::store::store_meta::IdempAuthorityExpectation {
        current_image_id: Some(image_id),
        pending_image_id: None,
        anchor,
    };
    let loaded = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(lineage),
        Some(&finalized),
    )
    .expect("read back the flushed idempotency image");
    assert!(matches!(&loaded, IdempLoad::Loaded(e) if e.len() == 2));
    // The PENDING token also admits — the publish-before-finalize crash window.
    let pending = crate::store::store_meta::IdempAuthorityExpectation {
        current_image_id: Some(0x01D),
        pending_image_id: Some(image_id),
        anchor,
    };
    let via_pending = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(lineage),
        Some(&pending),
    )
    .expect("a pending-token image is the legal publish crash window");
    assert!(matches!(&via_pending, IdempLoad::Loaded(e) if e.len() == 2));
    // A foreign lineage is refused even with the authorized token (#189).
    let foreign = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(lineage + 1),
        Some(&finalized),
    );
    assert!(
        matches!(
            foreign,
            Err(StoreError::IdempotencyAuthorityForeign {
                kind: crate::store::IdempAuthorityForeignKind::Lineage,
            })
        ),
        "a lineage mismatch must refuse the image"
    );
    // An UNAUTHORIZED token behind the recorded anchor classifies as stale
    // (an old self image restored from backup).
    let unknown_token_behind = crate::store::store_meta::IdempAuthorityExpectation {
        current_image_id: Some(0xDEAD),
        pending_image_id: None,
        anchor: crate::store::store_meta::IdempAuthorityAnchor {
            covered_global_sequence: 9,
            event_id_at: 9,
            chain_commitment: [9; 32],
        },
    };
    let stale = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(lineage),
        Some(&unknown_token_behind),
    );
    assert!(
        matches!(
            stale,
            Err(StoreError::IdempotencyAuthorityStale {
                image_covered: 8,
                expected_covered: 9,
            })
        ),
        "an unauthorized image behind the recorded anchor refuses as stale"
    );
    // An UNAUTHORIZED token at the SAME anchor is a sibling transplant —
    // frontier arithmetic and anchor equality must never admit it (#189, the
    // farther-ahead/anchor-identical sibling hole).
    let unknown_token_equal_anchor = crate::store::store_meta::IdempAuthorityExpectation {
        current_image_id: Some(0xDEAD),
        pending_image_id: None,
        anchor,
    };
    let transplant = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(lineage),
        Some(&unknown_token_equal_anchor),
    );
    assert!(
        matches!(
            transplant,
            Err(StoreError::IdempotencyAuthorityForeign {
                kind: crate::store::IdempAuthorityForeignKind::HistoryAnchor,
            })
        ),
        "an unauthorized anchor-identical image refuses as a sibling transplant"
    );
}

#[test]
fn future_version_header_constants_are_stable() {
    // Format identity guard: magic + current version must not drift
    // silently. A future-version on-disk file (version > IDEMP_VERSION) is
    // exercised end-to-end as a hard error in the corruption-recovery
    // integration suite (load_idempotency_authority is crate-private).
    assert_eq!(IDEMP_MAGIC, b"FBATID");
    assert_eq!(IDEMP_VERSION, 2);
}

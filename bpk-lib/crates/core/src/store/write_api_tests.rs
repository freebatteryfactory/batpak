use super::*;
use tempfile::TempDir;

#[test]
fn append_submission_waits_behind_lifecycle_gate() {
    let dir = TempDir::new().expect("temp dir");
    let store = Arc::new(Store::open(StoreConfig::new(dir.path())).expect("open store"));
    let lifecycle = store.lifecycle_gate.lock();
    let coord = Coordinate::new("entity:lifecycle-gated", "scope:test").expect("coord");
    let (started_tx, started_rx) = flume::bounded(1);
    let (done_tx, done_rx) = flume::bounded(1);
    let worker_store = Arc::clone(&store);

    let worker = std::thread::Builder::new()
        .name("batpak-lifecycle-gate-regression".into())
        .spawn(move || {
            started_tx.send(()).expect("notify started");
            let result = worker_store.append(
                &coord,
                EventKind::DATA,
                &serde_json::json!({"blocked": true}),
            );
            done_tx.send(result).expect("send append result");
        })
        .expect("spawn append worker");

    started_rx.recv().expect("worker started");
    assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err(),
            "PROPERTY: writer submissions must not pass the lifecycle gate while compaction/snapshot/close owns it"
        );

    drop(lifecycle);
    drop(
        done_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("append completes after lifecycle gate opens")
            .expect("append succeeds"),
    );
    worker.join().expect("append worker joins");
}

#[test]
fn append_rejects_reserved_kinds_and_admits_data() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let coord = Coordinate::new("entity:reserved", "scope:test").expect("coord");
    let payload = serde_json::json!({"forged": true});

    // append() with a reserved system marker is rejected with index: None.
    let err = store
        .append(&coord, EventKind::SYSTEM_BATCH_BEGIN, &payload)
        .expect_err("PROPERTY: append must reject reserved system kinds");
    assert!(
            matches!(
                err,
                StoreError::ReservedKind {
                    index: None,
                    kind
                } if kind == EventKind::SYSTEM_BATCH_BEGIN.as_raw_u16()
            ),
            "PROPERTY: reserved single-event append must surface ReservedKind {{ index: None }}, got {err:?}"
        );

    // append_with_options() with TOMBSTONE and EFFECT_ERROR are rejected too.
    for reserved in [EventKind::TOMBSTONE, EventKind::EFFECT_ERROR] {
        let err = store
            .append_with_options(&coord, reserved, &payload, AppendOptions::default())
            .expect_err("PROPERTY: append_with_options must reject reserved kinds");
        assert!(
            matches!(
                err,
                StoreError::ReservedKind { index: None, kind } if kind == reserved.as_raw_u16()
            ),
            "PROPERTY: reserved append_with_options must surface ReservedKind, got {err:?}"
        );
    }

    // DATA still appends successfully through the same funnel.
    drop(
        store
            .append(&coord, EventKind::DATA, &payload)
            .expect("PROPERTY: DATA append must still succeed after the reserved-kind guard"),
    );

    store.close().expect("close store");
}

#[test]
fn append_batch_rejects_reserved_item_and_admits_clean_batch() {
    use crate::store::append::{BatchAppendItem, CausationRef};

    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let coord = Coordinate::new("entity:reserved-batch", "scope:test").expect("coord");
    let payload = serde_json::json!({"n": 1});

    let forged = BatchAppendItem::new(
        coord.clone(),
        EventKind::SYSTEM_BATCH_COMMIT,
        &payload,
        AppendOptions::default(),
        CausationRef::None,
    )
    .expect("build forged batch item");
    let result = store.append_batch(vec![forged]);
    assert!(
        matches!(
            result,
            Err(StoreError::ReservedKind { index: Some(0), kind })
                if kind == EventKind::SYSTEM_BATCH_COMMIT.as_raw_u16()
        ),
        "PROPERTY: reserved batch item must surface ReservedKind {{ index: Some(0) }}"
    );

    let clean = BatchAppendItem::new(
        coord.clone(),
        EventKind::DATA,
        &payload,
        AppendOptions::default(),
        CausationRef::None,
    )
    .expect("build clean batch item");
    store
        .append_batch(vec![clean])
        .expect("PROPERTY: a clean batch must still commit after the reserved-kind guard");

    store.close().expect("close store");
}

#[test]
fn guard_typed_payload_version_rejects_zero_and_admits_nonzero() {
    // CATCHES: the `version == 0` guard swapped to `!=` (which would admit the
    // reserved 0 sentinel and reject real versions); the whole-body `-> Ok(())`
    // (which would admit version 0); and the `kind` field of the
    // InvalidPayloadVersion error being replaced.
    let kind = EventKind::custom(0xC, 7);
    let err = Store::<Open>::guard_typed_payload_version(kind, 0)
        .expect_err("PROPERTY: the reserved version-0 sentinel must be rejected");
    assert!(
            matches!(
                err,
                StoreError::InvalidPayloadVersion { kind: raw } if raw == kind.as_raw_u16()
            ),
            "PROPERTY: version 0 must surface InvalidPayloadVersion carrying the exact kind, got {err:?}"
        );

    // A genuine (non-zero) typed version passes the guard cleanly at both the
    // lower boundary and the ceiling.
    Store::<Open>::guard_typed_payload_version(kind, 1)
        .expect("PROPERTY: a non-zero payload version must pass the guard");
    Store::<Open>::guard_typed_payload_version(EventKind::DATA, u16::MAX)
        .expect("PROPERTY: the maximum payload version must also pass");
}

#[test]
fn submit_batch_rejects_item_exceeding_single_append_cap() {
    use crate::store::append::{BatchAppendItem, CausationRef};

    // CATCHES: the `size > per_item_cap` gate in submit_batch replaced/negated
    // (whole-body early return, `>` -> `<`/`>=`), and the index/limit fields of
    // the BatchItemTooLarge error.
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path()).with_single_append_max_bytes(64))
        .expect("open store");
    let coord = Coordinate::new("entity:batch-cap", "scope:test").expect("coord");

    // An item whose serialized payload blows past the 64-byte per-item cap.
    let too_big = BatchAppendItem::new(
        coord.clone(),
        EventKind::DATA,
        &serde_json::json!({ "blob": "x".repeat(256) }),
        AppendOptions::default(),
        CausationRef::None,
    )
    .expect("build oversized batch item");
    let err = store
        .submit_batch(vec![too_big])
        .err()
        .expect("PROPERTY: an item over single_append_max_bytes must be rejected");
    assert!(
        matches!(
            err,
            StoreError::BatchItemTooLarge { index: 0, size, limit: 64 } if size > 64
        ),
        "PROPERTY: an oversized batch item must surface BatchItemTooLarge \
             {{ index: 0, limit: 64, size > 64 }}, got {err:?}"
    );

    // A small item under the same cap still passes the per-item gate and
    // commits — proving the guard is a bound, not a blanket reject.
    let small = BatchAppendItem::new(
        coord,
        EventKind::DATA,
        &serde_json::json!({ "i": 1 }),
        AppendOptions::default(),
        CausationRef::None,
    )
    .expect("build small batch item");
    let receipts = store
        .submit_batch(vec![small])
        .expect("PROPERTY: an item under the cap must pass the size gate")
        .wait()
        .expect("PROPERTY: the small batch must commit");
    assert_eq!(
        receipts.len(),
        1,
        "the small batch commits exactly one event"
    );
    store.close().expect("close store");
}

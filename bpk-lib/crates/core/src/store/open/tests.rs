use super::*;
use tempfile::TempDir;

#[test]
fn next_active_segment_id_is_one_past_latest_existing_segment() -> Result<(), StoreError> {
    let dir = TempDir::new()?;
    crate::store::platform::fs::write_derivative_file_atomically(
        dir.path(),
        &dir.path().join(segment::segment_filename(1)),
        "test segment",
        b"",
    )?;
    crate::store::platform::fs::write_derivative_file_atomically(
        dir.path(),
        &dir.path().join(segment::segment_filename(7)),
        "test segment",
        b"",
    )?;

    assert_eq!(
        next_active_segment_id(dir.path())?,
        8,
        "PROPERTY: reader active segment must be one past the highest existing segment so the last sealed segment remains mmap-eligible"
    );
    Ok(())
}

/// GAUNTLET (#63 scheduler seam): the COOPERATIVE writer drive runs with ZERO
/// OS writer threads — it is pumped inline at the reply funnel — while the
/// THREADED drive spawns the writer as before.
///
/// CATCHES: a cooperative path that silently falls back to spawning a thread
/// (the deadlock the recovery.rs scheduler-note used to defer). This is the red
/// fixture for the seam: the threaded branch first proves the spawn counter
/// actually ticks (`>= 1`), so a cooperative regression to spawning would fail
/// the `== 0` assertion rather than pass vacuously.
#[cfg(feature = "dangerous-test-hooks")]
#[test]
fn cooperative_open_spawns_zero_writer_threads_but_threaded_spawns_one() {
    use crate::store::platform::spawn::{JobHandle, Spawn, ThreadSpawn};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Counts every Spawn::spawn call, then delegates to a real ThreadSpawn so
    // the threaded join contract still holds end-to-end.
    struct CountingSpawn {
        count: Arc<AtomicUsize>,
        inner: ThreadSpawn,
    }
    impl Spawn for CountingSpawn {
        fn spawn(
            &self,
            name: String,
            stack_size: Option<usize>,
            body: Box<dyn FnOnce() + Send + 'static>,
        ) -> Result<Box<dyn JobHandle>, crate::store::platform::spawn::SpawnError> {
            self.count.fetch_add(1, Ordering::Release);
            self.inner.spawn(name, stack_size, body)
        }
    }

    // Threaded baseline: opening spawns the writer thread (counter must tick —
    // this is what proves the fixture would catch a cooperative spawn-fallback).
    let threaded_count = Arc::new(AtomicUsize::new(0));
    let threaded_dir = TempDir::new().expect("temp dir");
    let threaded_spawner: Arc<dyn Spawn> = Arc::new(CountingSpawn {
        count: Arc::clone(&threaded_count),
        inner: ThreadSpawn,
    });
    let threaded =
        Store::open(StoreConfig::new(threaded_dir.path()).with_spawner(threaded_spawner))
            .expect("open threaded");
    threaded.close().expect("close threaded");
    assert!(
        threaded_count.load(Ordering::Acquire) >= 1,
        "PROPERTY: the THREADED writer must spawn at least one OS thread (proves the counter bites)"
    );

    // Cooperative: opening spawns NOTHING; the writer is driven inline by the
    // reply-funnel pump. A real append proves the inline drive actually runs.
    let coop_count = Arc::new(AtomicUsize::new(0));
    let coop_dir = TempDir::new().expect("temp dir");
    let coop_spawner: Arc<dyn Spawn> = Arc::new(CountingSpawn {
        count: Arc::clone(&coop_count),
        inner: ThreadSpawn,
    });
    let coop =
        Store::open_cooperative(StoreConfig::new(coop_dir.path()).with_spawner(coop_spawner))
            .expect("open cooperative");
    let coord = Coordinate::new("entity:coop-seam", "scope:test").expect("coord");
    let _ = coop
        .append(
            &coord,
            EventKind::custom(0xC, 0x0B),
            &serde_json::json!({ "x": 1 }),
        )
        .expect("cooperative append drives inline");
    coop.close().expect("close cooperative");
    assert_eq!(
        coop_count.load(Ordering::Acquire),
        0,
        "PROPERTY: the COOPERATIVE writer must spawn ZERO OS threads — it runs inline via the reply-funnel pump"
    );
}

#[test]
fn highest_index_hlc_reports_non_origin_point_for_appended_entry() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let coord = Coordinate::new("entity:highest-hlc", "scope:test").expect("coord");
    let receipt = store
        .append(
            &coord,
            EventKind::custom(0xF, 0x77),
            &serde_json::json!({"x": 1}),
        )
        .expect("append");

    let point = highest_index_hlc(&store.index);

    assert_eq!(
        point.global_sequence, receipt.global_sequence,
        "PROPERTY: highest_index_hlc must observe the committed entry's global sequence"
    );
    assert!(
        point > HlcPoint::ORIGIN,
        "PROPERTY: highest_index_hlc must not collapse a non-empty index to origin/default"
    );

    store.close().expect("close");
}

/// The at-open recompute DECISION is exercised directly: a default `Crc` store
/// skips the rehash and a `Recompute` store over untampered data accepts. The
/// fail-closed branch maps a non-intact report to
/// [`StoreError::ChainVerificationFailed`] carrying both integrity counts.
///
/// Forging a CRC-valid-but-content-mismatched frame is not reachable through the
/// public API (the per-frame CRC drops a torn frame before `verify_chain` ever
/// sees it), so the fail-closed path is proven through the pure decision helper
/// rather than on-disk tampering.
#[test]
fn run_open_chain_verification_decides_crc_skip_recompute_pass_and_fail_closed(
) -> Result<(), StoreError> {
    let dir = TempDir::new()?;
    // Crc (default): the helper must be a no-op that never rejects the open.
    {
        let store = Store::open(StoreConfig::new(dir.path()))?;
        run_open_chain_verification(&store)?;
        let _ = store.append(
            &Coordinate::new("alice", "scope")?,
            EventKind::custom(0xF, 0x012),
            &serde_json::json!({ "n": 1 }),
        )?;
        store.close()?;
    }
    // Recompute over the untampered store: the helper recomputes and accepts.
    let store = Store::open(
        StoreConfig::new(dir.path()).with_chain_verification(ChainVerification::Recompute),
    )?;
    run_open_chain_verification(&store)?;
    store.close()?;

    // Pure report -> error mapping: intact verifies, non-intact fails closed
    // with both counts surfaced.
    use crate::id::EventId;
    assert!(
        chain_verification_failure(&ChainVerificationReport::default()).is_none(),
        "an intact report must not fail the open"
    );
    let tampered = ChainVerificationReport {
        events_checked: 4,
        content_hash_mismatches: vec![EventId::from_u128(1), EventId::from_u128(2)],
        dangling_links: vec![EventId::from_u128(3)],
    };
    let failure = chain_verification_failure(&tampered);
    assert!(
        matches!(
            failure,
            Some(StoreError::ChainVerificationFailed {
                content_hash_mismatches: 2,
                dangling_links: 1,
            })
        ),
        "a non-intact report must fail closed with both integrity counts, got {failure:?}"
    );
    Ok(())
}

#[test]
fn last_close_hlc_reports_the_recorded_close_point_not_default_or_open() -> Result<(), StoreError> {
    let dir = TempDir::new()?;
    // Session 1: append a user event, then close (writes SYSTEM_CLOSE_COMPLETED).
    {
        let store = Store::open(StoreConfig::new(dir.path()))?;
        let _ = store.append(
            &Coordinate::new("entity:lc", "scope:lc")?,
            EventKind::custom(0xF, 0x33),
            &serde_json::json!({ "n": 1 }),
        )?;
        store.close()?;
    }
    // Session 2: reopen. The index now holds the prior SYSTEM_CLOSE_COMPLETED plus
    // a NEW SYSTEM_OPEN_COMPLETED at a strictly HIGHER sequence.
    let store = Store::open(StoreConfig::new(dir.path()))?;
    let expected = store
        .index
        .all_entries()
        .into_iter()
        .filter(|e| e.kind == EventKind::SYSTEM_CLOSE_COMPLETED)
        .map(|e| HlcPoint {
            wall_ms: e.wall_ms,
            global_sequence: e.global_sequence,
        })
        .reduce(HlcPoint::max_by_sequence)
        .expect("a close event must be present after reopen");
    let got = last_close_hlc(&store.index)?;
    // Kills `Ok(Default::default())` (origin) and the `== -> !=` kind filter
    // (which would select the newer, higher-sequence OPEN event instead).
    assert_eq!(
        got, expected,
        "last_close_hlc must return the recorded SYSTEM_CLOSE_COMPLETED point"
    );
    assert!(got.global_sequence > 0, "the close point is not the origin");
    let max_all = highest_index_hlc(&store.index);
    assert!(
        max_all.global_sequence > got.global_sequence,
        "premise: a later OPEN event outranks the close, so a kind-filter flip is observable"
    );
    store.close()?;
    Ok(())
}

#[test]
fn validate_bootstrap_hlc_rejects_open_below_either_floor() {
    let open = HlcPoint {
        wall_ms: 5,
        global_sequence: 5,
    };
    let higher_recovered = HlcPoint {
        wall_ms: 10,
        global_sequence: 10,
    };
    let lower_close = HlcPoint {
        wall_ms: 1,
        global_sequence: 1,
    };
    // open_hlc is below max_recovered but ABOVE last_close: the `||` guard must
    // still reject. Kills `|| -> &&` (which needs BOTH violated) and the
    // whole-body `-> Ok(())` stub.
    let result = validate_bootstrap_hlc(open, higher_recovered, lower_close);
    assert!(
        matches!(
            result,
            Err(StoreError::InvariantViolation {
                kind: StoreInvariant::BootstrapHlcOutOfOrder { .. }
            })
        ),
        "an open_hlc below max_recovered_hlc must fail even when above last_close_hlc, got {result:?}"
    );
    // A well-ordered open (at/above both floors) validates.
    validate_bootstrap_hlc(higher_recovered, higher_recovered, lower_close)
        .expect("an open at/above both floors must validate");
}

#[test]
fn bootstrap_open_hlc_carries_the_recovered_frontier_sequence() -> Result<(), StoreError> {
    let dir = TempDir::new()?;
    let store = Store::open(StoreConfig::new(dir.path()))?;
    let _ = store.append(
        &Coordinate::new("entity:boot", "scope:boot")?,
        EventKind::custom(0xF, 0x44),
        &serde_json::json!({ "n": 1 }),
    )?;
    let recovered = highest_index_hlc(&store.index);
    assert!(
        recovered.global_sequence > 0,
        "premise: the store has committed events"
    );
    // Kills `Ok(Default::default())` (origin/0): the bootstrap open point must
    // carry the recovered frontier's global_sequence.
    let open_hlc = bootstrap_open_hlc(&store.runtime, &store.index)?;
    assert_eq!(
        open_hlc.global_sequence, recovered.global_sequence,
        "the bootstrap open point must carry the recovered frontier's sequence"
    );
    store.close()?;
    Ok(())
}

/// Kills the payload-encryption open-path mutants: `load_key_store -> Ok(None)`
/// / `Ok(Some(default keystore))` and `payload_key_count -> None / Some(0) /
/// Some(1)`. A store opened over a flushed two-key keyset must rehydrate exactly
/// two keys; a store opened without encryption holds none.
#[cfg(feature = "payload-encryption")]
#[test]
fn payload_key_count_reflects_rehydrated_keyset_and_absence() -> Result<(), StoreError> {
    use crate::coordinate::Coordinate;
    use crate::id::EventId;
    use crate::store::{scope_for, KeyScope, KeyScopeGranularity, KeyStore};

    let gran = KeyScopeGranularity::PerEntity;
    let dir = TempDir::new()?;
    let scope = |entity: &str| -> KeyScope {
        scope_for(
            gran,
            &Coordinate::new(entity, "scope:k").expect("coord"),
            EventKind::custom(0xF, 1),
            EventId::from(1u128),
        )
    };
    let mut ks = KeyStore::new(gran);
    let _ = ks.get_or_create(&scope("entity:a")).expect("mint a");
    let _ = ks.get_or_create(&scope("entity:b")).expect("mint b");
    ks.flush(dir.path()).expect("flush keyset");

    let store = Store::open(StoreConfig::new(dir.path()).with_payload_encryption(gran))?;
    assert_eq!(
        store.payload_key_count(),
        Some(2),
        "an encrypted open must rehydrate the two flushed keys (not None, Some(0), or Some(1))"
    );
    store.close()?;

    let plain = Store::open(StoreConfig::new(dir.path()))?;
    assert_eq!(
        plain.payload_key_count(),
        None,
        "a store opened without encryption holds no keyset"
    );
    plain.close()?;
    Ok(())
}

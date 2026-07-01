//! Crash-safety proof for [`KeyStore::flush`]: a `SimFs` `PersistTemp` fault
//! during a flush leaves the on-disk keyset either the OLD intact version or
//! fully written — NEVER torn. Mirrors the atomic-publish proofs in
//! `sim/atomic_fault.rs`, which fault the same routed
//! `persist_temp_with_parent_sync` publish point.
//!
//! Internal (not an integration test) because [`SimFs`] is a `pub(crate)`
//! fault-injection seam; the public round-trip / shred / corrupt proofs live in
//! `tests/keyscope_persist.rs`.

use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::id::EventId;
use crate::store::keyscope::{scope_for, KeyScope, KeyScopeGranularity, KeyStore};
use crate::store::sim::fs::{CrashOp, SimFs};

const GRAN: KeyScopeGranularity = KeyScopeGranularity::PerEntity;
const NONCE: [u8; 24] = [0x5A; 24];

fn scope(entity: &str) -> KeyScope {
    let coord = Coordinate::new(entity, "scope:keyset-crash").expect("coordinate");
    scope_for(
        GRAN,
        &coord,
        EventKind::custom(0xF, 1),
        EventId::from(1u128),
    )
}

#[test]
fn flush_persist_fault_leaves_the_old_keyset_intact_never_torn() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let scope_a = scope("entity:durable");
    let scope_b = scope("entity:torn");

    // ---- V1: mint key A, seal a ciphertext under it, flush durably. ----
    let mut store = KeyStore::new(GRAN);
    let ciphertext = store
        .get_or_create(&scope_a)
        .expect("mint key A")
        .seal(&NONCE, b"aad", b"survives the torn flush")
        .expect("seal under key A");
    store
        .flush(dir.path())
        .expect("V1 flush of key A must succeed");

    // ---- CONTROL: an UNFAULTED SimFs flush of V2 (adds key B) publishes both.
    // Proves the setup genuinely reaches the atomic publish and the fully-written
    // outcome recovers both keys. ----
    {
        let control_dir = tempfile::tempdir().expect("tmpdir");
        let mut control = KeyStore::new(GRAN);
        let _ = control.get_or_create(&scope_a).expect("mint A");
        control.flush(control_dir.path()).expect("control V1");
        let _ = control.get_or_create(&scope_b).expect("mint B");
        let honest = SimFs::new(0xC0FFEE, 0);
        control
            .flush_with_fs(control_dir.path(), &honest)
            .expect("control V2 (unfaulted) must publish");
        let reloaded = KeyStore::load(control_dir.path(), GRAN).expect("reload control");
        assert!(
            reloaded.get(&scope_a).is_some() && reloaded.get(&scope_b).is_some(),
            "PROPERTY: a fully-written flush recovers every key"
        );
    }

    // ---- FAULT: mint key B, then flush V2 through a SimFs armed to tear the
    // atomic publish. The rename never lands, so the temp is discarded and the
    // on-disk keyset stays exactly V1. ----
    let _ = store.get_or_create(&scope_b).expect("mint key B");
    let faulting = SimFs::new(0xDEAD_BEEF, 0).with_fault_on(CrashOp::PersistTemp, 1);
    let flush_result = store.flush_with_fs(dir.path(), &faulting);
    assert!(
        matches!(flush_result, Err(crate::store::StoreError::Io(_))),
        "PROPERTY: a torn atomic publish must surface as StoreError::Io, got {flush_result:?}"
    );

    // ---- Reload: the keyset must be the OLD intact V1 — key A present and able
    // to open the pre-flush ciphertext, key B (the torn addition) ABSENT. Never a
    // half-written mix. ----
    let recovered = KeyStore::load(dir.path(), GRAN).expect("reload after torn flush");
    let key_a = recovered
        .get(&scope_a)
        .expect("PROPERTY: the OLD intact keyset still holds key A after a torn flush");
    assert_eq!(
        key_a
            .open(&NONCE, b"aad", &ciphertext)
            .expect("recovered key A opens the pre-flush ciphertext")
            .as_slice(),
        b"survives the torn flush",
        "PROPERTY: the surviving key A is byte-identical (opens old ciphertext)"
    );
    assert!(
        recovered.get(&scope_b).is_none(),
        "PROPERTY: the torn V2 addition (key B) never landed — keyset is not torn, it is OLD-intact"
    );
}

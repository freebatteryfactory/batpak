//! D24 — the snapshot/fork keyset portability contract.
//!
//! Encrypted-store snapshot/fork must never silently produce an unrestorable
//! copy. By default they FAIL CLOSED (`KeysetNotPortable`); `KeysetPolicy::
//! ExcludeKeys` opts into a keys-excluded copy whose report is stamped, and whose
//! keyset must then be managed out-of-band. Restoring a keys-excluded copy
//! without its keyset reports `KeysetMissing` — a LOUD, distinct fact from a
//! deliberate crypto-shred (`PayloadShredded`), so "the operator lost the keys" and
//! "this scope was deliberately erased" never blur. Keys never travel with the
//! ciphertext, so a backup cannot resurrect crypto-shredded data.
//!
//! Gated behind `payload-encryption` (the whole file compiles out of a default
//! build; the plaintext no-breakage path is also covered by the ungated
//! `store_fork` / `store_snapshot_compaction` suites).
#![cfg(feature = "payload-encryption")]

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::store::{
    ForkFinding, ForkOptions, KeyScopeGranularity, KeysetPolicy, ShredScope, SnapshotFinding,
    SnapshotOptions, Store, StoreConfig, StoreError,
};

const GRAN: KeyScopeGranularity = KeyScopeGranularity::PerEntity;
const KIND: EventKind = EventKind::DATA;

fn open_encrypted(dir: &std::path::Path) -> Store {
    Store::open(StoreConfig::new(dir).with_payload_encryption(GRAN)).expect("open encrypted store")
}

fn open_plaintext(dir: &std::path::Path) -> Store {
    Store::open(StoreConfig::new(dir)).expect("open plaintext store")
}

/// (a) An encryption-active store REFUSES snapshot and fork under the default
/// `KeysetPolicy::Refuse`, with the typed `KeysetNotPortable` error naming the
/// operation. The explicit `Refuse` policy behaves identically to the default.
#[test]
fn encrypted_snapshot_and_fork_fail_closed_by_default() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let store = open_encrypted(dir.path());
    let coord = Coordinate::new("entity:a", "scope:s").expect("coord");
    let _ = store
        .append(&coord, KIND, &serde_json::json!({ "secret": "x" }))
        .expect("append");

    // Default snapshot refuses.
    let snap_dest = tempfile::tempdir().expect("snap dest");
    let snap_err = store
        .snapshot_with_evidence(snap_dest.path())
        .expect_err("default snapshot of an encrypted store must fail closed");
    assert!(
        matches!(&snap_err, StoreError::KeysetNotPortable { operation } if *operation == "snapshot"),
        "default snapshot refuses with KeysetNotPortable, got {snap_err:?}"
    );
    assert!(
        format!("{snap_err}").contains("refused"),
        "the Display message names the refusal: {snap_err}"
    );

    // Explicit Refuse is identical to the default.
    let snap_dest_explicit = tempfile::tempdir().expect("snap dest explicit");
    let explicit_err = store
        .snapshot_with_evidence_with_options(
            snap_dest_explicit.path(),
            SnapshotOptions {
                keyset_policy: KeysetPolicy::Refuse,
            },
        )
        .expect_err("explicit Refuse fails closed too");
    assert!(
        matches!(&explicit_err, StoreError::KeysetNotPortable { .. }),
        "explicit Refuse refuses with KeysetNotPortable, got {explicit_err:?}"
    );

    // Default fork refuses.
    let fork_dest = tempfile::tempdir().expect("fork dest");
    let fork_err = store
        .fork_with_evidence(fork_dest.path(), ForkOptions::default())
        .expect_err("default fork of an encrypted store must fail closed");
    assert!(
        matches!(&fork_err, StoreError::KeysetNotPortable { operation } if *operation == "fork"),
        "default fork refuses with KeysetNotPortable, got {fork_err:?}"
    );
}

/// (b) `KeysetPolicy::ExcludeKeys` lets snapshot and fork proceed, and stamps the
/// returned report with the keys-excluded marker (the acknowledgment lives in the
/// report, not just the call site).
#[test]
fn exclude_keys_proceeds_and_stamps_the_report() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let store = open_encrypted(dir.path());
    let coord = Coordinate::new("entity:a", "scope:s").expect("coord");
    let _ = store
        .append(&coord, KIND, &serde_json::json!({ "secret": "x" }))
        .expect("append");

    let snap_dest = tempfile::tempdir().expect("snap dest");
    let snap_report = store
        .snapshot_with_evidence_with_options(
            snap_dest.path(),
            SnapshotOptions {
                keyset_policy: KeysetPolicy::ExcludeKeys,
            },
        )
        .expect("ExcludeKeys snapshot succeeds");
    assert!(
        snap_report
            .body
            .findings
            .contains(&SnapshotFinding::KeysExcluded),
        "an ExcludeKeys snapshot stamps the report keys-excluded: {:?}",
        snap_report.body.findings
    );

    let fork_dest = tempfile::tempdir().expect("fork dest");
    let fork_report = store
        .fork_with_evidence(
            fork_dest.path(),
            ForkOptions {
                keyset_policy: KeysetPolicy::ExcludeKeys,
                ..Default::default()
            },
        )
        .expect("ExcludeKeys fork succeeds");
    assert!(
        fork_report
            .body
            .findings
            .contains(&ForkFinding::KeysExcluded),
        "an ExcludeKeys fork stamps the report keys-excluded: {:?}",
        fork_report.body.findings
    );
}

/// (c) Opening a keys-excluded copy without its keyset reports the LOUD, typed
/// `KeysetMissing` — never a `PayloadShredded` lookalike. "Keys lost" and "scope
/// deliberately erased" stay distinct facts.
#[test]
fn restoring_without_the_keyset_reads_keyset_missing_not_shredded() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let snap_dest = tempfile::tempdir().expect("snap dest");
    let event_id;
    {
        let store = open_encrypted(dir.path());
        let coord = Coordinate::new("entity:a", "scope:s").expect("coord");
        let receipt = store
            .append(&coord, KIND, &serde_json::json!({ "secret": "x" }))
            .expect("append");
        event_id = receipt.event_id;
        // Sanity: readable in the source (keys present).
        assert_eq!(
            store.get(event_id).expect("source get").event.payload,
            serde_json::json!({ "secret": "x" })
        );
        store
            .snapshot_with_evidence_with_options(
                snap_dest.path(),
                SnapshotOptions {
                    keyset_policy: KeysetPolicy::ExcludeKeys,
                },
            )
            .expect("ExcludeKeys snapshot");
    }

    // Restore: encryption configured, but the keyset file was excluded from the copy.
    let restored = open_encrypted(snap_dest.path());
    let err = restored
        .get(event_id)
        .expect_err("reading an encrypted event with no keyset must error loudly");
    assert!(
        matches!(&err, StoreError::KeysetMissing { .. }),
        "a keys-excluded restore reads KeysetMissing (keys lost), got {err:?}"
    );
    assert!(
        !matches!(&err, StoreError::PayloadShredded { .. }),
        "KeysetMissing must NOT masquerade as a deliberate shred"
    );
    assert!(
        format!("{err}").contains("keyset is entirely absent"),
        "the Display message names the absent keyset: {err}"
    );
}

/// (d) The durability pin: shred a scope, snapshot with `ExcludeKeys`, restore —
/// the shredded scope stays unreadable. Keys never travelled with the ciphertext,
/// so a backup cannot resurrect crypto-shredded data.
#[test]
fn backups_cannot_resurrect_a_shredded_scope() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let snap_dest = tempfile::tempdir().expect("snap dest");
    let event_id;
    {
        let store = open_encrypted(dir.path());
        let coord = Coordinate::new("entity:secret", "scope:s").expect("coord");
        let receipt = store
            .append(&coord, KIND, &serde_json::json!({ "secret": "top" }))
            .expect("append");
        event_id = receipt.event_id;
        // Crypto-shred the scope in the source: plaintext is destroyed.
        store
            .shred_scope(ShredScope::Entity(&coord))
            .expect("shred");
        assert!(
            matches!(store.get(event_id), Err(StoreError::PayloadShredded { .. })),
            "the source reads Shredded after shred_scope"
        );
        store
            .snapshot_with_evidence_with_options(
                snap_dest.path(),
                SnapshotOptions {
                    keyset_policy: KeysetPolicy::ExcludeKeys,
                },
            )
            .expect("ExcludeKeys snapshot");
    }

    // Restore and attempt to read the shredded event: it must NOT come back as
    // plaintext. The keyset never travelled, so nothing can resurrect it.
    let restored = open_encrypted(snap_dest.path());
    let err = restored
        .get(event_id)
        .expect_err("a shredded event must not resurrect from a keys-excluded backup");
    assert!(
        matches!(&err, StoreError::KeysetMissing { .. }),
        "the restored copy has no keyset, so the read is KeysetMissing (unreadable), got {err:?}"
    );
    assert!(
        restored.get(event_id).is_err(),
        "the shredded scope stays unreadable in the restore — no resurrection"
    );
}

/// (e) A store WITHOUT payload encryption is untouched by the gate: snapshot and
/// fork succeed under any policy and never stamp a keys-excluded marker (nothing
/// to exclude). The truly-feature-off path is covered by the ungated suites.
#[test]
fn plaintext_store_is_unaffected_by_the_keyset_gate() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let store = open_plaintext(dir.path());
    let coord = Coordinate::new("entity:a", "scope:s").expect("coord");
    let _ = store
        .append(&coord, KIND, &serde_json::json!({ "n": 1 }))
        .expect("append");

    let snap_dest = tempfile::tempdir().expect("snap dest");
    let snap_report = store
        .snapshot_with_evidence(snap_dest.path())
        .expect("plaintext snapshot succeeds");
    assert!(
        !snap_report
            .body
            .findings
            .contains(&SnapshotFinding::KeysExcluded),
        "a plaintext snapshot never stamps keys-excluded"
    );

    let fork_dest = tempfile::tempdir().expect("fork dest");
    let fork_report = store
        .fork_with_evidence(fork_dest.path(), ForkOptions::default())
        .expect("plaintext fork succeeds");
    assert!(
        !fork_report
            .body
            .findings
            .contains(&ForkFinding::KeysExcluded),
        "a plaintext fork never stamps keys-excluded"
    );

    // Even ExcludeKeys on a plaintext store is a no-op: nothing to exclude.
    let snap_dest2 = tempfile::tempdir().expect("snap dest2");
    let report2 = store
        .snapshot_with_evidence_with_options(
            snap_dest2.path(),
            SnapshotOptions {
                keyset_policy: KeysetPolicy::ExcludeKeys,
            },
        )
        .expect("ExcludeKeys on a plaintext store still succeeds");
    assert!(
        !report2
            .body
            .findings
            .contains(&SnapshotFinding::KeysExcluded),
        "ExcludeKeys on a plaintext store stamps nothing (no keyset to exclude)"
    );
}

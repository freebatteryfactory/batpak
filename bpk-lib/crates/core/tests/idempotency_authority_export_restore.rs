//! GAUNT-IDEMPOTENCY-AUTHORITY-EXPORT-RESTORE (#188): the public opaque
//! export/restore seam over the ONE canonical authority image.
//!
//! PROVES: INV-IDEMPOTENCY-EXPORT-RESTORE-FAIL-CLOSED — export is the current
//! authority image (or `None` when the store carries no user obligations), and
//! restore is a SINGLE offline associated door (contract A13: the live adopt
//! door was deleted). The offline door operates on a CLOSED, locked directory —
//! keyed-admission blocking is therefore STRUCTURAL, not a runtime race — and
//! the restore call IS the authorization ceremony: it validates lineage +
//! coverage, rejects rollback, and MINTS a fresh generation for the admitted
//! image (the dead store never has to have pre-authorized it). Restored entries
//! keep their ORIGINAL receipt identity AND extensions across eviction, sibling
//! adoption, and repeated restore.
//! CATCHES: an export that leaks a fresh store's empty image; a restore that
//! drops receipt extensions; a restore that resurrects an evicted frame instead
//! of replaying the stored receipt; a restore that races a live store (must be
//! locked out); a restore that is not repeatable.
//! SEEDED: real stores in tempdirs; deterministic key constants; a retention
//! compaction that evicts every user frame; a pre-authority fork sibling.
//! (Rejection matrix — corruption, future format, foreign lineage, stale
//! rollback, etc. — lives in `idempotency_authority_restore_refusals.rs`.)

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::{EntityIdType, IdempotencyKey};
use batpak::store::{
    AppendOptions, AppendReceipt, CompactionConfig, CompactionStrategy, ExtensionKey,
    IdempotencyAuthorityExport, Store, StoreConfig, StoreError,
};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 4);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:export-restore").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

/// A distinct-per-`tag` receipt extension so preservation is witnessed by value,
/// not merely by non-emptiness.
fn ext_key() -> ExtensionKey {
    ExtensionKey::new("acme.restore_witness").expect("valid extension key")
}

/// Keyed append carrying a receipt extension; the returned receipt's identity
/// and extension map are the fixture the restore path must reproduce on retry.
fn append_keyed_ext(store: &Store, key: u128, tag: u8) -> AppendReceipt {
    let payload_tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            KIND,
            &serde_json::json!({ "k": payload_tag }),
            AppendOptions::new()
                .with_idempotency(IdempotencyKey::from(key))
                .with_extension(ext_key(), vec![tag, 0xEE, tag]),
        )
        .expect("keyed append with extension")
}

fn key_event_count(store: &Store, key: u128) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_kind() == KIND && e.event_id().as_u128() == key)
        .count()
}

/// Export that MUST carry user obligations (a `None` here is a fixture bug).
fn export_some(store: &Store) -> IdempotencyAuthorityExport {
    store
        .export_idempotency_authority()
        .expect("export idempotency authority")
        .expect("store has user obligations to export")
}

/// Retention strategy that evicts EVERY user event of `KIND`, forcing keyed
/// event frames out of the store while keeping the idempotency obligation.
fn evict_all_user_events() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != KIND
        })),
        min_segments: 1,
    }
}

/// Seal segments then run the evicting retention compaction; assert the keyed
/// frame is really gone so the durable image is the ONLY replay authority.
fn evict_key_via_compaction(store: &Store, key: u128) {
    for i in 0..8 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "filler": i }))
            .expect("append filler event");
    }
    let (_result, _report) = store
        .compact(&evict_all_user_events())
        .expect("retention compaction");
    let survivors = store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_id().as_u128() == key)
        .count();
    assert_eq!(
        survivors, 0,
        "PRECONDITION: retention compaction evicted the keyed event frame"
    );
}

/// A restored keyed retry reconstructs the ORIGINAL receipt: identity fields and
/// the receipt extension map, field-by-field.
fn assert_receipt_eq(replay: &AppendReceipt, original: &AppendReceipt, ctx: &str) {
    assert_eq!(
        u128::from(replay.event_id),
        u128::from(original.event_id),
        "{ctx}: event id must match the original receipt"
    );
    assert_eq!(
        replay.global_sequence, original.global_sequence,
        "{ctx}: global sequence must match the original receipt"
    );
    assert_eq!(
        replay.content_hash, original.content_hash,
        "{ctx}: content hash must match the original receipt"
    );
    assert_eq!(
        replay.extensions, original.extensions,
        "{ctx}: receipt extensions must be preserved verbatim"
    );
}

// ---------------------------------------------------------------------------
// 1. A store with no user obligations exports NOTHING (not an empty image).
// ---------------------------------------------------------------------------
#[test]
fn export_without_user_obligations_is_none() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let store = Store::open(config(dir.path())).expect("open");
    let exported = store
        .export_idempotency_authority()
        .expect("export succeeds on a store with no user obligations");
    assert!(
        exported.is_none(),
        "PROPERTY (#188): a store with no user idempotency obligations has nothing to export"
    );
    store.close().expect("close");
    assert!(
        !dir.path().join(IDEMP_FILENAME).exists(),
        "export retired the empty authority image; no sidecar remains on disk"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. FLAGSHIP: an evicted-key store whose sidecar is lost refuses to open;
// offline restore of a prior export heals it and the retry replays the
// ORIGINAL receipt (identity + extensions) even though the frame is gone.
// ---------------------------------------------------------------------------
#[test]
fn export_then_offline_restore_heals_evicted_authority() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let key = 0x1111_2222_3333_4444_5555_6666_7777_8888u128;

    let (original, export) = {
        let store = Store::open(config(dir.path())).expect("open");
        let receipt = append_keyed_ext(&store, key, 0x21);
        evict_key_via_compaction(&store, key);
        let export = export_some(&store);
        store.close().expect("close");
        (receipt, export)
    };

    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;

    // The disaster is real: the keyed store refuses to open without its image.
    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#188): a keyed store missing its authority image must refuse open",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityMissing { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };

    Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect("offline restore heals the missing authority");

    let store = Store::open(config(dir.path())).expect("restored authority admits the store");
    let replay = append_keyed_ext(&store, key, 0x21);
    assert_receipt_eq(&replay, &original, "offline-restore retry");
    assert_eq!(
        key_event_count(&store, key),
        0,
        "the evicted frame stays evicted — the restored retry re-appended NOTHING"
    );
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Adopt into a pre-authority sibling (E=None, same lineage): the offline
// door MINTS a fresh generation for the image the sibling never authorized.
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_into_pre_authority_sibling_adopts() -> Result<(), Box<dyn std::error::Error>> {
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");
    let key = 0x3333_0000_0000_0000_0000_0000_0000_0003u128;

    let (original, export) = {
        let a = Store::open(config(a_dir.path())).expect("open a");
        // Fork BEFORE any user obligation: the sibling shares A's lineage and
        // its store.meta records NO authority expectation (nothing yet to
        // authorize) — the adopt shape after the live door's deletion (A13).
        a.fork(f_dir.path()).expect("fork pre-authority sibling");
        let receipt = append_keyed_ext(&a, key, 0x33);
        let export = export_some(&a);
        a.close().expect("close a");
        (receipt, export)
    };

    // The restore call is the authorization: same lineage, coverage ahead of
    // the sibling's committed history (not a rollback), fresh generation minted.
    Store::restore_idempotency_authority(&config(f_dir.path()), &export)
        .expect("offline restore adopts into the pre-authority sibling");

    let f = Store::open(config(f_dir.path())).expect("open restored sibling");
    let replay = append_keyed_ext(&f, key, 0x33);
    assert_receipt_eq(&replay, &original, "sibling-adopt retry");
    assert_eq!(
        key_event_count(&f, key),
        0,
        "the adopted key's frame lives only in A; the sibling retry re-appends nothing"
    );
    f.close().expect("close fork");
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Restore is repeatable: re-applying the same export (same lineage, same
// coverage — not a rollback) re-mints and stays admissible.
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_is_repeatable() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let key = 0x4444_0000_0000_0000_0000_0000_0000_0004u128;

    let (original, export) = {
        let store = Store::open(config(dir.path())).expect("open");
        let receipt = append_keyed_ext(&store, key, 0x44);
        let export = export_some(&store);
        store.close().expect("close");
        (receipt, export)
    };
    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;

    Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect("first offline restore");
    {
        let store = Store::open(config(dir.path())).expect("open after first restore");
        let replay = append_keyed_ext(&store, key, 0x44);
        assert_receipt_eq(&replay, &original, "first restore retry");
        store.close().expect("close");
    }

    // Re-applying the identical export is idempotent, not a stale rollback.
    Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect("repeated offline restore is admissible");
    let store = Store::open(config(dir.path())).expect("open after repeated restore");
    let replay = append_keyed_ext(&store, key, 0x44);
    assert_receipt_eq(&replay, &original, "repeated restore retry");
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Multi-key restore into a store whose frames are LIVE: every retry is a
// no-op replay of the original receipt and no frame is duplicated.
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_into_live_event_store_preserves_every_receipt(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let keys = [
        0x5000_0000_0000_0000_0000_0000_0000_0001u128,
        0x5000_0000_0000_0000_0000_0000_0000_0002u128,
        0x5000_0000_0000_0000_0000_0000_0000_0003u128,
    ];

    let (originals, export) = {
        let store = Store::open(config(dir.path())).expect("open");
        let mut originals = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            let tag = u8::try_from(0x50 + i).expect("tag fits u8");
            originals.push(append_keyed_ext(&store, *key, tag));
        }
        let export = export_some(&store);
        store.close().expect("close");
        (originals, export)
    };

    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;
    // Missing sidecar on a keyed store refuses; offline restore heals it.
    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#188): a keyed store missing its authority image must refuse open",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityMissing { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };

    Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect("offline restore heals the multi-key authority");

    let store = Store::open(config(dir.path())).expect("open after restore");
    for (i, key) in keys.iter().enumerate() {
        let tag = u8::try_from(0x50 + i).expect("tag fits u8");
        let replay = append_keyed_ext(&store, *key, tag);
        assert_receipt_eq(&replay, &originals[i], "multi-key live retry");
        assert_eq!(
            key_event_count(&store, *key),
            1,
            "each live keyed event stays single after the restored retry"
        );
    }
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Structural blocking: the offline door needs the store-dir lock, so it
// REFUSES while a live store holds it, and admits once the store is closed.
// This is why no keyed append can ever race a restore.
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_requires_a_closed_store() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let key = 0x6666_0000_0000_0000_0000_0000_0000_0006u128;

    let a = Store::open(config(dir.path())).expect("open");
    let original = append_keyed_ext(&a, key, 0x66);
    let export = export_some(&a);

    let err = match Store::restore_idempotency_authority(&config(dir.path()), &export) {
        Ok(()) => {
            return Err(std::io::Error::other(
                "PROPERTY (#188): offline restore must refuse while the store is open (locked)",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::StoreLocked { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };

    a.close().expect("close");
    Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect("offline restore admitted once the store is closed");
    let store = Store::open(config(dir.path())).expect("open after lock release");
    let replay = append_keyed_ext(&store, key, 0x66);
    assert_receipt_eq(&replay, &original, "post-lock-release retry");
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Export runs the canonical authorize->publish->finalize flush; the
// generation it leaves on disk must be consistent enough to close and reopen.
// ---------------------------------------------------------------------------
#[test]
fn export_flushes_a_finalized_generation() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let key = 0x7777_0000_0000_0000_0000_0000_0000_0007u128;

    let original = {
        let store = Store::open(config(dir.path())).expect("open");
        let receipt = append_keyed_ext(&store, key, 0x77);
        let _export = export_some(&store);
        store.close().expect("close after export");
        receipt
    };

    let store = Store::open(config(dir.path())).expect("reopen after export+close");
    let replay = append_keyed_ext(&store, key, 0x77);
    assert_receipt_eq(&replay, &original, "post-export reopen retry");
    assert_eq!(
        key_event_count(&store, key),
        1,
        "the single keyed event stays single across export + reopen"
    );
    store.close().expect("close");
    Ok(())
}

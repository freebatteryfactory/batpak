//! T2 (#188 / #226 A13): the rejection matrix of the OFFLINE idempotency-authority
//! restore door, driven end-to-end through the public seam.
//!
//! Restore is a single closed-directory associated fn
//! (`Store::restore_idempotency_authority(config, export)`, A13): it acquires the
//! store-dir lock, refuses while a pending-compaction transaction is unsettled,
//! validates the export against `store.meta` (lineage, coverage, rollback,
//! sibling divergence), then MINTS a fresh local generation and republishes. The
//! call IS the authorization ceremony — it never requires the dead store to have
//! pre-authorized the incoming image's token (no circularity), so the old
//! "unauthorized image generation" refusal is GONE: a same-anchor own export now
//! ADMITS (the green companion here pins that, killing any reintroduced
//! token-authorization gate).
//!
//! PROVES: every offline-decidable refusal is a distinct typed
//! `StoreError::IdempotencyRestoreRefused { reason }` nested variant — a corrupt
//! body, a future format, a foreign lineage, a rollback, a diverged sibling
//! fork, an unresolved pending compaction — plus the two structural guards
//! (`StoreMetadataMissing` on a bare directory, `StoreLocked` on a live one).
//! CATCHES: a byte-flip admitted blind, a version gate weakened to frontier
//! arithmetic, a lineage check dropped, a rollback silently narrowing the dedup
//! contract, a sibling image adopted at an equal frontier, a restore admitted
//! over an in-flight compaction, and a reintroduced circular pre-authorization.
//! SEEDED: real stores with tiny segments; genuine exports byte-surgered on
//! `export.clone().into_bytes()`; sibling forks leveled to a shared frontier.
//!
//! The purely intrinsic rows (a forged image whose user entry sits beyond its own
//! coverage, an anchor-less image) live in the in-crate island
//! `store/idemp_transfer_tests.rs` (P1b) — the opaque public surface cannot forge
//! those honestly.

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::id::IdempotencyKey;
use batpak::store::{
    AppendOptions, AppendReceipt, IdempAuthorityCorruption, IdempotencyAuthorityExport,
    IdempotencyRestoreRefusal, Store, StoreConfig, StoreError,
};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 4);

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:restore-refusals").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

fn append_keyed(store: &Store, key: u128) -> AppendReceipt {
    let payload_tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            KIND,
            &serde_json::json!({ "k": payload_tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed append")
}

/// Export the store's durable authority; a store with a keyed obligation always
/// has an image to export (the `None` branch is a T1 happy-path row).
fn export_of(store: &Store) -> IdempotencyAuthorityExport {
    store
        .export_idempotency_authority()
        .expect("export succeeds")
        .expect("a keyed store has a durable authority image to export")
}

// ---------------------------------------------------------------------------
// Matrix row 1 — corruption: one flipped body byte fails the CRC check before
// any store state is consulted.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_export_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let export = {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x1111_2222_3333_4444_5555_6666_7777_8888u128);
        let export = export_of(&store);
        store.close().expect("close");
        export
    };

    let mut bytes = export.clone().into_bytes();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    let patched = IdempotencyAuthorityExport::from_bytes(bytes);

    let err = Store::restore_idempotency_authority(&config(dir.path()), &patched)
        .expect_err("PROPERTY: a body-corrupted export must refuse, never admit blind");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::Corrupt {
                    kind: IdempAuthorityCorruption::CrcMismatch { .. }
                }
            }
        ),
        "wrong refusal: {err:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix row 2 — future version: the header version is checked before the CRC,
// so patching it (CRC covers the body only) surfaces the exact found/supported
// pair rather than a corruption.
// ---------------------------------------------------------------------------
#[test]
fn future_version_export_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let export = {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x2222_0000_0000_0000_0000_0000_0000_0002u128);
        let export = export_of(&store);
        store.close().expect("close");
        export
    };

    // Header layout (shared `wire_header`): magic(6) | version u16 LE (bytes
    // 6..8) | crc32(4) | body. The build wrote its current version; bump it by
    // one so `supported` is read from the same export, not hard-coded.
    let mut bytes = export.clone().into_bytes();
    let current = u16::from_le_bytes([bytes[6], bytes[7]]);
    let future = current + 1;
    bytes[6..8].copy_from_slice(&future.to_le_bytes());
    let patched = IdempotencyAuthorityExport::from_bytes(bytes);

    let err = Store::restore_idempotency_authority(&config(dir.path()), &patched)
        .expect_err("PROPERTY: a future-version export must refuse");
    let StoreError::IdempotencyRestoreRefused {
        reason: IdempotencyRestoreRefusal::FutureVersion { found, supported },
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong refusal: {err:?}")).into());
    };
    assert_eq!(found, future, "the refusal reports the export's declared version");
    assert_eq!(
        supported, current,
        "the refusal reports the build's supported version"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix row 4 — foreign lineage: an export is restorable only into its own
// lineage; a second, independently opened store has a different minted lineage.
// ---------------------------------------------------------------------------
#[test]
fn foreign_lineage_export_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let a_dir = TempDir::new().expect("tempdir a");
    let b_dir = TempDir::new().expect("tempdir b");

    let export = {
        let a = Store::open(config(a_dir.path())).expect("open a");
        let _ = append_keyed(&a, 0x3333_0000_0000_0000_0000_0000_0000_0003u128);
        let export = export_of(&a);
        a.close().expect("close a");
        export
    };
    {
        let b = Store::open(config(b_dir.path())).expect("open b");
        let _ = append_keyed(&b, 0x4444_0000_0000_0000_0000_0000_0000_0004u128);
        b.close().expect("close b");
    }

    let err = Store::restore_idempotency_authority(&config(b_dir.path()), &export)
        .expect_err("PROPERTY: another lineage's export must refuse");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::ForeignLineage { .. }
            }
        ),
        "wrong refusal: {err:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix row 3 — stale rollback: an export taken before a newer publication
// covers a shorter frontier than the store's recorded authority; restoring it
// would narrow the dedup contract.
// ---------------------------------------------------------------------------
#[test]
fn stale_export_is_rejected_after_newer_publication() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let early = {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x5555_0000_0000_0000_0000_0000_0000_0001u128);
        let early = export_of(&store);
        // A second keyed obligation advances the covered frontier; close
        // finalizes the newer generation on disk.
        let _ = append_keyed(&store, 0x5555_0000_0000_0000_0000_0000_0000_0002u128);
        store.close().expect("close");
        early
    };

    let err = Store::restore_idempotency_authority(&config(dir.path()), &early)
        .expect_err("PROPERTY: an older export must refuse against advanced authority");
    let StoreError::IdempotencyRestoreRefused {
        reason: IdempotencyRestoreRefusal::StaleRollback {
            image_covered,
            expected_covered,
        },
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong refusal: {err:?}")).into());
    };
    assert!(
        image_covered < expected_covered,
        "the refusal reports the frontier regression ({image_covered} < {expected_covered})"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix row 5 — sibling divergence: two forks share the lineage (#205), so
// lineage + frontier alone cannot tell them apart. Leveled to the SAME covered
// sequence but diverged with a different keyed event there, the fork's export
// contradicts the target's recorded anchor identity.
// ---------------------------------------------------------------------------
#[test]
fn sibling_fork_export_is_rejected_by_divergence() -> Result<(), Box<dyn std::error::Error>> {
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");

    {
        let store = Store::open(config(a_dir.path())).expect("open");
        let _ = append_keyed(&store, 0x6000_0000_0000_0000_0000_0000_0000_0001u128);
        store.fork(f_dir.path()).expect("fork");
        store.close().expect("close");
    }

    let a = Store::open(config(a_dir.path())).expect("reopen a");
    let f = Store::open(config(f_dir.path())).expect("open fork");
    // Equalize the frontiers (lifecycle events skew them) so the divergent
    // keyed appends below land at the SAME recorded sequence — the only shape
    // frontier comparison cannot separate.
    for _ in 0..64 {
        let (ga, gf) = (a.stats().global_sequence, f.stats().global_sequence);
        if ga == gf {
            break;
        }
        let lagging = if ga < gf { &a } else { &f };
        let _ = lagging
            .append(&coord(), KIND, &serde_json::json!({ "filler": "level" }))
            .expect("leveling append");
    }
    assert_eq!(
        a.stats().global_sequence,
        f.stats().global_sequence,
        "PRECONDITION: siblings sit at one frontier before the divergent keyed appends"
    );
    let _ = append_keyed(&a, 0x6000_0000_0000_0000_0000_0000_0000_00a0u128);
    let _ = append_keyed(&f, 0x6000_0000_0000_0000_0000_0000_0000_00f0u128);
    // Export the fork's image, then finalize A's own (different) anchor.
    let fork_export = export_of(&f);
    f.close().expect("close fork");
    a.close().expect("close a");

    let err = Store::restore_idempotency_authority(&config(a_dir.path()), &fork_export)
        .expect_err("PROPERTY: a same-frontier sibling export must refuse by divergence");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::SiblingDivergence { .. }
            }
        ),
        "wrong refusal: {err:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A13 green companion — NO circular pre-authorization. The dead
// "UnauthorizedGeneration" refusal covered a store's own export at an unchanged
// anchor; the offline door now MINTS its own generation and ADMITS it. This
// pins that the token-authorization gate is not reintroduced.
// ---------------------------------------------------------------------------
#[test]
fn same_anchor_own_export_restores_without_pre_authorization(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let export = {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x7777_0000_0000_0000_0000_0000_0000_0007u128);
        let export = export_of(&store);
        // Close mints a fresh token but adds no user obligation — the anchor is
        // unchanged, so the earlier export is neither a rollback nor a sibling.
        store.close().expect("close");
        export
    };

    Store::restore_idempotency_authority(&config(dir.path()), &export).expect(
        "PROPERTY (A13): the store's own same-anchor export restores by minting a fresh \
         generation — restore is the authorization ceremony, not a token pre-authorization gate",
    );
    // The minted generation leaves the directory openable.
    let store = Store::open(config(dir.path())).expect("reopen after restore");
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// Structural guard — a bare directory has no `store.meta`; restore validates
// the export structurally, then refuses fail-closed with the metadata-missing
// error (never a fresh-store admission).
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_into_missing_meta_dir_refuses() -> Result<(), Box<dyn std::error::Error>> {
    let src = TempDir::new().expect("source tempdir");
    let export = {
        let store = Store::open(config(src.path())).expect("open");
        let _ = append_keyed(&store, 0x8888_0000_0000_0000_0000_0000_0000_0008u128);
        let export = export_of(&store);
        store.close().expect("close");
        export
    };

    let bare = TempDir::new().expect("bare tempdir");
    let err = Store::restore_idempotency_authority(&config(bare.path()), &export)
        .expect_err("PROPERTY: a directory with no store.meta must refuse");
    assert!(
        matches!(err, StoreError::StoreMetadataMissing { .. }),
        "wrong error: {err:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Structural guard — keyed-admission exclusion is the dir lock: a live store
// holds it, so an offline restore into the same directory cannot begin.
// ---------------------------------------------------------------------------
#[test]
fn offline_restore_while_store_open_is_locked() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let store = Store::open(config(dir.path())).expect("open");
    let _ = append_keyed(&store, 0x9999_0000_0000_0000_0000_0000_0000_0009u128);
    let export = export_of(&store);

    let err = Store::restore_idempotency_authority(&config(dir.path()), &export)
        .expect_err("PROPERTY: a live store's lock blocks an offline restore");
    assert!(
        matches!(err, StoreError::StoreLocked { .. }),
        "wrong error: {err:?}"
    );

    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix — pending compaction unresolved: the directory's durable authority is
// not yet settled into exactly the old or the new generation, so restore
// refuses until a writable open completes recovery. A real durable marker is
// left by crashing a compaction on `ShadowFs` with freeze-on-fault so in-process
// rollback cannot clear it; the marker survives to the crashed image.
// ---------------------------------------------------------------------------
#[cfg(feature = "dangerous-test-hooks")]
#[test]
fn pending_compaction_marker_blocks_restore() -> Result<(), Box<dyn std::error::Error>> {
    use batpak::store::{CompactionConfig, CrashOp, ShadowFs};
    use std::sync::Arc;

    let dir = TempDir::new().expect("tempdir");
    let fs = ShadowFs::new();
    let store = Store::open(
        config(dir.path()).with_fs(Arc::new(fs.clone())),
    )
    .expect("open on shadow fs");

    // Seal several segments so the compaction genuinely merges (and thus writes
    // the pending marker) rather than skipping.
    let blob = "x".repeat(300);
    for i in 0..12 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "i": i, "blob": blob }))
            .expect("filler append");
    }

    // Fail the marker CLEAR (the transaction's last durable step) and poison the
    // fs so rollback cannot remove the marker: every prior step commits, the
    // marker stays durable.
    fs.arm_op_error_at(CrashOp::RemoveFile, "compaction.pending", 1);
    fs.set_freeze_on_fault(true);
    let _ = store.compact(&CompactionConfig {
        min_segments: 1,
        ..CompactionConfig::default()
    });
    drop(store);
    fs.crash();

    // The marker is checked before the export is parsed, so any bytes suffice.
    let dummy = IdempotencyAuthorityExport::from_bytes(vec![0u8; 8]);
    let err = Store::restore_idempotency_authority(
        &config(dir.path()).with_fs(Arc::new(fs.clone())),
        &dummy,
    )
    .expect_err("PROPERTY: an unresolved pending compaction must block restore");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::PendingCompactionUnresolved { .. }
            }
        ),
        "wrong refusal: {err:?}"
    );
    Ok(())
}

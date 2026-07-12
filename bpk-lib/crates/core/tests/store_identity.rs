//! #205: `Store::identity()` — the store's LINEAGE identity, persisted in the
//! `store.meta` sidecar.
//!
//! PROVES: INV-STORE-LINEAGE-IDENTITY.
//!
//! PROVES: the lineage semantics ruled for #205 — minted once at first
//! writable open; stable across close/reopen; copied by snapshot AND by fork
//! (a fork that minted a fresh identity would see its own copied idempotency
//! authority as foreign); distinct across unrelated stores; corruption and
//! future versions of `store.meta` fail closed with exact typed variants; a
//! post-migration witness (idempotency sidecar declaring format v2+) makes an
//! ABSENT `store.meta` a typed refusal instead of a silent remint
//! (the never-remint law); read-only opens of unmigrated legacy directories
//! mint nothing.
//! CATCHES: silent identity reminting (the identity-reset trap), a fork/
//! snapshot that drops or re-mints the lineage file, corruption downgraded to
//! absence, and a remint racing past the post-migration witness.
//! SEEDED: real stores in temp directories whose `store.meta`/`index.idemp`
//! bytes are surgically deleted, corrupted, version-patched, or hand-forged
//! (header-level v2 idempotency witness).

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::id::StoreIdentity;
use batpak::store::{Store, StoreConfig, StoreError};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0x1, 1);

fn open(dir: &Path) -> Store<batpak::store::Open> {
    Store::open(StoreConfig::new(dir)).expect("open store")
}

fn seed_one_event(store: &Store<batpak::store::Open>) {
    let coord = Coordinate::new("entity:identity", "scope:test").expect("valid coord");
    let _receipt = store
        .append(&coord, KIND, &serde_json::json!({"seed": true}))
        .expect("seed append");
}

fn meta_path(dir: &Path) -> std::path::PathBuf {
    dir.join("store.meta")
}

#[test]
fn identity_is_minted_once_and_stable_across_reopen() {
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    let minted = store
        .identity()
        .expect("a writable open mints the identity");
    assert!(
        meta_path(dir.path()).exists(),
        "the lineage identity is PERSISTED at mint time (store.meta on disk)"
    );
    seed_one_event(&store);
    store.close().expect("close");

    let reopened = open(dir.path());
    let after = reopened
        .identity()
        .expect("identity resolves on every subsequent open");
    assert_eq!(
        minted, after,
        "PROPERTY (#205): the lineage identity is stable across close/reopen"
    );
    reopened.close().expect("close reopened");
}

#[test]
fn two_fresh_stores_mint_distinct_lineages() {
    let a_dir = TempDir::new().expect("temp dir a");
    let b_dir = TempDir::new().expect("temp dir b");
    let a = open(a_dir.path());
    let b = open(b_dir.path());
    let a_id = a.identity().expect("a mints");
    let b_id = b.identity().expect("b mints");
    assert_ne!(
        a_id, b_id,
        "PROPERTY (#205): unrelated store lineages are distinguishable"
    );
    a.close().expect("close a");
    b.close().expect("close b");
}

#[test]
fn snapshot_and_fork_copy_the_lineage_identity() {
    let dir = TempDir::new().expect("temp dir");
    let snap_dir = TempDir::new().expect("snapshot dir");
    let fork_dir = TempDir::new().expect("fork dir");

    let store = open(dir.path());
    let lineage = store.identity().expect("source identity");
    seed_one_event(&store);
    store
        .snapshot_with_evidence(snap_dir.path())
        .expect("snapshot");
    store.fork(fork_dir.path()).expect("fork");
    store.close().expect("close source");

    let snap = open(snap_dir.path());
    assert_eq!(
        snap.identity().expect("snapshot identity"),
        lineage,
        "PROPERTY (#205): a snapshot carries the SAME lineage identity (it carries the same \
         idempotency authority)"
    );
    snap.close().expect("close snapshot");

    let fork = open(fork_dir.path());
    assert_eq!(
        fork.identity().expect("fork identity"),
        lineage,
        "PROPERTY (#205, owner ruling): a fork COPIES the lineage identity — minting a fresh one \
         would make the copied idempotency authority image read as foreign"
    );
    fork.close().expect("close fork");
}

#[test]
fn corrupt_store_meta_refuses_open_with_exact_variant() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    seed_one_event(&store);
    store.close().expect("close");

    // Flip one body byte (past the 12-byte header) so the CRC fails.
    let path = meta_path(dir.path());
    let mut bytes = std::fs::read(&path)?;
    let corrupt_at = bytes.len() - 1;
    bytes[corrupt_at] ^= 0xFF;
    std::fs::write(&path, bytes)?;

    let err = match Store::open(StoreConfig::new(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#205): corrupt store.meta must fail closed, not remint or ignore",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::StoreMetadataCorrupt {
        path: err_path,
        kind,
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    // The open path canonicalizes data_dir (Windows: `\?\` UNC form), so
    // compare the load-bearing leaf, not the full path spelling.
    assert_eq!(
        err_path.file_name().and_then(|name| name.to_str()),
        Some("store.meta"),
        "the refusal names the metadata file; got {}",
        err_path.display()
    );
    assert!(
        matches!(kind, batpak::store::StoreMetaCorruption::CrcMismatch { .. }),
        "a flipped body byte is a CRC mismatch; got {kind:?}"
    );
    Ok(())
}

#[test]
fn future_version_store_meta_refuses_canonically() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    store.close().expect("close");

    // Patch the header's version field (bytes 6..8, little-endian) to a future
    // version. The version sits OUTSIDE the CRC region, so the body still
    // CRC-validates — the refusal must key on the version alone.
    let path = meta_path(dir.path());
    let mut bytes = std::fs::read(&path)?;
    bytes[6..8].copy_from_slice(&999u16.to_le_bytes());
    std::fs::write(&path, bytes)?;

    let err = match Store::open(StoreConfig::new(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: a future-version store.meta must refuse canonically",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::StoreMetadataFutureVersion {
        found, supported, ..
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert_eq!(found, 999, "the refusal reports the on-disk version");
    assert_eq!(
        supported,
        batpak::store::STORE_META_VERSION,
        "the refusal reports the supported ceiling"
    );
    Ok(())
}

#[test]
fn post_migration_witness_makes_missing_meta_refuse_never_remint(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    seed_one_event(&store);
    store.close().expect("close");

    // Simulate the identity-reset trap: store.meta vanishes while a
    // POST-MIGRATION WITNESS remains — an idempotency sidecar declaring format
    // v2+. (Forged at header level: magic FBATID + version 2 LE + crc; the
    // witness peek reads only the 12-byte header.)
    std::fs::remove_file(meta_path(dir.path()))?;
    let mut forged_idemp = Vec::new();
    forged_idemp.extend_from_slice(b"FBATID");
    forged_idemp.extend_from_slice(&2u16.to_le_bytes());
    forged_idemp.extend_from_slice(&crc32fast::hash(&[]).to_le_bytes());
    std::fs::write(dir.path().join("index.idemp"), forged_idemp)?;

    let err =
        match Store::open(StoreConfig::new(dir.path())) {
            Ok(_) => return Err(std::io::Error::other(
                "PROPERTY (#205, never-remint): missing store.meta with a post-migration witness \
                 must refuse — reminting would reset the lineage identity",
            )
            .into()),
            Err(e) => e,
        };
    let StoreError::StoreMetadataMissing { path } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("store.meta"),
        "the refusal names the expected metadata path; got {}",
        path.display()
    );
    assert!(
        !meta_path(dir.path()).exists(),
        "the refusal must not have written a replacement file (never remint)"
    );
    Ok(())
}

#[test]
fn legacy_v1_directory_without_witness_migrates_and_is_stable_thereafter() {
    // A pre-store.meta directory (v1-era idempotency sidecar, no v2 witness)
    // takes the one-time migration at the next writable open. The minted
    // identity is NEW — pre-migration history has no lineage to preserve —
    // and every open after the migration returns the same value.
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    seed_one_event(&store);
    let pre_delete = store.identity().expect("first mint");
    store.close().expect("close");
    // Erase the metadata AND the (v2, post-migration-witness) idempotency
    // sidecar: what remains is a pre-store.meta, pre-durable-idempotency
    // legacy directory shape with no witness, so migration may mint.
    std::fs::remove_file(meta_path(dir.path())).expect("remove store.meta");
    std::fs::remove_file(dir.path().join("index.idemp")).expect("remove idemp sidecar");

    let migrated = open(dir.path());
    let minted = migrated
        .identity()
        .expect("legacy migration mints an identity");
    assert_ne!(
        minted, pre_delete,
        "a legacy directory has no lineage to preserve — migration mints fresh (this is why the \
         v2 witness exists: it closes exactly this reset for migrated stores)"
    );
    migrated.close().expect("close migrated");

    let reopened = open(dir.path());
    assert_eq!(
        reopened.identity().expect("post-migration identity"),
        minted,
        "PROPERTY (#205): after the one-time migration the identity is stable"
    );
    reopened.close().expect("close reopened");
}

#[test]
fn read_only_open_of_unmigrated_legacy_dir_mints_nothing() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new().expect("temp dir");
    let store = open(dir.path());
    seed_one_event(&store);
    store.close().expect("close");
    std::fs::remove_file(meta_path(dir.path()))?;
    // Remove the v2 sidecar too: with it present the post-migration witness
    // (correctly) refuses the open outright instead of exercising the
    // read-only no-mint path under test here.
    std::fs::remove_file(dir.path().join("index.idemp"))?;

    let read_only = Store::open_read_only(StoreConfig::new(dir.path()))
        .expect("read-only open of a legacy directory must still work");
    let err = match read_only.identity() {
        Ok(identity) => {
            let raw: u128 = identity.into();
            return Err(std::io::Error::other(format!(
                "PROPERTY (#205): a read-only open must not mint an identity; got {raw:x}"
            ))
            .into());
        }
        Err(e) => e,
    };
    let StoreError::StoreMetadataMissing { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        !meta_path(dir.path()).exists(),
        "read-only opens must not write store.meta"
    );
    drop(read_only);

    // A writable open then performs the migration.
    let writable = open(dir.path());
    let _minted: StoreIdentity = writable
        .identity()
        .expect("the next writable open migrates");
    assert!(meta_path(dir.path()).exists());
    writable.close().expect("close writable");
    Ok(())
}

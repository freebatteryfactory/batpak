//! The handle-abstracted `StoreFs` seam (0.10.0, issues #164/#168).
//!
//! PROVES: `StoreConfig::with_fs` is public and functional, and the seam's
//! backend contract holds over the production backend through ABSTRACT
//! handles ([`StoreFile`], [`StagedFile`], [`StoreDirLockGuard`]) — no
//! concrete `std::fs` type appears in any assertion. The contract body is
//! parameterized over `&dyn StoreFs` so every backend (RealFs here; MemFs in
//! `store_fs_backend_conformance`) runs the identical corpus.

use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{ParentDirSyncAdmission, RealFs};

mod common;

#[derive(serde::Serialize, serde::Deserialize, EventPayload)]
#[batpak(category = 0xF, type_id = 21)]
struct SeamProbe {
    value: i64,
}

#[test]
fn store_opened_with_installed_real_fs_appends_and_reads() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let config = StoreConfig::new(dir.path()).with_fs(Arc::new(RealFs));
    let store = Store::open(config)?;

    let coord = Coordinate::new("entity:seam", "scope:fs")?;
    let receipt = store.append_typed(&coord, &SeamProbe { value: 7 })?;
    let fetched = store.get(receipt.event_id)?;
    assert_eq!(fetched.event.header.event_id, receipt.event_id);
    assert!(
        store.verify_append_receipt(&receipt).is_valid(),
        "a store on an explicitly-installed RealFs must verify its own receipt"
    );

    store.close()?;
    Ok(())
}

#[test]
fn real_fs_upholds_the_documented_backend_contract() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    common::backend_upholds_the_documented_contract(&RealFs, dir.path())
}

#[test]
fn parent_dir_sync_admission_is_a_shareable_proof_token() {
    // The admission token has no public constructor — it is minted by the
    // store's platform-evidence check and handed TO a backend. Its public
    // contract is nameability + the auto-traits a backend relies on to hold
    // or forward it.
    fn assert_token_contract<T: Send + Sync + Copy + std::fmt::Debug>() {}
    assert_token_contract::<ParentDirSyncAdmission>();
}

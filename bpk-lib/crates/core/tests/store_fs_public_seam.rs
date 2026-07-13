//! The handle-abstracted `StoreFs` seam (0.10.0, issues #164/#168).
//!
//! PROVES (default lane): `StoreConfig::with_fs` is public and functional over
//! the production `RealFs` backend — a store installed on an explicitly
//! installed backend appends, reads, and verifies its own receipt through the
//! abstract handles, and `ParentDirSyncAdmission` is a shareable,
//! constructor-less proof token.
//!
//! The full parameterized backend contract corpus (the body that gates
//! `RealFs`, `MemFs`, and `ShadowFs`) now lives at `batpak::store::conformance`
//! (feature-gated), driven by `tests/store_fs_conformance_corpus.rs`. This file
//! keeps only the default-lane runtime falsifier (#168).

use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{ParentDirSyncAdmission, RealFs};

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
fn parent_dir_sync_admission_is_a_shareable_proof_token() {
    // The admission token has no public constructor — it is minted by the
    // store's platform-evidence check and handed TO a backend. Its public
    // contract is nameability + the auto-traits a backend relies on to hold
    // or forward it.
    fn assert_token_contract<T: Send + Sync + Copy + std::fmt::Debug>() {}
    assert_token_contract::<ParentDirSyncAdmission>();
}

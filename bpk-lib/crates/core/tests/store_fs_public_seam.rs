//! The promoted `StoreFs` seam (0.10.0, issue #164).
//!
//! PROVES: `StoreConfig::with_fs` is public and functional — a store opened
//! with an explicitly-installed [`batpak::store::RealFs`] backend appends,
//! reads, and verifies identically to the default construction, so the
//! promotion is wired, not merely visible.

use std::io::Write;
use std::sync::Arc;

use batpak::prelude::*;
use batpak::store::{
    CopyPreference, CowStrategyUsed, ParentDirSyncAdmission, PositionedReadError, RealFs, StoreFs,
};

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
    let fs: &dyn StoreFs = &RealFs;

    // create_new_file is exclusive: a second create at the same path must fail
    // rather than silently truncate the first.
    let file_path = dir.path().join("segment.probe");
    let mut file = fs.create_new_file(&file_path)?;
    file.write_all(b"0123456789")?;
    fs.sync_file_all(&file, &file_path)?;
    drop(file);
    let second = fs.create_new_file(&file_path);
    assert!(
        second.is_err(),
        "create_new_file must refuse an existing path (create-new exclusivity)"
    );

    // read_exact_at past the end surfaces the typed short-read error, not a
    // zero-fill or a generic failure.
    let mut reopened = std::fs::File::open(&file_path)?;
    let mut buf = [0u8; 8];
    let short = match fs.read_exact_at(&mut reopened, 6, &mut buf) {
        Ok(()) => {
            return Err(std::io::Error::other(
                "PROPERTY: reading 8 bytes at offset 6 of a 10-byte file must short-read",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(short, PositionedReadError::ShortRead { bytes_read: 4 }),
        "expected ShortRead with 4 bytes read, got {short:?}"
    );

    // cow_copy_file reports the strategy the filesystem actually delivered;
    // DeepCopyOnly must never report a link-based strategy.
    let copy_path = dir.path().join("segment.copy");
    let used = fs.cow_copy_file(&file_path, &copy_path, CopyPreference::DeepCopyOnly)?;
    assert_eq!(
        used,
        CowStrategyUsed::DeepCopy,
        "DeepCopyOnly preference must report a deep copy"
    );
    assert_eq!(fs.metadata(&copy_path)?.len(), 10);

    // remove_file_if_present reports presence honestly on both edges.
    assert!(fs.remove_file_if_present(&copy_path)?);
    assert!(!fs.remove_file_if_present(&copy_path)?);

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

//! Shared backend-contract corpus: every `StoreFs` implementation runs the
//! SAME body (`store_fs_public_seam.rs` drives RealFs;
//! `store_fs_backend_conformance.rs` drives MemFs and proves the corpus
//! detects divergence). One harness, many backends — the shared-drive rule.

use std::path::Path;

use batpak::store::{
    CopyPreference, CowStrategyUsed, DirEntryInfo, FileKind, FileStat, PositionedReadError,
    StagedFile, StoreDirLockGuard, StoreFile, StoreFs,
};

/// The backend contract, over abstract handles only. Runs identically for
/// every `StoreFs` implementation — the shared-drive corpus body.
pub fn backend_upholds_the_documented_contract(
    fs: &dyn StoreFs,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // create_new_file is exclusive and returns a writable abstract handle.
    let file_path = root.join("segment.probe");
    let mut file: Box<dyn StoreFile> = fs.create_new_file(&file_path)?;
    file.write_all(b"0123456789")?;
    file.sync_all()?;
    assert_eq!(file.len()?, 10, "handle length reflects the synced bytes");
    assert!(!file.is_empty()?);
    drop(file);
    let second = fs.create_new_file(&file_path);
    assert!(
        second.is_err(),
        "create_new_file must refuse an existing path (create-new exclusivity)"
    );

    // open_file is the read half of the same seam; positioned reads are on
    // the handle, and a read past the end surfaces the typed short read.
    let mut reopened = fs.open_file(&file_path)?;
    let mut exact = [0u8; 4];
    reopened.read_exact_at(2, &mut exact)?;
    assert_eq!(
        &exact, b"2345",
        "read_exact_at reads at the absolute offset"
    );
    let mut buf = [0u8; 8];
    let short = match reopened.read_exact_at(6, &mut buf) {
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
    assert_eq!(
        short.to_string(),
        "short read: only 4 byte(s) read before EOF"
    );
    assert!(std::error::Error::source(&short).is_none());

    // Whole-file reads route through the seam too (the keyset load path).
    assert_eq!(fs.read(&file_path)?, b"0123456789");

    // Directory enumeration returns owned entries with honest kinds.
    fs.create_dir_all(&root.join("subdir"))?;
    let entries: Vec<DirEntryInfo> = fs.read_dir(root)?;
    let kind_of = |name: &str| -> Option<FileKind> {
        entries
            .iter()
            .find(|e| e.name == std::ffi::OsStr::new(name))
            .map(|e| e.kind)
    };
    assert_eq!(kind_of("segment.probe"), Some(FileKind::File));
    assert_eq!(kind_of("subdir"), Some(FileKind::Dir));

    // Metadata is the owned FileStat, not an OS type.
    let stat: FileStat = fs.metadata(&file_path)?;
    assert_eq!(stat.len, 10);
    assert_eq!(stat.kind, FileKind::File);
    assert_eq!(fs.symlink_metadata(&file_path)?.kind, FileKind::File);

    // Staging: named_temp_in mints a StagedFile the caller writes and syncs.
    // (The atomic publish itself is store-side — `persist` consumes the
    // admission token the platform evidence mints — and is exercised through
    // store operations; dropping an unpersisted stage must leave no final
    // file behind.)
    let staged_final = root.join("staged.artifact");
    let mut staged: Box<dyn StagedFile> = fs.named_temp_in(root)?;
    staged.write_all(b"staged-bytes")?;
    staged.sync_all()?;
    drop(staged);
    assert!(
        matches!(fs.read(&staged_final), Err(error) if error.kind() == std::io::ErrorKind::NotFound),
        "an unpersisted stage must not appear at the final path"
    );

    // cow_copy_file reports the strategy actually delivered.
    let copy_path = root.join("segment.copy");
    let used = fs.cow_copy_file(&file_path, &copy_path, CopyPreference::DeepCopyOnly)?;
    assert_eq!(
        used,
        CowStrategyUsed::DeepCopy,
        "DeepCopyOnly preference must report a deep copy"
    );
    assert_eq!(fs.metadata(&copy_path)?.len, 10);

    // remove_file_if_present reports presence honestly on both edges.
    assert!(fs.remove_file_if_present(&copy_path)?);
    assert!(!fs.remove_file_if_present(&copy_path)?);

    // remove_dir_all clears a directory's contents through the seam — the
    // snapshot/fork cleanup depends on this being backend-agnostic (a host
    // remove_dir_all silently no-ops on a virtual backend, which would leave
    // stale segments/cursor state at a reused destination).
    let cleanup_dir = root.join("cleanup_dir");
    fs.create_dir_all(&cleanup_dir)?;
    let stale = cleanup_dir.join("stale.bin");
    let mut stale_file = fs.create_new_file(&stale)?;
    stale_file.write_all(b"stale-artifact")?;
    drop(stale_file);
    assert!(
        fs.read(&stale).is_ok(),
        "planted artifact exists before removal"
    );
    assert!(
        fs.remove_dir_all_if_present(&cleanup_dir)?,
        "an existing directory reports removed"
    );
    assert!(
        matches!(fs.read(&stale), Err(error) if error.kind() == std::io::ErrorKind::NotFound),
        "remove_dir_all must clear directory contents through the seam"
    );
    // The directory entry itself is gone: a second removal is a clean no-op.
    assert!(
        !fs.remove_dir_all_if_present(&cleanup_dir)?,
        "an already-removed directory reports Ok(false)"
    );
    // Reading a directory path fails closed (not NotFound), so a corrupt
    // virtual store is never mistaken for an absent artifact.
    fs.create_dir_all(&cleanup_dir)?;
    assert!(
        matches!(fs.read(&cleanup_dir), Err(error) if error.kind() != std::io::ErrorKind::NotFound),
        "reading a directory path must fail closed, not report NotFound"
    );
    // Copying onto a directory path fails closed (like RealFs's std::fs::copy),
    // so a reused destination can never become both a file and a directory.
    assert!(
        fs.cow_copy_file(&file_path, &cleanup_dir, CopyPreference::DeepCopyOnly)
            .is_err(),
        "copying onto a directory must fail, not create a file+dir collision"
    );
    // Renaming onto a directory path fails closed too (like RealFs's
    // std::fs::rename), leaving the source intact — never a file+dir collision.
    assert!(
        fs.rename(&file_path, &cleanup_dir).is_err(),
        "renaming onto a directory must fail, not create a file+dir collision"
    );
    assert_eq!(
        fs.metadata(&file_path)?.kind,
        FileKind::File,
        "a rename refused for a directory destination must leave the source intact"
    );

    // The store-directory lock excludes a second cooperating owner while the
    // guard lives, and frees the slot when it drops.
    let lock_path = root.join(".probe.lock");
    let guard: Box<dyn StoreDirLockGuard> = match fs.try_lock_store_dir(&lock_path) {
        Ok(Some(guard)) => guard,
        Ok(None) => return Err(std::io::Error::other("first lock acquisition must succeed").into()),
        Err(error) => return Err(error.into()),
    };
    assert!(
        matches!(fs.try_lock_store_dir(&lock_path), Ok(None)),
        "a held lock must refuse a second owner with Ok(None)"
    );
    drop(guard);
    assert!(
        matches!(fs.try_lock_store_dir(&lock_path), Ok(Some(_))),
        "dropping the guard must release the lock"
    );

    Ok(())
}

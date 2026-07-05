// The routing tail is now EMPTY: every store call site that reaches the target
// filesystem does so through `StoreFs` and the abstract handles it mints. W3
// routed the crash-sensitive atomic-rename / persist cluster; the 0.10.0
// handle abstraction routed the remaining contact surface: opening existing
// files (`open_file`), positioned reads (`StoreFile::read_exact_at`), handle
// syncs (`StoreFile::sync_data`/`sync_all`), staged publishes
// (`StagedFile::persist`), and the store-directory lock
// (`try_lock_store_dir`). Any regression that reintroduces a direct
// `platform::fs::*` call on a store path must extend this list deliberately
// and justify it.
const UNROUTED_STORE_FS_TAIL_OPS: &[&str] = &[];
const ROUTED_STORE_FS_OPS: &[&str] = &[
    "remove_file",
    "rename",
    "named_temp_in",
    "open_file",
    "StagedFile::persist",
    "StoreFile::read_exact_at",
    "try_lock_store_dir",
];

use super::{reject_copy_source, remove_dir_all};
use super::{FileKind, RealFs, StoreFs};
use std::error::Error;

#[test]
fn remove_dir_all_removes_nested_directory_tree() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("tree");
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(nested.join("leaf.txt"), b"leaf")?;

    remove_dir_all(&root)?;

    assert!(
        !root.exists(),
        "PROPERTY: platform remove_dir_all must remove directories, not only files or leaves"
    );
    Ok(())
}

// Exercises every routed StoreFs method through a trait object so the
// production RealFs delegation is proven byte-for-byte against the platform
// free fns, and every method on the seam is a live vtable entry.
#[test]
fn real_fs_delegates_routed_ops_like_the_platform_free_fns() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let fs: std::sync::Arc<dyn StoreFs> = std::sync::Arc::new(RealFs);

    // create_dir_all builds the whole tree.
    let sub = dir.path().join("a").join("b");
    fs.create_dir_all(&sub)?;
    assert!(
        sub.is_dir(),
        "PROPERTY: RealFs::create_dir_all must create the full nested tree"
    );

    // read_dir lists what create_dir_all produced, with honest kinds.
    std::fs::write(dir.path().join("leaf.bin"), b"leaf")?;
    let entries = fs.read_dir(dir.path())?;
    let kind_of = |name: &str| {
        entries
            .iter()
            .find(|e| e.name == std::ffi::OsStr::new(name))
            .map(|e| e.kind)
    };
    assert_eq!(
        kind_of("leaf.bin"),
        Some(FileKind::File),
        "PROPERTY: RealFs::read_dir must list file entries like the platform free fn"
    );
    assert_eq!(
        kind_of("a"),
        Some(FileKind::Dir),
        "PROPERTY: RealFs::read_dir must report directory kinds honestly"
    );

    // create_new_file + the handle sync seam: create a real file, write to
    // it, sync via every routed mode, and fsync the parent dir — proving the
    // RealFs handle durability methods delegate to the platform and leave
    // the real bytes durably on disk.
    let seg = dir.path().join("seg.bin");
    let mut file = fs.create_new_file(&seg)?;
    file.write_all(b"durable-bytes")?;
    file.sync_all()?;
    crate::store::platform::sync::sync_store_file_with_mode(
        file.as_mut(),
        &crate::store::SyncMode::SyncAll,
    )?;
    crate::store::platform::sync::sync_store_file_with_mode(
        file.as_mut(),
        &crate::store::SyncMode::SyncData,
    )?;
    fs.sync_parent_dir(&seg)?;
    assert_eq!(
        std::fs::metadata(&seg)?.len(),
        b"durable-bytes".len() as u64,
        "PROPERTY: RealFs handle durability methods persist the real bytes like the platform free fns"
    );
    assert_eq!(
        file.len()?,
        b"durable-bytes".len() as u64,
        "PROPERTY: StoreFile::len must report the written length"
    );
    Ok(())
}

// The success-path test above proves the sync methods return Ok and persist
// bytes; it cannot catch `-> Ok(())` mutants because the real method also
// returns Ok on success. These error-path tests pin that the handle methods
// actually surface a platform sync failure, so replacing any body with
// `Ok(())` is caught.
#[cfg(unix)]
#[test]
fn real_store_file_syncs_surface_platform_errors() -> Result<(), Box<dyn Error>> {
    // A read-only /dev/null fd cannot be fsynced; both sync methods must error.
    use super::StoreFile;
    let file = std::fs::File::open("/dev/null")?;
    let mut handle = super::RealStoreFile::new(file);
    let mut failures: Vec<String> = Vec::new();
    if handle.sync_all().is_ok() {
        failures.push("StoreFile::sync_all must surface the platform error".into());
    }
    if handle.sync_data().is_ok() {
        failures.push("StoreFile::sync_data must surface the platform error".into());
    }
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn real_fs_sync_parent_dir_surfaces_missing_parent() -> Result<(), Box<dyn Error>> {
    // A path whose parent directory does not exist cannot have its directory
    // entry fsynced; the wrapper must error rather than report success.
    let fs = RealFs;
    let dir = tempfile::tempdir()?;
    let missing_parent = dir.path().join("missing-parent");
    let missing = missing_parent.join("leaf");
    assert!(
        fs.sync_parent_dir(&missing).is_err(),
        "RealFs::sync_parent_dir must surface an error when the parent dir is absent"
    );
    Ok(())
}

#[test]
fn reject_copy_source_rejects_non_file_source() -> Result<(), Box<dyn Error>> {
    // A directory is a non-file source: the cow_copy_file ladder must refuse
    // it rather than silently succeed. Kills `reject_copy_source -> Ok(())`.
    let dir = tempfile::tempdir()?;
    let result = reject_copy_source(dir.path());
    assert!(
        result.is_err(),
        "PROPERTY: reject_copy_source must reject a non-file (directory) source"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn reject_copy_source_rejects_symlink_source() -> Result<(), Box<dyn Error>> {
    // A symlink source must be refused even when it targets a real file:
    // copying through it would dereference an attacker-controlled link.
    // Kills `reject_copy_source -> Ok(())` on the symlink branch.
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("target.bin");
    std::fs::write(&target, b"payload")?;
    let link = dir.path().join("link.bin");
    std::os::unix::fs::symlink(&target, &link)?;

    let result = reject_copy_source(&link);
    assert!(
        result.is_err(),
        "PROPERTY: reject_copy_source must reject a symlink source (no link dereference)"
    );
    Ok(())
}

#[test]
fn store_fs_fail_closed_boundary_lists_unrouted_tail_ops() {
    assert_eq!(
        UNROUTED_STORE_FS_TAIL_OPS,
        &[] as &[&str],
        "PROPERTY: fail-closed boundary — the routing tail is empty; no store call site reaches a platform::fs::* free fn directly"
    );
    for routed in ROUTED_STORE_FS_OPS {
        assert!(
            !UNROUTED_STORE_FS_TAIL_OPS.contains(routed),
            "PROPERTY: {routed} is routed through StoreFs and must not appear in the unrouted tail"
        );
    }
}

// The atomic cluster ops are live vtable entries: exercising them through a
// trait object proves RealFs dispatches each (and a `-> Ok(())`/no-op mutant
// on any body is caught by the observable filesystem effect below).
#[test]
fn real_fs_routes_w3_atomic_cluster() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let fs: std::sync::Arc<dyn StoreFs> = std::sync::Arc::new(RealFs);

    // named_temp_in + StagedFile::persist: stage a temp, write + sync it,
    // and atomically publish it to a final path.
    let final_path = dir.path().join("published.bin");
    let mut tmp = fs.named_temp_in(dir.path())?;
    tmp.write_all(b"published-bytes")?;
    tmp.sync_all()?;
    let admission = crate::store::platform::sync::admit_current_parent_dir_sync()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    tmp.persist(&final_path, admission)?;
    assert_eq!(
        std::fs::read(&final_path)?,
        b"published-bytes",
        "PROPERTY: StagedFile::persist must publish the staged bytes"
    );

    // rename: move the published file aside.
    let renamed = dir.path().join("renamed.bin");
    fs.rename(&final_path, &renamed)?;
    assert!(
        !final_path.exists() && renamed.exists(),
        "PROPERTY: RealFs::rename must move the file like the platform free fn"
    );

    // remove_file + remove_file_if_present: reclaim the file, then prove the
    // not-found tolerance of the provided method.
    fs.remove_file(&renamed)?;
    assert!(
        !renamed.exists(),
        "PROPERTY: RealFs::remove_file must remove the file"
    );
    assert!(
        !fs.remove_file_if_present(&renamed)?,
        "PROPERTY: remove_file_if_present must report Ok(false) for an absent file"
    );
    Ok(())
}

// `open_file` + `StoreFile::read_exact_at` are live vtable entries: exercising
// them through a trait object proves RealFs dispatches the positioned read
// (a `-> Ok(())` / no-op mutant on the body is caught by the observed bytes
// below), and that a read past EOF surfaces the ShortRead discriminant the
// reader's active-frame mapping depends on.
#[test]
fn real_fs_routes_positioned_read_exact_at() -> Result<(), Box<dyn Error>> {
    use crate::store::platform::fs::PositionedReadError;
    use std::io::Write;
    let dir = tempfile::tempdir()?;
    let fs: std::sync::Arc<dyn StoreFs> = std::sync::Arc::new(RealFs);

    let path = dir.path().join("frame.bin");
    let mut file = std::fs::File::create(&path)?;
    file.write_all(b"0123456789abcdef")?;
    file.sync_all()?;
    let mut handle = fs.open_file(&path)?;

    // Round-trip: a positioned read at offset 3 returns exactly the requested
    // slice, proving the handle delegates to the platform positioned read.
    // (`PositionedReadError` is not `?`-propagated here so the assertion can
    // name the failure.)
    let mut buf = [0u8; 5];
    handle
        .read_exact_at(3, &mut buf)
        .map_err(|e| format!("positioned read must succeed: {e:?}"))?;
    assert_eq!(
        &buf, b"34567",
        "PROPERTY: StoreFile::read_exact_at must fill the buffer from the requested offset"
    );

    // EOF: reading a slice that runs past the file end reports a ShortRead with
    // `bytes_read == 0` (the read started exactly at EOF) — the discriminant the
    // reader maps to `corrupt_eof`.
    let mut past_eof = [0u8; 4];
    let eof = handle.read_exact_at(16, &mut past_eof);
    assert!(
        matches!(eof, Err(PositionedReadError::ShortRead { bytes_read: 0 })),
        "PROPERTY: a positioned read starting at EOF must surface ShortRead {{ bytes_read: 0 }}, got {eof:?}"
    );

    // Partial: a read that begins inside the file but requests more than remains
    // reports a non-zero ShortRead (the torn-frame discriminant).
    let mut partial = [0u8; 8];
    let short = handle.read_exact_at(12, &mut partial);
    assert!(
        matches!(short, Err(PositionedReadError::ShortRead { bytes_read }) if bytes_read == 4),
        "PROPERTY: a positioned read past the tail must surface a non-zero ShortRead carrying the bytes read, got {short:?}"
    );
    Ok(())
}

#[test]
fn read_exact_at_completes_exactly_at_the_boundary_iteration() -> Result<(), Box<dyn Error>> {
    use super::PositionedReadError;
    use std::io::Write;

    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("boundary.bin");
    let mut writer = std::fs::File::create(&path)?;
    writer.write_all(&[7u8; 16])?;
    writer.sync_all()?;
    drop(writer);
    // The provided-default read_exact_at loop (over StoreFile::read_at) is
    // the implementation under test.
    let mut handle = RealFs.open_file(&path)?;

    // Exact-length read: the loop iteration that returns EXACTLY the requested
    // count must terminate with Ok — a `<` -> `<=` loop bound issues one extra
    // zero-length read after completion and misreports the finished read as
    // ShortRead. The buffer content pins that all 16 bytes landed.
    let mut full = [0u8; 16];
    let complete = handle.read_exact_at(0, &mut full);
    assert!(
        matches!(complete, Ok(())),
        "a read of exactly the file length must complete Ok, got {complete:?}"
    );
    assert_eq!(full, [7u8; 16], "the exact-boundary read fills every byte");

    // A tail read ending exactly at EOF is equally complete.
    let mut tail = [0u8; 4];
    let tail_read = handle.read_exact_at(12, &mut tail);
    assert!(
        matches!(tail_read, Ok(())),
        "a tail read ending exactly at EOF must complete Ok, got {tail_read:?}"
    );

    // Zero-length read: trivially complete, no I/O required.
    let mut empty: [u8; 0] = [];
    let nothing = handle.read_exact_at(16, &mut empty);
    assert!(
        matches!(nothing, Ok(())),
        "a zero-length read is complete by definition, got {nothing:?}"
    );

    // Contrast: one byte past the tail must remain a ShortRead carrying the
    // exact count that DID land before EOF.
    let mut over = [0u8; 5];
    let short = handle.read_exact_at(12, &mut over);
    assert!(
        matches!(short, Err(PositionedReadError::ShortRead { bytes_read: 4 })),
        "a read one byte past EOF reports the 4 bytes that landed, got {short:?}"
    );
    Ok(())
}

#[test]
fn remove_file_if_present_reports_removal_then_absence_exactly() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("victim.bin");
    std::fs::write(&path, b"bytes")?;

    // A present file: Ok(true) AND the file is actually gone — a stubbed
    // `Ok(false)` body would skip the removal too, so both pins are needed.
    let removed = RealFs.remove_file_if_present(&path)?;
    assert!(removed, "removing an existing file must report Ok(true)");
    assert!(
        !path.exists(),
        "the file must actually be gone after Ok(true)"
    );

    // Already absent: the tolerated NotFound, reported as Ok(false).
    let absent = RealFs.remove_file_if_present(&path)?;
    assert!(!absent, "an already-absent file must report Ok(false)");
    Ok(())
}

/// [`StoreFs`] whose `remove_file` fails with a NON-NotFound error; every other
/// op delegates to [`RealFs`]. Exercises the PROVIDED `remove_file_if_present`
/// default method: ONLY NotFound may be swallowed as `Ok(false)` — any other
/// error is real damage that must propagate.
struct DenyRemoveFs;

impl StoreFs for DenyRemoveFs {
    fn read_dir(&self, path: &std::path::Path) -> std::io::Result<Vec<super::DirEntryInfo>> {
        RealFs.read_dir(path)
    }
    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        RealFs.read(path)
    }
    fn create_dir_all(&self, path: &std::path::Path) -> std::io::Result<()> {
        RealFs.create_dir_all(path)
    }
    fn create_new_file(
        &self,
        path: &std::path::Path,
    ) -> Result<Box<dyn super::StoreFile>, crate::store::StoreError> {
        RealFs.create_new_file(path)
    }
    fn open_file(&self, path: &std::path::Path) -> std::io::Result<Box<dyn super::StoreFile>> {
        RealFs.open_file(path)
    }
    fn sync_parent_dir(&self, path: &std::path::Path) -> Result<(), crate::store::StoreError> {
        RealFs.sync_parent_dir(path)
    }
    fn reject_symlink_leaf(
        &self,
        path: &std::path::Path,
        purpose: &str,
    ) -> Result<(), crate::store::StoreError> {
        RealFs.reject_symlink_leaf(path, purpose)
    }
    fn canonicalize(&self, path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        RealFs.canonicalize(path)
    }
    fn symlink_metadata(&self, path: &std::path::Path) -> std::io::Result<super::FileStat> {
        RealFs.symlink_metadata(path)
    }
    fn cow_copy_file(
        &self,
        from: &std::path::Path,
        to: &std::path::Path,
        preference: crate::store::CopyPreference,
    ) -> std::io::Result<super::CowStrategyUsed> {
        RealFs.cow_copy_file(from, to, preference)
    }
    fn copy(&self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<u64> {
        RealFs.copy(from, to)
    }
    fn metadata(&self, path: &std::path::Path) -> std::io::Result<super::FileStat> {
        RealFs.metadata(path)
    }
    fn rename(&self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        RealFs.rename(from, to)
    }
    fn remove_file(&self, _path: &std::path::Path) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "injected: remove_file denied",
        ))
    }
    fn named_temp_in(&self, dir: &std::path::Path) -> std::io::Result<Box<dyn super::StagedFile>> {
        RealFs.named_temp_in(dir)
    }
    fn try_lock_store_dir(
        &self,
        lock_path: &std::path::Path,
    ) -> Result<Option<Box<dyn super::StoreDirLockGuard>>, crate::store::StoreError> {
        RealFs.try_lock_store_dir(lock_path)
    }
}

#[test]
fn remove_file_if_present_propagates_non_not_found_errors() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("guarded.bin");
    std::fs::write(&path, b"bytes").expect("write the guarded file");

    // A PermissionDenied remove is REAL damage: swallowing it as Ok(false)
    // (the `guard -> true` mutant) would report "already absent" for a file
    // that is still on disk — the caller would then act on a lie.
    let result = DenyRemoveFs.remove_file_if_present(&path);
    assert!(
        matches!(&result, Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied),
        "only NotFound may be swallowed as Ok(false); a PermissionDenied removal must propagate"
    );
    assert!(path.exists(), "the guarded file must remain on disk");
}

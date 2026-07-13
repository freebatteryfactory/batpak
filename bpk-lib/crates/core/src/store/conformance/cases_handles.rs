//! Conformance corpus — the `Handles` family (C1–C7): a backend's abstract
//! `StoreFile` seam over `create_new_file` / `open_file` / positioned reads /
//! metadata / directory enumeration. Migrated in substance from the retired
//! `tests/common/mod.rs` body, converted from `assert!` to the typed `check`
//! helper — src is the production lint domain, so a case NEVER asserts/panics;
//! a violation is a returned `CaseFailure::Violation`.

use std::path::PathBuf;

use super::{check, CaseCx, CaseFailure, CaseResult};
use crate::store::{DirEntryInfo, FileKind, PositionedReadError, StoreFile};

/// Map any fallible seam call into a corpus violation carrying `what` as context.
/// The corpus reports typed failures; it does not unwrap or panic.
fn or_violation<T, E: std::fmt::Display>(result: Result<T, E>, what: &str) -> Result<T, CaseFailure> {
    result.map_err(|error| CaseFailure::Violation(format!("{what}: {error}")))
}

/// Fresh probe file: `segment.probe` holding the 10 synced bytes every handle
/// case reads back. The handle is dropped on return (the name/bytes are durable).
fn write_probe(cx: &CaseCx<'_>) -> Result<PathBuf, CaseFailure> {
    let path = cx.root.join("segment.probe");
    let mut file = or_violation(cx.fs.create_new_file(&path), "create_new_file(segment.probe)")?;
    or_violation(file.write_all(b"0123456789"), "write_all")?;
    or_violation(file.sync_all(), "sync_all")?;
    Ok(path)
}

/// C1 `create-new-exclusive`: a second create-new on the same path must refuse —
/// a segment can never silently truncate a predecessor.
pub(crate) fn create_new_exclusive(cx: &CaseCx<'_>) -> CaseResult {
    let path = write_probe(cx)?;
    let second = cx.fs.create_new_file(&path);
    check(
        second.is_err(),
        "create_new_file must refuse an existing path (create-new exclusivity)",
    )
}

/// C2 `handle-length-reflects-synced-bytes`: the live handle reports the synced
/// length and is not empty.
pub(crate) fn handle_length_reflects_synced_bytes(cx: &CaseCx<'_>) -> CaseResult {
    let path = cx.root.join("segment.probe");
    let mut file = or_violation(cx.fs.create_new_file(&path), "create_new_file")?;
    or_violation(file.write_all(b"0123456789"), "write_all")?;
    or_violation(file.sync_all(), "sync_all")?;
    let len = or_violation(file.len(), "len")?;
    check(len == 10, format!("handle length reflects the synced bytes: got {len}"))?;
    let empty = or_violation(file.is_empty(), "is_empty")?;
    check(!empty, "a 10-byte handle must not report empty")
}

/// C3 `positioned-read-exact`: `read_exact_at` reads at the absolute offset on a
/// freshly reopened handle.
pub(crate) fn positioned_read_exact(cx: &CaseCx<'_>) -> CaseResult {
    let path = write_probe(cx)?;
    let mut reopened = or_violation(cx.fs.open_file(&path), "open_file")?;
    let mut exact = [0u8; 4];
    or_violation(reopened.read_exact_at(2, &mut exact), "read_exact_at(2)")?;
    check(
        &exact == b"2345",
        format!("read_exact_at reads at the absolute offset: got {exact:?}"),
    )
}

/// C4 `positioned-short-read-typed`: reading past EOF surfaces the exact typed
/// short read — variant, stable Display, and no source.
pub(crate) fn positioned_short_read_typed(cx: &CaseCx<'_>) -> CaseResult {
    let path = write_probe(cx)?;
    let mut reopened = or_violation(cx.fs.open_file(&path), "open_file")?;
    let mut buf = [0u8; 8];
    let short = match reopened.read_exact_at(6, &mut buf) {
        Ok(()) => {
            return Err(CaseFailure::Violation(
                "reading 8 bytes at offset 6 of a 10-byte file must short-read".into(),
            ))
        }
        Err(error) => error,
    };
    check(
        matches!(short, PositionedReadError::ShortRead { bytes_read: 4 }),
        format!("expected ShortRead {{ bytes_read: 4 }}, got {short:?}"),
    )?;
    check(
        short.to_string() == "short read: only 4 byte(s) read before EOF",
        format!("short-read Display must be the stable message, got {short}"),
    )?;
    check(
        std::error::Error::source(&short).is_none(),
        "a short read has no error source",
    )
}

/// C5 `whole-file-read-routes-through-seam`: the whole-file read serves the exact
/// bytes through the seam (the keyset load path).
pub(crate) fn whole_file_read_routes_through_seam(cx: &CaseCx<'_>) -> CaseResult {
    let path = write_probe(cx)?;
    let bytes = or_violation(cx.fs.read(&path), "read")?;
    check(
        bytes == b"0123456789",
        format!("whole-file read returns the exact bytes, got {bytes:?}"),
    )
}

/// C6 `metadata-owned-types`: `metadata`/`symlink_metadata` return the owned
/// `FileStat`, not an OS type.
pub(crate) fn metadata_owned_types(cx: &CaseCx<'_>) -> CaseResult {
    let path = write_probe(cx)?;
    let stat = or_violation(cx.fs.metadata(&path), "metadata")?;
    check(stat.len == 10, format!("metadata len is 10, got {}", stat.len))?;
    check(
        stat.kind == FileKind::File,
        format!("metadata kind is File, got {:?}", stat.kind),
    )?;
    let sym = or_violation(cx.fs.symlink_metadata(&path), "symlink_metadata")?;
    check(
        sym.kind == FileKind::File,
        format!("symlink_metadata kind is File, got {:?}", sym.kind),
    )
}

/// C7 `read-dir-owned-kinds`: directory enumeration returns owned `DirEntryInfo`
/// with honest kinds for both a file and a subdirectory.
pub(crate) fn read_dir_owned_kinds(cx: &CaseCx<'_>) -> CaseResult {
    let _probe = write_probe(cx)?;
    or_violation(
        cx.fs.create_dir_all(&cx.root.join("subdir")),
        "create_dir_all(subdir)",
    )?;
    let entries: Vec<DirEntryInfo> = or_violation(cx.fs.read_dir(cx.root), "read_dir")?;
    let kind_of = |name: &str| -> Option<FileKind> {
        entries
            .iter()
            .find(|entry| entry.name == std::ffi::OsStr::new(name))
            .map(|entry| entry.kind)
    };
    check(
        kind_of("segment.probe") == Some(FileKind::File),
        format!("segment.probe reports File, got {:?}", kind_of("segment.probe")),
    )?;
    check(
        kind_of("subdir") == Some(FileKind::Dir),
        format!("subdir reports Dir, got {:?}", kind_of("subdir")),
    )
}

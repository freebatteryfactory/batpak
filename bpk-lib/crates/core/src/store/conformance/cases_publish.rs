//! Conformance corpus — Publish family (C8–C23).
//!
//! Staging/publish, copy, rename, and removal obligations of the `StoreFs`
//! seam: an atomic stage→persist that leaves no torn or orphan name, same-path
//! copy/rename that never destroys the source, fail-closed publish edges that
//! keep the source intact, and directory-clearing that routes through the seam.
//! Bodies migrate the shared `tests/common/mod.rs` corpus into typed
//! `CaseResult`s — a failed precondition is a [`CaseFailure::Violation`], never
//! an `assert!`/`panic!`/`.expect` (this is the production lint domain).

use std::io;
use std::path::{Path, PathBuf};

use crate::store::{CopyPreference, CowStrategyUsed, FileKind, StagedFile, StoreFile, StoreFs};

use super::{check, CaseCx, CaseFailure, CaseResult};

/// A setup/read-back step whose failure is an unexpected corpus violation
/// (the backend was asked to do something the contract guarantees). Keeps the
/// bodies free of `.expect`, which the production lint domain denies.
fn require<T>(result: Result<T, impl std::fmt::Display>, step: &str) -> Result<T, CaseFailure> {
    result.map_err(|error| CaseFailure::Violation(format!("{step} failed: {error}")))
}

/// Plant a durable 10-byte source file the publish cases operate on.
fn plant_probe_file(cx: &CaseCx<'_>) -> Result<PathBuf, CaseFailure> {
    let path = cx.root.join("segment.probe");
    let mut file = require(cx.fs.create_new_file(&path), "create the probe file")?;
    require(file.write_all(b"0123456789"), "write the probe file")?;
    require(file.sync_all(), "sync the probe file")?;
    drop(file);
    Ok(path)
}

/// A publish edge refused for a bad destination must leave the source intact.
fn source_still_file(cx: &CaseCx<'_>, path: &Path, edge: &str) -> CaseResult {
    let stat = require(cx.fs.metadata(path), "stat the source after a refused publish")?;
    check(
        stat.kind == FileKind::File,
        format!("{edge} must leave the source a file, kind is {:?}", stat.kind),
    )
}

/// C8: dropping an unpersisted stage leaves no final name.
pub(crate) fn unpersisted_stage_leaves_no_final_name(cx: &CaseCx<'_>) -> CaseResult {
    let final_path = cx.root.join("staged.artifact");
    let mut staged = require(cx.fs.named_temp_in(cx.root), "stage a temp file")?;
    require(staged.write_all(b"staged-bytes"), "write staged bytes")?;
    require(staged.sync_all(), "sync staged bytes")?;
    drop(staged);
    let observed = cx.fs.read(&final_path);
    check(
        matches!(&observed, Err(error) if error.kind() == io::ErrorKind::NotFound),
        format!("an unpersisted stage must not appear at the final path, got {observed:?}"),
    )
}

/// C9: stage → write → sync → persist installs the exact bytes at a fresh path.
pub(crate) fn staged_publish_atomic_fresh_path(cx: &CaseCx<'_>) -> CaseResult {
    let final_path = cx.root.join("published.artifact");
    let mut staged = require(cx.fs.named_temp_in(cx.root), "stage a temp file")?;
    require(staged.write_all(b"0123456789"), "write staged bytes")?;
    require(staged.sync_all(), "sync staged bytes")?;
    let admission = require(
        crate::store::platform::sync::admit_current_parent_dir_sync(),
        "admit the parent-dir sync",
    )?;
    require(staged.persist(&final_path, admission), "persist the staged bytes")?;
    let read_back = require(cx.fs.read(&final_path), "read the published file")?;
    check(
        read_back == b"0123456789",
        format!("published bytes must round-trip exactly, got {read_back:?}"),
    )
}

/// C10: persisting onto a directory path fails closed; the path stays a dir.
pub(crate) fn staged_publish_onto_directory_fails_closed(cx: &CaseCx<'_>) -> CaseResult {
    let dir_path = cx.root.join("occupied_dir");
    require(cx.fs.create_dir_all(&dir_path), "create the occupying directory")?;
    let mut staged = require(cx.fs.named_temp_in(cx.root), "stage a temp file")?;
    require(staged.write_all(b"staged-bytes"), "write staged bytes")?;
    require(staged.sync_all(), "sync staged bytes")?;
    let admission = require(
        crate::store::platform::sync::admit_current_parent_dir_sync(),
        "admit the parent-dir sync",
    )?;
    let outcome = staged.persist(&dir_path, admission);
    check(
        outcome.is_err(),
        format!("persisting onto a directory must fail closed, got {outcome:?}"),
    )?;
    check(
        matches!(cx.fs.metadata(&dir_path), Ok(stat) if stat.kind == FileKind::Dir),
        "the destination must remain a directory after a refused persist",
    )
}

/// C11: persisting into a missing parent fails closed and orphans nothing.
pub(crate) fn staged_publish_missing_parent_fails_closed(cx: &CaseCx<'_>) -> CaseResult {
    let target = cx.root.join("absent_dir").join("target.bin");
    let mut staged = require(cx.fs.named_temp_in(cx.root), "stage a temp file")?;
    require(staged.write_all(b"staged-bytes"), "write staged bytes")?;
    require(staged.sync_all(), "sync staged bytes")?;
    let admission = require(
        crate::store::platform::sync::admit_current_parent_dir_sync(),
        "admit the parent-dir sync",
    )?;
    let outcome = staged.persist(&target, admission);
    check(
        outcome.is_err(),
        format!("persisting into a missing parent must fail closed, got {outcome:?}"),
    )?;
    check(
        matches!(cx.fs.read(&target), Err(error) if error.kind() == io::ErrorKind::NotFound),
        "a refused persist must not leave an orphan at the target path",
    )
}

/// C12: `cow_copy_file(DeepCopyOnly)` reports the deep-copy strategy honestly.
pub(crate) fn copy_reports_strategy_honestly(cx: &CaseCx<'_>) -> CaseResult {
    let source = plant_probe_file(cx)?;
    let copy_path = cx.root.join("segment.copy");
    let used = require(
        cx.fs.cow_copy_file(&source, &copy_path, CopyPreference::DeepCopyOnly),
        "deep-copy the source",
    )?;
    check(
        used == CowStrategyUsed::DeepCopy,
        format!("DeepCopyOnly must report a deep copy, got {used:?}"),
    )?;
    let stat = require(cx.fs.metadata(&copy_path), "stat the copy")?;
    check(stat.len == 10, format!("the copy must have length 10, got {}", stat.len))
}

/// C13: `copy(p, p)` / `cow_copy_file(p, p, ..)` must never destroy the source.
pub(crate) fn same_path_copy_non_destructive(cx: &CaseCx<'_>) -> CaseResult {
    let path = plant_probe_file(cx)?;
    // Ok(len) or a typed refusal are both fine; only DESTRUCTION is a violation.
    let _ = cx.fs.copy(&path, &path);
    let after_copy = require(cx.fs.read(&path), "read after a same-path copy")?;
    check(
        after_copy == b"0123456789",
        format!("copy(p, p) must not destroy the file, got {after_copy:?}"),
    )?;
    let _ = cx.fs.cow_copy_file(&path, &path, CopyPreference::DeepCopyOnly);
    let after_cow = require(cx.fs.read(&path), "read after a same-path cow copy")?;
    check(
        after_cow == b"0123456789",
        format!("cow_copy_file(p, p) must not destroy the file, got {after_cow:?}"),
    )
}

/// C14: `rename(p, p)` is a non-destructive no-op (never a deletion).
pub(crate) fn same_path_rename_non_destructive(cx: &CaseCx<'_>) -> CaseResult {
    let path = plant_probe_file(cx)?;
    // Ok no-op (POSIX) or a typed refusal are both fine; deletion is a violation.
    let _ = cx.fs.rename(&path, &path);
    let after = require(cx.fs.read(&path), "read after a same-path rename")?;
    check(
        after == b"0123456789",
        format!("rename(p, p) must not delete the file, got {after:?}"),
    )
}

/// C15: renaming onto a directory fails closed; source stays a file.
pub(crate) fn rename_onto_directory_fails_closed_source_intact(cx: &CaseCx<'_>) -> CaseResult {
    let source = plant_probe_file(cx)?;
    let dir_path = cx.root.join("occupied_dir");
    require(cx.fs.create_dir_all(&dir_path), "create the occupying directory")?;
    let outcome = cx.fs.rename(&source, &dir_path);
    check(
        outcome.is_err(),
        format!("renaming onto a directory must fail closed, got {outcome:?}"),
    )?;
    source_still_file(cx, &source, "a rename onto a directory")
}

/// C16: renaming into a missing parent fails closed; source stays a file.
pub(crate) fn rename_missing_parent_fails_closed_source_intact(cx: &CaseCx<'_>) -> CaseResult {
    let source = plant_probe_file(cx)?;
    let target = cx.root.join("absent_dir").join("target.bin");
    let outcome = cx.fs.rename(&source, &target);
    check(
        outcome.is_err(),
        format!("renaming into a missing parent must fail closed, got {outcome:?}"),
    )?;
    source_still_file(cx, &source, "a rename into a missing parent")
}

/// C17: copying onto a directory fails closed; source stays a file.
pub(crate) fn copy_onto_directory_fails_closed(cx: &CaseCx<'_>) -> CaseResult {
    let source = plant_probe_file(cx)?;
    let dir_path = cx.root.join("occupied_dir");
    require(cx.fs.create_dir_all(&dir_path), "create the occupying directory")?;
    let outcome = cx.fs.cow_copy_file(&source, &dir_path, CopyPreference::DeepCopyOnly);
    check(
        outcome.is_err(),
        format!("copying onto a directory must fail closed, got {outcome:?}"),
    )?;
    source_still_file(cx, &source, "a copy onto a directory")
}

/// C18: copying into a missing parent fails closed; source stays a file.
pub(crate) fn copy_missing_parent_fails_closed(cx: &CaseCx<'_>) -> CaseResult {
    let source = plant_probe_file(cx)?;
    let target = cx.root.join("absent_dir").join("target.bin");
    let outcome = cx.fs.cow_copy_file(&source, &target, CopyPreference::DeepCopyOnly);
    check(
        outcome.is_err(),
        format!("copying into a missing parent must fail closed, got {outcome:?}"),
    )?;
    source_still_file(cx, &source, "a copy into a missing parent")
}

/// C19: `remove_file_if_present` reports presence honestly on both edges.
pub(crate) fn remove_file_if_present_honest(cx: &CaseCx<'_>) -> CaseResult {
    let path = plant_probe_file(cx)?;
    let first = require(cx.fs.remove_file_if_present(&path), "remove the planted file")?;
    check(first, "remove_file_if_present must report Ok(true) for a present file")?;
    let second = require(cx.fs.remove_file_if_present(&path), "remove the absent file")?;
    check(!second, "remove_file_if_present must report Ok(false) once absent")
}

/// C20: `remove_file` on a directory errors (never a silent `Ok(false)`).
pub(crate) fn remove_file_on_directory_errors(cx: &CaseCx<'_>) -> CaseResult {
    let dir_path = cx.root.join("occupied_dir");
    require(cx.fs.create_dir_all(&dir_path), "create the directory")?;
    let outcome = cx.fs.remove_file_if_present(&dir_path);
    check(
        outcome.is_err(),
        format!("remove_file on a directory must error, not report Ok(false), got {outcome:?}"),
    )
}

/// C21: `read_dir` on a file fails closed (not-a-directory), never `NotFound`.
pub(crate) fn read_dir_on_file_fails_closed_not_not_found(cx: &CaseCx<'_>) -> CaseResult {
    let path = plant_probe_file(cx)?;
    let outcome = cx.fs.read_dir(&path);
    check(
        matches!(&outcome, Err(error) if error.kind() != io::ErrorKind::NotFound),
        format!("read_dir on a file must fail closed (not NotFound), got {outcome:?}"),
    )
}

/// C22: `read` on a directory fails closed, never `NotFound`.
pub(crate) fn read_on_directory_fails_closed_not_not_found(cx: &CaseCx<'_>) -> CaseResult {
    let dir_path = cx.root.join("occupied_dir");
    require(cx.fs.create_dir_all(&dir_path), "create the directory")?;
    let outcome = cx.fs.read(&dir_path);
    check(
        matches!(&outcome, Err(error) if error.kind() != io::ErrorKind::NotFound),
        format!("reading a directory path must fail closed (not NotFound), got {outcome:?}"),
    )
}

/// C23: `remove_dir_all_if_present` clears directory contents through the seam.
pub(crate) fn remove_dir_all_clears_through_seam(cx: &CaseCx<'_>) -> CaseResult {
    let cleanup_dir = cx.root.join("cleanup_dir");
    require(cx.fs.create_dir_all(&cleanup_dir), "create the cleanup directory")?;
    let stale = cleanup_dir.join("stale.bin");
    let mut stale_file = require(cx.fs.create_new_file(&stale), "plant a stale artifact")?;
    require(stale_file.write_all(b"stale-artifact"), "write the stale artifact")?;
    drop(stale_file);
    check(cx.fs.read(&stale).is_ok(), "the planted artifact must exist before removal")?;
    let removed = require(cx.fs.remove_dir_all_if_present(&cleanup_dir), "remove the cleanup directory")?;
    check(removed, "removing an existing directory must report Ok(true)")?;
    check(
        matches!(cx.fs.read(&stale), Err(error) if error.kind() == io::ErrorKind::NotFound),
        "remove_dir_all must clear directory contents through the seam",
    )?;
    let again = require(
        cx.fs.remove_dir_all_if_present(&cleanup_dir),
        "remove the already-removed directory",
    )?;
    check(!again, "an already-removed directory must report Ok(false)")
}

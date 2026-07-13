//! LockLifecycle family (C24–C26): store-directory lock exclusivity and the
//! delete-then-recreate identity edge. Each case runs over `&dyn StoreFs` only
//! and reports typed failures — never `assert!`/panic (production lint domain).

use super::{CaseCx, CaseFailure, CaseResult, check};

/// C24 `lock-excludes-second-owner`: the first `try_lock_store_dir` acquires the
/// slot; while the guard lives a second acquisition must report `Ok(None)`.
pub(crate) fn lock_excludes_second_owner(cx: &CaseCx<'_>) -> CaseResult {
    let lock_path = cx.root.join(".probe.lock");
    // The guard must outlive the second acquisition — bind it, do not discard.
    let _guard = match cx.fs.try_lock_store_dir(&lock_path) {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            return Err(CaseFailure::Violation(
                "first lock acquisition must succeed, got Ok(None)".into(),
            ))
        }
        Err(error) => {
            return Err(CaseFailure::Violation(format!(
                "first lock acquisition errored: {error}"
            )))
        }
    };
    check(
        matches!(cx.fs.try_lock_store_dir(&lock_path), Ok(None)),
        "a held lock must refuse a second owner with Ok(None)",
    )
}

/// C25 `lock-releases-on-drop`: dropping the guard frees the slot, so a fresh
/// acquisition at the same path returns `Ok(Some(_))`.
pub(crate) fn lock_releases_on_drop(cx: &CaseCx<'_>) -> CaseResult {
    let lock_path = cx.root.join(".probe.lock");
    let guard = match cx.fs.try_lock_store_dir(&lock_path) {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            return Err(CaseFailure::Violation(
                "initial lock acquisition must succeed, got Ok(None)".into(),
            ))
        }
        Err(error) => {
            return Err(CaseFailure::Violation(format!(
                "initial lock acquisition errored: {error}"
            )))
        }
    };
    drop(guard);
    check(
        matches!(cx.fs.try_lock_store_dir(&lock_path), Ok(Some(_))),
        "dropping the guard must release the lock",
    )
}

/// C26 `delete-then-recreate-starts-empty`: create+write+sync, unlink, then a
/// second `create_new_file` at the same path must succeed AND start empty — a
/// recreated path can never inherit the predecessor's bytes.
pub(crate) fn delete_then_recreate_starts_empty(cx: &CaseCx<'_>) -> CaseResult {
    let path = cx.root.join("recreated.probe");
    let mut file = cx
        .fs
        .create_new_file(&path)
        .map_err(|e| CaseFailure::Violation(format!("first create_new_file failed: {e}")))?;
    file.write_all(b"0123456789")
        .map_err(|e| CaseFailure::Violation(format!("write_all failed: {e}")))?;
    file.sync_all()
        .map_err(|e| CaseFailure::Violation(format!("sync_all failed: {e}")))?;
    drop(file);
    cx.fs
        .remove_file(&path)
        .map_err(|e| CaseFailure::Violation(format!("remove_file failed: {e}")))?;

    // Second incarnation at the reused path must be exclusive-fresh, not inherit.
    let recreated = cx
        .fs
        .create_new_file(&path)
        .map_err(|e| CaseFailure::Violation(format!("recreate at same path must succeed: {e}")))?;
    let len = recreated
        .len()
        .map_err(|e| CaseFailure::Violation(format!("len() on recreated handle failed: {e}")))?;
    check(
        len == 0,
        format!("a recreated path must start empty, saw {len} byte(s)"),
    )?;
    drop(recreated);
    let bytes = cx
        .fs
        .read(&path)
        .map_err(|e| CaseFailure::Violation(format!("reading recreated {path:?} failed: {e}")))?;
    check(
        bytes.is_empty(),
        format!(
            "a recreated path must never inherit predecessor bytes, saw {} byte(s)",
            bytes.len()
        ),
    )
}

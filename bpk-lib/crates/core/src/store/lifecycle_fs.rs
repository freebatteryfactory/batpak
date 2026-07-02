//! Shared filesystem helpers for lifecycle snapshot/fork/compact paths.

use crate::store::platform::fs as platform_fs;
use crate::store::StoreError;

pub(super) fn remove_file_if_present(path: &std::path::Path) -> Result<bool, StoreError> {
    platform_fs::remove_file_if_present(path).map_err(StoreError::Io)
}

pub(super) fn remove_dir_all_if_present(path: &std::path::Path) -> Result<bool, StoreError> {
    platform_fs::remove_dir_all_if_present(path).map_err(StoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::platform_fs;
    use super::remove_dir_all_if_present;

    #[test]
    fn remove_dir_all_if_present_reports_removal_then_absence_exactly() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let victim = dir.path().join("victim-tree");
        platform_fs::create_dir_all(&victim.join("nested")).expect("create nested tree");

        // A present tree: Ok(true) AND the tree is actually gone — a stubbed
        // `Ok(false)` body would skip the removal too, so both pins are needed.
        let removed = remove_dir_all_if_present(&victim).expect("remove existing tree");
        assert!(
            removed,
            "removing an existing directory tree must report Ok(true)"
        );
        assert!(
            !victim.exists(),
            "the tree must actually be gone after Ok(true)"
        );

        // Already absent: the tolerated NotFound, reported as Ok(false).
        let absent = remove_dir_all_if_present(&victim).expect("tolerate an absent tree");
        assert!(!absent, "an already-absent tree must report Ok(false)");
    }
}

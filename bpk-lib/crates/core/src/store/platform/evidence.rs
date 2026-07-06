use crate::store::platform::fs::{StoreFile, StoreFs};
use crate::store::stats::{
    ActiveSegmentReadEvidence, ClockEvidence, HostEvidenceSummary, LockLeafSymlinkProtection,
    MmapAdmissionSummary, MmapEvidence, ParentDirSyncAdmissionSummary, ParentDirSyncEvidence,
    PlatformAdmissionSummary, PlatformEvidenceSummary, StoreLockAdmissionSummary,
    StorePathEvidenceSummary, StorePathStatusEvidence,
};
use crate::store::Clock;
use memmap2::MmapOptions;
use std::path::Path;

pub(crate) fn collect_for_store_path(
    data_dir: &Path,
    clock: &dyn Clock,
    fs: &dyn StoreFs,
) -> PlatformEvidenceSummary {
    let mmap_evidence = mmap_evidence_for_store_path(data_dir, fs);
    let store_path = StorePathEvidenceSummary {
        path_status: path_status(data_dir),
        parent_dir_sync: parent_dir_sync_evidence(),
        lock_leaf_symlink_protection: lock_leaf_symlink_protection(),
        mmap_index: mmap_evidence,
        sealed_segment_mmap: mmap_evidence,
        active_segment_read: active_segment_read_evidence(),
    };
    let admission = admission_from_store_path(&store_path);
    PlatformEvidenceSummary {
        host: HostEvidenceSummary {
            process_clock_epoch_marker_ns: clock.process_boot_ns(),
            monotonic_clock: ClockEvidence::ProcessLocalInstantAnchor,
        },
        store_path,
        admission,
    }
}

pub(crate) fn path_status(data_dir: &Path) -> StorePathStatusEvidence {
    match super::fs::metadata(data_dir) {
        Ok(metadata) if metadata.is_dir() => StorePathStatusEvidence::ObservedDirectory,
        Ok(_) => StorePathStatusEvidence::ObservedUnsupportedNotDirectory,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            StorePathStatusEvidence::UnknownMissing
        }
        Err(error) => StorePathStatusEvidence::ProbeFailed {
            reason: error.to_string(),
        },
    }
}

pub(crate) fn parent_dir_sync_evidence() -> ParentDirSyncEvidence {
    #[cfg(unix)]
    {
        ParentDirSyncEvidence::UnixFsync
    }
    #[cfg(not(unix))]
    {
        ParentDirSyncEvidence::RenameOnly
    }
}

pub(crate) fn lock_leaf_symlink_protection() -> LockLeafSymlinkProtection {
    #[cfg(unix)]
    {
        LockLeafSymlinkProtection::AtomicNoFollow
    }
    #[cfg(not(unix))]
    {
        LockLeafSymlinkProtection::BestEffortCheckThenOpen
    }
}

pub(crate) fn active_segment_read_evidence() -> ActiveSegmentReadEvidence {
    #[cfg(unix)]
    {
        ActiveSegmentReadEvidence::UnixReadAt
    }
    #[cfg(not(unix))]
    {
        ActiveSegmentReadEvidence::LockedSeekRead
    }
}

/// Leaf name of the private one-byte mmap-capability probe file.
const MMAP_PROBE_LEAF: &str = ".batpak-mmap-probe";

pub(crate) fn mmap_evidence_for_store_path(data_dir: &Path, fs: &dyn StoreFs) -> MmapEvidence {
    match path_status(data_dir) {
        StorePathStatusEvidence::ObservedDirectory => {}
        StorePathStatusEvidence::ObservedUnsupportedNotDirectory => {
            return MmapEvidence::ObservedUnsupported;
        }
        StorePathStatusEvidence::UnknownMissing => return MmapEvidence::Unknown,
        StorePathStatusEvidence::ProbeFailed { .. } => return MmapEvidence::ProbeFailed,
    }

    // Probe THROUGH the configured backend so a purely in-memory store never
    // touches the host filesystem (issue #171): a MemFs probe stays in memory
    // and its `as_std_file()` is `None`, so mmap is observed-unsupported and NO
    // host tempfile is ever created; RealFs mints a real file to actually map.
    let probe_path = data_dir.join(MMAP_PROBE_LEAF);
    let _ = fs.remove_file_if_present(&probe_path);
    let Ok(mut probe) = fs.create_new_file(&probe_path) else {
        return MmapEvidence::ProbeFailed;
    };
    let evidence = probe_mmap_admission(&mut *probe);
    let _ = fs.remove_file_if_present(&probe_path);
    evidence
}

fn probe_mmap_admission(probe: &mut dyn StoreFile) -> MmapEvidence {
    if probe.write_all(&[0]).is_err() {
        return MmapEvidence::ProbeFailed;
    }
    // A virtual backend's handle exposes no OS file, so mmap is definitively
    // unsupported for it — reported without touching the host.
    let Some(file) = probe.as_std_file() else {
        return MmapEvidence::ObservedUnsupported;
    };
    // SAFETY: the probe file is private to this function, one byte long, and
    // dropped immediately after the map attempt. This observes the target mmap
    // mechanism; it does not establish store semantics.
    match unsafe { MmapOptions::new().len(1).map(file) } {
        Ok(_map) => MmapEvidence::FileBacked,
        Err(error) => mmap_map_error_evidence(&error),
    }
}

fn mmap_map_error_evidence(error: &std::io::Error) -> MmapEvidence {
    if matches!(
        error.kind(),
        std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied
    ) {
        return MmapEvidence::ObservedUnsupported;
    }
    MmapEvidence::ProbeFailed
}

pub(crate) fn admission_from_store_path(
    store_path: &StorePathEvidenceSummary,
) -> PlatformAdmissionSummary {
    PlatformAdmissionSummary {
        store_lock: match store_path.lock_leaf_symlink_protection {
            LockLeafSymlinkProtection::AtomicNoFollow => StoreLockAdmissionSummary::AtomicNoFollow,
            LockLeafSymlinkProtection::BestEffortCheckThenOpen => {
                StoreLockAdmissionSummary::BestEffortCheckThenOpen
            }
            LockLeafSymlinkProtection::Unknown
            | LockLeafSymlinkProtection::ObservedUnsupported
            | LockLeafSymlinkProtection::ProbeFailed => StoreLockAdmissionSummary::Rejected,
        },
        parent_dir_sync: match store_path.parent_dir_sync {
            ParentDirSyncEvidence::UnixFsync => ParentDirSyncAdmissionSummary::UnixFsync,
            ParentDirSyncEvidence::RenameOnly => ParentDirSyncAdmissionSummary::RenameOnly,
            ParentDirSyncEvidence::Unknown | ParentDirSyncEvidence::ProbeFailed => {
                ParentDirSyncAdmissionSummary::Rejected
            }
        },
        mmap_index: mmap_admission(store_path.mmap_index),
        sealed_segment_mmap: mmap_admission(store_path.sealed_segment_mmap),
    }
}

fn mmap_admission(evidence: MmapEvidence) -> MmapAdmissionSummary {
    match evidence {
        MmapEvidence::FileBacked => MmapAdmissionSummary::FileBacked,
        MmapEvidence::Unknown | MmapEvidence::ObservedUnsupported | MmapEvidence::ProbeFailed => {
            MmapAdmissionSummary::Rejected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::stats::{
        ActiveSegmentReadEvidence, LockLeafSymlinkProtection, ParentDirSyncEvidence,
        StorePathEvidenceSummary,
    };
    use std::error::Error;

    #[test]
    fn path_status_distinguishes_missing_paths_from_probe_failures() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let missing = dir.path().join("missing-store-path");

        assert_eq!(
            path_status(&missing),
            StorePathStatusEvidence::UnknownMissing,
            "PROPERTY: a missing store path is unknown/missing evidence, not a probe failure"
        );
        Ok(())
    }

    #[test]
    fn path_status_distinguishes_regular_files_from_directories() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let file_path = dir.path().join("store-file");
        std::fs::write(&file_path, b"not a directory")?;

        assert_eq!(
            path_status(&file_path),
            StorePathStatusEvidence::ObservedUnsupportedNotDirectory,
            "PROPERTY: an existing non-directory path must be unsupported evidence, not accepted as a store directory"
        );
        Ok(())
    }

    /// Traversing THROUGH a regular file makes `metadata` fail with a
    /// non-NotFound error (ENOTDIR / `NotADirectory`) — a real probe failure,
    /// NOT absence. The `guard -> true` mutant classifies EVERY metadata error
    /// as `UnknownMissing`, reporting an unreadable store path as merely
    /// absent and discarding the error text the evidence exists to carry.
    #[cfg(unix)]
    #[test]
    fn path_status_reports_non_not_found_probe_errors_as_probe_failed() -> Result<(), Box<dyn Error>>
    {
        let dir = tempfile::tempdir()?;
        let squatter = dir.path().join("squatting-file");
        std::fs::write(&squatter, b"a file squatting on a directory component")?;

        let probed = path_status(&squatter.join("child"));
        assert!(
            matches!(
                &probed,
                StorePathStatusEvidence::ProbeFailed { reason } if !reason.is_empty()
            ),
            "PROPERTY: a non-NotFound metadata error must be ProbeFailed evidence \
             carrying the error text, never UnknownMissing; got {probed:?}"
        );

        // Contrast anchor: genuinely-missing stays UnknownMissing — NotFound is
        // the ONLY error kind the guard may classify as absence.
        let missing = dir.path().join("missing-store-path");
        assert_eq!(
            path_status(&missing),
            StorePathStatusEvidence::UnknownMissing,
            "PROPERTY: NotFound remains the one error classified as absence"
        );
        Ok(())
    }

    #[test]
    fn mmap_evidence_keeps_missing_paths_unknown() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let missing = dir.path().join("missing-store-path");

        assert_eq!(
            mmap_evidence_for_store_path(&missing, &crate::store::platform::fs::RealFs),
            MmapEvidence::Unknown,
            "PROPERTY: mmap evidence for a missing path must stay Unknown, not ProbeFailed"
        );
        Ok(())
    }

    #[test]
    fn mmap_evidence_rejects_non_directory_store_paths() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let file_path = dir.path().join("store-file");
        std::fs::write(&file_path, b"not a directory")?;

        assert_eq!(
            mmap_evidence_for_store_path(&file_path, &crate::store::platform::fs::RealFs),
            MmapEvidence::ObservedUnsupported,
            "PROPERTY: mmap probing must not create temp probes under a non-directory store path"
        );
        Ok(())
    }

    #[test]
    fn admission_from_store_path_maps_each_evidence_family_independently() {
        let store_path = StorePathEvidenceSummary {
            path_status: StorePathStatusEvidence::ObservedDirectory,
            parent_dir_sync: ParentDirSyncEvidence::ProbeFailed,
            lock_leaf_symlink_protection: LockLeafSymlinkProtection::ObservedUnsupported,
            mmap_index: MmapEvidence::FileBacked,
            sealed_segment_mmap: MmapEvidence::Unknown,
            active_segment_read: ActiveSegmentReadEvidence::ProbeFailed,
        };

        let admission = admission_from_store_path(&store_path);

        assert_eq!(
            admission.store_lock,
            StoreLockAdmissionSummary::Rejected,
            "PROPERTY: unsupported lock evidence must fail only the lock admission"
        );
        assert_eq!(
            admission.parent_dir_sync,
            ParentDirSyncAdmissionSummary::Rejected,
            "PROPERTY: failed parent-dir sync evidence must fail only the sync admission"
        );
        assert_eq!(
            admission.mmap_index,
            MmapAdmissionSummary::FileBacked,
            "PROPERTY: file-backed mmap evidence remains admitted even when other families reject"
        );
        assert_eq!(
            admission.sealed_segment_mmap,
            MmapAdmissionSummary::Rejected,
            "PROPERTY: unknown sealed-segment mmap evidence must be rejected"
        );
    }

    #[test]
    fn mmap_map_error_classifies_unsupported_and_permission_as_observed_unsupported() {
        assert_eq!(
            mmap_map_error_evidence(&std::io::Error::from(std::io::ErrorKind::Unsupported)),
            MmapEvidence::ObservedUnsupported,
            "PROPERTY: unsupported mmap must be descriptive evidence, not a probe failure"
        );
        assert_eq!(
            mmap_map_error_evidence(&std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            MmapEvidence::ObservedUnsupported,
            "PROPERTY: permission-denied mmap must be descriptive evidence, not a probe failure"
        );
        assert_eq!(
            mmap_map_error_evidence(&std::io::Error::from(std::io::ErrorKind::Other)),
            MmapEvidence::ProbeFailed,
            "PROPERTY: unrelated mmap errors must remain probe failures"
        );
    }
}

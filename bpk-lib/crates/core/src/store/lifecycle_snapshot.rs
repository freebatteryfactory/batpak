//! Point-in-time store snapshot (deep copy of share-safe substrate files).

use super::sync;
use crate::store::cold_start::latest_segment_watermark;
use crate::store::file_classification::StoreFileKind;
use crate::store::snapshot_report::{
    destination_path_digest, snapshot_evidence_report, SnapshotEvidenceReport, SnapshotFileKind,
    SnapshotFinding, SnapshotOptions, SnapshotReportInput,
};
use crate::store::{Open, Store, StoreError};

/// Accumulates which share-safe substrate files a snapshot copied, folded across
/// the directory walk so the walk body stays flat (mirrors fork's accumulator and
/// keeps `snapshot` within the function-complexity budget).
#[derive(Default)]
struct SnapshotCopyAcc {
    copied_segment_ids_sorted: Vec<u64>,
    copied_visibility_ranges_present: bool,
    copied_pending_compaction_marker_present: bool,
    copied_idempotency_store_present: bool,
}

impl SnapshotCopyAcc {
    fn record(&mut self, file_kind: SnapshotFileKind, source_kind: &StoreFileKind) {
        match file_kind {
            SnapshotFileKind::Segment => {
                if let Some(segment_id) = source_kind.segment_id() {
                    self.copied_segment_ids_sorted.push(segment_id.as_u64());
                }
            }
            SnapshotFileKind::VisibilityRanges => self.copied_visibility_ranges_present = true,
            SnapshotFileKind::PendingCompactionMarker => {
                self.copied_pending_compaction_marker_present = true;
            }
            SnapshotFileKind::IdempotencyStore => self.copied_idempotency_store_present = true,
        }
    }
}

pub(crate) fn snapshot(
    store: &Store<Open>,
    dest: &std::path::Path,
    options: SnapshotOptions,
) -> Result<SnapshotEvidenceReport, StoreError> {
    tracing::debug!(
        target: "batpak::flow",
        flow = "snapshot",
        destination = %dest.display()
    );
    // Keyset portability gate (D24): fail closed on an encryption-active store
    // unless the caller opted into a keys-excluded copy. Errs before any
    // destination mutation. Always false without the payload-encryption feature.
    let keys_excluded = super::resolve_keyset_exclusion(store, options.keyset_policy, "snapshot")?;
    let fs = store.config.fs();
    let _lifecycle = store.lifecycle_gate.lock();
    let snapshot_fence = store.begin_visibility_fence()?;
    let fence_token = snapshot_fence.token();
    sync(store)?;
    store
        .index
        .idemp
        .flush(&store.config.data_dir, fs.as_ref())?;
    let (source_watermark_segment_id, source_watermark_offset) =
        latest_segment_watermark(&store.config.data_dir, fs.as_ref())?;
    fs.reject_symlink_leaf(dest, "snapshot destination")?;
    fs.create_dir_all(dest).map_err(StoreError::Io)?;
    let cleared_artifact_count = clear_snapshot_store_artifacts(fs.as_ref(), dest)?;
    let entries = fs
        .read_dir(&store.config.data_dir)
        .map_err(StoreError::Io)?;
    let mut acc = SnapshotCopyAcc::default();
    let mut findings = Vec::new();
    if cleared_artifact_count > 0 {
        findings.push(SnapshotFinding::DestinationCleared {
            artifact_count: cleared_artifact_count,
        });
    }
    for entry in entries {
        let path = store.config.data_dir.join(&entry.name);
        let source_kind = StoreFileKind::from_path(&path);
        if let Some(file_kind) = snapshot_source_file_kind(&source_kind) {
            let dest_path = dest.join(&entry.name);
            fs.reject_symlink_leaf(&dest_path, "snapshot entry")?;
            fs.copy(&path, &dest_path).map_err(StoreError::Io)?;
            acc.record(file_kind, &source_kind);
        }
    }
    snapshot_fence.cancel()?;
    findings.push(SnapshotFinding::FenceTokenCancelled);
    findings.push(SnapshotFinding::CopyByteHashUnavailable {
        reason:
            "snapshot v1 records structural file identity; per-file byte hash table is out of scope"
                .to_string(),
        file_kind: SnapshotFileKind::Segment,
    });
    // D24: record that an encryption-active store's keyset was deliberately
    // excluded (branch-free push so this stays within the complexity ratchet).
    findings.extend(keys_excluded.then_some(SnapshotFinding::KeysExcluded));
    Ok(snapshot_evidence_report(SnapshotReportInput {
        fence_token,
        source_watermark_segment_id,
        source_watermark_offset,
        copied_segment_ids_sorted: acc.copied_segment_ids_sorted,
        copied_visibility_ranges_present: acc.copied_visibility_ranges_present,
        copied_pending_compaction_marker_present: acc.copied_pending_compaction_marker_present,
        copied_idempotency_store_present: acc.copied_idempotency_store_present,
        destination_path_digest: destination_path_digest(dest),
        findings,
    })?)
}

fn snapshot_source_file_kind(file_kind: &StoreFileKind) -> Option<SnapshotFileKind> {
    if !file_kind.should_copy_into_snapshot() {
        return None;
    }
    match file_kind {
        StoreFileKind::Segment(_) => Some(SnapshotFileKind::Segment),
        StoreFileKind::VisibilityRanges => Some(SnapshotFileKind::VisibilityRanges),
        StoreFileKind::IdempotencyStore => Some(SnapshotFileKind::IdempotencyStore),
        StoreFileKind::PendingCompactionMarker => Some(SnapshotFileKind::PendingCompactionMarker),
        // Keyset is excluded from `should_copy_into_snapshot` in Stage B, so the
        // guard above already returned `None` for it; it stays in the None group.
        StoreFileKind::MalformedSegment(_)
        | StoreFileKind::Checkpoint
        | StoreFileKind::MmapIndex
        | StoreFileKind::CompactSource
        | StoreFileKind::CursorDirectory
        | StoreFileKind::Keyset
        | StoreFileKind::Other => None,
    }
}

pub(super) fn snapshot_destination_should_clear(path: &std::path::Path) -> bool {
    StoreFileKind::from_path(path).should_clear_from_snapshot_destination()
}

pub(super) fn clear_snapshot_store_artifacts(
    fs: &dyn crate::store::platform::fs::StoreFs,
    dest: &std::path::Path,
) -> Result<usize, StoreError> {
    use super::lifecycle_fs::{remove_dir_all_if_present, remove_file_if_present};

    let entries = fs.read_dir(dest).map_err(StoreError::Io)?;
    let mut removed = 0;
    for entry in entries {
        let path = dest.join(&entry.name);
        if snapshot_destination_should_clear(&path) {
            removed += usize::from(remove_file_if_present(&path)?);
            continue;
        }

        if entry.kind == crate::store::platform::fs::FileKind::Dir
            && StoreFileKind::from_path(&path) == StoreFileKind::CursorDirectory
        {
            removed += usize::from(remove_dir_all_if_present(&path)?);
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::clear_snapshot_store_artifacts;
    use crate::store::file_classification::CURSOR_DIRECTORY;
    use crate::store::platform::fs::{write_file_atomically, RealFs};
    use crate::store::StoreError;

    #[test]
    fn clear_removes_cursor_state_only_when_both_directory_and_name_match() {
        // The cursor-state clear is guarded by a CONJUNCTION: the entry must
        // be a directory AND classify as the cursor directory. Each test entry
        // below satisfies exactly ONE conjunct, so neither may be touched; the
        // `&&` -> `||` mutant removes the foreign directory (inflating the
        // removed count) or errors trying to remove-dir-all a regular file —
        // either iteration order convicts it.
        let dest = tempfile::TempDir::new().expect("create snapshot destination");
        let write_fixture = |name: &str, bytes: &'static [u8]| {
            write_file_atomically(dest.path(), &dest.path().join(name), name, |file| {
                file.write_all(bytes).map_err(StoreError::Io)
            })
            .expect("write fixture through the platform seam");
        };
        write_fixture("000001.fbat", b"seg");
        crate::store::platform::fs::create_dir_all(&dest.path().join("caller-owned"))
            .expect("create foreign directory");
        write_fixture(CURSOR_DIRECTORY, b"not a dir");

        let removed = clear_snapshot_store_artifacts(&RealFs, dest.path())
            .expect("clearing touches only store artifacts and cannot error here");

        assert_eq!(removed, 1, "exactly the segment artifact is cleared");
        assert!(
            !dest.path().join("000001.fbat").exists(),
            "the segment artifact is gone"
        );
        assert!(
            dest.path().join("caller-owned").is_dir(),
            "a foreign directory survives: being a directory alone must not qualify"
        );
        assert!(
            dest.path().join(CURSOR_DIRECTORY).is_file(),
            "a cursor-NAMED regular file survives: the name alone must not qualify"
        );
    }

    #[test]
    fn snapshot_source_file_kind_maps_copyable_kinds_and_rejects_the_rest() {
        use super::snapshot_source_file_kind;
        use crate::store::file_classification::StoreFileKind;
        use crate::store::segment::SegmentId;
        use crate::store::snapshot_report::SnapshotFileKind;

        // Each share-safe source kind maps to its exact SnapshotFileKind; kills
        // every match-arm swap (a segment recorded as visibility-ranges, etc.).
        let segment = StoreFileKind::Segment(SegmentId::from_stem("4").expect("segment id"));
        assert_eq!(
            snapshot_source_file_kind(&segment),
            Some(SnapshotFileKind::Segment)
        );
        assert_eq!(
            snapshot_source_file_kind(&StoreFileKind::VisibilityRanges),
            Some(SnapshotFileKind::VisibilityRanges)
        );
        assert_eq!(
            snapshot_source_file_kind(&StoreFileKind::IdempotencyStore),
            Some(SnapshotFileKind::IdempotencyStore)
        );
        assert_eq!(
            snapshot_source_file_kind(&StoreFileKind::PendingCompactionMarker),
            Some(SnapshotFileKind::PendingCompactionMarker)
        );

        // The `!should_copy_into_snapshot()` guard NEGATED would return `None` for
        // the copyable kinds above (convicted there); here a non-copyable kind
        // must map to `None` through the classifier.
        assert_eq!(snapshot_source_file_kind(&StoreFileKind::Checkpoint), None);
        assert_eq!(snapshot_source_file_kind(&StoreFileKind::Keyset), None);
        assert_eq!(snapshot_source_file_kind(&StoreFileKind::Other), None);
    }
}

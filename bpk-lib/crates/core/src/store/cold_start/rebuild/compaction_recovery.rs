//! Pending-compaction recovery — the namespace-transaction decision + physical
//! repair (#177/#195).
//!
//! The recovery law: every reopen of a directory carrying a pending-compaction
//! marker produces EXACTLY one of
//! - the OLD complete generation (roll back), or
//! - the NEW complete generation (roll forward), or
//! - a typed canonical [`StoreError::CompactionRecoveryRefused`] (fail closed).
//!
//! Direction is decided by the THREE-ARTIFACT agreement (contract A8), never by
//! file existence: the marker's `compaction_id` must equal the
//! `store.meta` `last_compaction_commit.compaction_id` (same branded token) AND
//! the marker's `merged_id` must equal the commit's `merged_segment_id`.
//! Match ⇒ the transaction committed durably ⇒ roll forward; a non-matching or
//! absent commit ⇒ the transaction never committed ⇒ roll back. The third
//! artifact — the on-disk authority IMAGE — is verified against `store.meta`'s
//! expectation by the existing typed idempotency-authority load in
//! [`super::open_index`] (which fails closed on any image/meta disagreement),
//! so this module decides marker↔meta agreement and never re-reads the image.
//!
//! A legacy (pre-token) marker never flows through this state machine: it is a
//! DIAGNOSTIC parse only and refuses with
//! [`CompactionRecoveryRefusal::LegacyMarkerUnsupported`] (contract A9) — offline
//! migration or a typed refusal are the only exits for old bytes, never an
//! automatic v1 recovery.
//!
//! Repair is PHYSICAL and runs at WRITABLE open under the store-dir lock (see
//! [`super::open_index`] / `store/open.rs`). A read-only open with any pending
//! marker refuses ([`CompactionRecoveryRefusal::RepairRequiresWritableOpen`]);
//! the retired path-substitution "recovery" produced an index whose `DiskPos`
//! entries resolve ids to the FINAL segment name and could not be read back, so
//! it was never a working mode.

use super::topology::{
    clear_pending_compaction, compaction_source_temp_path, compaction_staged_path,
    load_pending_compaction, LoadedMarker, COMPACTION_MARKER_FILENAME,
};
use crate::store::error::CompactionRecoveryRefusal;
use crate::store::platform::fs::StoreFs;
use crate::store::segment::segment_filename;
use crate::store::store_meta::StoreMetaData;
use crate::store::StoreError;
use std::path::{Path, PathBuf};

/// The outcome of resolving a pending-compaction transaction at open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactionRecoveryAction {
    /// No pending marker — an ordinary open.
    None,
    /// The transaction had not committed; the OLD generation was restored.
    RolledBack,
    /// The transaction had committed; the NEW generation was completed.
    RolledForward,
}

/// A classified pending-compaction transaction: exactly the physical work the
/// repair step must perform, or the marker was absent.
pub(crate) enum CompactionResolution {
    NoTransaction,
    RollBack(RollBackPlan),
    RollForward(RollForwardPlan),
}

/// Roll-back plan (uncommitted transaction → restore the OLD generation).
pub(crate) struct RollBackPlan {
    pub(crate) marker_path: PathBuf,
    /// A leftover staged replacement to delete, if the `.compact-new` file
    /// survived.
    pub(crate) staged_leftover: Option<PathBuf>,
    /// `(from `.compact-src`, to final segment name)` — `None` when the merged-id
    /// source still sits at its original (final) name because the crash preceded
    /// the rename-away.
    pub(crate) restore_source: Option<(PathBuf, PathBuf)>,
}

/// Roll-forward plan (committed transaction → complete the NEW generation).
pub(crate) struct RollForwardPlan {
    pub(crate) marker_path: PathBuf,
    /// `(staged, final)` redo rename — `None` when the final name is already
    /// durable.
    pub(crate) rename_staged: Option<(PathBuf, PathBuf)>,
    /// Remaining source files to retire (original-name sources still present +
    /// the merged-id `.compact-src` + any stale `.compact-new`). Only paths that
    /// still exist are listed; retirement is idempotent (repair may itself crash
    /// and rerun).
    pub(crate) retire: Vec<PathBuf>,
}

/// Resolve any pending-compaction transaction for `data_dir`.
///
/// - No marker → [`CompactionRecoveryAction::None`].
/// - Marker present on a read-only open → typed
///   [`CompactionRecoveryRefusal::RepairRequiresWritableOpen`].
/// - Otherwise classify (marker↔`store.meta` agreement) then physically repair.
pub(crate) fn resolve_pending_compaction(
    data_dir: &Path,
    fs: &dyn StoreFs,
    store_meta: Option<&StoreMetaData>,
    writable: bool,
) -> Result<CompactionRecoveryAction, StoreError> {
    if load_pending_compaction(data_dir, fs)?.is_none() {
        return Ok(CompactionRecoveryAction::None);
    }
    if !writable {
        return Err(refused(
            data_dir,
            CompactionRecoveryRefusal::RepairRequiresWritableOpen,
        ));
    }
    let resolution = classify_pending_compaction(data_dir, fs, store_meta)?;
    repair_pending_compaction(data_dir, fs, resolution)
}

/// Classify the pending transaction into the exact repair plan or a typed
/// refusal. File facts (`final`/`.compact-src`/`.compact-new` existence, source
/// presence) shape the plan but NEVER decide the roll-back/roll-forward
/// direction — the `store.meta` commit record does (contract A8).
fn classify_pending_compaction(
    data_dir: &Path,
    fs: &dyn StoreFs,
    store_meta: Option<&StoreMetaData>,
) -> Result<CompactionResolution, StoreError> {
    let marker = match load_pending_compaction(data_dir, fs)? {
        Option::None => return Ok(CompactionResolution::NoTransaction),
        Some(LoadedMarker::Legacy(v1)) => {
            return Err(refused(
                data_dir,
                CompactionRecoveryRefusal::LegacyMarkerUnsupported {
                    path: data_dir.join(COMPACTION_MARKER_FILENAME),
                    detail: format!(
                        "v1 marker merged_id={} source_segment_ids={:?}",
                        v1.merged_id, v1.source_segment_ids
                    ),
                },
            ));
        }
        Some(LoadedMarker::V2(marker)) => marker,
    };

    // A v2 marker is only ever written by a binary that mints `store.meta`; its
    // absence here is undecidable, never a silent roll-forward.
    let meta = match store_meta {
        Some(meta) => meta,
        Option::None => {
            return Err(refused(
                data_dir,
                CompactionRecoveryRefusal::MetadataUnavailable,
            ))
        }
    };

    // A transplanted marker (wrong lineage) is a hard refusal — repairing another
    // store's transaction into this namespace would corrupt history.
    if marker.lineage != meta.lineage {
        return Err(refused(
            data_dir,
            CompactionRecoveryRefusal::MarkerForeignLineage {
                marker_lineage: marker.lineage,
                store_lineage: meta.lineage,
            },
        ));
    }

    let layout = MarkerLayout::probe(data_dir, fs, marker.merged_id);

    // The ONE durable fact that flips direction: does `store.meta` hold a commit
    // record bound to THIS transaction's token and merged id?
    let committed = matches!(
        &meta.last_compaction_commit,
        Some(commit)
            if commit.compaction_id == marker.compaction_id
                && commit.merged_segment_id == marker.merged_id
    );

    if committed {
        classify_roll_forward(data_dir, fs, &marker.source_segment_ids, layout)
    } else {
        classify_roll_back(data_dir, fs, &marker.source_segment_ids, layout)
    }
}

/// The resolved namespace paths for a marker plus their probed existence — the
/// file facts that SHAPE a repair plan (never decide its direction).
struct MarkerLayout {
    merged_id: u64,
    marker_path: PathBuf,
    final_path: PathBuf,
    final_exists: bool,
    src_path: PathBuf,
    src_exists: bool,
    staged_path: PathBuf,
    staged_exists: bool,
}

impl MarkerLayout {
    fn probe(data_dir: &Path, fs: &dyn StoreFs, merged_id: u64) -> Self {
        let final_path = data_dir.join(segment_filename(merged_id));
        let src_path = compaction_source_temp_path(data_dir, merged_id);
        let staged_path = compaction_staged_path(data_dir, merged_id);
        Self {
            merged_id,
            marker_path: data_dir.join(COMPACTION_MARKER_FILENAME),
            final_exists: fs.metadata(&final_path).is_ok(),
            src_exists: fs.metadata(&src_path).is_ok(),
            staged_exists: fs.metadata(&staged_path).is_ok(),
            final_path,
            src_path,
            staged_path,
        }
    }
}

/// Committed transaction: complete the NEW generation. The replacement must be
/// recoverable under the final or the staged name; otherwise the commit record
/// and the namespace disagree — a typed [`CompactionRecoveryRefusal::ReplacementMissing`].
fn classify_roll_forward(
    data_dir: &Path,
    fs: &dyn StoreFs,
    source_segment_ids: &[u64],
    layout: MarkerLayout,
) -> Result<CompactionResolution, StoreError> {
    let mut retire =
        existing_non_merged_sources(data_dir, fs, source_segment_ids, layout.merged_id);
    if layout.src_exists {
        retire.push(layout.src_path);
    }
    if layout.final_exists {
        // Final durable (rows 6–8, and row 5′ already renamed): retire the
        // sources and any stale staged leftover.
        if layout.staged_exists {
            retire.push(layout.staged_path);
        }
        Ok(CompactionResolution::RollForward(RollForwardPlan {
            marker_path: layout.marker_path,
            rename_staged: None,
            retire,
        }))
    } else if layout.staged_exists {
        // Committed but the staged→final rename is not durable (row 5′): redo it.
        Ok(CompactionResolution::RollForward(RollForwardPlan {
            marker_path: layout.marker_path,
            rename_staged: Some((layout.staged_path, layout.final_path)),
            retire,
        }))
    } else {
        Err(refused(
            data_dir,
            CompactionRecoveryRefusal::ReplacementMissing {
                merged_id: layout.merged_id,
            },
        ))
    }
}

/// Uncommitted transaction: restore the OLD generation. Every marker-named
/// source must be recoverable at its original name (or, for the merged id, under
/// `.compact-src`); a source recoverable under no name, or an impossible
/// both-names state, is a typed refusal.
fn classify_roll_back(
    data_dir: &Path,
    fs: &dyn StoreFs,
    source_segment_ids: &[u64],
    layout: MarkerLayout,
) -> Result<CompactionResolution, StoreError> {
    // Production never creates the final name pre-commit; both names present is a
    // namespace state the protocol cannot produce.
    if layout.src_exists && layout.final_exists {
        return Err(refused(
            data_dir,
            CompactionRecoveryRefusal::ConflictingNames {
                merged_id: layout.merged_id,
            },
        ));
    }
    let restore_source = if layout.src_exists {
        Some((layout.src_path, layout.final_path))
    } else if layout.final_exists {
        // Crash before the rename-away: the file at the final name IS the
        // original merged-id source.
        Option::None
    } else {
        return Err(refused(
            data_dir,
            CompactionRecoveryRefusal::SourceMissing {
                segment_id: layout.merged_id,
            },
        ));
    };
    for source_id in source_segment_ids
        .iter()
        .copied()
        .filter(|id| *id != layout.merged_id)
    {
        let path = data_dir.join(segment_filename(source_id));
        if fs.metadata(&path).is_err() {
            return Err(refused(
                data_dir,
                CompactionRecoveryRefusal::SourceMissing {
                    segment_id: source_id,
                },
            ));
        }
    }
    let staged_leftover = layout.staged_exists.then_some(layout.staged_path);
    Ok(CompactionResolution::RollBack(RollBackPlan {
        marker_path: layout.marker_path,
        staged_leftover,
        restore_source,
    }))
}

/// Execute the classified plan against the filesystem seam, then clear the
/// marker LAST. Each namespace batch is parent-dir-synced before the marker is
/// removed so that "marker absent" implies "the batch is durable".
fn repair_pending_compaction(
    data_dir: &Path,
    fs: &dyn StoreFs,
    resolution: CompactionResolution,
) -> Result<CompactionRecoveryAction, StoreError> {
    match resolution {
        CompactionResolution::NoTransaction => Ok(CompactionRecoveryAction::None),
        CompactionResolution::RollBack(plan) => {
            if let Some(leftover) = plan.staged_leftover {
                fs.remove_file_if_present(&leftover).map_err(StoreError::Io)?;
            }
            if let Some((from, to)) = plan.restore_source {
                fs.rename(&from, &to).map_err(StoreError::Io)?;
            }
            fs.sync_parent_dir(&plan.marker_path)?;
            // Removes the marker + parent-syncs — the LAST durable act.
            clear_pending_compaction(data_dir, fs)?;
            Ok(CompactionRecoveryAction::RolledBack)
        }
        CompactionResolution::RollForward(plan) => {
            if let Some((from, to)) = plan.rename_staged {
                fs.rename(&from, &to).map_err(StoreError::Io)?;
                fs.sync_parent_dir(&to)?;
            }
            for path in &plan.retire {
                fs.remove_file_if_present(path).map_err(StoreError::Io)?;
            }
            fs.sync_parent_dir(&plan.marker_path)?;
            clear_pending_compaction(data_dir, fs)?;
            Ok(CompactionRecoveryAction::RolledForward)
        }
    }
}

/// Non-merged source paths that still exist, for the roll-forward retire list.
fn existing_non_merged_sources(
    data_dir: &Path,
    fs: &dyn StoreFs,
    source_segment_ids: &[u64],
    merged_id: u64,
) -> Vec<PathBuf> {
    source_segment_ids
        .iter()
        .copied()
        .filter(|id| *id != merged_id)
        .map(|id| data_dir.join(segment_filename(id)))
        .filter(|path| fs.metadata(path).is_ok())
        .collect()
}

/// Build the typed open-refusal at the canonical marker path.
fn refused(data_dir: &Path, kind: CompactionRecoveryRefusal) -> StoreError {
    StoreError::CompactionRecoveryRefused {
        marker_path: data_dir.join(COMPACTION_MARKER_FILENAME),
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::super::topology::PendingCompactionV2;
    use super::*;
    use crate::store::generation_ids::{AuthorityImageId, CompactionId};
    use crate::store::platform::fs::{create_new_file, RealFs};
    use crate::store::segment::segment_filename;
    use crate::store::store_meta::{CompactionCommit, StoreMetaData};
    use crate::store::SystemClock;

    fn mint_compaction_id() -> CompactionId {
        CompactionId::generate(&SystemClock::new())
    }

    fn mint_authority_image_id() -> AuthorityImageId {
        AuthorityImageId::generate(&SystemClock::new())
    }

    fn meta_with(lineage: u128, commit: Option<CompactionCommit>) -> StoreMetaData {
        StoreMetaData {
            lineage,
            idemp_authority: None,
            last_compaction_commit: commit,
        }
    }

    fn write_v2_marker(
        data_dir: &Path,
        compaction_id: CompactionId,
        lineage: u128,
        merged_id: u64,
        source_segment_ids: &[u64],
    ) {
        super::super::topology::write_pending_compaction(
            data_dir,
            &PendingCompactionV2 {
                compaction_id,
                lineage,
                merged_id,
                source_segment_ids: source_segment_ids.to_vec(),
            },
            &RealFs,
        )
        .expect("write v2 pending-compaction marker");
    }

    fn touch(path: &Path) {
        drop(create_new_file(path).expect("create fixture file"));
    }

    fn refusal(result: Result<CompactionRecoveryAction, StoreError>) -> CompactionRecoveryRefusal {
        // `if let` (not a `match` with a named catch-all arm) so no
        // `clippy::wildcard_enum_match_arm` over `StoreError`.
        if let Err(StoreError::CompactionRecoveryRefused { kind, .. }) = result {
            return kind;
        }
        fail_property(&format!(
            "PROPERTY: expected CompactionRecoveryRefused, got {result:?}"
        ))
    }

    fn fail_property(message: &str) -> ! {
        assert!(std::hint::black_box(false), "{message}");
        unreachable!("assert above always fires")
    }

    #[test]
    fn resolve_returns_none_when_no_marker_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let action = resolve_pending_compaction(dir.path(), &RealFs, None, true)
            .expect("no marker resolves cleanly");
        assert_eq!(action, CompactionRecoveryAction::None);
    }

    #[test]
    fn read_only_open_with_a_marker_refuses() {
        // A pending marker cannot be repaired without write access; the open must
        // refuse rather than limp on an unreadable substituted view.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        write_v2_marker(dir.path(), cid, 1, 7, &[1, 2, 7]);
        let meta = meta_with(1, None);
        let kind = refusal(resolve_pending_compaction(
            dir.path(),
            &RealFs,
            Some(&meta),
            false,
        ));
        assert!(
            matches!(kind, CompactionRecoveryRefusal::RepairRequiresWritableOpen),
            "PROPERTY: read-only open with a pending marker must refuse for repair; got {kind:?}"
        );
    }

    #[test]
    fn v2_marker_without_store_meta_is_undecidable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        write_v2_marker(dir.path(), cid, 9, 7, &[1, 7]);
        let kind = refusal(resolve_pending_compaction(dir.path(), &RealFs, None, true));
        assert!(
            matches!(kind, CompactionRecoveryRefusal::MetadataUnavailable),
            "PROPERTY: a v2 marker with no store.meta is undecidable; got {kind:?}"
        );
    }

    #[test]
    fn foreign_lineage_marker_refuses() {
        // A marker whose lineage disagrees with store.meta was transplanted;
        // repairing it would splice another store's transaction into ours.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        write_v2_marker(dir.path(), cid, 0x11, 7, &[1, 7]);
        let meta = meta_with(0x22, None);
        let kind = refusal(resolve_pending_compaction(
            dir.path(),
            &RealFs,
            Some(&meta),
            true,
        ));
        assert!(
            matches!(
                kind,
                CompactionRecoveryRefusal::MarkerForeignLineage {
                    marker_lineage: 0x11,
                    store_lineage: 0x22,
                }
            ),
            "PROPERTY: a foreign-lineage marker refuses with both lineages; got {kind:?}"
        );
    }

    #[test]
    fn uncommitted_missing_non_merged_source_refuses() {
        // Uncommitted (no commit record): merged-id source recoverable at the
        // final name, but a non-merged source (3) is gone — roll-back cannot
        // reconstruct the OLD generation, so it fails closed.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        touch(&dir.path().join(segment_filename(7)));
        touch(&dir.path().join(segment_filename(1)));
        write_v2_marker(dir.path(), cid, 5, 7, &[1, 3, 7]);
        let meta = meta_with(5, None);
        let kind = refusal(resolve_pending_compaction(
            dir.path(),
            &RealFs,
            Some(&meta),
            true,
        ));
        assert!(
            matches!(
                kind,
                CompactionRecoveryRefusal::SourceMissing { segment_id: 3 }
            ),
            "PROPERTY: a missing non-merged source aborts roll-back; got {kind:?}"
        );
    }

    #[test]
    fn uncommitted_conflicting_names_refuses() {
        // Both the final name and `.compact-src` exist for an uncommitted
        // transaction — a namespace state the protocol cannot produce.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        touch(&dir.path().join(segment_filename(7)));
        touch(&compaction_source_temp_path(dir.path(), 7));
        touch(&dir.path().join(segment_filename(1)));
        write_v2_marker(dir.path(), cid, 5, 7, &[1, 7]);
        let meta = meta_with(5, None);
        let kind = refusal(resolve_pending_compaction(
            dir.path(),
            &RealFs,
            Some(&meta),
            true,
        ));
        assert!(
            matches!(
                kind,
                CompactionRecoveryRefusal::ConflictingNames { merged_id: 7 }
            ),
            "PROPERTY: final + .compact-src both present is a refusal; got {kind:?}"
        );
    }

    #[test]
    fn uncommitted_rolls_back_and_restores_the_source_set() {
        // No commit record: the merged-id source was renamed to `.compact-src`
        // and a staged replacement was written, but nothing committed. Roll back:
        // restore `.compact-src` → final, delete the staged leftover, clear.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        let src = compaction_source_temp_path(dir.path(), 7);
        let staged = compaction_staged_path(dir.path(), 7);
        touch(&src);
        touch(&staged);
        touch(&dir.path().join(segment_filename(1)));
        touch(&dir.path().join(segment_filename(2)));
        write_v2_marker(dir.path(), cid, 5, 7, &[1, 2, 7]);
        let meta = meta_with(5, None);

        let action = resolve_pending_compaction(dir.path(), &RealFs, Some(&meta), true)
            .expect("uncommitted transaction rolls back");
        assert_eq!(action, CompactionRecoveryAction::RolledBack);
        assert!(
            RealFs.metadata(&dir.path().join(segment_filename(7))).is_ok(),
            "PROPERTY: the merged-id source is restored to the final name"
        );
        assert!(
            RealFs.metadata(&src).is_err(),
            "PROPERTY: the .compact-src temp is gone after restore"
        );
        assert!(
            RealFs.metadata(&staged).is_err(),
            "PROPERTY: the staged .compact-new leftover is deleted"
        );
        assert!(
            RealFs
                .metadata(&dir.path().join(COMPACTION_MARKER_FILENAME))
                .is_err(),
            "PROPERTY: the marker is cleared last"
        );
    }

    #[test]
    fn committed_rolls_forward_and_completes_the_new_generation() {
        // The commit record's token AND merged id match the marker: the staged
        // replacement is renamed to final, sources retired, marker cleared.
        let dir = tempfile::tempdir().expect("tempdir");
        let cid = mint_compaction_id();
        let staged = compaction_staged_path(dir.path(), 7);
        let src = compaction_source_temp_path(dir.path(), 7);
        touch(&staged);
        touch(&src);
        touch(&dir.path().join(segment_filename(1)));
        touch(&dir.path().join(segment_filename(2)));
        write_v2_marker(dir.path(), cid, 5, 7, &[1, 2, 7]);
        let meta = meta_with(
            5,
            Some(CompactionCommit {
                compaction_id: cid,
                authority_image_id: mint_authority_image_id(),
                merged_segment_id: 7,
                source_segment_ids: vec![1, 2, 7],
            }),
        );

        let action = resolve_pending_compaction(dir.path(), &RealFs, Some(&meta), true)
            .expect("committed transaction rolls forward");
        assert_eq!(action, CompactionRecoveryAction::RolledForward);
        assert!(
            RealFs.metadata(&dir.path().join(segment_filename(7))).is_ok(),
            "PROPERTY: the replacement is durable at the final name"
        );
        assert!(
            RealFs.metadata(&staged).is_err() && RealFs.metadata(&src).is_err(),
            "PROPERTY: staged and .compact-src leftovers are retired"
        );
        assert!(
            RealFs.metadata(&dir.path().join(segment_filename(1))).is_err()
                && RealFs.metadata(&dir.path().join(segment_filename(2))).is_err(),
            "PROPERTY: the non-merged sources are retired"
        );
        assert!(
            RealFs
                .metadata(&dir.path().join(COMPACTION_MARKER_FILENAME))
                .is_err(),
            "PROPERTY: the marker is cleared last"
        );
    }

    #[test]
    fn commit_token_mismatch_rolls_back_not_forward() {
        // MUTATION KILLER: a commit record IS present but its token differs from
        // the marker's (a different, unrelated transaction). Direction must be
        // roll-BACK — a mutant that treats "commit present" as committed would
        // wrongly roll forward onto an uncommitted namespace.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_cid = mint_compaction_id();
        let other_cid = mint_compaction_id();
        // Uncommitted namespace: merged-id source still at the final name, all
        // sources present, no staged replacement.
        touch(&dir.path().join(segment_filename(7)));
        touch(&dir.path().join(segment_filename(1)));
        touch(&dir.path().join(segment_filename(2)));
        write_v2_marker(dir.path(), marker_cid, 5, 7, &[1, 2, 7]);
        let meta = meta_with(
            5,
            Some(CompactionCommit {
                compaction_id: other_cid,
                authority_image_id: mint_authority_image_id(),
                merged_segment_id: 7,
                source_segment_ids: vec![1, 2, 7],
            }),
        );

        let action = resolve_pending_compaction(dir.path(), &RealFs, Some(&meta), true)
            .expect("token mismatch resolves as roll-back");
        assert_eq!(
            action,
            CompactionRecoveryAction::RolledBack,
            "PROPERTY: a non-matching commit token means the transaction never committed"
        );
    }

    #[test]
    fn legacy_marker_is_unsupported() {
        // A pre-token (v1) marker never flows through the state machine: it is a
        // diagnostic parse that refuses (contract A9), never an automatic v1
        // recovery.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("compaction.pending.json"),
            br#"{"merged_id":7,"source_segment_ids":[1,2,7]}"#,
        )
        .expect("write legacy json marker");
        let meta = meta_with(5, None);
        let kind = refusal(resolve_pending_compaction(
            dir.path(),
            &RealFs,
            Some(&meta),
            true,
        ));
        assert!(
            matches!(kind, CompactionRecoveryRefusal::LegacyMarkerUnsupported { .. }),
            "PROPERTY: a legacy v1 marker is unsupported and refuses; got {kind:?}"
        );
    }
}

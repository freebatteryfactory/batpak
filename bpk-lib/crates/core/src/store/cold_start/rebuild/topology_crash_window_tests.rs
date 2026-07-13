//! Unit-level crash-window fixtures for the compaction namespace transaction
//! (#177/#195), driven against the recovery predicate itself.
//!
//! PROVES: each of the 9 declared crash boundaries of the compaction transaction
//! (plus the two undecidable refusal states) resolves — via
//! [`resolve_pending_compaction`] under the three-artifact agreement law
//! (contract A8) — into EXACTLY the old complete generation (roll back), the new
//! complete generation (roll forward), or a typed
//! [`StoreError::CompactionRecoveryRefused`]. Direction is decided by the
//! `store.meta` commit record matching the marker's branded `compaction_id` +
//! merged id — NEVER by file existence.
//! CATCHES: roll-forward authorized by a bare final-name existence
//! (`refusal_final_name_alone_must_not_authorize_roll_forward`); a present-but-
//! non-matching commit token wrongly treated as committed
//! (`window_5`); a source retired/undead through the recovery predicate
//! (`window_6..8` resolve NEW-only — no source id survives); a missing
//! non-merged source silently rolled back (`refusal_missing_nonmerged_source_*`).
//! SEEDED: hand-constructed durable namespaces in a `RealFs` tempdir, every
//! constructed fact asserted through the fs BEFORE the predicate runs
//! (anti-vacuity), then the exact [`CompactionRecoveryAction`] and the exact
//! post-repair segment set (or exact refusal variant) are asserted.
//!
//! This module is `#[cfg(test)]`-included from `topology.rs` (contract R16) so it
//! can drive the `pub(crate)`/`pub(super)` marker + segment-listing internals
//! that `tests/` cannot reach; the roll-back/roll-forward decision + physical
//! repair live in the sibling `compaction_recovery` module (re-exported by
//! `rebuild`). Names follow the plan-05 FILE 5 `window_{1..9}_*` / `refusal_*`
//! grammar the L4-coverage gate counts by prefix.

use super::super::{resolve_pending_compaction, CompactionRecoveryAction};
use super::{
    compaction_source_temp_path, compaction_staged_path, segment_paths, write_pending_compaction,
    PendingCompactionV2, COMPACTION_MARKER_FILENAME,
};
use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
use crate::store::platform::fs::{create_new_file, RealFs, StoreFs};
use crate::store::segment::segment_filename;
use crate::store::store_meta::{CompactionCommit, StoreMetaData};
use crate::store::{CompactionRecoveryRefusal, StoreError};
use std::path::{Path, PathBuf};

/// One fixed store lineage shared by every marker + `store.meta` in this module,
/// so agreement never trips on a spurious foreign-lineage refusal.
const LINEAGE: u128 = 0x0BAD_C0DE_0BAD_C0DE_0BAD_C0DE_0BAD_C0DE;

/// The transaction token stamped in a marker AND (for committed windows) in the
/// `store.meta` commit record — the one durable fact that flips direction.
fn txn_id() -> CompactionId {
    CompactionId::from_u128(0x0000_1177_0000_0195_0000_1177_0000_0195)
}

/// A DIFFERENT token, for the present-but-non-matching-commit mutation killers.
fn other_txn_id() -> CompactionId {
    CompactionId::from_u128(0x0000_DEAD_0000_BEEF_0000_DEAD_0000_BEEF)
}

fn image_id() -> AuthorityImageId {
    AuthorityImageId::from_u128(0x0000_00FF_0000_00FF_0000_00FF_0000_00FF)
}

fn write_marker(dir: &Path, compaction_id: CompactionId, merged_id: u64, sources: &[u64]) {
    write_pending_compaction(
        dir,
        &PendingCompactionV2 {
            compaction_id,
            lineage: StoreLineage::from_u128(LINEAGE),
            merged_id,
            source_segment_ids: sources.to_vec(),
            expected_authority_image_id: image_id(),
        },
        &RealFs,
    )
    .expect("write v2 pending-compaction marker");
}

fn commit(compaction_id: CompactionId, merged_id: u64, sources: &[u64]) -> CompactionCommit {
    CompactionCommit {
        compaction_id,
        authority_image_id: image_id(),
        merged_segment_id: merged_id,
        source_segment_ids: sources.to_vec(),
    }
}

fn meta(commit: Option<CompactionCommit>) -> StoreMetaData {
    StoreMetaData {
        lineage: LINEAGE,
        idemp_authority: None,
        last_compaction_commit: commit,
    }
}

fn seg(dir: &Path, id: u64) -> PathBuf {
    dir.join(segment_filename(id))
}

fn touch(path: &Path) {
    drop(create_new_file(path).expect("create fixture file"));
}

fn marker_path(dir: &Path) -> PathBuf {
    dir.join(COMPACTION_MARKER_FILENAME)
}

fn assert_present(path: &Path, what: &str) {
    assert!(
        RealFs.metadata(path).is_ok(),
        "PROPERTY (anti-vacuity): {what} must exist at {}",
        path.display()
    );
}

fn assert_absent(path: &Path, what: &str) {
    assert!(
        RealFs.metadata(path).is_err(),
        "PROPERTY (anti-vacuity): {what} must be absent at {}",
        path.display()
    );
}

/// The id-sorted segment set the post-repair predicate resolves — the OLD or NEW
/// generation, depending on the window.
fn resolved_ids(dir: &Path) -> Vec<u64> {
    let mut ids: Vec<u64> = segment_paths(dir, &RealFs)
        .expect("segment listing succeeds once the marker is cleared")
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    ids.sort_unstable();
    ids
}

/// Extract the typed refusal reason, or fail the property loudly. `if let` (not a
/// `match` with a named catch-all) keeps clear of `clippy::wildcard_enum_match_arm`
/// over `StoreError`.
fn refusal(result: Result<CompactionRecoveryAction, StoreError>) -> CompactionRecoveryRefusal {
    if let Err(StoreError::CompactionRecoveryRefused { kind, .. }) = result {
        return kind;
    }
    fail_property(&format!(
        "PROPERTY: expected a CompactionRecoveryRefused, got {result:?}"
    ))
}

fn fail_property(message: &str) -> ! {
    assert!(std::hint::black_box(false), "{message}");
    unreachable!("the assert above always fires")
}

fn resolve(dir: &Path, meta: &StoreMetaData) -> Result<CompactionRecoveryAction, StoreError> {
    resolve_pending_compaction(dir, &RealFs, Some(meta), true)
}

// ---- OLD-generation windows (1–5): uncommitted → roll back ----------------

#[test]
fn window_1_marker_only_resolves_the_original_source_set() {
    // Marker durable, no source rename yet: the merged-id source is still at its
    // ORIGINAL (final) name and every non-merged source is present. No commit
    // record ⇒ roll back to the OLD generation.
    let dir = tempfile::tempdir().expect("tempdir");
    touch(&seg(dir.path(), 1));
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(None);

    assert_present(&seg(dir.path(), 1), "merged-id source at its final name");
    assert_present(&seg(dir.path(), 2), "non-merged source");
    assert_absent(
        &compaction_source_temp_path(dir.path(), 1),
        ".compact-src (no rename-away yet)",
    );
    assert_absent(&compaction_staged_path(dir.path(), 1), ".compact-new staged");

    let action = resolve(dir.path(), &meta).expect("marker-only rolls back");
    assert_eq!(action, CompactionRecoveryAction::RolledBack);
    assert_absent(&marker_path(dir.path()), "marker (cleared last)");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY (window 1): recovers exactly the OLD source set"
    );
}

#[test]
fn window_2_partial_source_rename_resolves_the_original_source_set() {
    // The merged-id source has been renamed away to `.compact-src`; the final
    // name is absent and no staged replacement exists. Roll back restores the
    // source and recovers the OLD set.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    touch(&src);
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(None);

    assert_present(&src, "merged-id source relocated to .compact-src");
    assert_absent(&seg(dir.path(), 1), "final name (rename-away happened)");
    assert_present(&seg(dir.path(), 2), "non-merged source at original name");

    let action = resolve(dir.path(), &meta).expect("partial rename rolls back");
    assert_eq!(action, CompactionRecoveryAction::RolledBack);
    assert_present(&seg(dir.path(), 1), "merged-id source restored to final name");
    assert_absent(&src, ".compact-src consumed by the restore");
    assert_absent(&marker_path(dir.path()), "marker");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY (window 2): the relocated source resolves back into the OLD set"
    );
}

#[test]
fn window_3_partial_staged_replacement_resolves_the_original_source_set() {
    // As window 2, plus a partial `.compact-new` staged file. The staged
    // replacement NEVER enters the resolved set: roll back deletes it and
    // recovers the OLD generation.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    let staged = compaction_staged_path(dir.path(), 1);
    touch(&src);
    touch(&staged);
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(None);

    assert_present(&src, "merged-id .compact-src");
    assert_present(&staged, "partial .compact-new staged replacement");
    assert_absent(&seg(dir.path(), 1), "final name");

    let action = resolve(dir.path(), &meta).expect("partial staged rolls back");
    assert_eq!(action, CompactionRecoveryAction::RolledBack);
    assert_absent(&staged, "staged leftover deleted on roll back");
    assert_present(&seg(dir.path(), 1), "merged-id source restored");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY (window 3): staged bytes never join the resolved OLD set"
    );
}

#[test]
fn window_4_complete_staged_unpublished_authority_resolves_old() {
    // Complete staged replacement, source relocated, but store.meta carries NO
    // commit record (the authority was never published for this transaction).
    // Absence of a matching commit ⇒ roll back to OLD.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    let staged = compaction_staged_path(dir.path(), 1);
    touch(&src);
    touch(&staged);
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(None);

    assert!(
        meta.last_compaction_commit.is_none(),
        "PROPERTY (anti-vacuity): the authority is unpublished (no commit record)"
    );
    assert_present(&staged, "complete staged replacement");
    assert_absent(&seg(dir.path(), 1), "final name (unpublished)");

    let action = resolve(dir.path(), &meta).expect("unpublished authority rolls back");
    assert_eq!(action, CompactionRecoveryAction::RolledBack);
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY (window 4): an unpublished authority recovers the OLD generation"
    );
}

#[test]
fn window_5_published_authority_final_absent_resolves_old() {
    // A commit record IS present, but it binds a DIFFERENT (prior) transaction
    // token — the authority image on disk is a legal superset, yet THIS marker's
    // transaction never committed. Non-matching token ⇒ roll back to OLD. A
    // mutant that treated "some commit present" as committed would wrongly roll
    // forward onto an uncommitted namespace.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    let staged = compaction_staged_path(dir.path(), 1);
    touch(&src);
    touch(&staged);
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(Some(commit(other_txn_id(), 1, &[1, 2])));

    match &meta.last_compaction_commit {
        Some(c) => assert_ne!(
            c.compaction_id,
            txn_id(),
            "PROPERTY (anti-vacuity): the present commit binds a non-matching token"
        ),
        None => fail_property("PROPERTY (anti-vacuity): window 5 requires a present commit record"),
    }
    assert_absent(&seg(dir.path(), 1), "final name absent (uncommitted)");

    let action = resolve(dir.path(), &meta).expect("non-matching commit rolls back");
    assert_eq!(action, CompactionRecoveryAction::RolledBack);
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY (window 5): a non-matching commit token recovers the OLD generation"
    );
}

// ---- NEW-generation windows (6–9): committed → roll forward ----------------

#[test]
fn window_6_final_present_sources_unretired_resolves_new_only() {
    // Final replacement durable; the marker's commit token matches; every source
    // is still present (merged-id at `.compact-src`, non-merged at its original
    // name). Roll forward retires the sources: the resolved set is NEW-ONLY — no
    // source id survives (the undead gate at the predicate).
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    touch(&seg(dir.path(), 1));
    touch(&src);
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(Some(commit(txn_id(), 1, &[1, 2])));

    assert_present(&seg(dir.path(), 1), "final replacement");
    assert_present(&src, "merged-id source still at .compact-src");
    assert_present(&seg(dir.path(), 2), "non-merged source still present");

    let action = resolve(dir.path(), &meta).expect("committed transaction rolls forward");
    assert_eq!(action, CompactionRecoveryAction::RolledForward);
    assert_absent(&src, ".compact-src retired");
    assert_absent(&seg(dir.path(), 2), "non-merged source retired (undead gate)");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1],
        "PROPERTY (window 6): the ambiguous namespace resolves to the NEW generation only"
    );
}

#[test]
fn window_7_partially_retired_sources_resolve_new_only() {
    // Final durable; a PROPER subset of sources already retired (source 2 gone,
    // merged-id `.compact-src` remains). Roll forward retires the remainder.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = compaction_source_temp_path(dir.path(), 1);
    touch(&seg(dir.path(), 1));
    touch(&src);
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(Some(commit(txn_id(), 1, &[1, 2])));

    assert_present(&seg(dir.path(), 1), "final replacement");
    assert_present(&src, "merged-id .compact-src not yet retired");
    assert_absent(&seg(dir.path(), 2), "non-merged source already retired");

    let action = resolve(dir.path(), &meta).expect("partially retired rolls forward");
    assert_eq!(action, CompactionRecoveryAction::RolledForward);
    assert_absent(&src, "remaining .compact-src retired");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1],
        "PROPERTY (window 7): a partial retirement completes to the NEW generation"
    );
}

#[test]
fn window_8_stale_marker_after_full_retirement_resolves_new() {
    // Final durable; every source retired; only the (stale) marker remains. Roll
    // forward simply clears the marker.
    let dir = tempfile::tempdir().expect("tempdir");
    touch(&seg(dir.path(), 1));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(Some(commit(txn_id(), 1, &[1, 2])));

    assert_present(&seg(dir.path(), 1), "final replacement");
    assert_present(&marker_path(dir.path()), "stale marker");
    assert_absent(&compaction_source_temp_path(dir.path(), 1), ".compact-src");
    assert_absent(&seg(dir.path(), 2), "non-merged source");

    let action = resolve(dir.path(), &meta).expect("stale marker rolls forward");
    assert_eq!(action, CompactionRecoveryAction::RolledForward);
    assert_absent(&marker_path(dir.path()), "stale marker cleared");
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1],
        "PROPERTY (window 8): a stale marker after full retirement resolves NEW"
    );
}

#[test]
fn window_9_cleared_marker_resolves_the_steady_state() {
    // The marker is already gone: recovery is a no-op and the plain segment
    // listing is the NEW steady state.
    let dir = tempfile::tempdir().expect("tempdir");
    touch(&seg(dir.path(), 1));
    let meta = meta(Some(commit(txn_id(), 1, &[1, 2])));

    assert_absent(&marker_path(dir.path()), "marker (cleared)");
    assert_present(&seg(dir.path(), 1), "steady-state final replacement");

    let action = resolve(dir.path(), &meta).expect("no marker resolves cleanly");
    assert_eq!(
        action,
        CompactionRecoveryAction::None,
        "PROPERTY (window 9): a cleared marker is an ordinary open (no recovery)"
    );
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1],
        "PROPERTY (window 9): the steady state lists exactly the NEW generation"
    );
}

// ---- Undecidable refusal states -------------------------------------------

#[test]
fn refusal_missing_nonmerged_source_under_rollback_is_typed() {
    // Uncommitted: the merged-id source is recoverable (at its final name), but a
    // NON-MERGED source named by the marker is gone — roll back cannot rebuild
    // the OLD generation, so it fails closed with the exact segment id.
    let dir = tempfile::tempdir().expect("tempdir");
    touch(&seg(dir.path(), 7));
    touch(&seg(dir.path(), 1));
    write_marker(dir.path(), txn_id(), 7, &[1, 3, 7]);
    let meta = meta(None);

    assert_present(&seg(dir.path(), 7), "merged-id source at final name");
    assert_present(&seg(dir.path(), 1), "non-merged source 1");
    assert_absent(&seg(dir.path(), 3), "non-merged source 3 (missing)");

    let kind = refusal(resolve(dir.path(), &meta));
    assert!(
        matches!(kind, CompactionRecoveryRefusal::SourceMissing { segment_id: 3 }),
        "PROPERTY: a missing non-merged source aborts roll back with SourceMissing {{ 3 }}; \
         got {kind:?}"
    );
    assert_present(
        &marker_path(dir.path()),
        "marker untouched (fail closed, not cleared)",
    );
}

#[test]
fn refusal_final_name_alone_must_not_authorize_roll_forward() {
    // THE #177 headline: the final merged name is present AND every source is
    // present, but store.meta holds NO matching commit record. File existence
    // alone MUST NOT authorize roll forward — the transaction never committed, so
    // recovery rolls BACK to the OLD generation (both sources survive), never to
    // the NEW-only set.
    let dir = tempfile::tempdir().expect("tempdir");
    touch(&seg(dir.path(), 1));
    touch(&seg(dir.path(), 2));
    write_marker(dir.path(), txn_id(), 1, &[1, 2]);
    let meta = meta(None);

    assert_present(&seg(dir.path(), 1), "final merged name present");
    assert_present(&seg(dir.path(), 2), "non-merged source present");
    assert!(
        meta.last_compaction_commit.is_none(),
        "PROPERTY (anti-vacuity): no commit record authorizes this final name"
    );

    let action = resolve(dir.path(), &meta).expect("uncommitted final name rolls back");
    assert_eq!(
        action,
        CompactionRecoveryAction::RolledBack,
        "PROPERTY: file existence alone may NOT authorize roll forward"
    );
    assert_eq!(
        resolved_ids(dir.path()),
        vec![1, 2],
        "PROPERTY: recovery keeps the OLD generation (both sources), never the NEW-only set"
    );
}

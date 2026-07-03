//! PROVES: the fork orchestration's two pure classifiers — `fork_copy_strategy`
//! maps each `CowStrategyUsed` rung to its evidence `ForkCopyStrategy`, and
//! `record_deep_copied_presence` sets exactly the one presence flag that matches
//! the deep-copied artifact kind (and nothing for a segment).
//! CATCHES: each `fork_copy_strategy` match-arm swap (Reflink reported as
//! Hardlink, etc.) and each `record_deep_copied_presence` arm swap/delete (a
//! deep-copied visibility-ranges file recorded under the wrong flag, or no flag
//! at all).
//! SEEDED: deterministic — no randomness, no clock, no store.

use super::{fork_copy_strategy, record_deep_copied_presence, ForkAccumulator};
use crate::store::file_classification::StoreFileKind;
use crate::store::fork_report::ForkCopyStrategy;
use crate::store::platform::fs::CowStrategyUsed;
use crate::store::segment::SegmentId;

#[test]
fn fork_copy_strategy_maps_each_cow_rung_to_its_evidence_strategy() {
    // The evidence strategy is the only record of HOW a forked file was copied.
    // Kills each match-arm swap in the CowStrategyUsed -> ForkCopyStrategy map.
    assert_eq!(
        fork_copy_strategy(CowStrategyUsed::Reflink),
        ForkCopyStrategy::Reflink
    );
    assert_eq!(
        fork_copy_strategy(CowStrategyUsed::Hardlink),
        ForkCopyStrategy::Hardlink
    );
    assert_eq!(
        fork_copy_strategy(CowStrategyUsed::DeepCopy),
        ForkCopyStrategy::DeepCopy
    );
}

#[test]
fn record_deep_copied_presence_sets_exactly_the_matching_presence_flag() {
    // Each deep-copied authority artifact toggles its OWN presence flag and no
    // other; a plain segment toggles none. Kills every match-arm swap/delete:
    // a visibility-ranges deep copy recorded under the pending-compaction or
    // idempotency flag (or dropped) loses fork evidence identity.
    assert_eq!(
        presence_after(&StoreFileKind::VisibilityRanges),
        (true, false, false),
        "visibility-ranges deep copy sets only the visibility flag"
    );
    assert_eq!(
        presence_after(&StoreFileKind::PendingCompactionMarker),
        (false, true, false),
        "pending-compaction deep copy sets only the marker flag"
    );
    assert_eq!(
        presence_after(&StoreFileKind::IdempotencyStore),
        (false, false, true),
        "idempotency-store deep copy sets only the idempotency flag"
    );
    let segment = StoreFileKind::Segment(SegmentId::from_stem("3").expect("segment id"));
    assert_eq!(
        presence_after(&segment),
        (false, false, false),
        "a deep-copied segment sets no authority presence flag"
    );
}

/// Run `record_deep_copied_presence` on a fresh accumulator and return the
/// `(visibility, pending_compaction, idempotency)` presence flags.
fn presence_after(kind: &StoreFileKind) -> (bool, bool, bool) {
    let mut acc = ForkAccumulator::default();
    record_deep_copied_presence(&mut acc, kind);
    (
        acc.copied_visibility_ranges_present,
        acc.copied_pending_compaction_marker_present,
        acc.copied_idempotency_store_present,
    )
}

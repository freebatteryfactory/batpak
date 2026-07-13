use super::{
    ForkStrategy, StoreFileKind, COMPACT_SOURCE_EXTENSION, COMPACT_STAGED_EXTENSION,
    CURSOR_DIRECTORY, KEYSET_FILENAME,
};
use crate::store::segment::SegmentId;
use std::path::Path;

#[test]
fn from_path_classifies_the_keyset_file_as_its_exact_kind() {
    // PROPERTY: the classifier is the single source of truth for
    // recognising the durable crypto-shred keyset — scans, snapshot, and
    // fork semantics all key off this kind, and a DEFAULT build (no
    // `payload-encryption`) must still recognise the file so it is never
    // mistaken for a segment or foreign junk. Deleting the match arm
    // demotes it to `Other`. Kills `delete match arm Some(KEYSET_FILENAME)`
    // in `StoreFileKind::from_path`.
    assert_eq!(
        StoreFileKind::from_path(Path::new(KEYSET_FILENAME)),
        StoreFileKind::Keyset,
        "the bare keyset filename must classify as Keyset"
    );
    assert_eq!(
        StoreFileKind::from_path(Path::new("/data/store-a/keyset.fbatk")),
        StoreFileKind::Keyset,
        "a full keyset path must classify by its file name"
    );
    assert_eq!(
        StoreFileKind::from_path(Path::new("keyset.fbatk.bak")),
        StoreFileKind::Other,
        "only the exact keyset file name is the keyset — no filename folklore"
    );
}

#[test]
fn should_clear_from_snapshot_destination_discriminates_store_artifacts_from_other() {
    // PROPERTY: the snapshot pre-clear pass must wipe store-shaped
    // artifacts left in a destination but leave foreign files (`Other`)
    // AND the crypto-shred keyset untouched — clearing a resident keyset
    // would crypto-shred every encrypted payload it protects. Mirrors the
    // fork-destination twin below; assert BOTH polarities. Kills
    // `should_clear_from_snapshot_destination -> bool with true`.
    let segment_id = SegmentId::from_stem("0").expect("base-10 stem parses");
    assert!(
        StoreFileKind::Segment(segment_id).should_clear_from_snapshot_destination(),
        "a store segment MUST be cleared from a snapshot destination before copy"
    );
    assert!(
        !StoreFileKind::Other.should_clear_from_snapshot_destination(),
        "a foreign (Other) file must NOT be cleared from a snapshot destination"
    );
    assert!(
        !StoreFileKind::Keyset.should_clear_from_snapshot_destination(),
        "the crypto-shred keyset must NOT be cleared from a snapshot destination"
    );
}

#[test]
fn should_clear_from_fork_destination_discriminates_store_artifacts_from_other() {
    // PROPERTY: the fork pre-clear pass must wipe store-shaped artifacts left
    // in a destination but leave foreign files (`Other`) untouched. A blanket
    // `-> true` would also clear caller files, so assert BOTH polarities.
    // Kills `should_clear_from_fork_destination -> bool with true`.
    assert!(
        !StoreFileKind::Other.should_clear_from_fork_destination(),
        "a foreign (Other) file must NOT be cleared from a fork destination"
    );
    let segment_id = SegmentId::from_stem("0").expect("base-10 stem parses");
    assert!(
        StoreFileKind::Segment(segment_id).should_clear_from_fork_destination(),
        "a store segment MUST be cleared from a fork destination before copy"
    );
}

#[test]
fn from_path_classifies_every_store_file_by_its_exact_name_or_extension() {
    // PROPERTY: `from_path` is the single classifier for every store-owned
    // artifact. Each `Some(<FILENAME>) => Self::Kind` match arm and each
    // extension branch must map to its OWN kind; deleting or reordering any
    // arm demotes that artifact to a wrong kind (or `Other`), and a whole-body
    // replacement would collapse them all. Kills the individual match-arm
    // deletions in `StoreFileKind::from_path` and the two `is_some_and(...)`
    // extension guards.
    //
    // Reference the source-of-truth constants (not string literals) so the
    // test tracks the classifier rather than inventing filename folklore.
    use crate::store::cold_start::checkpoint::CHECKPOINT_FILENAME;
    use crate::store::cold_start::mmap::MMAP_INDEX_FILENAME;
    use crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME;
    use crate::store::hidden_ranges::VISIBILITY_RANGES_FILENAME;
    use crate::store::index::idemp::IDEMP_FILENAME;
    use crate::store::segment::SEGMENT_EXTENSION;

    // A `*.fbat` with a valid base-10 stem is a Segment carrying that id.
    let segment_path = format!("42.{SEGMENT_EXTENSION}");
    assert_eq!(
        StoreFileKind::from_path(Path::new(&segment_path)),
        StoreFileKind::Segment(SegmentId::from_stem("42").expect("stem parses")),
        "a well-formed segment filename must classify as Segment with its id"
    );
    // A `*.fbat` whose stem is not a base-10 u64 is a MalformedSegment, NOT Other.
    let malformed_path = format!("not-a-number.{SEGMENT_EXTENSION}");
    assert!(
        matches!(
            StoreFileKind::from_path(Path::new(&malformed_path)),
            StoreFileKind::MalformedSegment(_)
        ),
        "a `.fbat` with a non-numeric stem must classify as MalformedSegment"
    );
    // The compaction-source extension branch.
    let compact_path = format!("7.{COMPACT_SOURCE_EXTENSION}");
    assert_eq!(
        StoreFileKind::from_path(Path::new(&compact_path)),
        StoreFileKind::CompactSource,
        "the compact-source extension must classify as CompactSource"
    );

    // Every by-filename arm maps to its own distinct kind.
    for (name, expected) in [
        (VISIBILITY_RANGES_FILENAME, StoreFileKind::VisibilityRanges),
        (CHECKPOINT_FILENAME, StoreFileKind::Checkpoint),
        (MMAP_INDEX_FILENAME, StoreFileKind::MmapIndex),
        (IDEMP_FILENAME, StoreFileKind::IdempotencyStore),
        (
            COMPACTION_MARKER_FILENAME,
            StoreFileKind::PendingCompactionMarker,
        ),
        (CURSOR_DIRECTORY, StoreFileKind::CursorDirectory),
        (KEYSET_FILENAME, StoreFileKind::Keyset),
    ] {
        assert_eq!(
            StoreFileKind::from_path(Path::new(name)),
            expected,
            "{name} must classify as {expected:?}"
        );
    }

    // A foreign file matches no arm and falls through to Other.
    assert_eq!(
        StoreFileKind::from_path(Path::new("README.md")),
        StoreFileKind::Other,
        "an unrecognised filename must classify as Other, not any store artifact"
    );
}

#[test]
fn segment_id_is_some_only_for_segment_kinds() {
    // PROPERTY: `segment_id()` exposes the id for a Segment and `None` for
    // every other kind. Kills `segment_id -> None` (would hide a real id) and
    // any arm that leaks `Some(..)` for a non-segment kind.
    let id = SegmentId::from_stem("13").expect("stem parses");
    assert_eq!(
        StoreFileKind::Segment(id).segment_id(),
        Some(id),
        "a Segment must yield its own id"
    );
    assert_eq!(
        StoreFileKind::VisibilityRanges.segment_id(),
        None,
        "a non-segment kind must yield None"
    );
    assert_eq!(
        StoreFileKind::Keyset.segment_id(),
        None,
        "the keyset kind has no segment id"
    );
}

#[test]
fn should_copy_into_snapshot_selects_only_the_durable_authorities() {
    // PROPERTY: only Segment / VisibilityRanges / IdempotencyStore /
    // PendingCompactionMarker are carried into a snapshot; regenerable caches
    // (Checkpoint, MmapIndex), the keyset, and foreign files are NOT. Kills
    // `should_copy_into_snapshot -> true` / `-> false` and any arm addition or
    // removal by asserting BOTH polarities across the boundary.
    let id = SegmentId::from_stem("1").expect("stem parses");
    for kind in [
        StoreFileKind::Segment(id),
        StoreFileKind::VisibilityRanges,
        StoreFileKind::IdempotencyStore,
        StoreFileKind::PendingCompactionMarker,
    ] {
        assert!(
            kind.should_copy_into_snapshot(),
            "{kind:?} is a durable authority and MUST be copied into a snapshot"
        );
    }
    for kind in [
        StoreFileKind::Checkpoint,
        StoreFileKind::MmapIndex,
        StoreFileKind::Keyset,
        StoreFileKind::Other,
    ] {
        assert!(
            !kind.should_copy_into_snapshot(),
            "{kind:?} must NOT be copied into a snapshot"
        );
    }
}

#[test]
fn fork_strategy_partitions_segments_by_active_boundary_and_maps_each_kind() {
    // PROPERTY: `fork_strategy` splits segments on the active boundary — an
    // OLDER sealed segment is ShareIfPossible, the ACTIVE segment is
    // DeepCopyAlways, and a NEWER (>active) segment falls through to Exclude.
    // Kills the `segment_id.as_u64() < active` `<`↔(`<=`/`==`/`>`) swap and the
    // `== active` `==`↔(`<`/`<=`/`>=`) swap: at the exact boundary and one on
    // each side the three outcomes must differ.
    let active = 5u64;
    let older = SegmentId::from_stem("4").expect("stem parses");
    let at = SegmentId::from_stem("5").expect("stem parses");
    let newer = SegmentId::from_stem("6").expect("stem parses");

    assert_eq!(
        StoreFileKind::Segment(older).fork_strategy(active),
        ForkStrategy::ShareIfPossible,
        "a sealed segment below the active id is shareable"
    );
    assert_eq!(
        StoreFileKind::Segment(at).fork_strategy(active),
        ForkStrategy::DeepCopyAlways,
        "the active segment (== boundary) must be deep-copied"
    );
    assert_eq!(
        StoreFileKind::Segment(newer).fork_strategy(active),
        ForkStrategy::Exclude,
        "a segment above the active id matches no share/copy arm and is excluded"
    );

    // The by-kind arms each map to their own strategy (arm-deletion kills).
    assert_eq!(
        StoreFileKind::VisibilityRanges.fork_strategy(active),
        ForkStrategy::DeepCopyAlways,
        "visibility ranges are a durable authority: deep-copied into a fork"
    );
    assert_eq!(
        StoreFileKind::Checkpoint.fork_strategy(active),
        ForkStrategy::CacheRegenerable,
        "the checkpoint is a regenerable cache in a fork"
    );
    assert_eq!(
        StoreFileKind::MmapIndex.fork_strategy(active),
        ForkStrategy::CacheRegenerable,
        "the mmap index is a regenerable cache in a fork"
    );
    assert_eq!(
        StoreFileKind::Keyset.fork_strategy(active),
        ForkStrategy::Exclude,
        "the keyset is excluded from Stage-B fork copy"
    );
    assert_eq!(
        StoreFileKind::Other.fork_strategy(active),
        ForkStrategy::Exclude,
        "a foreign file is excluded from a fork"
    );
}

#[test]
fn compact_staged_is_recognised_and_excluded_from_every_copy_set() {
    // PROPERTY: an in-flight compaction replacement under its staged name
    // (`NNNNNN.fbat.compact-new`, #177) classifies as `CompactStaged` and is
    // never part of a committed generation — it is EXCLUDED from snapshot and
    // fork copies and carries no segment id, yet it MUST be cleared from
    // snapshot and fork destinations so a stale staged file cannot survive a
    // copy. Kills deletion of the `COMPACT_STAGED_EXTENSION` arm in `from_path`
    // and any polarity flip on the four staged predicates.
    use crate::store::segment::SEGMENT_EXTENSION;

    // The staged extension is the LAST extension of the canonical staged name.
    let staged_path = format!("000007.{SEGMENT_EXTENSION}.{COMPACT_STAGED_EXTENSION}");
    assert_eq!(
        StoreFileKind::from_path(Path::new(&staged_path)),
        StoreFileKind::CompactStaged,
        "the compact-new staged extension must classify as CompactStaged"
    );
    assert_eq!(
        StoreFileKind::CompactStaged.segment_id(),
        None,
        "a staged replacement is not a committed segment and carries no id"
    );
    assert!(
        !StoreFileKind::CompactStaged.should_copy_into_snapshot(),
        "an uncommitted staged replacement must NOT be copied into a snapshot"
    );
    assert_eq!(
        StoreFileKind::CompactStaged.fork_strategy(5),
        ForkStrategy::Exclude,
        "an uncommitted staged replacement is excluded from a fork"
    );
    assert!(
        StoreFileKind::CompactStaged.should_clear_from_snapshot_destination(),
        "a stale staged replacement MUST be cleared from a snapshot destination"
    );
    assert!(
        StoreFileKind::CompactStaged.should_clear_from_fork_destination(),
        "a stale staged replacement MUST be cleared from a fork destination"
    );
}

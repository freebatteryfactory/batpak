use super::{StoreError, StoreLockMode};
use crate::coordinate::CoordinateError;
use std::error::Error as _;
use std::io;

fn assert_display_contains(error: &StoreError, needle: &str) {
    let display = error.to_string();
    assert!(
        display.contains(needle),
        "helper constructor display should contain {needle:?}, got {display:?}"
    );
}

#[test]
fn batch_failed_helper_preserves_item_index_and_source() {
    let error = StoreError::batch_failed(
        3,
        StoreError::Io(io::Error::new(io::ErrorKind::TimedOut, "append timed out")),
    );

    assert!(matches!(
        &error,
        StoreError::BatchFailed {
            item_index: 3,
            source
        } if matches!(source.as_ref(), StoreError::Io(_))
    ));
    assert_display_contains(&error, "batch failed at item 3");
    assert_display_contains(&error, "append timed out");
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("append timed out")),
        "BatchFailed helper should expose the wrapped StoreError as source"
    );
}

#[test]
fn batch_sync_failed_helper_preserves_count_and_source() {
    let error = StoreError::batch_sync_failed(4, StoreError::Io(io::Error::other("fsync failed")));

    assert!(matches!(
        &error,
        StoreError::BatchSyncFailed {
            item_count: 4,
            source
        } if matches!(source.as_ref(), StoreError::Io(_))
    ));
    assert_display_contains(&error, "batch sync failed after writing 4 items");
    assert_display_contains(&error, "fsync failed");
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("fsync failed")),
        "BatchSyncFailed helper should expose the wrapped StoreError as source"
    );
}

#[test]
fn corrupt_magic_helper_builds_corrupt_segment() {
    let error = StoreError::corrupt_magic(9);

    assert!(
        matches!(
            error,
            StoreError::CorruptSegment {
                segment_id: 9,
                ref detail
            } if detail == "bad magic"
        ),
        "expected bad-magic CorruptSegment, got {error:?}"
    );
    assert_display_contains(&error, "corrupt segment 9");
    assert_display_contains(&error, "bad magic");
    assert!(error.source().is_none());
}

#[test]
fn corrupt_eof_helper_builds_corrupt_segment() {
    let error = StoreError::corrupt_eof(11);

    assert!(
        matches!(
            error,
            StoreError::CorruptSegment {
                segment_id: 11,
                ref detail
            } if detail == "unexpected EOF during read"
        ),
        "expected EOF CorruptSegment, got {error:?}"
    );
    assert_display_contains(&error, "corrupt segment 11");
    assert_display_contains(&error, "unexpected EOF during read");
    assert!(error.source().is_none());
}

#[test]
fn corrupt_version_helper_builds_corrupt_segment() {
    let error = StoreError::corrupt_version(12, 99);

    assert!(
        matches!(
            error,
            StoreError::CorruptSegment {
                segment_id: 12,
                ref detail
            } if detail.contains("unsupported segment version: 99")
        ),
        "expected version CorruptSegment, got {error:?}"
    );
    assert_display_contains(&error, "corrupt segment 12");
    assert_display_contains(&error, "unsupported segment version: 99");
    assert!(error.source().is_none());
}

#[test]
fn cache_msg_helper_builds_cache_failed_without_typed_source() {
    let error = StoreError::cache_msg("cache metadata short read");

    assert!(matches!(error, StoreError::CacheFailed(_)));
    assert_display_contains(&error, "cache error");
    assert_display_contains(&error, "cache metadata short read");
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("cache metadata short read")),
        "CacheFailed helper should expose the boxed message error as source"
    );
}

#[test]
fn cache_error_helper_builds_cache_failed_with_typed_source() {
    let error = StoreError::cache_error(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "cache dir denied",
    ));

    assert!(matches!(error, StoreError::CacheFailed(_)));
    assert_display_contains(&error, "cache error");
    assert_display_contains(&error, "cache dir denied");
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("cache dir denied")),
        "CacheFailed typed helper should expose the wrapped source"
    );
}

#[test]
fn ser_msg_helper_builds_serialization_error() {
    let error = StoreError::ser_msg("frame exceeds u32::MAX");

    assert!(matches!(error, StoreError::Serialization(_)));
    assert_display_contains(&error, "serialization error");
    assert_display_contains(&error, "frame exceeds u32::MAX");
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("frame exceeds u32::MAX")),
        "Serialization helper should expose the boxed message error as source"
    );
}

#[test]
fn corrupt_segment_with_detail_helper_builds_corrupt_segment() {
    let error = StoreError::corrupt_segment_with_detail(13, "valid CRC but malformed msgpack");

    assert!(
        matches!(
            error,
            StoreError::CorruptSegment {
                segment_id: 13,
                ref detail
            } if detail == "valid CRC but malformed msgpack"
        ),
        "expected detail-preserving CorruptSegment, got {error:?}"
    );
    assert_display_contains(&error, "corrupt segment 13");
    assert_display_contains(&error, "valid CRC but malformed msgpack");
    assert!(error.source().is_none());
}

#[test]
fn store_locked_display_names_modes() {
    let read_only = StoreError::StoreLocked {
        path: "fixtures/store".into(),
        mode: StoreLockMode::ReadOnly,
    };
    let mutable = StoreError::StoreLocked {
        path: "fixtures/store".into(),
        mode: StoreLockMode::Mutable,
    };

    assert_display_contains(&read_only, "read-only");
    assert_display_contains(&mutable, "mutable");
}

#[test]
fn from_coordinate_error_routes_each_variant_to_its_dedicated_store_error() {
    // PROPERTY: `From<CoordinateError>` splits the hardening-specific rejections
    // (NUL / control / traversal) into their OWN top-level `StoreError` variants so
    // callers can match precise failure modes, while the remaining coordinate
    // rejections stay wrapped in `StoreError::Coordinate(_)`. Deleting or
    // reordering any arm in that `match` would mis-route a rejection. Kills the
    // per-variant match-arm deletions in `impl From<CoordinateError> for StoreError`.
    assert!(
        matches!(
            StoreError::from(CoordinateError::NulByte),
            StoreError::CoordinateNulByte
        ),
        "NulByte must route to the dedicated CoordinateNulByte variant"
    );
    assert!(
        matches!(
            StoreError::from(CoordinateError::ControlChar),
            StoreError::CoordinateControlChar
        ),
        "ControlChar must route to the dedicated CoordinateControlChar variant"
    );
    assert!(
        matches!(
            StoreError::from(CoordinateError::PathTraversal),
            StoreError::CoordinatePathTraversal
        ),
        "PathTraversal must route to the dedicated CoordinatePathTraversal variant"
    );

    // The remaining rejections stay wrapped, preserving the inner error.
    assert!(matches!(
        StoreError::from(CoordinateError::EmptyEntity),
        StoreError::Coordinate(CoordinateError::EmptyEntity)
    ));
    assert!(matches!(
        StoreError::from(CoordinateError::EmptyScope),
        StoreError::Coordinate(CoordinateError::EmptyScope)
    ));
    assert!(matches!(
        StoreError::from(CoordinateError::ForbiddenSeparator),
        StoreError::Coordinate(CoordinateError::ForbiddenSeparator)
    ));
    assert!(matches!(
        StoreError::from(CoordinateError::EntityTooLong { len: 5, max: 4 }),
        StoreError::Coordinate(CoordinateError::EntityTooLong { len: 5, max: 4 })
    ));
    assert!(matches!(
        StoreError::from(CoordinateError::ScopeTooLong { len: 9, max: 8 }),
        StoreError::Coordinate(CoordinateError::ScopeTooLong { len: 9, max: 8 })
    ));
}

#[test]
fn from_io_error_wraps_as_the_io_variant() {
    // Kills a mis-mapped `From<std::io::Error>` (the wrap must land in `Io`, not
    // some other variant).
    let error = StoreError::from(io::Error::new(io::ErrorKind::NotFound, "missing segment"));
    assert!(matches!(error, StoreError::Io(_)));
    assert!(
        error
            .source()
            .is_some_and(|source| source.to_string().contains("missing segment")),
        "the wrapped io::Error must be exposed as the error source"
    );
}

#[test]
fn source_exposes_wrapped_errors_and_is_none_for_leaf_variants() {
    // PROPERTY: `source()` returns the underlying error for wrapping variants and
    // `None` for self-contained (leaf) ones. Kills arm swaps in `source()` that
    // would either hide a wrapped source or fabricate one for a leaf variant.
    assert!(
        StoreError::Io(io::Error::other("disk gone"))
            .source()
            .is_some(),
        "Io must expose its inner io::Error as source"
    );
    assert!(
        StoreError::Coordinate(CoordinateError::EmptyEntity)
            .source()
            .is_some(),
        "Coordinate must expose its inner CoordinateError as source"
    );
    assert!(
        StoreError::WriterCrashed.source().is_none(),
        "WriterCrashed is a leaf variant with no source"
    );
    assert!(
        StoreError::IdempotencyRequired.source().is_none(),
        "IdempotencyRequired is a leaf variant with no source"
    );
}

#[test]
fn delegated_display_helpers_render_their_group_nonempty() {
    // PROPERTY: `Display::fmt` delegates whole variant groups to `fmt_*` helper
    // methods (kept off the main match to hold its complexity ratchet). Each helper
    // returns `std::fmt::Result`, so a `-> Ok(())` mutation would render an EMPTY
    // string for every variant it owns while `Display` still "succeeds". Pinning a
    // representative variant per helper to a distinctive substring kills the
    // `fmt_coordinate_violation`, `fmt_future_version`, `fmt_walk_violation`,
    // `fmt_chain_verification_failed`, `fmt_projection_state_violation`, and
    // `fmt_platform_violation` `-> Ok(())` mutants.

    // fmt_coordinate_violation
    assert_display_contains(&StoreError::CoordinateNulByte, "NUL");
    assert_display_contains(
        &StoreError::ReservedKind {
            index: Some(2),
            kind: 0x0001,
        },
        "reserved kind",
    );
    assert_display_contains(
        &StoreError::InvalidCoordinate {
            index: None,
            reason: "empty entity".to_owned(),
        },
        "invalid coordinate",
    );

    // fmt_future_version (mmap subject) — carries both version numbers.
    let mmap = StoreError::MmapFutureVersion {
        found: 9,
        supported: 3,
    };
    assert_display_contains(&mmap, "mmap index");
    assert_display_contains(&mmap, "version 9");
    assert_display_contains(&mmap, "version 3");

    // fmt_walk_violation
    assert_display_contains(
        &StoreError::RangeMalformed { start: 8, end: 4 },
        "malformed range",
    );
    assert_display_contains(
        &StoreError::AncestryCorrupt {
            cycle_at: crate::id::EventId::from(1u128),
        },
        "cycle",
    );

    // fmt_chain_verification_failed — echoes both defect counts.
    let chain = StoreError::ChainVerificationFailed {
        content_hash_mismatches: 2,
        dangling_links: 5,
    };
    assert_display_contains(&chain, "hash-chain verification failed");
    assert_display_contains(&chain, "2 content-hash");
    assert_display_contains(&chain, "5 dangling");

    // fmt_projection_state_violation
    assert_display_contains(
        &StoreError::ProjectionStateContractUnspecified {
            projection: "proj:orders".to_owned(),
        },
        "growth contract",
    );

    // fmt_platform_violation
    assert_display_contains(
        &StoreError::PlatformAdmissionFailed {
            capability: "mmap",
            reason: "no huge pages".to_owned(),
        },
        "platform admission failed",
    );
}

//! Store-owned filename classification.
//!
//! This keeps segment, snapshot, and cold-start artifact recognition in one
//! place so lifecycle code does not grow local filename folklore.

use crate::store::segment::{id::SegmentNameError, SegmentId, SEGMENT_EXTENSION};
use std::path::Path;

pub(crate) const COMPACT_SOURCE_EXTENSION: &str = "compact-src";
pub(crate) const CURSOR_DIRECTORY: &str = "cursors";

/// Filename of the durable crypto-shred keyset (opt-in `payload-encryption`).
///
/// Defined UNGATED (not behind `payload-encryption`) on purpose: a default build
/// that opens a store previously written with encryption must still RECOGNISE the
/// keyset file so scans never mistake it for a segment and so snapshot/fork copy
/// paths carry it forward instead of dropping it — dropping the keyset would
/// crypto-shred every encrypted payload. The gated persistence layer
/// (`keyscope::persist`) references this same constant so the on-disk name has a
/// single source of truth.
pub(crate) const KEYSET_FILENAME: &str = "keyset.fbatk";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StoreFileKind {
    Segment(SegmentId),
    MalformedSegment(SegmentNameError),
    VisibilityRanges,
    Checkpoint,
    MmapIndex,
    IdempotencyStore,
    PendingCompactionMarker,
    CompactSource,
    CursorDirectory,
    /// The durable crypto-shred keyset (`keyset.fbatk`). Recognised so scans and
    /// cleanup never mistake it for a segment. Stage B classifies it (and reads
    /// it at open); making it a first-class snapshot/fork authority — which
    /// touches the public snapshot/fork report wire formats — is Stage C work,
    /// landed with the encryption wiring and its schema bump.
    Keyset,
    /// The store lineage-metadata sidecar (`store.meta`, #205): the lineage
    /// identity plus the expected durable idempotency-authority anchor. A
    /// correctness authority — snapshot and fork must carry it (both copy the
    /// idempotency authority it binds), and cleanup must never mistake it for
    /// scratch.
    StoreMeta,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ForkStrategy {
    ShareIfPossible,
    DeepCopyAlways,
    CacheRegenerable,
    Exclude,
}

impl StoreFileKind {
    pub(crate) fn from_path(path: &Path) -> Self {
        if path.extension().is_some_and(|ext| ext == SEGMENT_EXTENSION) {
            return match SegmentId::from_filename(path) {
                Ok(segment_id) => Self::Segment(segment_id),
                Err(error) => Self::MalformedSegment(error),
            };
        }

        if path
            .extension()
            .is_some_and(|ext| ext == COMPACT_SOURCE_EXTENSION)
        {
            return Self::CompactSource;
        }

        match path.file_name().and_then(|name| name.to_str()) {
            Some(crate::store::hidden_ranges::VISIBILITY_RANGES_FILENAME) => Self::VisibilityRanges,
            Some(crate::store::cold_start::checkpoint::CHECKPOINT_FILENAME) => Self::Checkpoint,
            Some(crate::store::cold_start::mmap::MMAP_INDEX_FILENAME) => Self::MmapIndex,
            Some(crate::store::index::idemp::IDEMP_FILENAME) => Self::IdempotencyStore,
            Some(crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME) => {
                Self::PendingCompactionMarker
            }
            Some(CURSOR_DIRECTORY) => Self::CursorDirectory,
            Some(KEYSET_FILENAME) => Self::Keyset,
            Some(crate::store::store_meta::STORE_META_FILENAME) => Self::StoreMeta,
            _ => Self::Other,
        }
    }

    pub(crate) fn segment_id(&self) -> Option<SegmentId> {
        match self {
            Self::Segment(segment_id) => Some(*segment_id),
            Self::MalformedSegment(_)
            | Self::VisibilityRanges
            | Self::Checkpoint
            | Self::MmapIndex
            | Self::IdempotencyStore
            | Self::PendingCompactionMarker
            | Self::CompactSource
            | Self::CursorDirectory
            | Self::Keyset
            | Self::StoreMeta
            | Self::Other => None,
        }
    }

    pub(crate) fn should_copy_into_snapshot(&self) -> bool {
        // The durable idempotency store is a correctness authority, so a
        // snapshot must carry it forward — otherwise restoring from the
        // snapshot would silently lose cross-compaction dedup memory.
        // justifies: INV-IDEMPOTENCY-DURABLE-WINDOW
        //
        // The crypto-shred keyset is deliberately EXCLUDED from snapshots. Keys
        // must never travel with the ciphertext they open: a snapshot that
        // carried the keyset could, after a `shred_scope`, be restored to
        // resurrect crypto-shredded data (D24). So keyset portability is
        // fail-closed — `snapshot`/`fork` REFUSE an encryption-active store by
        // default and require `KeysetPolicy::ExcludeKeys` to proceed with a
        // keys-excluded copy whose keyset is managed out-of-band. Restoring such
        // a copy without its keyset reports `StoreError::KeysetMissing`, never a
        // Shredded lookalike.
        // store.meta is COPIED by snapshot too, but through an explicit
        // unreported copy step in `lifecycle_snapshot` — routing it through
        // this list would push it into the public snapshot-report vocabulary
        // (`SnapshotFileKind`), which is a wire-schema change (the same
        // Stage-C boundary as the keyset note above). See #205.
        matches!(
            self,
            Self::Segment(_)
                | Self::VisibilityRanges
                | Self::IdempotencyStore
                | Self::PendingCompactionMarker
        )
    }

    pub(crate) fn should_clear_from_snapshot_destination(&self) -> bool {
        matches!(
            self,
            Self::Segment(_)
                | Self::MalformedSegment(_)
                | Self::VisibilityRanges
                | Self::Checkpoint
                | Self::MmapIndex
                | Self::IdempotencyStore
                | Self::PendingCompactionMarker
                | Self::CompactSource
                | Self::StoreMeta
        )
    }

    pub(crate) fn fork_strategy(&self, active_segment_id: u64) -> ForkStrategy {
        match self {
            Self::Segment(segment_id) if segment_id.as_u64() < active_segment_id => {
                ForkStrategy::ShareIfPossible
            }
            Self::Segment(segment_id) if segment_id.as_u64() == active_segment_id => {
                ForkStrategy::DeepCopyAlways
            }
            // store.meta rides with the idempotency authority: a fork mints NO
            // new lineage identity (owner ruling, #205) — it copies the file,
            // because the copied authority image is bound to that lineage.
            Self::VisibilityRanges
            | Self::IdempotencyStore
            | Self::PendingCompactionMarker
            | Self::StoreMeta => ForkStrategy::DeepCopyAlways,
            Self::Checkpoint | Self::MmapIndex => ForkStrategy::CacheRegenerable,
            // Keyset EXCLUDED from fork for the same D24 reason it is excluded
            // from snapshot: keys must never travel with their ciphertext (a
            // restored copy could otherwise resurrect crypto-shredded data). See
            // `should_copy_into_snapshot`.
            Self::Segment(_)
            | Self::MalformedSegment(_)
            | Self::CompactSource
            | Self::CursorDirectory
            | Self::Keyset
            | Self::Other => ForkStrategy::Exclude,
        }
    }

    pub(crate) fn should_clear_from_fork_destination(&self) -> bool {
        matches!(
            self,
            Self::Segment(_)
                | Self::MalformedSegment(_)
                | Self::VisibilityRanges
                | Self::Checkpoint
                | Self::MmapIndex
                | Self::IdempotencyStore
                | Self::PendingCompactionMarker
                | Self::CompactSource
                | Self::CursorDirectory
                | Self::StoreMeta
        )
    }
}

#[cfg(test)]
#[path = "file_classification_tests.rs"]
mod tests;

//! Published downstream-handling taxonomy for [`StoreError`] (#180).
//!
//! This is the ONE source of truth for how a `StoreError` variant should be
//! handled by a caller. The classification used to live as a private `classify`
//! helper in `crates/testkit/src/store_error_contract.rs`; it is promoted here
//! (public API) so downstream code can branch on stable handling semantics
//! without re-deriving them, and the testkit contract table now asserts through
//! this function.
//!
//! [`handling_class`](StoreError::handling_class) is an EXHAUSTIVE match with no
//! catch-all arm. `StoreError` is defined in this crate, so `#[non_exhaustive]`
//! imposes no wildcard obligation here (that only binds downstream crates), and
//! the repo-wide `clippy::wildcard_enum_match_arm` denial makes silent drift
//! impossible: every future variant added to `StoreError` forces a reviewed arm
//! in this file before the crate compiles. justifies: INV-STOREERROR-HANDLING-CLASS

use super::StoreError;

/// How a caller should handle a [`StoreError`]. Each `StoreError` variant maps
/// to exactly one class via [`StoreError::handling_class`]; the mapping is a
/// stable contract (drift is caught by `tests/store_error_contract.rs`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlingClass {
    /// A caller-fault rejection the caller can correct and retry (bad
    /// coordinate, CAS mismatch, misconfiguration, a mis-declared projection).
    Domain,
    /// A transient operational fault a caller may safely retry unchanged
    /// (I/O timeout, cache miss, soft-cap backpressure, a durability wait that
    /// timed out).
    RetryableOperational,
    /// Corruption, tampering, or an invariant violation that must HALT rather
    /// than retry — the store fails closed and the operator must remediate
    /// (restore from backup, upgrade the reader, or repair on-disk state).
    FailClosedOperational,
}

impl StoreError {
    /// Classify this error into its stable downstream [`HandlingClass`] (#180).
    ///
    /// The batch-wrapper variants forward to the class of the error they wrap,
    /// so `batch_failed(io_timeout)` stays retryable and `batch_failed(corrupt)`
    /// stays fail-closed.
    pub fn handling_class(&self) -> HandlingClass {
        match self {
            // Caller-fault rejections: correct the input and retry.
            Self::StoreLocked { .. }
            | Self::Coordinate(_)
            | Self::CheckpointId(_)
            | Self::NotFound(_)
            | Self::SequenceMismatch { .. }
            | Self::Configuration(_)
            | Self::EventPayloadRegistry(_)
            | Self::UpcastChainIncomplete(_)
            | Self::InvalidPayloadVersion { .. }
            | Self::IdempotencyRequired
            | Self::VisibilityFenceActive
            | Self::VisibilityFenceNotActive
            | Self::VisibilityFenceCancelled
            | Self::IdempotencyPartialBatch { .. }
            | Self::RangeMalformed { .. }
            | Self::InvalidCoordinate { .. }
            | Self::ReservedKind { .. }
            | Self::InvalidCausation { .. }
            | Self::InvalidCommitMetadata { .. }
            | Self::CoordinateNulByte
            | Self::CoordinatePathTraversal
            | Self::CoordinateControlChar
            | Self::BatchItemTooLarge { .. }
            | Self::EntityClockOverflow { .. }
            | Self::InvalidClock { .. }
            // Projection-materialization contract faults: the caller's projection
            // declaration is wrong (no growth contract, unreportable extent, or a
            // declared bound the data outgrew). Corrected by fixing the projection.
            | Self::ProjectionStateContractUnspecified { .. }
            | Self::ProjectionStateExtentUnavailable { .. }
            | Self::ProjectionStateBoundExceeded { .. } => HandlingClass::Domain,

            // Transient operational faults: safe to retry unchanged.
            Self::Io(_)
            | Self::CacheFailed(_)
            | Self::CheckpointWriteFailed { .. }
            | Self::IdempotencyOverflowFailClosed { .. }
            | Self::WaitTimeout { .. } => HandlingClass::RetryableOperational,

            // Batch wrappers inherit the class of the error they carry.
            Self::BatchFailed { source, .. } | Self::BatchSyncFailed { source, .. } => {
                source.handling_class()
            }

            // Corruption, invariant violations, and fail-closed recovery refusals:
            // must halt, never retry. Restore/repair/upgrade to remediate.
            Self::Serialization(_)
            | Self::CrcMismatch { .. }
            | Self::CorruptSegment { .. }
            | Self::PlatformProfileInvalid { .. }
            | Self::PlatformProfileMismatch { .. }
            | Self::PlatformAdmissionFailed { .. }
            | Self::WriterCrashed
            | Self::SequenceGateViolation { .. }
            | Self::CorruptFrame { .. }
            | Self::SegmentTooManyEntries { .. }
            | Self::InternerExhausted { .. }
            | Self::DataDirMalformed { .. }
            | Self::AncestryCorrupt { .. }
            | Self::IdempotencyFutureVersion { .. }
            | Self::MmapFutureVersion { .. }
            | Self::CheckpointFutureVersion { .. }
            | Self::HiddenRangesFutureVersion { .. }
            | Self::ForkEvidenceFutureVersion { .. }
            | Self::ImportProvenanceFutureVersion { .. }
            | Self::SidxFutureVersion { .. }
            | Self::HiddenRangesCorrupt { .. }
            | Self::StoreMetadataCorrupt { .. }
            | Self::StoreMetadataMissing { .. }
            | Self::StoreMetadataFutureVersion { .. }
            // A pending-compaction transaction that cannot be resolved fails
            // closed in the `StoreMetadataMissing` family (contract A17 / A13).
            | Self::CompactionRecoveryRefused { .. }
            | Self::IdempotencyAuthorityCorrupt { .. }
            | Self::IdempotencyAuthorityMissing { .. }
            | Self::IdempotencyAuthorityStale { .. }
            | Self::IdempotencyAuthorityForeign { .. }
            // A refused idempotency-authority restore shares the foreign-authority
            // fail-closed family (plan-03 P12).
            | Self::IdempotencyRestoreRefused { .. }
            | Self::CursorCheckpointCorrupt { .. }
            | Self::CursorCheckpointRegionMismatch { .. }
            | Self::InvariantViolation { .. }
            // An at-open chain recompute that found tampering must not hand back
            // the store.
            | Self::ChainVerificationFailed { .. } => HandlingClass::FailClosedOperational,

            #[cfg(feature = "payload-encryption")]
            // A destroyed payload key (crypto-shred) and a non-portable keyset are
            // expected, caller-actionable domain facts, not corruption: shredding
            // is deliberate and the portability refusal is cleared by passing an
            // explicit `KeysetPolicy`. A selector that cannot address the store's
            // key-scope granularity is a typed programming error.
            Self::PayloadShredded { .. }
            | Self::KeysetNotPortable { .. }
            | Self::ShredSelectorMismatch { .. } => HandlingClass::Domain,

            #[cfg(feature = "payload-encryption")]
            // A corrupt/absent keyset, a failed authenticated seal, and a decrypt
            // failure with the key present (tampering) all fail closed — the same
            // posture as the durable idempotency authority.
            Self::KeysetCorrupt { .. }
            | Self::PayloadSealFailed { .. }
            | Self::PayloadDecryptFailed { .. }
            | Self::KeysetMissing { .. } => HandlingClass::FailClosedOperational,

            #[cfg(feature = "dangerous-test-hooks")]
            Self::FaultInjected(_) => HandlingClass::FailClosedOperational,
        }
    }
}

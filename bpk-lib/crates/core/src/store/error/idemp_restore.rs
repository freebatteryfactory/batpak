/// Typed reason a caller-supplied idempotency-authority export was refused at
/// restore (#188). Restore is the authorization ceremony: an offline,
/// closed-directory door that validates the export against this store's durable
/// state, MINTS a fresh local authority image id, and republishes through the
/// canonical `store.meta` flow (it never requires the dead store to have
/// pre-authorized the incoming image's token — no circularity). These variants
/// are therefore guards on lineage, coverage, rollback, format integrity, and an
/// in-progress compaction transaction — not a token-authorization check.
#[derive(Debug)]
#[non_exhaustive]
pub enum IdempotencyRestoreRefusal {
    /// The export bytes are not a readable canonical authority image.
    Corrupt {
        /// Typed corruption reason (same taxonomy as the on-disk sidecar).
        kind: super::IdempAuthorityCorruption,
    },
    /// The export declares a format version newer than this binary understands.
    FutureVersion {
        /// Format version stamped in the export header.
        found: u16,
        /// Highest format version this build understands.
        supported: u16,
    },
    /// The export was written for a different store lineage.
    ForeignLineage {
        /// Lineage stamped on the export.
        image_lineage: crate::id::StoreIdentity,
        /// Lineage of the target store.
        store_lineage: crate::id::StoreIdentity,
    },
    /// The export's covered frontier is behind authority state the target
    /// already holds; restoring it would narrow the dedup contract.
    StaleRollback {
        /// Covered sequence declared by the export.
        image_covered: u64,
        /// Covered sequence the target store already holds authority through.
        expected_covered: u64,
    },
    /// The export's history anchor (or a restored entry) contradicts this
    /// store's own committed history: a diverged sibling fork's image.
    SiblingDivergence {
        /// Event id at the contradicting frontier.
        event_id: crate::id::EventId,
        /// Commit-order sequence at which the histories disagree.
        global_sequence: u64,
    },
    /// The export contains user entries recorded past its own declared
    /// covered frontier — no canonical publication produces this shape.
    EntriesBeyondCoverage {
        /// Covered sequence the export declares.
        covered_global_sequence: u64,
        /// Sequence of the first user entry found beyond that coverage.
        first_offender_sequence: u64,
    },
    /// The target directory has a pending-compaction transaction that has not
    /// been resolved into exactly the old or the new generation, so its durable
    /// authority state is not yet settled. Restore refuses until the namespace
    /// transaction is completed by a writable open (#188/A13). Replaces the
    /// dead "unauthorized image generation" refusal: the offline door mints its
    /// own generation and never inspects the incoming token's authorization.
    PendingCompactionUnresolved {
        /// Path of the pending-compaction marker observed in the directory.
        marker_path: std::path::PathBuf,
    },
}

impl std::fmt::Display for IdempotencyRestoreRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupt { kind } => write!(
                f,
                "the idempotency-authority export could not be read as a canonical \
                 image ({kind}); the bytes were damaged in transit or storage — \
                 take a fresh export from a healthy store"
            ),
            Self::FutureVersion { found, supported } => write!(
                f,
                "the idempotency-authority export declares format version {found} but \
                 this build understands at most {supported}; upgrade batpak to a build \
                 that understands this export, or re-export under the current build"
            ),
            Self::ForeignLineage {
                image_lineage,
                store_lineage,
            } => write!(
                f,
                "the idempotency-authority export was written for {image_lineage} but \
                 this store is {store_lineage}; an export is restorable only into its \
                 own lineage — take an export from THIS store's lineage"
            ),
            Self::StaleRollback {
                image_covered,
                expected_covered,
            } => write!(
                f,
                "the idempotency-authority export covers sequence {image_covered} but \
                 this store already holds authority through {expected_covered}; \
                 restoring it would narrow the dedup contract — restore a current \
                 export, never an older one"
            ),
            Self::SiblingDivergence {
                event_id,
                global_sequence,
            } => write!(
                f,
                "the idempotency-authority export's history at sequence \
                 {global_sequence} (event {event_id}) contradicts this store's \
                 committed history; the export belongs to a diverged sibling fork and \
                 cannot be restored here"
            ),
            Self::EntriesBeyondCoverage {
                covered_global_sequence,
                first_offender_sequence,
            } => write!(
                f,
                "the idempotency-authority export declares coverage through sequence \
                 {covered_global_sequence} but carries a user entry at sequence \
                 {first_offender_sequence} beyond it — no canonical publication \
                 produces this shape; the export is malformed"
            ),
            Self::PendingCompactionUnresolved { marker_path } => write!(
                f,
                "the store directory has an unresolved pending-compaction transaction \
                 ({}); open the store writable to complete recovery before restoring \
                 an idempotency-authority export",
                marker_path.display()
            ),
        }
    }
}

impl IdempotencyRestoreRefusal {
    pub(super) fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Corrupt { kind } => kind.source(),
            Self::FutureVersion { .. }
            | Self::ForeignLineage { .. }
            | Self::StaleRollback { .. }
            | Self::SiblingDivergence { .. }
            | Self::EntriesBeyondCoverage { .. }
            | Self::PendingCompactionUnresolved { .. } => None,
        }
    }
}

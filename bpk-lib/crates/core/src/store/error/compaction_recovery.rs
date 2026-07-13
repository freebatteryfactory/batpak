/// Typed reason a pending-compaction transaction could not be resolved into
/// exactly the old or the new generation (#177/#195). Mirrors
/// [`super::StoreMetaCorruption`]: the recovery direction is decided by the
/// durable `store.meta` commit record, never guessed from file existence, so
/// any undecidable namespace state fails closed here — a partial replacement or
/// a resurrected source must never be silently promoted into accepted history.
#[derive(Debug)]
#[non_exhaustive]
pub enum CompactionRecoveryRefusal {
    /// The pending-compaction marker (`compaction.pending`) is present but
    /// undecodable (bad magic, CRC mismatch, or truncated payload).
    MarkerCorrupt {
        /// Human-readable description of the decode failure.
        detail: String,
    },
    /// The marker declares a schema version newer than this binary supports.
    MarkerFutureVersion {
        /// Marker schema version observed on disk.
        found: u16,
        /// Highest marker schema version this crate understands.
        supported: u16,
    },
    /// The marker's stamped lineage does not match this store's `store.meta`
    /// lineage — a marker transplanted from a different directory.
    MarkerForeignLineage {
        /// Lineage stamped inside the marker.
        marker_lineage: u128,
        /// Lineage recorded in this store's `store.meta`.
        store_lineage: u128,
    },
    /// A versioned marker exists but `store.meta` could not be loaded, so the
    /// commit record that decides the recovery direction is unavailable.
    MetadataUnavailable,
    /// Recovery requires filesystem repair; reopen writable to complete it.
    RepairRequiresWritableOpen,
    /// A source segment named by the marker is recoverable under neither its
    /// original name nor its `.compact-src` staging name.
    SourceMissing {
        /// Segment id that could not be located.
        segment_id: u64,
    },
    /// The transaction committed but the replacement exists under neither the
    /// staged (`.compact-new`) nor the final name.
    ReplacementMissing {
        /// Merged (replacement) segment id.
        merged_id: u64,
    },
    /// Both the final name and the `.compact-src` staging name exist for an
    /// UNCOMMITTED transaction — a namespace state the protocol cannot produce.
    ConflictingNames {
        /// Merged (replacement) segment id.
        merged_id: u64,
    },
    /// A legacy (pre-token) `compaction.pending.json` marker was found. v1
    /// markers are never rolled forward or back automatically — the ordering
    /// that predated the commit token cannot prove old-or-new. Parsed for
    /// diagnostics only: complete the transaction offline under a `≤0.10.x`
    /// binary, or restore from backup.
    LegacyMarkerUnsupported {
        /// Path of the legacy JSON marker (diagnostic only).
        path: std::path::PathBuf,
        /// Merged (replacement) segment id observed in the legacy marker.
        merged_id: u64,
        /// Source segment ids observed in the legacy marker.
        source_segment_ids: Vec<u64>,
    },
    /// A live pending marker was encountered where open-time recovery should
    /// already have resolved it (`compact()` entry, or an internal scan).
    PendingTransactionUnresolved,
}

impl std::fmt::Display for CompactionRecoveryRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MarkerCorrupt { detail } => write!(
                f,
                "pending-compaction marker is present but undecodable ({detail}); \
                 do not delete files by hand — restore this directory from a backup"
            ),
            Self::MarkerFutureVersion { found, supported } => write!(
                f,
                "pending-compaction marker declares schema version {found} but this \
                 binary supports at most {supported}; upgrade batpak to a build that \
                 understands this marker, or restore from backup"
            ),
            Self::MarkerForeignLineage {
                marker_lineage,
                store_lineage,
            } => write!(
                f,
                "pending-compaction marker was written for store lineage \
                 {marker_lineage:032x} but this store is lineage {store_lineage:032x}; \
                 the marker was transplanted from a different directory — resolve it \
                 with an offline migration, never by deleting files by hand"
            ),
            Self::MetadataUnavailable => write!(
                f,
                "a versioned pending-compaction marker exists but store.meta could not \
                 be loaded, so the transaction direction is undecidable; restore \
                 store.meta (or the whole directory) from a backup"
            ),
            Self::RepairRequiresWritableOpen => write!(
                f,
                "resolving the pending compaction requires filesystem repair; reopen \
                 the store writable to complete recovery"
            ),
            Self::SourceMissing { segment_id } => write!(
                f,
                "compaction source segment {segment_id} is recoverable under neither \
                 its original name nor its .compact-src staging name; restore the \
                 directory from a backup"
            ),
            Self::ReplacementMissing { merged_id } => write!(
                f,
                "the compaction committed but replacement segment {merged_id} exists \
                 under neither its staged nor its final name; restore the directory \
                 from a backup"
            ),
            Self::ConflictingNames { merged_id } => write!(
                f,
                "both the final name and the .compact-src staging name exist for \
                 uncommitted merged segment {merged_id} — a namespace state the \
                 protocol cannot produce; restore from backup, do not delete files by hand"
            ),
            Self::LegacyMarkerUnsupported {
                path,
                merged_id,
                source_segment_ids,
            } => write!(
                f,
                "legacy (pre-token) compaction marker at {} (merged_id {merged_id}, \
                 sources {source_segment_ids:?}) is not automatically recoverable; \
                 complete it offline under a ≤0.10.x binary or restore from backup — \
                 this build never rolls a v1 marker forward or back",
                path.display()
            ),
            Self::PendingTransactionUnresolved => write!(
                f,
                "a live pending-compaction marker was encountered where open-time \
                 recovery should already have resolved it; reopen the store to run recovery"
            ),
        }
    }
}

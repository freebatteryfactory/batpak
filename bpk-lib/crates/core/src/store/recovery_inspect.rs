//! Non-opening forensic inspection of a store directory's recovery state (#180,
//! contract A14).
//!
//! [`Store::inspect_recovery_state`] reads the three durable recovery artifacts —
//! the pending-compaction marker, the `store.meta` commit record, and the
//! `store.meta` idempotency-authority authorization — WITHOUT constructing a
//! `Store<Open>` and WITHOUT mutating a single byte on disk. It is the read-only
//! diagnostic door for a directory that will not open (a pending transaction, a
//! corrupt/legacy marker), reporting the same TYPED reasons an open would surface
//! plus the committed generation and marker summary, so an operator can decide
//! whether to reopen writable, migrate offline, or restore from backup.
//!
//! It deliberately does NOT forecast roll-forward vs roll-back. That direction is
//! decided by the three-artifact agreement UNDER the store-dir lock at writable
//! open (`cold_start::rebuild::compaction_recovery`); re-deriving it here would
//! either duplicate the decision machine or, worse, produce an OPTIMISTIC guess
//! that cannot see the file-level refusals (missing source, conflicting names)
//! the real classifier catches. Instead the report states the honest coarse
//! action — `WritableRepairRequired` for a readable marker, `Refuse` for one the
//! load itself rejects — and exposes the raw agreement facts
//! ([`PendingMarkerSummary::committed_by_store_meta`],
//! [`PendingMarkerSummary::lineage_matches_store`]) so the reader can anticipate
//! the direction without this module asserting it. Field types are public-safe:
//! hex id strings, paths, plain scalars, and enums — no internal branded ids or
//! tuples leak across the boundary.

use crate::store::cold_start::rebuild::{load_pending_compaction, PendingCompactionV2};
use crate::store::store_meta::{load_store_meta, StoreMetaData};
use crate::store::{Open, Store, StoreConfig, StoreError};
use std::path::{Path, PathBuf};

/// A read-only snapshot of a store directory's recovery state (contract A14).
///
/// Every id is rendered as a 32-char lowercase hex string; the report carries no
/// branded id types, tuples, or other crate-internal shapes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryInspection {
    /// The inspected directory (the config's data dir, verbatim — never
    /// canonicalized or created, since inspection must not mutate disk).
    pub data_dir: PathBuf,
    /// Whether a `store.meta` sidecar is present and decoded intact. `false` for
    /// an unmigrated legacy directory that never wrote `store.meta`.
    pub store_meta_present: bool,
    /// The store lineage identity from `store.meta` (32-hex), or `None` when
    /// `store.meta` is absent.
    pub store_lineage: Option<String>,
    /// The last durably AUTHORIZED compaction transition recorded in
    /// `store.meta` (the commit record), or `None` before any compaction has
    /// committed.
    pub committed_generation: Option<CommittedGeneration>,
    /// The `store.meta` idempotency-authority authorization descriptor, or
    /// `None` while the store holds no user idempotency obligations.
    pub authorized_image: Option<AuthorizedImage>,
    /// The pending-compaction marker on disk, if a readable canonical marker is
    /// present. A marker the load itself REFUSES (corrupt/future/legacy) is
    /// reported through [`RequiredRecoveryAction::Refuse`] instead, since its
    /// fields cannot be trusted.
    pub pending_marker: Option<PendingMarkerSummary>,
    /// What a writable open would be REQUIRED to do to resolve the on-disk state.
    pub required_action: RequiredRecoveryAction,
}

/// The last durably authorized compaction transition (`store.meta` commit
/// record) — the fact that flips a matching pending marker to roll-forward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedGeneration {
    /// Branded transaction token, 32-hex.
    pub compaction_id: String,
    /// Authority image id this transition published, 32-hex.
    pub authority_image_id: String,
    /// Segment id of the committed replacement segment.
    pub merged_segment_id: u64,
    /// The retired source segment ids, sorted.
    pub source_segment_ids: Vec<u64>,
}

/// The `store.meta` idempotency-authority authorization: which image
/// publication(s) the store's metadata has authorized (admission is by this
/// generation token, never by file presence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedImage {
    /// Generation token of the last FINALIZED image publication (32-hex), or
    /// `None` before the first publication finalizes.
    pub current_image_id: Option<String>,
    /// Generation token of an in-flight publication (32-hex), present only
    /// across the publish crash window; `None` otherwise.
    pub pending_image_id: Option<String>,
    /// Highest global sequence covered by the authorized image (diagnostic
    /// anchor).
    pub covered_global_sequence: u64,
}

/// Summary of a readable, canonical v2 pending-compaction marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingMarkerSummary {
    /// Path of the binary `compaction.pending` marker.
    pub marker_path: PathBuf,
    /// The transaction token stamped in the marker, 32-hex.
    pub compaction_id: String,
    /// The store lineage stamped in the marker, 32-hex.
    pub lineage: String,
    /// The merged (replacement) segment id.
    pub merged_segment_id: u64,
    /// The source segment ids the transaction consumes.
    pub source_segment_ids: Vec<u64>,
    /// The authority image id the marker expects the transition to publish,
    /// 32-hex.
    pub expected_authority_image_id: String,
    /// Whether `store.meta`'s commit record binds THIS transaction: the marker's
    /// `compaction_id` equals the commit's `compaction_id` AND the marker's
    /// merged id equals the commit's `merged_segment_id`. This is the exact
    /// predicate the writable-open classifier uses to choose roll-forward
    /// (`true`) over roll-back (`false`) — reported here as a fact, not a repair
    /// forecast (file-level refusals are only detectable under the lock).
    pub committed_by_store_meta: bool,
    /// Whether the marker's stamped lineage matches `store.meta`'s lineage. A
    /// `false` here is a transplanted marker that a writable open would refuse.
    pub lineage_matches_store: bool,
}

/// The action a writable open is required to take to resolve the inspected
/// directory's recovery state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequiredRecoveryAction {
    /// No pending transaction — the directory opens without recovery.
    NoneRequired,
    /// A readable pending marker exists; a writable open must run under the
    /// store-dir lock to resolve it (roll forward or back per the three-artifact
    /// agreement, or fail closed on an unrecoverable namespace). The direction
    /// is NOT forecast here.
    WritableRepairRequired,
    /// A writable open would REFUSE: the marker itself is unreadable, declares a
    /// future version, or is a legacy (pre-token) marker that is never recovered
    /// automatically. The condition is definite regardless of writability.
    Refuse {
        /// The canonical refusal, rendered public-safe.
        refusal: RecoveryRefusalSummary,
    },
}

/// A public-safe rendering of a compaction-recovery refusal, so inspection does
/// not leak the crate-internal `#[non_exhaustive]` refusal enum across the API
/// boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryRefusalSummary {
    /// The marker path the refusal concerns.
    pub marker_path: PathBuf,
    /// The refusal's canonical operator-facing message (its `Display`), which
    /// already carries the remediation guidance.
    pub message: String,
}

impl Store<Open> {
    /// Inspect a store directory's recovery state WITHOUT opening it and without
    /// mutating disk (contract A14).
    ///
    /// Reads the pending-compaction marker and the `store.meta` commit/authority
    /// descriptors from `config`'s data directory and reports the committed
    /// generation, the marker summary (when readable), and the writable action a
    /// full open would be required to take. This is a forensic door for a
    /// directory that will not open; it takes NO store lock, so a concurrent live
    /// writer can produce a momentarily torn view — treat the report as a
    /// point-in-time diagnostic, not a synchronized read.
    ///
    /// # Errors
    /// Returns the same typed `store.meta` failures a real open would surface
    /// ([`StoreError::StoreMetadataCorrupt`], [`StoreError::StoreMetadataFutureVersion`]),
    /// or [`StoreError::Io`] for a non-refusal filesystem error. A corrupt,
    /// future-version, or legacy MARKER is NOT an error here — it is captured in
    /// [`RequiredRecoveryAction::Refuse`] so the report still renders.
    pub fn inspect_recovery_state(
        config: &StoreConfig,
    ) -> Result<RecoveryInspection, StoreError> {
        inspect_recovery_state_at(&config.data_dir, config.fs().as_ref())
    }
}

/// The lock-free, fs-seam implementation behind [`Store::inspect_recovery_state`]
/// (kept separate so it is unit-testable against any [`StoreFs`] backend).
fn inspect_recovery_state_at(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
) -> Result<RecoveryInspection, StoreError> {
    // A corrupt / future-version store.meta is a hard, typed failure — surface it
    // exactly as an open would rather than paper over the store's identity.
    let meta = load_store_meta(data_dir, fs)?;

    let store_meta_present = meta.is_some();
    let store_lineage = meta.as_ref().map(|meta| hex_u128(meta.lineage));
    let committed_generation = meta
        .as_ref()
        .and_then(|meta| meta.last_compaction_commit.as_ref())
        .map(|commit| CommittedGeneration {
            compaction_id: hex_u128(commit.compaction_id.as_u128()),
            authority_image_id: hex_u128(commit.authority_image_id.as_u128()),
            merged_segment_id: commit.merged_segment_id,
            source_segment_ids: commit.source_segment_ids.clone(),
        });
    let authorized_image = meta
        .as_ref()
        .and_then(|meta| meta.idemp_authority.as_ref())
        .map(|expectation| AuthorizedImage {
            current_image_id: expectation
                .current_image_id
                .map(|id| hex_u128(id.as_u128())),
            pending_image_id: expectation
                .pending_image_id
                .map(|id| hex_u128(id.as_u128())),
            covered_global_sequence: expectation.anchor.covered_global_sequence,
        });

    let (pending_marker, required_action) = match load_pending_compaction(data_dir, fs) {
        Ok(None) => (None, RequiredRecoveryAction::NoneRequired),
        Ok(Some(marker)) => {
            let summary = summarize_marker(data_dir, &marker, meta.as_ref());
            (Some(summary), RequiredRecoveryAction::WritableRepairRequired)
        }
        // A marker the load refuses (corrupt / future / legacy) is exactly what a
        // forensic inspection must REPORT, not bail on. Any other error (real
        // I/O) propagates.
        Err(error) => {
            let StoreError::CompactionRecoveryRefused { marker_path, kind } = &error else {
                return Err(error);
            };
            let refusal = RecoveryRefusalSummary {
                marker_path: marker_path.clone(),
                message: kind.to_string(),
            };
            (None, RequiredRecoveryAction::Refuse { refusal })
        }
    };

    Ok(RecoveryInspection {
        data_dir: data_dir.to_path_buf(),
        store_meta_present,
        store_lineage,
        committed_generation,
        authorized_image,
        pending_marker,
        required_action,
    })
}

/// Build the public-safe marker summary, folding in the two three-artifact
/// agreement facts against `store.meta` (the exact predicates the writable-open
/// classifier uses, reported as facts rather than a direction forecast).
fn summarize_marker(
    data_dir: &Path,
    marker: &PendingCompactionV2,
    meta: Option<&StoreMetaData>,
) -> PendingMarkerSummary {
    let committed_by_store_meta = matches!(
        meta.and_then(|meta| meta.last_compaction_commit.as_ref()),
        Some(commit)
            if commit.compaction_id == marker.compaction_id
                && commit.merged_segment_id == marker.merged_id
    );
    let lineage_matches_store =
        matches!(meta, Some(meta) if meta.lineage == marker.lineage.as_u128());
    PendingMarkerSummary {
        marker_path: data_dir.join(crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME),
        compaction_id: hex_u128(marker.compaction_id.as_u128()),
        lineage: hex_u128(marker.lineage.as_u128()),
        merged_segment_id: marker.merged_id,
        source_segment_ids: marker.source_segment_ids.clone(),
        expected_authority_image_id: hex_u128(marker.expected_authority_image_id.as_u128()),
        committed_by_store_meta,
        lineage_matches_store,
    }
}

/// Render a `u128` id as the store's canonical 32-char lowercase hex string.
fn hex_u128(value: u128) -> String {
    format!("{value:032x}")
}

#[cfg(test)]
mod tests {
    use super::{inspect_recovery_state_at, RequiredRecoveryAction};
    use crate::store::cold_start::rebuild::{write_pending_compaction, PendingCompactionV2};
    use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
    use crate::store::platform::fs::RealFs;
    use crate::store::store_meta::{write_store_meta, CompactionCommit, StoreMetaData};
    use crate::store::{Store, StoreConfig};

    fn meta_with(lineage: u128, commit: Option<CompactionCommit>) -> StoreMetaData {
        StoreMetaData {
            lineage,
            idemp_authority: None,
            last_compaction_commit: commit,
        }
    }

    #[test]
    fn public_associated_fn_inspects_without_opening() {
        // The public door: `Store::inspect_recovery_state` takes a plain config
        // (default RealFs) and never constructs `Store<Open>`. Exercises the
        // associated fn so store_pub_fn_coverage sees a live call site.
        let dir = tempfile::tempdir().expect("tempdir");
        let config = StoreConfig::new(dir.path());
        let report = Store::inspect_recovery_state(&config).expect("public inspection");
        assert_eq!(report.data_dir, dir.path().to_path_buf());
        assert_eq!(report.required_action, RequiredRecoveryAction::NoneRequired);
    }

    #[test]
    fn empty_directory_requires_no_recovery() {
        // A directory with neither store.meta nor a marker is a clean no-op:
        // nothing to recover, and no store.meta means the report says so.
        let dir = tempfile::tempdir().expect("tempdir");
        let report =
            inspect_recovery_state_at(dir.path(), &RealFs).expect("inspection of empty dir");
        assert!(!report.store_meta_present, "empty dir has no store.meta");
        assert_eq!(report.store_lineage, None);
        assert_eq!(report.committed_generation, None);
        assert_eq!(report.pending_marker, None);
        assert_eq!(
            report.required_action,
            RequiredRecoveryAction::NoneRequired,
            "PROPERTY: no marker means no writable action is required"
        );
    }

    #[test]
    fn legacy_json_marker_reports_refuse() {
        // A ≤0.10.x JSON marker is never recovered automatically: inspection
        // captures the load's LegacyMarkerUnsupported refusal as a Refuse action
        // rather than erroring out of the report.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("compaction.pending.json"),
            br#"{"merged_id":7,"source_segment_ids":[1,2,7]}"#,
        )
        .expect("write legacy marker");

        let report =
            inspect_recovery_state_at(dir.path(), &RealFs).expect("inspection still renders");
        assert_eq!(report.pending_marker, None, "a refused marker is not summarized");
        match report.required_action {
            RequiredRecoveryAction::Refuse { refusal } => {
                assert!(
                    refusal.marker_path.ends_with("compaction.pending.json"),
                    "the refusal names the legacy marker path; got {}",
                    refusal.marker_path.display()
                );
                assert!(
                    !refusal.message.is_empty(),
                    "the refusal carries an operator-facing message"
                );
            }
            RequiredRecoveryAction::NoneRequired
            | RequiredRecoveryAction::WritableRepairRequired => assert!(
                std::hint::black_box(false),
                "PROPERTY: a legacy JSON marker must report a Refuse action"
            ),
        }
    }

    #[test]
    fn committed_v2_marker_reports_repair_required_with_agreement_facts() {
        // A readable v2 marker whose token AND merged id match store.meta's commit
        // record: the coarse action is WritableRepairRequired, and the summary
        // carries the exact agreement facts (committed + lineage match) plus hex
        // ids — without this module forecasting the roll direction.
        let dir = tempfile::tempdir().expect("tempdir");
        let compaction_id = CompactionId::from_u128(0x0102_0304);
        let lineage: u128 = 0x0a0b_0c0d;
        let marker = PendingCompactionV2 {
            compaction_id,
            lineage: StoreLineage::from_u128(lineage),
            merged_id: 7,
            source_segment_ids: vec![1, 2, 7],
            expected_authority_image_id: AuthorityImageId::from_u128(0x00ff_00ff),
        };
        write_pending_compaction(dir.path(), &marker, &RealFs).expect("write v2 marker");
        let meta = meta_with(
            lineage,
            Some(CompactionCommit {
                compaction_id,
                authority_image_id: AuthorityImageId::from_u128(0x00ff_00ff),
                merged_segment_id: 7,
                source_segment_ids: vec![1, 2, 7],
            }),
        );
        write_store_meta(dir.path(), &meta, &RealFs).expect("write store.meta");

        let report = inspect_recovery_state_at(dir.path(), &RealFs).expect("inspection");
        assert_eq!(
            report.required_action,
            RequiredRecoveryAction::WritableRepairRequired,
            "PROPERTY: a readable marker requires a writable repair"
        );
        let summary = report
            .pending_marker
            .expect("PROPERTY: a readable v2 marker is summarized");
        assert!(
            summary.committed_by_store_meta,
            "PROPERTY: a matching token+merged id is reported as committed"
        );
        assert!(
            summary.lineage_matches_store,
            "PROPERTY: a matching lineage is reported as such"
        );
        assert_eq!(summary.compaction_id, format!("{:032x}", 0x0102_0304_u128));
        assert_eq!(summary.compaction_id.len(), 32, "ids render as fixed 32-hex");
        assert_eq!(summary.merged_segment_id, 7);
        assert_eq!(summary.source_segment_ids, vec![1, 2, 7]);
        let committed = report
            .committed_generation
            .expect("PROPERTY: store.meta's commit record is reported");
        assert_eq!(committed.compaction_id, summary.compaction_id);
        assert_eq!(committed.merged_segment_id, 7);
    }

    #[test]
    fn v2_marker_without_matching_commit_reports_uncommitted() {
        // A readable marker whose token does NOT match store.meta's commit is an
        // uncommitted transaction: committed_by_store_meta is false, which is the
        // exact fact a reader uses to anticipate a roll-back.
        let dir = tempfile::tempdir().expect("tempdir");
        let lineage: u128 = 0x55;
        let marker = PendingCompactionV2 {
            compaction_id: CompactionId::from_u128(0x1111),
            lineage: StoreLineage::from_u128(lineage),
            merged_id: 9,
            source_segment_ids: vec![3, 9],
            expected_authority_image_id: AuthorityImageId::from_u128(0x22),
        };
        write_pending_compaction(dir.path(), &marker, &RealFs).expect("write v2 marker");
        // Commit record for a DIFFERENT transaction token.
        let meta = meta_with(
            lineage,
            Some(CompactionCommit {
                compaction_id: CompactionId::from_u128(0x9999),
                authority_image_id: AuthorityImageId::from_u128(0x22),
                merged_segment_id: 9,
                source_segment_ids: vec![3, 9],
            }),
        );
        write_store_meta(dir.path(), &meta, &RealFs).expect("write store.meta");

        let report = inspect_recovery_state_at(dir.path(), &RealFs).expect("inspection");
        let summary = report.pending_marker.expect("readable marker summarized");
        assert!(
            !summary.committed_by_store_meta,
            "PROPERTY: a non-matching commit token is reported as uncommitted"
        );
        assert!(summary.lineage_matches_store, "same lineage still matches");
    }
}

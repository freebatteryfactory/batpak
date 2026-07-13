//! Published, case-addressable `StoreFs` conformance corpus.
//!
//! This is the SAME body that gates `RealFs` and `MemFs` in-tree, lifted so an
//! out-of-tree backend can run it through the public seam (`&dyn StoreFs` + the
//! published handle traits only — no crate-private shortcuts). Every obligation
//! is a stable, kebab-addressed case; a per-case fresh backend factory keeps
//! runs order-independent and safe to shard; a typed [`Qualification`] records a
//! skip so a SKIP IS NEVER A PASS; the machine-readable [`ConformanceReport`]
//! serializes the verdicts and proves planted faults actually fire.
//!
//! Versioning: bump [`CORPUS_VERSION`] whenever any case obligation changes or a
//! variant is added to [`StoreFsConformanceCase`] — the case count is pinned by
//! a test so the bump is deliberate.
//!
//! Backends: `RealFs` and `MemFs` carry no crash controls and answer the Crash
//! family with a typed qualification; `ShadowFs` is the namespace-truth crash
//! reference and supplies real [`HostileControls`]. The corpus NEVER
//! asserts/panics — case bodies return typed [`CaseResult`]s via [`check`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::store::sim::fs::CrashOp;
use crate::store::sim::ShadowFs;
use crate::store::{MemFs, RealFs, StoreFs};

mod cases_crash;
mod cases_handles;
mod cases_lock_lifecycle;
mod cases_publish;

/// Corpus obligation version. Bump on any obligation/case-set change.
pub const CORPUS_VERSION: u32 = 1;

/// One stable, addressable backend obligation. Kebab ids are the wire identity
/// ([`Self::id`]/[`Self::parse`]); the enum is `#[non_exhaustive]` so downstream
/// consumers must tolerate future cases.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum StoreFsConformanceCase {
    // Handles (7)
    CreateNewExclusive,
    HandleLengthReflectsSyncedBytes,
    PositionedReadExact,
    PositionedShortReadTyped,
    WholeFileReadRoutesThroughSeam,
    MetadataOwnedTypes,
    ReadDirOwnedKinds,
    // Publish (16)
    UnpersistedStageLeavesNoFinalName,
    StagedPublishAtomicFreshPath,
    StagedPublishOntoDirectoryFailsClosed,
    StagedPublishMissingParentFailsClosed,
    CopyReportsStrategyHonestly,
    SamePathCopyNonDestructive,
    SamePathRenameNonDestructive,
    RenameOntoDirectoryFailsClosedSourceIntact,
    RenameMissingParentFailsClosedSourceIntact,
    CopyOntoDirectoryFailsClosed,
    CopyMissingParentFailsClosed,
    RemoveFileIfPresentHonest,
    RemoveFileOnDirectoryErrors,
    ReadDirOnFileFailsClosedNotNotFound,
    ReadOnDirectoryFailsClosedNotNotFound,
    RemoveDirAllClearsThroughSeam,
    // LockLifecycle (3)
    LockExcludesSecondOwner,
    LockReleasesOnDrop,
    DeleteThenRecreateStartsEmpty,
    // Crash (8)
    CrashPreservesSyncedBytesAndDurableName,
    CrashDropsUnsyncedTail,
    UnsyncedNameMayVanishNeverTorn,
    RenameWithoutParentSyncCrashOldOrNew,
    RemoveWithoutParentSyncCrashMayResurrect,
    DurableRemoveNeverResurrects,
    PersistWithoutParentSyncCrashOldOrNew,
    PlantedFaultActuallyFires,
}

impl StoreFsConformanceCase {
    /// Every case in stable order. Length is pinned by a test; extend with a
    /// [`CORPUS_VERSION`] bump.
    pub const ALL: [StoreFsConformanceCase; 34] = [
        StoreFsConformanceCase::CreateNewExclusive,
        StoreFsConformanceCase::HandleLengthReflectsSyncedBytes,
        StoreFsConformanceCase::PositionedReadExact,
        StoreFsConformanceCase::PositionedShortReadTyped,
        StoreFsConformanceCase::WholeFileReadRoutesThroughSeam,
        StoreFsConformanceCase::MetadataOwnedTypes,
        StoreFsConformanceCase::ReadDirOwnedKinds,
        StoreFsConformanceCase::UnpersistedStageLeavesNoFinalName,
        StoreFsConformanceCase::StagedPublishAtomicFreshPath,
        StoreFsConformanceCase::StagedPublishOntoDirectoryFailsClosed,
        StoreFsConformanceCase::StagedPublishMissingParentFailsClosed,
        StoreFsConformanceCase::CopyReportsStrategyHonestly,
        StoreFsConformanceCase::SamePathCopyNonDestructive,
        StoreFsConformanceCase::SamePathRenameNonDestructive,
        StoreFsConformanceCase::RenameOntoDirectoryFailsClosedSourceIntact,
        StoreFsConformanceCase::RenameMissingParentFailsClosedSourceIntact,
        StoreFsConformanceCase::CopyOntoDirectoryFailsClosed,
        StoreFsConformanceCase::CopyMissingParentFailsClosed,
        StoreFsConformanceCase::RemoveFileIfPresentHonest,
        StoreFsConformanceCase::RemoveFileOnDirectoryErrors,
        StoreFsConformanceCase::ReadDirOnFileFailsClosedNotNotFound,
        StoreFsConformanceCase::ReadOnDirectoryFailsClosedNotNotFound,
        StoreFsConformanceCase::RemoveDirAllClearsThroughSeam,
        StoreFsConformanceCase::LockExcludesSecondOwner,
        StoreFsConformanceCase::LockReleasesOnDrop,
        StoreFsConformanceCase::DeleteThenRecreateStartsEmpty,
        StoreFsConformanceCase::CrashPreservesSyncedBytesAndDurableName,
        StoreFsConformanceCase::CrashDropsUnsyncedTail,
        StoreFsConformanceCase::UnsyncedNameMayVanishNeverTorn,
        StoreFsConformanceCase::RenameWithoutParentSyncCrashOldOrNew,
        StoreFsConformanceCase::RemoveWithoutParentSyncCrashMayResurrect,
        StoreFsConformanceCase::DurableRemoveNeverResurrects,
        StoreFsConformanceCase::PersistWithoutParentSyncCrashOldOrNew,
        StoreFsConformanceCase::PlantedFaultActuallyFires,
    ];

    /// Stable kebab-case identity. Inverse of [`Self::parse`].
    pub fn id(self) -> &'static str {
        match self {
            StoreFsConformanceCase::CreateNewExclusive => "create-new-exclusive",
            StoreFsConformanceCase::HandleLengthReflectsSyncedBytes => {
                "handle-length-reflects-synced-bytes"
            }
            StoreFsConformanceCase::PositionedReadExact => "positioned-read-exact",
            StoreFsConformanceCase::PositionedShortReadTyped => "positioned-short-read-typed",
            StoreFsConformanceCase::WholeFileReadRoutesThroughSeam => {
                "whole-file-read-routes-through-seam"
            }
            StoreFsConformanceCase::MetadataOwnedTypes => "metadata-owned-types",
            StoreFsConformanceCase::ReadDirOwnedKinds => "read-dir-owned-kinds",
            StoreFsConformanceCase::UnpersistedStageLeavesNoFinalName => {
                "unpersisted-stage-leaves-no-final-name"
            }
            StoreFsConformanceCase::StagedPublishAtomicFreshPath => "staged-publish-atomic-fresh-path",
            StoreFsConformanceCase::StagedPublishOntoDirectoryFailsClosed => {
                "staged-publish-onto-directory-fails-closed"
            }
            StoreFsConformanceCase::StagedPublishMissingParentFailsClosed => {
                "staged-publish-missing-parent-fails-closed"
            }
            StoreFsConformanceCase::CopyReportsStrategyHonestly => "copy-reports-strategy-honestly",
            StoreFsConformanceCase::SamePathCopyNonDestructive => "same-path-copy-non-destructive",
            StoreFsConformanceCase::SamePathRenameNonDestructive => "same-path-rename-non-destructive",
            StoreFsConformanceCase::RenameOntoDirectoryFailsClosedSourceIntact => {
                "rename-onto-directory-fails-closed-source-intact"
            }
            StoreFsConformanceCase::RenameMissingParentFailsClosedSourceIntact => {
                "rename-missing-parent-fails-closed-source-intact"
            }
            StoreFsConformanceCase::CopyOntoDirectoryFailsClosed => "copy-onto-directory-fails-closed",
            StoreFsConformanceCase::CopyMissingParentFailsClosed => "copy-missing-parent-fails-closed",
            StoreFsConformanceCase::RemoveFileIfPresentHonest => "remove-file-if-present-honest",
            StoreFsConformanceCase::RemoveFileOnDirectoryErrors => "remove-file-on-directory-errors",
            StoreFsConformanceCase::ReadDirOnFileFailsClosedNotNotFound => {
                "read-dir-on-file-fails-closed-not-not-found"
            }
            StoreFsConformanceCase::ReadOnDirectoryFailsClosedNotNotFound => {
                "read-on-directory-fails-closed-not-not-found"
            }
            StoreFsConformanceCase::RemoveDirAllClearsThroughSeam => "remove-dir-all-clears-through-seam",
            StoreFsConformanceCase::LockExcludesSecondOwner => "lock-excludes-second-owner",
            StoreFsConformanceCase::LockReleasesOnDrop => "lock-releases-on-drop",
            StoreFsConformanceCase::DeleteThenRecreateStartsEmpty => "delete-then-recreate-starts-empty",
            StoreFsConformanceCase::CrashPreservesSyncedBytesAndDurableName => {
                "crash-preserves-synced-bytes-and-durable-name"
            }
            StoreFsConformanceCase::CrashDropsUnsyncedTail => "crash-drops-unsynced-tail",
            StoreFsConformanceCase::UnsyncedNameMayVanishNeverTorn => {
                "unsynced-name-may-vanish-never-torn"
            }
            StoreFsConformanceCase::RenameWithoutParentSyncCrashOldOrNew => {
                "rename-without-parent-sync-crash-old-or-new"
            }
            StoreFsConformanceCase::RemoveWithoutParentSyncCrashMayResurrect => {
                "remove-without-parent-sync-crash-may-resurrect"
            }
            StoreFsConformanceCase::DurableRemoveNeverResurrects => "durable-remove-never-resurrects",
            StoreFsConformanceCase::PersistWithoutParentSyncCrashOldOrNew => {
                "persist-without-parent-sync-crash-old-or-new"
            }
            StoreFsConformanceCase::PlantedFaultActuallyFires => "planted-fault-actually-fires",
        }
    }

    /// Parse a stable kebab id back to its case. Inverse of [`Self::id`].
    pub fn parse(raw: &str) -> Option<Self> {
        StoreFsConformanceCase::ALL
            .into_iter()
            .find(|case| case.id() == raw)
    }

    /// The family this case belongs to (report grouping + crash-control gating).
    pub fn family(self) -> CaseFamily {
        match self {
            StoreFsConformanceCase::CreateNewExclusive
            | StoreFsConformanceCase::HandleLengthReflectsSyncedBytes
            | StoreFsConformanceCase::PositionedReadExact
            | StoreFsConformanceCase::PositionedShortReadTyped
            | StoreFsConformanceCase::WholeFileReadRoutesThroughSeam
            | StoreFsConformanceCase::MetadataOwnedTypes
            | StoreFsConformanceCase::ReadDirOwnedKinds => CaseFamily::Handles,
            StoreFsConformanceCase::UnpersistedStageLeavesNoFinalName
            | StoreFsConformanceCase::StagedPublishAtomicFreshPath
            | StoreFsConformanceCase::StagedPublishOntoDirectoryFailsClosed
            | StoreFsConformanceCase::StagedPublishMissingParentFailsClosed
            | StoreFsConformanceCase::CopyReportsStrategyHonestly
            | StoreFsConformanceCase::SamePathCopyNonDestructive
            | StoreFsConformanceCase::SamePathRenameNonDestructive
            | StoreFsConformanceCase::RenameOntoDirectoryFailsClosedSourceIntact
            | StoreFsConformanceCase::RenameMissingParentFailsClosedSourceIntact
            | StoreFsConformanceCase::CopyOntoDirectoryFailsClosed
            | StoreFsConformanceCase::CopyMissingParentFailsClosed
            | StoreFsConformanceCase::RemoveFileIfPresentHonest
            | StoreFsConformanceCase::RemoveFileOnDirectoryErrors
            | StoreFsConformanceCase::ReadDirOnFileFailsClosedNotNotFound
            | StoreFsConformanceCase::ReadOnDirectoryFailsClosedNotNotFound
            | StoreFsConformanceCase::RemoveDirAllClearsThroughSeam => CaseFamily::Publish,
            StoreFsConformanceCase::LockExcludesSecondOwner
            | StoreFsConformanceCase::LockReleasesOnDrop
            | StoreFsConformanceCase::DeleteThenRecreateStartsEmpty => CaseFamily::LockLifecycle,
            StoreFsConformanceCase::CrashPreservesSyncedBytesAndDurableName
            | StoreFsConformanceCase::CrashDropsUnsyncedTail
            | StoreFsConformanceCase::UnsyncedNameMayVanishNeverTorn
            | StoreFsConformanceCase::RenameWithoutParentSyncCrashOldOrNew
            | StoreFsConformanceCase::RemoveWithoutParentSyncCrashMayResurrect
            | StoreFsConformanceCase::DurableRemoveNeverResurrects
            | StoreFsConformanceCase::PersistWithoutParentSyncCrashOldOrNew
            | StoreFsConformanceCase::PlantedFaultActuallyFires => CaseFamily::Crash,
        }
    }
}

/// Case families — grouping + which cases require crash [`HostileControls`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CaseFamily {
    Handles,
    Publish,
    LockLifecycle,
    Crash,
}

/// A typed skip. A qualification records that a backend cannot exercise a
/// control — it is NEVER a pass (see [`ConformanceReport::full_pass`]).
#[non_exhaustive]
#[derive(Clone, Debug, serde::Serialize)]
pub enum Qualification {
    ControlUnsupported {
        control: &'static str,
        backend: String,
        reason: String,
    },
}

/// Case outcome inside the corpus: a violation string or a typed qualification.
/// The corpus never asserts/panics — bodies fail through this type only.
pub(crate) enum CaseFailure {
    Violation(String),
    Qualified(Qualification),
}

/// The result a case body returns; `Ok(())` means the obligation held.
pub(crate) type CaseResult = Result<(), CaseFailure>;

/// Typed check — the corpus's only failure primitive. `false` becomes a typed
/// violation instead of a panic (this module is production lint domain).
pub(crate) fn check(cond: bool, msg: impl Into<String>) -> CaseResult {
    if cond {
        Ok(())
    } else {
        Err(CaseFailure::Violation(msg.into()))
    }
}

/// Crash-edition controls a backend may expose. A backend WITHOUT a control
/// returns the typed qualification itself — there is no silent default-skip.
pub trait HostileControls: Send + Sync {
    /// Simulate power loss: visible namespace collapses to the durable one.
    fn crash(&self) -> Result<(), Qualification>;
    /// Silently drop the next parent-dir sync (lying disk on the name axis).
    fn drop_next_parent_dir_sync(&self) -> Result<(), Qualification>;
    /// Inject a torn-atomic error on the next occurrence of `op`.
    fn fault_next_op(&self, op: CrashOp) -> Result<(), Qualification>;
    /// Anti-vacuity witness: how many armed faults actually fired.
    fn faults_fired(&self) -> u32;
}

/// A fresh, isolated backend for exactly one case. `keepalive` holds owners
/// (e.g. a `TempDir`) whose lifetime must span the case.
pub struct FreshBackend {
    pub fs: Arc<dyn StoreFs>,
    pub root: PathBuf,
    pub controls: Option<Arc<dyn HostileControls>>,
    pub keepalive: Option<Box<dyn std::any::Any + Send>>,
}

/// Mints a FRESH backend per case — order-independent, shardable (the Duroxide
/// mechanic: a case never inherits another case's tree).
pub trait BackendFactory {
    fn backend_name(&self) -> String;
    fn backend_fingerprint(&self) -> Option<String> {
        None
    }
    fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification>;
}

/// Borrowed context handed to each case body.
pub(crate) struct CaseCx<'a> {
    pub fs: &'a dyn StoreFs,
    pub root: &'a Path,
    pub controls: Option<&'a dyn HostileControls>,
}

/// Per-case verdict. `Qualified` is distinct from `Passed` by construction.
#[derive(Clone, Debug, serde::Serialize)]
pub enum CaseVerdict {
    Passed,
    Failed { reason: String },
    Qualified(Qualification),
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CaseOutcome {
    pub case: StoreFsConformanceCase,
    pub verdict: CaseVerdict,
}

/// Machine-readable run report — the published artifact a backend author emits.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ConformanceReport {
    pub corpus_version: u32,
    pub batpak_version: String,
    pub backend_name: String,
    pub backend_fingerprint: Option<String>,
    pub outcomes: Vec<CaseOutcome>,
}

impl ConformanceReport {
    /// True only when EVERY case passed — no `Failed` AND no `Qualified`.
    pub fn full_pass(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| matches!(outcome.verdict, CaseVerdict::Passed))
    }

    pub fn failures(&self) -> Vec<&CaseOutcome> {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome.verdict, CaseVerdict::Failed { .. }))
            .collect()
    }

    pub fn qualified(&self) -> Vec<&CaseOutcome> {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome.verdict, CaseVerdict::Qualified(_)))
            .collect()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Run one case against a freshly minted backend and map the typed result to a
/// verdict. A factory qualification (cannot even set up) surfaces as `Qualified`.
pub fn run_case(
    factory: &dyn BackendFactory,
    case: StoreFsConformanceCase,
) -> CaseOutcome {
    let backend = match factory.fresh(case) {
        Ok(backend) => backend,
        Err(qualification) => {
            return CaseOutcome {
                case,
                verdict: CaseVerdict::Qualified(qualification),
            }
        }
    };
    let cx = CaseCx {
        fs: backend.fs.as_ref(),
        root: backend.root.as_path(),
        controls: backend.controls.as_deref(),
    };
    let verdict = match dispatch(case, &cx) {
        Ok(()) => CaseVerdict::Passed,
        Err(CaseFailure::Violation(reason)) => CaseVerdict::Failed { reason },
        Err(CaseFailure::Qualified(qualification)) => CaseVerdict::Qualified(qualification),
    };
    CaseOutcome { case, verdict }
}

pub fn run_cases(
    factory: &dyn BackendFactory,
    cases: &[StoreFsConformanceCase],
) -> ConformanceReport {
    let outcomes = cases
        .iter()
        .map(|&case| run_case(factory, case))
        .collect();
    ConformanceReport {
        corpus_version: CORPUS_VERSION,
        batpak_version: env!("CARGO_PKG_VERSION").to_string(),
        backend_name: factory.backend_name(),
        backend_fingerprint: factory.backend_fingerprint(),
        outcomes,
    }
}

pub fn run_all(factory: &dyn BackendFactory) -> ConformanceReport {
    run_cases(factory, &StoreFsConformanceCase::ALL)
}

/// Exhaustive case → body dispatch. A new variant forces a new arm here.
fn dispatch(case: StoreFsConformanceCase, cx: &CaseCx<'_>) -> CaseResult {
    match case {
        StoreFsConformanceCase::CreateNewExclusive => cases_handles::create_new_exclusive(cx),
        StoreFsConformanceCase::HandleLengthReflectsSyncedBytes => {
            cases_handles::handle_length_reflects_synced_bytes(cx)
        }
        StoreFsConformanceCase::PositionedReadExact => cases_handles::positioned_read_exact(cx),
        StoreFsConformanceCase::PositionedShortReadTyped => {
            cases_handles::positioned_short_read_typed(cx)
        }
        StoreFsConformanceCase::WholeFileReadRoutesThroughSeam => {
            cases_handles::whole_file_read_routes_through_seam(cx)
        }
        StoreFsConformanceCase::MetadataOwnedTypes => cases_handles::metadata_owned_types(cx),
        StoreFsConformanceCase::ReadDirOwnedKinds => cases_handles::read_dir_owned_kinds(cx),
        StoreFsConformanceCase::UnpersistedStageLeavesNoFinalName => {
            cases_publish::unpersisted_stage_leaves_no_final_name(cx)
        }
        StoreFsConformanceCase::StagedPublishAtomicFreshPath => {
            cases_publish::staged_publish_atomic_fresh_path(cx)
        }
        StoreFsConformanceCase::StagedPublishOntoDirectoryFailsClosed => {
            cases_publish::staged_publish_onto_directory_fails_closed(cx)
        }
        StoreFsConformanceCase::StagedPublishMissingParentFailsClosed => {
            cases_publish::staged_publish_missing_parent_fails_closed(cx)
        }
        StoreFsConformanceCase::CopyReportsStrategyHonestly => {
            cases_publish::copy_reports_strategy_honestly(cx)
        }
        StoreFsConformanceCase::SamePathCopyNonDestructive => {
            cases_publish::same_path_copy_non_destructive(cx)
        }
        StoreFsConformanceCase::SamePathRenameNonDestructive => {
            cases_publish::same_path_rename_non_destructive(cx)
        }
        StoreFsConformanceCase::RenameOntoDirectoryFailsClosedSourceIntact => {
            cases_publish::rename_onto_directory_fails_closed_source_intact(cx)
        }
        StoreFsConformanceCase::RenameMissingParentFailsClosedSourceIntact => {
            cases_publish::rename_missing_parent_fails_closed_source_intact(cx)
        }
        StoreFsConformanceCase::CopyOntoDirectoryFailsClosed => {
            cases_publish::copy_onto_directory_fails_closed(cx)
        }
        StoreFsConformanceCase::CopyMissingParentFailsClosed => {
            cases_publish::copy_missing_parent_fails_closed(cx)
        }
        StoreFsConformanceCase::RemoveFileIfPresentHonest => {
            cases_publish::remove_file_if_present_honest(cx)
        }
        StoreFsConformanceCase::RemoveFileOnDirectoryErrors => {
            cases_publish::remove_file_on_directory_errors(cx)
        }
        StoreFsConformanceCase::ReadDirOnFileFailsClosedNotNotFound => {
            cases_publish::read_dir_on_file_fails_closed_not_not_found(cx)
        }
        StoreFsConformanceCase::ReadOnDirectoryFailsClosedNotNotFound => {
            cases_publish::read_on_directory_fails_closed_not_not_found(cx)
        }
        StoreFsConformanceCase::RemoveDirAllClearsThroughSeam => {
            cases_publish::remove_dir_all_clears_through_seam(cx)
        }
        StoreFsConformanceCase::LockExcludesSecondOwner => {
            cases_lock_lifecycle::lock_excludes_second_owner(cx)
        }
        StoreFsConformanceCase::LockReleasesOnDrop => cases_lock_lifecycle::lock_releases_on_drop(cx),
        StoreFsConformanceCase::DeleteThenRecreateStartsEmpty => {
            cases_lock_lifecycle::delete_then_recreate_starts_empty(cx)
        }
        StoreFsConformanceCase::CrashPreservesSyncedBytesAndDurableName => {
            cases_crash::crash_preserves_synced_bytes_and_durable_name(cx)
        }
        StoreFsConformanceCase::CrashDropsUnsyncedTail => cases_crash::crash_drops_unsynced_tail(cx),
        StoreFsConformanceCase::UnsyncedNameMayVanishNeverTorn => {
            cases_crash::unsynced_name_may_vanish_never_torn(cx)
        }
        StoreFsConformanceCase::RenameWithoutParentSyncCrashOldOrNew => {
            cases_crash::rename_without_parent_sync_crash_old_or_new(cx)
        }
        StoreFsConformanceCase::RemoveWithoutParentSyncCrashMayResurrect => {
            cases_crash::remove_without_parent_sync_crash_may_resurrect(cx)
        }
        StoreFsConformanceCase::DurableRemoveNeverResurrects => {
            cases_crash::durable_remove_never_resurrects(cx)
        }
        StoreFsConformanceCase::PersistWithoutParentSyncCrashOldOrNew => {
            cases_crash::persist_without_parent_sync_crash_old_or_new(cx)
        }
        StoreFsConformanceCase::PlantedFaultActuallyFires => {
            cases_crash::planted_fault_actually_fires(cx)
        }
    }
}

/// A backend that failed even to set up its root is unsupported HERE — a typed
/// qualification (never a pass), the only typed channel `fresh` exposes.
fn setup_unsupported(backend: &str, error: std::io::Error) -> Qualification {
    Qualification::ControlUnsupported {
        control: "backend-setup",
        backend: backend.to_string(),
        reason: error.to_string(),
    }
}

/// Virtual `MemFs` backend, fresh tree per case; no crash controls.
pub struct MemFsFactory;

impl BackendFactory for MemFsFactory {
    fn backend_name(&self) -> String {
        "mem-fs".to_string()
    }

    fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
        let fs = MemFs::new();
        let root = PathBuf::from(format!("/conformance/{}", case.id()));
        fs.create_dir_all(&root)
            .map_err(|error| setup_unsupported("mem-fs", error))?;
        Ok(FreshBackend {
            fs: Arc::new(fs),
            root,
            controls: None,
            keepalive: None,
        })
    }
}

/// Namespace-truth `ShadowFs` backend — the crash-envelope reference; supplies
/// real [`HostileControls`] (the fs itself, tree-shared with the driver clone).
pub struct ShadowFsFactory;

impl BackendFactory for ShadowFsFactory {
    fn backend_name(&self) -> String {
        "shadow-fs".to_string()
    }

    fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
        let shadow = ShadowFs::new();
        let root = PathBuf::from(format!("/conformance/{}", case.id()));
        shadow
            .create_dir_all(&root)
            .map_err(|error| setup_unsupported("shadow-fs", error))?;
        let controls: Arc<dyn HostileControls> = Arc::new(shadow.clone());
        Ok(FreshBackend {
            fs: Arc::new(shadow),
            root,
            controls: Some(controls),
            keepalive: None,
        })
    }
}

/// Host `RealFs` backend over a per-case `tempfile::tempdir()`; no crash
/// controls (crash cases qualify). The `TempDir` lives in `keepalive`.
pub struct RealFsFactory;

impl BackendFactory for RealFsFactory {
    fn backend_name(&self) -> String {
        "real-fs".to_string()
    }

    fn fresh(&self, _case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
        let dir = tempfile::tempdir().map_err(|error| setup_unsupported("real-fs", error))?;
        let root = dir.path().to_path_buf();
        Ok(FreshBackend {
            fs: Arc::new(RealFs),
            root,
            controls: None,
            keepalive: Some(Box::new(dir)),
        })
    }
}

/// `ShadowFs` is the reference crash backend — arming is relative to "now" so a
/// case always targets the NEXT occurrence (H4: persist consumes a parent-sync
/// occurrence too). Lives here to keep `sim/` free of conformance vocabulary.
impl HostileControls for ShadowFs {
    fn crash(&self) -> Result<(), Qualification> {
        ShadowFs::crash(self);
        Ok(())
    }

    fn drop_next_parent_dir_sync(&self) -> Result<(), Qualification> {
        self.arm_parent_sync_drop(self.parent_syncs_observed() + 1);
        Ok(())
    }

    fn fault_next_op(&self, op: CrashOp) -> Result<(), Qualification> {
        self.arm_op_error(op, self.ops_observed(op) + 1);
        Ok(())
    }

    fn faults_fired(&self) -> u32 {
        self.armed_faults_fired()
    }
}

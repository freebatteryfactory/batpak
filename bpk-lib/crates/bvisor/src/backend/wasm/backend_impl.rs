//! [`WasmBackend`] real WASI confinement.
//!
//! The backend lowers an admitted [`BoundaryPlan`] into a wasmi +
//! `wasi_snapshot_preview1` instance. Filesystem authority is expressed only as WASI
//! preopens, environment is built from the explicit contract policy, stdio is captured
//! through in-memory WASI streams, and network is denied by never installing socket
//! capabilities. Unsupported native controls fail closed before a guest is run.

#[path = "backend_impl_budget.rs"]
mod budget_profile;
#[path = "plan_build.rs"]
mod plan_build;
#[path = "poll_once.rs"]
mod poll_once;
#[path = "backend_impl_report.rs"]
mod report_mapping;
#[path = "run.rs"]
mod run;
#[path = "warm.rs"]
mod warm;

#[cfg(feature = "dangerous-test-hooks")]
#[path = "backend_impl_proof.rs"]
mod proof;

use crate::contract::backend::Backend;
use crate::contract::capability::{Enforcement, EvidenceClaim, EvidenceSet, SupportVerdict};
use crate::contract::ids::BackendId;
use crate::contract::plan::{BoundaryPlan, BoundaryRequirement};
use crate::contract::report::{BoundaryReportBody, ObservedFact};
use crate::contract::secret::{MapSecretResolver, SecretResolver};
use crate::contract::support::{
    BackendProfile, BackendProfileSnapshot, RequirementKind, SupportMatrix,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The Wasm boundary backend: wasmi + WASI preopen confinement.
pub struct WasmBackend {
    id: BackendId,
    support: SupportMatrix,
    secret_resolver: Arc<dyn SecretResolver + Send + Sync>,
    /// Shared engine + content-addressed compiled-module cache (Phase 1), warmed
    /// once and reused across every `execute()` so repeated runs of the same guest
    /// skip the dominant `Module::new` compile cost.
    warm: warm::WarmCache,
}

impl WasmBackend {
    /// The stable id of the wasm backend.
    pub const ID: &'static str = "wasm";

    /// Construct the wasm backend with its honest support matrix.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: BackendId::new(Self::ID),
            support: super::support_matrix(),
            secret_resolver: default_secret_resolver(),
            warm: warm::WarmCache::new(),
        }
    }

    /// Construct the backend with an explicit secret resolver used for
    /// `Environment::Exact` `SecretLease` lowering.
    #[must_use]
    pub fn with_secret_resolver(resolver: Arc<dyn SecretResolver + Send + Sync>) -> Self {
        Self {
            id: BackendId::new(Self::ID),
            support: super::support_matrix(),
            secret_resolver: resolver,
            warm: warm::WarmCache::new(),
        }
    }

    /// The production ceiling advertised when the wasmi runtime is present.
    #[must_use]
    pub(crate) fn ceiling(&self) -> BackendProfile {
        let mut ceiling = BTreeMap::new();
        insert(
            &mut ceiling,
            RequirementKind::LaunchWorkload,
            Enforcement::Enforced,
            &[EvidenceClaim::TerminalOutcome],
        );
        insert(
            &mut ceiling,
            RequirementKind::CaptureStreams,
            Enforcement::Enforced,
            &[EvidenceClaim::CapturedStreams],
        );
        insert(
            &mut ceiling,
            RequirementKind::Filesystem,
            Enforcement::Enforced,
            &[
                EvidenceClaim::AllowedActions,
                EvidenceClaim::FilesystemDelta,
                EvidenceClaim::MechanismAttestation,
            ],
        );
        insert(
            &mut ceiling,
            RequirementKind::Environment,
            Enforcement::Enforced,
            &[EvidenceClaim::MechanismAttestation],
        );
        insert(
            &mut ceiling,
            RequirementKind::TempRoot,
            Enforcement::Enforced,
            &[EvidenceClaim::MechanismAttestation],
        );
        insert(
            &mut ceiling,
            RequirementKind::CommitArtifact,
            Enforcement::Enforced,
            &[EvidenceClaim::ArtifactLineage],
        );
        insert(
            &mut ceiling,
            RequirementKind::DiscardArtifact,
            Enforcement::Enforced,
            &[EvidenceClaim::ArtifactLineage],
        );
        insert(
            &mut ceiling,
            RequirementKind::ListOutputs,
            Enforcement::Enforced,
            &[EvidenceClaim::ArtifactLineage],
        );
        insert(
            &mut ceiling,
            RequirementKind::NetworkDenyAll,
            Enforcement::Enforced,
            &[EvidenceClaim::DeniedAttempts],
        );
        insert(
            &mut ceiling,
            RequirementKind::InheritedFdsNone,
            Enforcement::Enforced,
            &[EvidenceClaim::MechanismAttestation],
        );
        BackendProfile::from_ceiling(ceiling)
    }
}

impl Default for WasmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for WasmBackend {
    fn id(&self) -> BackendId {
        self.id.clone()
    }

    fn support(&self) -> &SupportMatrix {
        &self.support
    }

    fn probe(&self) -> BackendProfileSnapshot {
        let mut probed = BTreeMap::new();
        probed.insert("runtime".to_string(), "wasmi".to_string());
        probed.insert("wasi".to_string(), "wasi_snapshot_preview1".to_string());
        probed.insert("filesystem".to_string(), "wasi_preopen".to_string());
        probed.insert("network".to_string(), "no_socket_cap".to_string());
        BackendProfileSnapshot {
            backend: self.id.clone(),
            probed,
            budget: budget_profile::observed_budget_profile(),
        }
    }

    fn profile(&self, _snap: &BackendProfileSnapshot) -> BackendProfile {
        self.ceiling()
    }

    fn classify(&self, req: &BoundaryRequirement, profile: &BackendProfile) -> SupportVerdict {
        self.support.classify(req, profile)
    }

    fn mechanism(&self, requirement: &BoundaryRequirement, enforcement: Enforcement) -> String {
        let primitive = match RequirementKind::of(requirement) {
            RequirementKind::Filesystem => "wasi_preopen",
            RequirementKind::NetworkDenyAll => "no_socket_cap",
            RequirementKind::Environment => "wasi_env",
            RequirementKind::LaunchWorkload => "wasi_instantiate",
            RequirementKind::CaptureStreams | RequirementKind::InheritedFdsNone => "wasi_stdio",
            RequirementKind::TempRoot => "wasi_preopen_tmp",
            RequirementKind::CommitArtifact | RequirementKind::DiscardArtifact => "preopen_commit",
            RequirementKind::ListOutputs => "preopen_readdir",
            RequirementKind::ChildSpawnDenyNewTasks
            | RequirementKind::ChildSpawnAllowThreads
            | RequirementKind::ChildSpawnAllowDescendants
            | RequirementKind::Kill
            | RequirementKind::ExposePath
            | RequirementKind::NetworkAllowList
            | RequirementKind::InheritedFdsOnly => "none/structurally-unsupported",
        };
        format!("{}:{primitive}:{enforcement:?}", self.id)
    }

    /// Execute a `Workload::Wasm` guest under wasmi + WASI confinement.
    ///
    /// The controlled terminals are report outcomes, not Rust errors: unsupported
    /// native cells and setup gaps return `Unsupported`/`SupervisorFault`, guest traps
    /// return `Failed`, fuel exhaustion returns `Timeout`, and clean guest exit returns
    /// `Completed`.
    fn execute(&self, plan: &BoundaryPlan) -> BoundaryReportBody {
        let observed = vec![ObservedFact {
            kind: "backend_runtime".to_string(),
            detail: "wasmi+wasi_snapshot_preview1".to_string(),
        }];
        match plan_build::build(self, plan, observed) {
            Ok((config, observed)) => {
                let observation = run::run(&self.warm, &config);
                report_mapping::map_observation(self, plan, &observation, observed)
            }
            Err(failure) => {
                report_mapping::fail_closed(self, plan, failure.outcome, failure.observed)
            }
        }
    }
}

#[must_use]
pub(super) fn default_secret_resolver() -> Arc<dyn SecretResolver + Send + Sync> {
    Arc::new(MapSecretResolver::new())
}

fn insert(
    table: &mut BTreeMap<RequirementKind, SupportVerdict>,
    kind: RequirementKind,
    enforcement: Enforcement,
    evidence: &[EvidenceClaim],
) {
    table.insert(
        kind,
        SupportVerdict::new(
            enforcement,
            evidence.iter().copied().collect::<EvidenceSet>(),
        ),
    );
}

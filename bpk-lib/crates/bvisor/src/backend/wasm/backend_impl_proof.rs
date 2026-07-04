//! `dangerous-test-hooks` proof hooks for the Wasm backend coupling proof.

use super::super::support_matrix;
use super::{default_secret_resolver, WasmBackend};
use crate::contract::backend::Backend;
use crate::contract::capability::Enforcement;
use crate::contract::ids::BackendId;
use crate::contract::plan::BoundaryRequirement;
use crate::contract::support::{BackendProfile, RequirementKind};

impl WasmBackend {
    /// A representative production-shaped Wasm backend for coupling tests.
    #[must_use]
    pub fn for_proof() -> Self {
        Self {
            id: BackendId::new(Self::ID),
            support: support_matrix(),
            secret_resolver: default_secret_resolver(),
            warm: super::warm::WarmCache::new(),
        }
    }

    /// The production ceiling this backend advertises.
    #[must_use]
    pub fn proof_ceiling(&self) -> BackendProfile {
        self.ceiling()
    }

    /// Structural facts for Wasm: no Linux kernel floors are part of the proof.
    #[must_use]
    pub fn proof_facts(&self) -> crate::contract::qualification::ProfileFacts {
        crate::contract::qualification::ProfileFacts {
            landlock_abi: 0,
            has_cgroup_kill: false,
            has_pids_peak: false,
            has_unprivileged_userns: false,
            has_seccomp_filter: false,
        }
    }

    /// The backend's live mechanism string for a requirement kind.
    #[must_use]
    pub fn proof_mechanism(&self, kind: RequirementKind, enforcement: Enforcement) -> String {
        self.mechanism(&representative_requirement(kind), enforcement)
    }
}

fn representative_requirement(kind: RequirementKind) -> BoundaryRequirement {
    use crate::contract::capability::{
        Capability, EnvPolicy, FdPolicy, FsAccess, FsConfinement, NetDest, NetPolicy, PathSet,
        SpawnPolicy,
    };
    use crate::contract::host_control::{
        CommitDurability, HostControl, KillGuarantee, KillTarget, PathView, StdStreams,
    };
    match kind {
        RequirementKind::Filesystem => BoundaryRequirement::Capability(Capability::Filesystem {
            access: FsAccess::ReadWrite,
            scope: PathSet::empty(),
            recursive: true,
            confinement: FsConfinement::DeclaredRootsOnly,
        }),
        RequirementKind::NetworkDenyAll => BoundaryRequirement::Capability(Capability::Network {
            policy: NetPolicy::DenyAll,
        }),
        RequirementKind::NetworkAllowList => BoundaryRequirement::Capability(Capability::Network {
            policy: NetPolicy::AllowList(vec![NetDest {
                host: "example".to_string(),
                port: 443,
            }]),
        }),
        RequirementKind::ChildSpawnDenyNewTasks => {
            BoundaryRequirement::Capability(Capability::ChildSpawn {
                policy: SpawnPolicy::DenyNewTasks,
            })
        }
        RequirementKind::ChildSpawnAllowThreads => {
            BoundaryRequirement::Capability(Capability::ChildSpawn {
                policy: SpawnPolicy::AllowThreadsWithinBoundary,
            })
        }
        RequirementKind::ChildSpawnAllowDescendants => {
            BoundaryRequirement::Capability(Capability::ChildSpawn {
                policy: SpawnPolicy::AllowDescendantsWithinBoundary,
            })
        }
        RequirementKind::Environment => BoundaryRequirement::Capability(Capability::Environment {
            policy: EnvPolicy::Exact(Vec::new()),
        }),
        RequirementKind::InheritedFdsNone => {
            BoundaryRequirement::Capability(Capability::InheritedFds {
                policy: FdPolicy::None,
            })
        }
        RequirementKind::InheritedFdsOnly => {
            BoundaryRequirement::Capability(Capability::InheritedFds {
                policy: FdPolicy::Only(vec![3]),
            })
        }
        RequirementKind::LaunchWorkload => {
            BoundaryRequirement::HostControl(HostControl::LaunchWorkload)
        }
        RequirementKind::CaptureStreams => {
            BoundaryRequirement::HostControl(HostControl::CaptureStreams {
                streams: StdStreams::capture_out_err(),
            })
        }
        RequirementKind::TempRoot => BoundaryRequirement::HostControl(HostControl::TempRoot {
            visibility: PathView::PrivateToBoundary,
        }),
        RequirementKind::ExposePath => BoundaryRequirement::HostControl(HostControl::ExposePath {
            source: String::new(),
            dest: String::new(),
            access: FsAccess::Read,
            view: PathView::PrivateToBoundary,
        }),
        RequirementKind::CommitArtifact => {
            BoundaryRequirement::HostControl(HostControl::CommitArtifact {
                durability: CommitDurability::Atomic,
            })
        }
        RequirementKind::DiscardArtifact => {
            BoundaryRequirement::HostControl(HostControl::DiscardArtifact)
        }
        RequirementKind::Kill => BoundaryRequirement::HostControl(HostControl::Kill {
            target: KillTarget::RunTree,
            guarantee: KillGuarantee::Atomic,
        }),
        RequirementKind::ListOutputs => BoundaryRequirement::HostControl(HostControl::ListOutputs),
    }
}

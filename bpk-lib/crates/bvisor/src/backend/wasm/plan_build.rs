//! Lower an admitted [`BoundaryPlan`] into a WASI p1 runtime configuration.
//!
//! This module is pure contract translation except for creating private temp-root
//! directories the host provisions before instantiation. It never grants ambient
//! filesystem, environment, network, or fd authority.

use super::WasmBackend;
use crate::contract::capability::{Capability, EnvPolicy, FdPolicy, FsAccess, NetPolicy};
use crate::contract::host_control::HostControl;
use crate::contract::plan::{BoundaryPlan, BoundaryRequirement, Workload};
use crate::contract::report::{ObservedFact, Outcome};
use crate::contract::secret::lower_env;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MIN_MEMORY_LIMIT: u64 = 64 * 1024;
const TEMP_ROOT_CREATE_ATTEMPTS: u8 = 64;

static TEMP_ROOT_SEQ: AtomicU64 = AtomicU64::new(1);

/// One host directory preopened into the WASI guest.
#[derive(Clone, Debug)]
pub(super) struct WasiPreopen {
    pub host_path: PathBuf,
    pub guest_path: String,
    pub access: FsAccess,
}

/// Fully lowered wasmi/WASI configuration.
#[derive(Debug)]
pub(super) struct WasmRunConfig {
    pub module_ref: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub preopens: Vec<WasiPreopen>,
    pub temp_roots: Vec<PathBuf>,
    pub fuel: u64,
    pub memory_limit: usize,
    pub filesystem_confined: bool,
    pub notes: Vec<String>,
}

/// A controlled build failure. The report mapper turns this into a fail-closed
/// body without running a guest.
#[derive(Debug)]
pub(super) struct PlanBuildFailure {
    pub outcome: Outcome,
    pub observed: Vec<ObservedFact>,
}

/// Build the WASI runtime config from an admitted plan.
pub(super) fn build(
    backend: &WasmBackend,
    plan: &BoundaryPlan,
    mut observed: Vec<ObservedFact>,
) -> Result<(WasmRunConfig, Vec<ObservedFact>), PlanBuildFailure> {
    let module_ref = match &plan.workload {
        Workload::Wasm { module_ref } => module_ref.clone(),
        Workload::Process { .. } => {
            observed.push(ObservedFact {
                kind: "workload_unsupported".to_string(),
                detail: "wasm backend only executes Workload::Wasm".to_string(),
            });
            return Err(PlanBuildFailure {
                outcome: Outcome::Unsupported,
                observed,
            });
        }
    };

    let memory_limit = usize_from_u64(
        plan.budgets
            .resident_bytes
            .effective_limit
            .max(MIN_MEMORY_LIMIT),
        &mut observed,
    )?;

    let mut config = WasmRunConfig {
        module_ref,
        args: vec!["bvisor-wasm-guest".to_string()],
        env: Vec::new(),
        preopens: Vec::new(),
        temp_roots: Vec::new(),
        fuel: plan.budgets.cpu_micros.effective_limit.max(1),
        memory_limit,
        filesystem_confined: false,
        notes: Vec::new(),
    };

    for admitted in &plan.admitted {
        match &admitted.requirement {
            BoundaryRequirement::Capability(capability) => {
                lower_capability(backend, capability, &mut config, &mut observed)?
            }
            BoundaryRequirement::HostControl(control) => {
                lower_control(control, &mut config, &mut observed)?
            }
        }
    }

    observed.push(ObservedFact {
        kind: "wasi_config_built".to_string(),
        detail: format!(
            "preopens={} env={} fuel={} memory_limit={}",
            config.preopens.len(),
            config.env.len(),
            config.fuel,
            config.memory_limit
        ),
    });
    Ok((config, observed))
}

fn lower_capability(
    backend: &WasmBackend,
    capability: &Capability,
    config: &mut WasmRunConfig,
    observed: &mut Vec<ObservedFact>,
) -> Result<(), PlanBuildFailure> {
    match capability {
        Capability::Filesystem {
            access,
            scope,
            recursive,
            ..
        } => {
            if !recursive {
                observed.push(ObservedFact {
                    kind: "filesystem_non_recursive".to_string(),
                    detail: "WASI preopens are directory-tree scoped; non-recursive scope refused"
                        .to_string(),
                });
                return Err(failure(Outcome::Unsupported, observed));
            }
            for root in &scope.roots {
                let guest_path = if config.preopens.is_empty() {
                    ".".to_string()
                } else {
                    format!("root{}", config.preopens.len())
                };
                config.preopens.push(WasiPreopen {
                    host_path: PathBuf::from(root),
                    guest_path,
                    access: *access,
                });
            }
            config.filesystem_confined = true;
            observed.push(ObservedFact {
                kind: "filesystem_confined".to_string(),
                detail: format!(
                    "wasi_preopen installed for {} declared root(s)",
                    scope.roots.len()
                ),
            });
        }
        Capability::Network {
            policy: NetPolicy::DenyAll,
        } => {
            config
                .notes
                .push("network_deny_all=no_socket_cap".to_string());
            observed.push(ObservedFact {
                kind: "network_deny_all".to_string(),
                detail: "no WASI socket capabilities installed".to_string(),
            });
        }
        Capability::Network {
            policy: NetPolicy::AllowList(_),
        } => {
            observed.push(ObservedFact {
                kind: "network_allow_list_unsupported".to_string(),
                detail: "WasmBackend has no network allow-list broker".to_string(),
            });
            return Err(failure(Outcome::Unsupported, observed));
        }
        Capability::ChildSpawn { policy } => {
            observed.push(ObservedFact {
                kind: "child_spawn_unsupported".to_string(),
                detail: format!("WASI guest cannot receive native task policy {policy:?}"),
            });
            return Err(failure(Outcome::Unsupported, observed));
        }
        Capability::Environment { policy } => {
            lower_environment(backend, policy, config, observed)?;
        }
        Capability::InheritedFds {
            policy: FdPolicy::None,
        } => {
            config.notes.push("inherited_fds=none".to_string());
            observed.push(ObservedFact {
                kind: "inherited_fds_none".to_string(),
                detail: "guest receives only WASI stdio and declared preopens".to_string(),
            });
        }
        Capability::InheritedFds {
            policy: FdPolicy::Only(_),
        } => {
            observed.push(ObservedFact {
                kind: "inherited_fds_only_unsupported".to_string(),
                detail: "WASI has no raw host-fd keep list".to_string(),
            });
            return Err(failure(Outcome::Unsupported, observed));
        }
    }
    Ok(())
}

fn lower_control(
    control: &HostControl,
    config: &mut WasmRunConfig,
    observed: &mut Vec<ObservedFact>,
) -> Result<(), PlanBuildFailure> {
    match control {
        HostControl::LaunchWorkload => {
            config.notes.push("launch=wasi_instantiate".to_string());
        }
        HostControl::CaptureStreams { streams } => {
            config.notes.push(format!(
                "capture_stdout={} capture_stderr={} stdin={}",
                streams.stdout, streams.stderr, streams.stdin
            ));
        }
        HostControl::TempRoot { .. } => {
            let temp_root = create_temp_root(observed)?;
            let guest_path = if config.preopens.is_empty() {
                ".".to_string()
            } else {
                format!("tmp{}", config.temp_roots.len())
            };
            config.preopens.push(WasiPreopen {
                host_path: temp_root.clone(),
                guest_path,
                access: FsAccess::ReadWrite,
            });
            config.temp_roots.push(temp_root);
            config.filesystem_confined = true;
            observed.push(ObservedFact {
                kind: "temp_root_preopened".to_string(),
                detail: "private WASI temp root preopened".to_string(),
            });
        }
        HostControl::CommitArtifact { .. } => {
            config.notes.push("commit=preopen_commit".to_string());
        }
        HostControl::DiscardArtifact => {
            config.notes.push("discard=preopen_commit".to_string());
        }
        HostControl::ListOutputs => {
            config
                .notes
                .push("list_outputs=preopen_readdir".to_string());
        }
        HostControl::ExposePath { .. } => {
            observed.push(ObservedFact {
                kind: "expose_path_unsupported".to_string(),
                detail: "WasmBackend does not expose undeclared host paths".to_string(),
            });
            return Err(failure(Outcome::Unsupported, observed));
        }
        HostControl::Kill { .. } => {
            observed.push(ObservedFact {
                kind: "kill_unsupported".to_string(),
                detail: "WasmBackend has no native run-tree kill primitive".to_string(),
            });
            return Err(failure(Outcome::Unsupported, observed));
        }
    }
    Ok(())
}

fn lower_environment(
    backend: &WasmBackend,
    policy: &EnvPolicy,
    config: &mut WasmRunConfig,
    observed: &mut Vec<ObservedFact>,
) -> Result<(), PlanBuildFailure> {
    match lower_env(policy, backend.secret_resolver.as_ref()) {
        Ok(env) => {
            observed.push(ObservedFact {
                kind: "environment_lowered".to_string(),
                detail: format!("wasi_env exact entries={}", env.len()),
            });
            config.env.extend(env);
            Ok(())
        }
        Err(error) => {
            observed.push(ObservedFact {
                kind: "environment_lowering_failed".to_string(),
                detail: error.to_string(),
            });
            Err(failure(Outcome::Unsupported, observed))
        }
    }
}

fn create_temp_root(observed: &mut Vec<ObservedFact>) -> Result<PathBuf, PlanBuildFailure> {
    for _ in 0..TEMP_ROOT_CREATE_ATTEMPTS {
        let seq = TEMP_ROOT_SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("bvisor-wasm-tmp-{}-{seq}", std::process::id()));
        match create_private_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                observed.push(ObservedFact {
                    kind: "temp_root_create_failed".to_string(),
                    detail: error.to_string(),
                });
                return Err(failure(Outcome::SupervisorFault, observed));
            }
        }
    }
    observed.push(ObservedFact {
        kind: "temp_root_create_failed".to_string(),
        detail: "exclusive temp-root name allocation exhausted".to_string(),
    });
    Err(failure(Outcome::SupervisorFault, observed))
}

fn create_private_dir(path: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn failure(outcome: Outcome, observed: &[ObservedFact]) -> PlanBuildFailure {
    PlanBuildFailure {
        outcome,
        observed: observed.to_vec(),
    }
}

fn usize_from_u64(value: u64, observed: &mut Vec<ObservedFact>) -> Result<usize, PlanBuildFailure> {
    match usize::try_from(value) {
        Ok(limit) => Ok(limit),
        Err(error) => {
            observed.push(ObservedFact {
                kind: "memory_limit_unsupported".to_string(),
                detail: format!("resident_bytes does not fit usize: {error}"),
            });
            Err(failure(Outcome::Unsupported, observed))
        }
    }
}

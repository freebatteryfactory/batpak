//! [`LinuxBackend`] — REAL landlock filesystem confinement (step b, part 1).
//!
//! SCOPE of THIS chunk: REAL filesystem confinement ONLY. `execute()` installs a
//! landlock ruleset restricting FS access to exactly the declared roots, launches
//! the workload with that confinement in force, and captures stdout/stderr. The
//! orchestration here is SAFE; the two raw `unsafe` blocks (the ABI probe and the
//! `pre_exec` ruleset installation) live in [`super::sys`], registered in the
//! unsafe ledger.
//!
//! HONESTY (the cardinal rule): `profile()` backs ONLY what this chunk genuinely
//! implements with a syscall — `Filesystem` (landlock, gated by the live ABI
//! floor), `LaunchWorkload` (process spawn), and `CaptureStreams` (piped
//! stdout/stderr). EVERYTHING ELSE (`ChildSpawn`, `Kill`, `NetworkDenyAll`,
//! `TempRoot`, …) is ABSENT from the ceiling, so it floors to `Unsupported` and
//! `plan()` fails closed. The family `support_matrix()` keeps the §4 aspiration;
//! the machine ceiling reflects reality. Claiming more than `execute()` delivers
//! is the exact lie the gauntlet must catch — so we do not.

use crate::backend::linux::sys::{self, ConfinedRoot};
use crate::contract::backend::Backend;
use crate::contract::budget::{BudgetAvailability, BudgetProfile};
use crate::contract::budget_witness::BudgetWitnesses;
use crate::contract::capability::{
    Capability, Enforcement, EvidenceClaim, EvidenceSet, FsAccess, FsConfinement, PathSet,
    SupportVerdict,
};
use crate::contract::ids::BackendId;
use crate::contract::plan::{BoundaryPlan, BoundaryRequirement, Workload};
use crate::contract::report::{
    BoundaryReportBody, CaptureRefs, DeniedAttempt, ExitStatus, ObservedFact, Outcome,
    BOUNDARY_REPORT_SCHEMA_VERSION,
};
use crate::contract::support::{
    BackendProfile, BackendProfileSnapshot, RequirementKind, SupportMatrix,
};
use landlock::CompatLevel;
use std::collections::BTreeMap;

/// The minimum landlock ABI this backend requires to floor `Filesystem` to
/// `Enforced`. ABI v1 already enforces path-beneath read/write/execute access —
/// the foundation of declared-roots-only confinement — so v1 is the honest floor.
/// Below it (ABI 0 = landlock unavailable) `Filesystem` floors to `Unsupported`.
const LANDLOCK_ABI_FLOOR: i64 = 1;

/// System paths a confined workload needs READ+EXECUTE to run at all (the loader,
/// shared libraries, the binary's usual locations). These are granted READ-ONLY
/// (never write), IN ADDITION to the declared data roots — a workload must be able
/// to load its own image, but the confinement of its DATA access to the declared
/// roots is unaffected (these dirs hold no secret/quarantine target).
const SYSTEM_EXEC_ROOTS: &[&str] = &["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];

/// The Linux boundary backend: REAL landlock filesystem confinement.
pub struct LinuxBackend {
    id: BackendId,
    support: SupportMatrix,
    /// The live landlock ABI integer, probed once at construction from the kernel.
    landlock_abi: i64,
}

impl LinuxBackend {
    /// The stable id of the Linux backend.
    pub const ID: &'static str = "linux";

    /// Construct the Linux backend, probing the live landlock ABI from the kernel
    /// (the raw probe is the sanctioned `super::sys` basement).
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: BackendId::new(Self::ID),
            support: super::support_matrix(),
            landlock_abi: sys::probe_landlock_abi(),
        }
    }

    /// Construct a backend with a FORCED landlock ABI, for proving the below-floor
    /// fail-closed path on a host whose live ABI is above the floor. Test-only.
    #[cfg(test)]
    fn with_abi_for_test(landlock_abi: i64) -> Self {
        Self {
            id: BackendId::new(Self::ID),
            support: super::support_matrix(),
            landlock_abi,
        }
    }

    /// Whether the live ABI meets the floor required to enforce FS confinement.
    fn filesystem_enforced(&self) -> bool {
        self.landlock_abi >= LANDLOCK_ABI_FLOOR
    }

    /// The honest machine ceiling: `Filesystem` Enforced ONLY when the ABI floor
    /// is met (else absent ⇒ Unsupported), plus `LaunchWorkload`+`CaptureStreams`
    /// (process spawn + pipe capture, which this chunk genuinely performs).
    /// Nothing else is listed — every other kind floors to `Unsupported`, so
    /// `plan()` fails closed for capabilities this chunk does not back.
    fn ceiling(&self) -> BackendProfile {
        let mut ceiling = BTreeMap::new();
        if self.filesystem_enforced() {
            ceiling.insert(
                RequirementKind::Filesystem,
                SupportVerdict::new(
                    Enforcement::Enforced,
                    [
                        EvidenceClaim::AllowedActions,
                        EvidenceClaim::DeniedAttempts,
                        EvidenceClaim::FilesystemDelta,
                        EvidenceClaim::MechanismAttestation,
                    ]
                    .into_iter()
                    .collect(),
                ),
            );
        }
        ceiling.insert(
            RequirementKind::LaunchWorkload,
            SupportVerdict::new(
                Enforcement::Enforced,
                [EvidenceClaim::TerminalOutcome, EvidenceClaim::ProcessTree]
                    .into_iter()
                    .collect(),
            ),
        );
        ceiling.insert(
            RequirementKind::CaptureStreams,
            SupportVerdict::new(
                Enforcement::Enforced,
                [EvidenceClaim::CapturedStreams].into_iter().collect(),
            ),
        );
        BackendProfile::from_ceiling(ceiling)
    }
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// The honest budget profile for THIS chunk. Budget ENFORCEMENT (cgroup limits) is
/// part 2 — NOT implemented here — so no dimension is claimed `Enforced`. The OS
/// DOES account a spawned process's resources, and the runner observes its
/// terminal, so each dimension is declared `Mediated` (supervised, not
/// structurally capped) with an honest mechanism string and NO resource-usage
/// evidence claim. This lets a budgeted FS spec admit (the launch needs nonzero
/// derived minimums) WITHOUT claiming a cap this chunk does not install.
fn observed_budget_profile() -> BudgetProfile {
    let observed = |mechanism: &str| BudgetAvailability {
        // Headroom only — we do NOT cap, so we never refuse on capacity here; the
        // honest signal is the `Mediated` (not `Enforced`) guarantee + empty
        // evidence, which forbids a spec from demanding a witnessed/enforced cap.
        available: u64::MAX,
        enforcement: Enforcement::Mediated,
        evidence: EvidenceSet::new(),
        mechanism: mechanism.to_string(),
    };
    BudgetProfile {
        wall_micros: observed("os_process_wait:observed-not-capped"),
        cpu_micros: observed("os_rusage:observed-not-capped"),
        resident_bytes: observed("os_rusage:observed-not-capped"),
        process_count: observed("os_process:observed-not-capped"),
        handle_count: observed("os_fd:observed-not-capped"),
        storage_bytes: observed("os_fs:observed-not-capped"),
        network_bytes: observed("os_net:observed-not-capped"),
    }
}

impl Backend for LinuxBackend {
    fn id(&self) -> BackendId {
        self.id.clone()
    }

    fn support(&self) -> &SupportMatrix {
        &self.support
    }

    fn probe(&self) -> BackendProfileSnapshot {
        // REAL probe facts: the live landlock ABI integer and whether the FS floor
        // is met. Deterministic given the kernel, so replay re-derives identically.
        let mut probed = BTreeMap::new();
        probed.insert("landlock_abi".to_string(), self.landlock_abi.to_string());
        probed.insert(
            "filesystem_confinement".to_string(),
            if self.filesystem_enforced() {
                "landlock".to_string()
            } else {
                "unsupported-below-abi-floor".to_string()
            },
        );
        BackendProfileSnapshot {
            backend: self.id.clone(),
            probed,
            budget: observed_budget_profile(),
        }
    }

    fn profile(&self, _snap: &BackendProfileSnapshot) -> BackendProfile {
        // The ceiling is derived from the live ABI: FS Enforced only above the
        // floor; otherwise FS is absent ⇒ Unsupported ⇒ plan() fails closed.
        self.ceiling()
    }

    fn classify(&self, req: &BoundaryRequirement, profile: &BackendProfile) -> SupportVerdict {
        self.support.classify(req, profile)
    }

    fn mechanism(&self, requirement: &BoundaryRequirement, enforcement: Enforcement) -> String {
        // This backend authors only the mechanisms it actually performs this
        // chunk; everything else is honestly named as unimplemented-this-chunk.
        // Exhaustive ON PURPOSE (no bare wildcard over known variants): this chunk
        // backs exactly Filesystem/LaunchWorkload/CaptureStreams; every other kind
        // is honestly named unimplemented-this-chunk. A future variant must declare
        // its mechanism here rather than silently inheriting the unimplemented tag.
        let primitive = match RequirementKind::of(requirement) {
            RequirementKind::Filesystem => "landlock",
            RequirementKind::LaunchWorkload => "process_spawn",
            RequirementKind::CaptureStreams => "pipe_capture",
            RequirementKind::NetworkDenyAll
            | RequirementKind::NetworkAllowList
            | RequirementKind::ChildSpawn
            | RequirementKind::Environment
            | RequirementKind::InheritedFds
            | RequirementKind::TempRoot
            | RequirementKind::ExposePath
            | RequirementKind::CommitArtifact
            | RequirementKind::DiscardArtifact
            | RequirementKind::Kill
            | RequirementKind::ListOutputs => "none/unimplemented-this-chunk",
        };
        format!("{}:{primitive}:{enforcement:?}", self.id)
    }

    fn execute(&self, plan: &BoundaryPlan) -> BoundaryReportBody {
        execute_confined(self, plan)
    }
}

/// Run the admitted plan with REAL landlock confinement, returning the honest
/// observed body. Never panics: every failure resolves to an honest [`Outcome`].
fn execute_confined(backend: &LinuxBackend, plan: &BoundaryPlan) -> BoundaryReportBody {
    let mut observed = Vec::new();
    let denied = Vec::new();

    // Only a native process workload is runnable by this backend.
    let (exe, args) = match &plan.workload {
        Workload::Process { exe, args } => (exe.clone(), args.clone()),
        Workload::Wasm { module_ref } => {
            observed.push(ObservedFact {
                kind: "workload_unsupported".to_string(),
                detail: format!("linux backend cannot run wasm module {module_ref}"),
            });
            return body(
                backend,
                plan,
                Outcome::Unsupported,
                None,
                CaptureRefs::default(),
                observed,
                denied,
            );
        }
    };

    // Gather the declared FS roots from the admitted Filesystem capability. The
    // plan was admitted against our ceiling, so a Filesystem capability here is
    // DeclaredRootsOnly + a PathSet. Absent ⇒ no FS confinement was requested.
    let fs = filesystem_capability(plan);

    // Build the confinement root set: the declared data roots (read, or read+write
    // when the access grants writing) PLUS read-only system exec roots so the
    // workload image can load. If the ABI floor is unmet we MUST NOT run unconfined
    // while reporting Filesystem enforcement — fail closed instead.
    let confine = match &fs {
        Some((access, scope)) if !backend.filesystem_enforced() => {
            let _ = (access, scope);
            observed.push(ObservedFact {
                kind: "filesystem_confinement_unavailable".to_string(),
                detail: format!(
                    "landlock abi {} below floor {LANDLOCK_ABI_FLOOR}; refusing to run unconfined",
                    backend.landlock_abi
                ),
            });
            return body(
                backend,
                plan,
                Outcome::Unsupported,
                None,
                CaptureRefs::default(),
                observed,
                denied,
            );
        }
        Some((access, scope)) => Some(build_roots(*access, scope)),
        None => None,
    };

    // Record the confinement mechanism + the exact roots as honest evidence.
    if let Some(roots) = &confine {
        let writable: Vec<&str> = roots
            .iter()
            .filter(|r| r.writable)
            .map(|r| r.path.as_str())
            .collect();
        let readonly: Vec<&str> = roots
            .iter()
            .filter(|r| !r.writable)
            .map(|r| r.path.as_str())
            .collect();
        observed.push(ObservedFact {
            kind: "filesystem_confined".to_string(),
            detail: format!(
                "landlock abi {}: read-roots {readonly:?}, write-roots {writable:?}",
                backend.landlock_abi
            ),
        });
    }

    // Launch. With confinement the spawn goes through the `pre_exec` basement; an
    // unconfined launch (no FS capability) still captures streams honestly.
    let output = match &confine {
        Some(roots) => sys::spawn_confined(&exe, &args, roots, CompatLevel::HardRequirement),
        None => std::process::Command::new(&exe).args(&args).output(),
    };

    match output {
        Ok(out) => {
            let ctx = LaunchContext {
                exe: &exe,
                confined: confine.is_some(),
                fs: fs.as_ref(),
            };
            handle_output(backend, plan, &ctx, &out, observed, denied)
        }
        Err(error) => {
            observed.push(ObservedFact {
                kind: "workload_launch_failed".to_string(),
                detail: format!("linux could not spawn {exe}: {error}"),
            });
            body(
                backend,
                plan,
                Outcome::Failed,
                None,
                CaptureRefs::default(),
                observed,
                denied,
            )
        }
    }
}

/// The launch context threaded into [`handle_output`] (avoids a long arg list).
struct LaunchContext<'a> {
    exe: &'a str,
    confined: bool,
    fs: Option<&'a (FsAccess, PathSet)>,
}

/// Turn a captured [`Output`] into the honest report body: launch + capture
/// observations, an OBSERVED denied attempt when landlock blocked an out-of-root
/// access (deny-more OK; report-less ILLEGAL), and the terminal classification.
fn handle_output(
    backend: &LinuxBackend,
    plan: &BoundaryPlan,
    ctx: &LaunchContext<'_>,
    out: &std::process::Output,
    mut observed: Vec<ObservedFact>,
    mut denied: Vec<DeniedAttempt>,
) -> BoundaryReportBody {
    observed.push(ObservedFact {
        kind: "workload_launched".to_string(),
        detail: format!("linux spawned {} (confined={})", ctx.exe, ctx.confined),
    });
    observed.push(ObservedFact {
        kind: "stream_captured".to_string(),
        detail: format!(
            "captured {} stdout byte(s), {} stderr byte(s)",
            out.stdout.len(),
            out.stderr.len()
        ),
    });

    if ctx.confined && !out.status.success() {
        record_denial(ctx.fs, out, &mut observed, &mut denied);
    }

    let exit = exit_from_output(&out.status);
    let captured = CaptureRefs {
        stdout: Some(format!("inline:{}b", out.stdout.len())),
        stderr: Some(format!("inline:{}b", out.stderr.len())),
    };
    let outcome = terminal_outcome(&exit, !denied.is_empty());
    body(backend, plan, outcome, exit, captured, observed, denied)
}

/// Record an OBSERVED filesystem denial when the confined workload's stderr names
/// a permission denial — honest danger evidence the report never hides.
fn record_denial(
    fs: Option<&(FsAccess, PathSet)>,
    out: &std::process::Output,
    observed: &mut Vec<ObservedFact>,
    denied: &mut Vec<DeniedAttempt>,
) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !(stderr.contains("Permission denied") || stderr.contains("Operation not permitted")) {
        return;
    }
    observed.push(ObservedFact {
        kind: "filesystem_access_denied".to_string(),
        detail: format!("landlock denied an out-of-root access: {}", stderr.trim()),
    });
    if let Some((access, scope)) = fs {
        denied.push(DeniedAttempt {
            requirement: BoundaryRequirement::Capability(Capability::Filesystem {
                access: *access,
                scope: scope.clone(),
                recursive: true,
                confinement: FsConfinement::DeclaredRootsOnly,
            }),
            detail: format!(
                "landlock blocked access outside declared roots: {}",
                stderr.trim()
            ),
        });
    }
}

/// Extract the admitted Filesystem capability's access + scope, if one was
/// admitted into the plan.
fn filesystem_capability(plan: &BoundaryPlan) -> Option<(FsAccess, PathSet)> {
    plan.admitted.iter().find_map(|a| match &a.requirement {
        BoundaryRequirement::Capability(Capability::Filesystem { access, scope, .. }) => {
            Some((*access, scope.clone()))
        }
        BoundaryRequirement::Capability(_) | BoundaryRequirement::HostControl(_) => None,
    })
}

/// Build the confinement root set from the declared scope + access. Declared
/// roots get write access only when the grant includes writing; system exec roots
/// are always read-only so the workload image can load.
fn build_roots(access: FsAccess, scope: &PathSet) -> Vec<ConfinedRoot> {
    let writable = matches!(access, FsAccess::Write | FsAccess::ReadWrite);
    let mut roots: Vec<ConfinedRoot> = scope
        .roots
        .iter()
        .map(|path| ConfinedRoot {
            path: path.clone(),
            writable,
        })
        .collect();
    for sys_root in SYSTEM_EXEC_ROOTS {
        roots.push(ConfinedRoot {
            path: (*sys_root).to_string(),
            writable: false,
        });
    }
    roots
}

/// Assemble the honest report body.
fn body(
    backend: &LinuxBackend,
    plan: &BoundaryPlan,
    outcome: Outcome,
    exit: Option<ExitStatus>,
    captured: CaptureRefs,
    observed: Vec<ObservedFact>,
    denied: Vec<DeniedAttempt>,
) -> BoundaryReportBody {
    BoundaryReportBody {
        schema_version: BOUNDARY_REPORT_SCHEMA_VERSION,
        plan_id: plan.plan_id,
        backend: backend.id.clone(),
        profile: backend.probe(),
        outcome,
        admitted: plan.admitted.clone(),
        observed,
        denied,
        exit,
        captured,
        budget: BudgetWitnesses::unwitnessed(&plan.budgets),
        artifacts: Vec::new(),
        findings: Vec::new(),
    }
}

/// Map the captured exit status into the run-time [`Outcome`]. A run that the
/// boundary actively denied is classified [`Outcome::Denied`] (the confinement
/// bit), distinct from a plain non-zero [`Outcome::Failed`].
fn terminal_outcome(exit: &Option<ExitStatus>, was_denied: bool) -> Outcome {
    match exit {
        Some(ExitStatus::Code(0)) => Outcome::Completed,
        _ if was_denied => Outcome::Denied,
        Some(ExitStatus::Code(_)) | Some(ExitStatus::Signal(_)) | None => Outcome::Failed,
    }
}

/// Convert a `std::process::ExitStatus` into the portable [`ExitStatus`].
fn exit_from_output(status: &std::process::ExitStatus) -> Option<ExitStatus> {
    if let Some(code) = status.code() {
        return Some(ExitStatus::Code(code));
    }
    use std::os::unix::process::ExitStatusExt;
    status.signal().map(ExitStatus::Signal)
}

#[cfg(test)]
mod tests {
    use super::{LinuxBackend, LANDLOCK_ABI_FLOOR};
    use crate::contract::backend::Backend;
    use crate::contract::capability::{Capability, Enforcement, FsAccess, FsConfinement, PathSet};
    use crate::contract::plan::BoundaryRequirement;
    use crate::contract::support::RequirementKind;

    fn fs_requirement() -> BoundaryRequirement {
        BoundaryRequirement::Capability(Capability::Filesystem {
            access: FsAccess::Read,
            // An inert scope path — classify() never touches disk, so the value is
            // immaterial; a relative placeholder avoids leaking an absolute path.
            scope: PathSet {
                roots: vec!["quarantine/root".to_string()],
            },
            recursive: true,
            confinement: FsConfinement::DeclaredRootsOnly,
        })
    }

    #[test]
    fn filesystem_is_enforced_at_or_above_the_abi_floor() {
        // At the floor the machine ceiling backs Filesystem, so classify is Enforced.
        let backend = LinuxBackend::with_abi_for_test(LANDLOCK_ABI_FLOOR);
        let profile = backend.profile(&backend.probe());
        let verdict = backend.classify(&fs_requirement(), &profile);
        assert_eq!(
            verdict.enforcement,
            Enforcement::Enforced,
            "at/above the ABI floor, Filesystem must be Enforced"
        );
        // The ceiling lists Filesystem at the floor.
        assert_eq!(
            profile.ceiling_for(RequirementKind::Filesystem).enforcement,
            Enforcement::Enforced
        );
    }

    #[test]
    fn filesystem_fails_closed_below_the_abi_floor() {
        // Below the floor (e.g. landlock unavailable ⇒ probed ABI 0) the machine
        // ceiling does NOT back Filesystem, so the family Enforced best-case is
        // floored to Unsupported — and plan() will refuse a FS spec fail-closed.
        let backend = LinuxBackend::with_abi_for_test(LANDLOCK_ABI_FLOOR - 1);
        let profile = backend.profile(&backend.probe());
        let verdict = backend.classify(&fs_requirement(), &profile);
        assert_eq!(
            verdict.enforcement,
            Enforcement::Unsupported,
            "below the ABI floor, Filesystem MUST fail closed (no unbacked guarantee)"
        );
    }

    #[test]
    fn unimplemented_kinds_fail_closed_this_chunk() {
        // HONESTY: this chunk backs ONLY Filesystem/LaunchWorkload/CaptureStreams.
        // Kill / NetworkDenyAll / ChildSpawn / TempRoot are NOT in the ceiling, so
        // they floor to Unsupported and plan() fails closed for them.
        let backend = LinuxBackend::with_abi_for_test(LANDLOCK_ABI_FLOOR);
        let profile = backend.profile(&backend.probe());
        for kind in [
            RequirementKind::Kill,
            RequirementKind::NetworkDenyAll,
            RequirementKind::ChildSpawn,
            RequirementKind::TempRoot,
        ] {
            assert_eq!(
                profile.ceiling_for(kind).enforcement,
                Enforcement::Unsupported,
                "{kind:?} must stay Unsupported until its chunk lands (no inflation)"
            );
        }
    }
}

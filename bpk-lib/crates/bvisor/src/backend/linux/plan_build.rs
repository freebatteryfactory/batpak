//! HOST-SIDE launcher-plan construction for the Linux backend (split out of
//! `backend_impl.rs` to keep each production file under the structural-check size cap).
//!
//! This module assembles the [`LinuxLaunchPlanV1`] + the pre-opened authority handles
//! the launcher inherits: the descriptor table (authority handles keyed to their slot fd
//! numbers), the lowering schedule the launcher SERVES (scrub + optional landlock-apply +
//! exec), and the launch body's identity binding (`h_l = blake3(canonical(lowering))`).
//! All of it is SAFE std (`File::open`) — authority rides an OWNED handle, never a
//! reopened path (CVE-2019-5736 / Leaky-Vessels class). The OS spawn/confinement lives in
//! the `sys` basement + the launcher itself; nothing here is `unsafe`.

use crate::backend::linux::launch::AuthorityFd;
use crate::backend::linux::protocol::{
    DescriptorKind, DescriptorRole, DescriptorShape, DescriptorSlotV1, LinuxLaunchBodyV1,
    LinuxLaunchPlanV1, LoweringWireV1, NetworkNsRequest, SeccompRequest, TargetSpecV1,
    UserNsRequest,
};
use crate::contract::capability::{EvidenceSet, FsAccess, PathSet, SupportVerdict};
use crate::contract::ids::{
    AdmissionProgramHash, AttemptId, BackendProfileHash, BoundaryPlanHash, Digest32,
};
use crate::contract::lowering::compile_schedule;
use crate::contract::plan::{BoundaryPlan, BoundaryRequirement};
use crate::contract::primitive::{
    LoweringPhase, PrimitiveDecl, PrimitiveId, PrimitiveVersion, Privilege,
};
use crate::contract::report::BoundaryReportBody;
use crate::contract::support::{BackendProfile, RequirementKind};
use std::os::fd::{OwnedFd, RawFd};

/// System paths a confined workload needs READ+EXECUTE to run at all (the loader,
/// shared libraries, the binary's usual locations). These are granted READ-ONLY
/// (never write), IN ADDITION to the declared data roots — a workload must be able
/// to load its own image, but the confinement of its DATA access to the declared
/// roots is unaffected (these dirs hold no secret/quarantine target). Only dirs that
/// EXIST on the host are wired as ReadRoot slots (opening a missing dir would fail
/// the launch); the rest are skipped.
const SYSTEM_EXEC_ROOTS: &[&str] = &["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];

// ── Frozen launcher-wire constants (mirror `launcher/linux/imp.rs`) ────────────
// The backend MUST mirror exactly the primitive ids + phase codes the launcher
// SERVES, or the launcher refuses `MissingPrimitive`. These are the launcher's own
// frozen constants, restated here so the two sides agree without a shared module
// (the protocol carries the wire shapes, not these string/code literals).

/// The fd-scrub primitive (AmbientAuthority phase) — the launcher's MANDATORY
/// ambient-authority action. Every launch schedules exactly one.
const ID_AMBIENT_SCRUB: &str = "linux.ambient.scrub.v1";
/// The landlock-apply primitive (Confinement phase). Scheduled ONLY when the plan
/// carries a Filesystem capability; its absence ⇒ Confinement resolves NotRequired.
const ID_LANDLOCK_APPLY: &str = "linux.landlock.apply.v1";
/// The seccomp-apply primitive (Confinement phase, S10). Scheduled ONLY when a seccomp
/// denylist is engaged (ChildSpawn::DenyNewTasks and/or the NetworkDenyAll DiD); its
/// absence ⇒ no seccomp filter is installed (the child window is unchanged).
const ID_SECCOMP_APPLY: &str = "linux.seccomp.apply.v1";
/// The launch primitive (marks the `fexecve` step). Always scheduled.
const ID_EXEC: &str = "linux.exec.v1";
/// The declaration revision every launcher action carries (all ids end in `.v1`).
const SCHEDULE_PRIMITIVE_VERSION: u32 = 1;

// ── Descriptor-table slot fd numbers (slot_index == the fd the launcher reads) ──
// The launcher reads each authority handle at the fd number equal to its slot
// index, and the harness places its OWN channel fds (plan/control/error) strictly
// ABOVE every authority slot, so these fixed low numbers never collide. They must
// stay > 2 (the stdio floor the harness reserves) and dense enough to be distinct.

/// The target executable handle (`DescriptorRole::TargetExe`).
const SLOT_EXE: RawFd = 10;
/// The single declared read root (`DescriptorRole::ReadRoot`), when the access is
/// read-only. With a write grant the data root rides [`SLOT_WRITE_ROOT`] instead.
const SLOT_READ_ROOT: RawFd = 15;
/// The single declared write root (`DescriptorRole::WriteRoot`), when the access
/// grants writing.
const SLOT_WRITE_ROOT: RawFd = 16;
/// The cgroup leaf directory (`DescriptorRole::CgroupDir`), when the host created a
/// per-run leaf — the launcher births the child INTO it via `CLONE_INTO_CGROUP`.
const SLOT_CGROUP: RawFd = 17;
/// The base fd for the read-only system-exec roots (loader/libs), one per present
/// dir at `SLOT_SYS_ROOT_BASE + i`.
const SLOT_SYS_ROOT_BASE: RawFd = 20;

/// Everything the host built for one launcher run: the sealed-later plan, the
/// pre-opened authority handles keyed to their slot fds, the human-readable root
/// lists (honest evidence), and whether a landlock action was scheduled.
pub(super) struct Prepared {
    pub(super) launch_plan: LinuxLaunchPlanV1,
    pub(super) authority: Vec<AuthorityFd>,
    pub(super) read_roots: Vec<String>,
    pub(super) write_roots: Vec<String>,
    pub(super) confined: bool,
}

/// The lowered launch inputs `prepare_launch` assembles a [`LinuxLaunchPlanV1`] from.
/// Bundled into one struct so `prepare_launch` stays within the argument budget (zero
/// `#[allow]` doctrine — no `too_many_arguments`). Borrows the plan/fs by reference and
/// MOVES the owned `cgroup_dir_fd` / `envp` into the build.
pub(super) struct LaunchInputs<'a> {
    /// The target executable path (opened host-side as the exe authority handle).
    pub(super) exe: &'a str,
    /// The workload argument vector (after `argv[0]`).
    pub(super) args: &'a [String],
    /// The admitted boundary plan (its identity binds `h_l`/`h_p`/`plan_id`).
    pub(super) plan: &'a BoundaryPlan,
    /// The admitted Filesystem capability (access + scope), or `None` (no FS confinement).
    pub(super) fs: Option<&'a (FsAccess, PathSet)>,
    /// The prepared per-run cgroup leaf dir fd, or `None` (no cgroup placement).
    pub(super) cgroup_dir_fd: Option<OwnedFd>,
    /// The explicit environment served to the workload (lowered Environment::Exact).
    pub(super) envp: Vec<(String, String)>,
    /// Whether the admitted NetworkDenyAll engages the empty netns (S9 / D3).
    pub(super) deny_network: bool,
    /// Whether the admitted ChildSpawn::DenyNewTasks engages the seccomp task-creation
    /// denylist (S10).
    pub(super) deny_new_tasks: bool,
}

/// Build the [`LinuxLaunchPlanV1`] + pre-opened authority handles from the admitted
/// plan, host-side with SAFE std (`File::open`). Returns a human-readable error
/// string on any wiring fault (the caller fails closed). The descriptor table, the
/// lowering schedule, and the authority handles are all assembled here so the
/// launcher reads each handle at its declared slot fd number.
pub(super) fn prepare_launch(inputs: LaunchInputs<'_>) -> Result<Prepared, String> {
    let LaunchInputs {
        exe,
        args,
        plan,
        fs,
        cgroup_dir_fd,
        envp,
        deny_network,
        deny_new_tasks,
    } = inputs;
    let mut table: Vec<DescriptorSlotV1> = Vec::new();
    let mut authority: Vec<AuthorityFd> = Vec::new();
    let mut read_roots: Vec<String> = Vec::new();
    let mut write_roots: Vec<String> = Vec::new();

    // 1. The target executable rides a handle, never a path (exec is `fexecve` on
    //    the inherited fd in the launcher child).
    let exe_handle = open_handle(exe)?;
    authority.push(AuthorityFd {
        slot_index: SLOT_EXE,
        handle: exe_handle,
    });
    table.push(exe_slot());

    // 2. The declared data root (read-only, or read+write when the grant writes),
    //    plus the read-only system-exec roots so the workload image can load.
    let confined = fs.is_some();
    if let Some((access, scope)) = fs {
        let writable = matches!(access, FsAccess::Write | FsAccess::ReadWrite);
        for path in &scope.roots {
            let handle = open_handle(path)?;
            let (slot, role) = if writable {
                write_roots.push(path.clone());
                (SLOT_WRITE_ROOT, DescriptorRole::WriteRoot)
            } else {
                read_roots.push(path.clone());
                (SLOT_READ_ROOT, DescriptorRole::ReadRoot)
            };
            authority.push(AuthorityFd {
                slot_index: slot,
                handle,
            });
            table.push(root_slot(slot, role));
        }
        // System-exec roots: one ReadRoot slot per dir that EXISTS on the host.
        let mut sys_i: RawFd = 0;
        for sys_root in SYSTEM_EXEC_ROOTS {
            if !std::path::Path::new(sys_root).is_dir() {
                continue;
            }
            let handle = open_handle(sys_root)?;
            let slot = SLOT_SYS_ROOT_BASE
                .checked_add(sys_i)
                .ok_or_else(|| "system-exec root slot overflow".to_string())?;
            authority.push(AuthorityFd {
                slot_index: slot,
                handle,
            });
            table.push(root_slot(slot, DescriptorRole::ReadRoot));
            read_roots.push((*sys_root).to_string());
            sys_i += 1;
        }
    }

    // 2b. The cgroup leaf directory, when the host created a per-run leaf: the launcher
    //     resolves this singleton CgroupDir slot and births the workload child INSIDE the
    //     leaf via CLONE_INTO_CGROUP (no post-fork migration race). The fd is a
    //     non-writable directory (File::open is O_RDONLY); it is NOT a lowering action, so
    //     it does NOT enter the schedule / H_L — it is driven purely by the slot's
    //     presence.
    if let Some(fd) = cgroup_dir_fd {
        authority.push(AuthorityFd {
            slot_index: SLOT_CGROUP,
            handle: fd,
        });
        table.push(cgroup_slot());
    }

    // Compose the OPT-IN seccomp DENYLIST request (S10): ChildSpawn::DenyNewTasks denies the
    // task-creation family, and a NetworkDenyAll engagement ADDS the socket(2) defense-in-
    // depth (D3) on TOP of the structural empty netns. `None` ⇒ no filter (the child window
    // is byte-for-byte unchanged). Both flags fold into ONE denylist the launcher compiles.
    let seccomp_request = seccomp_request(deny_new_tasks, deny_network);

    // 3. The lowering schedule the launcher SERVES, compiled through the ONE validated
    //    compiler `compile_schedule` (`K:(S,P,D)→L`) and PROJECTED onto the wire via
    //    `LoweringWireV1::from_schedule` — never hand-assembled. This is the single
    //    source of truth: the wire is a projection of a real, phase-ordered,
    //    conflict-checked, canonical `LoweringSchedule`, not a parallel restatement.
    //    Actions: scrub (mandatory) + landlock-apply (only when confining) +
    //    seccomp-apply (only when a denylist is engaged) + exec. Their distinct phases
    //    (`FdHygiene` < `PolicyInstall` < `Launch`) yield exactly the canonical order the
    //    launcher's frozen ids/phase-codes expect, else the launcher refuses
    //    MissingPrimitive. (The admission-side H_L/attempt reconciliation is #75.)
    let lowering = compile_lowering(confined, seccomp_request.is_some())?;

    let body = build_body(BuildBody {
        plan,
        lowering,
        table,
        exe,
        args,
        envp,
        deny_network,
        seccomp_request,
    })?;
    Ok(Prepared {
        launch_plan: LinuxLaunchPlanV1 { body },
        authority,
        read_roots,
        write_roots,
        confined,
    })
}

/// Compile the launcher lowering schedule through the ONE validated compiler and
/// project it onto the wire. `confined` schedules the landlock-apply action;
/// `seccomp` schedules the seccomp-apply action. Scrub and exec are always present.
///
/// The action decls are SCHEDULE-only (see [`LauncherActionDecl`]); routing them
/// through [`compile_schedule`] means the wire's per-entry `decl_digest`/`param_digest`
/// are the REAL compiled digests, and the launcher binds the whole projection via
/// `h_l = blake3(canonical(lowering))` (computed identically on both sides).
fn compile_lowering(confined: bool, seccomp: bool) -> Result<LoweringWireV1, String> {
    let scrub = LauncherActionDecl::new(ID_AMBIENT_SCRUB, LoweringPhase::FdHygiene);
    let landlock = LauncherActionDecl::new(ID_LANDLOCK_APPLY, LoweringPhase::PolicyInstall);
    let seccomp_apply = LauncherActionDecl::new(ID_SECCOMP_APPLY, LoweringPhase::PolicyInstall);
    let exec = LauncherActionDecl::new(ID_EXEC, LoweringPhase::Launch);

    let mut decls: Vec<&dyn PrimitiveDecl> = vec![&scrub];
    if confined {
        decls.push(&landlock);
    }
    if seccomp {
        decls.push(&seccomp_apply);
    }
    decls.push(&exec);

    let schedule = compile_schedule(&decls)
        .map_err(|e| format!("cannot compile the lowering schedule: {e}"))?;
    Ok(LoweringWireV1::from_schedule(&schedule))
}

/// A schedule-only [`PrimitiveDecl`] for one Linux launcher action (scrub / landlock-
/// apply / seccomp-apply / exec).
///
/// The Linux confinement schedule is compiled through the ONE validated compiler
/// [`compile_schedule`] rather than hand-assembled, so the launcher wire is a
/// PROJECTION of a real, phase-ordered, conflict-checked [`crate::contract::lowering::LoweringSchedule`]
/// — there is no second, parallel schedule source of truth.
///
/// These decls carry only the SCHEDULE-relevant facts (id, version, phase). They
/// deliberately `covers()` NOTHING: support classification for the Linux backend lives
/// in the family `support_matrix()` + admission path, not here (the primitive-adapter
/// support-classification integration is the #75 track-A membrane). Because `covers()`
/// is empty, [`crate::contract::primitive::classify_via_primitives`] never reaches
/// `classify`, so its fail-closed `unsupported()` result is inert-by-construction —
/// never a live verdict, only a required-method stub.
struct LauncherActionDecl {
    id: PrimitiveId,
    phase: LoweringPhase,
}

impl LauncherActionDecl {
    fn new(id: &str, phase: LoweringPhase) -> Self {
        Self {
            id: PrimitiveId::new(id),
            phase,
        }
    }
}

impl PrimitiveDecl for LauncherActionDecl {
    fn id(&self) -> PrimitiveId {
        self.id.clone()
    }

    fn version(&self) -> PrimitiveVersion {
        PrimitiveVersion::new(SCHEDULE_PRIMITIVE_VERSION)
    }

    fn covers(&self) -> &[RequirementKind] {
        &[]
    }

    fn classify(&self, _req: &BoundaryRequirement, _profile: &BackendProfile) -> SupportVerdict {
        // Inert by construction: `covers()` is empty, so `classify_via_primitives`
        // never calls this. Support classification stays with `support_matrix()` /
        // the admission path (#75). The fail-closed bottom is the safe stub.
        SupportVerdict::unsupported()
    }

    fn phase(&self) -> LoweringPhase {
        self.phase
    }

    fn prerequisites(&self) -> &[PrimitiveId] {
        &[]
    }

    fn conflicts(&self) -> &[PrimitiveId] {
        &[]
    }

    fn required_privileges(&self) -> &[Privilege] {
        &[]
    }

    fn witness(&self, _observed: &BoundaryReportBody, _out: &mut EvidenceSet) {}
}

/// Compose the OPT-IN [`SeccompRequest`] from the lowering flags: `deny_new_tasks`
/// (ChildSpawn::DenyNewTasks) denies the task-creation family; `deny_network`
/// (NetworkDenyAll engaged) ADDS the `socket(2)` defense-in-depth (D3) on top of the
/// structural empty netns. Returns `None` when NEITHER is set, so a plan with no
/// child-task / no network-DiD omits the field entirely (the off-path bytes stay stable
/// and the child window installs no filter).
fn seccomp_request(deny_new_tasks: bool, deny_network: bool) -> Option<SeccompRequest> {
    if !deny_new_tasks && !deny_network {
        return None;
    }
    let request = SeccompRequest {
        deny_new_tasks,
        // NetworkDenyAll DiD: deny externally-routable socket creation (D3) ON TOP of the
        // empty netns (the netns stays the structural guarantee; seccomp strengthens it).
        deny_inet_sockets: deny_network,
    };
    // Defensive: only emit a request that actually denies something.
    request.denies_anything().then_some(request)
}

/// Open a directory/file path as an owned handle with SAFE std (`File::open`). The
/// path is opened HOST-SIDE so authority rides the handle, never a reopened path
/// (CVE-2019-5736 / Leaky-Vessels class). A failure is a host wiring fault string.
fn open_handle(path: &str) -> Result<OwnedFd, String> {
    std::fs::File::open(path)
        .map(OwnedFd::from)
        .map_err(|e| format!("cannot open authority path {path}: {e}"))
}

/// The `DescriptorRole::TargetExe` slot declaration (a regular file, read-only —
/// exec rides the fd; the launcher `fstat`-checks the shape).
fn exe_slot() -> DescriptorSlotV1 {
    DescriptorSlotV1 {
        slot_index: slot_u32(SLOT_EXE),
        role: DescriptorRole::TargetExe,
        expected: DescriptorShape {
            kind: DescriptorKind::Regular,
            writable: false,
        },
    }
}

/// A confinement-root slot declaration. A directory fd is never writable per
/// `O_ACCMODE`, so the declared shape is `writable:false`; the landlock WRITE grant
/// is driven by the `role` (WriteRoot), NOT the fd's open mode.
fn root_slot(fd: RawFd, role: DescriptorRole) -> DescriptorSlotV1 {
    DescriptorSlotV1 {
        slot_index: slot_u32(fd),
        role,
        expected: DescriptorShape {
            kind: DescriptorKind::Directory,
            writable: false,
        },
    }
}

/// The `DescriptorRole::CgroupDir` slot declaration (a directory, read-only — the
/// launcher passes it to `clone3(CLONE_INTO_CGROUP)`; the kernel consumes it at fork).
fn cgroup_slot() -> DescriptorSlotV1 {
    DescriptorSlotV1 {
        slot_index: slot_u32(SLOT_CGROUP),
        role: DescriptorRole::CgroupDir,
        expected: DescriptorShape {
            kind: DescriptorKind::Directory,
            writable: false,
        },
    }
}

/// The inputs `build_body` assembles a [`LinuxLaunchBodyV1`] from. Bundled to keep
/// `build_body` within the argument budget (no `too_many_arguments` lint, doctrine-clean).
struct BuildBody<'a> {
    plan: &'a BoundaryPlan,
    lowering: LoweringWireV1,
    table: Vec<DescriptorSlotV1>,
    exe: &'a str,
    args: &'a [String],
    envp: Vec<(String, String)>,
    deny_network: bool,
    seccomp_request: Option<SeccompRequest>,
}

/// Assemble the launcher body. Identity binding:
/// - `plan_id` is the REAL admitted-plan identity;
/// - `h_p` is the honest digest of the plan's bound profile snapshot
///   ([`BackendProfileHash::of`] over its canonical bytes);
/// - `h_l = blake3(canonical(lowering))` — the launcher re-derives + compares this
///   exact binding (the real `H_L`/schedule reconciliation is #75; do NOT invent a
///   different binding here);
/// - `attempt_id`/`h_a` are derived deterministically from the plan identity (the
///   `BoundaryPlan` carries neither, and the launcher does NOT verify them — it
///   checks ONLY `h_l`; the real attempt/admission-program threading is #75). They
///   are domain-separated so they never collide with each other or with `plan_id`.
fn build_body(b: BuildBody<'_>) -> Result<LinuxLaunchBodyV1, String> {
    let BuildBody {
        plan,
        lowering,
        table,
        exe,
        args,
        envp,
        deny_network,
        seccomp_request,
    } = b;
    let lowering_bytes = batpak::canonical::to_bytes(&lowering)
        .map_err(|e| format!("cannot canonically encode the lowering schedule: {e}"))?;
    let h_l: Digest32 = batpak::event::hash::compute_hash(&lowering_bytes);

    let profile_bytes = batpak::canonical::to_bytes(&plan.profile)
        .map_err(|e| format!("cannot canonically encode the profile snapshot: {e}"))?;
    let h_p = BackendProfileHash::of(&profile_bytes);

    // argv[0] is the conventional program name; the rest are the workload args.
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(exe.to_string());
    argv.extend(args.iter().cloned());

    Ok(LinuxLaunchBodyV1 {
        attempt_id: AttemptId(derive_id(plan.plan_id, b"bvisor.attempt.v1")),
        plan_id: plan.plan_id,
        h_a: AdmissionProgramHash(derive_id(plan.plan_id, b"bvisor.h_a.v1")),
        h_p,
        h_l,
        lowering,
        descriptor_table: table,
        target: TargetSpecV1 {
            argv,
            // The target environment is EXACTLY the lowered Environment::Exact table
            // (literals + parent-resolved leases) — nothing inherited. No implicit
            // PATH: the spec DECLARES every variable it needs (proof-spine §5 D2 —
            // platform-generated entries must be explicit, never invisible).
            envp,
            exe_slot: slot_u32(SLOT_EXE),
            // S9 / D3: an admitted NetworkDenyAll engages the empty network namespace —
            // which requires the S8 userns rendezvous (unprivileged CLONE_NEWNET needs the
            // child to be root-in-userns). So `deny_network` drives BOTH fields ON together;
            // OFF, BOTH stay `None` and the canonical bytes are byte-for-byte identical to
            // the pre-S8/S9 wire form (both fields are omitted from the encoding).
            user_namespace: deny_network.then(UserNsRequest::new),
            network_namespace: deny_network.then(NetworkNsRequest::new),
            // S10: the OPT-IN seccomp denylist request (ChildSpawn::DenyNewTasks and/or the
            // NetworkDenyAll socket DiD). `None` ⇒ no filter (the off-path bytes are
            // byte-for-byte unchanged — the field is omitted from the canonical encoding).
            seccomp: seccomp_request,
        },
    })
}

/// Derive a domain-separated digest from the plan identity. Used for the launch
/// identity fields the `BoundaryPlan` does not carry (`attempt_id`/`h_a`) so they
/// are deterministic + bound to plan identity yet never alias each other or
/// `plan_id`. The launcher does not verify these (it checks only `h_l`); the real
/// attempt/admission-program threading is #75.
fn derive_id(plan_id: BoundaryPlanHash, domain: &[u8]) -> Digest32 {
    let mut framed = Vec::with_capacity(domain.len() + 1 + 32);
    framed.extend_from_slice(domain);
    framed.push(0u8); // length-unambiguous separator (domain is NUL-free)
    framed.extend_from_slice(&plan_id.0);
    batpak::event::hash::compute_hash(&framed)
}

/// A slot fd number as the `u32` the descriptor table declares. The slot constants
/// are small positive literals, so the conversion cannot fail; on the impossible
/// negative it saturates to `u32::MAX` (a fd the launcher will fail to `fstat`,
/// fail-closed — never a silent wrong slot).
fn slot_u32(fd: RawFd) -> u32 {
    u32::try_from(fd).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod schedule_wire_tests {
    use super::{compile_lowering, ID_AMBIENT_SCRUB, ID_EXEC, ID_LANDLOCK_APPLY, ID_SECCOMP_APPLY};

    // The frozen wire phase codes the launcher's `classify_schedule` matches against
    // (`LoweringPhase::{FdHygiene, PolicyInstall, Launch}.code()`).
    const PHASE_SCRUB: u8 = 3;
    const PHASE_CONFINE: u8 = 4;
    const PHASE_EXEC: u8 = 5;

    #[test]
    fn compiled_wire_matches_the_launcher_served_shape() {
        // Confined + seccomp: the ONE validated compiler must emit the launcher's
        // canonical served order — scrub (FdHygiene) → landlock → seccomp
        // (PolicyInstall, id-tiebroken) → exec (Launch).
        let wire = compile_lowering(true, true).expect("schedule compiles");
        let shape: Vec<(&str, u8)> = wire
            .entries
            .iter()
            .map(|e| (e.id.as_str(), e.phase_code))
            .collect();
        assert_eq!(
            shape,
            vec![
                (ID_AMBIENT_SCRUB, PHASE_SCRUB),
                (ID_LANDLOCK_APPLY, PHASE_CONFINE),
                (ID_SECCOMP_APPLY, PHASE_CONFINE),
                (ID_EXEC, PHASE_EXEC),
            ],
            "PROPERTY: the compiled+projected wire IS the launcher's canonical served shape"
        );
    }

    #[test]
    fn unconfined_schedule_is_scrub_then_exec_only() {
        let wire = compile_lowering(false, false).expect("schedule compiles");
        let ids: Vec<&str> = wire.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![ID_AMBIENT_SCRUB, ID_EXEC],
            "PROPERTY: a no-confinement plan schedules only the mandatory scrub + exec"
        );
    }

    #[test]
    fn projected_entries_carry_real_compiled_digests_not_zero_sentinels() {
        // The point of routing through `compile_schedule`: the wire's per-entry digests
        // are the REAL compiled decl/param digests, not the old hand-built zero
        // sentinels. This is the observable difference that proves the two-truths is
        // gone (the launcher binds the whole projection via `h_l`, so non-zero digests
        // are self-consistent across host + launcher).
        let wire = compile_lowering(true, true).expect("schedule compiles");
        for entry in &wire.entries {
            assert_ne!(
                entry.decl_digest, [0u8; 32],
                "PROPERTY: decl_digest is the real compiled digest, never a zero sentinel"
            );
            assert_ne!(
                entry.param_digest, [0u8; 32],
                "PROPERTY: param_digest is the real compiled (empty-param) digest, never zero"
            );
        }
    }
}

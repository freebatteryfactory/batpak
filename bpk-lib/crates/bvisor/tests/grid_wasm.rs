// REAL wasmi/WASI confinement through WasmBackend::execute().
#![cfg(all(feature = "backend-wasm", feature = "dangerous-test-hooks"))]

use bvisor::{
    Backend, BackendRegistry, BoundaryPlanner, BoundaryReportBody, BoundarySpec, BudgetFinding,
    BudgetRequirements, Capability, EnvEntry, EnvPolicy, EvidenceRequirements, FdPolicy, FsAccess,
    FsConfinement, HostControl, MinGuarantee, NetDest, NetPolicy, Outcome, PathSet,
    RequirementKind, StdStreams, WasmBackend, Workload,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct WasmFixture {
    _dir: tempfile::TempDir,
    module: PathBuf,
}

impl WasmFixture {
    fn module_ref(&self) -> String {
        self.module.to_string_lossy().into_owned()
    }
}

struct WasiFsGroundTruth {
    marker: String,
    witness_path: PathBuf,
}

impl WasiFsGroundTruth {
    fn danger_occurred(&self) -> bool {
        marker_present(&self.witness_path, &self.marker)
    }
}

fn compile_guest(name: &str, wat_src: &str) -> WasmFixture {
    let dir = tempfile::tempdir().expect("guest module tempdir");
    let wasm = wat::parse_str(wat_src).expect("WAT fixture compiles to wasm");
    let module = dir.path().join(format!("{name}.wasm"));
    std::fs::write(&module, wasm).expect("write wasm fixture");
    WasmFixture { _dir: dir, module }
}

fn run(spec: &BoundarySpec) -> BoundaryReportBody {
    // Dogfood the public one-call API: run_confined does registry -> plan -> run.
    bvisor::run_confined(spec, Arc::new(WasmBackend::new()))
        .expect("WasmBackend runs the spec")
        .body
}

fn plan_only(spec: &BoundarySpec) -> (Arc<WasmBackend>, bvisor::BoundaryPlan) {
    let backend = Arc::new(WasmBackend::new());
    let id = backend.id();
    let mut registry = BackendRegistry::new();
    registry.register(Arc::clone(&backend) as Arc<dyn Backend>);
    let plan = BoundaryPlanner::new(&registry)
        .plan(spec, &id)
        .expect("WasmBackend admits the test spec");
    (backend, plan)
}

fn fs_spec(module_ref: String, access: FsAccess, root: &Path) -> BoundarySpec {
    BoundarySpec {
        workload: Workload::Wasm { module_ref },
        capabilities: vec![Capability::Filesystem {
            access,
            scope: PathSet {
                roots: vec![root.to_string_lossy().into_owned()],
            },
            recursive: true,
            confinement: FsConfinement::DeclaredRootsOnly,
        }],
        controls: vec![
            HostControl::LaunchWorkload,
            HostControl::CaptureStreams {
                streams: StdStreams::capture_out_err(),
            },
        ],
        budgets: BudgetRequirements::uniform(1_000_000, MinGuarantee::Mediated),
        evidence: EvidenceRequirements::default(),
    }
}

fn base_spec(module_ref: String) -> BoundarySpec {
    BoundarySpec {
        workload: Workload::Wasm { module_ref },
        capabilities: Vec::new(),
        controls: vec![
            HostControl::LaunchWorkload,
            HostControl::CaptureStreams {
                streams: StdStreams::capture_out_err(),
            },
        ],
        budgets: BudgetRequirements::uniform(1_000_000, MinGuarantee::Mediated),
        evidence: EvidenceRequirements::default(),
    }
}

fn marker_present(path: &Path, marker: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).contains(marker),
        Err(_) => false,
    }
}

fn observed_contains(body: &BoundaryReportBody, kind: &str, needle: &str) -> bool {
    body.observed
        .iter()
        .any(|f| f.kind == kind && f.detail.contains(needle))
}

#[test]
fn wasm_profile_admits_declared_roots_spec() {
    let root = tempfile::tempdir().expect("host root");
    let fixture = compile_guest(
        "stdout_marker",
        include_str!("fixtures/wasm/stdout_marker.wat"),
    );
    let spec = fs_spec(fixture.module_ref(), FsAccess::ReadWrite, root.path());
    let (backend, plan) = plan_only(&spec);
    let ceiling = backend.profile(&plan.profile);
    let enforced = ceiling.enforced_kinds();
    assert!(
        enforced.contains(&RequirementKind::Filesystem)
            && enforced.contains(&RequirementKind::LaunchWorkload)
            && enforced.contains(&RequirementKind::CaptureStreams),
        "wasm profile must advertise the enforced cells the runtime backs: {enforced:?}"
    );
    assert!(
        !plan.admitted.is_empty(),
        "declared-roots wasm spec must be admitted, not refused by an empty ceiling"
    );
}

#[test]
fn g1_wasi_preopen_denies_secret_read_outside_declared_root() {
    let scratch = tempfile::tempdir().expect("scratch");
    let root = scratch.path().join("root");
    std::fs::create_dir_all(&root).expect("root dir");
    let outside = scratch.path().join("outside.txt");
    let marker = "OUTSIDE-G1-MARKER".to_string();
    std::fs::write(&outside, "host-side outside marker").expect("outside marker");
    let exfil = root.join("exfil.txt");
    let fixture = compile_guest(
        "read_outside_preopen",
        include_str!("fixtures/wasm/read_outside_preopen.wat"),
    );

    let body = run(&fs_spec(fixture.module_ref(), FsAccess::ReadWrite, &root));
    let gt = WasiFsGroundTruth {
        marker,
        witness_path: exfil,
    };
    assert!(
        !gt.danger_occurred(),
        "GROUNDTRUTH: WASI preopen must block out-of-root read; marker landed on disk. report: {:?}",
        body.observed
    );
    assert_eq!(body.outcome, Outcome::Completed);
    assert!(body.denied.is_empty(), "denials are proven by disk oracle");
    assert!(
        observed_contains(&body, "filesystem_confined", "wasi preopen"),
        "report must attest the preopen mechanism: {:?}",
        body.observed
    );
}

#[test]
fn g3_wasi_preopen_denies_write_outside_quarantine() {
    let scratch = tempfile::tempdir().expect("scratch");
    let root = scratch.path().join("root");
    std::fs::create_dir_all(&root).expect("root dir");
    let escape = scratch.path().join("escape.txt");
    let fixture = compile_guest(
        "write_outside_preopen",
        include_str!("fixtures/wasm/write_outside_preopen.wat"),
    );

    let body = run(&fs_spec(fixture.module_ref(), FsAccess::ReadWrite, &root));
    let gt = WasiFsGroundTruth {
        marker: "OUTSIDE-G3-MARKER".to_string(),
        witness_path: escape,
    };
    assert!(
        !gt.danger_occurred(),
        "GROUNDTRUTH: WASI preopen must block out-of-root write; marker landed on disk. report: {:?}",
        body.observed
    );
    assert_eq!(body.outcome, Outcome::Completed);
    assert!(body.denied.is_empty(), "denials are proven by disk oracle");
}

#[test]
fn g7_wasi_preopen_commit_allows_in_root_output() {
    let root = tempfile::tempdir().expect("root");
    let fixture = compile_guest(
        "write_inside_preopen",
        include_str!("fixtures/wasm/write_inside_preopen.wat"),
    );
    let mut spec = fs_spec(fixture.module_ref(), FsAccess::ReadWrite, root.path());
    spec.controls.push(HostControl::CommitArtifact {
        durability: bvisor::CommitDurability::Atomic,
    });
    spec.controls.push(HostControl::ListOutputs);

    let body = run(&spec);
    let output = root.path().join("commit.txt");
    assert!(
        marker_present(&output, "COMMIT-G7-MARKER"),
        "GROUNDTRUTH: in-root committed output must land on the real host dir"
    );
    assert_eq!(body.outcome, Outcome::Completed);
    assert!(body.denied.is_empty(), "denials are proven by disk oracle");
    assert!(
        body.body_hash().is_ok(),
        "Wasm report body must be stable and sealable"
    );
}

#[test]
fn wasi_read_only_preopen_blocks_in_root_write() {
    let root = tempfile::tempdir().expect("root");
    let fixture = compile_guest(
        "write_inside_preopen",
        include_str!("fixtures/wasm/write_inside_preopen.wat"),
    );

    let body = run(&fs_spec(fixture.module_ref(), FsAccess::Read, root.path()));
    let output = root.path().join("commit.txt");
    assert!(
        !marker_present(&output, "COMMIT-G7-MARKER"),
        "GROUNDTRUTH: read-only WASI preopen must not allow an in-root write"
    );
    assert_eq!(
        body.outcome,
        Outcome::Completed,
        "guest handled the denied write by exiting cleanly; disk oracle proves denial"
    );
    assert!(body.denied.is_empty(), "denials are proven by disk oracle");
}

#[test]
fn fuel_exhaustion_maps_to_timeout() {
    let fixture = compile_guest("burn_fuel", include_str!("fixtures/wasm/burn_fuel.wat"));
    let mut spec = base_spec(fixture.module_ref());
    spec.budgets.cpu_micros.limit = 10;
    let body = run(&spec);
    assert_eq!(body.outcome, Outcome::Timeout);
    assert_eq!(
        body.budget.cpu_micros.finding,
        BudgetFinding::LimitReachedEnforced
    );
}

#[test]
fn wasi_socket_capability_is_absent() {
    let fixture = compile_guest("try_socket", include_str!("fixtures/wasm/try_socket.wat"));
    // DenyAll admission path runs to completion; the report records the mechanism.
    let mut deny = base_spec(fixture.module_ref());
    deny.capabilities.push(Capability::Network {
        policy: NetPolicy::DenyAll,
    });
    let body = run(&deny);
    assert_eq!(body.outcome, Outcome::Completed);
    assert_eq!(body.captured.stdout, Some("inline:13b".to_string()));
    assert!(
        observed_contains(&body, "network_deny_all", "no WASI socket"),
        "report must record the no-socket mechanism: {:?}",
        body.observed
    );

    // ANTI-VACUOUS witness. preview1 exposes no sock_open, so a guest can never
    // obtain a socket regardless of policy — a guest probe cannot witness a leak (a
    // leaked socket sits at an fd the guest never names; see the fixture note). The
    // property that ACTUALLY flips if a network capability is ever added is the
    // admissibility of a REACHABLE network: the wasm backend installs no allow-list
    // broker, so an AllowList request MUST be refused. A future broker would make it
    // admissible and red this. (The ceiling half also lives in coupling_proof_wasm.)
    let mut reachable = base_spec(fixture.module_ref());
    reachable.capabilities.push(Capability::Network {
        policy: NetPolicy::AllowList(vec![NetDest {
            host: "example".to_string(),
            port: 443,
        }]),
    });
    // Exercises run_confined's Err (admission-refused) path.
    assert!(
        bvisor::run_confined(&reachable, Arc::new(WasmBackend::new())).is_err(),
        "a reachable-network (AllowList) request must be refused — no network broker exists"
    );
}

#[test]
fn wasi_env_exact_is_delivered_to_guest() {
    let fixture = compile_guest("env_exact", include_str!("fixtures/wasm/env_exact.wat"));
    let mut spec = base_spec(fixture.module_ref());
    spec.capabilities.push(Capability::Environment {
        policy: EnvPolicy::Exact(vec![EnvEntry::literal("BV_GRID", "EXPECTED")]),
    });
    let body = run(&spec);
    assert_eq!(body.outcome, Outcome::Completed);
    assert_eq!(body.captured.stdout, Some("inline:15b".to_string()));
    assert!(
        observed_contains(&body, "environment_lowered", "entries=1"),
        "report must record explicit env lowering: {:?}",
        body.observed
    );
}

#[test]
fn wasi_temp_root_is_private_and_writable() {
    let fixture = compile_guest(
        "write_inside_preopen",
        include_str!("fixtures/wasm/write_inside_preopen.wat"),
    );
    let mut spec = base_spec(fixture.module_ref());
    spec.controls.push(HostControl::TempRoot {
        visibility: bvisor::PathView::PrivateToBoundary,
    });
    spec.controls.push(HostControl::DiscardArtifact);
    let body = run(&spec);
    assert_eq!(body.outcome, Outcome::Completed);
    assert_eq!(body.captured.stdout, Some("inline:16b".to_string()));
    assert!(
        observed_contains(&body, "wasm_note", "temp_root_removed="),
        "private temp root must be cleaned after run: {:?}",
        body.observed
    );
}

#[test]
fn wasi_capture_streams_records_guest_output() {
    let fixture = compile_guest(
        "stdout_marker",
        include_str!("fixtures/wasm/stdout_marker.wat"),
    );
    let mut spec = base_spec(fixture.module_ref());
    spec.evidence.require_captured_streams = true;
    let body = run(&spec);
    assert_eq!(body.outcome, Outcome::Completed);
    assert_eq!(body.captured.stdout, Some("inline:21b".to_string()));
    assert!(observed_contains(&body, "stream_captured", "21 stdout"));
}

#[test]
fn wasi_no_inherited_fds_contract_runs_without_raw_fd_authority() {
    let fixture = compile_guest(
        "stdout_marker",
        include_str!("fixtures/wasm/stdout_marker.wat"),
    );
    let mut spec = base_spec(fixture.module_ref());
    spec.capabilities.push(Capability::InheritedFds {
        policy: FdPolicy::None,
    });
    let body = run(&spec);
    assert_eq!(body.outcome, Outcome::Completed);
    assert!(
        observed_contains(&body, "inherited_fds_none", "WASI stdio"),
        "report must record that no raw host fds were inherited: {:?}",
        body.observed
    );
}

#![cfg(all(feature = "backend-wasm", feature = "dangerous-test-hooks"))]
//! Wasm qualification coupling gate.
//!
//! For the representative production Wasm profile, every `Enforced` ceiling cell
//! must have a `Proven` row in `WASM_QUALIFICATION_LEDGER`, a satisfied profile
//! floor, and a mechanism digest equal to the backend's live mechanism string.

use bvisor::{
    BackendProfile, Enforcement, MechanismDigest, ProfileFacts, QualificationStatus,
    RequirementKind, WasmBackend, WASM_QUALIFICATION_LEDGER,
};

#[derive(Debug, PartialEq, Eq)]
enum CouplingViolation {
    NoLedgerRow(RequirementKind),
    NotProven(RequirementKind, QualificationStatus),
    FloorNotSatisfied(RequirementKind),
    MechanismDigestMismatch(RequirementKind),
}

impl CouplingViolation {
    fn describe(&self) -> String {
        match self {
            Self::NoLedgerRow(kind) => {
                format!("Enforced Wasm cell {kind:?} has no ledger row")
            }
            Self::NotProven(kind, status) => {
                format!("Enforced Wasm cell {kind:?} is {status:?}, not Proven")
            }
            Self::FloorNotSatisfied(kind) => {
                format!("Enforced Wasm cell {kind:?} row floor is not satisfied")
            }
            Self::MechanismDigestMismatch(kind) => {
                format!("Enforced Wasm cell {kind:?} mechanism digest mismatches ledger")
            }
        }
    }
}

fn check_coupling(
    enforced: &[RequirementKind],
    facts: &ProfileFacts,
    mechanism_digest_of: &dyn Fn(RequirementKind) -> MechanismDigest,
) -> Result<(), CouplingViolation> {
    for &kind in enforced {
        let row = WASM_QUALIFICATION_LEDGER
            .iter()
            .find(|r| r.key == kind)
            .ok_or(CouplingViolation::NoLedgerRow(kind))?;
        if row.status != QualificationStatus::Proven {
            return Err(CouplingViolation::NotProven(kind, row.status));
        }
        if !row.profile_floor.satisfied_by(facts) {
            return Err(CouplingViolation::FloorNotSatisfied(kind));
        }
        if row.mechanism_digest() != mechanism_digest_of(kind) {
            return Err(CouplingViolation::MechanismDigestMismatch(kind));
        }
    }
    Ok(())
}

fn live_digest_fn(backend: &WasmBackend) -> impl Fn(RequirementKind) -> MechanismDigest + '_ {
    move |kind| MechanismDigest::of_mechanism(&backend.proof_mechanism(kind, Enforcement::Enforced))
}

fn coupling_for(backend: &WasmBackend) -> Result<(), CouplingViolation> {
    let ceiling: BackendProfile = backend.proof_ceiling();
    let enforced = ceiling.enforced_kinds();
    check_coupling(&enforced, &backend.proof_facts(), &live_digest_fn(backend))
}

#[test]
fn wasm_profile_couples_every_enforced_cell_to_a_proven_row() {
    let backend = WasmBackend::for_proof();
    let enforced = backend.proof_ceiling().enforced_kinds();
    for kind in [
        RequirementKind::Filesystem,
        RequirementKind::LaunchWorkload,
        RequirementKind::CaptureStreams,
        RequirementKind::Environment,
        RequirementKind::TempRoot,
        RequirementKind::CommitArtifact,
        RequirementKind::DiscardArtifact,
        RequirementKind::ListOutputs,
        RequirementKind::NetworkDenyAll,
        RequirementKind::InheritedFdsNone,
    ] {
        assert!(
            enforced.contains(&kind),
            "Wasm profile must advertise the proven enforced cell {kind:?}: {enforced:?}"
        );
    }
    for kind in [
        RequirementKind::ChildSpawnDenyNewTasks,
        RequirementKind::ChildSpawnAllowThreads,
        RequirementKind::ChildSpawnAllowDescendants,
        RequirementKind::Kill,
        RequirementKind::ExposePath,
        RequirementKind::NetworkAllowList,
        RequirementKind::InheritedFdsOnly,
    ] {
        assert!(
            !enforced.contains(&kind),
            "frozen unsupported Wasm cell {kind:?} must not enter the ceiling: {enforced:?}"
        );
    }
    let result = coupling_for(&backend);
    assert!(
        result.is_ok(),
        "Wasm profile must couple every Enforced cell to Proven ledger rows: {}",
        result.err().map_or_else(String::new, |v| v.describe())
    );
}

#[test]
fn wasm_ledger_mechanism_digest_equals_the_backend_live_digest() {
    let backend = WasmBackend::for_proof();
    let row = WASM_QUALIFICATION_LEDGER
        .iter()
        .find(|r| r.key == RequirementKind::Filesystem)
        .expect("Filesystem ledger row");
    let live = MechanismDigest::of_mechanism(
        &backend.proof_mechanism(RequirementKind::Filesystem, Enforcement::Enforced),
    );
    assert_eq!(
        row.mechanism_digest(),
        live,
        "Wasm Filesystem ledger mechanism must equal the backend's live mechanism"
    );
}

#[test]
fn swapped_wasm_mechanism_is_rejected_by_digest_match() {
    let backend = WasmBackend::for_proof();
    let enforced = backend.proof_ceiling().enforced_kinds();
    let liar = |kind: RequirementKind| {
        if kind == RequirementKind::Filesystem {
            MechanismDigest::of_mechanism("wasm:SWAPPED-UNPROVEN-MECHANISM:Enforced")
        } else {
            MechanismDigest::of_mechanism(&backend.proof_mechanism(kind, Enforcement::Enforced))
        }
    };
    let result = check_coupling(&enforced, &backend.proof_facts(), &liar);
    assert!(
        matches!(
            result,
            Err(CouplingViolation::MechanismDigestMismatch(
                RequirementKind::Filesystem
            ))
        ),
        "a swapped Wasm Filesystem mechanism must be rejected, got {result:?}"
    );
}

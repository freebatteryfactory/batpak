use crate::proof::ProofUnitTerminal;
use super::*;

/// One observed piece of evidence: the same axes the requirement declares
/// (the route rides inside `basis`), plus the execution facts admission
/// interrogates. It carries NO enforcement — policy is authored, never
/// observed. This is a claim ABOUT an execution, not a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequirementEvidenceObservation {
    pub method: VerificationMethod,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub lane: VerificationLane,
    /// The proof actually executed (a plan is not an execution).
    pub executed: bool,
    /// The evidence still binds the current source, policy, model,
    /// toolchain, profile, and affected boundary.
    pub fresh: bool,
    /// Every required artifact is bound to the evidence.
    pub artifacts_bound: bool,
    /// A counterexample from any witness remains standing.
    pub counterexample_outstanding: bool,
    pub terminal: ProofUnitTerminal,
}

/// Why an observation fails to be admitted against an admitted requirement.
/// Every axis matches EXACTLY — there is no coverage ranking and no lane
/// substitution. The terminal must be a conclusive semantic verdict; a
/// non-verdict terminal (unsupported, skipped, expired, superseded) is not an
/// observation of anything and is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAdmissionError {
    MethodMismatch { required: VerificationMethod, observed: VerificationMethod },
    /// The observed basis (route included) does not equal the required basis.
    BasisMismatch,
    CoverageMismatch { required: VerificationCoverage, observed: VerificationCoverage },
    LaneMismatch { required: VerificationLane, observed: VerificationLane },
    NotExecuted,
    Stale,
    ArtifactsUnbound,
    CounterexampleOutstanding,
    /// The terminal is not a conclusive semantic verdict.
    InconclusiveTerminal { terminal: ProofUnitTerminal },
}

/// An observation admitted against one admitted requirement: shape- and
/// execution-coherent, carrying a conclusive verdict. Sealed:
/// `admit_observation` is the only constructor. Admission is not a pass/fail
/// certificate — the retained terminal is the verdict the jury adjudicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedRequirementObservation {
    requirement: VerificationRequirement,
    terminal: ProofUnitTerminal,
    _seal: (),
}

impl AdmittedRequirementObservation {
    pub const fn requirement(&self) -> &VerificationRequirement {
        &self.requirement
    }
    pub const fn terminal(&self) -> ProofUnitTerminal {
        self.terminal
    }
}

/// Whether a terminal is a conclusive semantic verdict. `Passed`, `Failed`,
/// and `Refused` conclude (a refusal is a verdict); the remaining terminals
/// carry no verdict — a platform that cannot express the obligation, an
/// authorized skip, a lapsed freshness bound, or a superseded row observe
/// nothing this admission can adjudicate.
const fn terminal_is_conclusive_verdict(terminal: ProofUnitTerminal) -> bool {
    matches!(
        terminal,
        ProofUnitTerminal::Passed | ProofUnitTerminal::Failed | ProofUnitTerminal::Refused
    )
}

/// Admit an observation against an admitted requirement. Exact match on every
/// axis through derived equality — no axis converts to a number, ranks
/// another, or substitutes for one. Refusal on any execution defect or a
/// non-verdict terminal. Total and panic-free.
pub fn admit_observation(
    admitted: &AdmittedVerificationRequirement,
    observation: &RequirementEvidenceObservation,
) -> Result<AdmittedRequirementObservation, ObservationAdmissionError> {
    let required = admitted.requirement;
    if required.method != observation.method {
        return Err(ObservationAdmissionError::MethodMismatch {
            required: required.method,
            observed: observation.method,
        });
    }
    if required.basis != observation.basis {
        return Err(ObservationAdmissionError::BasisMismatch);
    }
    if required.coverage != observation.coverage {
        return Err(ObservationAdmissionError::CoverageMismatch {
            required: required.coverage,
            observed: observation.coverage,
        });
    }
    if required.lane != observation.lane {
        return Err(ObservationAdmissionError::LaneMismatch {
            required: required.lane,
            observed: observation.lane,
        });
    }
    if !observation.executed {
        return Err(ObservationAdmissionError::NotExecuted);
    }
    if !observation.fresh {
        return Err(ObservationAdmissionError::Stale);
    }
    if !observation.artifacts_bound {
        return Err(ObservationAdmissionError::ArtifactsUnbound);
    }
    if observation.counterexample_outstanding {
        return Err(ObservationAdmissionError::CounterexampleOutstanding);
    }
    if !terminal_is_conclusive_verdict(observation.terminal) {
        return Err(ObservationAdmissionError::InconclusiveTerminal {
            terminal: observation.terminal,
        });
    }
    Ok(AdmittedRequirementObservation {
        requirement: required,
        terminal: observation.terminal,
        _seal: (),
    })
}

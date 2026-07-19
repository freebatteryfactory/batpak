use crate::proof::ProofUnitTerminal;
use super::*;

/// Why a whole proof-row plan fails to assess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowAssessmentError {
    /// An active row must carry at least one requirement.
    EmptyPlan,
    /// A plan lists the same requirement twice.
    DuplicateRequirement { index: usize },
    /// A plan requirement is incoherent as authored.
    Inadmissible { index: usize, error: RequirementAdmissionError },
    /// Fewer or more observation entries than plan requirements.
    ObservationCountMismatch { required: usize, presented: usize },
    /// A blocking requirement's observation failed admission. Reported after
    /// every observation is validated, and BEFORE any witness disagreement:
    /// an invalid observation is a defect, never a vote.
    BlockingRequirementFailed { index: usize, error: ObservationAdmissionError },
    /// Two admitted projection witnesses disagree (SEED-MODEL-TRACE-SEPARATION).
    ProjectionWitnessesDisagree { first: ProofUnitTerminal, second: ProofUnitTerminal },
    /// Two admitted independent-reference witnesses disagree.
    IndependentWitnessesDisagree { first: ProofUnitTerminal, second: ProofUnitTerminal },
    /// A projection-derived witness and an independent-reference witness
    /// disagree. The proof refuses; it never chooses the generated
    /// projection (SEED-MODEL-TRACE-SEPARATION).
    ProjectionDisagreesWithIndependentReference {
        projection: ProofUnitTerminal,
        independent: ProofUnitTerminal,
    },
}

/// A fully assessed proof row: every observation was validated against its
/// admitted requirement, the blocking observations all admitted, and no jury
/// of witnesses disagreed. Sealed: `assess_plan` is the only constructor.
/// Advisory failures are counted and never block; quarantine failures are
/// counted and drop `is_positive` to false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowPlanAssessment {
    claim: VerificationClaimKind,
    satisfied: usize,
    advisory_failures: usize,
    quarantine_failures: usize,
    _seal: (),
}

impl ProofRowPlanAssessment {
    pub const fn claim(&self) -> VerificationClaimKind {
        self.claim
    }
    pub const fn satisfied(&self) -> usize {
        self.satisfied
    }
    pub const fn advisory_failures(&self) -> usize {
        self.advisory_failures
    }
    pub const fn quarantine_failures(&self) -> usize {
        self.quarantine_failures
    }
    /// Positive assessment: no quarantine failures. Advisory failures remain
    /// visible and never block, never confer.
    pub const fn is_positive(&self) -> bool {
        self.quarantine_failures == 0
    }
}

/// Assess one active proof row: its claim, its authored plan, and one
/// observation per plan requirement, in plan order.
///
/// The order is the law (it kills last-witness-wins and
/// invalid-evidence-triggers-disagreement): validate the whole plan, then
/// validate every observation, then report the first blocking failure, and
/// only then convene the jury. An invalid observation can never masquerade as
/// a witness in a disagreement.
pub fn assess_plan(
    claim: VerificationClaimKind,
    plan: &[VerificationRequirement],
    observations: &[RequirementEvidenceObservation],
) -> Result<ProofRowPlanAssessment, RowAssessmentError> {
    // 1a. A row must carry a plan.
    if plan.is_empty() {
        return Err(RowAssessmentError::EmptyPlan);
    }
    // 1b. No requirement is listed twice.
    let mut i = 0;
    while i < plan.len() {
        let mut j = 0;
        while j < i {
            if plan[j] == plan[i] {
                return Err(RowAssessmentError::DuplicateRequirement { index: i });
            }
            j += 1;
        }
        i += 1;
    }
    // 1c. Every requirement is admissible as authored.
    let mut i = 0;
    while i < plan.len() {
        if let Err(error) = admit(plan[i]) {
            return Err(RowAssessmentError::Inadmissible { index: i, error });
        }
        i += 1;
    }
    // 2. One observation per requirement.
    if observations.len() != plan.len() {
        return Err(RowAssessmentError::ObservationCountMismatch {
            required: plan.len(),
            presented: observations.len(),
        });
    }
    // 3. Validate every observation before comparing any. Collect the first
    // blocking failure but keep validating; count advisory and quarantine
    // failures; retain admitted terminals per basis family for the jury.
    let mut satisfied = 0usize;
    let mut advisory_failures = 0usize;
    let mut quarantine_failures = 0usize;
    let mut first_blocking: Option<(usize, ObservationAdmissionError)> = None;
    let mut projection_first: Option<ProofUnitTerminal> = None;
    let mut projection_conflict: Option<ProofUnitTerminal> = None;
    let mut independent_first: Option<ProofUnitTerminal> = None;
    let mut independent_conflict: Option<ProofUnitTerminal> = None;
    let mut index = 0;
    while index < plan.len() {
        let admitted = match admit(plan[index]) {
            Ok(admitted) => admitted,
            Err(error) => return Err(RowAssessmentError::Inadmissible { index, error }),
        };
        match admit_observation(&admitted, &observations[index]) {
            Ok(admitted_observation) => {
                satisfied += 1;
                let terminal = admitted_observation.terminal();
                match observations[index].basis {
                    VerificationBasis::ContractProjection => match projection_first {
                        None => projection_first = Some(terminal),
                        Some(first) => {
                            if terminal != first && projection_conflict.is_none() {
                                projection_conflict = Some(terminal);
                            }
                        }
                    },
                    VerificationBasis::IndependentReference { .. } => match independent_first {
                        None => independent_first = Some(terminal),
                        Some(first) => {
                            if terminal != first && independent_conflict.is_none() {
                                independent_conflict = Some(terminal);
                            }
                        }
                    },
                    VerificationBasis::DirectBoundary { .. }
                    | VerificationBasis::RuntimeObservation => {}
                }
            }
            Err(error) => match plan[index].enforcement {
                VerificationEnforcementPosture::Blocking => {
                    if first_blocking.is_none() {
                        first_blocking = Some((index, error));
                    }
                }
                VerificationEnforcementPosture::Quarantine => quarantine_failures += 1,
                VerificationEnforcementPosture::Advisory => advisory_failures += 1,
            },
        }
        index += 1;
    }
    // 4. A blocking failure is reported before any disagreement.
    if let Some((index, error)) = first_blocking {
        return Err(RowAssessmentError::BlockingRequirementFailed { index, error });
    }
    // 5. Jury over admitted observations only. Intra-family first (projection,
    // then independent), then cross-family.
    if let (Some(first), Some(second)) = (projection_first, projection_conflict) {
        return Err(RowAssessmentError::ProjectionWitnessesDisagree { first, second });
    }
    if let (Some(first), Some(second)) = (independent_first, independent_conflict) {
        return Err(RowAssessmentError::IndependentWitnessesDisagree { first, second });
    }
    if let (Some(projection), Some(independent)) = (projection_first, independent_first) {
        if projection != independent {
            return Err(RowAssessmentError::ProjectionDisagreesWithIndependentReference {
                projection,
                independent,
            });
        }
    }
    // 6. A coherent row.
    Ok(ProofRowPlanAssessment {
        claim,
        satisfied,
        advisory_failures,
        quarantine_failures,
        _seal: (),
    })
}

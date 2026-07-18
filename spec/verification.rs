//! Typed verification plane (5.5F2; DEC-077/DEC-078; docs/38).
//!
//! docs/38 owns the layered-axes law in prose; this file is the typed
//! authority for the closed vocabularies and the sealed qualification
//! algebra that law depends on. Every axis is a separate type: no variant
//! here derives or implements an ordering, exposes a rank, level, or score,
//! or converts to a number — an aggregate assurance ladder is forbidden by
//! shape (SEED-LAYERED-DYNAMIC-VERIFICATION), and the audit refuses one.
//!
//! The Tier 0 cross-run law is an ANALOGY for why self-corroboration is
//! refused; nothing here imports or reuses `tier0_cross_run` as product
//! verification authority.

use crate::proof::ProofUnitTerminal;

/// The relationship of the evidence source to the subject and its semantic
/// owner (docs/38 section 2). A generated projection or a self-observation
/// can never be the sole independent oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationBasis {
    /// Generated from typed authority; drives inputs, never concludes.
    ContractProjection,
    /// A separately owned model, evaluator, or relation authored against the law.
    IndependentReference,
    /// A hostile probe of the real boundary under test.
    DirectBoundary,
    /// Evidence recorded from an executing system.
    RuntimeObservation,
}

pub const VERIFICATION_BASES: &[VerificationBasis] = &[
    VerificationBasis::ContractProjection,
    VerificationBasis::IndependentReference,
    VerificationBasis::DirectBoundary,
    VerificationBasis::RuntimeObservation,
];

/// The evidence-producing TECHNIQUE (docs/38 section 1). Method is never a
/// strength: "exhaustive" and "sampled" are coverage facts and no method
/// variant names a tool — a technique class survives only while at least two
/// admissible adapters or one observable evidence property defines it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationMethod {
    StructuralRule,
    CompileRefusal,
    PropertySequence,
    BoundedStateExploration,
    ScheduleExploration,
    DeterministicSimulation,
    DifferentialExecution,
    TranslationValidation,
    FaultInjection,
    CrashRecovery,
    Fuzzing,
    Mutation,
    ComplexityContract,
    BenchmarkEnvelope,
    HistoryReplay,
}

pub const VERIFICATION_METHODS: &[VerificationMethod] = &[
    VerificationMethod::StructuralRule,
    VerificationMethod::CompileRefusal,
    VerificationMethod::PropertySequence,
    VerificationMethod::BoundedStateExploration,
    VerificationMethod::ScheduleExploration,
    VerificationMethod::DeterministicSimulation,
    VerificationMethod::DifferentialExecution,
    VerificationMethod::TranslationValidation,
    VerificationMethod::FaultInjection,
    VerificationMethod::CrashRecovery,
    VerificationMethod::Fuzzing,
    VerificationMethod::Mutation,
    VerificationMethod::ComplexityContract,
    VerificationMethod::BenchmarkEnvelope,
    VerificationMethod::HistoryReplay,
];

/// The property being challenged (docs/38 section 5). The kinds are distinct;
/// one is never a weaker or stronger form of another, and the near neighbors
/// (Liveness / BoundedResponse / Convergence / NonOscillation) are distinct
/// on purpose: a limit-cycle can meet deadlines, terminate, and sit in a
/// bounded region while admitting no new evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationClaimKind {
    Safety,
    Liveness,
    BoundedResponse,
    Convergence,
    Stability,
    NonOscillation,
    Determinism,
    Refinement,
    Conformance,
    ResourceEnvelope,
}

pub const VERIFICATION_CLAIM_KINDS: &[VerificationClaimKind] = &[
    VerificationClaimKind::Safety,
    VerificationClaimKind::Liveness,
    VerificationClaimKind::BoundedResponse,
    VerificationClaimKind::Convergence,
    VerificationClaimKind::Stability,
    VerificationClaimKind::NonOscillation,
    VerificationClaimKind::Determinism,
    VerificationClaimKind::Refinement,
    VerificationClaimKind::Conformance,
    VerificationClaimKind::ResourceEnvelope,
];

/// Coverage strength (docs/38 section 4): distinct kinds of covered space,
/// not an ordered ladder. A requirement declaring one strength is satisfied
/// only by evidence of exactly that strength — exhaustive evidence does not
/// satisfy a sampled requirement, and coverage of a declared model is not
/// coverage of its law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationCoverage {
    Sampled,
    Bounded,
    ExhaustiveWithinDeclaredModel,
    ObservedHistory,
}

pub const VERIFICATION_COVERAGES: &[VerificationCoverage] = &[
    VerificationCoverage::Sampled,
    VerificationCoverage::Bounded,
    VerificationCoverage::ExhaustiveWithinDeclaredModel,
    VerificationCoverage::ObservedHistory,
];

/// Gate posture of a requirement — distinct from the runtime
/// refuse-or-append behavior law (docs/38 section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationEnforcementPosture {
    Blocking,
    Quarantine,
    Advisory,
}

pub const VERIFICATION_ENFORCEMENT_POSTURES: &[VerificationEnforcementPosture] = &[
    VerificationEnforcementPosture::Blocking,
    VerificationEnforcementPosture::Quarantine,
    VerificationEnforcementPosture::Advisory,
];

/// Where the requirement executes. A requirement is satisfied in its
/// declared lane, not a faster one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationLane {
    PullRequestFast,
    Merge,
    Scheduled,
    Release,
    Runtime,
}

pub const VERIFICATION_LANES: &[VerificationLane] = &[
    VerificationLane::PullRequestFast,
    VerificationLane::Merge,
    VerificationLane::Scheduled,
    VerificationLane::Release,
    VerificationLane::Runtime,
];

/// WHICH kind of independent route supplies independence (docs/38 section 7).
/// Closed by adoption, not aspiration: every variant is consumed by at least
/// one active proof-row plan today, and a route kind without a live adopter
/// does not exist here — it arrives with its adopter
/// (`PromotionRequirement::IndependentEvidenceRoute` names the requirement
/// that at least one admitted route exist; this vocabulary names which).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndependentEvidenceRouteKind {
    /// A second, separately owned implementation executed differentially.
    DifferentialImplementation,
    /// A hostile probe of the real boundary, owned apart from the subject.
    HostileBoundary,
    /// Independent re-execution of a recorded history against the admitted model.
    IndependentHistoryReplay,
}

pub const INDEPENDENT_EVIDENCE_ROUTE_KINDS: &[IndependentEvidenceRouteKind] = &[
    IndependentEvidenceRouteKind::DifferentialImplementation,
    IndependentEvidenceRouteKind::HostileBoundary,
    IndependentEvidenceRouteKind::IndependentHistoryReplay,
];

/// The three runtime verification modes (docs/38 section 6). Each mode owns
/// its lawful results through `admits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeVerificationMode {
    PreventiveGuard,
    InBandObservation,
    OfflineReplay,
}

pub const RUNTIME_VERIFICATION_MODES: &[RuntimeVerificationMode] = &[
    RuntimeVerificationMode::PreventiveGuard,
    RuntimeVerificationMode::InBandObservation,
    RuntimeVerificationMode::OfflineReplay,
];

/// Results a runtime verification may carry. Which results a mode may emit
/// is the `admits` matrix — an in-band monitor can state that no divergence
/// was observed in this trace, never conformance; only independent offline
/// replay of a complete, current observed history may state conformance for
/// that history, and even that says nothing about unobserved executions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeVerificationDisposition {
    GuardAdmitted,
    GuardRefused,
    NoDivergenceObserved,
    ConformantForObservedHistory,
    Divergent,
    Incomplete,
    Stale,
    Unsupported,
}

pub const RUNTIME_VERIFICATION_DISPOSITIONS: &[RuntimeVerificationDisposition] = &[
    RuntimeVerificationDisposition::GuardAdmitted,
    RuntimeVerificationDisposition::GuardRefused,
    RuntimeVerificationDisposition::NoDivergenceObserved,
    RuntimeVerificationDisposition::ConformantForObservedHistory,
    RuntimeVerificationDisposition::Divergent,
    RuntimeVerificationDisposition::Incomplete,
    RuntimeVerificationDisposition::Stale,
    RuntimeVerificationDisposition::Unsupported,
];

impl RuntimeVerificationMode {
    /// The exact mode/result admission matrix (docs/38 section 6).
    /// Exhaustive on both axes: a new mode or disposition must be classified
    /// here, not defaulted.
    pub const fn admits(self, disposition: RuntimeVerificationDisposition) -> bool {
        use RuntimeVerificationDisposition as D;
        match self {
            RuntimeVerificationMode::PreventiveGuard => matches!(
                disposition,
                D::GuardAdmitted | D::GuardRefused | D::Unsupported
            ),
            RuntimeVerificationMode::InBandObservation => matches!(
                disposition,
                D::NoDivergenceObserved | D::Divergent | D::Incomplete | D::Stale | D::Unsupported
            ),
            RuntimeVerificationMode::OfflineReplay => matches!(
                disposition,
                D::ConformantForObservedHistory
                    | D::Divergent
                    | D::Incomplete
                    | D::Stale
                    | D::Unsupported
            ),
        }
    }
}

/// One authored verification requirement: the axes a piece of evidence must
/// match EXACTLY to qualify. No axis ranks another and no strength converts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationRequirement {
    pub method: VerificationMethod,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub lane: VerificationLane,
    pub enforcement: VerificationEnforcementPosture,
    /// The independent route this requirement demands, when it demands one.
    /// A `ContractProjection` can never supply one (admission refuses it).
    pub independent_route: Option<IndependentEvidenceRouteKind>,
}

/// Why a requirement cannot be admitted as authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementAdmissionError {
    /// A projection generates inputs and never concludes; it cannot be the
    /// basis that supplies an independent evidence route.
    ProjectionCannotSupplyIndependentRoute,
    /// Runtime observation covers only executions that occurred.
    RuntimeObservationRequiresObservedHistory,
    /// Observed-history coverage exists only for runtime observation.
    ObservedHistoryRequiresRuntimeObservation,
    /// The runtime lane carries runtime-observation evidence only.
    RuntimeLaneRequiresRuntimeObservation,
    /// Runtime observation executes in the runtime lane only.
    RuntimeObservationRequiresRuntimeLane,
}

/// A requirement that passed admission. Sealed: `admit` is the only
/// constructor, so an incoherent requirement is unrepresentable downstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedVerificationRequirement {
    requirement: VerificationRequirement,
    _seal: (),
}

impl AdmittedVerificationRequirement {
    pub const fn requirement(&self) -> &VerificationRequirement {
        &self.requirement
    }
}

/// Admit an authored requirement, refusing the incoherent shapes.
pub const fn admit(
    requirement: VerificationRequirement,
) -> Result<AdmittedVerificationRequirement, RequirementAdmissionError> {
    if matches!(requirement.basis, VerificationBasis::ContractProjection)
        && requirement.independent_route.is_some()
    {
        return Err(RequirementAdmissionError::ProjectionCannotSupplyIndependentRoute);
    }
    if matches!(requirement.basis, VerificationBasis::RuntimeObservation)
        && !matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
    {
        return Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory);
    }
    if matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
        && !matches!(requirement.basis, VerificationBasis::RuntimeObservation)
    {
        return Err(RequirementAdmissionError::ObservedHistoryRequiresRuntimeObservation);
    }
    if matches!(requirement.lane, VerificationLane::Runtime)
        && !matches!(requirement.basis, VerificationBasis::RuntimeObservation)
    {
        return Err(RequirementAdmissionError::RuntimeLaneRequiresRuntimeObservation);
    }
    if matches!(requirement.basis, VerificationBasis::RuntimeObservation)
        && !matches!(requirement.lane, VerificationLane::Runtime)
    {
        return Err(RequirementAdmissionError::RuntimeObservationRequiresRuntimeLane);
    }
    Ok(AdmittedVerificationRequirement { requirement, _seal: () })
}

/// Evidence as executed: the same axes the requirement declares, plus the
/// execution facts qualification interrogates. This is a claim ABOUT an
/// execution, not a proof — `qualify` is what turns it into one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutedRequirementEvidence {
    pub method: VerificationMethod,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub lane: VerificationLane,
    pub enforcement: VerificationEnforcementPosture,
    pub route: Option<IndependentEvidenceRouteKind>,
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

/// Why executed evidence fails to qualify against an admitted requirement.
/// Every axis matches EXACTLY — there is no coverage ranking, no lane
/// substitution, and a green terminal alone is never sufficient
/// (`ProofUnitTerminal::is_positive_semantic_terminal` is one input among
/// eleven, never the verdict).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementQualificationError {
    MethodMismatch { required: VerificationMethod, executed: VerificationMethod },
    BasisMismatch { required: VerificationBasis, executed: VerificationBasis },
    CoverageMismatch { required: VerificationCoverage, executed: VerificationCoverage },
    LaneMismatch { required: VerificationLane, executed: VerificationLane },
    EnforcementMismatch {
        required: VerificationEnforcementPosture,
        executed: VerificationEnforcementPosture,
    },
    /// The requirement demands an independent route the evidence lacks or
    /// mislabels.
    RouteMismatch {
        required: Option<IndependentEvidenceRouteKind>,
        executed: Option<IndependentEvidenceRouteKind>,
    },
    NotExecuted,
    Stale,
    ArtifactsUnbound,
    CounterexampleOutstanding,
    /// The terminal is not a positive semantic terminal.
    NotPositiveSemanticTerminal { terminal: ProofUnitTerminal },
}

/// Evidence that qualified against one admitted requirement. Sealed:
/// `qualify` is the only constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualifiedRequirementEvidence {
    requirement: VerificationRequirement,
    terminal: ProofUnitTerminal,
    _seal: (),
}

impl QualifiedRequirementEvidence {
    pub const fn requirement(&self) -> &VerificationRequirement {
        &self.requirement
    }
    pub const fn terminal(&self) -> ProofUnitTerminal {
        self.terminal
    }
}

/// Qualify executed evidence against an admitted requirement. Exact match on
/// every axis through derived equality — no axis converts to a number, ranks
/// another, or substitutes for one. Refusal on any execution defect. Total
/// and panic-free.
pub fn qualify(
    admitted: &AdmittedVerificationRequirement,
    evidence: &ExecutedRequirementEvidence,
) -> Result<QualifiedRequirementEvidence, RequirementQualificationError> {
    let required = admitted.requirement;
    if required.method != evidence.method {
        return Err(RequirementQualificationError::MethodMismatch {
            required: required.method,
            executed: evidence.method,
        });
    }
    if required.basis != evidence.basis {
        return Err(RequirementQualificationError::BasisMismatch {
            required: required.basis,
            executed: evidence.basis,
        });
    }
    if required.coverage != evidence.coverage {
        return Err(RequirementQualificationError::CoverageMismatch {
            required: required.coverage,
            executed: evidence.coverage,
        });
    }
    if required.lane != evidence.lane {
        return Err(RequirementQualificationError::LaneMismatch {
            required: required.lane,
            executed: evidence.lane,
        });
    }
    if required.enforcement != evidence.enforcement {
        return Err(RequirementQualificationError::EnforcementMismatch {
            required: required.enforcement,
            executed: evidence.enforcement,
        });
    }
    if required.independent_route != evidence.route {
        return Err(RequirementQualificationError::RouteMismatch {
            required: required.independent_route,
            executed: evidence.route,
        });
    }
    if !evidence.executed {
        return Err(RequirementQualificationError::NotExecuted);
    }
    if !evidence.fresh {
        return Err(RequirementQualificationError::Stale);
    }
    if !evidence.artifacts_bound {
        return Err(RequirementQualificationError::ArtifactsUnbound);
    }
    if evidence.counterexample_outstanding {
        return Err(RequirementQualificationError::CounterexampleOutstanding);
    }
    if !evidence.terminal.is_positive_semantic_terminal() {
        return Err(RequirementQualificationError::NotPositiveSemanticTerminal {
            terminal: evidence.terminal,
        });
    }
    Ok(QualifiedRequirementEvidence {
        requirement: required,
        terminal: evidence.terminal,
        _seal: (),
    })
}

/// Why a whole proof-row plan fails to qualify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowQualificationError {
    /// An active row must carry at least one requirement.
    EmptyPlan,
    /// A plan lists the same requirement twice.
    DuplicateRequirement { index: usize },
    /// A plan requirement is incoherent as authored.
    Inadmissible { index: usize, error: RequirementAdmissionError },
    /// Fewer or more evidence entries than plan requirements.
    EvidenceCountMismatch { required: usize, presented: usize },
    /// One requirement's evidence failed qualification.
    RequirementFailed { index: usize, error: RequirementQualificationError },
    /// A projection-derived witness and an independent-reference witness
    /// disagree. The proof refuses; it never chooses the generated
    /// projection (SEED-MODEL-TRACE-SEPARATION).
    ProjectionDisagreesWithIndependentReference {
        projection: ProofUnitTerminal,
        independent: ProofUnitTerminal,
    },
}

/// A fully qualified proof row: every requirement in the plan matched its
/// executed evidence exactly, and no projection witness overruled an
/// independent one. Sealed: `qualify_row` is the only constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowQualification {
    claim: VerificationClaimKind,
    requirements: usize,
    _seal: (),
}

impl ProofRowQualification {
    pub const fn claim(&self) -> VerificationClaimKind {
        self.claim
    }
    pub const fn requirements(&self) -> usize {
        self.requirements
    }
}

/// Qualify one active proof row: its claim, its authored plan, and one
/// executed-evidence entry per plan requirement, in plan order.
pub fn qualify_row(
    claim: VerificationClaimKind,
    plan: &[VerificationRequirement],
    evidence: &[ExecutedRequirementEvidence],
) -> Result<ProofRowQualification, RowQualificationError> {
    if plan.is_empty() {
        return Err(RowQualificationError::EmptyPlan);
    }
    let mut i = 0;
    while i < plan.len() {
        let mut j = 0;
        while j < i {
            if plan[j] == plan[i] {
                return Err(RowQualificationError::DuplicateRequirement { index: i });
            }
            j += 1;
        }
        i += 1;
    }
    if evidence.len() != plan.len() {
        return Err(RowQualificationError::EvidenceCountMismatch {
            required: plan.len(),
            presented: evidence.len(),
        });
    }
    // A projection-derived witness may corroborate; it may never overrule an
    // independent reference. Disagreement refuses the whole row.
    let mut projection: Option<ProofUnitTerminal> = None;
    let mut independent: Option<ProofUnitTerminal> = None;
    for entry in evidence {
        match entry.basis {
            VerificationBasis::ContractProjection => projection = Some(entry.terminal),
            VerificationBasis::IndependentReference => independent = Some(entry.terminal),
            VerificationBasis::DirectBoundary | VerificationBasis::RuntimeObservation => {}
        }
    }
    if let (Some(p), Some(ind)) = (projection, independent) {
        if p != ind {
            return Err(RowQualificationError::ProjectionDisagreesWithIndependentReference {
                projection: p,
                independent: ind,
            });
        }
    }
    let mut index = 0;
    while index < plan.len() {
        let admitted = match admit(plan[index]) {
            Ok(admitted) => admitted,
            Err(error) => return Err(RowQualificationError::Inadmissible { index, error }),
        };
        if let Err(error) = qualify(&admitted, &evidence[index]) {
            return Err(RowQualificationError::RequirementFailed { index, error });
        }
        index += 1;
    }
    Ok(ProofRowQualification { claim, requirements: plan.len(), _seal: () })
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn base_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
            independent_route: Some(IndependentEvidenceRouteKind::HostileBoundary),
        }
    }

    const fn matching_evidence() -> ExecutedRequirementEvidence {
        ExecutedRequirementEvidence {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
            route: Some(IndependentEvidenceRouteKind::HostileBoundary),
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal: ProofUnitTerminal::Passed,
        }
    }

    #[test]
    fn matching_evidence_qualifies() {
        let admitted = admit(base_requirement()).expect("coherent requirement admits");
        let qualified = qualify(&admitted, &matching_evidence()).expect("exact match qualifies");
        assert_eq!(qualified.terminal(), ProofUnitTerminal::Passed);
    }

    #[test]
    fn projection_cannot_supply_independent_route() {
        let mut requirement = base_requirement();
        requirement.basis = VerificationBasis::ContractProjection;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::ProjectionCannotSupplyIndependentRoute)
        );
    }

    #[test]
    fn runtime_observation_pairs_only_with_observed_history_and_runtime_lane() {
        let mut requirement = base_requirement();
        requirement.basis = VerificationBasis::RuntimeObservation;
        requirement.independent_route = None;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory)
        );
        requirement.coverage = VerificationCoverage::ObservedHistory;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeObservationRequiresRuntimeLane)
        );
        requirement.lane = VerificationLane::Runtime;
        assert!(admit(requirement).is_ok());
        let mut sampled = base_requirement();
        sampled.coverage = VerificationCoverage::ObservedHistory;
        assert_eq!(
            admit(sampled),
            Err(RequirementAdmissionError::ObservedHistoryRequiresRuntimeObservation)
        );
    }

    #[test]
    fn exhaustive_evidence_does_not_satisfy_a_sampled_requirement() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut evidence = matching_evidence();
        evidence.coverage = VerificationCoverage::ExhaustiveWithinDeclaredModel;
        assert!(matches!(
            qualify(&admitted, &evidence),
            Err(RequirementQualificationError::CoverageMismatch { .. })
        ));
    }

    #[test]
    fn faster_lane_does_not_satisfy_the_declared_lane() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut evidence = matching_evidence();
        evidence.lane = VerificationLane::PullRequestFast;
        assert!(matches!(
            qualify(&admitted, &evidence),
            Err(RequirementQualificationError::LaneMismatch { .. })
        ));
    }

    #[test]
    fn green_terminal_alone_is_not_qualification() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut evidence = matching_evidence();
        evidence.fresh = false;
        assert_eq!(
            qualify(&admitted, &evidence),
            Err(RequirementQualificationError::Stale)
        );
        let mut unexecuted = matching_evidence();
        unexecuted.executed = false;
        assert_eq!(
            qualify(&admitted, &unexecuted),
            Err(RequirementQualificationError::NotExecuted)
        );
        let mut unbound = matching_evidence();
        unbound.artifacts_bound = false;
        assert_eq!(
            qualify(&admitted, &unbound),
            Err(RequirementQualificationError::ArtifactsUnbound)
        );
        let mut contested = matching_evidence();
        contested.counterexample_outstanding = true;
        assert_eq!(
            qualify(&admitted, &contested),
            Err(RequirementQualificationError::CounterexampleOutstanding)
        );
    }

    #[test]
    fn non_positive_terminal_is_refused() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut evidence = matching_evidence();
        evidence.terminal = ProofUnitTerminal::Expired;
        assert!(matches!(
            qualify(&admitted, &evidence),
            Err(RequirementQualificationError::NotPositiveSemanticTerminal { .. })
        ));
    }

    #[test]
    fn projection_disagreement_refuses_rather_than_trusting_the_projection() {
        // The DEC-075 planted-disagreement law: the generated projection says
        // Passed, the independent reference says Failed — the row refuses.
        let plan = [
            VerificationRequirement {
                basis: VerificationBasis::IndependentReference,
                method: VerificationMethod::HistoryReplay,
                independent_route: Some(IndependentEvidenceRouteKind::IndependentHistoryReplay),
                ..base_requirement()
            },
            VerificationRequirement {
                basis: VerificationBasis::ContractProjection,
                independent_route: None,
                ..base_requirement()
            },
        ];
        let independent = ExecutedRequirementEvidence {
            basis: VerificationBasis::IndependentReference,
            method: VerificationMethod::HistoryReplay,
            route: Some(IndependentEvidenceRouteKind::IndependentHistoryReplay),
            terminal: ProofUnitTerminal::Failed,
            ..matching_evidence()
        };
        let projection = ExecutedRequirementEvidence {
            basis: VerificationBasis::ContractProjection,
            route: None,
            terminal: ProofUnitTerminal::Passed,
            ..matching_evidence()
        };
        assert!(matches!(
            qualify_row(
                VerificationClaimKind::Determinism,
                &plan,
                &[independent, projection]
            ),
            Err(RowQualificationError::ProjectionDisagreesWithIndependentReference {
                projection: ProofUnitTerminal::Passed,
                independent: ProofUnitTerminal::Failed,
            })
        ));
    }

    #[test]
    fn empty_and_duplicate_plans_are_refused() {
        assert_eq!(
            qualify_row(VerificationClaimKind::Safety, &[], &[]),
            Err(RowQualificationError::EmptyPlan)
        );
        let plan = [base_requirement(), base_requirement()];
        assert!(matches!(
            qualify_row(
                VerificationClaimKind::Safety,
                &plan,
                &[matching_evidence(), matching_evidence()]
            ),
            Err(RowQualificationError::DuplicateRequirement { index: 1 })
        ));
    }

    #[test]
    fn full_row_qualifies_when_every_requirement_matches() {
        let plan = [base_requirement()];
        let qualified = qualify_row(
            VerificationClaimKind::Safety,
            &plan,
            &[matching_evidence()],
        )
        .expect("matching row qualifies");
        assert_eq!(qualified.claim(), VerificationClaimKind::Safety);
        assert_eq!(qualified.requirements(), 1);
    }

    #[test]
    fn runtime_matrix_is_exact() {
        use RuntimeVerificationDisposition as D;
        use RuntimeVerificationMode as M;
        // The three refused pairings are the law.
        assert!(!M::InBandObservation.admits(D::ConformantForObservedHistory));
        assert!(!M::PreventiveGuard.admits(D::ConformantForObservedHistory));
        assert!(!M::OfflineReplay.admits(D::NoDivergenceObserved));
        // A preventive guard needs no replay before refusing.
        assert!(M::PreventiveGuard.admits(D::GuardRefused));
        assert!(M::PreventiveGuard.admits(D::GuardAdmitted));
        // In-band can honestly report a trace-scoped absence of divergence.
        assert!(M::InBandObservation.admits(D::NoDivergenceObserved));
        // Only offline replay concludes conformance, for observed history only.
        assert!(M::OfflineReplay.admits(D::ConformantForObservedHistory));
        // Guard dispositions belong to the guard alone.
        assert!(!M::InBandObservation.admits(D::GuardAdmitted));
        assert!(!M::OfflineReplay.admits(D::GuardRefused));
    }
}

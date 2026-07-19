//! Typed verification plane (5.5F2 normalized in 5.5F3; DEC-077/DEC-078;
//! docs/38).
//!
//! docs/38 owns the layered-axes law in prose; this file is the typed
//! authority for the closed vocabularies and the sealed admission and
//! assessment algebra that law depends on. Every axis is a separate type: no
//! variant here derives or implements an ordering, exposes a rank, level, or
//! score, or converts to a number — an aggregate assurance ladder is forbidden
//! by shape (SEED-LAYERED-DYNAMIC-VERIFICATION), and the audit refuses one.
//!
//! The Tier 0 cross-run law is an ANALOGY for why self-corroboration is
//! refused; nothing here imports or reuses `tier0_cross_run` as product
//! verification authority.
//!
//! Observations are claims ABOUT execution, not proof. `admit_observation`
//! and `assess_plan` decide whether a claimed observation is coherent with an
//! authored plan and whether the witnesses agree — they never certify that the
//! caller honestly computed the execution, freshness, artifact, or
//! counterexample facts it presents. Phase 6 TestPak receipts and independent
//! verifiers convert concrete evidence into release-eligible proof.

use crate::guarantees::ContractId;
use crate::proof::ProofUnitTerminal;

/// The relationship of the evidence source to the subject and its semantic
/// owner (docs/38 section 2). A generated projection or a self-observation
/// can never be the sole independent oracle. The independent route a basis
/// supplies lives INSIDE the basis: a projection or a runtime self-observation
/// structurally carries none, an independent reference always names one, and a
/// direct boundary names one only when the probe is separately owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationBasis {
    /// Generated from typed authority; drives inputs, never concludes.
    ContractProjection,
    /// A separately owned model, evaluator, or relation authored against the
    /// law. Its route is either a differential second implementation or an
    /// independent replay of a recorded history — never a hostile boundary,
    /// which is the direct boundary's own route.
    IndependentReference { route: IndependentEvidenceRouteKind },
    /// A hostile probe of the real boundary under test. When the probe is
    /// separately owned it names the `HostileBoundary` route; when it is the
    /// subject's own boundary check it carries no independent route.
    DirectBoundary { route: Option<IndependentEvidenceRouteKind> },
    /// Evidence recorded from an executing system.
    RuntimeObservation,
}

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
/// refuse-or-append behavior law (docs/38 section 6). Blocking blocks;
/// Quarantine quarantines the affected candidate or artifact and never
/// blocks the row; Advisory stays visible and never blocks and never confers.
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
/// match EXACTLY to admit. No axis ranks another and no strength converts.
/// The independent route rides inside `basis`; `enforcement` is authored
/// policy and is never observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationRequirement {
    pub method: VerificationMethod,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub lane: VerificationLane,
    pub enforcement: VerificationEnforcementPosture,
}

/// Why a requirement cannot be admitted as authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementAdmissionError {
    /// The route inside the basis does not match the basis: an independent
    /// reference names a hostile boundary, or a direct boundary names a
    /// non-hostile route.
    RouteKindDoesNotMatchBasis,
    /// Runtime observation covers only executions that occurred.
    RuntimeObservationRequiresObservedHistory,
    /// Observed-history coverage exists only for runtime observation or for an
    /// independent replay of a recorded history.
    ObservedHistoryRequiresReplayOrRuntime,
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

/// Admit an authored requirement, refusing the incoherent shapes. Total,
/// panic-free, and const: every check is a structural match, never a numeric
/// comparison of an axis.
pub const fn admit(
    requirement: VerificationRequirement,
) -> Result<AdmittedVerificationRequirement, RequirementAdmissionError> {
    // Route/basis matching: an independent reference is never a hostile
    // boundary, and a direct boundary's only independent route is hostile.
    match requirement.basis {
        VerificationBasis::IndependentReference { route } => {
            if !matches!(
                route,
                IndependentEvidenceRouteKind::DifferentialImplementation
                    | IndependentEvidenceRouteKind::IndependentHistoryReplay
            ) {
                return Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis);
            }
        }
        VerificationBasis::DirectBoundary { route: Some(kind) } => {
            if !matches!(kind, IndependentEvidenceRouteKind::HostileBoundary) {
                return Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis);
            }
        }
        VerificationBasis::ContractProjection
        | VerificationBasis::DirectBoundary { route: None }
        | VerificationBasis::RuntimeObservation => {}
    }
    if matches!(requirement.basis, VerificationBasis::RuntimeObservation)
        && !matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
    {
        return Err(RequirementAdmissionError::RuntimeObservationRequiresObservedHistory);
    }
    if matches!(requirement.coverage, VerificationCoverage::ObservedHistory)
        && !matches!(
            requirement.basis,
            VerificationBasis::RuntimeObservation
                | VerificationBasis::IndependentReference {
                    route: IndependentEvidenceRouteKind::IndependentHistoryReplay
                }
        )
    {
        return Err(RequirementAdmissionError::ObservedHistoryRequiresReplayOrRuntime);
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

/// A claimed runtime verification result, before the runtime admission law
/// checks it against its mode. The route rides inside `basis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeVerificationObservation {
    pub mode: RuntimeVerificationMode,
    pub disposition: RuntimeVerificationDisposition,
    pub basis: VerificationBasis,
    pub coverage: VerificationCoverage,
    pub fresh: bool,
    pub history_complete: bool,
}

/// A runtime verification observation that passed the runtime admission law.
/// Sealed: `admit_runtime_verification` is the only constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedRuntimeVerification {
    observation: RuntimeVerificationObservation,
    _seal: (),
}

impl AdmittedRuntimeVerification {
    pub const fn observation(&self) -> &RuntimeVerificationObservation {
        &self.observation
    }
}

/// Why a claimed runtime verification is refused. Every arm names the runtime
/// law it violated; the owner is the dynamic-verification contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeVerificationError {
    /// The mode does not admit the claimed disposition (the `admits` matrix).
    DispositionNotAdmittedByMode,
    /// A preventive guard probes the real boundary directly.
    GuardRequiresDirectBoundary,
    /// An in-band observation records from the executing system.
    InBandRequiresRuntimeObservation,
    /// An in-band observation covers only the history it observed.
    InBandRequiresObservedHistory,
    /// Offline replay is an independent re-execution of a recorded history.
    ReplayRequiresIndependentHistoryReplay,
    /// Offline replay covers only the observed history it re-executes.
    ReplayRequiresObservedHistory,
    /// Conformance for an observed history requires the history to be current.
    ConformanceRequiresFreshHistory,
    /// Conformance for an observed history requires the history to be complete.
    ConformanceRequiresCompleteHistory,
}

impl RuntimeVerificationError {
    /// The violated law, stated as the requirement.
    pub const fn law(self) -> &'static str {
        match self {
            Self::DispositionNotAdmittedByMode => {
                "a runtime verification carries only a disposition its mode admits"
            }
            Self::GuardRequiresDirectBoundary => {
                "a preventive guard verifies against a direct boundary"
            }
            Self::InBandRequiresRuntimeObservation => {
                "an in-band observation records from the executing runtime"
            }
            Self::InBandRequiresObservedHistory => {
                "an in-band observation covers only its observed history"
            }
            Self::ReplayRequiresIndependentHistoryReplay => {
                "offline replay verifies through an independent history replay"
            }
            Self::ReplayRequiresObservedHistory => {
                "offline replay covers only the observed history it re-executes"
            }
            Self::ConformanceRequiresFreshHistory => {
                "conformance for an observed history requires a fresh history"
            }
            Self::ConformanceRequiresCompleteHistory => {
                "conformance for an observed history requires a complete history"
            }
        }
    }

    /// The typed owner of the violated law.
    pub const fn owner(self) -> ContractId {
        ContractId("BP-DYNAMIC-VERIFICATION-1")
    }

    /// The repair direction, stated on the offending observation's terms.
    pub const fn repair(self) -> &'static str {
        match self {
            Self::DispositionNotAdmittedByMode => {
                "state a disposition the mode admits, or the mode that admits this disposition"
            }
            Self::GuardRequiresDirectBoundary => {
                "record the guard against a direct-boundary basis"
            }
            Self::InBandRequiresRuntimeObservation => {
                "record the in-band observation against a runtime-observation basis"
            }
            Self::InBandRequiresObservedHistory => {
                "record the in-band observation with observed-history coverage"
            }
            Self::ReplayRequiresIndependentHistoryReplay => {
                "route the replay through an independent-history-replay reference"
            }
            Self::ReplayRequiresObservedHistory => {
                "record the replay with observed-history coverage"
            }
            Self::ConformanceRequiresFreshHistory => {
                "refresh the observed history before claiming conformance"
            }
            Self::ConformanceRequiresCompleteHistory => {
                "complete the observed history before claiming conformance"
            }
        }
    }
}

/// Admit a claimed runtime verification against the runtime law. Total,
/// panic-free, and const: every check is a structural match.
pub const fn admit_runtime_verification(
    observation: RuntimeVerificationObservation,
) -> Result<AdmittedRuntimeVerification, RuntimeVerificationError> {
    // (a) The mode must admit the disposition.
    if !observation.mode.admits(observation.disposition) {
        return Err(RuntimeVerificationError::DispositionNotAdmittedByMode);
    }
    match observation.mode {
        // (b) A preventive guard probes a direct boundary (any route posture).
        RuntimeVerificationMode::PreventiveGuard => {
            if !matches!(observation.basis, VerificationBasis::DirectBoundary { .. }) {
                return Err(RuntimeVerificationError::GuardRequiresDirectBoundary);
            }
        }
        // (c) An in-band observation is a runtime observation over observed history.
        RuntimeVerificationMode::InBandObservation => {
            if !matches!(observation.basis, VerificationBasis::RuntimeObservation) {
                return Err(RuntimeVerificationError::InBandRequiresRuntimeObservation);
            }
            if !matches!(observation.coverage, VerificationCoverage::ObservedHistory) {
                return Err(RuntimeVerificationError::InBandRequiresObservedHistory);
            }
        }
        // (d) Offline replay is an independent history replay over observed history.
        RuntimeVerificationMode::OfflineReplay => {
            if !matches!(
                observation.basis,
                VerificationBasis::IndependentReference {
                    route: IndependentEvidenceRouteKind::IndependentHistoryReplay
                }
            ) {
                return Err(RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay);
            }
            if !matches!(observation.coverage, VerificationCoverage::ObservedHistory) {
                return Err(RuntimeVerificationError::ReplayRequiresObservedHistory);
            }
            // (e) Conformance additionally requires a fresh, complete history.
            if matches!(
                observation.disposition,
                RuntimeVerificationDisposition::ConformantForObservedHistory
            ) {
                if !observation.fresh {
                    return Err(RuntimeVerificationError::ConformanceRequiresFreshHistory);
                }
                if !observation.history_complete {
                    return Err(RuntimeVerificationError::ConformanceRequiresCompleteHistory);
                }
            }
        }
    }
    Ok(AdmittedRuntimeVerification { observation, _seal: () })
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn base_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary {
                route: Some(IndependentEvidenceRouteKind::HostileBoundary),
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn matching_observation() -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method: VerificationMethod::PropertySequence,
            basis: VerificationBasis::DirectBoundary {
                route: Some(IndependentEvidenceRouteKind::HostileBoundary),
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal: ProofUnitTerminal::Passed,
        }
    }

    #[test]
    fn matching_observation_admits() {
        let admitted = admit(base_requirement()).expect("coherent requirement admits");
        let observed =
            admit_observation(&admitted, &matching_observation()).expect("exact match admits");
        assert_eq!(observed.terminal(), ProofUnitTerminal::Passed);
        assert_eq!(observed.requirement().method, VerificationMethod::PropertySequence);
    }

    #[test]
    fn route_kind_must_match_basis() {
        // An independent reference is never a hostile boundary.
        let mut hostile_reference = base_requirement();
        hostile_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::HostileBoundary,
        };
        assert_eq!(
            admit(hostile_reference),
            Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
        );
        // A direct boundary's only independent route is hostile.
        let mut differential_boundary = base_requirement();
        differential_boundary.basis = VerificationBasis::DirectBoundary {
            route: Some(IndependentEvidenceRouteKind::DifferentialImplementation),
        };
        assert_eq!(
            admit(differential_boundary),
            Err(RequirementAdmissionError::RouteKindDoesNotMatchBasis)
        );
    }

    #[test]
    fn lawful_route_and_basis_pairings_admit() {
        let mut differential_reference = base_requirement();
        differential_reference.method = VerificationMethod::DifferentialExecution;
        differential_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::DifferentialImplementation,
        };
        assert!(admit(differential_reference).is_ok());

        let mut replay_reference = base_requirement();
        replay_reference.method = VerificationMethod::HistoryReplay;
        replay_reference.basis = VerificationBasis::IndependentReference {
            route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
        };
        assert!(admit(replay_reference).is_ok());

        let mut own_boundary = base_requirement();
        own_boundary.basis = VerificationBasis::DirectBoundary { route: None };
        assert!(admit(own_boundary).is_ok());

        let mut projection = base_requirement();
        projection.basis = VerificationBasis::ContractProjection;
        assert!(admit(projection).is_ok());
    }

    #[test]
    fn runtime_observation_pairs_only_with_observed_history_and_runtime_lane() {
        let mut requirement = base_requirement();
        requirement.basis = VerificationBasis::RuntimeObservation;
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
    }

    #[test]
    fn observed_history_is_lawful_only_for_runtime_or_independent_replay() {
        // A direct boundary cannot carry observed-history coverage.
        let mut boundary = base_requirement();
        boundary.coverage = VerificationCoverage::ObservedHistory;
        assert_eq!(
            admit(boundary),
            Err(RequirementAdmissionError::ObservedHistoryRequiresReplayOrRuntime)
        );
        // An independent history replay is the newly lawful non-runtime pairing.
        let replay = VerificationRequirement {
            method: VerificationMethod::HistoryReplay,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        };
        assert!(admit(replay).is_ok());
    }

    #[test]
    fn runtime_lane_requires_runtime_observation() {
        let mut requirement = base_requirement();
        requirement.lane = VerificationLane::Runtime;
        assert_eq!(
            admit(requirement),
            Err(RequirementAdmissionError::RuntimeLaneRequiresRuntimeObservation)
        );
    }

    #[test]
    fn exhaustive_observation_does_not_satisfy_a_sampled_requirement() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut observation = matching_observation();
        observation.coverage = VerificationCoverage::ExhaustiveWithinDeclaredModel;
        assert!(matches!(
            admit_observation(&admitted, &observation),
            Err(ObservationAdmissionError::CoverageMismatch { .. })
        ));
    }

    #[test]
    fn faster_lane_does_not_satisfy_the_declared_lane() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut observation = matching_observation();
        observation.lane = VerificationLane::PullRequestFast;
        assert!(matches!(
            admit_observation(&admitted, &observation),
            Err(ObservationAdmissionError::LaneMismatch { .. })
        ));
    }

    #[test]
    fn method_and_basis_mismatches_are_refused() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut wrong_method = matching_observation();
        wrong_method.method = VerificationMethod::StructuralRule;
        assert!(matches!(
            admit_observation(&admitted, &wrong_method),
            Err(ObservationAdmissionError::MethodMismatch { .. })
        ));
        let mut wrong_basis = matching_observation();
        wrong_basis.basis = VerificationBasis::ContractProjection;
        assert_eq!(
            admit_observation(&admitted, &wrong_basis),
            Err(ObservationAdmissionError::BasisMismatch)
        );
    }

    #[test]
    fn conclusive_verdict_alone_is_not_admission() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut stale = matching_observation();
        stale.fresh = false;
        assert_eq!(
            admit_observation(&admitted, &stale),
            Err(ObservationAdmissionError::Stale)
        );
        let mut unexecuted = matching_observation();
        unexecuted.executed = false;
        assert_eq!(
            admit_observation(&admitted, &unexecuted),
            Err(ObservationAdmissionError::NotExecuted)
        );
        let mut unbound = matching_observation();
        unbound.artifacts_bound = false;
        assert_eq!(
            admit_observation(&admitted, &unbound),
            Err(ObservationAdmissionError::ArtifactsUnbound)
        );
        let mut contested = matching_observation();
        contested.counterexample_outstanding = true;
        assert_eq!(
            admit_observation(&admitted, &contested),
            Err(ObservationAdmissionError::CounterexampleOutstanding)
        );
    }

    #[test]
    fn non_verdict_terminal_is_refused() {
        let admitted = admit(base_requirement()).expect("admits");
        let mut expired = matching_observation();
        expired.terminal = ProofUnitTerminal::Expired;
        assert!(matches!(
            admit_observation(&admitted, &expired),
            Err(ObservationAdmissionError::InconclusiveTerminal { .. })
        ));
    }

    #[test]
    fn empty_and_duplicate_plans_are_refused() {
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &[], &[]),
            Err(RowAssessmentError::EmptyPlan)
        );
        let plan = [base_requirement(), base_requirement()];
        assert!(matches!(
            assess_plan(
                VerificationClaimKind::Safety,
                &plan,
                &[matching_observation(), matching_observation()]
            ),
            Err(RowAssessmentError::DuplicateRequirement { index: 1 })
        ));
    }

    #[test]
    fn observation_count_must_match_plan() {
        let plan = [base_requirement()];
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &[]),
            Err(RowAssessmentError::ObservationCountMismatch { required: 1, presented: 0 })
        );
    }

    #[test]
    fn full_row_assesses_when_every_observation_admits() {
        let plan = [base_requirement()];
        let assessment =
            assess_plan(VerificationClaimKind::Safety, &plan, &[matching_observation()])
                .expect("matching row assesses");
        assert_eq!(assessment.claim(), VerificationClaimKind::Safety);
        assert_eq!(assessment.satisfied(), 1);
        assert_eq!(assessment.advisory_failures(), 0);
        assert_eq!(assessment.quarantine_failures(), 0);
        assert!(assessment.is_positive());
    }

    #[test]
    fn advisory_failure_does_not_block() {
        let mut requirement = base_requirement();
        requirement.enforcement = VerificationEnforcementPosture::Advisory;
        let plan = [requirement];
        let mut observation = matching_observation();
        observation.executed = false;
        let assessment = assess_plan(VerificationClaimKind::ResourceEnvelope, &plan, &[observation])
            .expect("an advisory failure never blocks");
        assert_eq!(assessment.satisfied(), 0);
        assert_eq!(assessment.advisory_failures(), 1);
        assert_eq!(assessment.quarantine_failures(), 0);
        assert!(assessment.is_positive());
    }

    #[test]
    fn quarantine_failure_is_not_advisory_and_is_not_positive() {
        let mut requirement = base_requirement();
        requirement.enforcement = VerificationEnforcementPosture::Quarantine;
        let plan = [requirement];
        let mut observation = matching_observation();
        observation.executed = false;
        let assessment = assess_plan(VerificationClaimKind::Refinement, &plan, &[observation])
            .expect("a quarantine failure is recorded, not returned");
        assert_eq!(assessment.satisfied(), 0);
        assert_eq!(assessment.advisory_failures(), 0);
        assert_eq!(assessment.quarantine_failures(), 1);
        assert!(!assessment.is_positive());
    }

    #[test]
    fn blocking_failure_is_returned() {
        let plan = [base_requirement()];
        let mut observation = matching_observation();
        observation.executed = false;
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &[observation]),
            Err(RowAssessmentError::BlockingRequirementFailed {
                index: 0,
                error: ObservationAdmissionError::NotExecuted,
            })
        );
    }

    // A projection requirement carrying a distinct method, so two of them are
    // not a duplicate plan entry.
    const fn projection_requirement(method: VerificationMethod) -> VerificationRequirement {
        VerificationRequirement {
            method,
            basis: VerificationBasis::ContractProjection,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn projection_observation(
        method: VerificationMethod,
        terminal: ProofUnitTerminal,
    ) -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method,
            basis: VerificationBasis::ContractProjection,
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal,
        }
    }

    const fn replay_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::HistoryReplay,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn differential_requirement() -> VerificationRequirement {
        VerificationRequirement {
            method: VerificationMethod::DifferentialExecution,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::DifferentialImplementation,
            },
            coverage: VerificationCoverage::Sampled,
            lane: VerificationLane::Merge,
            enforcement: VerificationEnforcementPosture::Blocking,
        }
    }

    const fn independent_observation(
        requirement: VerificationRequirement,
        terminal: ProofUnitTerminal,
    ) -> RequirementEvidenceObservation {
        RequirementEvidenceObservation {
            method: requirement.method,
            basis: requirement.basis,
            coverage: requirement.coverage,
            lane: requirement.lane,
            executed: true,
            fresh: true,
            artifacts_bound: true,
            counterexample_outstanding: false,
            terminal,
        }
    }

    #[test]
    fn two_projection_witnesses_that_disagree_refuse() {
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            projection_requirement(VerificationMethod::PropertySequence),
        ];
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            projection_observation(VerificationMethod::PropertySequence, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Safety, &plan, &observations),
            Err(RowAssessmentError::ProjectionWitnessesDisagree {
                first: ProofUnitTerminal::Passed,
                second: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn two_independent_witnesses_that_disagree_refuse() {
        let differential = differential_requirement();
        let replay = replay_requirement();
        let plan = [differential, replay];
        let observations = [
            independent_observation(differential, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Refinement, &plan, &observations),
            Err(RowAssessmentError::IndependentWitnessesDisagree {
                first: ProofUnitTerminal::Passed,
                second: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn projection_disagreeing_with_independent_reference_refuses() {
        // The DEC-075 planted-disagreement law: the generated projection says
        // Passed, the independent reference says Failed — the row refuses and
        // never chooses the projection.
        let replay = replay_requirement();
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            replay,
        ];
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Determinism, &plan, &observations),
            Err(RowAssessmentError::ProjectionDisagreesWithIndependentReference {
                projection: ProofUnitTerminal::Passed,
                independent: ProofUnitTerminal::Failed,
            })
        );
    }

    #[test]
    fn a_blocking_defect_is_reported_before_any_disagreement() {
        // A genuine cross-family disagreement (projection Passed vs independent
        // Failed) coexists with an invalid blocking observation. Every
        // observation is validated first, and the blocking defect is reported
        // instead of the disagreement: invalid evidence is never a witness.
        let replay = replay_requirement();
        let plan = [
            projection_requirement(VerificationMethod::StructuralRule),
            replay,
            base_requirement(),
        ];
        let mut defective = matching_observation();
        defective.executed = false;
        let observations = [
            projection_observation(VerificationMethod::StructuralRule, ProofUnitTerminal::Passed),
            independent_observation(replay, ProofUnitTerminal::Failed),
            defective,
        ];
        assert_eq!(
            assess_plan(VerificationClaimKind::Determinism, &plan, &observations),
            Err(RowAssessmentError::BlockingRequirementFailed {
                index: 2,
                error: ObservationAdmissionError::NotExecuted,
            })
        );
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

    #[test]
    fn runtime_admission_green_paths() {
        // A preventive guard against a direct boundary admits.
        let guard = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::GuardAdmitted,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert!(admit_runtime_verification(guard).is_ok());
        // An in-band no-divergence observation over observed history admits.
        let in_band = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert!(admit_runtime_verification(in_band).is_ok());
        // Offline replay concludes conformance only with a fresh, complete history.
        let replay = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: true,
            history_complete: true,
        };
        assert!(admit_runtime_verification(replay).is_ok());
    }

    #[test]
    fn runtime_admission_refuses_every_error_arm() {
        // (a) The mode does not admit the disposition.
        let mismatch = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(mismatch),
            Err(RuntimeVerificationError::DispositionNotAdmittedByMode)
        );
        // (b) A guard against a non-boundary basis.
        let guard_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::PreventiveGuard,
            disposition: RuntimeVerificationDisposition::GuardAdmitted,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(guard_basis),
            Err(RuntimeVerificationError::GuardRequiresDirectBoundary)
        );
        // (c) In-band against a non-runtime basis.
        let in_band_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(in_band_basis),
            Err(RuntimeVerificationError::InBandRequiresRuntimeObservation)
        );
        // (c) In-band without observed-history coverage.
        let in_band_coverage = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::InBandObservation,
            disposition: RuntimeVerificationDisposition::NoDivergenceObserved,
            basis: VerificationBasis::RuntimeObservation,
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(in_band_coverage),
            Err(RuntimeVerificationError::InBandRequiresObservedHistory)
        );
        // (d) Replay against a non-replay basis.
        let replay_basis = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            basis: VerificationBasis::DirectBoundary { route: None },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(replay_basis),
            Err(RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay)
        );
        // (d) Replay without observed-history coverage.
        let replay_coverage = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::Divergent,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::Sampled,
            fresh: false,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(replay_coverage),
            Err(RuntimeVerificationError::ReplayRequiresObservedHistory)
        );
        // (e) Conformance without a fresh history.
        let stale_conformance = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: false,
            history_complete: true,
        };
        assert_eq!(
            admit_runtime_verification(stale_conformance),
            Err(RuntimeVerificationError::ConformanceRequiresFreshHistory)
        );
        // (e) Conformance without a complete history.
        let incomplete_conformance = RuntimeVerificationObservation {
            mode: RuntimeVerificationMode::OfflineReplay,
            disposition: RuntimeVerificationDisposition::ConformantForObservedHistory,
            basis: VerificationBasis::IndependentReference {
                route: IndependentEvidenceRouteKind::IndependentHistoryReplay,
            },
            coverage: VerificationCoverage::ObservedHistory,
            fresh: true,
            history_complete: false,
        };
        assert_eq!(
            admit_runtime_verification(incomplete_conformance),
            Err(RuntimeVerificationError::ConformanceRequiresCompleteHistory)
        );
    }

    #[test]
    fn runtime_error_law_owner_and_repair_are_stated_for_every_arm() {
        let arms = [
            RuntimeVerificationError::DispositionNotAdmittedByMode,
            RuntimeVerificationError::GuardRequiresDirectBoundary,
            RuntimeVerificationError::InBandRequiresRuntimeObservation,
            RuntimeVerificationError::InBandRequiresObservedHistory,
            RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay,
            RuntimeVerificationError::ReplayRequiresObservedHistory,
            RuntimeVerificationError::ConformanceRequiresFreshHistory,
            RuntimeVerificationError::ConformanceRequiresCompleteHistory,
        ];
        for arm in arms {
            assert!(!arm.law().is_empty());
            assert!(!arm.repair().is_empty());
            assert_eq!(arm.owner(), ContractId("BP-DYNAMIC-VERIFICATION-1"));
        }
    }
}

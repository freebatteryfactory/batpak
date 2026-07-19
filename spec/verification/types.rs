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

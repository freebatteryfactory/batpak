use core::num::NonZeroU64;

use crate::guarantees::{ContractId, GuaranteeRef};
use crate::promotion::PromotionRequirement;
use crate::proof::ProofRowId;
use crate::verification::IndependentEvidenceRouteKind;

/// HOW a candidate was produced. This axis records provenance only; it confers
/// no authority. Whether a candidate may be promoted is decided against
/// `PromotionRequirement::ALL`, never inferred from the origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateOriginKind {
    /// Produced by a deterministic generator from a named input frontier.
    DeterministicGeneration,
    /// Produced by a bounded search under a declared, receipted budget.
    BoundedSearch,
    /// Reused from an earlier admitted candidate under a reuse key.
    TransferReuse,
    /// Written by a human.
    HumanAuthored,
    /// Produced by machine-assisted synthesis (a human-in-the-loop generator).
    MachineAssistedSynthesis,
    /// Produced by repairing an earlier candidate named on the manifest.
    RepairOfCandidate,
}

/// The frozen inventory of candidate origins. Origin is provenance, not
/// authority: every member faces the identical promotion denominator.
pub const CANDIDATE_ORIGIN_KINDS: &[CandidateOriginKind] = &[
    CandidateOriginKind::DeterministicGeneration,
    CandidateOriginKind::BoundedSearch,
    CandidateOriginKind::TransferReuse,
    CandidateOriginKind::HumanAuthored,
    CandidateOriginKind::MachineAssistedSynthesis,
    CandidateOriginKind::RepairOfCandidate,
];

/// Whether a candidate preserves the realized law or changes it. Both classes
/// satisfy the complete conjunctive promotion denominator; the class ADDS an
/// authority path, it never replaces a common requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateChangeClass {
    /// The candidate realizes an existing law without changing it. Its repair
    /// authority may be mechanical or a bounded search, never architect-level.
    RealizationPreserving,
    /// The candidate changes the realized law. It requires an architect and the
    /// explicit amendment of the actual semantic owner.
    LawChanging,
}

/// The frozen inventory of change classes.
pub const CANDIDATE_CHANGE_CLASSES: &[CandidateChangeClass] = &[
    CandidateChangeClass::RealizationPreserving,
    CandidateChangeClass::LawChanging,
];

/// The role an evaluation set plays. Search and holdout sets are disjoint: a
/// candidate never sees its holdout during search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationSetRole {
    /// Drives the candidate search; never overlaps the holdout.
    Search,
    /// Qualifies a candidate for promotion consideration.
    Qualification,
    /// Held out of search; a failure here quarantines the candidate.
    Holdout,
    /// Guards against regression of already-realized law.
    Regression,
}

/// The frozen inventory of evaluation-set roles.
pub const EVALUATION_SET_ROLES: &[EvaluationSetRole] = &[
    EvaluationSetRole::Search,
    EvaluationSetRole::Qualification,
    EvaluationSetRole::Holdout,
    EvaluationSetRole::Regression,
];

/// How much of a realization obligation is actually realized. `Scaffold` counts
/// as UNREALIZED alongside `Missing`: a placeholder that type-checks realizes
/// nothing and cannot close the obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealizationPosture {
    /// Nothing realizes the obligation.
    Missing,
    /// A placeholder that type-checks but realizes nothing. Counts in the
    /// unrealized denominator exactly like `Missing`.
    Scaffold,
    /// A genuine realization candidate.
    Candidate,
}

/// The frozen inventory of realization postures.
pub const REALIZATION_POSTURES: &[RealizationPosture] = &[
    RealizationPosture::Missing,
    RealizationPosture::Scaffold,
    RealizationPosture::Candidate,
];

/// The authority a repair may exercise. A realization-preserving candidate may
/// repair mechanically or by bounded search; only a law-changing candidate may
/// require an architect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairAuthority {
    /// A mechanical, meaning-preserving repair.
    Mechanical,
    /// A repair produced by a bounded search under a declared budget.
    BoundedSearch,
    /// A repair that only an architect may authorize; required for a
    /// law-changing candidate, refused for a realization-preserving one.
    ArchitectRequired,
}

/// The frozen inventory of repair authorities.
pub const REPAIR_AUTHORITIES: &[RepairAuthority] = &[
    RepairAuthority::Mechanical,
    RepairAuthority::BoundedSearch,
    RepairAuthority::ArchitectRequired,
];

/// A declared, receiptable search budget. Every field is a `NonZeroU64`: a
/// zero budget is not a budget, and the type makes "unbudgeted search"
/// unrepresentable rather than policed. This carries the DECLARED envelope
/// only; whether a run honored it is a receipt-derived fact, not a field here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSearchBudget {
    /// The maximum number of candidates the search may consider.
    pub max_candidates: NonZeroU64,
    /// The maximum logical work the search may spend.
    pub max_logical_work: NonZeroU64,
    /// The maximum memory, in bytes, the search may occupy.
    pub max_memory_bytes: NonZeroU64,
    /// The maximum monotonic ticks the search may run.
    pub max_monotonic_ticks: NonZeroU64,
}

/// An authored candidate-promotion plan awaiting admission. It names the change
/// class, the repair authority, the selected independent-evidence route, and
/// the promotion requirements it claims to satisfy. Construction is unsealed —
/// a hostile fixture may author any plan; ADMISSION through
/// `admit_promotion_plan` is the law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidatePromotionPlan {
    /// Whether the candidate preserves or changes the realized law.
    pub change_class: CandidateChangeClass,
    /// The authority the candidate's repair exercises.
    pub repair_authority: RepairAuthority,
    /// The selected independent-evidence route (the concrete kind naming the
    /// `PromotionRequirement::IndependentEvidenceRoute` need).
    pub independent_route: IndependentEvidenceRouteKind,
    /// The named proof targets that bind the candidate's semantic purpose
    /// (E7, TL-1): the exact `spec::proof` rows promotion under this plan
    /// claims to realize — `PromotionRequirement::NamedProofTarget` must
    /// actually NAME targets, and a diagnostic `unit=` label never
    /// substitutes. Admission refuses an empty or duplicated set; outside
    /// this crate a `ProofRowId` is only obtainable from the `spec/proof/`
    /// catalog, and Active-row resolution is seedcheck/receiptcheck law.
    pub proof_targets: &'static [ProofRowId],
    /// The promotion requirements the plan claims to satisfy. Admission checks
    /// these are set-equal to `PromotionRequirement::ALL` with no duplicate.
    pub requirements: &'static [PromotionRequirement],
}

/// The closed vocabulary of promotion-plan admission refusals. Each arm is the
/// RULE a fixture asserts against, never a prose substring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionPlanError {
    /// A conjunctive member of `PromotionRequirement::ALL` is absent from the
    /// plan's requirements.
    MissingRequirement { requirement: PromotionRequirement },
    /// A requirement appears more than once; the index is the first repeat.
    DuplicateRequirement { index: usize },
    /// The plan names no proof target (E7): `NamedProofTarget` must actually
    /// name targets — an empty set binds no semantic purpose and admits
    /// nothing.
    EmptyProofTargets,
    /// A proof target appears more than once; the index is the first repeat.
    DuplicateProofTarget { index: usize },
    /// A law-changing candidate did not require an architect. A law change
    /// cannot ride a mechanical or bounded-search repair authority.
    LawChangingRequiresArchitect,
    /// A realization-preserving candidate required an architect. Preserving the
    /// realized law admits mechanical or bounded-search repair only.
    RealizationPreservingCannotRequireArchitect,
}

/// Proof that `admit_promotion_plan` produced this plan. The seal is private:
/// no generator, auditor, projector, or fixture can mint one, so a plan wearing
/// this wrapper has passed the admission law by construction — the same
/// sealed-witness shape as `AdmittedGuarantee` and `AdmittedVerificationRequirement`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedCandidatePromotionPlan {
    pub(super) plan: CandidatePromotionPlan,
    pub(super) _seal: (),
}

impl AdmittedCandidatePromotionPlan {
    /// The admitted plan. A consumer reads it; only `admit_promotion_plan`
    /// mints the wrapper.
    pub const fn plan(&self) -> &CandidatePromotionPlan {
        &self.plan
    }
}

/// The DEC-073 specialized-plan candidate policy: the authored posture the
/// PakVM ISA specialization surface admits candidates under. It is
/// realization-preserving, admits every origin (origin confers no authority),
/// selects the differential-implementation route, and carries the complete
/// conjunctive promotion denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecializedPlanCandidatePolicy {
    /// The contract that owns this policy's semantics.
    pub semantic_owner: ContractId,
    /// The standing decision that admits this policy.
    pub admission_basis: GuaranteeRef,
    /// The change class every candidate under this policy holds.
    pub change_class: CandidateChangeClass,
    /// The origins this policy admits. Every origin is admitted: origin records
    /// how a candidate was produced, never whether it is authorized.
    pub allowed_origins: &'static [CandidateOriginKind],
    /// The independent-evidence route this policy selects.
    pub independent_route: IndependentEvidenceRouteKind,
    /// The promotion requirements — the complete `PromotionRequirement::ALL`
    /// denominator, consumed not restated.
    pub requirements: &'static [PromotionRequirement],
}

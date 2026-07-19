//! The typed candidate-sprouting vocabulary and specialized-plan policy
//! (5.5F3).
//!
//! This file owns the CLOSED axes a sprouted candidate is described on and the
//! typed law that admits a candidate-promotion plan. It owns no candidate
//! records, no manifest schema, no serialized representation, and no version
//! type: `spec/promotion.rs` owns the conjunctive promotion denominator
//! (consumed here, never restated); `spec/verification.rs` owns the
//! independent-evidence route vocabulary; docs/39 owns the sprouting doctrine
//! in prose; docs/07 owns the DEC-073 specialization surface.
//!
//! Two laws this vocabulary refuses to blur:
//!
//! - **Candidate origin confers no authority.** A candidate's ORIGIN records
//!   only HOW it was produced (deterministic generation, bounded search,
//!   transfer reuse, a human, machine-assisted synthesis, or repair of an
//!   earlier candidate). Its AUTHORITY POSTURE — whether it is admitted for
//!   promotion — is a SEPARATE axis, decided by `admit_promotion_plan` against
//!   the conjunctive `PromotionRequirement::ALL` denominator. A machine-authored
//!   candidate and a human-authored candidate face the identical denominator;
//!   no origin is self-blessing and none is disqualifying. Origin and authority
//!   are never one fact.
//!
//! - **`Scaffold` counts as unrealized.** `RealizationPosture` distinguishes a
//!   `Missing` realization, a `Scaffold` (a placeholder that type-checks but
//!   realizes nothing), and a genuine `Candidate`. A `Scaffold` sits in the
//!   UNREALIZED denominator exactly like `Missing`: it cannot close a
//!   realization obligation and cannot be counted as progress.
//!
//! Qualification, reuse, promotion, invalidation, and supersession are
//! RECEIPT-DERIVED EVENTS, never mutable lifecycle variants. There is no
//! candidate lifecycle ladder here, no `Invalidates` edge registry, and no
//! generic `CandidateEvidenceRelation` enum: a candidate's relations are
//! DERIVED from required manifest and receipt fields (parent candidate
//! identities, the source-frontier commitment, dependency commitments, the
//! content commitment, generator or pilot evidence, qualification receipts, the
//! reuse key, and the promotion, refusal, and invalidation receipts), never
//! from a parallel edge vocabulary. This file mints no such vocabulary.
//!
//! Every axis is a closed enum with a frozen `ALL`-style inventory and derives
//! `Clone`/`Copy`/`Debug`/`PartialEq`/`Eq` ONLY: no ordering, rank, level,
//! score, or numeric conversion — a sprouting axis is a name, not a magnitude.

use core::num::NonZeroU64;

use crate::guarantees::{ContractId, GuaranteeRef};
use crate::promotion::PromotionRequirement;
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
    plan: CandidatePromotionPlan,
    _seal: (),
}

impl AdmittedCandidatePromotionPlan {
    /// The admitted plan. A consumer reads it; only `admit_promotion_plan`
    /// mints the wrapper.
    pub const fn plan(&self) -> &CandidatePromotionPlan {
        &self.plan
    }
}

/// Admit a candidate-promotion plan. The plan's requirements must be SET-EQUAL
/// to `PromotionRequirement::ALL` with no duplicate, and its repair authority
/// must cohere with its change class: a `LawChanging` candidate requires an
/// architect, a `RealizationPreserving` candidate refuses one and admits
/// mechanical or bounded-search repair. Panic-free: the requirement scan is
/// index-bounded and no arm unwraps.
///
/// The conjunctive denominator is CONSUMED from `spec/promotion.rs`, never
/// restated: a change to `PromotionRequirement::ALL` changes what every plan
/// must satisfy here, with no second list to drift.
pub fn admit_promotion_plan(
    plan: CandidatePromotionPlan,
) -> Result<AdmittedCandidatePromotionPlan, PromotionPlanError> {
    let reqs = plan.requirements;

    // No requirement may appear twice: report the first repeat by index.
    for (index, req) in reqs.iter().enumerate() {
        if reqs[..index].contains(req) {
            return Err(PromotionPlanError::DuplicateRequirement { index });
        }
    }

    // Every conjunctive member of ALL must be present.
    for &required in PromotionRequirement::ALL {
        if !reqs.contains(&required) {
            return Err(PromotionPlanError::MissingRequirement {
                requirement: required,
            });
        }
    }

    // No length backstop exists: over the closed requirement enum, a
    // duplicate-free slice containing every member of ALL is exactly ALL by
    // pigeonhole, so a residual-length arm would be an unreachable variant —
    // vocabulary claiming to be law (the deleted-GuaranteeEdgeKind defect).

    // Change class and repair authority must cohere.
    match plan.change_class {
        CandidateChangeClass::LawChanging => {
            if plan.repair_authority != RepairAuthority::ArchitectRequired {
                return Err(PromotionPlanError::LawChangingRequiresArchitect);
            }
        }
        CandidateChangeClass::RealizationPreserving => {
            if plan.repair_authority == RepairAuthority::ArchitectRequired {
                return Err(PromotionPlanError::RealizationPreservingCannotRequireArchitect);
            }
        }
    }

    Ok(AdmittedCandidatePromotionPlan { plan, _seal: () })
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

/// The DEC-073 specialized-plan candidate policy (owner BP-PAKVM-ISA-1).
pub const SPECIALIZED_PLAN_CANDIDATE_POLICY: SpecializedPlanCandidatePolicy =
    SpecializedPlanCandidatePolicy {
        semantic_owner: ContractId("BP-PAKVM-ISA-1"),
        admission_basis: GuaranteeRef::dec("DEC-073"),
        change_class: CandidateChangeClass::RealizationPreserving,
        allowed_origins: CANDIDATE_ORIGIN_KINDS,
        independent_route: IndependentEvidenceRouteKind::DifferentialImplementation,
        requirements: PromotionRequirement::ALL,
    };

#[cfg(test)]
mod tests {
    use super::*;

    /// The conjunctive denominator, in ALL order, as an owned static slice.
    const FULL_REQUIREMENTS: &[PromotionRequirement] = &[
        PromotionRequirement::IndependentEvidenceRoute,
        PromotionRequirement::NamedProofTarget,
        PromotionRequirement::QualifiedHostileEvidence,
        PromotionRequirement::AuditablePromotionReceipt,
    ];

    fn preserving_plan() -> CandidatePromotionPlan {
        CandidatePromotionPlan {
            change_class: CandidateChangeClass::RealizationPreserving,
            repair_authority: RepairAuthority::Mechanical,
            independent_route: IndependentEvidenceRouteKind::DifferentialImplementation,
            requirements: FULL_REQUIREMENTS,
        }
    }

    #[test]
    fn realization_preserving_plan_admits() {
        let admitted = admit_promotion_plan(preserving_plan()).expect("coherent plan admits");
        assert_eq!(
            admitted.plan().change_class,
            CandidateChangeClass::RealizationPreserving
        );
        assert_eq!(admitted.plan().requirements, PromotionRequirement::ALL);
    }

    #[test]
    fn law_changing_with_architect_admits() {
        let mut plan = preserving_plan();
        plan.change_class = CandidateChangeClass::LawChanging;
        plan.repair_authority = RepairAuthority::ArchitectRequired;
        assert!(admit_promotion_plan(plan).is_ok());
    }

    #[test]
    fn realization_preserving_admits_bounded_search() {
        let mut plan = preserving_plan();
        plan.repair_authority = RepairAuthority::BoundedSearch;
        assert!(admit_promotion_plan(plan).is_ok());
    }

    #[test]
    fn missing_requirement_is_refused() {
        // Drop AuditablePromotionReceipt: a conjunctive member is absent.
        const SHORT: &[PromotionRequirement] = &[
            PromotionRequirement::IndependentEvidenceRoute,
            PromotionRequirement::NamedProofTarget,
            PromotionRequirement::QualifiedHostileEvidence,
        ];
        let mut plan = preserving_plan();
        plan.requirements = SHORT;
        assert_eq!(
            admit_promotion_plan(plan),
            Err(PromotionPlanError::MissingRequirement {
                requirement: PromotionRequirement::AuditablePromotionReceipt,
            })
        );
    }

    #[test]
    fn duplicate_requirement_is_refused() {
        const DUP: &[PromotionRequirement] = &[
            PromotionRequirement::IndependentEvidenceRoute,
            PromotionRequirement::IndependentEvidenceRoute,
            PromotionRequirement::NamedProofTarget,
            PromotionRequirement::QualifiedHostileEvidence,
        ];
        let mut plan = preserving_plan();
        plan.requirements = DUP;
        assert_eq!(
            admit_promotion_plan(plan),
            Err(PromotionPlanError::DuplicateRequirement { index: 1 })
        );
    }

    #[test]
    fn law_changing_without_architect_is_refused() {
        let mut plan = preserving_plan();
        plan.change_class = CandidateChangeClass::LawChanging;
        plan.repair_authority = RepairAuthority::Mechanical;
        assert_eq!(
            admit_promotion_plan(plan),
            Err(PromotionPlanError::LawChangingRequiresArchitect)
        );
    }

    #[test]
    fn realization_preserving_with_architect_is_refused() {
        let mut plan = preserving_plan();
        plan.repair_authority = RepairAuthority::ArchitectRequired;
        assert_eq!(
            admit_promotion_plan(plan),
            Err(PromotionPlanError::RealizationPreservingCannotRequireArchitect)
        );
    }

    #[test]
    fn candidate_search_budget_requires_nonzero_fields() {
        let n = NonZeroU64::new(8).expect("8 is nonzero");
        let budget = CandidateSearchBudget {
            max_candidates: n,
            max_logical_work: n,
            max_memory_bytes: n,
            max_monotonic_ticks: n,
        };
        assert_eq!(budget.max_candidates.get(), 8);
        assert_eq!(budget.max_monotonic_ticks.get(), 8);
    }

    #[test]
    fn specialized_plan_policy_invariants() {
        let policy = SPECIALIZED_PLAN_CANDIDATE_POLICY;
        assert_eq!(policy.semantic_owner, ContractId("BP-PAKVM-ISA-1"));
        assert_eq!(policy.admission_basis, GuaranteeRef::dec("DEC-073"));
        assert_eq!(policy.change_class, CandidateChangeClass::RealizationPreserving);
        assert_eq!(policy.allowed_origins, CANDIDATE_ORIGIN_KINDS);
        assert_eq!(policy.allowed_origins.len(), 6);
        assert_eq!(
            policy.independent_route,
            IndependentEvidenceRouteKind::DifferentialImplementation
        );
        assert_eq!(policy.requirements, PromotionRequirement::ALL);
        // The policy's own plan admits: origin confers no authority, so the
        // policy posture is lawful independent of any producing origin.
        let plan = CandidatePromotionPlan {
            change_class: policy.change_class,
            repair_authority: RepairAuthority::Mechanical,
            independent_route: policy.independent_route,
            requirements: policy.requirements,
        };
        assert!(admit_promotion_plan(plan).is_ok());
    }

    #[test]
    fn inventories_are_frozen() {
        assert_eq!(CANDIDATE_ORIGIN_KINDS.len(), 6);
        assert_eq!(CANDIDATE_CHANGE_CLASSES.len(), 2);
        assert_eq!(EVALUATION_SET_ROLES.len(), 4);
        assert_eq!(REALIZATION_POSTURES.len(), 3);
        assert_eq!(REPAIR_AUTHORITIES.len(), 3);
        // Scaffold is an unrealized posture, distinct from a genuine Candidate.
        assert_ne!(RealizationPosture::Scaffold, RealizationPosture::Candidate);
    }
}

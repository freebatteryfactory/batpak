use core::num::NonZeroU64;

use crate::campaign::MINI_SUPERNOVA_PROFILE;
use crate::guarantees::{ContractId, GuaranteeRef};
use crate::promotion::PromotionRequirement;
use crate::proof::ProofRowId;
use crate::verification::IndependentEvidenceRouteKind;

use super::*;

/// The conjunctive denominator, in ALL order, as an owned static slice.
const FULL_REQUIREMENTS: &[PromotionRequirement] = &[
    PromotionRequirement::IndependentEvidenceRoute,
    PromotionRequirement::NamedProofTarget,
    PromotionRequirement::QualifiedHostileEvidence,
    PromotionRequirement::AuditablePromotionReceipt,
];

/// The rehearsal-shaped fixture plan (R5: templates are frozen test-owned
/// rehearsal fixtures). Its proof targets are the campaign evidence profile
/// rows the mini-supernova rehearsal realizes literally — honest names, not
/// decoration.
fn preserving_plan() -> CandidatePromotionPlan {
    CandidatePromotionPlan {
        change_class: CandidateChangeClass::RealizationPreserving,
        repair_authority: RepairAuthority::Mechanical,
        independent_route: IndependentEvidenceRouteKind::DifferentialImplementation,
        proof_targets: MINI_SUPERNOVA_PROFILE.realized_rows,
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
fn empty_proof_targets_are_refused() {
    // E7: NamedProofTarget must actually name targets — a plan naming none
    // binds no semantic purpose and is refused before any other law runs.
    let mut plan = preserving_plan();
    plan.proof_targets = &[];
    assert_eq!(
        admit_promotion_plan(plan),
        Err(PromotionPlanError::EmptyProofTargets)
    );
}

#[test]
fn duplicate_proof_target_is_refused() {
    const DUP_TARGETS: &[ProofRowId] = &[
        ProofRowId("planted_semantic_mutant_is_activated_and_killed"),
        ProofRowId("planted_semantic_mutant_is_activated_and_killed"),
    ];
    let mut plan = preserving_plan();
    plan.proof_targets = DUP_TARGETS;
    assert_eq!(
        admit_promotion_plan(plan),
        Err(PromotionPlanError::DuplicateProofTarget { index: 1 })
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
    // policy posture is lawful independent of any producing origin. Its
    // proof target is the one Active DEC-073 row binding the specialized-plan
    // lane — the plan's admission basis IS that row's guarantee.
    let plan = CandidatePromotionPlan {
        change_class: policy.change_class,
        repair_authority: RepairAuthority::Mechanical,
        independent_route: policy.independent_route,
        proof_targets: &[ProofRowId("specialized_plan_benchmark_is_advisory_only")],
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

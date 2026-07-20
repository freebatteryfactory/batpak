use crate::promotion::PromotionRequirement;

use super::types::*;

/// Admit a candidate-promotion plan. The plan must NAME its proof targets —
/// nonempty and duplicate-free (E7: `NamedProofTarget` must actually name
/// targets); its requirements must be SET-EQUAL to
/// `PromotionRequirement::ALL` with no duplicate; and its repair authority
/// must cohere with its change class: a `LawChanging` candidate requires an
/// architect, a `RealizationPreserving` candidate refuses one and admits
/// mechanical or bounded-search repair. Panic-free: every scan is
/// index-bounded and no arm unwraps.
///
/// The conjunctive denominator is CONSUMED from `spec/promotion/types.rs`, never
/// restated: a change to `PromotionRequirement::ALL` changes what every plan
/// must satisfy here, with no second list to drift. Proof-target SHAPE is the
/// law here; resolution beyond shape is not restated: outside this crate a
/// `ProofRowId` is only obtainable from the `spec/proof/` catalog (TL-1), and
/// refusing a retired/unknown target or a target whose guarantee is unrelated
/// to the declared authority path is seedcheck/receiptcheck law over the wire.
pub fn admit_promotion_plan(
    plan: CandidatePromotionPlan,
) -> Result<AdmittedCandidatePromotionPlan, PromotionPlanError> {
    // The plan names its proof targets: an empty set is refused, and a
    // repeated target is reported by the index of its first repeat,
    // mirroring the requirement scan below.
    let targets = plan.proof_targets;
    if targets.is_empty() {
        return Err(PromotionPlanError::EmptyProofTargets);
    }
    for (index, target) in targets.iter().enumerate() {
        if targets[..index].contains(target) {
            return Err(PromotionPlanError::DuplicateProofTarget { index });
        }
    }

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

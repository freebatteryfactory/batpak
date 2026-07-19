use crate::promotion::PromotionRequirement;

use super::types::*;

/// Admit a candidate-promotion plan. The plan's requirements must be SET-EQUAL
/// to `PromotionRequirement::ALL` with no duplicate, and its repair authority
/// must cohere with its change class: a `LawChanging` candidate requires an
/// architect, a `RealizationPreserving` candidate refuses one and admits
/// mechanical or bounded-search repair. Panic-free: the requirement scan is
/// index-bounded and no arm unwraps.
///
/// The conjunctive denominator is CONSUMED from `spec/promotion/types.rs`, never
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

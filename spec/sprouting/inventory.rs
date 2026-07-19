use crate::guarantees::{ContractId, GuaranteeRef};
use crate::promotion::PromotionRequirement;
use crate::verification::IndependentEvidenceRouteKind;

use super::types::*;

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

/// Candidate material is disposable and derived. It is never durable proof
/// truth, and there is no canonical direct-write path from generation into
/// tracked source.
pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";

/// Tracked surfaces a generated candidate may never write directly. Promotion
/// is a separate admitted action with its own receipt.
pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS: &[&str] =
    &["src/", "tests/", "spec/", "docs/", "companion/"];

//! The typed candidate-sprouting vocabulary and specialized-plan policy
//! (5.5F3).
//!
//! This file owns the CLOSED axes a sprouted candidate is described on and the
//! typed law that admits a candidate-promotion plan. It owns no candidate
//! records, no manifest schema, no serialized representation, and no version
//! type: `spec/promotion/` owns the conjunctive promotion denominator
//! (consumed here, never restated); `spec/verification/` owns the
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
//! content commitment, named proof targets, and the reuse key on the immutable
//! V2 manifest; evidence references and the qualification, holdout, promotion,
//! refusal, invalidation, and escalation events ride append-only campaign
//! receipts in the nursery receipt store, never manifest fields), never
//! from a parallel edge vocabulary. This file mints no such vocabulary.
//!
//! Every axis is a closed enum with a frozen `ALL`-style inventory and derives
//! `Clone`/`Copy`/`Debug`/`PartialEq`/`Eq` ONLY: no ordering, rank, level,
//! score, or numeric conversion — a sprouting axis is a name, not a magnitude.

mod types;
mod inventory;
mod admission;

pub use types::{
    AdmittedCandidatePromotionPlan, CandidateChangeClass, CandidateOriginKind,
    CandidatePromotionPlan, CandidateSearchBudget, EvaluationSetRole, PromotionPlanError,
    RealizationPosture, RepairAuthority, SpecializedPlanCandidatePolicy,
    CANDIDATE_CHANGE_CLASSES, CANDIDATE_ORIGIN_KINDS, EVALUATION_SET_ROLES,
    REALIZATION_POSTURES, REPAIR_AUTHORITIES,
};
pub use inventory::{
    CANDIDATE_FORBIDDEN_WRITE_ROOTS, CANDIDATE_OUTPUT_ROOT, SPECIALIZED_PLAN_CANDIDATE_POLICY,
};
pub use admission::admit_promotion_plan;

#[cfg(test)]
mod tests;

//! The typed candidate-promotion requirements (5.5E3i).
//!
//! Four bouncers, each with a badge, all four on duty: `ALL` is the
//! CONJUNCTIVE promotion denominator — every member is required, a missing
//! member refuses promotion, and there is no optional flag, waiver variant,
//! or second required-items array. No requirement satisfies another: a
//! killed mutant does not prove the evidence route was independent, a named
//! guarantee does not prove the hostile activated, and a receipt cannot
//! manufacture missing evidence.
//!
//! `spec/promotion.rs` owns which requirements exist, their canonical order
//! and spellings, and their typed owner/basis/surface/gate bindings.
//! docs/12 owns what each requirement MEANS; docs/24 owns hostile-rule
//! qualification and proof-row meaning; docs/31 consumes the typed law and
//! owns no second list. This slice adds no candidate identity, no
//! promotion-receipt schema, no generic proof-target type, and no actual
//! promotion records.

use crate::architecture::ProofPolicySurface;
use crate::gates::GateId;
use crate::guarantees::{ContractId, GuaranteeRef};

/// The conjunctive candidate-promotion requirements (DEC-015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionRequirement {
    /// The candidate and its generator may not be their own sole oracle;
    /// two wrappers around one implementation are not independence.
    IndependentEvidenceRoute,
    /// Promotion names a resolvable guarantee, or a documented proof gap
    /// with a stable name, a contract owner, the affected boundary, and a
    /// repair target — never a generic issue link or a commit message.
    NamedProofTarget,
    /// A real semantic mutant Killed under a qualified baseline with
    /// observed activation, or hostile evidence satisfying the docs/24
    /// Rule qualification law. Survived, NotActivated, Refused,
    /// Unbuildable, TimedOut, InfrastructureFailure, an unwitnessed
    /// EquivalentCandidate, an arbitrary failing test, and a baseline
    /// failure all satisfy nothing.
    QualifiedHostileEvidence,
    /// The complete auditable receipt docs/12 specifies, binding the
    /// resulting tracked-content commitment. A commit message is not this
    /// receipt.
    AuditablePromotionReceipt,
}

/// Which proof-policy surface promotion changes classify under (DEC-074).
pub const PROMOTION_POLICY_SURFACE: ProofPolicySurface = ProofPolicySurface::CandidatePromotion;

/// The decision governing changes that strengthen, preserve, or weaken the
/// promotion boundary — distinct from DEC-015, which admits the
/// requirements themselves.
pub const PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec("DEC-074");

/// Where candidate-promotion policy is enforced.
pub const PROMOTION_ENFORCEMENT_GATE: GateId = GateId::G3;

/// Where policy changes must be release-visibly qualified.
pub const PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::G9;

impl PromotionRequirement {
    /// The conjunctive promotion denominator. Every member is required.
    pub const ALL: &'static [PromotionRequirement] = &[
        PromotionRequirement::IndependentEvidenceRoute,
        PromotionRequirement::NamedProofTarget,
        PromotionRequirement::QualifiedHostileEvidence,
        PromotionRequirement::AuditablePromotionReceipt,
    ];

    pub const fn spelling(self) -> &'static str {
        match self {
            PromotionRequirement::IndependentEvidenceRoute => "IndependentEvidenceRoute",
            PromotionRequirement::NamedProofTarget => "NamedProofTarget",
            PromotionRequirement::QualifiedHostileEvidence => "QualifiedHostileEvidence",
            PromotionRequirement::AuditablePromotionReceipt => "AuditablePromotionReceipt",
        }
    }

    /// Explicit in every arm — no wildcard silently blesses a future
    /// variant.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            PromotionRequirement::IndependentEvidenceRoute => ContractId("BP-TESTPAK-1"),
            PromotionRequirement::NamedProofTarget => ContractId("BP-TESTPAK-1"),
            PromotionRequirement::QualifiedHostileEvidence => ContractId("BP-TESTPAK-1"),
            PromotionRequirement::AuditablePromotionReceipt => ContractId("BP-TESTPAK-1"),
        }
    }

    /// The decision that admits which requirements promotion must satisfy.
    pub const fn admission_basis(self) -> GuaranteeRef {
        match self {
            PromotionRequirement::IndependentEvidenceRoute => GuaranteeRef::dec("DEC-015"),
            PromotionRequirement::NamedProofTarget => GuaranteeRef::dec("DEC-015"),
            PromotionRequirement::QualifiedHostileEvidence => GuaranteeRef::dec("DEC-015"),
            PromotionRequirement::AuditablePromotionReceipt => GuaranteeRef::dec("DEC-015"),
        }
    }
}

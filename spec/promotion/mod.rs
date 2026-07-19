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

mod types;

pub use types::{
    PromotionRequirement, PROMOTION_CHANGE_BASIS, PROMOTION_ENFORCEMENT_GATE,
    PROMOTION_POLICY_SURFACE, PROMOTION_RELEASE_VISIBILITY_GATE,
};

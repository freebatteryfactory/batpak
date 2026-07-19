//! The authenticated-history claim contract (DEC-071): what a store's
//! history verification actually proves. The profile a caller selects, the
//! typed witness policy it permits, the closed witness dispositions, and the
//! four independent claim axes of a success bundle live here; an invalid
//! pairing is refused, never normalized into a neighbour, and a refusal is
//! never a weaker success. A contract, not an implementation: bootstrap
//! performs no signature, accumulator, witness, freshness, or cryptographic
//! verification of any kind.

mod types;

pub use types::{
    AuthenticatedHistoryClaims, AuthenticatedHistoryProfile, AuthenticityClaim, FreshnessClaim,
    IntegrityClaim, RollbackResistanceClaim, WitnessDisposition, WitnessPolicy,
    INTERNAL_CONSISTENCY_SUCCESS, OPTIONAL_WITNESS_REFUSAL_SET, REFUSAL_PARTIAL_CLAIM_LAW,
    REQUIRED_WITNESS_FAILURE_SET, SIGNED_UNANCHORED_SUCCESS, WITNESSED_SUCCESS,
};

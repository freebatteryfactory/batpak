//! Frozen clean-room legacy semantic obligation facts.

mod inventory;
mod types;

pub use types::{
    CompatibilityDisposition, DeletionCondition, LegacyObligation, LegacyWitnessRequirement,
    ObligationStatus, legacy_lifetime,
};
pub use inventory::OBLIGATIONS;

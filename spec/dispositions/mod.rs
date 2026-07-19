//! Frozen clean-room architecture decision facts.

mod inventory;
mod types;

pub use types::{DecisionClass, DecisionSpec, Disposition, StaleContext};
pub use inventory::DECISIONS;

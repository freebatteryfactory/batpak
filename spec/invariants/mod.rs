//! Frozen bootstrap invariants. Stable IDs become TestPak contract IDs.
//!
//! Each SEED fact carries its own guarantee classification (DEC-070): kind,
//! lifetime, owner, gates, witness, source-native failure disposition, and
//! explicit relations. Shared types live in `spec/guarantees.rs`; this file is
//! the sole owner of the SEED facts and their classification.

mod inventory;
mod types;

pub use types::InvariantSpec;
pub use inventory::INVARIANTS;

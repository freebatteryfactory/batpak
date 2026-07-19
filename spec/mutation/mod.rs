//! The canonical mutation vocabulary (5.5E3f).
//!
//! Two closed types, moved out of `spec/architecture.rs` and stripped of
//! their shadow tables: the lane facts and result classifications are TOTAL
//! const functions on the enums, so there is no parallel row catalog whose
//! only job is to keep agreeing with the variants. The lettered nicknames
//! (Lane A/B/C) are dead: they were descriptions, not identities, and the
//! canonical spellings below are the only lawful citations.
//!
//! Muterprater is a TestPak plane, never a standalone product, a second
//! semantic authority, or a replacement compiler. Bootstrap compiles no
//! mutant, activates no slot, runs no nextest, invokes no rustc, kills
//! nothing, proves no equivalence, and promotes nothing. It proves the
//! contract.

mod types;

pub use types::{MutationLane, MutationResult};

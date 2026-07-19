//! Typed verification plane (5.5F2 normalized in 5.5F3; DEC-077/DEC-078;
//! docs/38).
//!
//! docs/38 owns the layered-axes law in prose; this file is the typed
//! authority for the closed vocabularies and the sealed admission and
//! assessment algebra that law depends on. Every axis is a separate type: no
//! variant here derives or implements an ordering, exposes a rank, level, or
//! score, or converts to a number — an aggregate assurance ladder is forbidden
//! by shape (SEED-LAYERED-DYNAMIC-VERIFICATION), and the audit refuses one.
//!
//! The Tier 0 cross-run law is an ANALOGY for why self-corroboration is
//! refused; nothing here imports or reuses `tier0_cross_run` as product
//! verification authority.
//!
//! Observations are claims ABOUT execution, not proof. `admit_observation`
//! and `assess_plan` decide whether a claimed observation is coherent with an
//! authored plan and whether the witnesses agree — they never certify that the
//! caller honestly computed the execution, freshness, artifact, or
//! counterexample facts it presents. Phase 6 TestPak receipts and independent
//! verifiers convert concrete evidence into release-eligible proof.

mod axes;
mod requirements;
mod observation;
mod assessment;
mod runtime;
#[cfg(test)]
mod tests;

pub use axes::*;
pub use requirements::*;
pub use observation::*;
pub use assessment::*;
pub use runtime::*;

//! Proof-unit vocabulary (5.5E1 ruling; extended by the 5.5E2 bake).
//!
//! docs/12 owns the audited-denominator doctrine in prose; this file is the
//! typed authority for the closed vocabularies that doctrine depends on. The
//! sibling eight-state `MutationResult` earned a typed owner long before this
//! file existed; the general proof denominator now gets the same treatment.

mod types;
mod inventory;
mod policy;

pub use types::{
    ProofRowId, ProofRowRecord, ProofRowState, ProofUnitTerminal, PROOF_UNIT_TERMINALS,
};
pub use inventory::{
    PLAN_BENCHMARK_ADVISORY, PLAN_COMPILE_REFUSAL, PLAN_COMPLEXITY, PLAN_CRASH_RECOVERY,
    PLAN_DIFFERENTIAL, PLAN_FAULT_INJECTION, PLAN_FRONTIER_EXPLORATION, PLAN_HISTORY_REPLAY,
    PLAN_HOLDOUT_QUARANTINE, PLAN_HOSTILE_BOUNDARY, PLAN_LAW_CHANGE_REFUSAL, PLAN_PROPERTY,
    PLAN_RECONCILIATION_REFINEMENT, PLAN_STRUCTURAL, PROOF_ROWS,
};
pub use policy::{ProofPolicyChangeClass, ProofPolicySurface};

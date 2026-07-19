//! Shared guarantee-classification types (DEC-070).
//!
//! This file owns the SHARED semantic types for the guarantee system. It does
//! NOT own the individual guarantees: SEED facts are classified in
//! `spec/invariants.rs`; LEG, DEC, architecture, and qualification facts keep
//! their native authority and schemas in their own files. This file is not a
//! second SEED table and is not a graph generator — the derived Guarantee Graph
//! is produced by `bootstrap/project.py` from ADMITTED views and independently
//! verified by `bootstrap/audit.py`. The graph is a derived structural index,
//! never a normative authority, and cannot replace or silently revise an owning
//! fact. Neither script may author family semantics; see `GuaranteeView`.

mod types;
mod policy;

pub use types::{
    ArchitectureGuaranteeId, BootstrapToolId, ContractId, DecisionId, GuaranteeKind,
    GuaranteeLifetime, GuaranteeRef, LegacyId, QualificationId, SeedId, WitnessRef,
};
pub use policy::{
    AdmittedGuarantee, GUARANTEE_FAMILY_POLICIES, GatePosture, GatePostureRule,
    GuaranteeAdmissionFailure, GuaranteeAdmissionRule, GuaranteeFamilyPolicy, GuaranteeSource,
    GuaranteeView, KindRule, LifetimeRule, OwnerRule, SourceFailure, WitnessPosture, admit,
};

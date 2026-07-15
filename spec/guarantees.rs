//! Shared guarantee-classification types (DEC-070).
//!
//! This file owns the SHARED semantic types for the guarantee system. It does
//! NOT own the individual guarantees: SEED facts are classified in
//! `spec/invariants.rs`; LEG, DEC, architecture, and qualification facts keep
//! their native authority and schemas in their own files. This file is not a
//! second SEED table and is not a graph generator — the derived Guarantee Graph
//! is produced by `bootstrap/project.py` and independently re-derived by
//! `bootstrap/audit.py`. The graph is a derived structural index, never a
//! normative authority, and cannot replace or silently revise an owning fact.

/// What kind of guarantee a node is. Each authored family maps to a kind; the
/// kinds do not merge the families' authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuaranteeKind {
    /// Permanent machine law (a load-bearing beam).
    SemanticLaw,
    /// Architecture/topology constraint.
    ArchitectureConstraint,
    /// Bootstrap construction assertion (a repository/self-hosting structural rule).
    BootstrapAssertion,
    /// Retained legacy behavior obligation.
    LegacyObligation,
    /// Target/profile qualification requirement.
    QualificationRequirement,
    /// Architectural decision/disposition.
    Decision,
}

/// When a guarantee stops being an active obligation. Lifetime is orthogonal to
/// active/closed status, compatibility disposition, and deletion condition;
/// `DeletionCondition::Never` means the historical row is never removed, NOT
/// that the guarantee is active or permanent forever.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuaranteeLifetime {
    /// No automatic expiry; may still be replaced only through explicit supersession.
    Permanent,
    /// Active until a named gate completes with qualifying evidence.
    UntilGate,
    /// Active until a named compatibility condition and closure evidence are satisfied.
    UntilCompatibilityExpiry,
    /// Active until a named successor is implemented, qualified, and receipted.
    UntilSuccessor,
    /// Retained for provenance or denominator accounting; cannot gate implementation or release.
    HistoricalCoverageOnly,
    /// No longer an active implementation obligation; retained as historical evidence.
    ClosedEvidence,
}

/// Explicit, declared relations between guarantees. A relation exists only when
/// declared by an owning fact or a deterministic adapter rule — never inferred
/// from similar wording, shared nouns, shared owner, document section, or fuzzy
/// similarity. There is no confidence score and no prose archaeology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuaranteeEdgeKind {
    DerivesFrom,
    Refines,
    Discharges,
    Supersedes,
    Closes,
}

/// A reference to another guarantee by its stable id.
pub type GuaranteeRef = &'static str;

/// Source-qualified failure disposition. The derived view preserves the native
/// meaning per family rather than flattening every failure into one generic
/// severity ladder. The `&str` payload is the family-native disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFailure {
    Seed(&'static str),
    Legacy(&'static str),
    Decision(&'static str),
    Architecture(&'static str),
    Qualification(&'static str),
}

/// The shape of one derived Guarantee Graph node. This is a declarative record
/// type only; the adapters that produce it live in the independent generator and
/// auditor, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuaranteeView {
    pub id: GuaranteeRef,
    pub kind: GuaranteeKind,
    pub lifetime: GuaranteeLifetime,
    pub owner: &'static str,
    pub gates: &'static str,
    pub witness: &'static str,
    pub failure: SourceFailure,
}

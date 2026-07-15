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

use crate::gates::GateId;

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

/// Whether and when a guarantee's qualification is scheduled. This is AUTHORED
/// posture, never a resolution outcome: a gate schedules qualification, it never
/// owns semantic meaning. `GateIndependent` states that no gate schedules this
/// law — it is a positive claim, not the absence of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatePosture {
    Scheduled(&'static [GateId]),
    GateIndependent,
}

/// The environment a qualification requirement is qualified against, such as
/// `no_std + alloc`, `std`, or `wasm32 host`. A target is NOT a gate: it names
/// where the requirement holds, not when it is scheduled. It never inhabits a
/// gate field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationTarget(pub &'static str);

/// The outcome of RESOLVING a gate posture from an authored fact. This is
/// deliberately a separate type from `GatePosture`: "the law intentionally has no
/// gate" and "nobody specified one" must never share a drawer. An empty value
/// cannot mean no gates, gate-independent, unresolved, and not-applicable at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateResolution {
    Resolved(GatePosture),
    /// The owning fact declares nothing where a posture is required.
    Missing,
    /// The owning fact declares something that is not a lawful posture.
    Malformed,
}

/// How a family's guarantees are witnessed. Declared once per family; an absent
/// witness column is not a witness policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessPosture {
    /// The native row names its own witness (SEED).
    RowDeclared,
    /// Structural qualification against the authored package inventory (ARCH).
    StructuralArchitecture,
    /// A target-specific qualification receipt (QUAL).
    QualificationReceipt,
    /// The family declares no witness of its own. This is an authored statement,
    /// not missing data.
    NoFamilyWitness,
}

/// Where one `GuaranteeView` field legitimately comes from. Every admitted field
/// resolves through exactly one of these; a projector may serialize an admitted
/// view but may never complete a missing one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindRule {
    RowDeclared,
    FamilyConstant(GuaranteeKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerRule {
    RowDeclared,
    FamilyConstant(&'static str),
}

/// A lifetime is authored on the row, fixed by family policy, or produced by ONE
/// named lawful derivation from an authored field. There is no fourth option and
/// no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifetimeRule {
    RowDeclared,
    FamilyConstant(GuaranteeLifetime),
    /// LEG: derived from `active_or_closed_status` + `deletion_condition`.
    FromLegacyStatusAndDeletion,
    /// DEC: derived from the authored `Disposition` via
    /// `Disposition::guarantee_lifetime`.
    FromDecisionDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatePostureRule {
    /// The native row declares its own gate list.
    RowDeclared,
    FamilyConstant(GatePosture),
    /// DEC: `Scheduled` when the row declares gates. `GateIndependent` when the
    /// row declares none AND `DecisionClass::requires_gate()` is false — that is
    /// an authored claim carried by the class, not absence used as policy. A row
    /// declaring no gates while its class requires them fails admission.
    ///
    /// `requires_gate()` is a floor, not a ceiling: a class that does not require
    /// a gate may still declare one, and then the declared gates win.
    FromDecisionClassAndRowGates,
}

/// Typed family policy: the family-wide semantics an individual row does not and
/// should not repeat. This exists because the alternative — letting a projector
/// fill the silence — is what produced invented lifetimes and a qualification
/// target sitting in a gate column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuaranteeFamilyPolicy {
    pub family: &'static str,
    pub kind: KindRule,
    pub owner: OwnerRule,
    pub lifetime: LifetimeRule,
    pub gate_posture: GatePostureRule,
    pub witness: WitnessPosture,
}

/// The five accepted guarantee families. Declared once, here. Row-specific
/// variation stays on the native row; nothing below is repeated per row.
pub const GUARANTEE_FAMILY_POLICIES: &[GuaranteeFamilyPolicy] = &[
    GuaranteeFamilyPolicy {
        family: "SEED",
        kind: KindRule::RowDeclared,
        owner: OwnerRule::RowDeclared,
        lifetime: LifetimeRule::RowDeclared,
        gate_posture: GatePostureRule::RowDeclared,
        witness: WitnessPosture::RowDeclared,
    },
    GuaranteeFamilyPolicy {
        family: "LEG",
        kind: KindRule::FamilyConstant(GuaranteeKind::LegacyObligation),
        owner: OwnerRule::RowDeclared,
        lifetime: LifetimeRule::FromLegacyStatusAndDeletion,
        gate_posture: GatePostureRule::RowDeclared,
        witness: WitnessPosture::NoFamilyWitness,
    },
    GuaranteeFamilyPolicy {
        family: "DEC",
        kind: KindRule::FamilyConstant(GuaranteeKind::Decision),
        owner: OwnerRule::FamilyConstant("docs/30_DECISION_AND_REJECTION_LEDGER.md"),
        lifetime: LifetimeRule::FromDecisionDisposition,
        gate_posture: GatePostureRule::FromDecisionClassAndRowGates,
        witness: WitnessPosture::NoFamilyWitness,
    },
    GuaranteeFamilyPolicy {
        family: "ARCH",
        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),
        owner: OwnerRule::FamilyConstant("spec/architecture.rs"),
        lifetime: LifetimeRule::FamilyConstant(GuaranteeLifetime::Permanent),
        gate_posture: GatePostureRule::FamilyConstant(GatePosture::Scheduled(&[GateId::G0])),
        witness: WitnessPosture::StructuralArchitecture,
    },
    GuaranteeFamilyPolicy {
        family: "QUAL",
        kind: KindRule::FamilyConstant(GuaranteeKind::QualificationRequirement),
        owner: OwnerRule::FamilyConstant("spec/architecture.rs"),
        lifetime: LifetimeRule::FamilyConstant(GuaranteeLifetime::Permanent),
        // Scheduling is row-specific: it cannot be derived from a compilation
        // target or from neighbouring SEED vocabulary.
        gate_posture: GatePostureRule::RowDeclared,
        witness: WitnessPosture::QualificationReceipt,
    },
];

/// One admitted guarantee node.
///
/// ADMISSION LAW (5.5D4b authority closure). Every field below is supplied by
/// exactly one of: a direct authored value on the native row, a declared
/// `GuaranteeFamilyPolicy`, or one named lawful derivation. A projector
/// serializes an admitted view; it may not complete one. Admission fails closed
/// when a required field has no such source.
///
/// This replaces the prior instruction that "the adapters that produce it live in
/// the independent generator and auditor". That delegation is what allowed
/// `project.py` and `audit.py` to author family semantics, invent `Permanent`,
/// and place a qualification target in a gate field while parity passed because
/// both scripts carried the same constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuaranteeView {
    pub id: GuaranteeRef,
    pub family: &'static str,
    pub kind: GuaranteeKind,
    pub lifetime: GuaranteeLifetime,
    pub owner: &'static str,
    pub gate_posture: GatePosture,
    /// Present only for families that qualify against an environment.
    pub qualification_target: Option<QualificationTarget>,
    pub witness: WitnessPosture,
    pub failure: SourceFailure,
}

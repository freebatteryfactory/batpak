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

// GuaranteeEdgeKind { DerivesFrom, Refines, Discharges, Supersedes, Closes }
// stood here from 5.5C1 until the D4d saturation audit. The edge vocabulary it
// duplicated already lives in the relation FIELDS of the owning facts —
// derives_from, refines, discharges, supersedes — and both independent graph
// derivations consume those field names directly. Nothing ever constructed or
// matched the enum, and it quietly disagreed with the authority it shadowed:
// five kinds where the authored facts declare four (no fact carries a `closes`
// relation). Deleted rather than kept, same law as GateResolution below: a
// second vocabulary for an owned semantic space is a duplicate authority the
// moment it exists, and an unconsumed one guarantees nothing. The law the doc
// comment stated — relations exist only when declared by an owning fact, never
// inferred from wording — is stated and enforced at the relation fields.

/// A typed reference to a guarantee: the family is the discriminant and the
/// payload is a SEALED per-family identity. ARCH and QUAL identity is
/// STRUCTURAL — the package name, the (package, profile) pair — never a
/// formatted string: composing "ARCH-{package}" at a call site was an
/// allocation pretending to be an identity law, and the projectors that need
/// a rendered form own their own rendering.
///
/// The id newtypes carry private fields with `pub(crate)` construction, so
/// the crate boundary IS the seal: the spec's own tables author references
/// and `admit` derives them, while a bootstrap binary or fixture can only
/// RECEIVE one — reading it through `raw()` — never mint one that claims a
/// family it did not come from. A reference is a name, not a witness:
/// whether the name resolves to a declared row is an executed seedcheck law,
/// not a construction-time promise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuaranteeRef {
    Seed(SeedId),
    Legacy(LegacyId),
    Decision(DecisionId),
    Architecture(ArchitectureGuaranteeId),
    Qualification(QualificationId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedId(pub(crate) &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyId(pub(crate) &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionId(pub(crate) &'static str);

/// ARCH guarantee identity IS the package identity (5.5E3b): a string
/// cannot mint one, and a package that is not a PackageId variant has no
/// ARCH guarantee to claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArchitectureGuaranteeId(pub(crate) crate::architecture::PackageId);

/// QUAL guarantee identity IS the (package identity, profile) pair
/// (package typed since 5.5E3b: a string cannot carry the package half).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationId {
    pub(crate) package: crate::architecture::PackageId,
    pub(crate) profile: &'static str,
}

impl SeedId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}
impl LegacyId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}
impl DecisionId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}
impl ArchitectureGuaranteeId {
    pub const fn package(self) -> crate::architecture::PackageId {
        self.0
    }
}
impl QualificationId {
    pub const fn package(self) -> crate::architecture::PackageId {
        self.package
    }
    pub const fn profile(self) -> &'static str {
        self.profile
    }
}

impl GuaranteeRef {
    /// Authoring shorthands for the relation fields on native rows. They are
    /// `pub(crate)` — only the spec's own tables mint references. Only the
    /// families a relation actually cites today have a shorthand; a seed,
    /// ARCH, or QUAL constructor appears when an authored relation first
    /// needs one, not before — an unconsumed constructor is vocabulary
    /// claiming to be law.
    pub(crate) const fn leg(raw: &'static str) -> GuaranteeRef {
        GuaranteeRef::Legacy(LegacyId(raw))
    }
    pub(crate) const fn dec(raw: &'static str) -> GuaranteeRef {
        GuaranteeRef::Decision(DecisionId(raw))
    }
    /// A SEED guarantee reference. Authored first in 5.5F1, where the charter's
    /// SEED refinements are the first native relations to cite a sibling SEED row
    /// (e.g. SEED-MODEL-TRACE-SEPARATION refines SEED-INDEPENDENT-ORACLE) — the
    /// exact "appears when an authored relation first needs one" condition above.
    pub(crate) const fn seed(raw: &'static str) -> GuaranteeRef {
        GuaranteeRef::Seed(SeedId(raw))
    }
}

/// One typed witness citation (5.5E2): WHICH owned evidence obligation a law
/// depends on. The human explanation lives in the row's `witness_note`, never
/// inside the reference — the typed portion answers "which obligation", the
/// note answers "how a human should read that evidence". Every reference must
/// RESOLVE: a witness citing an undeclared guarantee, an unknown contract id,
/// or a tool that does not exist on disk is refused by executed seedcheck law.
///
/// `ProofRow(ProofRowId)` and `Qualification(QualificationId)` join this enum
/// with their first authored citation — the proof-row registry bake is
/// expected to author the first `ProofRow` witness when it binds refusal
/// families to typed proof rows. Declaring them before any witness cites one
/// would be the deleted-`TestFixture` defect wearing a new name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessRef {
    /// Another guarantee whose obligation witnesses this law.
    Guarantee(GuaranteeRef),
    /// A contract document, by its declared `contract_id` front matter.
    Contract(ContractId),
    /// A bootstrap tool whose executed checks witness this law.
    BootstrapTool(BootstrapToolId),
}

/// A contract document's declared identity. Sealed like the per-family
/// guarantee ids: the spec's own tables author citations, a consumer reads
/// them through `raw()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractId(pub(crate) &'static str);

impl ContractId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}

/// The closed inventory of ALL bootstrap tools (Wave-2 ruling: the meaning is
/// the factory's complete tool set, not merely the currently-cited witness
/// tools). Closed by the factory's actual tools, not by citation: seedcheck
/// consumes every variant by requiring `path()` to exist in the tree it
/// checks, executed over `ALL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapToolId {
    ProjectPy,
    AuditPy,
    FreezePy,
    SelftestPy,
    Seedcheck,
    Materialize,
    /// The independent Tier 0 evidence computer (5.5E6b/E6c2).
    Receiptcheck,
}

impl BootstrapToolId {
    /// Every tool, in declaration order.
    pub const ALL: &'static [BootstrapToolId] = &[
        BootstrapToolId::ProjectPy,
        BootstrapToolId::AuditPy,
        BootstrapToolId::FreezePy,
        BootstrapToolId::SelftestPy,
        BootstrapToolId::Seedcheck,
        BootstrapToolId::Materialize,
        BootstrapToolId::Receiptcheck,
    ];

    /// The repository path the identity names.
    pub const fn path(self) -> &'static str {
        match self {
            BootstrapToolId::ProjectPy => "bootstrap/project.py",
            BootstrapToolId::AuditPy => "bootstrap/audit.py",
            BootstrapToolId::FreezePy => "bootstrap/freeze.py",
            BootstrapToolId::SelftestPy => "bootstrap/selftest.py",
            BootstrapToolId::Seedcheck => "bootstrap/seedcheck.rs",
            BootstrapToolId::Materialize => "bootstrap/materialize.rs",
            BootstrapToolId::Receiptcheck => "bootstrap/receiptcheck.rs",
        }
    }

    /// The short rendering the generated projections use.
    pub const fn display(self) -> &'static str {
        match self {
            BootstrapToolId::ProjectPy => "project.py",
            BootstrapToolId::AuditPy => "audit.py",
            BootstrapToolId::FreezePy => "freeze.py",
            BootstrapToolId::SelftestPy => "selftest.py",
            BootstrapToolId::Seedcheck => "seedcheck",
            BootstrapToolId::Materialize => "materialize",
            BootstrapToolId::Receiptcheck => "receiptcheck",
        }
    }
}

impl WitnessRef {
    /// Authoring shorthands, `pub(crate)` for the same reason as the
    /// relation shorthands: only the spec's own tables mint citations.
    pub(crate) const fn leg(raw: &'static str) -> WitnessRef {
        WitnessRef::Guarantee(GuaranteeRef::leg(raw))
    }
    pub(crate) const fn dec(raw: &'static str) -> WitnessRef {
        WitnessRef::Guarantee(GuaranteeRef::dec(raw))
    }
    pub(crate) const fn contract(raw: &'static str) -> WitnessRef {
        WitnessRef::Contract(ContractId(raw))
    }
}

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
    /// An ACTIVE qualification or construction binding: this guarantee is
    /// scheduled to qualify at these gates.
    Scheduled(&'static [GateId]),
    /// The gates this guarantee was originally established against, retained as
    /// PROVENANCE after supersession or retirement. A historical association is
    /// not a schedule: a `HistoricalCoverageOnly` guarantee keeps its authored
    /// GateIds without ever appearing actively scheduled.
    HistoricalAssociation(&'static [GateId]),
    /// No gate schedules this law. A positive claim, not the absence of one.
    GateIndependent,
}

// QualificationTarget(&'static str) stood here from 5.5D4b until 5.5E3a1. It
// wrapped free prose ("std", "no_std + alloc") that only convention kept from
// becoming a target triple or a typo, so seedcheck grew a membership law to
// police what the type could not say. Replaced by the closed
// `architecture::QualificationEnvironment`, where a triple has no spelling
// and the membership law dissolved into construction. An environment is NOT
// a gate: it names the semantic assumptions the requirement holds under, not
// when it is scheduled, and it never inhabits a gate field.

// GateResolution { Resolved, Missing, Malformed } stood here from 954d24b7 until
// the D4c2 smell sweep. It was declared to keep "the law intentionally has no
// gate" apart from "nobody specified one" — a real distinction, which the very
// same pass then solved a different way: GatePosture gained Scheduled,
// HistoricalAssociation, and GateIndependent, and admission raises AdmissionFailure
// for the missing and malformed cases. The type was orphaned the moment its job
// was done elsewhere, and nothing ever constructed, read, or enforced it.
//
// Deleted rather than kept. A type that no code builds and no rule consults is
// vocabulary claiming to be law: it reads as a guarantee during review and
// guarantees nothing at runtime. That is the same species as a refusal branch
// nothing can reach and a fixture whose mutation never applied — this campaign
// has now found all three.

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
    /// DEC posture, derived from three authored facts: the disposition, the
    /// class, and the row's own gates.
    ///
    /// - the disposition retires the decision (its lifetime is
    ///   `HistoricalCoverageOnly`) and the row declares gates
    ///     -> `HistoricalAssociation`: the GateIds are kept as provenance, never
    ///        deleted and never reinterpreted as an active schedule;
    /// - the row declares gates and the decision is still active
    ///     -> `Scheduled`;
    /// - the row declares none and `DecisionClass::requires_gate()` is false
    ///     -> `GateIndependent`, an authored claim carried by the class rather
    ///        than absence used as policy;
    /// - the row declares none while its class requires a gate
    ///     -> admission fails.
    ///
    /// `requires_gate()` is a floor, not a ceiling: a class that does not require
    /// a gate may still declare one, and then the declared gates win.
    FromDecisionDispositionClassAndGates,
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
        gate_posture: GatePostureRule::FromDecisionDispositionClassAndGates,
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
    pub qualification_target: Option<crate::architecture::QualificationEnvironment>,
    pub witness: WitnessPosture,
    pub failure: SourceFailure,
}

/// The raw material admission accepts: ONE native row, carried whole under its
/// family (5.5E2c). The first bake modeled this as a flat bag of `Option`
/// fields plus a family string, and the option soup tolerated what the law
/// forbids: an unknown family string, a QUAL row wearing a SEED failure, a row
/// supplying a field its family constant already owns — accepted whenever the
/// two sources happened to agree. Equality is not ownership. With one variant
/// per family carrying only the fields that family's native row can author,
/// duplicate authority, irrelevant fields, family/failure mismatches, and
/// undeclared families are UNREPRESENTABLE rather than policed.
///
/// Constructing a native row is not sealed — ADMISSION is: a hostile fixture
/// may author any row it likes and must still come through `admit`.
#[derive(Clone, Copy, Debug)]
pub enum GuaranteeSource {
    Seed(&'static crate::invariants::InvariantSpec),
    Legacy(&'static crate::legacy_obligations::LegacyObligation),
    Decision(&'static crate::dispositions::DecisionSpec),
    Architecture(&'static crate::architecture::PackageSpec),
    Qualification(&'static crate::architecture::QualificationProfile),
}

/// The closed vocabulary of admission refusals. Semantic identity is the RULE,
/// not its English rendering (Refusals are API, docs/14): a fixture asserts
/// `rule ==`, never a substring, so rewording a law cannot silently detach the
/// fixture from the detector it plants against. Each rule projects the
/// violated law, the typed owner of that law, and the repair direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuaranteeAdmissionRule {
    /// A source admits only through a declared family policy.
    DeclaredFamilyPolicy,
    /// A policy reads each field only from a source the family's native row
    /// can author. This refusal is reachable ONLY by mutating the policy
    /// table (e.g. declaring LEG's kind RowDeclared when a LegacyObligation
    /// authors no kind): the row side is structural and cannot drift.
    PolicyMatchesNativeRowShape,
    /// An admitted guarantee names a non-empty owner.
    NonEmptyOwner,
    /// A family whose policy reads gates from the row schedules at least one.
    RowNamesScheduledGates,
    /// An implementation-bearing decision names at least one gate (DEC-072).
    ImplementationBearingDecisionNamesGate,
}

// GuaranteeAdmissionRule::QualificationNamesTargetEnvironment died in 5.5E3a1
// when QualificationProfile::environment became the closed typed enum: an
// empty or absent environment no longer has a spelling, so the refusal it
// guarded became unrepresentable. Same law as the UNKNOWN-family hostile.

impl GuaranteeAdmissionRule {
    /// The violated law, stated as the requirement.
    pub const fn law(self) -> &'static str {
        match self {
            Self::DeclaredFamilyPolicy => {
                "a source admits only through a declared family policy"
            }
            Self::PolicyMatchesNativeRowShape => {
                "a family policy reads each field only from a source \
                 the family's native row can author"
            }
            Self::NonEmptyOwner => "an admitted guarantee names a non-empty owner",
            Self::RowNamesScheduledGates => {
                "a family whose policy reads gates from the row \
                 schedules at least one gate"
            }
            Self::ImplementationBearingDecisionNamesGate => {
                "an implementation-bearing decision names at least one gate"
            }
        }
    }

    /// The typed owner of the violated law.
    pub const fn owner(self) -> &'static str {
        match self {
            Self::DeclaredFamilyPolicy | Self::PolicyMatchesNativeRowShape => {
                "spec/guarantees.rs GUARANTEE_FAMILY_POLICIES"
            }
            Self::NonEmptyOwner => "spec/guarantees.rs GuaranteeView::owner",
            Self::RowNamesScheduledGates => "spec/guarantees.rs GatePostureRule::RowDeclared",
            Self::ImplementationBearingDecisionNamesGate => {
                "spec/dispositions.rs DecisionClass::requires_gate"
            }
        }
    }

    /// The repair direction, stated on the offending row's terms.
    pub const fn repair(self) -> &'static str {
        match self {
            Self::DeclaredFamilyPolicy => {
                "declare the family's policy row or route the row through \
                 an accepted family"
            }
            Self::PolicyMatchesNativeRowShape => {
                "restore the family's field-source rule to match what its \
                 native row schema authors"
            }
            Self::NonEmptyOwner => "author the owning document or module on the row",
            Self::RowNamesScheduledGates => "name the gates where this guarantee qualifies",
            Self::ImplementationBearingDecisionNamesGate => {
                "name the implementation gate or reclassify the decision as \
                 HistoricalReceipt or Naming"
            }
        }
    }
}

/// A refusal names the offending row and the violated rule. The rule — not a
/// prose string — is the identity a fixture asserts against; `rule.law()`,
/// `rule.owner()`, and `rule.repair()` project the human/agent diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuaranteeAdmissionFailure {
    pub id: GuaranteeRef,
    pub rule: GuaranteeAdmissionRule,
}

/// Proof that `admit` produced this view. The seal is private: no generator,
/// auditor, projector, or fixture can mint one, so a `GuaranteeView` wearing
/// this wrapper has passed the admission law by construction — the same
/// sealed-witness shape as `AdmittedAsPublicSemanticNode` and
/// `AdmittedCrossing`, arriving at the family that waited longest for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedGuarantee {
    pub view: GuaranteeView,
    seal: (),
}

/// Admit one native row through its family policy. Every `GuaranteeView`
/// field resolves through EXACTLY ONE of: a direct authored row value, a
/// declared family constant, or one named lawful derivation — and admission
/// fails closed when a required field has no such source. This function IS
/// the admission law: until the 5.5E2 bake it existed only as a Python
/// reconstruction (`audit.admit_view`), which meant the cross-family law had
/// Rust vocabulary and no Rust execution path.
///
/// The policy table remains the consumed authority, not documentation: every
/// field resolution below pairs the table's rule with the variant's native
/// row, so a table mutation (renaming a family, blanking a constant,
/// reassigning a field to a source the row cannot author) refuses every row
/// of that family at runtime instead of passing while both Python
/// derivations happen to agree.
///
/// The refusals "the row contradicts its family's constant kind" and "only
/// qualification requirements carry a target environment" died structurally
/// in 5.5E2c: a family-variant source cannot author a second kind, a foreign
/// failure disposition, an unknown family, or a target on a non-QUAL row.
pub fn admit(source: GuaranteeSource) -> Result<AdmittedGuarantee, GuaranteeAdmissionFailure> {
    use GuaranteeAdmissionRule as Rule;
    let (family, id) = match source {
        GuaranteeSource::Seed(r) => ("SEED", GuaranteeRef::Seed(SeedId(r.id))),
        GuaranteeSource::Legacy(r) => ("LEG", GuaranteeRef::Legacy(LegacyId(r.id))),
        GuaranteeSource::Decision(r) => ("DEC", GuaranteeRef::Decision(DecisionId(r.id))),
        GuaranteeSource::Architecture(p) => (
            "ARCH",
            GuaranteeRef::Architecture(ArchitectureGuaranteeId(p.id)),
        ),
        GuaranteeSource::Qualification(q) => (
            "QUAL",
            GuaranteeRef::Qualification(QualificationId {
                package: q.package,
                profile: q.profile,
            }),
        ),
    };
    let refuse = |rule: Rule| GuaranteeAdmissionFailure { id, rule };
    let policy = GUARANTEE_FAMILY_POLICIES
        .iter()
        .find(|p| p.family == family)
        .ok_or(refuse(Rule::DeclaredFamilyPolicy))?;
    let kind = match (policy.kind, source) {
        (KindRule::RowDeclared, GuaranteeSource::Seed(r)) => r.kind,
        (KindRule::FamilyConstant(k), _) => k,
        _ => return Err(refuse(Rule::PolicyMatchesNativeRowShape)),
    };
    let owner = match (policy.owner, source) {
        (OwnerRule::RowDeclared, GuaranteeSource::Seed(r)) => r.owner,
        (OwnerRule::RowDeclared, GuaranteeSource::Legacy(r)) => r.clean_owner,
        (OwnerRule::FamilyConstant(o), _) => o,
        _ => return Err(refuse(Rule::PolicyMatchesNativeRowShape)),
    };
    if owner.trim().is_empty() {
        return Err(refuse(Rule::NonEmptyOwner));
    }
    let lifetime = match (policy.lifetime, source) {
        (LifetimeRule::RowDeclared, GuaranteeSource::Seed(r)) => r.lifetime,
        (LifetimeRule::FamilyConstant(l), _) => l,
        (LifetimeRule::FromLegacyStatusAndDeletion, GuaranteeSource::Legacy(r)) => {
            crate::legacy_obligations::legacy_lifetime(
                r.active_or_closed_status,
                r.deletion_condition,
            )
        }
        (LifetimeRule::FromDecisionDisposition, GuaranteeSource::Decision(r)) => {
            r.disposition.guarantee_lifetime()
        }
        _ => return Err(refuse(Rule::PolicyMatchesNativeRowShape)),
    };
    let scheduled = |gates: &'static [GateId]| {
        if gates.is_empty() {
            Err(refuse(Rule::RowNamesScheduledGates))
        } else {
            Ok(GatePosture::Scheduled(gates))
        }
    };
    let gate_posture = match (policy.gate_posture, source) {
        (GatePostureRule::RowDeclared, GuaranteeSource::Seed(r)) => scheduled(r.gates)?,
        (GatePostureRule::RowDeclared, GuaranteeSource::Legacy(r)) => scheduled(r.gates)?,
        (GatePostureRule::RowDeclared, GuaranteeSource::Qualification(q)) => scheduled(q.gates)?,
        (GatePostureRule::FamilyConstant(g), _) => g,
        (GatePostureRule::FromDecisionDispositionClassAndGates, GuaranteeSource::Decision(r)) => {
            let retired =
                r.disposition.guarantee_lifetime() == GuaranteeLifetime::HistoricalCoverageOnly;
            if !r.gates.is_empty() {
                if retired {
                    GatePosture::HistoricalAssociation(r.gates)
                } else {
                    GatePosture::Scheduled(r.gates)
                }
            } else if r.class.requires_gate() {
                return Err(refuse(Rule::ImplementationBearingDecisionNamesGate));
            } else {
                GatePosture::GateIndependent
            }
        }
        _ => return Err(refuse(Rule::PolicyMatchesNativeRowShape)),
    };
    // A QUAL row's environment is structural (5.5E3a1): the closed enum has
    // no empty spelling, so the presence refusal that once lived here became
    // unrepresentable.
    let qualification_target = match source {
        GuaranteeSource::Qualification(q) => Some(q.environment),
        _ => None,
    };
    // The witness/target pairing is a TABLE fact the structure cannot carry:
    // a policy declaring QualificationReceipt for a family whose row has no
    // target (or the reverse) is a policy/row-shape contradiction.
    if matches!(policy.witness, WitnessPosture::QualificationReceipt)
        != qualification_target.is_some()
    {
        return Err(refuse(Rule::PolicyMatchesNativeRowShape));
    }
    let failure = match source {
        GuaranteeSource::Seed(r) => SourceFailure::Seed(r.failure_disposition),
        GuaranteeSource::Legacy(r) => SourceFailure::Legacy(r.law),
        GuaranteeSource::Decision(r) => SourceFailure::Decision(r.subject),
        GuaranteeSource::Architecture(p) => SourceFailure::Architecture(p.role),
        GuaranteeSource::Qualification(q) => SourceFailure::Qualification(q.requirement),
    };
    Ok(AdmittedGuarantee {
        view: GuaranteeView {
            id,
            family: policy.family,
            kind,
            lifetime,
            owner,
            gate_posture,
            qualification_target,
            witness: policy.witness,
            failure,
        },
        seal: (),
    })
}

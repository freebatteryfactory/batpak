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

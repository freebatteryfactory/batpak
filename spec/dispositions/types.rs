use crate::gates::GateId;
use crate::guarantees::GuaranteeLifetime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Keep,
    Lock,
    Kill,
    Supersede,
    Demote,
    Defer,
    OpenImplementation,
    RetainAsEvidence,
}

impl Disposition {
    pub const ALL: &'static [Disposition] = &[
        Disposition::Keep,
        Disposition::Lock,
        Disposition::Kill,
        Disposition::Supersede,
        Disposition::Demote,
        Disposition::Defer,
        Disposition::OpenImplementation,
        Disposition::RetainAsEvidence,
    ];

    /// The documentary tag (5.5E4c): the one spelling the generated decision
    /// ledger renders — no Python spelling map exists.
    pub const fn spelling(self) -> &'static str {
        match self {
            Disposition::Keep => "KEEP",
            Disposition::Lock => "LOCK",
            Disposition::Kill => "KILL",
            Disposition::Supersede => "SUPERSEDE",
            Disposition::Demote => "DEMOTE",
            Disposition::Defer => "DEFER",
            Disposition::OpenImplementation => "OPEN-IMPLEMENTATION",
            Disposition::RetainAsEvidence => "RETAIN-AS-EVIDENCE",
        }
    }

    /// The documentary meaning of each tag — owned here, projected into the
    /// generated ledger legend, never restated by hand.
    pub const fn meaning(self) -> &'static str {
        match self {
            Disposition::Keep => "retained as current architecture",
            Disposition::Lock => "retained and frozen against casual reopening",
            Disposition::Kill => "forbidden from the clean target",
            Disposition::Supersede => "old meaning survives through a named successor",
            Disposition::Demote => "retained only for compatibility, oracle, or evidence",
            Disposition::Defer => "outside V1; requires a real adopter and new ruling",
            Disposition::OpenImplementation => {
                "owner and law fixed; exact constant selected at named gate"
            }
            Disposition::RetainAsEvidence => {
                "historical material may be consulted, never treated as law"
            }
        }
    }

    /// The one lawful DEC lifetime derivation (5.5D4b authority closure).
    ///
    /// A DecisionSpec declares no lifetime field, so a lifetime is DERIVED from
    /// the authored disposition here — in the typed owner — and nowhere else. No
    /// projector may supply it.
    ///
    /// `Kill` is Permanent, not historical: a prohibition stays active permanent
    /// law even though the rejected mechanism is dead. `Defer` is Permanent
    /// policy excluding the subject from V1 until a new ruling reopens it, not an
    /// unfinished construction obligation.
    pub const fn guarantee_lifetime(self) -> GuaranteeLifetime {
        match self {
            Disposition::Lock
            | Disposition::Keep
            | Disposition::Kill
            | Disposition::Defer => GuaranteeLifetime::Permanent,
            Disposition::OpenImplementation => GuaranteeLifetime::UntilGate,
            Disposition::Demote => GuaranteeLifetime::UntilCompatibilityExpiry,
            Disposition::Supersede | Disposition::RetainAsEvidence => {
                GuaranteeLifetime::HistoricalCoverageOnly
            }
        }
    }

    /// Whether a decision with this disposition may own a `NotYetAdmittedBy`
    /// identity residue disposition (5.5E3d1). Only a standing posture that
    /// excludes the subject NOW while stating a lawful future entry path
    /// qualifies. `Kill` is Permanent like `Lock`, but it is a permanent
    /// PROHIBITION — a dead passport, never a pending application; `Keep` is
    /// admitted retained policy, not an exclusion; `Demote` and
    /// `OpenImplementation` are active postures with nothing pending; the
    /// historical pair has no forward policy authority at all. Expanding
    /// this set requires an explicit ruling, not a lifetime derivation.
    pub const fn may_own_not_yet_admitted_identity(self) -> bool {
        matches!(self, Disposition::Lock | Disposition::Defer)
    }
}

/// The CLOSED vocabulary of semantic contexts the stale-vocabulary scan
/// speaks (5.5E2 completion). Two kinds of variant live here and the
/// difference is law:
///
/// - the five PERMISSIVE contexts, which a retiring decision may name in
///   `stale_allowed_contexts` so a retired term appears there without a
///   marker (a ledger must be able to state what it retired);
/// - the two DEFAULT contexts, `ProductionSource` and `OrdinaryAuthoritative`,
///   which the scanner assigns to everything else. They are never permissive
///   and may never appear in a row's `stale_allowed_contexts`: a stale term
///   there requires an inline `[STALE-REF: DEC-...]` marker, always.
///
/// Before this completion the two defaults existed only as strings inside the
/// auditor — the scanner spoke a vocabulary the typed owner could not name,
/// and the enum carried `TestFixture` and `GeneratedProjection`, which no row,
/// scanner, or document ever consumed. Dead variants deleted, spoken variants
/// declared: a closed vocabulary is closed in both directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaleContext {
    DecisionLedger,
    RejectionRecord,
    SupersessionGuide,
    LegacyEvidence,
    MigrationCompatibility,
    /// DEFAULT context for production/config source. Never permissive.
    ProductionSource,
    /// DEFAULT context for every other authoritative surface. Never permissive.
    OrdinaryAuthoritative,
}

impl StaleContext {
    /// Whether a retiring decision may name this context in
    /// `stale_allowed_contexts`. The defaults refuse: allow-listing them
    /// would exempt production source or ordinary authoritative prose from
    /// the inline-marker law wholesale.
    pub const fn permissive(self) -> bool {
        match self {
            StaleContext::DecisionLedger
            | StaleContext::RejectionRecord
            | StaleContext::SupersessionGuide
            | StaleContext::LegacyEvidence
            | StaleContext::MigrationCompatibility => true,
            StaleContext::ProductionSource | StaleContext::OrdinaryAuthoritative => false,
        }
    }
}

/// What a decision actually does. The class -- not the title, ID range, document
/// section, or keyword -- decides whether the row must name an implementation or
/// qualification gate (DEC-072).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionClass {
    /// Implementation-bearing architecture: structure a gate must realize.
    Architecture,
    /// A capability the product exposes.
    Capability,
    /// A compatibility window, historical reader, or interop posture.
    Compatibility,
    /// A repository rule enforced by tooling at a gate.
    Enforcement,
    /// A record of a past outcome; carries no forward implementation obligation.
    HistoricalReceipt,
    /// Freezes a public or internal name only.
    Naming,
    /// Owner and law fixed; the mechanism or constant is selected at its gate.
    ImplementationPosture,
}

impl DecisionClass {
    pub const ALL: &'static [DecisionClass] = &[
        DecisionClass::Architecture,
        DecisionClass::Capability,
        DecisionClass::Compatibility,
        DecisionClass::Enforcement,
        DecisionClass::HistoricalReceipt,
        DecisionClass::Naming,
        DecisionClass::ImplementationPosture,
    ];

    /// The documentary spelling (5.5E4c): the variant name is the spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            DecisionClass::Architecture => "Architecture",
            DecisionClass::Capability => "Capability",
            DecisionClass::Compatibility => "Compatibility",
            DecisionClass::Enforcement => "Enforcement",
            DecisionClass::HistoricalReceipt => "HistoricalReceipt",
            DecisionClass::Naming => "Naming",
            DecisionClass::ImplementationPosture => "ImplementationPosture",
        }
    }

    /// Implementation-bearing classes must name at least one gate. There is no
    /// "architecture, but perhaps not implementation-bearing" escape hatch: a
    /// row that only records an outcome is a HistoricalReceipt, and a row that
    /// only freezes a name is Naming.
    pub const fn requires_gate(self) -> bool {
        match self {
            DecisionClass::Architecture
            | DecisionClass::Capability
            | DecisionClass::Compatibility
            | DecisionClass::Enforcement
            | DecisionClass::ImplementationPosture => true,
            DecisionClass::HistoricalReceipt | DecisionClass::Naming => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionSpec {
    pub id: &'static str,
    pub class: DecisionClass,
    pub gates: &'static [GateId],
    pub disposition: Disposition,
    pub subject: &'static str,
    pub successor: &'static str,
    /// Retired names for the concept this decision killed/superseded/demoted.
    /// Populated only on decisions that retire vocabulary; the stale-term
    /// matcher is DERIVED from these fields, never hand-maintained beside them.
    pub stale_aliases: &'static [&'static str],
    /// Contexts in which those aliases may lawfully appear without a STALE-REF.
    pub stale_allowed_contexts: &'static [StaleContext],
    /// The clean successor a reader should use instead.
    pub replacement_contract: Option<&'static str>,
}

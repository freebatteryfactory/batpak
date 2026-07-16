//! The admitted contract kinds (5.5E3c).
//!
//! docs/06 owns the MacBat contract doctrine in prose; this file is the typed
//! authority for WHICH kinds are admitted at Gate 1. The doc's own entry law
//! decides membership: a kind enters only when one declaration becomes
//! canonical, a real adopter exists, and every lowering/proof surface is
//! named. The old "Expected families include" list carried ten names; two of
//! them — StateMachine and EvidenceBody — appeared nowhere else in the
//! corpus: no owning law, no adopter, no proof surface. They are NOT
//! variants. The enum is the admitted border crossing, not a brochure for
//! countries that might someday exist; either enters later by the same entry
//! law that admitted the eight.

use crate::guarantees::GuaranteeRef;

/// One admitted contract kind. Every kind CITES the guarantee that admits
/// it — a typed, resolvable reference, so a kind whose law is deleted or
/// retired reddens the running seedcheck instead of surviving as a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    /// Typed refusal/error shape (Refusals are API; error class and public
    /// shape never drift into side tables).
    Error,
    /// Durable event identity and bytes (accepted bytes immutable; shape
    /// evolves on read).
    Event,
    /// Language-neutral schema identity, versions, canonical bytes, goldens,
    /// and their codecs.
    SchemaCodec,
    /// Projection query authority: contract, selector, shape, bounds, source
    /// stamp, completeness, and proof.
    Projection,
    /// Subscription sessions with explicit open/receive/close/overrun
    /// terminals.
    Subscription,
    /// Declared operations and their typed effect rows crossing capability
    /// terminals.
    OperationEffect,
    /// Process identity and lifecycle: ProcessContract + Coordinate +
    /// ProcessGeneration.
    Process,
    /// Module/composition facts with deterministic composition identity.
    Composition,
}

impl ContractKind {
    /// Every admitted kind, in docs/06 order.
    pub const ALL: &'static [ContractKind] = &[
        ContractKind::Error,
        ContractKind::Event,
        ContractKind::SchemaCodec,
        ContractKind::Projection,
        ContractKind::Subscription,
        ContractKind::OperationEffect,
        ContractKind::Process,
        ContractKind::Composition,
    ];

    /// The canonical documentary spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            ContractKind::Error => "Error",
            ContractKind::Event => "Event",
            ContractKind::SchemaCodec => "SchemaCodec",
            ContractKind::Projection => "Projection",
            ContractKind::Subscription => "Subscription",
            ContractKind::OperationEffect => "OperationEffect",
            ContractKind::Process => "Process",
            ContractKind::Composition => "Composition",
        }
    }

    /// The guarantee that ADMITS this kind: the corpus law under which one
    /// declaration is already canonical with real adopters and named proof
    /// surfaces. Resolution is executed seedcheck law.
    pub const fn admitting_guarantee(self) -> GuaranteeRef {
        match self {
            ContractKind::Error => GuaranteeRef::leg("LEG-047"),
            ContractKind::Event => GuaranteeRef::leg("LEG-002"),
            ContractKind::SchemaCodec => GuaranteeRef::leg("LEG-040"),
            ContractKind::Projection => GuaranteeRef::leg("LEG-029"),
            ContractKind::Subscription => GuaranteeRef::leg("LEG-060"),
            ContractKind::OperationEffect => GuaranteeRef::leg("LEG-036"),
            ContractKind::Process => GuaranteeRef::dec("DEC-009"),
            ContractKind::Composition => GuaranteeRef::leg("LEG-041"),
        }
    }
}

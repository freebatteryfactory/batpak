//! Pass 4 — Contract IR root.
//!
//! Lowers a [`ValidatedDeclaration`] into the typed [`ContractIr`]: the single
//! semantic representation every downstream pass (lowering → emit → normalize →
//! tokenize) consumes. This root owns the front-door kind vocabulary
//! ([`ContractKindIr`]) and the exhaustive dispatch into the per-kind builders;
//! the per-kind sub-IRs themselves live in the sibling modules
//! (`error`, `event`, `variant_inventory`, `projection`, `subscription`,
//! `operation`), each owned by its own build cluster.
//!
//! # Extension contract
//!
//! [`ContractKindIr`] has **exactly six** variants in v1. Adding a seventh kind is
//! a pure addition made in that kind's own crate packet — never here in crate 1:
//! add a [`ContractKind`] variant, add a `ContractKindIr` variant plus an
//! `ir/<kind>.rs` with `build`, and add the matching arm to [`build`]. Because a
//! wildcard/named catch-all over an enum is denied repo-wide, the exhaustive match
//! in [`build`] mechanically fails to compile until the new arm exists — that
//! compiler-enforced non-exhaustiveness *is* the extension contract. The other
//! seven reserved kind names (Effect, StateMachine, Evidence, Codec,
//! IntegrityDomain, PlatformCapability, DependencyFacade, Module) exist ONLY as a
//! documented reserved list in `lowering/mod.rs` + the README — never as variants
//! here, and never as speculative empty drawers.

// Per-kind sub-IR modules. Owned by the six-surface fan-out clusters (7–12); this
// root only declares them and dispatches into their `build` functions. Declared
// here so those clusters slot in without editing this file.
pub mod error;
pub mod event;
pub mod operation;
pub mod projection;
pub mod subscription;
pub mod variant_inventory;

pub use self::error::ErrorIr;
pub use self::event::EventIr;
pub use self::operation::OperationIr;
pub use self::projection::ProjectionIr;
pub use self::subscription::SubscriptionIr;
pub use self::variant_inventory::VariantInventoryIr;

use crate::diagnostic::Diagnostics;
use crate::identity::{
    Concept, ContractId, ContractKind, ContractVersion, InvariantId, ObservationId, Owner, WitnessId,
};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// The typed intermediate representation of one contract declaration.
///
/// Constructed by [`build`] from a [`ValidatedDeclaration`]; consumed by the
/// lowering pass. Identity fields (`id`, `version`, `concept`, `owner`) are
/// kind-agnostic; the kind-specific payload lives in `kind`.
pub struct ContractIr {
    /// Deterministic, expansion-time contract identity (`concept/kind/ident`).
    pub id: ContractId,
    /// Declaration schema version (not the wire version).
    pub version: ContractVersion,
    /// The comp-sci concept this contract realizes.
    pub concept: Concept,
    /// The kind-specific typed payload.
    pub kind: ContractKindIr,
    /// Authoring blame handle — symbolic (caller-module sentinel) at expansion
    /// time; resolved globally by Muterprater in crate 3.
    pub owner: Owner,
    /// Reserved, typed-empty in v1 (no speculative payload structs).
    pub invariants: Vec<InvariantId>,
    /// Reserved, typed-empty in v1.
    pub witnesses: Vec<WitnessId>,
    /// Reserved, typed-empty in v1.
    pub observations: Vec<ObservationId>,
}

/// The kind-specific typed payload of a [`ContractIr`].
///
/// Exactly six variants in v1. A wildcard/named catch-all over this enum is denied
/// repo-wide, so every consumer that matches it is forced to handle each kind — the
/// mechanism that makes adding a kind a compile-checked, purely-additive change.
pub enum ContractKindIr {
    /// A `#[derive(Error)]` contract.
    Error(ErrorIr),
    /// A `#[derive(Event)]` payload contract.
    Event(EventIr),
    /// A `#[derive(VariantInventory)]` unit-enum contract.
    VariantInventory(VariantInventoryIr),
    /// A `#[derive(Projection)]` event-sourced contract.
    Projection(ProjectionIr),
    /// A `#[derive(Subscription)]` multi-event reactor contract.
    Subscription(SubscriptionIr),
    /// An `#[operation]` attribute contract.
    Operation(OperationIr),
}

/// Build the typed contract IR from a validated declaration.
///
/// Dispatches on the declared [`ContractKind`] with an exhaustive six-arm match
/// into the per-kind builder, then assembles the kind-agnostic identity fields.
///
/// # Errors
///
/// Returns [`Diagnostics`] when the per-kind builder rejects the declaration.
pub fn build(decl: &ValidatedDeclaration) -> Result<ContractIr, Diagnostics> {
    let ident = declared_ident(&decl.item);
    let id = ContractId::new(decl.kind.concept(), decl.kind, ident);
    // Owner is SYMBOLIC at expansion time: MacBat cannot know the caller's module
    // path, so the CallerModule sentinel is the documented-correct value here.
    // Global resolution is Muterprater enrichment in crate 3. The declared ident
    // is already carried in the ContractId — duplicating it into owner would fake
    // a provenance MacBat does not have.
    let owner = Owner::caller_module();

    let kind = match decl.kind {
        ContractKind::Error => ContractKindIr::Error(self::error::build(decl)?),
        ContractKind::Event => ContractKindIr::Event(self::event::build(decl)?),
        ContractKind::VariantInventory => {
            ContractKindIr::VariantInventory(self::variant_inventory::build(decl)?)
        }
        ContractKind::Projection => ContractKindIr::Projection(self::projection::build(decl)?),
        ContractKind::Subscription => {
            ContractKindIr::Subscription(self::subscription::build(decl)?)
        }
        ContractKind::Operation => ContractKindIr::Operation(self::operation::build(decl)?),
    };

    Ok(ContractIr {
        id,
        // FLAG (underspecified): the contract pins no default/const for the v1
        // declaration schema version; `1` is used as the v1 schema version.
        version: ContractVersion(1),
        concept: decl.kind.concept(),
        kind,
        owner,
        invariants: Vec::new(),
        witnesses: Vec::new(),
        observations: Vec::new(),
    })
}

/// The declared item's identifier — the type ident for a derive, the function
/// ident for an attribute macro.
fn declared_ident(item: &SynItem) -> &syn::Ident {
    match item {
        SynItem::Derive(input) => &input.ident,
        SynItem::Attribute { item, .. } => &item.sig.ident,
    }
}

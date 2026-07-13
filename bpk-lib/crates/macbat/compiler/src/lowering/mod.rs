//! Pass 5 — lowering: Contract IR → [`LoweringPlan`], plus the LOWERING
//! EXTENSION CONTRACT that is the sole seam by which new contract kinds join the
//! compiler.
//!
//! # Lowering extension contract
//!
//! [`plan`] dispatches with an EXHAUSTIVE six-arm match on [`ContractKindIr`]
//! (no wildcard, no named catch-all). Because a wildcard/named catch-all over an
//! enum is denied repo-wide, adding a seventh [`ContractKindIr`] variant
//! MECHANICALLY forces a new arm here (and a new [`ContractLowering`] impl) or
//! the build fails. That compiler-enforced non-exhaustiveness *is* the extension
//! contract — no kind can be silently unhandled.
//!
//! To add a kind later (in its OWN crate's packet — never speculatively in
//! crate 1):
//!
//! 1. add a `ContractKind` variant (`identity.rs`);
//! 2. add a [`ContractKindIr`] variant plus `ir/<kind>.rs` with its `build`;
//! 3. add `lowering/<kind>.rs` implementing [`ContractLowering`], a new arm in
//!    [`plan`]'s match, and any new `LoweringRole` variants;
//! 4. add one `AttributeGrammar` const in `grammar/catalog.rs` and a
//!    `grammar_for` arm;
//! 5. add a specimen directory.
//!
//! That is a PURE ADDITION: no existing arm changes semantics, and the
//! exhaustive matches prove no kind is silently dropped. Zero speculative
//! structs, zero empty drawers.
//!
//! ## Reserved kind names (documented only — NOT enum variants)
//!
//! The following contract-kind names are RESERVED for future crates' packets and
//! exist ONLY as this documented list (also in the subsystem README). They are
//! deliberately NOT `ContractKindIr`/`ContractKind` variants until their owning
//! crate's packet arrives: `Effect`, `StateMachine`, `Evidence`, `Codec`,
//! `IntegrityDomain`, `PlatformCapability`, `DependencyFacade`, `Module`.

pub mod error;
pub mod event;
pub mod operation;
pub mod projection;
pub mod subscription;
pub mod variant_inventory;

use crate::diagnostic::Diagnostics;
use crate::emit::EmittedItemModel;
use crate::hygiene::LoweringContext;
use crate::identity::{ContractId, LoweringRole};
use crate::ir::{ContractIr, ContractKindIr};
use crate::origin::OriginEdge;

/// A contract's complete lowering: the emitted item model plus the cross-family
/// reference edges (each records which crate root a reference resolves through).
#[derive(Clone)]
pub struct LoweringPlan {
    pub contract: ContractId,
    pub model: EmittedItemModel,
    pub edges: Vec<LoweringEdge>,
}

/// One cross-family reference produced by a lowering: the role that emitted it,
/// the crate root it resolves through, and its origin/blame edge.
#[derive(Clone)]
pub struct LoweringEdge {
    pub role: LoweringRole,
    pub target: CratePathRef,
    pub origin: OriginEdge,
}

/// Which crate root a cross-family reference resolves through. `Runtime` is
/// `::syncbat` in v1 (a single literal in `hygiene.rs` that crate 4 flips);
/// `CoreStd` marks the `::core`/`::std`-only lowerings (Error, VariantInventory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CratePathRef {
    Batpak,
    Runtime,
    CoreStd,
}

/// The extension seam. Each surface (Clusters 7-12) provides a marker type
/// implementing this trait — `ErrorLowering`, `EventLowering`,
/// `VariantInventoryLowering`, `ProjectionLowering`, `SubscriptionLowering`,
/// `OperationLowering` — with `Ir` set to that kind's sub-IR.
pub trait ContractLowering {
    /// The per-kind sub-IR this lowering consumes (e.g. `ErrorIr`).
    type Ir;

    /// Lower this kind's IR into a plan.
    ///
    /// # Errors
    ///
    /// Returns a [`Diagnostics`] when the kind's IR cannot be lowered (e.g. an
    /// output-shape violation this pass is responsible for detecting).
    fn plan(ir: &Self::Ir, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics>;
}

/// Dispatch a contract's IR to its per-kind lowering.
///
/// Exhaustive over [`ContractKindIr`] — the extension seam (see module docs).
///
/// # Errors
///
/// Propagates the per-kind [`ContractLowering::plan`] error unchanged.
pub fn plan(ir: &ContractIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
    match &ir.kind {
        ContractKindIr::Error(inner) => {
            <error::ErrorLowering as ContractLowering>::plan(inner, ctx)
        }
        ContractKindIr::Event(inner) => {
            <event::EventLowering as ContractLowering>::plan(inner, ctx)
        }
        ContractKindIr::VariantInventory(inner) => {
            <variant_inventory::VariantInventoryLowering as ContractLowering>::plan(inner, ctx)
        }
        ContractKindIr::Projection(inner) => {
            <projection::ProjectionLowering as ContractLowering>::plan(inner, ctx)
        }
        ContractKindIr::Subscription(inner) => {
            <subscription::SubscriptionLowering as ContractLowering>::plan(inner, ctx)
        }
        ContractKindIr::Operation(inner) => {
            <operation::OperationLowering as ContractLowering>::plan(inner, ctx)
        }
    }
}

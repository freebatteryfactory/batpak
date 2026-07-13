//! Cluster 8 — Event lowering (Pass 5 for the Event surface).
//!
//! Lowers an [`EventIr`] into the three emitted items the current
//! `#[derive(EventPayload)]` produces, byte-equivalent to `event_payload.rs`:
//!
//! 1. [`LoweringRole::EventPayloadImpl`] — `impl ::batpak::event::EventPayload`
//!    with `const KIND = EventKind::custom(category, type_id)` and
//!    `const PAYLOAD_VERSION`.
//! 2. [`LoweringRole::EventInventorySubmit`] — an unconditional
//!    `const _: () = { inventory::submit! { EventPayloadRegistration { .. } } };`
//!    so payloads in dependency crates stay visible to a downstream registry
//!    validator.
//! 3. [`LoweringRole::EventCollisionCheck`] — a `#[cfg(test)] #[test]` fn calling
//!    `assert_no_kind_collisions()`, whose name keeps the legacy observable
//!    scheme `__batpak_kind_collision_check_<snake>` (parity exception, minted
//!    via [`ReservedIdent::collision_check`]).
//!
//! The payload self-type comes from `ir.ident` (true user span, preserved into
//! every [`OriginEdge`] for blame); events reject generics upstream so the bare
//! ident suffices. Every `::batpak` root routes through the hygiene crate-path,
//! and the `__private` path is preserved verbatim so generated code is
//! unchanged. This surface is proven structurally in-lane (snapshot + positive
//! `syn::parse2`); real trait behavior against `::batpak` waits for the J1/J2
//! integration event.

use quote::quote;

use crate::diagnostic::Diagnostics;
use crate::emit::item_model::{EmittedItem, EmittedItemModel, ItemNode};
use crate::hygiene::{LoweringContext, ReservedIdent};
use crate::identity::{ContractId, EmittedItemId, LoweringRole};
use crate::ir::event::EventIr;
use crate::lowering::{ContractLowering, CratePathRef, LoweringEdge, LoweringPlan};
use crate::origin::OriginEdge;

/// The Event surface's [`ContractLowering`] implementation.
pub struct EventLowering;

impl ContractLowering for EventLowering {
    type Ir = EventIr;

    fn plan(ir: &EventIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
        let contract = ctx.contract;
        let ident = &ir.ident;
        let user_span = ident.span();
        let crate_path = &ctx.hygiene.crate_path;

        // `::batpak::…` roots, all spanned through the hygiene crate-path.
        let event_payload = crate_path.join("event::EventPayload");
        let event_kind = crate_path.join("event::EventKind");
        let inventory = crate_path.join("__private::inventory");
        let registration = crate_path.join("__private::EventPayloadRegistration");
        let assert_no_collisions = crate_path.join("__private::assert_no_kind_collisions");

        // Interpolated as narrow suffixed literals (`10u8`, `1u16`, `40961u16`),
        // matching the derive's `#category`/`#type_id`/`#kind_bits` emission.
        let category = ir.category;
        let type_id = ir.type_id;
        let payload_version = ir.payload_version;
        let kind_bits = ir.kind_bits;

        // Item 1 — the trait impl. Events reject generics upstream, so the
        // self-type is the bare ident (no generics / where-clause to carry).
        let payload_impl = ItemNode::TraitImpl {
            trait_path: event_payload,
            self_ty: quote! { #ident },
            body: quote! {
                const KIND: #event_kind =
                    #event_kind::custom(#category, #type_id);
                const PAYLOAD_VERSION: u16 = #payload_version;
            },
        };

        // Item 2 — the unconditional inventory registration block.
        let inventory_submit = ItemNode::AnonBlock {
            tokens: quote! {
                const _: () = {
                    #inventory::submit! {
                        #registration {
                            kind_bits: #kind_bits, payload_version: #payload_version,
                            type_name: concat!(module_path!(), "::", stringify!(#ident)),
                        }
                    }
                };
            },
        };

        // Item 3 — the test-only collision guard; legacy observable name.
        let collision_fn = ReservedIdent::collision_check(ident);
        let collision_check = ItemNode::TestFn {
            ident: collision_fn.clone(),
            tokens: quote! {
                #[cfg(test)]
                #[test]
                fn #collision_fn() {
                    #assert_no_collisions();
                }
            },
        };

        // Assemble the shared spine. Ids 0/1/2 are source-order placeholders;
        // `emit::finalize` re-densifies (ratified convention). `normalize`
        // re-sorts by `(role, sort_key)` — the three distinct roles keep the
        // same relative order either way. Each `OriginEdge` / id / role is
        // constructed fresh (never moved twice) so the code depends only on
        // the pinned derives, not on `Copy` for the identity newtypes.
        let items = vec![
            EmittedItem {
                id: EmittedItemId::new(0),
                role: LoweringRole::EventPayloadImpl,
                sort_key: contract.as_str().to_owned(),
                node: payload_impl,
                origin: origin_for(0, LoweringRole::EventPayloadImpl, contract, user_span),
            },
            EmittedItem {
                id: EmittedItemId::new(1),
                role: LoweringRole::EventInventorySubmit,
                sort_key: contract.as_str().to_owned(),
                node: inventory_submit,
                origin: origin_for(1, LoweringRole::EventInventorySubmit, contract, user_span),
            },
            EmittedItem {
                id: EmittedItemId::new(2),
                role: LoweringRole::EventCollisionCheck,
                sort_key: contract.as_str().to_owned(),
                node: collision_check,
                origin: origin_for(2, LoweringRole::EventCollisionCheck, contract, user_span),
            },
        ];

        // All three roles resolve through the `::batpak` root.
        let edges = vec![
            LoweringEdge {
                role: LoweringRole::EventPayloadImpl,
                target: CratePathRef::Batpak,
                origin: origin_for(0, LoweringRole::EventPayloadImpl, contract, user_span),
            },
            LoweringEdge {
                role: LoweringRole::EventInventorySubmit,
                target: CratePathRef::Batpak,
                origin: origin_for(1, LoweringRole::EventInventorySubmit, contract, user_span),
            },
            LoweringEdge {
                role: LoweringRole::EventCollisionCheck,
                target: CratePathRef::Batpak,
                origin: origin_for(2, LoweringRole::EventCollisionCheck, contract, user_span),
            },
        ];

        Ok(LoweringPlan {
            contract: contract.clone(),
            model: EmittedItemModel {
                contract: contract.clone(),
                items,
            },
            edges,
        })
    }
}

/// Build a fresh [`OriginEdge`] for one emitted item.
///
/// `user_span` is the payload type's true ident span (from `ir.ident`), so
/// origin blame points at the user's declaration, not `call_site`. Constructed
/// anew at each call so neither the [`EmittedItemId`] nor the [`LoweringRole`]
/// is moved twice — the lowering needs no `Copy` bound on the identity
/// newtypes, only the pinned `Clone` on [`ContractId`].
fn origin_for(
    index: u32,
    role: LoweringRole,
    contract: &ContractId,
    user_span: proc_macro2::Span,
) -> OriginEdge {
    OriginEdge {
        emitted: EmittedItemId::new(index),
        contract: contract.clone(),
        lowering: role,
        declaration_field: None,
        user_span,
    }
}

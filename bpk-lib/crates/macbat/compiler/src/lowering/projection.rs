//! Cluster 10 — Projection lowering (Pass 5 for the Projection surface).
//!
//! Lowers a [`ProjectionIr`] into the two emitted items the current
//! `#[derive(EventSourced)]` produces, structurally equivalent to
//! `expand_event_sourced`:
//!
//! 1. [`LoweringRole::ProjectionInputAssertion`] — a
//!    `const _: fn() = || { … };` block pinning the `input =` type to
//!    `::batpak::event::ProjectionInput` at expansion site (a non-`ProjectionInput`
//!    input errors here, with the attribute path visible in the trace).
//! 2. [`LoweringRole::ProjectionImpl`] — the `impl ::batpak::event::EventSourced`
//!    with `type Input`, the optional `STATE_CONTRACT` + `state_extent` pair,
//!    `from_events`, `apply_event` (handler-signature pins + per-binding dispatch
//!    arms), `relevant_event_kinds` (a `static KINDS` array), and
//!    `schema_version` (the `cache_version`).
//!
//! **O-21 interim decision.** The matched-kind decode-failure arm emits a call to
//! `::batpak::__private::projection_decode_abort(stringify!(EventTy), &__e)` — a
//! `-> !` hard-signal helper in `macbat-registration` — **not** a literal
//! `::core::panic!` token. This preserves today's hard-signal semantics and
//! message content while keeping the emitted token stream free of a `panic!`
//! macro; the fallible-trait successor (a returned typed error) lands in crate 2.
//!
//! Every `::batpak` root routes through [`CratePath`]; the `__private` path is
//! preserved so generated code is unchanged. This surface is proven structurally
//! in-lane (snapshot + positive `syn::parse2`); real trait behavior against
//! `::batpak` waits for the J1/J2 integration event.

use quote::quote;

use crate::diagnostic::Diagnostics;
use crate::emit::item_model::{EmittedItem, EmittedItemModel, ItemNode};
use crate::hygiene::LoweringContext;
use crate::identity::{ContractId, EmittedItemId, LoweringRole};
use crate::ir::projection::ProjectionIr;
use crate::lowering::{ContractLowering, CratePathRef, LoweringEdge, LoweringPlan};
use crate::origin::OriginEdge;

/// The Projection surface's [`ContractLowering`] implementation.
pub struct ProjectionLowering;

impl ContractLowering for ProjectionLowering {
    type Ir = ProjectionIr;

    fn plan(ir: &ProjectionIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
        let contract = ctx.contract;
        // Target ident + generics come off the IR directly (never string-parsed
        // from the local `ContractId`). Generic projection targets are supported
        // by riding `#impl_generics` / `#ty_generics` / `#where_clause` inside the
        // `TraitImpl` token slots.
        let ident = &ir.ident;
        let (impl_generics, ty_generics, where_clause) = ir.generics.split_for_impl();
        let ident_span = ir.ident.span();
        let crate_path = &ctx.hygiene.crate_path;

        // `::batpak::…` roots, all spanned through the hygiene crate-path.
        let event_sourced = crate_path.join("event::EventSourced");
        let projection_event = crate_path.join("event::ProjectionEvent");
        let event_kind = crate_path.join("event::EventKind");
        let event_payload = crate_path.join("event::EventPayload");
        let decode_typed = crate_path.join("event::DecodeTyped");
        let projection_input = crate_path.join("event::ProjectionInput");
        let projection_state_contract = crate_path.join("event::ProjectionStateContract");
        let state_extent = crate_path.join("event::StateExtent");
        let state_extent_cost = crate_path.join("event::StateExtentCost");
        // O-21: the `-> !` hard-signal helper — NOT a literal `panic!` token.
        let decode_abort = crate_path.join("__private::projection_decode_abort");

        let input_path = &ir.input;

        // STATE_CONTRACT + state_extent, emitted only when `state_max_cardinality`
        // was declared (always 1 in v1; Pass 3 rejects any other value). The
        // reported `state_extent` is a fixed cardinality of 1.
        let state_contract_impl = match &ir.state_contract {
            None => quote! {},
            Some(state) => {
                let max_cardinality = state.max_cardinality;
                quote! {
                    const STATE_CONTRACT: #projection_state_contract =
                        #projection_state_contract::Bounded {
                            key_space: ::core::concat!(
                                ::core::module_path!(),
                                "::",
                                ::core::stringify!(#ident)
                            ),
                            max_cardinality: #max_cardinality,
                            retention_policy: "derive-event-sourced-state-object",
                            compaction_policy: "projection-cache-overwrite",
                            checkpoint_policy: "projection-cache",
                        };

                    fn state_extent(&self) -> #state_extent {
                        let _ = self;
                        #state_extent::cardinality(
                            1,
                            #state_extent_cost::ConstantTime,
                        )
                    }
                }
            }
        };

        // Handler-signature pins — one per binding, inside the generic impl so
        // they can reference `Self`. A wrong `fn(&mut self, &T)` signature errors
        // at this fn-pointer coercion, not inside an opaque dispatch arm.
        let handler_checks: Vec<proc_macro2::TokenStream> = ir
            .bindings
            .iter()
            .map(|b| {
                let event_ty = &b.event;
                let handler_fn = &b.handler;
                quote! {
                    let _: fn(&mut Self, &#event_ty) = Self::#handler_fn;
                }
            })
            .collect();

        // Dispatch arms — one per binding. `Ok(Some)` → matched kind decoded,
        // call handler; `Ok(None)` → wrong kind, fall through; `Err` → matched
        // kind but decode failed, a hard correctness signal (O-21 abort helper).
        let arms: Vec<proc_macro2::TokenStream> = ir
            .bindings
            .iter()
            .map(|b| {
                let event_ty = &b.event;
                let handler_fn = &b.handler;
                quote! {
                    match #decode_typed::route_typed::<#event_ty>(event) {
                        ::core::result::Result::Ok(::core::option::Option::Some(__p)) => {
                            self.#handler_fn(&__p);
                            return;
                        }
                        ::core::result::Result::Ok(::core::option::Option::None) => {}
                        ::core::result::Result::Err(__e) => {
                            #decode_abort(::core::stringify!(#event_ty), &__e);
                        }
                    }
                }
            })
            .collect();

        // `relevant_event_kinds`: compile-time const array from the `event =`
        // list (single source of truth — sync drift is impossible).
        let kind_exprs: Vec<proc_macro2::TokenStream> = ir
            .bindings
            .iter()
            .map(|b| {
                let event_ty = &b.event;
                quote! {
                    <#event_ty as #event_payload>::KIND
                }
            })
            .collect();
        let kind_count = ir.bindings.len();
        let cache_version = ir.cache_version;

        // Item 1 — the input-type assertion. The local fn name is a fixed
        // emit-local ident (never cross-item), renamed `__batpak_*` → `__bpk_*`
        // versus the legacy derive; behavior-neutral inside the `const _` block.
        let input_assertion = ItemNode::AnonBlock {
            tokens: quote! {
                const _: fn() = || {
                    fn __bpk_assert_projection_input<T: #projection_input>() {}
                    __bpk_assert_projection_input::<#input_path>();
                };
            },
        };

        // Item 2 — the `EventSourced` trait impl. Impl generics ride in the
        // `trait_path` slot (`impl #impl_generics <trait> …`) and the type
        // generics + where-clause ride in `self_ty`
        // (`… for #ident #ty_generics #where_clause`), so a generic projection
        // target lowers byte-equivalently to the legacy expansion.
        let projection_impl = ItemNode::TraitImpl {
            trait_path: quote! { #impl_generics #event_sourced },
            self_ty: quote! { #ident #ty_generics #where_clause },
            body: quote! {
                type Input = #input_path;

                #state_contract_impl

                fn from_events(
                    events: &[#projection_event<Self>],
                ) -> ::core::option::Option<Self> {
                    if events.is_empty() {
                        return ::core::option::Option::None;
                    }
                    let mut state: Self = ::core::default::Default::default();
                    for __ev in events {
                        state.apply_event(__ev);
                    }
                    ::core::option::Option::Some(state)
                }

                fn apply_event(&mut self, event: &#projection_event<Self>) {
                    #(#handler_checks)*
                    #(#arms)*
                    // Fall-through: a kind outside `relevant_event_kinds()` — a
                    // normal skip, not an error.
                    let _ = event;
                }

                fn relevant_event_kinds() -> &'static [#event_kind] {
                    static KINDS: [#event_kind; #kind_count] = [
                        #(#kind_exprs),*
                    ];
                    &KINDS
                }

                fn schema_version() -> u64 {
                    #cache_version
                }
            },
        };

        // Assemble the shared spine. Dense source-order ids (0,1) fix the
        // tokenize concatenation order; `normalize` re-sorts by `(role, sort_key)`
        // — the two distinct roles keep the same relative order either way.
        let sort_key = contract.as_str().to_owned();

        let mut items = Vec::with_capacity(2);
        let mut edges = Vec::with_capacity(2);

        emit(
            &mut items,
            &mut edges,
            EmittedItemId::new(0),
            LoweringRole::ProjectionInputAssertion,
            contract,
            &sort_key,
            ident_span,
            input_assertion,
        );
        emit(
            &mut items,
            &mut edges,
            EmittedItemId::new(1),
            LoweringRole::ProjectionImpl,
            contract,
            &sort_key,
            ident_span,
            projection_impl,
        );

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

/// Push one emitted item plus its lowering edge, sharing a single origin.
///
/// Both the [`EmittedItem`] and its [`LoweringEdge`] carry the same
/// [`OriginEdge`] (emitted id, contract, role, user span); both Projection roles
/// resolve through the `::batpak` root ([`CratePathRef::Batpak`]). The
/// `user_span` is the target ident's span, so origin blame points at the
/// declared projection type.
fn emit(
    items: &mut Vec<EmittedItem>,
    edges: &mut Vec<LoweringEdge>,
    id: EmittedItemId,
    role: LoweringRole,
    contract: &ContractId,
    sort_key: &str,
    user_span: proc_macro2::Span,
    node: ItemNode,
) {
    let origin = OriginEdge {
        emitted: id,
        contract: contract.clone(),
        lowering: role,
        declaration_field: None,
        user_span,
    };
    items.push(EmittedItem {
        id,
        role,
        sort_key: sort_key.to_owned(),
        node,
        origin: origin.clone(),
    });
    edges.push(LoweringEdge {
        role,
        target: CratePathRef::Batpak,
        origin,
    });
}

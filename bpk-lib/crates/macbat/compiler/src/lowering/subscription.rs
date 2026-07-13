//! Pass 5 lowering for the `Subscription` contract kind.
//!
//! Reproduces `expand_multi_event_reactor` (the legacy `macros/src/lib.rs`
//! `MultiEventReactor` derive) behaviourally: two attribute-site assertions
//! (`input: ProjectionInput`; `error: Send + Sync + 'static + Error`) plus the
//! `MultiReactive` impl whose `dispatch` routes each bound event kind through
//! `DecodeTyped::route_typed`, builds a typed `StoredEvent`, and calls the
//! handler.
//!
//! DELIBERATE DIVERGENCE FROM PROJECTION (O-26), PRESERVED: a user handler error
//! maps to `MultiDispatchError::User`, while a matched-kind decode failure
//! **returns** `MultiDispatchError::Decode(__e)` — it does NOT hard-signal the
//! way the projection surface does. That distinction is intrinsic to this
//! surface and is kept byte-equivalent to the legacy expansion.
//!
//! The only structural changes from the legacy expansion: the local assertion
//! fn idents move from the `__batpak_assert_*` prefix to `__bpk_*` (behaviour-
//! neutral — they are local, non-cross-item names, so a fixed literal rename
//! per the hygiene convention is correct), and every `::batpak` root is spanned
//! through `CratePath` instead of a bare literal.

use quote::quote;
use syn::spanned::Spanned;

use crate::diagnostic::Diagnostics;
use crate::emit::item_model::{EmittedItem, EmittedItemModel, ItemNode};
use crate::hygiene::LoweringContext;
use crate::identity::{ContractId, EmittedItemId, LoweringRole};
use crate::ir::subscription::SubscriptionIr;
use crate::lowering::{ContractLowering, CratePathRef, LoweringEdge, LoweringPlan};
use crate::origin::OriginEdge;

/// The `Subscription` lowering seam. `plan` is also exposed as a module-level
/// free function so the Pass-5 dispatcher can call it directly without importing
/// the trait.
pub struct SubscriptionLowering;

impl ContractLowering for SubscriptionLowering {
    type Ir = SubscriptionIr;

    fn plan(ir: &Self::Ir, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
        // A bare `plan(..)` resolves to the module free fn below (value
        // namespace), never to this associated method, so this is a delegation
        // and not a recursion.
        plan(ir, ctx)
    }
}

/// Lower a subscription sub-IR into its emitted-item plan.
///
/// # Errors
///
/// Returns [`Diagnostics`] to satisfy the [`ContractLowering`] seam; the
/// subscription lowering is total over a well-formed [`SubscriptionIr`] and does
/// not itself produce diagnostics.
pub fn plan(ir: &SubscriptionIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
    let contract = ctx.contract;
    let crate_path = &ctx.hygiene.crate_path;
    let event_root = crate_path.join("event");
    let store_root = crate_path.join("store");

    let ident = &ir.ident;
    let generics = &ir.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let input_path = &ir.input;
    let error_path = &ir.error;
    let bindings = &ir.bindings;

    let kind_count = bindings.len();
    let kind_exprs: Vec<proc_macro2::TokenStream> = bindings
        .iter()
        .map(|binding| {
            let event_ty = &binding.event;
            quote! { <#event_ty as #event_root::EventPayload>::KIND }
        })
        .collect();

    // Handler-signature pins live inside the generic impl so they can reference
    // `Self`-with-type-params; module-scope pins cannot reintroduce generics. A
    // mismatched handler signature surfaces at the user's handler.
    let handler_checks: Vec<proc_macro2::TokenStream> = bindings
        .iter()
        .map(|binding| {
            let event_ty = &binding.event;
            let handler_fn = &binding.handler;
            quote! {
                let _: fn(
                    &mut Self,
                    &#event_root::StoredEvent<#event_ty>,
                    &mut #store_root::ReactionBatch,
                    ::core::option::Option<&#store_root::AtLeastOnce>,
                ) -> ::core::result::Result<(), #error_path> = Self::#handler_fn;
            }
        })
        .collect();

    // One dispatch arm per binding. `route_typed` decodes matched kinds to the
    // bound type; wrong-kind events fall through and dispatch returns `Ok(())`;
    // a matched-kind decode failure returns `MultiDispatchError::Decode` (O-26).
    let arms: Vec<proc_macro2::TokenStream> = bindings
        .iter()
        .map(|binding| {
            let event_ty = &binding.event;
            let handler_fn = &binding.handler;
            quote! {
                match #event_root::DecodeTyped::route_typed::<#event_ty>(&event.event) {
                    ::core::result::Result::Ok(::core::option::Option::Some(__p)) => {
                        let __typed_event = #event_root::StoredEvent {
                            coordinate: event.coordinate.clone(),
                            event: #event_root::Event {
                                header: event.event.header.clone(),
                                payload: __p,
                                hash_chain: event.event.hash_chain.clone(),
                            },
                        };
                        return self
                            .#handler_fn(&__typed_event, out, at_least_once)
                            .map_err(#event_root::MultiDispatchError::User);
                    }
                    ::core::result::Result::Ok(::core::option::Option::None) => {}
                    ::core::result::Result::Err(__e) => {
                        return ::core::result::Result::Err(
                            #event_root::MultiDispatchError::Decode(__e)
                        );
                    }
                }
            }
        })
        .collect();

    // Attribute-site assertions: pin `input: ProjectionInput` and
    // `error: Send + Sync + 'static + Error` at the declaration site so a wrong
    // attribute value errors with the attribute path visible, not from inside
    // trait-impl machinery. Local idents renamed `__batpak_assert_*` -> `__bpk_*`.
    let assert_input = quote! {
        const _: fn() = || {
            fn __bpk_assert_projection_input<T: #event_root::ProjectionInput>() {}
            __bpk_assert_projection_input::<#input_path>();
        };
    };
    let assert_error = quote! {
        const _: fn() = || {
            fn __bpk_assert_error<
                T: ::core::marker::Send
                    + ::core::marker::Sync
                    + 'static
                    + ::std::error::Error,
            >() {}
            __bpk_assert_error::<#error_path>();
        };
    };

    let impl_body = quote! {
        type Error = #error_path;

        fn relevant_event_kinds() -> &'static [#event_root::EventKind] {
            static KINDS: [#event_root::EventKind; #kind_count] = [
                #(#kind_exprs),*
            ];
            &KINDS
        }

        fn dispatch(
            &mut self,
            event: &#event_root::StoredEvent<
                <#input_path as #event_root::ProjectionInput>::Payload,
            >,
            out: &mut #store_root::ReactionBatch,
            at_least_once: ::core::option::Option<&#store_root::AtLeastOnce>,
        ) -> ::core::result::Result<(), #event_root::MultiDispatchError<Self::Error>> {
            #(#handler_checks)*
            #(#arms)*
            // Wrong kind / no binding matched — silent filter.
            ::core::result::Result::Ok(())
        }
    };

    // Generics are folded into the structured `TraitImpl` header: impl-generics
    // lead the trait path (`impl <T> Trait`), and the type-generics + where
    // clause trail the self type (`for Ident<T> where …`). Both render empty for
    // a non-generic reactor, so the header is byte-identical there.
    let trait_path = quote! { #impl_generics #event_root::MultiReactive<#input_path> };
    let self_ty = quote! { #ident #ty_generics #where_clause };

    let mut items: Vec<EmittedItem> = Vec::new();
    let mut edges: Vec<LoweringEdge> = Vec::new();

    let (input_item, input_edge) = assemble(
        0,
        LoweringRole::SubscriptionAttrAssertion,
        "subscription-assert-input",
        ItemNode::AnonBlock {
            tokens: assert_input,
        },
        contract,
        input_path.span(),
    );
    items.push(input_item);
    edges.push(input_edge);

    let (error_item, error_edge) = assemble(
        1,
        LoweringRole::SubscriptionAttrAssertion,
        "subscription-assert-error",
        ItemNode::AnonBlock {
            tokens: assert_error,
        },
        contract,
        error_path.span(),
    );
    items.push(error_item);
    edges.push(error_edge);

    let (impl_item, impl_edge) = assemble(
        2,
        LoweringRole::SubscriptionImpl,
        "subscription-impl",
        ItemNode::TraitImpl {
            trait_path,
            self_ty,
            body: impl_body,
        },
        contract,
        ident.span(),
    );
    items.push(impl_item);
    edges.push(impl_edge);

    Ok(LoweringPlan {
        contract: contract.clone(),
        model: EmittedItemModel {
            contract: contract.clone(),
            items,
        },
        edges,
    })
}

/// Assemble one emitted item plus its plan edge for a source-ordered sequence
/// position. Two independent [`OriginEdge`] values are built (rather than one
/// cloned) so this helper need not assume `OriginEdge: Clone`. `emit::finalize`
/// re-assigns the dense, source-order [`EmittedItemId`]; the `sequence` passed
/// here is the already-source-ordered provisional value it confirms.
fn assemble(
    sequence: u32,
    role: LoweringRole,
    sort_key: &str,
    node: ItemNode,
    contract: &ContractId,
    user_span: proc_macro2::Span,
) -> (EmittedItem, LoweringEdge) {
    let item = EmittedItem {
        id: EmittedItemId::new(sequence),
        role: role.clone(),
        sort_key: sort_key.to_owned(),
        node,
        origin: OriginEdge {
            emitted: EmittedItemId::new(sequence),
            contract: contract.clone(),
            lowering: role.clone(),
            declaration_field: None,
            user_span,
        },
    };
    let edge = LoweringEdge {
        role: role.clone(),
        target: CratePathRef::Batpak,
        origin: OriginEdge {
            emitted: EmittedItemId::new(sequence),
            contract: contract.clone(),
            lowering: role,
            declaration_field: None,
            user_span,
        },
    };
    (item, edge)
}

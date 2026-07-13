//! Cluster 12 — Operation contract lowering.
//!
//! Lowers [`OperationIr`] onto the uniform emitted-item spine, byte-behaviour
//! matching today's `#[operation]` expansion (packet 04 §E.6):
//!
//! 1. the re-emitted handler function (role `OperationFunction`);
//! 2. its `OperationDescriptor` declaration — a `const` when there is no effect
//!    row, else a `::std::sync::LazyLock` `static` (role `OperationDescriptor`);
//! 3. the handler signature pin `const _: fn(&[u8], &mut Ctx<'_>) -> HandlerResult`
//!    (role `OperationSignaturePin`);
//! 4. the optional `register_item` free fn (role `OperationRegisterItem`);
//! 5. the optional `register` free fn (role `OperationRegister`).
//!
//! Every family path (`OperationDescriptor`, `EffectClass`, `OperationEffectRow`,
//! `Ctx`, `HandlerResult`, `OperationRegisterItem`, `CoreBuilder`, `BuildError`)
//! is routed through the injected [`CratePath`] in `ctx.hygiene.crate_path`,
//! which the driver sets to `CratePath::runtime()` (`::syncbat` in v1) for
//! Operation contracts. Only that one literal (in `hygiene.rs`) ever names the
//! runtime family. `::std` / `::core` paths are emitted literally.

use quote::quote;

use crate::diagnostic::Diagnostics;
use crate::emit::item_model::{EmittedItem, EmittedItemModel, ItemNode};
use crate::hygiene::LoweringContext;
use crate::identity::{ContractId, EmittedItemId, LoweringRole};
use crate::ir::operation::{EffectClass, EffectRow, OperationIr};
use crate::lowering::{ContractLowering, CratePathRef, LoweringEdge, LoweringPlan};
use crate::origin::OriginEdge;

/// Lowering entry point for the Operation kind (the `plan` dispatch in
/// `lowering::mod` routes the `ContractKindIr::Operation` arm here).
pub struct OperationLowering;

impl ContractLowering for OperationLowering {
    type Ir = OperationIr;

    fn plan(ir: &OperationIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
        let contract = ctx.contract.clone();
        let fn_name = &ir.function.sig.ident;
        let descriptor = &ir.descriptor;
        let has_effect_row = !ir.effect_row.is_empty();

        // --- descriptor value expression + declaration item -----------------
        let descriptor_path = rt(ctx, "OperationDescriptor");
        let base_expr = descriptor_base_expr(ir, ctx, &descriptor_path);
        let descriptor_expr = if has_effect_row {
            let row = effect_row_expr(&ir.effect_row, ctx);
            quote! { #base_expr.with_effect_row(#row) }
        } else {
            base_expr
        };
        let descriptor_node = if has_effect_row {
            ItemNode::Static {
                ident: descriptor.clone(),
                tokens: quote! {
                    static #descriptor: ::std::sync::LazyLock<#descriptor_path> =
                        ::std::sync::LazyLock::new(|| #descriptor_expr);
                },
            }
        } else {
            ItemNode::Const {
                ident: descriptor.clone(),
                tokens: quote! {
                    const #descriptor: #descriptor_path = #descriptor_expr;
                },
            }
        };

        // A `LazyLock`-wrapped descriptor is behind a deref; a plain `const`
        // descriptor is a value — clone accordingly, matching today's output.
        let descriptor_clone_expr = if has_effect_row {
            quote! { ::std::clone::Clone::clone(&*#descriptor) }
        } else {
            quote! { #descriptor.clone() }
        };

        // --- signature pin --------------------------------------------------
        let ctx_ty = rt(ctx, "Ctx");
        let handler_result = rt(ctx, "HandlerResult");
        let signature_pin = quote! {
            const _: fn(&[u8], &mut #ctx_ty<'_>) -> #handler_result = #fn_name;
        };

        // --- optional register_item / register free fns ---------------------
        let register_item_node = ir.register_item.as_ref().map(|register_item| {
            let item_ty = rt(ctx, "OperationRegisterItem");
            ItemNode::FreeFn {
                ident: register_item.clone(),
                tokens: quote! {
                    pub fn #register_item() -> #item_ty {
                        #item_ty::new(#descriptor_clone_expr, #fn_name)
                    }
                },
            }
        });

        let item_expr = match &ir.register_item {
            Some(register_item) => quote! { #register_item() },
            None => {
                let item_ty = rt(ctx, "OperationRegisterItem");
                quote! { #item_ty::new(#descriptor_clone_expr, #fn_name) }
            }
        };

        let register_node = ir.register.as_ref().map(|register| {
            let builder_ty = rt(ctx, "CoreBuilder");
            let build_error = rt(ctx, "BuildError");
            ItemNode::FreeFn {
                ident: register.clone(),
                tokens: quote! {
                    pub fn #register(
                        builder: &mut #builder_ty,
                    ) -> ::std::result::Result<&mut #builder_ty, #build_error> {
                        builder.register_item(#item_expr)
                    }
                },
            }
        });

        // --- assemble the spine in source order -----------------------------
        let function = &ir.function;
        let mut items: Vec<EmittedItem> = Vec::new();
        let mut edges: Vec<LoweringEdge> = Vec::new();
        let mut next: u32 = 0;

        push_item(
            &mut items,
            &mut edges,
            &mut next,
            &contract,
            PlannedItem {
                role: LoweringRole::OperationFunction,
                target: CratePathRef::CoreStd,
                sort_key: fn_name.to_string(),
                node: ItemNode::FreeFn {
                    ident: fn_name.clone(),
                    tokens: quote! { #function },
                },
                user_span: fn_name.span(),
            },
        );
        push_item(
            &mut items,
            &mut edges,
            &mut next,
            &contract,
            PlannedItem {
                role: LoweringRole::OperationDescriptor,
                target: CratePathRef::Runtime,
                sort_key: descriptor.to_string(),
                node: descriptor_node,
                user_span: descriptor.span(),
            },
        );
        push_item(
            &mut items,
            &mut edges,
            &mut next,
            &contract,
            PlannedItem {
                role: LoweringRole::OperationSignaturePin,
                target: CratePathRef::Runtime,
                sort_key: fn_name.to_string(),
                node: ItemNode::AnonBlock {
                    tokens: signature_pin,
                },
                user_span: fn_name.span(),
            },
        );
        if let (Some(node), Some(register_item)) = (register_item_node, &ir.register_item) {
            push_item(
                &mut items,
                &mut edges,
                &mut next,
                &contract,
                PlannedItem {
                    role: LoweringRole::OperationRegisterItem,
                    target: CratePathRef::Runtime,
                    sort_key: register_item.to_string(),
                    node,
                    user_span: register_item.span(),
                },
            );
        }
        if let (Some(node), Some(register)) = (register_node, &ir.register) {
            push_item(
                &mut items,
                &mut edges,
                &mut next,
                &contract,
                PlannedItem {
                    role: LoweringRole::OperationRegister,
                    target: CratePathRef::Runtime,
                    sort_key: register.to_string(),
                    node,
                    user_span: register.span(),
                },
            );
        }

        Ok(LoweringPlan {
            contract: contract.clone(),
            model: EmittedItemModel { contract, items },
            edges,
        })
    }
}

/// Join `tail` onto the injected family crate path (`::syncbat` for Operation
/// in v1). The single family literal lives in `hygiene.rs::CratePath::runtime`.
fn rt(ctx: &LoweringContext<'_>, tail: &str) -> proc_macro2::TokenStream {
    ctx.hygiene.crate_path.join(tail)
}

/// `OperationDescriptor::new(...)` (or `new_with_title(...)` when a title is
/// present), byte-matching today's descriptor constructor selection.
fn descriptor_base_expr(
    ir: &OperationIr,
    ctx: &LoweringContext<'_>,
    descriptor_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let name = &ir.name;
    let input_schema = &ir.input_schema;
    let output_schema = &ir.output_schema;
    let receipt_kind = &ir.receipt_kind;
    let effect_class = rt(ctx, "EffectClass");
    let effect = effect_ident(ir.effect);
    match &ir.title {
        Some(title) => quote! {
            #descriptor_path::new_with_title(
                #name,
                #effect_class::#effect,
                #input_schema,
                #output_schema,
                #receipt_kind,
                #title,
            )
        },
        None => quote! {
            #descriptor_path::new(
                #name,
                #effect_class::#effect,
                #input_schema,
                #output_schema,
                #receipt_kind,
            )
        },
    }
}

/// `OperationEffectRow::new()` chained with one builder call per declared lane
/// entry, matching today's `build_effect_row_expr` method vocabulary.
fn effect_row_expr(row: &EffectRow, ctx: &LoweringContext<'_>) -> proc_macro2::TokenStream {
    let row_ty = rt(ctx, "OperationEffectRow");
    let mut expr = quote! { #row_ty::new() };
    for target in &row.reads_events {
        expr = quote! { #expr.reads_event(#target) };
    }
    for target in &row.appends_events {
        expr = quote! { #expr.appends_event(#target) };
    }
    for target in &row.queries_projections {
        expr = quote! { #expr.queries_projection(#target) };
    }
    for target in &row.emits_receipts {
        expr = quote! { #expr.emits_receipt(#target) };
    }
    for target in &row.uses_host_controls {
        expr = quote! { #expr.uses_host_control(#target) };
    }
    for target in &row.requires_capabilities {
        expr = quote! { #expr.requires_capability(#target) };
    }
    expr
}

/// The `EffectClass` path segment ident for a given effect (exhaustive; no
/// wildcard over the enum).
fn effect_ident(effect: EffectClass) -> proc_macro2::Ident {
    let name = match effect {
        EffectClass::Inspect => "Inspect",
        EffectClass::Compute => "Compute",
        EffectClass::Persist => "Persist",
        EffectClass::Emit => "Emit",
        EffectClass::Control => "Control",
    };
    proc_macro2::Ident::new(name, proc_macro2::Span::call_site())
}

/// One planned emitted item, bundled so [`push_item`] stays under the argument
/// budget while carrying everything an item + its lowering edge need.
struct PlannedItem {
    role: LoweringRole,
    target: CratePathRef,
    sort_key: String,
    node: ItemNode,
    user_span: proc_macro2::Span,
}

/// Append one emitted item + its lowering edge, assigning the next dense
/// source-order [`EmittedItemId`]. Two origin edges are built (one for the
/// item, one for the lowering edge) to avoid depending on `OriginEdge: Clone`.
fn push_item(
    items: &mut Vec<EmittedItem>,
    edges: &mut Vec<LoweringEdge>,
    next: &mut u32,
    contract: &ContractId,
    planned: PlannedItem,
) {
    let PlannedItem {
        role,
        target,
        sort_key,
        node,
        user_span,
    } = planned;
    let id = EmittedItemId::new(*next);
    *next += 1;
    items.push(EmittedItem {
        id,
        role: role.clone(),
        sort_key,
        node,
        origin: OriginEdge {
            emitted: id,
            contract: contract.clone(),
            lowering: role.clone(),
            declaration_field: None,
            user_span,
        },
    });
    edges.push(LoweringEdge {
        role: role.clone(),
        target,
        origin: OriginEdge {
            emitted: id,
            contract: contract.clone(),
            lowering: role,
            declaration_field: None,
            user_span,
        },
    });
}

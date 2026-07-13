//! Cluster 7 lowering — renders `#[derive(Error)]` to byte-identical
//! `Display` / `Error::source` / `From` tokens (§E.1, PreserveExact).
//!
//! Every emitted token matches `bpk-lib/crates/macros/src/error.rs` exactly:
//! the per-variant `Display` arm via [`FormatSpec::literal`], the `source()`
//! impl emitted only when a variant carries a source (Box → `.as_ref()`, else
//! the field directly; empty `impl … Error … {}` otherwise), and one
//! `impl From<Ty>` per `#[from]` field. The `tests/error_derive.rs` oracle
//! (moved in verbatim) must pass unchanged against this lowering.
//!
//! Only `::core` / `::std` paths are emitted — no [`CratePath`] — so
//! [`CratePathRef::CoreStd`] tags every edge.
//!
//! [`FormatSpec::literal`]: crate::ir::error::FormatSpec::literal
//! [`CratePath`]: crate::hygiene::CratePath

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

use crate::diagnostic::Diagnostics;
use crate::emit::item_model::{EmittedItem, EmittedItemModel, ItemNode};
use crate::hygiene::LoweringContext;
use crate::identity::{EmittedItemId, LoweringRole};
use crate::ir::error::{ErrorFields, ErrorIr, ErrorVariant, SourceMember};
use crate::lowering::{ContractLowering, CratePathRef, LoweringEdge, LoweringPlan};
use crate::origin::OriginEdge;

/// The `#[derive(Error)]` extension-seam lowering.
pub struct ErrorLowering;

impl ContractLowering for ErrorLowering {
    type Ir = ErrorIr;

    fn plan(ir: &Self::Ir, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
        lower(ir, ctx)
    }
}

/// One emitted item, before emit assigns its dense source-order id.
struct ItemSpec {
    role: LoweringRole,
    sort_key: String,
    span: Span,
    node: ItemNode,
}

/// Build the full `Display` + `Error` + `From` plan for the error enum.
///
/// Generics parity: the old derive splits the input generics once
/// (error.rs:55 / :249) and stamps `#impl_generics … for #ident #ty_generics
/// #where_clause` on every impl. The renderer prints
/// `impl #trait_path for #self_ty { #body }`, so the impl-generics ride at the
/// front of the `trait_path` slot and the type-generics + where-clause ride in
/// the `self_ty` slot — token-identical to the old output (and empty streams
/// for non-generic enums, the whole oracle corpus).
fn lower(ir: &ErrorIr, ctx: &LoweringContext<'_>) -> Result<LoweringPlan, Diagnostics> {
    let contract = ctx.contract;
    let enum_ident = &ir.ident;
    let (impl_generics, ty_generics, where_clause) = ir.generics.split_for_impl();
    let self_ty = quote!(#enum_ident #ty_generics #where_clause);

    let mut specs: Vec<ItemSpec> = Vec::new();

    // 1. `impl ::core::fmt::Display` — always emitted, one arm per variant.
    let display_arms = ir.variants.iter().map(display_arm).collect::<Vec<_>>();
    specs.push(ItemSpec {
        role: LoweringRole::DisplayImpl,
        sort_key: enum_ident.to_string(),
        span: enum_ident.span(),
        node: ItemNode::TraitImpl {
            trait_path: quote!(#impl_generics ::core::fmt::Display),
            self_ty: self_ty.clone(),
            body: quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        #(#display_arms)*
                    }
                }
            },
        },
    });

    // 2. `impl ::std::error::Error` — always emitted; a `source()` body iff any
    //    variant carries a source, otherwise an empty impl.
    let has_source = ir.variants.iter().any(|v| v.source.is_some());
    let error_body = if has_source {
        let source_arms = ir.variants.iter().map(source_arm).collect::<Vec<_>>();
        quote! {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    #(#source_arms)*
                }
            }
        }
    } else {
        TokenStream::new()
    };
    specs.push(ItemSpec {
        role: LoweringRole::ErrorSourceImpl,
        sort_key: enum_ident.to_string(),
        span: enum_ident.span(),
        node: ItemNode::TraitImpl {
            trait_path: quote!(#impl_generics ::std::error::Error),
            self_ty: self_ty.clone(),
            body: error_body,
        },
    });

    // 3. `impl ::core::convert::From<Ty>` per `#[from]` field, declaration order.
    let impl_generics_tokens = quote!(#impl_generics);
    for variant in &ir.variants {
        let Some(node) = from_impl_node(variant, &self_ty, &impl_generics_tokens) else {
            continue;
        };
        specs.push(ItemSpec {
            role: LoweringRole::FromImpl,
            sort_key: variant.ident.to_string(),
            span: variant.ident.span(),
            node,
        });
    }

    let mut items = Vec::with_capacity(specs.len());
    let mut edges = Vec::with_capacity(specs.len());
    let mut next: u32 = 0;
    for spec in specs {
        let id = EmittedItemId::new(next);
        next = next.saturating_add(1);
        let origin = OriginEdge {
            emitted: id,
            contract: contract.clone(),
            lowering: spec.role,
            declaration_field: None,
            user_span: spec.span,
        };
        edges.push(LoweringEdge {
            role: spec.role,
            target: CratePathRef::CoreStd,
            origin: origin.clone(),
        });
        items.push(EmittedItem {
            id,
            role: spec.role,
            sort_key: spec.sort_key,
            node: spec.node,
            origin,
        });
    }

    Ok(LoweringPlan {
        contract: contract.clone(),
        model: EmittedItemModel {
            contract: contract.clone(),
            items,
        },
        edges,
    })
}

/// The `write!`-bearing `Display` match arm for one variant.
fn display_arm(variant: &ErrorVariant) -> TokenStream {
    let vident = &variant.ident;
    let lit = variant.display.literal();
    match &variant.fields {
        ErrorFields::Named(fields) => {
            let pats = fields.iter().map(|field| {
                let ident = &field.ident;
                if variant.display.used_names.contains(&ident.to_string()) {
                    quote!(#ident)
                } else {
                    quote!(#ident: _)
                }
            });
            quote! {
                Self::#vident { #(#pats),* } => ::core::write!(f, #lit),
            }
        }
        ErrorFields::Unnamed(count) => {
            let pats = (0..*count).map(|i| {
                if variant.display.used_positions.contains(&i) {
                    let binding = format_ident!("__arg_{}", i);
                    quote!(#binding)
                } else {
                    quote!(_)
                }
            });
            quote! {
                Self::#vident( #(#pats),* ) => ::core::write!(f, #lit),
            }
        }
        ErrorFields::Unit => quote! {
            Self::#vident => ::core::write!(f, #lit),
        },
    }
}

/// The `source()` match arm for one variant.
fn source_arm(variant: &ErrorVariant) -> TokenStream {
    let vident = &variant.ident;
    match &variant.source {
        Some(source) => {
            let expr = if source.is_box {
                quote!(__source.as_ref())
            } else {
                quote!(__source)
            };
            match &source.member {
                SourceMember::Named(name) => quote! {
                    Self::#vident { #name: __source, .. } =>
                        ::core::option::Option::Some(#expr),
                },
                SourceMember::Indexed(index) => {
                    let count = match &variant.fields {
                        ErrorFields::Unnamed(count) => *count,
                        ErrorFields::Named(_) | ErrorFields::Unit => 0,
                    };
                    let pats = (0..count).map(|i| {
                        if i == *index {
                            quote!(__source)
                        } else {
                            quote!(_)
                        }
                    });
                    quote! {
                        Self::#vident( #(#pats),* ) =>
                            ::core::option::Option::Some(#expr),
                    }
                }
            }
        }
        None => match &variant.fields {
            ErrorFields::Named(_) => quote! {
                Self::#vident { .. } => ::core::option::Option::None,
            },
            ErrorFields::Unnamed(_) => quote! {
                Self::#vident( .. ) => ::core::option::Option::None,
            },
            ErrorFields::Unit => quote! {
                Self::#vident => ::core::option::Option::None,
            },
        },
    }
}

/// `impl From<SourceTy> for Enum`, emitted only for a `#[from]` field. The
/// enum's impl-generics ride at the front of the trait-path slot, matching the
/// old derive's `emit_from_impl` (error.rs:249) byte-for-byte.
fn from_impl_node(
    variant: &ErrorVariant,
    self_ty: &TokenStream,
    impl_generics: &TokenStream,
) -> Option<ItemNode> {
    let source = variant.source.as_ref()?;
    if !source.is_from {
        return None;
    }
    let vident = &variant.ident;
    let ty = &source.ty;
    let construct = match &source.member {
        SourceMember::Named(name) => quote!(Self::#vident { #name: __value }),
        SourceMember::Indexed(_) => quote!(Self::#vident(__value)),
    };
    Some(ItemNode::TraitImpl {
        trait_path: quote!(#impl_generics ::core::convert::From<#ty>),
        self_ty: self_ty.clone(),
        body: quote! {
            fn from(__value: #ty) -> Self {
                #construct
            }
        },
    })
}

//! Pass 6 spine — the uniform, kind-agnostic emitted-item model.
//!
//! Every per-kind lowering (Clusters 7-12) builds [`ItemNode`] values whose
//! bodies are already produced hygienically by `quote!` over deterministic
//! inputs. `normalize` and `tokenize` consume ONLY the types in this module and
//! never look at the originating contract kind — the spine is fully
//! kind-agnostic, so per-surface clusters never touch these files.
//!
//! Purity: `Vec` only, no `HashMap`, no `std::fs`/`std::env`/`std::time`. Token
//! assembly is a pure function of the structured header plus the pre-built body.

use crate::identity::{ContractId, EmittedItemId, LoweringRole};
use crate::origin::OriginEdge;
use proc_macro2::TokenStream;
use quote::quote;

/// The complete, kind-agnostic set of items one contract lowers to.
///
/// `items` are held in source (declaration) order; [`crate::emit::finalize`]
/// stamps each with a dense [`EmittedItemId`] matching that order, so iterating
/// `items` yields items in `EmittedItemId` order.
#[derive(Clone)]
pub struct EmittedItemModel {
    pub contract: ContractId,
    pub items: Vec<EmittedItem>,
}

/// One generated item plus the metadata `normalize`/`tokenize`/origin blame need.
///
/// `sort_key` is a lowering-provided, stable, ident-derived string; `normalize`
/// orders items by `(role as u8, sort_key)` so the canonical text is
/// byte-identical across runs for the same declaration and compiler version.
#[derive(Clone)]
pub struct EmittedItem {
    pub id: EmittedItemId,
    pub role: LoweringRole,
    pub sort_key: String,
    pub node: ItemNode,
    pub origin: OriginEdge,
}

/// A generated item: a structured header (for the two `impl` shapes, whose head
/// is assembled here) plus a deterministic, hygienically pre-built token body.
///
/// For the non-`impl` shapes the `tokens` field is the COMPLETE item token
/// stream produced by the lowering (e.g. the whole `fn`, `const`, or
/// `const _: () = { … };`); the retained `ident` is naming metadata for origin
/// blame and is not re-assembled into the output.
#[derive(Clone, Debug)]
pub enum ItemNode {
    TraitImpl {
        trait_path: TokenStream,
        self_ty: TokenStream,
        body: TokenStream,
    },
    InherentImpl {
        self_ty: TokenStream,
        body: TokenStream,
    },
    FreeFn {
        ident: syn::Ident,
        tokens: TokenStream,
    },
    Const {
        ident: syn::Ident,
        tokens: TokenStream,
    },
    Static {
        ident: syn::Ident,
        tokens: TokenStream,
    },
    TestFn {
        ident: syn::Ident,
        tokens: TokenStream,
    },
    AnonBlock {
        tokens: TokenStream,
    },
}

impl ItemNode {
    /// Assemble this node into its full item token stream.
    ///
    /// The single source of truth for node → tokens, shared by `tokenize`
    /// (final output) and `normalize` (canonical string). The two `impl` shapes
    /// are wrapped here; every other shape's `tokens` is already the complete
    /// item and is returned unchanged.
    #[must_use]
    pub fn to_item_tokens(&self) -> TokenStream {
        match self {
            ItemNode::TraitImpl {
                trait_path,
                self_ty,
                body,
            } => quote! { impl #trait_path for #self_ty { #body } },
            ItemNode::InherentImpl { self_ty, body } => quote! { impl #self_ty { #body } },
            ItemNode::FreeFn { ident: _, tokens }
            | ItemNode::Const { ident: _, tokens }
            | ItemNode::Static { ident: _, tokens }
            | ItemNode::TestFn { ident: _, tokens }
            | ItemNode::AnonBlock { tokens } => tokens.clone(),
        }
    }

    /// A deterministic estimate of the node's token count, used by
    /// [`crate::emit::finalize`] to enforce the `max_tokens` output budget.
    /// Counts every leaf token and every group delimiter recursively.
    #[must_use]
    pub fn estimated_token_count(&self) -> usize {
        count_token_trees(&self.to_item_tokens())
    }
}

/// Recursively count token trees: each leaf (ident/punct/literal) is one, each
/// group counts as one delimiter plus its contents. Deterministic and pure.
fn count_token_trees(stream: &TokenStream) -> usize {
    stream
        .clone()
        .into_iter()
        .map(|tree| match tree {
            proc_macro2::TokenTree::Group(group) => 1 + count_token_trees(&group.stream()),
            proc_macro2::TokenTree::Ident(_)
            | proc_macro2::TokenTree::Punct(_)
            | proc_macro2::TokenTree::Literal(_) => 1,
        })
        .sum()
}

//! Pass 1 — token intake.
//!
//! The first typed pass. It does exactly one thing: turn a raw
//! `proc_macro2::TokenStream` into a syntactically-parsed [`SyntaxTree`] carrying
//! the front-door [`ContractKind`], the parsed item, and the call-site span.
//! No grammar, no vocabulary, no shape law lives here — those are Pass 2/3. A
//! failure at this pass is a raw syntactic failure (the tokens are not a valid
//! `DeriveInput` / `ItemFn`), surfaced as a [`Diagnostics`] carrying the syn
//! error text under the kind's shape code.
//!
//! Purity: this file is a pure function of its inputs (no fs/env/time/net/rand).

use proc_macro2::{Span, TokenStream};
use syn::{DeriveInput, ItemFn};

use crate::diagnostic::{Diagnostics, MacroDiagnostic, RequiredAction};
use crate::identity::{ContractKind, DiagnosticCode, ShapeRule};

/// The Pass-1 product: a front-door kind + the syntactically-parsed item.
pub struct SyntaxTree {
    pub kind: ContractKind,
    pub item: SynItem,
    pub call_site: Span,
}

/// The parsed item, in the one of two front-door shapes the six kinds use.
///
/// `Derive` carries the whole `DeriveInput` (per-kind IR builders read its shape
/// downstream); `Attribute` carries the attribute-macro arguments token stream
/// plus the annotated free function.
pub enum SynItem {
    Derive(DeriveInput),
    Attribute { attr: TokenStream, item: ItemFn },
}

/// Parse a derive-macro input token stream into a [`SyntaxTree`].
///
/// # Errors
///
/// Returns [`Diagnostics`] when `item` does not parse as a `syn::DeriveInput`.
pub fn parse_derive(kind: ContractKind, item: TokenStream) -> Result<SyntaxTree, Diagnostics> {
    match syn::parse2::<DeriveInput>(item) {
        Ok(parsed) => Ok(SyntaxTree {
            kind,
            item: SynItem::Derive(parsed),
            call_site: Span::call_site(),
        }),
        Err(err) => Err(Diagnostics::one(parse_failure(kind, &err, false))),
    }
}

/// Parse an attribute-macro invocation (`attr` args + annotated `item`) into a
/// [`SyntaxTree`].
///
/// # Errors
///
/// Returns [`Diagnostics`] when `item` does not parse as a `syn::ItemFn`.
pub fn parse_attribute(
    kind: ContractKind,
    attr: TokenStream,
    item: TokenStream,
) -> Result<SyntaxTree, Diagnostics> {
    match syn::parse2::<ItemFn>(item) {
        Ok(function) => Ok(SyntaxTree {
            kind,
            item: SynItem::Attribute {
                attr,
                item: function,
            },
            call_site: Span::call_site(),
        }),
        Err(err) => Err(Diagnostics::one(parse_failure(kind, &err, true))),
    }
}

/// The declared item shape a kind targets — used to phrase a syntactic failure's
/// `ChangeShape` action, without needing the (not-yet-available) grammar here.
fn kind_shape(kind: ContractKind) -> ShapeRule {
    match kind {
        ContractKind::Event | ContractKind::Projection | ContractKind::Subscription => {
            ShapeRule::NamedFieldStruct
        }
        ContractKind::Error => ShapeRule::Enum,
        ContractKind::VariantInventory => ShapeRule::UnitOnlyEnum,
        ContractKind::Operation => ShapeRule::FreeFunction,
    }
}

/// The registry code for a Pass-1 syntactic failure (no dedicated syntactic
/// code exists in the registry; the kind's primary shape code is the closest
/// stable carrier for the syn error text).
fn parse_failure_code(kind: ContractKind, is_attribute: bool) -> DiagnosticCode {
    if is_attribute {
        return DiagnosticCode::OperationArityNotTwo;
    }
    match kind {
        ContractKind::Event | ContractKind::Projection | ContractKind::Subscription => {
            DiagnosticCode::NotNamedFieldStruct
        }
        ContractKind::Error => DiagnosticCode::ErrorNotEnum,
        ContractKind::VariantInventory => DiagnosticCode::VariantInventoryNotEnum,
        ContractKind::Operation => DiagnosticCode::OperationArityNotTwo,
    }
}

/// Build the Pass-1 syntactic-failure diagnostic from a `syn::Error`.
fn parse_failure(kind: ContractKind, err: &syn::Error, is_attribute: bool) -> MacroDiagnostic {
    let expected = if is_attribute {
        ShapeRule::FreeFunction
    } else {
        kind_shape(kind)
    };
    MacroDiagnostic::from_spec(
        parse_failure_code(kind, is_attribute),
        err.span(),
        RequiredAction::ChangeShape { expected },
        err.to_string(),
    )
}

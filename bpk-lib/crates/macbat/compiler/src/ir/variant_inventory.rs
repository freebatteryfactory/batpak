//! Pass 4 sub-IR for the `VariantInventory` contract kind.
//!
//! A `VariantInventory` contract targets a fieldless (unit-only) enum and lowers
//! to a generated `pub const ALL: [Self; N]` listing every variant in
//! declaration order (see `lowering::variant_inventory`). This module lifts the
//! ordered variant idents out of the validated declaration into a typed shape.
//!
//! The enum-only / every-variant-unit law is enforced upstream by the grammar's
//! `UnitOnlyEnum` shape rule in Pass 3 (`validation`). This builder re-reads the
//! same syntax to collect the idents, and re-surfaces the two shape diagnostics
//! defensively so a mis-dispatched declaration can never silently collapse into
//! an empty inventory.

use crate::diagnostic::{Diagnostics, MacroDiagnostic, RequiredAction};
use crate::identity::{DiagnosticCode, ShapeRule};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;
use syn::spanned::Spanned;
use syn::{Data, Fields};

/// The typed shape a `VariantInventory` contract lowers from: the target enum's
/// identity, its generics, and every variant in declaration (source) order.
pub struct VariantInventoryIr {
    /// The target enum's ident — the `Self` type the generated const attaches to.
    pub ident: syn::Ident,
    /// The target enum's generics, carried whole so the lowering can
    /// `split_for_impl` exactly as the legacy `AllVariants` derive does —
    /// generic unit-only enums are accepted today and must stay accepted.
    pub generics: syn::Generics,
    /// The enum's variants, in declaration order. Every one is a unit variant
    /// (guaranteed by the `UnitOnlyEnum` shape rule applied in Pass 3).
    pub variants: Vec<syn::Ident>,
}

/// Build the [`VariantInventoryIr`] from a validated declaration.
///
/// # Errors
///
/// Returns [`Diagnostics`] carrying `BP-SHAPE-0020` when the target is not an
/// enum, or `BP-SHAPE-0021` when a variant carries fields. Both conditions are
/// already rejected in Pass 3; the defensive re-check keeps the builder total
/// over the syntax it reads without a silent fallback.
pub fn build(decl: &ValidatedDeclaration) -> Result<VariantInventoryIr, Diagnostics> {
    let input = match &decl.item {
        SynItem::Derive(input) => input,
        SynItem::Attribute { item, .. } => return Err(not_an_enum(item.sig.ident.span())),
    };

    let data = match &input.data {
        Data::Enum(data) => data,
        Data::Struct(_) | Data::Union(_) => return Err(not_an_enum(input.ident.span())),
    };

    let mut variants = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        match &variant.fields {
            Fields::Unit => variants.push(variant.ident.clone()),
            Fields::Named(_) | Fields::Unnamed(_) => {
                return Err(non_unit_variant(variant.span()));
            }
        }
    }

    Ok(VariantInventoryIr {
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        variants,
    })
}

/// `BP-SHAPE-0020` (`variant-inventory-not-enum`): the target of a
/// `VariantInventory` contract is not an enum.
fn not_an_enum(primary: proc_macro2::Span) -> Diagnostics {
    Diagnostics::one(MacroDiagnostic::from_spec(
        DiagnosticCode::VariantInventoryNotEnum,
        primary,
        RequiredAction::ChangeShape {
            expected: ShapeRule::UnitOnlyEnum,
        },
        "a variant inventory can only be derived for an enum".to_owned(),
    ))
}

/// `BP-SHAPE-0021` (`variant-inventory-non-unit-variant`): a `VariantInventory`
/// enum carries a variant with fields.
fn non_unit_variant(primary: proc_macro2::Span) -> Diagnostics {
    Diagnostics::one(MacroDiagnostic::from_spec(
        DiagnosticCode::VariantInventoryNonUnitVariant,
        primary,
        RequiredAction::HandWrite {
            reason: "a data-carrying variant has no canonical zero-arg value to list in ALL",
        },
        "every variant of a variant inventory must be a unit variant".to_owned(),
    ))
}

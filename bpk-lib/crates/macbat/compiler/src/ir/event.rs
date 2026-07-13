//! Cluster 8 — Event sub-IR (Pass 4 projection for the Event surface).
//!
//! Reads the Pass-3-validated `#[batpak(category = N, type_id = N, version = N)]`
//! keys off a [`ValidatedDeclaration`] and packs them into the [`EventIr`] the
//! Event lowering consumes. The reserved-range / bit-width law for `category`
//! and `type_id` is enforced upstream in `validation` via `grammar::kind_rule`
//! (cluster 3's checked-in projection of the core `EventCategory` universe), and
//! `version == 0` is rejected there too; this pass only reads the
//! already-legal literals and computes `kind_bits`.
//!
//! `kind_bits = (category << 12) | type_id` is byte-identical to
//! `EventKind::as_raw_u16`; it is stored on the IR rather than recomputed in the
//! lowering so the emitted token stream interpolates one deterministic literal.

use proc_macro2::Span;

use crate::diagnostic::{Diagnostics, ExpectedForm, MacroDiagnostic, RequiredAction};
use crate::identity::{DiagnosticCode, KeyName, ShapeRule};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// Packed identity of a declared event payload.
///
/// Every numeric field is a narrow, in-range value: `category` fits 4 bits,
/// `type_id` fits 12 bits, and `payload_version` is a non-zero `u16` — all
/// guaranteed by Pass 3 before `build` runs. `ident` is the payload type's
/// original ident (true user span preserved for origin blame); events reject
/// generic parameters upstream, so the ident alone identifies the self-type.
pub struct EventIr {
    pub ident: syn::Ident,
    pub category: u8,
    pub type_id: u16,
    pub payload_version: u16,
    pub kind_bits: u16,
}

/// Build the Event sub-IR from a validated declaration.
///
/// # Errors
///
/// Returns a [`Diagnostics`] accumulator if the declaration is not a derive
/// item, a required key is absent, or a literal does not fit its narrow
/// integer type. All of these are already rejected in Passes 1–3, so a
/// well-formed pipeline never observes an `Err`; the fail-closed arms exist so
/// the width conversions stay total without a denied `.unwrap()`/`.expect()`
/// or a truncating `as` cast.
pub fn build(decl: &ValidatedDeclaration) -> Result<EventIr, Diagnostics> {
    // Event is a derive surface; an attribute-shaped item reaching this builder
    // is a pipeline routing defect, surfaced as a typed shape diagnostic
    // (fail-closed) rather than a denied panic. `let else` keeps the two-variant
    // enum handled without a catch-all match arm.
    let SynItem::Derive(input) = &decl.item else {
        return Err(Diagnostics::one(MacroDiagnostic::from_spec(
            DiagnosticCode::NotNamedFieldStruct,
            Span::call_site(),
            RequiredAction::ChangeShape {
                expected: ShapeRule::NamedFieldStruct,
            },
            "#[derive(Event)] requires a named-field struct item".to_owned(),
        )));
    };
    let ident = input.ident.clone();

    let category_lit = decl.keys.as_int("category").ok_or_else(|| {
        Diagnostics::one(MacroDiagnostic::from_spec(
            DiagnosticCode::EventMissingCategory,
            ident.span(),
            RequiredAction::AddKey {
                key: KeyName::new("category"),
                form: ExpectedForm::IntLit { bits: 8 },
            },
            "`#[batpak(...)]` requires `category = N`".to_owned(),
        ))
    })?;
    // Parse directly into the narrow `u8`: an in-range literal succeeds; an
    // out-of-range one surfaces as a typed diagnostic instead of truncating.
    let category: u8 = category_lit.base10_parse::<u8>().map_err(|err| {
        Diagnostics::one(MacroDiagnostic::from_spec(
            DiagnosticCode::EventCategoryExceedsU8,
            category_lit.span(),
            RequiredAction::ReplaceValue {
                key: KeyName::new("category"),
                form: ExpectedForm::IntLit { bits: 8 },
            },
            err.to_string(),
        ))
    })?;

    let type_id_lit = decl.keys.as_int("type_id").ok_or_else(|| {
        Diagnostics::one(MacroDiagnostic::from_spec(
            DiagnosticCode::EventMissingTypeId,
            ident.span(),
            RequiredAction::AddKey {
                key: KeyName::new("type_id"),
                form: ExpectedForm::IntLit { bits: 16 },
            },
            "`#[batpak(...)]` requires `type_id = N`".to_owned(),
        ))
    })?;
    let type_id: u16 = type_id_lit.base10_parse::<u16>().map_err(|err| {
        Diagnostics::one(MacroDiagnostic::from_spec(
            DiagnosticCode::EventTypeIdExceedsU16,
            type_id_lit.span(),
            RequiredAction::ReplaceValue {
                key: KeyName::new("type_id"),
                form: ExpectedForm::IntLit { bits: 16 },
            },
            err.to_string(),
        ))
    })?;

    // `version` defaults to 1 when absent, matching the `EventPayload`
    // associated-const default so existing derives keep emitting version 1. The
    // `version == 0` rejection (reserved legacy/untyped sentinel) lives in Pass 3.
    let payload_version: u16 = match decl.keys.as_int("version") {
        None => 1,
        Some(version_lit) => version_lit.base10_parse::<u16>().map_err(|err| {
            Diagnostics::one(MacroDiagnostic::from_spec(
                DiagnosticCode::EventVersionExceedsU16,
                version_lit.span(),
                RequiredAction::ReplaceValue {
                    key: KeyName::new("version"),
                    form: ExpectedForm::IntLit { bits: 16 },
                },
                err.to_string(),
            ))
        })?,
    };

    // Must agree byte-for-byte with `EventKind::as_raw_u16`; the packed formula
    // is inlined here because `EventKind` is unavailable at expansion time.
    let kind_bits: u16 = (u16::from(category) << 12) | type_id;

    Ok(EventIr {
        ident,
        category,
        type_id,
        payload_version,
        kind_bits,
    })
}

//! Pass 4 sub-IR for the `Subscription` contract kind (the legacy
//! `MultiEventReactor` parity surface).
//!
//! Grammar, arity, value-domain, and cross-attribute rules for a subscription
//! declaration are all decided upstream in Pass 2 (`declaration`) and Pass 3
//! (`validation`): `input`/`error` present exactly once, `cache_version` and
//! `state_max_cardinality` rejected here, at least one `event`/`handler`
//! binding, single-segment event paths, and no duplicate bindings. This pass is
//! a pure projection of the already-validated facts into the typed
//! [`SubscriptionIr`] the lowering consumes; it introduces no new user-facing
//! rule. The single guarded branch below is validation-guaranteed unreachable
//! and exists only to keep `build` total without `unwrap`/`panic`.

use crate::declaration::EventBindingRow;
use crate::diagnostic::{Diagnostics, MacroDiagnostic, RequiredAction};
use crate::identity::{DiagnosticCode, ShapeRule};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// Typed projection of a validated `#[derive(Subscription)]` (legacy
/// `#[derive(MultiEventReactor)]`) declaration.
///
/// Beyond the input/error/bindings trio, the sub-IR carries the target type
/// identity and its generic parameters (ratified pinned fields, matching the
/// other sub-IRs) so the lowering can reproduce the original
/// `impl … MultiReactive … for <Ident> …` header — including generic reactor
/// targets — behaviourally.
pub struct SubscriptionIr {
    pub input: syn::Path,
    pub error: syn::Path,
    pub bindings: Vec<EventBindingRow>,
    pub ident: syn::Ident,
    pub generics: syn::Generics,
}

/// Project a validated subscription declaration into its typed sub-IR.
///
/// # Errors
///
/// Returns a [`Diagnostics`] accumulator if the validated declaration is not a
/// derive target or is missing its (validation-guaranteed) `input`/`error`
/// config keys. That path is unreachable for any declaration that passed
/// Pass 3; the collapsed guard keeps the function total without
/// `unwrap`/`panic`, carried by the section-D shape code.
pub fn build(decl: &ValidatedDeclaration) -> Result<SubscriptionIr, Diagnostics> {
    let (SynItem::Derive(derive), Some(input), Some(error)) = (
        &decl.item,
        decl.keys.as_path("input"),
        decl.keys.as_path("error"),
    ) else {
        // Unreachable by construction: the `Subscription` kind only arrives
        // through the derive front door, and Pass 3 guarantees both required
        // config keys are present exactly once. One refutable let-else covers
        // all three facts with the one already-catalogued shape code.
        return Err(shape_violation());
    };

    Ok(SubscriptionIr {
        input: input.clone(),
        error: error.clone(),
        bindings: decl.bindings.clone(),
        ident: derive.ident.clone(),
        generics: derive.generics.clone(),
    })
}

/// The single (validation-guaranteed unreachable) failure of `build`: the
/// declaration reaching Pass 4 did not carry the validated subscription shape.
/// Uses the shared section-D shape code `BP-SHAPE-0001` with its catalogued
/// `ChangeShape` action; every payload is built from pinned identity types.
fn shape_violation() -> Diagnostics {
    Diagnostics::one(MacroDiagnostic::from_spec(
        DiagnosticCode::NotNamedFieldStruct,
        proc_macro2::Span::call_site(),
        RequiredAction::ChangeShape {
            expected: ShapeRule::NamedFieldStruct,
        },
        "subscription contracts derive on a named-field struct with `#[batpak(input = …)]` and `#[batpak(error = …)]`"
            .to_owned(),
    ))
}

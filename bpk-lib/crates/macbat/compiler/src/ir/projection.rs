//! Cluster 10 — Projection sub-IR (Pass 4 projection for the Projection surface).
//!
//! Reads the Pass-3-validated projection declaration off a
//! [`ValidatedDeclaration`] and packs it into the [`ProjectionIr`] the Projection
//! lowering consumes. The current `#[derive(EventSourced)]` shape is:
//!
//!   * `#[batpak(input = <Path>)]` — required config key (the `ProjectionInput`
//!     lane type);
//!   * `#[batpak(cache_version = N)]` — optional, defaults to `0` (the
//!     projection-cache invalidation key, `schema_version()`);
//!   * `#[batpak(state_max_cardinality = 1)]` — optional; when present it is
//!     always `1` (Pass 3 rejects any other value), and drives the
//!     `STATE_CONTRACT` + `state_extent` emission in the lowering;
//!   * `#[batpak(event = <Payload>, handler = <fn>)]` — one or more paired
//!     bindings, carried verbatim as [`EventBindingRow`]s.
//!
//! Closed-vocabulary enforcement, `state_max_cardinality == 1`, single-segment
//! `event` paths, duplicate rejection, and the `error`-key rejection all live
//! upstream in `declaration` / `validation`; this pass only reads already-legal
//! bindings. Per [G1 ruling 3] there is deliberately **no** decode-failure-policy
//! field — the matched-kind decode-failure behavior is fixed in the lowering, not
//! carried as IR data.

use proc_macro2::Span;

use crate::declaration::EventBindingRow;
use crate::diagnostic::{Diagnostics, ExpectedForm, MacroDiagnostic, RequiredAction};
use crate::identity::{DiagnosticCode, KeyName};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// The declared shape of an event-sourced projection.
///
/// `ident` and `generics` are the target type's own identifier and generic
/// parameters, carried directly off the declaration — `ContractId` is a *local*
/// id (symbolic owner) and must never be string-parsed to recover the ident. The
/// old `EventSourced` expansion supports generic projection targets, so the
/// generics ride the impl header verbatim (see the lowering).
///
/// `cache_version` is the parsed `schema_version()` value (0 when the key is
/// absent). `state_contract` is `Some` exactly when `state_max_cardinality` was
/// declared, and its `max_cardinality` is always `1` in v1 (Pass 3 rejects any
/// other value).
pub struct ProjectionIr {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub input: syn::Path,
    pub bindings: Vec<EventBindingRow>,
    pub cache_version: u64,
    pub state_contract: Option<StateContract>,
}

/// The single-aggregate state bound declared via `state_max_cardinality`.
///
/// `max_cardinality` is always `1` in v1 — the derive supports only
/// single-aggregate state, and Pass 3 rejects a declared bound `!= 1` (a
/// higher bound would be vacuous against the fixed `state_extent` of 1).
pub struct StateContract {
    pub max_cardinality: u64,
}

// Registry codes (Section D) reused for the defensive, Pass-3-guaranteed
// branches below. Presence of `input`, single-`1` `state_max_cardinality`, and
// `cache_version`'s `IntLit { bits: 64 }` width are all law by the time `build`
// runs, so these arms are unreachable in a well-formed pipeline — but the
// pipeline stays fail-closed (a typed `Err`), never reaching for a denied
// `.unwrap()`/`.expect()`, when `build` is exercised directly.
const MISSING_INPUT: DiagnosticCode = DiagnosticCode::ProjectionMissingInput;
const STATE_MAX_NOT_ONE: DiagnosticCode = DiagnosticCode::ProjectionStateMaxNotOne;

/// Build the Projection sub-IR from a validated declaration.
///
/// # Errors
///
/// Returns a [`Diagnostics`] accumulator if the required `input` key is absent
/// or the `state_max_cardinality` literal does not fit `u64`. Both conditions
/// are already rejected in Pass 3, so a well-formed pipeline never observes an
/// `Err`; the fail-closed arms exist so the reads stay total without a denied
/// `.unwrap()`/`.expect()`.
pub fn build(decl: &ValidatedDeclaration) -> Result<ProjectionIr, Diagnostics> {
    // A projection is always a derive; the `Attribute` arm (impossible here) is
    // handled totally rather than via a denied catch-all so the match is
    // exhaustive over `SynItem`'s two variants.
    let (ident, generics) = match &decl.item {
        SynItem::Derive(input) => (input.ident.clone(), input.generics.clone()),
        SynItem::Attribute { item, .. } => (item.sig.ident.clone(), item.sig.generics.clone()),
    };

    let input = decl
        .keys
        .as_path("input")
        .ok_or_else(|| {
            Diagnostics::one(MacroDiagnostic::from_spec(
                MISSING_INPUT,
                Span::call_site(),
                RequiredAction::AddKey {
                    key: KeyName::new("input"),
                    form: ExpectedForm::SinglePath,
                },
                "`#[batpak(...)]` requires `input = <Path>`".to_owned(),
            ))
        })?
        .clone();

    // `cache_version` defaults to 0 when absent (matching `schema_version()`'s
    // documented default). Its `IntLit { bits: 64 }` grammar means a literal
    // exceeding `u64` is rejected by value-shape validation before `build` runs,
    // so the parse is total here. Section D reserves no projection-specific
    // BP-VALUE code for the overflow, so the total path is asserted rather than
    // surfaced under a semantically-wrong code. [FLAG: no cache_version value code]
    let cache_version: u64 = match decl.keys.as_int("cache_version") {
        None => 0,
        Some(lit) => match lit.base10_parse::<u64>() {
            Ok(value) => value,
            Err(_err) => {
                unreachable!("cache_version validated to fit u64 before ir::build")
            }
        },
    };

    // `state_max_cardinality` is `Some` only when declared, and Pass 3 guarantees
    // the value is exactly `1`; the literal is read (not hard-coded) so the IR
    // carries the declared value, with a fail-closed arm on the total parse.
    let state_contract = match decl.keys.as_int("state_max_cardinality") {
        None => None,
        Some(lit) => {
            let max_cardinality = lit.base10_parse::<u64>().map_err(|err| {
                Diagnostics::one(MacroDiagnostic::from_spec(
                    STATE_MAX_NOT_ONE,
                    lit.span(),
                    RequiredAction::ReplaceValue {
                        key: KeyName::new("state_max_cardinality"),
                        form: ExpectedForm::IntLit { bits: 64 },
                    },
                    err.to_string(),
                ))
            })?;
            Some(StateContract { max_cardinality })
        }
    };

    let bindings = decl.bindings.clone();

    Ok(ProjectionIr {
        ident,
        generics,
        input,
        bindings,
        cache_version,
        state_contract,
    })
}

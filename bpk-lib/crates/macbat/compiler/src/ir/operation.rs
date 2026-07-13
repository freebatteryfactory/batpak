//! Cluster 12 — Operation contract IR.
//!
//! Builds [`OperationIr`] from a [`ValidatedDeclaration`] for `#[operation]`
//! free functions. Every grammar / shape / value-domain rejection ran in
//! Pass 2 ([`crate::declaration`]) and Pass 3 ([`crate::validation`]); those
//! passes are the single source of truth for Operation rejections. This pass
//! is a *total* mapping from the already-validated declaration into the typed
//! IR — it never re-diagnoses (SSOT). The type-visible states that Pass 3 has
//! already excluded (absent required key, wrong `KeyValue` shape, an `effect`
//! ident outside the closed vocabulary, a non-attribute [`SynItem`]) are
//! genuinely unrepresentable here and are proven so with `unreachable!()`
//! (lint-clean under the workspace lint table, and the doctrine-sanctioned
//! marker for an impossible branch — never a `panic!`/`.unwrap()`).

use crate::declaration::{KeyBindings, KeyValue};
use crate::diagnostic::Diagnostics;
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// Closed effect vocabulary for an operation. Parsed (as an ident) in Pass 2/3;
/// mapped to this enum here. Adding a variant is a deliberate vocabulary change.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EffectClass {
    Inspect,
    Compute,
    Persist,
    Emit,
    Control,
}

/// The six declared effect-row lanes. Empty across all lanes => no effect row
/// (the descriptor is emitted as a `const`; otherwise a `LazyLock` static).
#[derive(Clone)]
pub struct EffectRow {
    pub reads_events: Vec<syn::Lit>,
    pub appends_events: Vec<syn::Lit>,
    pub queries_projections: Vec<syn::Lit>,
    pub emits_receipts: Vec<syn::Lit>,
    pub uses_host_controls: Vec<syn::Lit>,
    pub requires_capabilities: Vec<syn::Lit>,
}

impl EffectRow {
    pub fn is_empty(&self) -> bool {
        self.reads_events.is_empty()
            && self.appends_events.is_empty()
            && self.queries_projections.is_empty()
            && self.emits_receipts.is_empty()
            && self.uses_host_controls.is_empty()
            && self.requires_capabilities.is_empty()
    }
}

/// Typed IR for a single `#[operation]` contract.
#[derive(Clone)]
pub struct OperationIr {
    pub function: syn::ItemFn,
    pub descriptor: syn::Ident,
    pub name: syn::Lit,
    pub effect: EffectClass,
    pub input_schema: syn::Lit,
    pub output_schema: syn::Lit,
    pub receipt_kind: syn::Lit,
    pub title: Option<syn::Lit>,
    pub effect_row: EffectRow,
    pub register: Option<syn::Ident>,
    pub register_item: Option<syn::Ident>,
}

/// Lower a validated Operation declaration into [`OperationIr`].
///
/// Total for Operation: every failure mode is decided upstream in Pass 2/3, so
/// this returns `Ok` on every reachable input. The `Result<_, Diagnostics>`
/// return matches the uniform per-kind `ir::<kind>::build` seam (other kinds do
/// perform IR-level checks that can fail).
pub fn build(decl: &ValidatedDeclaration) -> Result<OperationIr, Diagnostics> {
    let keys = &decl.keys;
    Ok(OperationIr {
        function: operation_function(decl),
        descriptor: required_ident(keys, "descriptor"),
        name: required_str(keys, "name"),
        effect: required_effect(keys),
        input_schema: required_str(keys, "input_schema"),
        output_schema: required_str(keys, "output_schema"),
        receipt_kind: required_str(keys, "receipt_kind"),
        title: optional_str(keys, "title"),
        effect_row: EffectRow {
            reads_events: str_list(keys, "reads_events"),
            appends_events: str_list(keys, "appends_events"),
            queries_projections: str_list(keys, "queries_projections"),
            emits_receipts: str_list(keys, "emits_receipts"),
            uses_host_controls: str_list(keys, "uses_host_controls"),
            requires_capabilities: str_list(keys, "requires_capabilities"),
        },
        register: optional_ident(keys, "register"),
        register_item: optional_ident(keys, "register_item"),
    })
}

/// The handler function is always parsed via `tokens::parse_attribute` into
/// `SynItem::Attribute` for an attribute-macro kind; the `Derive` shape is
/// unrepresentable for Operation.
fn operation_function(decl: &ValidatedDeclaration) -> syn::ItemFn {
    let SynItem::Attribute { item, .. } = &decl.item else {
        unreachable!("operation contracts are parsed via parse_attribute into SynItem::Attribute")
    };
    item.clone()
}

/// Borrow an ident-shaped key value, or `None` when the key is absent or (per
/// grammar, impossible here) carries a non-ident shape.
fn key_ident<'a>(keys: &'a KeyBindings, name: &str) -> Option<&'a syn::Ident> {
    match keys.get(name).map(|entry| &entry.value) {
        Some(KeyValue::Ident(ident)) => Some(ident),
        Some(KeyValue::Path(_) | KeyValue::Int(_) | KeyValue::Str(_) | KeyValue::StrList(_)) => None,
        None => None,
    }
}

fn required_ident(keys: &KeyBindings, name: &str) -> syn::Ident {
    match key_ident(keys, name) {
        Some(ident) => ident.clone(),
        None => unreachable!("Pass 3 guarantees required ident key `{name}` is present"),
    }
}

fn optional_ident(keys: &KeyBindings, name: &str) -> Option<syn::Ident> {
    key_ident(keys, name).cloned()
}

fn required_str(keys: &KeyBindings, name: &str) -> syn::Lit {
    match keys.as_str(name) {
        Some(lit) => syn::Lit::Str(lit.clone()),
        None => unreachable!("Pass 3 guarantees required string key `{name}` is present"),
    }
}

fn optional_str(keys: &KeyBindings, name: &str) -> Option<syn::Lit> {
    keys.as_str(name).map(|lit| syn::Lit::Str(lit.clone()))
}

fn str_list(keys: &KeyBindings, name: &str) -> Vec<syn::Lit> {
    match keys.as_str_list(name) {
        Some(lits) => lits
            .iter()
            .map(|lit| syn::Lit::Str(lit.clone()))
            .collect(),
        None => Vec::new(),
    }
}

fn required_effect(keys: &KeyBindings) -> EffectClass {
    let ident = match key_ident(keys, "effect") {
        Some(ident) => ident,
        None => unreachable!("Pass 3 guarantees the `effect` key is a present ident"),
    };
    // Match over `&str` (not the enum): a `_` arm here is not
    // `wildcard_enum_match_arm`; Pass 3 restricts the value to the five idents.
    match ident.to_string().as_str() {
        "Inspect" => EffectClass::Inspect,
        "Compute" => EffectClass::Compute,
        "Persist" => EffectClass::Persist,
        "Emit" => EffectClass::Emit,
        "Control" => EffectClass::Control,
        _ => unreachable!("Pass 3 restricts `effect` to the closed EffectClass vocabulary"),
    }
}

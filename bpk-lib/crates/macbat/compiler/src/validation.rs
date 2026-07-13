//! Pass 3 — validation.
//!
//! A typestate gate: it takes a grammar-legal [`ParsedDeclaration`] and, if
//! every value-domain and cross-field rule holds, hands back a structurally
//! identical but distinctly-typed [`ValidatedDeclaration`]. Nothing here parses
//! new syntax; it only judges the already-sourced keys and bindings.
//!
//! It applies: required-key presence, arity (across-attribute duplicates,
//! minimum binding count), incompatibility sets (recognized-but-rejected keys),
//! value-domain rules (reserved category/type_id via `kind_rule`, version = 0,
//! `state_max_cardinality` = 1, single-segment event paths, duplicate bindings,
//! the closed effect set), and INPUT-shaped budgets (max fields / variants /
//! composition depth). Output-shaped budgets (generated items, tokens) are the
//! emit pass, not here.
//!
//! Purity: pure function of its inputs (no fs/env/time/net/rand; deterministic
//! collections only).

use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Data, Fields, GenericArgument, LitInt, Path, PathArguments, Type};

use crate::budget::{BudgetKind, BudgetViolation, ExpansionBudgets};
use crate::declaration::{EventBindingRow, KeyBindings, KeyValue, ParsedDeclaration, SpanTable};
use crate::diagnostic::{Diagnostics, ExpectedForm, MacroDiagnostic, RequiredAction};
use crate::grammar::kind_rule;
use crate::grammar::{expected_form, Arity, AttributeGrammar, KeySpec};
use crate::identity::{ContractKind, DiagnosticCode, KeyName};
use crate::tokens::SynItem;

/// The Pass-3 product: a validated declaration. Structurally equal to
/// [`ParsedDeclaration`], but a distinct type so only validated declarations
/// can be lowered.
pub struct ValidatedDeclaration {
    pub kind: ContractKind,
    pub item: SynItem,
    pub keys: KeyBindings,
    pub bindings: Vec<EventBindingRow>,
    pub spans: SpanTable,
}

/// Validate a parsed declaration against its grammar and expansion budgets.
///
/// # Errors
///
/// Returns [`Diagnostics`] on any presence, arity, incompatibility,
/// value-domain, or input-budget violation.
pub fn validate(
    decl: ParsedDeclaration,
    grammar: &AttributeGrammar,
    budgets: &ExpansionBudgets,
) -> Result<ValidatedDeclaration, Diagnostics> {
    let mut acc: Option<Diagnostics> = None;

    let kind = decl.kind;
    let call_site = decl.spans.call_site();

    match kind {
        ContractKind::Event => validate_event(&decl, grammar, &mut acc),
        ContractKind::Projection => validate_projection(&decl, grammar, call_site, &mut acc),
        ContractKind::Subscription => validate_subscription(&decl, grammar, call_site, &mut acc),
        ContractKind::Operation => validate_operation(&decl, grammar, call_site, &mut acc),
        ContractKind::Error | ContractKind::VariantInventory => {}
    }

    apply_input_budgets(&decl.item, budgets, call_site, &mut acc);

    match acc {
        Some(diags) => Err(diags),
        None => Ok(ValidatedDeclaration {
            kind: decl.kind,
            item: decl.item,
            keys: decl.keys,
            bindings: decl.bindings,
            spans: decl.spans,
        }),
    }
}

fn record(acc: &mut Option<Diagnostics>, diagnostic: MacroDiagnostic) {
    match acc.take() {
        Some(mut existing) => {
            existing.push(diagnostic);
            *acc = Some(existing);
        }
        None => *acc = Some(Diagnostics::one(diagnostic)),
    }
}

// ─── Event value-domain ──────────────────────────────────────────────────────

fn validate_event(decl: &ParsedDeclaration, grammar: &AttributeGrammar, acc: &mut Option<Diagnostics>) {
    let call_site = decl.spans.call_site();

    require_key(
        &decl.keys,
        grammar,
        "category",
        DiagnosticCode::EventMissingCategory,
        call_site,
        acc,
    );
    require_key(
        &decl.keys,
        grammar,
        "type_id",
        DiagnosticCode::EventMissingTypeId,
        call_site,
        acc,
    );

    if let Some(lit) = decl.keys.as_int("category") {
        check_category(lit, key_name(grammar, "category"), form_for(grammar, "category"), acc);
    }
    if let Some(lit) = decl.keys.as_int("type_id") {
        check_type_id(lit, key_name(grammar, "type_id"), form_for(grammar, "type_id"), acc);
    }
    if let Some(lit) = decl.keys.as_int("version") {
        check_version(lit, key_name(grammar, "version"), form_for(grammar, "version"), acc);
    }
}

fn check_category(lit: &LitInt, key: KeyName, form: ExpectedForm, acc: &mut Option<Diagnostics>) {
    let value = parse_u64_or_overflow(lit, u64::from(u8::MAX) + 1);
    if value > u64::from(u8::MAX) {
        record(
            acc,
            replace(
                DiagnosticCode::EventCategoryExceedsU8,
                lit.span(),
                key,
                form,
                "category exceeds the u8 range".to_string(),
            ),
        );
        return;
    }
    let category = match u8::try_from(value) {
        Ok(category) => category,
        Err(_) => {
            record(
                acc,
                replace(
                    DiagnosticCode::EventCategoryExceedsU8,
                    lit.span(),
                    key,
                    form,
                    "category exceeds the u8 range".to_string(),
                ),
            );
            return;
        }
    };
    if kind_rule::validate_category(category).is_err() {
        let code = if category == 0x0 {
            DiagnosticCode::EventCategoryReserved0x0
        } else if category == 0xD {
            DiagnosticCode::EventCategoryReserved0xD
        } else {
            DiagnosticCode::EventCategoryExceedsU8
        };
        record(
            acc,
            replace(
                code,
                lit.span(),
                key,
                form,
                "category is outside the legal 4-bit custom range".to_string(),
            ),
        );
    }
}

fn check_type_id(lit: &LitInt, key: KeyName, form: ExpectedForm, acc: &mut Option<Diagnostics>) {
    let value = parse_u64_or_overflow(lit, u64::from(u16::MAX) + 1);
    if value > u64::from(u16::MAX) {
        record(
            acc,
            replace(
                DiagnosticCode::EventTypeIdExceedsU16,
                lit.span(),
                key,
                form,
                "type_id exceeds the u16 range".to_string(),
            ),
        );
        return;
    }
    let type_id = match u16::try_from(value) {
        Ok(type_id) => type_id,
        Err(_) => {
            record(
                acc,
                replace(
                    DiagnosticCode::EventTypeIdExceedsU16,
                    lit.span(),
                    key,
                    form,
                    "type_id exceeds the u16 range".to_string(),
                ),
            );
            return;
        }
    };
    if kind_rule::validate_type_id(type_id).is_err() {
        record(
            acc,
            replace(
                DiagnosticCode::EventTypeIdExceeds12Bit,
                lit.span(),
                key,
                form,
                "type_id exceeds the 12-bit range (0x000..=0x0FFF)".to_string(),
            ),
        );
    }
}

fn check_version(lit: &LitInt, key: KeyName, form: ExpectedForm, acc: &mut Option<Diagnostics>) {
    let value = parse_u64_or_overflow(lit, u64::from(u16::MAX) + 1);
    if value == 0 {
        record(
            acc,
            replace(
                DiagnosticCode::EventVersionZeroReserved,
                lit.span(),
                key,
                form,
                "version 0 is reserved for legacy/untyped frames".to_string(),
            ),
        );
        return;
    }
    if value > u64::from(u16::MAX) {
        record(
            acc,
            replace(
                DiagnosticCode::EventVersionExceedsU16,
                lit.span(),
                key,
                form,
                "version exceeds the u16 range".to_string(),
            ),
        );
    }
}

// ─── Projection value-domain ─────────────────────────────────────────────────

fn validate_projection(
    decl: &ParsedDeclaration,
    grammar: &AttributeGrammar,
    call_site: Span,
    acc: &mut Option<Diagnostics>,
) {
    require_key(
        &decl.keys,
        grammar,
        "input",
        DiagnosticCode::ProjectionMissingInput,
        call_site,
        acc,
    );

    // `error` is grammar-recognized on projections but rejected: projections
    // carry no associated error type.
    if let Some(entry) = decl.keys.get("error") {
        record(
            acc,
            remove(
                DiagnosticCode::ProjectionErrorKeyRejected,
                entry.span,
                entry.name.clone(),
                "projections have no associated error type".to_string(),
            ),
        );
    }

    if let Some(lit) = decl.keys.as_int("state_max_cardinality") {
        if parse_u64_or_overflow(lit, 2) != 1 {
            record(
                acc,
                replace(
                    DiagnosticCode::ProjectionStateMaxNotOne,
                    lit.span(),
                    key_name(grammar, "state_max_cardinality"),
                    form_for(grammar, "state_max_cardinality"),
                    "state_max_cardinality supports only single-aggregate state (= 1)".to_string(),
                ),
            );
        }
    }

    check_across_attr_duplicates(&decl.keys, grammar, acc);

    if decl.bindings.is_empty() {
        record(
            acc,
            no_bindings(
                DiagnosticCode::ProjectionNoBindings,
                grammar,
                call_site,
            ),
        );
    }
    validate_bindings(grammar, &decl.bindings, acc);
}

// ─── Subscription value-domain ───────────────────────────────────────────────

fn validate_subscription(
    decl: &ParsedDeclaration,
    grammar: &AttributeGrammar,
    call_site: Span,
    acc: &mut Option<Diagnostics>,
) {
    require_key(
        &decl.keys,
        grammar,
        "input",
        DiagnosticCode::SubscriptionMissingInput,
        call_site,
        acc,
    );
    require_key(
        &decl.keys,
        grammar,
        "error",
        DiagnosticCode::SubscriptionMissingError,
        call_site,
        acc,
    );

    if let Some(entry) = decl.keys.get("cache_version") {
        record(
            acc,
            remove(
                DiagnosticCode::SubscriptionCacheVersionRejected,
                entry.span,
                entry.name.clone(),
                "cache_version is a projection key, not a subscription key".to_string(),
            ),
        );
    }
    if let Some(entry) = decl.keys.get("state_max_cardinality") {
        record(
            acc,
            remove(
                DiagnosticCode::SubscriptionStateMaxRejected,
                entry.span,
                entry.name.clone(),
                "state cardinality is a projection contract, not a subscription key".to_string(),
            ),
        );
    }

    check_across_attr_duplicates(&decl.keys, grammar, acc);

    if decl.bindings.is_empty() {
        record(
            acc,
            no_bindings(
                DiagnosticCode::SubscriptionNoBindings,
                grammar,
                call_site,
            ),
        );
    }
    validate_bindings(grammar, &decl.bindings, acc);
}

// ─── Operation value-domain ──────────────────────────────────────────────────

fn validate_operation(
    decl: &ParsedDeclaration,
    grammar: &AttributeGrammar,
    call_site: Span,
    acc: &mut Option<Diagnostics>,
) {
    for spec in grammar.keys {
        if spec.required && !decl.keys.contains(spec.name.as_str()) {
            record(
                acc,
                MacroDiagnostic::from_spec(
                    DiagnosticCode::OperationMissingRequiredKey,
                    call_site,
                    RequiredAction::AddKey {
                        key: spec.name.clone(),
                        form: expected_form(&spec.value),
                    },
                    format!("operation is missing required key `{}`", spec.name.as_str()),
                ),
            );
        }
    }

    if let Some(entry) = decl.keys.get("effect") {
        let legal = match &entry.value {
            KeyValue::Ident(ident) => matches!(
                ident.to_string().as_str(),
                "Inspect" | "Compute" | "Persist" | "Emit" | "Control"
            ),
            KeyValue::Path(_) | KeyValue::Int(_) | KeyValue::Str(_) | KeyValue::StrList(_) => false,
        };
        if !legal {
            record(
                acc,
                replace(
                    DiagnosticCode::OperationUnsupportedEffect,
                    entry.span,
                    entry.name.clone(),
                    ExpectedForm::EffectClass,
                    "effect must be one of Inspect | Compute | Persist | Emit | Control".to_string(),
                ),
            );
        }
    }
}

// ─── Shared binding checks (projection + subscription) ───────────────────────

fn validate_bindings(
    grammar: &AttributeGrammar,
    bindings: &[EventBindingRow],
    acc: &mut Option<Diagnostics>,
) {
    let mut seen: Vec<String> = Vec::new();
    for binding in bindings {
        if binding.event.leading_colon.is_some() || binding.event.segments.len() != 1 {
            record(
                acc,
                replace(
                    DiagnosticCode::EventPathNotSingleSegment,
                    binding.event.span(),
                    binding_event_key(grammar),
                    ExpectedForm::SinglePath,
                    "event must be a single-segment path (bring it into scope with `use`)"
                        .to_string(),
                ),
            );
        }
        let signature = path_signature(&binding.event);
        if seen.iter().any(|s| s == &signature) {
            record(
                acc,
                remove(
                    DiagnosticCode::DuplicateEventBinding,
                    binding.event.span(),
                    binding_event_key(grammar),
                    "each event payload may be bound to exactly one handler".to_string(),
                ),
            );
        } else {
            seen.push(signature);
        }
    }
}

fn check_across_attr_duplicates(
    keys: &KeyBindings,
    grammar: &AttributeGrammar,
    acc: &mut Option<Diagnostics>,
) {
    for spec in grammar.keys {
        let bounded = matches!(spec.arity, Arity::ExactlyOnce | Arity::AtMostOnce);
        if bounded && keys.count(spec.name.as_str()) > 1 {
            let span = keys
                .get(spec.name.as_str())
                .map_or_else(Span::call_site, |entry| entry.span);
            record(
                acc,
                remove(
                    DiagnosticCode::DuplicateConfigKeyAcrossAttrs,
                    span,
                    spec.name.clone(),
                    format!("`{}` may appear at most once", spec.name.as_str()),
                ),
            );
        }
    }
}

// ─── Input-shaped budgets ────────────────────────────────────────────────────

fn apply_input_budgets(
    item: &SynItem,
    budgets: &ExpansionBudgets,
    call_site: Span,
    acc: &mut Option<Diagnostics>,
) {
    let derive = match item {
        SynItem::Derive(derive) => derive,
        SynItem::Attribute { .. } => return,
    };

    let mut types: Vec<&Type> = Vec::new();
    match &derive.data {
        Data::Struct(data) => {
            let field_count = data.fields.len();
            check_budget(
                BudgetKind::Fields,
                to_u64(field_count),
                u64::from(budgets.max_fields),
                call_site,
                acc,
            );
            for field in &data.fields {
                types.push(&field.ty);
            }
        }
        Data::Enum(data) => {
            check_budget(
                BudgetKind::Variants,
                to_u64(data.variants.len()),
                u64::from(budgets.max_variants),
                call_site,
                acc,
            );
            let mut widest = 0usize;
            for variant in &data.variants {
                let count = match &variant.fields {
                    Fields::Named(named) => named.named.len(),
                    Fields::Unnamed(unnamed) => unnamed.unnamed.len(),
                    Fields::Unit => 0,
                };
                widest = widest.max(count);
                for field in &variant.fields {
                    types.push(&field.ty);
                }
            }
            check_budget(
                BudgetKind::Fields,
                to_u64(widest),
                u64::from(budgets.max_fields),
                call_site,
                acc,
            );
        }
        Data::Union(_) => {}
    }

    let mut deepest = 0u32;
    for ty in types {
        deepest = deepest.max(type_depth(ty));
    }
    check_budget(
        BudgetKind::CompositionDepth,
        u64::from(deepest),
        u64::from(budgets.max_composition_depth),
        call_site,
        acc,
    );
}

fn check_budget(
    budget: BudgetKind,
    observed: u64,
    limit: u64,
    at: Span,
    acc: &mut Option<Diagnostics>,
) {
    if observed > limit {
        record(
            acc,
            BudgetViolation {
                budget,
                limit,
                observed,
                at,
            }
            .into_diagnostic(),
        );
    }
}

/// Composition depth of a type: 1 for a leaf, `1 + max(child depths)` through
/// angle-bracketed generic arguments (e.g. `Vec<Box<Option<T>>>` = 4).
fn type_depth(ty: &Type) -> u32 {
    let Type::Path(type_path) = ty else {
        return 1;
    };
    let mut deepest_arg = 0u32;
    for segment in &type_path.path.segments {
        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            for arg in &args.args {
                if let GenericArgument::Type(inner) = arg {
                    deepest_arg = deepest_arg.max(type_depth(inner));
                }
            }
        }
    }
    1 + deepest_arg
}

// ─── Small shared helpers ────────────────────────────────────────────────────

/// Parse a `LitInt` to `u64`; on a parse failure (value wider than `u64`),
/// return `overflow_sentinel` so the caller's range guard rejects it — never a
/// silent accept.
fn parse_u64_or_overflow(lit: &LitInt, overflow_sentinel: u64) -> u64 {
    match lit.base10_parse::<u64>() {
        Ok(value) => value,
        Err(_) => overflow_sentinel,
    }
}

fn to_u64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

fn path_signature(path: &Path) -> String {
    path.to_token_stream().to_string()
}

fn require_key(
    keys: &KeyBindings,
    grammar: &AttributeGrammar,
    name: &str,
    code: DiagnosticCode,
    call_site: Span,
    acc: &mut Option<Diagnostics>,
) {
    if keys.contains(name) {
        return;
    }
    record(
        acc,
        MacroDiagnostic::from_spec(
            code,
            call_site,
            RequiredAction::AddKey {
                key: key_name(grammar, name),
                form: form_for(grammar, name),
            },
            format!("missing required key `{name}`"),
        ),
    );
}

fn no_bindings(code: DiagnosticCode, grammar: &AttributeGrammar, call_site: Span) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(
        code,
        call_site,
        RequiredAction::AddKey {
            key: binding_event_key(grammar),
            form: ExpectedForm::SinglePath,
        },
        "at least one `event = <Payload>, handler = <fn>` binding is required".to_string(),
    )
}

fn key_name(grammar: &AttributeGrammar, name: &str) -> KeyName {
    spec_of(grammar, name).map_or_else(
        || KeyName::new("input"),
        |spec| spec.name.clone(),
    )
}

fn form_for(grammar: &AttributeGrammar, name: &str) -> ExpectedForm {
    spec_of(grammar, name).map_or(ExpectedForm::Ident, |spec| expected_form(&spec.value))
}

fn spec_of<'g>(grammar: &'g AttributeGrammar, name: &str) -> Option<&'g KeySpec> {
    grammar.keys.iter().find(|spec| spec.name.as_str() == name)
}

fn binding_event_key(grammar: &AttributeGrammar) -> KeyName {
    grammar
        .binding_keys
        .as_ref()
        .map_or_else(|| KeyName::new("event"), |binding| binding.event.clone())
}

fn replace(
    code: DiagnosticCode,
    span: Span,
    key: KeyName,
    form: ExpectedForm,
    message: String,
) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(code, span, RequiredAction::ReplaceValue { key, form }, message)
}

fn remove(code: DiagnosticCode, span: Span, key: KeyName, message: String) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(code, span, RequiredAction::RemoveKey { key }, message)
}

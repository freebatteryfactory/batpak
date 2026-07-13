//! Pass 2 — declaration.
//!
//! A generic interpreter over the [`AttributeGrammar`] catalog. It sources the
//! declared keys and event-binding rows from the parsed item, enforces the
//! declared item shape, and applies the *closed-vocabulary* law: every key is
//! matched against the grammar; an unknown key is rejected with ranked
//! alternatives; nothing is silently ignored. It also classifies each
//! `#[batpak(...)]` attribute as config-bearing or binding-bearing, rejects
//! mixing the two in one attribute, and rejects in-attribute duplicate keys.
//!
//! Value-domain rules (numeric ranges, reserved sentinels, single-segment
//! paths, required-key presence, arity, budgets) are NOT here — they are Pass 3
//! ([`crate::validation`]). This pass answers "is every token a grammar-legal
//! key/binding of a legally-shaped item?", not "is every value in range?".
//!
//! Purity: pure function of its inputs (no fs/env/time/net/rand; deterministic
//! `Vec` collections only).

use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Data, Expr, ExprLit, Fields, FnArg, Ident, Lit, LitInt, LitStr, MetaNameValue, Path, Token};

use crate::diagnostic::{Diagnostics, ExpectedForm, MacroDiagnostic, RequiredAction, Suggestion};
use crate::grammar::{expected_form, rank_alternatives, AttributeGrammar, KeySpec, ValueShape};
use crate::identity::{ContractKind, DiagnosticCode, KeyName, ShapeRule};
use crate::tokens::{SynItem, SyntaxTree};

/// How many ranked alternatives to attach to an unknown-key rejection. The
/// final per-diagnostic cap is `ExpansionBudgets.max_suggestions`, applied
/// downstream; this is a generous pre-cap (this pass is not budget-bearing).
const SUGGESTION_CAP: u8 = 5;

/// The Pass-2 product: kind + retained item + grammar-checked keys + binding
/// rows + span table. Structurally the input to Pass 3.
pub struct ParsedDeclaration {
    pub kind: ContractKind,
    pub item: SynItem,
    pub keys: KeyBindings,
    pub bindings: Vec<EventBindingRow>,
    pub spans: SpanTable,
}

/// The grammar-checked scalar/config keys of a declaration (order-preserving;
/// may hold across-attribute duplicates, which Pass 3 rejects by arity).
pub struct KeyBindings {
    entries: Vec<KeyEntry>,
}

/// One grammar-legal key binding: its canonical [`KeyName`] (from the grammar,
/// never user text), its parsed value, and the key's span.
#[derive(Clone)]
pub struct KeyEntry {
    pub name: KeyName,
    pub value: KeyValue,
    pub span: Span,
}

/// A parsed key value, in one of the shapes the value-shapes admit.
#[derive(Clone)]
pub enum KeyValue {
    Ident(Ident),
    Path(Path),
    Int(LitInt),
    Str(LitStr),
    StrList(Vec<LitStr>),
}

/// One `#[batpak(event = X, handler = h)]` binding row. Cloned into the
/// projection/subscription sub-IRs downstream.
#[derive(Clone)]
pub struct EventBindingRow {
    pub event: Path,
    pub handler: Ident,
    pub span: Span,
}

/// `KeyName -> Span` plus the call site, for span-accurate downstream blame.
pub struct SpanTable {
    entries: Vec<(KeyName, Span)>,
    call_site: Span,
}

/// A best-effort classification of the declared item's shape. Convenience for
/// downstream readers; Pass 2 shape *enforcement* is separate (see [`parse_declaration`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeclShape {
    NamedFieldStruct,
    Enum,
    UnitOnlyEnum,
    FreeFunction,
}

impl KeyBindings {
    pub fn get(&self, name: &str) -> Option<&KeyEntry> {
        self.entries.iter().find(|entry| entry.name.as_str() == name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Count occurrences of a key across all attributes (arity input for Pass 3).
    pub fn count(&self, name: &str) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.name.as_str() == name)
            .count()
    }

    pub fn as_int(&self, name: &str) -> Option<&LitInt> {
        match self.get(name) {
            Some(entry) => match &entry.value {
                KeyValue::Int(lit) => Some(lit),
                KeyValue::Ident(_)
                | KeyValue::Path(_)
                | KeyValue::Str(_)
                | KeyValue::StrList(_) => None,
            },
            None => None,
        }
    }

    pub fn as_path(&self, name: &str) -> Option<&Path> {
        match self.get(name) {
            Some(entry) => match &entry.value {
                KeyValue::Path(path) => Some(path),
                KeyValue::Ident(_)
                | KeyValue::Int(_)
                | KeyValue::Str(_)
                | KeyValue::StrList(_) => None,
            },
            None => None,
        }
    }

    pub fn as_str(&self, name: &str) -> Option<&LitStr> {
        match self.get(name) {
            Some(entry) => match &entry.value {
                KeyValue::Str(lit) => Some(lit),
                KeyValue::Ident(_)
                | KeyValue::Int(_)
                | KeyValue::Path(_)
                | KeyValue::StrList(_) => None,
            },
            None => None,
        }
    }

    pub fn as_str_list(&self, name: &str) -> Option<&[LitStr]> {
        match self.get(name) {
            Some(entry) => match &entry.value {
                KeyValue::StrList(list) => Some(list.as_slice()),
                KeyValue::Ident(_)
                | KeyValue::Int(_)
                | KeyValue::Path(_)
                | KeyValue::Str(_) => None,
            },
            None => None,
        }
    }
}

impl SpanTable {
    pub fn call_site(&self) -> Span {
        self.call_site
    }

    pub fn span_for(&self, name: &str) -> Option<Span> {
        self.entries
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, span)| *span)
    }
}

impl DeclShape {
    /// Best-effort shape classification of a parsed item.
    pub fn of(item: &SynItem) -> DeclShape {
        let derive = match item {
            SynItem::Attribute { .. } => return DeclShape::FreeFunction,
            SynItem::Derive(derive) => derive,
        };
        match &derive.data {
            Data::Struct(_) => DeclShape::NamedFieldStruct,
            Data::Enum(data) => {
                if data.variants.iter().all(|v| matches!(v.fields, Fields::Unit)) {
                    DeclShape::UnitOnlyEnum
                } else {
                    DeclShape::Enum
                }
            }
            Data::Union(_) => DeclShape::Enum,
        }
    }
}

/// Mutable accumulator threaded through the pass.
struct Accum {
    keys: Vec<KeyEntry>,
    bindings: Vec<EventBindingRow>,
    spans: Vec<(KeyName, Span)>,
    diags: Option<Diagnostics>,
}

impl Accum {
    fn record(&mut self, diagnostic: MacroDiagnostic) {
        match self.diags.take() {
            Some(mut existing) => {
                existing.push(diagnostic);
                self.diags = Some(existing);
            }
            None => self.diags = Some(Diagnostics::one(diagnostic)),
        }
    }
}

/// Interpret a parsed declaration against its grammar.
///
/// # Errors
///
/// Returns [`Diagnostics`] on any closed-vocabulary, shape, classification, or
/// in-attribute-duplicate violation.
pub fn parse_declaration(
    tree: SyntaxTree,
    grammar: &AttributeGrammar,
) -> Result<ParsedDeclaration, Diagnostics> {
    let SyntaxTree {
        kind,
        item,
        call_site,
    } = tree;

    let mut acc = Accum {
        keys: Vec::new(),
        bindings: Vec::new(),
        spans: Vec::new(),
        diags: None,
    };

    enforce_shape(kind, grammar, &item, call_site, &mut acc);

    match &item {
        SynItem::Attribute {
            attr,
            item: _function,
        } => parse_operation_args(attr, grammar, &mut acc),
        SynItem::Derive(derive) => {
            // Error / VariantInventory carry no `#[batpak(...)]` grammar; their
            // per-variant attributes and format strings are Pass-4 (IR) concerns.
            if grammar.keys.is_empty() && grammar.binding_keys.is_none() {
                // nothing to source
            } else {
                parse_batpak_attrs(kind, grammar, derive, call_site, &mut acc);
            }
        }
    }

    match acc.diags {
        Some(diags) => Err(diags),
        None => Ok(ParsedDeclaration {
            kind,
            item,
            keys: KeyBindings { entries: acc.keys },
            bindings: acc.bindings,
            spans: SpanTable {
                entries: acc.spans,
                call_site,
            },
        }),
    }
}

// ─── Shape enforcement (grammar.shape_rule driven) ───────────────────────────

fn enforce_shape(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    item: &SynItem,
    call_site: Span,
    acc: &mut Accum,
) {
    match &grammar.shape_rule {
        ShapeRule::NamedFieldStruct => enforce_named_field_struct(kind, item, call_site, acc),
        ShapeRule::Enum => enforce_enum(item, call_site, acc),
        ShapeRule::UnitOnlyEnum => enforce_unit_only_enum(item, call_site, acc),
        ShapeRule::FreeFunction => enforce_free_function(item, call_site, acc),
    }
}

fn enforce_named_field_struct(
    kind: ContractKind,
    item: &SynItem,
    call_site: Span,
    acc: &mut Accum,
) {
    let derive = match item {
        SynItem::Derive(derive) => derive,
        SynItem::Attribute { .. } => {
            acc.record(shape_change(
                DiagnosticCode::NotNamedFieldStruct,
                call_site,
                ShapeRule::NamedFieldStruct,
                "expected a named-field struct".to_string(),
            ));
            return;
        }
    };
    // Event forbids generic params (EventPayload cannot key on a generic);
    // projection/subscription support generics via split_for_impl and do not.
    if matches!(kind, ContractKind::Event) && !derive.generics.params.is_empty() {
        acc.record(MacroDiagnostic::from_spec(
            DiagnosticCode::EventGenericParams,
            derive.ident.span(),
            RequiredAction::HandWrite {
                reason: "hand-write EventPayload for a generic payload type",
            },
            "event payloads may not carry generic parameters".to_string(),
        ));
    }
    let named_struct = match &derive.data {
        Data::Struct(data) => matches!(&data.fields, Fields::Named(_)),
        Data::Enum(_) | Data::Union(_) => false,
    };
    if !named_struct {
        acc.record(shape_change(
            DiagnosticCode::NotNamedFieldStruct,
            derive.ident.span(),
            ShapeRule::NamedFieldStruct,
            "expected a named-field struct".to_string(),
        ));
    }
}

fn enforce_enum(item: &SynItem, call_site: Span, acc: &mut Accum) {
    let is_enum = match item {
        SynItem::Derive(derive) => matches!(&derive.data, Data::Enum(_)),
        SynItem::Attribute { .. } => false,
    };
    if !is_enum {
        acc.record(shape_change(
            DiagnosticCode::ErrorNotEnum,
            call_site,
            ShapeRule::Enum,
            "expected an enum".to_string(),
        ));
    }
}

fn enforce_unit_only_enum(item: &SynItem, call_site: Span, acc: &mut Accum) {
    let derive = match item {
        SynItem::Derive(derive) => derive,
        SynItem::Attribute { .. } => {
            acc.record(shape_change(
                DiagnosticCode::VariantInventoryNotEnum,
                call_site,
                ShapeRule::UnitOnlyEnum,
                "expected a unit-only enum".to_string(),
            ));
            return;
        }
    };
    let data = match &derive.data {
        Data::Enum(data) => data,
        Data::Struct(_) | Data::Union(_) => {
            acc.record(shape_change(
                DiagnosticCode::VariantInventoryNotEnum,
                derive.ident.span(),
                ShapeRule::UnitOnlyEnum,
                "expected a unit-only enum".to_string(),
            ));
            return;
        }
    };
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            acc.record(MacroDiagnostic::from_spec(
                DiagnosticCode::VariantInventoryNonUnitVariant,
                variant.span(),
                RequiredAction::HandWrite {
                    reason: "a data-carrying variant has no zero-argument value to enumerate",
                },
                "every variant must be a unit variant".to_string(),
            ));
        }
    }
}

fn enforce_free_function(item: &SynItem, call_site: Span, acc: &mut Accum) {
    let function = match item {
        SynItem::Attribute { item, .. } => item,
        SynItem::Derive(_) => {
            acc.record(shape_change(
                DiagnosticCode::OperationArityNotTwo,
                call_site,
                ShapeRule::FreeFunction,
                "expected a free function".to_string(),
            ));
            return;
        }
    };
    let sig = &function.sig;
    if let Some(asyncness) = &sig.asyncness {
        acc.record(hand_write(
            DiagnosticCode::OperationAsync,
            asyncness.span,
            "operations may not be async",
            "async operation".to_string(),
        ));
    }
    if let Some(unsafety) = &sig.unsafety {
        acc.record(hand_write(
            DiagnosticCode::OperationUnsafe,
            unsafety.span,
            "operations may not be unsafe",
            "unsafe operation".to_string(),
        ));
    }
    if let Some(abi) = &sig.abi {
        let is_rust = abi.name.as_ref().is_some_and(|name| name.value() == "Rust");
        if !is_rust {
            acc.record(hand_write(
                DiagnosticCode::OperationNonRustAbi,
                abi.extern_token.span,
                "operations must use the Rust ABI",
                "non-Rust ABI operation".to_string(),
            ));
        }
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        acc.record(hand_write(
            DiagnosticCode::OperationGeneric,
            sig.generics.span(),
            "operations may not be generic",
            "generic operation".to_string(),
        ));
    }
    if sig.inputs.len() != 2 {
        acc.record(shape_change(
            DiagnosticCode::OperationArityNotTwo,
            sig.inputs.span(),
            ShapeRule::FreeFunction,
            "operations take exactly two parameters".to_string(),
        ));
    }
    if sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_))) {
        acc.record(shape_change(
            DiagnosticCode::OperationHasReceiver,
            sig.inputs.span(),
            ShapeRule::FreeFunction,
            "operations must be free functions with no receiver".to_string(),
        ));
    }
}

// ─── Key sourcing: `#[batpak(...)]` derive attributes ────────────────────────

fn parse_batpak_attrs(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    derive: &syn::DeriveInput,
    call_site: Span,
    acc: &mut Accum,
) {
    let attrs: Vec<&Attribute> = derive
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("batpak"))
        .collect();

    // Attribute-count rule: Event admits exactly one; projection/subscription
    // (binding-bearing) admit one-or-more. Zero is a per-kind missing-attr.
    if attrs.is_empty() {
        acc.record(missing_attr_diag(kind, grammar, call_site));
        return;
    }
    if matches!(kind, ContractKind::Event) && attrs.len() > 1 {
        // The second attribute is the offending one.
        let offender = attrs.get(1).map_or(call_site, |attr| attr.span());
        acc.record(MacroDiagnostic::from_spec(
            DiagnosticCode::EventMultipleAttrs,
            offender,
            RequiredAction::HandWrite {
                reason: "exactly one #[batpak(...)] attribute is allowed",
            },
            "expected exactly one #[batpak(...)] attribute".to_string(),
        ));
        return;
    }

    for attr in attrs {
        parse_one_batpak_attr(kind, grammar, attr, acc);
    }
}

fn parse_one_batpak_attr(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    attr: &Attribute,
    acc: &mut Accum,
) {
    let pairs = match attr
        .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
    {
        Ok(pairs) => pairs,
        Err(err) => {
            acc.record(MacroDiagnostic::from_spec(
                DiagnosticCode::GrammarMetaPathNotIdent,
                attr.span(),
                RequiredAction::HandWrite {
                    reason: "#[batpak(...)] takes a comma-separated list of `key = value` pairs",
                },
                err.to_string(),
            ));
            return;
        }
    };

    if pairs.is_empty() {
        // Only binding-bearing kinds treat an empty attribute as a distinct
        // error; for Event, the missing keys surface in Pass 3.
        if grammar.binding_keys.is_some() {
            acc.record(add_key_diag(
                DiagnosticCode::EmptyBatpakAttr,
                attr.span(),
                first_required_key(grammar),
                first_required_form(grammar),
                "#[batpak(...)] must declare at least one key".to_string(),
            ));
        }
        return;
    }

    let mut has_config = false;
    let mut has_binding = false;
    let mut pending_event: Option<(Path, Span)> = None;
    let mut pending_handler: Option<Ident> = None;
    let mut seen: Vec<String> = Vec::new();

    for pair in &pairs {
        let ident = match pair.path.get_ident() {
            Some(ident) => ident,
            None => {
                acc.record(MacroDiagnostic::from_spec(
                    DiagnosticCode::GrammarMetaPathNotIdent,
                    pair.path.span(),
                    RequiredAction::HandWrite {
                        reason: "an attribute key must be a bare identifier",
                    },
                    "expected a bare identifier key".to_string(),
                ));
                continue;
            }
        };
        let name = ident.to_string();
        if seen.iter().any(|s| s == &name) {
            acc.record(duplicate_in_attr(kind, grammar, &name, pair.path.span()));
            continue;
        }
        seen.push(name.clone());

        // Binding-pair keys first (event / handler), if the grammar has them.
        if let Some(binding) = &grammar.binding_keys {
            if binding.event.as_str() == name {
                has_binding = true;
                match expr_as_path(&pair.value) {
                    Some(path) => pending_event = Some((path, pair.value.span())),
                    None => acc.record(value_mismatch_diag(
                        kind,
                        &ValueShape::Path {
                            single_segment: true,
                        },
                        binding.event.clone(),
                        ExpectedForm::SinglePath,
                        pair.value.span(),
                        "event must be a path".to_string(),
                    )),
                }
                continue;
            }
            if binding.handler.as_str() == name {
                has_binding = true;
                match expr_as_ident(&pair.value) {
                    Some(handler) => pending_handler = Some(handler),
                    None => acc.record(value_mismatch_diag(
                        kind,
                        &ValueShape::Ident,
                        binding.handler.clone(),
                        ExpectedForm::Ident,
                        pair.value.span(),
                        "handler must be an identifier".to_string(),
                    )),
                }
                continue;
            }
        }

        // Config keys from the grammar.
        match find_key(grammar, &name) {
            Some(spec) => {
                has_config = true;
                if let Some(value) = extract_value(kind, spec, &pair.value, acc) {
                    acc.keys.push(KeyEntry {
                        name: spec.name.clone(),
                        value,
                        span: pair.path.span(),
                    });
                    acc.spans.push((spec.name.clone(), pair.path.span()));
                }
            }
            None => acc.record(unknown_key_diag(kind, grammar, &name, pair.path.span())),
        }
    }

    // Config-vs-binding classification.
    if has_config && has_binding {
        let key = grammar
            .binding_keys
            .as_ref()
            .map_or_else(synthetic_key, |binding| binding.event.clone());
        acc.record(MacroDiagnostic::from_spec(
            DiagnosticCode::AttrConfigAndBindingMixed,
            attr.span(),
            RequiredAction::RemoveKey { key },
            "an attribute may carry config keys or an event binding, not both".to_string(),
        ));
        return;
    }

    if has_binding {
        finish_binding(grammar, attr.span(), pending_event, pending_handler, acc);
    }
}

fn finish_binding(
    grammar: &AttributeGrammar,
    attr_span: Span,
    pending_event: Option<(Path, Span)>,
    pending_handler: Option<Ident>,
    acc: &mut Accum,
) {
    let event = match pending_event {
        Some(event) => event,
        None => {
            acc.record(add_key_diag(
                DiagnosticCode::BindingMissingEvent,
                attr_span,
                binding_event_key(grammar),
                ExpectedForm::SinglePath,
                "event binding is missing `event = <Payload>`".to_string(),
            ));
            return;
        }
    };
    let handler = match pending_handler {
        Some(handler) => handler,
        None => {
            acc.record(MacroDiagnostic::from_spec(
                DiagnosticCode::BindingMissingHandler,
                attr_span,
                RequiredAction::AddKey {
                    key: binding_handler_key(grammar),
                    form: ExpectedForm::Ident,
                },
                "event binding is missing `handler = <fn>`".to_string(),
            ));
            return;
        }
    };
    let (event_path, event_span) = event;
    acc.bindings.push(EventBindingRow {
        event: event_path,
        handler,
        span: event_span,
    });
}

// ─── Key sourcing: operation attribute-macro args ────────────────────────────

fn parse_operation_args(
    attr: &proc_macro2::TokenStream,
    grammar: &AttributeGrammar,
    acc: &mut Accum,
) {
    use syn::parse::Parser;

    let pairs = match Punctuated::<MetaNameValue, Token![,]>::parse_terminated
        .parse2(attr.clone())
    {
        Ok(pairs) => pairs,
        Err(err) => {
            acc.record(MacroDiagnostic::from_spec(
                DiagnosticCode::OperationKeyPathNotIdent,
                attr.span(),
                RequiredAction::HandWrite {
                    reason: "operation arguments are a comma-separated list of `key = value` pairs",
                },
                err.to_string(),
            ));
            return;
        }
    };

    let mut seen: Vec<String> = Vec::new();
    for pair in &pairs {
        let ident = match pair.path.get_ident() {
            Some(ident) => ident,
            None => {
                acc.record(MacroDiagnostic::from_spec(
                    DiagnosticCode::OperationKeyPathNotIdent,
                    pair.path.span(),
                    RequiredAction::HandWrite {
                        reason: "an operation key must be a bare identifier",
                    },
                    "expected a bare identifier key".to_string(),
                ));
                continue;
            }
        };
        let name = ident.to_string();
        if seen.iter().any(|s| s == &name) {
            acc.record(MacroDiagnostic::from_spec(
                DiagnosticCode::OperationDuplicateKey,
                pair.path.span(),
                RequiredAction::RemoveKey {
                    key: find_key(grammar, &name)
                        .map_or_else(synthetic_key, |spec| spec.name.clone()),
                },
                format!("duplicate `{name}` key"),
            ));
            continue;
        }
        seen.push(name.clone());

        match find_key(grammar, &name) {
            Some(spec) => {
                if let Some(value) = extract_value(ContractKind::Operation, spec, &pair.value, acc)
                {
                    acc.keys.push(KeyEntry {
                        name: spec.name.clone(),
                        value,
                        span: pair.path.span(),
                    });
                    acc.spans.push((spec.name.clone(), pair.path.span()));
                }
            }
            None => acc.record(unknown_key_diag(
                ContractKind::Operation,
                grammar,
                &name,
                pair.path.span(),
            )),
        }
    }
}

// ─── Value extraction per declared value-shape ───────────────────────────────

fn extract_value(
    kind: ContractKind,
    spec: &KeySpec,
    value: &Expr,
    acc: &mut Accum,
) -> Option<KeyValue> {
    let span = value.span();
    match &spec.value {
        ValueShape::Ident | ValueShape::EffectClass => match expr_as_ident(value) {
            Some(ident) => Some(KeyValue::Ident(ident)),
            None => {
                acc.record(value_mismatch_diag(
                    kind,
                    &spec.value,
                    spec.name.clone(),
                    expected_form(&spec.value),
                    span,
                    "expected an identifier".to_string(),
                ));
                None
            }
        },
        ValueShape::Path { .. } => match expr_as_path(value) {
            Some(path) => Some(KeyValue::Path(path)),
            None => {
                acc.record(value_mismatch_diag(
                    kind,
                    &spec.value,
                    spec.name.clone(),
                    expected_form(&spec.value),
                    span,
                    "expected a path".to_string(),
                ));
                None
            }
        },
        ValueShape::IntLit { .. } => match expr_as_litint(value) {
            Some(lit) => Some(KeyValue::Int(lit)),
            None => {
                acc.record(value_mismatch_diag(
                    kind,
                    &spec.value,
                    spec.name.clone(),
                    expected_form(&spec.value),
                    span,
                    "expected an integer literal".to_string(),
                ));
                None
            }
        },
        ValueShape::StrLit => match expr_as_litstr(value) {
            Some(lit) => Some(KeyValue::Str(lit)),
            None => {
                acc.record(value_mismatch_diag(
                    kind,
                    &spec.value,
                    spec.name.clone(),
                    expected_form(&spec.value),
                    span,
                    "expected a string literal".to_string(),
                ));
                None
            }
        },
        ValueShape::StrList => match expr_as_strlist(value) {
            Ok(list) => Some(KeyValue::StrList(list)),
            Err(StrListError::NotArray) => {
                acc.record(value_mismatch_diag(
                    kind,
                    &spec.value,
                    spec.name.clone(),
                    expected_form(&spec.value),
                    span,
                    "expected an array of string literals".to_string(),
                ));
                None
            }
            Err(StrListError::ElementNotStr(element_span)) => {
                acc.record(MacroDiagnostic::from_spec(
                    element_list_code(kind),
                    element_span,
                    RequiredAction::ReplaceValue {
                        key: spec.name.clone(),
                        form: expected_form(&spec.value),
                    },
                    "array elements must be string literals".to_string(),
                ));
                None
            }
        },
    }
}

enum StrListError {
    NotArray,
    ElementNotStr(Span),
}

fn expr_as_ident(value: &Expr) -> Option<Ident> {
    let Expr::Path(path) = value else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    path.path.get_ident().cloned()
}

fn expr_as_path(value: &Expr) -> Option<Path> {
    let Expr::Path(path) = value else {
        return None;
    };
    Some(path.path.clone())
}

fn expr_as_litint(value: &Expr) -> Option<LitInt> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(lit), ..
    }) = value
    else {
        return None;
    };
    Some(lit.clone())
}

fn expr_as_litstr(value: &Expr) -> Option<LitStr> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = value
    else {
        return None;
    };
    Some(lit.clone())
}

fn expr_as_strlist(value: &Expr) -> Result<Vec<LitStr>, StrListError> {
    let Expr::Array(array) = value else {
        return Err(StrListError::NotArray);
    };
    let mut out = Vec::with_capacity(array.elems.len());
    for element in &array.elems {
        match expr_as_litstr(element) {
            Some(lit) => out.push(lit),
            None => return Err(StrListError::ElementNotStr(element.span())),
        }
    }
    Ok(out)
}

// ─── Grammar lookup + per-kind diagnostic-code selection ─────────────────────

fn find_key<'g>(grammar: &'g AttributeGrammar, name: &str) -> Option<&'g KeySpec> {
    grammar.keys.iter().find(|spec| spec.name.as_str() == name)
}

fn first_required_key(grammar: &AttributeGrammar) -> KeyName {
    grammar
        .keys
        .iter()
        .find(|spec| spec.required)
        .or_else(|| grammar.keys.first())
        .map_or_else(synthetic_key, |spec| spec.name.clone())
}

fn binding_event_key(grammar: &AttributeGrammar) -> KeyName {
    grammar
        .binding_keys
        .as_ref()
        .map_or_else(synthetic_key, |binding| binding.event.clone())
}

fn binding_handler_key(grammar: &AttributeGrammar) -> KeyName {
    grammar
        .binding_keys
        .as_ref()
        .map_or_else(synthetic_key, |binding| binding.handler.clone())
}

/// A stable fallback [`KeyName`] for the (unreachable in practice) case where a
/// binding/action needs a key but the grammar has none in hand. Keeps the
/// closed-vocabulary invariant total without constructing user text.
fn synthetic_key() -> KeyName {
    KeyName::new("input")
}

fn unknown_key_diag(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    unknown: &str,
    span: Span,
) -> MacroDiagnostic {
    let ranked = rank_alternatives(unknown, grammar.keys, SUGGESTION_CAP);
    let action = unknown_key_action(grammar, &ranked);
    let mut diagnostic = MacroDiagnostic::from_spec(
        unknown_key_code(kind),
        span,
        action,
        format!("unknown key `{unknown}`"),
    );
    diagnostic.suggestions = ranked;
    diagnostic
}

fn unknown_key_action(grammar: &AttributeGrammar, ranked: &[Suggestion]) -> RequiredAction {
    let target = ranked
        .first()
        .and_then(|suggestion| {
            grammar
                .keys
                .iter()
                .find(|spec| spec.name.as_str() == suggestion.replacement)
        })
        .or_else(|| grammar.keys.first());
    match target {
        Some(spec) => RequiredAction::ReplaceValue {
            key: spec.name.clone(),
            form: expected_form(&spec.value),
        },
        None => RequiredAction::HandWrite {
            reason: "unknown key with no known alternative",
        },
    }
}

fn unknown_key_code(kind: ContractKind) -> DiagnosticCode {
    match kind {
        ContractKind::Operation => DiagnosticCode::OperationUnknownKey,
        ContractKind::Event
        | ContractKind::Error
        | ContractKind::VariantInventory
        | ContractKind::Projection
        | ContractKind::Subscription => DiagnosticCode::GrammarUnknownKey,
    }
}

fn duplicate_in_attr(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    name: &str,
    span: Span,
) -> MacroDiagnostic {
    let key = resolve_key_name(grammar, name);
    let code = match kind {
        ContractKind::Operation => DiagnosticCode::OperationDuplicateKey,
        ContractKind::Event
        | ContractKind::Error
        | ContractKind::VariantInventory
        | ContractKind::Projection
        | ContractKind::Subscription => DiagnosticCode::GrammarDuplicateKeyInAttr,
    };
    MacroDiagnostic::from_spec(
        code,
        span,
        RequiredAction::RemoveKey { key },
        format!("duplicate `{name}` key"),
    )
}

/// Resolve a user key string back to a canonical grammar [`KeyName`] (config or
/// binding key). Only known keys reach here (unknown keys are rejected earlier).
fn resolve_key_name(grammar: &AttributeGrammar, name: &str) -> KeyName {
    if let Some(spec) = find_key(grammar, name) {
        return spec.name.clone();
    }
    if let Some(binding) = &grammar.binding_keys {
        if binding.event.as_str() == name {
            return binding.event.clone();
        }
        if binding.handler.as_str() == name {
            return binding.handler.clone();
        }
    }
    synthetic_key()
}

fn value_mismatch_diag(
    kind: ContractKind,
    shape: &ValueShape,
    key: KeyName,
    form: ExpectedForm,
    span: Span,
    message: String,
) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(
        value_mismatch_code(kind, shape),
        span,
        RequiredAction::ReplaceValue { key, form },
        message,
    )
}

fn value_mismatch_code(kind: ContractKind, shape: &ValueShape) -> DiagnosticCode {
    match kind {
        ContractKind::Operation => match shape {
            ValueShape::Ident => DiagnosticCode::OperationIdentValueNotIdent,
            ValueShape::StrLit => DiagnosticCode::OperationStringValueNotStrlit,
            ValueShape::StrList => DiagnosticCode::OperationListNotArray,
            ValueShape::EffectClass => DiagnosticCode::OperationEffectNotIdent,
            ValueShape::Path { .. } | ValueShape::IntLit { .. } => {
                DiagnosticCode::OperationIdentValueNotIdent
            }
        },
        // Derive kinds have no dedicated "value wrong shape" code; carry it
        // under the closest structural code (a malformed meta value).
        ContractKind::Event
        | ContractKind::Error
        | ContractKind::VariantInventory
        | ContractKind::Projection
        | ContractKind::Subscription => DiagnosticCode::GrammarMetaPathNotIdent,
    }
}

fn element_list_code(kind: ContractKind) -> DiagnosticCode {
    match kind {
        ContractKind::Operation => DiagnosticCode::OperationListElementNotStrlit,
        ContractKind::Event
        | ContractKind::Error
        | ContractKind::VariantInventory
        | ContractKind::Projection
        | ContractKind::Subscription => DiagnosticCode::GrammarMetaPathNotIdent,
    }
}

fn missing_attr_diag(
    kind: ContractKind,
    grammar: &AttributeGrammar,
    span: Span,
) -> MacroDiagnostic {
    let code = match kind {
        ContractKind::Event => DiagnosticCode::EventMissingAttr,
        ContractKind::Projection => DiagnosticCode::ProjectionMissingBatpakAttr,
        ContractKind::Subscription => DiagnosticCode::SubscriptionMissingBatpakAttr,
        ContractKind::Error | ContractKind::VariantInventory | ContractKind::Operation => {
            DiagnosticCode::EventMissingAttr
        }
    };
    add_key_diag(
        code,
        span,
        first_required_key(grammar),
        first_required_form(grammar),
        "a #[batpak(...)] attribute is required".to_string(),
    )
}

// ─── Small diagnostic constructors ───────────────────────────────────────────

fn shape_change(
    code: DiagnosticCode,
    span: Span,
    expected: ShapeRule,
    message: String,
) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(code, span, RequiredAction::ChangeShape { expected }, message)
}

fn hand_write(
    code: DiagnosticCode,
    span: Span,
    reason: &'static str,
    message: String,
) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(code, span, RequiredAction::HandWrite { reason }, message)
}

fn add_key_diag(
    code: DiagnosticCode,
    span: Span,
    key: KeyName,
    form: ExpectedForm,
    message: String,
) -> MacroDiagnostic {
    MacroDiagnostic::from_spec(code, span, RequiredAction::AddKey { key, form }, message)
}

/// The expected form of the grammar's first required (else first) config key —
/// the natural "add this key" hint for a whole-attribute absence.
fn first_required_form(grammar: &AttributeGrammar) -> ExpectedForm {
    grammar
        .keys
        .iter()
        .find(|spec| spec.required)
        .or_else(|| grammar.keys.first())
        .map_or(ExpectedForm::Ident, |spec| expected_form(&spec.value))
}

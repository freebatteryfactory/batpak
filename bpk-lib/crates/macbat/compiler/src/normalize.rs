//! Pass 7 — normalize: the determinism witnesses (exit gate 20).
//!
//! Two canonical, span-free, deterministic projections live here:
//!
//! * [`canonicalize`] renders an [`EmittedItemModel`] to the byte-identical
//!   `expansion.norm` text — items sorted by `(role, sort_key)`, each node
//!   rendered to a string, joined with `\n`.
//! * [`ContractSnapshot`] is the span-free projection of a [`ContractIr`] that
//!   the snapshot harness (Cluster 14) writes/asserts as `contract.ir`. Every
//!   syntax-bearing value (`Ident`/`Path`/`Type`/literal) becomes a normalized
//!   token string; NO `Debug` formatting of `syn` types is ever emitted (that
//!   would leak spans and `syn`-version confetti). Pipeline:
//!   `ContractIr → ContractSnapshot → canonical text`.
//!
//! Purity: `Vec`/`BTreeSet` only, no `HashMap`; token rendering via
//! `to_token_stream().to_string()` is deterministic and span-free.

use crate::artifact::NormalizedExpansion;
use crate::declaration::EventBindingRow;
use crate::emit::{EmittedItem, EmittedItemModel};
use crate::ir::error::{
    ErrorFields, ErrorIr, ErrorVariant, FormatSpec, NamedField, SourceBinding, SourceMember,
};
use crate::ir::event::EventIr;
use crate::ir::operation::{EffectClass, EffectRow, OperationIr};
use crate::ir::projection::{ProjectionIr, StateContract};
use crate::ir::subscription::SubscriptionIr;
use crate::ir::variant_inventory::VariantInventoryIr;
use crate::ir::{ContractIr, ContractKindIr};
use quote::ToTokens;

/// Render the emitted item model to its canonical, byte-identical text.
///
/// Items are ordered by `(role as u8, sort_key)` (a stable sort, so ties keep
/// source order), each node is rendered to a string via its assembled tokens,
/// and the strings are joined with `\n`. Same declaration + same compiler
/// version yields byte-identical output.
#[must_use]
pub fn canonicalize(model: &EmittedItemModel) -> NormalizedExpansion {
    let mut ordered: Vec<&EmittedItem> = model.items.iter().collect();
    ordered.sort_by(|left, right| {
        let left_key = (left.role.clone() as u8, left.sort_key.as_str());
        let right_key = (right.role.clone() as u8, right.sort_key.as_str());
        left_key.cmp(&right_key)
    });
    let canonical = ordered
        .iter()
        .map(|item| item.node.to_item_tokens().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    NormalizedExpansion {
        contract: model.contract.clone(),
        canonical,
    }
}

/// The span-free, canonically-serializable projection of a [`ContractIr`].
///
/// The rich [`ContractIr`] stays the working representation; this is the stable
/// review artifact. The `owner` field is deliberately omitted: it is symbolic
/// blame metadata, not part of the emitted semantic contract.
#[derive(Clone, Debug)]
pub struct ContractSnapshot {
    pub contract_id: String,
    pub version: u16,
    pub concept: &'static str,
    pub kind: &'static str,
    pub reserved_invariants: usize,
    pub reserved_witnesses: usize,
    pub reserved_observations: usize,
    pub body: SnapshotValue,
}

/// A canonical, span-free value in a contract snapshot. All syntax-bearing
/// values are already flattened to `Text` token strings; there is no `syn` type
/// anywhere in this tree.
#[derive(Clone, Debug)]
pub enum SnapshotValue {
    Text(String),
    Int(u64),
    Bool(bool),
    Symbol(&'static str),
    Optional(Option<Box<SnapshotValue>>),
    List(Vec<SnapshotValue>),
    Record(Vec<SnapshotField>),
}

/// A named field inside a [`SnapshotValue::Record`].
#[derive(Clone, Debug)]
pub struct SnapshotField {
    pub name: &'static str,
    pub value: SnapshotValue,
}

impl ContractSnapshot {
    /// Project a contract IR into its span-free snapshot.
    #[must_use]
    pub fn project(ir: &ContractIr) -> Self {
        let (concept, kind, body) = match &ir.kind {
            ContractKindIr::Error(inner) => ("error", "Error", project_error(inner)),
            ContractKindIr::Event(inner) => ("event", "Event", project_event(inner)),
            ContractKindIr::VariantInventory(inner) => {
                ("module", "VariantInventory", project_variant_inventory(inner))
            }
            ContractKindIr::Projection(inner) => {
                ("projection", "Projection", project_projection(inner))
            }
            ContractKindIr::Subscription(inner) => {
                ("subscription", "Subscription", project_subscription(inner))
            }
            ContractKindIr::Operation(inner) => {
                ("operation", "Operation", project_operation(inner))
            }
        };
        ContractSnapshot {
            contract_id: ir.id.as_str().to_owned(),
            version: ir.version.0,
            concept,
            kind,
            reserved_invariants: ir.invariants.len(),
            reserved_witnesses: ir.witnesses.len(),
            reserved_observations: ir.observations.len(),
            body,
        }
    }

    /// Serialize the snapshot to deterministic canonical text (fixed field
    /// order, no `Debug` formatting, no spans).
    #[must_use]
    pub fn canonical_text(&self) -> String {
        let root = vec![
            field("contract_id", SnapshotValue::Text(self.contract_id.clone())),
            field("version", SnapshotValue::Int(u64::from(self.version))),
            field("concept", SnapshotValue::Symbol(self.concept)),
            field("kind", SnapshotValue::Symbol(self.kind)),
            field(
                "reserved_invariants",
                int_of_usize(self.reserved_invariants),
            ),
            field("reserved_witnesses", int_of_usize(self.reserved_witnesses)),
            field(
                "reserved_observations",
                int_of_usize(self.reserved_observations),
            ),
            field("body", self.body.clone()),
        ];
        let mut out = String::new();
        write_record(&mut out, &root, 0);
        out
    }
}

// -- value constructors -----------------------------------------------------

fn field(name: &'static str, value: SnapshotValue) -> SnapshotField {
    SnapshotField { name, value }
}

/// Flatten any syntax-bearing value to a span-free token string.
fn syntax<T: ToTokens>(value: &T) -> SnapshotValue {
    SnapshotValue::Text(value.to_token_stream().to_string())
}

fn int_of_usize(value: usize) -> SnapshotValue {
    SnapshotValue::Int(match u64::try_from(value) {
        Ok(widened) => widened,
        Err(_) => u64::MAX,
    })
}

// -- per-kind projections (read the pub sub-IR fields from B.5) --------------

fn project_error(ir: &ErrorIr) -> SnapshotValue {
    let variants = ir.variants.iter().map(project_error_variant).collect();
    SnapshotValue::Record(vec![
        field("ident", syntax(&ir.ident)),
        field("generics", syntax(&ir.generics)),
        field("variants", SnapshotValue::List(variants)),
    ])
}

fn project_error_variant(variant: &ErrorVariant) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&variant.ident)),
        field("fields", project_error_fields(&variant.fields)),
        field("display", project_format_spec(&variant.display)),
        field("source", project_optional_source(&variant.source)),
    ])
}

fn project_error_fields(fields: &ErrorFields) -> SnapshotValue {
    match fields {
        ErrorFields::Named(named) => SnapshotValue::Record(vec![
            field("shape", SnapshotValue::Symbol("named")),
            field(
                "fields",
                SnapshotValue::List(named.iter().map(project_named_field).collect()),
            ),
        ]),
        ErrorFields::Unnamed(count) => SnapshotValue::Record(vec![
            field("shape", SnapshotValue::Symbol("unnamed")),
            field("count", int_of_usize(*count)),
        ]),
        ErrorFields::Unit => SnapshotValue::Symbol("unit"),
    }
}

fn project_named_field(named: &NamedField) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&named.ident)),
        field("ty", syntax(&named.ty)),
    ])
}

fn project_format_spec(spec: &FormatSpec) -> SnapshotValue {
    let used_names = spec
        .used_names
        .iter()
        .map(|name| SnapshotValue::Text(name.clone()))
        .collect();
    let used_positions = spec
        .used_positions
        .iter()
        .map(|position| int_of_usize(*position))
        .collect();
    SnapshotValue::Record(vec![
        field("original", syntax(&spec.original)),
        field("rewritten", SnapshotValue::Text(spec.rewritten.clone())),
        field("used_names", SnapshotValue::List(used_names)),
        field("used_positions", SnapshotValue::List(used_positions)),
    ])
}

fn project_optional_source(source: &Option<SourceBinding>) -> SnapshotValue {
    match source {
        None => SnapshotValue::Optional(None),
        Some(binding) => {
            SnapshotValue::Optional(Some(Box::new(project_source_binding(binding))))
        }
    }
}

fn project_source_binding(binding: &SourceBinding) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("member", project_source_member(&binding.member)),
        field("is_box", SnapshotValue::Bool(binding.is_box)),
        field("ty", syntax(&binding.ty)),
        field("is_from", SnapshotValue::Bool(binding.is_from)),
    ])
}

fn project_source_member(member: &SourceMember) -> SnapshotValue {
    match member {
        SourceMember::Named(ident) => SnapshotValue::Record(vec![
            field("kind", SnapshotValue::Symbol("named")),
            field("ident", syntax(ident)),
        ]),
        SourceMember::Indexed(index) => SnapshotValue::Record(vec![
            field("kind", SnapshotValue::Symbol("indexed")),
            field("index", int_of_usize(*index)),
        ]),
    }
}

fn project_event(ir: &EventIr) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&ir.ident)),
        field("category", SnapshotValue::Int(u64::from(ir.category))),
        field("type_id", SnapshotValue::Int(u64::from(ir.type_id))),
        field(
            "payload_version",
            SnapshotValue::Int(u64::from(ir.payload_version)),
        ),
        field("kind_bits", SnapshotValue::Int(u64::from(ir.kind_bits))),
    ])
}

fn project_variant_inventory(ir: &VariantInventoryIr) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&ir.ident)),
        field("generics", syntax(&ir.generics)),
        field(
            "variants",
            SnapshotValue::List(ir.variants.iter().map(syntax).collect()),
        ),
    ])
}

fn project_projection(ir: &ProjectionIr) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&ir.ident)),
        field("generics", syntax(&ir.generics)),
        field("input", syntax(&ir.input)),
        field("cache_version", SnapshotValue::Int(ir.cache_version)),
        field("state_contract", project_optional_state(&ir.state_contract)),
        field(
            "bindings",
            SnapshotValue::List(ir.bindings.iter().map(project_binding).collect()),
        ),
    ])
}

fn project_optional_state(state: &Option<StateContract>) -> SnapshotValue {
    match state {
        None => SnapshotValue::Optional(None),
        Some(contract) => SnapshotValue::Optional(Some(Box::new(SnapshotValue::Record(vec![
            field(
                "max_cardinality",
                SnapshotValue::Int(contract.max_cardinality),
            ),
        ])))),
    }
}

fn project_binding(row: &EventBindingRow) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("event", syntax(&row.event)),
        field("handler", syntax(&row.handler)),
    ])
}

fn project_subscription(ir: &SubscriptionIr) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("ident", syntax(&ir.ident)),
        field("generics", syntax(&ir.generics)),
        field("input", syntax(&ir.input)),
        field("error", syntax(&ir.error)),
        field(
            "bindings",
            SnapshotValue::List(ir.bindings.iter().map(project_binding).collect()),
        ),
    ])
}

fn project_operation(ir: &OperationIr) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("function", syntax(&ir.function)),
        field("descriptor", syntax(&ir.descriptor)),
        field("name", syntax(&ir.name)),
        field("effect", SnapshotValue::Symbol(effect_symbol(&ir.effect))),
        field("input_schema", syntax(&ir.input_schema)),
        field("output_schema", syntax(&ir.output_schema)),
        field("receipt_kind", syntax(&ir.receipt_kind)),
        field("title", project_optional_syntax(ir.title.as_ref())),
        field("effect_row", project_effect_row(&ir.effect_row)),
        field("register", project_optional_syntax(ir.register.as_ref())),
        field(
            "register_item",
            project_optional_syntax(ir.register_item.as_ref()),
        ),
    ])
}

fn effect_symbol(effect: &EffectClass) -> &'static str {
    match effect {
        EffectClass::Inspect => "inspect",
        EffectClass::Compute => "compute",
        EffectClass::Persist => "persist",
        EffectClass::Emit => "emit",
        EffectClass::Control => "control",
    }
}

fn project_effect_row(row: &EffectRow) -> SnapshotValue {
    SnapshotValue::Record(vec![
        field("reads_events", lit_list(&row.reads_events)),
        field("appends_events", lit_list(&row.appends_events)),
        field("queries_projections", lit_list(&row.queries_projections)),
        field("emits_receipts", lit_list(&row.emits_receipts)),
        field("uses_host_controls", lit_list(&row.uses_host_controls)),
        field(
            "requires_capabilities",
            lit_list(&row.requires_capabilities),
        ),
    ])
}

fn lit_list(lits: &[syn::Lit]) -> SnapshotValue {
    SnapshotValue::List(lits.iter().map(syntax).collect())
}

fn project_optional_syntax<T: ToTokens>(value: Option<&T>) -> SnapshotValue {
    match value {
        None => SnapshotValue::Optional(None),
        Some(inner) => SnapshotValue::Optional(Some(Box::new(syntax(inner)))),
    }
}

// -- canonical text serializer ----------------------------------------------

fn write_record(out: &mut String, fields: &[SnapshotField], indent: usize) {
    for entry in fields {
        indent_pad(out, indent);
        out.push_str(entry.name);
        out.push_str(" =");
        write_child(out, &entry.value, indent);
    }
}

fn write_list(out: &mut String, items: &[SnapshotValue], indent: usize) {
    for item in items {
        indent_pad(out, indent);
        out.push('-');
        write_child(out, item, indent);
    }
}

/// Write a value that follows a `name =` or `-` marker: inline on the same line
/// when scalar, otherwise a newline plus an indented block.
fn write_child(out: &mut String, value: &SnapshotValue, indent: usize) {
    if let Some(text) = inline_text(value) {
        out.push(' ');
        out.push_str(&text);
        out.push('\n');
        return;
    }
    match value {
        SnapshotValue::Optional(Some(inner)) => write_child(out, inner, indent),
        SnapshotValue::List(items) => {
            if items.is_empty() {
                out.push_str(" []\n");
            } else {
                out.push('\n');
                write_list(out, items, indent + 1);
            }
        }
        SnapshotValue::Record(fields) => {
            if fields.is_empty() {
                out.push_str(" {}\n");
            } else {
                out.push('\n');
                write_record(out, fields, indent + 1);
            }
        }
        // Scalars are handled by `inline_text` above; enumerated for
        // exhaustiveness (no wildcard).
        SnapshotValue::Text(_)
        | SnapshotValue::Int(_)
        | SnapshotValue::Bool(_)
        | SnapshotValue::Symbol(_)
        | SnapshotValue::Optional(None) => {}
    }
}

/// The inline scalar rendering of a value, or `None` when it must render as a
/// block (a non-empty list/record, or a present optional wrapping a block).
fn inline_text(value: &SnapshotValue) -> Option<String> {
    match value {
        SnapshotValue::Text(text) => Some(quoted(text)),
        SnapshotValue::Int(number) => Some(number.to_string()),
        SnapshotValue::Bool(flag) => Some(if *flag { "true".to_owned() } else { "false".to_owned() }),
        SnapshotValue::Symbol(symbol) => Some((*symbol).to_owned()),
        SnapshotValue::Optional(None) => Some("none".to_owned()),
        SnapshotValue::Optional(Some(_))
        | SnapshotValue::List(_)
        | SnapshotValue::Record(_) => None,
    }
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn indent_pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

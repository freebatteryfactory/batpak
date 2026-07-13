//! The six concrete attribute grammars, one per v1 contract kind.
//!
//! These are const *data* lifted verbatim-in-behaviour from the current
//! parsers (Event ← `event_payload.rs`, Error ← `error.rs`, VariantInventory ←
//! `all_variants.rs`, Projection/Subscription ← `lib.rs`, Operation ←
//! `operation.rs`). Every input the current parsers accept stays accepted and
//! every input they reject stays rejected; only diagnostic *text* may change.
//!
//! Recognized-but-rejected keys: the shared config parser recognizes the union
//! of config keys, so a key legal on one kind but illegal on another is listed
//! here (recognized, no ranked-alternatives) and rejected by a named
//! value-domain rule in validation — `error` on Projection, and `cache_version`
//! / `state_max_cardinality` on Subscription.

use super::{Arity, AttributeGrammar, BindingKeySpec, GrammarDocs, KeySpec, ValueShape};
use crate::identity::{ContractKind, KeyName, ShapeRule};

/// The `event`/`handler` binding pair shared by projection and subscription.
const EVENT_HANDLER_BINDING: BindingKeySpec = BindingKeySpec {
    event: KeyName::new("event"),
    handler: KeyName::new("handler"),
};

/// Config keys are mutually exclusive with the binding pair within one
/// attribute; naming the binding keys makes that incompatibility explicit.
const BINDING_KEY_NAMES: &[&str] = &["event", "handler"];

/// Event payload grammar: a named-field struct declaring `category`, `type_id`,
/// and an optional `version`, delivered via exactly one `#[batpak(...)]`.
pub const EVENT: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::Event,
    keys: &[
        KeySpec {
            name: KeyName::new("category"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::IntLit { bits: 8 },
            required: true,
            help: "4-bit custom event category (0x1-0xF, excluding 0x0 and 0xD)",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("type_id"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::IntLit { bits: 16 },
            required: true,
            help: "12-bit type identifier within the category (0x000-0x0FFF)",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("version"),
            arity: Arity::AtMostOnce,
            value: ValueShape::IntLit { bits: 16 },
            required: false,
            help: "payload schema version (defaults to 1; 0 is reserved)",
            incompatible_with: &[],
        },
    ],
    shape_rule: ShapeRule::NamedFieldStruct,
    binding_keys: None,
    docs: GrammarDocs {
        summary: "Event payload: a named-field struct carrying category, type_id, and optional version.",
    },
};

/// Error grammar: an enum whose variants carry `#[error(\"...\")]` display and
/// optional `#[source]`/`#[from]` bindings. These are helper attributes, not
/// `#[batpak(...)]` keys, so the grammar declares no keys.
pub const ERROR: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::Error,
    keys: &[],
    shape_rule: ShapeRule::Enum,
    binding_keys: None,
    docs: GrammarDocs {
        summary: "Error enum: variants use `#[error(...)]` display plus optional `#[source]`/`#[from]` helper attributes.",
    },
};

/// Variant-inventory grammar: a unit-only enum whose variants are enumerated in
/// declaration order. No `#[batpak(...)]` attribute, no keys.
pub const VARIANT_INVENTORY: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::VariantInventory,
    keys: &[],
    shape_rule: ShapeRule::UnitOnlyEnum,
    binding_keys: None,
    docs: GrammarDocs {
        summary: "Variant inventory: a unit-only enum enumerated in declaration order.",
    },
};

/// Projection grammar: a named-field struct with a required `input`, optional
/// `cache_version` / `state_max_cardinality`, and one or more `(event, handler)`
/// bindings. `error` is recognized here but rejected by a value-domain rule
/// (projections have no associated error type).
pub const PROJECTION: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::Projection,
    keys: &[
        KeySpec {
            name: KeyName::new("input"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::Path { single_segment: false },
            required: true,
            help: "the projection input type path",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("cache_version"),
            arity: Arity::AtMostOnce,
            value: ValueShape::IntLit { bits: 64 },
            required: false,
            help: "projection cache schema version (defaults to 0)",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("state_max_cardinality"),
            arity: Arity::AtMostOnce,
            value: ValueShape::IntLit { bits: 64 },
            required: false,
            help: "bounded-state cardinality; only the value 1 is accepted",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("error"),
            arity: Arity::AtMostOnce,
            value: ValueShape::Path { single_segment: false },
            required: false,
            help: "not valid on projections: projections have no associated error type (rejected)",
            incompatible_with: BINDING_KEY_NAMES,
        },
    ],
    shape_rule: ShapeRule::NamedFieldStruct,
    binding_keys: Some(EVENT_HANDLER_BINDING),
    docs: GrammarDocs {
        summary: "Projection: a named-field struct with an input path and one or more (event, handler) bindings.",
    },
};

/// Subscription grammar: a named-field struct with a required `input` and
/// `error`, and one or more `(event, handler)` bindings. `cache_version` and
/// `state_max_cardinality` are recognized here but rejected by value-domain
/// rules (subscriptions carry neither).
pub const SUBSCRIPTION: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::Subscription,
    keys: &[
        KeySpec {
            name: KeyName::new("input"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::Path { single_segment: false },
            required: true,
            help: "the subscription input type path",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("error"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::Path { single_segment: false },
            required: true,
            help: "the subscription error type path",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("cache_version"),
            arity: Arity::AtMostOnce,
            value: ValueShape::IntLit { bits: 64 },
            required: false,
            help: "not valid on subscriptions (rejected)",
            incompatible_with: BINDING_KEY_NAMES,
        },
        KeySpec {
            name: KeyName::new("state_max_cardinality"),
            arity: Arity::AtMostOnce,
            value: ValueShape::IntLit { bits: 64 },
            required: false,
            help: "not valid on subscriptions (rejected)",
            incompatible_with: BINDING_KEY_NAMES,
        },
    ],
    shape_rule: ShapeRule::NamedFieldStruct,
    binding_keys: Some(EVENT_HANDLER_BINDING),
    docs: GrammarDocs {
        summary: "Subscription: a named-field struct with input and error paths and one or more (event, handler) bindings.",
    },
};

/// Operation grammar: a free function annotated with a `Punctuated`
/// `key = value` list. Six required scalar keys, three optional scalar keys,
/// and six optional string-list effect-row lanes.
pub const OPERATION: AttributeGrammar = AttributeGrammar {
    kind: ContractKind::Operation,
    keys: &[
        KeySpec {
            name: KeyName::new("descriptor"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::Ident,
            required: true,
            help: "identifier for the generated operation descriptor",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("name"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::StrLit,
            required: true,
            help: "the operation name string",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("effect"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::EffectClass,
            required: true,
            help: "effect class: Inspect, Compute, Persist, Emit, or Control",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("input_schema"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::StrLit,
            required: true,
            help: "the input schema string",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("output_schema"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::StrLit,
            required: true,
            help: "the output schema string",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("receipt_kind"),
            arity: Arity::ExactlyOnce,
            value: ValueShape::StrLit,
            required: true,
            help: "the receipt kind string",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("register"),
            arity: Arity::AtMostOnce,
            value: ValueShape::Ident,
            required: false,
            help: "identifier for the generated register fn",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("register_item"),
            arity: Arity::AtMostOnce,
            value: ValueShape::Ident,
            required: false,
            help: "identifier for the generated register-item fn",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("title"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrLit,
            required: false,
            help: "optional human-readable operation title",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("reads_events"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: event kinds the operation reads",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("appends_events"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: event kinds the operation appends",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("queries_projections"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: projections the operation queries",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("emits_receipts"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: receipts the operation emits",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("uses_host_controls"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: host controls the operation uses",
            incompatible_with: &[],
        },
        KeySpec {
            name: KeyName::new("requires_capabilities"),
            arity: Arity::AtMostOnce,
            value: ValueShape::StrList,
            required: false,
            help: "effect-row lane: capabilities the operation requires",
            incompatible_with: &[],
        },
    ],
    shape_rule: ShapeRule::FreeFunction,
    binding_keys: None,
    docs: GrammarDocs {
        summary: "Operation: a free function with six required keys, three optional keys, and six effect-row lanes.",
    },
};

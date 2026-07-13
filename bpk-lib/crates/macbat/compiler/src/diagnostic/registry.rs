//! The const diagnostic registry (master contract §D, GATE-1 review correction).
//!
//! [`DiagnosticCode`] is a CLOSED ENUM — one variant per registry entry — so
//! [`spec`] is total BY CONSTRUCTION: an exhaustive `match` maps every variant to
//! its metadata with no fallible lookup and no panic-free fallback to keep. The
//! registry holds only STABLE METADATA (severity, concept, violated rule,
//! explanation, action KIND, and the human slug). It deliberately does NOT
//! manufacture [`crate::diagnostic::RequiredAction`] payloads — an `AddKey` needs a
//! key name and expected form the registry cannot know — so the failure site
//! supplies the contextual action to [`crate::diagnostic::MacroDiagnostic::from_spec`].
//!
//! The stable `BP-…` string is a rendering of the variant
//! (`DiagnosticCode::stable_str`, owned by `identity.rs`), never the identity here.
//!
//! Reserved future-kind bands (`-1000` per namespace) and the reserved
//! `BP-CONTRACT-` / `ProvideLowering` missing-lowering diagnostic are absent from
//! v1 — all six kinds have complete lowerings.

use crate::diagnostic::Severity;
use crate::identity::{Concept, DiagnosticCode, ExplanationId, RuleId};

/// The immutable, per-code facts a diagnostic is stamped from.
pub struct DiagnosticSpec {
    pub code: DiagnosticCode,
    pub rule: RuleId,
    pub explanation: ExplanationId,
    pub severity: Severity,
    pub concept: Concept,
    pub action_kind: ActionKind,
    pub slug: &'static str,
}

/// The class of fix a code prescribes, for documentation. Distinct from
/// [`crate::diagnostic::RequiredAction`]: this is the payload-free classification
/// held in the registry; the failure site builds the payload-bearing action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionKind {
    AddKey,
    RemoveKey,
    ReplaceValue,
    ChangeShape,
    ProvideLowering,
    HandWrite,
}

/// Build one registry row. `rule` and `explanation` are keyed by the stable slug
/// (the rule that fires is 1:1 with the code; the explanation catalog is keyed by
/// slug until it lands).
const fn row(
    code: DiagnosticCode,
    slug: &'static str,
    concept: Concept,
    severity: Severity,
    action_kind: ActionKind,
) -> DiagnosticSpec {
    DiagnosticSpec {
        code,
        rule: RuleId::new(slug),
        explanation: ExplanationId::new(slug),
        severity,
        concept,
        action_kind,
        slug,
    }
}

// ── BP-SHAPE- : item or fn shape ────────────────────────────────────────────
const NOT_NAMED_FIELD_STRUCT: DiagnosticSpec = row(
    DiagnosticCode::NotNamedFieldStruct,
    "not-named-field-struct",
    Concept::MODULE,
    Severity::Error,
    ActionKind::ChangeShape,
);
const EVENT_GENERIC_PARAMS: DiagnosticSpec = row(
    DiagnosticCode::EventGenericParams,
    "event-generic-params",
    Concept::EVENT,
    Severity::Error,
    ActionKind::HandWrite,
);
const ERROR_NOT_ENUM: DiagnosticSpec = row(
    DiagnosticCode::ErrorNotEnum,
    "error-not-enum",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ChangeShape,
);
const VARIANT_INVENTORY_NOT_ENUM: DiagnosticSpec = row(
    DiagnosticCode::VariantInventoryNotEnum,
    "variant-inventory-not-enum",
    Concept::MODULE,
    Severity::Error,
    ActionKind::ChangeShape,
);
const VARIANT_INVENTORY_NON_UNIT_VARIANT: DiagnosticSpec = row(
    DiagnosticCode::VariantInventoryNonUnitVariant,
    "variant-inventory-non-unit-variant",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const OPERATION_ASYNC: DiagnosticSpec = row(
    DiagnosticCode::OperationAsync,
    "operation-async",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::HandWrite,
);
const OPERATION_UNSAFE: DiagnosticSpec = row(
    DiagnosticCode::OperationUnsafe,
    "operation-unsafe",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::HandWrite,
);
const OPERATION_NON_RUST_ABI: DiagnosticSpec = row(
    DiagnosticCode::OperationNonRustAbi,
    "operation-non-rust-abi",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::HandWrite,
);
const OPERATION_GENERIC: DiagnosticSpec = row(
    DiagnosticCode::OperationGeneric,
    "operation-generic",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::HandWrite,
);
const OPERATION_ARITY_NOT_TWO: DiagnosticSpec = row(
    DiagnosticCode::OperationArityNotTwo,
    "operation-arity-not-two",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ChangeShape,
);
const OPERATION_HAS_RECEIVER: DiagnosticSpec = row(
    DiagnosticCode::OperationHasReceiver,
    "operation-has-receiver",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ChangeShape,
);

// ── BP-GRAMMAR- : key vocabulary / presence / arity / duplicates ────────────
const EVENT_MISSING_ATTR: DiagnosticSpec = row(
    DiagnosticCode::EventMissingAttr,
    "event-missing-attr",
    Concept::EVENT,
    Severity::Error,
    ActionKind::AddKey,
);
const EVENT_MULTIPLE_ATTRS: DiagnosticSpec = row(
    DiagnosticCode::EventMultipleAttrs,
    "event-multiple-attrs",
    Concept::EVENT,
    Severity::Error,
    ActionKind::RemoveKey,
);
const GRAMMAR_UNKNOWN_KEY: DiagnosticSpec = row(
    DiagnosticCode::GrammarUnknownKey,
    "unknown-key",
    Concept::MODULE,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const GRAMMAR_DUPLICATE_KEY_IN_ATTR: DiagnosticSpec = row(
    DiagnosticCode::GrammarDuplicateKeyInAttr,
    "duplicate-key-in-attr",
    Concept::MODULE,
    Severity::Error,
    ActionKind::RemoveKey,
);
const GRAMMAR_META_PATH_NOT_IDENT: DiagnosticSpec = row(
    DiagnosticCode::GrammarMetaPathNotIdent,
    "meta-path-not-ident",
    Concept::MODULE,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_MISSING_CATEGORY: DiagnosticSpec = row(
    DiagnosticCode::EventMissingCategory,
    "event-missing-category",
    Concept::EVENT,
    Severity::Error,
    ActionKind::AddKey,
);
const EVENT_MISSING_TYPE_ID: DiagnosticSpec = row(
    DiagnosticCode::EventMissingTypeId,
    "event-missing-type-id",
    Concept::EVENT,
    Severity::Error,
    ActionKind::AddKey,
);
const ERROR_VARIANT_MISSING_ERROR_ATTR: DiagnosticSpec = row(
    DiagnosticCode::ErrorVariantMissingErrorAttr,
    "error-variant-missing-error-attr",
    Concept::ERROR,
    Severity::Error,
    ActionKind::AddKey,
);
const ERROR_VARIANT_DUPLICATE_ERROR_ATTR: DiagnosticSpec = row(
    DiagnosticCode::ErrorVariantDuplicateErrorAttr,
    "error-variant-duplicate-error-attr",
    Concept::ERROR,
    Severity::Error,
    ActionKind::RemoveKey,
);
const PROJECTION_MISSING_BATPAK_ATTR: DiagnosticSpec = row(
    DiagnosticCode::ProjectionMissingBatpakAttr,
    "projection-missing-batpak-attr",
    Concept::PROJECTION,
    Severity::Error,
    ActionKind::AddKey,
);
const PROJECTION_MISSING_INPUT: DiagnosticSpec = row(
    DiagnosticCode::ProjectionMissingInput,
    "projection-missing-input",
    Concept::PROJECTION,
    Severity::Error,
    ActionKind::AddKey,
);
const PROJECTION_NO_BINDINGS: DiagnosticSpec = row(
    DiagnosticCode::ProjectionNoBindings,
    "projection-no-bindings",
    Concept::PROJECTION,
    Severity::Error,
    ActionKind::AddKey,
);
const BINDING_MISSING_EVENT: DiagnosticSpec = row(
    DiagnosticCode::BindingMissingEvent,
    "binding-missing-event",
    Concept::MODULE,
    Severity::Error,
    ActionKind::AddKey,
);
const BINDING_MISSING_HANDLER: DiagnosticSpec = row(
    DiagnosticCode::BindingMissingHandler,
    "binding-missing-handler",
    Concept::MODULE,
    Severity::Error,
    ActionKind::AddKey,
);
const EMPTY_BATPAK_ATTR: DiagnosticSpec = row(
    DiagnosticCode::EmptyBatpakAttr,
    "empty-batpak-attr",
    Concept::MODULE,
    Severity::Error,
    ActionKind::AddKey,
);
const SUBSCRIPTION_MISSING_BATPAK_ATTR: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionMissingBatpakAttr,
    "subscription-missing-batpak-attr",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::AddKey,
);
const SUBSCRIPTION_MISSING_INPUT: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionMissingInput,
    "subscription-missing-input",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::AddKey,
);
const SUBSCRIPTION_MISSING_ERROR: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionMissingError,
    "subscription-missing-error",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::AddKey,
);
const SUBSCRIPTION_NO_BINDINGS: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionNoBindings,
    "subscription-no-bindings",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::AddKey,
);
const OPERATION_KEY_PATH_NOT_IDENT: DiagnosticSpec = row(
    DiagnosticCode::OperationKeyPathNotIdent,
    "operation-key-path-not-ident",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_UNKNOWN_KEY: DiagnosticSpec = row(
    DiagnosticCode::OperationUnknownKey,
    "operation-unknown-key",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_DUPLICATE_KEY: DiagnosticSpec = row(
    DiagnosticCode::OperationDuplicateKey,
    "operation-duplicate-key",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::RemoveKey,
);
const OPERATION_MISSING_REQUIRED_KEY: DiagnosticSpec = row(
    DiagnosticCode::OperationMissingRequiredKey,
    "operation-missing-required-key",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::AddKey,
);

// ── BP-VALUE- : a value violates its value-shape / range / fmt syntax ────────
const EVENT_CATEGORY_EXCEEDS_U8: DiagnosticSpec = row(
    DiagnosticCode::EventCategoryExceedsU8,
    "event-category-exceeds-u8",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_TYPE_ID_EXCEEDS_U16: DiagnosticSpec = row(
    DiagnosticCode::EventTypeIdExceedsU16,
    "event-type-id-exceeds-u16",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_TYPE_ID_EXCEEDS_12_BIT: DiagnosticSpec = row(
    DiagnosticCode::EventTypeIdExceeds12Bit,
    "event-type-id-exceeds-12-bit",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_VERSION_EXCEEDS_U16: DiagnosticSpec = row(
    DiagnosticCode::EventVersionExceedsU16,
    "event-version-exceeds-u16",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const ERROR_FMT_UNMATCHED_BRACE: DiagnosticSpec = row(
    DiagnosticCode::ErrorFmtUnmatchedBrace,
    "error-fmt-unmatched-brace",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const ERROR_FMT_DOLLAR_ARG: DiagnosticSpec = row(
    DiagnosticCode::ErrorFmtDollarArg,
    "error-fmt-dollar-arg",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const ERROR_FMT_UNTERMINATED_BRACE: DiagnosticSpec = row(
    DiagnosticCode::ErrorFmtUnterminatedBrace,
    "error-fmt-unterminated-brace",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const ERROR_FMT_INVALID_POSITION: DiagnosticSpec = row(
    DiagnosticCode::ErrorFmtInvalidPosition,
    "error-fmt-invalid-position",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_IDENT_VALUE_NOT_IDENT: DiagnosticSpec = row(
    DiagnosticCode::OperationIdentValueNotIdent,
    "operation-ident-value-not-ident",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_STRING_VALUE_NOT_STRLIT: DiagnosticSpec = row(
    DiagnosticCode::OperationStringValueNotStrlit,
    "operation-string-value-not-strlit",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_LIST_NOT_ARRAY: DiagnosticSpec = row(
    DiagnosticCode::OperationListNotArray,
    "operation-list-not-array",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_LIST_ELEMENT_NOT_STRLIT: DiagnosticSpec = row(
    DiagnosticCode::OperationListElementNotStrlit,
    "operation-list-element-not-strlit",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const OPERATION_EFFECT_NOT_IDENT: DiagnosticSpec = row(
    DiagnosticCode::OperationEffectNotIdent,
    "operation-effect-not-ident",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);

// ── BP-RULE- : reserved sentinels + cross-field / cross-attr rules ──────────
const EVENT_CATEGORY_RESERVED_0X0: DiagnosticSpec = row(
    DiagnosticCode::EventCategoryReserved0x0,
    "event-category-reserved-0x0",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_CATEGORY_RESERVED_0XD: DiagnosticSpec = row(
    DiagnosticCode::EventCategoryReserved0xD,
    "event-category-reserved-0xD",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_VERSION_ZERO_RESERVED: DiagnosticSpec = row(
    DiagnosticCode::EventVersionZeroReserved,
    "event-version-zero-reserved",
    Concept::EVENT,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const ERROR_MULTIPLE_SOURCE_FIELDS: DiagnosticSpec = row(
    DiagnosticCode::ErrorMultipleSourceFields,
    "error-multiple-source-fields",
    Concept::ERROR,
    Severity::Error,
    ActionKind::RemoveKey,
);
const ERROR_FROM_REQUIRES_SINGLE_FIELD: DiagnosticSpec = row(
    DiagnosticCode::ErrorFromRequiresSingleField,
    "error-from-requires-single-field",
    Concept::ERROR,
    Severity::Error,
    ActionKind::ChangeShape,
);
const ATTR_CONFIG_AND_BINDING_MIXED: DiagnosticSpec = row(
    DiagnosticCode::AttrConfigAndBindingMixed,
    "attr-config-and-binding-mixed",
    Concept::MODULE,
    Severity::Error,
    ActionKind::RemoveKey,
);
const PROJECTION_ERROR_KEY_REJECTED: DiagnosticSpec = row(
    DiagnosticCode::ProjectionErrorKeyRejected,
    "projection-error-key-rejected",
    Concept::PROJECTION,
    Severity::Error,
    ActionKind::RemoveKey,
);
const DUPLICATE_CONFIG_KEY_ACROSS_ATTRS: DiagnosticSpec = row(
    DiagnosticCode::DuplicateConfigKeyAcrossAttrs,
    "duplicate-config-key-across-attrs",
    Concept::MODULE,
    Severity::Error,
    ActionKind::RemoveKey,
);
const PROJECTION_STATE_MAX_NOT_ONE: DiagnosticSpec = row(
    DiagnosticCode::ProjectionStateMaxNotOne,
    "projection-state-max-not-one",
    Concept::PROJECTION,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const EVENT_PATH_NOT_SINGLE_SEGMENT: DiagnosticSpec = row(
    DiagnosticCode::EventPathNotSingleSegment,
    "event-path-not-single-segment",
    Concept::MODULE,
    Severity::Error,
    ActionKind::ReplaceValue,
);
const DUPLICATE_EVENT_BINDING: DiagnosticSpec = row(
    DiagnosticCode::DuplicateEventBinding,
    "duplicate-event-binding",
    Concept::MODULE,
    Severity::Error,
    ActionKind::RemoveKey,
);
const SUBSCRIPTION_CACHE_VERSION_REJECTED: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionCacheVersionRejected,
    "subscription-cache-version-rejected",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::RemoveKey,
);
const SUBSCRIPTION_STATE_MAX_REJECTED: DiagnosticSpec = row(
    DiagnosticCode::SubscriptionStateMaxRejected,
    "subscription-state-max-rejected",
    Concept::SUBSCRIPTION,
    Severity::Error,
    ActionKind::RemoveKey,
);
const OPERATION_UNSUPPORTED_EFFECT: DiagnosticSpec = row(
    DiagnosticCode::OperationUnsupportedEffect,
    "operation-unsupported-effect",
    Concept::OPERATION,
    Severity::Error,
    ActionKind::ReplaceValue,
);

// ── BP-BUDGET- : expansion limits (max-trace-records is a non-fatal marker) ──
const BUDGET_MAX_FIELDS: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxFields,
    "max-fields",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const BUDGET_MAX_VARIANTS: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxVariants,
    "max-variants",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const BUDGET_MAX_COMPOSITION_DEPTH: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxCompositionDepth,
    "max-composition-depth",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const BUDGET_MAX_GENERATED_ITEMS: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxGeneratedItems,
    "max-generated-items",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const BUDGET_MAX_TOKENS: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxTokens,
    "max-tokens",
    Concept::MODULE,
    Severity::Error,
    ActionKind::HandWrite,
);
const BUDGET_MAX_TRACE_RECORDS: DiagnosticSpec = row(
    DiagnosticCode::BudgetMaxTraceRecords,
    "max-trace-records",
    Concept::MODULE,
    Severity::Warning,
    ActionKind::HandWrite,
);

/// The complete v1 diagnostic registry (master contract §D). Ordered by
/// namespace; used for iteration (e.g. tests asserting every code has metadata).
pub const DIAGNOSTICS: &[DiagnosticSpec] = &[
    NOT_NAMED_FIELD_STRUCT,
    EVENT_GENERIC_PARAMS,
    ERROR_NOT_ENUM,
    VARIANT_INVENTORY_NOT_ENUM,
    VARIANT_INVENTORY_NON_UNIT_VARIANT,
    OPERATION_ASYNC,
    OPERATION_UNSAFE,
    OPERATION_NON_RUST_ABI,
    OPERATION_GENERIC,
    OPERATION_ARITY_NOT_TWO,
    OPERATION_HAS_RECEIVER,
    EVENT_MISSING_ATTR,
    EVENT_MULTIPLE_ATTRS,
    GRAMMAR_UNKNOWN_KEY,
    GRAMMAR_DUPLICATE_KEY_IN_ATTR,
    GRAMMAR_META_PATH_NOT_IDENT,
    EVENT_MISSING_CATEGORY,
    EVENT_MISSING_TYPE_ID,
    ERROR_VARIANT_MISSING_ERROR_ATTR,
    ERROR_VARIANT_DUPLICATE_ERROR_ATTR,
    PROJECTION_MISSING_BATPAK_ATTR,
    PROJECTION_MISSING_INPUT,
    PROJECTION_NO_BINDINGS,
    BINDING_MISSING_EVENT,
    BINDING_MISSING_HANDLER,
    EMPTY_BATPAK_ATTR,
    SUBSCRIPTION_MISSING_BATPAK_ATTR,
    SUBSCRIPTION_MISSING_INPUT,
    SUBSCRIPTION_MISSING_ERROR,
    SUBSCRIPTION_NO_BINDINGS,
    OPERATION_KEY_PATH_NOT_IDENT,
    OPERATION_UNKNOWN_KEY,
    OPERATION_DUPLICATE_KEY,
    OPERATION_MISSING_REQUIRED_KEY,
    EVENT_CATEGORY_EXCEEDS_U8,
    EVENT_TYPE_ID_EXCEEDS_U16,
    EVENT_TYPE_ID_EXCEEDS_12_BIT,
    EVENT_VERSION_EXCEEDS_U16,
    ERROR_FMT_UNMATCHED_BRACE,
    ERROR_FMT_DOLLAR_ARG,
    ERROR_FMT_UNTERMINATED_BRACE,
    ERROR_FMT_INVALID_POSITION,
    OPERATION_IDENT_VALUE_NOT_IDENT,
    OPERATION_STRING_VALUE_NOT_STRLIT,
    OPERATION_LIST_NOT_ARRAY,
    OPERATION_LIST_ELEMENT_NOT_STRLIT,
    OPERATION_EFFECT_NOT_IDENT,
    EVENT_CATEGORY_RESERVED_0X0,
    EVENT_CATEGORY_RESERVED_0XD,
    EVENT_VERSION_ZERO_RESERVED,
    ERROR_MULTIPLE_SOURCE_FIELDS,
    ERROR_FROM_REQUIRES_SINGLE_FIELD,
    ATTR_CONFIG_AND_BINDING_MIXED,
    PROJECTION_ERROR_KEY_REJECTED,
    DUPLICATE_CONFIG_KEY_ACROSS_ATTRS,
    PROJECTION_STATE_MAX_NOT_ONE,
    EVENT_PATH_NOT_SINGLE_SEGMENT,
    DUPLICATE_EVENT_BINDING,
    SUBSCRIPTION_CACHE_VERSION_REJECTED,
    SUBSCRIPTION_STATE_MAX_REJECTED,
    OPERATION_UNSUPPORTED_EFFECT,
    BUDGET_MAX_FIELDS,
    BUDGET_MAX_VARIANTS,
    BUDGET_MAX_COMPOSITION_DEPTH,
    BUDGET_MAX_GENERATED_ITEMS,
    BUDGET_MAX_TOKENS,
    BUDGET_MAX_TRACE_RECORDS,
];

/// The metadata for a code. Total by construction: the exhaustive `match` forces a
/// new arm the instant a [`DiagnosticCode`] variant is added, so no code can ship
/// without registry metadata and the function never panics.
#[must_use]
pub fn spec(code: DiagnosticCode) -> &'static DiagnosticSpec {
    match code {
        DiagnosticCode::NotNamedFieldStruct => &NOT_NAMED_FIELD_STRUCT,
        DiagnosticCode::EventGenericParams => &EVENT_GENERIC_PARAMS,
        DiagnosticCode::ErrorNotEnum => &ERROR_NOT_ENUM,
        DiagnosticCode::VariantInventoryNotEnum => &VARIANT_INVENTORY_NOT_ENUM,
        DiagnosticCode::VariantInventoryNonUnitVariant => &VARIANT_INVENTORY_NON_UNIT_VARIANT,
        DiagnosticCode::OperationAsync => &OPERATION_ASYNC,
        DiagnosticCode::OperationUnsafe => &OPERATION_UNSAFE,
        DiagnosticCode::OperationNonRustAbi => &OPERATION_NON_RUST_ABI,
        DiagnosticCode::OperationGeneric => &OPERATION_GENERIC,
        DiagnosticCode::OperationArityNotTwo => &OPERATION_ARITY_NOT_TWO,
        DiagnosticCode::OperationHasReceiver => &OPERATION_HAS_RECEIVER,
        DiagnosticCode::EventMissingAttr => &EVENT_MISSING_ATTR,
        DiagnosticCode::EventMultipleAttrs => &EVENT_MULTIPLE_ATTRS,
        DiagnosticCode::GrammarUnknownKey => &GRAMMAR_UNKNOWN_KEY,
        DiagnosticCode::GrammarDuplicateKeyInAttr => &GRAMMAR_DUPLICATE_KEY_IN_ATTR,
        DiagnosticCode::GrammarMetaPathNotIdent => &GRAMMAR_META_PATH_NOT_IDENT,
        DiagnosticCode::EventMissingCategory => &EVENT_MISSING_CATEGORY,
        DiagnosticCode::EventMissingTypeId => &EVENT_MISSING_TYPE_ID,
        DiagnosticCode::ErrorVariantMissingErrorAttr => &ERROR_VARIANT_MISSING_ERROR_ATTR,
        DiagnosticCode::ErrorVariantDuplicateErrorAttr => &ERROR_VARIANT_DUPLICATE_ERROR_ATTR,
        DiagnosticCode::ProjectionMissingBatpakAttr => &PROJECTION_MISSING_BATPAK_ATTR,
        DiagnosticCode::ProjectionMissingInput => &PROJECTION_MISSING_INPUT,
        DiagnosticCode::ProjectionNoBindings => &PROJECTION_NO_BINDINGS,
        DiagnosticCode::BindingMissingEvent => &BINDING_MISSING_EVENT,
        DiagnosticCode::BindingMissingHandler => &BINDING_MISSING_HANDLER,
        DiagnosticCode::EmptyBatpakAttr => &EMPTY_BATPAK_ATTR,
        DiagnosticCode::SubscriptionMissingBatpakAttr => &SUBSCRIPTION_MISSING_BATPAK_ATTR,
        DiagnosticCode::SubscriptionMissingInput => &SUBSCRIPTION_MISSING_INPUT,
        DiagnosticCode::SubscriptionMissingError => &SUBSCRIPTION_MISSING_ERROR,
        DiagnosticCode::SubscriptionNoBindings => &SUBSCRIPTION_NO_BINDINGS,
        DiagnosticCode::OperationKeyPathNotIdent => &OPERATION_KEY_PATH_NOT_IDENT,
        DiagnosticCode::OperationUnknownKey => &OPERATION_UNKNOWN_KEY,
        DiagnosticCode::OperationDuplicateKey => &OPERATION_DUPLICATE_KEY,
        DiagnosticCode::OperationMissingRequiredKey => &OPERATION_MISSING_REQUIRED_KEY,
        DiagnosticCode::EventCategoryExceedsU8 => &EVENT_CATEGORY_EXCEEDS_U8,
        DiagnosticCode::EventTypeIdExceedsU16 => &EVENT_TYPE_ID_EXCEEDS_U16,
        DiagnosticCode::EventTypeIdExceeds12Bit => &EVENT_TYPE_ID_EXCEEDS_12_BIT,
        DiagnosticCode::EventVersionExceedsU16 => &EVENT_VERSION_EXCEEDS_U16,
        DiagnosticCode::ErrorFmtUnmatchedBrace => &ERROR_FMT_UNMATCHED_BRACE,
        DiagnosticCode::ErrorFmtDollarArg => &ERROR_FMT_DOLLAR_ARG,
        DiagnosticCode::ErrorFmtUnterminatedBrace => &ERROR_FMT_UNTERMINATED_BRACE,
        DiagnosticCode::ErrorFmtInvalidPosition => &ERROR_FMT_INVALID_POSITION,
        DiagnosticCode::OperationIdentValueNotIdent => &OPERATION_IDENT_VALUE_NOT_IDENT,
        DiagnosticCode::OperationStringValueNotStrlit => &OPERATION_STRING_VALUE_NOT_STRLIT,
        DiagnosticCode::OperationListNotArray => &OPERATION_LIST_NOT_ARRAY,
        DiagnosticCode::OperationListElementNotStrlit => &OPERATION_LIST_ELEMENT_NOT_STRLIT,
        DiagnosticCode::OperationEffectNotIdent => &OPERATION_EFFECT_NOT_IDENT,
        DiagnosticCode::EventCategoryReserved0x0 => &EVENT_CATEGORY_RESERVED_0X0,
        DiagnosticCode::EventCategoryReserved0xD => &EVENT_CATEGORY_RESERVED_0XD,
        DiagnosticCode::EventVersionZeroReserved => &EVENT_VERSION_ZERO_RESERVED,
        DiagnosticCode::ErrorMultipleSourceFields => &ERROR_MULTIPLE_SOURCE_FIELDS,
        DiagnosticCode::ErrorFromRequiresSingleField => &ERROR_FROM_REQUIRES_SINGLE_FIELD,
        DiagnosticCode::AttrConfigAndBindingMixed => &ATTR_CONFIG_AND_BINDING_MIXED,
        DiagnosticCode::ProjectionErrorKeyRejected => &PROJECTION_ERROR_KEY_REJECTED,
        DiagnosticCode::DuplicateConfigKeyAcrossAttrs => &DUPLICATE_CONFIG_KEY_ACROSS_ATTRS,
        DiagnosticCode::ProjectionStateMaxNotOne => &PROJECTION_STATE_MAX_NOT_ONE,
        DiagnosticCode::EventPathNotSingleSegment => &EVENT_PATH_NOT_SINGLE_SEGMENT,
        DiagnosticCode::DuplicateEventBinding => &DUPLICATE_EVENT_BINDING,
        DiagnosticCode::SubscriptionCacheVersionRejected => &SUBSCRIPTION_CACHE_VERSION_REJECTED,
        DiagnosticCode::SubscriptionStateMaxRejected => &SUBSCRIPTION_STATE_MAX_REJECTED,
        DiagnosticCode::OperationUnsupportedEffect => &OPERATION_UNSUPPORTED_EFFECT,
        DiagnosticCode::BudgetMaxFields => &BUDGET_MAX_FIELDS,
        DiagnosticCode::BudgetMaxVariants => &BUDGET_MAX_VARIANTS,
        DiagnosticCode::BudgetMaxCompositionDepth => &BUDGET_MAX_COMPOSITION_DEPTH,
        DiagnosticCode::BudgetMaxGeneratedItems => &BUDGET_MAX_GENERATED_ITEMS,
        DiagnosticCode::BudgetMaxTokens => &BUDGET_MAX_TOKENS,
        DiagnosticCode::BudgetMaxTraceRecords => &BUDGET_MAX_TRACE_RECORDS,
    }
}

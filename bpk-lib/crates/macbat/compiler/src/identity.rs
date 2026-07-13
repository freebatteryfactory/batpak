//! Shared newtype vocabulary for the MacBat contract compiler.
//!
//! Every downstream module in `macbat-compiler` imports from here; this module
//! imports nothing else in the tree (it is the root of the dependency DAG). All
//! public string-shaped types are transparent newtypes — a bare `String`/`&str`
//! never appears in a public signature — so a value's *meaning* travels with its
//! type. The closed enums (`ContractKind`, `ShapeRule`, `LoweringRole`) are the
//! v1 vocabulary; adding a kind is a pure addition that the exhaustive matches
//! downstream mechanically force (see `lowering::plan`).
//!
//! This file is part of the pure compiler tree: no `std::fs`/`std::env`/
//! `std::time`, no `HashMap`, no I/O. It never imports `batpak` core.

/// Identity of one contract expansion, **local to that expansion artifact**.
///
/// A pure proc-macro pass cannot know the caller's crate or Rust module path, so
/// this id claims none: it is derived deterministically from the declaration's
/// [`ContractKind`] and ident ONLY, at expansion time — never from a runtime
/// `module_path!`. The same declaration always yields the same id, which is why
/// everything downstream (`ReservedIdent` hashing, origin maps, per-specimen
/// snapshots) can key off it: generated idents are module-scoped and snapshots
/// are per-specimen, so local determinism is sufficient and correct.
///
/// Global resolution — the actual crate + module path where this contract lives
/// — is Muterprater / Repo-IR enrichment: a `ResolvedContractId` owned by
/// crate 3, NOT defined here. MacBat knows what the declaration *means* locally;
/// Muterprater knows *where* it exists globally.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContractId(String);

/// Declaration schema version (NOT the on-wire payload version).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContractVersion(pub u16);

/// A closed-vocabulary concept tag. Constructed only via the associated
/// constants (`Concept::EVENT`, …) — never from arbitrary text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Concept(&'static str);

/// Symbolic authoring provenance, retained for blame in diagnostics/origins.
///
/// At expansion time MacBat cannot know the caller's real module path, so the
/// default is the symbolic [`Owner::caller_module`] sentinel rather than a faked
/// path. Concrete global provenance is Muterprater enrichment in crate 3.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Owner(String);

/// A stable diagnostic code, rendered as a `"BP-…"` string by
/// [`DiagnosticCode::stable_str`].
///
/// This is a CLOSED ENUM — exactly one variant per registry entry (master
/// contract §D), so `diagnostic::registry::spec` is total by construction: its
/// exhaustive `match` forces a new arm the instant a variant is added, and no
/// code can ship without registry metadata. The `"BP-…"` string is a *rendering*
/// of the variant, never its identity; the variant is the identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagnosticCode {
    // ── BP-SHAPE- : item or fn shape ────────────────────────────────────────
    NotNamedFieldStruct,
    EventGenericParams,
    ErrorNotEnum,
    VariantInventoryNotEnum,
    VariantInventoryNonUnitVariant,
    OperationAsync,
    OperationUnsafe,
    OperationNonRustAbi,
    OperationGeneric,
    OperationArityNotTwo,
    OperationHasReceiver,
    // ── BP-GRAMMAR- : key vocabulary / presence / arity / duplicates ────────
    EventMissingAttr,
    EventMultipleAttrs,
    GrammarUnknownKey,
    GrammarDuplicateKeyInAttr,
    GrammarMetaPathNotIdent,
    EventMissingCategory,
    EventMissingTypeId,
    ErrorVariantMissingErrorAttr,
    ErrorVariantDuplicateErrorAttr,
    ProjectionMissingBatpakAttr,
    ProjectionMissingInput,
    ProjectionNoBindings,
    BindingMissingEvent,
    BindingMissingHandler,
    EmptyBatpakAttr,
    SubscriptionMissingBatpakAttr,
    SubscriptionMissingInput,
    SubscriptionMissingError,
    SubscriptionNoBindings,
    OperationKeyPathNotIdent,
    OperationUnknownKey,
    OperationDuplicateKey,
    OperationMissingRequiredKey,
    // ── BP-VALUE- : a value violates its value-shape / range / fmt syntax ───
    EventCategoryExceedsU8,
    EventTypeIdExceedsU16,
    EventTypeIdExceeds12Bit,
    EventVersionExceedsU16,
    ErrorFmtUnmatchedBrace,
    ErrorFmtDollarArg,
    ErrorFmtUnterminatedBrace,
    ErrorFmtInvalidPosition,
    OperationIdentValueNotIdent,
    OperationStringValueNotStrlit,
    OperationListNotArray,
    OperationListElementNotStrlit,
    OperationEffectNotIdent,
    // ── BP-RULE- : reserved sentinels + cross-field / cross-attr rules ──────
    EventCategoryReserved0x0,
    EventCategoryReserved0xD,
    EventVersionZeroReserved,
    ErrorMultipleSourceFields,
    ErrorFromRequiresSingleField,
    AttrConfigAndBindingMixed,
    ProjectionErrorKeyRejected,
    DuplicateConfigKeyAcrossAttrs,
    ProjectionStateMaxNotOne,
    EventPathNotSingleSegment,
    DuplicateEventBinding,
    SubscriptionCacheVersionRejected,
    SubscriptionStateMaxRejected,
    OperationUnsupportedEffect,
    // ── BP-BUDGET- : expansion limits (max-trace-records is a non-fatal marker) ─
    BudgetMaxFields,
    BudgetMaxVariants,
    BudgetMaxCompositionDepth,
    BudgetMaxGeneratedItems,
    BudgetMaxTokens,
    BudgetMaxTraceRecords,
}

/// Points into the grammar/validation rule that fired.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(&'static str);

/// Key into the explanation catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExplanationId(&'static str);

/// Emitted-item identity: assigned in `emit`, deterministic, dense, source
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EmittedItemId(u32);

/// Origin blame path, e.g. `"variant.field"` / `"keys.category"`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldPath(String);

/// A grammar-declared key name. Only keys the grammar knows are ever
/// constructed (the grammar catalog is the sole caller of `new`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyName(&'static str);

/// Typed empty in v1 (reserved). See `ir::ContractIr::invariants`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InvariantId(&'static str);

/// Typed empty in v1 (reserved). See `ir::ContractIr::witnesses`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WitnessId(&'static str);

/// Typed empty in v1 (reserved). See `ir::ContractIr::observations`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObservationId(&'static str);

/// The front-door dispatch kind. EXACTLY six in v1; the other reserved kinds
/// live only as a documented list in the lowering extension contract, never as
/// variants here until their owning crate's packet lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ContractKind {
    Error,
    Event,
    VariantInventory,
    Projection,
    Subscription,
    Operation,
}

/// The declared item-shape law for a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShapeRule {
    NamedFieldStruct,
    Enum,
    UnitOnlyEnum,
    FreeFunction,
}

/// The closed set of lowering roles across all v1 kinds. Adding a kind adds its
/// roles here; the exhaustive matches downstream then force every new role to be
/// handled. `#[repr(u8)]` gives dense, stable discriminants for the
/// `(role, sort_key)` normalize ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum LoweringRole {
    // Error
    DisplayImpl,
    ErrorSourceImpl,
    FromImpl,
    // Event
    EventPayloadImpl,
    EventInventorySubmit,
    EventCollisionCheck,
    // VariantInventory
    VariantInventoryConst,
    // Projection
    ProjectionInputAssertion,
    ProjectionImpl,
    // Subscription
    SubscriptionAttrAssertion,
    SubscriptionImpl,
    // Operation
    OperationFunction,
    OperationDescriptor,
    OperationSignaturePin,
    OperationRegisterItem,
    OperationRegister,
}

impl Concept {
    /// Event concept tag.
    pub const EVENT: Concept = Concept("event");
    /// Error concept tag.
    pub const ERROR: Concept = Concept("error");
    /// Projection concept tag.
    pub const PROJECTION: Concept = Concept("projection");
    /// Subscription concept tag.
    pub const SUBSCRIPTION: Concept = Concept("subscription");
    /// Operation concept tag.
    pub const OPERATION: Concept = Concept("operation");
    /// Module concept tag — the concept of a `VariantInventory`.
    pub const MODULE: Concept = Concept("module");

    /// The transparent string form of this concept.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl ContractKind {
    /// The concept this kind reports under.
    pub const fn concept(self) -> Concept {
        match self {
            ContractKind::Error => Concept::ERROR,
            ContractKind::Event => Concept::EVENT,
            ContractKind::VariantInventory => Concept::MODULE,
            ContractKind::Projection => Concept::PROJECTION,
            ContractKind::Subscription => Concept::SUBSCRIPTION,
            ContractKind::Operation => Concept::OPERATION,
        }
    }

    /// `true` for the one attribute-macro kind (`Operation`), `false` for the
    /// derive kinds.
    pub const fn is_attribute(self) -> bool {
        match self {
            ContractKind::Operation => true,
            ContractKind::Error
            | ContractKind::Event
            | ContractKind::VariantInventory
            | ContractKind::Projection
            | ContractKind::Subscription => false,
        }
    }

    /// The snake_case segment used in the frozen `ContractId` formula.
    const fn kind_snake(self) -> &'static str {
        match self {
            ContractKind::Error => "error",
            ContractKind::Event => "event",
            ContractKind::VariantInventory => "variant_inventory",
            ContractKind::Projection => "projection",
            ContractKind::Subscription => "subscription",
            ContractKind::Operation => "operation",
        }
    }
}

impl ContractId {
    /// Build the deterministic, expansion-time **local** identity from the
    /// declaration's kind and ident: `"{kind_snake}/{ident}"`, e.g.
    /// `"event/OrderPlaced"`.
    ///
    /// `concept` is accepted for call-site stability (the pinned interface and
    /// the already-built `ir::build` pass both pass `kind.concept()`), but it is
    /// deliberately NOT folded into the id: `concept` is a pure function of
    /// `kind`, so it adds no locally-resolvable information, and a
    /// concept-prefixed, path-shaped id would fake a global module structure
    /// MacBat cannot resolve (that resolution is crate 3's `ResolvedContractId`).
    pub fn new(_concept: Concept, kind: ContractKind, ident: &syn::Ident) -> Self {
        ContractId(format!("{}/{}", kind.kind_snake(), ident))
    }

    /// The stable local string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Owner {
    /// The symbolic sentinel string for "the caller's module, unknown at
    /// expansion time." Obviously not a real path, so a later reader (or
    /// crate 3's resolver) can tell it apart from a resolved provenance.
    pub const CALLER_MODULE: &'static str = "<caller-module>";

    /// The default, symbolic provenance: the caller's module, which a pure
    /// proc-macro pass cannot resolve. Crate 3 replaces this with a concrete
    /// `ResolvedContractId`-scoped path.
    pub fn caller_module() -> Self {
        Owner(String::from(Self::CALLER_MODULE))
    }

    /// Wrap a concrete provenance fragment (retained for the pinned surface;
    /// prefer [`Owner::caller_module`] at expansion time).
    pub fn new(path: impl Into<String>) -> Self {
        Owner(path.into())
    }

    /// The transparent string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FieldPath {
    /// Wrap an origin blame path such as `"variant.field"`.
    pub fn new(path: impl Into<String>) -> Self {
        FieldPath(path.into())
    }

    /// The transparent string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DiagnosticCode {
    /// Render this code as its stable `"BP-…"` string (master contract §D). The
    /// slug is stable; this string is the tested identity in golden output.
    pub const fn stable_str(&self) -> &'static str {
        match self {
            // BP-SHAPE-
            DiagnosticCode::NotNamedFieldStruct => "BP-SHAPE-0001",
            DiagnosticCode::EventGenericParams => "BP-SHAPE-0002",
            DiagnosticCode::ErrorNotEnum => "BP-SHAPE-0010",
            DiagnosticCode::VariantInventoryNotEnum => "BP-SHAPE-0020",
            DiagnosticCode::VariantInventoryNonUnitVariant => "BP-SHAPE-0021",
            DiagnosticCode::OperationAsync => "BP-SHAPE-0030",
            DiagnosticCode::OperationUnsafe => "BP-SHAPE-0031",
            DiagnosticCode::OperationNonRustAbi => "BP-SHAPE-0032",
            DiagnosticCode::OperationGeneric => "BP-SHAPE-0033",
            DiagnosticCode::OperationArityNotTwo => "BP-SHAPE-0034",
            DiagnosticCode::OperationHasReceiver => "BP-SHAPE-0035",
            // BP-GRAMMAR-
            DiagnosticCode::EventMissingAttr => "BP-GRAMMAR-0001",
            DiagnosticCode::EventMultipleAttrs => "BP-GRAMMAR-0002",
            DiagnosticCode::GrammarUnknownKey => "BP-GRAMMAR-0003",
            DiagnosticCode::GrammarDuplicateKeyInAttr => "BP-GRAMMAR-0004",
            DiagnosticCode::GrammarMetaPathNotIdent => "BP-GRAMMAR-0005",
            DiagnosticCode::EventMissingCategory => "BP-GRAMMAR-0006",
            DiagnosticCode::EventMissingTypeId => "BP-GRAMMAR-0007",
            DiagnosticCode::ErrorVariantMissingErrorAttr => "BP-GRAMMAR-0010",
            DiagnosticCode::ErrorVariantDuplicateErrorAttr => "BP-GRAMMAR-0011",
            DiagnosticCode::ProjectionMissingBatpakAttr => "BP-GRAMMAR-0020",
            DiagnosticCode::ProjectionMissingInput => "BP-GRAMMAR-0021",
            DiagnosticCode::ProjectionNoBindings => "BP-GRAMMAR-0022",
            DiagnosticCode::BindingMissingEvent => "BP-GRAMMAR-0023",
            DiagnosticCode::BindingMissingHandler => "BP-GRAMMAR-0024",
            DiagnosticCode::EmptyBatpakAttr => "BP-GRAMMAR-0025",
            DiagnosticCode::SubscriptionMissingBatpakAttr => "BP-GRAMMAR-0030",
            DiagnosticCode::SubscriptionMissingInput => "BP-GRAMMAR-0031",
            DiagnosticCode::SubscriptionMissingError => "BP-GRAMMAR-0032",
            DiagnosticCode::SubscriptionNoBindings => "BP-GRAMMAR-0033",
            DiagnosticCode::OperationKeyPathNotIdent => "BP-GRAMMAR-0040",
            DiagnosticCode::OperationUnknownKey => "BP-GRAMMAR-0041",
            DiagnosticCode::OperationDuplicateKey => "BP-GRAMMAR-0042",
            DiagnosticCode::OperationMissingRequiredKey => "BP-GRAMMAR-0043",
            // BP-VALUE-
            DiagnosticCode::EventCategoryExceedsU8 => "BP-VALUE-0001",
            DiagnosticCode::EventTypeIdExceedsU16 => "BP-VALUE-0002",
            DiagnosticCode::EventTypeIdExceeds12Bit => "BP-VALUE-0003",
            DiagnosticCode::EventVersionExceedsU16 => "BP-VALUE-0004",
            DiagnosticCode::ErrorFmtUnmatchedBrace => "BP-VALUE-0010",
            DiagnosticCode::ErrorFmtDollarArg => "BP-VALUE-0011",
            DiagnosticCode::ErrorFmtUnterminatedBrace => "BP-VALUE-0012",
            DiagnosticCode::ErrorFmtInvalidPosition => "BP-VALUE-0013",
            DiagnosticCode::OperationIdentValueNotIdent => "BP-VALUE-0040",
            DiagnosticCode::OperationStringValueNotStrlit => "BP-VALUE-0041",
            DiagnosticCode::OperationListNotArray => "BP-VALUE-0042",
            DiagnosticCode::OperationListElementNotStrlit => "BP-VALUE-0043",
            DiagnosticCode::OperationEffectNotIdent => "BP-VALUE-0044",
            // BP-RULE-
            DiagnosticCode::EventCategoryReserved0x0 => "BP-RULE-0001",
            DiagnosticCode::EventCategoryReserved0xD => "BP-RULE-0002",
            DiagnosticCode::EventVersionZeroReserved => "BP-RULE-0003",
            DiagnosticCode::ErrorMultipleSourceFields => "BP-RULE-0010",
            DiagnosticCode::ErrorFromRequiresSingleField => "BP-RULE-0011",
            DiagnosticCode::AttrConfigAndBindingMixed => "BP-RULE-0020",
            DiagnosticCode::ProjectionErrorKeyRejected => "BP-RULE-0021",
            DiagnosticCode::DuplicateConfigKeyAcrossAttrs => "BP-RULE-0022",
            DiagnosticCode::ProjectionStateMaxNotOne => "BP-RULE-0023",
            DiagnosticCode::EventPathNotSingleSegment => "BP-RULE-0024",
            DiagnosticCode::DuplicateEventBinding => "BP-RULE-0025",
            DiagnosticCode::SubscriptionCacheVersionRejected => "BP-RULE-0030",
            DiagnosticCode::SubscriptionStateMaxRejected => "BP-RULE-0031",
            DiagnosticCode::OperationUnsupportedEffect => "BP-RULE-0040",
            // BP-BUDGET-
            DiagnosticCode::BudgetMaxFields => "BP-BUDGET-0001",
            DiagnosticCode::BudgetMaxVariants => "BP-BUDGET-0002",
            DiagnosticCode::BudgetMaxCompositionDepth => "BP-BUDGET-0003",
            DiagnosticCode::BudgetMaxGeneratedItems => "BP-BUDGET-0004",
            DiagnosticCode::BudgetMaxTokens => "BP-BUDGET-0005",
            DiagnosticCode::BudgetMaxTraceRecords => "BP-BUDGET-0006",
        }
    }

    /// Alias of [`DiagnosticCode::stable_str`], kept because the snapshot harness
    /// reads codes via `.code.as_str()`.
    pub const fn as_str(&self) -> &'static str {
        self.stable_str()
    }
}

impl RuleId {
    /// Wrap a rule-id literal (registry/grammar const data is the caller).
    pub const fn new(id: &'static str) -> Self {
        RuleId(id)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl ExplanationId {
    /// Wrap an explanation-id literal (registry const data is the caller).
    pub const fn new(id: &'static str) -> Self {
        ExplanationId(id)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl KeyName {
    /// Wrap a grammar-declared key name (the grammar catalog is the caller).
    pub const fn new(name: &'static str) -> Self {
        KeyName(name)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl InvariantId {
    /// Wrap an invariant-id literal.
    pub const fn new(id: &'static str) -> Self {
        InvariantId(id)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl WitnessId {
    /// Wrap a witness-id literal.
    pub const fn new(id: &'static str) -> Self {
        WitnessId(id)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl ObservationId {
    /// Wrap an observation-id literal.
    pub const fn new(id: &'static str) -> Self {
        ObservationId(id)
    }

    /// The transparent string form.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl EmittedItemId {
    /// Wrap a dense, source-order item index (`emit` is the caller).
    pub const fn new(id: u32) -> Self {
        EmittedItemId(id)
    }

    /// The dense index value.
    pub const fn get(&self) -> u32 {
        self.0
    }
}

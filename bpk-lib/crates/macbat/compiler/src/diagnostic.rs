//! Diagnostic model for the MacBat compiler.
//!
//! Every failure the semantic compiler reports is a [`MacroDiagnostic`]: a stable
//! [`DiagnosticCode`], the BatPak-owned exact message text (the tested contract),
//! and enough machine-actionable structure ([`RequiredAction`], [`Suggestion`]s,
//! related spans) for a tool to guide a fix. Diagnostics are surfaced to `rustc`
//! two ways (constitution §5): as a spanned [`syn::Error`] (Class 1, byte-exact
//! trybuild goldens) or as a `compile_error!` token stream carrying the message
//! (Class 2). `proc_macro::Diagnostic` is nightly on the 1.97 lane and is never
//! used.
//!
//! The immutable per-code facts (severity, rule, explanation, action shape) live
//! as `const` data in [`registry`]; a diagnostic is stamped from that registry by
//! [`MacroDiagnostic::from_spec`] and then enriched with call-site payload by the
//! pass that raised it.

pub mod registry;

use proc_macro2::Span;

use crate::identity::{
    Concept, ContractId, DiagnosticCode, ExplanationId, KeyName, LoweringRole, RuleId, ShapeRule,
};

/// Whether a diagnostic aborts the expansion or is an advisory note.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Severity {
    Error,
    Warning,
}

/// A note attached to a secondary span, explaining its role in the failure.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RelatedNote(pub String);

/// A ranked replacement candidate offered to the author (e.g. the closest known
/// key for an unknown one). Lower `rank` sorts first.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Suggestion {
    pub replacement: String,
    pub rank: u8,
}

/// The value shape a key or position was expected to take, rendered into the
/// human message and the machine-actionable [`RequiredAction`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExpectedForm {
    Ident,
    SinglePath,
    IntLit { bits: u8 },
    StrLit,
    StrList,
    EffectClass,
}

/// The semantic fix a diagnostic prescribes. Per GATE-1 ruling 6 these are
/// SEMANTIC-ONLY: no repository file path is ever carried here. Muterprater
/// (crate 3) later enriches [`RequiredAction::ProvideLowering`] into a concrete
/// target file plus a `materialize` command; there is deliberately no
/// `Materialize { file }` variant in crate 1.
#[derive(Clone, PartialEq, Eq)]
pub enum RequiredAction {
    AddKey { key: KeyName, form: ExpectedForm },
    RemoveKey { key: KeyName },
    ReplaceValue { key: KeyName, form: ExpectedForm },
    ChangeShape { expected: ShapeRule },
    ProvideLowering { role: LoweringRole },
    HandWrite { reason: &'static str },
}

/// A single diagnostic. The `message` is the BatPak-owned exact text and is the
/// tested contract; the remaining fields are the machine-readable structure.
#[derive(Clone)]
pub struct MacroDiagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub concept: Concept,
    pub contract: Option<ContractId>,
    pub primary: Span,
    pub related: Vec<(Span, RelatedNote)>,
    pub violated_rule: RuleId,
    pub required_action: RequiredAction,
    pub suggestions: Vec<Suggestion>,
    pub explanation: ExplanationId,
    pub message: String,
}

impl MacroDiagnostic {
    /// Class 1 (constitution §5): render as a spanned [`syn::Error`] carrying the
    /// exact BatPak-owned text. Related notes are combined as additional spanned
    /// errors so each points at its own source location.
    #[must_use]
    pub fn into_syn_error(&self) -> syn::Error {
        let mut error = syn::Error::new(self.primary, self.message.clone());
        for (span, note) in &self.related {
            error.combine(syn::Error::new(*span, note.0.clone()));
        }
        error
    }

    /// Class 2 (constitution §5): render as a `compile_error!` token stream. Pinned
    /// to the 1.97 lane; the message is the same tested contract as `into_syn_error`.
    #[must_use]
    pub fn into_compile_error(&self) -> proc_macro2::TokenStream {
        self.into_syn_error().to_compile_error()
    }

    /// Stamp a diagnostic from the const registry by its code, taking the
    /// contextual `required_action` from the failure site, plus the call-site
    /// `primary` span and BatPak-owned `message`. The registry supplies the stable
    /// metadata (severity, concept, violated rule, explanation); the payload-bearing
    /// action can only come from the site that has the key name / expected form /
    /// shape in hand, so it is a parameter, never manufactured here. A raising pass
    /// may still refine `concept`, `contract`, `related`, and `suggestions` via the
    /// public fields.
    #[must_use]
    pub fn from_spec(
        code: DiagnosticCode,
        primary: Span,
        required_action: RequiredAction,
        message: String,
    ) -> Self {
        let entry = registry::spec(code);
        MacroDiagnostic {
            code,
            severity: entry.severity,
            concept: entry.concept,
            contract: None,
            primary,
            related: Vec::new(),
            violated_rule: entry.rule,
            required_action,
            suggestions: Vec::new(),
            explanation: entry.explanation,
            message,
        }
    }
}

/// The diagnostic accumulator threaded through the fallible passes. A pass that
/// fails returns `Err(Diagnostics)`; a pass may collect several before bailing.
#[derive(Clone, Default)]
pub struct Diagnostics(Vec<MacroDiagnostic>);

impl Diagnostics {
    /// A single-diagnostic accumulator.
    #[must_use]
    pub fn one(d: MacroDiagnostic) -> Self {
        Diagnostics(vec![d])
    }

    /// Append a diagnostic.
    pub fn push(&mut self, d: MacroDiagnostic) {
        self.0.push(d);
    }

    /// Whether no diagnostics have been collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume the accumulator into its ordered diagnostics.
    #[must_use]
    pub fn into_vec(self) -> Vec<MacroDiagnostic> {
        self.0
    }
}

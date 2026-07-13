//! Declared syntax classes: the attribute grammar is *data*, and every
//! downstream behaviour — the closed-vocabulary parser, the expected-form
//! diagnostic messages, the ranked-alternative suggestions, and the rustdoc —
//! derives from these tables rather than from hand-rolled per-kind parsers.
//!
//! A grammar is a pure, `'static` description of one contract kind: which keys
//! are legal, each key's arity and value shape, the required item shape, and
//! (for projection/subscription) the paired `event`/`handler` binding keys.
//! The six concrete grammars live in [`catalog`]; the cross-phase event-category
//! rule (a checked-in generated projection of core's rule) lives in [`kind_rule`].
//!
//! Purity: this module performs no I/O, no clock/env reads, and uses only
//! deterministic collections — the ranked-alternative ranking is a pure
//! edit-distance sort so identical inputs yield identical suggestions.

mod catalog;
pub mod kind_rule;

pub use catalog::{ERROR, EVENT, OPERATION, PROJECTION, SUBSCRIPTION, VARIANT_INVENTORY};

use crate::diagnostic::{ExpectedForm, Suggestion};
use crate::identity::{ContractKind, KeyName, ShapeRule};

/// The complete declared grammar for one contract kind.
///
/// Constructed once, as a `'static` const, in [`catalog`]; every parser and
/// diagnostic reads its tables rather than re-encoding the rules.
pub struct AttributeGrammar {
    /// The contract kind this grammar governs.
    pub kind: ContractKind,
    /// The closed set of legal keys. A key not present here is an unknown key
    /// and is answered with ranked alternatives (see [`rank_alternatives`]).
    pub keys: &'static [KeySpec],
    /// The required declared item shape (struct / enum / free fn).
    pub shape_rule: ShapeRule,
    /// The paired binding keys, `Some` only for projection and subscription.
    pub binding_keys: Option<BindingKeySpec>,
    /// The single documentation source for this grammar.
    pub docs: GrammarDocs,
}

/// One legal key within an [`AttributeGrammar`].
pub struct KeySpec {
    /// The grammar-declared key name.
    pub name: KeyName,
    /// How many times the key may appear.
    pub arity: Arity,
    /// The shape a value bound to this key must take.
    pub value: ValueShape,
    /// Whether the declaration is rejected when the key is absent.
    pub required: bool,
    /// Human-readable help, surfaced in diagnostics and rustdoc.
    pub help: &'static str,
    /// Key names that may not co-occur with this key. Config keys name the
    /// binding keys here; the reverse direction is enforced structurally when
    /// the declaration is classified into config vs. binding groups.
    pub incompatible_with: &'static [&'static str],
}

/// How many times a key may appear across a declaration.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arity {
    /// Present exactly once (the required-scalar case).
    ExactlyOnce,
    /// Present zero or one time (the optional-scalar case).
    AtMostOnce,
    /// Present one or more times (the repeated-binding case).
    OneOrMore,
}

/// The shape a key's bound value must take.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueShape {
    /// A single bare identifier.
    Ident,
    /// A path; `single_segment` requires exactly one segment.
    Path {
        /// Whether the path must be a single segment (no `::`).
        single_segment: bool,
    },
    /// An integer literal that must fit in `bits` bits.
    IntLit {
        /// The declared bit width the literal must fit within.
        bits: u8,
    },
    /// A string literal.
    StrLit,
    /// An array of string literals.
    StrList,
    /// One of the closed `EffectClass` identifiers.
    EffectClass,
}

/// The paired binding keys for kinds that carry repeated `(event, handler)`
/// rows. `event` is the first field, `handler` the second.
pub struct BindingKeySpec {
    /// The key naming the event type in a binding row.
    pub event: KeyName,
    /// The key naming the handler in a binding row.
    pub handler: KeyName,
}

/// The single documentation source derived into rustdoc and messages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GrammarDocs {
    /// A one-line summary of the grammar.
    pub summary: &'static str,
}

/// Return the `'static` grammar for a contract kind.
///
/// The match is exhaustive over all six v1 kinds; adding a seventh kind forces
/// a new arm here (no wildcard), which is the mechanical extension contract.
pub fn grammar_for(kind: ContractKind) -> &'static AttributeGrammar {
    match kind {
        ContractKind::Error => &ERROR,
        ContractKind::Event => &EVENT,
        ContractKind::VariantInventory => &VARIANT_INVENTORY,
        ContractKind::Projection => &PROJECTION,
        ContractKind::Subscription => &SUBSCRIPTION,
        ContractKind::Operation => &OPERATION,
    }
}

/// Render a value shape into the diagnostic-facing [`ExpectedForm`].
///
/// This is the one renderer behind every "expected …" message, so the wording
/// stays consistent across kinds. Both path shapes render as
/// [`ExpectedForm::SinglePath`] — the diagnostic vocabulary has a single path
/// form — while the `single_segment` requirement is enforced separately by the
/// value-domain rules, not by the rendered form.
pub fn expected_form(shape: &ValueShape) -> ExpectedForm {
    match *shape {
        ValueShape::Ident => ExpectedForm::Ident,
        ValueShape::Path { .. } => ExpectedForm::SinglePath,
        ValueShape::IntLit { bits } => ExpectedForm::IntLit { bits },
        ValueShape::StrLit => ExpectedForm::StrLit,
        ValueShape::StrList => ExpectedForm::StrList,
        ValueShape::EffectClass => ExpectedForm::EffectClass,
    }
}

/// Rank the grammar's legal keys as alternatives to an `unknown` key.
///
/// Keys are ordered by ascending edit distance to `unknown` (ties broken by
/// key name, so the ordering is total and deterministic), then the closest
/// `cap` are returned with dense ranks `0, 1, 2, …`. A `cap` of zero yields no
/// suggestions.
pub fn rank_alternatives(unknown: &str, keys: &[KeySpec], cap: u8) -> Vec<Suggestion> {
    let limit = usize::from(cap);
    if limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &str)> = keys
        .iter()
        .map(|spec| {
            let name = spec.name.as_str();
            (edit_distance(unknown, name), name)
        })
        .collect();
    scored.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)));

    let mut suggestions: Vec<Suggestion> = Vec::new();
    let mut rank: u8 = 0;
    for (_, name) in scored.into_iter().take(limit) {
        suggestions.push(Suggestion {
            replacement: name.to_owned(),
            rank,
        });
        // Ranks stay below `cap` (<= 255); the saturating step is a total guard.
        rank = rank.saturating_add(1);
    }
    suggestions
}

/// Levenshtein edit distance between two strings.
///
/// Two rolling rows of a dynamic-programming table (no `HashMap`, no
/// allocation beyond the two `Vec`s), so the result is pure and deterministic.
fn edit_distance(source: &str, target: &str) -> usize {
    let source: Vec<char> = source.chars().collect();
    let target: Vec<char> = target.chars().collect();
    let width = target.len() + 1;

    let mut prev: Vec<usize> = (0..width).collect();
    let mut curr: Vec<usize> = vec![0; width];

    for (i, source_char) in source.iter().enumerate() {
        curr[0] = i + 1;
        for (j, target_char) in target.iter().enumerate() {
            let substitution_cost = if source_char == target_char { 0 } else { 1 };
            let deletion = prev[j + 1] + 1;
            let insertion = curr[j] + 1;
            let substitution = prev[j] + substitution_cost;
            curr[j + 1] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[target.len()]
}

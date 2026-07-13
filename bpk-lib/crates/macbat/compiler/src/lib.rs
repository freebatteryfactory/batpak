//! MacBat contract compiler — the pure semantic core of the macro lane.
//!
//! `macbat-compiler` turns a contract declaration (a `#[derive(...)]` input or
//! an `#[operation]` attribute item) into either emitted tokens or a set of
//! diagnostics, as eight typed passes:
//!
//! `tokens → declaration → validation → ir → lowering → emit → normalize →
//! tokenize`
//!
//! with `identity / diagnostic / budget / metrics / trace / origin / grammar /
//! hygiene / artifact` cross-cutting. [`compile_derive`] and
//! [`compile_attribute`] are the only entry points; each runs passes 1..8 and
//! folds any `Err(Diagnostics)` into the returned [`ExpansionArtifact`].
//!
//! ## Purity
//!
//! Every file under `src/` is a pure function: no `std::fs`/`std::env`/
//! `std::time`/`std::net`, no `rand`, no `SystemTime`, no thread-locals, and no
//! `HashMap` in any emit/normalize path (`Vec`/`BTreeMap`/`BTreeSet` only, for
//! deterministic output). Disk lives in `tests/`, `examples/`, and `specimens/`.
//! This crate never imports `batpak` core.
//!
//! ## Lowering extension contract
//!
//! Adding a contract kind later (in that kind's own crate's packet — never
//! here) is a *pure addition*: add a [`identity::ContractKind`] variant, a
//! `ir::ContractKindIr` variant + `ir/<kind>.rs`, a `lowering/<kind>.rs` + a new
//! arm in `lowering::plan`'s exhaustive match + any new
//! [`identity::LoweringRole`] variants, one `grammar::AttributeGrammar` const,
//! and a specimen directory. No existing arm changes semantics; the repo-wide
//! ban on wildcard/catch-all enum arms means a new kind mechanically *forces* a
//! new `plan` arm or the build fails. The other reserved kinds (`Effect`,
//! `StateMachine`, `Evidence`, `Codec`, `IntegrityDomain`, `PlatformCapability`,
//! `DependencyFacade`, `Module`) live only as this documented list — zero
//! speculative structs, zero empty drawers — until their owning packet lands.

// Sanctioned test-only bail: `.expect("…")` is denied in production `src/` (cure
// type-first) but permitted in tests, exactly as the parent workspace pins it.
#![cfg_attr(not(test), deny(clippy::expect_used))]

pub mod artifact;
pub mod budget;
pub mod declaration;
pub mod diagnostic;
pub mod emit;
pub mod grammar;
pub mod hygiene;
pub mod identity;
pub mod ir;
pub mod lowering;
pub mod metrics;
pub mod normalize;
pub mod origin;
pub mod tokenize;
pub mod tokens;
pub mod trace;
pub mod validation;

pub use crate::artifact::{ExpandOptions, ExpansionArtifact, ExpansionOutcome};
pub use crate::identity::{ContractKind, ShapeRule};
pub use crate::normalize::ContractSnapshot;

use crate::budget::ExpansionBudgets;
use crate::diagnostic::Diagnostics;
use crate::hygiene::{CratePath, HygieneContext, LoweringContext};
use crate::ir::ContractIr;
use crate::metrics::ExpansionMetrics;
use crate::origin::OriginMap;
use crate::tokens::SyntaxTree;
use crate::trace::ExpansionTrace;

/// Run the derive pipeline for `kind` over `item` and return the finished
/// artifact.
///
/// Pure: tokens in, artifact out — no `&self`, no I/O handle. On any pass
/// failure the artifact's outcome is [`ExpansionOutcome::Rejected`] and no
/// tokens are produced.
///
/// `options.trace` is accepted for interface stability but is not yet threaded:
/// in v1 no pass returns `TraceRecord`s through its pinned signature, so the
/// trace is always empty (see the FLAG in the crate's lane notes).
pub fn compile_derive(
    kind: ContractKind,
    item: proc_macro2::TokenStream,
    _options: ExpandOptions,
) -> ExpansionArtifact {
    match crate::tokens::parse_derive(kind, item) {
        Ok(tree) => run(tree),
        Err(diagnostics) => rejected(None, diagnostics),
    }
}

/// Run the attribute pipeline for `kind` over `(attr, item)` and return the
/// finished artifact. Same purity and failure semantics as [`compile_derive`].
pub fn compile_attribute(
    kind: ContractKind,
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
    _options: ExpandOptions,
) -> ExpansionArtifact {
    match crate::tokens::parse_attribute(kind, attr, item) {
        Ok(tree) => run(tree),
        Err(diagnostics) => rejected(None, diagnostics),
    }
}

/// Passes 2..8 over a parsed [`SyntaxTree`]. Kept private: the crate's contract
/// with the outside world is exactly the two entry points above.
fn run(tree: SyntaxTree) -> ExpansionArtifact {
    let kind = tree.kind;
    let grammar = crate::grammar::grammar_for(kind);
    let budgets = ExpansionBudgets::DEFAULT;

    // Passes 2..4: declaration → validation → IR. No contract identity exists
    // yet, so a failure here rejects with `partial_contract: None`.
    let declaration = match crate::declaration::parse_declaration(tree, grammar) {
        Ok(declaration) => declaration,
        Err(diagnostics) => return rejected(None, diagnostics),
    };
    let validated = match crate::validation::validate(declaration, grammar, &budgets) {
        Ok(validated) => validated,
        Err(diagnostics) => return rejected(None, diagnostics),
    };
    let ir = match crate::ir::build(&validated) {
        Ok(ir) => ir,
        Err(diagnostics) => return rejected(None, diagnostics),
    };

    // Passes 5..8: the contract IR now exists, so a later failure carries it as
    // `partial_contract: Some(ir)`. Hygiene is built per contract here; the
    // borrow of `ir` is scoped so `ir` is free to move afterwards.
    let plan = {
        let crate_path = if matches!(kind, ContractKind::Operation) {
            CratePath::runtime()
        } else {
            CratePath::batpak()
        };
        let hygiene = HygieneContext { crate_path };
        let context = LoweringContext {
            hygiene: &hygiene,
            contract: &ir.id,
        };
        crate::lowering::plan(&ir, &context)
    };
    let plan = match plan {
        Ok(plan) => plan,
        Err(diagnostics) => return rejected(Some(ir), diagnostics),
    };
    let (model, origin_map) = match crate::emit::finalize(plan, &budgets) {
        Ok(finished) => finished,
        Err(diagnostics) => return rejected(Some(ir), diagnostics),
    };
    let normalized = crate::normalize::canonicalize(&model);
    let tokens = crate::tokenize::to_tokens(&model);

    let metrics = ExpansionMetrics {
        keys_parsed: 0,
        variants: 0,
        emitted_items: count_u32(model.items.len()),
        tokens: count_u32(tokens.clone().into_iter().count()),
        diagnostics: 0,
    };
    ExpansionArtifact {
        outcome: ExpansionOutcome::Emitted {
            contract: ir,
            normalized,
            tokens,
        },
        origin_map,
        trace: ExpansionTrace::default(),
        metrics,
    }
}

/// Assemble a rejected artifact: no tokens, the accumulated diagnostics, and
/// whatever contract identity had been established (`None` before the IR pass).
fn rejected(partial_contract: Option<ContractIr>, diagnostics: Diagnostics) -> ExpansionArtifact {
    let diagnostics = diagnostics.into_vec();
    let metrics = ExpansionMetrics {
        keys_parsed: 0,
        variants: 0,
        emitted_items: 0,
        tokens: 0,
        diagnostics: count_u32(diagnostics.len()),
    };
    ExpansionArtifact {
        outcome: ExpansionOutcome::Rejected {
            partial_contract,
            diagnostics,
        },
        origin_map: OriginMap::default(),
        trace: ExpansionTrace::default(),
        metrics,
    }
}

/// Total, lint-clean `usize → u32` for counters: a macro expansion never
/// approaches `u32::MAX` items/tokens, so saturating is honest and avoids the
/// denied `unwrap`/`expect`/`as`-cast paths (the `Err` arm binds the error
/// payload — it is not a catch-all enum arm).
fn count_u32(len: usize) -> u32 {
    match u32::try_from(len) {
        Ok(count) => count,
        Err(_) => u32::MAX,
    }
}

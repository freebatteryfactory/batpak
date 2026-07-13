//! Expansion-budget enforcement proof (Cluster 14).
//!
//! Each test drives an oversized specimen through the pure compiler and asserts
//! the EXACT `BP-BUDGET-*` diagnostic code via `matches!` on the rejected code
//! string. A budget is a fail-closed ceiling, never a hang.
//!
//! COVERAGE GAP (flagged for the orchestrator / Cluster 2): packet 04 does not
//! disclose the numeric `ExpansionBudgets::DEFAULT` constants (they are the
//! unshown "§F values", owned by Cluster 2). Only the two INPUT-shaped budgets
//! whose trigger is a monotone count — `max_fields` (BP-BUDGET-0001) and
//! `max_variants` (BP-BUDGET-0002) — can be provoked deterministically from a
//! single large declaration, and only under the (documented) assumption that
//! each default ceiling is below `OVERSIZE`. The remaining four codes cannot be
//! authored blind and are intentionally NOT shipped as guessed/ignored tests:
//!   * BP-BUDGET-0003 (max-composition-depth): needs a defined notion of v1
//!     "composition depth" — the reserved invariant/witness composition is empty
//!     in v1, so it may be unreachable by any v1 declaration.
//!   * BP-BUDGET-0004 (max-generated-items) / 0005 (max-tokens): OUTPUT budgets;
//!     an input large enough to breach them trips an INPUT budget (fields/variants)
//!     FIRST unless the exact default ordering is known.
//!   * BP-BUDGET-0006 (max-trace-records): a NON-FATAL truncation marker under
//!     `options.trace`; the restructured `ExpansionOutcome::Emitted` carries no
//!     diagnostics slot, so its surface is undefined here.
//! Provide the §F ceilings (and the 0006 warning channel) to close these.

use macbat_compiler::{ContractKind, ExpandOptions, ExpansionOutcome, compile_derive};

/// A count comfortably above any plausible per-declaration budget ceiling.
const OVERSIZE: usize = 4096;

/// Run a derive over `src` and return the rejected diagnostic codes (empty if the
/// input was, unexpectedly, accepted).
fn reject_codes(kind: ContractKind, src: &str) -> Vec<String> {
    let item: proc_macro2::TokenStream =
        src.parse().expect("budget specimen must lex as a token stream");
    let artifact = compile_derive(kind, item, ExpandOptions::DEFAULT);
    match artifact.outcome {
        ExpansionOutcome::Rejected { diagnostics, .. } => {
            diagnostics.iter().map(|d| d.code.as_str().to_owned()).collect()
        }
        ExpansionOutcome::Emitted { .. } => Vec::new(),
    }
}

/// A named-field struct (Event shape) with `n` fields and an otherwise-valid
/// `#[batpak(..)]` header, so ONLY the field-count budget can fire.
fn many_field_struct(n: usize) -> String {
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!("    f{i}: u8,\n"));
    }
    format!("#[batpak(category = 1, type_id = 1)]\nstruct Oversized {{\n{body}}}\n")
}

/// A unit-only enum (VariantInventory shape) with `n` variants, so ONLY the
/// variant-count budget can fire.
fn many_variant_enum(n: usize) -> String {
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!("    V{i},\n"));
    }
    format!("enum Oversized {{\n{body}}}\n")
}

#[test]
fn max_fields_budget_is_enforced() {
    let codes = reject_codes(ContractKind::Event, &many_field_struct(OVERSIZE));
    assert!(
        codes.iter().any(|c| matches!(c.as_str(), "BP-BUDGET-0001")),
        "expected BP-BUDGET-0001 (max-fields) for a {OVERSIZE}-field struct, got {codes:?}"
    );
}

#[test]
fn max_variants_budget_is_enforced() {
    let codes = reject_codes(ContractKind::VariantInventory, &many_variant_enum(OVERSIZE));
    assert!(
        codes.iter().any(|c| matches!(c.as_str(), "BP-BUDGET-0002")),
        "expected BP-BUDGET-0002 (max-variants) for a {OVERSIZE}-variant enum, got {codes:?}"
    );
}

//! Expansion budgets: the fail-fast ceilings that keep a single macro invocation
//! from generating an unbounded amount of code (or an unbounded trace). A crossed
//! ceiling is a typed [`BudgetViolation`] that lowers to a `BP-BUDGET-…`
//! [`MacroDiagnostic`] — never a hang. Input-shaped budgets (`max_fields`,
//! `max_variants`, `max_composition_depth`) are checked in Pass 3; output-shaped
//! budgets (`max_generated_items`, `max_tokens`) in Pass 6; `max_trace_records`
//! bounds the optional trace with a non-fatal truncation marker.

use proc_macro2::Span;

use crate::diagnostic::{MacroDiagnostic, RequiredAction};
use crate::identity::DiagnosticCode;

/// The full set of expansion ceilings, passed to the passes that enforce them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExpansionBudgets {
    pub max_fields: u32,
    pub max_variants: u32,
    pub max_composition_depth: u8,
    pub max_generated_items: u32,
    pub max_tokens: u32,
    pub max_suggestions: u8,
    pub max_trace_records: u16,
}

impl ExpansionBudgets {
    /// The default ceilings (master contract §F / packet 02 §2f).
    pub const DEFAULT: Self = ExpansionBudgets {
        max_fields: 128,
        max_variants: 256,
        max_composition_depth: 8,
        max_generated_items: 64,
        max_tokens: 65_536,
        max_suggestions: 5,
        max_trace_records: 512,
    };
}

/// Which ceiling was crossed. There is no `Suggestions` kind: `max_suggestions` is
/// an internal ranked-truncation cap, not a diagnosable violation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BudgetKind {
    Fields,
    Variants,
    CompositionDepth,
    GeneratedItems,
    Tokens,
    TraceRecords,
}

/// A crossed ceiling: which budget, its limit, the observed count, and the span to
/// blame. Widened to `u64` so any counter (field/variant counts up through token
/// estimates) fits without a lossy cast.
#[derive(Clone)]
pub struct BudgetViolation {
    pub budget: BudgetKind,
    pub limit: u64,
    pub observed: u64,
    pub at: Span,
}

impl BudgetViolation {
    /// Lower to the `BP-BUDGET-…` diagnostic for this budget. The prescribed action
    /// is always to split or hand-write the declaration, carrying a budget-specific
    /// reason.
    #[must_use]
    pub fn into_diagnostic(self) -> MacroDiagnostic {
        let action = RequiredAction::HandWrite {
            reason: budget_reason(self.budget),
        };
        let message = format!(
            "expansion exceeds the {} budget (limit {}, observed {})",
            budget_label(self.budget),
            self.limit,
            self.observed
        );
        MacroDiagnostic::from_spec(budget_code(self.budget), self.at, action, message)
    }
}

/// The diagnostic code for a crossed budget.
fn budget_code(kind: BudgetKind) -> DiagnosticCode {
    match kind {
        BudgetKind::Fields => DiagnosticCode::BudgetMaxFields,
        BudgetKind::Variants => DiagnosticCode::BudgetMaxVariants,
        BudgetKind::CompositionDepth => DiagnosticCode::BudgetMaxCompositionDepth,
        BudgetKind::GeneratedItems => DiagnosticCode::BudgetMaxGeneratedItems,
        BudgetKind::Tokens => DiagnosticCode::BudgetMaxTokens,
        BudgetKind::TraceRecords => DiagnosticCode::BudgetMaxTraceRecords,
    }
}

/// The human label for a budget, used in the diagnostic message.
fn budget_label(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Fields => "field count",
        BudgetKind::Variants => "variant count",
        BudgetKind::CompositionDepth => "composition depth",
        BudgetKind::GeneratedItems => "generated item count",
        BudgetKind::Tokens => "emitted token count",
        BudgetKind::TraceRecords => "trace record count",
    }
}

/// The prescribed hand-write reason for a crossed budget.
fn budget_reason(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Fields => "split the declaration or hand-write the item",
        BudgetKind::Variants => "split the enum into sub-enums",
        BudgetKind::CompositionDepth => "flatten the composition",
        BudgetKind::GeneratedItems => "reduce the lowerings or hand-write the item",
        BudgetKind::Tokens => "split the declaration",
        BudgetKind::TraceRecords => "trace truncated at the limit",
    }
}

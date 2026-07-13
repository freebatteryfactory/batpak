//! Pass 6 — emit: finalize a [`LoweringPlan`] into the deterministic
//! [`EmittedItemModel`] plus its [`OriginMap`].
//!
//! This pass is the authority on [`EmittedItemId`]: surface lowerings stamp
//! placeholder ids in local source order (`EmittedItemId::new(n)`), and
//! `finalize` re-densifies them to their final, globally dense, source-ordered
//! ids (B.8). It also enforces the OUTPUT budgets (`max_generated_items` and the
//! `max_tokens` estimate) and assembles the origin/blame map. Kind-agnostic.
//!
//! Purity: `Vec` only, no `HashMap`, no clocks/fs/env. Width conversions are
//! total (`try_from` with an explicit saturating fallback via `match`).

pub mod item_model;

pub use item_model::{EmittedItem, EmittedItemModel, ItemNode};

use crate::budget::{BudgetKind, BudgetViolation, ExpansionBudgets};
use crate::diagnostic::Diagnostics;
use crate::identity::EmittedItemId;
use crate::lowering::LoweringPlan;
use crate::origin::OriginMap;
use proc_macro2::Span;

/// Finalize a lowering plan: re-densify emitted-item ids, enforce output
/// budgets, and assemble the origin map.
///
/// # Errors
///
/// Returns a [`Diagnostics`] carrying a `BP-BUDGET-*` diagnostic when the plan
/// exceeds `max_generated_items` (item count) or the estimated `max_tokens`
/// output budget. Never hangs — a budget breach is always a typed diagnostic.
pub fn finalize(
    plan: LoweringPlan,
    budgets: &ExpansionBudgets,
) -> Result<(EmittedItemModel, OriginMap), Diagnostics> {
    // A representative span for any budget diagnostic: the first item's user
    // span (declaration site) if present, else the macro call site. The
    // fallback is unreachable on the error paths below (an empty model can
    // exceed no budget), but `map_or_else` requires it.
    let site = plan
        .model
        .items
        .first()
        .map_or_else(Span::call_site, |item| item.origin.user_span);

    let observed_items = widen(plan.model.items.len());
    let item_limit = u64::from(budgets.max_generated_items);
    if observed_items > item_limit {
        return Err(budget_diagnostic(
            BudgetKind::GeneratedItems,
            item_limit,
            observed_items,
            site,
        ));
    }

    let contract = plan.model.contract;
    let source_items = plan.model.items;
    let mut items = Vec::with_capacity(source_items.len());
    let mut origin_map = OriginMap::default();
    let mut token_total: u64 = 0;
    let mut next_index: u32 = 0;

    for item in source_items {
        let id = EmittedItemId::new(next_index);
        // Guaranteed within budget (item count already checked), so no
        // saturation occurs; saturating_add keeps this total and panic-free.
        next_index = next_index.saturating_add(1);
        token_total = token_total.saturating_add(widen(item.node.estimated_token_count()));

        let EmittedItem {
            id: _,
            role,
            sort_key,
            node,
            mut origin,
        } = item;
        origin.emitted = id.clone();
        origin_map.push(origin.clone());
        items.push(EmittedItem {
            id,
            role,
            sort_key,
            node,
            origin,
        });
    }

    let token_limit = u64::from(budgets.max_tokens);
    if token_total > token_limit {
        return Err(budget_diagnostic(
            BudgetKind::Tokens,
            token_limit,
            token_total,
            site,
        ));
    }

    let model = EmittedItemModel { contract, items };
    Ok((model, origin_map))
}

/// Widen a `usize` count to `u64` totally. On any supported target
/// (`target_pointer_width <= 64`) the conversion always succeeds; the saturating
/// `u64::MAX` arm keeps this panic-free without `unwrap`/`expect`.
fn widen(value: usize) -> u64 {
    match u64::try_from(value) {
        Ok(widened) => widened,
        Err(_) => u64::MAX,
    }
}

/// Build a single-diagnostic [`Diagnostics`] from a budget breach.
fn budget_diagnostic(budget: BudgetKind, limit: u64, observed: u64, at: Span) -> Diagnostics {
    Diagnostics::one(
        BudgetViolation {
            budget,
            limit,
            observed,
            at,
        }
        .into_diagnostic(),
    )
}

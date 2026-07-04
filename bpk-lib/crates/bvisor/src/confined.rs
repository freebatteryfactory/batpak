//! One-call confined execution: [`run_confined`].
//!
//! Callers otherwise wire the [`BackendRegistry`] → [`BoundaryPlanner`] →
//! [`BoundaryRunner`] pipeline by hand for the common single-backend case. This
//! collapses that boilerplate into one call while keeping the same fail-closed
//! semantics — no new policy, no hidden defaults.

use crate::contract::backend::Backend;
use crate::contract::plan::{BoundarySpec, PlanError};
use crate::contract::registry::{BackendRegistry, BoundaryPlanner, BoundaryRunner};
use crate::contract::report::BoundaryReport;
use std::sync::Arc;

/// Plan `spec` against `backend`, execute it, and return the sealed
/// [`BoundaryReport`] — the one-call form of the `registry → plan → run`
/// pipeline.
///
/// The backend is registered into a fresh single-backend [`BackendRegistry`],
/// the spec is admitted by a [`BoundaryPlanner`] (fail-closed), and the plan is
/// driven + sealed by a [`BoundaryRunner`]. Controlled *execution* terminals
/// (unsupported cell, guest trap, timeout, fail-closed setup) are not errors:
/// they are encoded in the returned report's outcome, exactly as when the
/// pipeline is driven by hand.
///
/// For multi-backend selection or reuse of a registry across many runs, use the
/// [`BackendRegistry`] / [`BoundaryPlanner`] / [`BoundaryRunner`] pipeline
/// directly — this wrapper deliberately owns a single-backend registry per call.
///
/// # Errors
///
/// Returns [`PlanError`] when `spec` cannot be admitted against `backend` — an
/// unknown backend id, an unsupported required capability or host control, an
/// unsatisfiable evidence/budget floor, or an admission-shadow divergence.
/// Execution terminals are *not* errors; they ride the returned report's outcome.
pub fn run_confined(
    spec: &BoundarySpec,
    backend: Arc<dyn Backend>,
) -> Result<BoundaryReport, PlanError> {
    let id = backend.id();
    let mut registry = BackendRegistry::new();
    registry.register(backend);
    let plan = BoundaryPlanner::new(&registry).plan(spec, &id)?;
    BoundaryRunner::new(&registry).run(&plan)
}

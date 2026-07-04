//! Wasm runner-observation → report-body mapping.
//!
//! Denials are not self-reported here: filesystem and socket denial proofs live in
//! the independent grid oracles. The report records mechanism facts, terminal state,
//! captured stream refs, and only the budget dimensions actually observed by the runner.

use super::run::{WasmRunObservation, WasmTerminal};
use super::WasmBackend;
use crate::contract::backend::Backend;
use crate::contract::budget_witness::{BudgetFinding, BudgetWitness, BudgetWitnesses};
use crate::contract::plan::BoundaryPlan;
use crate::contract::report::{
    BoundaryReportBody, CaptureRefs, ExitStatus, ObservedFact, Outcome,
    BOUNDARY_REPORT_SCHEMA_VERSION,
};

/// Map a wasmtime run observation onto the durable report body.
pub(super) fn map_observation(
    backend: &WasmBackend,
    plan: &BoundaryPlan,
    obs: &WasmRunObservation,
    mut observed: Vec<ObservedFact>,
) -> BoundaryReportBody {
    observed.push(ObservedFact {
        kind: "workload_launched".to_string(),
        detail: format!(
            "wasmtime module {} (confined={})",
            obs.module_ref, obs.filesystem_confined
        ),
    });
    observed.push(ObservedFact {
        kind: "wasm_terminal".to_string(),
        detail: format!(
            "terminal={:?} outcome={:?} detail={}",
            obs.terminal,
            obs.terminal.outcome(),
            obs.terminal.detail()
        ),
    });
    for note in &obs.notes {
        observed.push(ObservedFact {
            kind: "wasm_note".to_string(),
            detail: note.clone(),
        });
    }
    if obs.filesystem_confined {
        observed.push(ObservedFact {
            kind: "filesystem_confined".to_string(),
            detail: "wasi preopen confinement installed".to_string(),
        });
    }
    observed.push(ObservedFact {
        kind: "stream_captured".to_string(),
        detail: format!(
            "captured {} stdout byte(s), {} stderr byte(s) via WASI pipes",
            obs.stdout.len(),
            obs.stderr.len()
        ),
    });
    if let Some(used) = obs.fuel_consumed {
        observed.push(ObservedFact {
            kind: "fuel_witnessed".to_string(),
            detail: format!(
                "wasmtime fuel consumed={used} against limit={}",
                plan.budgets.cpu_micros.effective_limit
            ),
        });
    }

    let captured = capture_refs(obs);
    let outcome = obs.terminal.outcome();
    let exit = exec_exit(&obs.terminal);
    let budget = budget_witnesses(plan, obs, outcome);
    body(backend, plan, outcome, exit, captured, observed, budget)
}

/// Assemble a fail-closed report body before the guest runs.
pub(super) fn fail_closed(
    backend: &WasmBackend,
    plan: &BoundaryPlan,
    outcome: Outcome,
    observed: Vec<ObservedFact>,
) -> BoundaryReportBody {
    body(
        backend,
        plan,
        outcome,
        None,
        CaptureRefs::default(),
        observed,
        BudgetWitnesses::unwitnessed(&plan.budgets),
    )
}

fn exec_exit(terminal: &WasmTerminal) -> Option<ExitStatus> {
    terminal.exit()
}

fn capture_refs(obs: &WasmRunObservation) -> CaptureRefs {
    match obs.terminal {
        WasmTerminal::Unsupported(_) | WasmTerminal::SupervisorFault(_)
            if obs.wall_micros.is_none() =>
        {
            CaptureRefs::default()
        }
        WasmTerminal::Exit(_)
        | WasmTerminal::Failed(_)
        | WasmTerminal::Timeout(_)
        | WasmTerminal::Unsupported(_)
        | WasmTerminal::SupervisorFault(_) => CaptureRefs {
            stdout: Some(format!("inline:{}b", obs.stdout.len())),
            stderr: Some(format!("inline:{}b", obs.stderr.len())),
        },
    }
}

fn budget_witnesses(
    plan: &BoundaryPlan,
    obs: &WasmRunObservation,
    outcome: Outcome,
) -> BudgetWitnesses {
    let mut budget = BudgetWitnesses::unwitnessed(&plan.budgets);
    if let Some(wall) = obs.wall_micros {
        budget.wall_micros = BudgetWitness::witnessed(&plan.budgets.wall_micros, wall);
    }
    if let Some(fuel) = obs.fuel_consumed {
        let mut witness = BudgetWitness::witnessed(&plan.budgets.cpu_micros, fuel);
        if outcome == Outcome::Timeout {
            witness.finding = BudgetFinding::LimitReachedEnforced;
        }
        budget.cpu_micros = witness;
    }
    budget
}

/// Assemble the honest report body. `denied` stays empty by design; independent
/// oracles prove denied effects from host state.
fn body(
    backend: &WasmBackend,
    plan: &BoundaryPlan,
    outcome: Outcome,
    exit: Option<ExitStatus>,
    captured: CaptureRefs,
    observed: Vec<ObservedFact>,
    budget: BudgetWitnesses,
) -> BoundaryReportBody {
    BoundaryReportBody {
        schema_version: BOUNDARY_REPORT_SCHEMA_VERSION,
        plan_id: plan.plan_id,
        backend: backend.id.clone(),
        profile: backend.probe(),
        outcome,
        admitted: plan.admitted.clone(),
        observed,
        denied: Vec::new(),
        exit,
        captured,
        budget,
        artifacts: Vec::new(),
        findings: Vec::new(),
    }
}

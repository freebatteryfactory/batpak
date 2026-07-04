//! Wasm backend budget profile.
//!
//! Wasmtime supplies a real fuel counter for instruction progress and a real linear
//! memory grow limit through `StoreLimits`. Other dimensions are honestly mediated or
//! structurally zero by the WASI capability set; they are not reported as measured
//! usage unless the runner observes them.

use crate::contract::budget::{BudgetAvailability, BudgetProfile};
use crate::contract::capability::{Enforcement, EvidenceClaim, EvidenceSet};

/// The budget profile advertised by the wasmi backend.
#[must_use]
pub(super) fn observed_budget_profile() -> BudgetProfile {
    let mediated = |mechanism: &str| BudgetAvailability {
        available: u64::MAX,
        enforcement: Enforcement::Mediated,
        evidence: EvidenceSet::new(),
        mechanism: mechanism.to_string(),
    };
    let hard = |mechanism: &str, evidence: EvidenceSet| BudgetAvailability {
        available: u64::MAX,
        enforcement: Enforcement::Enforced,
        evidence,
        mechanism: mechanism.to_string(),
    };
    BudgetProfile {
        wall_micros: mediated("wasmi_fuel:mediated-wall"),
        cpu_micros: mediated("wasmi_fuel:mediated-cpu"),
        resident_bytes: hard("wasmi_store_limits:memory_size", EvidenceSet::new()),
        process_count: hard("wasm_single_instance:no-child-process", EvidenceSet::new()),
        handle_count: mediated("wasi_handles:preopen-stdio"),
        storage_bytes: mediated("wasi_preopen:host-storage"),
        network_bytes: hard(
            "wasi_no_socket_cap:deny-all",
            [EvidenceClaim::DeniedAttempts].into_iter().collect(),
        ),
    }
}

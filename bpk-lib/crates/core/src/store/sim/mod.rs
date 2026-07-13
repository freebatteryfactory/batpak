//! Cooperative single-thread seeded simulation runtime (GAUNT-SIM-2c).
//!
//! Boundary (contract A12): this module compiles under the
//! `conformance-harness` feature — the clean external `StoreFs`-proof surface.
//! `dangerous-test-hooks` implies `conformance-harness`, so the dangerous lane
//! still sees everything. The namespace-truth backend [`shadow_fs::ShadowFs`]
//! and the published conformance corpus are the `conformance-harness` surface;
//! the byte-fault [`fs::SimFs`] layer and the seeded DST
//! workload/scheduler/recovery machinery (including the `__sim` namespace
//! matrix) stay behind `dangerous-test-hooks`. It composes three deterministic
//! backends over the production seams introduced earlier in the gauntlet:
//!
//!   * [`clock::SimClock`] implements [`crate::store::platform::clock::Clock`]
//!     as *logical* time that the scheduler — not the wall clock — advances.
//!   * [`scheduler::SimScheduler`] implements
//!     [`crate::store::platform::spawn::Spawn`] cooperatively: spawned bodies
//!     are queued and stepped on the calling thread in a deterministic order,
//!     so there is never any OS-thread nondeterminism.
//!   * [`fs::SimFs`] implements [`crate::store::platform::fs::StoreFs`] as a
//!     durability-fault injector over REAL tempfiles — not an in-memory
//!     filesystem. Reads pass through to the platform fs; the write path
//!     consults a ChaCha8-style seeded PRNG ([`fastrand`]) to apply faults
//!     (torn-write, short-read, fsync-drop) keyed off
//!     [`crate::store::fault::InjectionPoint`]. Because the bytes live on a
//!     real filesystem behind concrete `File` handles, `SimFs` is not a
//!     wasm-portable or diskless backend (see the `StoreFs` trait docs).
//!
//! On top of those, [`workload`] drives a small seeded workload and
//! [`invariants`] checks per-step safety properties (hash-chain continuity,
//! monotonic visible frontier, no-loss-after-crash-recover).
//!
//! Two durability axes are modeled independently: BYTE durability by the
//! [`fs::SimFs`] layer (an honest/lying disk over any inner; directory-entry
//! names stay durable by design) and NAMESPACE durability by the
//! self-contained [`shadow_fs::ShadowFs`] backend (directory-entry truth vs.
//! byte truth, resolved by a tree-swap `crash()`). The namespace-recovery
//! legality matrix that drives a real `Store` over `ShadowFs` lives in
//! [`namespace_recovery`].
//!
//! Determinism contract: a [`Sim`] constructed from the same seed produces a
//! byte-identical op-trace. `BATPAK_SEED=N` selects the seed for replay. The
//! `sim_is_deterministic` integration test (`crates/core/tests/sim.rs`) runs a
//! seeded workload twice and asserts the op-trace hashes match.
//!
//! Scope (GAUNT-SIM-2c): the three trait backends, seeded workload/invariant
//! engine, and determinism test are live. `Store` over `SimFs` is wired for
//! fork/import/recovery DST paths (`recovery.rs`, `fork_recovery.rs`,
//! `import_recovery.rs`). Optional follow-on: route the writer thread through
//! `SimScheduler` for full cooperative scheduling — not required for current
//! corpus proofs.

// The byte-fault DST machinery (fault schedules, cooperative scheduler, seeded
// workload/recovery oracles, and the namespace matrix) is the internal
// `dangerous-test-hooks` surface. Only the namespace-truth backend
// (`shadow_fs`) and the byte-fault filesystem it reuses `CrashOp` from (`fs`)
// stay on the clean `conformance-harness` surface (contract A12); `fs` gates
// its own `SimFs` promotion internally.
#[cfg(all(test, feature = "dangerous-test-hooks"))]
mod atomic_fault;
#[cfg(feature = "dangerous-test-hooks")]
pub mod clock;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod corpus;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod fault_model;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod fork_hostile;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod fork_recovery;
pub(crate) mod fs;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod import_recovery;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod invariants;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod namespace_recovery;
#[cfg(all(test, feature = "dangerous-test-hooks"))]
mod read_fault;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod recovery;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod recovery_matrix;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod scheduler;
pub(crate) mod shadow_fs;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) mod workload;

#[cfg(feature = "dangerous-test-hooks")]
use std::sync::Arc;

#[cfg(feature = "dangerous-test-hooks")]
pub use clock::SimClock;
pub use shadow_fs::ShadowFs;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) use fault_model::InMemFaultFs;
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) use scheduler::SimScheduler;

/// Read the replay seed from the `BATPAK_SEED` environment variable.
///
/// Returns the parsed seed when the variable is present and parses as a `u64`,
/// otherwise the supplied `default`. This is the single entry point tests use
/// so that `BATPAK_SEED=N cargo nextest ...` deterministically replays a run.
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) fn seed_from_env(default: u64) -> u64 {
    match std::env::var("BATPAK_SEED") {
        Ok(raw) => raw.trim().parse::<u64>().unwrap_or(default),
        Err(_) => default,
    }
}

/// A fully wired deterministic simulation: a shared logical clock, a
/// cooperative scheduler, and a fault-injecting filesystem (interposing on
/// real tempfiles), all derived from a single seed.
///
/// The three backends are exposed as `Arc<dyn Trait>` so they can be installed
/// on a [`crate::store::StoreConfig`] via `with_clock` / `with_spawner` /
/// `with_fs`. The [`workload`] engine also drives them directly for determinism
/// proofs; full Store-over-`SimScheduler` wiring is optional follow-on.
#[cfg(feature = "dangerous-test-hooks")]
pub(crate) struct Sim {
    /// Seed every backend and the workload PRNG derive from.
    pub(crate) seed: u64,
    /// Logical clock advanced by the scheduler, shared with the store.
    pub(crate) clock: Arc<SimClock>,
    /// Cooperative scheduler that runs spawned bodies on the calling thread.
    pub(crate) scheduler: Arc<SimScheduler>,
    /// In-memory, fault-injecting model keyed off the same seed (model-only
    /// determinism witness; the real-`Store` composition uses [`fs::SimFs`]).
    pub(crate) fs: Arc<InMemFaultFs>,
}

#[cfg(feature = "dangerous-test-hooks")]
impl Sim {
    /// Construct a simulation from `seed`. All randomness — scheduler ordering
    /// jitter, fault decisions, and workload op selection — is derived from
    /// this single seed, so two `Sim::new(seed)` instances are identical.
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            seed,
            clock: Arc::new(SimClock::new()),
            scheduler: Arc::new(SimScheduler::new()),
            // The fs PRNG is keyed off a stable, seed-derived sub-stream so
            // that changing the workload PRNG draw count never perturbs which
            // faults fire — each backend owns an independent sub-seed.
            fs: Arc::new(InMemFaultFs::new(seed ^ 0x5F5F_5F5F_5F5F_5F5F)),
        }
    }

    /// Run the seeded workload to completion and return the op-trace digest.
    ///
    /// The digest is a FNV-1a hash over the ordered op-trace. Two runs from the
    /// same seed MUST return the same digest; that is the determinism contract
    /// asserted by `sim_is_deterministic`. Real-Store safety over the sim
    /// backends is proven separately by the DST recovery corpus, not re-checked
    /// here.
    ///
    /// # Errors
    /// Returns a seed-tagged `Err` only if the workload driver surfaces a
    /// backend failure; the in-memory simulation backends do not, so a completed
    /// run always yields `Ok(digest)`. The `Result` is retained for the
    /// seed-tagged harness contract.
    pub(crate) fn run_workload(&self, steps: usize) -> Result<u64, String> {
        workload::run(self, steps)
    }
}

/// Test-only entry point: build a `Sim` from `seed` and run a `steps`-long
/// seeded workload, returning the op-trace digest. Re-exported (doc-hidden) at
/// the crate root as `batpak::__sim::run_seeded_workload` so the
/// `sim_is_deterministic` integration test can drive the simulator without
/// exposing the `pub(crate)` backends. `BATPAK_SEED` is honored by the caller
/// passing `seed_from_env(default)`.
///
/// # Errors
/// Returns a seed-tagged `Err` only if the workload driver surfaces a backend
/// failure; the in-memory simulation backends do not, so a completed run always
/// yields `Ok(digest)`. The `Result` is retained for the seed-tagged harness
/// contract.
#[cfg(feature = "dangerous-test-hooks")]
pub fn run_seeded_workload(seed: u64, steps: usize) -> Result<u64, String> {
    Sim::new(seed).run_workload(steps)
}

/// Test-only re-export of `seed_from_env` for `BATPAK_SEED` replay from
/// integration tests.
#[cfg(feature = "dangerous-test-hooks")]
pub fn replay_seed(default: u64) -> u64 {
    seed_from_env(default)
}

#[cfg(all(test, feature = "dangerous-test-hooks"))]
mod tests {
    use super::*;

    #[test]
    fn seed_from_env_falls_back_to_default_when_unset() {
        // The test process must not have BATPAK_SEED set for this to be
        // meaningful; if it is set (replay), we accept the parsed value.
        let chosen = seed_from_env(42);
        assert!(
            chosen == 42 || std::env::var("BATPAK_SEED").is_ok(),
            "PROPERTY: absent BATPAK_SEED yields the supplied default seed"
        );
    }

    #[test]
    fn same_seed_same_digest() {
        let a = Sim::new(7)
            .run_workload(64)
            .expect("workload must hold invariants");
        let b = Sim::new(7)
            .run_workload(64)
            .expect("workload must hold invariants");
        assert_eq!(
            a, b,
            "PROPERTY: identical seeds must yield identical op-trace digests"
        );
    }

    #[test]
    fn different_seeds_diverge() {
        let a = Sim::new(1)
            .run_workload(64)
            .expect("workload must hold invariants");
        let b = Sim::new(2)
            .run_workload(64)
            .expect("workload must hold invariants");
        assert_ne!(
            a, b,
            "PROPERTY: distinct seeds should (almost surely) diverge over 64 steps"
        );
    }

    #[test]
    fn run_seeded_workload_is_the_real_seed_dependent_digest() {
        // Drive the PUBLIC entry point directly (the integration determinism
        // gate goes through it). A constant `Ok(0)`/`Ok(1)` body would make
        // every seed collapse to the same value, so distinct seeds diverging is
        // exactly the property that kills those constant mutants.
        let d1 = run_seeded_workload(7, 64).expect("seed 7 must hold invariants");
        let d1_again = run_seeded_workload(7, 64).expect("seed 7 replay must hold invariants");
        let d2 = run_seeded_workload(9, 64).expect("seed 9 must hold invariants");

        assert_eq!(
            d1, d1_again,
            "PROPERTY: run_seeded_workload must be deterministic for a fixed seed"
        );
        assert_ne!(
            d1, d2,
            "PROPERTY: run_seeded_workload must depend on the seed — a constant body \
             (Ok(0)/Ok(1)) would collapse distinct seeds to the same digest"
        );
    }
}

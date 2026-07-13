// The namespace-truth sim, its crash hook, and the `__sim` matrix entry points
// live behind `dangerous-test-hooks`; without the feature the file is empty.
#![cfg(feature = "dangerous-test-hooks")]
//! Namespace-durability recovery matrix (#177) — the byte-truth-vs-name-truth
//! companion to the B3 byte-recovery oracle (`tests/recovery_oracle.rs`).
//!
//! justifies: INV-SIMFS-NAMESPACE-TRUTH — a real `Store` driven over the
//! self-contained `ShadowFs` namespace-truth backend, crashed under each
//! namespace fault mode (honest crash, a dropped parent-dir sync at the 1st/2nd/
//! 3rd occurrence, and all-parent-syncs dropped), then reopened over the
//! persisted (name-and-byte-truncated) tree, must recover a state that is EXACTLY
//! one of {CommittedPrefix | RolledBack | CanonicalRefusal} — the legality oracle
//! inside `run_namespace_matrix` fail-closes on any undead event, broken chain,
//! honest-mode lost-durable commit, or non-canonical reopen. The SAME seed sweeps
//! the matrix to the IDENTICAL cell vector (determinism), and the mode set can
//! NEVER silently shed its drop-parent-sync cells (#197 vacuity guard).
//!
//! PROVES: the namespace-durability model is swept end-to-end through the real
//! `Store` recovery path and every cell lands in a legal, deterministic outcome.
//! CATCHES: a matrix that drops its lying-namespace (dropped-dir-sync) cells and
//! silently reports an all-honest pass; a non-deterministic recovery; an illegal
//! recovered namespace state escaping classification.
//!
//! Requires `--features dangerous-test-hooks`. Replay a specific seed with
//! `BATPAK_SEED=N cargo nextest run -p batpak --features dangerous-test-hooks
//! -E 'test(namespace_recovery_matrix_is_legal_and_deterministic)'`.

use batpak::__sim::{namespace_replay_seed, run_namespace_matrix, NamespaceMatrixCell, RecoveredClass};

/// The matrix seed used by both witnesses (env-overridable via `BATPAK_SEED`).
const DEFAULT_SEED: u64 = 0x5EED_1701;
/// Op-stream length per cell — enough to force segment rotations (create +
/// parent-sync edges) so the dropped-dir-sync modes actually bite.
const STEPS: usize = 48;

/// Every cell must classify as one of the three legal recovered outcomes.
fn assert_legal_classification(cell: &NamespaceMatrixCell) {
    assert!(
        matches!(
            cell.class,
            RecoveredClass::CommittedPrefix
                | RecoveredClass::RolledBack
                | RecoveredClass::CanonicalRefusal
        ),
        "ILLEGAL RECOVERED NAMESPACE: cell `{}` classified as {:?}; the only legal outcomes are \
         CommittedPrefix, RolledBack, or CanonicalRefusal",
        cell.mode,
        cell.class
    );
}

/// GREEN (every-PR, dangerous-test-hooks lane): the full namespace fault matrix —
/// honest crash, a dropped parent-dir sync at each of the first three occurrences,
/// and all-parent-syncs dropped — must recover a LEGAL state in EVERY cell, and
/// the SAME seed must sweep to the IDENTICAL cell vector (determinism).
#[test]
fn namespace_recovery_matrix_is_legal_and_deterministic(
) -> Result<(), Box<dyn std::error::Error>> {
    let seed = namespace_replay_seed(DEFAULT_SEED);

    let first: Vec<NamespaceMatrixCell> =
        run_namespace_matrix(seed, STEPS).map_err(std::io::Error::other)?;
    let second: Vec<NamespaceMatrixCell> =
        run_namespace_matrix(seed, STEPS).map_err(std::io::Error::other)?;

    // The five namespace modes (honest + three drop-nth + drop-all) must all be
    // present — a shrunk matrix is exactly the #197 vacuity failure.
    assert!(
        first.len() >= 5,
        "the matrix must cover honest crash, drop-parent-sync-nth 1..=3, and drop-all-parent-syncs \
         (>= 5 cells), got {}",
        first.len()
    );

    assert_eq!(
        first, second,
        "PROPERTY: identical seed (0x{seed:X}) must sweep the namespace matrix to the identical \
         cell vector (replay with BATPAK_SEED={seed})"
    );

    for cell in &first {
        assert_legal_classification(cell);
    }

    Ok(())
}

/// The matrix must retain its lying-namespace (dropped parent-dir sync) cells: a
/// matrix that silently collapses to honest-only crash cells would report a
/// vacuous all-honest pass. Pin the exact drop-mode labels so a rename/removal of
/// a namespace mode surfaces here rather than passing quietly.
#[test]
fn namespace_matrix_retains_its_drop_parent_sync_modes(
) -> Result<(), Box<dyn std::error::Error>> {
    let seed = namespace_replay_seed(DEFAULT_SEED);
    let cells: Vec<NamespaceMatrixCell> =
        run_namespace_matrix(seed, STEPS).map_err(std::io::Error::other)?;

    let modes: Vec<&str> = cells.iter().map(|c| c.mode.as_str()).collect();
    for required in [
        "honest-namespace-crash",
        "drop-parent-sync-nth-1",
        "drop-parent-sync-nth-2",
        "drop-parent-sync-nth-3",
        "drop-all-parent-syncs",
    ] {
        assert!(
            modes.contains(&required),
            "PROPERTY: namespace matrix lost its `{required}` cell — present modes: {modes:?}"
        );
    }

    Ok(())
}

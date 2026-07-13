//! Namespace-truth crash-recovery legality driver over [`ShadowFs`].
//!
//! This is the namespace-durability sibling of [`super::recovery`] (B2) and
//! [`super::recovery_matrix`] (B3). Those two oracles model the BYTE-truth axis
//! over [`super::fs::SimFs`] (an honest/lying disk that truncates the unsynced
//! tail on crash). This oracle models the NAMESPACE-truth axis over
//! [`ShadowFs`]: a self-contained virtual backend where directory-entry
//! durability advances ONLY at an honored parent-dir sync, and `crash()` swaps
//! the visible namespace for the durable one.
//!
//! The composition is the same genuine one: a REAL [`Store::open`] with
//! `ShadowFs` installed via [`StoreConfig::with_fs`], driven through the real
//! `append`/`append_batch`/`sync` API (shared [`super::recovery::run_op_plan`]),
//! crashed by abandoning the store and calling [`ShadowFs::crash`], then
//! REOPENED over the SAME shadow clone (the durable tree lives in the shared
//! `Arc`, not on real disk). For every fault mode the recovered visible state
//! must be EXACTLY one of {`CommittedPrefix` | `RolledBack` | `CanonicalRefusal`}
//! and LEGAL: a prefix of the appended op-log (no undead events), an intact hash
//! chain, and — for the honest mode — the sacred no-loss rule. The
//! parent-dir-sync DROP modes relax the no-loss rule (a name whose durability
//! depended on the dropped dir sync may legally be absent after the crash), the
//! exact analogue of B3's lying-disk relaxation — but they NEVER permit an
//! undead event or a broken chain.
//!
//! Determinism: `ShadowFs` decides purely from explicit armed schedules (no
//! background randomness here), and the op plan is seed-derived, so the same
//! `(seed, mode)` recovers the identical classification + digest.

use super::recovery::{
    fold, is_canonical_refusal, recovered_user_events, run_op_plan, verify_hash_chain, OpPlan,
    Violation, FNV_OFFSET,
};
use super::recovery_matrix::RecoveredClass;
use super::seed_from_env;
use super::shadow_fs::ShadowFs;
use crate::coordinate::Coordinate;
use crate::store::platform::fs::StoreFs;
use crate::store::{Store, StoreConfig, SyncMode};
use std::sync::Arc;

/// The virtual store root the driver opens under `ShadowFs` (the shadow backend
/// is self-contained, so this path never touches the real filesystem).
const NS_ROOT: &str = "/shadow/ns-recovery";

/// A namespace-durability fault mode swept by the matrix. `Honest` keeps the
/// sacred no-loss rule; the `Drop*` modes silently drop parent-dir syncs so a
/// name's durability can be lost across the crash (no-loss relaxed, no-undead
/// preserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NamespaceFaultMode {
    /// Every parent-dir sync honored; the crash truncates only what was never
    /// made durable by an honored sync. The sacred no-loss rule holds.
    HonestNamespaceCrash,
    /// The `nth` (1-based, monotonic) parent-dir-sync event is silently dropped
    /// (returns `Ok`, durable namespace NOT advanced) — a lying dir sync.
    DropParentSyncNth {
        /// 1-based parent-dir-sync occurrence to drop.
        nth: u32,
    },
    /// Every parent-dir sync is silently dropped for the whole run.
    DropAllParentSyncs,
}

impl NamespaceFaultMode {
    /// Arm `shadow` for this mode before the store opens.
    fn arm(self, shadow: &ShadowFs) {
        match self {
            NamespaceFaultMode::HonestNamespaceCrash => {}
            NamespaceFaultMode::DropParentSyncNth { nth } => shadow.arm_parent_sync_drop(nth),
            NamespaceFaultMode::DropAllParentSyncs => shadow.arm_parent_sync_drop_all(),
        }
    }

    /// A stable digest token discriminating the recovered classification per mode.
    fn token(self) -> u64 {
        match self {
            NamespaceFaultMode::HonestNamespaceCrash => fold(0x5A_DE_00, 0),
            NamespaceFaultMode::DropParentSyncNth { nth } => fold(0x5A_DE_01, u64::from(nth)),
            NamespaceFaultMode::DropAllParentSyncs => fold(0x5A_DE_02, 0),
        }
    }

    /// A short, run-stable label for the public matrix cell.
    fn label(self) -> String {
        match self {
            NamespaceFaultMode::HonestNamespaceCrash => "honest-namespace-crash".to_string(),
            NamespaceFaultMode::DropParentSyncNth { nth } => {
                format!("drop-parent-sync-nth-{nth}")
            }
            NamespaceFaultMode::DropAllParentSyncs => "drop-all-parent-syncs".to_string(),
        }
    }

    /// Whether this mode relaxes the no-loss rule (a dropped parent-dir sync may
    /// legally lose a name across the crash; undead/broken-chain never legal).
    fn is_lying(self) -> bool {
        matches!(
            self,
            NamespaceFaultMode::DropParentSyncNth { .. } | NamespaceFaultMode::DropAllParentSyncs
        )
    }
}

/// The full namespace fault-mode matrix the public oracle sweeps. Consumed by the
/// `recovery_matrix.rs` witness (`>= 4` modes) — keep at least the honest mode
/// plus the drop modes.
pub(crate) fn all_namespace_modes() -> Vec<NamespaceFaultMode> {
    vec![
        NamespaceFaultMode::HonestNamespaceCrash,
        NamespaceFaultMode::DropParentSyncNth { nth: 1 },
        NamespaceFaultMode::DropParentSyncNth { nth: 2 },
        NamespaceFaultMode::DropParentSyncNth { nth: 3 },
        NamespaceFaultMode::DropAllParentSyncs,
    ]
}

/// Build a fresh `Arc<dyn StoreFs>` sharing `shadow`'s durable tree.
fn fs_handle(shadow: &ShadowFs) -> Arc<dyn StoreFs> {
    Arc::new(shadow.clone())
}

/// Run one namespace matrix cell, seed/mode-tagging any legality violation.
///
/// Returns `(cell, durable_acked, faults_fired)`: the public matrix keeps only
/// `cell`; the extras are the no-loss floor and the anti-vacuity fired-fault
/// count the inline tests witness (they are NOT carried on the public cell, so a
/// non-test feature build never sees an unread struct field).
///
/// # Errors
/// Returns a seed/mode-tagged violation string if the recovered state is illegal
/// for the cell's fault mode (lost durable name under honest crash, an undead
/// event, a broken hash chain, or a non-canonical reopen error).
pub(crate) fn run(
    seed: u64,
    steps: usize,
    mode: NamespaceFaultMode,
) -> Result<(NamespaceMatrixCell, usize, u32), String> {
    drive(seed, steps, mode)
        .map_err(|v| format!("namespace DST violation (seed={seed}, mode={mode:?}): {v}"))
}

/// Internal driver returning the typed [`Violation`] for [`run`] to seed-tag.
fn drive(
    seed: u64,
    steps: usize,
    mode: NamespaceFaultMode,
) -> Result<(NamespaceMatrixCell, usize, u32), Violation> {
    let shadow = ShadowFs::new();
    mode.arm(&shadow);
    let coord = Coordinate::new("entity:ns", "scope:recovery")
        .map_err(|e| Violation::NonCanonicalReopen(e.to_string()))?;
    let plan = OpPlan::seeded(seed, steps);
    let mut digest = fold(fold(FNV_OFFSET, seed), mode.token());

    // ── Phase 1: drive the seeded op stream over the real Store, then crash. ──
    let appended;
    let durable_acked;
    let faults_fired;
    {
        let config = StoreConfig::new(NS_ROOT)
            .with_fs(fs_handle(&shadow))
            // Small segments so rotations exercise create + parent-sync edges.
            .with_segment_max_bytes(512)
            // Leave durability to the explicit `Op::Sync` boundaries in the plan.
            .with_sync_every_n_events(1_000_000)
            .with_sync_mode(SyncMode::SyncAll);
        let store = Store::open(config).map_err(|e| Violation::NonCanonicalReopen(e.to_string()))?;

        let (a, d, run_digest) = run_op_plan(&store, &coord, &plan, digest)?;
        appended = a;
        durable_acked = d;
        digest = run_digest;

        // Abandon the writer WITHOUT a clean shutdown, capture the fired-fault
        // count BEFORE the crash (the drops fired during the drive), then swap the
        // visible namespace for the durable one.
        store.abandon_without_shutdown();
        faults_fired = shadow.armed_faults_fired();
        shadow.crash();
    }

    // ── Phase 2: reopen the real Store over the SAME (now-durable) shadow tree. ──
    match Store::open(StoreConfig::new(NS_ROOT).with_fs(fs_handle(&shadow))) {
        Ok(store) => classify(&store, appended, durable_acked, mode, digest, faults_fired),
        Err(error) if is_canonical_refusal(&error) => Ok((
            NamespaceMatrixCell {
                mode: mode.label(),
                class: RecoveredClass::CanonicalRefusal,
                digest: fold(digest, 0xCA11_AB1E),
                recovered_visible: 0,
            },
            durable_acked,
            faults_fired,
        )),
        Err(other) => Err(Violation::NonCanonicalReopen(format!("{other:?}"))),
    }
}

/// Classify + legality-check the recovered visible state of a reopened store,
/// returning `(cell, durable_acked, faults_fired)`.
fn classify(
    store: &Store,
    appended: usize,
    durable_acked: usize,
    mode: NamespaceFaultMode,
    digest: u64,
    faults_fired: u32,
) -> Result<(NamespaceMatrixCell, usize, u32), Violation> {
    let recovered = recovered_user_events(store);
    let recovered_visible = recovered.len();

    // Sacred rule (honest namespace crash only): no acknowledged-durable commit
    // may be lost. A drop mode RELAXES this — a name whose durability depended on
    // the dropped parent-dir sync may legally be absent.
    if !mode.is_lying() && recovered_visible < durable_acked {
        return Err(Violation::LostDurableCommit {
            durable: durable_acked,
            recovered: recovered_visible,
        });
    }
    // No-undead rule (EVERY mode): never recover more than was appended — the FS
    // may lose a name, never resurrect an event that was never submitted.
    if recovered_visible > appended {
        return Err(Violation::UndeadEvent {
            recovered: recovered_visible,
            appended,
        });
    }
    // Intact hash chain across the recovered visible prefix (EVERY mode).
    verify_hash_chain(&recovered)?;

    let mut digest = fold(fold(digest, recovered_visible as u64), durable_acked as u64);
    for ev in &recovered {
        digest = fold(digest, ev.event_hash_token);
    }
    let class = if recovered_visible == 0 {
        RecoveredClass::RolledBack
    } else {
        RecoveredClass::CommittedPrefix
    };
    Ok((
        NamespaceMatrixCell {
            mode: mode.label(),
            class,
            digest,
            recovered_visible,
        },
        durable_acked,
        faults_fired,
    ))
}

// ── Test-only public surface, re-exported (doc-hidden) at `batpak::__sim`. ──

/// One cell of the public namespace-recovery matrix sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceMatrixCell {
    /// Human-readable fault-mode label (e.g. `"honest-namespace-crash"`).
    pub mode: String,
    /// Recovered classification (shared with the B3 matrix).
    pub class: RecoveredClass,
    /// FNV-1a determinism digest for this cell.
    pub digest: u64,
    /// Recovered visible event count after the crash.
    pub recovered_visible: usize,
}

/// Test-only entry point re-exported (doc-hidden) at `batpak::__sim`: sweep every
/// [`NamespaceFaultMode`] for `seed` and return one [`NamespaceMatrixCell`] per
/// cell. Each cell's legality oracle fail-closes inside [`run`].
///
/// # Errors
/// Returns a seed/mode-tagged violation string on the first illegal cell.
pub fn run_namespace_matrix(seed: u64, steps: usize) -> Result<Vec<NamespaceMatrixCell>, String> {
    all_namespace_modes()
        .into_iter()
        .map(|mode| run(seed, steps, mode).map(|(cell, _durable, _fired)| cell))
        .collect()
}

/// Test-only replay-seed helper for `BATPAK_SEED`.
pub fn namespace_replay_seed(default: u64) -> u64 {
    seed_from_env(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_recovers_legally() {
        for mode in all_namespace_modes() {
            let result = run(0x5EED_5A01, 48, mode);
            assert!(
                result.is_ok(),
                "mode {mode:?} must recover legally: {result:?}"
            );
        }
    }

    #[test]
    fn same_seed_same_cell_determinism() {
        let a = run_namespace_matrix(0x5EED_5A02, 48).expect("legal matrix sweep");
        let b = run_namespace_matrix(0x5EED_5A02, 48).expect("legal matrix sweep");
        assert_eq!(
            a, b,
            "PROPERTY: identical (seed, steps) must sweep the identical namespace matrix"
        );
    }

    #[test]
    fn matrix_contains_the_drop_parent_sync_modes() {
        // Pins that the matrix cannot silently lose its namespace-drop cells (the
        // #197 vacuity concern): the honest mode alone would be a trivial pass.
        let modes = all_namespace_modes();
        assert!(
            modes.len() >= 4,
            "PROPERTY: the namespace matrix must keep the honest mode plus the drop modes"
        );
        assert!(
            modes.contains(&NamespaceFaultMode::DropAllParentSyncs),
            "PROPERTY: the drop-all-parent-syncs mode must be swept"
        );
    }

    #[test]
    fn drop_modes_actually_fire() {
        // Anti-vacuity of the fault SCHEDULE itself: an armed parent-sync drop that
        // never fires is a vacuous fixture. Each drop mode must fire at least once.
        for mode in [
            NamespaceFaultMode::DropParentSyncNth { nth: 1 },
            NamespaceFaultMode::DropParentSyncNth { nth: 2 },
            NamespaceFaultMode::DropParentSyncNth { nth: 3 },
            NamespaceFaultMode::DropAllParentSyncs,
        ] {
            let (_cell, _durable, fired) =
                run(0x5EED_5A03, 48, mode).expect("legal recovery under a drop mode");
            assert!(
                fired > 0,
                "PROPERTY: armed drop mode {mode:?} must actually fire (non-vacuous fixture)"
            );
        }
    }

    #[test]
    fn honest_mode_preserves_the_durable_prefix() {
        let (cell, durable_acked, _fired) =
            run(0x5EED_5A04, 64, NamespaceFaultMode::HonestNamespaceCrash)
                .expect("legal honest recovery");
        assert!(
            cell.recovered_visible >= durable_acked,
            "SACRED RULE: an honest namespace crash never loses an acknowledged-durable commit"
        );
    }
}

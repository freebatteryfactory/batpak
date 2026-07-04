//! Wasm backend — wasmi + WASI-preopen confinement.
//!
//! The HONEST per-platform [`SupportMatrix`] is pure data and always compiled. With
//! `backend-wasm` enabled, [`WasmBackend`] instantiates [`Workload::Wasm`](crate::Workload::Wasm)
//! modules with wasmi and `wasi_snapshot_preview1`: declared filesystem roots
//! become WASI preopens, `Environment::Exact` becomes explicit WASI env, stdio is
//! captured with WASI pipes, temp roots are private preopens, and network deny-all is
//! enforced by installing no socket capability. Wasm has no raw OS basement here.
//!
//! HONESTY (SCOPE §4 — Wasm): FS/env/stdio/`TempRoot`/`Commit`/`Discard`/`List` and
//! `InheritedFdsNone` are Enforced through the WASI capability set. `ChildSpawn` /
//! `Kill` / `ExposePath`(Mount), `NetworkAllowList`, and selective inherited-fd
//! keep-lists are STRUCTURALLY [`Enforcement::Unsupported`] — a wasm guest has no
//! native fork/kill/mount or raw host-fd broker. These are load-bearing fail-closed
//! cells; never emulate a native primitive in the report.

use crate::contract::capability::{Enforcement, EvidenceClaim, SupportVerdict};
use crate::contract::support::{RequirementKind, SupportMatrix};
use std::collections::BTreeMap;

/// The HONEST Wasm family support matrix (SCOPE §4). Pure data — constructible
/// and unit-testable on any host.
#[must_use]
pub fn support_matrix() -> SupportMatrix {
    let mut best = BTreeMap::new();

    insert(
        &mut best,
        RequirementKind::LaunchWorkload,
        Enforcement::Enforced,
        &[EvidenceClaim::TerminalOutcome],
    );
    insert(
        &mut best,
        RequirementKind::CaptureStreams,
        Enforcement::Enforced,
        &[EvidenceClaim::CapturedStreams],
    );

    // FS via WASI preopen: structurally confined to the preopened dirs.
    insert(
        &mut best,
        RequirementKind::Filesystem,
        Enforcement::Enforced,
        &[
            EvidenceClaim::AllowedActions,
            EvidenceClaim::FilesystemDelta,
            EvidenceClaim::MechanismAttestation,
        ],
    );
    insert(
        &mut best,
        RequirementKind::Environment,
        Enforcement::Enforced,
        &[EvidenceClaim::MechanismAttestation],
    );
    insert(
        &mut best,
        RequirementKind::TempRoot,
        Enforcement::Enforced,
        &[EvidenceClaim::MechanismAttestation],
    );
    insert(
        &mut best,
        RequirementKind::CommitArtifact,
        Enforcement::Enforced,
        &[EvidenceClaim::ArtifactLineage],
    );
    insert(
        &mut best,
        RequirementKind::DiscardArtifact,
        Enforcement::Enforced,
        &[EvidenceClaim::ArtifactLineage],
    );
    insert(
        &mut best,
        RequirementKind::ListOutputs,
        Enforcement::Enforced,
        &[EvidenceClaim::ArtifactLineage],
    );

    // Deny-all network: the guest gets no WASI socket capabilities at all.
    insert(
        &mut best,
        RequirementKind::NetworkDenyAll,
        Enforcement::Enforced,
        &[EvidenceClaim::DeniedAttempts],
    );

    // InheritedFds: a wasm guest inherits NO host fds (only its WASI preopens), so
    // `None` is structurally Enforced; `Only` has no WASI mechanism — Unsupported.
    insert(
        &mut best,
        RequirementKind::InheritedFdsNone,
        Enforcement::Enforced,
        &[EvidenceClaim::MechanismAttestation],
    );
    insert(
        &mut best,
        RequirementKind::InheritedFdsOnly,
        Enforcement::Unsupported,
        &[],
    );

    // STRUCTURALLY UNSUPPORTED — no native fork/kill/mount in a wasm guest, no
    // allow-list broker. Listed explicitly so the honesty is a stated answer.
    // The three FROZEN S6 child-task semantics: all structurally UNSUPPORTED — a wasm
    // guest has no native fork/thread-spawn. Stated explicitly per key.
    insert(
        &mut best,
        RequirementKind::ChildSpawnDenyNewTasks,
        Enforcement::Unsupported,
        &[],
    );
    insert(
        &mut best,
        RequirementKind::ChildSpawnAllowThreads,
        Enforcement::Unsupported,
        &[],
    );
    insert(
        &mut best,
        RequirementKind::ChildSpawnAllowDescendants,
        Enforcement::Unsupported,
        &[],
    );
    insert(
        &mut best,
        RequirementKind::Kill,
        Enforcement::Unsupported,
        &[],
    );
    insert(
        &mut best,
        RequirementKind::ExposePath,
        Enforcement::Unsupported,
        &[],
    );
    insert(
        &mut best,
        RequirementKind::NetworkAllowList,
        Enforcement::Unsupported,
        &[],
    );

    SupportMatrix::from_best_case(best)
}

fn insert(
    table: &mut BTreeMap<RequirementKind, SupportVerdict>,
    kind: RequirementKind,
    enforcement: Enforcement,
    evidence: &[EvidenceClaim],
) {
    table.insert(
        kind,
        SupportVerdict::new(enforcement, evidence.iter().copied().collect()),
    );
}

// The wasm runtime backend compiles only behind the feature (the interpreter is
// host-OS-independent, so it is NOT target_os-gated — only feature-gated).
#[cfg(feature = "backend-wasm")]
mod backend_impl;
#[cfg(feature = "backend-wasm")]
pub use backend_impl::WasmBackend;

#[cfg(test)]
mod tests {
    use super::support_matrix;
    use crate::contract::capability::Enforcement;
    use crate::contract::support::RequirementKind;

    #[test]
    fn spawn_kill_mount_and_allow_list_are_structurally_unsupported() {
        // SCOPE §4 load-bearing honest cells: no native fork/kill/mount/broker.
        let m = support_matrix();
        for kind in [
            RequirementKind::ChildSpawnDenyNewTasks,
            RequirementKind::ChildSpawnAllowThreads,
            RequirementKind::ChildSpawnAllowDescendants,
            RequirementKind::Kill,
            RequirementKind::ExposePath,
            RequirementKind::NetworkAllowList,
        ] {
            assert_eq!(
                m.best_case_for(kind).enforcement,
                Enforcement::Unsupported,
                "{kind:?} must be structurally Unsupported on wasm"
            );
        }
    }

    #[test]
    fn filesystem_is_enforced_via_preopen() {
        let m = support_matrix();
        assert_eq!(
            m.best_case_for(RequirementKind::Filesystem).enforcement,
            Enforcement::Enforced
        );
    }
}

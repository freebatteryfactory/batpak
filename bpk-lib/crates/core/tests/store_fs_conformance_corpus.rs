//! Published `StoreFs` conformance corpus, driven over every backend (#179).
//!
//! PROVES: `batpak::store::conformance` is the single, case-addressable body
//! that pins the documented backend contract. The production backends `RealFs`
//! and `MemFs` pass every non-crash obligation and qualify (never silently
//! skip) only the crash-family cases they cannot drive without hostile
//! controls; the reference virtual crash simulator `ShadowFs` is a full pass
//! across the whole corpus including the crash envelope.
//!
//! CATCHES: a divergent backend (create-new exclusivity violated) is reported
//! as a typed `Failed` verdict — the corpus REPORTS divergence, it does not
//! panic — so a passing report over a broken backend is impossible. It also
//! pins that a skip is a typed `Qualified`, never a pass, and that the case-id
//! surface round-trips and stays size-pinned to the versioned corpus.
//!
//! Feature gating (contract A12, overriding plan-02/F10's single top gate):
//! the corpus + `ShadowFs` live behind the clean `conformance-harness`
//! surface, so this file gates on that. Test 4 additionally exercises the
//! internal `SimFs` byte-fault layer, which stays behind `dangerous-test-hooks`
//! (implied by, and thus a superset of, `conformance-harness`).

#![cfg(feature = "conformance-harness")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use batpak::store::conformance::{
    run_all, run_case, run_cases, BackendFactory, CaseFamily, CaseVerdict, FreshBackend,
    MemFsFactory, Qualification, RealFsFactory, ShadowFsFactory, StoreFsConformanceCase,
};
use batpak::store::{
    CopyPreference, CowStrategyUsed, DirEntryInfo, FileStat, MemFs, StagedFile, StoreDirLockGuard,
    StoreError, StoreFile, StoreFs,
};

/// A deliberately-divergent backend: create-new exclusivity is violated (an
/// existing file is silently clobbered instead of refused). Everything else
/// delegates to a conforming `MemFs`. Migrated from
/// `tests/store_fs_backend_conformance.rs`, now consumed through the published
/// corpus as a RED factory (F19 dropped the in-tree copy).
struct BrokenExclusivityFs {
    inner: MemFs,
}

impl StoreFs for BrokenExclusivityFs {
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<DirEntryInfo>> {
        self.inner.read_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        // THE PLANTED DIVERGENCE: silently clobber instead of refusing.
        drop(self.inner.remove_file(path));
        self.inner.create_new_file(path)
    }

    fn open_file(&self, path: &Path) -> std::io::Result<Box<dyn StoreFile>> {
        self.inner.open_file(path)
    }

    fn sync_parent_dir(&self, path: &Path) -> Result<(), StoreError> {
        self.inner.sync_parent_dir(path)
    }

    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError> {
        self.inner.reject_symlink_leaf(path, purpose)
    }

    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.inner.read(path)
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }

    fn symlink_metadata(&self, path: &Path) -> std::io::Result<FileStat> {
        self.inner.symlink_metadata(path)
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: CopyPreference,
    ) -> std::io::Result<CowStrategyUsed> {
        self.inner.cow_copy_file(from, to, preference)
    }

    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        self.inner.copy(from, to)
    }

    fn metadata(&self, path: &Path) -> std::io::Result<FileStat> {
        self.inner.metadata(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)
    }

    fn named_temp_in(&self, dir: &Path) -> std::io::Result<Box<dyn StagedFile>> {
        self.inner.named_temp_in(dir)
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        self.inner.try_lock_store_dir(lock_path)
    }
}

/// Factory wrapping the RED backend: a fresh `MemFs`-backed clobbering FS per
/// case, no hostile controls.
struct BrokenFactory;

impl BackendFactory for BrokenFactory {
    fn backend_name(&self) -> String {
        "broken-create-new-exclusivity".into()
    }

    fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
        let broken = BrokenExclusivityFs { inner: MemFs::new() };
        let root = PathBuf::from(format!("/broken/{}", case.id()));
        broken
            .create_dir_all(&root)
            .expect("in-memory dir creation is infallible");
        Ok(FreshBackend {
            fs: Arc::new(broken),
            root,
            controls: None,
            keepalive: None,
        })
    }
}

#[test]
fn real_fs_full_corpus_passes_with_only_crash_qualifications(
) -> Result<(), Box<dyn std::error::Error>> {
    let report = run_all(&RealFsFactory);
    let failures = report.failures();
    assert!(
        failures.is_empty(),
        "RealFs must pass every non-crash obligation; failures: {failures:?}"
    );
    let qualified = report.qualified();
    // The 8 crash-family cases have no controls on RealFs, so each qualifies —
    // pinned so a corpus that silently stopped running the crash family cannot
    // pass vacuously (#197).
    assert_eq!(
        qualified.len(),
        8,
        "exactly the crash family may qualify on a controls-less RealFs: {qualified:?}"
    );
    for outcome in qualified {
        assert_eq!(
            outcome.case.family(),
            CaseFamily::Crash,
            "only crash-family cases may qualify without controls: {:?}",
            outcome.case
        );
    }
    Ok(())
}

#[test]
fn mem_fs_full_corpus_passes_with_only_crash_qualifications(
) -> Result<(), Box<dyn std::error::Error>> {
    let report = run_all(&MemFsFactory);
    let failures = report.failures();
    assert!(
        failures.is_empty(),
        "MemFs must pass every non-crash obligation; failures: {failures:?}"
    );
    let qualified = report.qualified();
    assert_eq!(
        qualified.len(),
        8,
        "exactly the crash family may qualify on a controls-less MemFs: {qualified:?}"
    );
    for outcome in qualified {
        assert_eq!(
            outcome.case.family(),
            CaseFamily::Crash,
            "only crash-family cases may qualify without controls: {:?}",
            outcome.case
        );
    }
    Ok(())
}

#[test]
fn shadow_fs_full_corpus_is_a_full_pass() {
    // ShadowFs is the namespace-truth crash simulator: it supplies hostile
    // controls, so the crash envelope is EXECUTED (not qualified) and the whole
    // corpus is a clean pass — the reference against which other backends'
    // crash outcomes are judged.
    let report = run_all(&ShadowFsFactory);
    assert!(
        report.full_pass(),
        "ShadowFs must fully pass the corpus; failures: {:?}, qualified: {:?}",
        report.failures(),
        report.qualified()
    );
}

#[test]
fn conformance_corpus_detects_a_divergent_backend() -> Result<(), Box<dyn std::error::Error>> {
    // RED-fixture discipline: a backend that violates create-new exclusivity
    // must produce a typed `Failed` verdict. The corpus reports divergence; it
    // does not panic (no catch_unwind needed). `CaseVerdict` is #[non_exhaustive]
    // (foreign to this test crate) so the wanted variant is extracted with
    // let-else, never a wildcard match.
    let outcome = run_case(&BrokenFactory, StoreFsConformanceCase::CreateNewExclusive);
    let CaseVerdict::Failed { reason } = &outcome.verdict else {
        return Err(std::io::Error::other(format!(
            "PROPERTY: a create-new exclusivity violation must yield Failed, got {:?}",
            outcome.verdict
        ))
        .into());
    };
    assert!(
        !reason.is_empty(),
        "a Failed verdict must carry a non-empty reason"
    );
    Ok(())
}

#[test]
fn case_ids_are_stable_and_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        StoreFsConformanceCase::ALL.len(),
        34,
        "corpus size is pinned; bump deliberately with CORPUS_VERSION"
    );
    let mut ids: Vec<&'static str> = Vec::new();
    for case in StoreFsConformanceCase::ALL {
        let id = case.id();
        assert_eq!(
            StoreFsConformanceCase::parse(id),
            Some(case),
            "id `{id}` must round-trip through parse"
        );
        ids.push(id);
    }
    ids.sort_unstable();
    let total = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), total, "case ids must be unique");
    Ok(())
}

#[test]
fn report_json_is_machine_readable() -> Result<(), Box<dyn std::error::Error>> {
    // serde_json is a production dep of batpak but NOT a dev-dependency, so this
    // test crate cannot parse the string back into a Value. Instead assert the
    // machine-readable schema keys are present in the pretty JSON `to_json`
    // emits — enough to prove the report serializes with its named fields.
    let report = run_cases(
        &MemFsFactory,
        &[
            StoreFsConformanceCase::CreateNewExclusive,
            StoreFsConformanceCase::HandleLengthReflectsSyncedBytes,
        ],
    );
    let json = report.to_json()?;
    for key in ["corpus_version", "batpak_version", "backend_name", "outcomes"] {
        assert!(
            json.contains(key),
            "report JSON must expose the `{key}` field: {json}"
        );
    }
    Ok(())
}

/// The internal `SimFs` byte-fault layer stays behind `dangerous-test-hooks`
/// (contract A12); it is exercised here over a virtual `MemFs` inner to prove
/// (a) the corpus runs a wrapped backend and (b) the dir-sync-drop control is
/// unrepresentable on the byte layer and surfaces as a TYPED qualification, not
/// a silent pass.
#[cfg(feature = "dangerous-test-hooks")]
mod sim_layer {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use batpak::store::conformance::{
        run_all, BackendFactory, CaseFamily, FreshBackend, HostileControls, Qualification,
        StoreFsConformanceCase,
    };
    use batpak::store::{CrashOp, MemFs, SimFs, StoreFs};

    /// Hostile controls over a shared `SimFs`. `SimFs` models directory-entry
    /// NAMES as always durable (namespace truth is `ShadowFs`'s job), so the
    /// parent-dir-sync-drop control is a typed qualification. `SimFs` exposes no
    /// fired-count accessor, so anti-vacuity is tracked locally: every armed
    /// fault is immediately triggered by the case that armed it, and the case
    /// independently asserts the faulted op erred.
    struct SimControls {
        sim: Arc<SimFs>,
        armed: AtomicU32,
    }

    impl HostileControls for SimControls {
        fn crash(&self) -> Result<(), Qualification> {
            self.sim.crash();
            Ok(())
        }

        fn drop_next_parent_dir_sync(&self) -> Result<(), Qualification> {
            Err(Qualification::ControlUnsupported {
                control: "drop_next_parent_dir_sync",
                backend: "sim-over-mem".into(),
                reason: "SimFs models directory-entry names as always durable".into(),
            })
        }

        fn fault_next_op(&self, op: CrashOp) -> Result<(), Qualification> {
            self.sim.arm_fault_on(op, 1);
            self.armed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn faults_fired(&self) -> u32 {
            self.armed.load(Ordering::Relaxed)
        }
    }

    struct SimOverMemFactory;

    impl BackendFactory for SimOverMemFactory {
        fn backend_name(&self) -> String {
            "sim-over-mem".into()
        }

        fn fresh(&self, case: StoreFsConformanceCase) -> Result<FreshBackend, Qualification> {
            // fsync_drop_one_in = 0: no background sync drops, so crash outcomes
            // are driven only by the explicit corpus arming — deterministic.
            let sim = Arc::new(SimFs::layered(0x5142, 0, Arc::new(MemFs::new())));
            let root = PathBuf::from(format!("/sim-conformance/{}", case.id()));
            sim.create_dir_all(&root)
                .expect("in-memory dir creation through the sim layer is infallible");
            let controls = SimControls {
                sim: Arc::clone(&sim),
                armed: AtomicU32::new(0),
            };
            Ok(FreshBackend {
                fs: sim,
                root,
                controls: Some(Arc::new(controls)),
                keepalive: None,
            })
        }
    }

    #[test]
    fn sim_layer_over_mem_fs_runs_wrapped_backend_with_typed_dir_sync_qualification(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = run_all(&SimOverMemFactory);
        let failures = report.failures();
        assert!(
            failures.is_empty(),
            "the SimFs byte layer over MemFs must pass every obligation it can run; \
             failures: {failures:?}"
        );

        // EXACTLY the three cases that arm a dropped parent-dir sync qualify —
        // the byte layer cannot represent that control. Never a silent skip,
        // never a spurious extra qualification.
        let expected = [
            StoreFsConformanceCase::RenameWithoutParentSyncCrashOldOrNew,
            StoreFsConformanceCase::RemoveWithoutParentSyncCrashMayResurrect,
            StoreFsConformanceCase::PersistWithoutParentSyncCrashOldOrNew,
        ];
        let qualified = report.qualified();
        assert_eq!(
            qualified.len(),
            expected.len(),
            "only the dir-sync-drop cases may qualify on the byte layer: {qualified:?}"
        );
        for case in expected {
            assert!(
                qualified.iter().any(|outcome| outcome.case == case),
                "case {case:?} must qualify (needs the unrepresentable dir-sync-drop control)"
            );
            assert_eq!(case.family(), CaseFamily::Crash);
        }
        Ok(())
    }
}

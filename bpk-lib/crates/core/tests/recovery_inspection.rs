//! PROVES: the public non-opening forensic door `Store::inspect_recovery_state`
//! (contract A14, #180/#177) reads a directory's recovery state WITHOUT
//! constructing `Store<Open>` and WITHOUT mutating a single byte on disk —
//! reporting `store.meta` presence/lineage, the coarse writable action, and a
//! legacy-marker refusal as a captured report rather than an error, while a
//! corrupt `store.meta` surfaces the SAME typed failure a real open would.
//! CATCHES: an inspection that opens or writes the directory, swallows a legacy
//! marker into `NoneRequired`, or papers a corrupt store identity over instead
//! of failing closed.
//! SEEDED: deterministic tempdir fixtures + a real store lifecycle; no randomness.
//!
//! Scope: this file drives only the states reachable through the PUBLIC surface
//! (empty dir, a legitimately-opened store, a legacy JSON marker, a corrupt
//! `store.meta`). The marker-present roll-forward/roll-back agreement facts
//! (`WritableRepairRequired` + `committed_generation`) require crate-internal
//! marker construction and are proven by the `recovery_inspect` src unit-test
//! module; here we assert the public door's contract, not the classifier.

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::store::{
    RecoveryInspection, RequiredRecoveryAction, Store, StoreConfig, StoreError,
};
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xA, 1);

/// The public associated fn, invoked exactly as an operator would (default
/// `RealFs` config). Also the call site `store_pub_fn_coverage` requires.
fn inspect(config: &StoreConfig) -> RecoveryInspection {
    Store::inspect_recovery_state(config).expect("PROPERTY: inspection renders a report")
}

#[test]
fn public_door_inspects_fresh_directory_without_opening() {
    // A directory that never held a store: no `store.meta`, no marker, nothing
    // to recover. The public door returns a full report, never `Store<Open>`.
    let dir = TempDir::new().expect("tempdir");
    let config = StoreConfig::new(dir.path());

    let report = inspect(&config);

    assert_eq!(report.data_dir, dir.path().to_path_buf());
    assert!(!report.store_meta_present, "a fresh dir has no store.meta");
    assert_eq!(report.store_lineage, None, "no store.meta means no lineage");
    assert_eq!(report.committed_generation, None, "nothing committed yet");
    assert_eq!(report.authorized_image, None, "no authority obligations");
    assert_eq!(report.pending_marker, None, "no pending transaction");
    assert_eq!(
        report.required_action,
        RequiredRecoveryAction::NoneRequired,
        "PROPERTY: a marker-free directory requires no writable recovery"
    );
}

#[test]
fn inspection_neither_opens_nor_mutates_the_directory() {
    // The forensic contract: inspection must not create `store.meta`, take the
    // dir lock, or otherwise touch disk. Assert the directory contents are byte
    // identical (here: empty) across the call.
    let dir = TempDir::new().expect("tempdir");
    let config = StoreConfig::new(dir.path());
    let before = entry_count(dir.path());
    assert_eq!(before, 0, "fixture starts empty");

    let report = inspect(&config);
    assert_eq!(
        report.required_action,
        RequiredRecoveryAction::NoneRequired
    );

    let after = entry_count(dir.path());
    assert_eq!(
        after, 0,
        "PROPERTY: inspection created no files (before {before}, after {after})"
    );
}

#[test]
fn inspection_reads_a_real_store_meta_without_opening_it() {
    // A store opened, written, and cleanly closed writes `store.meta` (lineage
    // minted at writable open). Inspecting the CLOSED directory reads that meta
    // — present, with a 32-hex lineage — without reopening, and finds no pending
    // transaction on a clean close.
    let dir = TempDir::new().expect("tempdir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let coord = Coordinate::new("account", "ledger").expect("valid coordinate");
    for i in 0..3u32 {
        let _ = store
            .append(&coord, KIND, &serde_json::json!({ "i": i }))
            .expect("append event");
    }
    store.close().expect("clean close");

    let config = StoreConfig::new(dir.path());
    let report = inspect(&config);

    assert!(
        report.store_meta_present,
        "PROPERTY: a real store's close leaves a readable store.meta"
    );
    let lineage = report
        .store_lineage
        .expect("PROPERTY: a present store.meta carries a lineage");
    assert_eq!(lineage.len(), 32, "ids render as fixed 32-hex; got {lineage}");
    assert!(
        lineage.chars().all(|c| c.is_ascii_hexdigit()),
        "lineage is lowercase hex; got {lineage}"
    );
    assert_eq!(report.pending_marker, None, "a clean close has no pending marker");
    assert_eq!(
        report.required_action,
        RequiredRecoveryAction::NoneRequired,
        "PROPERTY: a cleanly-closed store requires no recovery"
    );
}

#[test]
fn legacy_json_marker_is_reported_as_refuse_not_an_error() {
    // A pre-token JSON marker (`compaction.pending.json`) is never recovered
    // automatically. Inspection must CAPTURE the load's refusal as a `Refuse`
    // action carrying an operator-facing message — not bail the whole report.
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("compaction.pending.json"),
        br#"{"merged_id":7,"source_segment_ids":[1,2,7]}"#,
    )
    .expect("write legacy JSON marker");
    let config = StoreConfig::new(dir.path());

    let report = inspect(&config);
    assert_eq!(
        report.pending_marker, None,
        "a refused marker is never summarized as trustworthy"
    );
    match report.required_action {
        RequiredRecoveryAction::Refuse { refusal } => {
            assert!(
                refusal.marker_path.ends_with("compaction.pending.json"),
                "PROPERTY: the refusal names the legacy marker path; got {}",
                refusal.marker_path.display()
            );
            assert!(
                !refusal.message.is_empty(),
                "PROPERTY: the refusal carries an operator-facing message"
            );
        }
        RequiredRecoveryAction::NoneRequired | RequiredRecoveryAction::WritableRepairRequired => {
            assert!(
                std::hint::black_box(false),
                "PROPERTY: a legacy JSON marker must report a Refuse action, not {:?}",
                report.required_action
            )
        }
    }
}

#[test]
fn corrupt_store_meta_surfaces_the_same_typed_failure_an_open_would() {
    // A garbled `store.meta` is a hard identity corruption. Inspection must fail
    // closed with the EXACT typed error a writable open surfaces, never a
    // silent default — the store's identity is not something inspection guesses.
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("store.meta"), b"this is not a valid store.meta prefix")
        .expect("write corrupt store.meta");
    let config = StoreConfig::new(dir.path());

    // `RecoveryInspection` derives `Debug`, so `expect_err` is available here
    // (unlike `Store`/`Receipt`); assert the EXACT variant, not merely "errored".
    let err = Store::inspect_recovery_state(&config)
        .expect_err("PROPERTY: corrupt store.meta must be a typed failure");
    assert!(
        matches!(err, StoreError::StoreMetadataCorrupt { .. }),
        "PROPERTY: a corrupt store.meta is StoreMetadataCorrupt; got {err:?}"
    );
}

/// Count of directory entries — the read-only witness that inspection created
/// nothing.
fn entry_count(path: &std::path::Path) -> usize {
    std::fs::read_dir(path)
        .expect("read data dir")
        .filter_map(Result::ok)
        .count()
}

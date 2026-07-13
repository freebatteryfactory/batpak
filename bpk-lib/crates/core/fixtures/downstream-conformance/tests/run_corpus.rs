//! Out-of-tree acceptance for the published StoreFs conformance corpus (#179).
//!
//! This test lives in a STANDALONE workspace that depends on `batpak` the way a
//! crates.io consumer would — through the clean external `conformance-harness`
//! feature alone, with no access to the internal `dangerous-test-hooks` fault or
//! poison levers (contract A12). It proves a downstream backend author can run
//! `run_all` over their own `BackendFactory` and publish a machine-readable
//! report as a preserved build artifact.
//!
//! `DownstreamFactory` supplies no hostile controls, so every Crash-family case
//! is TYPED-QUALIFIED (a skip is never a pass) while the byte/namespace contract
//! cases must fully pass.

use std::path::Path;

use batpak::store::conformance::{run_all, CaseFamily, StoreFsConformanceCase};
use batpak_storefs_conformance_downstream_fixture::DownstreamFactory;

#[test]
fn downstream_backend_upholds_the_published_conformance_corpus() {
    let report = run_all(&DownstreamFactory);

    // Anti-vacuity: the whole corpus ran, not zero cases.
    assert_eq!(
        report.outcomes.len(),
        StoreFsConformanceCase::ALL.len(),
        "corpus did not run every case",
    );

    // No obligation may be VIOLATED by the delegating out-of-tree backend.
    assert!(
        report.failures().is_empty(),
        "downstream backend violated the StoreFs contract: {:#?}",
        report.failures(),
    );

    // The only permitted qualifications are crash-control unavailability: a
    // MemFs-delegate factory declares no HostileControls, so exactly the Crash
    // family qualifies. Any other qualified family would be a masked skip.
    for outcome in report.qualified() {
        assert_eq!(
            outcome.case.family(),
            CaseFamily::Crash,
            "non-crash case `{}` was qualified (a skip is never a pass): {:?}",
            outcome.case.id(),
            outcome.verdict,
        );
    }

    // Preserve the report as a build artifact (the #179 receipt addendum): a
    // downstream author publishes this JSON to attest their backend's posture.
    let json = report
        .to_json()
        .expect("serialize the conformance report to JSON");
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::create_dir_all(&out_dir).expect("create the target dir for the report artifact");
    std::fs::write(out_dir.join("storefs-conformance-report.json"), json)
        .expect("write the conformance report artifact");
}

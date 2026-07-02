//! Round-3 mutation kills for the repo-wide NO-DEFAULT-FEATURES shard —
//! read-walk input-frontier seam. Runs unchanged on every feature surface
//! (public API only, nothing feature-gated).
//!
//! PROVES: `Store::query_with_read_walk_evidence` reports the input frontier
//! honestly on BOTH sides of the empty-store boundary: (1) a store with a
//! ZERO visible upper bound — publicly reachable only as a READ-ONLY store
//! over an empty data directory, since a writable open always appends a
//! visible `SYSTEM_OPEN_COMPLETED` event — reports the ORIGIN frontier
//! (wall_ms 0, global_sequence 0) as a KNOWN frontier with NO findings,
//! never a false `InputFrontierUnknown`; (2) a populated store reports the
//! frontier of the LAST VISIBLE event (its exact global_sequence), never the
//! ORIGIN sentinel.
//! CATCHES: read_walk.rs:288 `==` -> `!=` in the `visible_upper_bound == 0`
//! guard of `query_with_read_walk_evidence`. The flipped guard sends an empty
//! store down the HLC-lookup arm (no entry resolves -> frontier None + a false
//! `InputFrontierUnknown` finding) and sends a populated store down the
//! ORIGIN arm (the real frontier is laundered to sequence 0).
//! SEEDED: deterministic — fixed payloads, no randomness, no wall-clock
//! assertions (the populated-store pin is on global_sequence, not wall_ms).

use batpak_testkit::prelude::*;
use batpak_testkit::small_store as small_store_support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn empty_store_read_walk_reports_origin_frontier_with_no_findings() -> TestResult {
    // A writable open appends SYSTEM_OPEN_COMPLETED, so the zero-visible
    // boundary is reached the only way the public API allows: a read-only
    // store over a data directory that has never committed an event.
    let data_dir_guard = tempfile::TempDir::new()?;
    let config = small_store_support::small_segment_store_config(data_dir_guard.path());
    let store = Store::<ReadOnly>::open_read_only(config)?;

    let request = ReadWalkRequest::full(Region::all());
    let (entries, report) = store.query_with_read_walk_evidence(&request)?;

    assert_eq!(entries.len(), 0, "an empty store returns no entries");
    assert_eq!(report.body.matched_count, 0);
    assert_eq!(
        report.body.input_frontier,
        Some(ReadWalkInputFrontier {
            kind: ReadWalkFrontierKind::Visible,
            wall_ms: 0,
            global_sequence: 0,
        }),
        "PROPERTY: an empty store (visible upper bound 0) reports the ORIGIN \
         frontier as KNOWN; the flipped `!=` guard resolves no HLC entry and \
         degrades the frontier to None instead"
    );
    assert_eq!(
        report.body.findings,
        Vec::<ReadWalkFinding>::new(),
        "an unlimited full-region walk over an empty store has NO findings; \
         InputFrontierUnknown here is the mutant's false alarm"
    );
    Ok(())
}

#[test]
fn populated_store_read_walk_reports_the_last_visible_sequence_not_origin() -> TestResult {
    let (data_dir_guard, store) = small_store_support::small_segment_store()?;
    assert!(data_dir_guard.path().exists());
    let coord = Coordinate::new("entity:readwalk:frontier", "scope:frontier")?;
    let kind = EventKind::custom(0xE, 0x43);
    let mut last_sequence = 0_u64;
    for n in 0..3_u64 {
        let receipt = store.append(&coord, kind, &serde_json::json!({ "n": n }))?;
        last_sequence = receipt.global_sequence;
    }
    assert_ne!(
        last_sequence, 0,
        "seed sanity: three appends must advance past the origin sequence"
    );

    let request = ReadWalkRequest::full(Region::scope("scope:frontier"));
    let (entries, report) = store.query_with_read_walk_evidence(&request)?;
    assert_eq!(entries.len(), 3);

    let frontier = report
        .body
        .input_frontier
        .expect("a populated store must resolve a Known input frontier");
    assert_eq!(frontier.kind, ReadWalkFrontierKind::Visible);
    assert_eq!(
        frontier.global_sequence, last_sequence,
        "PROPERTY: the input frontier names the LAST VISIBLE event's exact \
         global_sequence; the flipped `!=` guard reports the ORIGIN sentinel \
         (sequence 0) for every non-empty store"
    );
    assert!(
        !report
            .body
            .findings
            .iter()
            .any(|finding| matches!(finding, ReadWalkFinding::InputFrontierUnknown)),
        "a resolvable frontier must never be reported as unknown"
    );
    Ok(())
}

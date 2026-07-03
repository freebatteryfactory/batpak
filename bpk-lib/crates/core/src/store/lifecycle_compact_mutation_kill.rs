//! PROVES: the `compact` orchestration's decision guard and reclaim accounting
//! against a real store laid out with exactly two SEALED segments and one active
//! tail. The sealed/active split (`*id < active_segment_id`) and the skip guard
//! (`sealed.len() < config.min_segments`) together decide Perform vs Skip, and
//! the removal loop tallies `segments_removed`/`bytes_reclaimed`.
//! CATCHES: the skip-guard comparison mutants (`<`->`<=`/`==` would skip a
//! runnable compaction; `<`->`>` would run a sub-threshold one), the sealed
//! filter comparison mutants (miscounting `sealed_segment_count`), and the loop
//! `segments_removed += 1` / `bytes_reclaimed += meta.len()` arithmetic mutants
//! (`-=` underflows, `*=` sticks at zero).
//! SEEDED: deterministic — fixed payloads, a 512-byte segment cap, and
//! sync-every-1 make the two-sealed layout reproducible; no randomness/clock.

use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::store::segment::CompactionOutcome;
use crate::store::{CompactionConfig, CompactionStrategy, Store, StoreConfig};

const KIND: EventKind = EventKind::custom(0xC, 0x01);

/// Build a store whose layout guarantees exactly two SEALED segments (ids below
/// the active tail) plus a separate active segment. Each oversized filler frame
/// exceeds `segment_max_bytes`, so the following append rotates the segment.
fn store_with_two_sealed_segments() -> (tempfile::TempDir, Store) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let config = StoreConfig::new(dir.path())
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1);
    let store = Store::open(config).expect("open store");
    let coord = |entity: &str| Coordinate::new(entity, "scope:compact-guard").expect("coord");
    let big = serde_json::json!({ "blob": "x".repeat(2000) });

    // seg0: small + oversized filler (rotates on the next append).
    let _ = store
        .append(&coord("entity:a"), KIND, &serde_json::json!({ "n": 1 }))
        .expect("append a1");
    let _ = store
        .append(&coord("entity:a"), KIND, &big)
        .expect("append filler 1");
    // seg1: small + oversized filler (rotates on the next append).
    let _ = store
        .append(&coord("entity:b"), KIND, &serde_json::json!({ "n": 2 }))
        .expect("append b1");
    let _ = store
        .append(&coord("entity:b"), KIND, &big)
        .expect("append filler 2");
    // seg2 (active): anchor keeps seg0/seg1 strictly in the sealed set.
    let _ = store
        .append(&coord("entity:c"), KIND, &serde_json::json!({ "n": 3 }))
        .expect("append anchor");

    (dir, store)
}

#[test]
fn compact_performs_at_the_threshold_and_tallies_the_reclaimed_sealed_segments() {
    // Two sealed segments with min_segments == 2: `2 < 2` is false, so compaction
    // must PERFORM. The `<`->`<=` and `<`->`==` skip-guard mutants make `2 (<=|==)
    // 2` true and wrongly skip. The removal loop must count both sealed files and
    // reclaim a positive byte total.
    let (_dir, store) = store_with_two_sealed_segments();
    let (result, report) = store
        .compact(&CompactionConfig {
            strategy: CompactionStrategy::Merge,
            min_segments: 2,
        })
        .expect("compact runs");

    assert!(
        matches!(result.outcome, CompactionOutcome::Performed),
        "2 sealed >= min_segments 2 must PERFORM; the `<`->`<=`/`==` guard mutants skip it"
    );
    assert_eq!(
        report.sealed_segment_count, 2,
        "exactly the two sub-active segments are sources; the sealed filter \
         `*id < active` -> `<=`/`==`/`>` mutant miscounts them"
    );
    assert_eq!(
        report.segments_removed, 2,
        "both sealed files are removed; `segments_removed += 1` -> `-=`/`*=` miscounts"
    );
    assert!(
        report.bytes_reclaimed > 0,
        "the reclaim total sums the removed sealed sizes; `bytes_reclaimed += meta.len()` \
         -> `*=` sticks it at zero (and `-=` underflows)"
    );
    assert_eq!(
        report.segments_removed, result.segments_removed,
        "the report mirrors the engine result's removed count"
    );
    assert_eq!(
        report.bytes_reclaimed, result.bytes_reclaimed,
        "the report mirrors the engine result's reclaimed bytes"
    );
    assert_eq!(report.outcome, CompactionOutcome::Performed);
}

#[test]
fn compact_skips_below_the_threshold_without_touching_the_reclaim_columns() {
    // Two sealed segments with min_segments == 3: `2 < 3` is true, so compaction
    // must SKIP. The `<`->`>` skip-guard mutant makes `2 > 3` false and wrongly
    // performs. A skip still reports both sealed sources but zero reclaim.
    let (_dir, store) = store_with_two_sealed_segments();
    let (result, report) = store
        .compact(&CompactionConfig {
            strategy: CompactionStrategy::Merge,
            min_segments: 3,
        })
        .expect("compact runs");

    assert!(
        matches!(result.outcome, CompactionOutcome::Skipped),
        "2 sealed < min_segments 3 must SKIP; the `<`->`>` guard mutant performs it"
    );
    assert_eq!(
        report.sealed_segment_count, 2,
        "the skip report still counts the two sealed sources it declined to merge"
    );
    assert_eq!(report.segments_removed, 0, "a skip removes no segments");
    assert_eq!(report.bytes_reclaimed, 0, "a skip reclaims no bytes");
    assert_eq!(report.outcome, CompactionOutcome::Skipped);
    assert!(
        report.merged_segment_id.is_none(),
        "a skip never nominates a merged output segment"
    );
}

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

/// The deterministic sealed-segment count the fixture produces, learned by
/// performing a compaction with `min_segments == 1` on a throwaway store (which
/// always performs when >= 1 segment is sealed). Deriving `n` — instead of
/// hard-coding the exact rotation count — keeps the two boundary tests robust to
/// the segment-rotation heuristic while still pinning the `sealed.len() < min`
/// guard at its EXACT threshold (`min == n` performs, `min == n+1` skips).
fn deterministic_sealed_count() -> usize {
    let (_dir, store) = store_with_two_sealed_segments();
    let (_result, report) = store
        .compact(&CompactionConfig {
            strategy: CompactionStrategy::Merge,
            min_segments: 1,
        })
        .expect("probe compact runs");
    assert!(
        matches!(report.outcome, CompactionOutcome::Performed),
        "min_segments 1 must PERFORM whenever the fixture seals >= 1 segment"
    );
    report.sealed_segment_count
}

#[test]
fn compact_performs_at_the_threshold_and_tallies_the_reclaimed_sealed_segments() {
    // At the EXACT threshold `min_segments == sealed count n`: `n < n` is false, so
    // compaction must PERFORM. The `<`->`<=`/`==` skip-guard mutants make `n (<=|==)
    // n` true and wrongly skip. The removal loop removes EVERY sealed source, so
    // `segments_removed == sealed_segment_count == n` and the reclaim is positive.
    let n = deterministic_sealed_count();
    assert!(
        n >= 2,
        "the fixture seals multiple segments (each oversized filler rotates)"
    );

    let (_dir, store) = store_with_two_sealed_segments();
    let (result, report) = store
        .compact(&CompactionConfig {
            strategy: CompactionStrategy::Merge,
            min_segments: n,
        })
        .expect("compact runs");

    assert!(
        matches!(result.outcome, CompactionOutcome::Performed),
        "n sealed >= min_segments n must PERFORM; the `<`->`<=`/`==` guard mutants skip it"
    );
    assert_eq!(
        report.sealed_segment_count, n,
        "the sealed filter `*id < active` counts every sub-active segment"
    );
    assert_eq!(
        report.segments_removed, n,
        "the removal loop removes every sealed source; `segments_removed += 1` -> `-=`/`*=` miscounts"
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
    // One ABOVE the sealed count `min_segments == n + 1`: `n < n+1` is true, so
    // compaction must SKIP. The `<`->`>` skip-guard mutant makes `n > n+1` false and
    // wrongly performs. A skip still reports the sealed sources but zero reclaim.
    let n = deterministic_sealed_count();
    assert!(n >= 2, "the fixture seals multiple segments");

    let (_dir, store) = store_with_two_sealed_segments();
    let (result, report) = store
        .compact(&CompactionConfig {
            strategy: CompactionStrategy::Merge,
            min_segments: n + 1,
        })
        .expect("compact runs");

    assert!(
        matches!(result.outcome, CompactionOutcome::Skipped),
        "n sealed < min_segments n+1 must SKIP; the `<`->`>` guard mutant performs it"
    );
    assert_eq!(
        report.sealed_segment_count, n,
        "the skip report still counts the sealed sources it declined to merge"
    );
    assert_eq!(report.segments_removed, 0, "a skip removes no segments");
    assert_eq!(report.bytes_reclaimed, 0, "a skip reclaims no bytes");
    assert_eq!(report.outcome, CompactionOutcome::Skipped);
    assert!(
        report.merged_segment_id.is_none(),
        "a skip never nominates a merged output segment"
    );
}

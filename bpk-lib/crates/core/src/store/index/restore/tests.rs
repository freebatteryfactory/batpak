use super::*;
use crate::coordinate::Coordinate;
use crate::event::{EventKind, HashChain};
use crate::store::index::{interner::InternId, DiskPos};
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

fn entry(seq: u64, entity: &str) -> IndexEntry {
    IndexEntry {
        event_id: u128::from(seq),
        correlation_id: u128::from(seq),
        causation_id: None,
        coord: Coordinate::new(entity, "scope").expect("coordinate"),
        entity_id: InternId::sentinel(),
        scope_id: InternId::sentinel(),
        kind: EventKind::custom(0x1, 1),
        wall_ms: seq,
        clock: u32::try_from(seq).expect("test sequence fits u32"),
        dag_lane: 0,
        dag_depth: 0,
        hash_chain: HashChain::default(),
        disk_pos: DiskPos::new(0, seq * 64, 64),
        global_sequence: seq,
        receipt_extensions: BTreeMap::new(),
    }
}

fn sorted_arcs(entries: Vec<IndexEntry>) -> (Vec<Arc<IndexEntry>>, Vec<Arc<IndexEntry>>) {
    let entries_by_sequence: Vec<_> = entries.into_iter().map(Arc::new).collect();
    let mut entries_by_entity = entries_by_sequence.clone();
    sort_entries_by_entity(&mut entries_by_entity);
    (entries_by_sequence, entries_by_entity)
}

#[test]
fn restore_chunk_ranges_uses_valid_persisted_chunks() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let routing = RoutingSummary::from_sorted_entries(&entries, 2);

    assert_eq!(
        restore_chunk_ranges(entries.len(), &routing),
        vec![(0, 2), (2, 2)]
    );
}

#[test]
fn restore_chunk_ranges_falls_back_for_malformed_chunks() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let mut routing = RoutingSummary::from_sorted_entries(&entries, 2);
    routing.chunks[1].start = 3;

    assert_eq!(restore_chunk_ranges(entries.len(), &routing), vec![(0, 4)]);
}

#[test]
fn restore_chunk_ranges_falls_back_on_a_single_violated_condition() {
    // The fallback guard is `len == 0 || start != expected_start || end > entry_count`.
    // The existing malformed-chunk test trips TWO conditions at once, so the
    // `|| -> &&` mutant (between the start-mismatch and out-of-bounds terms) still
    // falls back. Here a chunk is non-empty AND in-bounds (end == entry_count) but
    // start-discontinuous, so ONLY the middle condition holds: the real `||` falls
    // back to even ranges, while the `&&` mutant wrongly accepts the bad chunk.
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let mut routing = RoutingSummary::from_sorted_entries(&entries, 1);
    routing.chunks[0].start = 1; // != expected_start (0)
    routing.chunks[0].len = 3; // end = 1 + 3 = 4 == entry_count (NOT > )

    assert_eq!(
        restore_chunk_ranges(entries.len(), &routing),
        vec![(0, 4)],
        "a start-discontinuous but in-bounds chunk must fall back to even ranges; the \
         `|| -> &&` mutant would instead accept the bad chunk and yield [(1, 3)]"
    );
}

#[test]
fn routing_summary_entity_run_scan_makes_forward_progress() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name("routing-summary-progress-regression".to_owned())
        .spawn(move || {
            let summary = RoutingSummary::from_sorted_entries(&entries, 2);
            tx.send(summary.entity_runs)
                .expect("routing summary receiver is alive");
        })
        .expect("spawn routing summary progress regression thread");

    let runs = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("PROPERTY: routing summary entity scan must not stall");
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].entity, "alpha");
    assert_eq!(runs[0].len, 2);
    assert_eq!(runs[1].entity, "beta");
    assert_eq!(runs[1].len, 2);
}

#[test]
fn routing_summary_validate_accepts_in_bounds_entity_runs() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let summary = RoutingSummary::from_sorted_entries(&entries, 2);
    let (entries_by_sequence, entries_by_entity) = sorted_arcs(entries);

    assert!(
        summary.validate(&entries_by_sequence, &entries_by_entity),
        "PROPERTY: valid in-bounds entity runs must validate; a run ending before the full entity array length is still valid when its own slice is correct"
    );
    assert_eq!(
        summary.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Valid
    );
}

#[test]
fn routing_summary_validate_rejects_chunk_boundary_mismatches_independently() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let summary = RoutingSummary::from_sorted_entries(&entries, 2);
    let (entries_by_sequence, entries_by_entity) = sorted_arcs(entries);

    let mut wrong_first = summary.clone();
    wrong_first.chunks[0].first_sequence += 100;
    assert_eq!(
        wrong_first.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::ChunkFirstSequenceMismatch)
    );
    assert!(
        !wrong_first.validate(&entries_by_sequence, &entries_by_entity),
        "PROPERTY: chunk validation must reject a mismatched first sequence even when the last sequence still matches"
    );

    let mut wrong_last = summary;
    wrong_last.chunks[0].last_sequence += 100;
    assert_eq!(
        wrong_last.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::ChunkLastSequenceMismatch)
    );
    assert!(
        !wrong_last.validate(&entries_by_sequence, &entries_by_entity),
        "PROPERTY: chunk validation must reject a mismatched last sequence even when the first sequence still matches"
    );
}

#[test]
fn routing_summary_validate_rejects_empty_or_out_of_bounds_entity_runs() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let summary = RoutingSummary::from_sorted_entries(&entries, 2);
    let (entries_by_sequence, entries_by_entity) = sorted_arcs(entries);

    let mut empty_run = summary.clone();
    empty_run.entity_runs[0].len = 0;
    assert_eq!(
        empty_run.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::EntityRunLenZero)
    );
    assert!(
        !empty_run.validate(&entries_by_sequence, &entries_by_entity),
        "PROPERTY: zero-length entity runs are invalid and must not be accepted as harmless no-ops"
    );

    let mut out_of_bounds_run = summary;
    out_of_bounds_run.entity_runs[0].start = entries_by_entity.len() as u64;
    out_of_bounds_run.entity_runs[0].len = 1;
    assert_eq!(
        out_of_bounds_run.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::EntityRunEndOutOfBounds)
    );
    assert!(
        !out_of_bounds_run.validate(&entries_by_sequence, &entries_by_entity),
        "PROPERTY: entity runs whose end exceeds the entity-sorted table are invalid"
    );
}

#[test]
fn routing_summary_validate_detailed_reports_count_and_total_mismatches() {
    let entries = vec![
        entry(0, "alpha"),
        entry(1, "alpha"),
        entry(2, "beta"),
        entry(3, "beta"),
    ];
    let summary = RoutingSummary::from_sorted_entries(&entries, 2);
    let (entries_by_sequence, entries_by_entity) = sorted_arcs(entries);

    let mut wrong_entry_count = summary.clone();
    wrong_entry_count.entry_count += 1;
    assert_eq!(
        wrong_entry_count.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::EntryCountMismatch)
    );

    let mut wrong_chunk_count = summary.clone();
    wrong_chunk_count.chunk_count += 1;
    assert_eq!(
        wrong_chunk_count.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::ChunkCountMismatch)
    );

    let mut missing_chunk = summary.clone();
    missing_chunk.chunks.pop();
    missing_chunk.chunk_count = missing_chunk.chunks.len() as u64;
    assert_eq!(
        missing_chunk.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::ChunkTotalMismatch)
    );

    let mut missing_run = summary;
    missing_run.entity_runs.pop();
    assert_eq!(
        missing_run.validate_detailed(&entries_by_sequence, &entries_by_entity),
        RoutingValidation::Invalid(RoutingValidationError::EntityRunTotalMismatch)
    );
}

// ── Mutation-kill: recommended_restore_chunk_count boundaries ──────────────────
//
// `recommended_restore_chunk_count(n) = n.div_ceil(65_536).clamp(1, 32)`.
// The clamp floor of 1 and ceiling of 32, plus the 65_536-per-chunk step, are
// the load-bearing constants. Pinning the exact boundary values kills the
// whole-body return replacements (`-> 0`, `-> 1`) and any drift in the clamp
// bounds.
#[test]
fn recommended_restore_chunk_count_clamps_to_the_1_to_32_window() {
    assert_eq!(
        recommended_restore_chunk_count(0),
        1,
        "an empty corpus still needs one chunk (kills `-> 0` and a dropped clamp floor)"
    );
    assert_eq!(
        recommended_restore_chunk_count(1),
        1,
        "a single entry is one chunk"
    );
    assert_eq!(
        recommended_restore_chunk_count(65_536),
        1,
        "exactly one chunk's worth of entries stays a single chunk (div_ceil boundary)"
    );
    assert_eq!(
        recommended_restore_chunk_count(65_537),
        2,
        "one entry past the chunk step rolls to two chunks (kills `-> 1` and a `<`/`<=` drift)"
    );
    assert_eq!(
        recommended_restore_chunk_count(65_536 * 32),
        32,
        "32 chunks' worth of entries hits the clamp ceiling exactly"
    );
    assert_eq!(
        recommended_restore_chunk_count(65_536 * 32 + 1),
        32,
        "one entry past the ceiling is clamped back to 32 (kills a dropped clamp ceiling)"
    );
    assert_eq!(
        recommended_restore_chunk_count(usize::MAX),
        32,
        "an absurd corpus size never exceeds the 32-chunk ceiling"
    );
}

// ── Mutation-kill: from_entries chunk-split remainder arithmetic ───────────────
//
// The chunk splitter computes `base = len / chunk_count`, `remainder = len %
// chunk_count`, `len_i = base + (chunk_index < remainder)`, `end = start +
// len_i`, and reads `last_sequence` from `entries_by_sequence[end - 1]`. A
// remainder-bearing split (5 entries into 2 chunks → [3, 2]) is the only shape
// that forces each of `/`, `%`, `+`, and the `chunk_index < remainder`
// comparison to be exactly right; an even split hides all of them.
#[test]
fn routing_summary_from_entries_splits_the_remainder_into_the_leading_chunk() {
    let entries = vec![
        entry(0, "solo"),
        entry(1, "solo"),
        entry(2, "solo"),
        entry(3, "solo"),
        entry(4, "solo"),
    ];
    let summary = RoutingSummary::from_sorted_entries(&entries, 2);

    assert_eq!(
        summary.chunk_count, 2,
        "five entries requested as two chunks must produce two chunks"
    );
    assert_eq!(summary.chunks.len(), 2);

    // base = 5 / 2 = 2, remainder = 5 % 2 = 1: chunk 0 gets base + 1 = 3, chunk
    // 1 gets base + 0 = 2. Any of `/ -> *`, `% -> /`, `+ -> -`, or `< -> <=/>`
    // shifts these lengths (or panics on an out-of-bounds slice).
    assert_eq!(summary.chunks[0].start, 0);
    assert_eq!(
        summary.chunks[0].len, 3,
        "the leading chunk absorbs the remainder (kills the base/remainder/`<` mutants)"
    );
    assert_eq!(
        summary.chunks[0].first_sequence, 0,
        "chunk 0 starts at the first sequence"
    );
    assert_eq!(
        summary.chunks[0].last_sequence, 2,
        "chunk 0 ends at entries[start + len - 1]; kills the `end - 1` -> `end + 1` mutant"
    );
    assert_eq!(
        summary.chunks[1].start, 3,
        "chunk 1 starts where chunk 0 ended; kills the `end = start + len` mutant"
    );
    assert_eq!(summary.chunks[1].len, 2);
    assert_eq!(summary.chunks[1].first_sequence, 3);
    assert_eq!(summary.chunks[1].last_sequence, 4);

    // A requested chunk_count of 0 must be clamped to 1 (a raw `len / 0` would
    // panic), so the whole corpus collapses into a single chunk.
    let clamped = RoutingSummary::from_sorted_entries(&entries, 0);
    assert_eq!(
        clamped.chunk_count, 1,
        "chunk_count is clamped to at least 1 before dividing"
    );
    assert_eq!(clamped.chunks[0].len, 5);
}

// ── Mutation-kill: EntityRun::usize_range overflow fail-closed ─────────────────
//
// `usize_range` converts a persisted `[start, start+len)` run into a usize
// range, mapping any `usize::try_from` failure or `start + len` overflow to a
// typed `CorruptSegment` rather than an unchecked cast/add.
#[test]
fn entity_run_usize_range_accepts_valid_and_rejects_overflowing_runs() {
    let ok = EntityRun {
        entity: "solo".to_owned(),
        start: 2,
        len: 3,
        first_sequence: 0,
        last_sequence: 0,
    };
    assert_eq!(
        ok.usize_range().expect("a valid in-range run must convert"),
        2..5,
        "PROPERTY: a well-formed run maps to `start..start+len`"
    );

    // start = usize::MAX (fits usize on every target) + len = 1 overflows the
    // `checked_add`, which must fail closed instead of wrapping to 0.
    let overflow = EntityRun {
        entity: "solo".to_owned(),
        start: usize::MAX as u64,
        len: 1,
        first_sequence: 0,
        last_sequence: 0,
    };
    let err = overflow
        .usize_range()
        .expect_err("PROPERTY: start + len overflow must be a typed corruption error, not a wrap");
    assert!(
        matches!(err, StoreError::CorruptSegment { .. }),
        "PROPERTY: overflow maps to CorruptSegment, got {err:?}"
    );
}

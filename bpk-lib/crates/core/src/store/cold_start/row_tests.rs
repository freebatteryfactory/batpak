use super::{ColdStartIndexRow, ColdStartSource};
use crate::event::{EventKind, HashChain};
use crate::store::index::interner::InternId;
use crate::store::index::DiskPos;

#[test]
fn cold_start_row_to_event_header_preserves_lane_depth_and_ids() {
    let row = ColdStartIndexRow {
        source: ColdStartSource::Sidx,
        event_id: 1,
        correlation_id: 2,
        causation_id: Some(3),
        entity_id: InternId(1),
        scope_id: InternId(2),
        kind: EventKind::DATA,
        wall_ms: 1_700_000_000_000,
        clock: 9,
        dag_lane: 4,
        dag_depth: 2,
        hash_chain: HashChain::default(),
        disk_pos: DiskPos::new(7, 64, 32),
        global_sequence: 11,
    };

    let header = row.to_event_header();

    assert_eq!(header.event_id, crate::id::EventId::from(1u128));
    assert_eq!(header.correlation_id, crate::id::CorrelationId::from(2u128));
    assert_eq!(
        header.causation_id,
        Some(crate::id::CausationId::from(3u128))
    );
    assert_eq!(header.timestamp_us, 1_700_000_000_000_000);
    assert_eq!(header.position.wall_ms, 1_700_000_000_000);
    assert_eq!(header.position.sequence, 9);
    assert_eq!(header.position.lane, 4);
    assert_eq!(header.position.depth, 2);
    assert_eq!(header.event_kind, EventKind::DATA);
    assert_eq!(header.payload_size, 0);
    assert_eq!(header.flags, 0);
    assert_eq!(header.content_hash, [0u8; 32]);
}

fn row_with_wall_ms(wall_ms: u64) -> ColdStartIndexRow {
    ColdStartIndexRow {
        source: ColdStartSource::Sidx,
        event_id: 1,
        correlation_id: 2,
        causation_id: Some(3),
        entity_id: InternId(1),
        scope_id: InternId(2),
        kind: EventKind::DATA,
        wall_ms,
        clock: 9,
        dag_lane: 4,
        dag_depth: 2,
        hash_chain: HashChain::default(),
        disk_pos: DiskPos::new(7, 64, 32),
        global_sequence: 11,
    }
}

#[test]
fn cold_start_row_to_event_header_saturates_overflowing_wall_ms() {
    // u64::MAX * 1000 overflows u64 (caught by checked_mul). Under release
    // overflow-checks the old `* 1000` would panic; this must not panic and
    // must saturate the derived timestamp while preserving the raw wall_ms.
    let row = row_with_wall_ms(u64::MAX);
    let header = row.to_event_header();
    assert_eq!(
        header.timestamp_us,
        i64::MAX,
        "PROPERTY: overflowing wall_ms saturates timestamp_us to i64::MAX without panic"
    );
    assert_eq!(
        header.position.wall_ms,
        u64::MAX,
        "PROPERTY: raw wall_ms is preserved unchanged in the DAG position"
    );
}

#[test]
fn cold_start_row_to_event_header_saturates_at_i64_try_from_boundary() {
    // This value multiplies cleanly by 1000 in u64 but exceeds i64::MAX,
    // so checked_mul succeeds and i64::try_from is the guard that fires.
    // Locks the i64::try_from guard against a revert to plain `as i64`,
    // which would wrap to a negative value here.
    let wall_ms = (i64::MAX as u64) / 1000 + 1;
    let row = row_with_wall_ms(wall_ms);
    let header = row.to_event_header();
    assert_eq!(
        header.timestamp_us,
        i64::MAX,
        "PROPERTY: u64->i64 truncation boundary saturates to i64::MAX, never wraps negative"
    );
    assert_eq!(header.position.wall_ms, wall_ms);
}

// ── Mutation-kill: raw_to_kind category dispatch ──────────────────────────
//
// `raw_to_kind_impl` computes `category = (raw >> 12) as u8` and dispatches:
// reserved-system (0x0) and reserved-effect (0xD) match known constants (and
// fall back to DATA / EFFECT_ERROR for unknowns), while every other category
// decodes to `EventKind::custom(category, raw & 0x0FFF)`.
#[test]
fn raw_to_kind_decodes_open_category_from_the_high_nibble() {
    use super::raw_to_kind;
    let decoded = raw_to_kind(0x1ABC);
    assert_eq!(
        decoded,
        EventKind::custom(0x1, 0xABC),
        "PROPERTY: an open category decodes via `(raw >> 12)`; the `>> -> <<` mutant collapses \
             the category to 0x0 and mis-routes this to the system-fallback DATA"
    );
    assert_eq!(decoded.category(), 0x1, "category is the high nibble");
    assert_eq!(
        decoded.type_id(),
        0xABC,
        "type_id is the low 12 bits (`raw & 0x0FFF`); kills the `& -> |`/`^` mutant"
    );
}

#[test]
fn raw_to_kind_matches_reserved_constants_and_falls_back_on_unknowns() {
    use super::raw_to_kind;
    assert_eq!(
        raw_to_kind(0x0006),
        EventKind::SYSTEM_BATCH_BEGIN,
        "a known reserved-system value decodes to its exact constant"
    );
    assert_eq!(
        raw_to_kind(0xD001),
        EventKind::EFFECT_ERROR,
        "a known reserved-effect value decodes to its exact constant"
    );
    assert_eq!(
        raw_to_kind(0x00AB),
        EventKind::DATA,
        "PROPERTY: an unrecognized reserved-SYSTEM value falls back to DATA (kills a deleted \
             system-fallback arm)"
    );
    assert_eq!(
        raw_to_kind(0xD0AB),
        EventKind::EFFECT_ERROR,
        "PROPERTY: an unrecognized reserved-EFFECT value falls back to EFFECT_ERROR"
    );
}

// ── Mutation-kill: ReservedKindFallbackStats counting/merge arithmetic ────
#[test]
fn reserved_kind_fallback_stats_count_system_and_effect_fallbacks() {
    use super::{raw_to_kind_counted, ReservedKindFallbackStats};

    let mut counts = ReservedKindFallbackStats::default();
    assert_eq!(
        raw_to_kind_counted(0x00AB, &mut counts),
        EventKind::DATA,
        "unknown system kind still decodes to DATA while being counted"
    );
    assert_eq!(
        counts.system, 1,
        "the first unknown system kind increments the counter to 1 (kills `+= 1` -> `-= 1`)"
    );
    assert_eq!(counts.system_histogram.get(&0x00AB), Some(&1));

    let _ = raw_to_kind_counted(0x00AB, &mut counts);
    assert_eq!(
        counts.system, 2,
        "a second fallback of the same raw value increments to 2 (kills a constant-0 counter)"
    );
    assert_eq!(
        counts.system_histogram.get(&0x00AB),
        Some(&2),
        "the per-raw histogram accumulates alongside the total"
    );

    assert_eq!(
        raw_to_kind_counted(0xD0AB, &mut counts),
        EventKind::EFFECT_ERROR
    );
    assert_eq!(
        counts.effect, 1,
        "effect fallbacks are counted on their own axis (kills a system/effect field swap)"
    );
    assert_eq!(counts.effect_histogram.get(&0xD0AB), Some(&1));
    assert_eq!(
        counts.system, 2,
        "recording an effect fallback must not touch the system counter"
    );
}

#[test]
fn reserved_kind_fallback_stats_merge_sums_both_axes() {
    use super::ReservedKindFallbackStats;

    let mut left = ReservedKindFallbackStats::default();
    left.record_system(0x00AB);

    let mut right = ReservedKindFallbackStats::default();
    right.record_system(0x00AB);
    right.record_effect(0xD0AB);

    let merged = left.add(&right);
    assert_eq!(
        merged.system, 2,
        "add() sums the system totals of both operands (kills `+= -> -=` in merge_from and an \
             `add -> Default::default()` whole-body replacement)"
    );
    assert_eq!(
        merged.effect, 1,
        "add() sums the effect totals of both operands"
    );
    assert_eq!(
        merged.system_histogram.get(&0x00AB),
        Some(&2),
        "shared histogram keys accumulate across the merge"
    );
    assert_eq!(merged.effect_histogram.get(&0xD0AB), Some(&1));
}

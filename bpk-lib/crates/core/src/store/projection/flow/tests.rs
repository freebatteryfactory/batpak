use super::*;
use crate::event::{Event, EventKind};
use crate::store::index::columnar::CachedProjectionSlot;
use crate::store::StoreConfig;
use std::error::Error;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
struct Counter;

impl EventSourced for Counter {
    type Input = crate::event::JsonValueInput;

    fn apply_event(&mut self, event: &Event<serde_json::Value>) {
        std::hint::black_box(event.event_kind());
    }

    fn from_events(events: &[Event<serde_json::Value>]) -> Option<Self> {
        (!events.is_empty()).then_some(Self)
    }

    fn relevant_event_kinds() -> &'static [EventKind] {
        static KINDS: [EventKind; 1] = [EventKind::custom(0xF, 1)];
        &KINDS
    }
}

#[test]
fn projection_replay_plan_matches_legacy_stream_filtering() -> TestResult {
    let dir = TempDir::new()?;
    let store = Store::open(StoreConfig::new(dir.path()))?;
    let coord = crate::coordinate::Coordinate::new("entity:proj", "scope:test")?;
    let kept = EventKind::custom(0xF, 1);
    let skipped = EventKind::custom(0xF, 2);

    for (kind, payload) in [
        (kept, serde_json::json!({"n": 1})),
        (skipped, serde_json::json!({"n": 2})),
        (kept, serde_json::json!({"n": 3})),
    ] {
        store.append(&coord, kind, &payload)?;
    }

    let Some(plan) = store
        .index
        .projection_replay_plan("entity:proj", Counter::relevant_event_kinds())
    else {
        return Err(std::io::Error::other("expected projection replay plan").into());
    };

    let legacy_entries = store.index.stream("entity:proj");
    let legacy_entries: Vec<_> = legacy_entries
        .into_iter()
        .filter(|entry| Counter::relevant_event_kinds().contains(&entry.kind))
        .collect();
    let legacy_items: Vec<_> = legacy_entries
        .iter()
        .map(|entry| (entry.global_sequence, entry.disk_pos))
        .collect();
    let planned_items: Vec<_> = plan
        .items
        .iter()
        .map(|item| (item.global_sequence, item.disk_pos))
        .collect();
    let Some(legacy_watermark) = legacy_entries.last().map(|entry| entry.global_sequence) else {
        return Err(std::io::Error::other("expected legacy filtered entries").into());
    };

    assert_eq!(plan.watermark, legacy_watermark);
    assert_eq!(
        plan.generation,
        store.index.entity_generation("entity:proj").unwrap_or(0)
    );
    assert_eq!(planned_items, legacy_items);

    store.close()?;
    Ok(())
}

#[test]
fn projection_timings_cold_path_breakdown() -> TestResult {
    let dir = TempDir::new()?;
    let store = Store::open(StoreConfig::new(dir.path()))?;
    let coord = crate::coordinate::Coordinate::new("entity:timed", "scope:test")?;
    let kind = EventKind::custom(0xF, 1);
    for i in 0..1_000u32 {
        store.append(&coord, kind, &serde_json::json!({"i": i}))?;
    }

    // Close and reopen to get a true cold path
    store.close()?;
    let store = Store::open(StoreConfig::new(dir.path()))?;

    let mut timings = ProjectionTimings::default();
    let result: Option<Counter> =
        project_timed(&store, "entity:timed", &Freshness::Consistent, &mut timings)?;
    assert!(result.is_some(), "projection must produce a value");

    let accounted = timings.plan_build_us
        + timings.cache_key_build_us
        + timings.group_local_lookup_us
        + timings.prefetch_us
        + timings.external_cache_probe_us
        + timings.disk_read_us
        + timings.event_extract_us
        + timings.replay_fold_us
        + timings.cache_store_us;

    assert!(timings.total_us > 0, "total must be positive");
    assert!(
        accounted <= timings.total_us,
        "phase timings must not exceed total"
    );
    store.close()?;
    Ok(())
}

#[test]
fn compute_strategy_exhaustive() {
    let slot = CachedProjectionSlot {
        bytes: vec![],
        watermark: 42,
        generation: 1,
    };

    // Slot present + fresh -> GroupLocalHit
    assert_eq!(
        compute_strategy(Some(&slot), true, false, false, false),
        ProjectionStrategy::GroupLocalHit,
    );
    assert_eq!(
        compute_strategy(Some(&slot), true, true, true, true),
        ProjectionStrategy::GroupLocalHit,
    );

    // Slot present + stale + incremental supported + enabled -> GroupLocalIncremental
    assert_eq!(
        compute_strategy(Some(&slot), false, true, true, false),
        ProjectionStrategy::GroupLocalIncremental,
    );
    assert_eq!(
        compute_strategy(Some(&slot), false, true, true, true),
        ProjectionStrategy::GroupLocalIncremental,
    );

    // Slot present + stale + incremental disabled -> falls through to cache check
    assert_eq!(
        compute_strategy(Some(&slot), false, true, false, false),
        ProjectionStrategy::ExternalCacheThenReplay,
    );
    assert_eq!(
        compute_strategy(Some(&slot), false, true, false, true),
        ProjectionStrategy::DirectReplay,
    );

    // Slot present + stale + incremental NOT supported -> falls through to cache check
    assert_eq!(
        compute_strategy(Some(&slot), false, false, false, false),
        ProjectionStrategy::ExternalCacheThenReplay,
    );
    assert_eq!(
        compute_strategy(Some(&slot), false, false, true, false),
        ProjectionStrategy::ExternalCacheThenReplay,
    );
    assert_eq!(
        compute_strategy(Some(&slot), false, false, false, true),
        ProjectionStrategy::DirectReplay,
    );

    // No slot + noop cache -> DirectReplay
    assert_eq!(
        compute_strategy(None, false, false, false, true),
        ProjectionStrategy::DirectReplay,
    );

    // No slot + real cache -> ExternalCacheThenReplay
    assert_eq!(
        compute_strategy(None, false, false, false, false),
        ProjectionStrategy::ExternalCacheThenReplay,
    );
    assert_eq!(
        compute_strategy(None, false, true, true, false),
        ProjectionStrategy::ExternalCacheThenReplay,
    );
}

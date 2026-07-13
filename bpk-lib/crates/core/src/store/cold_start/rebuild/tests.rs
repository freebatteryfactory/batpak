use super::topology::{compaction_source_temp_path, compaction_staged_path};
use super::*;
use crate::prelude::*;
use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
use crate::store::platform::fs::{create_new_file, RealFs};
use crate::store::segment;
use crate::store::store_meta::{CompactionCommit, StoreMetaData};
use std::collections::BTreeMap;
use tempfile::TempDir;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScanSummaryRow {
    event_id: u128,
    entity: String,
    scope: String,
    category: u8,
    type_id: u16,
    global_sequence: u64,
    offset: u64,
    length: u32,
}

fn rotating_store_config(dir: &TempDir) -> StoreConfig {
    StoreConfig::new(dir.path())
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
}

fn scanned_summary(
    entries: &[crate::store::segment::scan::ScannedIndexEntry],
) -> Vec<ScanSummaryRow> {
    use crate::id::EntityIdType;
    entries
        .iter()
        .map(|entry| ScanSummaryRow {
            event_id: entry.header.event_id.as_u128(),
            entity: entry.entity.clone(),
            scope: entry.scope.clone(),
            category: entry.header.event_kind.category(),
            type_id: entry.header.event_kind.type_id(),
            global_sequence: entry.global_sequence.unwrap_or(0),
            offset: entry.offset,
            length: entry.length,
        })
        .collect()
}

fn sample_index_entries(count: u64, segment_id: u64) -> (Vec<IndexEntry>, Vec<String>) {
    let interner = StringInterner::new();
    let mut entries = Vec::new();
    for i in 0..count {
        let coord = Coordinate::new(format!("entity:{i}"), "scope:rebuild").expect("valid coord");
        let entity_id = interner.intern(coord.entity()).expect("intern");
        let scope_id = interner.intern(coord.scope()).expect("intern");
        entries.push(IndexEntry {
            event_id: (i + 1) as u128,
            correlation_id: (i + 1) as u128,
            causation_id: None,
            coord,
            entity_id,
            scope_id,
            kind: EventKind::custom(0x1, u16::try_from(i + 1).expect("sample type id fits u16")),
            wall_ms: 1_700_000_000_000 + i * 1000,
            clock: u32::try_from(i + 1).expect("clock fits u32"),
            dag_lane: 0,
            dag_depth: 0,
            hash_chain: HashChain::default(),
            disk_pos: DiskPos::new(segment_id, i * 256, 256),
            global_sequence: i,
            receipt_extensions: BTreeMap::new(),
        });
    }
    let interner_strings = full_interner_snapshot(&interner);
    (entries, interner_strings)
}

#[test]
fn parallel_sidx_footer_read_matches_sequential_footer_read() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(rotating_store_config(&dir)).expect("open store");
    let coord = Coordinate::new("entity:sidx", "scope:rebuild").expect("coord");
    let kind = EventKind::custom(0xF, 9);
    let payload = serde_json::json!({
        "blob": "payload that forces rapid segment rotation and sealed footer generation"
    });

    for n in 0..64u32 {
        let _ = store
            .append(
                &coord,
                kind,
                &serde_json::json!({"n": n, "payload": payload}),
            )
            .expect("append");
    }
    store.close().expect("close store");

    let entries =
        segment_paths(dir.path(), &crate::store::platform::fs::RealFs).expect("segment paths");
    let active_segment = entries.last().expect("at least one segment").0;
    let sealed_segments: Vec<_> = entries
        .into_iter()
        .filter(|(segment_id, _)| *segment_id < active_segment)
        .collect();

    assert!(
        !sealed_segments.is_empty(),
        "PROPERTY: tiny segments should produce at least one sealed segment with an SIDX footer."
    );

    let reader = Reader::new(
        dir.path().to_path_buf(),
        16,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let (parallel, _) =
        read_sealed_sidx_entries_parallel(&reader, &sealed_segments, NO_FAULT_INJECTOR)
            .expect("parallel SIDX footer read should succeed");
    let sequential = read_sealed_sidx_entries_sequential(&reader, &sealed_segments)
        .expect("sequential SIDX footer read should succeed");

    assert_eq!(
        scanned_summary(&parallel),
        scanned_summary(&sequential),
        "PROPERTY: parallel SIDX footer rebuild must match sequential footer semantics exactly."
    );
}

#[test]
fn build_snapshot_plan_keeps_chunk_count_when_tail_is_empty() {
    let dir = TempDir::new().expect("temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let clock = crate::store::SystemClock::new();
    let planner = RestorePlanner {
        reader: &reader,
        data_dir: dir.path(),
        policy: ColdStartPolicy::new(false, false),
        clock: &clock,
        fault_injector: NO_FAULT_INJECTOR,
        compaction_recovery: CompactionRecoveryAction::None,
    };
    // DISCRIMINATING FIXTURE: use ENOUGH entries that a +1 to `chunk_count`
    // produces an observably different routing. With 0 entries the chunk
    // partition is always empty (empty chunks are skipped), so `chunk_count`
    // of 1 vs 2 both collapse to 0 — a degenerate case that CANNOT catch the
    // `tail_count > 0` -> `>= 0` mutant. With 4 entries and an input routing
    // of 1 chunk, the empty-tail path must preserve `chunk_count == 1`, whereas
    // the mutant (always +1) re-partitions into 2 non-empty chunks of 2.
    // `receipt_extensions_hydrated: true` lets the entries skip frame backing
    // (the planner does not re-read receipts), so no segments are required.
    let (entries, interner_strings) = sample_index_entries(4, 0);
    let routing = RoutingSummary::from_sorted_entries(&entries, 1);
    let expected_chunk_count = routing.chunk_count;
    assert_eq!(
        expected_chunk_count, 1,
        "SANITY: the fixture must start from a single-chunk routing so a +1 is observable as 2"
    );

    let plan = planner
        .build_snapshot_plan(
            RestoreSource::Checkpoint,
            SnapshotPlanInput {
                entries,
                interner_strings,
                watermark: WatermarkInfo {
                    watermark_segment_id: 99,
                    watermark_offset: 0,
                },
                stored_allocator: 2,
                routing,
                reopen_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
                persisted_cumulative_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
                receipt_extensions_hydrated: true,
                snapshot_loads: SnapshotLoadDiagnostics::default(),
            },
        )
        .expect("build snapshot plan");

    assert_eq!(
        plan.tail_entries, 0,
        "SANITY: empty temp dir should produce no tail replay"
    );
    assert_eq!(
        plan.restored_entries, 4,
        "SANITY: all four snapshot entries must survive into the plan"
    );
    assert_eq!(
            plan.routing.chunk_count,
            expected_chunk_count,
            "PROPERTY: a snapshot plan with no tail entries must preserve the existing routing chunk count (1) instead of synthesizing an extra chunk (2 under `tail_count >= 0`)"
        );
}

#[test]
fn build_snapshot_plan_rejects_snapshot_entries_without_backing_frames() {
    let dir = TempDir::new().expect("temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let clock = crate::store::SystemClock::new();
    let planner = RestorePlanner {
        reader: &reader,
        data_dir: dir.path(),
        policy: ColdStartPolicy::new(false, false),
        clock: &clock,
        fault_injector: NO_FAULT_INJECTOR,
        compaction_recovery: CompactionRecoveryAction::None,
    };
    let (entries, interner_strings) = sample_index_entries(1, 0);
    let routing = RoutingSummary::from_sorted_entries(&entries, 1);

    let result = planner.build_snapshot_plan(
        RestoreSource::Checkpoint,
        SnapshotPlanInput {
            entries,
            interner_strings,
            watermark: WatermarkInfo {
                watermark_segment_id: 99,
                watermark_offset: 0,
            },
            stored_allocator: 1,
            routing,
            reopen_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
            persisted_cumulative_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
            receipt_extensions_hydrated: false,
            snapshot_loads: SnapshotLoadDiagnostics::default(),
        },
    );
    assert!(
        matches!(result, Err(StoreError::Io(_))),
        "PROPERTY: snapshot entries without backing frames must fail closed with an IO error"
    );
}

#[test]
fn build_snapshot_plan_adds_chunk_when_tail_is_present() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(rotating_store_config(&dir)).expect("open store");
    let coord = Coordinate::new("entity:tail-plan", "scope:rebuild").expect("coord");
    let kind = EventKind::custom(0xE, 8);
    for n in 0..16u32 {
        let _ = store
            .append(&coord, kind, &serde_json::json!({ "n": n }))
            .expect("append tail event");
    }
    store.close().expect("close store");

    let entries =
        segment_paths(dir.path(), &crate::store::platform::fs::RealFs).expect("segment paths");
    let watermark_segment_id = entries
        .first()
        .map(|(segment_id, _)| *segment_id)
        .expect("watermark segment id");
    let active_after_tail = entries
        .last()
        .map(|(segment_id, _)| segment_id.saturating_add(1))
        .expect("active segment id");

    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    reader.set_active_segment(active_after_tail);
    let clock = crate::store::SystemClock::new();
    let planner = RestorePlanner {
        reader: &reader,
        data_dir: dir.path(),
        policy: ColdStartPolicy::new(false, false),
        clock: &clock,
        fault_injector: NO_FAULT_INJECTOR,
        compaction_recovery: CompactionRecoveryAction::None,
    };
    let routing = RoutingSummary::from_sorted_entries(&[], 1);

    let plan = planner
        .build_snapshot_plan(
            RestoreSource::Checkpoint,
            SnapshotPlanInput {
                entries: Vec::new(),
                interner_strings: Vec::new(),
                watermark: WatermarkInfo {
                    watermark_segment_id,
                    watermark_offset: 0,
                },
                stored_allocator: 0,
                routing,
                reopen_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
                persisted_cumulative_reserved_kind_fallbacks: ReservedKindFallbackStats::default(),
                receipt_extensions_hydrated: false,
                snapshot_loads: SnapshotLoadDiagnostics::default(),
            },
        )
        .expect("build snapshot plan with tail");

    assert!(
        plan.tail_entries > 0,
        "SANITY: fixture should collect tail entries from the watermark segment onward"
    );
    assert_eq!(
            plan.routing.chunk_count,
            2,
            "PROPERTY: snapshot restore must add exactly one routing chunk when tail replay contributes entries"
        );
}

#[test]
fn entry_from_scan_normalizes_zero_causation() {
    use crate::coordinate::DagPosition;
    use crate::event::{EventHeader, EventKind, HashChain};
    use crate::store::segment::scan::ScannedIndexEntry;

    let interner = StringInterner::new();
    let se = ScannedIndexEntry {
        header: EventHeader {
            event_id: crate::id::EventId::from(1u128),
            correlation_id: crate::id::CorrelationId::from(1u128),
            causation_id: Some(crate::id::CausationId::from(0u128)),
            timestamp_us: 0,
            position: DagPosition::new(0, 0, 1),
            payload_size: 0,
            event_kind: EventKind::custom(0x1, 1),
            flags: 0,
            content_hash: [0u8; 32],
            payload_version: 0,
            #[cfg(feature = "payload-encryption")]
            payload_encryption: None,
        },
        entity: "entity:test".to_string(),
        scope: "scope:test".to_string(),
        hash_chain: HashChain::default(),
        segment_id: 0,
        offset: 0,
        length: 64,
        receipt_extensions: BTreeMap::new(),
        global_sequence: Some(0),
    };
    let entry = entry_from_scan(&interner, se, 0).expect("entry_from_scan");
    assert_eq!(
        entry.causation_id, None,
        "INVARIANT: Some(0) causation_id from scan must normalize to None"
    );
}

#[test]
fn entry_from_scan_preserves_nonzero_causation() {
    use crate::coordinate::DagPosition;
    use crate::event::{EventHeader, EventKind, HashChain};
    use crate::store::segment::scan::ScannedIndexEntry;

    let interner = StringInterner::new();
    let se = ScannedIndexEntry {
        header: EventHeader {
            event_id: crate::id::EventId::from(2u128),
            correlation_id: crate::id::CorrelationId::from(1u128),
            causation_id: Some(crate::id::CausationId::from(99u128)),
            timestamp_us: 0,
            position: DagPosition::new(0, 0, 1),
            payload_size: 0,
            event_kind: EventKind::custom(0x1, 1),
            flags: 0,
            content_hash: [0u8; 32],
            payload_version: 0,
            #[cfg(feature = "payload-encryption")]
            payload_encryption: None,
        },
        entity: "entity:test".to_string(),
        scope: "scope:test".to_string(),
        hash_chain: HashChain::default(),
        segment_id: 0,
        offset: 0,
        length: 64,
        receipt_extensions: BTreeMap::new(),
        global_sequence: Some(1),
    };
    let entry = entry_from_scan(&interner, se, 1).expect("entry_from_scan");
    assert_eq!(entry.causation_id, Some(99));
}

#[test]
fn resolve_rolls_forward_then_segment_paths_lists_the_new_generation() {
    // End-to-end (A16): a COMMITTED transaction — store.meta carries a commit
    // record whose branded token AND merged id match the live marker — rolls
    // FORWARD: the staged replacement is renamed to the final name, the sources
    // and `.compact-src`/`.compact-new` leftovers are retired, and the marker is
    // cleared. `segment_paths` fails closed while any marker is live, so it can
    // only list AFTER the repair, and it must then yield exactly the merged
    // segment plus the untouched segments the transaction never named.
    let dir = TempDir::new().expect("temp dir");
    let compaction_id = CompactionId::from_u128(0x00c0_ffee);
    let lineage: u128 = 0x0011_2233;
    let authority_image_id = AuthorityImageId::from_u128(0x00a0_1dea);

    let staged = compaction_staged_path(dir.path(), 7);
    let src = compaction_source_temp_path(dir.path(), 7);
    let final_path = dir.path().join(segment::segment_filename(7));
    let untouched = dir.path().join(segment::segment_filename(9));
    create_new_file(&staged).expect("stage replacement");
    create_new_file(&src).expect("relocated merged-id source");
    create_new_file(&dir.path().join(segment::segment_filename(1))).expect("source 1");
    create_new_file(&dir.path().join(segment::segment_filename(2))).expect("source 2");
    create_new_file(&untouched).expect("untouched segment");

    write_pending_compaction(
        dir.path(),
        &PendingCompactionV2 {
            compaction_id,
            lineage: StoreLineage::from_u128(lineage),
            merged_id: 7,
            source_segment_ids: vec![1, 2, 7],
            expected_authority_image_id: authority_image_id,
        },
        &RealFs,
    )
    .expect("write v2 marker");

    let meta = StoreMetaData {
        lineage,
        idemp_authority: None,
        last_compaction_commit: Some(CompactionCommit {
            compaction_id,
            authority_image_id,
            merged_segment_id: 7,
            source_segment_ids: vec![1, 2, 7],
        }),
    };

    let action = resolve_pending_compaction(dir.path(), &RealFs, Some(&meta), true)
        .expect("committed transaction rolls forward");
    assert_eq!(
        action,
        CompactionRecoveryAction::RolledForward,
        "PROPERTY: a marker whose token and merged id match the store.meta commit record rolls forward"
    );
    assert!(
        final_path.exists() && !staged.exists() && !src.exists(),
        "PROPERTY: roll-forward publishes the replacement at the final name and retires the staged/src leftovers"
    );
    assert!(
        !dir.path().join(COMPACTION_MARKER_FILENAME).exists(),
        "PROPERTY: the marker is cleared last"
    );

    let entries =
        segment_paths(dir.path(), &RealFs).expect("segment listing succeeds once the marker is cleared");
    let ids: Vec<u64> = entries.iter().map(|(segment_id, _)| *segment_id).collect();
    assert_eq!(
        ids,
        vec![7, 9],
        "PROPERTY: after roll-forward, listing yields the merged segment plus the untouched segment; sources 1 and 2 are retired"
    );
}

#[test]
fn clear_pending_compaction_is_idempotent_when_marker_is_absent() {
    let dir = TempDir::new().expect("temp dir");

    clear_pending_compaction(dir.path(), &crate::store::platform::fs::RealFs)
        .expect("PROPERTY: clearing an absent pending-compaction marker must be idempotent");
}

#[test]
fn open_index_forces_full_rebuild_when_a_compaction_was_recovered() {
    // A physical compaction repair clears the marker BEFORE the index is planned,
    // but any mmap/checkpoint artifact still describes the PRE-crash index (it can
    // carry entries for compacted-away events whose restore would resurrect them).
    // So a `CompactionRecoveryAction` other than `None` must force the full segment
    // rebuild past the stale fast path — the property the retired marker-presence
    // probe used to gate, now driven by the explicit recovery parameter (A16).
    let dir = TempDir::new().expect("temp dir");
    let config = crate::store::StoreConfig::new(dir.path())
        .with_enable_checkpoint(true)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1);
    let store = crate::store::Store::open(config).expect("open");
    let coord =
        crate::coordinate::Coordinate::new("entity:recovered-open", "scope:test").expect("coord");
    let kind = crate::event::EventKind::custom(0xE, 1);
    for i in 0..20u32 {
        let _ = store
            .append(&coord, kind, &serde_json::json!({ "i": i }))
            .expect("append");
    }
    store.close().expect("close");

    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let open_with = |index: &StoreIndex, recovery: CompactionRecoveryAction| {
        open_index(
            index,
            &reader,
            dir.path(),
            ColdStartPolicy::new(true, false),
            &crate::store::SystemClock::new(),
            NO_FAULT_INJECTOR,
            None,
            recovery,
        )
    };

    // SANITY / non-vacuity: with NO recovery this open, the durable checkpoint is
    // a trusted fast path — so the rebuild below is a real bypass, not a store
    // that could only ever rebuild.
    let baseline = open_with(&StoreIndex::new(), CompactionRecoveryAction::None)
        .expect("baseline open with no recovery");
    assert_ne!(
        baseline.report.path,
        OpenIndexPath::Rebuild,
        "SANITY: without a recovery this open, the durable checkpoint is a trusted fast path"
    );

    let recovered = open_with(&StoreIndex::new(), CompactionRecoveryAction::RolledForward)
        .expect("open after a compaction was recovered");
    assert_eq!(
        recovered.report.path,
        OpenIndexPath::Rebuild,
        "PROPERTY: a compaction recovered this open must take the full segment rebuild, never a stale checkpoint/mmap fast path"
    );
}

#[test]
fn collect_tail_entries_keeps_events_from_the_watermark_segment() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(rotating_store_config(&dir)).expect("open store");
    let coord = Coordinate::new("entity:tail", "scope:watermark").expect("coord");
    let kind = EventKind::custom(0xE, 7);

    for n in 0..64u32 {
        let _ = store
            .append(&coord, kind, &serde_json::json!({ "n": n }))
            .expect("append");
    }
    store.close().expect("close");

    let entries =
        segment_paths(dir.path(), &crate::store::platform::fs::RealFs).expect("segment paths");
    assert!(
        entries.len() >= 2,
        "SANITY: rotating config should create multiple segments for watermark-tail testing"
    );
    let watermark_segment_id = entries
        .first()
        .map(|(segment_id, _)| *segment_id)
        .expect("watermark segment id");
    let highest_segment_id = entries
        .last()
        .map(|(segment_id, _)| *segment_id)
        .expect("highest segment id");

    let interner = StringInterner::new();
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    reader.set_active_segment(highest_segment_id + 1);
    let tail_entries = collect_tail_entries(
        &interner,
        &reader,
        dir.path(),
        &WatermarkInfo {
            watermark_segment_id,
            watermark_offset: 0,
        },
        0,
        NO_FAULT_INJECTOR,
    )
    .expect("collect tail entries");

    assert!(
            tail_entries
                .iter()
                .any(|entry| entry.disk_pos.segment_id == watermark_segment_id),
            "PROPERTY: replay tail must include events from the watermark segment itself when the watermark offset is at the segment start"
        );
}

// ── Mutation-kill: SequenceTracker synthesis + max tracking ───────────────────
//
// `SequenceTracker` synthesizes the `global_sequence` for scan frames that lack
// one. `synthesize_next` returns `0` before anything is inserted and
// `max_seen + 1` afterward; `note_seen` folds each observed sequence into
// `max_seen` via `.max(..)` and latches `inserted_any`. These are the numbers
// that decide how the allocator floor advances during rebuild/tail replay.
#[test]
fn sequence_tracker_synthesizes_zero_before_insert_and_max_plus_one_after() {
    let mut tracker = SequenceTracker::default();
    assert_eq!(
        tracker.synthesize_next(),
        0,
        "PROPERTY: a fresh tracker synthesizes 0 (kills the `inserted_any` guard -> true and a \
         whole-body -> max_seen + 1)"
    );

    tracker.note_seen(5);
    assert_eq!(
        tracker.synthesize_next(),
        6,
        "PROPERTY: after seeing 5 the next synthesized sequence is max_seen + 1 = 6 (kills a \
         whole-body -> 0, the guard -> false, and `saturating_add(1)` dropping the + 1)"
    );

    tracker.note_seen(3);
    assert_eq!(
        tracker.synthesize_next(),
        6,
        "PROPERTY: observing a LOWER sequence must not lower max_seen; still 6 (kills `.max(..)` \
         degenerating to min or a plain assignment)"
    );

    tracker.note_seen(9);
    assert_eq!(
        tracker.synthesize_next(),
        10,
        "PROPERTY: a higher sequence advances max_seen, so the next synthesized value is 10"
    );
}

use super::*;
use crate::coordinate::{Coordinate, Region};
use crate::event::{EventKind, HashChain};
use crate::store::{IndexConfig, IndexTopology};

fn make_entry(seq: u64, entity: &str, scope: &str) -> IndexEntry {
    let coord = Coordinate::new(entity, scope).expect("coord");
    IndexEntry {
        event_id: seq as u128 + 1,
        correlation_id: seq as u128 + 1,
        causation_id: None,
        entity_id: self::interner::InternId::sentinel(),
        scope_id: self::interner::InternId::sentinel(),
        coord,
        kind: EventKind::custom(0xF, 1),
        wall_ms: seq,
        clock: u32::try_from(seq).expect("small seq"),
        dag_lane: 0,
        dag_depth: 0,
        hash_chain: HashChain::default(),
        disk_pos: DiskPos {
            segment_id: 0,
            offset: seq * 16,
            length: 16,
        },
        global_sequence: seq,
        receipt_extensions: BTreeMap::new(),
    }
}

#[test]
fn hlc_for_global_sequence_matches_the_queried_sequence_exactly() {
    // `find(... global_sequence == g)` -> `!=` would return the HLC of some OTHER
    // entry. `make_entry` sets `wall_ms == seq`, so the entry for seq=2 is the only
    // one whose wall_ms is 2; an inverted match resolves a different entry whose
    // wall_ms is never 2.
    let index = StoreIndex::new();
    let entity_id = index.interner.intern("entity:hlc").expect("intern");
    let scope_id = index.interner.intern("scope:hlc").expect("intern");
    for seq in 0..3 {
        let mut entry = make_entry(seq, "entity:hlc", "scope:hlc");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        index.insert(entry);
    }

    let point = index
        .hlc_for_global_sequence(2)
        .expect("global_sequence 2 must resolve to an HlcPoint");
    let mut failures: Vec<String> = Vec::new();
    if point.global_sequence != 2 {
        failures.push(format!(
            "returned HlcPoint must name the queried sequence 2, got {}",
            point.global_sequence
        ));
    }
    if point.wall_ms != 2 {
        failures.push(format!(
            "HLC must come from the entry whose global_sequence == 2 (wall_ms==seq); an \
             inverted `!=` match returns a different entry's wall_ms, got {}",
            point.wall_ms
        ));
    }
    assert!(
        failures.is_empty(),
        "hlc_for_global_sequence mismatches: {failures:?}"
    );
}

#[test]
fn clock_key_orders_by_wall_then_clock_then_uuid() {
    let mut keys = [
        ClockKey {
            wall_ms: 10,
            clock: 3,
            uuid: 9,
        },
        ClockKey {
            wall_ms: 9,
            clock: 99,
            uuid: 1,
        },
        ClockKey {
            wall_ms: 10,
            clock: 2,
            uuid: 99,
        },
        ClockKey {
            wall_ms: 10,
            clock: 3,
            uuid: 4,
        },
    ];

    keys.sort();

    assert_eq!(
            keys,
            [
                ClockKey {
                    wall_ms: 9,
                    clock: 99,
                    uuid: 1,
                },
                ClockKey {
                    wall_ms: 10,
                    clock: 2,
                    uuid: 99,
                },
                ClockKey {
                    wall_ms: 10,
                    clock: 3,
                    uuid: 4,
                },
                ClockKey {
                    wall_ms: 10,
                    clock: 3,
                    uuid: 9,
                },
            ],
            "PROPERTY: ClockKey ordering must be wall_ms first, then clock, then uuid as the deterministic tiebreaker"
        );
}

#[test]
fn bulk_restore_keeps_entries_invisible_until_publish() {
    let index = StoreIndex::new();
    let entity_id = index.interner.intern("entity:bulk").expect("intern");
    let scope_id = index.interner.intern("scope:bulk").expect("intern");
    let entries = (0..3)
        .map(|seq| {
            let mut entry = make_entry(seq, "entity:bulk", "scope:bulk");
            entry.entity_id = entity_id;
            entry.scope_id = scope_id;
            entry
        })
        .collect();

    index
        .restore_sorted_entries_with_before_publish(entries, 3, |index| {
            assert_eq!(
                index.visible_sequence(),
                0,
                "visibility watermark must not advance until every view is rebuilt"
            );
            assert!(
                index.query(&Region::all()).is_empty(),
                "PROPERTY: reads must observe neither base maps nor overlays before publish"
            );
        })
        .expect("bulk restore publish must succeed");

    assert_eq!(index.query(&Region::all()).len(), 3);
    assert_eq!(index.visible_sequence(), 3);
}

#[test]
fn upgrade_with_visibility_snapshot_rejects_cancelled_ranges() {
    let index = StoreIndex::new();
    let entity_id = index.interner.intern("entity:visibility").expect("intern");
    let scope_id = index.interner.intern("scope:visibility").expect("intern");
    for seq in 0..3 {
        let mut entry = make_entry(seq, "entity:visibility", "scope:visibility");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        index.insert(entry);
    }
    index
        .publish(3, "test-publish")
        .expect("publish test entries");
    index.restore_cancelled_visibility_ranges(CancelledVisibilityRanges {
        global: vec![(1, 2)],
        lanes: BTreeMap::new(),
    });

    let hidden = QueryHit {
        event_id: 2,
        global_sequence: 1,
        disk_pos: DiskPos::new(0, 16, 16),
        kind: EventKind::custom(0xF, 1),
        clock: 1,
        dag_lane: 0,
    };
    let (hits, visibility) = index.query_hits_with_snapshot(&Region::all());

    assert_eq!(
            hits.iter()
                .map(|hit| hit.global_sequence)
                .collect::<Vec<_>>(),
            vec![0, 2],
            "PROPERTY: query-hit collection must skip cancelled hidden ranges below the visible watermark"
        );
    assert!(
            index
                .upgrade_hit_with_visibility(hidden, &visibility)
                .is_none(),
            "PROPERTY: hit upgrade must use the same hidden-range visibility predicate as query collection"
        );
}

#[test]
fn cancel_visibility_fence_only_records_lanes_inside_half_open_range() {
    // Drive `cancel_visibility_fence` over a fence range of [2, 4). The
    // per-entry collection at index/mod.rs uses the half-open predicate
    // `seq >= start && seq < end`. We seed entries straddling both
    // boundaries on distinct lanes so the resulting per-lane cancelled map
    // pins the `< end` comparison against every off-by-one mutation:
    //   seq 2 (lane 10): inside  -> recorded
    //   seq 3 (lane 11): inside  -> recorded
    //   seq 4 (lane 12): == end  -> EXCLUDED (kills `<=` and `==`)
    //   seq 5 (lane 13): > end   -> EXCLUDED (kills `>`)
    let index = StoreIndex::new();
    let entity_id = index.interner.intern("entity:fence").expect("intern");
    let scope_id = index.interner.intern("scope:fence").expect("intern");
    for (seq, lane) in [(2u64, 10u32), (3, 11), (4, 12), (5, 13)] {
        let mut entry = make_entry(seq, "entity:fence", "scope:fence");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        entry.dag_lane = lane;
        index.insert(entry);
    }

    let token = index
        .begin_visibility_fence()
        .expect("begin visibility fence");
    index
        .note_visibility_fence_progress(token, 2, 4)
        .expect("note fence range [2, 4)");
    index
        .cancel_visibility_fence(token)
        .expect("cancel visibility fence");

    let cancelled = index.cancelled_visibility_ranges();
    let recorded_lanes: Vec<u32> = cancelled.lanes.keys().copied().collect();
    assert_eq!(
            recorded_lanes,
            vec![10, 11],
            "PROPERTY: cancel must record only lanes whose entry sequence lies in the half-open fence range [start, end)"
        );
    assert_eq!(
        cancelled.lanes.get(&10).map(Vec::as_slice),
        Some([(2, 3)].as_slice()),
        "lane 10 entry at the inclusive lower bound must be cancelled"
    );
    assert_eq!(
        cancelled.lanes.get(&11).map(Vec::as_slice),
        Some([(3, 4)].as_slice()),
        "lane 11 interior entry must be cancelled"
    );
    assert!(
        !cancelled.lanes.contains_key(&12),
        "entry at == end must NOT be cancelled (half-open upper bound)"
    );
    assert!(
        !cancelled.lanes.contains_key(&13),
        "entry beyond end must NOT be cancelled"
    );
}

#[test]
fn query_any_hits_after_excludes_wrong_lane_or_hidden_entries() {
    // `query_any_hits_after` is taken for a lane-scoped, fact-Any region
    // with no entity/scope filter. Its per-entry guard is the disjunction
    // `entry.dag_lane != lane || !is_visible_on_lane(seq, lane)`: an entry
    // is skipped if it is on the wrong lane OR hidden on the target lane.
    // We seed exactly the two cases where ONE disjunct is true (so `||`
    // skips but a `&&` mutant would NOT), plus a control that must survive:
    //   seq 0 (lane 7, hidden):  right lane, not visible -> skipped
    //   seq 1 (lane 9, visible): wrong lane              -> skipped
    //   seq 2 (lane 7, visible): right lane, visible     -> KEPT (control)
    let index = StoreIndex::new();
    let entity_id = index.interner.intern("entity:lane").expect("intern");
    let scope_id = index.interner.intern("scope:lane").expect("intern");
    for (seq, lane) in [(0u64, 7u32), (1, 9), (2, 7)] {
        let mut entry = make_entry(seq, "entity:lane", "scope:lane");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        entry.dag_lane = lane;
        index.insert(entry);
    }
    // Make seq < 3 visible on both lanes 7 and 9.
    index
        .publish_on_lanes(3, [(7, 3), (9, 3)], "test-lane-publish")
        .expect("publish lanes 7 and 9");
    // Hide seq 0 on lane 7 so it is on the right lane yet not visible.
    index.restore_cancelled_visibility_ranges(CancelledVisibilityRanges {
        global: Vec::new(),
        lanes: BTreeMap::from([(7u32, vec![(0u64, 1u64)])]),
    });

    let region = Region::all()
        .with_fact(crate::coordinate::KindFilter::Any)
        .with_lane(7);
    let hits = index.query_hits_after(&region, 0, false, 100);
    let seqs: Vec<u64> = hits.iter().map(|h| h.global_sequence).collect();

    assert_eq!(
            seqs,
            vec![2],
            "PROPERTY: a lane-scoped Any query must drop both wrong-lane and hidden-on-lane entries, keeping only the visible same-lane entry"
        );
}

#[test]
fn projection_replay_plan_preserves_scan_watermark_when_tail_candidate_is_hidden() {
    let index = StoreIndex::with_config(&IndexConfig {
        topology: IndexTopology::entity_local(),
        ..IndexConfig::default()
    });
    let entity_id = index
        .interner
        .intern("entity:projection-hidden-tail")
        .expect("intern");
    let scope_id = index
        .interner
        .intern("scope:projection-hidden-tail")
        .expect("intern");
    for seq in 0..2 {
        let mut entry = make_entry(
            seq,
            "entity:projection-hidden-tail",
            "scope:projection-hidden-tail",
        );
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        index.insert(entry);
    }
    index
        .publish_on_lanes(1, [(0, 1)], "test-projection-plan")
        .expect("publish only the first lane-0 candidate");

    let plan = index
        .projection_replay_plan(
            "entity:projection-hidden-tail",
            &[EventKind::custom(0xF, 1)],
        )
        .expect("projection plan exists even when its tail candidate is hidden");

    assert_eq!(
            plan.watermark, 1,
            "PROPERTY: projection plan watermark must remain at the scan candidate watermark, not the last currently-visible item"
        );
    assert_eq!(
            plan.items
                .iter()
                .map(|item| item.global_sequence)
                .collect::<Vec<_>>(),
            vec![0],
            "PROPERTY: only visible candidates are replayed, while the watermark still records the scan high-water mark"
        );
}

#[test]
fn mark_idemp_evicted_against_live_flags_exactly_the_missing_frames() {
    // Round-3 mutation kill: index/mod.rs `mark_idemp_evicted_against_live`
    // body replaced with `()`. The compaction tail calls this sweep so a
    // durable idempotency entry whose event frame did NOT survive into the
    // live index is honestly flagged `event_evicted` (the flag is persisted
    // in `index.idemp` and is how a later no-op deduplicated against an
    // evicted event is distinguished from one whose frame is still live).
    // Seed two durable entries: one whose event IS in `by_id` (control —
    // must stay unflagged) and one whose frame was never inserted (must be
    // flagged). The no-op mutant leaves BOTH unflagged.
    let index = StoreIndex::new();
    let entity_id = index
        .interner
        .intern("entity:idemp-evict")
        .expect("intern entity");
    let scope_id = index
        .interner
        .intern("scope:idemp-evict")
        .expect("intern scope");

    let mut live = make_entry(0, "entity:idemp-evict", "scope:idemp-evict");
    live.entity_id = entity_id;
    live.scope_id = scope_id;
    let live_id = live.event_id;
    let dropped = make_entry(1, "entity:idemp-evict", "scope:idemp-evict");
    let dropped_id = dropped.event_id;

    index
        .idemp
        .record(idemp::IdempEntry::from_index_entry(&live, 0));
    index
        .idemp
        .record(idemp::IdempEntry::from_index_entry(&dropped, 1));
    // Only the live entry's frame goes into the index; the other simulates a
    // frame that retention compaction dropped from the rebuilt live index.
    index.insert(live);

    index.mark_idemp_evicted_against_live();

    let live_entry = index.idemp.get(live_id).expect("live key was recorded");
    assert!(
        !live_entry.is_event_evicted(),
        "a key whose frame is still in the live index must NOT be flagged"
    );
    let dropped_entry = index
        .idemp
        .get(dropped_id)
        .expect("dropped-frame key was recorded");
    assert!(
        dropped_entry.is_event_evicted(),
        "PROPERTY: the compaction-tail sweep must flag a durable idempotency \
         entry whose event frame is no longer in the live index; the no-op \
         mutant leaves it dishonestly unflagged"
    );
}

#[test]
fn default_idempotency_window_constants_hold_their_exact_values() {
    // Round-3 mutation kill: idemp.rs:63 `*` -> `/` in `16 * 1024 * 1024`
    // (`16 * 1024 / 1024` collapses the default keep-window to 16 sequences).
    // Pin both default constants at their exact composed values so either
    // `*` operand mutating to `/` is caught.
    assert_eq!(idemp::DEFAULT_KEEP_SEQUENCES, 16_777_216);
    assert_eq!(idemp::DEFAULT_MAX_KEYS, 67_108_864);
}

#[test]
fn default_retention_window_covers_a_million_sequence_frontier() {
    // Behavioral twin of the constant pin: under the DEFAULT hybrid policy a
    // key recorded at sequence 0 is still INSIDE the window at frontier
    // 1_000_000 (the window floor saturates to 0 because the window is 16Mi
    // sequences deep). The `/` mutant shrinks the window to 16 sequences, so
    // the floor becomes 999_984 and the key ages out — a "within-window"
    // keyed retry would re-append a duplicate.
    let store = idemp::IdempotencyStore::new(
        idemp::IdempotencyRetention::default(),
        idemp::OverflowPolicy::Warn,
    );
    let genesis = make_entry(0, "entity:idemp-window", "scope:idemp-window");
    let genesis_id = genesis.event_id;
    store.record(idemp::IdempEntry::from_index_entry(&genesis, 0));

    let report = store.evict(1_000_000);

    assert_eq!(
        report.aged_out, 0,
        "nothing ages out of a 16Mi-deep window at a 1M frontier"
    );
    assert_eq!(report.remaining, 1);
    assert!(
        store.get(genesis_id).is_some(),
        "PROPERTY: the default keep-window (16 * 1024 * 1024 sequences) must \
         retain a genesis-recorded key at a 1M frontier; the 16-sequence \
         mutant window evicts it"
    );
}

#[test]
fn query_any_hits_after_returns_exactly_the_limit_smallest_sequences_past_the_cursor() {
    // Exact hit-set pin for the fact-Any fast path (`query_any_hits_after`),
    // crossing its mid-scan trim boundary: with limit 2 the trim threshold is
    // max(2*2, 2+1).min(1 << 20) = 4, and the cursor (after_seq 1, started)
    // admits four candidates (sequences 2..=5), so the amortized trim fires
    // mid-scan. The result must be EXACTLY the `limit` smallest visible
    // sequences strictly after the cursor, ascending — regardless of when
    // (or how often) the amortized trim runs.
    let index = StoreIndex::new();
    let entity_id = index
        .interner
        .intern("entity:any-after")
        .expect("intern entity");
    let scope_id = index
        .interner
        .intern("scope:any-after")
        .expect("intern scope");
    for seq in 0..6 {
        let mut entry = make_entry(seq, "entity:any-after", "scope:any-after");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        index.insert(entry);
    }
    index
        .publish(6, "test-any-after")
        .expect("publish the six seeded entries");

    let region = Region::all().with_fact(crate::coordinate::KindFilter::Any);
    let hits = index.query_hits_after(&region, 1, true, 2);
    let seqs: Vec<u64> = hits.iter().map(|h| h.global_sequence).collect();
    assert_eq!(
        seqs,
        vec![2, 3],
        "PROPERTY: the Any fast path returns exactly the `limit` smallest \
         visible sequences strictly after the cursor, in ascending order"
    );
}

#[test]
fn r4_any_fast_path_clock_range_keeps_the_in_range_band_and_drops_below_min() {
    // Kills query.rs:290 `entry.clock < min` -> `>` in `query_any_hits_after`
    // (the fact-Any fast path's INCLUSIVE clock-range lower bound; distinct
    // from the identical filter in `filter_region_hits`, which this path never
    // calls). `make_entry` sets clock == seq, so clocks 0..=5 filtered by the
    // range [2, 4] must return EXACTLY seqs {2, 3, 4}: both inclusive
    // endpoints kept, everything below min dropped. The flipped comparison
    // (`clock > min || clock > max` -> skip) instead drops the whole in-range
    // band above min and admits the below-min band, returning {0, 1, 2}.
    let index = StoreIndex::new();
    let entity_id = index
        .interner
        .intern("entity:any-clock")
        .expect("intern entity");
    let scope_id = index
        .interner
        .intern("scope:any-clock")
        .expect("intern scope");
    for seq in 0..6 {
        let mut entry = make_entry(seq, "entity:any-clock", "scope:any-clock");
        entry.entity_id = entity_id;
        entry.scope_id = scope_id;
        index.insert(entry);
    }
    index
        .publish(6, "test-any-clock")
        .expect("publish the six seeded entries");

    let region = Region::all()
        .with_fact(crate::coordinate::KindFilter::Any)
        .with_clock_range(
            crate::coordinate::ClockRange::new(2, 4).expect("2..=4 is a valid clock range"),
        );
    let hits = index.query_hits_after(&region, 0, false, 10);
    let seqs: Vec<u64> = hits.iter().map(|h| h.global_sequence).collect();
    assert_eq!(
        seqs,
        vec![2, 3, 4],
        "PROPERTY: the Any fast path's clock-range filter is inclusive on BOTH \
         endpoints and excludes every clock below min; the flipped lower-bound \
         comparison drops the in-range band and admits the below-min one instead"
    );
}

#[test]
fn idemp_authority_load_distinguishes_missing_from_unreadable() {
    use crate::store::index::idemp::{load_idempotency_authority, IdempLoad, IDEMP_FILENAME};

    // Absent file with NO expectation — the first-open case — MUST load as
    // Missing, never as an error. The NotFound guard mutant swaps outcomes.
    let dir = tempfile::TempDir::new().expect("create temp data dir");
    let missing =
        load_idempotency_authority(dir.path(), &crate::store::platform::fs::RealFs, None, None)
            .expect("an absent idemp file with no expectation is not an error");
    assert!(
        matches!(missing, IdempLoad::Missing),
        "an absent index.idemp with no expectation is Missing (first open)"
    );

    // Absent file WITH an expectation is authority LOSS, not a fresh store
    // (#189): the expectation-check removal mutant collapses this to Missing.
    let expected = crate::store::store_meta::IdempAuthorityAnchor {
        covered_global_sequence: 3,
        event_id_at: 3,
        chain_commitment: [3; 32],
    };
    let lost = load_idempotency_authority(
        dir.path(),
        &crate::store::platform::fs::RealFs,
        Some(1),
        Some(&expected),
    );
    assert!(
        matches!(
            lost,
            Err(crate::store::StoreError::IdempotencyAuthorityMissing { .. })
        ),
        "an absent index.idemp with a recorded expectation refuses as authority loss"
    );

    // Present-but-unreadable — a directory squatting on the filename makes the
    // read fail with a NON-NotFound error — MUST fail closed as CORRUPTION
    // carrying the read error, never be misreported as the silent Missing.
    std::fs::create_dir(dir.path().join(IDEMP_FILENAME))
        .expect("squat a directory on the idemp filename");
    let unreadable =
        load_idempotency_authority(dir.path(), &crate::store::platform::fs::RealFs, None, None);
    assert!(
        matches!(
            unreadable,
            Err(crate::store::StoreError::IdempotencyAuthorityCorrupt {
                kind: crate::store::IdempAuthorityCorruption::ReadFailed(_),
                ..
            })
        ),
        "an unreadable index.idemp fails closed with the read error as its typed reason"
    );
}

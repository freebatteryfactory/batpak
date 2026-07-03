//! Mutation-kill tests for the single-event / query read surface of
//! `store/read_api.rs`.
//!
//! PROVES: the read APIs are a pinned, assertable contract —
//! [`ChainVerificationReport::is_intact`] is the AND of both mismatch lists being
//! empty; [`Store::verify_chain`] counts EXACTLY the visible events and reports a
//! normally-appended chain as intact (no content-hash mismatches, no dangling
//! links); [`Store::get`] / [`Store::read_raw`] round-trip a committed event and
//! surface a `NotFound` for an unknown id; the receipt verifiers reject a receipt
//! whose event is not committed (and a denial receipt whose committed entry is
//! not a `SYSTEM_DENIAL`); the Region query family returns the exact set it is
//! scoped to; and [`Store::query_entries_after`] pages strictly by
//! `global_sequence`.
//! CATCHES: the `&&`→`||` / whole-body true|false swaps in `is_intact`; the
//! `events_checked += 1` operator swaps, the `recomputed == event_hash`
//! comparison swap, and the `prev != [0;32] && !verified` dangling-link logic in
//! `verify_chain`; the `ok_or(NotFound)` / missing-committed-event guard
//! deletions in `get` / `read_raw` / `verify_append_receipt` /
//! `verify_append_receipt_wire_detailed` / `verify_denial_receipt`; the whole-body
//! `-> vec![]` replacements across the query family; and the
//! `started = after.is_some()` / pagination-boundary swaps in
//! `query_entries_after`.
//! SEEDED: deterministic — every fixture is a normal synchronous append into a
//! fresh store, and sequence numbering is the store's own committed order.

use crate::coordinate::{Coordinate, KindFilter, Region};
use crate::event::EventKind;
use crate::id::EventId;
use crate::store::{
    AppendReceipt, ChainVerificationReport, DenialReceipt, ReceiptVerification,
    ReceiptVerificationError, Store, StoreConfig, StoreError,
};
use std::collections::BTreeMap;
use tempfile::TempDir;

/// A deterministic store with the incidental caches disabled, mirroring the
/// house style in `read_api_tests.rs`.
fn open_store() -> (TempDir, Store) {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(
        StoreConfig::new(dir.path())
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )
    .expect("open store");
    (dir, store)
}

fn append(store: &Store, entity: &str, scope: &str, kind: EventKind, n: u64) -> AppendReceipt {
    let coord = Coordinate::new(entity, scope).expect("valid coordinate");
    store
        .append(&coord, kind, &serde_json::json!({ "n": n }))
        .expect("append")
}

/// An id no committed event can carry (real ids are 128-bit content hashes), so
/// `index.get_by_id` is guaranteed to miss it.
const ABSENT_ID: u128 = 0x0000_0000_DEAD_BEEF;

#[test]
fn is_intact_is_the_and_of_both_lists_empty() {
    // Kills read_api.rs `is_intact`:
    //   `content_hash_mismatches.is_empty() && dangling_links.is_empty()`
    // * `&&`→`||`      : the "only one list non-empty" cases flip true↔false.
    // * whole body → true  : the non-empty cases would wrongly report intact.
    // * whole body → false : the empty case would wrongly report tampered.
    let intact = ChainVerificationReport::default();
    assert!(
        intact.is_intact(),
        "PROPERTY: an all-empty report is intact"
    );

    let only_content = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: vec![EventId::from_u128(1)],
        dangling_links: vec![],
    };
    assert!(
        !only_content.is_intact(),
        "PROPERTY: a content-hash mismatch alone breaks intactness; \
         the `&&`→`||` mutant (false || true) would report intact"
    );

    let only_dangling = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: vec![],
        dangling_links: vec![EventId::from_u128(2)],
    };
    assert!(
        !only_dangling.is_intact(),
        "PROPERTY: a dangling link alone breaks intactness; the `&&`→`||` mutant \
         (true || false) would report intact"
    );

    let both = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: vec![EventId::from_u128(1)],
        dangling_links: vec![EventId::from_u128(2)],
    };
    assert!(
        !both.is_intact(),
        "PROPERTY: two broken lists are not intact"
    );
}

#[test]
fn verify_chain_reports_intact_and_counts_every_visible_event() {
    // Kills read_api.rs `verify_chain`:
    // * `report.events_checked += 1` → `-=` (usize underflow panic) / `*=` (stuck
    //   at 0): the count would not equal the visible-event count.
    // * `recomputed == entry.hash_chain().event_hash` → `!=`: every event would be
    //   flagged as a content-hash mismatch and NONE inserted into the verified
    //   set, breaking intactness.
    // * `verified_hashes.insert(..)` deletion: the verified set stays empty, so
    //   every non-genesis link is then reported dangling.
    // * `prev != [0u8;32] && !verified_hashes.contains(&prev)`:
    //     - `!=`→`==` flags genesis as dangling;
    //     - `&&`→`||` and the `!contains` inversion flag intact non-genesis links.
    // * whole body → `Ok(Default::default())`: events_checked would be 0.
    let (_dir, store) = open_store();
    let kind = EventKind::custom(0x7, 1);
    for n in 0..4 {
        // Two entities so both same-entity and cross-entity prev_hash links exist.
        let entity = if n % 2 == 0 {
            "entity:vc-a"
        } else {
            "entity:vc-b"
        };
        let _ = append(&store, entity, "scope:vc", kind, n);
    }

    let visible = store.query(&Region::all()).len();
    assert!(
        visible >= 4,
        "sanity: at least the four appends are visible"
    );

    let report = store.verify_chain().expect("verify chain");
    assert_eq!(
        report.events_checked, visible,
        "PROPERTY: verify_chain recomputes exactly the visible-event set; the \
         `+=`→`*=` / body→Default mutants leave events_checked at 0"
    );
    assert!(
        report.content_hash_mismatches.is_empty(),
        "PROPERTY: a normally-appended chain has no content-hash mismatch; the \
         `==`→`!=` comparison swap would flag every event, got {:?}",
        report.content_hash_mismatches
    );
    assert!(
        report.dangling_links.is_empty(),
        "PROPERTY: every non-genesis link resolves in a normal chain; the \
         insert-deletion, `!=`→`==`, `&&`→`||`, and `!contains`-inversion mutants \
         all fabricate dangling links, got {:?}",
        report.dangling_links
    );
    assert!(
        report.is_intact(),
        "PROPERTY: a normally-appended store verifies as intact"
    );
}

#[test]
fn get_round_trips_payload_and_reports_not_found() {
    // Kills read_api.rs `get`:
    // * `.ok_or(StoreError::NotFound(event_id))` deletion / body → Err: a
    //   committed event would fail to read.
    // * body → Ok(Default): the payload would not round-trip.
    // * the NotFound guard itself: an absent id must be NotFound, not a panic or a
    //   fabricated event.
    let (_dir, store) = open_store();
    let receipt = append(
        &store,
        "entity:get",
        "scope:get",
        EventKind::custom(0x7, 2),
        7,
    );

    let stored = store.get(receipt.event_id).expect("get committed event");
    assert_eq!(
        stored.event.payload,
        serde_json::json!({ "n": 7 }),
        "PROPERTY: get returns the exact committed payload"
    );

    let err = store
        .get(EventId::from_u128(ABSENT_ID))
        .expect_err("absent id must not resolve");
    assert!(
        matches!(err, StoreError::NotFound(_)),
        "PROPERTY: an unknown id yields NotFound; got {err:?}"
    );
}

#[test]
fn read_raw_round_trips_bytes_and_reports_not_found() {
    // Kills read_api.rs `read_raw`: the `ok_or(NotFound)` guard and the
    // reader.read_entry_raw delegation (body → Err / Ok(Default) would drop the
    // committed msgpack bytes).
    let (_dir, store) = open_store();
    let receipt = append(
        &store,
        "entity:raw",
        "scope:raw",
        EventKind::custom(0x7, 3),
        11,
    );

    let stored = store.read_raw(receipt.event_id).expect("read raw bytes");
    assert!(
        !stored.event.payload.is_empty(),
        "PROPERTY: read_raw yields the stored msgpack bytes for a committed event"
    );

    let err = store
        .read_raw(EventId::from_u128(ABSENT_ID))
        .expect_err("absent id must not resolve");
    assert!(
        matches!(err, StoreError::NotFound(_)),
        "PROPERTY: read_raw reports NotFound for an unknown id; got {err:?}"
    );
}

#[test]
fn verify_append_receipt_flags_missing_committed_event() {
    // Kills read_api.rs `verify_append_receipt`'s
    //   `let Some(entry) = index.get_by_id(..) else { return Invalid(Missing) }`
    // guard: repointing an otherwise-valid receipt at an uncommitted id must yield
    // MissingCommittedEvent, not an accept or a different error.
    let (_dir, store) = open_store();
    let mut receipt = append(
        &store,
        "entity:va-missing",
        "scope:va",
        EventKind::custom(0x7, 4),
        1,
    );
    receipt.event_id = EventId::from_u128(ABSENT_ID);

    assert_eq!(
        store.verify_append_receipt(&receipt),
        ReceiptVerification::Invalid(ReceiptVerificationError::MissingCommittedEvent),
        "PROPERTY: a receipt for an uncommitted event is Invalid(MissingCommittedEvent)"
    );
}

#[test]
fn verify_append_receipt_wire_detailed_flags_missing_committed_event() {
    // Kills read_api.rs `verify_append_receipt_wire_detailed`'s missing-entry
    // guard (the `let Some(entry) = .. else return Invalid(Missing)` branch).
    let (_dir, store) = open_store();
    let _seed = append(
        &store,
        "entity:wire-missing",
        "scope:wire",
        EventKind::custom(0x7, 5),
        1,
    );

    let verification = store.verify_append_receipt_wire_detailed(
        EventId::from_u128(ABSENT_ID),
        0,
        [0u8; 32],
        [0u8; 32],
        None,
        BTreeMap::new(),
    );
    assert_eq!(
        verification,
        ReceiptVerification::Invalid(ReceiptVerificationError::MissingCommittedEvent),
        "PROPERTY: hydrating a wire receipt for an uncommitted event is \
         Invalid(MissingCommittedEvent)"
    );
}

#[test]
fn verify_denial_receipt_flags_missing_and_non_denial_kind() {
    // Kills read_api.rs `verify_denial_receipt`:
    // * the missing-entry `else { return Invalid(Missing) }` branch;
    // * the `if let Some(error) = denial_receipt_index_mismatch(..) { return
    //   Invalid(error) }` branch — a committed NON-denial entry must be rejected
    //   with DenialKindMismatch rather than delegated to the signing registry.
    let (_dir, store) = open_store();
    let receipt = append(
        &store,
        "entity:vd",
        "scope:vd",
        EventKind::custom(0x7, 6),
        1,
    );

    // A denial receipt pointing at a committed, but non-SYSTEM_DENIAL, event.
    let denial = DenialReceipt {
        event_id: receipt.event_id,
        global_sequence: receipt.global_sequence,
        disk_pos: receipt.disk_pos,
        content_hash: receipt.content_hash,
        key_id: receipt.key_id,
        signature: receipt.signature,
        extensions: receipt.extensions.clone(),
    };
    assert_eq!(
        store.verify_denial_receipt(&denial),
        ReceiptVerification::Invalid(ReceiptVerificationError::DenialKindMismatch),
        "PROPERTY: a denial receipt whose committed entry is not SYSTEM_DENIAL is \
         rejected with DenialKindMismatch"
    );

    // Same receipt, repointed at an uncommitted id.
    let mut missing = denial;
    missing.event_id = EventId::from_u128(ABSENT_ID);
    assert_eq!(
        store.verify_denial_receipt(&missing),
        ReceiptVerification::Invalid(ReceiptVerificationError::MissingCommittedEvent),
        "PROPERTY: a denial receipt for an uncommitted event is \
         Invalid(MissingCommittedEvent)"
    );
}

#[test]
fn query_family_counts_match_appended_events() {
    // Kills read_api.rs whole-body `-> vec![]` replacements on `query`,
    // `by_entity`, `by_scope`, and `by_fact`, and the distinct Region each builds
    // (a mixed-up Region would count the wrong events). Two entities / scopes /
    // kinds with different cardinalities make each count independently load-bearing.
    let (_dir, store) = open_store();
    let kind_a = EventKind::custom(0x7, 10);
    let kind_b = EventKind::custom(0x7, 11);
    for n in 0..2 {
        let _ = append(&store, "entity:qf-a", "scope:qf-1", kind_a, n);
    }
    for n in 0..3 {
        let _ = append(&store, "entity:qf-b", "scope:qf-2", kind_b, n);
    }

    assert_eq!(
        store.by_entity("entity:qf-a").len(),
        2,
        "PROPERTY: by_entity returns exactly that entity's events"
    );
    assert_eq!(store.by_entity("entity:qf-b").len(), 3);
    assert_eq!(
        store.by_scope("scope:qf-1").len(),
        2,
        "PROPERTY: by_scope returns exactly that scope's events"
    );
    assert_eq!(store.by_scope("scope:qf-2").len(), 3);
    assert_eq!(
        store.by_fact(kind_a).len(),
        2,
        "PROPERTY: by_fact returns exactly that kind's events"
    );
    assert_eq!(store.by_fact(kind_b).len(), 3);
    assert!(
        store.query(&Region::all()).len() >= 5,
        "PROPERTY: query(Region::all()) returns at least every appended event; the \
         body→vec![] mutant returns none"
    );
    // Region::all().with_fact(Exact(kind_a)) must not leak kind_b's events.
    assert_eq!(
        store
            .query(&Region::all().with_fact(KindFilter::Exact(kind_a)))
            .len(),
        2,
        "PROPERTY: an exact-kind Region filters to that kind only"
    );
}

#[test]
fn query_entries_after_pages_strictly_by_global_sequence() {
    // Kills read_api.rs `query_entries_after`:
    // * `started = after_global_sequence.is_some()` → `is_none()` / forced `false`:
    //   a Some(after) page would stop filtering and re-emit already-seen events.
    // * `after_seq = after_global_sequence.unwrap_or(0)` / the forced-`true`
    //   `is_some` mutant: the first-page (None) request would drop the
    //   global-sequence-0 event and return fewer than the full set.
    // * whole body → vec![]: every page would be empty.
    let (_dir, store) = open_store();
    let region = Region::entity("entity:page");
    let kind = EventKind::custom(0x7, 20);
    for n in 0..4 {
        let _ = append(&store, "entity:page", "scope:page", kind, n);
    }

    // First page (None) must include the whole set, INCLUDING the global-seq-0
    // event that a forced-`started = true` mutant would exclude.
    let all = store.query_entries_after(&region, None, 100);
    assert_eq!(
        all.len(),
        4,
        "PROPERTY: a None first page returns every event, seq-0 included"
    );

    let page1 = store.query_entries_after(&region, None, 2);
    assert_eq!(page1.len(), 2, "PROPERTY: limit bounds the page size");
    assert!(
        page1[0].global_sequence() < page1[1].global_sequence(),
        "PROPERTY: a page is ascending by global_sequence"
    );

    let boundary = page1[1].global_sequence();
    let page2 = store.query_entries_after(&region, Some(boundary), 100);
    assert_eq!(
        page2.len(),
        2,
        "PROPERTY: resuming after the boundary returns exactly the remainder; the \
         `is_some`→`is_none` / `started=false` mutant re-emits the first page"
    );
    for entry in &page2 {
        assert!(
            entry.global_sequence() > boundary,
            "PROPERTY: every resumed entry is strictly past the boundary {boundary}"
        );
    }

    assert!(
        store.query_entries_after(&region, None, 0).is_empty(),
        "PROPERTY: a zero limit returns an empty page"
    );
}

#[test]
fn lane_reads_reflect_the_default_lane() {
    // Kills read_api.rs whole-body `-> vec![]` / `-> None` replacements on
    // `query_lane`, `by_entity_lane`, `stream_lane`, and `latest_lane`. Normal
    // appends land on lane 0, so the lane-0 reads must equal the un-laned reads.
    let (_dir, store) = open_store();
    let kind = EventKind::custom(0x7, 30);
    let mut last = None;
    for n in 0..2 {
        last = Some(append(&store, "entity:lane", "scope:lane", kind, n));
    }
    let last = last.expect("appended at least once");

    assert_eq!(
        store.by_entity_lane("entity:lane", 0).len(),
        2,
        "PROPERTY: by_entity_lane(_, 0) sees every default-lane event"
    );
    assert_eq!(
        store.stream_lane("entity:lane", 0).len(),
        2,
        "PROPERTY: stream_lane delegates to by_entity_lane"
    );
    assert_eq!(
        store.query_lane(&Region::entity("entity:lane"), 0).len(),
        2,
        "PROPERTY: query_lane on lane 0 returns the default-lane events"
    );

    let latest = store
        .latest_lane("entity:lane", 0)
        .expect("PROPERTY: latest_lane must find the last default-lane event");
    assert_eq!(
        latest.event_id(),
        last.event_id,
        "PROPERTY: latest_lane returns the most recent event on the lane; the \
         body→None mutant would drop it"
    );
}

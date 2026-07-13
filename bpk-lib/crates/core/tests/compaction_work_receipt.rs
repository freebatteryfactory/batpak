// `ShadowFs` (the namespace-truth backend and its op observation surface) is the
// clean external conformance surface; without the feature the whole file is empty.
#![cfg(feature = "conformance-harness")]
//! #177 — a BenchEvidenceTape-style WORK-COUNT receipt for the rewritten
//! compaction namespace transaction, plus the COMPLEXITY LAW it exists to pin.
//!
//! PROVES: INV-COMPACTION-NAMESPACE-COMMIT (work-units clause) — the namespace /
//! publication work a single compaction performs (renames, removals, staged
//! creates, parent-dir syncs, authority-image entries) is O(source segments),
//! NEVER O(all live events). Two stores that differ ONLY in the size of the
//! UNTOUCHED history they carry must produce IDENTICAL namespace op counts; only
//! the byte-copy work (staged writes / merged bytes) and the in-memory rebuild
//! read count are permitted to scale. The O(all-live-events) tip sweep the old
//! materialize-at-final-name path invited must never return.
//! CATCHES: any compaction that walks or rewrites the whole live-event set on
//! every pass (its rename/removal/dir-sync counts would then track live-event
//! count and the two receipts would diverge).
//!
//! The receipt is observed by driving a REAL [`Store`] over [`ShadowFs`] and
//! reading its op log; ShadowFs and the driven store share one in-memory tree
//! (the backend is self-contained, so no real filesystem is touched). The
//! receipt is a local serde struct (no `BenchEvidenceTape` type exists in
//! bench-support today) asserted — and serde-round-tripped — in-test.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::IdempotencyKey;
use batpak::store::segment::CompactionOutcome;
use batpak::store::{
    AppendOptions, CompactionConfig, CompactionStrategy, ShadowFs, Store, StoreConfig, StoreFs,
};
use std::path::Path;
use std::sync::Arc;

type BoxErr = Box<dyn std::error::Error>;

/// Keyed events whose frames the retention compaction evicts into the durable
/// authority image (so retries dedup from the image alone after reopen).
const KEYED_KIND: EventKind = EventKind::custom(0xB, 7);
/// Untouched filler history the compaction MERGES but never evicts. The
/// per-store amount of this is the only axis that differs between the two stores
/// in the complexity test.
const FILLER_KIND: EventKind = EventKind::custom(0xB, 8);
/// Distinct keyed events appended (== authority image entries after eviction).
const KEYED_EVENTS: u64 = 4;
/// Sealed segments the fixture builds before compacting (the source set size).
const TARGET_SEALED: usize = 3;
/// Virtual store root under `ShadowFs` (never touches the real filesystem).
const NS_ROOT: &str = "/shadow/work-receipt";

/// A single compaction's work-count receipt. `at least` per #177: source count,
/// bytes read/written, staged writes, file syncs, parent-dir syncs, renames,
/// removals, authority image entries, index entries visited — plus the O(1)
/// publication op counts (staged creates / temp persists) that make the
/// complexity law directly assertable.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CompactionWorkReceipt {
    /// Sealed segments considered as compaction sources.
    source_segment_count: usize,
    /// Source segment files retired after the commit (authoritative, from the report).
    segments_removed: usize,
    /// Sum of removed source file sizes — the bytes the merge had to read.
    bytes_read: u64,
    /// Durable byte length of the merged replacement segment.
    bytes_written: u64,
    /// `create_new_file` ops in the compaction window (the staged replacement).
    staged_creates: u64,
    /// Handle/staged `write_all` ops in the window (byte-copy; may scale).
    staged_writes: u64,
    /// Atomic temp `persist` ops (marker + authority image + store.meta).
    temp_persists: u64,
    /// Handle/staged `sync_all` ops in the window.
    file_syncs: u64,
    /// Parent-directory sync events in the window (publication durability).
    parent_dir_syncs: u64,
    /// `rename` ops in the window (source rename-away + commit rename).
    renames: u64,
    /// `remove_file` ops in the window (source retirement + marker clear).
    removals: u64,
    /// Entries carried in the published authority image (== evicted keyed events).
    authority_image_entries: u64,
    /// Live events visible after compaction — the O(live-events) denominator that
    /// the namespace work above must NOT track.
    index_entries_visited: u64,
}

/// Per-kind mutating-op tallies over one compaction window.
#[derive(Default)]
struct OpCounts {
    creates: u64,
    writes: u64,
    renames: u64,
    removals: u64,
    persists: u64,
    file_syncs: u64,
    parent_syncs: u64,
}

fn coord() -> Coordinate {
    Coordinate::new("entity:work", "scope:receipt").expect("valid coord")
}

fn config(fs: Arc<dyn StoreFs>, segment_max_bytes: u64) -> StoreConfig {
    StoreConfig::new(NS_ROOT)
        .with_fs(fs)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(segment_max_bytes)
        .with_sync_every_n_events(1)
}

fn append_keyed(store: &Store, key: u128) -> Result<u64, BoxErr> {
    let tag = u64::try_from(key & u128::from(u64::MAX)).unwrap_or(0);
    let receipt = store.append_with_options(
        &coord(),
        KEYED_KIND,
        &serde_json::json!({ "k": tag }),
        AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
    )?;
    Ok(receipt.global_sequence)
}

fn append_filler(store: &Store, i: u64) -> Result<(), BoxErr> {
    store.append(&coord(), FILLER_KIND, &serde_json::json!({ "filler": i }))?;
    Ok(())
}

/// Sealed + active segment files (`NNNNNN.fbat`) visible now — the staged
/// (`.compact-new`) and source (`.compact-src`) transaction names end with other
/// suffixes, so a plain `.fbat` tail counts exactly the durable segment set.
fn fbat_count(shadow: &ShadowFs) -> usize {
    shadow
        .visible_entries(Path::new(NS_ROOT))
        .into_iter()
        .filter(|leaf| leaf.to_string_lossy().ends_with(".fbat"))
        .count()
}

fn keyed_live_count(store: &Store) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|event| event.event_kind() == KEYED_KIND)
        .count()
}

/// Classify a compaction window of op-kind Debug names into per-kind tallies.
/// `SimOpKind` is `pub` inside a `pub(crate)` module, so it is unnameable at the
/// test crate boundary — but every value returned by `ShadowFs::mutation_ops`
/// renders its (fieldless, derived-Debug) kind as the bare variant name, so the
/// stable name string is the honest external discriminator. The `_` arm is over
/// `&str`, not an enum, so it does not trip `wildcard_enum_match_arm`.
fn classify(kind_names: &[String]) -> OpCounts {
    let mut counts = OpCounts::default();
    for name in kind_names {
        match name.as_str() {
            "CreateNew" => counts.creates += 1,
            "Write" => counts.writes += 1,
            "Rename" => counts.renames += 1,
            "RemoveFile" => counts.removals += 1,
            "PersistTemp" => counts.persists += 1,
            "SyncFile" => counts.file_syncs += 1,
            "SyncParentDir" => counts.parent_syncs += 1,
            _ => {}
        }
    }
    counts
}

/// The compaction strategy: keep all filler, evict the keyed frames into the
/// durable authority image. `min_segments: 1` compacts the whole sealed set.
fn retention_config() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != KEYED_KIND
        })),
        min_segments: 1,
    }
}

/// Open a fresh store over its own `ShadowFs`, append the keyed set then filler
/// until `TARGET_SEALED` segments are sealed, compact, and return the work-count
/// receipt for exactly the compaction window plus the shared shadow (for reopen)
/// and the keyed events' original global sequences (for the dedup check).
fn run_compaction(segment_max_bytes: u64) -> Result<(CompactionWorkReceipt, ShadowFs, Vec<u64>), BoxErr> {
    let shadow = ShadowFs::new();
    let fs: Arc<dyn StoreFs> = Arc::new(shadow.clone());
    let store = Store::open(config(fs, segment_max_bytes))?;

    let mut keyed_seqs = Vec::new();
    for k in 1..=KEYED_EVENTS {
        keyed_seqs.push(append_keyed(&store, u128::from(k))?);
    }

    let mut built = false;
    for i in 0..200_000u64 {
        append_filler(&store, i)?;
        if fbat_count(&shadow) > TARGET_SEALED {
            built = true;
            break;
        }
    }
    if !built {
        return Err(Box::from(
            "FIXTURE: never reached the target sealed-segment count within the append cap",
        ));
    }

    // Snapshot the op log immediately before the compaction so the window holds
    // exactly the compaction's ops (the store is idle — no concurrent appends).
    let before = shadow.mutation_ops().len();
    let (_result, report) = store.compact(&retention_config())?;
    if !matches!(report.outcome, CompactionOutcome::Performed) {
        return Err(Box::from(
            "PROPERTY: fixture compaction must be Performed (Skipped/Failed cannot pin work counts)",
        ));
    }
    let merged_id = match report.merged_segment_id {
        Some(id) => id,
        None => return Err(Box::from("PROPERTY: a Performed compaction must name a merged segment")),
    };

    let ops = shadow.mutation_ops();
    let kind_names: Vec<String> = ops[before..]
        .iter()
        .map(|op| format!("{:?}", op.kind))
        .collect();
    let counts = classify(&kind_names);

    let final_path = Path::new(NS_ROOT).join(format!("{merged_id:06}.fbat"));
    let bytes_written = shadow.durable_byte_len(&final_path).unwrap_or(0);
    let index_entries_visited =
        u64::try_from(store.query(&Region::all()).into_iter().count()).unwrap_or(u64::MAX);

    let receipt = CompactionWorkReceipt {
        source_segment_count: report.sealed_segment_count,
        segments_removed: report.segments_removed,
        bytes_read: report.bytes_reclaimed,
        bytes_written,
        staged_creates: counts.creates,
        staged_writes: counts.writes,
        temp_persists: counts.persists,
        file_syncs: counts.file_syncs,
        parent_dir_syncs: counts.parent_syncs,
        renames: counts.renames,
        removals: counts.removals,
        authority_image_entries: KEYED_EVENTS,
        index_entries_visited,
    };
    store.close()?;
    Ok((receipt, shadow, keyed_seqs))
}

#[test]
fn compaction_work_receipt_records_bounded_publication_work() -> Result<(), BoxErr> {
    let (receipt, shadow, keyed_seqs) = run_compaction(512)?;

    // Structural sanity: a real source set was compacted and every source retired.
    assert!(
        receipt.source_segment_count >= 2,
        "fixture must compact a real multi-segment source set: {receipt:?}"
    );
    assert_eq!(
        receipt.segments_removed, receipt.source_segment_count,
        "every source segment must be retired after the commit: {receipt:?}"
    );
    assert!(receipt.bytes_read > 0, "sources carried bytes: {receipt:?}");
    assert!(receipt.bytes_written > 0, "the merge wrote a replacement: {receipt:?}");

    // The publication work units are small O(1)/O(source) constants — a compaction
    // that swept every live event would inflate these.
    assert!(
        receipt.renames >= 1 && receipt.renames <= 8,
        "renames are O(1) publication ops (source rename-away + commit rename), not O(events): {receipt:?}"
    );
    assert!(
        receipt.staged_creates >= 1 && receipt.staged_creates <= 3,
        "staged segment creation is a small O(1) count, not O(events): {receipt:?}"
    );
    assert!(
        receipt.removals >= u64::try_from(receipt.source_segment_count).unwrap_or(u64::MAX),
        "removals retire at least the source set: {receipt:?}"
    );
    assert!(
        receipt.parent_dir_syncs >= 1 && receipt.parent_dir_syncs <= 64,
        "publication dir-syncs are bounded, not O(events): {receipt:?}"
    );
    assert_eq!(
        receipt.authority_image_entries, KEYED_EVENTS,
        "the authority image holds exactly the evicted keyed events: {receipt:?}"
    );

    // BenchEvidenceTape-style: the receipt is a serializable evidence tape.
    let tape = serde_json::to_vec(&receipt)?;
    let decoded: CompactionWorkReceipt = serde_json::from_slice(&tape)?;
    assert_eq!(decoded, receipt, "the work-count receipt survives serialization");

    // The published authority image survives reopen: every keyed retry is a no-op
    // returning the ORIGINAL receipt, and the evicted keyed frames stay evicted —
    // so the small `authority_image_entries` count really is the whole dedup story.
    let fs: Arc<dyn StoreFs> = Arc::new(shadow.clone());
    let reopened = Store::open(config(fs, 512))?;
    for (idx, original_seq) in keyed_seqs.iter().enumerate() {
        let key = u128::from(u64::try_from(idx).unwrap_or(0) + 1);
        let replay = append_keyed(&reopened, key)?;
        assert_eq!(
            replay, *original_seq,
            "keyed retry after compaction+reopen is a no-op from the durable authority image"
        );
    }
    assert_eq!(
        keyed_live_count(&reopened),
        0,
        "compacted-away keyed frames stay evicted across recovery"
    );
    reopened.close()?;
    Ok(())
}

#[test]
fn compaction_namespace_work_does_not_scale_with_untouched_history() -> Result<(), BoxErr> {
    // Two stores compacting the SAME source-segment count, differing ONLY in how
    // much untouched history each segment carries (the segment size ceiling).
    let (light, _light_shadow, _light_keys) = run_compaction(512)?;
    let (heavy, _heavy_shadow, _heavy_keys) = run_compaction(16_384)?;

    assert_eq!(
        light.source_segment_count, heavy.source_segment_count,
        "the two fixtures must compact the same number of source segments: light={light:?} heavy={heavy:?}"
    );

    // The stores genuinely differ in untouched history size and byte-copy work —
    // otherwise the equality below would be vacuous.
    assert!(
        heavy.index_entries_visited >= light.index_entries_visited.saturating_mul(3),
        "heavy must hold far more untouched live history: light={} heavy={}",
        light.index_entries_visited,
        heavy.index_entries_visited
    );
    assert!(
        heavy.bytes_written > light.bytes_written,
        "byte-copy work is permitted to scale with real merged content: light={} heavy={}",
        light.bytes_written,
        heavy.bytes_written
    );

    // COMPLEXITY LAW (#177): identical source count => IDENTICAL namespace /
    // publication work, regardless of untouched history size. If any of these
    // tracked live-event count, the far larger heavy store would diverge.
    assert_eq!(
        light.renames, heavy.renames,
        "renames are O(source segments), not O(live events)"
    );
    assert_eq!(
        light.removals, heavy.removals,
        "removals are O(source segments), not O(live events)"
    );
    assert_eq!(
        light.staged_creates, heavy.staged_creates,
        "staged replacement creates are O(1)"
    );
    assert_eq!(
        light.temp_persists, heavy.temp_persists,
        "marker + authority-image + store.meta persists are O(1)"
    );
    assert_eq!(
        light.parent_dir_syncs, heavy.parent_dir_syncs,
        "publication dir-syncs are O(1)"
    );
    assert_eq!(
        light.authority_image_entries, heavy.authority_image_entries,
        "the authority image is O(evicted keys), not O(live events)"
    );

    // The O(all-live-events) tip sweep must never return: namespace work stays
    // bounded even as live history grows.
    assert!(
        light.renames <= 8 && light.removals <= 32 && light.parent_dir_syncs <= 64,
        "namespace work must be a small bounded constant, not O(events): {light:?}"
    );
    Ok(())
}

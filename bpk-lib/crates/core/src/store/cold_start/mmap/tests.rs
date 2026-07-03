use super::*;
use crate::coordinate::Coordinate;
use crate::event::{EventKind, HashChain};
use crate::store::index::{DiskPos, StoreIndex};
use std::collections::BTreeMap;
use tempfile::TempDir;

fn make_index(count: u64) -> StoreIndex {
    let idx = StoreIndex::new();
    for i in 0..count {
        let coord = Coordinate::new(format!("entity:{i}"), "scope:test").expect("valid coordinate");
        let entity_id = idx.interner.intern(coord.entity()).expect("intern");
        let scope_id = idx.interner.intern(coord.scope()).expect("intern");
        idx.insert(IndexEntry {
            event_id: (i + 1) as u128,
            correlation_id: (i + 1) as u128,
            causation_id: (i > 0).then_some(i as u128),
            coord,
            entity_id,
            scope_id,
            kind: EventKind::custom(
                0x1,
                u16::try_from(i & 0x0FFF).expect("masked to 12 bits, fits u16"),
            ),
            wall_ms: 10_000 + i,
            clock: u32::try_from(i).expect("fits u32"),
            dag_lane: 0,
            dag_depth: 0,
            hash_chain: HashChain::default(),
            disk_pos: DiskPos {
                segment_id: 7,
                offset: i * 64,
                length: 64,
            },
            global_sequence: i,
            receipt_extensions: BTreeMap::new(),
        });
    }
    idx
}

#[test]
fn mmap_index_roundtrip_restores_entries() {
    let tmp = TempDir::new().expect("temp dir");
    let segment_path = tmp.path().join(crate::store::segment::segment_filename(7));
    crate::store::platform::fs::write_derivative_file_atomically(
        tmp.path(),
        &segment_path,
        "test segment",
        &vec![0u8; 4096],
    )
    .expect("segment file");

    let src = make_index(8);
    write_mmap_index(&src, tmp.path(), 7, 512).expect("write mmap index");

    let snapshot = try_load_mmap_snapshot(tmp.path(), &crate::store::SystemClock::new())
        .expect("load snapshot");
    assert_eq!(snapshot.routing.entry_count, 8);
    assert!(
        !snapshot.routing.chunks.is_empty(),
        "v2 mmap index must persist chunk summaries"
    );

    let dst = StoreIndex::new();
    let restored = try_restore_mmap_index(&dst, tmp.path()).expect("restore");
    assert_eq!(restored.0.watermark_segment_id, 7);
    assert_eq!(restored.0.watermark_offset, 512);
    assert_eq!(dst.len(), 8);
    assert_eq!(dst.visible_sequence(), 8);
}

#[test]
fn mmap_index_roundtrip_restores_receipt_extensions() {
    let tmp = TempDir::new().expect("temp dir");
    let segment_path = tmp.path().join(crate::store::segment::segment_filename(7));
    crate::store::platform::fs::write_derivative_file_atomically(
        tmp.path(),
        &segment_path,
        "test segment",
        &vec![0u8; 4096],
    )
    .expect("segment file");

    let idx = StoreIndex::new();
    let coord = Coordinate::new("entity:mmap-ext", "scope:test").expect("coord");
    let entity_id = idx.interner.intern(coord.entity()).expect("intern");
    let scope_id = idx.interner.intern(coord.scope()).expect("intern");
    let mut receipt_extensions = BTreeMap::new();
    receipt_extensions.insert(
        ExtensionKey::new("app.audit").expect("valid extension key"),
        vec![0xFA, 0xCE, 0x05],
    );
    idx.insert(IndexEntry {
        event_id: 1,
        correlation_id: 1,
        causation_id: None,
        coord,
        entity_id,
        scope_id,
        kind: EventKind::DATA,
        wall_ms: 1_700_000_000_000,
        clock: 1,
        dag_lane: 0,
        dag_depth: 0,
        hash_chain: HashChain::default(),
        disk_pos: DiskPos {
            segment_id: 7,
            offset: 0,
            length: 64,
        },
        global_sequence: 0,
        receipt_extensions: receipt_extensions.clone(),
    });

    write_mmap_index(&idx, tmp.path(), 7, 512).expect("write mmap index");

    let snapshot = try_load_mmap_snapshot(tmp.path(), &crate::store::SystemClock::new())
        .expect("load snapshot");
    assert!(
        snapshot.receipt_extensions_hydrated,
        "PROPERTY: mmap v5 snapshots must carry receipt-extension maps directly."
    );
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        snapshot.entries[0].receipt_extensions, receipt_extensions,
        "PROPERTY: mmap v5 extension blob table must preserve opaque receipt-extension bytes."
    );
}

// ── Mutation-kill: decode_receipt_extensions_from_blob bounds + digest ────────
//
// The blob decoder validates an entry's `[extension_offset, +extension_len)`
// slice against the shared blob before decoding: `extension_len == 0`
// short-circuits to an empty map; `offset + len` uses `checked_add`; the slice
// must satisfy `end <= blob.len()`; and the slice digest must equal the
// recorded `extension_hash`. The happy-path roundtrip above only exercises the
// success arm, so these pin the fail-closed edges.
#[test]
fn decode_receipt_extensions_from_blob_validates_bounds_and_digest() {
    let mut map = BTreeMap::new();
    map.insert(
        ExtensionKey::new("app.audit").expect("valid extension key"),
        vec![0x01u8, 0x02, 0x03],
    );
    let bytes = super::encode_receipt_extensions(&map).expect("encode extensions");
    assert!(
        !bytes.is_empty(),
        "SANITY: a non-empty map encodes to bytes"
    );
    let blob_len = u64::try_from(bytes.len()).expect("blob length fits u64");
    let good_hash = super::extension_blob_digest(&bytes);

    // A backing IndexEntry only supplies default (all-zero) extension fields;
    // each case overwrites offset/len/hash to isolate one validation edge.
    let idx = make_index(1);
    let backing = idx.all_entries();
    let base = &backing[0];

    // Exact fit: end == blob.len(). The real guard is `end > blob.len()`, so the
    // `> -> >=` mutant would reject this legitimate exact-fill slice.
    let mut exact = super::format::MmapIndexEntry::from_index_entry(base);
    exact.extension_offset = 0;
    exact.extension_len = blob_len;
    exact.extension_hash = good_hash;
    let decoded = super::decode_receipt_extensions_from_blob(&exact, &bytes)
        .expect("exact-fit slice decodes");
    assert_eq!(
        decoded, map,
        "PROPERTY: a slice that exactly fills the blob must decode (kills `end > blob.len()` -> \
         `>=` and the `!=` digest-guard inversion, which would both reject this valid slice)"
    );

    // extension_len == 0 short-circuits to an empty map without touching the
    // blob or checking the (zeroed) hash. Kills `== 0` -> `!= 0`.
    let mut empty = super::format::MmapIndexEntry::from_index_entry(base);
    empty.extension_offset = 0;
    empty.extension_len = 0;
    empty.extension_hash = [0u8; 32];
    assert!(
        super::decode_receipt_extensions_from_blob(&empty, &bytes)
            .expect("zero-length extension decodes")
            .is_empty(),
        "PROPERTY: extension_len == 0 short-circuits to an empty map"
    );

    // Digest mismatch on a well-sized slice must fail closed. Kills a deleted
    // digest check / `!=` -> `==`.
    let mut wrong_hash = super::format::MmapIndexEntry::from_index_entry(base);
    wrong_hash.extension_offset = 0;
    wrong_hash.extension_len = blob_len;
    wrong_hash.extension_hash = [0xFFu8; 32];
    assert!(
        super::decode_receipt_extensions_from_blob(&wrong_hash, &bytes).is_err(),
        "PROPERTY: a slice whose digest disagrees with the recorded hash must fail closed"
    );

    // end past the blob (len one byte too long) must fail closed. Kills removal
    // of the `end > blob.len()` bound entirely.
    let mut over = super::format::MmapIndexEntry::from_index_entry(base);
    over.extension_offset = 0;
    over.extension_len = blob_len + 1;
    over.extension_hash = good_hash;
    assert!(
        super::decode_receipt_extensions_from_blob(&over, &bytes).is_err(),
        "PROPERTY: an extension range extending past the blob must fail closed"
    );

    // offset + len overflowing usize must fail closed via checked_add, not wrap.
    let mut overflow = super::format::MmapIndexEntry::from_index_entry(base);
    overflow.extension_offset = usize::MAX as u64;
    overflow.extension_len = 1;
    overflow.extension_hash = good_hash;
    assert!(
        super::decode_receipt_extensions_from_blob(&overflow, &bytes).is_err(),
        "PROPERTY: offset + len that overflows usize must fail closed (kills checked_add -> \
         wrapping_add)"
    );
}

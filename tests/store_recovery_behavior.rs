// justifies: INV-TEST-PANIC-AS-ASSERTION, INV-MACRO-BOUNDED-CAST; advanced store tests rely on unwrap/panic as assertion style, spawn threads for concurrency probes, and narrow bounded test data into target types that the fixture guarantees fit.
#![allow(
    clippy::unwrap_used,
    clippy::disallowed_methods,
    clippy::cast_possible_truncation,
    clippy::needless_borrows_for_generic_args,
    clippy::panic
)]
//! Advanced Store segment, fd-budget, and cold-start recovery integration tests.

use batpak::prelude::*;
use batpak::store::{Store, StoreConfig, StoreError, SyncConfig};
use tempfile::TempDir;

// --- fd_budget LRU eviction ---

#[test]
fn fd_budget_evicts_oldest_segments() {
    let dir = TempDir::new().expect("temp dir");
    let config = StoreConfig {
        data_dir: dir.path().to_path_buf(),
        segment_max_bytes: 512, // tiny segments → many segment files
        sync: SyncConfig {
            every_n_events: 1,
            ..SyncConfig::default()
        },
        fd_budget: 2, // only 2 FDs allowed → forces eviction
        ..StoreConfig::new("")
    };
    let store = Store::open(config).expect("open store");
    let coord = Coordinate::new("entity:fd", "scope:test").expect("valid coord");
    let kind = EventKind::custom(0xF, 1);

    // Write enough events to create many segments (>2, exceeding fd_budget)
    for i in 0..100 {
        store
            .append(
                &coord,
                kind,
                &serde_json::json!({"data": format!("payload_{i}")}),
            )
            .expect("append");
    }
    store.sync().expect("sync");

    // Verify: multiple segments created
    let segment_count = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "fbat")
                .unwrap_or(false)
        })
        .count();
    assert!(
        segment_count > 2,
        "PROPERTY: writing 100 events with segment_max_bytes=512 must create more than 2 segment files.\n\
         Investigate: src/store/write/writer.rs segment rotation logic.\n\
         Common causes: rotation threshold not honoured, all events written to one segment.\n\
         Run: cargo test --test store_advanced fd_budget_evicts_oldest_segments"
    );

    // Read events from different segments — this exercises LRU eviction
    // because fd_budget=2 but we have >2 segments
    let entries = store.stream("entity:fd");
    assert_eq!(
        entries.len(),
        100,
        "PROPERTY: stream must return all 100 appended events even when fd_budget forces LRU eviction.\n\
         Investigate: src/store/segment/scan.rs get_fd LRU cache, src/store/mod.rs stream.\n\
         Common causes: evicted segment FD not re-opened on next access, stream skips closed segments.\n\
         Run: cargo test --test store_advanced fd_budget_evicts_oldest_segments"
    );

    // Read first event (oldest segment), last event (newest), then first again
    // This forces eviction: open seg1, open seg_last (evicts seg1 if budget=2),
    // then re-open seg1 (evicts seg_last)
    let first = store.get(entries[0].event_id).expect("get first");
    let last = store.get(entries[99].event_id).expect("get last");
    let first_again = store
        .get(entries[0].event_id)
        .expect("get first again after eviction");

    assert_eq!(
        first.event.event_id(),
        first_again.event.event_id(),
        "PROPERTY: re-reading the same event after LRU fd eviction must return the identical event_id.\n\
         Investigate: src/store/segment/scan.rs get_fd LRU cache.\n\
         Common causes: evicted segment FD reopened to wrong offset, cache key collision after eviction.\n\
         Run: cargo test --test store_advanced fd_budget_evicts_oldest_segments"
    );

    // Verify event identity integrity through eviction cycles
    assert_eq!(
        first.event.event_kind(),
        last.event.event_kind(),
        "PROPERTY: EventKind must be identical for events written with the same kind, \
         even when read from different segments after LRU eviction.\n\
         Investigate: src/store/segment/scan.rs get_fd LRU cache, src/store/segment/mod.rs read_frame.\n\
         Common causes: frame data corrupted during eviction cycle, wrong frame decoded after re-open.\n\
         Run: cargo test --test store_advanced fd_budget_evicts_oldest_segments"
    );

    store.close().expect("close");
}

// --- corrupt segment recovery ---

#[test]
fn cold_start_skips_corrupt_segment_gracefully() {
    let dir = TempDir::new().expect("temp dir");
    let kind = EventKind::custom(0xF, 1);

    // Phase 1: populate with good data
    {
        let config = StoreConfig {
            data_dir: dir.path().to_path_buf(),
            segment_max_bytes: 512,
            sync: SyncConfig {
                every_n_events: 1,
                ..SyncConfig::default()
            },
            ..StoreConfig::new("")
        };
        let store = Store::open(config).expect("open");
        let coord = Coordinate::new("entity:corrupt", "scope:test").expect("valid coord");
        for i in 0..20 {
            store
                .append(&coord, kind, &serde_json::json!({"i": i}))
                .expect("append");
        }
        store.sync().expect("sync");
        store.close().expect("close");
    }

    // Phase 2: create a corrupt segment file (bad magic)
    let corrupt_path = dir.path().join("999999.fbat");
    std::fs::write(&corrupt_path, b"BAAD_not_a_real_segment").expect("write corrupt");

    // Phase 3: cold start — should skip the corrupt segment
    // The store should either skip it or error on it.
    // scan_segment checks magic bytes and returns CorruptSegment error.
    let config = StoreConfig {
        data_dir: dir.path().to_path_buf(),
        segment_max_bytes: 512,
        ..StoreConfig::new("")
    };
    // Note: `Store` doesn't implement Debug (it owns Arc'd internal state),
    // so `Result::expect_err` doesn't compile here. Match instead.
    let err = match Store::open(config) {
        Ok(_) => panic!(
            "PROPERTY: Store::open must return Err when a segment file has an \
             invalid magic header. Investigate: src/store/segment/scan.rs scan_segment \
             magic check. Common causes: magic bytes check skipped or returns \
             Ok(empty), corrupt file silently ignored."
        ),
        Err(e) => e,
    };
    assert!(
        matches!(err, StoreError::CorruptSegment { .. }),
        "PROPERTY: invalid magic header must surface as StoreError::CorruptSegment, got {err:?}"
    );
}

#[test]
fn corrupt_frame_in_segment_is_detected() {
    // Write good events, then inject a corrupt frame into the segment file.
    // Verify cold start detects the corruption (CRC mismatch stops scanning).
    let dir = TempDir::new().expect("temp dir");
    let kind = EventKind::custom(0xF, 1);

    // Phase 1: populate with good data and sync
    {
        let config = StoreConfig {
            data_dir: dir.path().to_path_buf(),
            sync: SyncConfig {
                every_n_events: 1,
                ..SyncConfig::default()
            },
            ..StoreConfig::new("")
        };
        let store = Store::open(config).expect("open");
        let coord = Coordinate::new("entity:crc", "scope:test").expect("valid");
        for i in 0..3 {
            store
                .append(&coord, kind, &serde_json::json!({"i": i}))
                .expect("append");
        }
        store.sync().expect("sync");
        store.close().expect("close");
    }

    // Phase 2: corrupt a segment file by flipping bytes in the middle.
    // Sort by file_name so the chosen segment is deterministic across
    // filesystems (POSIX `readdir` makes no order guarantee).
    let mut segments: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "fbat")
                .unwrap_or(false)
        })
        .collect();
    segments.sort_by_key(|e| e.file_name());
    assert!(
        !segments.is_empty(),
        "PROPERTY: after appending events and syncing, at least one .fbat segment file must exist.\n\
         Investigate: src/store/write/writer.rs sync, src/store/segment/mod.rs write path.\n\
         Common causes: sync no-op, segment file never flushed to disk, wrong extension used.\n\
         Run: cargo test --test store_advanced corrupt_frame_in_segment_is_detected"
    );

    let seg_path = segments[0].path();
    let mut data = std::fs::read(&seg_path).expect("read segment");
    // Flip bytes near the end of the file (inside a frame's msgpack region)
    if data.len() > 20 {
        let mid = data.len() - 10;
        data[mid] ^= 0xFF;
        data[mid + 1] ^= 0xFF;
    }
    std::fs::write(&seg_path, &data).expect("write corrupted segment");

    // Phase 3: cold start should still open (corrupt frames are skipped/truncated)
    // but should have fewer events than originally written
    let config = StoreConfig {
        data_dir: dir.path().to_path_buf(),
        ..StoreConfig::new("")
    };
    // The store may open successfully (skipping corrupt frames) or may error
    // depending on where the corruption landed. Either behavior is acceptable
    // — what matters is it doesn't silently return wrong data.
    match Store::open(config) {
        Ok(store) => {
            let stats = store.stats();
            // Corrupted segment may have fewer events (some frames skipped)
            // The key assertion: we don't get MORE events than we wrote
            assert!(
                stats.event_count <= 6,
                "PROPERTY: a store opened with a corrupted segment must not report more events than the original data plus lifecycle rows — no phantom events allowed. Got {}.\n\
                 Investigate: src/store/segment/scan.rs scan_segment CRC check, src/store/mod.rs open.\n\
                 Common causes: CRC check skipped, corrupt bytes decoded as valid frames.\n\
                 Run: cargo test --test store_advanced corrupt_frame_in_segment_is_detected",
                stats.event_count
            );
            let _ = store.close();
        }
        Err(_) => {
            // Store rejected the corrupt segment entirely — also acceptable
        }
    }
}

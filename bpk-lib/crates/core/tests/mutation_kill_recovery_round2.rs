// PROVES: LAW-007 (Codebase Accuses Itself — adversarial self-testing)
// CATCHES: FM-019 (Non-Replayable Truth — tail replay renumbers recovered events after a crash)
// SEEDED: deterministic crash-image copy of a live data dir (no randomness; sequences pinned to pre-crash receipts)
//! Mutation-kill tests — cold-start recovery, round 2 (WP-B).
//!
//! Each test pins the exact observable behavior of a public-API recovery path
//! so a specific surviving mutant is caught: a test here fails iff that
//! mutation is applied.
//!
//! Covered mutant: `allocator_floor > 0` -> `allocator_floor < 0` in
//! `cold_start::rebuild::collect_tail_entries` (rebuild.rs:570). The sequence
//! tracker seeds `inserted_any` from a NONZERO checkpoint allocator floor; the
//! mutant makes the first synthesized tail sequence restart at 0 and shifts
//! every later synthesized sequence down by one — silently renumbering the
//! recovered log relative to the receipts handed out before the crash.

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::store::cold_start::rebuild::OpenIndexPath;
use batpak::store::{Store, StoreConfig};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xE, 0x51);
const ENTITY: &str = "entity:tail-floor";

fn recovery_config(path: &Path) -> StoreConfig {
    StoreConfig::new(path)
        .with_enable_checkpoint(true)
        .with_enable_mmap_index(false)
        .with_sync_every_n_events(1)
}

/// Copy a crash image of `src` into `dst`: every regular file except the live
/// dir lock (a real crash never carries a held lock into the next boot).
fn copy_crash_image(src: &Path, dst: &Path) {
    for entry in std::fs::read_dir(src).expect("read live data dir") {
        let entry = entry.expect("live data dir entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if entry.file_name().to_string_lossy() == ".batpak.lock" {
            continue;
        }
        std::fs::copy(&path, dst.join(entry.file_name())).expect("copy crash-image file");
    }
}

/// Kills `allocator_floor > 0` -> `< 0` in `collect_tail_entries`: with a
/// checkpoint on disk whose stored allocator is nonzero, the tail events
/// replayed from the (unsealed) active segment carry no durable sequences and
/// are SYNTHESIZED starting at that allocator floor — reproducing the exact
/// pre-crash receipt sequences. The mutant restarts the first synthesized
/// sequence at 0, shifting the recovered user events off their receipts.
#[test]
fn checkpoint_tail_replay_synthesizes_sequences_from_the_stored_allocator_floor() {
    let live = TempDir::new().expect("live dir");
    let coord = Coordinate::new(ENTITY, "scope:recovery").expect("coord");

    // Epoch 1: two events + clean close => checkpoint (nonzero allocator).
    let store = Store::open(recovery_config(live.path())).expect("open epoch 1");
    let g1 = store
        .append(&coord, KIND, &serde_json::json!({"n": 1}))
        .expect("append e1")
        .global_sequence;
    let g2 = store
        .append(&coord, KIND, &serde_json::json!({"n": 2}))
        .expect("append e2")
        .global_sequence;
    store.close().expect("close epoch 1");

    // Epoch 2: two more durable events, then a crash image taken WITHOUT a
    // close — the image's checkpoint still carries epoch 1's watermark and
    // allocator, so the epoch-2 events must replay from the segment tail.
    let store = Store::open(recovery_config(live.path())).expect("open epoch 2");
    let g3 = store
        .append(&coord, KIND, &serde_json::json!({"n": 3}))
        .expect("append e3")
        .global_sequence;
    let g4 = store
        .append(&coord, KIND, &serde_json::json!({"n": 4}))
        .expect("append e4")
        .global_sequence;
    let crash = TempDir::new().expect("crash dir");
    copy_crash_image(live.path(), crash.path());
    store.close().expect("close epoch 2 (live dir done)");

    assert!(
        crash.path().join("index.ckpt").exists(),
        "scenario shape: the crash image must carry epoch 1's checkpoint"
    );

    let recovered = Store::open(recovery_config(crash.path())).expect("open crash image");
    let report = recovered
        .diagnostics()
        .open_report
        .expect("recovery open produces diagnostics");
    assert_eq!(
        report.path,
        OpenIndexPath::Checkpoint,
        "scenario shape: recovery must take the checkpoint + tail-replay path"
    );
    assert!(
        report.tail_entries >= 2,
        "scenario shape: the post-checkpoint events replay from the tail, got {}",
        report.tail_entries
    );

    let recovered_sequences: Vec<u64> = recovered
        .by_entity(ENTITY)
        .iter()
        .map(batpak::store::index::IndexEntry::global_sequence)
        .collect();
    assert_eq!(
        recovered_sequences,
        vec![g1, g2, g3, g4],
        "synthesized tail sequences must continue at the checkpoint's stored allocator \
         floor, reproducing the pre-crash receipt sequences exactly"
    );
    recovered.close().expect("close recovered store");
}

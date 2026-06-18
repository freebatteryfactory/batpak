//! Fork isolation proofs (parent<->fork independence after the fork boundary).
//!
//! PROVES: INV-FORK-ISOLATION. After the fork boundary, the parent's later
//! writes never appear in a freshly reopened fork, a fork's own writes never
//! leak back to the parent, and the fork's idempotency authority is copied (not
//! shared) — a key recorded in the parent post-fork is new in the fork, while a
//! key present pre-fork is inherited and deduplicates.
//! CATCHES: shared active/idempotency inodes leaking writes across the fork
//! boundary; idempotency dedup state shared instead of copied at fork time.
//! SEEDED: tempfile-backed stores, tiny segment rotation, deterministic
//! coordinates and idempotency keys.

mod support;
use batpak::store::{ReadOnly, Store, StoreConfig};
use support::prelude::*;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn store_with_small_segments(dir: &TempDir) -> TestResult<Store> {
    Ok(Store::open(
        StoreConfig::new(dir.path())
            .with_segment_max_bytes(512)
            .with_sync_every_n_events(1)
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )?)
}

fn append_blob_events(store: &Store, entity: &str, count: usize) -> TestResult {
    let coord = Coordinate::new(entity, "scope:fork")?;
    let kind = EventKind::custom(0xF, 0x71);
    let blob = "x".repeat(300);
    for i in 0..count {
        store.append(&coord, kind, &serde_json::json!({"i": i, "blob": blob}))?;
    }
    Ok(())
}

#[test]
fn fork_isolates_parent_writes_on_fresh_reopen() -> TestResult {
    // Obligation 2a (strengthened): a fork must NOT leak the parent's post-fork
    // writes even when the fork is opened FRESH from disk (a brand-new handle),
    // ruling out a shared active/tail segment inode behind the in-memory view.
    let source_dir = TempDir::new()?;
    let store = store_with_small_segments(&source_dir)?;
    append_blob_events(&store, "entity:fork:reopen", 6)?;
    let n = store.stats().event_count;

    let fork_dir = TempDir::new()?;
    store.fork(fork_dir.path())?;

    // Parent appends MORE events after the fork boundary, then syncs to disk.
    append_blob_events(&store, "entity:fork:reopen", 4)?;
    store.sync()?;
    let parent_after = store.stats().event_count;
    assert_eq!(
        parent_after,
        n + 4,
        "parent must observe its own post-fork writes (sanity: writes happened)"
    );

    // Open the fork FRESH from disk — no reuse of any in-memory fork handle.
    // Read-only so the reopen does not itself append a lifecycle marker; the
    // count must then be EXACTLY the fork-boundary count, not the parent's
    // post-fork writes.
    let fresh_fork = Store::<ReadOnly>::open_read_only(
        StoreConfig::new(fork_dir.path())
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )?;
    assert_eq!(
        fresh_fork.stats().event_count,
        n,
        "freshly reopened fork must hold exactly the fork-boundary count, not the parent's post-fork writes"
    );

    store.close()?;
    Ok(())
}

#[test]
fn fork_write_does_not_leak_to_parent() -> TestResult {
    // Obligation 2b (was entirely missing): writes INTO a fork must not mutate
    // any inode the parent reads, i.e. they must not appear in the parent.
    let source_dir = TempDir::new()?;
    let store = store_with_small_segments(&source_dir)?;
    append_blob_events(&store, "entity:fork:parent-isolation", 5)?;
    let n = store.stats().event_count;

    let fork_dir = TempDir::new()?;
    store.fork(fork_dir.path())?;

    // Open the fork in its own scope so its directory lock releases afterward.
    // A writable reopen appends one SYSTEM_OPEN_COMPLETED lifecycle marker, so
    // capture the fork baseline AFTER open and assert the +3 delta against it.
    let (fork_baseline, fork_after) = {
        let fork = Store::open(
            StoreConfig::new(fork_dir.path())
                .with_segment_max_bytes(512)
                .with_sync_every_n_events(1)
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false),
        )?;
        let baseline = fork.stats().event_count;
        append_blob_events(&fork, "entity:fork:in-fork", 3)?;
        fork.sync()?;
        let after = fork.stats().event_count;
        fork.close()?;
        (baseline, after)
    };

    assert_eq!(
        fork_after,
        fork_baseline + 3,
        "fork must observe its own writes (sanity: writes landed in the fork)"
    );
    assert_eq!(
        store.stats().event_count,
        n,
        "fork writes must not leak back into the parent's event count"
    );

    store.close()?;
    Ok(())
}

#[test]
fn fork_idempotency_store_is_copied_not_shared() -> TestResult {
    use batpak::id::{EntityIdType, IdempotencyKey};

    // Obligation 2d (behavioral proof): the idempotency authority must be COPIED
    // at fork time, not shared. A key the parent records AFTER the fork must be
    // unknown to the fork (no dedup), while a key present BEFORE the fork must be
    // inherited (dedup).
    let source_dir = TempDir::new()?;
    let store = store_with_small_segments(&source_dir)?;
    let coord = Coordinate::new("entity:fork:idemp", "scope:fork")?;
    let kind = EventKind::custom(0xF, 0x74);
    let key_x = IdempotencyKey::for_operation("fork-idemp", &["x"]);
    let key_y = IdempotencyKey::for_operation("fork-idemp", &["y"]);

    // Seed key X into the parent BEFORE the fork.
    store.append_with_options(
        &coord,
        kind,
        &serde_json::json!({"k": "x"}),
        AppendOptions::default().with_idempotency(key_x),
    )?;

    let fork_dir = TempDir::new()?;
    store.fork(fork_dir.path())?;

    // Mutate the PARENT's idempotency authority AFTER the fork by recording key Y.
    store.append_with_options(
        &coord,
        kind,
        &serde_json::json!({"k": "y"}),
        AppendOptions::default().with_idempotency(key_y),
    )?;

    let fork = Store::open(
        StoreConfig::new(fork_dir.path())
            .with_segment_max_bytes(512)
            .with_sync_every_n_events(1)
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )?;
    let before = fork.stats().event_count;

    // (a) Key Y was recorded in the parent AFTER the fork. If idemp were SHARED,
    // the fork would dedup Y (no new event). Because it was copied at fork time,
    // the fork has never seen Y and MUST create a new event.
    let y_receipt = fork.append_with_options(
        &coord,
        kind,
        &serde_json::json!({"k": "y"}),
        AppendOptions::default().with_idempotency(key_y),
    )?;
    let after_y = fork.stats().event_count;
    assert_eq!(
        after_y,
        before + 1,
        "key Y (recorded in parent post-fork) must be NEW in the fork — idemp was copied, not shared"
    );
    assert_eq!(
        u128::from(y_receipt.event_id),
        key_y.as_u128(),
        "a keyed append must commit under the idempotency key as its event id"
    );

    // (b) Key X was present BEFORE the fork. The copy must carry it, so re-appending
    // X MUST deduplicate (no new event), and return the original keyed receipt.
    let x_receipt = fork.append_with_options(
        &coord,
        kind,
        &serde_json::json!({"k": "x"}),
        AppendOptions::default().with_idempotency(key_x),
    )?;
    let after_x = fork.stats().event_count;
    assert_eq!(
        after_x,
        after_y,
        "key X (present pre-fork) must DEDUPLICATE in the fork — idemp authority was copied at fork time"
    );
    assert_eq!(
        u128::from(x_receipt.event_id),
        key_x.as_u128(),
        "deduplicated keyed append must return the original keyed receipt"
    );

    fork.close()?;
    store.close()?;
    Ok(())
}

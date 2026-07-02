//! Round-3 mutation kills for the WP-C survivors (projection / delivery /
//! writer-pump / ancestry work package) — the plaintext-surface half.
//!
//! PROVES: (1) `NativeCache::delete_prefix` deletes EXACTLY the keys carrying
//! the prefix, retains everything else, and SKIPS non-matching shard
//! directories entirely (it never even reads them); (2) `CursorWatcherError`
//! exposes its wrapped `StoreError` through `std::error::Error::source`;
//! (3) an external-cache row whose watermark lies BEYOND the current replay
//! watermark is never trusted by the incremental-apply path — the projection
//! falls back to a full replay of what the store actually holds;
//! (4) `project_if_changed` returns the honest, exact nonzero generation the
//! state was materialized at; (5) `Subscription::pull_batch(max)` delivers AT
//! MOST `max` notifications, in exact sequence order; (6) a cursor worker
//! under `RestartPolicy::Bounded { max_restarts: N }` invokes its panicking
//! handler EXACTLY N+1 times and then self-stops.
//! CATCHES: projection/mod.rs:390 delete of `!` in `delete_prefix`'s
//! shard-skip guard (the mutant scans locked/foreign shards it must skip);
//! watch.rs:88 `CursorWatcherError::source -> None`; external_cache.rs:113
//! `>` -> `==` in `try_finish_incremental_cache_row` (a future-watermark cache
//! row would be served instead of replayed); outcome.rs:87
//! `returned_generation -> 0` (public-surface generation pin; the accessor
//! itself is pinned by a unit test beside it); subscription.rs:113 `+` -> `*`
//! in `pull_batch`'s fill loop (a `max`-sized pull would deliver max+1);
//! worker.rs:634 `+=` -> `*=` in `restart_budget_ok` (a restart counter stuck
//! at 0 never exhausts the Bounded budget).
//! SEEDED: deterministic — fixed payloads, no randomness; the cursor-worker
//! proof synchronizes on the handler's own channel (worker self-exit drops the
//! sender), never on wall-clock scheduling.

use batpak::store::projection::{CacheCapabilities, CacheMeta, NativeCache, ProjectionCache};
use batpak::store::{
    Canal, CanalBatch, CursorWatcherError, Freshness, RestartPolicy, Store, StoreConfig, StoreError,
};
use batpak_testkit::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tempfile::TempDir;

// ─── C3: NativeCache::delete_prefix — retention + shard-skip guard ──────────

fn forge_meta(watermark: u64) -> CacheMeta {
    CacheMeta {
        watermark,
        cached_at_us: 1_000_000,
        cached_at_mono_ns: None,
        process_boot_ns: None,
    }
}

/// Retention semantics: keys WITH the prefix are gone, keys WITHOUT survive,
/// and the removal count is exact.
#[test]
fn native_delete_prefix_removes_exactly_the_prefixed_keys_and_retains_the_rest() {
    let dir = TempDir::new().expect("temp dir");
    let cache = NativeCache::open(dir.path().join("cache")).expect("open native cache");

    let key_a1: &[u8] = &[0xAA, 0x01];
    let key_a2: &[u8] = &[0xAA, 0x02];
    let key_b: &[u8] = &[0xBB, 0x01];
    cache
        .put(key_a1, b"value-a1", forge_meta(1))
        .expect("put a1");
    cache
        .put(key_a2, b"value-a2", forge_meta(2))
        .expect("put a2");
    cache.put(key_b, b"value-b", forge_meta(3)).expect("put b");

    let removed = cache.delete_prefix(&[0xAA]).expect("delete prefix 0xAA");
    assert_eq!(
        removed, 2,
        "PROPERTY: delete_prefix(0xAA) must remove EXACTLY the two 0xAA-prefixed keys"
    );

    let a1 = cache.get(key_a1).expect("get a1 after delete");
    let a2 = cache.get(key_a2).expect("get a2 after delete");
    assert!(
        a1.is_none() && a2.is_none(),
        "PROPERTY: every key WITH the deleted prefix must be gone, got a1={a1:?} a2={a2:?}"
    );
    let survivor = cache
        .get(key_b)
        .expect("get b after delete")
        .expect("PROPERTY: a key WITHOUT the deleted prefix must survive delete_prefix");
    assert_eq!(
        survivor.0, b"value-b",
        "PROPERTY: the surviving key must retain its exact cached value bytes"
    );
}

/// Shard-skip guard: `delete_prefix` must never READ a shard directory that
/// cannot contain the prefix. With the guard's `!` deleted (mutant), the
/// non-matching `bb` shard is scanned; making it unreadable turns that illegal
/// scan into a `CacheFailed` error, while the real code skips it and succeeds.
/// (Under root the permission lock is ineffective and this collapses to the
/// retention check — CI and the mutation lanes run unprivileged.)
#[cfg(unix)]
#[test]
fn native_delete_prefix_skips_non_matching_shards_without_reading_them() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let cache_root = dir.path().join("cache");
    let cache = NativeCache::open(&cache_root).expect("open native cache");

    let key_a1: &[u8] = &[0xAA, 0x01];
    let key_a2: &[u8] = &[0xAA, 0x02];
    let key_b: &[u8] = &[0xBB, 0x01];
    cache
        .put(key_a1, b"value-a1", forge_meta(1))
        .expect("put a1");
    cache
        .put(key_a2, b"value-a2", forge_meta(2))
        .expect("put a2");
    cache.put(key_b, b"value-b", forge_meta(3)).expect("put b");

    // Lock the NON-matching shard. The real code never opens it (skip guard);
    // the `!`-deleted mutant scans every shard and trips PermissionDenied here.
    let locked_shard = cache_root.join("bb");
    let mut lock = std::fs::metadata(&locked_shard)
        .expect("bb shard metadata")
        .permissions();
    lock.set_mode(0o000);
    std::fs::set_permissions(&locked_shard, lock).expect("lock bb shard");

    let result = cache.delete_prefix(&[0xAA]);

    // Restore before asserting so TempDir cleanup works even on failure.
    let mut unlock = std::fs::metadata(&locked_shard)
        .expect("bb shard metadata after delete")
        .permissions();
    unlock.set_mode(0o755);
    std::fs::set_permissions(&locked_shard, unlock).expect("unlock bb shard");

    let removed = result.expect(
        "PROPERTY: delete_prefix must SKIP shards that cannot contain the prefix — an error \
         here means the shard-skip guard inverted and the locked non-matching shard was read",
    );
    assert_eq!(
        removed, 2,
        "PROPERTY: skipping the non-matching shard must still delete exactly the prefixed keys"
    );
    let survivor = cache
        .get(key_b)
        .expect("get b after delete")
        .expect("PROPERTY: the key in the skipped shard must survive untouched");
    assert_eq!(
        survivor.0, b"value-b",
        "PROPERTY: the skipped shard's value bytes must be untouched"
    );
}

// ─── C4: CursorWatcherError::source exposes the wrapped StoreError ──────────

#[test]
fn cursor_watcher_error_source_exposes_the_wrapped_store_error() {
    let inner = StoreError::WriterCrashed;
    let inner_msg = inner.to_string();

    let err = CursorWatcherError::from(inner);
    assert!(
        matches!(err, CursorWatcherError::Store(_)),
        "PROPERTY: From<StoreError> must produce the Store variant, got {err:?}"
    );

    let source = std::error::Error::source(&err).expect(
        "PROPERTY: CursorWatcherError::Store must expose its wrapped StoreError via source() — \
         None severs the error chain callers use to classify watcher failures",
    );
    assert_eq!(
        source.to_string(),
        inner_msg,
        "PROPERTY: source() must return the wrapped StoreError verbatim"
    );
    assert_eq!(
        err.to_string(),
        format!("cursor projection watcher failed: {inner_msg}"),
        "PROPERTY: Display must wrap the inner error message verbatim"
    );
}

// ─── C5: future-watermark cache rows must NOT feed incremental apply ────────

const FORGE_KIND: EventKind = EventKind::custom(0xC, 0x51);

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct IncrementalCounter {
    count: u32,
}

impl EventSourced for IncrementalCounter {
    type Input = JsonValueInput;
    const STATE_CONTRACT: ProjectionStateContract =
        ProjectionStateContract::single_entity("mutation-kill-wpc-incremental-counter");

    fn from_events(events: &[Event<serde_json::Value>]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }
        let mut state = Self::default();
        for event in events {
            state.apply_event(event);
        }
        Some(state)
    }

    fn apply_event(&mut self, _event: &Event<serde_json::Value>) {
        self.count += 1;
    }

    fn relevant_event_kinds() -> &'static [EventKind] {
        &[FORGE_KIND]
    }

    fn supports_incremental_apply() -> bool {
        true
    }

    fn state_extent(&self) -> StateExtent {
        StateExtent::single_entity()
    }
}

#[derive(Default)]
struct ForgeCacheInner {
    row: Option<(Vec<u8>, CacheMeta)>,
    watermark_bump: u64,
}

/// External-cache test double: stores the last `put` row and, on `get`,
/// replays it under ANY key with its watermark inflated by `watermark_bump` —
/// modelling a shared/external cache backend serving a row stamped BEYOND the
/// probing store's replay watermark.
struct ForgeCache(Arc<Mutex<ForgeCacheInner>>);

impl ForgeCache {
    fn lock(&self) -> std::sync::MutexGuard<'_, ForgeCacheInner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ProjectionCache for ForgeCache {
    fn capabilities(&self) -> CacheCapabilities {
        CacheCapabilities::none()
    }

    fn get(&self, _key: &[u8]) -> Result<Option<(Vec<u8>, CacheMeta)>, StoreError> {
        let inner = self.lock();
        Ok(inner.row.clone().map(|(bytes, mut meta)| {
            meta.watermark += inner.watermark_bump;
            (bytes, meta)
        }))
    }

    fn put(&self, _key: &[u8], value: &[u8], meta: CacheMeta) -> Result<(), StoreError> {
        self.lock().row = Some((value.to_vec(), meta));
        Ok(())
    }

    fn delete_prefix(&self, _prefix: &[u8]) -> Result<u64, StoreError> {
        let removed = u64::from(self.lock().row.take().is_some());
        Ok(removed)
    }

    fn sync(&self) -> Result<(), StoreError> {
        Ok(())
    }
}

#[test]
fn incremental_path_refuses_cache_rows_from_beyond_the_replay_watermark() {
    let dir = TempDir::new().expect("temp dir");
    let shared = Arc::new(Mutex::new(ForgeCacheInner::default()));
    let entity = "entity:forge-watermark";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");
    let config = StoreConfig::new(dir.path()).with_incremental_projection(true);

    // Store A: seed 2 events and cache the 2-event state at its real watermark.
    let store_a = Store::open_with_cache(config.clone(), Box::new(ForgeCache(Arc::clone(&shared))))
        .expect("open store A");
    for n in 0..2u32 {
        drop(
            store_a
                .append(&coord, FORGE_KIND, &serde_json::json!({ "n": n }))
                .expect("append seed event"),
        );
    }
    let first = store_a
        .project::<IncrementalCounter>(entity, &Freshness::Consistent)
        .expect("project on store A")
        .expect("2-event projection state");
    assert_eq!(
        first,
        IncrementalCounter { count: 2 },
        "PROPERTY: the seed projection folds both events"
    );
    store_a.close().expect("close store A");

    // Store B (fresh group-local state, same segments, same external cache):
    // append 2 MORE events, then forge the cached row's watermark far beyond
    // any watermark this store can replay to (>= 2 generations past it).
    let store_b = Store::open_with_cache(config, Box::new(ForgeCache(Arc::clone(&shared))))
        .expect("open store B");
    for n in 2..4u32 {
        drop(
            store_b
                .append(&coord, FORGE_KIND, &serde_json::json!({ "n": n }))
                .expect("append later event"),
        );
    }
    shared
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .watermark_bump = 1_000;

    // The row is stale-but-decodable and stamped IN THE FUTURE. The honest
    // guard (`meta.watermark > replay watermark`) must reject BOTH cache
    // finishes and fall back to a full replay of the 4 real events. The `==`
    // mutant instead feeds the 2-event row to incremental apply, finds no
    // events beyond the forged watermark, and returns the stale count of 2.
    let projected = store_b
        .project::<IncrementalCounter>(entity, &Freshness::Consistent)
        .expect("project on store B")
        .expect("4-event projection state");
    assert_eq!(
        projected,
        IncrementalCounter { count: 4 },
        "PROPERTY: a cache row whose watermark exceeds the replay watermark must be ignored in \
         favor of a full replay — serving it would time-travel state past the store's own log"
    );
    store_b.close().expect("close store B");
}

// ─── C6 (public pin): project_if_changed returns the exact honest generation ─

const GENERATION_KIND: EventKind = EventKind::custom(0xC, 0x52);

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct GenerationProbe {
    seen: u32,
}

impl EventSourced for GenerationProbe {
    type Input = JsonValueInput;
    const STATE_CONTRACT: ProjectionStateContract =
        ProjectionStateContract::single_entity("mutation-kill-wpc-generation-probe");

    fn from_events(events: &[Event<serde_json::Value>]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }
        Some(Self {
            seen: u32::try_from(events.len()).expect("test appends < 2^32 events"),
        })
    }

    fn apply_event(&mut self, _event: &Event<serde_json::Value>) {
        self.seen += 1;
    }

    fn relevant_event_kinds() -> &'static [EventKind] {
        &[GENERATION_KIND]
    }

    fn state_extent(&self) -> StateExtent {
        StateExtent::single_entity()
    }
}

#[test]
fn project_if_changed_returns_the_exact_nonzero_materialization_generation() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let entity = "entity:generation-probe";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");

    for n in 0..3u32 {
        drop(
            store
                .append(&coord, GENERATION_KIND, &serde_json::json!({ "n": n }))
                .expect("append probe event"),
        );
    }
    let live_generation = store
        .entity_generation(entity)
        .expect("entity generation after appends");
    assert_eq!(
        live_generation, 3,
        "PROPERTY: three appends advance the entity generation to exactly 3"
    );

    let (generation, state) = store
        .project_if_changed::<GenerationProbe>(entity, 0, &Freshness::Consistent)
        .expect("project_if_changed")
        .expect("generation advanced from 0, so a projection must be returned");
    assert_eq!(
        generation, 3,
        "PROPERTY: the returned generation is the exact nonzero generation the state was \
         materialized at — a zeroed generation would let a watcher silently consume appends"
    );
    assert_eq!(
        state,
        Some(GenerationProbe { seen: 3 }),
        "PROPERTY: the returned state folds exactly the three appended events"
    );

    let unchanged = store
        .project_if_changed::<GenerationProbe>(entity, generation, &Freshness::Consistent)
        .expect("project_if_changed at current generation");
    assert!(
        unchanged.is_none(),
        "PROPERTY: replaying the returned generation back reports no change, got {unchanged:?}"
    );
    store.close().expect("close store");
}

// ─── C7: Subscription::pull_batch caps the batch at exactly `max` ───────────

const PULL_KIND: EventKind = EventKind::custom(0xC, 0x53);

#[test]
fn subscription_pull_batch_delivers_at_most_max_in_exact_sequence_order() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let entity = "entity:pull-batch-cap";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");

    let mut subscription = store.subscribe_lossy(&Region::entity(entity));

    // Appends complete only AFTER the writer broadcast, so all five
    // notifications are deterministically queued before the first pull.
    let mut sequences = Vec::new();
    for n in 0..5u32 {
        let receipt = store
            .append(&coord, PULL_KIND, &serde_json::json!({ "n": n }))
            .expect("append notification event");
        sequences.push(receipt.global_sequence);
    }

    let first =
        Canal::pull_batch(&mut subscription, 3, Duration::from_secs(5)).expect("first pull_batch");
    match first {
        CanalBatch::Many(items) => {
            let got: Vec<u64> = items.iter().map(|n| n.sequence).collect();
            assert_eq!(
                got,
                sequences[..3],
                "PROPERTY: pull_batch(max=3) with 5 queued notifications delivers EXACTLY the \
                 first 3, in commit order — 4 delivered means the fill loop's `+ 1` headroom \
                 degenerated and the batch overshot max"
            );
        }
        other @ (CanalBatch::Empty | CanalBatch::One(_)) => {
            assert!(
                matches!(other, CanalBatch::Many(_)),
                "PROPERTY: 5 queued notifications must fill a max=3 batch, got {other:?}"
            );
        }
    }

    let second =
        Canal::pull_batch(&mut subscription, 3, Duration::from_secs(5)).expect("second pull_batch");
    match second {
        CanalBatch::Many(items) => {
            let got: Vec<u64> = items.iter().map(|n| n.sequence).collect();
            assert_eq!(
                got,
                sequences[3..],
                "PROPERTY: the second pull resumes at the exact 4th notification (nothing \
                 skipped, nothing re-delivered)"
            );
        }
        other @ (CanalBatch::Empty | CanalBatch::One(_)) => {
            assert!(
                matches!(other, CanalBatch::Many(_)),
                "PROPERTY: the remaining 2 notifications must arrive as one Many batch, got \
                 {other:?}"
            );
        }
    }

    let drained = Canal::pull_batch(&mut subscription, 3, Duration::from_millis(0))
        .expect("drained pull_batch");
    assert!(
        matches!(drained, CanalBatch::Empty),
        "PROPERTY: after 5 delivered notifications the channel is empty, got {drained:?}"
    );
    store.close().expect("close store");
}

// ─── C8: Bounded restart budget exhausts after exactly max_restarts + 1 ─────

const RESTART_KIND: EventKind = EventKind::custom(0xC, 0x54);

#[test]
fn bounded_restart_budget_exhausts_after_exactly_max_restarts_plus_one_invocations() {
    let dir = TempDir::new().expect("temp dir");
    let store = Arc::new(
        Store::open(
            StoreConfig::new(dir.path())
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false)
                .with_sync_every_n_events(1),
        )
        .expect("open store"),
    );
    let entity = "entity:restart-budget-arith";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");
    drop(
        store
            .append(&coord, RESTART_KIND, &serde_json::json!({ "seed": true }))
            .expect("append seed event"),
    );

    let mut worker_config = CursorWorkerConfig::default();
    worker_config.batch_size = 1;
    worker_config.idle_sleep = Duration::from_millis(1);
    // Huge window: the rolling-window reset can never fire, so the ONLY way
    // the worker stops is the restart counter reaching max_restarts.
    worker_config.restart = RestartPolicy::Bounded {
        max_restarts: 2,
        within_ms: 3_600_000,
    };

    let invocations = Arc::new(AtomicUsize::new(0));
    let (invocation_tx, invocation_rx) = std::sync::mpsc::channel::<usize>();
    let worker = store
        .cursor_worker(&Region::entity(entity), worker_config, {
            let invocations = Arc::clone(&invocations);
            move |_batch, _store, _witness| {
                let call = invocations.fetch_add(1, Ordering::SeqCst) + 1;
                invocation_tx
                    .send(call)
                    .expect("report handler invocation to the test");
                // Panic on EVERY invocation: with Bounded{max_restarts: 2} the
                // worker must stop after the 3rd panic (initial + 2 restarts).
                // black_box keeps the deliberate panic clippy-clean.
                assert!(
                    std::hint::black_box(false),
                    "intentional panic: exhaust the bounded restart budget"
                );
                CursorWorkerAction::Continue
            }
        })
        .expect("spawn cursor worker");

    // Initial attempt + exactly two budgeted restarts.
    for expected in 1..=3usize {
        let call = invocation_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("handler invocation within the bounded window");
        assert_eq!(
            call, expected,
            "PROPERTY: handler invocations arrive in order (initial, restart 1, restart 2)"
        );
    }

    // After the 3rd panic the budget (2 restarts) is exhausted: the worker
    // must self-stop, dropping the handler — and with it our sender. A 4th
    // invocation arriving instead means the restart counter never advanced
    // (`+=` degenerated to `*=` on a counter starting at 0).
    let after_exhaustion = invocation_rx.recv_timeout(Duration::from_secs(10));
    assert!(
        matches!(
            after_exhaustion,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        ),
        "PROPERTY: Bounded{{max_restarts: 2}} allows EXACTLY 3 handler invocations, then the \
         worker exits and drops the handler (sender disconnects); got {after_exhaustion:?}"
    );

    worker
        .stop_and_join()
        .expect("stop and join the exhausted worker");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        3,
        "PROPERTY: the joined worker ran the handler exactly max_restarts + 1 times"
    );

    let store = Arc::try_unwrap(store)
        .map_err(|_| "store still shared")
        .expect("PROPERTY: the exhausted worker released its store handle");
    store.close().expect("close store");
}

//! PROVES: the `CursorWorkerHandle` lifecycle surface — its boxed
//! [`CanalHandle::stop_and_join`], its `Drop`, and the `RestartPolicy::Once`
//! restart-budget accounting — carries real behavior, not stubbed constants.
//! CATCHES: `stop_and_join -> Ok(())` swallowing a stashed terminal error, a
//! no-op `Drop` that leaks the worker thread, and the `restarts += 1 -> *= 1`
//! mutation that turns a one-shot restart budget into an unbounded one.
//! SEEDED: deterministic — direct handle construction and a store-backed worker
//! driven purely by the store's own visibility/join primitives (no timing).

use super::{CanalHandle, CursorWorkerAction, CursorWorkerConfig, CursorWorkerHandle};
use crate::coordinate::{Coordinate, Region};
use crate::event::EventKind;
use crate::store::platform::spawn::{Spawn, ThreadSpawn};
use crate::store::{RestartPolicy, Store, StoreConfig, StoreError};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Build a `CursorWorkerHandle` whose backing job has already finished, with the
/// supplied error pre-seeded in the worker error slot. The join field references
/// a real (finished) `ThreadSpawn` job so `finish_join` exercises a genuine,
/// non-panicking `JobHandle::join`.
fn handle_with_seeded_error(seeded: Option<StoreError>) -> (CursorWorkerHandle, Arc<AtomicBool>) {
    let stop = Arc::new(AtomicBool::new(false));
    let error_slot = Arc::new(Mutex::new(seeded));
    let job = ThreadSpawn
        .spawn(
            "cursor-worker-handle-proof".to_string(),
            None,
            Box::new(|| {}),
        )
        .expect("spawn a trivial finished job");
    let handle = CursorWorkerHandle {
        stop: Arc::clone(&stop),
        join: Some(job),
        error_slot,
    };
    (handle, stop)
}

#[test]
fn boxed_canal_handle_stop_and_join_surfaces_the_stashed_error() {
    // Kills worker.rs:266 `<CanalHandle for CursorWorkerHandle>::stop_and_join
    // -> Ok(())`. A worker that stashed a terminal StoreError must surface it
    // through the boxed CanalHandle path, not a blanket Ok(()).
    let (handle, _stop) = handle_with_seeded_error(Some(StoreError::WriterCrashed));
    let boxed: Box<dyn CanalHandle> = Box::new(handle);
    let result = boxed.stop_and_join();
    assert!(
        matches!(result, Err(StoreError::WriterCrashed)),
        "PROPERTY: the boxed CanalHandle::stop_and_join must return the stashed \
         terminal error; the Ok(()) mutant swallows it, got {result:?}"
    );
}

#[test]
fn boxed_canal_handle_stop_and_join_is_ok_without_a_stashed_error() {
    // Guards the kill above from vacuity: a clean worker joins Ok(()), so the
    // Err assertion is pinned to the seeded error, not to the path always failing.
    let (handle, _stop) = handle_with_seeded_error(None);
    let boxed: Box<dyn CanalHandle> = Box::new(handle);
    boxed
        .stop_and_join()
        .expect("a worker with no stashed error joins cleanly");
}

#[test]
fn drop_signals_stop_to_the_background_worker() {
    // Kills worker.rs:272 `<Drop for CursorWorkerHandle>::drop -> ()`. Dropping
    // the handle must raise the stop flag so the background worker winds down;
    // the no-op mutant leaks the worker thread.
    let (handle, stop) = handle_with_seeded_error(None);
    assert!(
        !stop.load(Ordering::Acquire),
        "sanity: the stop flag starts unset"
    );
    drop(handle);
    assert!(
        stop.load(Ordering::Acquire),
        "PROPERTY: Drop must raise the stop flag; the `()` mutant leaks the worker"
    );
}

#[test]
fn once_restart_policy_consumes_its_single_budget_slot() {
    // Kills worker.rs:612 `state.restarts += 1` -> `*= 1` in the
    // `RestartPolicy::Once` arm of `restart_budget_ok`. With `+= 1` a fresh
    // counter (0) becomes 1 and the SECOND panic exhausts the budget: the
    // handler runs exactly twice (initial + one restart) and the worker stops.
    // With `*= 1` the counter stays 0 forever, so `Once` never exhausts and the
    // handler would run a THIRD time (where it stops itself). We pin the exact
    // invocation count of 2.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let store = Arc::new(
        Store::open(
            StoreConfig::new(dir.path())
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false)
                .with_sync_every_n_events(1),
        )
        .expect("open store"),
    );
    let coord = Coordinate::new("entity:once-budget", "scope:test").expect("coord");
    let kind = EventKind::custom(0xF, 7);
    let _receipt = store
        .append(&coord, kind, &serde_json::json!({"n": 0}))
        .expect("append seed event");

    let invocations = Arc::new(AtomicUsize::new(0));
    let worker_config = CursorWorkerConfig {
        batch_size: 1,
        idle_sleep: Duration::from_millis(1),
        restart: RestartPolicy::Once,
        ..CursorWorkerConfig::default()
    };

    let worker = store
        .cursor_worker(&Region::entity("entity:once-budget"), worker_config, {
            let invocations = Arc::clone(&invocations);
            move |_batch, _store, _witness| {
                let count = invocations.fetch_add(1, Ordering::SeqCst) + 1;
                if count <= 2 {
                    // Deliberate handler panic to drive the restart path;
                    // black_box keeps the assert clippy-clean.
                    assert!(
                        std::hint::black_box(false),
                        "intentional cursor worker panic (invocation {count})"
                    );
                }
                CursorWorkerAction::Stop
            }
        })
        .expect("spawn worker");

    // The worker stops itself once the Once budget is exhausted; join is a
    // passive wait for that self-driven exit — no timing, no stop() race.
    worker
        .join()
        .expect("Once-budget exhaustion is a clean stop, not an error");

    assert_eq!(
        invocations.load(Ordering::SeqCst),
        2,
        "PROPERTY: RestartPolicy::Once permits exactly one restart, so the handler \
         runs twice before the budget is exhausted; the `*= 1` mutant never consumes \
         the slot and would run it a third time"
    );

    let store = Arc::try_unwrap(store)
        .map_err(|_| "shared")
        .expect("cursor worker released the last Arc");
    store.close().expect("close store");
}

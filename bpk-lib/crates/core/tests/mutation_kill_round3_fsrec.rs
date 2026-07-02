//! Round-3 mutation kill for the fs/recovery cluster's cursor-worker
//! survivor — the rolling-window division in the bounded restart budget.
//!
//! PROVES: the `RestartPolicy::Bounded` rolling window measures elapsed time
//! as WHOLE monotonic milliseconds (`elapsed_ns / 1_000_000`): a scripted
//! store clock advanced by exactly 1_500_000 ns after the first consumed
//! restart yields `elapsed_ms == 1`, which does NOT exceed `within_ms: 1`,
//! so the restart counter keeps accumulating and the budget exhausts after
//! exactly `max_restarts + 1` handler invocations.
//! CATCHES: worker.rs:626 `/` -> `%` in `CursorWorkerLoop::restart_budget_ok`
//! (the sub-millisecond REMAINDER `1_500_000 % 1_000_000 = 500_000` dwarfs
//! `within_ms`, spuriously resetting the rolling window after the first
//! consumed restart and granting the worker a 4th handler invocation).
//! SEEDED: deterministic — the store clock's monotonic reading is a
//! test-owned atomic that only the handler advances (same thread, strictly
//! before its panic reaches `handle_panic`), and the proof synchronizes on
//! the handler's own channel (worker self-exit drops the sender); no
//! wall-clock scheduling anywhere.

use batpak::store::{Clock, RestartPolicy, Store, StoreConfig, SystemClock};
use batpak_testkit::prelude::*;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

const WINDOW_KIND: EventKind = EventKind::custom(0xC, 0x55);

/// Wall readings delegate to the real [`SystemClock`]; the monotonic reading
/// is an atomic that only this proof advances, so `restart_budget_ok`
/// observes exactly the elapsed nanoseconds the test scripts.
struct ScriptedMonoClock {
    wall: SystemClock,
    mono_ns: AtomicI64,
}

impl Clock for ScriptedMonoClock {
    fn now_us(&self) -> i64 {
        self.wall.now_us()
    }

    fn now_wall_ns(&self) -> i64 {
        self.wall.now_wall_ns()
    }

    fn now_mono_ns(&self) -> i64 {
        self.mono_ns.load(Ordering::SeqCst)
    }

    fn process_boot_ns(&self) -> u64 {
        self.wall.process_boot_ns()
    }
}

#[test]
fn bounded_restart_window_compares_whole_elapsed_ms_not_submilli_remainder() {
    let dir = TempDir::new().expect("temp dir");
    let clock = Arc::new(ScriptedMonoClock {
        wall: SystemClock::new(),
        mono_ns: AtomicI64::new(0),
    });
    let store = Arc::new(
        Store::open(
            StoreConfig::new(dir.path())
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false)
                .with_sync_every_n_events(1)
                .with_clock(Some(Arc::clone(&clock) as Arc<dyn Clock>)),
        )
        .expect("open store"),
    );
    let entity = "entity:restart-window-division";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");
    drop(
        store
            .append(&coord, WINDOW_KIND, &serde_json::json!({ "seed": true }))
            .expect("append seed event"),
    );

    let mut worker_config = CursorWorkerConfig::default();
    worker_config.batch_size = 1;
    worker_config.idle_sleep = Duration::from_millis(1);
    // The worker captures window_start_ns while the scripted clock still reads
    // 0. A single 1_500_000 ns advance then makes the true elapsed quotient
    // exactly 1 ms — equal to within_ms, so the reset path must stay cold.
    worker_config.restart = RestartPolicy::Bounded {
        max_restarts: 2,
        within_ms: 1,
    };

    let invocations = Arc::new(AtomicUsize::new(0));
    let (invocation_tx, invocation_rx) = std::sync::mpsc::channel::<usize>();
    let worker = store
        .cursor_worker(&Region::entity(entity), worker_config, {
            let invocations = Arc::clone(&invocations);
            let clock = Arc::clone(&clock);
            move |_batch, _store, _witness| {
                let call = invocations.fetch_add(1, Ordering::SeqCst) + 1;
                if call == 2 {
                    // One restart is already consumed (restarts == 1). Advance
                    // the scripted monotonic clock by exactly 1.5 ms: quotient
                    // 1 == within_ms keeps the real window intact, while the
                    // `%` mutant reads the 500_000 remainder as "elapsed ms",
                    // resets the window, and refunds the consumed restart.
                    clock.mono_ns.store(1_500_000, Ordering::SeqCst);
                }
                invocation_tx
                    .send(call)
                    .expect("report handler invocation to the test");
                // Panic on EVERY invocation: with Bounded{max_restarts: 2} the
                // worker must stop after the 3rd panic (initial + 2 restarts).
                // black_box keeps the deliberate panic clippy-clean.
                assert!(
                    std::hint::black_box(false),
                    "intentional panic: probe the rolling-window division"
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

    // The 3rd panic exhausts Bounded{max_restarts: 2}: the worker self-stops,
    // dropping the handler — and with it our sender. A 4th invocation arriving
    // instead means the 1.5 ms advance was misread as a 500_000 "ms" remainder
    // (`/` degenerated to `%`), spuriously resetting the restart window.
    let after_exhaustion = invocation_rx.recv_timeout(Duration::from_secs(10));
    assert!(
        matches!(
            after_exhaustion,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        ),
        "PROPERTY: whole-millisecond window math allows EXACTLY 3 handler invocations, then \
         the worker exits and drops the handler (sender disconnects); got {after_exhaustion:?}"
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

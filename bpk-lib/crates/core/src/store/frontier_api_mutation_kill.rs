//! PROVES: the `dangerous-test-hooks` projection/watermark surface on `Store`
//! actually mutates registry and condvar state — register/register_for install a
//! progress entry that holds the applied frontier back, unregister removes it and
//! lets the frontier recompute upward, and notify-waiters wakes a blocked
//! frontier waiter.
//! CATCHES: each hook stubbed to `()` — a no-op register that lets an unheld
//! frontier race ahead, a no-op unregister that pins the frontier to a departed
//! projection, and a no-op notify that never wakes a waiter.
//! SEEDED: deterministic — real appends drive visible; the notify-waiters proof
//! uses the store's own condvar wait primitive, not timing.

use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::store::stats::HlcPoint;
use crate::store::{Store, StoreConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

/// A projection witness type used only for its `TypeId`-derived registry id.
struct HoldProbe;

fn open_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    (dir, store)
}

fn append_and_snapshot_visible(store: &Store, entity: &str, n: u64) -> HlcPoint {
    let coord = Coordinate::new(entity, "scope:frontier-hooks").expect("coord");
    let _receipt = store
        .append(
            &coord,
            EventKind::custom(0xF, 0x21),
            &serde_json::json!({ "n": n }),
        )
        .expect("append advances the visible frontier");
    store.frontier().visible_hlc
}

#[test]
fn register_projection_holds_the_applied_frontier_at_the_registration_baseline() {
    // Kills frontier_api.rs:18 `dangerous_register_projection -> ()`. A projection
    // registered-but-never-notified pins the applied frontier at the baseline it
    // was registered at, because applied is the MIN across registered projections.
    // If register is a no-op the held projection never enters the registry, so a
    // second (notified) projection drags applied all the way up to `visible`.
    let (_dir, store) = open_store();
    let visible = append_and_snapshot_visible(&store, "entity:reg-hold", 0);
    let baseline = store.frontier().applied_hlc;
    assert!(
        baseline.global_sequence < visible.global_sequence,
        "sanity: the applied baseline {baseline:?} sits below visible {visible:?}, \
         so a held projection has somewhere to hold"
    );

    store.dangerous_register_projection("frontier:hold:never-notified");
    store.dangerous_notify_projection_applied("frontier:mover", visible);

    assert_eq!(
        store.frontier().applied_hlc,
        baseline,
        "PROPERTY: a registered, unnotified projection holds applied at its \
         registration baseline; the `()` register mutant lets the mover advance \
         applied to {visible:?}"
    );
}

#[test]
fn register_projection_for_type_holds_the_applied_frontier_at_the_baseline() {
    // Kills frontier_api.rs:24 `dangerous_register_projection_for -> ()`. Same
    // hold property, but the held projection is registered under the type-derived
    // id, exercising the `_for` variant specifically.
    let (_dir, store) = open_store();
    let visible = append_and_snapshot_visible(&store, "entity:reg-for-hold", 0);
    let baseline = store.frontier().applied_hlc;
    assert!(
        baseline.global_sequence < visible.global_sequence,
        "sanity: applied baseline {baseline:?} below visible {visible:?}"
    );

    store.dangerous_register_projection_for::<HoldProbe>("entity:reg-for-hold");
    store.dangerous_notify_projection_applied("frontier:mover", visible);

    assert_eq!(
        store.frontier().applied_hlc,
        baseline,
        "PROPERTY: register_projection_for must install a held progress entry; the \
         `()` mutant leaves the type-id projection unregistered so applied races up \
         to {visible:?}"
    );
}

#[test]
fn unregister_projection_recomputes_the_frontier_upward_from_the_survivor() {
    // Kills frontier_api.rs:38 `dangerous_unregister_projection -> ()`. With a
    // slow and a fast projection registered, applied is the min (= slow).
    // Unregistering the slow one must recompute applied up to the survivor (fast);
    // the `()` mutant leaves the departed slow projection pinning applied down.
    let (_dir, store) = open_store();
    let slow = append_and_snapshot_visible(&store, "entity:unreg", 0);
    let mut fast = slow;
    for n in 1..4 {
        fast = append_and_snapshot_visible(&store, "entity:unreg", n);
    }
    assert!(
        slow.global_sequence < fast.global_sequence,
        "sanity: slow {slow:?} strictly precedes fast {fast:?}"
    );

    store.dangerous_register_projection("frontier:unreg:slow");
    store.dangerous_register_projection("frontier:unreg:fast");
    store.dangerous_notify_projection_applied("frontier:unreg:slow", slow);
    store.dangerous_notify_projection_applied("frontier:unreg:fast", fast);
    assert_eq!(
        store.frontier().applied_hlc,
        slow,
        "sanity: applied is the min across both projections before unregister"
    );

    store.dangerous_unregister_projection("frontier:unreg:slow");
    assert_eq!(
        store.frontier().applied_hlc,
        fast,
        "PROPERTY: unregister must drop the slow projection and recompute applied up \
         to the survivor {fast:?}; the `()` mutant pins applied at {slow:?}"
    );
}

#[test]
fn notify_watermark_waiters_wakes_a_blocked_frontier_waiter() {
    // Kills frontier_api.rs:44 `dangerous_notify_watermark_waiters -> ()`. A thread
    // parked on the watermark condvar must be woken by the hook. The no-op mutant
    // never wakes it, so the waiter blocks until ITS own timeout (reported as
    // timed_out == true). Driven entirely by the store's own condvar primitive.
    let (_dir, store) = open_store();
    let store = Arc::new(store);
    let ran = Arc::new(AtomicBool::new(false));

    let waiter_store = Arc::clone(&store);
    let waiter_ran = Arc::clone(&ran);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let waiter = std::thread::Builder::new()
        .name("frontier-notify-waiters-proof".to_string())
        .spawn(move || {
            waiter_ran.store(true, Ordering::Release);
            ready_tx.send(()).expect("signal waiter readiness");
            let timed_out = waiter_store
                .watermark_handle
                .dangerous_wait_for_notification(Duration::from_secs(2));
            done_tx.send(timed_out).expect("signal waiter outcome");
        })
        .expect("spawn condvar waiter");

    ready_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter reached the condvar wait");
    assert!(ran.load(Ordering::Acquire), "sanity: waiter thread ran");

    let deadline = Instant::now() + Duration::from_secs(1);
    let timed_out = loop {
        store.dangerous_notify_watermark_waiters();
        match done_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(timed_out) => break timed_out,
            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                break true
            }
        }
    };
    waiter.join().expect("condvar waiter joins");

    assert!(
        !timed_out,
        "PROPERTY: dangerous_notify_watermark_waiters must wake a parked frontier \
         waiter before its timeout; the `()` mutant never wakes it"
    );
}

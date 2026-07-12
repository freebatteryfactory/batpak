//! #182: a store driven by a FULLY injected `Clock` must never touch the
//! ambient monotonic anchor (`SystemTime::now` / `Instant::now`) — on wasm32
//! under `workerd` that path traps during `Store::open`, before the injected
//! clock can own runtime timekeeping.
//!
//! PROVES: issue #182 acceptance at the native runtime level — the test
//! reaches a real `Store::open` + append + close (not merely
//! `cargo check --target wasm32-unknown-unknown`), and the oracle is ambient
//! ACCESS: while the `ForbidAmbientAnchorGuard` tripwire is installed, ANY
//! `MonotonicAnchor::get()` call fails the run. Deliberately NOT an
//! "is-the-OnceLock-initialized" probe — that global state depends on what ran
//! earlier in the process, so this binary holds exactly ONE test and the guard
//! spans the store's whole lifecycle (the tripwire is process-global and
//! observed from every thread, including the writer thread).
//!
//! CATCHES: any reintroduction of an eager ambient-anchor grab on the
//! injected-clock open path — the historical offenders being
//! `SystemClock::new()` / `FnClock::new()` capturing the anchor at
//! construction, and `WatermarkState::new`'s `..Self::default()` spread
//! constructing (and discarding) a `SystemClock` on every writer spawn.
//! SEEDED: a deterministic `IsolatedClock` (atomic counters; wall time from a
//! fixed epoch advancing 1ms per query, monotonic time advancing 1ms per
//! query, constant `process_boot_ns`) driving a real open → keyed append →
//! sync → close lifecycle in a fresh temp directory.
#![cfg(feature = "dangerous-test-hooks")]

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::store::{Clock, ForbidAmbientAnchorGuard, Store, StoreConfig};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

/// Deterministic injected clock: every reading derives from process-free
/// atomics. Wall time starts at a fixed epoch and advances 1ms per query (the
/// store requires positive, advancing microsecond timestamps); monotonic time
/// advances 1ms per query so watermark/gate waits always make progress;
/// `process_boot_ns` is a fixed marker.
struct IsolatedClock {
    wall_us: AtomicI64,
    mono_ns: AtomicI64,
}

impl IsolatedClock {
    fn new() -> Self {
        Self {
            // 2023-11-14T22:13:20Z — an arbitrary fixed, positive epoch.
            wall_us: AtomicI64::new(1_700_000_000_000_000),
            mono_ns: AtomicI64::new(1_000_000),
        }
    }
}

impl Clock for IsolatedClock {
    fn now_us(&self) -> i64 {
        self.wall_us.fetch_add(1_000, Ordering::SeqCst)
    }

    fn now_wall_ns(&self) -> i64 {
        self.now_us().saturating_mul(1_000)
    }

    fn now_mono_ns(&self) -> i64 {
        self.mono_ns.fetch_add(1_000_000, Ordering::SeqCst)
    }

    fn process_boot_ns(&self) -> u64 {
        0xB47_BA7
    }
}

#[test]
fn full_store_lifecycle_with_injected_clock_never_touches_ambient_anchor() {
    let dir = TempDir::new().expect("temp dir");
    let _guard = ForbidAmbientAnchorGuard::install();

    let store = Store::open(
        StoreConfig::new(dir.path()).with_clock(Some(Arc::new(IsolatedClock::new()))),
    )
    .expect("PROPERTY (#182): Store::open with a fully injected Clock must not touch the ambient anchor");

    let coord = Coordinate::new("entity:clock-isolation", "scope:test").expect("valid coord");
    let receipt = store
        .append(
            &coord,
            EventKind::custom(0x1, 1),
            &serde_json::json!({"isolated": true}),
        )
        .expect("append under the injected clock");
    assert!(
        receipt.global_sequence > 0,
        "the keyed lifecycle actually committed under the injected clock"
    );
    store.sync().expect("sync under the injected clock");
    store
        .close()
        .expect("PROPERTY (#182): close must not touch the ambient anchor either");
}

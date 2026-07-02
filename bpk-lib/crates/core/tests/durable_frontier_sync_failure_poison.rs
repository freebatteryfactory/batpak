#![cfg(feature = "dangerous-test-hooks")]
//! PROVES: a failed active-segment fsync poisons the writer FAIL-CLOSED.
//! Live-side companion of INV-FRONTIER-DURABLE-COVERS-RECOVERED (the durable
//! frontier must honestly classify what is on disk): (a) the append that
//! triggers the failing cadence sync has already received its receipt —
//! `execute_command` sends the receipt BEFORE `drive_command` runs the
//! periodic sync, so that receipt is committed-not-durable by design; (b) the
//! NEXT command surfaces exactly `StoreError::WriterCrashed`; (c) the durable
//! frontier never advances past the last successful sync, even when a LATER
//! sync attempt would succeed (fsyncgate: after an fsync error the kernel
//! clears the dirty page bits, so a later Ok fsync proves nothing about the
//! lost pages).
//!
//! CATCHES: reverting the poison to the pre-fix log-and-continue (the P1
//! fail-open observed live by the dm-flakey chaos run: an append after an
//! all-error flip still returned a receipt, with the sync failure only in the
//! log) — under that mutant the post-failure append succeeds and a later
//! lying Ok-sync advances the durable frontier over silently-lost data.
//!
//! SEEDED: deterministic — threaded writer processes commands in order, so
//! the armed injector fires on the cadence sync of the triggering append and
//! the poison is set before the next command is pulled. No timing waits.

use batpak::prelude::{Coordinate, EventKind};
use batpak::store::{FaultInjector, InjectionPoint, Store, StoreConfig, StoreError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Injects an fsync error at the writer's active-segment durability choke
/// point (`InjectionPoint::ActiveSegmentSync`) while armed. Disarming models
/// fsyncgate's lying later-Ok: the underlying fsync would now "succeed", and
/// the poisoned writer must still refuse to advance the durable frontier.
struct ArmedSyncFault {
    armed: AtomicBool,
}

impl FaultInjector for ArmedSyncFault {
    fn check(&self, point: InjectionPoint) -> Option<StoreError> {
        if !matches!(point, InjectionPoint::ActiveSegmentSync { .. }) {
            return None;
        }
        if self.armed.load(Ordering::Acquire) {
            Some(StoreError::FaultInjected(
                "simulated fsync EIO on active-segment durability sync".to_string(),
            ))
        } else {
            None
        }
    }
}

fn coord(entity: &str) -> Coordinate {
    Coordinate::new(entity, "scope:sync-poison").expect("valid test coordinate")
}

fn kind() -> EventKind {
    EventKind::custom(0xF, 0x93)
}

#[test]
fn failed_cadence_sync_poisons_writer_and_freezes_durable_frontier() {
    let dir = tempfile::tempdir().expect("tempdir");
    let injector = Arc::new(ArmedSyncFault {
        armed: AtomicBool::new(false),
    });
    let store = Store::open(
        StoreConfig::new(dir.path())
            .with_sync_every_n_events(1)
            .with_fault_injector(Some(Arc::clone(&injector) as Arc<dyn FaultInjector>)),
    )
    .expect("open store with disarmed sync-fault injector");

    // Baseline: one append, then an explicit sync BARRIER. The barrier reply
    // is sent only after the writer finished the append's cadence sync, so the
    // frontier read below is deterministic (no race with the writer thread).
    let _baseline = store
        .append(
            &coord("entity:sync-poison:baseline"),
            kind(),
            &serde_json::json!({ "value": 1 }),
        )
        .expect("baseline append");
    store.sync().expect("baseline explicit sync barrier");
    let pre_failure_durable = store.frontier().durable_hlc;

    injector.armed.store(true, Ordering::Release);

    // (a) The append that TRIGGERS the failing cadence sync still gets its
    // receipt: the writer sends it from `execute_command` before
    // `drive_command` runs the periodic sync. Committed-not-durable by design.
    let receipt = store
        .append(
            &coord("entity:sync-poison:trigger"),
            kind(),
            &serde_json::json!({ "value": 2 }),
        )
        .expect("PROPERTY: the receipt precedes the cadence sync, so the triggering append acks");
    assert!(
        pre_failure_durable.global_sequence <= receipt.global_sequence,
        "sanity: the triggering event lies past the pre-failure durable point"
    );

    // (b) The NEXT command surfaces the poison error EXACTLY.
    let err = store
        .append(
            &coord("entity:sync-poison:after"),
            kind(),
            &serde_json::json!({ "value": 3 }),
        )
        .expect_err("PROPERTY: the writer must stop accepting work after a failed sync");
    assert!(
        matches!(err, StoreError::WriterCrashed),
        "PROPERTY: the poisoned writer must reject the next append with exactly \
         StoreError::WriterCrashed, got {err:?}"
    );

    // (c1) The durable frontier froze at the pre-failure point; the triggering
    // event is visible (published) but NOT durable — committed-not-durable.
    let frontier = store.frontier();
    assert_eq!(
        frontier.durable_hlc, pre_failure_durable,
        "PROPERTY: the durable frontier must not advance past the last successful sync"
    );
    assert!(
        frontier.durable_hlc.global_sequence <= receipt.global_sequence,
        "PROPERTY: the durable frontier must never cover the receipt whose sync failed"
    );
    assert!(
        frontier.visible_hlc.global_sequence > frontier.durable_hlc.global_sequence,
        "PROPERTY: the triggering append is committed-not-durable (visible ran ahead)"
    );

    // (c2) fsyncgate: disarm the injector so a later fsync WOULD return Ok —
    // the poisoned writer must reject the sync outright and the durable
    // frontier must still not move.
    injector.armed.store(false, Ordering::Release);
    let sync_err = store
        .sync()
        .expect_err("PROPERTY: a poisoned writer must reject explicit sync attempts");
    assert!(
        matches!(sync_err, StoreError::WriterCrashed),
        "PROPERTY: post-poison sync must surface exactly StoreError::WriterCrashed, \
         got {sync_err:?}"
    );
    assert_eq!(
        store.frontier().durable_hlc,
        pre_failure_durable,
        "PROPERTY: even an Ok-capable later sync must never advance the durable \
         frontier once poisoned (fsyncgate)"
    );

    // Fail-closed teardown: close() surfaces the poison too (and must not hang
    // — the rejected Shutdown still exits the writer loop for the join).
    let close_err = store
        .close()
        .map(|_closed| ())
        .expect_err("PROPERTY: closing a poisoned store must fail closed");
    assert!(
        matches!(close_err, StoreError::WriterCrashed),
        "PROPERTY: close on a poisoned store must surface exactly \
         StoreError::WriterCrashed, got {close_err:?}"
    );
}

//! Round-3 mutation kill for the WP-C cooperative-writer-pump survivor.
//!
//! PROVES: `CooperativePump::pump` REALLY drains the writer command queue —
//! in cooperative mode there is no writer thread, so an un-awaited `submit`
//! leaves a measurable backlog, and `AppendTicket::wait()` (which pumps
//! before receiving) must drive that queued append to a real committed
//! receipt on the calling thread.
//! CATCHES: write/writer.rs:148 `CooperativePump::pump -> ()` — a no-op pump
//! never executes a queued command, so the cooperative pipeline stalls at its
//! first reply-await. The WHOLE scenario (open included: the open path itself
//! pumps-then-awaits, so under the mutant even `open_cooperative` hangs) runs
//! inside the testkit bounded-wait watchdog, converting any such stall into a
//! fast, bounded failure instead of a test-binary hang. (On the
//! `--no-default-features` mutation surface this function is compiled out
//! entirely — `CooperativePump` exists only under `dangerous-test-hooks` — so
//! no test on that surface can ever observe the mutation; this harness is the
//! all-features kill.)
//! SEEDED: deterministic — a single fixed append; the only timeout is the
//! watchdog that convicts a mutant-induced stall in bounded time.
#![cfg(feature = "dangerous-test-hooks")]

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::store::{Store, StoreConfig};
use batpak_testkit::bounded_blocking::blocking;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xC, 0x57);

#[test]
fn cooperative_pump_drains_the_queued_append_to_a_committed_receipt() {
    let dir = TempDir::new().expect("temp dir");
    let data_dir = dir.path().to_path_buf();

    // The ENTIRE cooperative scenario runs on the watchdog thread: with a
    // no-op pump the very first reply-await (inside open_cooperative) stalls
    // forever, and the watchdog must convict that in bounded time.
    let (queued, receipt) = blocking("cooperative-pump-scenario", move || {
        let store = Store::open_cooperative(
            StoreConfig::new(&data_dir)
                .with_sync_every_n_events(1)
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false),
        )
        .expect("open cooperative store");
        let coord = Coordinate::new("entity:cooperative-pump", "scope:mutation").expect("coord");

        // submit() enqueues WITHOUT pumping: with no writer thread the
        // command must still be sitting in the mailbox.
        let ticket = store
            .submit(&coord, KIND, &serde_json::json!({ "pumped": true }))
            .expect("submit append");
        let queued = store.diagnostics().writer_pressure.queue_len;

        // Deliberately suppress the store's Drop (ManuallyDrop, per
        // BANNED-002's sanctioned pattern): a cooperative store with a
        // backlog deadlocks in its drop-shutdown drain (there is no pumping
        // thread), so the conviction must be the watchdog, never an
        // unwind-into-Drop hang. The ticket holds its own pump handle; the
        // OS reclaims the suppressed handle at process exit.
        let _store = std::mem::ManuallyDrop::new(store);

        // wait() pumps the queue inline and then receives the reply. Real
        // pump: the append commits immediately. No-op pump (mutant): the
        // reply never arrives and the watchdog fails the test in bounded time.
        (queued, ticket.wait())
    });

    assert!(
        queued >= 1,
        "PROPERTY: an un-awaited cooperative submit leaves its command queued (no thread \
         drains it), got queue_len {queued}"
    );
    let receipt = receipt.expect("PROPERTY: the pumped cooperative append must commit and reply");
    assert_eq!(
        receipt.global_sequence, 1,
        "PROPERTY: the drained append is the store's first committed event"
    );
}

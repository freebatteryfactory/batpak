//! GAUNT-IDEMPOTENCY-AUTHORITY (#189) — availability scoping: the fail-closed
//! authority exists only for REAL user idempotency obligations.
//!
//! PROVES: INV-IDEMPOTENCY-DURABLE-WINDOW's scoping clause — system lifecycle
//! one-shots (`SYSTEM_OPEN`/`SYSTEM_CLOSE_COMPLETED` keys) never create a
//! durable authority obligation, so an ordinary store that never used user
//! idempotency keys does not acquire a fail-closed dependency on
//! `index.idemp`; while a USER key whose live frame was retention-evicted DOES
//! hold the obligation, and losing its authority image refuses the open.
//! CATCHES: the availability nerf where every normally closed store refuses
//! after `index.idemp` deletion; and its inverse, where retiring obligations
//! too eagerly forgets an evicted user key.
//! SEEDED: real stores with tiny segments; deterministic key constants.

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::id::IdempotencyKey;
use batpak::store::{
    AppendOptions, CompactionConfig, CompactionStrategy, Store, StoreConfig, StoreError,
};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 5);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:availability").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

#[test]
fn store_without_user_idempotency_does_not_require_idemp_authority() {
    // An ordinary store: unkeyed appends only. Close still appends keyed
    // SYSTEM lifecycle events internally — those must NOT create a durable
    // authority obligation.
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(config(dir.path())).expect("open");
    for i in 0..3 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "n": i }))
            .expect("unkeyed append");
    }
    store.close().expect("close");

    // No user obligations → no authority image is written at all.
    assert!(
        !dir.path().join(IDEMP_FILENAME).exists(),
        "PROPERTY: a store without user idempotency keys writes no index.idemp \
         (system lifecycle one-shots never create a fail-closed obligation)"
    );

    // And even a stray deletion of the (absent) sidecar cannot brick it: the
    // store reopens and keeps working.
    let reopened = Store::open(config(dir.path())).expect(
        "PROPERTY: a store without user idempotency obligations must reopen \
         without any authority sidecar",
    );
    let _ = reopened
        .append(&coord(), KIND, &serde_json::json!({ "after": "reopen" }))
        .expect("append after reopen");
    reopened.close().expect("close reopened");
}

#[test]
fn keyed_but_evicted_event_still_requires_authority() -> Result<(), Box<dyn std::error::Error>> {
    // The inverse guard: ordinary unkeyed history is exempt, but a USER key
    // whose live frame was retention-evicted is exactly the obligation the
    // authority exists for — deleting index.idemp on such a store is
    // authority LOSS, not a fresh start.
    let dir = TempDir::new().expect("temp dir");
    let key = 0x0B11_6A7E_0000_0000_0000_0000_0000_0001u128;
    {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = store
            .append_with_options(
                &coord(),
                KIND,
                &serde_json::json!({ "keyed": true }),
                AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
            )
            .expect("keyed append");
        // Seal the segment past the keyed frame, then evict every user event
        // of KIND via retention compaction: the frame is gone, the durable
        // idempotency entry is the only remaining proof of the ack.
        for i in 0..8 {
            let _ = store
                .append(&coord(), KIND, &serde_json::json!({ "filler": i }))
                .expect("filler append");
        }
        let _ = store
            .compact(&CompactionConfig {
                strategy: CompactionStrategy::Retention(Box::new(|stored| {
                    stored.event.header.event_kind != KIND
                })),
                min_segments: 1,
            })
            .expect("retention compaction");
        store.close().expect("close");
    }

    assert!(
        dir.path().join(IDEMP_FILENAME).exists(),
        "PRECONDITION: the evicted user key left a durable authority image"
    );
    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;

    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): deleting the authority image of a store with an \
                 evicted user obligation must refuse, never open fresh",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityMissing { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    Ok(())
}

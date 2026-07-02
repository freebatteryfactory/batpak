//! PROVES: the crypto-shred durability fence bites on the BATCH append path —
//! a `SimFs` fault on the keyset fence-flush during a batch that mints a fresh
//! scope key fails the WHOLE batch closed (typed `StoreError::Io`, zero events
//! visible, no keyset file published), and the unfaulted twin proves the same
//! fixture genuinely reaches the fence (the batch commits and the keyset file
//! is durable). The single-append twin lives in
//! `store::keyscope::persist::crash_tests`.
//! CATCHES: `raise_batch_durability_fence -> Ok(())` (writer/batch.rs:488) and
//! `|=` -> `&=` on `needs_fence` in `batch_event_hash` (writer/batch.rs:239) —
//! either skips the fence, so a batch would ack ciphertext whose key is on
//! disk nowhere (a silent, unintended crypto-shred of live data on the next
//! crash).
//! SEEDED: deterministic — fixed SimFs seeds, honest fsync schedule
//! (`fsync_drop_one_in = 0`), a single armed `PersistTemp` fault.
//!
//! Internal (not an integration test) because [`SimFs`] and
//! [`StoreConfig::with_fs`] are `pub(crate)` fault-injection seams.

use crate::coordinate::{Coordinate, Region};
use crate::event::EventKind;
use crate::store::file_classification::KEYSET_FILENAME;
use crate::store::keyscope::KeyScopeGranularity;
use crate::store::sim::fs::{CrashOp, SimFs};
use crate::store::{AppendOptions, BatchAppendItem, CausationRef, Store, StoreConfig, StoreError};
use std::sync::Arc;

const KIND: EventKind = EventKind::custom(0xE, 0x22);

fn minting_batch(coord: &Coordinate, count: usize) -> Vec<BatchAppendItem> {
    (0..count)
        .map(|i| {
            BatchAppendItem::new(
                coord.clone(),
                KIND,
                &serde_json::json!({ "i": i }),
                AppendOptions::default(),
                CausationRef::None,
            )
            .expect("construct batch item")
        })
        .collect()
}

fn encrypted_config(dir: &std::path::Path, sim_fs: &Arc<SimFs>) -> StoreConfig {
    StoreConfig::new(dir)
        .with_payload_encryption(KeyScopeGranularity::PerEntity)
        .with_fs(Arc::clone(sim_fs) as Arc<dyn crate::store::platform::fs::StoreFs>)
}

fn visible_user_events(store: &Store) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|entry| !entry.event_kind().is_reserved())
        .count()
}

#[test]
fn a_faulted_keyset_fence_flush_fails_a_minting_batch_closed() {
    // ---- CONTROL: the identical fixture over an HONEST SimFs commits, and the
    // minted key's fence-flush published the keyset file (proving the batch
    // path genuinely reaches the atomic keyset publish this test faults). ----
    {
        let dir = tempfile::tempdir().expect("tmpdir");
        let sim_fs = Arc::new(SimFs::new(0xBA7C_0001, 0));
        let store =
            Store::open(encrypted_config(dir.path(), &sim_fs)).expect("open encrypted store");
        let coord = Coordinate::new("entity:batch-fence", "scope:mk2").expect("coord");

        let receipts = store
            .append_batch(minting_batch(&coord, 2))
            .expect("an unfaulted minting batch commits");
        assert_eq!(receipts.len(), 2, "both batch items are acked");
        assert!(
            dir.path().join(KEYSET_FILENAME).exists(),
            "PROPERTY: the fence published the minted key's keyset BEFORE the batch acked"
        );
        assert_eq!(
            visible_user_events(&store),
            2,
            "both committed batch events are visible"
        );
        store.close().expect("close control store");
    }

    // ---- FAULT: arm the FIRST atomic keyset publish AFTER open, so the
    // durability fence of a batch that mints a fresh scope key is the op that
    // tears. The whole batch must fail CLOSED: typed Io error, no BEGIN marker
    // / frames (zero visible events), no keyset file on disk. ----
    let dir = tempfile::tempdir().expect("tmpdir");
    let sim_fs = Arc::new(SimFs::new(0xBA7C_0002, 0));
    let store = Store::open(encrypted_config(dir.path(), &sim_fs)).expect("open encrypted store");
    let coord = Coordinate::new("entity:batch-fence", "scope:mk2").expect("coord");
    sim_fs.arm_fault_on(CrashOp::PersistTemp, 1);

    let err = store
        .append_batch(minting_batch(&coord, 2))
        .expect_err("PROPERTY: a minting batch whose fence-flush tears must fail closed");
    assert!(
        matches!(err, StoreError::Io(_)),
        "PROPERTY: the torn fence publish must surface as StoreError::Io, got {err:?}"
    );
    assert_eq!(
        visible_user_events(&store),
        0,
        "PROPERTY: the failed batch published NOTHING — no partial visibility"
    );
    assert!(
        !dir.path().join(KEYSET_FILENAME).exists(),
        "PROPERTY: the torn publish left no keyset file (the temp never landed)"
    );

    // The fence failure must not poison the writer: the SAME batch retried over
    // the now-honest disk re-fences (the keyset stayed dirty) and commits.
    let receipts = store
        .append_batch(minting_batch(&coord, 2))
        .expect("PROPERTY: the retried batch re-raises the fence and commits");
    assert_eq!(receipts.len(), 2, "the retried batch acks both items");
    assert!(
        dir.path().join(KEYSET_FILENAME).exists(),
        "PROPERTY: the retried fence published the keyset durably"
    );
    store.close().expect("close faulted-then-recovered store");
}

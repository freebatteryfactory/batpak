use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::store::lifecycle_close::write_cold_start_artifacts_on_close;
use crate::store::write::control::AppendSubmission;
use crate::store::{
    AppendOptions, Closed, Open, Store, StoreDiagnostics, StoreError, StoreStats, WriterPressure,
};
use serde::Serialize;

#[path = "lifecycle_compact.rs"]
mod lifecycle_compact;
#[path = "lifecycle_fork.rs"]
mod lifecycle_fork;
#[path = "lifecycle_snapshot.rs"]
mod lifecycle_snapshot;

pub(crate) use lifecycle_compact::compact;
pub(crate) use lifecycle_fork::fork;
pub(crate) use lifecycle_snapshot::snapshot;

/// Keyset portability gate (D24), shared by `snapshot` and `fork`. Returns
/// whether the keyset was deliberately EXCLUDED from a copy of an
/// encryption-active store:
///
/// - `Ok(false)` — no payload encryption active (nothing to exclude); the copy
///   proceeds exactly as it always has.
/// - `Ok(true)` — encryption active and the caller passed
///   [`KeysetPolicy::ExcludeKeys`](crate::store::KeysetPolicy::ExcludeKeys); the
///   copy proceeds and its report is stamped keys-excluded.
/// - `Err(KeysetNotPortable)` — encryption active and the default
///   [`KeysetPolicy::Refuse`](crate::store::KeysetPolicy::Refuse) fails closed: a
///   copy without its keys is silently unrestorable, and a copy WITH its keys
///   would let a restored copy resurrect crypto-shredded data.
///
/// Always `Ok(false)` without the `payload-encryption` feature — no store can be
/// encrypted, so snapshot/fork behave exactly as they do today.
fn resolve_keyset_exclusion(
    store: &Store<Open>,
    policy: crate::store::KeysetPolicy,
    operation: &'static str,
) -> Result<bool, StoreError> {
    #[cfg(feature = "payload-encryption")]
    {
        if store.key_store.is_some() {
            return match policy {
                crate::store::KeysetPolicy::Refuse => {
                    Err(StoreError::KeysetNotPortable { operation })
                }
                crate::store::KeysetPolicy::ExcludeKeys => Ok(true),
            };
        }
    }
    #[cfg(not(feature = "payload-encryption"))]
    {
        let _ = (store, policy, operation);
    }
    Ok(false)
}

#[derive(Serialize)]
struct CloseLifecyclePayload {
    wall_ms: u64,
    global_sequence: u64,
}

fn append_close_completed_event(store: &Store<Open>) -> Result<(), StoreError> {
    let close_hlc = store.watermark_handle.lock().snapshot().visible_hlc;
    let coord = Coordinate::new("batpak:store", "batpak:lifecycle")?;
    let submission = AppendSubmission::with_options(
        AppendOptions::default().with_idempotency(crate::id::IdempotencyKey::from(
            crate::id::generate_v7_id_with_clock(store.runtime.clock()),
        )),
        store.runtime.clock(),
    );
    submission.validate_route(store)?;
    submission.validate_idempotency(store)?;

    let payload = CloseLifecyclePayload {
        wall_ms: close_hlc.wall_ms,
        global_sequence: close_hlc.global_sequence,
    };
    let event = submission.build_event(
        &payload,
        EventKind::SYSTEM_CLOSE_COMPLETED,
        super::timestamp_us_for_hlc(close_hlc)?,
    )?;

    let (tx, rx) = flume::bounded(1);
    let command = submission.into_command(coord, EventKind::SYSTEM_CLOSE_COMPLETED, event, tx);
    store
        .writer_handle()?
        .tx
        .send(command)
        .map_err(|_| StoreError::WriterCrashed)?;
    store.writer_handle()?.pump();
    drop(crate::store::recv_writer_reply(&rx)?);
    Ok(())
}

pub(crate) fn sync(store: &Store<Open>) -> Result<(), StoreError> {
    tracing::debug!(target: "batpak::flow", flow = "sync");
    let (tx, rx) = flume::bounded(1);
    store
        .writer_handle()?
        .tx
        .send(crate::store::write::writer::WriterCommand::Sync { respond: tx })
        .map_err(|_| StoreError::WriterCrashed)?;
    store.writer_handle()?.pump();
    crate::store::recv_writer_reply(&rx)?;
    // Flush the projection cache AFTER the durable segment sync. The cache is a
    // derivative of the segment log, so it must never advance ahead of it. For
    // rebuildable backends (`NoCache`, `NativeCache`) this is a no-op; a durable
    // custom `ProjectionCache` whose `put` buffers writes relies on this call to
    // persist them — without it, `sync()` on the store was a latent durability
    // trap that never reached the cache.
    store.cache.sync()
}

pub(crate) fn close(mut store: Store<Open>) -> Result<Closed, StoreError> {
    tracing::debug!(target: "batpak::flow", flow = "close");
    let _lifecycle = store.lifecycle_gate.lock();
    if let Err(error) = append_close_completed_event(&store) {
        tracing::warn!(
            target: "batpak::flow",
            flow = "close",
            "failed to append SYSTEM_CLOSE_COMPLETED lifecycle event: {error}"
        );
    }

    let (tx, rx) = flume::bounded(1);
    let writer = store.writer_handle()?;
    writer
        .tx
        .send(crate::store::write::writer::WriterCommand::Shutdown { respond: tx })
        .map_err(|_| StoreError::WriterCrashed)?;
    writer.pump();
    let result = crate::store::recv_writer_reply(&rx);

    if let Err(error) = result {
        // A poisoned writer (failed durability sync — see
        // `StoreError::WriterCrashed`) rejects the Shutdown with an error reply
        // and then exits its loop. Join it to quiescence and take over drop's
        // shutdown duty here: a second drop-time Shutdown would RACE the
        // exiting thread — the command can land in the channel queue just
        // before the writer drops its receiver, and its reply sender then sits
        // in the zombie queue (kept alive by our own `tx`) forever, hanging
        // the drop-time ack wait. Non-poisoned shutdown errors keep the old
        // contract: drop retries the shutdown drain.
        if store.watermark_handle.is_poisoned() {
            store.state.0.join()?;
            store.should_shutdown_on_drop = false;
        }
        return Err(error);
    }
    store.state.0.join()?;

    crate::store::store_meta::flush_idempotency_authority(
        &store.index,
        &store.config.data_dir,
        store.config.fs().as_ref(),
        store.runtime.clock(),
    )?;

    write_cold_start_artifacts_on_close(&store)?;

    // Flush the projection cache as the final durability step of close, after the
    // writer has shut down and the segment log + idempotency index are on disk.
    // A durable custom `ProjectionCache` relies on this to persist any buffered
    // writes before the handle is dropped; rebuildable backends no-op here.
    store.cache.sync()?;

    store.should_shutdown_on_drop = false;
    Ok(Closed)
}

pub(crate) fn stats<State: crate::store::StoreState>(store: &Store<State>) -> StoreStats {
    StoreStats {
        event_count: store.index.len(),
        global_sequence: store.index.global_sequence(),
    }
}

pub(crate) fn diagnostics<State: crate::store::StoreState>(
    store: &Store<State>,
) -> StoreDiagnostics {
    let frontier = store.watermark_handle.lock().snapshot_view();
    StoreDiagnostics {
        event_count: store.index.len(),
        global_sequence: store.index.global_sequence(),
        visible_sequence: store.index.visible_sequence(),
        data_dir: store.config.data_dir.clone(),
        segment_max_bytes: store.config.segment_max_bytes,
        fd_budget: store.config.fd_budget,
        restart_policy: store.config.writer.restart_policy.clone(),
        writer_pressure: store
            .state
            .writer_queue_len()
            .map(|queue_len| WriterPressure {
                queue_len,
                capacity: store.config.writer.channel_capacity,
            })
            .unwrap_or(WriterPressure {
                queue_len: 0,
                capacity: 0,
            }),
        frontier,
        index_topology: store.index.topology_name(),
        tile_count: store.index.tile_count(),
        open_report: store.open_report.clone(),
        platform_evidence: crate::store::platform::evidence::collect_for_store_path(
            &store.config.data_dir,
            store.runtime.clock(),
            store.config.fs().as_ref(),
        ),
    }
}

#[cfg(test)]
mod cache_sync_lifecycle_tests {
    use crate::store::projection::{CacheCapabilities, CacheMeta, ProjectionCache};
    use crate::store::{Store, StoreConfig, StoreError};
    use std::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A projection cache whose only job is to record how many times `sync()`
    /// was invoked, so a test can prove `Store::sync`/`Store::close` route
    /// through the projection cache's flush path.
    struct SyncCountingCache {
        syncs: Arc<AtomicUsize>,
    }

    impl ProjectionCache for SyncCountingCache {
        fn capabilities(&self) -> CacheCapabilities {
            CacheCapabilities::none()
        }

        fn get(&self, _key: &[u8]) -> Result<Option<(Vec<u8>, CacheMeta)>, StoreError> {
            Ok(None)
        }

        fn put(&self, _key: &[u8], _value: &[u8], _meta: CacheMeta) -> Result<(), StoreError> {
            Ok(())
        }

        fn delete_prefix(&self, _prefix: &[u8]) -> Result<u64, StoreError> {
            Ok(0)
        }

        fn sync(&self) -> Result<(), StoreError> {
            self.syncs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn store_sync_and_close_flush_the_projection_cache() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let syncs = Arc::new(AtomicUsize::new(0));
        let cache = Box::new(SyncCountingCache {
            syncs: Arc::clone(&syncs),
        });

        let store = Store::open_with_cache(StoreConfig::new(dir.path()), cache)?;

        // `open_with_cache` runs an internal `lifecycle::sync`, so the cache has
        // already been flushed at least once by the time open returns — the wire
        // is live from the first tick.
        let after_open = syncs.load(Ordering::SeqCst);
        assert!(
            after_open >= 1,
            "PROPERTY: opening a store flushes the projection cache via lifecycle::sync"
        );

        // UFCS call of the public `Store::sync` API (the `store/` bare-`.sync()` build.rs
        // guard targets internal segment syncs that must use `sync_with_mode`; this is the
        // public lifecycle entry point under test, invoked in `::` form to stay clear of it).
        Store::sync(&store)?;
        let after_sync = syncs.load(Ordering::SeqCst);
        assert!(
            after_sync > after_open,
            "PROPERTY: Store::sync flushes the projection cache (was {after_open}, now {after_sync})"
        );

        store.close()?;
        let after_close = syncs.load(Ordering::SeqCst);
        assert!(
            after_close > after_sync,
            "PROPERTY: Store::close flushes the projection cache (was {after_sync}, now {after_close})"
        );

        Ok(())
    }
}

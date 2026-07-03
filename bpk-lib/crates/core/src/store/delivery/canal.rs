use crate::store::index::IndexEntry;
use crate::store::platform::spawn::JobHandle;
use crate::store::write::fanout::Notification;
use crate::store::StoreError;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// One pulled canal batch without forcing one allocation for one-item canals.
#[derive(Debug)]
pub enum CanalBatch<I> {
    /// No matching item was available before the deadline.
    Empty,
    /// Exactly one item was available.
    One(I),
    /// More than one item was available.
    Many(Vec<I>),
}

impl<I> CanalBatch<I> {
    /// Returns true when this batch contains no item.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// Minimal event reference yielded by a [`Canal`].
pub trait CanalItem {
    /// Event id to fetch from the store replay lane.
    fn event_id(&self) -> crate::id::EventId;
}

impl CanalItem for IndexEntry {
    fn event_id(&self) -> crate::id::EventId {
        self.event_id()
    }
}

impl CanalItem for Notification {
    fn event_id(&self) -> crate::id::EventId {
        self.event_id
    }
}

/// Terminal canal closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanalClosed;

impl std::fmt::Display for CanalClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canal closed")
    }
}

impl std::error::Error for CanalClosed {}

/// Common consumption surface over shipped delivery primitives.
///
/// Implementors keep their own ordering, backpressure, durability, restart,
/// checkpoint, and witness contracts. `Canal` standardises only "produce the
/// next batch the caller should inspect".
pub trait Canal: Send {
    /// Per-item unit yielded by this canal.
    type Item: CanalItem + Send;
    /// Error returned by a terminal or failed pull.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Pull up to `max` items, blocking for at most `deadline` when no item is
    /// immediately available.
    ///
    /// An empty batch means timeout/idle. An error means the canal cannot
    /// produce more items and the caller should stop.
    ///
    /// # Errors
    /// Returns the implementation's terminal error when the canal is closed or
    /// can no longer produce items.
    fn pull_batch(
        &mut self,
        max: usize,
        deadline: Duration,
    ) -> Result<CanalBatch<Self::Item>, Self::Error>;
}

/// Lifecycle for a running canal-backed worker.
pub trait CanalHandle: Send {
    /// Signal stop without blocking.
    fn stop(&self);
    /// Wait passively for worker exit.
    ///
    /// # Errors
    /// Returns a store error when the worker panicked or stashed a terminal
    /// store-level failure before exiting.
    fn join(self: Box<Self>) -> Result<(), StoreError>;
    /// Signal stop, then wait for worker exit.
    ///
    /// # Errors
    /// Returns the same failures as [`join`](Self::join).
    fn stop_and_join(self: Box<Self>) -> Result<(), StoreError>;
}

/// Handle for lossy subscription-backed workers.
pub(crate) struct SubscriptionWorkerHandle {
    stop: Arc<AtomicBool>,
    join: Option<Box<dyn JobHandle>>,
    error_slot: Arc<Mutex<Option<StoreError>>>,
}

impl SubscriptionWorkerHandle {
    pub(crate) fn new(
        stop: Arc<AtomicBool>,
        join: Box<dyn JobHandle>,
        error_slot: Arc<Mutex<Option<StoreError>>>,
    ) -> Self {
        Self {
            stop,
            join: Some(join),
            error_slot,
        }
    }

    fn finish_join(&mut self) -> Result<(), StoreError> {
        if let Some(join) = self.join.take() {
            join.join().map_err(|_| StoreError::WriterCrashed)?;
        }
        let mut guard = self.error_slot.lock();
        guard.take().map_or(Ok(()), Err)
    }
}

impl CanalHandle for SubscriptionWorkerHandle {
    fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    fn join(mut self: Box<Self>) -> Result<(), StoreError> {
        self.finish_join()
    }

    fn stop_and_join(mut self: Box<Self>) -> Result<(), StoreError> {
        self.stop();
        self.finish_join()
    }
}

impl Drop for SubscriptionWorkerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Delivery canal used by typed reactor runners.
///
/// This is intentionally a selector over existing primitives, not a new owner
/// of delivery semantics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReactorCanal {
    /// Ordered pull replay through [`Cursor`](crate::store::Cursor).
    ///
    /// This is the default typed-reactor canal. It is at-least-once within the
    /// process and can become durable at-least-once when the reactor carries a
    /// checkpoint id.
    #[default]
    CursorGuaranteed,
    /// Lossy push observation through [`Subscription`](crate::store::Subscription).
    ///
    /// This keeps writer isolation and does not checkpoint, restart, or provide
    /// an [`AtLeastOnce`](crate::store::AtLeastOnce) witness. Use it only for
    /// live views that may skip work under backpressure.
    LossySubscription,
}

#[cfg(test)]
mod tests {
    use super::{CanalBatch, CanalClosed, CanalHandle, SubscriptionWorkerHandle};
    use crate::store::platform::spawn::{Spawn, ThreadSpawn};
    use crate::store::StoreError;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Build a `SubscriptionWorkerHandle` whose backing job has already run to
    /// completion, with the supplied error pre-seeded in the store error slot.
    /// The join field points at a real (finished) `ThreadSpawn` job so the
    /// finish_join path exercises a genuine, non-panicking `JobHandle::join`.
    fn handle_with_seeded_error(
        seeded: Option<StoreError>,
    ) -> (SubscriptionWorkerHandle, Arc<AtomicBool>) {
        let stop = Arc::new(AtomicBool::new(false));
        let error_slot = Arc::new(Mutex::new(seeded));
        let spawner = ThreadSpawn;
        let job = spawner
            .spawn("canal-finish-join-proof".to_string(), None, Box::new(|| {}))
            .expect("spawn a trivial finished job");
        let handle = SubscriptionWorkerHandle::new(Arc::clone(&stop), job, Arc::clone(&error_slot));
        (handle, stop)
    }

    #[test]
    fn join_surfaces_the_stashed_store_error_not_a_hardwired_ok() {
        // Kills canal.rs:124 `finish_join -> Ok(())` and canal.rs:138
        // `<CanalHandle>::join -> Ok(())`: a subscription worker that stashed a
        // terminal StoreError must surface it through `join`, never a blanket
        // Ok(()). Seed a distinctive error and require it back verbatim.
        let (handle, _stop) = handle_with_seeded_error(Some(StoreError::WriterCrashed));
        let result = Box::new(handle).join();
        assert!(
            matches!(result, Err(StoreError::WriterCrashed)),
            "PROPERTY: join must return the worker's stashed terminal error; \
             the Ok(()) mutants of finish_join/join swallow it, got {result:?}"
        );
    }

    #[test]
    fn stop_and_join_surfaces_the_stashed_store_error() {
        // Kills canal.rs:142 `<CanalHandle>::stop_and_join -> Ok(())`: the
        // stop-then-wait path must also propagate the stashed terminal error.
        let (handle, stop) = handle_with_seeded_error(Some(StoreError::WriterCrashed));
        let result = Box::new(handle).stop_and_join();
        assert!(
            matches!(result, Err(StoreError::WriterCrashed)),
            "PROPERTY: stop_and_join must return the stashed error; the Ok(()) \
             mutant swallows it, got {result:?}"
        );
        assert!(
            stop.load(Ordering::Acquire),
            "stop_and_join must also raise the stop flag"
        );
    }

    #[test]
    fn a_clean_worker_joins_ok() {
        // Guards the above from vacuity: with no stashed error a clean worker
        // joins Ok(()), so the Err assertions above are pinned to the seeded
        // error and not to join always failing.
        let (handle, _stop) = handle_with_seeded_error(None);
        Box::new(handle)
            .join()
            .expect("a worker with no stashed error joins cleanly");
    }

    #[test]
    fn drop_signals_stop_to_the_background_worker() {
        // Kills canal.rs:149 `<Drop>::drop -> ()`: dropping the handle must set
        // the stop flag so the (still-running, in production) worker winds down.
        // The no-op mutant leaves the flag false and leaks the worker.
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
    fn canal_closed_displays_its_terminal_text() {
        // Kills canal.rs:53 `<Display for CanalClosed>::fmt -> Ok(Default::default())`:
        // the mutant writes nothing, collapsing the message to an empty string.
        assert_eq!(
            CanalClosed.to_string(),
            "canal closed",
            "PROPERTY: CanalClosed renders exact terminal text; the mutant emits \"\""
        );
    }

    #[test]
    fn r4_canal_batch_is_empty_only_for_the_empty_variant() {
        // Kills canal.rs:25 `CanalBatch::is_empty` -> `false`: the typed
        // reactor pump gates its dispatch/idle decision on `!batch.is_empty()`
        // (reactor_typed.rs), so an Empty batch that reports non-empty turns
        // every idle/timeout poll into a phantom dispatch round. Pin all three
        // variants: exactly Empty is empty, One and Many are not.
        assert!(
            CanalBatch::<u32>::Empty.is_empty(),
            "PROPERTY: an Empty batch reports is_empty() == true; the `false` \
             mutant makes every idle poll look like deliverable work"
        );
        assert!(
            !CanalBatch::One(7_u32).is_empty(),
            "a One batch is non-empty"
        );
        assert!(
            !CanalBatch::Many(vec![1_u32, 2]).is_empty(),
            "a Many batch is non-empty"
        );
    }
}

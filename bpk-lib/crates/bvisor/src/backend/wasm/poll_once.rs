//! Resolve an already-complete future synchronously.
//!
//! wasi-common's `wasmtime_wasi::sync` backend performs its filesystem work
//! synchronously; its `WasiDir` futures are ready on first poll and never need
//! a runtime. This mirrors wasmtime's sync executor locally with safe std APIs
//! so the confinement layer can keep its policy logic in plain sync functions.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}

    fn wake_by_ref(self: &Arc<Self>) {}
}

/// Poll `future` exactly once and return its output when it is ready.
pub(super) fn resolve_ready<F: Future>(future: F) -> Option<F::Output> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_ready;

    #[test]
    fn resolves_a_ready_future() {
        assert_eq!(resolve_ready(std::future::ready(7u8)), Some(7));
    }

    #[test]
    fn reports_pending_as_none() {
        assert!(resolve_ready(std::future::pending::<u8>()).is_none());
    }
}

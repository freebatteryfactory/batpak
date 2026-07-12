//! Resolve an already-complete future synchronously.
//!
//! wasi-common's sync backend performs its filesystem work synchronously; its
//! `WasiDir` futures are ready on first poll and never need a runtime. This
//! mirrors the sync executor step locally with safe std APIs
//! so the confinement layer can keep its policy logic in plain sync functions.

use std::future::Future;
use std::task::{Context, Poll, Waker};

/// Poll `future` exactly once and return its output when it is ready.
pub(super) fn resolve_ready<F: Future>(future: F) -> Option<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
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

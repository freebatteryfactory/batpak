use super::{CanalHandle, ReactorError, StoreError, TypedReactorHandle};
use parking_lot::Mutex;
use std::sync::Arc;

/// A CanalHandle whose lifecycle calls always succeed, so a stashed reactor
/// error is the ONLY thing `TypedReactorHandle::stop_and_join` could surface.
struct OkCanalHandle;

impl CanalHandle for OkCanalHandle {
    fn stop(&self) {}
    fn join(self: Box<Self>) -> Result<(), StoreError> {
        Ok(())
    }
    fn stop_and_join(self: Box<Self>) -> Result<(), StoreError> {
        Ok(())
    }
}

#[test]
fn typed_reactor_stop_and_join_surfaces_the_stashed_reactor_error() {
    // Kills reactor_typed.rs:280 `TypedReactorHandle::stop_and_join ->
    // Ok(())`. The runner stashes terminal reactor errors (user/decode/store/
    // restart-budget) in the shared slot; stop_and_join must drain and return
    // them. A blanket Ok(()) silently reports a failed reactor as clean.
    let error_slot: Arc<Mutex<Option<ReactorError<std::io::Error>>>> =
        Arc::new(Mutex::new(Some(ReactorError::RestartBudgetExhausted)));
    let handle = TypedReactorHandle {
        inner: Box::new(OkCanalHandle),
        error_slot,
    };

    let result = handle.stop_and_join();
    assert!(
        matches!(result, Err(ReactorError::RestartBudgetExhausted)),
        "PROPERTY: stop_and_join must surface the stashed reactor error; the \
             Ok(()) mutant swallows it, got {result:?}"
    );
}

#[test]
fn typed_reactor_stop_and_join_is_ok_when_no_error_is_stashed() {
    // Guards the kill above from vacuity: with an empty slot and a clean inner
    // handle, stop_and_join returns Ok(()).
    let error_slot: Arc<Mutex<Option<ReactorError<std::io::Error>>>> = Arc::new(Mutex::new(None));
    let handle = TypedReactorHandle {
        inner: Box::new(OkCanalHandle),
        error_slot,
    };
    handle
        .stop_and_join()
        .expect("a reactor with no stashed error stops cleanly");
}

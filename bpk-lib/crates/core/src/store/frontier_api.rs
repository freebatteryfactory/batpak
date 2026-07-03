use super::*;

impl<State: crate::store::StoreState> Store<State> {
    /// Return the current operator-facing frontier view.
    pub fn frontier(&self) -> FrontierView {
        self.watermark_handle.lock().snapshot_view()
    }

    /// Return a coherent clone of the internal frontier watermarks.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_watermark_snapshot(&self) -> FrontierView {
        self.watermark_handle.lock().snapshot_view()
    }

    /// Register a projection ID in the applied-frontier registry.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_register_projection(&self, projection_id: &str) {
        self.projection_registry.register(projection_id.to_owned());
    }

    /// Register the same projection ID used by `project::<T>()` for `entity`.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_register_projection_for<T: 'static>(&self, entity: &str) {
        self.projection_registry
            .register(ProjectionRegistry::id_for_type::<T>(entity));
    }

    /// Report projection progress directly for focused frontier tests.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_notify_projection_applied(&self, projection_id: &str, point: HlcPoint) {
        self.projection_registry
            .notify_applied(projection_id.to_owned(), point);
    }

    /// Remove a projection ID from the applied-frontier registry.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_unregister_projection(&self, projection_id: &str) {
        self.projection_registry.unregister(projection_id);
    }

    /// Wake frontier waiters without advancing a watermark.
    #[cfg(any(test, feature = "dangerous-test-hooks"))]
    pub fn dangerous_notify_watermark_waiters(&self) {
        self.watermark_handle.dangerous_notify_all();
    }
}

#[cfg(test)]
#[path = "frontier_api_mutation_kill.rs"]
mod frontier_api_mutation_kill;

#[cfg(test)]
mod tests {
    use crate::coordinate::Coordinate;
    use crate::store::stats::HlcPoint;
    use crate::store::{Store, StoreConfig};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct FrontierProbe {
        n: u64,
    }
    impl crate::event::EventPayload for FrontierProbe {
        const KIND: crate::event::EventKind = crate::event::EventKind::custom(0xE, 0x4F4);
    }

    #[test]
    fn r4_dangerous_notify_projection_applied_advances_the_applied_frontier() {
        // Kills frontier_api.rs:30 `dangerous_notify_projection_applied` -> `()`:
        // the hook must actually push progress into the projection registry.
        // A no-op body silently freezes the applied frontier, so every focused
        // frontier test built on the hook asserts against a stale watermark.
        // Observable: `notify_applied` recomputes min-over-projections and
        // `set_applied`s it SYNCHRONOUSLY, so after registering ONE projection
        // and notifying it at an exact HlcPoint at the visible frontier (the
        // watermark invariant requires visible >= applied by sequence), the
        // public `frontier()` view's applied_hlc IS that point. Only the
        // registry writes applied, so the pin is race-free.
        let dir = tempfile::TempDir::new().expect("create temp data dir");
        let store = Store::open(StoreConfig::new(dir.path())).expect("open store");

        // Advance the visible frontier past the applied baseline so the notify
        // point has headroom under the visible >= applied invariant.
        let receipt = store
            .append_typed(
                &Coordinate::new("entity:r4-frontier", "scope:test").expect("valid coordinate"),
                &FrontierProbe { n: 1 },
            )
            .expect("append the probe event");
        let visible = store.frontier().visible_hlc;
        assert!(
            visible.global_sequence >= receipt.global_sequence,
            "sanity: an acked append is visible (visible {visible:?} covers the receipt)"
        );

        let point = HlcPoint {
            wall_ms: 7_777,
            global_sequence: visible.global_sequence,
        };
        let applied_before = store.frontier().applied_hlc;
        assert!(
            applied_before.global_sequence < point.global_sequence,
            "sanity: the applied baseline {applied_before:?} sits below the notify point, \
             so the pin below is not vacuous"
        );

        store.dangerous_register_projection("projection:r4-frontier");
        store.dangerous_notify_projection_applied("projection:r4-frontier", point);

        assert_eq!(
            store.frontier().applied_hlc,
            point,
            "PROPERTY: notify_applied must advance the applied frontier to the \
             exact notified point; a `()` body leaves it at the registration \
             baseline below the visible frontier"
        );
    }
}

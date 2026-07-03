//! Key-aware reactor delivery helpers (crypto-shred Stage E2).
//!
//! Split out of `reactor_typed` so that module stays under its size cap. These
//! decrypt an event at the core boundary before it is dispatched to a reactor,
//! and observe a crypto-shredded event as a loud skip. Without
//! `payload-encryption` — or on a store with no keyset — the fetch is exactly
//! [`Store::read_raw`], byte-identical.

use crate::event::StoredEvent;
use crate::store::{Open, Store, StoreError};

/// Raw-lane reactor fetch that is key-aware under `payload-encryption`: an
/// encrypted event is decrypted to its plaintext MessagePack bytes (so the
/// per-kind raw decode sees plaintext, not ciphertext), and a crypto-shredded
/// event surfaces as [`StoreError::PayloadShredded`] so the dispatch loop skips
/// it and advances the cursor.
pub(super) fn fetch_raw_key_aware(
    store: &Store<Open>,
    event_id: crate::id::EventId,
) -> Result<StoredEvent<Vec<u8>>, StoreError> {
    #[cfg(feature = "payload-encryption")]
    if store.key_store.is_some() {
        return match store.read_delivery_payload(event_id)? {
            crate::store::DeliveryPayload::Readable(stored) => Ok(*stored),
            crate::store::DeliveryPayload::Shredded { event_id } => {
                Err(StoreError::PayloadShredded { event_id })
            }
        };
    }
    store.read_raw(event_id)
}

/// Emit the observable, structured warn for a crypto-shredded event skipped
/// during reactor delivery. The reactor is not invoked for the event and the
/// cursor advances past it; the skip is LOUD (this warn), never silent.
#[cfg(feature = "payload-encryption")]
pub(super) fn warn_shredded_reactor_delivery(entity: &str, event_id: crate::id::EventId) {
    use crate::id::EntityIdType;
    tracing::warn!(
        target: "batpak::delivery",
        flow = "reactor",
        entity,
        event_id = event_id.as_u128(),
        "skipping a crypto-shredded event during reactor delivery; the reactor is not \
         invoked for it and the cursor advances past it (payload key destroyed — plaintext gone)"
    );
}

#[cfg(all(test, feature = "payload-encryption"))]
mod tests {
    use super::warn_shredded_reactor_delivery;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Level, Metadata, Subscriber};

    #[derive(Default)]
    struct Captured {
        events: Vec<(String, Level, String)>,
    }

    struct MessageVisitor<'a> {
        message: &'a mut String,
    }

    impl Visit for MessageVisitor<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                *self.message = format!("{value:?}");
            }
        }
    }

    struct CaptureSubscriber {
        captured: Arc<Mutex<Captured>>,
    }

    impl Subscriber for CaptureSubscriber {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            let metadata = event.metadata();
            let mut message = String::new();
            event.record(&mut MessageVisitor {
                message: &mut message,
            });
            self.captured.lock().expect("capture lock").events.push((
                metadata.target().to_string(),
                *metadata.level(),
                message,
            ));
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    #[test]
    fn warn_shredded_reactor_delivery_emits_the_loud_skip_warning() {
        // Kills reactor_delivery.rs:38 `warn_shredded_reactor_delivery -> ()`.
        // The crypto-shred skip is contractually LOUD (never silent): the reactor
        // delivery loop calls this to emit one WARN on target "batpak::delivery".
        // The `()` mutant emits nothing. Capture events via a scoped subscriber
        // and require exactly one WARN naming the crypto-shred skip.
        let captured = Arc::new(Mutex::new(Captured::default()));
        let subscriber = CaptureSubscriber {
            captured: Arc::clone(&captured),
        };
        let event_id = crate::id::EventId::from_u128(0x5152_5354);

        tracing::subscriber::with_default(subscriber, || {
            warn_shredded_reactor_delivery("entity:crypto-shred-warn", event_id);
        });

        let events = captured.lock().expect("capture lock").events.clone();
        assert_eq!(
            events.len(),
            1,
            "PROPERTY: exactly one warn fires; the `()` mutant emits none, got {events:?}"
        );
        let (target, level, message) = &events[0];
        assert_eq!(
            target, "batpak::delivery",
            "the loud skip is emitted on the delivery target"
        );
        assert_eq!(*level, Level::WARN, "the loud skip is a WARN");
        assert!(
            message.contains("crypto-shredded"),
            "the warn names the crypto-shred skip, got {message:?}"
        );
    }
}

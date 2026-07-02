//! ROUND 1 —
//! PROVES: invalid-ACK terminal errors carry the `malformed_stream_frame`
//! stream code (not the fallback `cursor_invalid`) for the two structural ACK
//! faults that share that classification.
//! CATCHES: the surviving delete-arm mutant in
//! `crates/syncbat/src/subscription_runtime/session.rs` (`ack_invalid_error`,
//! line 270) which would let "ack delivery index out of range" and "ack cursor
//! does not match sent cursor" fall through to the `cursor_invalid` arm.
//!
//! Everything else in the round-1 `syncbat-subscription-runtime` .missed list is
//! already killed by the existing suites and is intentionally NOT duplicated here:
//!   - cursor.rs / entity_cursor.rs / operation_status.rs /
//!     operation_status_sink.rs (and the envelope.rs field helpers): inline
//!     `#[cfg(test)]` helper/mutation modules.
//!   - registry.rs (all 87): `mutation_kill_syncbat-registry.rs`.
//!   - the per-stream apply_ack / delivery-index / watermark operators:
//!     `mutation_kill_syncbat-streams.rs`.
//!   - `operation_status.rs:166 schema_version -> 1`: EQUIVALENT (the real value
//!     is `u64::from(SCHEMA_VERSION)` == 1), documented in that file's inline test.
//!
//! ROUND 2 (WP-D, the key-aware `encode_for_entry` surface from f84e5ad8) —
//! PROVES: the public `*StreamEnvelopeV1::encode_for_entry` wrappers and the
//! `read_delivery_stored` primitive behind them deliver the REAL committed
//! event — the exact canonical envelope bytes carrying the exact plaintext
//! payload plus its real coordinates/hashes — and a crypto-shredded event is
//! skipped as `Ok(None)` with the loud `syncbat::delivery` warn, never a
//! fabricated stored event, never a placeholder byte string, never ciphertext.
//! CATCHES: envelope.rs:71 / :450 `encode_for_entry -> Ok(None) /
//! Ok(Some(vec![])) / Ok(Some(vec![0])) / Ok(Some(vec![1]))`; envelope.rs:367
//! `ReceiptStreamEnvelopeV1::encode_for_entry -> Ok(None)`; envelope.rs:532
//! `read_delivery_stored -> Ok(None)` and its `Ok(Some(StoredEvent...))`
//! fabrications (that variant compiles only WITHOUT `payload-encryption`, so
//! the exact-envelope tests here stay feature-agnostic); envelope.rs:546
//! `warn_shredded_delivery -> ()` (killable only in `payload-encryption` lanes;
//! without the feature the shredded-skip path is statically dead).
//! SEEDED: tempfile-backed stores (plaintext + PerEntity-encrypted), fixed
//! coordinates and payload notes, and a thread-local tracing capture.

use std::sync::Arc;
use std::time::Duration;

use batpak::id::EntityIdType;
use batpak::prelude::*;
use batpak::store::{IndexEntry, Store, StoreConfig};
use flume::{bounded, Sender};
use syncbat::{
    CompositeSubscriptionRuntime, EntityStreamEnvelopeV1, EventStreamEnvelopeV1, ReceiptEnvelope,
    ReceiptOutcome, ReceiptSink, ReceiptStreamEnvelopeV1, RuntimeCursor, SessionControl,
    SessionDelivery, SessionError, SessionPoll, StoreReceiptSink, SubscriptionId,
    SubscriptionRegistry, SubscriptionRoute, SubscriptionRuntimeConfig, SubscriptionSession,
    SubscriptionSessionFactory, SubscriptionStore,
};

type DynErr = Box<dyn std::error::Error>;

// The session source-of-truth: these reasons must map to `malformed_stream_frame`,
// not the `cursor_invalid` fallback. The constant is not exported, so the literal
// is mirrored here (a drift would itself be a test failure).
const MALFORMED_STREAM_FRAME: &str = "malformed_stream_frame";
const CURSOR_INVALID: &str = "cursor_invalid";

const SUBSCRIPTION_ID: &str = "receipts.echo.v1";
const RECEIPT_KIND: &str = "receipt.echo.v1";
const WIRE_SCHEMA: &str = "batpak.receipt-stream-envelope.v1";
const OPERATION: &str = "mod.a.echo";
const RECEIPT_ENTITY: &str = "syncbat:receipt";
const RECEIPT_SCOPE: &str = "scope:test";

// Round-2 fixtures: a plaintext event with a distinctive payload note, so a
// fabricated `StoredEvent` (or placeholder envelope bytes) can never coincide.
const KILL_EVENT_KIND: EventKind = EventKind::custom(0x0A, 0x01);
const KILL_ENTITY: &str = "entity:envelope-kill";
const KILL_SCOPE: &str = "scope:envelope-kill";
const KILL_SUB: &str = "events.envelope-kill.v1";
const KILL_NOTE: &str = "round2-envelope-kill-note";

fn test_store() -> Result<(Arc<Store>, tempfile::TempDir), DynErr> {
    let dir = tempfile::TempDir::new()?;
    let store = Store::open(
        StoreConfig::new(dir.path())
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false)
            .with_sync_every_n_events(1),
    )?;
    Ok((Arc::new(store), dir))
}

fn registry() -> Result<SubscriptionRegistry, DynErr> {
    let mut registry = SubscriptionRegistry::new();
    registry.insert(
        SubscriptionId::new(SUBSCRIPTION_ID)?,
        SubscriptionRoute::ReceiptStream {
            receipt_kind: RECEIPT_KIND.to_owned(),
            wire_payload_schema_ref: WIRE_SCHEMA.to_owned(),
            inner_receipt_schema_ref: None,
            backpressure_capacity: None,
        },
    )?;
    Ok(registry)
}

fn append(store: Arc<Store>) -> Result<(), DynErr> {
    let coord = Coordinate::new(RECEIPT_ENTITY, RECEIPT_SCOPE)?;
    let sink = StoreReceiptSink::new(store, coord);
    let envelope =
        ReceiptEnvelope::from_descriptor(OPERATION, RECEIPT_KIND, ReceiptOutcome::Completed);
    sink.record_receipt(&envelope)
        .map_err(|error| -> DynErr { Box::new(error) })?;
    Ok(())
}

/// Fetch the single committed index entry for `entity`, failing loudly if the
/// fixture ever holds anything other than exactly one event.
fn sole_entry(store: &Store, entity: &str) -> Result<IndexEntry, DynErr> {
    let mut entries = store.by_entity(entity);
    if entries.len() != 1 {
        return Err(std::io::Error::other(format!(
            "expected exactly one index entry for `{entity}`, got {}",
            entries.len()
        ))
        .into());
    }
    entries
        .pop()
        .ok_or_else(|| -> DynErr { std::io::Error::other("entry vanished after len check").into() })
}

fn append_kill_event(store: &Store) -> Result<(), DynErr> {
    let coord = Coordinate::new(KILL_ENTITY, KILL_SCOPE)?;
    let _receipt = store
        .append(
            &coord,
            KILL_EVENT_KIND,
            &serde_json::json!({ "note": KILL_NOTE }),
        )
        .map_err(|error| -> DynErr { Box::new(error) })?;
    Ok(())
}

/// Optional prev/event hash pair mirrored from the stored event's hash chain.
fn stored_hash_parts(stored: &StoredEvent<Vec<u8>>) -> (Option<[u8; 32]>, Option<[u8; 32]>) {
    (
        stored
            .event
            .hash_chain
            .as_ref()
            .map(|chain| chain.prev_hash),
        stored
            .event
            .hash_chain
            .as_ref()
            .map(|chain| chain.event_hash),
    )
}

/// Ground the plaintext fixture: the raw stored payload really is OUR note
/// (so a fabricated stored event or placeholder bytes can never coincide).
fn assert_stored_is_kill_note(stored: &StoredEvent<Vec<u8>>) -> Result<(), DynErr> {
    let payload_json: serde_json::Value = batpak::encoding::from_bytes(&stored.event.payload)?;
    assert_eq!(
        payload_json.get("note").and_then(serde_json::Value::as_str),
        Some(KILL_NOTE),
        "fixture ground truth: the stored payload must be the appended plaintext note"
    );
    assert_ne!(
        stored.event.header.content_hash, [0_u8; 32],
        "fixture ground truth: a real committed event has a real content hash"
    );
    Ok(())
}

fn open(
    store: Arc<Store>,
) -> Result<(Box<dyn SubscriptionSession>, Sender<SessionControl>), DynErr> {
    let (control_tx, control_rx) = bounded(8);
    let runtime = CompositeSubscriptionRuntime::new(
        SubscriptionStore::new(store),
        registry()?,
        SubscriptionRuntimeConfig::default(),
    );
    let session = runtime.open_session(SUBSCRIPTION_ID, None, 128, control_rx)?;
    Ok((session, control_tx))
}

fn collect(
    session: &mut dyn SubscriptionSession,
    max_steps: usize,
) -> Result<Vec<SessionDelivery>, DynErr> {
    let mut out = Vec::new();
    for _ in 0..max_steps {
        match session.poll(Duration::from_millis(250))? {
            SessionPoll::Delivery(delivery) => {
                let done = matches!(
                    delivery,
                    SessionDelivery::Error(_) | SessionDelivery::End(_)
                );
                out.push(delivery);
                if done {
                    break;
                }
            }
            SessionPoll::Blocked | SessionPoll::Ended => break,
        }
    }
    Ok(out)
}

fn first_event(deliveries: &[SessionDelivery]) -> Option<(u64, RuntimeCursor)> {
    deliveries.iter().find_map(|delivery| match delivery {
        SessionDelivery::Event(event) => Some((event.delivery_index, event.cursor_after.clone())),
        SessionDelivery::Watermark(_) | SessionDelivery::Error(_) | SessionDelivery::End(_) => None,
    })
}

fn first_error(deliveries: &[SessionDelivery]) -> Option<SessionError> {
    deliveries.iter().find_map(|delivery| match delivery {
        SessionDelivery::Error(error) => Some(error.clone()),
        SessionDelivery::Event(_) | SessionDelivery::Watermark(_) | SessionDelivery::End(_) => None,
    })
}

fn send_ack(
    control_tx: &Sender<SessionControl>,
    delivery_index: u64,
    cursor: RuntimeCursor,
) -> Result<(), DynErr> {
    control_tx
        .send(SessionControl::Ack {
            delivery_index,
            cursor,
        })
        .map_err(|_| -> DynErr { std::io::Error::other("ack send failed").into() })
}

fn message_of(error: &SessionError) -> String {
    String::from_utf8_lossy(&error.message).into_owned()
}

/// An out-of-range ACK index is one of the two reasons in the deleted arm; its
/// terminal error must classify as `malformed_stream_frame`. With the arm gone
/// it would fall to `cursor_invalid`.
#[test]
fn ack_index_out_of_range_uses_malformed_stream_frame_code() -> Result<(), DynErr> {
    let (store, _dir) = test_store()?;
    append(Arc::clone(&store))?;
    let (mut session, control_tx) = open(Arc::clone(&store))?;
    let (_, cursor) = first_event(&collect(session.as_mut(), 8)?)
        .ok_or_else(|| -> DynErr { std::io::Error::other("expected event").into() })?;

    send_ack(&control_tx, 999, cursor)?;
    let error = first_error(&collect(session.as_mut(), 8)?)
        .ok_or_else(|| -> DynErr { std::io::Error::other("expected ack error").into() })?;

    assert_eq!(
        message_of(&error),
        "ack delivery index out of range",
        "wrong terminal reason"
    );
    assert_eq!(
        error.code, MALFORMED_STREAM_FRAME,
        "out-of-range ACK must be a malformed-frame fault, not cursor_invalid"
    );
    assert_ne!(error.code, CURSOR_INVALID);
    Ok(())
}

/// A valid index whose cursor does not match the one the session sent is the
/// other reason in the deleted arm; it must also classify as
/// `malformed_stream_frame`. We mismatch the cursor by acking index 1 with a
/// second, route-valid-but-different sent cursor.
#[test]
fn ack_cursor_mismatch_uses_malformed_stream_frame_code() -> Result<(), DynErr> {
    let (store, _dir) = test_store()?;
    append(Arc::clone(&store))?;
    append(Arc::clone(&store))?;
    let (mut session, control_tx) = open(Arc::clone(&store))?;

    let deliveries = collect(session.as_mut(), 8)?;
    let mut events = deliveries.iter().filter_map(|delivery| match delivery {
        SessionDelivery::Event(event) => Some((event.delivery_index, event.cursor_after.clone())),
        SessionDelivery::Watermark(_) | SessionDelivery::Error(_) | SessionDelivery::End(_) => None,
    });
    let (first_index, _first_cursor) = events
        .next()
        .ok_or_else(|| -> DynErr { std::io::Error::other("expected first event").into() })?;
    let (_second_index, second_cursor) = events
        .next()
        .ok_or_else(|| -> DynErr { std::io::Error::other("expected second event").into() })?;

    // ACK the first delivery index but hand over the (decodable, route-valid)
    // cursor that belongs to the second delivery: the sent-cursor check fails.
    send_ack(&control_tx, first_index, second_cursor)?;
    let error = first_error(&collect(session.as_mut(), 8)?)
        .ok_or_else(|| -> DynErr { std::io::Error::other("expected ack error").into() })?;

    assert_eq!(
        message_of(&error),
        "ack cursor does not match sent cursor",
        "wrong terminal reason"
    );
    assert_eq!(
        error.code, MALFORMED_STREAM_FRAME,
        "cursor-mismatch ACK must be a malformed-frame fault, not cursor_invalid"
    );
    assert_ne!(error.code, CURSOR_INVALID);
    Ok(())
}

// ─── Round 2: key-aware `encode_for_entry` + `read_delivery_stored` ─────────

/// envelope.rs:71 (`EventStreamEnvelopeV1::encode_for_entry`) and, in the
/// non-encryption build, envelope.rs:532 (`read_delivery_stored`): a readable
/// committed event must encode to `Some` of the EXACT canonical envelope bytes
/// built from the REAL stored event — never `None`, never `vec![]`/`[0]`/`[1]`,
/// never a fabricated `StoredEvent`.
#[test]
fn event_stream_encode_for_entry_returns_exact_real_envelope_bytes() -> Result<(), DynErr> {
    let (store, _dir) = test_store()?;
    append_kill_event(&store)?;
    let entry = sole_entry(&store, KILL_ENTITY)?;
    assert_eq!(
        entry.event_kind(),
        KILL_EVENT_KIND,
        "fixture kind survives commit"
    );
    let stored = store.read_raw(entry.event_id())?;
    assert_stored_is_kill_note(&stored)?;

    let (prev_hash, event_hash) = stored_hash_parts(&stored);
    let expected = EventStreamEnvelopeV1 {
        schema_version: 1,
        subscription_id: KILL_SUB.to_owned(),
        event_id: entry.event_id().as_u128(),
        correlation_id: entry.correlation_id(),
        causation_id: entry.causation_id(),
        entity: KILL_ENTITY.to_owned(),
        scope: KILL_SCOPE.to_owned(),
        event_kind_raw: KILL_EVENT_KIND.as_raw_u16(),
        event_category: KILL_EVENT_KIND.category(),
        payload_version: stored.event.header.payload_version,
        timestamp_us: stored.event.header.timestamp_us,
        hlc_wall_ms: entry.wall_ms(),
        global_sequence: entry.global_sequence(),
        content_hash: stored.event.header.content_hash,
        prev_hash,
        event_hash,
        inner_event_payload_schema_ref: None,
        payload: stored.event.payload.clone(),
    };
    let expected_bytes = batpak::canonical::to_bytes(&expected)?;
    assert!(
        expected_bytes.len() > 1,
        "a real canonical envelope is never empty / one placeholder byte"
    );

    let encoded = EventStreamEnvelopeV1::encode_for_entry(&store, KILL_SUB, &entry, None)?;
    assert_eq!(
        encoded,
        Some(expected_bytes),
        "encode_for_entry must return the exact canonical envelope for the real stored event"
    );
    Ok(())
}

/// envelope.rs:450 (`EntityStreamEnvelopeV1::encode_for_entry`) and, in the
/// non-encryption build, envelope.rs:532: same exact-bytes doctrine as the
/// event-stream wrapper, on the entity-stream envelope type.
#[test]
fn entity_stream_encode_for_entry_returns_exact_real_envelope_bytes() -> Result<(), DynErr> {
    let (store, _dir) = test_store()?;
    append_kill_event(&store)?;
    let entry = sole_entry(&store, KILL_ENTITY)?;
    assert_eq!(
        entry.event_kind(),
        KILL_EVENT_KIND,
        "fixture kind survives commit"
    );
    let stored = store.read_raw(entry.event_id())?;
    assert_stored_is_kill_note(&stored)?;

    let (prev_hash, event_hash) = stored_hash_parts(&stored);
    let expected = EntityStreamEnvelopeV1 {
        schema_version: 1,
        subscription_id: KILL_SUB.to_owned(),
        event_id: entry.event_id().as_u128(),
        correlation_id: entry.correlation_id(),
        causation_id: entry.causation_id(),
        entity: KILL_ENTITY.to_owned(),
        scope: KILL_SCOPE.to_owned(),
        event_kind_raw: KILL_EVENT_KIND.as_raw_u16(),
        event_category: KILL_EVENT_KIND.category(),
        payload_version: stored.event.header.payload_version,
        timestamp_us: stored.event.header.timestamp_us,
        hlc_wall_ms: entry.wall_ms(),
        global_sequence: entry.global_sequence(),
        content_hash: stored.event.header.content_hash,
        prev_hash,
        event_hash,
        inner_event_payload_schema_ref: None,
        payload: stored.event.payload.clone(),
    };
    let expected_bytes = batpak::canonical::to_bytes(&expected)?;
    assert!(
        expected_bytes.len() > 1,
        "a real canonical envelope is never empty / one placeholder byte"
    );

    let encoded = EntityStreamEnvelopeV1::encode_for_entry(&store, KILL_SUB, &entry, None)?;
    assert_eq!(
        encoded,
        Some(expected_bytes),
        "encode_for_entry must return the exact canonical envelope for the real stored event"
    );
    Ok(())
}

/// envelope.rs:367 (`ReceiptStreamEnvelopeV1::encode_for_entry -> Ok(None)`):
/// a readable committed receipt must encode to `Some` of the exact typed
/// envelope AND its exact canonical bytes, built from the real stored receipt.
#[test]
fn receipt_stream_encode_for_entry_returns_exact_real_receipt_envelope() -> Result<(), DynErr> {
    let (store, _dir) = test_store()?;
    append(Arc::clone(&store))?;
    let entry = sole_entry(&store, RECEIPT_ENTITY)?;
    let stored = store.read_raw(entry.event_id())?;

    // Ground truth: the stored payload decodes to OUR completed receipt.
    let receipt: ReceiptEnvelope = batpak::canonical::from_bytes(&stored.event.payload)?;
    assert_eq!(
        receipt.descriptor_name, OPERATION,
        "real receipt descriptor"
    );
    assert_eq!(
        receipt.outcome.class(),
        "completed",
        "real receipt outcome class"
    );

    let receipt_bytes = batpak::canonical::to_bytes(&receipt)?;
    let receipt_hash = *blake3::hash(&receipt_bytes).as_bytes();
    let (prev_hash, event_hash) = stored_hash_parts(&stored);
    let expected = ReceiptStreamEnvelopeV1 {
        schema_version: 1,
        subscription_id: SUBSCRIPTION_ID.to_owned(),
        receipt_kind: RECEIPT_KIND.to_owned(),
        descriptor_name: OPERATION.to_owned(),
        outcome_class: "completed".to_owned(),
        event_id: entry.event_id().as_u128(),
        correlation_id: entry.correlation_id(),
        causation_id: entry.causation_id(),
        entity: RECEIPT_ENTITY.to_owned(),
        scope: RECEIPT_SCOPE.to_owned(),
        payload_version: stored.event.header.payload_version,
        timestamp_us: stored.event.header.timestamp_us,
        hlc_wall_ms: entry.wall_ms(),
        global_sequence: entry.global_sequence(),
        content_hash: stored.event.header.content_hash,
        prev_hash,
        event_hash,
        inner_receipt_schema_ref: None,
        receipt_hash,
        receipt: receipt_bytes,
    };
    let expected_bytes = batpak::canonical::to_bytes(&expected)?;

    let encoded = ReceiptStreamEnvelopeV1::encode_for_entry(
        &store,
        SUBSCRIPTION_ID,
        RECEIPT_KIND,
        &entry,
        None,
    )?;
    assert_eq!(
        encoded,
        Some((expected, expected_bytes)),
        "encode_for_entry must return the exact typed envelope + canonical bytes for the real \
         stored receipt, never the Ok(None) skip for a readable event"
    );
    Ok(())
}

/// Crypto-shred half of the round-2 doctrine (`payload-encryption` builds; the
/// shredded-skip path is statically dead without the feature).
#[cfg(feature = "payload-encryption")]
mod round2_crypto_shred {
    use std::collections::BTreeMap;
    use std::fmt;
    use std::sync::{Arc, Mutex};

    use batpak::id::EntityIdType;
    use batpak::prelude::*;
    use batpak::store::{KeyScopeGranularity, ShredScope};
    use flume::bounded;
    use syncbat::{
        CompositeSubscriptionRuntime, EntityStreamEnvelopeV1, EventStreamEnvelopeV1,
        SessionDelivery, SubscriptionId, SubscriptionRegistry, SubscriptionRoute,
        SubscriptionRuntimeConfig, SubscriptionSessionFactory, SubscriptionStore,
    };

    use super::{collect, sole_entry, DynErr};

    // Category 0x0A is a user (non-reserved) category, so its events ARE encrypted.
    const SECRET_KIND: EventKind = EventKind::custom(0x0A, 0x01);
    const SECRET_CATEGORY: u8 = 0x0A;
    const SECRET_SUB: &str = "secrets.round2.v1";
    const DOOMED_ENTITY: &str = "entity:doomed-r2";
    const LIVE_ENTITY: &str = "entity:live-r2";
    const VAULT_SCOPE: &str = "scope:vault-r2";
    const SECRET_NOTE: &str = "round2-secret-note";
    const LIVE_NOTE: &str = "round2-live-note";
    const WIRE: &str = "batpak.stream-envelope.v1";
    // Mirror of the envelope.rs `warn_shredded_delivery` message (a drift is
    // itself a test failure — the warn is the LOUD half of the skip contract).
    const SHRED_WARN_MESSAGE: &str = "skipping a crypto-shredded event during subscription \
                                      delivery; it is not delivered and the cursor advances past \
                                      it (payload key destroyed — plaintext gone)";
    const SHRED_WARN_TARGET: &str = "syncbat::delivery";

    fn open_encrypted() -> Result<(Arc<Store>, tempfile::TempDir), DynErr> {
        let dir = tempfile::TempDir::new()?;
        let store = Store::open(
            StoreConfig::new(dir.path())
                .with_payload_encryption(KeyScopeGranularity::PerEntity)
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false)
                .with_sync_every_n_events(1),
        )?;
        Ok((Arc::new(store), dir))
    }

    fn append_secret(store: &Store, entity: &str, note: &str) -> Result<(), DynErr> {
        let coord = Coordinate::new(entity, VAULT_SCOPE)?;
        let _receipt = store
            .append(&coord, SECRET_KIND, &serde_json::json!({ "note": note }))
            .map_err(|error| -> DynErr { Box::new(error) })?;
        Ok(())
    }

    /// envelope.rs:71 / :450: pre-shred an encrypted event encodes `Some` with
    /// the DECRYPTED plaintext (never the stored ciphertext); post-shred both
    /// raw-event wrappers yield exactly `Ok(None)` — the skip, not a
    /// `Some(vec![...])` placeholder and not ciphertext.
    #[test]
    fn encode_for_entry_delivers_plaintext_then_none_after_shred() -> Result<(), DynErr> {
        let (store, _dir) = open_encrypted()?;
        append_secret(&store, DOOMED_ENTITY, SECRET_NOTE)?;
        let entry = sole_entry(&store, DOOMED_ENTITY)?;
        let ciphertext = store.read_raw(entry.event_id())?.event.payload;

        let encoded = EventStreamEnvelopeV1::encode_for_entry(&store, SECRET_SUB, &entry, None)?
            .ok_or_else(|| -> DynErr {
                std::io::Error::other("a readable encrypted event must encode Some").into()
            })?;
        let envelope: EventStreamEnvelopeV1 = batpak::canonical::from_bytes(&encoded)?;
        let payload_json: serde_json::Value = batpak::encoding::from_bytes(&envelope.payload)?;
        assert_eq!(
            payload_json.get("note").and_then(serde_json::Value::as_str),
            Some(SECRET_NOTE),
            "the delivered envelope must carry the decrypted plaintext payload"
        );
        assert_ne!(
            envelope.payload, ciphertext,
            "the delivered payload must never be the stored ciphertext"
        );
        assert_eq!(envelope.event_id, entry.event_id().as_u128());

        let doomed = Coordinate::new(DOOMED_ENTITY, VAULT_SCOPE)?;
        assert!(
            store
                .shred_scope(ShredScope::Entity(&doomed))
                .map_err(|error| -> DynErr { Box::new(error) })?,
            "shredding the doomed entity must report key destruction"
        );
        assert_eq!(
            EventStreamEnvelopeV1::encode_for_entry(&store, SECRET_SUB, &entry, None)?,
            None,
            "a crypto-shredded event must be skipped as Ok(None), never placeholder bytes"
        );
        assert_eq!(
            EntityStreamEnvelopeV1::encode_for_entry(&store, SECRET_SUB, &entry, None)?,
            None,
            "a crypto-shredded event must be skipped as Ok(None), never placeholder bytes"
        );
        Ok(())
    }

    /// Thread-local capture of `syncbat::delivery` WARN events: field name →
    /// rendered value. The raw-event sessions deliver inline on the polling
    /// thread, so `with_default` sees the warn.
    #[derive(Clone, Default)]
    struct WarnCapture {
        warns: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
    }

    struct FieldGrab<'map>(&'map mut BTreeMap<String, String>);

    impl tracing::field::Visit for FieldGrab<'_> {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.insert(field.name().to_owned(), value.to_owned());
        }

        fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
            self.0.insert(field.name().to_owned(), value.to_string());
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            self.0.insert(field.name().to_owned(), format!("{value:?}"));
        }
    }

    impl tracing::Subscriber for WarnCapture {
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            metadata.target() == SHRED_WARN_TARGET
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() != tracing::Level::WARN {
                return;
            }
            let mut fields = BTreeMap::new();
            event.record(&mut FieldGrab(&mut fields));
            self.warns
                .lock()
                .expect("warn-capture mutex poisoned")
                .push(fields);
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}
    }

    fn category_registry() -> Result<SubscriptionRegistry, DynErr> {
        let mut registry = SubscriptionRegistry::new();
        registry.insert(
            SubscriptionId::new(SECRET_SUB)
                .map_err(|error| std::io::Error::other(format!("sub id: {error}")))?,
            SubscriptionRoute::EventCategory {
                category: SECRET_CATEGORY,
                wire_payload_schema_ref: WIRE.to_owned(),
                inner_event_payload_schema_ref: None,
                backpressure_capacity: None,
            },
        )?;
        Ok(registry)
    }

    /// envelope.rs:546 (`warn_shredded_delivery -> ()`): the shredded skip is
    /// LOUD — exactly one `syncbat::delivery` WARN with the exact stream,
    /// subscription id, shredded event id, and message — while the live event
    /// still delivers its plaintext. Deleting the warn body silences the skip.
    #[test]
    fn shredded_skip_emits_exact_loud_delivery_warn() -> Result<(), DynErr> {
        let (store, _dir) = open_encrypted()?;
        append_secret(&store, DOOMED_ENTITY, SECRET_NOTE)?;
        append_secret(&store, LIVE_ENTITY, LIVE_NOTE)?;
        let doomed_id = sole_entry(&store, DOOMED_ENTITY)?.event_id().as_u128();

        let doomed = Coordinate::new(DOOMED_ENTITY, VAULT_SCOPE)?;
        assert!(
            store
                .shred_scope(ShredScope::Entity(&doomed))
                .map_err(|error| -> DynErr { Box::new(error) })?,
            "shredding the doomed entity must report key destruction"
        );

        let capture = WarnCapture::default();
        let warns_handle = Arc::clone(&capture.warns);
        let (_control_tx, control_rx) = bounded(4);
        let deliveries = tracing::subscriber::with_default(capture, || -> Result<_, DynErr> {
            let runtime = CompositeSubscriptionRuntime::new(
                SubscriptionStore::new(Arc::clone(&store)),
                category_registry()?,
                SubscriptionRuntimeConfig::default(),
            );
            let mut session = runtime.open_session(SECRET_SUB, None, 128, control_rx)?;
            collect(session.as_mut(), 12)
        })?;

        // The skip stays coherent: only the live event is delivered, decrypted.
        let mut notes = Vec::new();
        for delivery in &deliveries {
            match delivery {
                SessionDelivery::Event(event) => {
                    let envelope: EventStreamEnvelopeV1 =
                        batpak::canonical::from_bytes(&event.envelope_bytes)?;
                    let payload_json: serde_json::Value =
                        batpak::encoding::from_bytes(&envelope.payload)?;
                    let note = payload_json
                        .get("note")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("<missing>");
                    notes.push(note.to_owned());
                }
                SessionDelivery::Error(error) => {
                    return Err(
                        std::io::Error::other(format!("delivery faulted: {error:?}")).into(),
                    );
                }
                SessionDelivery::Watermark(_) | SessionDelivery::End(_) => {}
            }
        }
        assert_eq!(
            notes,
            vec![LIVE_NOTE.to_owned()],
            "only the live event delivers; the shredded head is skipped"
        );

        let warns = warns_handle
            .lock()
            .expect("warn-capture mutex poisoned")
            .clone();
        assert_eq!(
            warns.len(),
            1,
            "the shredded skip must emit exactly one loud delivery warn"
        );
        let warn = warns
            .first()
            .ok_or_else(|| -> DynErr { std::io::Error::other("warn vanished").into() })?;
        let doomed_id_text = doomed_id.to_string();
        assert_eq!(warn.get("stream").map(String::as_str), Some("event_stream"));
        assert_eq!(
            warn.get("subscription_id").map(String::as_str),
            Some(SECRET_SUB)
        );
        assert_eq!(
            warn.get("event_id").map(String::as_str),
            Some(doomed_id_text.as_str()),
            "the warn must name the shredded event"
        );
        assert_eq!(
            warn.get("message").map(String::as_str),
            Some(SHRED_WARN_MESSAGE),
            "the warn must carry the exact loud-skip message"
        );
        Ok(())
    }
}

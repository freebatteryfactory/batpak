//! PROVES: `ValidatingEffectBackend` fails closed on every host-mediated append
//!         whose event kind is unbound or whose payload does not satisfy the
//!         bound schema, and reaches the inner backend ONLY after both checks
//!         pass — on both the typed (`append_typed_event`) and raw
//!         (`append_event`) axes (#26).
//! CATCHES: a regression that "opens the gate" — delegating to the inner backend
//!          before the binding lookup or the schema validation, which would let
//!          an unbound kind or a non-canonical payload reach durable storage.
//! SEEDED: a spy inner backend that records which method it reached and fails
//!         with a recognizable sentinel, so a delegated call is provable without
//!         constructing an `AppendReceipt` (which requires a live store).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use batpak::coordinate::Coordinate;
use batpak::event::{EventKind, EventPayload};
use batpak::store::AppendReceipt;
use serde::{Deserialize, Serialize};
use syncbat::effect_backend::{EffectBackend, EffectError, TypedEffectEvent};

use crate::schema::{
    GoldenVector, SchemaDescriptor, SchemaId, SchemaRegistry, SchemaRole, SchemaVersion,
};
use crate::validating_effect_backend::ValidatingEffectBackend;

const BOUND_KIND: EventKind = EventKind::custom(0xF, 3);
const SCHEMA_REF: &str = "evt.payload.v1";
/// Sentinel the spy inner returns so a delegated (validation-passed) call is
/// distinguishable from a fail-closed refusal without a real `AppendReceipt`.
const INNER_SENTINEL: &str = "spy-inner-reached";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct AuditPayload {
    sequence: u64,
}

impl EventPayload for AuditPayload {
    const KIND: EventKind = BOUND_KIND;
}

/// Inner backend that records which append method it reached (via a shared
/// `Rc<Cell<bool>>`, so a clone handed to the wrapper mutates a flag the test
/// still holds) and always fails closed with [`INNER_SENTINEL`].
#[derive(Clone, Default)]
struct SpyBackend {
    event_reached: Rc<Cell<bool>>,
    typed_reached: Rc<Cell<bool>>,
}

impl EffectBackend for SpyBackend {
    fn append_event(&mut self, _kind: EventKind, _payload: &[u8]) -> Result<(), EffectError> {
        self.event_reached.set(true);
        Err(EffectError::new(INNER_SENTINEL))
    }

    fn append_typed_event<'event>(
        &mut self,
        _coordinate: &Coordinate,
        _event: TypedEffectEvent<'event>,
    ) -> Result<AppendReceipt, EffectError> {
        self.typed_reached.set(true);
        Err(EffectError::new(INNER_SENTINEL))
    }
}

fn coordinate() -> Coordinate {
    Coordinate::new("audit:stream", "scope:test").expect("coordinate")
}

fn canonical_payload_bytes() -> Vec<u8> {
    batpak::canonical::to_bytes(&AuditPayload { sequence: 1 }).expect("payload encodes canonically")
}

/// A registry whose only descriptor binds [`SCHEMA_REF`] as an event payload.
fn registry() -> SchemaRegistry {
    let descriptor = SchemaDescriptor::new(
        SchemaId::new(SCHEMA_REF).expect("schema id"),
        SchemaVersion(1),
        SchemaRole::EventPayload,
        vec![GoldenVector::new("c", canonical_payload_bytes())],
    )
    .expect("descriptor");
    SchemaRegistry::from_descriptors([descriptor])
}

/// Bind [`BOUND_KIND`] to the registered schema ref.
fn bindings() -> BTreeMap<u16, String> {
    let mut bindings = BTreeMap::new();
    bindings.insert(BOUND_KIND.as_raw_u16(), SCHEMA_REF.to_owned());
    bindings
}

fn typed_event(spy: &SpyBackend, bindings: BTreeMap<u16, String>) -> ValidatingEffectBackend {
    ValidatingEffectBackend::new(Box::new(spy.clone()), bindings, registry())
}

#[test]
fn append_typed_event_fails_closed_when_kind_is_unbound() {
    let spy = SpyBackend::default();
    let mut backend = typed_event(&spy, BTreeMap::new());
    let payload = AuditPayload { sequence: 1 };
    let event = TypedEffectEvent::new(&payload).expect("typed event encodes");

    let error = backend
        .append_typed_event(&coordinate(), event)
        .expect_err("an unbound kind must fail closed");

    assert!(
        error.message().contains("no payload schema binding"),
        "expected an unbound-binding refusal, got: {}",
        error.message()
    );
    assert!(
        !spy.typed_reached.get(),
        "the inner backend must not be reached when the kind is unbound"
    );
}

#[test]
fn append_typed_event_fails_closed_when_schema_is_unregistered() {
    let spy = SpyBackend::default();
    let mut binding = BTreeMap::new();
    binding.insert(BOUND_KIND.as_raw_u16(), "missing.schema.v1".to_owned());
    let mut backend = typed_event(&spy, binding);
    let payload = AuditPayload { sequence: 1 };
    let event = TypedEffectEvent::new(&payload).expect("typed event encodes");

    let error = backend
        .append_typed_event(&coordinate(), event)
        .expect_err("a binding to an unregistered schema must fail closed");

    assert!(
        error.message().contains("payload schema validation failed"),
        "expected a schema-validation refusal, got: {}",
        error.message()
    );
    assert!(
        !spy.typed_reached.get(),
        "the inner backend must not be reached when schema validation fails"
    );
}

#[test]
fn append_typed_event_reaches_inner_after_validation_passes() {
    let spy = SpyBackend::default();
    let mut backend = typed_event(&spy, bindings());
    let payload = AuditPayload { sequence: 1 };
    let event = TypedEffectEvent::new(&payload).expect("typed event encodes");

    let error = backend
        .append_typed_event(&coordinate(), event)
        .expect_err("the spy inner returns the sentinel");

    assert_eq!(
        error.message(),
        INNER_SENTINEL,
        "a bound kind with a valid payload must pass validation and delegate to the inner backend"
    );
    assert!(
        spy.typed_reached.get(),
        "the inner backend must be reached after both checks pass"
    );
}

#[test]
fn append_event_fails_closed_when_kind_is_unbound() {
    let spy = SpyBackend::default();
    let mut backend = typed_event(&spy, BTreeMap::new());

    let error = backend
        .append_event(BOUND_KIND, &canonical_payload_bytes())
        .expect_err("an unbound kind must fail closed");

    assert!(
        error.message().contains("no payload schema binding"),
        "expected an unbound-binding refusal, got: {}",
        error.message()
    );
    assert!(
        !spy.event_reached.get(),
        "the inner backend must not be reached when the kind is unbound"
    );
}

#[test]
fn append_event_fails_closed_on_non_canonical_payload() {
    let spy = SpyBackend::default();
    let mut backend = typed_event(&spy, bindings());
    // Three unterminated container openings — an incomplete value under both the
    // CBOR and msgpack readings — so the canonical decode fails regardless of
    // the underlying codec.
    let non_canonical = vec![0x9f, 0x9f, 0x9f];

    let error = backend
        .append_event(BOUND_KIND, &non_canonical)
        .expect_err("a non-canonical payload must fail closed");

    assert!(
        error.message().contains("payload schema validation failed"),
        "expected a schema-validation refusal, got: {}",
        error.message()
    );
    assert!(
        !spy.event_reached.get(),
        "the inner backend must not be reached when the payload is non-canonical"
    );
}

#[test]
fn append_event_reaches_inner_after_validation_passes() {
    let spy = SpyBackend::default();
    let mut backend = typed_event(&spy, bindings());

    let error = backend
        .append_event(BOUND_KIND, &canonical_payload_bytes())
        .expect_err("the spy inner returns the sentinel");

    assert_eq!(
        error.message(),
        INNER_SENTINEL,
        "a bound kind with a canonical payload must pass validation and delegate to the inner backend"
    );
    assert!(
        spy.event_reached.get(),
        "the inner backend must be reached after both checks pass"
    );
}

//! The runtime-owned capability backend that makes operation effects real.
//!
//! This is what turns the effect row from a declaration into an enforced
//! boundary. An operation reaches durable effects ONLY through a `Ctx`-owned
//! capability handle, and every handle performs its effect through this backend.
//! Because the runtime owns the backend and hands it out only per-invocation
//! through `Ctx`, a handler cannot append an event (or touch any other declared
//! effect) the runtime did not mediate — so the observed effect row is
//! authoritative, not cooperative. A handler with no backend bound simply cannot
//! perform the effect; the attempt is a typed error.

use batpak::coordinate::Coordinate;
use batpak::event::{EventKind, EventPayload};
use batpak::store::{AppendReceipt, Open, Store, StoreError};

type StoreTypedAppend<'event> =
    Box<dyn FnOnce(&Store<Open>, &Coordinate) -> Result<AppendReceipt, StoreError> + 'event>;

/// A typed event append request that remains object-safe at the backend seam.
///
/// The carrier is built from an [`EventPayload`] so a store-backed backend can
/// persist it through `Store::append_typed`, while validating wrappers can still
/// inspect the canonical payload bytes before the append is performed.
pub struct TypedEffectEvent<'event> {
    kind: EventKind,
    payload_version: u16,
    payload_bytes: Vec<u8>,
    append_to_store: StoreTypedAppend<'event>,
}

impl<'event> TypedEffectEvent<'event> {
    /// Build a typed effect event from an [`EventPayload`].
    ///
    /// # Errors
    /// Returns [`EffectError`] when the payload cannot be canonically encoded.
    pub fn new<T: EventPayload>(payload: &'event T) -> Result<Self, EffectError> {
        let payload_bytes = batpak::canonical::to_bytes(payload).map_err(|error| {
            EffectError::new(format!("typed event payload encoding failed: {error}"))
        })?;
        Ok(Self {
            kind: T::KIND,
            payload_version: T::PAYLOAD_VERSION,
            payload_bytes,
            append_to_store: Box::new(move |store, coordinate| {
                store.append_typed(coordinate, payload)
            }),
        })
    }

    /// The event kind derived from the typed payload.
    #[must_use]
    pub const fn kind(&self) -> EventKind {
        self.kind
    }

    /// The payload schema version derived from the typed payload.
    #[must_use]
    pub const fn payload_version(&self) -> u16 {
        self.payload_version
    }

    /// Canonical payload bytes for validation before persistence.
    #[must_use]
    pub fn payload_bytes(&self) -> &[u8] {
        &self.payload_bytes
    }

    pub(crate) fn append_to_store(
        self,
        store: &Store<Open>,
        coordinate: &Coordinate,
    ) -> Result<AppendReceipt, StoreError> {
        (self.append_to_store)(store, coordinate)
    }
}

/// Durable-effect backend the runtime injects into each invocation context.
///
/// Implementations are store-backed (see `StoreEffectBackend`). The typed append
/// method carries a type-erased [`TypedEffectEvent`] so the trait remains usable
/// behind `Box<dyn EffectBackend>`.
pub trait EffectBackend {
    /// Append one event of `kind` carrying `payload` (already canonically
    /// encoded) to the runtime's durable store.
    ///
    /// # Errors
    /// Returns [`EffectError`] when the backend cannot perform the append.
    fn append_event(&mut self, kind: EventKind, payload: &[u8]) -> Result<(), EffectError>;

    /// Append one typed event at `coordinate` and return the store receipt.
    ///
    /// # Errors
    /// Returns [`EffectError`] when the backend cannot perform typed event
    /// persistence.
    fn append_typed_event<'event>(
        &mut self,
        _coordinate: &Coordinate,
        _event: TypedEffectEvent<'event>,
    ) -> Result<AppendReceipt, EffectError> {
        Err(EffectError::new(
            "typed event appends are not supported by this effect backend",
        ))
    }

    /// Mediate one declared event-category read for this invocation.
    ///
    /// # Errors
    /// Returns [`EffectError`] when this backend does not support event reads or
    /// rejects the read.
    fn read_event(&mut self, event_category: &str) -> Result<(), EffectError> {
        let _ = event_category;
        Err(EffectError::new(
            "event reads are not supported by this effect backend",
        ))
    }

    /// Mediate one declared projection query for this invocation.
    ///
    /// # Errors
    /// Returns [`EffectError`] when this backend does not support projection
    /// queries or rejects the query.
    fn query_projection(&mut self, projection_id: &str) -> Result<(), EffectError> {
        let _ = projection_id;
        Err(EffectError::new(
            "projection queries are not supported by this effect backend",
        ))
    }

    /// Mediate one declared receipt emission for this invocation.
    ///
    /// # Errors
    /// Returns [`EffectError`] when this backend does not support receipt
    /// emission or rejects the emission.
    fn emit_receipt(&mut self, receipt_kind: &str) -> Result<(), EffectError> {
        let _ = receipt_kind;
        Err(EffectError::new(
            "receipt emission is not supported by this effect backend",
        ))
    }

    /// Mediate one declared host-control use, identified by `control`, for this
    /// invocation.
    ///
    /// # Errors
    /// Returns [`EffectError`] when this backend does not support host controls
    /// or rejects the use.
    fn use_host_control(&mut self, control: &str) -> Result<(), EffectError> {
        let _ = control;
        Err(EffectError::new(
            "host controls are not supported by this effect backend",
        ))
    }
}

/// Failure performing a durable effect through an [`EffectBackend`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectError {
    message: String,
}

impl EffectError {
    /// Construct an effect error with a human-readable message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The failure message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "effect backend failure: {}", self.message)
    }
}

impl std::error::Error for EffectError {}

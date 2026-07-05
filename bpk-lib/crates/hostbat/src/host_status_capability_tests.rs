//! HostBuilder passthroughs for operation-status sinks and capability grants.
//!
//! PROVES: a `HostModule` mounted via [`HostBuilder`] behaves identically to a
//! bare `syncbat::CoreBuilder` for the two composition hooks the builder
//! forwards — `status_sink` (lifecycle facts for mounted operations) and
//! `grant_capability` / `grant_capabilities` (declared-authority admission).
//! CATCHES: a passthrough that silently drops the hook during lowering, which
//! would leave mounted operations without lifecycle facts or unconditionally
//! denied.

use std::sync::{Arc, Mutex};

use syncbat::{
    EffectClass, Handler, HandlerResult, OperationDescriptor, OperationEffectRow,
    OperationStatusFactV1, OperationStatusLifecycle, OperationStatusSink, OperationStatusSinkError,
    RuntimeError,
};

use crate::module::HostModule;
use crate::schema::{GoldenVector, SchemaDescriptor, SchemaId, SchemaRole, SchemaVersion};
use crate::HostBuilder;

const REQUIRED_CAP: &str = "texo.cap.model";
const UNRELATED_CAP: &str = "texo.cap.other";

fn canonical_bytes(value: &str) -> Vec<u8> {
    batpak::canonical::to_bytes(&value).expect("canonical fixture encodes")
}

struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&mut self, input: &[u8], _cx: &mut syncbat::Ctx<'_>) -> HandlerResult {
        Ok(input.to_vec())
    }
}

fn echo_descriptor(row: OperationEffectRow) -> OperationDescriptor {
    OperationDescriptor::new(
        "status.echo",
        EffectClass::Inspect,
        "schema.in.v1",
        "schema.out.v1",
        "receipt.v1",
    )
    .with_effect_row(row)
}

fn schema_with_role(id: &str, role: SchemaRole, bytes: &[u8]) -> SchemaDescriptor {
    SchemaDescriptor::new(
        SchemaId::new(id).expect("id"),
        SchemaVersion(1),
        role,
        vec![GoldenVector::new("c", bytes.to_vec())],
    )
    .expect("descriptor")
}

fn echo_module(row: OperationEffectRow) -> HostModule {
    HostModule::builder("mod.status", 1)
        .operation(echo_descriptor(row), EchoHandler)
        .expect("operation")
        .schema(schema_with_role(
            "schema.in.v1",
            SchemaRole::OperationInput,
            &canonical_bytes("default-in"),
        ))
        .expect("input schema")
        .schema(schema_with_role(
            "schema.out.v1",
            SchemaRole::OperationOutput,
            &canonical_bytes("default-out"),
        ))
        .expect("output schema")
        .schema(schema_with_role(
            "receipt.v1",
            SchemaRole::ReceiptPayload,
            &canonical_bytes("default-receipt"),
        ))
        .expect("receipt schema")
        .build()
        .expect("module")
}

type FactLog = Arc<Mutex<Vec<OperationStatusFactV1>>>;

#[derive(Clone, Default)]
struct RecordingStatusSink {
    facts: FactLog,
}

impl OperationStatusSink for RecordingStatusSink {
    fn record_fact(&self, fact: &OperationStatusFactV1) -> Result<(), OperationStatusSinkError> {
        self.facts.lock().expect("facts lock").push(fact.clone());
        Ok(())
    }
}

#[test]
fn status_sink_receives_started_and_terminal_facts_through_host_builder() {
    let sink = RecordingStatusSink::default();
    let facts = Arc::clone(&sink.facts);
    let mut host = HostBuilder::new()
        .mount(echo_module(OperationEffectRow::new()))
        .expect("mount")
        .status_sink(sink)
        .build()
        .expect("build");

    let payload = b"status-proof".to_vec();
    let result = host.invoke("status.echo", payload.clone()).expect("invoke");
    assert_eq!(result.output(), payload.as_slice());

    let facts = facts.lock().expect("facts lock");
    assert_eq!(
        facts.len(),
        2,
        "one started and one terminal fact, got {facts:?}"
    );
    assert_eq!(facts[0].operation, "status.echo");
    assert!(
        matches!(facts[0].lifecycle, OperationStatusLifecycle::Started),
        "first fact must be Started, got {facts:?}"
    );
    assert_eq!(facts[1].operation, "status.echo");
    assert!(
        matches!(facts[1].lifecycle, OperationStatusLifecycle::Completed),
        "second fact must be Completed, got {facts:?}"
    );
}

#[test]
fn capability_requiring_operation_is_denied_without_a_grant(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = HostBuilder::new()
        .mount(echo_module(
            OperationEffectRow::new().requires_capability(REQUIRED_CAP),
        ))
        .expect("mount")
        .build()
        .expect("build");

    let err = match host.invoke("status.echo", b"payload".to_vec()) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: an operation requiring an ungranted capability must be denied",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(
            err,
            RuntimeError::Denied { ref name, ref code, .. }
                if name == "status.echo" && code == "capability.denied"
        ),
        "expected a capability.denied denial, got {err:?}"
    );
    Ok(())
}

#[test]
fn grant_capability_admits_the_same_operation() {
    let mut host = HostBuilder::new()
        .mount(echo_module(
            OperationEffectRow::new().requires_capability(REQUIRED_CAP),
        ))
        .expect("mount")
        .grant_capability(REQUIRED_CAP)
        .build()
        .expect("build");

    let payload = b"granted".to_vec();
    let result = host
        .invoke("status.echo", payload.clone())
        .expect("granted invoke succeeds");
    assert_eq!(result.output(), payload.as_slice());
}

#[test]
fn grant_capabilities_admits_the_same_operation() {
    let mut host = HostBuilder::new()
        .mount(echo_module(
            OperationEffectRow::new().requires_capability(REQUIRED_CAP),
        ))
        .expect("mount")
        .grant_capabilities([REQUIRED_CAP, UNRELATED_CAP])
        .build()
        .expect("build");

    let payload = b"granted-iter".to_vec();
    let result = host
        .invoke("status.echo", payload.clone())
        .expect("granted invoke succeeds");
    assert_eq!(result.output(), payload.as_slice());
}

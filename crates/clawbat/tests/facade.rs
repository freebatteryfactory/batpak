#![allow(clippy::panic)]

use downstream-kit as cb;

#[cb::operation(
    descriptor = ECHO,
    register = register_echo,
    name = "claw.echo",
    effect = Compute,
    input_schema = "schema.claw.echo.input.v1",
    output_schema = "schema.claw.echo.output.v1",
    receipt_kind = "receipt.claw.echo.v1",
    title = "Claw Echo"
)]
fn echo(input: &[u8], cx: &mut syncbat::Cx<'_>) -> syncbat::HandlerResult {
    assert_eq!(cx.descriptor().name(), "claw.echo");
    let mut output = b"cb:".to_vec();
    output.extend_from_slice(input);
    Ok(output)
}

#[test]
fn cb_operation_macro_generates_syncbat_descriptor() {
    assert_eq!(ECHO.name, "claw.echo");
    assert_eq!(ECHO.title, Some("Claw Echo"));
    assert_eq!(ECHO.effect, cb::EffectClass::Compute);
    assert_eq!(ECHO.input_schema_ref, "schema.claw.echo.input.v1");
    assert_eq!(ECHO.output_schema_ref, "schema.claw.echo.output.v1");
    assert_eq!(ECHO.receipt_kind, "receipt.claw.echo.v1");
}

#[test]
fn generated_registration_invokes_through_syncbat_core() {
    let mut builder = syncbat::Core::builder();
    register_echo(&mut builder).expect("register generated cb operation");
    let mut core = builder.build().expect("syncbat core builds");

    let result = core
        .invoke("claw.echo", b"hello".to_vec())
        .expect("syncbat invokes cb operation");

    assert_eq!(result.descriptor().name(), "claw.echo");
    assert_eq!(result.output().as_slice(), b"cb:hello");
    assert!(result.recorded_receipt().is_none());
}

#[test]
fn cb_vocabulary_maps_to_syncbat_without_runtime_ownership() {
    let pass = cb::PassRef::new("pass.local.validate").expect("valid pass ref");
    let capability =
        cb::CapabilityRef::new("capability.store:append").expect("valid capability ref");

    let descriptor: cb::OperationDescriptor = syncbat::OperationDescriptor::new(
        "claw.vocab",
        cb::EffectClass::Inspect,
        pass.as_str(),
        capability.as_str(),
        "receipt.claw.vocab.v1",
    );
    let envelope = cb::ReceiptEnvelope::new(&descriptor, cb::ReceiptOutcome::Completed);

    assert_eq!(descriptor.name(), "claw.vocab");
    assert_eq!(descriptor.effect, syncbat::EffectClass::Inspect);
    assert_eq!(envelope.descriptor_name, "claw.vocab");
    assert_eq!(envelope.outcome, syncbat::ReceiptOutcome::Completed);
}

#[test]
fn cb_refs_reject_invalid_values() {
    assert!(matches!(cb::PassRef::new(""), Err(cb::RefError::Empty)));
    assert!(matches!(
        cb::CapabilityRef::new("capability with space"),
        Err(cb::RefError::InvalidByte {
            index: 10,
            byte: b' '
        })
    ));
}

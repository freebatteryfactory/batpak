// Lifted from syncbat/tests/operation_macro.rs (echo): register + register_item,
// no effect row => const descriptor.
#[operation(
    descriptor = ECHO,
    register = register_echo,
    register_item = echo_item,
    name = "echo",
    effect = Compute,
    input_schema = "schema.echo.input.v1",
    output_schema = "schema.echo.output.v1",
    receipt_kind = "receipt.echo.v1"
)]
fn echo(input: &[u8], cx: &mut syncbat::Ctx<'_>) -> syncbat::HandlerResult {
    Ok(input.to_vec())
}

// register + register_item present (Emit effect); no effect row.
#[operation(
    descriptor = REG,
    register = register_reg,
    register_item = reg_item,
    name = "reg",
    effect = Emit,
    input_schema = "schema.in.v1",
    output_schema = "schema.out.v1",
    receipt_kind = "receipt.v1"
)]
fn reg(input: &[u8], cx: &mut syncbat::Ctx<'_>) -> syncbat::HandlerResult {
    Ok(input.to_vec())
}

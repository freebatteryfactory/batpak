// Optional `title` => descriptor built via new_with_title(...); no register fns.
#[operation(
    descriptor = TITLED,
    name = "titled",
    title = "Human Title",
    effect = Inspect,
    input_schema = "schema.in.v1",
    output_schema = "schema.out.v1",
    receipt_kind = "receipt.v1"
)]
fn titled(input: &[u8], cx: &mut syncbat::Ctx<'_>) -> syncbat::HandlerResult {
    Ok(input.to_vec())
}

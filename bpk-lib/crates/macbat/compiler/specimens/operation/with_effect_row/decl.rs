// Effect row present (all six lanes) => LazyLock descriptor + .with_effect_row(...).
#[operation(
    descriptor = APPEND_ALL,
    register = register_append_all,
    name = "append.all",
    effect = Persist,
    input_schema = "schema.in.v1",
    output_schema = "schema.out.v1",
    receipt_kind = "receipt.v1",
    reads_events = ["evt.read"],
    appends_events = ["evt.append"],
    queries_projections = ["proj.query"],
    emits_receipts = ["receipt.emit"],
    uses_host_controls = ["ctrl.use"],
    requires_capabilities = ["cap.req"]
)]
fn append_all(input: &[u8], cx: &mut syncbat::Ctx<'_>) -> syncbat::HandlerResult {
    Ok(input.to_vec())
}

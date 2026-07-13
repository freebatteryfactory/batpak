use macbat::operation;
#[operation(
    descriptor = D,
    name = "n",
    effect = Compute,
    input_schema = "i",
    output_schema = "o",
    receipt_kind = "r",
    reads_events = "notarray"
)]
fn f(input: &[u8], cx: &mut u8) -> u8 {
    0
}
fn main() {}

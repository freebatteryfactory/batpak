use macbat::operation;
#[operation(
    descriptor = D,
    name = "a",
    name = "b",
    effect = Compute,
    input_schema = "i",
    output_schema = "o",
    receipt_kind = "r"
)]
fn f(input: &[u8], cx: &mut u8) -> u8 {
    0
}
fn main() {}

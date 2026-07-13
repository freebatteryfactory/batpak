use macbat::operation;
#[operation(
    descriptor = D,
    name = "n",
    effect = Compute,
    input_schema = "i",
    output_schema = "o",
    receipt_kind = "r"
)]
fn f<T>(input: &[u8], cx: &mut u8) -> u8 {
    0
}
fn main() {}

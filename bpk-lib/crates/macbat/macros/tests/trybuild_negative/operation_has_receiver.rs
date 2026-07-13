use macbat::operation;
#[operation(
    descriptor = D,
    name = "n",
    effect = Compute,
    input_schema = "i",
    output_schema = "o",
    receipt_kind = "r"
)]
fn f(&self, input: &[u8]) -> u8 {
    0
}
fn main() {}

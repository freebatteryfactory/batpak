use macbat::VariantInventory;
#[derive(VariantInventory)]
enum E {
    A,
    B(u8),
}
fn main() {}

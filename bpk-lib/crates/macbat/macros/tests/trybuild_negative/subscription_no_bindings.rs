use macbat::Subscription;
#[derive(Subscription)]
#[batpak(input = JsonValueInput, error = StoreError)]
struct S {
    a: u8,
}
fn main() {}

use macbat::Subscription;
#[derive(Subscription)]
#[batpak(input = JsonValueInput, error = StoreError)]
#[batpak(error = OtherError)]
#[batpak(event = Ev, handler = on_ev)]
struct S {
    a: u8,
}
fn main() {}

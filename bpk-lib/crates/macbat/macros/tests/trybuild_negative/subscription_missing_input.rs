use macbat::Subscription;
#[derive(Subscription)]
#[batpak(error = StoreError)]
#[batpak(event = Ev, handler = on_ev)]
struct S {
    a: u8,
}
fn main() {}

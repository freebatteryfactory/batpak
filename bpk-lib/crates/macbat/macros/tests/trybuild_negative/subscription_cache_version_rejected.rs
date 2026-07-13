use macbat::Subscription;
#[derive(Subscription)]
#[batpak(input = JsonValueInput, error = StoreError, cache_version = 1)]
#[batpak(event = Ev, handler = on_ev)]
struct S {
    a: u8,
}
fn main() {}

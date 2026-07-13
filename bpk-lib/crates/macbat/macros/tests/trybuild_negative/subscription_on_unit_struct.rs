use macbat::Subscription;
#[derive(Subscription)]
#[batpak(input = JsonValueInput, error = StoreError)]
#[batpak(event = Ev, handler = on_ev)]
struct S;
fn main() {}

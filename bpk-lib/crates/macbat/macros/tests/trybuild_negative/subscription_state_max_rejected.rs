use macbat::Subscription;
#[derive(Subscription)]
#[batpak(input = JsonValueInput, error = StoreError, state_max_cardinality = 1)]
#[batpak(event = Ev, handler = on_ev)]
struct S {
    a: u8,
}
fn main() {}

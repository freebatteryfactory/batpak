use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput, state_max_cardinality = 2)]
#[batpak(event = Ev, handler = on_ev)]
struct P {
    a: u8,
}
fn main() {}

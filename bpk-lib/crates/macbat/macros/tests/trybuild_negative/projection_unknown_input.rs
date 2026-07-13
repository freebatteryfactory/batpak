use macbat::Projection;
#[derive(Projection)]
#[batpak(inputt = JsonValueInput, event = Ev, handler = on_ev)]
struct P {
    a: u8,
}
fn main() {}

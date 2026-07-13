use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(event = Ev, handler = on_ev)]
enum P {
    A,
}
fn main() {}

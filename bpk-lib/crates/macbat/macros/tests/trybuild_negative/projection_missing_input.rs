use macbat::Projection;
#[derive(Projection)]
#[batpak(event = Ev, handler = on_ev)]
struct P {
    a: u8,
}
fn main() {}

use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(event = outer::Ev, handler = on_ev)]
struct P {
    a: u8,
}
fn main() {}

use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(event = Ev, handler = on_ev)]
struct P(u8);
fn main() {}

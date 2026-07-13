use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(input = RawMsgpackInput)]
#[batpak(event = Ev, handler = on_ev)]
struct P {
    a: u8,
}
fn main() {}

use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(event = Ev, handler = on_a)]
#[batpak(event = Ev, handler = on_b)]
struct P {
    a: u8,
}
fn main() {}

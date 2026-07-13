use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
struct P {
    a: u8,
}
fn main() {}

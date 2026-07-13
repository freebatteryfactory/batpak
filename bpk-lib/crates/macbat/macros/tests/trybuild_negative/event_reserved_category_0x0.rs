use macbat::Event;
#[derive(Event)]
#[batpak(category = 0x0, type_id = 1)]
struct E {
    a: u8,
}
fn main() {}

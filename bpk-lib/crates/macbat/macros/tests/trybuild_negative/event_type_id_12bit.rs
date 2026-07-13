use macbat::Event;
#[derive(Event)]
#[batpak(category = 1, type_id = 0x1000)]
struct E {
    a: u8,
}
fn main() {}

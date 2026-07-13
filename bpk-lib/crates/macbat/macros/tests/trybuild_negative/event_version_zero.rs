use macbat::Event;
#[derive(Event)]
#[batpak(category = 1, type_id = 1, version = 0)]
struct E {
    a: u8,
}
fn main() {}

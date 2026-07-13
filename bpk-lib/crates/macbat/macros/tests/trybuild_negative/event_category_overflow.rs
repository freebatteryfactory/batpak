use macbat::Event;
#[derive(Event)]
#[batpak(category = 256, type_id = 1)]
struct E {
    a: u8,
}
fn main() {}

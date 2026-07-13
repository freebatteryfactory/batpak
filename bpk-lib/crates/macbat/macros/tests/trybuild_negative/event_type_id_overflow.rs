use macbat::Event;
#[derive(Event)]
#[batpak(category = 1, type_id = 65536)]
struct E {
    a: u8,
}
fn main() {}

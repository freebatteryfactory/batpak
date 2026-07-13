use macbat::Event;
#[derive(Event)]
#[batpak(category = 1, type_id = 1)]
struct E(u8);
fn main() {}

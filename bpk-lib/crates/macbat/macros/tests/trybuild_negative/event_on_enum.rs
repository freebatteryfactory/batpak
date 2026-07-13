use macbat::Event;
#[derive(Event)]
#[batpak(category = 1, type_id = 1)]
enum E {
    A,
}
fn main() {}

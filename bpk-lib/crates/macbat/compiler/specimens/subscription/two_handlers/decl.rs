#[batpak(input = JsonValueInput, error = StoreError)]
#[batpak(event = First, handler = on_first)]
#[batpak(event = Second, handler = on_second)]
struct TwoHandlers {
    a: u32,
    b: u32,
}

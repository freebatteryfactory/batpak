// Two event/handler bindings; default cache_version, no state contract.
#[batpak(input = JsonValueInput)]
#[batpak(event = Opened, handler = on_opened)]
#[batpak(event = Closed, handler = on_closed)]
struct MultiBinding {
    open: u32,
    closed: u32,
}

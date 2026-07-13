// cache_version = 42 => schema_version() returns 42.
#[batpak(input = JsonValueInput, cache_version = 42)]
#[batpak(event = Recorded, handler = on_recorded)]
struct Cached {
    total: u64,
}

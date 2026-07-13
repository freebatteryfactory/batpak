// Lifted from examples/event_sourced_counter.rs: JsonValueInput, two bindings,
// cache_version = 0, state_max_cardinality = 1.
#[batpak(input = JsonValueInput, cache_version = 0, state_max_cardinality = 1)]
#[batpak(event = Incremented, handler = on_incremented)]
#[batpak(event = Decremented, handler = on_decremented)]
struct CounterState {
    value: i64,
    total_increments: u32,
    total_decrements: u32,
}

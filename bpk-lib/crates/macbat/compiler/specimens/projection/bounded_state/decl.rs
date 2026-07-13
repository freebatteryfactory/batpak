// state_max_cardinality = 1 present (the only legal cardinality) => StateContract emitted.
#[batpak(input = JsonValueInput, state_max_cardinality = 1)]
#[batpak(event = Ticked, handler = on_tick)]
struct BoundedState {
    ticks: u64,
}

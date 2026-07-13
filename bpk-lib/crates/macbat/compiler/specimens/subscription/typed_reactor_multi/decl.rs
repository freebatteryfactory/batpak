// Lifted from examples/typed_reactor_multi.rs: JsonValueInput + error = StoreError,
// two typed bindings.
#[batpak(input = JsonValueInput, error = StoreError)]
#[batpak(event = PayloadA, handler = on_a)]
#[batpak(event = PayloadB, handler = on_b)]
struct MultiReactor {
    seen: u32,
}

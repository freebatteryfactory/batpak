//! Compile-fail: the two-argument `define_entity_id!` form must not emit serde impls.

fn assert_serialize<T: serde::Serialize>(_value: &T) {}

fn main() {
    batpak::define_entity_id!(LocalOnlyId, "local-only");
    let id = LocalOnlyId::from(7u128);
    assert_serialize(&id);
}

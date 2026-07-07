//! Compile-fail coverage for `define_entity_id!` serde opt-in.
//!
//! The default two-argument macro form must not implement serde traits. Callers
//! opt in explicitly with `define_entity_id!(Name, "entity", serde)`.

#[test]
#[serial_test::file_serial(trybuild)]
fn non_opt_in_entity_id_does_not_implement_serde() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/entity_id_without_serde.rs");
}

//! Compile-fail coverage for the `#[derive(EventSourced)]` attribute contract.
//! Harness pattern: Fault-Injection Harness (compile-fail lane).
//!
//! PROVES: every `tests/ui/es_*` compile-fail fixture that violates the
//! `#[derive(EventSourced)]` contract fails to compile with a span-pointed,
//! wording-pinned error.
//! CATCHES: a derive that silently accepts an illegal `EventSourced` shape
//! (missing/unknown input, missing handler, duplicate event/input, wrong handler
//! signature, non-struct target, non-`u64` cache_version), or a regression in
//! error span/wording.
//! SEEDED: n/a — trybuild compile-fail corpus with deterministic `.stderr` goldens.
//!
//! Every fixture in `tests/ui/es_*.rs` violates a specific contract rule and
//! must fail to compile with a span-pointed error. The `.stderr` files pin
//! the exact error wording so regressions in message clarity or span quality
//! surface as trybuild diffs.

#[test]
#[serial_test::file_serial(trybuild)]
fn compile_fail_event_sourced_derive_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/es_missing_input.rs");
    t.compile_fail("tests/ui/es_unknown_input.rs");
    t.compile_fail("tests/ui/es_missing_handler.rs");
    t.compile_fail("tests/ui/es_duplicate_event.rs");
    t.compile_fail("tests/ui/es_handler_wrong_signature.rs");
    t.compile_fail("tests/ui/es_mixed_attr_keys.rs");
    t.compile_fail("tests/ui/es_duplicate_input.rs");
    t.compile_fail("tests/ui/es_cache_version_not_u64.rs");
    t.compile_fail("tests/ui/es_on_enum.rs");
    t.compile_fail("tests/ui/es_on_tuple_struct.rs");
    t.compile_fail("tests/ui/es_zero_bindings.rs");
    t.compile_fail("tests/ui/es_duplicate_event_qualified.rs");
}

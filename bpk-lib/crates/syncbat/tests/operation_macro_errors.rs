//! PROVES: INV-SYNCBAT-REGISTER-CATALOG-DETERMINISTIC
//! CATCHES: invalid operation macro inputs that would otherwise mint unstable runtime descriptors.
//! SEEDED: trybuild compile-fail fixtures.

// The `compile_fail_` prefix is a TIMEOUT CONTRACT, not style: nextest's ci and
// mutants profiles grant `test(compile_fail)` the 300s nested-build budget
// (.config/nextest.toml). Under the bare 60s default this trybuild run times
// out on a saturated mutation runner during the UNMUTATED BASELINE — which is
// exactly how the syncbat-subscription-runtime lane went red on run
// 28564535988 with zero surviving mutants.
#[test]
fn compile_fail_operation_macro_rejects_invalid_inputs() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/operation_macro_missing_name.rs");
    t.compile_fail("tests/ui/operation_macro_missing_descriptor.rs");
    t.compile_fail("tests/ui/operation_macro_unknown_key.rs");
    t.compile_fail("tests/ui/operation_macro_duplicate_key.rs");
    t.compile_fail("tests/ui/operation_macro_bad_effect.rs");
    t.compile_fail("tests/ui/operation_macro_async_fn.rs");
    t.compile_fail("tests/ui/operation_macro_generic_fn.rs");
    t.compile_fail("tests/ui/operation_macro_unsafe_fn.rs");
    t.compile_fail("tests/ui/operation_macro_non_rust_abi.rs");
    t.compile_fail("tests/ui/operation_macro_wrong_signature.rs");
}

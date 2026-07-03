//! Mutation-kill tests for the writer's single-append CAS guard.
//!
//! PROVES: `enforce_expected_sequence` is exact — with no committed entity the
//! actual sequence is 0, so `expected == 0` passes, any other expectation fails
//! with `SequenceMismatch` carrying the exact expected/actual/entity, and a
//! `None` expectation is a clean pass (CAS disabled).
//! CATCHES: the `actual != expected` comparison swapped to `==` (which would
//! pass on mismatch and fail on match); the `unwrap_or(0)` default; the
//! `if let Some(expected)` branch deletion / the whole-body `-> Ok(())`; and the
//! expected/actual fields of the `SequenceMismatch` error.

use super::WriterCore;
use crate::store::index::IndexEntry;
use crate::store::StoreError;

#[test]
fn enforce_expected_sequence_pins_cas_against_the_genesis_clock() {
    let no_latest: Option<&IndexEntry> = None;

    // CAS disabled: expected = None must pass regardless of state. The
    // `if let Some(expected)` branch / whole-body `-> Ok(())` collapse here.
    WriterCore::enforce_expected_sequence(no_latest, None, "entity:cas")
        .expect("PROPERTY: a None expectation disables the CAS check");

    // With no committed event the entity clock is 0 (`unwrap_or(0)`), so an
    // expectation of exactly 0 matches and passes — the `!=` boundary at
    // equality (an `==` mutant would spuriously reject here).
    WriterCore::enforce_expected_sequence(no_latest, Some(0), "entity:cas")
        .expect("PROPERTY: expected 0 matches the genesis clock 0");

    // Any non-zero expectation mismatches actual 0 and must fail closed with the
    // exact expected/actual/entity (an `==` mutant would spuriously pass here).
    let err = WriterCore::enforce_expected_sequence(no_latest, Some(7), "entity:cas")
        .expect_err("PROPERTY: a non-matching CAS expectation must fail");
    assert!(
        matches!(
            err,
            StoreError::SequenceMismatch { expected: 7, actual: 0, ref entity }
                if entity == "entity:cas"
        ),
        "PROPERTY: CAS mismatch must surface SequenceMismatch \
         {{ expected: 7, actual: 0, entity: \"entity:cas\" }}, got {err:?}"
    );
}

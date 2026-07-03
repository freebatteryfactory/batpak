//! Exact-contract tests for the fault-injection primitives (split out of
//! `fault.rs` to keep both files well under the size cap).
//!
//! PROVES: the injectors are a pinned, assertable contract — a
//! [`CountdownInjector`] set to fire after N matching checks is SILENT for the
//! first N-1 and returns the EXACT `StoreError::FaultInjected` on the Nth (and
//! stays fired thereafter); a filter excludes non-matching points from the
//! count entirely; a [`ProbabilisticInjector`] is deterministic at both
//! probability extremes; and `maybe_inject` propagates the injector's exact
//! error variant (and passes cleanly when no injector is installed).
//! CATCHES: the counting-logic operator swaps (`+`↔`*`/`-`, `<`↔`<=`/`==`/`>`),
//! the `!dominated` inversion, whole-body `-> None`, the probabilistic guard
//! `>=`↔`<` swap, and `maybe_inject -> Ok(())`.

use super::*;
use crate::store::StoreError;
use std::sync::Arc;

fn batch_item(idx: usize) -> InjectionPoint {
    InjectionPoint::BatchItemWritten {
        batch_id: 1,
        item_index: idx,
        total_items: 8,
    }
}

#[test]
fn countdown_fires_on_exactly_the_nth_matching_check_and_stays_fired() {
    // trigger_after = 3: checks 1 and 2 are silent, check 3 injects the
    // configured fault, and every later check keeps firing.
    let injector = CountdownInjector::new(3, CountdownAction::Fail("boom"));

    assert!(
        injector.check(batch_item(0)).is_none(),
        "1st matching check must be silent (count+1 = 1 < 3)"
    );
    assert!(
        injector.check(batch_item(1)).is_none(),
        "2nd matching check must be silent (count+1 = 2 < 3)"
    );

    let third = injector.check(batch_item(2));
    assert!(
        matches!(&third, Some(StoreError::FaultInjected(_))),
        "3rd matching check must inject FaultInjected (count+1 = 3, not < 3), got {third:?}"
    );
    if let Some(StoreError::FaultInjected(msg)) = &third {
        assert!(
            msg.contains("boom"),
            "the injected error must carry the configured message, got {msg:?}"
        );
    }

    assert!(
        matches!(
            injector.check(batch_item(3)),
            Some(StoreError::FaultInjected(_))
        ),
        "every check AFTER the trip point must keep firing"
    );
}

#[test]
fn countdown_filter_excludes_nonmatching_points_from_the_count() {
    // trigger_after = 1, filtered to BEGIN markers only.
    let injector = CountdownInjector::new(1, CountdownAction::Fail("boom"))
        .with_filter(|p| matches!(p, InjectionPoint::BatchBeginWritten { .. }));

    // Non-matching points are neither counted nor fired, however many arrive.
    assert!(
        injector.check(batch_item(0)).is_none(),
        "a non-matching point must be ignored, not counted"
    );
    assert!(
        injector.check(batch_item(1)).is_none(),
        "a non-matching point must never advance the count toward the trip"
    );

    let begin = InjectionPoint::BatchBeginWritten {
        batch_id: 1,
        item_count: 8,
    };
    assert!(
        matches!(injector.check(begin), Some(StoreError::FaultInjected(_))),
        "the FIRST matching point must fire (trigger_after = 1)"
    );
}

#[test]
fn probabilistic_extremes_are_exact_over_many_trials() {
    let point = InjectionPoint::BatchCommitWritten { batch_id: 7 };

    // p = 0.0: the guard `rng.f64() >= 0.0` is ALWAYS true, so it returns before
    // ever injecting. Swapping `>=` to `<` (`f64() < 0.0`, always false) would
    // start injecting here.
    let never = ProbabilisticInjector::new(0.0, CountdownAction::Fail("boom"));
    for trial in 0..256 {
        assert!(
            never.check(point.clone()).is_none(),
            "p=0.0 must NEVER inject (trial {trial})"
        );
    }

    // p = 1.0: `rng.f64()` is in [0.0, 1.0), so `>= 1.0` is always false and it
    // always injects. Whole-body `-> None` or the `!dominated` inversion would
    // silence it here.
    let always = ProbabilisticInjector::new(1.0, CountdownAction::Fail("boom"));
    for trial in 0..256 {
        assert!(
            matches!(
                always.check(point.clone()),
                Some(StoreError::FaultInjected(_))
            ),
            "p=1.0 must ALWAYS inject the configured fault (trial {trial})"
        );
    }
}

#[test]
fn maybe_inject_returns_the_exact_injected_error_and_passes_when_absent() {
    let injector: Arc<dyn FaultInjector> =
        Arc::new(CountdownInjector::new(1, CountdownAction::Fail("boom")));
    let point = InjectionPoint::BatchStart {
        batch_id: 9,
        item_count: 2,
    };

    let err =
        maybe_inject(point, &Some(injector)).expect_err("maybe_inject must propagate the fault");
    assert!(
        matches!(err, StoreError::FaultInjected(_)),
        "maybe_inject must return the injector's exact FaultInjected variant, got {err:?}"
    );

    // With no injector installed, maybe_inject is a clean pass.
    let absent: Option<Arc<dyn FaultInjector>> = None;
    maybe_inject(InjectionPoint::MmapIndexLoad, &absent)
        .expect("maybe_inject must pass cleanly when no injector is installed");
}

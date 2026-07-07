//! # outcome_saga
//!
//! **Witnesses:** the [`batpak::outcome`] saga algebra can model a multi-step
//! saga that SUSPENDS on an external condition and later RESUMES to a resolved
//! terminal. It exercises the public surface directly:
//!
//! - [`Outcome::and_then`] to sequence saga steps (the monad bind),
//! - [`Outcome::Pending`] + [`WaitCondition::Event`] + a resume token to suspend,
//! - [`Outcome::is_pending`] / [`Outcome::is_terminal`] to inspect saga state,
//! - [`CompensationAction`] to name the undo plan a failing branch would carry,
//! - [`zip`] and [`join_all`] to combine independent sub-outcomes, and to show
//!   that a suspended leaf PROPAGATES through a join (the whole saga suspends).
//!
//! The saga is expressed in mechanisms, not meanings: an abstract two-phase
//! pipeline (`acquire` then `commit`) whose commit waits on an external event.
//!
//! Run: `cargo run -p batpak-examples --bin outcome_saga`

use std::io::Write;

use batpak::outcome::{join_all, zip, CompensationAction, Outcome, WaitCondition};

/// The saga's accumulated state: an ordered log of completed step names.
type SagaLog = Vec<&'static str>;

/// Opaque correlation token linking the suspended saga to the event that resumes
/// it. In a real engine this is persisted alongside the pending state.
const RESUME_TOKEN: u128 = 0x0000_0000_0000_0000_0000_0000_dead_beef;
/// The external event id the `commit` step waits for.
const AWAITED_EVENT: u128 = 0x0000_0000_0000_0000_0000_0000_0000_0042;

/// Step 1: acquire the resource. Always succeeds in this scenario.
fn acquire() -> Outcome<SagaLog> {
    Outcome::ok(vec!["acquire"])
}

/// Step 2: attempt to commit. The external side is not ready, so the saga
/// SUSPENDS: it returns `Pending`, carrying the condition and resume token that
/// a driver persists to resume later.
fn attempt_commit(mut log: SagaLog) -> Outcome<SagaLog> {
    log.push("commit-attempt");
    Outcome::pending(
        WaitCondition::Event {
            event_id: AWAITED_EVENT,
        },
        RESUME_TOKEN,
    )
}

/// Resume the suspended saga: the awaited event has arrived. If it matches the
/// condition, the commit completes; otherwise the saga stays suspended.
fn resume_commit(saved: SagaLog, arrived_event: u128) -> Outcome<SagaLog> {
    if arrived_event == AWAITED_EVENT {
        let mut log = saved;
        log.push("commit");
        Outcome::ok(log)
    } else {
        // Still waiting on the right event — remain suspended.
        Outcome::pending(
            WaitCondition::Event {
                event_id: AWAITED_EVENT,
            },
            RESUME_TOKEN,
        )
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();

    // -- Drive the saga forward until it suspends --------------------------
    // `and_then` is the bind: acquire, then attempt to commit.
    let suspended = acquire().and_then(attempt_commit);
    assert!(
        suspended.is_pending(),
        "the saga must suspend at the commit step",
    );
    assert!(
        !suspended.is_terminal(),
        "a suspended saga has not reached a terminal",
    );
    let _ = writeln!(out, "saga suspended: {suspended:?}");

    // A driver would persist (RESUME_TOKEN, saved state). We reconstruct the
    // saved log the driver checkpointed just before suspending.
    let saved_log: SagaLog = vec!["acquire", "commit-attempt"];

    // -- The awaited event arrives; resume to a resolved terminal ----------
    let resolved = resume_commit(saved_log, AWAITED_EVENT)
        // Chain a final step after the resumed commit.
        .and_then(|mut log| {
            log.push("finalize");
            Outcome::ok(log)
        });
    assert!(resolved.is_ok(), "the resumed saga must resolve Ok");
    assert!(resolved.is_terminal(), "Ok is a terminal outcome");
    let final_log = resolved.into_result()?;
    assert_eq!(
        final_log,
        vec!["acquire", "commit-attempt", "commit", "finalize"],
        "the saga log records every step in order",
    );
    let _ = writeln!(out, "saga resolved through: {}", final_log.join(" -> "));

    // -- Compensation vocabulary: the undo plan a failing branch would carry --
    let compensation = CompensationAction::Rollback {
        event_ids: vec![AWAITED_EVENT],
    };
    assert!(
        matches!(compensation, CompensationAction::Rollback { ref event_ids } if event_ids == &[AWAITED_EVENT]),
        "a compensating branch names a Rollback of the committed events",
    );
    let _ = writeln!(out, "compensation plan on failure: {compensation:?}");

    // -- Combinators: independent sub-outcomes fuse ------------------------
    let zipped = zip(Outcome::ok(1_u32), Outcome::ok(2_u32));
    assert_eq!(zipped, Outcome::ok((1, 2)), "zip fuses two Ok outcomes");

    let gathered = join_all(vec![Outcome::ok(10_u32), Outcome::ok(20), Outcome::ok(30)]);
    assert_eq!(
        gathered,
        Outcome::ok(vec![10, 20, 30]),
        "join_all collects a Vec of Ok into an Ok of Vec",
    );

    // A suspended leaf PROPAGATES through a join: the whole batch suspends,
    // which is exactly the saga-suspension semantics composing.
    let with_pending = join_all(vec![
        Outcome::ok(1_u32),
        Outcome::pending(
            WaitCondition::Event {
                event_id: AWAITED_EVENT,
            },
            RESUME_TOKEN,
        ),
    ]);
    assert!(
        with_pending.is_pending(),
        "one pending leaf suspends the whole join",
    );
    let _ = writeln!(
        out,
        "combinators: zip + join_all fuse Ok; a pending leaf suspends the join",
    );

    let _ = writeln!(out, "OK: outcome saga suspends and resumes to a terminal");
    Ok(())
}

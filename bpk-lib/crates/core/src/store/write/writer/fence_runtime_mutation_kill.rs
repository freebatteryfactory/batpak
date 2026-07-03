//! Mutation-kill tests for the fence ledger's running-max bookkeeping and the
//! `CommandResult` builder flags.
//!
//! PROVES: `record_publish_up_to` keeps the running MAX of both `publish_up_to`
//! and `frontier_point` across calls (a lower value never regresses it); and
//! each `CommandResult` builder sets exactly its own flag while leaving the rest
//! of the `immediate()` baseline untouched.
//! CATCHES: the `.max` on `publish_up_to` / `frontier_point` dropped or swapped
//! for the seed (`unwrap_or(0)` / `unwrap_or(ORIGIN)`); every builder field
//! assignment being no-op'd; and `break_after_reply_if`'s condition inversion.

use super::{CommandResult, DeferredReply, FenceLedger};
use crate::store::stats::HlcPoint;
use crate::store::StoreError;

fn point(n: u64) -> HlcPoint {
    HlcPoint {
        wall_ms: n,
        global_sequence: n,
    }
}

#[test]
fn record_publish_up_to_keeps_the_running_max() {
    let mut ledger = FenceLedger::new(1);
    assert_eq!(
        ledger.publish_up_to, None,
        "a fresh ledger has no publish point"
    );
    assert_eq!(
        ledger.frontier_point, None,
        "a fresh ledger has no frontier point"
    );

    ledger.record_publish_up_to(5, point(5));
    assert_eq!(ledger.publish_up_to, Some(5));
    assert_eq!(ledger.frontier_point, Some(point(5)));

    // A LOWER value must not regress the running max.
    ledger.record_publish_up_to(3, point(3));
    assert_eq!(
        ledger.publish_up_to,
        Some(5),
        "PROPERTY: publish_up_to keeps the max; a lower value must not regress it"
    );
    assert_eq!(
        ledger.frontier_point,
        Some(point(5)),
        "PROPERTY: frontier_point keeps the max; a lower value must not regress it"
    );

    // A HIGHER value advances both.
    ledger.record_publish_up_to(9, point(9));
    assert_eq!(
        ledger.publish_up_to,
        Some(9),
        "PROPERTY: a higher publish_up_to advances the running max"
    );
    assert_eq!(
        ledger.frontier_point,
        Some(point(9)),
        "PROPERTY: a higher frontier_point advances the running max"
    );
}

#[test]
fn command_result_builders_each_set_their_own_flag() {
    // Baseline: immediate(delta) carries the delta and clears every flag.
    let base = CommandResult::immediate(3);
    assert_eq!(base.sync_event_delta, 3, "immediate carries the sync delta");
    assert!(
        !base.break_after_reply,
        "baseline break_after_reply is clear"
    );
    assert!(
        !base.must_sync_before_continue,
        "baseline must_sync_before_continue is clear"
    );
    assert!(!base.exit_writer, "baseline exit_writer is clear");
    assert!(
        !base.enter_group_commit_drain,
        "baseline enter_group_commit_drain is clear"
    );
    assert!(
        base.shutdown_drain_respond.is_none(),
        "baseline has no shutdown drain responder"
    );
    assert!(
        matches!(base.deferred_reply, DeferredReply::None),
        "baseline deferred reply is None"
    );

    assert!(
        CommandResult::immediate(0)
            .break_after_reply()
            .break_after_reply,
        "break_after_reply must set the flag"
    );
    // `break_after_reply_if` must gate on its argument, not invert it.
    assert!(
        CommandResult::immediate(0)
            .break_after_reply_if(true)
            .break_after_reply,
        "break_after_reply_if(true) sets the flag"
    );
    assert!(
        !CommandResult::immediate(0)
            .break_after_reply_if(false)
            .break_after_reply,
        "break_after_reply_if(false) leaves the flag clear"
    );
    assert!(
        CommandResult::immediate(0).exit_writer().exit_writer,
        "exit_writer must set the flag"
    );
    assert!(
        CommandResult::immediate(0)
            .enter_group_commit_drain()
            .enter_group_commit_drain,
        "enter_group_commit_drain must set the flag"
    );

    let synced = CommandResult::immediate(0).with_sync(DeferredReply::None);
    assert!(
        synced.must_sync_before_continue,
        "with_sync must arm the pre-continue sync"
    );

    let (tx, _rx) = flume::bounded::<Result<(), StoreError>>(1);
    let draining = CommandResult::immediate(0).enter_shutdown_drain(tx);
    assert!(
        draining.exit_writer,
        "enter_shutdown_drain must exit the writer"
    );
    assert!(
        draining.shutdown_drain_respond.is_some(),
        "enter_shutdown_drain must record the drain responder"
    );
}

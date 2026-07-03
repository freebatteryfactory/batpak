//! Mutation-kill tests for the append-ticket `try_check` poll seam.
//!
//! PROVES: `Ticket::try_check` maps a live-but-empty channel to `None`, a
//! delivered reply to `Some(value)` verbatim, and a disconnected writer to
//! `Some(Err(WriterCrashed))`.
//! CATCHES: each `try_recv` match arm — `Empty -> Some`, the delivered
//! `Ok(value) -> None`, `Disconnected -> None`, and the `WriterCrashed`
//! substitution on disconnect.

use super::{AppendReply, AppendTicket};
use crate::store::StoreError;

fn ticket(rx: flume::Receiver<AppendReply>) -> AppendTicket {
    // The `new` signature grows a cooperative-pump argument under
    // `dangerous-test-hooks`; mirror the store-bridge feature split so this
    // compiles on every lane.
    #[cfg(feature = "dangerous-test-hooks")]
    {
        AppendTicket::new(rx, None)
    }
    #[cfg(not(feature = "dangerous-test-hooks"))]
    {
        AppendTicket::new(rx)
    }
}

#[test]
fn try_check_reports_empty_value_and_disconnect_distinctly() {
    // Empty (writer alive, nothing delivered) -> None. An `Empty -> Some`
    // mutant would fabricate a reply here.
    let (tx, rx) = flume::bounded::<AppendReply>(1);
    let pending = ticket(rx);
    assert!(
        pending.try_check().is_none(),
        "PROPERTY: an empty, still-connected ticket polls to None"
    );

    // A delivered reply -> Some(value), passed through verbatim (NOT the
    // disconnect's WriterCrashed). A distinct, cheap unit error variant isolates
    // the value-passthrough arm from the disconnect arm.
    tx.send(Err(StoreError::VisibilityFenceActive))
        .expect("deliver a reply into the ticket channel");
    let delivered = pending
        .try_check()
        .expect("PROPERTY: a delivered reply must poll to Some");
    assert!(
        matches!(delivered, Err(StoreError::VisibilityFenceActive)),
        "PROPERTY: try_check passes the delivered value through unchanged, got {delivered:?}"
    );

    // Disconnected writer (sender dropped, nothing pending) -> Some(Err(
    // WriterCrashed)). A `Disconnected -> None` mutant would hang the poller;
    // any other substituted error would fail the variant match.
    let (dead_tx, dead_rx) = flume::bounded::<AppendReply>(1);
    drop(dead_tx);
    let crashed = ticket(dead_rx);
    let outcome = crashed
        .try_check()
        .expect("PROPERTY: a disconnected ticket must poll to Some");
    assert!(
        matches!(outcome, Err(StoreError::WriterCrashed)),
        "PROPERTY: a disconnected writer must surface WriterCrashed, got {outcome:?}"
    );
}

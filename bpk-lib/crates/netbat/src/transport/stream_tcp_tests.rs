//! Unit tests for private NETBAT/2 stream-TCP helpers. The public `serve_*`
//! entry points are exercised end-to-end in
//! `tests/mutation_kill_netbat-transport.rs`, `tests/subscription_concurrency.rs`,
//! and the `stream_runtime_*` integration tests; these cover the small pure
//! helpers and the control-reader loop without TCP timing.
//!
//! Linked from `stream_tcp.rs` via `#[cfg(test)] #[path] mod tests;` so the
//! production module stays within the absolute file-size cap.

use super::runtime_fault::{
    map_runtime_error, runtime_failure_code, INTERNAL_ERROR_CODE, MALFORMED_STREAM_FRAME_CODE,
};
use super::*;
use std::io::Cursor;
use syncbat::SessionEventDelivery;

fn cursor(byte: u8) -> RuntimeCursor {
    RuntimeCursor::from_bytes(vec![byte])
}

fn token() -> SubscriptionToken {
    SubscriptionToken::new("orders.open.v1", &Limits::default()).expect("token")
}

#[test]
fn maybe_cursor_bytes_maps_present_and_absent() {
    // KILLS stream_tcp.rs maybe_cursor_bytes (-> None / Some(vec![0]) /
    // Some(vec![1]) / Some(vec![])). Present must yield the exact wrapped bytes;
    // Absent must yield None.
    assert_eq!(maybe_cursor_bytes(MaybeCursor::Absent), None);
    assert_eq!(
        maybe_cursor_bytes(MaybeCursor::Present(CursorBytes::new(vec![7, 9, 3]))),
        Some(vec![7, 9, 3])
    );
}

#[test]
fn classify_accept_error_maps_each_kind() {
    // KILLS the accept-loop classification at the listener `accept()` site
    // (formerly the WouldBlock/Interrupted match guards): WouldBlock ->
    // retry-after-sleep, Interrupted (EINTR) -> retry-immediately, every other
    // kind -> fatal. A real TcpListener cannot be coerced into returning
    // Interrupted or an arbitrary fatal kind on demand, so this pure classifier
    // is the only deterministic seam. Asserting all three distinct outcomes
    // kills any constant-return or arm-swap mutation.
    assert_eq!(
        classify_accept_error(io::ErrorKind::WouldBlock),
        AcceptError::WouldBlock
    );
    assert_eq!(
        classify_accept_error(io::ErrorKind::Interrupted),
        AcceptError::Interrupted
    );
    assert_eq!(
        classify_accept_error(io::ErrorKind::BrokenPipe),
        AcceptError::Fatal
    );
    assert_eq!(
        classify_accept_error(io::ErrorKind::ConnectionReset),
        AcceptError::Fatal
    );
}

#[test]
fn timeout_kind_classifies_block_and_timeout_only() {
    // KILLS timeout_kind (-> false / true). WouldBlock and TimedOut are timeout
    // kinds; BrokenPipe is not.
    assert!(timeout_kind(io::ErrorKind::WouldBlock));
    assert!(timeout_kind(io::ErrorKind::TimedOut));
    assert!(!timeout_kind(io::ErrorKind::BrokenPipe));
}

#[test]
fn terminal_delivery_is_true_only_for_error_and_end() {
    // KILLS terminal_delivery (-> false). End is terminal; an Event is not.
    let end = SessionDelivery::End(SessionEnd {
        subscription_id: "orders.open.v1".to_owned(),
        reason_code: "stream.complete",
        cursor_after: None,
    });
    let event = SessionDelivery::Event(SessionEventDelivery {
        subscription_id: "orders.open.v1".to_owned(),
        delivery_index: 1,
        cursor_before: cursor(1),
        cursor_after: cursor(2),
        wire_payload_schema_ref: "hostbat.event.orders.v1".to_owned(),
        envelope_bytes: vec![0],
    });
    assert!(terminal_delivery(&end));
    assert!(!terminal_delivery(&event));
}

#[test]
fn merge_stats_sums_each_field() {
    // KILLS each `+=` in merge_stats (-> `*=`/`-=`) and the merge_stats -> ()
    // body-drop. Distinct nonzero source values mean a dropped or
    // multiplied/subtracted merge cannot reproduce the sums. Includes
    // worker_panics, the new concurrent-dispatch fault counter.
    let mut total = TcpSubscriptionServeStats::default();
    let connection = TcpSubscriptionServeStats {
        served_subscriptions: 2,
        failed_subscriptions: 3,
        malformed_pre_subscribe: 4,
        runtime_failures: 5,
        connection_io_failures: 6,
        worker_panics: 7,
        ..Default::default()
    };
    merge_stats(&mut total, connection);
    assert_eq!(total.served_subscriptions, 2);
    assert_eq!(total.failed_subscriptions, 3);
    assert_eq!(total.malformed_pre_subscribe, 4);
    assert_eq!(total.runtime_failures, 5);
    assert_eq!(total.connection_io_failures, 6);
    assert_eq!(total.worker_panics, 7);
}

const CANCEL_LINE: &[u8] = b"NETBAT/2 SUB_CANCEL orders.open.v1 client.cancel\n";

fn run_loop(reader: &mut impl Read, stop: &AtomicBool) -> Vec<SessionControl> {
    let (tx, rx) = flume::bounded(16);
    let limits = Limits::default();
    let id = token();
    let _ = read_control_loop(reader, &tx, &limits, &id, stop);
    drop(tx);
    rx.try_iter().collect()
}

#[test]
fn read_control_loop_matching_cancel_emits_cancel() {
    // KILLS the SUB_CANCEL id comparison (`!=` -> `==`). A SUB_CANCEL whose id
    // MATCHES the session must forward Cancel; under the inverted comparison a
    // matching id would be reported Malformed instead.
    let mut reader = Cursor::new(CANCEL_LINE.to_vec());
    let stop = AtomicBool::new(false);
    let got = run_loop(&mut reader, &stop);
    assert!(
        matches!(got.first(), Some(SessionControl::Cancel)),
        "expected Cancel, got {got:?}"
    );
}

/// Returns WouldBlock once, then replays `rest`.
struct WouldBlockThen {
    fired: bool,
    rest: Cursor<Vec<u8>>,
}

impl Read for WouldBlockThen {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.fired {
            self.fired = true;
            return Err(io::Error::from(io::ErrorKind::WouldBlock));
        }
        self.rest.read(buf)
    }
}

#[test]
fn read_control_loop_retries_after_timeout_then_reads_frame() {
    // KILLS the timeout_kind retry guard (-> false). A WouldBlock must be retried
    // so the following SUB_CANCEL is read and forwarded as Cancel; with the guard
    // false the WouldBlock is treated as a disconnect and Cancel never arrives.
    let mut reader = WouldBlockThen {
        fired: false,
        rest: Cursor::new(CANCEL_LINE.to_vec()),
    };
    let stop = AtomicBool::new(false);
    let got = run_loop(&mut reader, &stop);
    assert!(
        matches!(got.first(), Some(SessionControl::Cancel)),
        "expected Cancel after timeout retry, got {got:?}"
    );
}

/// Returns BrokenPipe and flips `stop` so the loop cannot spin forever when the
/// timeout guard is forced true.
struct BrokenPipeSetsStop {
    stop: Arc<AtomicBool>,
}

impl Read for BrokenPipeSetsStop {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        self.stop.store(true, Ordering::Release);
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[test]
fn read_control_loop_reports_disconnect_on_broken_pipe() {
    // KILLS the timeout_kind guard (-> true). A BrokenPipe is NOT a timeout, so
    // the loop must emit Disconnected. Under the guard forced true the error is
    // mistaken for a timeout and (with stop now set) the loop breaks WITHOUT
    // emitting Disconnected.
    let stop = Arc::new(AtomicBool::new(false));
    let mut reader = BrokenPipeSetsStop {
        stop: Arc::clone(&stop),
    };
    let got = run_loop(&mut reader, &stop);
    assert!(
        matches!(got.first(), Some(SessionControl::Disconnected)),
        "expected Disconnected on broken pipe, got {got:?}"
    );
}

/// Class of runtime poll failure the mock session raises.
#[derive(Clone, Copy)]
enum FailKind {
    /// A server-internal fault (a worker/store/envelope failure).
    Internal,
    /// A client/protocol fault (an invalid route).
    Protocol,
}

/// A session whose first `poll` fails with the chosen runtime error class, so
/// the delivery loop's `Err` arm is exercised deterministically.
struct FailingSession(FailKind);

impl SubscriptionSession for FailingSession {
    fn poll(
        &mut self,
        _timeout: std::time::Duration,
    ) -> Result<SessionPoll, SubscriptionRuntimeError> {
        Err(match self.0 {
            FailKind::Internal => SubscriptionRuntimeError::Worker("boom".to_owned()),
            FailKind::Protocol => SubscriptionRuntimeError::InvalidRoute {
                reason: "bad route",
            },
        })
    }
}

fn sub_err_code(frame: &StreamFrame) -> Option<&str> {
    if let StreamFrame::SubErr(err) = frame {
        Some(err.code.as_str())
    } else {
        None
    }
}

fn drive_runtime_failure(kind: FailKind) -> (TcpSubscriptionServeStats, StreamFrame) {
    let mut session = FailingSession(kind);
    let mut out: Vec<u8> = Vec::new();
    let mut stats = TcpSubscriptionServeStats::default();
    let limits = Limits::default();
    let id = token();
    let result = run_subscription_loop(&mut out, &mut session, &id, &limits, &mut stats);
    assert!(
        result.is_ok(),
        "a post-subscribe runtime poll failure must be contained (Ok), not escalated"
    );
    let frame =
        decode_stream_line(&out, &limits).expect("a SUB_ERR frame must be written before teardown");
    (stats, frame)
}

#[test]
fn runtime_poll_failure_emits_sub_err_counts_runtime_failure_and_distinguishes_classes() {
    // H3: a post-subscribe runtime poll failure must emit a terminal SUB_ERR to
    // the peer BEFORE teardown (never a bare close) and increment
    // `runtime_failures` (was never incremented; the failure escaped and was
    // miscounted as a failed_subscription). H2: a server-internal fault and a
    // client/protocol fault must carry DISTINCT wire codes so the peer can tell
    // "the server failed" from "your request was bad".
    let (internal_stats, internal_frame) = drive_runtime_failure(FailKind::Internal);
    let (client_stats, client_frame) = drive_runtime_failure(FailKind::Protocol);

    assert_eq!(
        internal_stats.runtime_failures, 1,
        "an internal poll failure must count in runtime_failures"
    );
    assert_eq!(
        internal_stats.failed_subscriptions, 0,
        "a post-subscribe runtime failure must NOT be miscounted as a failed_subscription"
    );
    assert_eq!(
        client_stats.runtime_failures, 1,
        "a client poll failure must also count in runtime_failures"
    );

    let internal_code = sub_err_code(&internal_frame).expect("internal failure must emit SUB_ERR");
    let client_code = sub_err_code(&client_frame).expect("client failure must emit SUB_ERR");
    assert_eq!(
        internal_code, INTERNAL_ERROR_CODE,
        "a server-internal fault must carry the internal_error wire code"
    );
    assert_eq!(
        client_code, MALFORMED_STREAM_FRAME_CODE,
        "a client/protocol fault must carry the malformed_stream_frame wire code"
    );
    assert_ne!(
        internal_code, client_code,
        "H2: internal and client failure classes must NOT collapse to one wire code"
    );
}

#[test]
fn runtime_failure_code_maps_each_class() {
    // KILLS an arm-collapse in runtime_failure_code: the three server-internal
    // variants map to internal_error; cursor faults keep their specific codes;
    // the remaining client/protocol faults map to malformed_stream_frame.
    assert_eq!(
        runtime_failure_code(&SubscriptionRuntimeError::Worker("x".to_owned())),
        INTERNAL_ERROR_CODE
    );
    assert_eq!(
        runtime_failure_code(&SubscriptionRuntimeError::EnvelopeEncoding("x".to_owned())),
        INTERNAL_ERROR_CODE
    );
    assert_eq!(
        runtime_failure_code(&SubscriptionRuntimeError::CursorInvalid { reason: "x" }),
        CURSOR_INVALID_CODE
    );
    assert_eq!(
        runtime_failure_code(&SubscriptionRuntimeError::CursorMismatch { reason: "x" }),
        CURSOR_MISMATCH_CODE
    );
    assert_eq!(
        runtime_failure_code(&SubscriptionRuntimeError::AckInvalid { reason: "x" }),
        MALFORMED_STREAM_FRAME_CODE
    );
}

#[test]
fn map_runtime_error_maps_each_variant() {
    // KILLS an arm-collapse in map_runtime_error (the open-failure reason mapper):
    // each mapped SubscriptionRuntimeError maps to its own MalformedStreamFrame
    // reason. The `reason`-forwarding arms pass a distinct token through (so
    // collapsing one into another is observable); the constant-reason arms assert
    // their exact literal. The `Store` arm is omitted for the same reason
    // `runtime_failure_code_maps_each_class` omits it: netbat deliberately takes
    // no core dependency, so a test here cannot name `batpak::store::StoreError`
    // to construct that variant. The other ten variants are covered.
    fn reason(error: &SubscriptionRuntimeError) -> &'static str {
        // map_runtime_error maps every variant to MalformedStreamFrame; a refutable
        // let-else (no wildcard match arm, no panic!) keeps clippy's restriction
        // lints satisfied while still failing loudly via the sentinel below if that
        // ever stops holding.
        let crate::transport::error::NetbatError::MalformedStreamFrame { reason } =
            map_runtime_error(error)
        else {
            return "UNEXPECTED: map_runtime_error did not return MalformedStreamFrame";
        };
        reason
    }

    assert_eq!(
        reason(&SubscriptionRuntimeError::InvalidSubscriptionId {
            reason: "invalid-id-token"
        }),
        "invalid-id-token"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::DuplicateSubscription {
            id: "dup".to_owned()
        }),
        "duplicate subscription route"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::InvalidRoute {
            reason: "invalid-route-token"
        }),
        "invalid-route-token"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::InvalidConfig {
            reason: "invalid-config-token"
        }),
        "invalid-config-token"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::UnknownSubscription {
            id: "missing".to_owned()
        }),
        "unknown subscription"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::CursorInvalid {
            reason: "cursor-invalid-token"
        }),
        "cursor-invalid-token"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::CursorMismatch {
            reason: "cursor-mismatch-token"
        }),
        "cursor-mismatch-token"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::EnvelopeEncoding("x".to_owned())),
        "envelope encoding failed"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::Worker("x".to_owned())),
        "subscription worker failed"
    );
    assert_eq!(
        reason(&SubscriptionRuntimeError::AckInvalid {
            reason: "ack-invalid-token"
        }),
        "ack-invalid-token"
    );
}

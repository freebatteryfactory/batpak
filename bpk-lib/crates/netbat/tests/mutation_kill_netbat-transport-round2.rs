//! PROVES: cloud-mutation survivors (round 2, WP-E) in the 0.9.0 transport
//! work — the ConnectionLimit-era listener join discipline and the TLS
//! surface — are killed by behavioural assertions on the public API.
//! CATCHES: a listener that stops joining in-flight workers before reporting
//! (dropping their stats and detaching their threads), inline/worker fault
//! counters that stop adding (`+=` -> `-=`/`*=`), a TlsServerConfig Debug that
//! stops redacting, a TLS session that miscounts a malformed first frame, and
//! a panic teardown that leaves the control-reader thread holding the client's
//! socket (a contained server-side panic the client could never observe).
//! SEEDED: localhost listeners with fake subscription runtimes (long-lived,
//! invalid-config, and panicking sessions), a ping core for the request
//! listener, and the committed throwaway test PKI for the TLS listener.

use netbat as nb;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::num::NonZeroUsize;
use std::thread;
use std::time::Duration;

use syncbat::{
    Core, EffectClass, Handler, HandlerResult, OperationDescriptor, RuntimeCursor, SessionControl,
    SessionDelivery, SessionEnd, SessionEventDelivery, SessionPoll, SubscriptionRuntimeError,
    SubscriptionSession, SubscriptionSessionFactory,
};

const WIRE_SCHEMA: &str = "batpak.event-stream-envelope.v1";
const SUBSCRIBE_LINE: &[u8] = b"NETBAT/2 SUBSCRIBE orders.open.v1 - 128\n";
/// A well-formed stream frame that is NOT a SUBSCRIBE: as a FIRST frame it is
/// rejected pre-subscribe (failed + malformed_pre_subscribe).
const CANCEL_LINE: &[u8] = b"NETBAT/2 SUB_CANCEL orders.open.v1 client.cancel\n";

fn lifetime(value: usize) -> nb::ConnectionLimit {
    nb::ConnectionLimit::Lifetime(NonZeroUsize::new(value).expect("nonzero connection limit"))
}

fn localhost_listener() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").expect("bind localhost listener")
}

fn connect(addr: std::net::SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("set client write timeout");
    stream
}

fn read_lines(stream: &mut TcpStream, want: usize, timeout: Duration) -> Vec<String> {
    stream
        .set_read_timeout(Some(timeout))
        .expect("set read timeout");
    let mut buf = Vec::new();
    let mut scratch = [0_u8; 4096];
    let mut lines = Vec::new();
    while lines.len() < want {
        match stream.read(&mut scratch) {
            Ok(0) => break,
            Ok(count) => {
                buf.extend_from_slice(&scratch[..count]);
                while let Some(pos) = buf.iter().position(|byte| *byte == b'\n') {
                    let line = buf.drain(..=pos).collect::<Vec<_>>();
                    let text = String::from_utf8_lossy(&line).trim().to_owned();
                    if !text.is_empty() {
                        lines.push(text);
                    }
                    if lines.len() >= want {
                        break;
                    }
                }
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Subscription runtime fixtures.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Mode {
    /// Sessions deliver one SUB_EVENT then stay open until the peer leaves.
    Live,
    /// open_session fails with InvalidConfig — the one open error that
    /// PROPAGATES out of the serve path as a non-IO error.
    InvalidConfig,
    /// Sessions panic on their first poll.
    Panic,
}

struct FakeRuntime(Mode);

impl SubscriptionSessionFactory for FakeRuntime {
    fn open_session(
        &self,
        subscription_id: &str,
        _resume_cursor: Option<&[u8]>,
        _client_window: u32,
        control_rx: flume::Receiver<SessionControl>,
    ) -> Result<Box<dyn SubscriptionSession>, SubscriptionRuntimeError> {
        match self.0 {
            Mode::Live => Ok(Box::new(LiveSession {
                event: Some(event_delivery(subscription_id)),
                control_rx,
            })),
            Mode::InvalidConfig => Err(SubscriptionRuntimeError::InvalidConfig {
                reason: "deliberate invalid-config fixture",
            }),
            Mode::Panic => Ok(Box::new(PanicSession)),
        }
    }
}

struct LiveSession {
    event: Option<SessionDelivery>,
    control_rx: flume::Receiver<SessionControl>,
}

impl LiveSession {
    fn ends(control: &SessionControl) -> bool {
        matches!(
            control,
            SessionControl::Disconnected | SessionControl::Cancel
        )
    }
}

impl SubscriptionSession for LiveSession {
    fn poll(&mut self, timeout: Duration) -> Result<SessionPoll, SubscriptionRuntimeError> {
        match self.control_rx.try_recv() {
            Ok(control) if Self::ends(&control) => return Ok(SessionPoll::Ended),
            Ok(_) => {}
            Err(flume::TryRecvError::Disconnected) => return Ok(SessionPoll::Ended),
            Err(flume::TryRecvError::Empty) => {}
        }
        if let Some(event) = self.event.take() {
            return Ok(SessionPoll::Delivery(event));
        }
        match self.control_rx.recv_timeout(timeout) {
            Ok(control) if Self::ends(&control) => Ok(SessionPoll::Ended),
            Ok(_) => Ok(SessionPoll::Blocked),
            Err(flume::RecvTimeoutError::Timeout) => Ok(SessionPoll::Blocked),
            Err(flume::RecvTimeoutError::Disconnected) => Ok(SessionPoll::Ended),
        }
    }
}

/// Panics on poll with a genuine runtime out-of-bounds index rather than
/// `panic!`/`unwrap`/`assert!`, staying inside the crate's zero-panic-macro
/// lint posture (same pattern as tcp_transport.rs's PanicHandler). The
/// containment path under test is agnostic to how the panic was raised.
struct PanicSession;

impl SubscriptionSession for PanicSession {
    fn poll(&mut self, _timeout: Duration) -> Result<SessionPoll, SubscriptionRuntimeError> {
        let probe: Vec<u8> = Vec::new();
        let escape = probe[probe.len() + 1];
        Ok(SessionPoll::Delivery(SessionDelivery::End(SessionEnd {
            subscription_id: format!("unreachable.{escape}.v1"),
            reason_code: "unreachable",
            cursor_after: None,
        })))
    }
}

fn event_delivery(subscription_id: &str) -> SessionDelivery {
    SessionDelivery::Event(SessionEventDelivery {
        subscription_id: subscription_id.to_owned(),
        delivery_index: 1,
        cursor_before: RuntimeCursor::from_bytes(vec![0]),
        cursor_after: RuntimeCursor::from_bytes(vec![1]),
        wire_payload_schema_ref: WIRE_SCHEMA.to_owned(),
        envelope_bytes: b"canonical-envelope-fixture".to_vec(),
    })
}

fn spawn_subscription_listener(
    name: &'static str,
    listener: TcpListener,
    mode: Mode,
    config: nb::TcpSubscriptionServerConfig,
    shutdown: nb::ShutdownHandle,
) -> flume::Receiver<Result<nb::TcpSubscriptionServeStats, nb::NetbatError>> {
    let (tx, rx) = flume::bounded(1);
    thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            let result = nb::serve_tcp_subscription_listener(
                listener,
                FakeRuntime(mode),
                &config,
                &shutdown,
            );
            let _ = tx.send(result);
        })
        .expect("spawn subscription listener");
    rx
}

// ---------------------------------------------------------------------------
// stream_tcp.rs:260 — the concurrent subscription listener must JOIN its
// in-flight workers before reporting.
// ---------------------------------------------------------------------------

#[test]
fn subscription_listener_joins_inflight_workers_before_reporting() {
    // KILLS stream_tcp.rs:260 (delete `!` in `workers.retain(|w|
    // !w.is_finished())`): the inverted retain PRUNES the still-running
    // workers instead of the finished ones, so on shutdown the listener joins
    // nothing that matters, returns while sessions A and B are still being
    // served, and their stats are lost (and their threads detached).
    //
    // Choreography: A and B hold live sessions. C's malformed first frame
    // makes its worker finish fast, and C's accept iteration runs the retain
    // pass while A and B are provably alive (their sockets are still open) —
    // under the mutant, A and B are pruned right there, and C (finished) is
    // the only handle left, so the mutated listener returns almost instantly
    // after shutdown with only C's stats. The real listener cannot return
    // within the probe window because joining A blocks until A's session ends.
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let config = nb::TcpSubscriptionServerConfig::default();
    let rx = spawn_subscription_listener(
        "mk2-sub-join",
        listener,
        Mode::Live,
        config,
        shutdown.clone(),
    );

    let mut client_a = connect(addr);
    client_a.write_all(SUBSCRIBE_LINE).expect("A subscribes");
    let a_lines = read_lines(&mut client_a, 1, Duration::from_secs(2));
    assert!(
        a_lines.iter().any(|line| line.contains("SUB_EVENT")),
        "subscriber A must be live before shutdown; got {a_lines:?}"
    );

    let mut client_b = connect(addr);
    client_b.write_all(SUBSCRIBE_LINE).expect("B subscribes");
    let b_lines = read_lines(&mut client_b, 1, Duration::from_secs(2));
    assert!(
        b_lines.iter().any(|line| line.contains("SUB_EVENT")),
        "subscriber B must be live before shutdown; got {b_lines:?}"
    );

    // C: malformed first frame — served and closed immediately. Reading C's
    // EOF proves C's worker ran to completion, which proves the accept loop
    // executed a retain pass while A's and B's workers were alive.
    let mut client_c = connect(addr);
    client_c
        .write_all(CANCEL_LINE)
        .expect("C sends non-SUBSCRIBE");
    let mut sink = Vec::new();
    client_c
        .read_to_end(&mut sink)
        .expect("C's worker closes its stream");

    shutdown.shutdown();

    // The REAL listener is now blocked joining A (whose session only ends when
    // A's socket drops), so it cannot report within this probe window; a
    // result arriving here means the join was skipped.
    let result = match rx.recv_timeout(Duration::from_millis(500)) {
        Ok(early) => early,
        Err(_) => {
            drop(client_a);
            drop(client_b);
            rx.recv_timeout(Duration::from_secs(10))
                .expect("listener exits once the held sessions end")
        }
    };
    let stats = result.expect("listener returns Ok");
    assert_eq!(
        stats.served_subscriptions, 2,
        "both in-flight workers must be joined and their stats merged; stats={stats:?}"
    );
    assert_eq!(stats.failed_subscriptions, 1, "stats={stats:?}");
    assert_eq!(stats.malformed_pre_subscribe, 1, "stats={stats:?}");
    assert_eq!(stats.worker_panics, 0, "stats={stats:?}");
    assert_eq!(stats.accepted_connections, 3, "stats={stats:?}");
    assert!(stats.shutdown_requested, "stats={stats:?}");
}

// ---------------------------------------------------------------------------
// stream_tcp.rs:343 — the Sequential (inline) path's connection_io_failures.
// ---------------------------------------------------------------------------

#[test]
fn sequential_dispatch_counts_each_inline_io_failure() {
    // KILLS stream_tcp.rs:343 (`+=` -> `-=`/`*=` on connection_io_failures in
    // serve_subscription_inline). Two idle peers each drive the inline
    // per-session read to a timeout (an Io error), so the SEQUENTIAL listener
    // must count exactly two inline connection IO failures: `*=` leaves the
    // counter at 0, `-=` underflows. (The concurrent-dispatch counterpart at
    // stream_tcp.rs:377 is pinned by round 1's
    // listener_counts_idle_read_timeout_as_io_failure.)
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let mut config = nb::TcpSubscriptionServerConfig::default();
    config.dispatch = nb::SubscriptionDispatch::Sequential;
    config.connection_limit = lifetime(2);
    config.timeouts = nb::IoTimeouts::default()
        .with_read(Some(Duration::from_millis(100)))
        .with_write(Some(Duration::from_secs(2)));
    let rx = spawn_subscription_listener(
        "mk2-sub-inline-io",
        listener,
        Mode::Live,
        config,
        shutdown.clone(),
    );

    // Connect two idle clients; the second waits in the backlog while the
    // first is served inline to its read timeout.
    let client_one = connect(addr);
    let client_two = connect(addr);

    let stats = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("listener exits on its lifetime budget")
        .expect("listener returns Ok");
    drop(client_one);
    drop(client_two);
    assert_eq!(stats.accepted_connections, 2, "stats={stats:?}");
    assert_eq!(
        stats.connection_io_failures, 2,
        "each idle peer is exactly one inline IO failure; stats={stats:?}"
    );
    assert_eq!(stats.served_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.failed_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.worker_panics, 0, "stats={stats:?}");
    assert!(!stats.shutdown_requested, "stats={stats:?}");
}

// ---------------------------------------------------------------------------
// stream_tcp.rs:382/383 — the concurrent worker's fault counters.
// ---------------------------------------------------------------------------

#[test]
fn worker_counts_non_io_session_error_as_failed_subscription() {
    // KILLS stream_tcp.rs:382 (`+=` -> `-=`/`*=` on failed_subscriptions in
    // the worker's non-IO error arm). An InvalidConfig open error is the one
    // open failure that PROPAGATES as a non-IO error out of the serve path, so
    // the containing worker must count exactly one failed subscription: `*=`
    // leaves 0, `-=` underflows (which escapes the worker and poisons the
    // listener's join into an error — also caught here by expecting Ok).
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let mut config = nb::TcpSubscriptionServerConfig::default();
    config.connection_limit = lifetime(1);
    let rx = spawn_subscription_listener(
        "mk2-sub-invalid-config",
        listener,
        Mode::InvalidConfig,
        config,
        shutdown.clone(),
    );

    let mut client = connect(addr);
    client.write_all(SUBSCRIBE_LINE).expect("subscribe");
    let mut sink = Vec::new();
    client
        .read_to_end(&mut sink)
        .expect("worker closes the stream after the open failure");

    let stats = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("listener exits on its lifetime budget")
        .expect("listener returns Ok");
    assert_eq!(stats.accepted_connections, 1, "stats={stats:?}");
    assert_eq!(
        stats.failed_subscriptions, 1,
        "the worker's non-IO error arm must count exactly once; stats={stats:?}"
    );
    assert_eq!(stats.served_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.malformed_pre_subscribe, 0, "stats={stats:?}");
    assert_eq!(stats.connection_io_failures, 0, "stats={stats:?}");
    assert_eq!(stats.worker_panics, 0, "stats={stats:?}");
}

#[test]
fn worker_panic_is_contained_and_counted_exactly_once() {
    // KILLS stream_tcp.rs:383 (`+=` -> `-=`/`*=` on worker_panics in the
    // caught-panic arm). A session that panics on poll is contained at the
    // worker boundary and counted exactly once: `*=` leaves 0, `-=` underflows
    // (escaping the worker, poisoning the join — caught by expecting Ok).
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let mut config = nb::TcpSubscriptionServerConfig::default();
    config.connection_limit = lifetime(1);
    let rx = spawn_subscription_listener(
        "mk2-sub-panic",
        listener,
        Mode::Panic,
        config,
        shutdown.clone(),
    );

    let mut client = connect(addr);
    client.write_all(SUBSCRIBE_LINE).expect("subscribe");
    // No EOF wait needed here: the Lifetime(1) budget already bounds the
    // listener (it exits once the panicked worker is joined). The client-visible
    // close on the panic path is pinned separately by
    // server_side_session_panic_is_client_visible_as_close.
    let stats = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("listener exits on its lifetime budget")
        .expect("listener returns Ok despite the contained panic");
    drop(client);
    assert_eq!(stats.accepted_connections, 1, "stats={stats:?}");
    assert_eq!(
        stats.worker_panics, 1,
        "the caught panic must be counted exactly once; stats={stats:?}"
    );
    assert_eq!(stats.served_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.failed_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.connection_io_failures, 0, "stats={stats:?}");
}

#[test]
fn server_side_session_panic_is_client_visible_as_close() {
    // PROVES stream_tcp.rs's StopReaderOnExit guard: a session that panics
    // mid-poll unwinds through serve_subscription_stream, and the guard's Drop
    // (not a post-loop store the unwind would skip) stops the control-reader
    // thread, which drops the LAST try_clone of the socket. The client must
    // therefore OBSERVE the connection close — an orderly EOF or a reset —
    // within its bounded read timeout, never a silent hang it can only resolve
    // by hanging up itself. Regression: before the guard, the reader thread
    // survived the caught panic holding the cloned socket open, and this read
    // ran to its timeout.
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let mut config = nb::TcpSubscriptionServerConfig::default();
    config.connection_limit = lifetime(1);
    let rx = spawn_subscription_listener(
        "mk2-sub-panic-close",
        listener,
        Mode::Panic,
        config,
        shutdown.clone(),
    );

    let mut client = connect(addr);
    client.write_all(SUBSCRIBE_LINE).expect("subscribe");
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set bounded client read timeout");
    let mut sink = Vec::new();
    match client.read_to_end(&mut sink) {
        // Orderly close: EOF with zero payload bytes (the session panicked
        // before any delivery was written).
        Ok(read) => assert_eq!(read, 0, "no delivery precedes the panic"),
        // A reset is an equally client-visible close.
        Err(error) => assert_eq!(
            error.kind(),
            io::ErrorKind::ConnectionReset,
            "the close must surface as EOF or a reset, never a read timeout"
        ),
    }

    let stats = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("listener exits on its lifetime budget")
        .expect("listener returns Ok despite the contained panic");
    assert_eq!(stats.accepted_connections, 1, "stats={stats:?}");
    assert_eq!(
        stats.worker_panics, 1,
        "the contained panic is still counted exactly once; stats={stats:?}"
    );
    assert_eq!(stats.served_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.failed_subscriptions, 0, "stats={stats:?}");
    assert_eq!(stats.connection_io_failures, 0, "stats={stats:?}");
}

// ---------------------------------------------------------------------------
// tcp.rs:353 — the request listener must JOIN its in-flight workers before
// reporting.
// ---------------------------------------------------------------------------

const PING: OperationDescriptor = OperationDescriptor::new(
    "ping",
    EffectClass::Inspect,
    "schema.ping.input.v1",
    "schema.ping.output.v1",
    "receipt.ping.v1",
);

struct PingHandler;

impl Handler for PingHandler {
    fn handle(&mut self, input: &[u8], _cx: &mut syncbat::Ctx<'_>) -> HandlerResult {
        Ok(input.to_vec())
    }
}

fn core_with_ping() -> Core {
    let mut builder = Core::builder();
    builder.register(PING, PingHandler).expect("register ping");
    builder.without_receipts();
    builder.build().expect("core builds")
}

fn ping_roundtrip(stream: &mut TcpStream) {
    stream
        .write_all(b"NETBAT/1 CALL ping 6869\n")
        .expect("write ping request");
    let mut response = String::new();
    BufReader::new(&*stream)
        .read_line(&mut response)
        .expect("read ping response");
    assert_eq!(response, "OK 6869\n");
}

#[test]
fn request_listener_joins_inflight_workers_before_reporting() {
    // KILLS tcp.rs:353 (delete `!` in `workers.retain(|w|
    // !w.is_finished())`): the inverted retain prunes the still-running
    // request workers, so on shutdown the listener returns without joining
    // them and their served-request stats are lost. Same choreography as the
    // subscription counterpart: A and B hold connections with their request
    // unsent (workers parked in read_line — the default config sets no read
    // timeout), C completes a full round trip (proving a retain pass ran while
    // A and B were alive) — the mutated listener then reports only C's request
    // within the probe window, while the real listener blocks joining A.
    let listener = localhost_listener();
    let addr = listener.local_addr().expect("listener addr");
    let shutdown = nb::ShutdownHandle::new();
    let server_shutdown = shutdown.clone();
    let config = nb::TcpServerConfig::default();
    let (tx, rx) = flume::bounded(1);
    thread::Builder::new()
        .name("mk2-req-join".to_owned())
        .spawn(move || {
            let factory = core_with_ping;
            let result = nb::serve_tcp_listener(listener, factory, &config, &server_shutdown);
            let _ = tx.send(result);
        })
        .expect("spawn request listener");

    let mut client_a = connect(addr);
    let mut client_b = connect(addr);
    let mut client_c = connect(addr);
    ping_roundtrip(&mut client_c);
    let mut sink = Vec::new();
    client_c
        .read_to_end(&mut sink)
        .expect("C's worker closes after its single request");

    shutdown.shutdown();

    let result = match rx.recv_timeout(Duration::from_millis(500)) {
        Ok(early) => early,
        Err(_) => {
            // Real listener: blocked joining A's worker. Complete A's and B's
            // requests so their workers finish and the join can proceed.
            ping_roundtrip(&mut client_a);
            ping_roundtrip(&mut client_b);
            rx.recv_timeout(Duration::from_secs(10))
                .expect("listener exits once the held connections are served")
        }
    };
    let stats = result.expect("listener returns Ok");
    assert_eq!(
        stats.served_requests, 3,
        "all in-flight workers must be joined and their stats merged; stats={stats:?}"
    );
    assert_eq!(stats.accepted_connections, 3, "stats={stats:?}");
    assert_eq!(stats.worker_panics, 0, "stats={stats:?}");
    assert_eq!(stats.connection_io_failures, 0, "stats={stats:?}");
    assert!(stats.shutdown_requested, "stats={stats:?}");
}

// ---------------------------------------------------------------------------
// TLS surface (feature = "tls"): TlsServerConfig Debug redaction and the TLS
// session's malformed-first-frame counters.
// ---------------------------------------------------------------------------

#[cfg(feature = "tls")]
mod tls_surface {
    use super::*;
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, ServerName};
    use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
    use std::sync::Arc;

    /// Committed throwaway localhost test PKI (server identity material only).
    const CA_PEM: &[u8] = include_bytes!("fixtures/tls_test_ca_cert.pem");
    const CERT_PEM: &[u8] = include_bytes!("fixtures/tls_test_cert.pem");
    const KEY_PEM: &[u8] = include_bytes!("fixtures/tls_test_key.pem");

    #[test]
    fn tls_server_config_debug_is_opaque_and_leaks_no_key_material() {
        // KILLS tls.rs:44 (<impl Debug for TlsServerConfig>::fmt ->
        // Ok(Default::default())). The hand-written Debug exists to keep the
        // wrapped private key opaque WHILE still identifying the type: the
        // mutant writes nothing at all, which breaks the diagnostic value the
        // impl is there to preserve. Pin all three properties: non-empty,
        // names the type with the non-exhaustive redaction marker, and never
        // contains a byte of the private key's PEM payload.
        let tls = nb::TlsServerConfig::from_pem(CERT_PEM, KEY_PEM).expect("build TlsServerConfig");
        let rendered = format!("{tls:?}");
        assert!(
            !rendered.is_empty(),
            "Debug must render an opaque marker, not nothing"
        );
        assert!(
            rendered.contains("TlsServerConfig"),
            "Debug names the type; got {rendered:?}"
        );
        assert!(
            rendered.contains(".."),
            "Debug shows the redaction marker; got {rendered:?}"
        );
        let key_text = std::str::from_utf8(KEY_PEM).expect("fixture key is UTF-8");
        for payload_line in key_text
            .lines()
            .map(str::trim)
            .filter(|line| line.len() > 10 && !line.starts_with('-'))
        {
            assert!(
                !rendered.contains(payload_line),
                "Debug output must never contain private key material"
            );
        }
    }

    /// Connect a real rustls client that trusts the committed test cert.
    fn tls_client(addr: std::net::SocketAddr) -> StreamOwned<ClientConnection, TcpStream> {
        let mut roots = RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(CA_PEM) {
            roots
                .add(cert.expect("parse fixture CA cert"))
                .expect("add fixture CA cert to client roots");
        }
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .expect("client protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
        let server_name = ServerName::try_from("localhost").expect("server name");
        let conn = ClientConnection::new(Arc::new(config), server_name).expect("client connection");
        let sock = TcpStream::connect(addr).expect("connect tls client");
        sock.set_read_timeout(Some(Duration::from_secs(3)))
            .expect("client read timeout");
        sock.set_write_timeout(Some(Duration::from_secs(3)))
            .expect("client write timeout");
        StreamOwned::new(conn, sock)
    }

    #[test]
    fn tls_session_counts_malformed_first_frame_exactly_once() {
        // KILLS stream_tcp_tls.rs:94/95 (`+=` -> `-=`/`*=` on
        // failed_subscriptions and malformed_pre_subscribe in the TLS
        // session's pre-subscribe rejection). A rustls client whose FIRST
        // frame is a well-formed non-SUBSCRIBE must be dropped with exactly
        // one failed + one malformed count: `*=` leaves 0, `-=` underflows
        // (escaping as a worker panic — pinned to 0 here).
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tls sub listener");
        let addr = listener.local_addr().expect("listener addr");
        let shutdown = nb::ShutdownHandle::new();
        let security = nb::TransportSecurity::Tls(
            nb::TlsServerConfig::from_pem(CERT_PEM, KEY_PEM).expect("build TlsServerConfig"),
        );
        let mut config = nb::TcpSubscriptionServerConfig::default();
        config.connection_limit = lifetime(1);
        config.idle_sleep = Duration::from_millis(1);
        config.timeouts = nb::IoTimeouts::default()
            .with_read(Some(Duration::from_secs(3)))
            .with_write(Some(Duration::from_secs(3)));
        let handle = thread::Builder::new()
            .name("mk2-tls-malformed".to_owned())
            .spawn(move || {
                nb::serve_tcp_subscription_listener_secured(
                    listener,
                    FakeRuntime(Mode::Live),
                    &config,
                    &security,
                    &shutdown,
                )
                .expect("serve tls subscription listener")
            })
            .expect("spawn tls subscription server");

        let mut client = tls_client(addr);
        client
            .write_all(CANCEL_LINE)
            .expect("send non-SUBSCRIBE first frame over tls");
        client.flush().expect("flush first frame");
        let mut sink = Vec::new();
        let err = client
            .read_to_end(&mut sink)
            .expect_err("server drops the session without close_notify");
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);

        let stats = handle.join().expect("server thread joins");
        assert_eq!(stats.accepted_connections, 1, "stats={stats:?}");
        assert_eq!(
            stats.failed_subscriptions, 1,
            "the rejected first frame counts exactly one failed subscription; stats={stats:?}"
        );
        assert_eq!(
            stats.malformed_pre_subscribe, 1,
            "the rejected first frame counts exactly one malformed pre-subscribe; stats={stats:?}"
        );
        assert_eq!(stats.served_subscriptions, 0, "stats={stats:?}");
        assert_eq!(stats.tls_handshake_failures, 0, "stats={stats:?}");
        assert_eq!(stats.worker_panics, 0, "stats={stats:?}");
    }
}

//! Unit tests for the single-threaded TLS subscription control drain.
//!
//! PROVES: the drain's io::ErrorKind classification (WouldBlock => keep
//! serving, unclean EOF => peer gone, hard socket error => peer gone), the
//! per-pass `MAX_TLS_READS_PER_DRAIN` socket-read budget boundary, and the
//! accumulator's exact line-cap boundary and lane back-pressure behavior.
//! CATCHES: a drain that misclassifies a quiet connection as a disconnect (or
//! vice versa), a budget counter/comparison that no longer stops a record
//! flood after exactly the capped number of socket reads, and an unterminated
//! line cap that fires one byte early.
//! SEEDED: real localhost rustls pairs built from the committed test PKI
//! (loaded at RUNTIME from `tests/fixtures`, which stays excluded from the
//! published crate), plus rustls-record-layer state injected through
//! `read_tls` where a kernel socket cannot produce the state on demand.
//!
//! The TLS session glue (handshake, delivery multiplex, cancel honoring) is
//! covered end-to-end by `tests/tls_subscription.rs`; these tests pin the
//! drain internals that the session loop depends on.

use super::*;

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

const ACK_LINE: &[u8] = b"NETBAT/2 SUB_ACK orders.open.v1 1 aabb\n";
const CANCEL_LINE: &[u8] = b"NETBAT/2 SUB_CANCEL orders.open.v1 client.cancel\n";

fn token() -> SubscriptionToken {
    SubscriptionToken::new("orders.open.v1", &Limits::default()).expect("token")
}

// ---------------------------------------------------------------------------
// ControlAccumulator: pure line reassembly, cap boundary, lane back-pressure.
// ---------------------------------------------------------------------------

#[test]
fn forwards_a_complete_ack_without_stopping() {
    // A well-formed, id-matching SUB_ACK is non-terminal: it is forwarded and
    // the drain keeps reading (NeedMore), with the line consumed from the
    // buffer.
    let (tx, rx) = flume::bounded(16);
    let mut acc = ControlAccumulator::new();
    acc.extend(ACK_LINE);
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::NeedMore
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Ack { .. })));
    assert!(acc.buffer.is_empty(), "the consumed line is drained");
}

#[test]
fn cancel_is_terminal_and_stops() {
    let (tx, rx) = flume::bounded(16);
    let mut acc = ControlAccumulator::new();
    acc.extend(CANCEL_LINE);
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::Stopped
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Cancel)));
}

#[test]
fn reassembles_a_frame_split_across_extends() {
    // A SUB_CANCEL split mid-line must NOT be forwarded until the newline
    // arrives, then forwarded exactly once as Cancel.
    let (tx, rx) = flume::bounded(16);
    let mut acc = ControlAccumulator::new();
    let split = CANCEL_LINE.len() / 2;
    acc.extend(&CANCEL_LINE[..split]);
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::NeedMore
    ));
    assert!(rx.try_recv().is_err(), "no frame before the line completes");
    acc.extend(&CANCEL_LINE[split..]);
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::Stopped
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Cancel)));
}

#[test]
fn oversize_unterminated_line_is_malformed_terminal() {
    // A line that grows past the cap without a newline must surface a
    // malformed terminal control, never grow the buffer without bound.
    let limits = Limits::default().with_max_line_bytes(8);
    let (tx, rx) = flume::bounded(16);
    let mut acc = ControlAccumulator::new();
    acc.extend(b"NETBAT/2 SUB_ACK no-newline-here-yet");
    assert!(matches!(
        acc.forward_complete_lines(&tx, &limits, &token()),
        LineFlow::Stopped
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Malformed)));
    assert!(acc.buffer.is_empty());
}

#[test]
fn full_lane_reports_backpressure_and_keeps_the_line() {
    // With the lane already full, a complete line cannot be forwarded: report
    // Backpressure and retain the line so it is retried after the next poll
    // drains the lane — no frame is dropped.
    let (tx, rx) = flume::bounded(1);
    tx.try_send(SessionControl::Cancel)
        .expect("prefill the lane");
    let mut acc = ControlAccumulator::new();
    acc.extend(ACK_LINE);
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::Backpressure
    ));
    assert_eq!(
        acc.buffer, ACK_LINE,
        "the unsent line is retained for retry"
    );
    // Drain the prefill; the retry now lands the ACK.
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Cancel)));
    assert!(matches!(
        acc.forward_complete_lines(&tx, &Limits::default(), &token()),
        LineFlow::NeedMore
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Ack { .. })));
}

#[test]
fn unterminated_line_at_the_exact_cap_is_not_yet_malformed() {
    // KILLS stream_tcp_tls.rs:320 (`>` -> `>=` on the unterminated-line cap in
    // forward_complete_lines). A partial line of EXACTLY max_line_bytes is
    // still legal — the terminating newline may arrive in the next drain — so
    // it must stay buffered as NeedMore; one byte MORE must surface the
    // malformed terminal control. Pinning both sides of the boundary kills the
    // off-by-one comparison mutants.
    let limits = Limits::default().with_max_line_bytes(8);
    let (tx, rx) = flume::bounded(16);
    let mut acc = ControlAccumulator::new();
    acc.extend(b"12345678"); // exactly the cap, no newline yet
    assert!(matches!(
        acc.forward_complete_lines(&tx, &limits, &token()),
        LineFlow::NeedMore
    ));
    assert!(
        rx.try_recv().is_err(),
        "an at-cap partial line is not malformed"
    );
    assert_eq!(acc.buffer, b"12345678", "the at-cap partial stays buffered");
    acc.extend(b"9"); // one byte over the cap
    assert!(matches!(
        acc.forward_complete_lines(&tx, &limits, &token()),
        LineFlow::Stopped
    ));
    assert!(matches!(rx.try_recv(), Ok(SessionControl::Malformed)));
    assert!(acc.buffer.is_empty());
}

// ---------------------------------------------------------------------------
// drain_control_frames over a real rustls pair: ErrorKind classification and
// the per-pass socket-read budget.
// ---------------------------------------------------------------------------

fn fixture_bytes(name: &str) -> Vec<u8> {
    // Runtime read (NOT include_bytes!) so the published crate — which
    // excludes tests/ — never references the fixture paths at compile time.
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read(&path).expect("read committed TLS test fixture")
}

/// Handshake a real localhost rustls server/client pair using the committed
/// test PKI. The client handshakes on a helper thread (both sides of a TLS
/// handshake must run concurrently); both sockets come back BLOCKING with
/// generous timeouts, exactly as the accept loop hands sessions to the worker.
fn tls_pair() -> (TlsStream, StreamOwned<ClientConnection, TcpStream>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost listener");
    let addr = listener.local_addr().expect("listener addr");

    let client_thread = thread::Builder::new()
        .name("netbat-tls-unit-client".to_owned())
        .spawn(move || {
            let mut roots = RootCertStore::empty();
            for cert in CertificateDer::pem_slice_iter(&fixture_bytes("tls_test_ca_cert.pem")) {
                roots
                    .add(cert.expect("parse fixture CA cert"))
                    .expect("add fixture CA cert");
            }
            let config = ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("client protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
            let server_name = ServerName::try_from("localhost").expect("server name");
            let mut conn =
                ClientConnection::new(Arc::new(config), server_name).expect("client connection");
            let mut sock = TcpStream::connect(addr).expect("connect tls client");
            sock.set_read_timeout(Some(Duration::from_secs(5)))
                .expect("client read timeout");
            sock.set_write_timeout(Some(Duration::from_secs(5)))
                .expect("client write timeout");
            conn.complete_io(&mut sock).expect("client handshake");
            StreamOwned::new(conn, sock)
        })
        .expect("spawn tls unit client");

    let (server_sock, _peer) = listener.accept().expect("accept tls client");
    server_sock
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("server read timeout");
    server_sock
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("server write timeout");
    let tls = TlsServerConfig::from_pem(
        &fixture_bytes("tls_test_cert.pem"),
        &fixture_bytes("tls_test_key.pem"),
    )
    .expect("build server TLS config from fixtures");
    let server = tls.handshake(server_sock).expect("server handshake");
    let client = client_thread.join().expect("client handshake thread");
    (server, client)
}

/// One drain pass with the socket flipped non-blocking around it, exactly as
/// `run_tls_subscription_loop` invokes it.
fn drain_once(
    server: &mut TlsStream,
    accumulator: &mut ControlAccumulator,
    control_tx: &flume::Sender<SessionControl>,
    limits: &Limits,
) -> ControlDrain {
    server
        .sock
        .set_nonblocking(true)
        .expect("flip server socket non-blocking");
    let outcome = drain_control_frames(server, accumulator, control_tx, limits, &token());
    server
        .sock
        .set_nonblocking(false)
        .expect("restore blocking server socket");
    outcome
}

#[test]
fn quiet_connection_drain_is_idle() {
    // KILLS stream_tcp_tls.rs:248 and :269 (`==` -> `!=` and guard -> false on
    // the WouldBlock guards). A healthy, quiet peer produces WouldBlock from
    // both the rustls plaintext reader and the socket read_tls; the drain MUST
    // classify that as Idle (keep serving). Under the mutated guards the
    // WouldBlock falls through to the catch-all and the drain reports the peer
    // gone, tearing down a healthy session.
    let (mut server, client) = tls_pair();
    let limits = Limits::default();
    let (control_tx, control_rx) = flume::bounded(16);
    let mut accumulator = ControlAccumulator::new();
    let outcome = drain_once(&mut server, &mut accumulator, &control_tx, &limits);
    assert!(
        matches!(outcome, ControlDrain::Idle),
        "a quiet healthy connection must drain to Idle"
    );
    assert!(
        control_rx.try_recv().is_err(),
        "no control frame is forwarded for a quiet connection"
    );
    drop(client);
}

#[test]
fn eof_without_close_notify_drains_to_peer_gone() {
    // KILLS stream_tcp_tls.rs:248 (guard -> true on the WouldBlock arm). Once
    // the record layer has seen EOF without a close_notify, the rustls reader
    // reports UnexpectedEof and the drain must classify the peer as gone. With
    // the WouldBlock guard forced true, the UnexpectedEof is swallowed by the
    // keep-going arm, the drain falls through to the (quiet) socket, and a
    // vanished peer is misreported as Idle — the session would keep polling a
    // dead stream. The EOF is injected through `read_tls` (a zero-byte reader
    // is the record layer's own EOF signal) because a kernel socket cannot be
    // put in this state while the connection object is mid-drain.
    let (mut server, client) = tls_pair();
    let consumed = server
        .conn
        .read_tls(&mut std::io::empty())
        .expect("record-layer EOF injection");
    assert_eq!(consumed, 0, "io::empty() must register EOF");
    let limits = Limits::default();
    let (control_tx, _control_rx) = flume::bounded(16);
    let mut accumulator = ControlAccumulator::new();
    let outcome = drain_once(&mut server, &mut accumulator, &control_tx, &limits);
    assert!(
        matches!(outcome, ControlDrain::PeerGone),
        "an unclean EOF must drain to PeerGone"
    );
    drop(client);
}

#[test]
fn reset_peer_drains_to_peer_gone() {
    // KILLS stream_tcp_tls.rs:269 (guard -> true on the read_tls WouldBlock
    // arm). A peer that dies with an RST surfaces a hard error
    // (ConnectionReset) from the socket read; the drain must classify that as
    // PeerGone via the catch-all. With the WouldBlock guard forced true the
    // hard error is reported as Idle and the session would keep polling a
    // reset stream forever. RST delivery on loopback is near-synchronous with
    // the peer's close(2) but not guaranteed ordered, so an attempt that
    // observes a still-quiet socket (Idle) retries with a FRESH pair; the
    // mutant returns Idle on the reset itself on EVERY attempt, so the retry
    // loop cannot save it.
    let mut outcome = ControlDrain::Idle;
    for _attempt in 0_u8..10 {
        let (mut server, client) = tls_pair();
        // Seed unread bytes in the client's receive queue so its close(2)
        // emits an RST rather than an orderly FIN.
        server
            .sock
            .write_all(b"unread-seed")
            .expect("seed unread client bytes");
        drop(client);
        let limits = Limits::default();
        let (control_tx, _control_rx) = flume::bounded(16);
        let mut accumulator = ControlAccumulator::new();
        outcome = drain_once(&mut server, &mut accumulator, &control_tx, &limits);
        if !matches!(outcome, ControlDrain::Idle) {
            break;
        }
    }
    assert!(
        matches!(outcome, ControlDrain::PeerGone),
        "a reset peer must drain to PeerGone"
    );
}

/// Build a TLS pair and flood the server-bound socket with newline-free
/// plaintext until the kernel buffer is full. Returns `(server, client,
/// wire_bytes)` — the caller must keep the client alive so the unread flood
/// remainder stays peekable on the socket.
fn flooded_tls_pair() -> (TlsStream, StreamOwned<ClientConnection, TcpStream>, usize) {
    let (server, mut client) = tls_pair();
    // The drain-budget property needs the kernel to stage MORE flood than one
    // budgeted pass can consume (~1.4 MiB: cap * per-read ceiling + slack).
    // Winsock's loopback defaults buffer only ~64-256 KiB and grant
    // SO_SNDBUF/SO_RCVBUF exactly as asked, so on Windows the test pins its
    // own buffer sizes instead of inheriting host defaults. Linux stays on
    // kernel autotuning ON PURPOSE: an explicit setsockopt there disables
    // autotuning and clamps to net.core.rmem_max (commonly ~208 KiB), which
    // would SHRINK the staged flood on the platform where defaults suffice.
    #[cfg(windows)]
    {
        const FLOOD_BUFFER_BYTES: usize = 2 * 1024 * 1024;
        socket2::SockRef::from(&client.sock)
            .set_send_buffer_size(FLOOD_BUFFER_BYTES)
            .expect("pin client send buffer for the flood");
        socket2::SockRef::from(&server.sock)
            .set_recv_buffer_size(FLOOD_BUFFER_BYTES)
            .expect("pin server recv buffer for the flood");
    }
    client
        .sock
        .set_nonblocking(true)
        .expect("non-blocking client socket for the flood");
    let payload = vec![b'x'; 16 * 1024]; // newline-free: never a control line
    let mut wire_bytes = 0_usize;
    'flood: loop {
        let _buffered = client
            .conn
            .writer()
            .write(&payload)
            .expect("buffer flood plaintext");
        while client.conn.wants_write() {
            match client.conn.write_tls(&mut client.sock) {
                Ok(count) => wire_bytes += count,
                Err(error) => {
                    assert_eq!(
                        error.kind(),
                        io::ErrorKind::WouldBlock,
                        "the flood must only stop on a full socket"
                    );
                    break 'flood;
                }
            }
        }
    }
    (server, client, wire_bytes)
}

#[test]
fn drain_budget_bounds_one_flood_pass() {
    // KILLS stream_tcp_tls.rs:258 (`+=` -> `*=` on the tls_reads counter):
    // with the counter frozen at zero the budget never trips and one drain
    // pass consumes an entire multi-megabyte record flood before yielding.
    // The real bound is deterministic: a pass performs at most
    // MAX_TLS_READS_PER_DRAIN socket reads of at most 4096 bytes each (the
    // rustls deframer READ_SIZE), so its plaintext accumulation is STRICTLY
    // below cap*4096 — and the flood left unread by the capped pass is still
    // waiting on the socket afterwards, which the mutant (having swallowed
    // everything) cannot reproduce.
    //
    // The lower bound pins that the pass ended on the BUDGET, not on supply:
    // more than half the cap's worth must have been drained. A legitimate
    // mid-pass EAGAIN under scheduler pressure could end a real pass early,
    // so a short pass retries with a fresh flood; the mutant fails the upper
    // bound on its very first pass, so retries cannot save it.
    //
    // NOTE stream_tcp_tls.rs:259 (`>` -> `>=`/`==`: the budget stops one
    // socket read early) is NOT killable through any seam available to tests:
    // distinguishing 63 from 64 recv(2) calls on the concrete TcpStream would
    // require syscall-level instrumentation (Linux task IO accounting counts
    // read(2), not recv(2) — verified empirically), kernel socket buffers
    // cannot pre-stage 64 deterministic maximal reads under default tcp_rmem,
    // and the one-read difference is behaviorally absorbed by the next drain
    // pass. Reported as an equivalence-class boundary, not silently skipped.
    //
    // Per-read ceiling: rustls's deframer never offers more than its
    // non-handshake buffer allowance to one read — MAX_WIRE_SIZE =
    // (16_384 + 2_048) + 5 (rustls src/msgs/message/mod.rs) — so a budgeted
    // pass consumes strictly less than cap * ceiling wire bytes, and the
    // plaintext it accumulates is strictly less than the wire it consumed.
    const PER_READ_CEILING: usize = 16_384 + 2_048 + 5;
    // Below any EAGAIN-free budgeted pass (worst case: 64 minimal 4096-byte
    // reads less one trapped partial record still exceeds ~240 KiB).
    const BUDGET_PASS_FLOOR: usize = 200_000;
    let upper = MAX_TLS_READS_PER_DRAIN * PER_READ_CEILING;
    let mut observed: Vec<usize> = Vec::new();
    for _attempt in 0_u8..5 {
        // The client must stay alive through the drain: dropping it would close
        // the socket and let a reset discard the unread flood remainder.
        let (mut server, _client, wire_bytes) = flooded_tls_pair();
        assert!(
            wire_bytes >= upper + 256 * 1024,
            "kernel socket buffers hold too little flood to exercise the budget: {wire_bytes}"
        );

        // The flood is one giant unterminated line; a generous line cap keeps
        // the accumulator in NeedMore so only the read budget can end the pass.
        let limits = Limits::default().with_max_line_bytes(64 * 1024 * 1024);
        let (control_tx, control_rx) = flume::bounded(16);
        let mut accumulator = ControlAccumulator::new();
        server
            .sock
            .set_nonblocking(true)
            .expect("non-blocking server socket");
        let outcome = drain_control_frames(
            &mut server,
            &mut accumulator,
            &control_tx,
            &limits,
            &token(),
        );
        server
            .sock
            .set_nonblocking(false)
            .expect("restore blocking server socket");

        assert!(
            matches!(outcome, ControlDrain::Idle),
            "a budget-bounded drain pass yields Idle so deliveries keep flowing"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "a newline-free flood forwards no control frame"
        );
        let drained = accumulator.buffer.len();
        assert!(
            drained < upper,
            "one drain pass must accumulate strictly less than cap * MAX_WIRE_SIZE \
             plaintext bytes; drained {drained} of a {wire_bytes}-byte flood"
        );
        // The capped pass must leave the rest of the flood on the socket.
        let mut probe = [0_u8; 1];
        let peeked = server
            .sock
            .peek(&mut probe)
            .expect("a budget-capped pass leaves flood bytes unread on the socket");
        assert_eq!(peeked, 1, "the unread flood remainder is peekable");

        observed.push(drained);
        if drained > BUDGET_PASS_FLOOR {
            break;
        }
    }
    let last = observed
        .last()
        .copied()
        .expect("at least one drain pass ran");
    assert!(
        last > BUDGET_PASS_FLOOR,
        "the pass must be ended by the read budget, not by supply; observed \
         plaintext per attempt: {observed:?}"
    );
}

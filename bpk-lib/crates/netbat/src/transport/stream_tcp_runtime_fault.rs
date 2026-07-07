//! NETBAT/2 runtime-fault mapping: a post-subscribe `SubscriptionRuntimeError`
//! from a session poll mapped to its wire outcome. `#[path]`-split from
//! `stream_tcp.rs` to keep that module within the file-size cap; the plaintext
//! and TLS delivery loops both drive these mappings.

use std::io::Write;

use syncbat::{SessionDelivery, SessionError, SubscriptionRuntimeError};

use super::super::error::NetbatError;
use super::super::limits::Limits;
use super::super::stream_frame::SubscriptionToken;
use super::{write_delivery, CURSOR_INVALID_CODE, CURSOR_MISMATCH_CODE};

/// Wire code for an unknown-subscription client fault. Mirrors syncbat's
/// `stream_code::UNKNOWN_SUBSCRIPTION`; that module is crate-private to syncbat,
/// so the token is duplicated here exactly as `CURSOR_*_CODE` already are.
pub(super) const UNKNOWN_SUBSCRIPTION_CODE: &str = "unknown_subscription";
/// Wire code for a malformed / other client-protocol fault. Mirrors syncbat's
/// `stream_code::MALFORMED_STREAM_FRAME`.
pub(super) const MALFORMED_STREAM_FRAME_CODE: &str = "malformed_stream_frame";
/// Wire code for a server-INTERNAL fault, DISTINCT from the client-fault codes
/// above so a peer can tell "the server failed" from "your request was bad".
/// Mirrors syncbat's `stream_code::INTERNAL`.
pub(super) const INTERNAL_ERROR_CODE: &str = "internal_error";

/// Wire code for a post-subscribe runtime poll failure, distinguishing a
/// server-INTERNAL fault from a client/protocol fault (H2) so the peer can tell
/// "the server failed" from "your request was bad". A server-side failure
/// (store read, envelope encoding, worker) routed through a client code would
/// misreport the fault to the peer.
pub(super) fn runtime_failure_code(error: &SubscriptionRuntimeError) -> &'static str {
    match error {
        // Server-internal faults.
        SubscriptionRuntimeError::Store(_)
        | SubscriptionRuntimeError::EnvelopeEncoding(_)
        | SubscriptionRuntimeError::Worker(_) => INTERNAL_ERROR_CODE,
        // Client cursor faults keep their specific codes.
        SubscriptionRuntimeError::CursorInvalid { .. } => CURSOR_INVALID_CODE,
        SubscriptionRuntimeError::CursorMismatch { .. } => CURSOR_MISMATCH_CODE,
        SubscriptionRuntimeError::UnknownSubscription { .. } => UNKNOWN_SUBSCRIPTION_CODE,
        // Remaining client/protocol faults collapse to the malformed code.
        SubscriptionRuntimeError::InvalidSubscriptionId { .. }
        | SubscriptionRuntimeError::DuplicateSubscription { .. }
        | SubscriptionRuntimeError::InvalidRoute { .. }
        | SubscriptionRuntimeError::InvalidConfig { .. }
        | SubscriptionRuntimeError::AckInvalid { .. } => MALFORMED_STREAM_FRAME_CODE,
    }
}

/// Emit the terminal `SUB_ERR` for a post-subscribe runtime poll failure BEFORE
/// the connection is torn down (H3), carrying the server-vs-client wire code
/// from [`runtime_failure_code`] (H2). Shared by the plaintext and TLS delivery
/// loops so both surface the same coded terminal frame to the peer instead of a
/// bare socket close. The caller increments `runtime_failures` once this
/// returns `Ok`.
pub(super) fn write_runtime_failure(
    writer: &mut impl Write,
    subscription_id: &SubscriptionToken,
    error: &SubscriptionRuntimeError,
    limits: &Limits,
) -> Result<(), NetbatError> {
    let delivery = SessionDelivery::Error(SessionError {
        subscription_id: Some(subscription_id.as_str().to_owned()),
        code: runtime_failure_code(error),
        last_delivered_cursor: None,
        last_acked_cursor: None,
        message: b"subscription runtime poll failed".to_vec(),
    });
    write_delivery(writer, &delivery, limits)
}

/// Map a session-poll `SubscriptionRuntimeError` to the [`NetbatError`] the
/// listener surfaces for a server-misconfiguration open failure (an
/// `InvalidConfig` propagated out of `open_session_for_subscribe`).
pub(super) fn map_runtime_error(error: &SubscriptionRuntimeError) -> NetbatError {
    NetbatError::MalformedStreamFrame {
        reason: match error {
            SubscriptionRuntimeError::Store(_) => "store error during stream poll",
            SubscriptionRuntimeError::InvalidSubscriptionId { reason } => reason,
            SubscriptionRuntimeError::DuplicateSubscription { .. } => {
                "duplicate subscription route"
            }
            SubscriptionRuntimeError::InvalidRoute { reason }
            | SubscriptionRuntimeError::InvalidConfig { reason } => reason,
            SubscriptionRuntimeError::UnknownSubscription { .. } => "unknown subscription",
            SubscriptionRuntimeError::CursorInvalid { reason } => reason,
            SubscriptionRuntimeError::CursorMismatch { reason } => reason,
            SubscriptionRuntimeError::EnvelopeEncoding(_) => "envelope encoding failed",
            SubscriptionRuntimeError::Worker(_) => "subscription worker failed",
            SubscriptionRuntimeError::AckInvalid { reason } => reason,
        },
    }
}

//! Store-free receipt verification: verify an append/denial receipt's
//! signature and dispositions from portable inputs — no [`Store`] handle, no
//! filesystem (issue #167).
//!
//! Receipts are the artifact the substrate hands to *other parties*. The
//! [`Store`]-side methods ([`Store::verify_append_receipt`] and friends)
//! verify a receipt *against the committed store*: they look the event up in
//! the index, cross-check the receipt's identity fields, then run the
//! signature check with entry-derived chain metadata. This module packages
//! that final step for a holder with no store: given the ack-shaped receipt
//! fields, the event's chain metadata (`coord`, `kind`, `prev_hash`), and a
//! [`ReceiptVerifyingKeys`] set, [`verify_receipt_claim`] returns the same
//! [`ReceiptVerification`] taxonomy the store returns.
//!
//! There is exactly ONE implementation: the store-side signing registry
//! delegates to the same functions defined here, so a store-free verdict can
//! never disagree with a store-side verdict on the same inputs.
//!
//! What the caller must supply (and trust) instead of the store's index: the
//! chain metadata. `coord`, `kind`, and `prev_hash` are not carried by the
//! receipt — the signature cover binds them, so verification proves the
//! receipt was signed *for that metadata*. Obtain them from the counterparty
//! you are auditing (the proof then shows internal consistency) or from your
//! own trusted record of the chain. What this module deliberately does NOT
//! check (store-bound by nature): event existence in a committed index,
//! disk-position equality, and the denial-kind cross-check.
//!
//! This module is `wasm32`-clean: pure computation over bytes
//! (blake3 + Ed25519), no filesystem or clock contact.
//!
//! [`Store`]: crate::store::Store
//! [`Store::verify_append_receipt`]: crate::store::Store::verify_append_receipt

use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::id::{EntityIdType, EventId};
use crate::store::{EncodedBytes, ExtensionKey, ReceiptVerification, ReceiptVerificationError};
use ed25519_compact::{PublicKey, Signature};
use std::collections::BTreeMap;

const COVER_VERSION_V1: u8 = 0x01;

/// Verifying-key set for receipt verification, keyed by key id.
///
/// A key id is `blake3(public_key_bytes)` — the same derivation the store's
/// signing registry uses — so a set built from the same public keys resolves
/// the same receipts. An [`empty`](Self::empty) set expresses the
/// unsigned-operation regime: unsigned receipts verify as
/// [`ReceiptVerification::UnsignedAccepted`]; with any key registered they are
/// rejected instead.
#[derive(Clone, Debug, Default)]
pub struct ReceiptVerifyingKeys {
    keys: BTreeMap<[u8; 32], [u8; 32]>,
}

impl ReceiptVerifyingKeys {
    /// The empty key set: the unsigned-operation regime.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a key set from raw Ed25519 public keys (32 bytes each); key ids
    /// are derived as `blake3(public_key)`.
    #[must_use]
    pub fn from_public_keys<I>(public_keys: I) -> Self
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        let keys = public_keys
            .into_iter()
            .map(|public_key| (key_id_for_public_key(&public_key), public_key))
            .collect();
        Self { keys }
    }

    /// Whether no verifying key is registered (the unsigned regime).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn lookup(&self, key_id: &[u8; 32]) -> Option<&[u8; 32]> {
        self.keys.get(key_id)
    }
}

/// The ack-shaped receipt fields a store-free verifier holds.
///
/// This mirrors the wire form of a receipt acknowledgment (the parameter list
/// of [`Store::verify_append_receipt_wire_detailed`]): receipts themselves are
/// `#[non_exhaustive]` and carry a store-internal disk position, so a
/// downstream holder builds a claim from the fields it was handed instead.
/// One claim shape serves append AND denial receipts — their signature covers
/// are identical; pass the denial entry's chain metadata for a denial.
///
/// [`Store::verify_append_receipt_wire_detailed`]: crate::store::Store::verify_append_receipt_wire_detailed
#[derive(Clone, Debug)]
pub struct ReceiptClaim {
    /// The event id the receipt acknowledges.
    pub event_id: EventId,
    /// The commit-order sequence the receipt claims.
    pub global_sequence: u64,
    /// The Blake3 content hash the receipt claims.
    pub content_hash: [u8; 32],
    /// Signing key id (`blake3(public_key)`), or all-zero for the unsigned
    /// sentinel.
    pub key_id: [u8; 32],
    /// The Ed25519 signature over the receipt cover, when signed.
    pub signature: Option<[u8; 64]>,
    /// Receipt extensions, exactly as carried by the receipt (they are bound
    /// by the signature cover).
    pub extensions: BTreeMap<ExtensionKey, EncodedBytes>,
}

/// Verify a receipt claim from portable inputs, without a store.
///
/// `coord`, `kind`, and `prev_hash` are the event's chain metadata (the
/// "expected frontier" — the hash-chain predecessor the receipt was minted
/// against); the signature cover binds all of them, so a verdict of
/// [`ReceiptVerification::Signed`] proves the receipt matches *that* metadata
/// under one of `keys`. Semantics are identical to store-side verification —
/// the store's own methods delegate to this implementation.
///
/// ```
/// use batpak::coordinate::Coordinate;
/// use batpak::event::EventKind;
/// use batpak::id::EventId;
/// use batpak::store::{
///     verify_receipt_claim, ReceiptClaim, ReceiptVerification, ReceiptVerifyingKeys,
/// };
/// use std::collections::BTreeMap;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // An unsigned receipt (sentinel key, no signature) from a store with no
/// // signing configured verifies as UnsignedAccepted under the empty key set.
/// let claim = ReceiptClaim {
///     event_id: EventId::from(7u128),
///     global_sequence: 1,
///     content_hash: [0x22; 32],
///     key_id: [0; 32],
///     signature: None,
///     extensions: BTreeMap::new(),
/// };
/// let coord = Coordinate::new("player:alice", "room:dungeon")?;
/// let verdict = verify_receipt_claim(
///     &claim,
///     &coord,
///     EventKind::custom(0xF, 1),
///     [0; 32],
///     &ReceiptVerifyingKeys::empty(),
/// );
/// assert_eq!(verdict, ReceiptVerification::UnsignedAccepted);
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn verify_receipt_claim(
    claim: &ReceiptClaim,
    coord: &Coordinate,
    kind: EventKind,
    prev_hash: [u8; 32],
    keys: &ReceiptVerifyingKeys,
) -> ReceiptVerification {
    verify_claim_parts(
        ClaimParts {
            event_id: claim.event_id.as_u128(),
            global_sequence: claim.global_sequence,
            content_hash: claim.content_hash,
            key_id: claim.key_id,
            signature: claim.signature,
            extensions: &claim.extensions,
            coord,
            kind,
            prev_hash,
        },
        keys,
    )
}

/// Borrowed view of one receipt claim plus its chain metadata — the input to
/// the single verification core. Field-borrowed so the store-side registry
/// adapters avoid cloning receipt extensions.
#[derive(Clone, Copy)]
pub(crate) struct ClaimParts<'a> {
    pub(crate) event_id: u128,
    pub(crate) global_sequence: u64,
    pub(crate) content_hash: [u8; 32],
    pub(crate) key_id: [u8; 32],
    pub(crate) signature: Option<[u8; 64]>,
    pub(crate) extensions: &'a BTreeMap<ExtensionKey, EncodedBytes>,
    pub(crate) coord: &'a Coordinate,
    pub(crate) kind: EventKind,
    pub(crate) prev_hash: [u8; 32],
}

/// The single verification core both the public store-free surface and the
/// store-side signing registry run.
pub(crate) fn verify_claim_parts(
    parts: ClaimParts<'_>,
    keys: &ReceiptVerifyingKeys,
) -> ReceiptVerification {
    // Sentinel-signed receipts (no signature, no key) bypass the cover
    // rebuild: signing was either not configured or it downgraded due to a
    // coordinate/extension encoding failure. Their validity is a property of
    // the key-set state, not of any computed cover.
    if parts.signature.is_none() && parts.key_id == [0; 32] {
        return if keys.is_empty() {
            ReceiptVerification::UnsignedAccepted
        } else {
            ReceiptVerification::Invalid(ReceiptVerificationError::UnsignedReceiptRejected)
        };
    }
    let cover = match cover_bytes(
        parts.event_id,
        parts.global_sequence,
        parts.coord,
        parts.kind,
        parts.prev_hash,
        parts.content_hash,
        parts.extensions,
    ) {
        Ok(cover) => cover,
        Err(error) => {
            tracing::error!(error = %error, "failed to rebuild receipt signature cover");
            return ReceiptVerification::Invalid(ReceiptVerificationError::CoverBuildFailed {
                reason: error.to_string(),
            });
        }
    };
    verify_signature_with_keys(keys, parts.key_id, parts.signature, cover)
}

/// Signature/unsigned dispositions over a rebuilt cover — the one place the
/// `Signed` / `UnsignedAccepted` / `Invalid` taxonomy is decided.
pub(crate) fn verify_signature_with_keys(
    keys: &ReceiptVerifyingKeys,
    key_id: [u8; 32],
    signature: Option<[u8; 64]>,
    cover: [u8; 32],
) -> ReceiptVerification {
    let Some(signature_bytes) = signature else {
        return if key_id == [0; 32] && keys.is_empty() {
            ReceiptVerification::UnsignedAccepted
        } else if key_id == [0; 32] {
            ReceiptVerification::Invalid(ReceiptVerificationError::UnsignedReceiptRejected)
        } else {
            ReceiptVerification::Invalid(ReceiptVerificationError::MissingSignature)
        };
    };
    if key_id == [0; 32] {
        return ReceiptVerification::Invalid(ReceiptVerificationError::ZeroKeyWithSignature);
    };
    let Some(public_key_bytes) = keys.lookup(&key_id) else {
        return ReceiptVerification::Invalid(ReceiptVerificationError::UnknownSigningKey);
    };
    let signature = Signature::new(signature_bytes);
    if PublicKey::new(*public_key_bytes)
        .verify(cover, &signature)
        .is_ok()
    {
        ReceiptVerification::Signed
    } else {
        ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
    }
}

/// Derive a receipt key id from an Ed25519 public key: `blake3(public_key)`.
pub(crate) fn key_id_for_public_key(public_key: &[u8; 32]) -> [u8; 32] {
    crate::event::hash::compute_hash(public_key)
}

#[derive(Debug)]
pub(crate) enum CoverBuildError {
    CoordinateEncoding(rmp_serde::encode::Error),
    ExtensionsEncoding(rmp_serde::encode::Error),
}

impl std::fmt::Display for CoverBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoordinateEncoding(error) => {
                write!(
                    f,
                    "coordinate encoding failed while building receipt cover: {error}"
                )
            }
            Self::ExtensionsEncoding(error) => {
                write!(
                    f,
                    "extension encoding failed while building receipt cover: {error}"
                )
            }
        }
    }
}

impl std::error::Error for CoverBuildError {}

/// The canonical signed-cover layout (COVER v1): blake3 over
/// `version || event_id_le || sequence_le || canonical(coord) || kind_le ||
/// prev_hash || content_hash || canonical(extensions)`. Single-sourced here so
/// the signing path and both verification surfaces can never diverge.
pub(crate) fn cover_bytes(
    event_id: u128,
    sequence: u64,
    coord: &Coordinate,
    kind: EventKind,
    prev_hash: [u8; 32],
    content_hash: [u8; 32],
    extensions: &BTreeMap<ExtensionKey, EncodedBytes>,
) -> Result<[u8; 32], CoverBuildError> {
    let mut cover = Vec::new();
    cover.push(COVER_VERSION_V1);
    cover.extend_from_slice(&event_id.to_le_bytes());
    cover.extend_from_slice(&sequence.to_le_bytes());
    let coord_bytes =
        crate::canonical::to_bytes(coord).map_err(CoverBuildError::CoordinateEncoding)?;
    cover.extend_from_slice(&coord_bytes);
    let raw_kind = kind.as_raw_u16();
    cover.extend_from_slice(&raw_kind.to_le_bytes());
    cover.extend_from_slice(&prev_hash);
    cover.extend_from_slice(&content_hash);
    let extension_bytes =
        crate::canonical::to_bytes(extensions).map_err(CoverBuildError::ExtensionsEncoding)?;
    cover.extend_from_slice(&extension_bytes);
    Ok(crate::event::hash::compute_hash(&cover))
}

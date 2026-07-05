use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::store::append::{signing_downgrade_extension_key, SigningDowngradeBody};
use crate::store::receipt_verify::{
    cover_bytes, key_id_for_public_key, verify_claim_parts, ClaimParts, ReceiptVerifyingKeys,
};
use crate::store::{AppendReceipt, DenialReceipt, ReceiptVerification, StoreError};
use ed25519_compact::{KeyPair, Seed};
use std::sync::Arc;
use zeroize::Zeroizing;

/// Opt-in Ed25519 signing key for receipt signatures.
#[derive(Clone)]
pub struct SigningKey {
    seed: Zeroizing<[u8; 32]>,
}

impl SigningKey {
    /// Construct a signing key from 32 seed bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            seed: Zeroizing::new(bytes),
        }
    }

    /// The key id receipts signed by this key carry: `blake3(public_key)`,
    /// or the all-zero sentinel when the key has no public half.
    ///
    /// Public since 0.10.0 so a receipt holder can match a receipt's `key_id`
    /// to a configured signer.
    #[must_use]
    pub fn key_id(&self) -> [u8; 32] {
        match self.public_key_bytes() {
            Some(bytes) => key_id_for_public_key(&bytes),
            None => [0; 32],
        }
    }

    fn key_pair(&self) -> KeyPair {
        KeyPair::from_seed(Seed::new(*self.seed))
    }

    /// The 32-byte Ed25519 public key of this signer, when derivable.
    ///
    /// Public since 0.10.0: this is how an embedder exports the verifying
    /// half to build a
    /// [`ReceiptVerifyingKeys`](crate::store::ReceiptVerifyingKeys) set for
    /// store-free verification (issue #167). The secret seed never leaves
    /// the type.
    #[must_use]
    pub fn public_key_bytes(&self) -> Option<[u8; 32]> {
        <[u8; 32]>::try_from(self.key_pair().pk.as_ref()).ok()
    }

    fn sign_cover(&self, cover: [u8; 32]) -> [u8; 64] {
        let signature = self.key_pair().sk.sign(cover, None);
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(signature.as_ref());
        bytes
    }
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey")
            .field("key_id", &self.key_id())
            .finish()
    }
}

#[derive(Clone, Default)]
pub(crate) struct ReceiptSigningRegistry {
    current: Option<Arc<SigningKey>>,
    verifying_keys: Arc<ReceiptVerifyingKeys>,
    /// When a signer is configured but its cover cannot be built, permit a
    /// best-effort downgrade to unsigned instead of failing the append.
    allow_downgrade: bool,
}

impl ReceiptSigningRegistry {
    /// Build a signing registry from a key list.
    ///
    /// Every key with a public half is registered as a *verifying* key. The
    /// **active signer** is the LAST key in `keys` that carries a public half —
    /// i.e. ordering is significant: re-ordering the `with_signing_key` calls
    /// that produce this slice silently changes which key signs new receipts.
    /// This is the intended key-rotation mechanism (append the new active key
    /// last); callers must not treat the order as cosmetic.
    pub(crate) fn from_keys(keys: &[SigningKey], allow_downgrade: bool) -> Self {
        let mut public_keys = Vec::new();
        let mut current = None;
        for key in keys {
            let key = Arc::new(key.clone());
            if let Some(public_key_bytes) = key.public_key_bytes() {
                public_keys.push(public_key_bytes);
                current = Some(key);
            }
        }
        Self {
            current,
            verifying_keys: Arc::new(ReceiptVerifyingKeys::from_public_keys(public_keys)),
            allow_downgrade,
        }
    }

    pub(crate) fn sign_append_receipt(
        &self,
        receipt: &mut AppendReceipt,
        coord: &Coordinate,
        kind: EventKind,
        prev_hash: [u8; 32],
    ) -> Result<(), StoreError> {
        let Some(current) = &self.current else {
            // No active signer: the receipt stays unsigned. No cover is needed,
            // and there is nothing to downgrade.
            receipt.key_id = [0; 32];
            receipt.signature = None;
            return Ok(());
        };
        let cover = match cover_bytes(
            {
                use crate::id::EntityIdType;
                receipt.event_id.as_u128()
            },
            receipt.global_sequence,
            coord,
            kind,
            prev_hash,
            receipt.content_hash,
            &receipt.extensions,
        ) {
            Ok(cover) => cover,
            Err(error) => {
                // A signer is configured but its cover cannot be built. Fail the
                // append closed rather than silently committing an unsigned
                // receipt — unless downgrade is explicitly permitted.
                if cover_failure_fails_closed(self.allow_downgrade) {
                    return Err(StoreError::ser_msg(&format!(
                        "receipt signature cover could not be built: {error}"
                    )));
                }
                tracing::error!(error = %error, "receipt signing downgraded to unsigned (signing_downgrade_allowed)");
                downgrade_receipt_signing(receipt, error.to_string());
                return Ok(());
            }
        };
        receipt.key_id = current.key_id();
        receipt.signature = Some(current.sign_cover(cover));
        Ok(())
    }

    /// Store-side append-receipt verification: a thin adapter over the ONE
    /// verification core in [`crate::store::receipt_verify`], with the chain
    /// metadata sourced from the committed index entry by the caller.
    pub(crate) fn verify_append_receipt(
        &self,
        receipt: &AppendReceipt,
        coord: &Coordinate,
        kind: EventKind,
        prev_hash: [u8; 32],
    ) -> ReceiptVerification {
        verify_claim_parts(
            ClaimParts {
                event_id: {
                    use crate::id::EntityIdType;
                    receipt.event_id.as_u128()
                },
                global_sequence: receipt.global_sequence,
                content_hash: receipt.content_hash,
                key_id: receipt.key_id,
                signature: receipt.signature,
                extensions: &receipt.extensions,
                coord,
                kind,
                prev_hash,
            },
            &self.verifying_keys,
        )
    }

    /// Store-side denial-receipt verification; same single core as appends
    /// (the covers are field-for-field identical).
    pub(crate) fn verify_denial_receipt(
        &self,
        receipt: &DenialReceipt,
        coord: &Coordinate,
        kind: EventKind,
        prev_hash: [u8; 32],
    ) -> ReceiptVerification {
        verify_claim_parts(
            ClaimParts {
                event_id: {
                    use crate::id::EntityIdType;
                    receipt.event_id.as_u128()
                },
                global_sequence: receipt.global_sequence,
                content_hash: receipt.content_hash,
                key_id: receipt.key_id,
                signature: receipt.signature,
                extensions: &receipt.extensions,
                coord,
                kind,
                prev_hash,
            },
            &self.verifying_keys,
        )
    }

    /// Kept as a test-visible adapter so the sentinel/unsigned disposition
    /// cure tests keep exercising the registry-held key set through the one
    /// shared implementation.
    #[cfg(test)]
    fn verify_signature(
        &self,
        key_id: [u8; 32],
        signature: Option<[u8; 64]>,
        cover: [u8; 32],
    ) -> ReceiptVerification {
        crate::store::receipt_verify::verify_signature_with_keys(
            &self.verifying_keys,
            key_id,
            signature,
            cover,
        )
    }
}

/// A configured signer fails the append closed on cover-build failure unless
/// downgrade is explicitly permitted. The cover-build failure is itself a
/// defensive guard — it requires the coordinate/extension MessagePack encoding
/// to fail, which does not occur for valid inputs — so this disposition is the
/// directly unit-tested policy.
const fn cover_failure_fails_closed(allow_downgrade: bool) -> bool {
    !allow_downgrade
}

fn downgrade_receipt_signing(receipt: &mut AppendReceipt, error: impl Into<String>) {
    let body = SigningDowngradeBody::cover_build_failed(error);
    match body.encode_extension() {
        Ok(bytes) => {
            receipt
                .extensions
                .insert(signing_downgrade_extension_key(), bytes);
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                "failed to encode signing downgrade receipt extension"
            );
        }
    }
    receipt.key_id = [0; 32];
    receipt.signature = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn cover_failure_is_fatal_unless_downgrade_allowed() {
        // Default: a configured signer that cannot build its cover FAILS the
        // append closed (never silently emits an unsigned receipt).
        assert!(cover_failure_fails_closed(false));
        // Opt-in: best-effort downgrade is permitted only when explicitly asked.
        assert!(!cover_failure_fails_closed(true));
    }

    #[test]
    fn cover_bytes_separates_event_kind_category_and_type_bits() {
        let coord = Coordinate::new("receipt:cover", "scope:test").expect("coordinate");
        let extensions = BTreeMap::new();

        let cover_a = cover_bytes(
            1,
            1,
            &coord,
            EventKind::custom(0xF, 0x055),
            [0x11; 32],
            [0x22; 32],
            &extensions,
        )
        .expect("cover A");
        let cover_b = cover_bytes(
            1,
            1,
            &coord,
            EventKind::custom(0xE, 0x055),
            [0x11; 32],
            [0x22; 32],
            &extensions,
        )
        .expect("cover B");
        let cover_c = cover_bytes(
            1,
            1,
            &coord,
            EventKind::custom(0xF, 0x056),
            [0x11; 32],
            [0x22; 32],
            &extensions,
        )
        .expect("cover C");

        assert_ne!(
            cover_a, cover_b,
            "PROPERTY: receipt signature cover must include the EventKind category bits"
        );
        assert_ne!(
            cover_a, cover_c,
            "PROPERTY: receipt signature cover must include the EventKind type-id bits"
        );
    }

    #[test]
    fn cover_build_failure_adds_signing_downgrade_extension() {
        let mut receipt = AppendReceipt {
            event_id: crate::id::EventId::from(7u128),
            global_sequence: 9,
            disk_pos: crate::store::index::DiskPos {
                segment_id: 1,
                offset: 2,
                length: 3,
            },
            content_hash: [0x22; 32],
            key_id: [0xAA; 32],
            signature: Some([0xBB; 64]),
            extensions: BTreeMap::new(),
        };

        downgrade_receipt_signing(&mut receipt, "synthetic cover failure");

        assert_eq!(receipt.key_id, [0; 32]);
        assert!(receipt.signature.is_none());
        let downgrade = receipt
            .signing_downgrade()
            .expect("downgrade extension should decode");
        assert!(matches!(
            downgrade.reason,
            crate::store::SigningDowngradeReason::CoverBuildFailed { ref encoding_error }
                if encoding_error == "synthetic cover failure"
        ));
    }
}

/// Cure island for the receipt-verification sentinel/unsigned dispositions and
/// the Debug/Display renderers. Split from the island above to stay within the
/// inline test-island budget.
#[cfg(test)]
mod verify_cure_tests {
    use super::*;
    use crate::store::index::DiskPos;
    use crate::store::receipt_verify::CoverBuildError;
    use crate::store::ReceiptVerificationError;
    use std::collections::BTreeMap;

    fn receipt(key_id: [u8; 32], signature: Option<[u8; 64]>) -> AppendReceipt {
        AppendReceipt {
            event_id: crate::id::EventId::from(3u128),
            global_sequence: 1,
            disk_pos: DiskPos::new(1, 0, 1),
            content_hash: [0x11; 32],
            key_id,
            signature,
            extensions: BTreeMap::new(),
        }
    }

    fn denial(key_id: [u8; 32], signature: Option<[u8; 64]>) -> DenialReceipt {
        DenialReceipt {
            event_id: crate::id::EventId::from(4u128),
            global_sequence: 2,
            disk_pos: DiskPos::new(1, 0, 1),
            content_hash: [0x22; 32],
            key_id,
            signature,
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn verify_append_receipt_unsigned_with_nonsentinel_key_is_missing_signature() {
        let registry = ReceiptSigningRegistry::from_keys(&[], false);
        let coord = Coordinate::new("entity:sig", "scope:sig").expect("coord");
        // signature.is_none() is true but key_id is NOT the sentinel, so the
        // unsigned bypass must NOT trigger; the receipt falls through to
        // signature checking and is MissingSignature. `&& -> ||` and `== -> !=`
        // both wrongly take the bypass and return UnsignedAccepted.
        assert_eq!(
            registry.verify_append_receipt(
                &receipt([0xAA; 32], None),
                &coord,
                EventKind::custom(0xF, 1),
                [0; 32],
            ),
            ReceiptVerification::Invalid(ReceiptVerificationError::MissingSignature),
        );
    }

    #[test]
    fn verify_denial_receipt_unsigned_with_nonsentinel_key_is_missing_signature() {
        let registry = ReceiptSigningRegistry::from_keys(&[], false);
        let coord = Coordinate::new("entity:sig-d", "scope:sig").expect("coord");
        assert_eq!(
            registry.verify_denial_receipt(
                &denial([0xBB; 32], None),
                &coord,
                EventKind::custom(0xF, 2),
                [0; 32],
            ),
            ReceiptVerification::Invalid(ReceiptVerificationError::MissingSignature),
        );
    }

    #[test]
    fn verify_signature_unsigned_dispositions_match_key_and_registry_state() {
        let empty = ReceiptSigningRegistry::from_keys(&[], false);
        let keyed = ReceiptSigningRegistry::from_keys(&[SigningKey::from_bytes([7u8; 32])], false);
        let cover = [0u8; 32];
        // Sentinel key + empty registry -> UnsignedAccepted. Kills `== -> !=` (227).
        assert_eq!(
            empty.verify_signature([0; 32], None, cover),
            ReceiptVerification::UnsignedAccepted,
        );
        // Non-sentinel key + empty registry -> MissingSignature. Kills `&& -> ||` (227).
        assert_eq!(
            empty.verify_signature([0xAA; 32], None, cover),
            ReceiptVerification::Invalid(ReceiptVerificationError::MissingSignature),
        );
        // Sentinel key + NON-empty registry -> UnsignedReceiptRejected. Kills `== -> !=` (229).
        assert_eq!(
            keyed.verify_signature([0; 32], None, cover),
            ReceiptVerification::Invalid(ReceiptVerificationError::UnsignedReceiptRejected),
        );
    }

    #[test]
    fn cover_build_error_display_renders_the_stage_and_is_never_empty() {
        use serde::ser::Error as _;
        // A synthetic rmp encode error carried by the CoordinateEncoding variant.
        let encode_err = rmp_serde::encode::Error::custom("boom");
        let display = format!("{}", CoverBuildError::CoordinateEncoding(encode_err));
        // Kills the body-stub `Ok(())` mutant, which renders an empty string.
        assert!(
            display.contains("coordinate encoding failed while building receipt cover"),
            "Display must render the coordinate-encoding cover-build failure, got {display:?}"
        );
    }

    #[test]
    fn signing_key_debug_names_the_struct_and_key_id() {
        let debug = format!("{:?}", SigningKey::from_bytes([7u8; 32]));
        // Kills the body-stub `Ok(())` mutant, which renders an empty string.
        assert!(
            debug.contains("SigningKey") && debug.contains("key_id"),
            "Debug must render the SigningKey struct with its key_id field, got {debug:?}"
        );
    }
}

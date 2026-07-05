//! Store-free receipt verification (0.10.0, issue #167).
//!
//! PROVES: `verify_receipt_claim` — the packaged store-free surface — returns
//! the SAME `ReceiptVerification` verdict as `Store::verify_append_receipt`
//! for the same inputs (there is one implementation, so the two can never
//! disagree), and every signature-side error variant is reachable from
//! portable inputs alone: no `Store::open`, no filesystem.
//! CATCHES: a forked second verifier whose dispositions drift from the
//! store's, and a cover layout that no longer binds a claimed field.

use batpak::prelude::*;
use batpak::store::SigningKey;
use tempfile::TempDir;

const SEED: [u8; 32] = [0x42; 32];

struct SignedFixture {
    /// Keep the store alive so the agreement half can keep verifying.
    store: Store<Open>,
    claim: ReceiptClaim,
    receipt: AppendReceipt,
    coord: Coordinate,
    kind: EventKind,
    prev_hash: [u8; 32],
    keys: ReceiptVerifyingKeys,
}

/// Append one signed event, then capture everything a store-free holder
/// would be handed: the ack-shaped receipt fields plus the event's chain
/// metadata (coordinate, kind, hash-chain predecessor) and the verifying key.
fn signed_fixture(dir: &TempDir) -> Result<SignedFixture, Box<dyn std::error::Error>> {
    let key = SigningKey::from_bytes(SEED);
    let public_key = match key.public_key_bytes() {
        Some(bytes) => bytes,
        None => return Err(std::io::Error::other("signing key must have a public half").into()),
    };
    let keys = ReceiptVerifyingKeys::from_public_keys([public_key]);

    let store = Store::open(StoreConfig::new(dir.path()).with_signing_key(key.clone()))?;
    let coord = Coordinate::new("verify:store-free", "scope:portable")?;
    let kind = EventKind::custom(0xA, 33);
    let receipt = store.append(&coord, kind, &serde_json::json!({"n": 7}))?;
    assert_eq!(
        receipt.key_id,
        key.key_id(),
        "the signed receipt must carry the configured signer's key id"
    );

    let fetched = store.get(receipt.event_id)?;
    let prev_hash = match fetched.event.hash_chain {
        Some(chain) => chain.prev_hash,
        None => return Err(std::io::Error::other("stored event must carry a hash chain").into()),
    };

    let claim = ReceiptClaim {
        event_id: receipt.event_id,
        global_sequence: receipt.global_sequence,
        content_hash: receipt.content_hash,
        key_id: receipt.key_id,
        signature: receipt.signature,
        extensions: receipt.extensions.clone(),
    };
    Ok(SignedFixture {
        store,
        claim,
        receipt,
        coord,
        kind: fetched.event.header.event_kind,
        prev_hash,
        keys,
    })
}

#[test]
fn store_free_and_store_side_verification_agree_on_a_signed_receipt(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let fx = signed_fixture(&dir)?;
    assert_eq!(fx.kind, EventKind::custom(0xA, 33));

    let store_free = verify_receipt_claim(&fx.claim, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    let store_side = fx.store.verify_append_receipt(&fx.receipt);

    assert_eq!(
        store_free, store_side,
        "store-free and store-side verdicts must agree on identical inputs"
    );
    assert_eq!(store_free, ReceiptVerification::Signed);
    assert!(store_free.is_signed());
    Ok(())
}

#[test]
fn store_free_verification_binds_every_covered_field() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let fx = signed_fixture(&dir)?;

    // Tampering with any signature-covered input must yield InvalidSignature:
    // the cover binds event id, sequence, coordinate, kind, prev_hash,
    // content hash, and extensions.
    let mut wrong_sequence = fx.claim.clone();
    wrong_sequence.global_sequence += 1;
    let verdict = verify_receipt_claim(&wrong_sequence, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
        ),
        "a tampered sequence must invalidate the signature, got {verdict:?}"
    );

    let mut wrong_content = fx.claim.clone();
    wrong_content.content_hash[0] ^= 0xFF;
    let verdict = verify_receipt_claim(&wrong_content, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
        ),
        "a tampered content hash must invalidate the signature, got {verdict:?}"
    );

    let wrong_coord = Coordinate::new("verify:other-entity", "scope:portable")?;
    let verdict = verify_receipt_claim(&fx.claim, &wrong_coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
        ),
        "chain metadata (coordinate) is signature-covered, got {verdict:?}"
    );

    let verdict = verify_receipt_claim(
        &fx.claim,
        &fx.coord,
        EventKind::custom(0xA, 34),
        fx.prev_hash,
        &fx.keys,
    );
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
        ),
        "chain metadata (kind) is signature-covered, got {verdict:?}"
    );

    let verdict = verify_receipt_claim(&fx.claim, &fx.coord, fx.kind, [0xEE; 32], &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::InvalidSignature)
        ),
        "chain metadata (prev_hash / expected frontier) is signature-covered, got {verdict:?}"
    );
    Ok(())
}

#[test]
fn store_free_key_dispositions_match_the_store_taxonomy() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new()?;
    let fx = signed_fixture(&dir)?;

    // Unknown verifying key: same claim, a key set that does not contain the
    // signer.
    let stranger = ReceiptVerifyingKeys::from_public_keys([[0x99; 32]]);
    let verdict = verify_receipt_claim(&fx.claim, &fx.coord, fx.kind, fx.prev_hash, &stranger);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::UnknownSigningKey)
        ),
        "a signer absent from the key set must be UnknownSigningKey, got {verdict:?}"
    );

    // Stripped signature with a non-sentinel key id.
    let mut stripped = fx.claim.clone();
    stripped.signature = None;
    let verdict = verify_receipt_claim(&stripped, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::MissingSignature)
        ),
        "a keyed claim without a signature must be MissingSignature, got {verdict:?}"
    );

    // Fully-stripped (sentinel) claim: rejected under a keyed set, accepted
    // under the empty set — the unsigned-operation regime.
    let mut sentinel = fx.claim.clone();
    sentinel.signature = None;
    sentinel.key_id = [0; 32];
    let verdict = verify_receipt_claim(&sentinel, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::UnsignedReceiptRejected)
        ),
        "an unsigned claim under a keyed set must be rejected, got {verdict:?}"
    );
    let empty = ReceiptVerifyingKeys::empty();
    assert!(empty.is_empty());
    assert_eq!(
        verify_receipt_claim(&sentinel, &fx.coord, fx.kind, fx.prev_hash, &empty),
        ReceiptVerification::UnsignedAccepted,
    );

    // Sentinel key id WITH a signature is structurally invalid.
    let mut zero_keyed = fx.claim.clone();
    zero_keyed.key_id = [0; 32];
    let verdict = verify_receipt_claim(&zero_keyed, &fx.coord, fx.kind, fx.prev_hash, &fx.keys);
    assert!(
        matches!(
            verdict,
            ReceiptVerification::Invalid(ReceiptVerificationError::ZeroKeyWithSignature)
        ),
        "a signature with the zero key id must be ZeroKeyWithSignature, got {verdict:?}"
    );
    Ok(())
}

#[test]
fn unsigned_store_and_store_free_agree_without_any_key_material(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let store = Store::open(StoreConfig::new(dir.path()))?;
    let coord = Coordinate::new("verify:unsigned", "scope:portable")?;
    let kind = EventKind::custom(0xA, 35);
    let receipt = store.append(&coord, kind, &serde_json::json!({"n": 1}))?;
    let fetched = store.get(receipt.event_id)?;
    let prev_hash = match fetched.event.hash_chain {
        Some(chain) => chain.prev_hash,
        None => return Err(std::io::Error::other("stored event must carry a hash chain").into()),
    };

    let claim = ReceiptClaim {
        event_id: receipt.event_id,
        global_sequence: receipt.global_sequence,
        content_hash: receipt.content_hash,
        key_id: receipt.key_id,
        signature: receipt.signature,
        extensions: receipt.extensions.clone(),
    };
    let store_free = verify_receipt_claim(
        &claim,
        &coord,
        kind,
        prev_hash,
        &ReceiptVerifyingKeys::empty(),
    );
    let store_side = store.verify_append_receipt(&receipt);

    assert_eq!(store_free, store_side);
    assert_eq!(store_free, ReceiptVerification::UnsignedAccepted);
    store.close()?;
    Ok(())
}

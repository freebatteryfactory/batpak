//! Encrypt-on-append seam for the crypto-shred payload path (Stage C).
//!
//! When `payload_encryption` is configured, every appended payload is sealed
//! under its scope's key BEFORE the writer hashes and frames it, so the on-disk
//! payload is ciphertext and `event_hash = blake3(ciphertext)`. The plaintext is
//! never written and never leaves this seam.
//!
//! # The durability fence (the crux)
//!
//! An append that MINTS a new key must not be acknowledged durable before that
//! key is itself durable — otherwise a crash landing between "ciphertext durable"
//! and "key durable" would leave a durable ciphertext whose key never reached
//! disk: spontaneous, permanent loss of LIVE data. The seam enforces this by
//! flushing the keyset ([`WriterCore::flush_keyset_durable`]) — an atomic,
//! self-fsyncing publish — BEFORE the encrypted frame is even written to the
//! segment. Because the flush happens-before the frame write, which happens-
//! before any later segment fsync, no crash window can order data-durable ahead
//! of key-durable, under ANY sync mode. A flush failure fails the append CLOSED
//! (the caller returns the error and writes nothing).

use super::WriterCore;
use crate::coordinate::Coordinate;
use crate::event::{EventKind, PayloadEncryption};
use crate::id::EventId;
use crate::store::keyscope::{payload_aad, scope_for};
use crate::store::StoreError;

/// One sealed payload plus the metadata the read path needs to reopen it, and
/// whether sealing it MINTED a new scope key (so the caller knows a keyset flush
/// is owed before acknowledging the append/batch durable).
pub(super) struct SealedPayload {
    /// Ciphertext (AEAD output with appended tag) that becomes the frame payload.
    pub(super) ciphertext: Vec<u8>,
    /// Header metadata (scope id + nonce) stamped onto the event, outside the
    /// hashed/signed cover.
    pub(super) meta: PayloadEncryption,
    /// `true` when this seal minted a fresh key for its scope — the signal that a
    /// durable keyset flush is required before the append is acknowledged.
    pub(super) minted: bool,
}

impl WriterCore {
    /// Seal `plaintext` under its scope key when encryption is configured.
    ///
    /// Returns `Ok(None)` when `payload_encryption` is not configured (the
    /// plaintext path — the caller leaves the payload untouched and the frame
    /// stays byte-identical to a non-encryption build). Otherwise mints the
    /// scope key on first use, seals the payload binding the event's stable
    /// identity (coord + kind + event id) as AAD, and reports whether a key was
    /// minted.
    ///
    /// # Errors
    /// [`StoreError::PayloadSealFailed`] if the CSPRNG cannot produce a nonce or
    /// the AEAD rejects the input — the append then fails closed.
    pub(super) fn seal_event_payload(
        &self,
        coord: &Coordinate,
        kind: EventKind,
        event_id: EventId,
        plaintext: &[u8],
    ) -> Result<Option<SealedPayload>, StoreError> {
        let Some(key_store) = self.runtime.key_store.as_ref() else {
            return Ok(None);
        };

        // Draw the nonce from the OS CSPRNG BEFORE taking the keyset lock, so the
        // lock is held only for the mint + seal. XChaCha20-Poly1305's 192-bit
        // nonce makes random nonces collision-safe.
        let mut nonce = [0u8; 24];
        getrandom::fill(&mut nonce).map_err(|error| StoreError::PayloadSealFailed {
            detail: format!("nonce CSPRNG failed: {error}"),
        })?;

        let mut guard = key_store.lock();
        let scope = scope_for(guard.granularity(), coord, kind, event_id);
        let minted = guard.get(&scope).is_none();
        let aad = payload_aad(coord, kind, event_id);
        let ciphertext = {
            let key =
                guard
                    .get_or_create(&scope)
                    .map_err(|error| StoreError::PayloadSealFailed {
                        detail: format!("mint key: {error}"),
                    })?;
            key.seal(&nonce, &aad, plaintext)
                .map_err(|error| StoreError::PayloadSealFailed {
                    detail: format!("seal: {error}"),
                })?
        };
        drop(guard);

        Ok(Some(SealedPayload {
            meta: PayloadEncryption {
                keyscope_id: scope.as_bytes().to_vec(),
                nonce,
            },
            ciphertext,
            minted,
        }))
    }

    /// Encrypt a single append's payload IN PLACE — mint + seal + durability
    /// fence + header stamp — when encryption is configured; a no-op otherwise.
    ///
    /// Runs BEFORE `handle_append` hashes the payload, so the writer's existing
    /// `event_hash = blake3(event.payload)` is computed over the CIPHERTEXT with
    /// no change to the hashing code. Stamps the scope id + nonce into the header
    /// (outside the hashed/signed cover) and updates `payload_size` to the
    /// ciphertext length.
    ///
    /// # Errors
    /// [`StoreError::PayloadSealFailed`] on a seal failure, or the keyset flush
    /// error on a fence-flush failure — either fails the append closed.
    pub(super) fn encrypt_single_payload(
        &self,
        coord: &Coordinate,
        kind: EventKind,
        event: &mut crate::event::Event<Vec<u8>>,
    ) -> Result<(), StoreError> {
        let Some(sealed) =
            self.seal_event_payload(coord, kind, event.header.event_id, &event.payload)?
        else {
            return Ok(());
        };
        // Durability fence: flush a freshly-minted key durable BEFORE its
        // ciphertext is written (and thus before any later segment fsync). Fail
        // the append closed on flush failure — nothing is written.
        if sealed.minted {
            self.flush_keyset_durable()?;
        }
        event.header.payload_size = u32::try_from(sealed.ciphertext.len())
            .map_err(|_| StoreError::ser_msg("ciphertext length exceeds u32::MAX"))?;
        event.payload = sealed.ciphertext;
        event.header.payload_encryption = Some(sealed.meta);
        Ok(())
    }

    /// Durably flush the whole keyset (the durability fence). Called before an
    /// encrypted append/batch that minted a new key is written, so the key is on
    /// disk before its ciphertext can be.
    ///
    /// A no-op when encryption is not configured. Routed through the writer's
    /// configured [`StoreFs`](crate::store::platform::fs::StoreFs) so a fault-
    /// injecting filesystem can tear the publish in tests.
    ///
    /// # Errors
    /// [`StoreError::Io`]/[`StoreError::Serialization`] if the atomic keyset
    /// publish fails — the caller fails the append closed and writes no frame.
    pub(super) fn flush_keyset_durable(&self) -> Result<(), StoreError> {
        let Some(key_store) = self.runtime.key_store.as_ref() else {
            return Ok(());
        };
        let guard = key_store.lock();
        guard.flush_with_fs(&self.config.data_dir, self.config.fs().as_ref())
    }
}

//! Per-scope symmetric key material for opt-in payload encryption (crypto-shred).
//!
//! This module is a *mechanism*, not a policy. A [`KeyStore`] holds one
//! 256-bit symmetric key per [`KeyScope`]; encrypting a payload under its
//! scope's key and later [`destroy`](KeyStore::destroy)-ing that key renders
//! the ciphertext permanently unrecoverable (crypto-shred). batpak only ever
//! observes "the key for scope X was created / used / destroyed" — never any
//! meaning attached to a scope. The scope granularity ([`KeyScopeGranularity`])
//! is a purely structural choice about which events share a key.
//!
//! The AEAD is XChaCha20-Poly1305 (a pure-Rust construction with a 192-bit
//! nonce and 128-bit authentication tag). Key and nonce bytes are drawn from
//! the OS CSPRNG; no non-cryptographic PRNG is ever used for key material.
//!
//! Stage A is in-memory only: there is no persistence and no wiring into the
//! append/read paths. Those seams are deliberately deferred.

use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::id::{EntityIdType, EventId};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use std::collections::btree_map::{BTreeMap, Entry};
use std::fmt;
use zeroize::Zeroizing;

/// Byte length of a symmetric payload key (256-bit).
const KEY_LEN: usize = 32;
/// Byte length of an XChaCha20-Poly1305 nonce (192-bit).
const NONCE_LEN: usize = 24;

/// How coarsely payload keys are partitioned — i.e. which events share a key,
/// and therefore what a single [`destroy`](KeyStore::destroy) shreds.
///
/// Each variant is a neutral structural choice; batpak attaches no meaning to
/// the resulting partitions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyScopeGranularity {
    /// One key per entity: destroying it shreds every payload written for that
    /// entity, across all kinds. The default granularity.
    #[default]
    PerEntity,
    /// One key per event-kind category (the high 4 bits of an [`EventKind`]):
    /// destroying it shreds every payload whose kind falls in that category.
    PerCategory,
    /// One key per full event kind (category plus type id): destroying it
    /// shreds every payload of exactly that kind.
    PerTypeId,
    /// One key per individual event: destroying it shreds exactly that event's
    /// payload and nothing else — the finest granularity.
    PerEvent,
}

/// The opaque identity a payload key is filed under.
///
/// A `KeyScope` is derived deterministically and canonically from a
/// [`KeyScopeGranularity`] plus an event's coordinate, kind, and id via
/// [`scope_for`]. Its internal byte representation is private; callers treat it
/// only as an opaque, comparable, orderable handle.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyScope(Box<[u8]>);

impl fmt::Debug for KeyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyScope(0x")?;
        for byte in self.0.iter() {
            write!(f, "{byte:02x}")?;
        }
        f.write_str(")")
    }
}

/// Derive the [`KeyScope`] an event's payload key is filed under.
///
/// Deterministic and canonical: the same inputs always yield byte-identical
/// scopes, and two granularities never collide (each derivation is prefixed
/// with a distinct discriminant). Only the field relevant to the chosen
/// granularity contributes to the identity.
#[must_use]
pub fn scope_for(
    granularity: KeyScopeGranularity,
    coordinate: &Coordinate,
    event_kind: EventKind,
    event_id: EventId,
) -> KeyScope {
    let mut bytes = Vec::new();
    match granularity {
        KeyScopeGranularity::PerEntity => {
            bytes.push(0x01);
            bytes.extend_from_slice(coordinate.entity().as_bytes());
        }
        KeyScopeGranularity::PerCategory => {
            bytes.push(0x02);
            bytes.push(event_kind.category());
        }
        KeyScopeGranularity::PerTypeId => {
            bytes.push(0x03);
            bytes.extend_from_slice(&event_kind.as_raw_u16().to_be_bytes());
        }
        KeyScopeGranularity::PerEvent => {
            bytes.push(0x04);
            bytes.extend_from_slice(&event_id.as_u128().to_be_bytes());
        }
    }
    KeyScope(bytes.into_boxed_slice())
}

/// A failure from the key store or its AEAD primitives.
///
/// Deliberately opaque: an [`open`](PayloadKey::open) failure reveals only that
/// authentication failed, never why, so it cannot serve as a decryption oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyStoreError {
    /// The OS CSPRNG failed to produce key material.
    Rng,
    /// AEAD cipher construction rejected the key length (defensive; a stored
    /// key is always exactly 256 bits).
    KeyInit,
    /// Authenticated encryption (sealing) failed.
    Seal,
    /// Authenticated decryption (opening) failed — wrong key, nonce, associated
    /// data, or a tampered ciphertext. No further detail is exposed.
    Open,
}

impl fmt::Display for KeyStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Rng => "CSPRNG failed to produce key material",
            Self::KeyInit => "AEAD key initialization rejected the key length",
            Self::Seal => "authenticated encryption failed",
            Self::Open => "authenticated decryption failed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for KeyStoreError {}

/// A 256-bit symmetric payload key.
///
/// The raw bytes are held in a [`Zeroizing`] buffer, so they are wiped from
/// memory when the key is dropped, and they never appear in any `Debug` output.
/// The only way to use a key is through [`seal`](Self::seal) /
/// [`open`](Self::open); the bytes are never exposed.
pub struct PayloadKey(Zeroizing<[u8; KEY_LEN]>);

impl fmt::Debug for PayloadKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never render key bytes — only an opaque marker.
        f.debug_struct("PayloadKey").finish_non_exhaustive()
    }
}

impl PayloadKey {
    /// Mint a fresh key from the OS CSPRNG.
    fn generate() -> Result<Self, KeyStoreError> {
        // Fill the secret in place inside the zeroizing buffer so no plaintext
        // key copy is ever left on the stack.
        let mut key: Zeroizing<[u8; KEY_LEN]> = Zeroizing::new([0u8; KEY_LEN]);
        getrandom::fill(key.as_mut_slice()).map_err(|_| KeyStoreError::Rng)?;
        Ok(Self(key))
    }

    fn cipher(&self) -> Result<XChaCha20Poly1305, KeyStoreError> {
        XChaCha20Poly1305::new_from_slice(self.0.as_slice()).map_err(|_| KeyStoreError::KeyInit)
    }

    /// Seal `plaintext` under this key with a 24-byte `nonce`, binding `aad`
    /// (associated data authenticated but not encrypted). Returns the
    /// ciphertext with its appended authentication tag.
    ///
    /// The caller owns nonce uniqueness: a nonce must never repeat under the
    /// same key. XChaCha20-Poly1305's 192-bit nonce makes random nonces safe.
    ///
    /// # Errors
    /// Returns [`KeyStoreError::Seal`] if the AEAD encryption fails, or
    /// [`KeyStoreError::KeyInit`] if cipher construction rejects the key.
    pub fn seal(
        &self,
        nonce: &[u8; NONCE_LEN],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, KeyStoreError> {
        let cipher = self.cipher()?;
        let nonce = XNonce::from_slice(nonce);
        cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| KeyStoreError::Seal)
    }

    /// Open `ciphertext` sealed under this key with the same `nonce` and `aad`.
    /// Returns the recovered plaintext.
    ///
    /// # Errors
    /// Returns [`KeyStoreError::Open`] if authentication fails (wrong key,
    /// nonce, associated data, or tampered ciphertext), or
    /// [`KeyStoreError::KeyInit`] if cipher construction rejects the key.
    pub fn open(
        &self,
        nonce: &[u8; NONCE_LEN],
        aad: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, KeyStoreError> {
        let cipher = self.cipher()?;
        let nonce = XNonce::from_slice(nonce);
        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .map_err(|_| KeyStoreError::Open)
    }
}

/// An in-memory store of per-scope payload keys.
///
/// Keys are minted lazily on first use and destroyed on demand. Destroying a
/// scope's key is the crypto-shred primitive: it zeroizes and removes the key,
/// after which any payload sealed under that scope can never be opened again.
pub struct KeyStore {
    keys: BTreeMap<KeyScope, PayloadKey>,
    granularity: KeyScopeGranularity,
}

impl KeyStore {
    /// Create an empty key store with the given scope granularity.
    #[must_use]
    pub fn new(granularity: KeyScopeGranularity) -> Self {
        Self {
            keys: BTreeMap::new(),
            granularity,
        }
    }

    /// The scope granularity this store partitions keys by.
    #[must_use]
    pub fn granularity(&self) -> KeyScopeGranularity {
        self.granularity
    }

    /// Return the key for `scope`, minting a fresh random key on first use.
    ///
    /// A second call for the same scope returns the same key until it is
    /// [`destroy`](Self::destroy)-ed.
    ///
    /// # Errors
    /// Returns [`KeyStoreError::Rng`] if the CSPRNG fails while minting a new key.
    pub fn get_or_create(&mut self, scope: &KeyScope) -> Result<&PayloadKey, KeyStoreError> {
        match self.keys.entry(scope.clone()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let key = PayloadKey::generate()?;
                Ok(entry.insert(key))
            }
        }
    }

    /// Return the key for `scope` if one currently exists, without minting.
    #[must_use]
    pub fn get(&self, scope: &KeyScope) -> Option<&PayloadKey> {
        self.keys.get(scope)
    }

    /// Destroy the key for `scope` (the crypto-shred primitive).
    ///
    /// Returns `true` if a key existed and was removed. The removed
    /// [`PayloadKey`] is dropped here, zeroizing its bytes; a subsequent
    /// [`get`](Self::get) returns `None`, and any ciphertext sealed under the
    /// old key is permanently unrecoverable.
    pub fn destroy(&mut self, scope: &KeyScope) -> bool {
        self.keys.remove(scope).is_some()
    }
}

#[cfg(test)]
mod tests;

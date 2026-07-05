//! The pluggable keyset-storage seam (0.10.0, issue #162).
//!
//! PROVES: a caller-supplied `KeysetBackend` fully replaces the in-directory
//! keyset file — mint/flush routes through it *before* the ciphertext's
//! append is acknowledged (the flush-before-ack fence, observable at the
//! seam), reopen rehydrates from it, shred publishes the shrunken keyset
//! through it, and the frozen fail-closed taxonomy is unchanged: corrupt
//! bytes are `KeysetCorrupt`, a lost keyset over pre-existing ciphertext is
//! `KeysetMissing`, a deliberate shred is `PayloadShredded`.
//! CATCHES: a keyset write path that silently bypasses the seam (falling back
//! to the store directory), and a backend integration that reorders the
//! key-durable-before-ciphertext-ack fence.
#![cfg(feature = "payload-encryption")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use batpak::prelude::*;
use batpak::store::{FileKeysetBackend, KeyScopeGranularity, KeysetBackend};
use zeroize::Zeroizing;

/// In-memory keyset storage: one image slot plus a persist counter, shared
/// across store lifetimes through `Arc` clones.
#[derive(Clone, Default)]
struct MemoryKeysetBackend {
    image: Arc<Mutex<Option<Vec<u8>>>>,
    persists: Arc<AtomicUsize>,
}

impl MemoryKeysetBackend {
    fn persist_count(&self) -> usize {
        self.persists.load(Ordering::SeqCst)
    }
}

impl KeysetBackend for MemoryKeysetBackend {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        Ok(self
            .image
            .lock()
            .expect("image lock")
            .clone()
            .map(Zeroizing::new))
    }

    fn persist(&self, encoded: &[u8]) -> Result<(), StoreError> {
        *self.image.lock().expect("image lock") = Some(encoded.to_vec());
        self.persists.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn encrypted_config(dir: &tempfile::TempDir, backend: &MemoryKeysetBackend) -> StoreConfig {
    StoreConfig::new(dir.path())
        .with_payload_encryption(KeyScopeGranularity::PerEntity)
        .with_keyset_backend(Arc::new(backend.clone()))
}

#[test]
fn mint_persists_through_the_backend_before_the_append_acks(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let backend = MemoryKeysetBackend::default();
    let store = Store::open(encrypted_config(&dir, &backend))?;
    assert_eq!(
        backend.persist_count(),
        0,
        "opening a fresh store mints nothing and persists nothing"
    );

    let coord = Coordinate::new("backend:alpha", "scope:seam")?;
    let receipt = store.append(
        &coord,
        EventKind::custom(0xB, 1),
        &serde_json::json!({"n": 1}),
    )?;
    // The append has acked — the flush-before-ack fence requires the minted
    // key to have been durably persisted through the seam already.
    assert_eq!(
        backend.persist_count(),
        1,
        "the first mint must persist through the backend before its append returns"
    );
    assert!(
        backend.image.lock().expect("image lock").is_some(),
        "the backend holds the keyset image, not the store directory"
    );
    assert!(
        !dir.path().join("keyset.fbatk").exists(),
        "no keyset file may appear in the store directory when a backend is installed"
    );

    // Same scope, same key: a second append mints nothing and re-persists nothing.
    let _same_key_receipt = store.append(
        &coord,
        EventKind::custom(0xB, 1),
        &serde_json::json!({"n": 2}),
    )?;
    assert_eq!(
        backend.persist_count(),
        1,
        "appends under an already-durable key must not re-flush the keyset"
    );

    let fetched = store.get(receipt.event_id)?;
    assert_eq!(fetched.event.header.event_id, receipt.event_id);
    store.close()?;
    Ok(())
}

#[test]
fn reopen_rehydrates_the_keyset_from_the_backend() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let backend = MemoryKeysetBackend::default();
    let coord = Coordinate::new("backend:reopen", "scope:seam")?;

    let store = Store::open(encrypted_config(&dir, &backend))?;
    let receipt = store.append(
        &coord,
        EventKind::custom(0xB, 2),
        &serde_json::json!({"v": 7}),
    )?;
    store.close()?;

    // Reopen with the SAME backend: the key comes back from it and the
    // ciphertext decrypts.
    let reopened = Store::open(encrypted_config(&dir, &backend))?;
    let fetched = reopened.get(receipt.event_id)?;
    assert_eq!(
        fetched.event.payload["v"], 7,
        "payload decrypts after reopen"
    );
    reopened.close()?;
    Ok(())
}

#[test]
fn shred_publishes_the_destruction_through_the_backend() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let backend = MemoryKeysetBackend::default();
    let coord = Coordinate::new("backend:shredme", "scope:seam")?;

    let store = Store::open(encrypted_config(&dir, &backend))?;
    let receipt = store.append(
        &coord,
        EventKind::custom(0xB, 3),
        &serde_json::json!({"s": 1}),
    )?;
    let persists_before_shred = backend.persist_count();

    let destroyed = store.shred_scope(batpak::store::ShredScope::Entity(&coord))?;
    assert!(destroyed, "the scope key existed and was destroyed");
    assert_eq!(
        backend.persist_count(),
        persists_before_shred + 1,
        "the shrunken keyset must be persisted through the backend before shred acks"
    );

    // In-process read is already fail-closed as Shredded.
    let err = match store.get(receipt.event_id) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: reading a shredded payload must fail closed",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(err, StoreError::PayloadShredded { .. }),
        "expected PayloadShredded, got {err:?}"
    );
    store.close()?;

    // Across a reopen from the shrunken backend image, the distinction holds:
    // deliberate shred, not a lost keyset.
    let reopened = Store::open(encrypted_config(&dir, &backend))?;
    let err = match reopened.get(receipt.event_id) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: a shredded payload must stay shredded across reopen",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(err, StoreError::PayloadShredded { .. }),
        "expected PayloadShredded after reopen, got {err:?}"
    );
    reopened.close()?;
    Ok(())
}

#[test]
fn corrupt_backend_bytes_fail_the_open_closed() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let backend = MemoryKeysetBackend::default();

    let store = Store::open(encrypted_config(&dir, &backend))?;
    let coord = Coordinate::new("backend:corrupt", "scope:seam")?;
    let _minting_receipt = store.append(
        &coord,
        EventKind::custom(0xB, 4),
        &serde_json::json!({"c": 1}),
    )?;
    store.close()?;

    // Flip a byte inside the persisted image: the CRC/decode law in persist.rs
    // must reject it exactly as it rejects a corrupt file.
    {
        let mut image = backend.image.lock().expect("image lock");
        let bytes = image.as_mut().expect("an image was persisted");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
    }
    let err = match Store::open(encrypted_config(&dir, &backend)) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: a corrupt keyset image must fail the open closed",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(err, StoreError::KeysetCorrupt { .. }),
        "expected KeysetCorrupt, got {err:?}"
    );
    Ok(())
}

#[test]
fn lost_backend_over_existing_ciphertext_reads_as_keyset_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let backend = MemoryKeysetBackend::default();
    let coord = Coordinate::new("backend:lost", "scope:seam")?;

    let store = Store::open(encrypted_config(&dir, &backend))?;
    let receipt = store.append(
        &coord,
        EventKind::custom(0xB, 5),
        &serde_json::json!({"l": 1}),
    )?;
    store.close()?;

    // Reopen with a FRESH, never-persisted backend: the segments still hold
    // ciphertext but the keyset is gone. The open succeeds (absent-on-load),
    // and the read reports the D24 lost-keyset disposition — KeysetMissing,
    // never a Shredded lookalike and never silent re-minting.
    let lost = MemoryKeysetBackend::default();
    let reopened = Store::open(encrypted_config(&dir, &lost))?;
    let err = match reopened.get(receipt.event_id) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY: ciphertext without its keyset must fail closed",
            )
            .into())
        }
        Err(error) => error,
    };
    assert!(
        matches!(err, StoreError::KeysetMissing { .. }),
        "expected KeysetMissing (lost keyset), got {err:?}"
    );
    reopened.close()?;
    Ok(())
}

#[test]
fn explicit_file_backend_matches_the_default_file_layout() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let coord = Coordinate::new("backend:file", "scope:seam")?;

    // An explicitly-installed FileKeysetBackend writes the exact artifact the
    // default (no backend configured) writes, so the two are interchangeable.
    // `with_store_fs` is the same constructor routed through an explicit
    // (here: production) StoreFs — the seam-composition form.
    let file_backend =
        FileKeysetBackend::with_store_fs(dir.path(), Arc::new(batpak::store::RealFs));
    drop(FileKeysetBackend::new(dir.path()));
    let config = StoreConfig::new(dir.path())
        .with_payload_encryption(KeyScopeGranularity::PerEntity)
        .with_keyset_backend(Arc::new(file_backend));
    let store = Store::open(config)?;
    let receipt = store.append(
        &coord,
        EventKind::custom(0xB, 6),
        &serde_json::json!({"f": 1}),
    )?;
    assert!(
        dir.path().join("keyset.fbatk").exists(),
        "the file backend keeps the keyset as the in-directory artifact"
    );
    store.close()?;

    // A default-configured store (no explicit backend) reads that artifact.
    let default_config =
        StoreConfig::new(dir.path()).with_payload_encryption(KeyScopeGranularity::PerEntity);
    let reopened = Store::open(default_config)?;
    let fetched = reopened.get(receipt.event_id)?;
    assert_eq!(fetched.event.payload["f"], 1);
    reopened.close()?;
    Ok(())
}

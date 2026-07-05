//! Pluggable keyset storage: where the crypto-shred keyset bytes live and
//! when they are durable (issue #162).
//!
//! The keyset's *format* and *semantics* stay single-sourced in
//! [`persist`](crate::store::keyscope::persist): one encoded image (magic |
//! version | crc | msgpack body), fail-closed decoding, the `Shredded` vs
//! `KeysetMissing` distinction, and the snapshot/fork "keys never travel"
//! contract are all unchanged. A
//! [`KeysetBackend`](crate::store::keyscope::backend::KeysetBackend) answers
//! only the storage question — so an embedder can hold key material outside
//! the store directory (a separate volume with a different durability
//! profile, an OS keychain, a database row wrapping the image with a KMS)
//! without filesystem surgery on store internals.
//!
//! The default is
//! [`FileKeysetBackend`](crate::store::keyscope::backend::FileKeysetBackend):
//! today's in-directory `keyset.fbatk` file, published through the store's
//! atomic-write seam.

use crate::store::file_classification::KEYSET_FILENAME;
use crate::store::platform::fs::{write_file_atomically_with_fs, RealFs, StoreFs};
use crate::store::StoreError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeroize::Zeroizing;

/// Storage seam for the crypto-shred keyset image.
///
/// The store hands a backend the fully-encoded keyset image (header + body —
/// an opaque byte string from the backend's point of view) and asks it back
/// on open. Implementations must uphold the durability obligations below;
/// the store's crypto-shred guarantees assume them.
///
/// # Durability obligations
///
/// - **`persist` acks only durable, atomic publishes.** It must not return
///   `Ok` before the snapshot survives a crash, and a torn persist must leave
///   the PREVIOUS complete snapshot loadable — never a partial mixture. The
///   store's flush-before-ack fence relies on this: a freshly-minted key is
///   persisted through this seam BEFORE the ciphertext it encrypts is
///   acknowledged.
/// - **`persist` is the crypto-shred point.** After `Ok`, key material absent
///   from the new snapshot must be unrecoverable from the backend (no stale
///   copies served by a later `load`). [`Store::shred_scope`] acknowledges a
///   destruction only after this returns `Ok`.
/// - **`load` returns the last persisted snapshot, or `None` only when
///   nothing was ever persisted.** Serving `None` for a keyset that existed
///   reads as a *lost* keyset: pre-existing encrypted payloads surface
///   `KeysetMissing` (fail closed), not silent re-minting.
///
/// [`Store::shred_scope`]: crate::store::Store::shred_scope
pub trait KeysetBackend: Send + Sync {
    /// Return the last durably persisted keyset image, or `Ok(None)` when no
    /// image was ever persisted (a genuinely fresh keyset).
    ///
    /// # Errors
    /// The backend's read failure; the store fails the open closed on it.
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError>;

    /// Atomically and durably publish `encoded` as the new keyset image.
    ///
    /// # Errors
    /// The backend's write failure; the store fails the triggering append,
    /// batch, or shred closed on it (no ciphertext is acked ahead of its key,
    /// and a shred is not acknowledged until the destruction is durable).
    fn persist(&self, encoded: &[u8]) -> Result<(), StoreError>;
}

/// The default [`KeysetBackend`]: a single `keyset.fbatk` file inside the
/// store directory, published through the store's crash-safe atomic-write
/// seam ([`StoreFs::named_temp_in`] + [`StoreFs::persist_temp_with_parent_sync`]).
///
/// [`StoreFs::named_temp_in`]: crate::store::StoreFs::named_temp_in
/// [`StoreFs::persist_temp_with_parent_sync`]: crate::store::StoreFs::persist_temp_with_parent_sync
pub struct FileKeysetBackend {
    dir: PathBuf,
    fs: Arc<dyn StoreFs>,
}

impl FileKeysetBackend {
    /// File backend over `dir` on the production filesystem ([`RealFs`]).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::with_store_fs(dir, Arc::new(RealFs))
    }

    /// File backend over `dir` routed through a caller-supplied [`StoreFs`]
    /// (fault injection, instrumentation, an alternate native volume).
    #[must_use]
    pub fn with_store_fs(dir: impl Into<PathBuf>, fs: Arc<dyn StoreFs>) -> Self {
        Self {
            dir: dir.into(),
            fs,
        }
    }
}

impl KeysetBackend for FileKeysetBackend {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        load_keyset_bytes(&self.dir, self.fs.as_ref())
    }

    fn persist(&self, encoded: &[u8]) -> Result<(), StoreError> {
        persist_keyset_bytes(&self.dir, self.fs.as_ref(), encoded)
    }
}

/// Borrowed file backend used by the `_with_fs` compatibility wrappers on
/// [`KeyStore`](super::KeyStore) so the file logic stays single-sourced
/// without requiring an owned `Arc<dyn StoreFs>`.
pub(crate) struct FileKeysetBackendRef<'a> {
    pub(crate) dir: &'a Path,
    pub(crate) fs: &'a dyn StoreFs,
}

impl KeysetBackend for FileKeysetBackendRef<'_> {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        load_keyset_bytes(self.dir, self.fs)
    }

    fn persist(&self, encoded: &[u8]) -> Result<(), StoreError> {
        persist_keyset_bytes(self.dir, self.fs, encoded)
    }
}

/// Read the raw keyset image from `dir`, `Ok(None)` when absent. The file
/// carries raw keys — bytes land in a [`Zeroizing`] buffer, and the leaf is
/// symlink-guarded on read exactly as on write.
fn load_keyset_bytes(
    dir: &Path,
    fs: &dyn StoreFs,
) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
    let path = dir.join(KEYSET_FILENAME);
    fs.reject_symlink_leaf(&path, "crypto-shred-keyset")?;
    // Read through the SAME seam the persist half publishes into, so a
    // virtualizing or fault-injecting backend serves its own last image
    // (PR #169 review finding: a free-fn read here bypassed the seam).
    match fs.read(&path) {
        Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StoreError::Io(error)),
    }
}

/// Publish the encoded keyset image to `dir` through the atomic-write seam:
/// symlink-guard, temp-file stage, fsync, rename-with-parent-sync.
fn persist_keyset_bytes(dir: &Path, fs: &dyn StoreFs, encoded: &[u8]) -> Result<(), StoreError> {
    let final_path = dir.join(KEYSET_FILENAME);
    write_file_atomically_with_fs(
        dir,
        &final_path,
        "crypto-shred-keyset",
        |file| {
            use std::io::Write;
            file.write_all(encoded).map_err(StoreError::Io)
        },
        fs,
    )
}

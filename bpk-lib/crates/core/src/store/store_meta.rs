//! The store lineage-metadata sidecar (`store.meta`) — the durable home of the
//! store's LINEAGE IDENTITY and (once keyed idempotency traffic exists) the
//! expected durable-authority anchor (#205, substrate for #189).
//!
//! Lineage semantics (owner ruling, 2026-07-12): the identity is
//! - minted once, at first writable open of a directory with no
//!   post-migration witness (fresh stores and legacy pre-`store.meta`
//!   directories take the same one-time action);
//! - copied by snapshot and by fork (both copy the idempotency authority
//!   alongside the logical history — a fork that minted a fresh identity
//!   would instantly see its own copied authority image as foreign);
//! - stable across path moves and reopen;
//! - a discriminator of unrelated store LINEAGES — it does NOT by itself
//!   detect rewind or sibling-fork divergence. External derived state must
//!   bind to the identity PLUS a history anchor (frontier sequence + the
//!   event id / chain commitment at that sequence).
//!
//! The never-remint law: once a post-migration witness exists (an idempotency
//! sidecar in format v2+ carries the lineage id), an absent `store.meta` is a
//! typed [`StoreError::StoreMetadataMissing`] refusal — silently reminting
//! would reset the identity out from under every externally anchored consumer
//! and un-bind the idempotency authority image. Corruption is likewise never
//! absence ([`StoreError::StoreMetadataCorrupt`], fail closed from day one).
//!
//! Migration is crash-restartable: `store.meta` is written atomically BEFORE
//! any idempotency-image upgrade can stamp the lineage id; a crash between the
//! two leaves `store.meta` present + a v1 image, which is simply the migrated
//! steady state until the next flush.

use crate::store::index::idemp::IdempVersionWitness;
use crate::store::platform::fs::StoreFs;
use crate::store::{StoreError, StoreMetaCorruption};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const STORE_META_MAGIC: &[u8; 6] = b"FBATSM";
/// On-disk format version stored in the `store.meta` header.
pub const STORE_META_VERSION: u16 = 1;
pub(crate) const STORE_META_FILENAME: &str = "store.meta";

/// Decoded `store.meta` body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoreMetaData {
    /// The store's lineage identity (UUIDv7 bits minted at first writable
    /// open, from the injected clock seam).
    #[serde(with = "crate::wire::u128_bytes")]
    pub(crate) lineage: u128,
    /// The expected durable idempotency-authority anchor: `None` until the
    /// first v2 idempotency image is flushed (#189 stamps and maintains it).
    /// Present-but-stale detection compares an image's own anchor against
    /// this expectation.
    #[serde(default)]
    pub(crate) idemp_authority: Option<IdempAuthorityAnchor>,
}

/// The compound anchor binding an idempotency-authority image to one exact
/// history tip (owner ruling: lineage UUID + numeric frontier alone is
/// forgeable across diverged sibling forks — the event id and chain
/// commitment at the covered sequence disambiguate).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdempAuthorityAnchor {
    /// Highest global sequence covered by the flushed image.
    pub(crate) covered_global_sequence: u64,
    /// The event id committed at that sequence.
    #[serde(with = "crate::wire::u128_bytes")]
    pub(crate) event_id_at: u128,
    /// The strongest existing v1 tip commitment at that sequence (the
    /// committed event's content hash chain value; NOT a signed seal — the
    /// authenticated spine is #190/#191).
    pub(crate) chain_commitment: [u8; 32],
}

/// Load `store.meta`, failing closed on corruption.
///
/// - No file at the expected path → `Ok(None)` (resolution against the
///   post-migration witness happens in [`resolve_on_open`]).
/// - Valid file → `Ok(Some(meta))`.
/// - Present but unreadable / wrong magic / bad CRC / decode failure /
///   unsupported version → [`StoreError::StoreMetadataCorrupt`].
/// - Future version → [`StoreError::StoreMetadataFutureVersion`]
///   (canonical forward-compat refusal, distinct from remediable corruption).
pub(crate) fn load_store_meta(
    data_dir: &Path,
    fs: &dyn StoreFs,
) -> Result<Option<StoreMetaData>, StoreError> {
    let path = data_dir.join(STORE_META_FILENAME);
    let raw = match fs.read(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(corrupt_meta(&path, StoreMetaCorruption::ReadFailed(error)));
        }
    };

    let prefix = match crate::store::wire_header::parse(&raw, STORE_META_MAGIC) {
        Ok(prefix) => prefix,
        Err(crate::store::wire_header::PrefixError::TooShort { len }) => {
            return Err(corrupt_meta(
                &path,
                StoreMetaCorruption::TooShort {
                    actual: len,
                    required: crate::store::wire_header::HEADER_LEN,
                },
            ));
        }
        Err(crate::store::wire_header::PrefixError::BadMagic) => {
            return Err(corrupt_meta(&path, StoreMetaCorruption::BadMagic));
        }
    };

    let version = prefix.version;
    // justifies: INV-ONDISK-FORWARD-COMPAT-CANONICAL
    if version > STORE_META_VERSION {
        tracing::warn!(
            target: "batpak::store_meta",
            path = %path.display(),
            version,
            supported = STORE_META_VERSION,
            "store metadata declares a future format version — refusing canonically"
        );
        return Err(StoreError::StoreMetadataFutureVersion {
            path: path.clone(),
            found: version,
            supported: STORE_META_VERSION,
        });
    }
    if !(1..=STORE_META_VERSION).contains(&version) {
        return Err(corrupt_meta(
            &path,
            StoreMetaCorruption::UnsupportedVersion {
                observed: version,
                expected: STORE_META_VERSION,
            },
        ));
    }

    let stored_crc = prefix.stored_crc;
    let body = prefix.body;
    let actual_crc = crc32fast::hash(body);
    if stored_crc != actual_crc {
        return Err(corrupt_meta(
            &path,
            StoreMetaCorruption::CrcMismatch {
                stored: stored_crc,
                computed: actual_crc,
            },
        ));
    }

    match crate::encoding::from_bytes::<StoreMetaData>(body) {
        Ok(meta) => Ok(Some(meta)),
        Err(error) => Err(corrupt_meta(
            &path,
            StoreMetaCorruption::DecodeFailed(error),
        )),
    }
}

/// Persist `store.meta` atomically (staged temp + fsync + atomic rename +
/// parent-dir sync), the same discipline as every other sidecar authority.
pub(crate) fn write_store_meta(
    data_dir: &Path,
    meta: &StoreMetaData,
    fs: &dyn StoreFs,
) -> Result<(), StoreError> {
    let final_path = data_dir.join(STORE_META_FILENAME);
    let body = crate::encoding::to_bytes(meta)
        .map_err(|error| StoreError::Serialization(Box::new(error)))?;
    let crc = crc32fast::hash(&body);
    crate::store::platform::fs::write_file_atomically_with_fs(
        data_dir,
        &final_path,
        "store metadata",
        |writer| {
            writer
                .write_all(&crate::store::wire_header::encode(
                    STORE_META_MAGIC,
                    STORE_META_VERSION,
                    crc,
                ))
                .map_err(StoreError::Io)?;
            writer.write_all(&body).map_err(StoreError::Io)
        },
        fs,
    )
}

/// Resolve the store's lineage metadata at open time (the migration law, #205):
///
/// 1. `store.meta` present and valid → use it (any open mode).
/// 2. Absent, but a post-migration witness exists (the idempotency sidecar
///    declares format v2+) → [`StoreError::StoreMetadataMissing`]; NEVER
///    remint (any open mode).
/// 3. Absent with no witness:
///    - writable open → one-time mint-and-persist (fresh directories and
///      legacy pre-`store.meta` directories take the same action);
///    - read-only open → `Ok(None)` (no mutation; [`crate::store::Store::identity`]
///      surfaces the absence as a typed error until a writable open migrates).
/// 4. An UNREADABLE idempotency header makes the witness undecidable: do not
///    mint (a wrong mint would mis-bind a later-restored authority image) and
///    do not refuse here — the idempotency loader owns that failure.
///
/// `mint` runs at most once, only on the writable-mint branch, and must come
/// from the injected clock seam (no ambient time — #182).
pub(crate) fn resolve_on_open(
    data_dir: &Path,
    fs: &dyn StoreFs,
    mint: impl FnOnce() -> u128,
    writable: bool,
) -> Result<Option<StoreMetaData>, StoreError> {
    if let Some(meta) = load_store_meta(data_dir, fs)? {
        return Ok(Some(meta));
    }
    match crate::store::index::idemp::peek_idemp_version(data_dir, fs) {
        IdempVersionWitness::Version(version) if version >= 2 => {
            Err(StoreError::StoreMetadataMissing {
                path: data_dir.join(STORE_META_FILENAME),
            })
        }
        IdempVersionWitness::Version(_) | IdempVersionWitness::Absent => {
            if !writable {
                return Ok(None);
            }
            let meta = StoreMetaData {
                lineage: mint(),
                idemp_authority: None,
            };
            write_store_meta(data_dir, &meta, fs)?;
            Ok(Some(meta))
        }
        IdempVersionWitness::Unreadable => Ok(None),
    }
}

fn corrupt_meta(path: &Path, kind: StoreMetaCorruption) -> StoreError {
    tracing::warn!(
        target: "batpak::store_meta",
        path = %path.display(),
        reason = %kind,
        "store metadata unreadable; failing closed"
    );
    StoreError::StoreMetadataCorrupt {
        path: path.to_path_buf(),
        kind,
    }
}

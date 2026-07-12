//! The durable idempotency authority IMAGE: on-disk format (v1 legacy, v2
//! lineage/anchor-bound) and the fail-closed admission rules
//! (GAUNT-IDEMPOTENCY-AUTHORITY, #189). Split from `idemp.rs` (the in-memory
//! store + flush) to keep each production file within the size cap; the
//! crate-internal call paths re-export through `super::idemp`.

use crate::store::{IdempAuthorityCorruption, StoreError};
use std::path::Path;

use super::idemp::{IdempEntry, IDEMP_FILENAME, IDEMP_MAGIC, IDEMP_VERSION};

/// The v2 on-disk image: entries PLUS the identity/history binding that makes
/// a transplanted, stale, or diverged-sibling image detectable at cold start
/// (GAUNT-IDEMPOTENCY-AUTHORITY, #189).
#[derive(serde::Serialize, serde::Deserialize)]
pub(in crate::store::index) struct IdempImageV2 {
    /// The store lineage this image belongs to (see `store.meta`).
    #[serde(with = "crate::wire::u128_bytes")]
    pub(in crate::store::index) lineage: u128,
    /// Compound history anchor at flush time; `None` when the store had no
    /// committed events yet.
    pub(in crate::store::index) anchor: Option<crate::store::store_meta::IdempAuthorityAnchor>,
    /// The durable dedup entries.
    pub(in crate::store::index) entries: Vec<IdempEntry>,
}

/// Post-migration witness probe for `store.meta`'s never-remint law (#205):
/// what a header-only peek of the on-disk idempotency sidecar revealed.
pub(crate) enum IdempVersionWitness {
    /// No sidecar on disk.
    Absent,
    /// Header parsed; the declared format version. A v2+ image proves the
    /// store was already migrated (v2 images carry the lineage id), so an
    /// absent `store.meta` must refuse instead of reminting.
    Version(u16),
    /// Bytes exist but the header cannot be read or parsed — the witness is
    /// undecidable; [`read_idemp_file`] owns that failure at its own stage.
    Unreadable,
}

/// Peek the on-disk idempotency sidecar's declared format version WITHOUT
/// loading or validating the body (no CRC/decode work). Never errors: this is
/// a witness probe only, and every degenerate shape maps to a witness state.
pub(crate) fn peek_idemp_version(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
) -> IdempVersionWitness {
    let path = data_dir.join(IDEMP_FILENAME);
    let raw = match fs.read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return IdempVersionWitness::Absent;
        }
        Err(_read_failed) => return IdempVersionWitness::Unreadable,
    };
    match crate::store::wire_header::parse(&raw, IDEMP_MAGIC) {
        Ok(prefix) => IdempVersionWitness::Version(prefix.version),
        Err(
            crate::store::wire_header::PrefixError::TooShort { .. }
            | crate::store::wire_header::PrefixError::BadMagic,
        ) => IdempVersionWitness::Unreadable,
    }
}

/// Result of admitting `index.idemp` at cold start.
pub(crate) enum IdempLoad {
    /// File present, valid, and bound to THIS store's lineage/history; the
    /// decoded entries.
    Loaded(Vec<IdempEntry>),
    /// File absent with no durable-authority expectation: a genuinely fresh
    /// store (or a legacy pre-migration store with no flushed image yet).
    Missing,
}

/// Read, validate, and ADMIT the durable idempotency authority (#189).
///
/// Corruption is not absence: after retention compaction evicts an event's
/// frames, this sidecar is the only remaining proof that a keyed retry is not
/// a new command, so every damaged-authority shape is a typed refusal instead
/// of the retired downgrade-to-empty posture:
///
/// * absent, no expectation → [`IdempLoad::Missing`] (fresh store);
/// * absent, expectation recorded in `store.meta` →
///   [`StoreError::IdempotencyAuthorityMissing`] (authority LOSS, not fresh);
/// * unreadable / short / wrong magic / bad CRC / decode failure / garbled
///   version → [`StoreError::IdempotencyAuthorityCorrupt`] with a typed reason;
/// * version NEWER than [`IDEMP_VERSION`] → hard
///   [`StoreError::IdempotencyFutureVersion`] (unchanged canonical refusal);
/// * legacy v1 image: admitted ONLY while no expectation exists (pre-migration
///   store; it upgrades to v2 at the next flush) — with an expectation it is
///   [`StoreError::IdempotencyAuthorityStale`] (a v1 image cannot prove it
///   covers the expected frontier);
/// * v2 image: lineage must match `store.meta`
///   ([`StoreError::IdempotencyAuthorityForeign`] otherwise); its anchor must
///   be AT OR PAST the expectation — behind is
///   [`StoreError::IdempotencyAuthorityStale`], and an equal covered sequence
///   with a diverged event id / chain commitment is
///   [`StoreError::IdempotencyAuthorityForeign`] (a sibling fork's image). An
///   image NEWER than the expectation is admitted: the flush protocol writes
///   the image before updating `store.meta`, so that window is a legal
///   superset state.
pub(crate) fn load_idempotency_authority(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
    store_lineage: Option<u128>,
    expectation: Option<&crate::store::store_meta::IdempAuthorityAnchor>,
) -> Result<IdempLoad, StoreError> {
    let path = data_dir.join(IDEMP_FILENAME);
    let raw = match fs.read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if expectation.is_some() {
                return Err(StoreError::IdempotencyAuthorityMissing { path });
            }
            return Ok(IdempLoad::Missing);
        }
        Err(error) => {
            return Err(corrupt_authority(
                &path,
                IdempAuthorityCorruption::ReadFailed(error),
            ));
        }
    };

    let prefix = match crate::store::wire_header::parse(&raw, IDEMP_MAGIC) {
        Ok(prefix) => prefix,
        Err(crate::store::wire_header::PrefixError::TooShort { len }) => {
            return Err(corrupt_authority(
                &path,
                IdempAuthorityCorruption::TooShort {
                    actual: len,
                    required: crate::store::wire_header::HEADER_LEN,
                },
            ));
        }
        Err(crate::store::wire_header::PrefixError::BadMagic) => {
            return Err(corrupt_authority(&path, IdempAuthorityCorruption::BadMagic));
        }
    };

    let version = prefix.version;
    if version > IDEMP_VERSION {
        // FutureVersion: hard error, mirror the schema-evolution stance.
        return Err(StoreError::IdempotencyFutureVersion {
            stored: version,
            current: IDEMP_VERSION,
        });
    }
    if !(1..=IDEMP_VERSION).contains(&version) {
        return Err(corrupt_authority(
            &path,
            IdempAuthorityCorruption::UnsupportedVersion {
                observed: version,
                expected: IDEMP_VERSION,
            },
        ));
    }

    let stored_crc = prefix.stored_crc;
    let body = prefix.body;
    let computed_crc = crc32fast::hash(body);
    if stored_crc != computed_crc {
        return Err(corrupt_authority(
            &path,
            IdempAuthorityCorruption::CrcMismatch {
                stored: stored_crc,
                computed: computed_crc,
            },
        ));
    }

    if version == 1 {
        return admit_legacy_v1(&path, body, expectation);
    }

    let image = match crate::encoding::from_bytes::<IdempImageV2>(body) {
        Ok(image) => image,
        Err(error) => {
            return Err(corrupt_authority(
                &path,
                IdempAuthorityCorruption::DecodeFailed(error),
            ));
        }
    };
    admit_v2_image(image, store_lineage, expectation)
}

/// Admit a legacy v1 image (bare `Vec<IdempEntry>`, no binding): only while
/// the store has never recorded a durable-authority expectation (it upgrades
/// to v2 at the next flush); with an expectation it is STALE by construction
/// (a v1 image cannot prove it covers the expected frontier).
fn admit_legacy_v1(
    path: &Path,
    body: &[u8],
    expectation: Option<&crate::store::store_meta::IdempAuthorityAnchor>,
) -> Result<IdempLoad, StoreError> {
    if let Some(expected) = expectation {
        return Err(StoreError::IdempotencyAuthorityStale {
            image_covered: 0,
            expected_covered: expected.covered_global_sequence,
        });
    }
    match crate::encoding::from_bytes::<Vec<IdempEntry>>(body) {
        Ok(entries) => Ok(IdempLoad::Loaded(entries)),
        Err(error) => Err(corrupt_authority(
            path,
            IdempAuthorityCorruption::DecodeFailed(error),
        )),
    }
}

/// Admit a decoded v2 image against the store's lineage and recorded
/// expectation: a foreign lineage is never acceptable; an anchor BEHIND the
/// expectation is stale; an equal covered sequence with a diverged event id /
/// chain commitment is a sibling-fork transplant (foreign); an anchor at or
/// past the expectation is admitted (the flush-then-meta crash window is a
/// legal superset state).
fn admit_v2_image(
    image: IdempImageV2,
    store_lineage: Option<u128>,
    expectation: Option<&crate::store::store_meta::IdempAuthorityAnchor>,
) -> Result<IdempLoad, StoreError> {
    if let Some(lineage) = store_lineage {
        if image.lineage != lineage {
            return Err(StoreError::IdempotencyAuthorityForeign {
                kind: crate::store::IdempAuthorityForeignKind::Lineage,
            });
        }
    }
    if let Some(expected) = expectation {
        let Some(anchor) = image.anchor else {
            return Err(StoreError::IdempotencyAuthorityStale {
                image_covered: 0,
                expected_covered: expected.covered_global_sequence,
            });
        };
        if anchor.covered_global_sequence < expected.covered_global_sequence {
            return Err(StoreError::IdempotencyAuthorityStale {
                image_covered: anchor.covered_global_sequence,
                expected_covered: expected.covered_global_sequence,
            });
        }
        if anchor.covered_global_sequence == expected.covered_global_sequence
            && (anchor.event_id_at != expected.event_id_at
                || anchor.chain_commitment != expected.chain_commitment)
        {
            return Err(StoreError::IdempotencyAuthorityForeign {
                kind: crate::store::IdempAuthorityForeignKind::HistoryAnchor,
            });
        }
    }
    Ok(IdempLoad::Loaded(image.entries))
}

fn corrupt_authority(path: &Path, kind: IdempAuthorityCorruption) -> StoreError {
    tracing::warn!(
        target: "batpak::idemp",
        path = %path.display(),
        reason = %kind,
        "durable idempotency authority unreadable; failing closed"
    );
    StoreError::IdempotencyAuthorityCorrupt {
        path: path.to_path_buf(),
        kind,
    }
}

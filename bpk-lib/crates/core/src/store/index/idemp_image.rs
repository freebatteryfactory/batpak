//! The durable idempotency authority IMAGE: on-disk format (v1 legacy, v2
//! lineage/anchor-bound) and the fail-closed admission rules
//! (GAUNT-IDEMPOTENCY-AUTHORITY, #189). Split from `idemp.rs` (the in-memory
//! store + flush) to keep each production file within the size cap; the
//! crate-internal call paths re-export through `super::idemp`.

use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
use crate::store::{IdempAuthorityCorruption, StoreError};
use std::path::Path;

use super::idemp::{IdempEntry, IDEMP_FILENAME, IDEMP_MAGIC, IDEMP_VERSION};

/// The v2 on-disk image: entries PLUS the identity/authorization binding that
/// makes a transplanted, stale, or diverged-sibling image detectable at cold
/// start (GAUNT-IDEMPOTENCY-AUTHORITY, #189). Every field that names a
/// generation carries a branded id (#226 A4) so the pending marker, the image,
/// and `store.meta` cannot silently disagree.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct IdempImageV2 {
    /// The store lineage this image belongs to (see `store.meta`).
    pub(crate) lineage: StoreLineage,
    /// The generation token `store.meta` authorized for THIS publication.
    /// Admission requires an exact match against the recorded current or
    /// pending token — random per publication, so sibling branches cannot
    /// independently mint an acceptable next generation.
    pub(crate) image_id: AuthorityImageId,
    /// The authority image this publication supersedes; `None` for the first
    /// publication of a lineage (#226 A4). Lets recovery reconstruct the
    /// publication chain without file-existence guessing.
    pub(crate) previous_image_id: Option<AuthorityImageId>,
    /// The compaction transaction that authorized this publication, when the
    /// image was published as part of a compaction namespace commit (#226 A4);
    /// `None` for ordinary flushes. Recovery cross-checks this against the
    /// pending marker and `store.meta`'s commit record.
    pub(crate) compaction_id: Option<CompactionId>,
    /// Diagnostic history anchor at flush time; `None` when the store had no
    /// user-keyed entries yet (such an image is never written in practice).
    pub(crate) anchor: Option<crate::store::store_meta::IdempAuthorityAnchor>,
    /// The durable dedup entries.
    pub(crate) entries: Vec<IdempEntry>,
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

/// The decoded shape of an `index.idemp` body, once the header, version, and
/// CRC have all validated: either a legacy v1 bare entry vector or a bound v2
/// image.
pub(crate) enum IdempImageParse {
    /// Legacy v1 body: a bare `Vec<IdempEntry>` with no identity binding.
    V1(Vec<IdempEntry>),
    /// v2 body: the lineage/anchor-bound image.
    V2(IdempImageV2),
}

/// Why a raw `index.idemp` byte string could not be parsed into an
/// [`IdempImageParse`]. Path-free and store-context-free: the two consumers
/// (cold-start admission, the public restore seam) attach their own remediation
/// (a path-bearing `StoreError` at open, a typed restore refusal at #188).
pub(crate) enum IdempImageParseError {
    /// Unreadable/unsupported bytes: short, wrong magic, unsupported version,
    /// CRC mismatch, or a decode failure of the body.
    Corrupt(IdempAuthorityCorruption),
    /// The declared version is newer than this binary understands.
    FutureVersion {
        /// The version the bytes declared.
        stored: u16,
        /// The newest version this binary understands.
        current: u16,
    },
}

/// The single decision core shared by cold-start admission and the public
/// restore seam (#188): validate the header, version, and CRC, then decode the
/// versioned body — the fuzz target `idemp_image` covers both consumers through
/// this one function. NO admission logic (lineage/token/anchor) lives here;
/// that stays with the caller who holds `store.meta` context.
pub(crate) fn parse_idemp_image(raw: &[u8]) -> Result<IdempImageParse, IdempImageParseError> {
    let prefix = match crate::store::wire_header::parse(raw, IDEMP_MAGIC) {
        Ok(prefix) => prefix,
        Err(crate::store::wire_header::PrefixError::TooShort { len }) => {
            return Err(IdempImageParseError::Corrupt(
                IdempAuthorityCorruption::TooShort {
                    actual: len,
                    required: crate::store::wire_header::HEADER_LEN,
                },
            ));
        }
        Err(crate::store::wire_header::PrefixError::BadMagic) => {
            return Err(IdempImageParseError::Corrupt(
                IdempAuthorityCorruption::BadMagic,
            ));
        }
    };

    let version = prefix.version;
    if version > IDEMP_VERSION {
        // FutureVersion: hard error, mirror the schema-evolution stance.
        return Err(IdempImageParseError::FutureVersion {
            stored: version,
            current: IDEMP_VERSION,
        });
    }
    if !(1..=IDEMP_VERSION).contains(&version) {
        return Err(IdempImageParseError::Corrupt(
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
        return Err(IdempImageParseError::Corrupt(
            IdempAuthorityCorruption::CrcMismatch {
                stored: stored_crc,
                computed: computed_crc,
            },
        ));
    }

    if version == 1 {
        return match crate::encoding::from_bytes::<Vec<IdempEntry>>(body) {
            Ok(entries) => Ok(IdempImageParse::V1(entries)),
            Err(error) => Err(IdempImageParseError::Corrupt(
                IdempAuthorityCorruption::DecodeFailed(error),
            )),
        };
    }

    match crate::encoding::from_bytes::<IdempImageV2>(body) {
        Ok(image) => Ok(IdempImageParse::V2(image)),
        Err(error) => Err(IdempImageParseError::Corrupt(
            IdempAuthorityCorruption::DecodeFailed(error),
        )),
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
/// a new command. Admission is by GENERATION TOKEN — `store.meta` records
/// exactly which publications it authorized, and only an image carrying the
/// current or pending token is accepted. Frontier arithmetic never admits an
/// image (a sibling fork can be farther ahead, and per-entity hash chains
/// cannot see divergence in other entities); anchors only CLASSIFY refusals.
///
/// * absent, no expectation → [`IdempLoad::Missing`] (fresh store, or a store
///   with no user idempotency obligations);
/// * absent, expectation with a FINALIZED `current_image_id` →
///   [`StoreError::IdempotencyAuthorityMissing`] (authority LOSS, not fresh);
/// * absent, expectation whose only token is PENDING (first publication
///   crashed before the image landed) → [`IdempLoad::Missing`] — the entries
///   are still recoverable from live segment frames;
/// * unreadable / short / wrong magic / bad CRC / decode failure / garbled
///   version → [`StoreError::IdempotencyAuthorityCorrupt`] with a typed reason;
/// * version NEWER than [`IDEMP_VERSION`] → hard
///   [`StoreError::IdempotencyFutureVersion`] (unchanged canonical refusal);
/// * legacy v1 image: admitted ONLY while no expectation exists (pre-migration
///   store; it upgrades to v2 at the next flush) — with an expectation it is
///   [`StoreError::IdempotencyAuthorityStale`] (a v1 image cannot prove it
///   covers the expected frontier);
/// * v2 image, no expectation → ignored as empty ([`IdempLoad::Missing`]):
///   the store has no obligations, and admitting an unauthorized image would
///   let a transplant fabricate receipts on a virgin store;
/// * v2 image with an expectation: lineage must match `store.meta`
///   ([`StoreError::IdempotencyAuthorityForeign`] otherwise); then the image's
///   `image_id` must be the recorded current or pending token. Any other
///   token is refused, classified by the diagnostic anchor —
///   behind the recorded anchor is [`StoreError::IdempotencyAuthorityStale`]
///   (an old self image), at-or-past is
///   [`StoreError::IdempotencyAuthorityForeign`] (a sibling-fork transplant,
///   including "farther ahead" and anchor-identical ones).
pub(crate) fn load_idempotency_authority(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
    store_lineage: Option<u128>,
    expectation: Option<&crate::store::store_meta::IdempAuthorityExpectation>,
) -> Result<IdempLoad, StoreError> {
    let path = data_dir.join(IDEMP_FILENAME);
    let raw = match fs.read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if expectation.is_some_and(|exp| exp.current_image_id.is_some()) {
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

    // The absent/expectation ladder above is open-time specific; the byte-level
    // parse is shared with the restore seam (#188) through `parse_idemp_image`,
    // and its errors map back to the path-bearing open-time refusals here.
    match parse_idemp_image(&raw) {
        Ok(IdempImageParse::V1(entries)) => admit_legacy_v1(entries, expectation),
        Ok(IdempImageParse::V2(image)) => admit_v2_image(image, store_lineage, expectation),
        Err(IdempImageParseError::FutureVersion { stored, current }) => {
            Err(StoreError::IdempotencyFutureVersion { stored, current })
        }
        Err(IdempImageParseError::Corrupt(kind)) => Err(corrupt_authority(&path, kind)),
    }
}

/// Admit a legacy v1 image (bare `Vec<IdempEntry>`, no binding): only while
/// the store has never recorded a durable-authority expectation (it upgrades
/// to v2 at the next flush); with an expectation it is STALE by construction
/// (a v1 image cannot prove it covers the expected frontier). The body is
/// already decoded by [`parse_idemp_image`]; this fn owns only the expectation
/// gate.
fn admit_legacy_v1(
    entries: Vec<IdempEntry>,
    expectation: Option<&crate::store::store_meta::IdempAuthorityExpectation>,
) -> Result<IdempLoad, StoreError> {
    if let Some(expected) = expectation {
        return Err(StoreError::IdempotencyAuthorityStale {
            image_covered: 0,
            expected_covered: expected.anchor.covered_global_sequence,
        });
    }
    Ok(IdempLoad::Loaded(entries))
}

/// Admit a decoded v2 image by generation token: a foreign lineage is never
/// acceptable; with no recorded expectation the image is IGNORED (empty — no
/// obligations exist, and an unauthorized image must not fabricate receipts);
/// with an expectation, only the recorded current or pending token admits.
/// Every other token is a typed refusal, classified by the diagnostic anchor:
/// behind the recorded anchor → an old self image (stale); at-or-past → a
/// sibling-fork transplant (foreign), which covers the farther-ahead and
/// anchor-identical divergence shapes that frontier arithmetic cannot see.
fn admit_v2_image(
    image: IdempImageV2,
    store_lineage: Option<u128>,
    expectation: Option<&crate::store::store_meta::IdempAuthorityExpectation>,
) -> Result<IdempLoad, StoreError> {
    if let Some(lineage) = store_lineage {
        if image.lineage.as_u128() != lineage {
            return Err(StoreError::IdempotencyAuthorityForeign {
                kind: crate::store::IdempAuthorityForeignKind::Lineage,
            });
        }
    }
    let Some(expected) = expectation else {
        // No obligations recorded: an authority-shaped file was never
        // authorized by this store's metadata. Start empty rather than admit
        // (transplant) or refuse (there is nothing to protect).
        tracing::warn!(
            target: "batpak::idemp",
            image_id = image.image_id.as_u128(),
            "unauthorized idempotency image present with no recorded expectation; ignoring as empty"
        );
        return Ok(IdempLoad::Missing);
    };
    if expected.authorizes(image.image_id) {
        return Ok(IdempLoad::Loaded(image.entries));
    }
    let image_covered = image
        .anchor
        .map(|anchor| anchor.covered_global_sequence)
        .unwrap_or(0);
    if image_covered < expected.anchor.covered_global_sequence {
        return Err(StoreError::IdempotencyAuthorityStale {
            image_covered,
            expected_covered: expected.anchor.covered_global_sequence,
        });
    }
    Err(StoreError::IdempotencyAuthorityForeign {
        kind: crate::store::IdempAuthorityForeignKind::HistoryAnchor,
    })
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

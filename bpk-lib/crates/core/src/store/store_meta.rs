//! The store lineage-metadata sidecar (`store.meta`) — the durable home of the
//! store's LINEAGE IDENTITY, the durable-authority AUTHORIZATION (the image
//! generation the store has authorized, plus a diagnostic history anchor), and
//! the compaction COMMIT RECORD (the last durably authorized compaction
//! transition). #205 substrate for #189; #177/#195 for the commit record.
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
//! v1 honesty (A10): `store.meta` has NEVER shipped, so [`STORE_META_VERSION`]
//! `= 1` is the COMPLETED initial schema, not a draft under evolution. There is
//! no supported earlier v1 reader and no additive-tolerance story: every v1
//! field is required-present in the encoding (a `None` value still serializes
//! its field via named-field msgpack), and a body MISSING a required field is
//! CORRUPTION (the fail-closed family), never silently defaulted. No field here
//! carries `#[serde(default)]`.
//!
//! Migration is crash-restartable: `store.meta` is written atomically BEFORE
//! any idempotency-image upgrade can stamp the lineage id; a crash between the
//! two leaves `store.meta` present + a v1 idempotency image, which is simply
//! the migrated steady state until the next flush.
//!
//! The compaction commit record (#177/#195): [`publish_idempotency_authority`]
//! writes a [`CompactionCommit`] in the SAME atomic `store.meta` replace as the
//! idempotency-authority finalize. That single durable fact — not any file
//! name — flips a pending compaction from roll-back to roll-forward; the
//! consumer is `cold_start::rebuild::compaction_recovery`, which requires
//! three-artifact agreement (marker id == commit id == on-disk image id) before
//! rolling forward. The pending marker, the published authority image, and this
//! record all carry the SAME branded ids ([`CompactionId`] +
//! [`AuthorityImageId`]).

use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
use crate::store::index::idemp::IdempVersionWitness;
use crate::store::platform::fs::StoreFs;
use crate::store::{StoreError, StoreMetaCorruption};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const STORE_META_MAGIC: &[u8; 6] = b"FBATSM";
/// On-disk format version stored in the `store.meta` header. Stays `1`: the
/// completed INITIAL schema (`store.meta` never shipped — see the module-level
/// v1-honesty note). A body missing any required v1 field is corruption.
pub const STORE_META_VERSION: u16 = 1;
pub(crate) const STORE_META_FILENAME: &str = "store.meta";

/// Decoded `store.meta` body. Every field is required-present in the encoding
/// (A10): no `#[serde(default)]`, so a body missing one fails closed as
/// [`StoreError::StoreMetadataCorrupt`] rather than silently defaulting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoreMetaData {
    /// The store's lineage identity (UUIDv7 bits minted at first writable
    /// open, from the injected clock seam).
    #[serde(with = "crate::wire::u128_bytes")]
    pub(crate) lineage: u128,
    /// The durable idempotency-authority expectation: `None` while the store
    /// holds no USER idempotency obligations (system lifecycle keys never
    /// create one). Once user-keyed traffic is flushed, this records exactly
    /// which authority image publications `store.meta` authorized (#189).
    pub(crate) idemp_authority: Option<IdempAuthorityExpectation>,
    /// The last durably AUTHORIZED compaction transition (#177/#195): its
    /// branded ids, merged segment id, and sorted source segment ids. `None`
    /// before any compaction has committed. Distinct from the in-progress
    /// pending marker — the marker states a transition IN PROGRESS, this states
    /// the last one AUTHORIZED. Written atomically with the idempotency-authority
    /// finalize; consulted by compaction recovery only while a pending marker
    /// with a matching `compaction_id` exists, then inert diagnostics.
    pub(crate) last_compaction_commit: Option<CompactionCommit>,
}

/// The compaction COMMIT RECORD (#177/#195): written in the SAME atomic
/// `store.meta` replace as the idempotency-authority finalize, it is the single
/// durable fact that flips a pending compaction from roll-back to roll-forward.
/// Recovery (`cold_start::rebuild::compaction_recovery`) rolls forward only when
/// the live marker's `compaction_id` AND the on-disk authority image's
/// `image_id`/`compaction_id` all agree with this record — file presence never
/// votes. Afterwards (marker cleared) it is inert diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CompactionCommit {
    /// Branded transaction token, minted in `lifecycle_compact` and stamped
    /// into the pending marker; identical here and in the authority image.
    pub(crate) compaction_id: CompactionId,
    /// Branded identity of the authority image this transition published —
    /// equals the image's own `image_id` and the finalized `current_image_id`
    /// (stamped by [`publish_idempotency_authority`], so store.meta and image
    /// agree by construction).
    pub(crate) authority_image_id: AuthorityImageId,
    /// Segment id of the committed replacement segment.
    pub(crate) merged_segment_id: u64,
    /// The retired source segment ids, sorted.
    pub(crate) source_segment_ids: Vec<u64>,
}

/// The `store.meta`-side authorization record for the durable idempotency
/// authority image. Admission is by IMAGE-GENERATION TOKEN
/// ([`AuthorityImageId`]), not by frontier arithmetic: every publication mints
/// a fresh id, `store.meta` authorizes it BEFORE the image is published, and
/// cold start accepts only an image carrying the current or the pending token.
/// A sibling fork's image — even one farther ahead, even one whose anchor
/// collides exactly (per-entity hash chains cannot see divergence in other
/// entities) — was never authorized by THIS store's metadata and is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdempAuthorityExpectation {
    /// Generation token of the last FINALIZED image publication. `None` while
    /// the very first publication is still in flight (a crash there leaves
    /// entries recoverable from live segment frames, so a missing image is
    /// legally fresh).
    pub(crate) current_image_id: Option<AuthorityImageId>,
    /// Generation token of an in-flight publication: set before the image is
    /// published, cleared by the finalize step. Present only across the
    /// publish crash window.
    pub(crate) pending_image_id: Option<AuthorityImageId>,
    /// Diagnostic history anchor at authorization time — used to CLASSIFY an
    /// unauthorized image (stale vs foreign) for the refusal message, never to
    /// admit one.
    pub(crate) anchor: IdempAuthorityAnchor,
}

impl IdempAuthorityExpectation {
    /// Whether `image_id` is one of the publications this store authorized
    /// (the finalized current image, or the in-flight pending one across the
    /// publish crash window).
    pub(crate) fn authorizes(&self, image_id: AuthorityImageId) -> bool {
        self.current_image_id == Some(image_id) || self.pending_image_id == Some(image_id)
    }
}

/// The history anchor recorded alongside an authorization (frontier sequence +
/// the event id / event-hash commitment at that sequence). Diagnostic only:
/// per-entity hash chains cannot commit to full store history, so anchors
/// classify refusals; the generation token decides admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdempAuthorityAnchor {
    /// Highest recorded global sequence covered by the flushed image.
    pub(crate) covered_global_sequence: u64,
    /// The event id committed at that sequence.
    #[serde(with = "crate::wire::u128_bytes")]
    pub(crate) event_id_at: u128,
    /// The committed event's content-hash chain value at that sequence (NOT a
    /// signed seal and NOT a global history commitment — the authenticated
    /// spine is #190/#191).
    pub(crate) chain_commitment: [u8; 32],
}

/// Load `store.meta`, failing closed on corruption.
///
/// - No file at the expected path → `Ok(None)` (resolution against the
///   post-migration witness happens in [`resolve_on_open`]).
/// - Valid file → `Ok(Some(meta))`.
/// - Present but unreadable / wrong magic / bad CRC / decode failure (INCLUDING
///   a body missing any required v1 field, per A10) / unsupported version →
///   [`StoreError::StoreMetadataCorrupt`].
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
                last_compaction_commit: None,
            };
            write_store_meta(data_dir, &meta, fs)?;
            Ok(Some(meta))
        }
        IdempVersionWitness::Unreadable => Ok(None),
    }
}

/// Publish the durable idempotency authority AND advance the recorded
/// authorization, as one protocol (GAUNT-IDEMPOTENCY-AUTHORITY, #189), OPTIONALLY
/// stamping a compaction commit record (#177/#195) in the finalize write. Runs
/// only from quiesced paths (close, compaction tail, snapshot, fork) of a
/// WRITABLE store, whose `store.meta` was minted/loaded at open.
///
/// Authority exists only for real USER idempotency obligations: when the durable
/// map holds no user-keyed entries (system lifecycle keys are internal
/// one-shots), no image is written or required — a previously recorded
/// expectation is retired and the image removed, so ordinary stores never
/// acquire a fail-closed dependency on `index.idemp`. A `commit` still lands even
/// on that retire branch (the segment merge happened and must be recorded).
///
/// Publication protocol (order is load-bearing; every crash point yields a state
/// the admission rule accepts):
/// 1. authorize: persist `store.meta` carrying a freshly minted random
///    `pending_image_id` (the previous `current_image_id` stays authorized;
///    `last_compaction_commit` is PRESERVED verbatim here — the commit only
///    lands at finalize);
/// 2. publish: flush the v2 image stamped with the lineage, the pending token
///    (its `image_id`), the previous token (`previous_image_id`), the
///    `commit`'s `CompactionId` (so the image and store.meta carry the SAME
///    branded ids — A4), and the map's own tip anchor (derived from the small
///    durable map — never an O(all-events) index sweep);
/// 3. finalize: persist `store.meta` with `current = pending, pending = None`
///    AND `last_compaction_commit = commit.or(previous)` — this ONE atomic
///    replace is simultaneously the idempotency-token finalize and (when
///    `commit` is `Some`) the compaction commit point. The stamped
///    `authority_image_id` equals the finalized token, so recovery's
///    three-artifact agreement holds by construction.
///
/// The random per-publication token is what makes a diverged sibling's image
/// unacceptable even when it is "farther ahead" or anchor-identical: sibling
/// branches cannot independently mint each other's next generation.
pub(crate) fn publish_idempotency_authority(
    idemp: &crate::store::index::idemp::IdempotencyStore,
    data_dir: &Path,
    fs: &dyn StoreFs,
    clock: &dyn crate::store::Clock,
    commit: Option<CompactionCommit>,
) -> Result<(), StoreError> {
    let mut meta = load_store_meta(data_dir, fs)?.ok_or_else(|| {
        // A writable store minted store.meta at open; its absence here means
        // it vanished mid-session — refuse rather than remint (#205).
        StoreError::StoreMetadataMissing {
            path: data_dir.join(STORE_META_FILENAME),
        }
    })?;

    let Some(anchor) = idemp.user_obligation_anchor() else {
        // No user obligations: retire any recorded authority image. Meta first
        // (authorization is the source of truth), then the image — a crash
        // between the two leaves an unauthorized image that cold start ignores
        // as empty. A supplied `commit` still lands: the segment merge happened
        // and the commit record must flip recovery to roll-forward, even though
        // this store has no authority image; the stamped `authority_image_id`
        // records the generation this retire authorized.
        let stamped = commit.map(|c| stamp_commit(c, AuthorityImageId::mint(clock)));
        if stamped.is_some() || meta.idemp_authority.is_some() {
            meta.idemp_authority = None;
            meta.last_compaction_commit = stamped.or(meta.last_compaction_commit);
            write_store_meta(data_dir, &meta, fs)?;
        }
        fs.remove_file_if_present(&data_dir.join(crate::store::index::idemp::IDEMP_FILENAME))
            .map_err(StoreError::Io)?;
        return Ok(());
    };

    // 1. Authorize a fresh publication token. `last_compaction_commit` is left
    //    exactly as loaded — the commit record only lands at finalize.
    let pending = AuthorityImageId::mint(clock);
    let current = meta
        .idemp_authority
        .as_ref()
        .and_then(|expectation| expectation.current_image_id);
    meta.idemp_authority = Some(IdempAuthorityExpectation {
        current_image_id: current,
        pending_image_id: Some(pending),
        anchor,
    });
    write_store_meta(data_dir, &meta, fs)?;

    // 2. Publish the image carrying that token + the branded compaction stamp.
    let compaction_id = commit.as_ref().map(|c| c.compaction_id);
    idemp.flush(
        data_dir,
        fs,
        StoreLineage::from_u128(meta.lineage),
        Some(anchor),
        pending,
        current,
        compaction_id,
    )?;

    // 3. Finalize the authorization AND (atomically) the compaction commit. The
    //    stamped `authority_image_id` is the now-current token, so store.meta and
    //    the published image agree by construction.
    meta.idemp_authority = Some(IdempAuthorityExpectation {
        current_image_id: Some(pending),
        pending_image_id: None,
        anchor,
    });
    meta.last_compaction_commit = commit
        .map(|c| stamp_commit(c, pending))
        .or(meta.last_compaction_commit);
    write_store_meta(data_dir, &meta, fs)?;
    Ok(())
}

/// Flush the durable idempotency authority AND advance the recorded
/// authorization, with NO compaction commit (the close/snapshot/fork callers).
/// A thin wrapper over [`publish_idempotency_authority`] passing `commit: None`
/// — every existing call site (`lifecycle.rs`, `lifecycle_snapshot.rs`,
/// `lifecycle_fork.rs`) stays unchanged.
pub(crate) fn flush_idempotency_authority(
    index: &crate::store::index::StoreIndex,
    data_dir: &Path,
    fs: &dyn StoreFs,
    clock: &dyn crate::store::Clock,
) -> Result<(), StoreError> {
    publish_idempotency_authority(&index.idemp, data_dir, fs, clock, None)
}

/// Stamp a caller-supplied compaction descriptor with the AUTHORITY IMAGE id the
/// publication actually minted, so the record in `store.meta` and the published
/// image carry identical branded ids (A4). The caller cannot know the image id
/// (it is minted inside [`publish_idempotency_authority`]); this fn fills it.
fn stamp_commit(commit: CompactionCommit, authority_image_id: AuthorityImageId) -> CompactionCommit {
    CompactionCommit {
        authority_image_id,
        ..commit
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

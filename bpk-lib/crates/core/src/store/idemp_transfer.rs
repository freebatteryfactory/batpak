//! Public out-of-band idempotency-authority export/restore (#188).
//!
//! The store's durable dedup authority lives in ONE canonical on-disk image
//! (`index.idemp`, FBATID v2). This module exposes it as an opaque, transportable
//! byte blob and a closed-directory restore ceremony so an embedder can back the
//! authority up out-of-band and heal the primary disaster: `index.idemp` lost
//! while `store.meta` survives, which makes the store refuse to open.
//!
//! Restore is OFFLINE-PRIMARY (#226 A13): it operates on a CLOSED directory under
//! the store-dir lock and IS the authorization ceremony. It never asks the dead
//! store to have pre-authorized the incoming image's token (that would be
//! circular). It validates lineage, coverage, and rollback, refuses while a
//! pending-compaction transaction is unsettled, MINTS a fresh local authority
//! image id, and republishes through the canonical `store.meta`
//! authorize→publish→finalize flow — leaving the directory openable. There is no
//! live "adopt" door: keyed-admission exclusion is structural (the directory is
//! closed and locked), not a writer-thread command.

use crate::store::index::idemp::{
    parse_idemp_image, IdempEntry, IdempImageParse, IdempImageParseError, IdempImageV2,
    IdempotencyRetention, IdempotencyStore, OverflowPolicy, IDEMP_FILENAME,
};
use crate::store::store_meta::{load_store_meta, StoreMetaData, STORE_META_FILENAME};
use crate::store::{
    IdempAuthorityCorruption, IdempotencyRestoreRefusal, Open, Store, StoreConfig, StoreError,
    StoreLockMode,
};
use std::path::Path;
use std::sync::Arc;

/// An opaque, transportable snapshot of a store's durable idempotency authority
/// (#188). The bytes are EXACTLY the canonical on-disk authority image; the
/// layout is not a public contract — transport and store them verbatim.
///
/// The redacted [`Debug`] deliberately prints only the length: entries carry
/// per-event entity/scope strings and receipt extensions that must never leak
/// into a log line.
#[derive(Clone, PartialEq, Eq)]
pub struct IdempotencyAuthorityExport {
    bytes: Vec<u8>,
}

impl std::fmt::Debug for IdempotencyAuthorityExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdempotencyAuthorityExport")
            .field("len", &self.bytes.len())
            .finish()
    }
}

impl IdempotencyAuthorityExport {
    /// Wrap raw export bytes. No validation happens here — restore validates the
    /// bytes against the target store's durable state.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Borrow the opaque export bytes for out-of-band persistence/transport.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the export, yielding the owned opaque bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Length in bytes of the opaque export.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the export carries no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Store<Open> {
    /// Export this store's durable idempotency authority as opaque bytes (#188).
    ///
    /// Runs the canonical publication protocol under the visibility fence (the
    /// same quiesce discipline as snapshot/fork) so the exported bytes are a
    /// finalized generation, then reads back the canonical `index.idemp` image.
    /// A store with no USER idempotency obligations has no authority image; the
    /// flush retires it and this returns `Ok(None)`.
    ///
    /// # Errors
    /// - [`StoreError::VisibilityFenceActive`] if a public visibility fence is
    ///   already active on this store.
    /// - [`StoreError::WriterCrashed`] if the writer thread has poisoned.
    /// - [`StoreError::StoreMetadataMissing`] if `store.meta` vanished mid-session.
    /// - [`StoreError::Io`] on a read/flush failure.
    pub fn export_idempotency_authority(
        &self,
    ) -> Result<Option<IdempotencyAuthorityExport>, StoreError> {
        tracing::debug!(target: "batpak::idemp", flow = "export_idempotency");
        let fs = self.config.fs();
        let _lifecycle = self.lifecycle_gate.lock();
        let fence = self.begin_visibility_fence()?;
        crate::store::lifecycle::sync(self)?;
        crate::store::store_meta::flush_idempotency_authority(
            &self.index,
            &self.config.data_dir,
            fs.as_ref(),
            self.runtime.clock(),
        )?;
        let path = self.config.data_dir.join(IDEMP_FILENAME);
        let export = match fs.read(&path) {
            Ok(bytes) => Some(IdempotencyAuthorityExport::from_bytes(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                fence.cancel()?;
                return Err(StoreError::Io(error));
            }
        };
        fence.cancel()?;
        Ok(export)
    }

    /// Restore a caller-supplied idempotency-authority export into a CLOSED store
    /// directory (#188, offline-primary #226 A13).
    ///
    /// This associated fn takes no live store: it acquires the store-dir lock,
    /// refuses while a pending-compaction transaction is unsettled, validates the
    /// export against `store.meta` (lineage, coverage, rollback, sibling
    /// divergence), then MINTS a fresh local authority image id and republishes
    /// through the canonical `store.meta` authorize→publish→finalize flow. The
    /// restore call is itself the authorization ceremony — the incoming image's
    /// own token is never trusted. Live durable entries win over restored ones
    /// on any key overlap. On success the directory is openable and cold start
    /// re-validates the freshly minted generation by the existing token law.
    ///
    /// # Errors
    /// - [`StoreError::StoreLocked`] if a live store holds the directory.
    /// - [`StoreError::StoreMetadataMissing`] / [`StoreError::StoreMetadataCorrupt`]
    ///   if `store.meta` is absent or unreadable.
    /// - [`StoreError::CompactionRecoveryRefused`] if a legacy/corrupt compaction
    ///   marker blocks recovery.
    /// - [`StoreError::IdempotencyRestoreRefused`] with a typed reason for a
    ///   corrupt/future-format/foreign-lineage/stale/diverged/malformed export or
    ///   an unresolved pending-compaction transaction.
    /// - [`StoreError::Io`] on a read/write failure.
    pub fn restore_idempotency_authority(
        config: &StoreConfig,
        export: &IdempotencyAuthorityExport,
    ) -> Result<(), StoreError> {
        tracing::debug!(target: "batpak::idemp", flow = "restore_idempotency");
        let fs = config.fs();
        let data_dir = config.data_dir.as_path();
        let _lock = crate::store::dir_lock::StoreDirLock::acquire(
            data_dir,
            StoreLockMode::Mutable,
            fs.as_ref(),
        )?;

        // A directory mid-compaction has an unsettled durable authority; a
        // writable open must complete recovery before an export can be admitted.
        if crate::store::cold_start::rebuild::load_pending_compaction(data_dir, fs.as_ref())?
            .is_some()
        {
            return Err(refuse(IdempotencyRestoreRefusal::PendingCompactionUnresolved {
                marker_path: data_dir
                    .join(crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME),
            }));
        }

        let image = parse_export_image(export.as_bytes())?;
        validate_export_structure(&image)?;

        let meta = load_store_meta(data_dir, fs.as_ref())?.ok_or_else(|| {
            StoreError::StoreMetadataMissing {
                path: data_dir.join(STORE_META_FILENAME),
            }
        })?;
        check_lineage_rollback_divergence(&image, &meta)?;

        // Seed the merge base with any healthy existing durable entries so a
        // restore never DELETES live authority (live-wins), then insert only the
        // export's absent keys and republish through the canonical protocol,
        // which mints the fresh authority image id and updates store.meta.
        let present = load_present_entries(data_dir, fs.as_ref(), meta.lineage);
        let idemp = IdempotencyStore::new(IdempotencyRetention::default(), OverflowPolicy::default());
        idemp.restore(present);
        let outcome = idemp.merge_restored(image.entries);
        tracing::info!(
            target: "batpak::idemp",
            inserted = outcome.inserted,
            preserved_live = outcome.preserved_live,
            "restored idempotency authority merged"
        );

        let clock = resolve_clock(config);
        crate::store::store_meta::publish_idempotency_authority(
            &idemp,
            data_dir,
            fs.as_ref(),
            clock.as_ref(),
            None,
        )
    }
}

/// Wrap a typed restore refusal in its [`StoreError`] carrier.
fn refuse(reason: IdempotencyRestoreRefusal) -> StoreError {
    StoreError::IdempotencyRestoreRefused { reason }
}

/// Parse opaque export bytes into the canonical v2 authority image, mapping the
/// shared decision core's failures to typed restore refusals. A legacy v1 body
/// carries no lineage/anchor binding and is not a restorable canonical image, so
/// it is refused as an unsupported format.
fn parse_export_image(bytes: &[u8]) -> Result<IdempImageV2, StoreError> {
    match parse_idemp_image(bytes) {
        Ok(IdempImageParse::V2(image)) => Ok(image),
        Ok(IdempImageParse::V1(_)) => Err(refuse(IdempotencyRestoreRefusal::Corrupt {
            kind: IdempAuthorityCorruption::UnsupportedVersion {
                observed: 1,
                expected: crate::store::index::idemp::IDEMP_VERSION,
            },
        })),
        Err(IdempImageParseError::Corrupt(kind)) => {
            Err(refuse(IdempotencyRestoreRefusal::Corrupt { kind }))
        }
        Err(IdempImageParseError::FutureVersion { stored, current }) => {
            Err(refuse(IdempotencyRestoreRefusal::FutureVersion {
                found: stored,
                supported: current,
            }))
        }
    }
}

/// Coverage structure check (intrinsic to the image): no USER entry may be
/// recorded past the image's own declared covered frontier — no canonical
/// publication produces that shape. SYSTEM entries are exempt: the canonical
/// flush anchors at the max USER entry, so legal exports routinely carry system
/// lifecycle keys recorded past the anchor. An image with no anchor treats its
/// coverage as 0, so any user entry is beyond it.
fn validate_export_structure(image: &IdempImageV2) -> Result<(), StoreError> {
    let covered = image.anchor.map(|anchor| anchor.covered_global_sequence);
    for entry in &image.entries {
        if entry.kind.is_reserved() {
            continue;
        }
        let offender = entry.recorded_global_sequence.max(entry.global_sequence);
        match covered {
            None => {
                return Err(refuse(IdempotencyRestoreRefusal::EntriesBeyondCoverage {
                    covered_global_sequence: 0,
                    first_offender_sequence: offender,
                }));
            }
            Some(covered_global_sequence) if offender > covered_global_sequence => {
                return Err(refuse(IdempotencyRestoreRefusal::EntriesBeyondCoverage {
                    covered_global_sequence,
                    first_offender_sequence: offender,
                }));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// Door validation against `store.meta` (offline, closed directory):
/// - the export must belong to THIS store's lineage;
/// - it must not roll authority coverage backwards;
/// - if it claims the exact recorded frontier with a different event identity,
///   it is a diverged sibling fork's image.
///
/// Frontier arithmetic never ADMITS an image (the fresh mint at republication is
/// the authorization); these are the offline-decidable refusals.
fn check_lineage_rollback_divergence(
    image: &IdempImageV2,
    meta: &StoreMetaData,
) -> Result<(), StoreError> {
    if image.lineage.as_u128() != meta.lineage {
        return Err(refuse(IdempotencyRestoreRefusal::ForeignLineage {
            image_lineage: crate::id::StoreIdentity::from_u128(image.lineage.as_u128()),
            store_lineage: crate::id::StoreIdentity::from_u128(meta.lineage),
        }));
    }
    let Some(expectation) = meta.idemp_authority.as_ref() else {
        return Ok(());
    };
    let expected = expectation.anchor;
    let image_covered = image
        .anchor
        .map(|anchor| anchor.covered_global_sequence)
        .unwrap_or(0);
    if image_covered < expected.covered_global_sequence {
        return Err(refuse(IdempotencyRestoreRefusal::StaleRollback {
            image_covered,
            expected_covered: expected.covered_global_sequence,
        }));
    }
    if let Some(anchor) = image.anchor {
        if anchor.covered_global_sequence == expected.covered_global_sequence
            && (anchor.event_id_at != expected.event_id_at
                || anchor.chain_commitment != expected.chain_commitment)
        {
            return Err(refuse(IdempotencyRestoreRefusal::SiblingDivergence {
                event_id: crate::id::EventId::from_u128(expected.event_id_at),
                global_sequence: expected.covered_global_sequence,
            }));
        }
    }
    Ok(())
}

/// Read the existing on-disk authority entries to seed the live-wins merge base.
/// LENIENT by design: a lost sidecar (the primary disaster) or an unparseable /
/// foreign / legacy image yields an empty base — that is exactly what restore
/// heals. Only a same-lineage v2 image contributes entries the restore must
/// preserve.
fn load_present_entries(
    data_dir: &Path,
    fs: &dyn crate::store::platform::fs::StoreFs,
    store_lineage: u128,
) -> Vec<IdempEntry> {
    let raw = match fs.read(&data_dir.join(IDEMP_FILENAME)) {
        Ok(bytes) => bytes,
        Err(_read_failed_or_absent) => return Vec::new(),
    };
    match parse_idemp_image(&raw) {
        Ok(IdempImageParse::V2(image)) if image.lineage.as_u128() == store_lineage => image.entries,
        Ok(IdempImageParse::V2(_)) | Ok(IdempImageParse::V1(_)) | Err(_) => Vec::new(),
    }
}

/// Resolve the clock for offline republication: the config's injected clock, or
/// the process [`SystemClock`](crate::store::platform::clock::SystemClock) when
/// none was configured (mirrors `store.meta`'s never-ambient-time discipline).
fn resolve_clock(config: &StoreConfig) -> Arc<dyn crate::store::Clock> {
    match &config.clock {
        Some(clock) => Arc::clone(clock),
        None => Arc::new(crate::store::platform::clock::SystemClock::new()),
    }
}

#[cfg(test)]
#[path = "idemp_transfer_tests.rs"]
mod tests;

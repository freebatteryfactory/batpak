//! Sealed-segment compaction as a namespace transaction with an explicit,
//! token-bound commit record (#177/#195).
//!
//! Protocol (the order IS the law — A11):
//!   P1  write the v2 pending marker (`compaction.pending`), carrying the freshly
//!       minted `compaction_id`, the store lineage, the source set, and the
//!       expected authority-image id.
//!   P2  rename the merged-id source away to `.compact-src` (+ parent sync), so
//!       the FINAL name is ABSENT until the commit rename.
//!   P3/P4  materialize the replacement at the STAGED name `.compact-new`, fully
//!       written, synced, sealed, and name-durable — the final name is NEVER
//!       created pre-commit.
//!   P5/P6  evict the durable idempotency image against the FRESH (staged-view)
//!       index on a SHADOW clone, publish it, then FINALIZE. The finalize
//!       `store.meta` replace carries the `CompactionCommit` and IS the atomic
//!       commit point: recovery direction is decided by the commit record
//!       matching the marker, never by file existence.
//!   P7  publish the final namespace: rename staged -> final (+ parent sync).
//!   swap  adopt the fresh index into the live index in ONE publication (reads
//!       resolve merged_id to the final name, which exists only post-rename).
//!   P8  retire the source segments (+ one parent sync).
//!   P9  clear the marker LAST.
//!
//! A pre-commit failure (through P6) rolls the disk state back to the verified
//! source set with the live index untouched. A failure AFTER the disk commit but
//! BEFORE the in-memory swap cannot roll back (the commit record is durable), so
//! it POISONS the store fail-closed: every further writer command is rejected
//! with `StoreError::WriterCrashed` and a reopen physically completes the
//! roll-forward. No return path leaves memory and disk on different generations
//! while the store serves commands.

use super::sync;
use crate::coordinate::Coordinate;
use crate::event::{Event, EventKind, StoredEvent};
use crate::store::file_classification::StoreFileKind;
use crate::store::lifecycle_close::write_cold_start_artifacts_on_close;
use crate::store::platform::fs::StoreFs;
use crate::store::segment::scan as reader;
use crate::store::segment::{self, Active, FramePayload};
use crate::store::{CompactionConfig, CompactionStrategy, Open, Store, StoreError};
use std::path::{Path, PathBuf};

pub(crate) fn compact(
    store: &Store<Open>,
    config: &CompactionConfig,
) -> Result<
    (
        segment::CompactionResult,
        crate::store::compaction_report::CompactionReportBody,
    ),
    StoreError,
> {
    tracing::debug!(target: "batpak::flow", flow = "compact");
    let fs = store.config.fs();
    let _lifecycle = store.lifecycle_gate.lock();
    sync(store)?;

    let data_dir = &store.config.data_dir;

    // A live marker on an OPEN store means a prior in-process compaction failed
    // after its disk commit but before the in-memory swap (the store was poisoned
    // per A11). Never overwrite a live transaction; the next open repairs it.
    if crate::store::cold_start::rebuild::load_pending_compaction(data_dir, fs.as_ref())?.is_some() {
        return Err(StoreError::CompactionRecoveryRefused {
            marker_path: data_dir
                .join(crate::store::cold_start::rebuild::COMPACTION_MARKER_FILENAME),
            kind: crate::store::CompactionRecoveryRefusal::PendingTransactionUnresolved,
        });
    }

    // Enumerate every segment; the full list feeds the staged-view rebuild.
    let mut all_segments: Vec<(u64, PathBuf)> = Vec::new();
    for entry in fs.read_dir(data_dir).map_err(StoreError::Io)? {
        let path = data_dir.join(&entry.name);
        let seg_id = match StoreFileKind::from_path(&path) {
            StoreFileKind::Segment(segment_id) => segment_id.as_u64(),
            StoreFileKind::MalformedSegment(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "skipping malformed segment filename"
                );
                continue;
            }
            StoreFileKind::VisibilityRanges
            | StoreFileKind::Checkpoint
            | StoreFileKind::MmapIndex
            | StoreFileKind::IdempotencyStore
            | StoreFileKind::PendingCompactionMarker
            | StoreFileKind::CompactSource
            | StoreFileKind::CompactStaged
            | StoreFileKind::CursorDirectory
            | StoreFileKind::Keyset
            | StoreFileKind::StoreMeta
            | StoreFileKind::Other => continue,
        };
        all_segments.push((seg_id, path));
    }
    all_segments.sort_by_key(|(id, _)| *id);

    let active_segment_id = all_segments.last().map(|(id, _)| *id).unwrap_or(0);
    let mut sealed: Vec<(u64, PathBuf)> = all_segments
        .iter()
        .filter(|(id, _)| *id < active_segment_id)
        .cloned()
        .collect();

    if sealed.len() < config.min_segments {
        let result = segment::CompactionResult {
            outcome: segment::CompactionOutcome::Skipped,
            segments_removed: 0,
            bytes_reclaimed: 0,
        };
        let report =
            crate::store::compaction_report::report_skipped(config, active_segment_id, &sealed)?;
        return Ok((result, report));
    }

    let merged_id = sealed[0].0;
    let final_path = data_dir.join(segment::segment_filename(merged_id));
    let staged_path = data_dir.join(format!(
        "{merged_id:06}.{}.{}",
        segment::SEGMENT_EXTENSION,
        crate::store::file_classification::COMPACT_STAGED_EXTENSION
    ));
    let src_path = data_dir.join(format!(
        "{merged_id:06}.{}.{}",
        segment::SEGMENT_EXTENSION,
        crate::store::file_classification::COMPACT_SOURCE_EXTENSION
    ));
    // `sealed` is sorted ascending, so the source id set is sorted.
    let source_segment_ids: Vec<u64> = sealed.iter().map(|(seg_id, _)| *seg_id).collect();
    let mut compact_source_path: Option<PathBuf> = None;

    // A writable store minted store.meta at open; a mid-session absence refuses,
    // mirroring `publish_idempotency_authority`.
    let meta = crate::store::store_meta::load_store_meta(data_dir, fs.as_ref())?.ok_or_else(|| {
        StoreError::StoreMetadataMissing {
            path: data_dir.join(crate::store::store_meta::STORE_META_FILENAME),
        }
    })?;

    // The transaction token and the expected authority-image id. All three
    // artifacts (marker, image, store.meta commit record) must agree on the
    // `compaction_id` for recovery to roll forward (A8).
    let compaction_id = crate::store::generation_ids::CompactionId::mint(store.runtime.clock());
    let authority_image_id =
        crate::store::generation_ids::AuthorityImageId::mint(store.runtime.clock());

    crate::store::cold_start::rebuild::write_pending_compaction(
        data_dir,
        &crate::store::cold_start::rebuild::PendingCompactionV2 {
            compaction_id,
            lineage: crate::store::generation_ids::StoreLineage::from_u128(meta.lineage),
            merged_id,
            source_segment_ids: source_segment_ids.clone(),
            expected_authority_image_id: authority_image_id,
        },
        fs.as_ref(),
    )?;

    let commit = crate::store::store_meta::CompactionCommit {
        compaction_id,
        authority_image_id,
        merged_segment_id: merged_id,
        source_segment_ids: source_segment_ids.clone(),
    };

    // PRE-COMMIT PHASE (relocate source, materialize staged replacement, build the
    // fresh index, publish the authority image + finalize). Any error here routes
    // through the disk rollback: the source set is restored and the live index was
    // never touched.
    let fresh_index = match materialize_and_commit(
        store,
        &config.strategy,
        &all_segments,
        &mut sealed,
        merged_id,
        &staged_path,
        &src_path,
        &source_segment_ids,
        &mut compact_source_path,
        commit,
    ) {
        Ok(fresh_index) => fresh_index,
        Err(error) => {
            return failed_compaction_with_rollback(&FailedCompactionCtx {
                config,
                active_segment_id,
                sealed: &sealed,
                merged_segment_id: merged_id,
                data_dir,
                staged_path: &staged_path,
                final_path: &final_path,
                compact_source_path: compact_source_path.as_deref(),
                error: &error,
                context: "compaction pre-commit phase failed",
                fs: fs.as_ref(),
            });
        }
    };

    // COMMITTED TAIL — the commit record is durable. Publishing the final
    // namespace (rename) and swapping the live index must both succeed; a failure
    // in this pre-swap window cannot roll back and POISONS the store (A11).
    if let Err(error) = fs.rename(&staged_path, &final_path).map_err(StoreError::Io) {
        return Err(poison_after_commit(store, error));
    }
    if let Err(error) = fs.sync_parent_dir(&final_path) {
        return Err(poison_after_commit(store, error));
    }
    if let Err(error) = store.index.replace_contents_from_fresh(fresh_index) {
        return Err(poison_after_commit(store, error));
    }

    // Live catch-up eviction: the durable image was already flushed from the
    // shadow; the live map's divergence is ordinary between-flush staleness. This
    // is post-swap, so a failure cannot split the memory/disk generation —
    // best-effort.
    store.index.mark_idemp_evicted_against_live();
    let _ = store.index.idemp.evict(store.index.global_sequence());

    // Retire the sources — post-swap. A failure here returns `Err` with NO poison:
    // memory and disk are both the NEW generation, and the on-disk state is a legal
    // roll-forward the next open completes. The merged-id entry now points at
    // `.compact-src` (relocated during materialization), so this batch retires it too.
    let mut bytes_reclaimed = 0_u64;
    let mut segments_removed = 0_usize;
    for (_, path) in sealed.iter() {
        if let Ok(meta) = fs.metadata(path) {
            bytes_reclaimed += meta.len;
        }
        fs.remove_file(path).map_err(StoreError::Io)?;
        segments_removed += 1;
    }
    // One parent-dir sync makes the whole removal batch durable strictly before
    // the marker removal begins (closes the mixed-generation resurrection window).
    fs.sync_parent_dir(&final_path)?;

    crate::store::cold_start::rebuild::clear_pending_compaction(data_dir, fs.as_ref())?;

    if let Err(e) = write_cold_start_artifacts_on_close(store) {
        tracing::warn!("post-compaction cold-start artifact write failed: {e}");
    }

    let result = segment::CompactionResult {
        outcome: segment::CompactionOutcome::Performed,
        segments_removed,
        bytes_reclaimed,
    };
    let report = crate::store::compaction_report::report_for_run(
        config,
        active_segment_id,
        &sealed,
        Some(merged_id),
        &result,
        Some(&final_path),
        fs.as_ref(),
    )?;
    Ok((result, report))
}

/// Poison the store after a post-commit / pre-swap failure (A11). The compaction
/// committed on disk (the `store.meta` commit record is durable) but the final
/// rename or the in-memory swap failed, so the store cannot serve a consistent
/// generation. Rollback is impossible; the store is latched fail-closed — every
/// subsequent writer command is rejected with `StoreError::WriterCrashed` and a
/// reopen physically completes the roll-forward. Returns the triggering error
/// unchanged (subsequent commands surface the poison error).
fn poison_after_commit(store: &Store<Open>, error: StoreError) -> StoreError {
    store.watermark_handle.mark_writer_crashed();
    tracing::error!(
        target: "batpak::flow",
        flow = "compact",
        %error,
        "compaction committed on disk but a post-commit step failed; store poisoned — reopen to complete recovery"
    );
    error
}

/// PRE-COMMIT: materialize the replacement off-side, build the fresh index from
/// the staged view, and publish + finalize the authority image (the finalize IS
/// the atomic commit point). Returns the fresh index for the post-commit swap.
fn materialize_and_commit(
    store: &Store<Open>,
    strategy: &CompactionStrategy,
    all_segments: &[(u64, PathBuf)],
    sealed: &mut [(u64, PathBuf)],
    merged_id: u64,
    staged_path: &Path,
    src_path: &Path,
    source_segment_ids: &[u64],
    compact_source_path: &mut Option<PathBuf>,
    commit: crate::store::store_meta::CompactionCommit,
) -> Result<crate::store::index::StoreIndex, StoreError> {
    materialize_compacted_segment(
        store,
        strategy,
        sealed,
        merged_id,
        staged_path,
        src_path,
        compact_source_path,
    )?;

    let fresh_index =
        build_fresh_index_from_staged_view(store, all_segments, source_segment_ids, merged_id, staged_path)?;

    // Evict the durable authority image against the FRESH index on a SHADOW clone,
    // then publish + finalize. The live map keeps serving appends untouched; only
    // the shadow (and the durable image) are evicted here.
    let shadow = store.index.idemp.shadow_clone();
    fresh_index.mark_idemp_shadow_evicted(&shadow);
    let frontier = fresh_index.global_sequence();
    let eviction = shadow.evict(frontier);
    tracing::debug!(
        target: "batpak::idemp",
        flow = "compact",
        frontier,
        aged_out = eviction.aged_out,
        cap_trimmed = eviction.cap_trimmed_out_of_window,
        within_window_exceeds_cap = eviction.within_window_exceeds_cap,
        remaining = eviction.remaining,
        "applied window-priority idempotency eviction against the fresh compaction index"
    );
    crate::store::store_meta::publish_idempotency_authority(
        &shadow,
        &store.config.data_dir,
        store.config.fs().as_ref(),
        store.runtime.clock(),
        Some(commit),
    )?;

    Ok(fresh_index)
}

/// Build the fresh in-memory index from the STAGED VIEW — the staged replacement
/// plus every segment that is NOT a compaction source (the active segment and any
/// non-source sealed segments). A data-dir scan cannot be used: `segment_paths`
/// fails closed while the pending marker is live.
fn build_fresh_index_from_staged_view(
    store: &Store<Open>,
    all_segments: &[(u64, PathBuf)],
    source_segment_ids: &[u64],
    merged_id: u64,
    staged_path: &Path,
) -> Result<crate::store::index::StoreIndex, StoreError> {
    let source_set: std::collections::HashSet<u64> = source_segment_ids.iter().copied().collect();
    let mut view: Vec<(u64, PathBuf)> = all_segments
        .iter()
        .filter(|(id, _)| !source_set.contains(id))
        .cloned()
        .collect();
    view.push((merged_id, staged_path.to_path_buf()));
    view.sort_by_key(|(id, _)| *id);

    let fresh_index = crate::store::index::StoreIndex::with_config(&store.config.index);
    crate::store::cold_start::rebuild::rebuild_from_segment_list(
        &fresh_index,
        &store.reader,
        &view,
    )?;
    if let Some(ranges) = crate::store::hidden_ranges::load_cancelled_ranges(
        &store.config.data_dir,
        store.config.fs().as_ref(),
    )? {
        fresh_index.restore_cancelled_visibility_ranges(ranges);
    }
    Ok(fresh_index)
}

/// Roll the PRE-COMMIT disk state back to the verified source set. The final name
/// was never created pre-commit, so only the staged replacement is removed (never
/// the final path — pre-commit it is either absent or still the ORIGINAL merged-id
/// source); if the merged-id source was relocated to `.compact-src`, restore it.
fn rollback_compaction_disk_state(
    data_dir: &Path,
    staged_path: &Path,
    final_path: &Path,
    compact_source_path: Option<&Path>,
    fs: &dyn StoreFs,
) -> Result<(), StoreError> {
    fs.remove_file_if_present(staged_path)
        .map_err(StoreError::Io)?;
    if let Some(temp_source_path) = compact_source_path {
        fs.rename(temp_source_path, final_path)
            .map_err(StoreError::Io)?;
        fs.sync_parent_dir(final_path)?;
    }
    crate::store::cold_start::rebuild::clear_pending_compaction(data_dir, fs)?;
    Ok(())
}

struct FailedCompactionCtx<'a> {
    config: &'a CompactionConfig,
    active_segment_id: u64,
    sealed: &'a [(u64, PathBuf)],
    merged_segment_id: u64,
    data_dir: &'a Path,
    staged_path: &'a Path,
    final_path: &'a Path,
    compact_source_path: Option<&'a Path>,
    error: &'a StoreError,
    context: &'a str,
    fs: &'a dyn StoreFs,
}

fn failed_compaction_with_rollback(
    ctx: &FailedCompactionCtx<'_>,
) -> Result<
    (
        segment::CompactionResult,
        crate::store::compaction_report::CompactionReportBody,
    ),
    StoreError,
> {
    rollback_compaction_disk_state(
        ctx.data_dir,
        ctx.staged_path,
        ctx.final_path,
        ctx.compact_source_path,
        ctx.fs,
    )?;
    let reason = format!("{}; disk layout rolled back: {}", ctx.context, ctx.error);
    tracing::error!(target: "batpak::flow", flow = "compact", error = %ctx.error, "{reason}");
    let result = segment::CompactionResult {
        outcome: segment::CompactionOutcome::Failed {
            reason: reason.clone(),
        },
        segments_removed: 0,
        bytes_reclaimed: 0,
    };
    let report = crate::store::compaction_report::report_for_run(
        ctx.config,
        ctx.active_segment_id,
        ctx.sealed,
        Some(ctx.merged_segment_id),
        &result,
        None,
        ctx.fs,
    )?;
    Ok((result, report))
}

fn scan_sealed_entries(
    store: &Store<Open>,
    sealed: &[(u64, PathBuf)],
) -> Result<Vec<reader::ScannedEntry>, StoreError> {
    let mut all_events = Vec::new();
    for (_, path) in sealed {
        all_events.extend(store.reader.scan_segment(path)?);
    }
    Ok(all_events)
}

fn scanned_entry_as_stored_event(
    entry: &reader::ScannedEntry,
) -> Result<StoredEvent<serde_json::Value>, StoreError> {
    Ok(StoredEvent {
        coordinate: Coordinate::new(&entry.entity, &entry.scope)?,
        event: entry.event.clone(),
    })
}

/// Evaluate a Retention/Tombstone predicate against an event's payload, giving
/// the predicate the DECRYPTED plaintext view of an encrypted event (Stage E1).
///
/// Returns whether the predicate KEEPS the event (Retention: survives the drop;
/// Tombstone: `true` ⇒ NOT rewritten to a tombstone). A crypto-shredded event
/// cannot be predicate-evaluated — its plaintext is permanently destroyed — so
/// the conservative default is to KEEP it: you cannot decide to DROP what you
/// cannot read, and keeping never silently loses data the operator did not ask
/// to erase. Either way the write side re-emits the original CIPHERTEXT bytes
/// verbatim, so a survivor's `event_hash` stays byte-stable.
fn compaction_predicate_keeps(
    store: &Store<Open>,
    entry: &reader::ScannedEntry,
    predicate: &crate::store::RetentionPredicate,
) -> Result<bool, StoreError> {
    #[cfg(feature = "payload-encryption")]
    if entry.event.header.payload_encryption.is_some() {
        return Ok(match decrypt_compaction_payload(store, entry)? {
            Some(stored) => predicate(&stored),
            None => true, // Shredded → conservative KEEP.
        });
    }
    #[cfg(not(feature = "payload-encryption"))]
    let _ = store;
    Ok(predicate(&scanned_entry_as_stored_event(entry)?))
}

/// Decrypt an encrypted scanned entry's ciphertext into the predicate's
/// `StoredEvent<Value>` view via the shared Stage C primitive, or `None` when the
/// scope key has been destroyed (a crypto-shred — the plaintext is gone).
#[cfg(feature = "payload-encryption")]
fn decrypt_compaction_payload(
    store: &Store<Open>,
    entry: &reader::ScannedEntry,
) -> Result<Option<StoredEvent<serde_json::Value>>, StoreError> {
    let coordinate = Coordinate::new(&entry.entity, &entry.scope)?;
    let header = &entry.event.header;
    let Some(meta) = header.payload_encryption.as_ref() else {
        // Caller routes only encrypted entries here; a plaintext entry decodes
        // straight from its already-decoded scanned value.
        return Ok(Some(scanned_entry_as_stored_event(entry)?));
    };
    match store.open_encrypted_payload_bytes(
        &coordinate,
        header.event_kind,
        header.event_id,
        meta,
        &entry.payload_bytes,
    )? {
        crate::store::read_api::PayloadPlaintext::Shredded => Ok(None),
        crate::store::read_api::PayloadPlaintext::Plaintext(plaintext) => {
            let value = crate::encoding::from_bytes::<serde_json::Value>(&plaintext)
                .map_err(|e| StoreError::Serialization(Box::new(e)))?;
            Ok(Some(StoredEvent {
                coordinate,
                event: Event {
                    header: entry.event.header.clone(),
                    payload: value,
                    hash_chain: entry.event.hash_chain.clone(),
                },
            }))
        }
    }
}

fn write_scanned_entry(
    merged_segment: &mut segment::Segment<Active>,
    entry: reader::ScannedEntry,
) -> Result<(), StoreError> {
    // Re-emit the survivor's ORIGINAL payload BYTES, never the decoded Value.
    // `entry.event.payload` is the `serde_json::Value` view kept only for the
    // keep/drop predicate; serializing THAT writes a msgpack MAP where the
    // reader's `FramePayload<Vec<u8>>` decode expects raw bytes, making every
    // survivor unreadable ("invalid type: map, expected a sequence"). Rebuilding
    // the frame from `entry.payload_bytes` re-encodes only the outer frame
    // envelope — the user payload is carried verbatim — so a kept frame is
    // byte-identical to the original and its `event_hash` (blake3 over
    // `event.payload`) is byte-stable across compaction. The Tombstone path's
    // in-place `event_kind` mutation rides through `entry.event.header` here.
    let event = Event {
        header: entry.event.header,
        payload: entry.payload_bytes,
        hash_chain: entry.event.hash_chain,
    };
    let frame_payload = FramePayload {
        event,
        entity: entry.entity,
        scope: entry.scope,
        receipt_extensions: entry.receipt_extensions,
    };
    let frame = segment::frame_encode(&frame_payload)?;
    merged_segment.write_frame(&frame)?;
    Ok(())
}

/// Relocate the merged-id source away to `.compact-src` (+ parent sync) so the
/// FINAL name is absent from this point until the commit rename. Only sets
/// `compact_source_path` on success, so rollback knows whether to restore.
fn relocate_merged_source_if_present(
    store: &Store<Open>,
    sealed: &mut [(u64, PathBuf)],
    merged_id: u64,
    src_path: &Path,
    compact_source_path: &mut Option<PathBuf>,
) -> Result<(), StoreError> {
    if let Some((_, source_path)) = sealed.iter_mut().find(|(seg_id, _)| *seg_id == merged_id) {
        let fs = store.config.fs();
        fs.remove_file_if_present(src_path).map_err(StoreError::Io)?;
        fs.rename(&*source_path, src_path).map_err(StoreError::Io)?;
        fs.sync_parent_dir(src_path)?;
        *source_path = src_path.to_path_buf();
        *compact_source_path = Some(src_path.to_path_buf());
    }
    Ok(())
}

fn materialize_compacted_segment(
    store: &Store<Open>,
    strategy: &CompactionStrategy,
    sealed: &mut [(u64, PathBuf)],
    merged_id: u64,
    staged_path: &Path,
    src_path: &Path,
    compact_source_path: &mut Option<PathBuf>,
) -> Result<(), StoreError> {
    // Evict reader FDs so the merged-id source can be renamed (Windows: rename
    // with an open handle fails).
    for (seg_id, _) in sealed.iter() {
        store.reader.evict_segment(*seg_id);
    }

    relocate_merged_source_if_present(store, sealed, merged_id, src_path, compact_source_path)?;

    // Stale-staged hygiene (recovery also removes these — defense in depth). The
    // FINAL name is NEVER touched pre-commit.
    store
        .config
        .fs()
        .remove_file_if_present(staged_path)
        .map_err(StoreError::Io)?;
    let mut merged_segment = segment::Segment::<Active>::create_at_path_with_created_ns_on(
        staged_path,
        merged_id,
        store.runtime.now_wall_ns(),
        store.config.fs(),
    )?;
    // Crypto-shred coupling — DELIBERATELY NONE (the safe semantics). Retention
    // (drop) and Tombstone (mark) compaction operate per EVENT, but a crypto-shred
    // key is per SCOPE, and under a coarse granularity (the default PerEntity) one
    // key covers every event of an entity. A predicate that drops/tombstones SOME
    // of a scope's events must NOT shred the WHOLE scope's key — that would
    // silently destroy the still-live siblings' plaintext (over-shred). Rather than
    // track "was this the scope's LAST event" here (fragile, and still an implicit
    // shred the operator never asked for), compaction destroys NO payload keys:
    // erasure stays the single explicit `Store::shred_scope` op, which is
    // granularity-agnostic and never over-shreds. Compaction therefore leaves the
    // keyset untouched; a surviving event of a partially-compacted scope still
    // decrypts under its (undestroyed) key.
    match strategy {
        CompactionStrategy::Merge => {
            for (_, path) in sealed.iter() {
                merged_segment.append_frames_from_segment(path)?;
            }
        }
        CompactionStrategy::Retention(predicate) => {
            for entry in scan_sealed_entries(store, sealed)? {
                if compaction_predicate_keeps(store, &entry, predicate)? {
                    write_scanned_entry(&mut merged_segment, entry)?;
                }
            }
        }
        CompactionStrategy::Tombstone(predicate) => {
            let tombstone_kind = EventKind::TOMBSTONE;
            for mut entry in scan_sealed_entries(store, sealed)? {
                if !compaction_predicate_keeps(store, &entry, predicate)? {
                    entry.event.header.event_kind = tombstone_kind;
                }
                write_scanned_entry(&mut merged_segment, entry)?;
            }
        }
    }

    merged_segment.sync_with_mode(&store.config.sync.mode)?;
    let _sealed_segment = merged_segment.seal();
    Ok(())
}

#[cfg(test)]
#[path = "lifecycle_compact_mutation_kill.rs"]
mod lifecycle_compact_mutation_kill;

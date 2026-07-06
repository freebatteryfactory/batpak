//! StoreFs-level import crash recovery: crash mid-import, reopen, re-import must
//! deduplicate via durable import keys and preserve payload bytes + hash chains.

use super::fs::SimFs;
use super::recovery::{fold, FNV_OFFSET};
use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::id::EntityIdType;
use crate::store::{ImportOptions, ImportSelector, ReadOnly, Store, StoreConfig};
use std::sync::Arc;

/// Outcome of one seeded import-under-fault scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportFaultOutcome {
    pub(crate) digest: u64,
    pub(crate) source_user_events: usize,
    pub(crate) dest_user_events: usize,
    pub(crate) reimport_deduplicated: u64,
}

fn outcome_digest(
    seed: u64,
    source_user_events: usize,
    dest_user_events: usize,
    reimport_deduplicated: u64,
) -> u64 {
    let mut d = fold(FNV_OFFSET, seed);
    d = fold(d, source_user_events as u64);
    d = fold(d, dest_user_events as u64);
    fold(d, reimport_deduplicated)
}

/// Drive import on a real `Store` over `SimFs`, crash without shutdown, reopen,
/// re-import, and verify deduplication plus byte-isomorphic payloads.
pub(crate) fn run_seeded_import_fault(seed: u64) -> Result<ImportFaultOutcome, String> {
    let inner: Arc<dyn crate::store::platform::fs::StoreFs> =
        Arc::new(crate::store::platform::fs::RealFs);
    run_seeded_import_fault_over(seed, &inner)
}

/// [`run_seeded_import_fault`] with the SimFs fault layer (over the clean
/// source and reopen stores too) composed over an arbitrary `inner` backend,
/// so the SAME seeded scenario drives a store over real files or over a pure
/// in-memory backend ([`MemFs`](crate::store::MemFs)).
pub(crate) fn run_seeded_import_fault_over(
    seed: u64,
    inner: &Arc<dyn crate::store::platform::fs::StoreFs>,
) -> Result<ImportFaultOutcome, String> {
    let root = tempfile::tempdir().map_err(|e| format!("seed=0x{seed:X}: tmpdir: {e}"))?;
    let source_path = root.path().join("source");
    let dest_path = root.path().join("dest");

    let event_count = 4 + (seed % 5) as usize;
    let entity = "entity:import-fault";
    let kind = EventKind::custom(0xF, 0x90);

    {
        let source = Store::open(
            StoreConfig::new(&source_path)
                .with_fs(Arc::clone(inner))
                .with_sync_every_n_events(1)
                .with_enable_checkpoint(false)
                .with_enable_mmap_index(false),
        )
        .map_err(|e| format!("seed=0x{seed:X}: open source: {e}"))?;
        let coord = Coordinate::new(entity, "scope:import")
            .map_err(|e| format!("seed=0x{seed:X}: coord: {e}"))?;
        for i in 0..event_count {
            drop(
                source
                    .append(&coord, kind, &serde_json::json!({ "n": i }))
                    .map_err(|e| format!("seed=0x{seed:X}: source append: {e}"))?,
            );
        }
        source
            .close()
            .map_err(|e| format!("seed=0x{seed:X}: close source: {e}"))?;
    }

    let source = Store::<ReadOnly>::open_read_only(
        StoreConfig::new(&source_path).with_fs(Arc::clone(inner)),
    )
    .map_err(|e| format!("seed=0x{seed:X}: reopen source: {e}"))?;

    let options = ImportOptions::new("source-fault")
        .map_err(|e| format!("seed=0x{seed:X}: options: {e}"))?
        .with_chunk_size(1);

    let fsync_drop = if seed.is_multiple_of(5) { 4 } else { 0 };
    let sim_fs = Arc::new(SimFs::layered(
        seed ^ 0x1B00_0001,
        fsync_drop,
        Arc::clone(inner),
    ));
    {
        let config = StoreConfig::new(&dest_path)
            .with_fs(Arc::clone(&sim_fs) as Arc<dyn crate::store::platform::fs::StoreFs>)
            .with_sync_every_n_events(1)
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false);
        let dest = Store::open(config).map_err(|e| format!("seed=0x{seed:X}: open dest: {e}"))?;
        dest.import_events(&source, &ImportSelector::all(), &options)
            .map_err(|e| format!("seed=0x{seed:X}: first import: {e}"))?;
        dest.abandon_without_shutdown();
        sim_fs.crash();
    }

    let dest = Store::open(
        StoreConfig::new(&dest_path)
            .with_fs(Arc::clone(inner))
            .with_sync_every_n_events(1)
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )
    .map_err(|e| format!("seed=0x{seed:X}: reopen dest: {e}"))?;

    let replay = dest
        .import_events(&source, &ImportSelector::all(), &options)
        .map_err(|e| format!("seed=0x{seed:X}: reimport: {e}"))?;

    let (source_user_events, dest_user_events) =
        verify_reimport_isomorphism(seed, &source, &dest, entity)?;

    Ok(ImportFaultOutcome {
        digest: outcome_digest(
            seed,
            source_user_events,
            dest_user_events,
            replay.deduplicated,
        ),
        source_user_events,
        dest_user_events,
        reimport_deduplicated: replay.deduplicated,
    })
}

/// Post-recovery oracle for [`run_seeded_import_fault`]: the destination must
/// hold exactly the source's user events with intact per-entity hash chains and
/// byte-isomorphic payloads/content hashes. Returns `(source_len, dest_len)`.
fn verify_reimport_isomorphism(
    seed: u64,
    source: &Store<ReadOnly>,
    dest: &Store,
    entity: &str,
) -> Result<(usize, usize), String> {
    let source_entries = source.by_entity(entity);
    let dest_entries = dest.by_entity(entity);
    if dest_entries.len() != source_entries.len() {
        return Err(format!(
            "seed=0x{seed:X}: dest user event count {} != source {}",
            dest_entries.len(),
            source_entries.len()
        ));
    }

    for window in dest_entries.windows(2) {
        if window[1].hash_chain().prev_hash != window[0].hash_chain().event_hash {
            return Err(format!(
                "seed=0x{seed:X}: broken hash chain at global_sequence {}",
                window[1].global_sequence()
            ));
        }
    }

    for (dest_entry, source_entry) in dest_entries.iter().zip(source_entries.iter()) {
        let dest_raw = dest
            .read_raw(dest_entry.event_id())
            .map_err(|e| format!("seed=0x{seed:X}: read dest raw: {e}"))?;
        let source_raw = source
            .read_raw(source_entry.event_id())
            .map_err(|e| format!("seed=0x{seed:X}: read source raw: {e}"))?;
        if dest_raw.event.payload != source_raw.event.payload {
            return Err(format!(
                "seed=0x{seed:X}: payload bytes diverged for source event {:032x}",
                source_entry.event_id().as_u128()
            ));
        }
        if dest_raw.event.header.content_hash != source_raw.event.header.content_hash {
            return Err(format!(
                "seed=0x{seed:X}: content hash diverged for source event {:032x}",
                source_entry.event_id().as_u128()
            ));
        }
    }

    Ok((source_entries.len(), dest_entries.len()))
}

/// Doc-hidden public mirror for integration tests (hidden via the
/// `#[doc(hidden)] pub mod __sim` re-export, mirroring `ForkFaultOutcomePublic`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFaultOutcomePublic {
    /// Determinism digest for this seed + outcome.
    pub digest: u64,
    /// User events in the source store.
    pub source_user_events: usize,
    /// User events visible in the destination after recovery + re-import.
    pub dest_user_events: usize,
    /// Events counted as deduplicated on the post-crash re-import pass.
    pub reimport_deduplicated: u64,
}

/// Run one seeded import-under-fault scenario (StoreFs-level).
///
/// # Errors
/// Returns a seed-tagged description string when the scenario cannot run or the
/// post-crash re-import fails to preserve payload bytes, hash chains, or dedup.
pub fn run_seeded_import_fault_public(seed: u64) -> Result<ImportFaultOutcomePublic, String> {
    run_seeded_import_fault(seed).map(|o| ImportFaultOutcomePublic {
        digest: o.digest,
        source_user_events: o.source_user_events,
        dest_user_events: o.dest_user_events,
        reimport_deduplicated: o.reimport_deduplicated,
    })
}

/// [`run_seeded_import_fault_public`], but with the SimFs fault layer composed
/// over a pure in-memory [`MemFs`](crate::store::MemFs) backend — the SAME
/// seeded import-crash-reimport scenario proven backend-agnostic, no host files.
///
/// # Errors
/// As [`run_seeded_import_fault_public`].
pub fn run_seeded_import_fault_mem_fs_public(
    seed: u64,
) -> Result<ImportFaultOutcomePublic, String> {
    let inner: Arc<dyn crate::store::platform::fs::StoreFs> = Arc::new(crate::store::MemFs::new());
    run_seeded_import_fault_over(seed, &inner).map(|o| ImportFaultOutcomePublic {
        digest: o.digest,
        source_user_events: o.source_user_events,
        dest_user_events: o.dest_user_events,
        reimport_deduplicated: o.reimport_deduplicated,
    })
}

/// Replay seed helper honoring `BATPAK_SEED`.
pub fn import_fault_replay_seed(default: u64) -> u64 {
    super::seed_from_env(default)
}

/// Outcome of one seeded same-store import ceiling scenario (no runaway re-import).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportCeilingOutcome {
    pub(crate) digest: u64,
}

fn ceiling_outcome_digest(
    seed: u64,
    imported: u64,
    replay_deduplicated: u64,
    final_event_count: usize,
) -> u64 {
    let mut d = fold(FNV_OFFSET, seed);
    d = fold(d, imported);
    d = fold(d, replay_deduplicated);
    fold(d, final_event_count as u64)
}

/// Same-store import must terminate at the call-time frontier even under segment
/// rotation — it must never re-import its own freshly-appended output.
pub(crate) fn run_seeded_import_same_store_ceiling(
    seed: u64,
) -> Result<ImportCeilingOutcome, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("seed=0x{seed:X}: tmpdir: {e}"))?;
    let count = 8 + (seed % 17) as usize;
    let chunk = 1 + (seed % 4) as usize;
    let blob_len = 200 + (seed % 80) as usize;
    let entity = "entity:import:rotate";
    let kind = EventKind::custom(0xF, 0x77);
    let blob = "x".repeat(blob_len);

    let store = Store::open(
        StoreConfig::new(dir.path())
            .with_segment_max_bytes(512)
            .with_sync_every_n_events(1)
            .with_enable_checkpoint(false)
            .with_enable_mmap_index(false),
    )
    .map_err(|e| format!("seed=0x{seed:X}: open store: {e}"))?;
    let coord = Coordinate::new(entity, "scope:import")
        .map_err(|e| format!("seed=0x{seed:X}: coord: {e}"))?;
    for i in 0..count {
        drop(
            store
                .append(&coord, kind, &serde_json::json!({ "i": i, "blob": blob }))
                .map_err(|e| format!("seed=0x{seed:X}: append: {e}"))?,
        );
    }
    let before = store.stats().event_count;

    let options = ImportOptions::new("self-rotate")
        .map_err(|e| format!("seed=0x{seed:X}: options: {e}"))?
        .with_chunk_size(chunk);
    let report = store
        .import_events(&store, &ImportSelector::all(), &options)
        .map_err(|e| format!("seed=0x{seed:X}: first import: {e}"))?;
    if report.imported != count as u64 {
        return Err(format!(
            "seed=0x{seed:X}: imported {} != expected {count}",
            report.imported
        ));
    }
    let after_first = store.stats().event_count;
    if after_first != before + count {
        return Err(format!(
            "seed=0x{seed:X}: event count after first import {after_first} != {before} + {count}"
        ));
    }

    let replay = store
        .import_events(&store, &ImportSelector::all(), &options)
        .map_err(|e| format!("seed=0x{seed:X}: replay import: {e}"))?;
    if replay.imported != 0 {
        return Err(format!(
            "seed=0x{seed:X}: replay imported {} events (expected 0)",
            replay.imported
        ));
    }
    if replay.deduplicated != count as u64 {
        return Err(format!(
            "seed=0x{seed:X}: replay deduplicated {} != {count}",
            replay.deduplicated
        ));
    }
    let final_count = store.stats().event_count;
    if final_count != after_first {
        return Err(format!(
            "seed=0x{seed:X}: final event count {final_count} != {after_first}"
        ));
    }

    Ok(ImportCeilingOutcome {
        digest: ceiling_outcome_digest(seed, report.imported, replay.deduplicated, final_count),
    })
}

#[cfg(test)]
mod tests {
    use super::run_seeded_import_fault;

    /// B15 mutation kill: the SimFs stream derivation `seed ^ 0x1B00_0001` is
    /// load-bearing. Seed 1075 is a multiple of 5 (fsync drops armed at 1-in-4),
    /// and its XOR-derived drop schedule loses the ENTIRE first-import durable
    /// prefix at the crash. The `^` -> `|` mutant derives the OR stream
    /// (`1075 | 0x1B00_0001`), whose schedule keeps every event durable
    /// (dedup 4, digest 7471031782833777414), so these exact pins fail.
    #[test]
    fn import_fault_xor_stream_derivation_is_load_bearing() {
        let outcome =
            run_seeded_import_fault(1075).expect("seed 1075 import-fault scenario must run");
        assert_eq!(
            outcome.source_user_events, 4,
            "PROPERTY: seed 1075 sources 4 + (1075 % 5) = 4 user events"
        );
        assert_eq!(
            outcome.dest_user_events, 4,
            "PROPERTY: the post-crash re-import must restore every source event"
        );
        assert_eq!(
            outcome.reimport_deduplicated, 0,
            "PROPERTY: seed 1075's XOR-derived drop schedule loses the whole first-import \
             durable prefix, so the re-import re-imports everything and deduplicates nothing; \
             the OR-derived stream keeps all four events durable (dedup 4) and must fail here"
        );
        assert_eq!(
            outcome.digest, 16_988_514_228_672_971_394,
            "PROPERTY: pinned outcome digest for seed 1075 under the XOR stream derivation \
             (the `^` -> `|` mutant digest is 7471031782833777414)"
        );
    }

    /// The seed-derived arming `if seed.is_multiple_of(5) { 4 } else { 0 }`
    /// is observable in BOTH directions: seed 195 (multiple of 5) arms drops and
    /// loses part of the durable prefix; seed 19 (not a multiple) keeps every
    /// fsync honored and the full prefix survives. Flipping the branch or the
    /// armed rate changes the pinned dedup counts and digests for these seeds
    /// (forcing drops on seed 19 yields dedup 7 / digest 9929498441239452817;
    /// disarming seed 195 yields dedup 4).
    #[test]
    fn fsync_drop_arming_follows_seed_multiple_of_five() {
        let armed = run_seeded_import_fault(195).expect("seed 195 import-fault scenario must run");
        assert_eq!(
            armed.source_user_events, 4,
            "PROPERTY: seed 195 sources 4 + (195 % 5) = 4 user events"
        );
        assert_eq!(
            armed.reimport_deduplicated, 3,
            "PROPERTY: with drops armed (1-in-4) seed 195's durable prefix is a PARTIAL 3 of 4 \
             events — a drops-off mutant keeps all four durable (dedup 4) and must fail here"
        );
        assert_eq!(
            armed.digest, 10_862_642_175_300_011_077,
            "PROPERTY: pinned digest for seed 195 — the committed \
             `traceability/dst_corpus.yaml` ImportReapply row identity"
        );

        let off = run_seeded_import_fault(19).expect("seed 19 import-fault scenario must run");
        assert_eq!(
            off.source_user_events, 8,
            "PROPERTY: seed 19 sources 4 + (19 % 5) = 8 user events"
        );
        assert_eq!(
            off.reimport_deduplicated, 8,
            "PROPERTY: with drops off every event is durable at the crash, so the re-import \
             deduplicates the whole source — a drops-forced mutant loses one event (dedup 7) \
             and must fail here"
        );
        assert_eq!(
            off.digest, 2_056_110_584_399_011_902,
            "PROPERTY: pinned outcome digest for seed 19 with fsync drops disarmed"
        );
    }
}

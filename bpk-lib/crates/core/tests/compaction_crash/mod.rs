//! Shared crash-workload harness for the compaction namespace-commit oracle
//! (#177/#185). Included per test binary via `#[path = "compaction_crash/mod.rs"]
//! mod harness;` (the `chaos/mod.rs` shared-module precedent) — it is NOT a test
//! binary and declares NO `#[test]` fns; each consumer owns its own `#[test]`s.
//!
//! The oracle is an event CENSUS state machine, not byte equality: the staged
//! namespace choreography relocates source bytes, so a filename/byte oracle would
//! false-fail. `classify_generation` admits a recovered census as EXACTLY the OLD
//! or the NEW complete generation and nothing in between — an invented, torn, or
//! undead id is a hard, id-naming failure.
//!
//! Legacy plan-05 vocabulary maps onto the reconciled surface (contract R17):
//! `NamespaceSimFs` → `ShadowFs`; `arm_crash_at(boundary)` → `arm_boundary`;
//! `crash_fired()` → `ShadowFs::armed_faults_fired() > 0`; poison-on-fault →
//! `ShadowFs::set_freeze_on_fault(true)`.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::IdempotencyKey;
use batpak::store::{
    AppendOptions, AppendReceipt, CompactionConfig, CompactionStrategy, Store, StoreConfig,
    StoreError,
};
use std::collections::BTreeSet;
use std::path::Path;

/// Keyed, retention-evicted lane. Chosen to not collide with sibling harnesses
/// (custom(0xB, 4) / custom(0xB, 5) are taken elsewhere).
pub const EVICT_KIND: EventKind = EventKind::custom(0xB, 6);
/// Keyed, retention-surviving lane.
pub const SURVIVOR_KIND: EventKind = EventKind::custom(0xB, 7);
/// The durable idempotency-authority image sidecar.
pub const IDEMP_FILENAME: &str = "index.idemp";
/// The canonical binary pending-compaction marker (contract A9: the marker is a
/// versioned binary artifact named `compaction.pending`; the legacy
/// `compaction.pending.json` is diagnostic-only and never recovers).
pub const MARKER_FILENAME: &str = "compaction.pending";

/// The declared workload manifest (issue #185: the input shape must not live in
/// tribal memory — it is a named, versioned artifact the fixtures seed against).
pub struct CrashWorkloadManifest {
    /// Manifest schema version; the seeding logic is written against v1.
    pub workload_version: u32,
    /// Keyed events in the retention-evicted lane.
    pub keyed_evict_events: u32,
    /// Keyed events in the retention-surviving lane.
    pub keyed_survivor_events: u32,
    /// Unkeyed `EVICT_KIND` filler events, sized to force segment rotation.
    pub filler_events: u32,
    /// Segment rotation ceiling in bytes.
    pub segment_max_bytes: u64,
    /// Minimum sealed segments before `compact()` runs.
    pub min_segments: usize,
    /// Deterministic key seed; keys are `(seed as u128) << 64 | n`.
    pub seed: u64,
}

/// The one declared workload used by every crash/workload fixture.
pub const MANIFEST_V1: CrashWorkloadManifest = CrashWorkloadManifest {
    workload_version: 1,
    keyed_evict_events: 3,
    keyed_survivor_events: 1,
    filler_events: 24,
    segment_max_bytes: 512,
    min_segments: 1,
    seed: 0xB185,
};

/// The single coordinate every workload event is committed under.
pub fn coord() -> Coordinate {
    Coordinate::new("entity:crash", "scope:maintenance").expect("valid coord")
}

/// Store config for the crash workload: checkpoint + mmap OFF (so recovery walks
/// the segment set, not a fast-path artifact), tiny segments (forces rotation),
/// and one sync per event (deterministic op schedule).
pub fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(MANIFEST_V1.segment_max_bytes)
        .with_sync_every_n_events(1)
}

/// Retention that evicts EVERY `EVICT_KIND` frame (keyed + filler), keeping the
/// survivor lane and system markers. The NEW generation's evict-kind census is
/// therefore empty.
pub fn evict_all_evict_kind() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != EVICT_KIND
        })),
        min_segments: MANIFEST_V1.min_segments,
    }
}

/// Keyed append. Keyed event id == idempotency key (submission.rs law), so census
/// membership by key is exact.
fn append_keyed(store: &Store, kind: EventKind, key: u128) -> AppendReceipt {
    let tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            kind,
            &serde_json::json!({ "k": tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed append")
}

/// Seed per `MANIFEST_V1`: keyed evict events first, then the keyed survivor
/// event, then the fillers (rotating segments past every keyed frame). Returns
/// `(key, original AppendReceipt)` for BOTH keyed lanes in deterministic order;
/// fillers are unkeyed and not returned. Preconditions are asserted by callers.
pub fn seed_workload(store: &Store, m: &CrashWorkloadManifest) -> Vec<(u128, AppendReceipt)> {
    assert_eq!(
        m.workload_version, 1,
        "the harness seeds only workload manifest version 1"
    );
    let base = u128::from(m.seed) << 64;
    let mut n: u128 = 0;
    let mut pairs = Vec::new();
    for _ in 0..m.keyed_evict_events {
        n += 1;
        let key = base | n;
        pairs.push((key, append_keyed(store, EVICT_KIND, key)));
    }
    for _ in 0..m.keyed_survivor_events {
        n += 1;
        let key = base | n;
        pairs.push((key, append_keyed(store, SURVIVOR_KIND, key)));
    }
    for i in 0..m.filler_events {
        let _ = store
            .append(&coord(), EVICT_KIND, &serde_json::json!({ "filler": i }))
            .expect("append filler event");
    }
    pairs
}

/// Census: the set of event ids of `kind` visible via `query(Region::all())`.
pub fn kind_census(store: &Store, kind: EventKind) -> BTreeSet<u128> {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|entry| entry.event_kind() == kind)
        .map(|entry| entry.event_id().as_u128())
        .collect()
}

/// The recovered generation classification (the `RecoveredState` precedent from
/// `gauntlet_s3_recovery_oracle.rs`, specialized to compaction generations).
#[derive(Debug, PartialEq, Eq)]
pub enum RecoveredGeneration {
    /// The recovered census is EXACTLY the pre-compaction (old) generation.
    OldComplete,
    /// The recovered census is EXACTLY the retention-survivor (new) generation.
    NewComplete,
}

/// Exactly-old-or-new classifier. No intermediate state is admitted: an invented
/// id (`recovered − old∪new`), an undead id (an evicted id still present), or a
/// torn id (`old − recovered` when neither set matches) is a named `Err`.
pub fn classify_generation(
    recovered: &BTreeSet<u128>,
    old: &BTreeSet<u128>,
    new: &BTreeSet<u128>,
) -> Result<RecoveredGeneration, String> {
    let union: BTreeSet<u128> = old.union(new).copied().collect();
    let invented: BTreeSet<u128> = recovered.difference(&union).copied().collect();
    if !invented.is_empty() {
        return Err(format!(
            "invented event ids not in old∪new: {invented:?} (recovered={recovered:?})"
        ));
    }
    if recovered == old {
        return Ok(RecoveredGeneration::OldComplete);
    }
    if recovered == new {
        return Ok(RecoveredGeneration::NewComplete);
    }
    let evicted: BTreeSet<u128> = old.difference(new).copied().collect();
    let undead: BTreeSet<u128> = recovered.intersection(&evicted).copied().collect();
    let torn: BTreeSet<u128> = old.difference(recovered).copied().collect();
    Err(format!(
        "intermediate/torn generation (neither complete OLD nor complete NEW): \
         undead(evicted-but-present)={undead:?}, torn(old-but-missing)={torn:?}, \
         recovered={recovered:?}, old={old:?}, new={new:?}"
    ))
}

/// Retry `key` and assert the returned receipt identity (`global_sequence`,
/// `event_id`, `content_hash`) equals `original` — the no-double-append witness.
/// A keyed retry deduplicates by `IdempotencyKey` alone (submission.rs law), so
/// the stored receipt (with the ORIGINAL content hash) comes back regardless of
/// the kind/payload replayed here.
pub fn assert_retry_is_original(store: &Store, key: u128, original: &AppendReceipt) {
    let tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    let retry = store
        .append_with_options(
            &coord(),
            EVICT_KIND,
            &serde_json::json!({ "retry": tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed retry append");
    assert_eq!(
        retry.global_sequence, original.global_sequence,
        "keyed retry must return the ORIGINAL global_sequence (no double-append) for key {key:#034x}"
    );
    assert_eq!(
        retry.event_id.as_u128(),
        original.event_id.as_u128(),
        "keyed retry must return the ORIGINAL event_id for key {key:#034x}"
    );
    assert_eq!(
        retry.content_hash, original.content_hash,
        "keyed retry must return the ORIGINAL content_hash for key {key:#034x}"
    );
}

/// Canonical-refusal guard for reopen `Err` arms. Mirrors the variant set of
/// `src/store/sim/recovery.rs::is_canonical_refusal` exactly (a `matches!` list,
/// not a `match`, so there is no catch-all-arm lint exposure): a recovery that
/// refuses fail-closed with one of these is the contract working, not a crash.
pub fn is_canonical_recovery_refusal(err: &StoreError) -> bool {
    matches!(
        err,
        StoreError::CorruptSegment { .. }
            | StoreError::CorruptFrame { .. }
            | StoreError::CrcMismatch { .. }
            | StoreError::DataDirMalformed { .. }
            | StoreError::MmapFutureVersion { .. }
            | StoreError::IdempotencyFutureVersion { .. }
            | StoreError::SidxFutureVersion { .. }
            | StoreError::IdempotencyAuthorityCorrupt { .. }
            | StoreError::IdempotencyAuthorityMissing { .. }
            | StoreError::IdempotencyAuthorityStale { .. }
            | StoreError::IdempotencyAuthorityForeign { .. }
            | StoreError::StoreMetadataCorrupt { .. }
            | StoreError::StoreMetadataMissing { .. }
            | StoreError::StoreMetadataFutureVersion { .. }
            | StoreError::CompactionRecoveryRefused { .. }
    )
}

// ---------------------------------------------------------------------------
// Boundary arming (drives `ShadowFs`, contract R3). Gated to the same feature
// as the crash-window / workload consumers (`compaction_crash_windows.rs`,
// `workload_crash_during_maintenance.rs`), which is where `ShadowFs`/`CrashOp`
// live and where `arm_boundary` is called.
// ---------------------------------------------------------------------------

#[cfg(feature = "dangerous-test-hooks")]
use batpak::store::{CrashOp, ShadowFs};

/// The nine declared crash boundaries of the compaction namespace transaction
/// (contract R3; PascalCase of plan-04's canonical kebab ids). Windows 1–5
/// recover the OLD complete generation, windows 6–9 the NEW.
#[cfg(feature = "dangerous-test-hooks")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactionCrashBoundary {
    MarkerDurableNoSourceRename,
    SourceRenamesPerformed,
    PartialReplacementMaterialization,
    StagedReplacementCompleteAuthorityUnpublished,
    AuthorityPublishedFinalRenameNotDurable,
    FinalReplacementDurableSourcesNotRetired,
    SourcesPartiallyRetired,
    MarkerNotCleared,
    MarkerCleared,
}

/// Map a named boundary to a `ShadowFs` arming per the compaction protocol op
/// order (plan-01 §A1 / plan-05 §D5): (1) marker durable, (2) source renames →
/// `.compact-src`, (3) materialize staged `.compact-new`, (4) publish
/// `index.idemp` authority, (5) COMMIT = rename staged → final `.fbat` + parent
/// sync, (6) retire sources, (7) clear marker.
///
/// Each boundary aborts the protocol at the op that OPENS the next phase and
/// freezes the fs (`set_freeze_on_fault`) so the in-process rollback cannot heal
/// the namespace; `ShadowFs::crash()` + reopen then reproduces power loss at that
/// boundary. `MarkerCleared` arms NOTHING — the transaction runs to completion
/// and the caller crashes (drops) after a clean commit.
///
/// `arm_op_error_at` matches an op's path — or, for a rename, its SOURCE path —
/// by suffix, disambiguating each phase: a source relocation renames a `.fbat`
/// source; the commit renames a `.compact-new` source; the authority publish
/// persists onto `index.idemp`; retirement removes a `.compact-src`.
#[cfg(feature = "dangerous-test-hooks")]
pub fn arm_boundary(fs: &ShadowFs, boundary: CompactionCrashBoundary) {
    const SEGMENT_SUFFIX: &str = ".fbat"; // an at-rest source segment name
    const SRC_SUFFIX: &str = ".compact-src"; // a relocated compaction source
    const STAGED_SUFFIX: &str = ".compact-new"; // the staged replacement segment

    match boundary {
        CompactionCrashBoundary::MarkerDurableNoSourceRename => {
            fs.arm_op_error_at(CrashOp::Rename, SEGMENT_SUFFIX, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::SourceRenamesPerformed => {
            // Abort the SECOND source relocation: ≥1 source is at `.compact-src`
            // and ≥1 remains at its original name (the partiality plan-05 w2
            // asserts). Recovery still rolls back to the complete OLD set.
            fs.arm_op_error_at(CrashOp::Rename, SEGMENT_SUFFIX, 2);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::PartialReplacementMaterialization => {
            fs.arm_op_error_at(CrashOp::Write, STAGED_SUFFIX, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::StagedReplacementCompleteAuthorityUnpublished => {
            fs.arm_op_error_at(CrashOp::PersistTemp, IDEMP_FILENAME, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::AuthorityPublishedFinalRenameNotDurable => {
            // The commit rename's SOURCE is the staged name; aborting it leaves
            // the published authority durable but the final `.fbat` absent.
            fs.arm_op_error_at(CrashOp::Rename, STAGED_SUFFIX, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::FinalReplacementDurableSourcesNotRetired => {
            fs.arm_op_error_at(CrashOp::RemoveFile, SRC_SUFFIX, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::SourcesPartiallyRetired => {
            fs.arm_op_error_at(CrashOp::RemoveFile, SRC_SUFFIX, 2);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::MarkerNotCleared => {
            fs.arm_op_error_at(CrashOp::RemoveFile, MARKER_FILENAME, 1);
            fs.set_freeze_on_fault(true);
        }
        CompactionCrashBoundary::MarkerCleared => {
            // No fault: the transaction commits and clears the marker; the crash
            // is a post-success drop and recovery is an ordinary NEW open.
        }
    }
}

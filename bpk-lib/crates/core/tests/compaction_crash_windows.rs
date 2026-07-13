// The namespace-truth crash simulator (`ShadowFs`) and its arming levers live
// behind the dangerous-test-hooks lane; without the feature this file is empty.
#![cfg(feature = "dangerous-test-hooks")]
//! Compaction namespace-transaction crash-window walk (#177/#195).
//!
//! PROVES: INV-COMPACTION-NAMESPACE-COMMIT — each of the 9 declared crash
//! boundaries of the compaction namespace transaction reopens WITHOUT a clean
//! close into EXACTLY the old complete generation (boundaries 1–5, before the
//! store.meta commit record is durable) or the new complete generation
//! (boundaries 6–9, after it), with keyed retries returning their original
//! receipts, no invented events, and no undead events. The commit record — not
//! the presence of the final filename — decides recovery direction.
//! CATCHES: roll-forward authorized by bare final-name existence; source
//! retirement before the commit; a marker cleared early; the authority image
//! published after (or its commit written before) the destructive steps.
//! SEEDED: the MANIFEST_V1 keyed workload on `ShadowFs`, crashed per named
//! boundary via the shared `compaction_crash/mod.rs::arm_boundary` lever; the
//! store is dropped (never closed) and the simulator crashed before reopen.

use batpak::store::{ShadowFs, Store};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

#[path = "compaction_crash/mod.rs"]
mod harness;

// Disk truth (verified against topology.rs / file_classification.rs, NOT the
// harness constants, so a stale harness constant cannot silence these probes).
const DATA_DIR: &str = "/shadow/compaction-crash";
const MARKER_FILENAME: &str = "compaction.pending";
const IDEMP_FILENAME: &str = "index.idemp";
const STAGED_SUFFIX: &str = ".compact-new";
const SOURCE_SUFFIX: &str = ".compact-src";
const SEGMENT_SUFFIX: &str = ".fbat";

/// The durable namespace after `crash()` — the exact bytes/names a power event
/// left behind. Every count is protocol-guaranteed; segment-rotation counts are
/// deliberately NOT asserted (they depend on filler rotation, not the boundary).
struct DurableView {
    marker: bool,
    staged_len: Option<u64>,
    src_count: usize,
    plain_fbat: usize,
    idemp_len: Option<u64>,
}

impl DurableView {
    fn capture(sim: &ShadowFs, data_dir: &Path) -> Self {
        let mut view = DurableView {
            marker: false,
            staged_len: None,
            src_count: 0,
            plain_fbat: 0,
            idemp_len: sim.durable_byte_len(&data_dir.join(IDEMP_FILENAME)),
        };
        for name in sim.durable_entries(data_dir) {
            let leaf = name.to_string_lossy();
            if leaf.as_ref() == MARKER_FILENAME {
                view.marker = true;
            } else if leaf.ends_with(STAGED_SUFFIX) {
                view.staged_len = sim.durable_byte_len(&data_dir.join(&name));
            } else if leaf.ends_with(SOURCE_SUFFIX) {
                view.src_count += 1;
            } else if leaf.ends_with(SEGMENT_SUFFIX) {
                view.plain_fbat += 1;
            }
        }
        view
    }

    fn staged_present(&self) -> bool {
        self.staged_len.is_some()
    }

    /// The authority image (`index.idemp`) changed relative to the pre-compact
    /// snapshot: the durable proof that the publication phase was reached.
    fn authority_published(&self, pre_idemp_len: Option<u64>) -> bool {
        self.idemp_len != pre_idemp_len
    }
}

struct WindowRun {
    generation: harness::RecoveredGeneration,
    compact_erred: bool,
    faults_fired: u32,
    pre_idemp_len: Option<u64>,
    post: DurableView,
}

/// Drive one crash boundary end-to-end: seed the workload on a fresh simulator,
/// arm the boundary, run the compaction, drop (never close) the store, crash the
/// simulator, then reopen and classify the recovered generation. Reopen MUST
/// succeed for all 9 declared boundaries — they are recoverable by construction;
/// a typed refusal here is a FAIL (refusal states are unit-tested elsewhere).
/// Retries are asserted original and censuses asserted unchanged after retries
/// (no double append, no invented event, no undead event).
fn run_window(
    label: &str,
    boundary: harness::CompactionCrashBoundary,
) -> Result<WindowRun, Box<dyn std::error::Error>> {
    let data_dir = Path::new(DATA_DIR);
    let sim = ShadowFs::new();

    let store = Store::open(harness::config(data_dir).with_fs(Arc::new(sim.clone())))
        .expect("open store over shadow fs");
    let pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
    let old_evict = harness::kind_census(&store, harness::EVICT_KIND);
    let survivors = harness::kind_census(&store, harness::SURVIVOR_KIND);
    let pre_idemp_len = sim.durable_byte_len(&data_dir.join(IDEMP_FILENAME));

    harness::arm_boundary(&sim, boundary);
    let compact_erred = store.compact(&harness::evict_all_evict_kind()).is_err();
    let faults_fired = sim.armed_faults_fired();

    // drop = crash: the writer shuts down against the poisoned simulator; the
    // simulator then reverts every visible-but-not-durable namespace effect.
    drop(store);
    sim.crash();

    let post = DurableView::capture(&sim, data_dir);

    let reopened = match Store::open(harness::config(data_dir).with_fs(Arc::new(sim.clone()))) {
        Ok(store) => store,
        Err(err) => {
            return Err(std::io::Error::other(format!(
                "PROPERTY (#177 {label}): a declared crash boundary must recover into a complete \
                 generation, but reopen refused: {err:?}"
            ))
            .into())
        }
    };

    let rec_evict = harness::kind_census(&reopened, harness::EVICT_KIND);
    let rec_surv = harness::kind_census(&reopened, harness::SURVIVOR_KIND);
    // The retention config evicts every EVICT_KIND event, so the new generation's
    // evict census is empty; classification is exactly old-or-new on that lane.
    let new_evict: BTreeSet<u128> = BTreeSet::new();
    let generation = harness::classify_generation(&rec_evict, &old_evict, &new_evict)
        .map_err(std::io::Error::other)?;

    assert_eq!(
        rec_surv, survivors,
        "PROPERTY (#177 {label}): the survivor census must be intact after recovery",
    );

    for (key, original) in &pairs {
        harness::assert_retry_is_original(&reopened, *key, original);
    }
    assert_eq!(
        harness::kind_census(&reopened, harness::EVICT_KIND),
        rec_evict,
        "PROPERTY (#177 {label}): keyed retries must re-append NOTHING — no undead evict event",
    );
    assert_eq!(
        harness::kind_census(&reopened, harness::SURVIVOR_KIND),
        survivors,
        "PROPERTY (#177 {label}): keyed retries must not disturb the survivor census",
    );

    drop(reopened);

    Ok(WindowRun {
        generation,
        compact_erred,
        faults_fired,
        pre_idemp_len,
        post,
    })
}

/// The sealed byte length of a fully-materialized staged replacement, taken from
/// an independent run crashed at the staged-complete boundary. Distinguishes a
/// partial (window 3) from a complete (window 4) `.compact-new`.
fn probe_sealed_staged_len() -> Result<u64, Box<dyn std::error::Error>> {
    let run = run_window(
        "probe",
        harness::CompactionCrashBoundary::StagedReplacementCompleteAuthorityUnpublished,
    )?;
    run.post.staged_len.ok_or_else(|| {
        std::io::Error::other(
            "probe: the staged-complete boundary must leave a durable `.compact-new` segment",
        )
        .into()
    })
}

/// Shared anti-vacuity for the eight faulted (pre-clean-completion) windows: the
/// armed crash actually fired and aborted the compaction.
fn assert_faulted(run: &WindowRun, label: &str) {
    assert!(
        run.faults_fired > 0,
        "{label}: the armed crash must actually fire (non-vacuity)",
    );
    assert!(
        run.compact_erred,
        "{label}: a crash inside the transaction must surface as a failed compaction",
    );
}

#[test]
fn crash_window_1_marker_durable_no_source_rename_recovers_old(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 1): a crash after the marker is durable but before any
    // source rename reopens into exactly the OLD complete generation.
    let run = run_window(
        "window 1",
        harness::CompactionCrashBoundary::MarkerDurableNoSourceRename,
    )?;
    assert_faulted(&run, "window 1");
    assert_eq!(run.generation, harness::RecoveredGeneration::OldComplete);
    assert!(run.post.marker, "window 1: the pending marker is durable");
    assert!(!run.post.staged_present(), "window 1: no replacement materialized");
    assert_eq!(run.post.src_count, 0, "window 1: no source renamed to `.compact-src`");
    assert!(
        !run.post.authority_published(run.pre_idemp_len),
        "window 1: the authority image was not published",
    );
    Ok(())
}

#[test]
fn crash_window_2_partial_source_renames_recovers_old(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 2): a crash after the merged-id source is relocated but
    // before materialization reopens into exactly the OLD complete generation.
    let run = run_window(
        "window 2",
        harness::CompactionCrashBoundary::SourceRenamesPerformed,
    )?;
    assert_faulted(&run, "window 2");
    assert_eq!(run.generation, harness::RecoveredGeneration::OldComplete);
    assert!(run.post.marker, "window 2: the pending marker is durable");
    assert!(!run.post.staged_present(), "window 2: no replacement materialized yet");
    assert!(run.post.src_count >= 1, "window 2: a source is relocated to `.compact-src`");
    assert!(
        run.post.plain_fbat >= 1,
        "window 2: a source is still at its original name (rename partiality)",
    );
    assert!(!run.post.authority_published(run.pre_idemp_len), "window 2: no authority image");
    Ok(())
}

#[test]
fn crash_window_3_partial_replacement_materialization_recovers_old(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 3): a crash mid-materialization leaves a partial staged
    // replacement and reopens into exactly the OLD complete generation.
    let probe = probe_sealed_staged_len()?;
    let run = run_window(
        "window 3",
        harness::CompactionCrashBoundary::PartialReplacementMaterialization,
    )?;
    assert_faulted(&run, "window 3");
    assert_eq!(run.generation, harness::RecoveredGeneration::OldComplete);
    assert!(run.post.marker, "window 3: the pending marker is durable");
    let staged = run.post.staged_len.ok_or_else(|| {
        std::io::Error::other("window 3: a partial `.compact-new` must be durable")
    })?;
    assert!(
        staged < probe,
        "window 3: the staged replacement is partial ({staged} < sealed {probe})",
    );
    assert!(!run.post.authority_published(run.pre_idemp_len), "window 3: no authority image");
    Ok(())
}

#[test]
fn crash_window_4_staged_complete_authority_unpublished_recovers_old(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 4): a crash after the staged replacement is sealed but
    // before the authority image is published reopens into the OLD generation.
    let probe = probe_sealed_staged_len()?;
    let run = run_window(
        "window 4",
        harness::CompactionCrashBoundary::StagedReplacementCompleteAuthorityUnpublished,
    )?;
    assert_faulted(&run, "window 4");
    assert_eq!(run.generation, harness::RecoveredGeneration::OldComplete);
    assert!(run.post.marker, "window 4: the pending marker is durable");
    assert_eq!(
        run.post.staged_len,
        Some(probe),
        "window 4: the staged replacement is complete (sealed length)",
    );
    assert!(
        !run.post.authority_published(run.pre_idemp_len),
        "window 4: the authority image is NOT yet published",
    );
    Ok(())
}

#[test]
fn crash_window_5_authority_published_final_rename_not_durable_recovers_old(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 5): a crash after the authority image is durable but
    // before the commit record + final rename reopens into the OLD generation —
    // the published image is a legal superset admitted via the pending token.
    let run = run_window(
        "window 5",
        harness::CompactionCrashBoundary::AuthorityPublishedFinalRenameNotDurable,
    )?;
    assert_faulted(&run, "window 5");
    assert_eq!(run.generation, harness::RecoveredGeneration::OldComplete);
    assert!(run.post.marker, "window 5: the pending marker is durable");
    assert!(run.post.staged_present(), "window 5: the final rename is not durable — staged remains");
    assert!(
        run.post.authority_published(run.pre_idemp_len),
        "window 5: the authority image IS durable (published before the commit record)",
    );
    Ok(())
}

#[test]
fn crash_window_6_final_durable_sources_unretired_recovers_new(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 6): a crash after the final rename is durable but before
    // any source is retired reopens into exactly the NEW complete generation.
    let run = run_window(
        "window 6",
        harness::CompactionCrashBoundary::FinalReplacementDurableSourcesNotRetired,
    )?;
    assert_faulted(&run, "window 6");
    assert_eq!(run.generation, harness::RecoveredGeneration::NewComplete);
    assert!(run.post.marker, "window 6: the marker is still present (cleared last)");
    assert!(!run.post.staged_present(), "window 6: the staged replacement was renamed to final");
    assert!(
        run.post.authority_published(run.pre_idemp_len),
        "window 6: the authority image is durable (committed generation)",
    );
    assert!(
        run.post.src_count >= 1 && run.post.plain_fbat >= 2,
        "window 6: every source is still present (final + originals + relocated source)",
    );
    Ok(())
}

#[test]
fn crash_window_7_sources_partially_retired_recovers_new(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 7): a crash mid-retirement reopens into exactly the NEW
    // complete generation — a partially-retired source set never resurrects.
    let run = run_window(
        "window 7",
        harness::CompactionCrashBoundary::SourcesPartiallyRetired,
    )?;
    assert_faulted(&run, "window 7");
    assert_eq!(run.generation, harness::RecoveredGeneration::NewComplete);
    assert!(run.post.marker, "window 7: the marker is still present (cleared last)");
    assert!(!run.post.staged_present(), "window 7: the staged replacement was renamed to final");
    assert!(
        run.post.authority_published(run.pre_idemp_len),
        "window 7: the authority image is durable (committed generation)",
    );
    Ok(())
}

#[test]
fn crash_window_8_marker_uncleared_recovers_new(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 8): a crash after every source is retired but before the
    // marker is cleared reopens into exactly the NEW complete generation.
    let run = run_window(
        "window 8",
        harness::CompactionCrashBoundary::MarkerNotCleared,
    )?;
    assert_faulted(&run, "window 8");
    assert_eq!(run.generation, harness::RecoveredGeneration::NewComplete);
    assert!(run.post.marker, "window 8: the marker is still present (not yet cleared)");
    assert!(!run.post.staged_present(), "window 8: the staged replacement was renamed to final");
    assert!(
        run.post.authority_published(run.pre_idemp_len),
        "window 8: the authority image is durable (committed generation)",
    );
    assert_eq!(
        run.post.src_count, 0,
        "window 8: the relocated source has been retired",
    );
    Ok(())
}

#[test]
fn crash_window_9_marker_cleared_recovers_new(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#177 window 9): a clean completion — the marker is durably cleared —
    // reopens into exactly the NEW complete generation.
    let run = run_window(
        "window 9",
        harness::CompactionCrashBoundary::MarkerCleared,
    )?;
    assert!(
        !run.compact_erred,
        "window 9: the transaction completes cleanly (no fault to abort it)",
    );
    assert_eq!(run.generation, harness::RecoveredGeneration::NewComplete);
    assert!(!run.post.marker, "window 9: the marker is durably cleared");
    assert!(!run.post.staged_present(), "window 9: the staged replacement was renamed to final");
    assert!(
        run.post.authority_published(run.pre_idemp_len),
        "window 9: the authority image is durable (committed generation)",
    );
    assert_eq!(run.post.src_count, 0, "window 9: every source has been retired");
    Ok(())
}

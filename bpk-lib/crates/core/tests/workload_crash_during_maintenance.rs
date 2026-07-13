// The namespace-truth crash simulator (`ShadowFs`) and its op-boundary arming
// lever live behind the dangerous-test-hooks lane; without the feature this
// file is empty.
#![cfg(feature = "dangerous-test-hooks")]
//! Workload crash-during-maintenance exhaustive boundary walk (#185/#177).
//!
//! PROVES: INV-COMPACTION-NAMESPACE-COMMIT — the declared MANIFEST_V1 workload
//! drives a compaction whose recorded namespace schedule is enumerated once
//! (test 1), then a crash is armed on EVERY publication op of that schedule in
//! turn (test 2). Each crash reopens WITHOUT a clean close into exactly the OLD
//! complete generation, the NEW complete generation, or a typed canonical
//! recovery refusal — never a mixed, double-appended, invented, or undead
//! state. Reproducible from (MANIFEST_V1, seed, op index); no clean-close heals.
//! CATCHES: any single namespace op whose loss yields a mixed generation, a
//! double append on keyed retry, an invented event, or an undead evicted event;
//! and a boundary walk that never crosses the commit point (both OLD and NEW
//! outcomes must be witnessed across the walk).
//! SEEDED: the MANIFEST_V1 keyed workload on `ShadowFs`, one armed run per
//! mutation-op index via `ShadowFs::arm_crash_at_op`, dropped (never closed)
//! and crashed before each reopen.

use batpak::store::segment::CompactionOutcome;
use batpak::store::{ShadowFs, Store};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

#[path = "compaction_crash/mod.rs"]
mod harness;

// Disk truth (verified against topology.rs / file_classification.rs, NOT the
// harness constants, so a stale harness constant cannot silence these probes).
const DATA_DIR: &str = "/shadow/workload-maintenance";
const MARKER_FILENAME: &str = "compaction.pending";
const IDEMP_FILENAME: &str = "index.idemp";
const STAGED_SUFFIX: &str = ".compact-new";
const SEGMENT_SUFFIX: &str = ".fbat";

#[test]
fn workload_manifest_probe_run_declares_the_publication_schedule(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#185): the declared workload manifest drives a compaction whose
    // recorded namespace schedule contains every publication-op shape the
    // protocol declares, in protocol order — so the sibling boundary walk cannot
    // be walking an empty or unrelated schedule (a contradiction with plan-01's
    // reconciled protocol surfaces here first, at the declared-schedule altitude).
    let sim = ShadowFs::new();
    let store = Store::open(harness::config(Path::new(DATA_DIR)).with_fs(Arc::new(sim.clone())))
        .expect("open store over shadow fs");
    let _pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
    let base = sim.mutation_ops().len();

    let (result, _report) = store
        .compact(&harness::evict_all_evict_kind())
        .expect("an unarmed workload compaction must succeed");
    let CompactionOutcome::Performed = result.outcome else {
        return Err(std::io::Error::other(format!(
            "the workload compaction must PERFORM to declare a schedule, not {:?}",
            result.outcome
        ))
        .into());
    };

    let all = sim.mutation_ops();
    let schedule = &all[base..];
    if schedule.is_empty() {
        return Err(std::io::Error::other(
            "PROPERTY (#185): a performed compaction must record a non-empty namespace schedule",
        )
        .into());
    }

    // The publication ops the protocol declares, in order. Each is located by op
    // kind (via the stable `SimOpKind` debug name) plus a path suffix against the
    // disk-truth constants above.
    let stages = [
        "marker publication (compaction.pending)",
        "staged replacement create (.compact-new)",
        "authority image publish (index.idemp)",
        "final staged->segment rename (.compact-new -> .fbat)",
        "source retirement remove",
        "marker clear remove (compaction.pending)",
    ];
    let mut cursor = 0usize;
    for op in schedule {
        let kind = format!("{:?}", op.kind);
        let path = op.path.to_string_lossy();
        let to = op.to.as_ref().map(|t| t.to_string_lossy().into_owned());
        let matched = match cursor {
            0 => {
                ((kind == "PersistTemp" || kind == "CreateNew")
                    && path.ends_with(MARKER_FILENAME))
                    || (kind == "Rename"
                        && to.as_deref().is_some_and(|t| t.ends_with(MARKER_FILENAME)))
            }
            1 => kind == "CreateNew" && path.ends_with(STAGED_SUFFIX),
            2 => kind == "PersistTemp" && path.ends_with(IDEMP_FILENAME),
            3 => {
                kind == "Rename"
                    && path.ends_with(STAGED_SUFFIX)
                    && to.as_deref().is_some_and(|t| t.ends_with(SEGMENT_SUFFIX))
            }
            4 => kind == "RemoveFile" && !path.ends_with(MARKER_FILENAME),
            5 => kind == "RemoveFile" && path.ends_with(MARKER_FILENAME),
            _ => false,
        };
        if matched {
            cursor += 1;
            if cursor == stages.len() {
                break;
            }
        }
    }

    if cursor != stages.len() {
        let observed: Vec<String> = schedule
            .iter()
            .map(|op| match &op.to {
                Some(to) => format!("{:?}({} -> {})", op.kind, op.path.display(), to.display()),
                None => format!("{:?}({})", op.kind, op.path.display()),
            })
            .collect();
        return Err(std::io::Error::other(format!(
            "PROPERTY (#185): the compaction publication schedule must contain, in protocol \
             order, every declared publication op; first missing stage {cursor} ({stage}). \
             Observed schedule: {observed:?}",
            stage = stages[cursor],
        ))
        .into());
    }

    drop(store);
    Ok(())
}

#[test]
fn workload_crash_at_every_publication_boundary_reopens_legal(
) -> Result<(), Box<dyn std::error::Error>> {
    // PROPERTY (#185/#177): for EVERY publication op k in 1..=T of the compaction
    // schedule, a crash on that op reopens WITHOUT close() into exactly the OLD
    // complete generation, the NEW complete generation, or a typed canonical
    // recovery refusal; every keyed retry returns its original receipt; and
    // across the whole walk BOTH old and new outcomes are witnessed (the walk
    // crossed the commit point). Reproducible from (MANIFEST_V1, seed, k).
    let seed = harness::MANIFEST_V1.seed;

    // Probe: enumerate the compaction publication schedule length T. The schedule
    // is deterministic (single writer, sync_every=1, checkpoint/mmap off), so T
    // and the per-op absolute indices are stable across the armed runs below.
    let publication_ops = {
        let sim = ShadowFs::new();
        let store =
            Store::open(harness::config(Path::new(DATA_DIR)).with_fs(Arc::new(sim.clone())))
                .expect("probe: open store over shadow fs");
        let _pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
        let base = sim.mutation_ops().len();
        let (result, _report) = store
            .compact(&harness::evict_all_evict_kind())
            .expect("probe: an unarmed compaction must succeed");
        let CompactionOutcome::Performed = result.outcome else {
            return Err(std::io::Error::other(format!(
                "probe: the compaction must PERFORM to enumerate a schedule (seed={seed:#x}); \
                 got {:?}",
                result.outcome
            ))
            .into());
        };
        let t = sim.mutation_ops().len() - base;
        drop(store);
        t
    };
    if publication_ops == 0 {
        return Err(std::io::Error::other(
            "probe: a performed compaction recorded no publication ops to walk",
        )
        .into());
    }

    let mut old_seen = 0usize;
    let mut new_seen = 0usize;
    let mut refusals = 0usize;
    // The retention config evicts every EVICT_KIND event, so the new generation's
    // evict census is empty; classification is exactly old-or-new on that lane.
    let new_evict: BTreeSet<u128> = BTreeSet::new();

    for k in 1..=publication_ops {
        let sim = ShadowFs::new();
        let store =
            Store::open(harness::config(Path::new(DATA_DIR)).with_fs(Arc::new(sim.clone())))
                .expect("open store over shadow fs");
        let pairs = harness::seed_workload(&store, &harness::MANIFEST_V1);
        let old_evict = harness::kind_census(&store, harness::EVICT_KIND);
        let survivors = harness::kind_census(&store, harness::SURVIVOR_KIND);

        // Arm a power loss on the k-th (1-based) publication op: absolute op index
        // base + (k - 1) into the shared mutation log. `arm_crash_at_op` fires AND
        // poisons, so the in-process rollback path cannot heal the namespace.
        let base = sim.mutation_ops().len();
        sim.arm_crash_at_op(base + (k - 1));

        // The compaction aborts at most boundaries; a warn-only final cleanup op
        // may instead complete in memory. Either way the armed op must have fired
        // — that, not the compaction's return value, is the crash's non-vacuity.
        let _outcome = store.compact(&harness::evict_all_evict_kind());
        assert!(
            sim.armed_faults_fired() > 0,
            "PROPERTY (#185): the crash armed on publication op k={k} of {publication_ops} \
             (seed={seed:#x}) must actually fire (non-vacuous)",
        );

        // drop = crash: abandon the writer against the poisoned simulator, then
        // revert every visible-but-not-durable namespace effect.
        drop(store);
        sim.crash();

        match Store::open(harness::config(Path::new(DATA_DIR)).with_fs(Arc::new(sim.clone()))) {
            Ok(reopened) => {
                let rec_evict = harness::kind_census(&reopened, harness::EVICT_KIND);
                let generation = harness::classify_generation(&rec_evict, &old_evict, &new_evict)
                    .map_err(|why| {
                        std::io::Error::other(format!(
                            "PROPERTY (#185/#177): reopen after a crash on op k={k} \
                             (seed={seed:#x}) recovered an ILLEGAL mixed generation: {why}"
                        ))
                    })?;
                assert_eq!(
                    harness::kind_census(&reopened, harness::SURVIVOR_KIND),
                    survivors,
                    "PROPERTY (#185): the survivor census must stay intact across a crash on \
                     op k={k} (seed={seed:#x})",
                );
                for (key, original) in &pairs {
                    harness::assert_retry_is_original(&reopened, *key, original);
                }
                assert_eq!(
                    harness::kind_census(&reopened, harness::EVICT_KIND),
                    rec_evict,
                    "PROPERTY (#185): keyed retries must re-append NOTHING (no undead evict \
                     event) after a crash on op k={k} (seed={seed:#x})",
                );
                assert_eq!(
                    harness::kind_census(&reopened, harness::SURVIVOR_KIND),
                    survivors,
                    "PROPERTY (#185): keyed retries must not disturb the survivor census after \
                     a crash on op k={k} (seed={seed:#x})",
                );
                match generation {
                    harness::RecoveredGeneration::OldComplete => old_seen += 1,
                    harness::RecoveredGeneration::NewComplete => new_seen += 1,
                }
                drop(reopened);
            }
            Err(err) => {
                if harness::is_canonical_recovery_refusal(&err) {
                    refusals += 1;
                } else {
                    return Err(std::io::Error::other(format!(
                        "PROPERTY (#185/#177): reopen after a crash on op k={k} (seed={seed:#x}) \
                         refused NON-canonically: {err:?}"
                    ))
                    .into());
                }
            }
        }
    }

    // Distribution anti-vacuity: the walk crossed the commit point, and every one
    // of the T boundaries resolved to exactly one legal outcome.
    assert!(
        old_seen >= 1,
        "PROPERTY (#185): the boundary walk must witness at least one OLD-complete recovery \
         (a pre-commit crash) — seed={seed:#x}",
    );
    assert!(
        new_seen >= 1,
        "PROPERTY (#185): the boundary walk must witness at least one NEW-complete recovery \
         (a post-commit crash) — seed={seed:#x}",
    );
    assert_eq!(
        old_seen + new_seen + refusals,
        publication_ops,
        "PROPERTY (#185): every one of the {publication_ops} publication boundaries must \
         resolve to exactly old / new / typed-refusal — seed={seed:#x}",
    );
    Ok(())
}

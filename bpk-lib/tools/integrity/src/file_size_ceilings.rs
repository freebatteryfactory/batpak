//! Monotonic per-file size ceiling (P0-1).
//!
//! Replaces the deleted editable size-override consts with a committed lock
//! (`traceability/file_size_ceilings.lock`) that may only ratchet DOWN. Every
//! production source file in the "pressure zone" (>= 60% of the absolute cap)
//! records its current nonblank line count; the gate then requires every file
//! to stay at or below `min(absolute cap, its recorded ceiling)`, and any blessing
//! that would RAISE a recorded ceiling is rejected. "Split, don't bump" is
//! mechanical: you cannot loosen a ceiling, only tighten it by shrinking the file.

use crate::repo_surface::{ensure, load_yaml, relative};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Files at or above this fraction (percent) of the absolute cap are tracked in
/// the lock. Below the floor a file needs no recorded ceiling.
pub(crate) const PRESSURE_ZONE_FLOOR_PCT: usize = 60;

/// The lock file path, repo-root-relative.
pub(crate) const LOCK_REL: &str = "traceability/file_size_ceilings.lock";

/// One recorded ceiling: a repo-root-relative path and its frozen nonblank count.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Ceiling {
    pub(crate) path: String,
    pub(crate) max_nonblank: usize,
}

pub(crate) fn floor(absolute_cap: usize) -> usize {
    absolute_cap * PRESSURE_ZONE_FLOOR_PCT / 100
}

pub(crate) fn lock_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(LOCK_REL)
}

/// Load the committed lock as a `path -> max_nonblank` map. A missing lock is an
/// empty map (the gate then demands entries for any pressure-zone file).
pub(crate) fn load_lock(repo_root: &Path) -> Result<BTreeMap<String, usize>> {
    let path = lock_path(repo_root);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let entries: Vec<Ceiling> = load_yaml(&path)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        ensure(
            map.insert(entry.path.clone(), entry.max_nonblank).is_none(),
            format!(
                "file_size_ceilings.lock: duplicate entry for `{}`",
                entry.path
            ),
        )?;
    }
    Ok(map)
}

/// Serialize a `path -> max_nonblank` map back into the lock's on-disk form
/// (sorted by path, since the map is a `BTreeMap`).
pub(crate) fn render_lock(ceilings: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(
        "# AUTO-MANAGED monotonic file-size ceiling (P0-1).\n\
         # The gate may only LOWER a value; raising a ceiling is rejected (\"split, don't bump\").\n\
         # Every production source file at or above 60% of the 850-line absolute cap is recorded.\n",
    );
    for (path, max_nonblank) in ceilings {
        out.push_str(&format!("- path: {path}\n  max_nonblank: {max_nonblank}\n"));
    }
    out
}

/// Gate result classification for one file (kept testable without filesystem).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CeilingVerdict {
    /// Within `min(cap, ceiling)` and not below a recorded ceiling — OK.
    Ok,
    /// `actual` exceeds the effective budget — split it.
    OverBudget { actual: usize, budget: usize },
    /// `actual` is below the recorded ceiling — the ceiling is stale and must be
    /// ratcheted DOWN to `actual`.
    StaleCeiling { actual: usize, recorded: usize },
    /// A pressure-zone file with no recorded ceiling — must be recorded.
    MissingEntry { actual: usize, floor: usize },
}

/// Classify one file against the absolute cap and its (optional) recorded ceiling.
pub(crate) fn classify(
    actual: usize,
    recorded: Option<usize>,
    absolute_cap: usize,
) -> CeilingVerdict {
    match recorded {
        Some(recorded) => {
            let budget = recorded.min(absolute_cap);
            if actual > budget {
                CeilingVerdict::OverBudget { actual, budget }
            } else if actual < recorded {
                CeilingVerdict::StaleCeiling { actual, recorded }
            } else {
                CeilingVerdict::Ok
            }
        }
        None => {
            if actual > absolute_cap {
                CeilingVerdict::OverBudget {
                    actual,
                    budget: absolute_cap,
                }
            } else if actual >= floor(absolute_cap) {
                CeilingVerdict::MissingEntry {
                    actual,
                    floor: floor(absolute_cap),
                }
            } else {
                CeilingVerdict::Ok
            }
        }
    }
}

/// Run the ratchet gate over `files` (repo-root-relative path + current nonblank
/// count), consulting the committed lock. Returns `Err` on the first violation.
/// `actuals` must already be restricted to tracked production files.
pub(crate) fn check(
    repo_root: &Path,
    actuals: &BTreeMap<String, usize>,
    absolute_cap: usize,
) -> Result<()> {
    let lock = load_lock(repo_root)?;

    for (rel, &actual) in actuals {
        match classify(actual, lock.get(rel).copied(), absolute_cap) {
            CeilingVerdict::Ok => {}
            CeilingVerdict::OverBudget { actual, budget } => ensure(
                false,
                format!(
                    "structural-check: file-size ceiling exceeded in {rel}: {actual} nonblank lines > budget {budget}.\n\
                     Split the file. The ceiling can only ratchet DOWN; it cannot be raised to admit growth."
                ),
            )?,
            CeilingVerdict::StaleCeiling { actual, recorded } => ensure(
                false,
                format!(
                    "structural-check: stale file-size ceiling for {rel}: recorded {recorded} but file is now {actual}.\n\
                     Lower `max_nonblank` to {actual} in {LOCK_REL} (the ceiling ratchets DOWN to track reality)."
                ),
            )?,
            CeilingVerdict::MissingEntry { actual, floor } => ensure(
                false,
                format!(
                    "structural-check: {rel} is {actual} nonblank lines (>= the pressure-zone floor {floor}) but has no entry in {LOCK_REL}.\n\
                     Add `- path: {rel}` with `max_nonblank: {actual}` so its ceiling is frozen and can only ratchet DOWN."
                ),
            )?,
        }
    }

    // Anti-rot: a lock entry naming a file not present in the tracked set is stale.
    for path in lock.keys() {
        ensure(
            actuals.contains_key(path),
            format!(
                "file_size_ceilings.lock: entry `{path}` names no tracked production file; remove the stale entry."
            ),
        )?;
    }
    Ok(())
}

/// Compute the blessed lock for `actuals`: every pressure-zone file records its
/// current count, but a recorded ceiling may only be LOWERED. Returns `Err` if
/// the new computed value for any path would exceed its existing recorded
/// ceiling (a raise), which is the forbidden direction.
pub(crate) fn bless(
    existing: &BTreeMap<String, usize>,
    actuals: &BTreeMap<String, usize>,
    absolute_cap: usize,
) -> Result<BTreeMap<String, usize>> {
    let floor = floor(absolute_cap);
    let mut blessed = BTreeMap::new();
    for (path, &actual) in actuals {
        if actual < floor {
            // Below the pressure zone: not recorded. If it had an entry it is
            // simply dropped (the file shrank out of the zone).
            continue;
        }
        if let Some(&recorded) = existing.get(path) {
            ensure(
                actual <= recorded,
                format!(
                    "file_size_ceilings.lock: refusing to RAISE the ceiling for `{path}` from {recorded} to {actual}. \
                     Ceilings only ratchet DOWN; split the file instead of bumping its ceiling."
                ),
            )?;
        }
        blessed.insert(path.clone(), actual);
    }
    Ok(blessed)
}

/// Write a freshly-blessed lock to disk (used by the one-shot generator path).
pub(crate) fn write_lock(repo_root: &Path, ceilings: &BTreeMap<String, usize>) -> Result<()> {
    let path = lock_path(repo_root);
    std::fs::write(&path, render_lock(ceilings))
        .with_context(|| format!("write {}", relative(repo_root, &path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP: usize = 850; // floor = 510

    #[test]
    fn classify_unrecorded_file_below_floor_is_ok() {
        assert_eq!(classify(400, None, CAP), CeilingVerdict::Ok);
    }

    #[test]
    fn classify_unrecorded_pressure_zone_file_requires_entry() {
        assert_eq!(
            classify(600, None, CAP),
            CeilingVerdict::MissingEntry {
                actual: 600,
                floor: 510
            }
        );
    }

    #[test]
    fn classify_recorded_file_at_ceiling_is_ok() {
        assert_eq!(classify(600, Some(600), CAP), CeilingVerdict::Ok);
    }

    // RED (a): a file exceeding its recorded ceiling -> OverBudget.
    #[test]
    fn classify_file_over_recorded_ceiling_is_over_budget() {
        assert_eq!(
            classify(601, Some(600), CAP),
            CeilingVerdict::OverBudget {
                actual: 601,
                budget: 600
            }
        );
    }

    #[test]
    fn classify_file_under_recorded_ceiling_is_stale() {
        assert_eq!(
            classify(590, Some(600), CAP),
            CeilingVerdict::StaleCeiling {
                actual: 590,
                recorded: 600
            }
        );
    }

    #[test]
    fn classify_unrecorded_over_absolute_cap_is_over_budget() {
        assert_eq!(
            classify(851, None, CAP),
            CeilingVerdict::OverBudget {
                actual: 851,
                budget: 850
            }
        );
    }

    // RED (b): blessing that would RAISE a recorded ceiling -> Err.
    #[test]
    fn bless_refuses_to_raise_a_ceiling() {
        let existing = BTreeMap::from([("a.rs".to_owned(), 600)]);
        let actuals = BTreeMap::from([("a.rs".to_owned(), 650)]);
        let err = bless(&existing, &actuals, CAP).expect_err("raising a ceiling must fail");
        assert!(err.to_string().contains("RAISE"), "{err:?}");
    }

    // GREEN: blessing that lowers a ceiling succeeds and records the lower value.
    #[test]
    fn bless_lowers_a_ceiling() {
        let existing = BTreeMap::from([("a.rs".to_owned(), 600)]);
        let actuals = BTreeMap::from([("a.rs".to_owned(), 550)]);
        let blessed = bless(&existing, &actuals, CAP).expect("lowering a ceiling must succeed");
        assert_eq!(blessed.get("a.rs"), Some(&550));
    }

    #[test]
    fn bless_drops_files_that_shrink_below_floor() {
        let existing = BTreeMap::from([("a.rs".to_owned(), 600)]);
        let actuals = BTreeMap::from([("a.rs".to_owned(), 100)]);
        let blessed = bless(&existing, &actuals, CAP).expect("shrinking below floor must succeed");
        assert!(blessed.is_empty());
    }

    #[test]
    fn render_then_parse_roundtrips() {
        let ceilings = BTreeMap::from([("a.rs".to_owned(), 600), ("b.rs".to_owned(), 700)]);
        let rendered = render_lock(&ceilings);
        let parsed: Vec<Ceiling> = yaml_serde::from_str(&rendered).expect("parse rendered lock");
        let back: BTreeMap<String, usize> = parsed
            .into_iter()
            .map(|c| (c.path, c.max_nonblank))
            .collect();
        assert_eq!(back, ceilings);
    }
}

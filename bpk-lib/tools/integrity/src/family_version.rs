//! GAUNT-FAMILY-VERSION — the family release line moves in LOCKSTEP.
//!
//! Every `*_semver_checklist.yaml` in `traceability/public_api/` records a
//! `current_version`. The batpak family versions in lockstep (one workspace
//! release line), so each `current_version` MUST equal the workspace family
//! version (`bpk-lib/crates/core/Cargo.toml`'s `package.version`).
//!
//! This replaces the previous ast-grep caliper, which was PROVABLY VACUOUS: it
//! matched `kind: plain_scalar` (whose node text is the VALUE only, e.g.
//! `0.8.2`) with a `^current_version: …$` regex that can never match a
//! value-only scalar, and it ran only under `just ast-grep` (never CI) behind an
//! external `sg` binary. This native check is non-vacuous, forward-safe (no
//! hand-coded stale-version table — it derives the target from the manifest), and
//! runs inside `structural-check` (hence `ci_fast`, hence CI).

use crate::repo_surface::{ensure, files_with_extension, load_yaml, relative};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

/// The `traceability/public_api/` directory holding the semver checklists.
/// Relative to the integrity `repo_root`, which is the WORKSPACE dir (`bpk-lib`),
/// not the git root — mirrors `unsafe_ledger`'s unprefixed paths.
const CHECKLIST_DIR_REL: &str = "traceability/public_api";

/// The workspace family-version source of truth (relative to the workspace-dir
/// `repo_root`).
const FAMILY_VERSION_MANIFEST_REL: &str = "crates/core/Cargo.toml";

/// The subset of a semver checklist this gate reads.
#[derive(Debug, Deserialize)]
struct SemverChecklist {
    current_version: String,
}

/// Reconcile every semver checklist's `current_version` against the workspace
/// family version, fail-closed on any drift.
pub(crate) fn check(repo_root: &Path) -> Result<()> {
    let workspace_version = read_family_version(repo_root)?;
    let dir = repo_root.join(CHECKLIST_DIR_REL);

    let mut drift: Vec<String> = Vec::new();
    for path in files_with_extension(&dir, "yaml") {
        let rel = relative(repo_root, &path);
        if !rel.ends_with("_semver_checklist.yaml") {
            continue;
        }
        let checklist: SemverChecklist = load_yaml(&path).with_context(|| format!("load {rel}"))?;
        if let Some(violation) =
            checklist_drift(&rel, &checklist.current_version, &workspace_version)
        {
            drift.push(violation);
        }
    }

    drift.sort();
    ensure(
        drift.is_empty(),
        format!(
            "family-version-lockstep: {} semver checklist(s) drifted from the workspace \
             release line `{workspace_version}`:\n  {}\nThe family versions move in lockstep \
             — sync each `current_version` to `{workspace_version}` [GAUNT-FAMILY-VERSION]",
            drift.len(),
            drift.join("\n  ")
        ),
    )
}

/// The drift predicate, factored out so the fixtures can convict the exact
/// in-tree drift the old vacuous gate missed (e.g. `0.8.2` under `0.10.0`).
fn checklist_drift(rel: &str, current_version: &str, workspace_version: &str) -> Option<String> {
    (current_version != workspace_version).then(|| {
        format!(
            "{rel}: current_version `{current_version}` != workspace family version \
             `{workspace_version}`"
        )
    })
}

/// Read `package.version` from the core manifest by text scan (integrity carries
/// no TOML parser). The package version is the sole top-level `version = "…"`
/// line; dependency versions are inline tables (`dep = {{ version = "…" }}`) or
/// indented, never a bare `version = "…"` at column zero.
fn read_family_version(repo_root: &Path) -> Result<String> {
    let manifest = repo_root.join(FAMILY_VERSION_MANIFEST_REL);
    let text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("read {}", manifest.display()))?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("version = \"") {
            if let Some(version) = rest.strip_suffix('"') {
                return Ok(version.to_owned());
            }
        }
    }
    bail!(
        "{} has no top-level `version = \"…\"` (the family version source of truth)",
        manifest.display()
    )
}

#[cfg(test)]
mod tests {
    use super::checklist_drift;

    /// Red fixture: the EXACT in-tree drift the old vacuous ast-grep gate missed
    /// — a checklist pinned to `0.8.2` while the family line is `0.10.0` — is now
    /// flagged. Before this gate that value-only `plain_scalar` regex never
    /// matched, so the drift shipped silently.
    #[test]
    fn stale_checklist_version_is_flagged() {
        let violation = checklist_drift(
            "bpk-lib/traceability/public_api/syncbat_semver_checklist.yaml",
            "0.8.2",
            "0.10.0",
        )
        .expect("a checklist below the family line must be flagged");
        assert!(violation.contains("0.8.2"));
        assert!(violation.contains("0.10.0"));
    }

    /// A checklist already on the family line is clean.
    #[test]
    fn in_lockstep_checklist_passes() {
        assert!(checklist_drift("batpak_semver_checklist.yaml", "0.10.0", "0.10.0").is_none());
    }
}

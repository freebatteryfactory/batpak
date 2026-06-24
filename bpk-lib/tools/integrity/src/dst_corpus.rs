//! DST corpus ledger consumer (Thread #64-B, slug `dst-corpus-currency`).
//!
//! `traceability/dst_corpus.yaml` is the durable graduated-seed corpus. This module
//! validates schema and non-emptiness on every `structural-check`. Digest replay
//! currency is proven by the `dst-corpus-currency` integration gate in
//! `crates/core/tests/dst_corpus_currency.rs` (requires `dangerous-test-hooks`).

use crate::repo_surface::load_yaml;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Repo-relative path to the graduated DST corpus ledger.
pub(crate) const DST_CORPUS_REL: &str = "traceability/dst_corpus.yaml";

/// One graduated corpus row. Mirrors the schema documented in the yaml header.
#[derive(Debug, Deserialize)]
struct DstCorpusRow {
    seed: u64,
    fault_mode: String,
    #[serde(default)]
    boundary: Option<String>,
    #[serde(default)]
    fsync_drop_one_in: Option<u32>,
    seam_touched: String,
    assurance_level: String,
    steps: u32,
    op_trace_digest: u64,
    outcome: String,
}

/// The durability-boundary labels a `CrashBeforeFsync` row may declare. Mirrors
/// `store::sim::recovery_matrix::Boundary`.
const BOUNDARY_LABELS: [&str; 4] = [
    "SingleAppendFrame",
    "BatchCommitMarker",
    "BatchPostFsyncPrePublish",
    "SegmentRotationCreate",
];

fn manifest_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(DST_CORPUS_REL)
}

fn load_rows(repo_root: &Path) -> Result<Vec<DstCorpusRow>> {
    load_yaml(&manifest_path(repo_root))
}

fn validate_row(row: &DstCorpusRow, index: usize) -> Result<()> {
    let seed = row.seed;
    if row.seam_touched.is_empty() {
        bail!("dst-corpus-currency: entry[{index}] (seed={seed}) has empty seam_touched");
    }
    if row.assurance_level.is_empty() {
        bail!("dst-corpus-currency: entry[{index}] (seed={seed}) has empty assurance_level");
    }
    if row.steps == 0 {
        bail!("dst-corpus-currency: entry[{index}] (seed={seed}) steps must be >= 1");
    }
    if row.op_trace_digest == 0 {
        bail!(
            "dst-corpus-currency: entry[{index}] (seed={seed}) op_trace_digest must be non-zero \
             (run graduation to fill the identity digest)"
        );
    }
    // The corpus routes three fault modes (honest disk via `recovery::run`,
    // lying-disk + crash-before-fsync via `recovery_matrix::run`). The boundary
    // and fsync-drop columns are load-bearing per mode: a CrashBeforeFsync row
    // MUST name a known boundary (and no drop rate); a LyingDiskFsyncDrop row MUST
    // name a `>= 1` drop rate (and no boundary); an honest-disk row leaves both
    // null. This keeps the structural schema in lockstep with the replay router.
    match row.fault_mode.as_str() {
        "HonestDiskCrash" => {
            if row.boundary.is_some() {
                bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) HonestDiskCrash must leave \
                     boundary null"
                );
            }
            if row.fsync_drop_one_in.is_some() {
                bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) HonestDiskCrash must leave \
                     fsync_drop_one_in null"
                );
            }
        }
        "LyingDiskFsyncDrop" => {
            if row.boundary.is_some() {
                bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) LyingDiskFsyncDrop must \
                     leave boundary null"
                );
            }
            match row.fsync_drop_one_in {
                Some(rate) if rate >= 1 => {}
                _ => bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) LyingDiskFsyncDrop requires \
                     fsync_drop_one_in >= 1"
                ),
            }
        }
        "CrashBeforeFsync" => {
            if row.fsync_drop_one_in.is_some() {
                bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) CrashBeforeFsync must leave \
                     fsync_drop_one_in null"
                );
            }
            match row.boundary.as_deref() {
                Some(label) if BOUNDARY_LABELS.contains(&label) => {}
                Some(other) => bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) unknown boundary `{other}`"
                ),
                None => bail!(
                    "dst-corpus-currency: entry[{index}] (seed={seed}) CrashBeforeFsync requires a \
                     boundary label"
                ),
            }
        }
        other => bail!(
            "dst-corpus-currency: entry[{index}] (seed={seed}) fault_mode `{other}` is not a \
             routed mode (HonestDiskCrash | LyingDiskFsyncDrop | CrashBeforeFsync)"
        ),
    }
    match row.outcome.as_str() {
        "CommittedPrefix" | "RolledBack" | "CanonicalRefusal" => {}
        other => bail!(
            "dst-corpus-currency: entry[{index}] outcome `{other}` is not a legal classification"
        ),
    }
    Ok(())
}

/// Structural entry: schema-validates the corpus and requires at least one entry.
pub(crate) fn check(repo_root: &Path) -> Result<()> {
    let rows = load_rows(repo_root).context("load dst_corpus.yaml")?;
    if rows.is_empty() {
        bail!("dst-corpus-currency: traceability/dst_corpus.yaml must be non-empty");
    }
    for (index, row) in rows.iter().enumerate() {
        validate_row(row, index)?;
    }
    outln!(
        "dst-corpus-currency: ok ({} graduated seed(s) in corpus)",
        rows.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_surface::repo_root;

    fn repo() -> std::path::PathBuf {
        repo_root().expect("repo root resolves from tools/integrity")
    }

    #[test]
    fn committed_corpus_passes_schema_check() {
        check(&repo()).expect("committed dst_corpus.yaml must pass schema check");
    }

    #[test]
    fn empty_corpus_fails_schema_check() {
        let err = validate_corpus_rows(&[]).expect_err("empty corpus must fail");
        assert!(
            err.to_string().contains("non-empty"),
            "error must mention non-empty requirement, got: {err}"
        );
    }

    #[test]
    fn zero_digest_fails_schema_check() {
        let rows = vec![DstCorpusRow {
            seed: 1,
            fault_mode: "HonestDiskCrash".to_owned(),
            boundary: None,
            fsync_drop_one_in: None,
            seam_touched: "writer-commit".to_owned(),
            assurance_level: "L4".to_owned(),
            steps: 32,
            op_trace_digest: 0,
            outcome: "CommittedPrefix".to_owned(),
        }];
        let err = validate_corpus_rows(&rows).expect_err("zero digest must fail");
        assert!(
            err.to_string().contains("op_trace_digest"),
            "error must mention digest, got: {err}"
        );
    }

    fn row(
        fault_mode: &str,
        boundary: Option<&str>,
        fsync_drop_one_in: Option<u32>,
    ) -> DstCorpusRow {
        DstCorpusRow {
            seed: 7,
            fault_mode: fault_mode.to_owned(),
            boundary: boundary.map(str::to_owned),
            fsync_drop_one_in,
            seam_touched: "writer-commit".to_owned(),
            assurance_level: "L4".to_owned(),
            steps: 32,
            op_trace_digest: 123,
            outcome: "CommittedPrefix".to_owned(),
        }
    }

    #[test]
    fn crash_before_fsync_requires_a_known_boundary() {
        // Missing boundary is rejected.
        let err = validate_row(&row("CrashBeforeFsync", None, None), 0)
            .expect_err("CrashBeforeFsync without a boundary must fail");
        assert!(
            err.to_string().contains("requires a"),
            "error must demand a boundary, got: {err}"
        );
        // Unknown boundary is rejected.
        let err = validate_row(&row("CrashBeforeFsync", Some("Nope"), None), 0)
            .expect_err("unknown boundary must fail");
        assert!(
            err.to_string().contains("unknown boundary"),
            "error must name the unknown boundary, got: {err}"
        );
        // A known boundary passes.
        validate_row(&row("CrashBeforeFsync", Some("BatchCommitMarker"), None), 0)
            .expect("known boundary must pass");
    }

    #[test]
    fn lying_disk_requires_a_drop_rate_and_no_boundary() {
        let err = validate_row(&row("LyingDiskFsyncDrop", None, None), 0)
            .expect_err("LyingDiskFsyncDrop without a drop rate must fail");
        assert!(
            err.to_string().contains("fsync_drop_one_in"),
            "error must demand a drop rate, got: {err}"
        );
        let err = validate_row(
            &row("LyingDiskFsyncDrop", Some("BatchCommitMarker"), Some(2)),
            0,
        )
        .expect_err("LyingDiskFsyncDrop with a boundary must fail");
        assert!(
            err.to_string().contains("boundary"),
            "error must reject the boundary, got: {err}"
        );
        validate_row(&row("LyingDiskFsyncDrop", None, Some(2)), 0)
            .expect("valid lying-disk row must pass");
    }

    #[test]
    fn honest_disk_rejects_boundary_and_drop_rate() {
        let err = validate_row(&row("HonestDiskCrash", Some("BatchCommitMarker"), None), 0)
            .expect_err("HonestDiskCrash with a boundary must fail");
        assert!(
            err.to_string().contains("boundary"),
            "error must reject the boundary, got: {err}"
        );
        let err = validate_row(&row("HonestDiskCrash", None, Some(2)), 0)
            .expect_err("HonestDiskCrash with a drop rate must fail");
        assert!(
            err.to_string().contains("fsync_drop_one_in"),
            "error must reject the drop rate, got: {err}"
        );
    }

    #[test]
    fn unknown_fault_mode_is_rejected() {
        let err = validate_row(&row("Teleport", None, None), 0)
            .expect_err("unknown fault_mode must fail");
        assert!(
            err.to_string().contains("routed mode"),
            "error must reject the unknown mode, got: {err}"
        );
    }

    fn validate_corpus_rows(rows: &[DstCorpusRow]) -> Result<()> {
        if rows.is_empty() {
            bail!("dst-corpus-currency: traceability/dst_corpus.yaml must be non-empty");
        }
        for (index, row) in rows.iter().enumerate() {
            validate_row(row, index)?;
        }
        Ok(())
    }
}

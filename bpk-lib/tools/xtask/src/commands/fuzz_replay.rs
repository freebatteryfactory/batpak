//! `cargo xtask fuzz-replay` — the PR-blocking fuzz-replay gate
//! (GAUNT-FUZZ-1 + GAUNT-PROOF-OF-PROOF #197).
//!
//! The historical false-green: `crates/core/tests/fuzz_replay.rs` (and its new
//! semantic sibling `fuzz_replay_semantics.rs`) are
//! `#![cfg(feature = "dangerous-test-hooks")]` — without the feature the binaries
//! compile to ZERO tests and a plain `cargo test` exits green having proved
//! nothing. This gate closes that hole from two directions:
//!   1. it `nextest list`s the two binaries UNDER the feature and FAILS if any of
//!      the intended tests is missing (compiled-out binary) or `#[ignore]`d
//!      (hollowed-out test);
//!   2. it runs them with `--no-tests=fail` as a belt-and-braces backstop, then
//!      writes a machine `ProofReceipt` recording the exact command, the enabled
//!      features, the executed/skipped counts, and the corpus + regression
//!      identity (content hashes) + commit SHA. The receipt schema uses
//!      `deny_unknown_fields` with NO `Option` fields, so a receipt that omits
//!      executed/skipped/features cannot even parse (gate-(f) by construction).
//!
//! The feature-independent lockstep + anti-ceremony law lives separately in
//! `batpak-integrity`'s `fuzz-replay-contract` (structural-check) — that one
//! cannot compile out. This command owns the "the binaries actually ran their
//! intended tests" leg plus the proof receipt.

use crate::commands::factory_ledger;
use crate::util::{self, cargo_target_dir, repo_root, run_output};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Every test the fuzz-replay gate claims to execute. The gate FAILS if nextest's
/// resolved list does not contain every one of these — the "binary silently
/// compiled out / zero intended tests" false-green (GAUNT-PROOF-OF-PROOF #197).
const REQUIRED_TESTS: &[&str] = &[
    "fuzz_replay::fuzz_replay_no_panic_on_committed_corpus",
    "fuzz_replay::fuzz_replay_covers_every_target",
    "fuzz_replay_semantics::sidx_manifest_regression_is_semantically_load_bearing",
    "fuzz_replay_semantics::idemp_image_regression_is_semantically_load_bearing",
];

/// The nextest filter expression selecting exactly the two proof binaries.
const NEXTEST_FILTER: &str = "binary(fuzz_replay) + binary(fuzz_replay_semantics)";

/// The feature set the gate compiles the proof binaries under. Recorded verbatim
/// in the receipt so a reader can see the binaries were NOT compiled out.
const FEATURES: &[&str] = &["dangerous-test-hooks"];

/// The test binaries the gate drives (receipt `binaries`).
const BINARIES: &[&str] = &["fuzz_replay", "fuzz_replay_semantics"];

/// A machine proof receipt for one fuzz-replay gate run. Every field is
/// required (no `Option`) and unknown fields are denied — a receipt that omits
/// the executed/skipped counts or the feature list cannot parse, so gate-(f)
/// (the receipt must carry executed/skipped counts + features) holds by
/// construction rather than by a runtime check.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofReceipt {
    pub schema_version: u32,
    pub gate: String,
    pub command: String,
    pub features: Vec<String>,
    pub binaries: Vec<String>,
    pub required_tests: Vec<String>,
    pub listed_tests: usize,
    pub executed_tests: usize,
    pub skipped_tests: usize,
    pub corpus_files: usize,
    pub corpus_hash: String,
    pub regression_files: usize,
    pub regression_hash: String,
    pub regression_manifest_hash: String,
    pub commit_sha: String,
    pub branch: String,
    pub started: String,
    pub ended: String,
    pub verdict: String,
}

pub(crate) fn run() -> Result<()> {
    let root = repo_root()?;
    let started = iso8601_now();

    outln!("fuzz-replay: listing the proof binaries under {FEATURES:?} ...");
    let (listed, skipped) = list_tests()?;
    assert_selected(&listed, &skipped)?;
    outln!(
        "fuzz-replay: {} required test(s) present, {} skipped; running ...",
        listed.len(),
        skipped.len()
    );

    // Actually run them. `--no-tests=fail` backstops the zero-test case; inherited
    // stdio (util::cargo) means the FULL, untruncated test output is on the wire.
    // A failing run bails here and no receipt is written — the receipt exists only
    // on a clean pass.
    let run_args = run_argv();
    let command = format!("cargo {}", run_args.join(" "));
    util::cargo(run_args.iter().map(String::as_str))
        .context("fuzz-replay: nextest run of the proof binaries failed")?;

    let fuzz_dir = root.join("fuzz");
    let (corpus_files, corpus_hash) = hash_tree(&fuzz_dir.join("corpus"), &root)?;
    let (regression_files, regression_hash) = hash_tree(&fuzz_dir.join("regressions"), &root)?;
    let regression_manifest_hash =
        hash_file(&fuzz_dir.join("regressions").join("manifest.yaml"))?;

    let receipt = ProofReceipt {
        schema_version: 1,
        gate: "fuzz-replay".to_owned(),
        command,
        features: FEATURES.iter().map(|s| (*s).to_owned()).collect(),
        binaries: BINARIES.iter().map(|s| (*s).to_owned()).collect(),
        required_tests: REQUIRED_TESTS.iter().map(|s| (*s).to_owned()).collect(),
        listed_tests: listed.len(),
        // retries=0, fail-fast=false, and the run above passed ⇒ every listed
        // test executed exactly once.
        executed_tests: listed.len(),
        skipped_tests: skipped.len(),
        corpus_files,
        corpus_hash,
        regression_files,
        regression_hash,
        regression_manifest_hash,
        commit_sha: util::git_output(&root, ["rev-parse", "HEAD"])
            .context("fuzz-replay: resolve HEAD commit sha")?,
        branch: util::git_output(&root, ["rev-parse", "--abbrev-ref", "HEAD"])
            .context("fuzz-replay: resolve current branch")?,
        started,
        ended: iso8601_now(),
        verdict: "PASS".to_owned(),
    };
    validate_proof_receipt(&receipt)?;

    let receipts_dir = cargo_target_dir()?.join("proof-receipts");
    fs::create_dir_all(&receipts_dir).with_context(|| {
        format!("create proof-receipts directory {}", receipts_dir.display())
    })?;
    let receipt_path = receipts_dir.join("fuzz-replay.json");
    let json = serde_json::to_string_pretty(&receipt)
        .context("serialize fuzz-replay proof receipt")?;
    fs::write(&receipt_path, json)
        .with_context(|| format!("write proof receipt {}", receipt_path.display()))?;
    outln!(
        "fuzz-replay: PASS — {} test(s), 0 skipped; receipt {}",
        receipt.executed_tests,
        receipt_path.display()
    );

    // Opt-in factory-ledger carry: records ONLY when a ledger store already exists.
    if factory_ledger::append_proof_receipt_if_ledger_exists(&receipt)
        .context("fuzz-replay: record proof receipt to the factory ledger")?
    {
        outln!("fuzz-replay: proof receipt recorded to the factory ledger");
    }
    Ok(())
}

/// The exact `cargo nextest run ...` argv (sans the leading `cargo`).
fn run_argv() -> Vec<String> {
    vec![
        "nextest".to_owned(),
        "run".to_owned(),
        "--profile".to_owned(),
        "ci".to_owned(),
        "-p".to_owned(),
        "batpak".to_owned(),
        "--features".to_owned(),
        FEATURES.join(","),
        "-E".to_owned(),
        NEXTEST_FILTER.to_owned(),
        "--no-tests=fail".to_owned(),
        "--no-capture".to_owned(),
    ]
}

/// `nextest list --message-format json` the two proof binaries and split the
/// resolved testcases into (selected, skipped) sets. Selected = `filter-match`
/// status `matches`; skipped = `ignored: true` (an `#[ignore]`d test in a proof
/// binary is a hollowing move, tracked separately).
fn list_tests() -> Result<(BTreeSet<String>, Vec<String>)> {
    let mut cmd = Command::new("cargo");
    cmd.env("CARGO_TARGET_DIR", cargo_target_dir()?);
    cmd.args([
        "nextest",
        "list",
        "--profile",
        "ci",
        "-p",
        "batpak",
        "--features",
        &FEATURES.join(","),
        "-E",
        NEXTEST_FILTER,
        "--message-format",
        "json",
    ]);
    let output = run_output(cmd).context("fuzz-replay: `cargo nextest list` failed")?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("fuzz-replay: parse `cargo nextest list` json")?;
    parse_listed(&json)
}

/// Testable core of [`list_tests`]: walk the `rust-suites` object of a nextest
/// list JSON document and collect `<binary-name>::<testcase>` ids.
fn parse_listed(json: &serde_json::Value) -> Result<(BTreeSet<String>, Vec<String>)> {
    let suites = json
        .get("rust-suites")
        .and_then(serde_json::Value::as_object)
        .context("fuzz-replay: nextest list json missing `rust-suites` object")?;
    let mut listed = BTreeSet::new();
    let mut skipped = Vec::new();
    for suite in suites.values() {
        let binary = suite
            .get("binary-name")
            .and_then(serde_json::Value::as_str)
            .context("fuzz-replay: nextest suite missing `binary-name`")?;
        let Some(testcases) = suite.get("testcases").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (name, case) in testcases {
            let id = format!("{binary}::{name}");
            if case
                .get("ignored")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                skipped.push(id.clone());
            }
            let matches = case
                .get("filter-match")
                .and_then(|m| m.get("status"))
                .and_then(serde_json::Value::as_str)
                == Some("matches");
            if matches {
                listed.insert(id);
            }
        }
    }
    skipped.sort();
    Ok((listed, skipped))
}

/// Testable core of the gate-(a) law: every [`REQUIRED_TESTS`] entry must be in
/// the nextest-resolved set, and nothing may be skipped.
fn assert_selected(listed: &BTreeSet<String>, skipped: &[String]) -> Result<()> {
    let missing: Vec<&str> = REQUIRED_TESTS
        .iter()
        .copied()
        .filter(|t| !listed.contains(*t))
        .collect();
    if !missing.is_empty() {
        bail!(
            "fuzz-replay: nextest resolved {} test(s) but is MISSING required test(s) {missing:?} — \
             the feature-gated binary compiled out / to zero intended tests (false-green). \
             Cure: build under `--features dangerous-test-hooks`.",
            listed.len()
        );
    }
    if !skipped.is_empty() {
        bail!(
            "fuzz-replay: {} required-binary test(s) are #[ignore]d {skipped:?} — an ignored test \
             in a proof binary is a hollowing move; un-ignore them.",
            skipped.len()
        );
    }
    if listed.is_empty() {
        bail!("fuzz-replay: nextest resolved zero tests under {NEXTEST_FILTER:?}");
    }
    Ok(())
}

/// Validate a receipt destined to claim `verdict = "PASS"`. A `PASS` receipt that
/// violates any of these is a laundered proof and fails the gate.
pub(crate) fn validate_proof_receipt(r: &ProofReceipt) -> Result<()> {
    if r.verdict != "PASS" {
        bail!("fuzz-replay receipt: verdict must be \"PASS\", got {:?}", r.verdict);
    }
    if r.executed_tests == 0 {
        bail!("fuzz-replay receipt: executed_tests is 0 — no proof test ran");
    }
    if r.executed_tests != r.listed_tests {
        bail!(
            "fuzz-replay receipt: executed_tests {} != listed_tests {} — a resolved test did not run",
            r.executed_tests,
            r.listed_tests
        );
    }
    if r.skipped_tests != 0 {
        bail!(
            "fuzz-replay receipt: skipped_tests is {} — an ignored proof test hollows the gate",
            r.skipped_tests
        );
    }
    if r.features.is_empty() {
        bail!("fuzz-replay receipt: features is empty — cannot prove the binaries were not compiled out");
    }
    if r.binaries.is_empty() {
        bail!("fuzz-replay receipt: binaries is empty");
    }
    if r.commit_sha.len() != 40 || !r.commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!(
            "fuzz-replay receipt: commit_sha must be 40 hex chars, got {:?}",
            r.commit_sha
        );
    }
    if r.corpus_files == 0 {
        bail!("fuzz-replay receipt: corpus_files is 0 — the replay floor is unmet");
    }
    if r.regression_files == 0 {
        bail!("fuzz-replay receipt: regression_files is 0 — no committed regression fixtures");
    }
    for (label, hash) in [
        ("corpus_hash", &r.corpus_hash),
        ("regression_hash", &r.regression_hash),
        ("regression_manifest_hash", &r.regression_manifest_hash),
    ] {
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("fuzz-replay receipt: {label} must be 64 hex chars, got {hash:?}");
        }
    }
    Ok(())
}

/// sha256 (lower-hex) over the sorted `(repo-relative-path, bytes)` stream of a
/// directory tree, plus the file count. A missing tree is an error — the gate
/// requires a real corpus and real regressions.
fn hash_tree(dir: &Path, repo_root: &Path) -> Result<(usize, String)> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    collect_files(dir, repo_root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, bytes) in &files {
        hasher.update(rel.as_bytes());
        hasher.update([0u8]);
        hasher.update(bytes);
    }
    let digest = hasher.finalize();
    Ok((files.len(), base16ct::lower::encode_string(digest.as_ref())))
}

fn collect_files(dir: &Path, repo_root: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("file type of {}", path.display()))?;
        if file_type.is_dir() {
            collect_files(&path, repo_root, out)?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
            out.push((rel, bytes));
        }
    }
    Ok(())
}

/// sha256 (lower-hex) over a single file's bytes.
fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(base16ct::lower::encode_string(hasher.finalize().as_ref()))
}

/// Best-effort ISO-8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`), dependency-free.
/// Receipt timestamps are audit metadata, not load-bearing for any assertion.
fn iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` (public-domain): Unix day count → (y,m,d).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let d = u32::try_from(d).unwrap_or(0);
    let m = u32::try_from(m).unwrap_or(0);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::{
        assert_selected, parse_listed, validate_proof_receipt, ProofReceipt, REQUIRED_TESTS,
    };
    use std::collections::BTreeSet;

    fn all_required() -> BTreeSet<String> {
        REQUIRED_TESTS.iter().map(|s| (*s).to_owned()).collect()
    }

    fn passing_receipt() -> ProofReceipt {
        ProofReceipt {
            schema_version: 1,
            gate: "fuzz-replay".to_owned(),
            command: "cargo nextest run".to_owned(),
            features: vec!["dangerous-test-hooks".to_owned()],
            binaries: vec!["fuzz_replay".to_owned()],
            required_tests: REQUIRED_TESTS.iter().map(|s| (*s).to_owned()).collect(),
            listed_tests: 4,
            executed_tests: 4,
            skipped_tests: 0,
            corpus_files: 12,
            corpus_hash: "a".repeat(64),
            regression_files: 20,
            regression_hash: "b".repeat(64),
            regression_manifest_hash: "c".repeat(64),
            commit_sha: "0".repeat(40),
            branch: "main".to_owned(),
            started: "2026-07-13T00:00:00Z".to_owned(),
            ended: "2026-07-13T00:00:01Z".to_owned(),
            verdict: "PASS".to_owned(),
        }
    }

    #[test]
    fn missing_required_test_is_rejected() {
        let err = assert_selected(&BTreeSet::new(), &[])
            .expect_err("an empty listed set must fail: the binary compiled out");
        assert!(err.to_string().contains("compiled out"), "{err:?}");
    }

    #[test]
    fn ignored_required_test_is_rejected() {
        let skipped = vec!["fuzz_replay::fuzz_replay_covers_every_target".to_owned()];
        let err = assert_selected(&all_required(), &skipped)
            .expect_err("an ignored proof test must fail the gate");
        assert!(err.to_string().contains("ignore"), "{err:?}");
    }

    #[test]
    fn full_required_selection_passes() {
        assert_selected(&all_required(), &[]).expect("all required present, none skipped");
    }

    #[test]
    fn parse_listed_splits_matches_and_ignored() {
        let json = serde_json::json!({
            "rust-suites": {
                "batpak::fuzz_replay": {
                    "binary-name": "fuzz_replay",
                    "testcases": {
                        "fuzz_replay_covers_every_target": {
                            "ignored": false,
                            "filter-match": { "status": "matches" }
                        },
                        "an_ignored_one": {
                            "ignored": true,
                            "filter-match": { "status": "matches" }
                        }
                    }
                }
            }
        });
        let (listed, skipped) = parse_listed(&json).expect("well-formed list json parses");
        assert!(listed.contains("fuzz_replay::fuzz_replay_covers_every_target"));
        assert_eq!(skipped, vec!["fuzz_replay::an_ignored_one".to_owned()]);
    }

    #[test]
    fn receipt_with_zero_executed_is_rejected() {
        let mut r = passing_receipt();
        r.executed_tests = 0;
        r.listed_tests = 0;
        let err = validate_proof_receipt(&r).expect_err("zero executed must fail");
        assert!(err.to_string().contains("executed_tests is 0"), "{err:?}");
    }

    #[test]
    fn receipt_with_skips_is_rejected() {
        let mut r = passing_receipt();
        r.skipped_tests = 1;
        let err = validate_proof_receipt(&r).expect_err("a skipped proof test must fail");
        assert!(err.to_string().contains("skipped_tests"), "{err:?}");
    }

    #[test]
    fn receipt_without_features_is_rejected() {
        let mut r = passing_receipt();
        r.features.clear();
        let err = validate_proof_receipt(&r).expect_err("empty features must fail");
        assert!(err.to_string().contains("features"), "{err:?}");
    }

    #[test]
    fn passing_receipt_validates() {
        validate_proof_receipt(&passing_receipt()).expect("a well-formed PASS receipt validates");
    }

    #[test]
    fn receipt_json_missing_executed_count_fails_parse() {
        // Every field present EXCEPT `executed_tests`: with `deny_unknown_fields`
        // and no `Option`, a missing required field cannot parse (gate-(f)).
        let json = r#"{
            "schema_version": 1,
            "gate": "fuzz-replay",
            "command": "cargo nextest run",
            "features": ["dangerous-test-hooks"],
            "binaries": ["fuzz_replay"],
            "required_tests": [],
            "listed_tests": 4,
            "skipped_tests": 0,
            "corpus_files": 12,
            "corpus_hash": "aaaa",
            "regression_files": 20,
            "regression_hash": "bbbb",
            "regression_manifest_hash": "cccc",
            "commit_sha": "0000000000000000000000000000000000000000",
            "branch": "main",
            "started": "2026-07-13T00:00:00Z",
            "ended": "2026-07-13T00:00:01Z",
            "verdict": "PASS"
        }"#;
        let err = serde_json::from_str::<ProofReceipt>(json)
            .expect_err("a receipt omitting executed_tests must fail to parse");
        assert!(err.to_string().contains("executed_tests"), "{err:?}");
    }
}

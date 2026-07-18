#![deny(warnings)]

//! bootstrap/receiptcheck.rs — the independent Tier 0 qualification verifier
//! (5.5E6b). This binary is the AUTHORITATIVE computer of qualification
//! evidence: it links the real `spec` rlib, parses the strict
//! `qualification.t0` artifact, recomputes every digest from the bytes on disk,
//! compares them against the artifact's claims, and only then calls the typed
//! `spec::bootstrap_qualification::verify`. It is a standalone gate tool, NOT a
//! sixth Tier 0 receipt, and it never imports or shells out to selftest.py.
//!
//! The private seal on `VerifiedTier0Qualification` prevents bypass of the
//! verified shape; it does not prove the inputs were computed honestly. That
//! honest computation is exactly what this program performs.
//!
//! Modes:
//! ```text
//! receiptcheck policy
//! receiptcheck verify <qualification.t0> --root <source-root> --evidence <bundle-root>
//!     --python-executable <interpreter> [--upload-ready]
//! receiptcheck compare
//!     --candidate-artifact <t0> --candidate-evidence <bundle> --candidate-run-metadata <meta>
//!     --cleanroom-artifact <t0> --cleanroom-evidence <bundle> --cleanroom-run-metadata <meta>
//!     --root <source-root> --python-executable <interpreter> [--require-promotion-confirmation]
//! ```
//!
//! The artifact must be the bundle's own `qualification.t0`; the bundle shape is
//! exact (no unmanifested `.pdb`/scratch); the interpreter is probed at the exact
//! path (never a `python`/`python3` search) and must be CPython at the artifact's
//! release. The v2 grammar binds the builder Python runtime (5.5E6c2).
//!
//! `compare` is the wire end of the cross-run algebra (5.5E6c1): it independently
//! verifies BOTH uploaded bundles, ties each to its hosted run's own record
//! (conclusion, head SHA, repository, workflow, run id, attempt), then calls the
//! sealed `compare_runs` and `confirm_promotion`. It computes the comparison —
//! Python never parses or duplicates the comparator law.

use spec::bootstrap_qualification::{
    self as bq, BootstrapRuntimeBinding, CompilationOutcome, ExecutionAttempt, ExecutionOutcome,
    GitCommitSha, GitHubActionsRunBinding, GitTreeSha, PythonRelease, Sha256Digest, SourceBinding,
    Tier0ArtifactEvidence, Tier0QualificationObservation, Tier0ReceiptKind, Tier0ReceiptObservation,
    ToolchainBinding, ToolchainCommit, VerifiedTier0Qualification,
};
use spec::tier0_cross_run::{compare_runs, confirm_promotion, CrossRunComparison};
use spec::toolchain::{RustRelease, RustTargetTriple};

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str);
    let result = match mode {
        Some("policy") => mode_policy(),
        Some("verify") => mode_verify(&args[2..]),
        Some("compare") => mode_compare(&args[2..]),
        _ => Err(
            "usage: receiptcheck policy | receiptcheck verify <artifact> \
             --root <root> --evidence <bundle> --python-executable <py> [--upload-ready] \
             | receiptcheck compare --candidate-artifact <t0> --candidate-evidence <bundle> \
             --candidate-run-metadata <meta> --cleanroom-artifact <t0> \
             --cleanroom-evidence <bundle> --cleanroom-run-metadata <meta> \
             --root <root> --python-executable <py> [--require-promotion-confirmation]"
                .to_owned(),
        ),
    };
    if let Err(message) = result {
        eprintln!("receiptcheck: FAIL — {message}");
        std::process::exit(1);
    }
}

// ===========================================================================
// policy: project the denominator and artifact policies from the linked spec
// ===========================================================================

fn mode_policy() -> Result<(), String> {
    for &kind in Tier0ReceiptKind::ALL {
        println!("{} {}", kind.slug(), kind.artifact_policy().slug());
    }
    Ok(())
}

// ===========================================================================
// verify: parse strict artifact, recompute digests, call typed verify()
// ===========================================================================

struct VerifyPaths {
    artifact: PathBuf,
    root: PathBuf,
    evidence: PathBuf,
    /// The exact interpreter that ran the Python gates (setup-python's
    /// `python-path` / `sys.executable`) — probed, never searched for (5.5E6c2).
    python_executable: PathBuf,
    /// Whether the evidence bundle is the upload-ready hosted shape (run-metadata
    /// required) rather than the pre-upload core shape (run-metadata absent).
    upload_ready: bool,
}

fn parse_verify_args(args: &[String]) -> Result<VerifyPaths, String> {
    let artifact = args
        .first()
        .ok_or("verify requires an artifact path")?
        .clone();
    let mut root: Option<String> = None;
    let mut evidence: Option<String> = None;
    let mut python: Option<String> = None;
    let mut upload_ready = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                root = Some(args.get(i + 1).ok_or("--root requires a value")?.clone());
                i += 2;
            }
            "--evidence" => {
                evidence = Some(args.get(i + 1).ok_or("--evidence requires a value")?.clone());
                i += 2;
            }
            "--python-executable" => {
                python =
                    Some(args.get(i + 1).ok_or("--python-executable requires a value")?.clone());
                i += 2;
            }
            "--upload-ready" => {
                upload_ready = true;
                i += 1;
            }
            other => return Err(format!("unknown verify flag {other}")),
        }
    }
    Ok(VerifyPaths {
        artifact: PathBuf::from(artifact),
        root: PathBuf::from(root.ok_or("verify requires --root")?),
        evidence: PathBuf::from(evidence.ok_or("verify requires --evidence")?),
        python_executable: PathBuf::from(python.ok_or("verify requires --python-executable")?),
        upload_ready,
    })
}

/// Independently verify ONE uploaded qualification: bind the artifact to the
/// bundle, read the strict artifact, recompute and compare every digest from the
/// bytes on disk, enforce the exact bundle shape, then call the sealed
/// `bq::verify`. Shared by `verify` and both sides of `compare`.
fn verify_one(paths: &VerifyPaths) -> Result<VerifiedTier0Qualification, String> {
    // The verified artifact must be the bundle's OWN qualification.t0 — no
    // external answer sheet may be swapped for the artifact inside the bundle.
    let bundle_artifact = paths.evidence.join("qualification.t0");
    let resolved_artifact = fs::canonicalize(&paths.artifact)
        .map_err(|e| format!("cannot resolve artifact path {}: {e}", paths.artifact.display()))?;
    let resolved_bundle = fs::canonicalize(&bundle_artifact).map_err(|e| {
        format!("cannot resolve bundle qualification.t0 {}: {e}", bundle_artifact.display())
    })?;
    if resolved_artifact != resolved_bundle {
        return Err(format!(
            "artifact {} is not the bundle's own qualification.t0 ({})",
            paths.artifact.display(),
            bundle_artifact.display()
        ));
    }

    let raw = fs::read(&paths.artifact)
        .map_err(|e| format!("cannot read artifact {}: {e}", paths.artifact.display()))?;
    let lines = strict_lines(&raw)?;
    let doc = parse_artifact(&lines)?;

    // Independently recompute and compare every digest the artifact claims, then
    // enforce the exact admitted evidence-bundle shape.
    recompute_and_compare(&doc, paths)?;
    check_bundle_shape(&paths.evidence, &doc, paths.upload_ready)?;

    // Build the typed observation and call the sealed verifier.
    let observation = doc.to_observation();
    bq::verify(observation).map_err(|errors| {
        let mut lines = vec![format!("typed verification refused ({} rule(s)):", errors.len())];
        for e in errors {
            let rule = e.rule();
            lines.push(format!(
                "  - {}: {} [owner {:?}; repair: {}]",
                debug_rule(rule),
                rule.law(),
                rule.owner(),
                rule.repair()
            ));
        }
        lines.join("\n")
    })
}

fn mode_verify(args: &[String]) -> Result<(), String> {
    // The internal SHA-256 must agree with the standard before it may judge.
    self_check_sha256()?;

    let paths = parse_verify_args(args)?;
    let verified = verify_one(&paths)?;

    // Success: print the FULL binding values (the hosted run banner may
    // abbreviate afterward, but the artifact-derived truth is printed in full).
    let (commit, tree) = match verified.source() {
        SourceBinding::GitCheckout { commit, tree, .. } => (commit.render(), tree.render()),
        SourceBinding::FrozenExport { .. } => (
            "-(frozen-export: no git commit)".to_owned(),
            "-(frozen-export: no git tree)".to_owned(),
        ),
    };
    let output_tree = verified
        .receipts()
        .iter()
        .find_map(|r| match r.artifact() {
            Tier0ArtifactEvidence::ExecutableAndOutputTree {
                output_tree_digest, ..
            } => Some(output_tree_digest.render()),
            _ => None,
        })
        .unwrap_or_else(|| "-(no materializer receipt)".to_owned());
    println!("receiptcheck: PASS");
    println!("source-commit: {commit}");
    println!("git-tree: {tree}");
    println!("target: {}", verified.target().triple());
    println!("materializer-output-tree: {output_tree}");
    print!("tier0-denominator:");
    for &kind in Tier0ReceiptKind::ALL {
        print!(" {}", kind.slug());
    }
    println!();
    Ok(())
}

fn debug_rule(rule: bq::Tier0VerificationRule) -> String {
    format!("{rule:?}")
}

// ===========================================================================
// compare: independently verify both bundles, tie each to its hosted run's own
// record, then run the sealed cross-run algebra (5.5E6c1)
// ===========================================================================

struct ComparePaths {
    candidate_artifact: PathBuf,
    candidate_evidence: PathBuf,
    candidate_run_metadata: PathBuf,
    cleanroom_artifact: PathBuf,
    cleanroom_evidence: PathBuf,
    cleanroom_run_metadata: PathBuf,
    root: PathBuf,
    python_executable: PathBuf,
    require_promotion_confirmation: bool,
}

fn parse_compare_args(args: &[String]) -> Result<ComparePaths, String> {
    let mut m: BTreeMap<&str, String> = BTreeMap::new();
    let mut require = false;
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        match flag {
            "--require-promotion-confirmation" => {
                require = true;
                i += 1;
            }
            "--candidate-artifact"
            | "--candidate-evidence"
            | "--candidate-run-metadata"
            | "--cleanroom-artifact"
            | "--cleanroom-evidence"
            | "--cleanroom-run-metadata"
            | "--root"
            | "--python-executable" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone();
                if m.insert(flag, value).is_some() {
                    return Err(format!("duplicate compare flag {flag}"));
                }
                i += 2;
            }
            other => return Err(format!("unknown compare flag {other}")),
        }
    }
    let get = |key: &str| -> Result<PathBuf, String> {
        m.get(key)
            .cloned()
            .map(PathBuf::from)
            .ok_or_else(|| format!("compare requires {key}"))
    };
    Ok(ComparePaths {
        candidate_artifact: get("--candidate-artifact")?,
        candidate_evidence: get("--candidate-evidence")?,
        candidate_run_metadata: get("--candidate-run-metadata")?,
        cleanroom_artifact: get("--cleanroom-artifact")?,
        cleanroom_evidence: get("--cleanroom-evidence")?,
        cleanroom_run_metadata: get("--cleanroom-run-metadata")?,
        root: get("--root")?,
        python_executable: get("--python-executable")?,
        require_promotion_confirmation: require,
    })
}

/// A hosted run's OWN record, exported from the GitHub Actions API by the
/// confirming workflow. `compare` ties each uploaded bundle to the run that
/// produced it: a branch name never becomes source identity, and only a run
/// whose own record says `success` and whose head SHA matches the bundle's git
/// commit can stand as a promotion side.
struct RunMetadata {
    conclusion: String,
    head_sha: GitCommitSha,
    repository: String,
    workflow_path: String,
    run_id: u64,
    run_attempt: u32,
}

fn parse_run_metadata(path: &Path) -> Result<RunMetadata, String> {
    let raw = fs::read(path)
        .map_err(|e| format!("cannot read run metadata {}: {e}", path.display()))?;
    let lines = strict_lines(&raw)?;
    let mut c = Cursor::new(&lines);
    c.expect_exact("BATPAK-TIER0-RUN-METADATA/1")?;
    let conclusion = c.expect_key("conclusion")?;
    let head_sha = GitCommitSha::from_hex(&c.expect_key("head-sha")?)
        .map_err(|e| format!("bad head-sha in run metadata: {e:?}"))?;
    let repository = c.expect_key("repository")?;
    let workflow_path = c.expect_key("workflow-path")?;
    let run_id = c
        .expect_key("run-id")?
        .parse()
        .map_err(|_| "run-id is not a u64".to_owned())?;
    let run_attempt = c
        .expect_key("run-attempt")?
        .parse()
        .map_err(|_| "run-attempt is not a u32".to_owned())?;
    if !c.at_end() {
        return Err("trailing content after run metadata".to_owned());
    }
    Ok(RunMetadata {
        conclusion,
        head_sha,
        repository,
        workflow_path,
        run_id,
        run_attempt,
    })
}

/// Tie a verified qualification to its run's own record: the record must say
/// `success`, name the same repository, workflow, run id, and attempt the
/// artifact bound, and its head SHA must equal the artifact's git commit.
fn cross_check_run(
    side: &str,
    verified: &VerifiedTier0Qualification,
    meta: &RunMetadata,
) -> Result<(), String> {
    if meta.conclusion != "success" {
        return Err(format!(
            "{side} run metadata conclusion is {:?}, not success",
            meta.conclusion
        ));
    }
    let run = verified
        .hosted_run()
        .ok_or_else(|| format!("{side} qualification binds no hosted run"))?;
    if run.repository != meta.repository {
        return Err(format!(
            "{side} artifact repository {:?} does not match run metadata {:?}",
            run.repository, meta.repository
        ));
    }
    if run.workflow_path != meta.workflow_path {
        return Err(format!(
            "{side} artifact workflow path {:?} does not match run metadata {:?}",
            run.workflow_path, meta.workflow_path
        ));
    }
    if run.run_id != meta.run_id {
        return Err(format!(
            "{side} artifact run id {} does not match run metadata {}",
            run.run_id, meta.run_id
        ));
    }
    if run.run_attempt != meta.run_attempt {
        return Err(format!(
            "{side} artifact run attempt {} does not match run metadata {}",
            run.run_attempt, meta.run_attempt
        ));
    }
    match verified.source() {
        SourceBinding::GitCheckout { commit, .. } => {
            if *commit != meta.head_sha {
                return Err(format!(
                    "{side} run metadata head-sha {} does not match the artifact git commit {}",
                    meta.head_sha.render(),
                    commit.render()
                ));
            }
        }
        SourceBinding::FrozenExport { .. } => {
            return Err(format!(
                "{side} artifact is a frozen export; a promotion side must be a git checkout"
            ));
        }
    }
    Ok(())
}

fn mode_compare(args: &[String]) -> Result<(), String> {
    // The internal SHA-256 must agree with the standard before it may judge.
    self_check_sha256()?;
    let p = parse_compare_args(args)?;

    // Independently verify BOTH uploaded bundles from the bytes on disk. Both are
    // downloaded hosted bundles, so both take the upload-ready shape.
    let candidate = verify_one(&VerifyPaths {
        artifact: p.candidate_artifact.clone(),
        root: p.root.clone(),
        evidence: p.candidate_evidence.clone(),
        python_executable: p.python_executable.clone(),
        upload_ready: true,
    })
    .map_err(|e| format!("candidate: {e}"))?;
    let cleanroom = verify_one(&VerifyPaths {
        artifact: p.cleanroom_artifact.clone(),
        root: p.root.clone(),
        evidence: p.cleanroom_evidence.clone(),
        python_executable: p.python_executable.clone(),
        upload_ready: true,
    })
    .map_err(|e| format!("cleanroom: {e}"))?;

    // Tie each bundle to its hosted run's own record.
    let candidate_meta = parse_run_metadata(&p.candidate_run_metadata)?;
    let cleanroom_meta = parse_run_metadata(&p.cleanroom_run_metadata)?;
    cross_check_run("candidate", &candidate, &candidate_meta)?;
    cross_check_run("cleanroom", &cleanroom, &cleanroom_meta)?;

    // Same-source is always computed (informational); promotion confirmation is
    // the strictly stronger verdict the confirming run requires.
    let comparison = compare_runs(&candidate, &cleanroom);
    let same_source = match &comparison {
        CrossRunComparison::SameSource(_) => "same-source".to_owned(),
        CrossRunComparison::DifferentSource(divs) => {
            format!("different-source ({} divergence(s): {divs:?})", divs.len())
        }
        CrossRunComparison::NotComparable(reason) => format!("not-comparable ({reason:?})"),
    };

    if p.require_promotion_confirmation {
        let proof = confirm_promotion(&candidate, &cleanroom)
            .map_err(|e| format!("promotion confirmation refused: {e:?}"))?;
        let commit = match candidate.source() {
            SourceBinding::GitCheckout { commit, .. } => commit.render(),
            SourceBinding::FrozenExport { .. } => "-(frozen-export)".to_owned(),
        };
        println!("receiptcheck: PASS");
        println!("mode: compare");
        println!("promotion-confirmed: yes");
        println!("same-source: {same_source}");
        println!("source-commit: {commit}");
        println!("target: {}", proof.target().triple());
        println!(
            "candidate-run: {} attempt {}",
            proof.candidate_run().run_id,
            proof.candidate_run().run_attempt
        );
        println!(
            "cleanroom-run: {} attempt {}",
            proof.cleanroom_run().run_id,
            proof.cleanroom_run().run_attempt
        );
    } else {
        println!("receiptcheck: PASS");
        println!("mode: compare");
        println!("promotion-confirmed: not-required");
        println!("same-source: {same_source}");
    }
    Ok(())
}

// ===========================================================================
// Strict line reader: ASCII, LF-only, final newline, no trailing whitespace
// ===========================================================================

fn strict_lines(raw: &[u8]) -> Result<Vec<String>, String> {
    if raw.iter().any(|&b| b == b'\r') {
        return Err("artifact contains a CR byte; the format is LF-only".to_owned());
    }
    if !raw.iter().all(|&b| b < 0x80) {
        return Err("artifact contains a non-ASCII byte".to_owned());
    }
    if raw.last() != Some(&b'\n') {
        return Err("artifact has no final newline".to_owned());
    }
    let text = std::str::from_utf8(raw).map_err(|_| "artifact is not valid UTF-8".to_owned())?;
    // Split on LF; the trailing newline yields a final empty element we drop.
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let last = lines.pop();
    if last.as_deref() != Some("") {
        return Err("artifact has no final newline".to_owned());
    }
    for line in &lines {
        if line.ends_with(' ') || line.ends_with('\t') {
            return Err(format!("artifact line has trailing whitespace: {line:?}"));
        }
    }
    Ok(lines)
}

// ===========================================================================
// Parsed artifact document
// ===========================================================================

enum ParsedSource {
    GitCheckout {
        commit: GitCommitSha,
        tree: GitTreeSha,
        spec_manifest_digest: Sha256Digest,
        workflow_path: String,
        workflow_digest: Sha256Digest,
    },
    FrozenExport {
        spec_manifest_digest: Sha256Digest,
        export_tree_digest: Sha256Digest,
    },
}

struct ParsedToolchain {
    rustc_release: RustRelease,
    rustc_commit: ToolchainCommit,
    cargo_release: RustRelease,
    cargo_commit: ToolchainCommit,
    toolchain_file_digest: Sha256Digest,
}

struct ParsedHostedRun {
    repository: String,
    workflow_path: String,
    run_id: u64,
    run_attempt: u32,
    runner_image_os: String,
    runner_image_version: String,
}

struct ParsedReceipt {
    kind: Tier0ReceiptKind,
    compilation: CompilationOutcome,
    execution: ExecutionAttempt,
    outcome: Option<ExecutionOutcome>,
    evidence: Tier0ArtifactEvidence,
}

struct ArtifactDoc {
    source: ParsedSource,
    toolchain: ParsedToolchain,
    python_release: PythonRelease,
    target: RustTargetTriple,
    hosted_run: Option<ParsedHostedRun>,
    receipts: Vec<ParsedReceipt>,
}

impl ArtifactDoc {
    fn to_observation(&self) -> Tier0QualificationObservation {
        let source = match &self.source {
            ParsedSource::GitCheckout {
                commit,
                tree,
                spec_manifest_digest,
                workflow_digest,
                ..
            } => SourceBinding::GitCheckout {
                commit: *commit,
                tree: *tree,
                spec_manifest_digest: *spec_manifest_digest,
                workflow_digest: *workflow_digest,
            },
            ParsedSource::FrozenExport {
                spec_manifest_digest,
                export_tree_digest,
            } => SourceBinding::FrozenExport {
                spec_manifest_digest: *spec_manifest_digest,
                export_tree_digest: *export_tree_digest,
            },
        };
        let toolchain = ToolchainBinding {
            rustc_release: self.toolchain.rustc_release,
            rustc_commit: self.toolchain.rustc_commit,
            cargo_release: self.toolchain.cargo_release,
            cargo_commit: self.toolchain.cargo_commit,
            toolchain_file_digest: self.toolchain.toolchain_file_digest,
        };
        let hosted_run = self.hosted_run.as_ref().map(|h| GitHubActionsRunBinding {
            repository: h.repository.clone(),
            workflow_path: h.workflow_path.clone(),
            run_id: h.run_id,
            run_attempt: h.run_attempt,
            runner_image_os: h.runner_image_os.clone(),
            runner_image_version: h.runner_image_version.clone(),
        });
        let receipts = self
            .receipts
            .iter()
            .map(|r| Tier0ReceiptObservation {
                kind: r.kind,
                target: self.target,
                available: true,
                compilation: r.compilation,
                execution: r.execution,
                outcome: r.outcome,
                artifact: Some(r.evidence),
            })
            .collect();
        Tier0QualificationObservation {
            source,
            toolchain,
            bootstrap_runtime: BootstrapRuntimeBinding {
                python_release: self.python_release,
            },
            target: self.target,
            hosted_run,
            receipts,
        }
    }
}

// ===========================================================================
// Strict, order-sensitive parser
// ===========================================================================

/// A cursor over the artifact lines that consumes keys in EXACT declared order,
/// rejecting any deviation.
struct Cursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(lines: &'a [String]) -> Self {
        Cursor { lines, pos: 0 }
    }

    fn expect_exact(&mut self, literal: &str) -> Result<(), String> {
        let line = self.lines.get(self.pos).ok_or_else(|| {
            format!("artifact ended early; expected {literal:?}")
        })?;
        if line != literal {
            return Err(format!("expected line {literal:?}, found {line:?}"));
        }
        self.pos += 1;
        Ok(())
    }

    /// Consume `key: value` in exact position; return the raw value.
    fn expect_key(&mut self, key: &str) -> Result<String, String> {
        let line = self
            .lines
            .get(self.pos)
            .ok_or_else(|| format!("artifact ended early; expected key {key:?}"))?;
        let prefix = format!("{key}: ");
        let value = line
            .strip_prefix(&prefix)
            .ok_or_else(|| format!("expected key {key:?} in order, found {line:?}"))?;
        if value.is_empty() {
            return Err(format!("key {key:?} has an empty value"));
        }
        self.pos += 1;
        Ok(value.to_owned())
    }

    fn peek(&self) -> Option<&str> {
        self.lines.get(self.pos).map(String::as_str)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }
}

/// Parse a canonical `MAJOR.MINOR.PATCH` release triple. The spelling must be
/// canonical: `parse -> render -> byte-equal(input)`, so `03.012.010` and other
/// noncanonical spellings that merely parse numerically are refused (5.5E6c2).
fn canonical_triple(s: &str) -> Result<(u16, u16, u16), String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("release {s:?} is not MAJOR.MINOR.PATCH"));
    }
    let major: u16 = parts[0].parse().map_err(|_| format!("bad major in {s:?}"))?;
    let minor: u16 = parts[1].parse().map_err(|_| format!("bad minor in {s:?}"))?;
    let patch: u16 = parts[2].parse().map_err(|_| format!("bad patch in {s:?}"))?;
    if format!("{major}.{minor}.{patch}") != s {
        return Err(format!(
            "release {s:?} is not canonical (expected {major}.{minor}.{patch})"
        ));
    }
    Ok((major, minor, patch))
}

fn parse_release(s: &str) -> Result<RustRelease, String> {
    let (major, minor, patch) = canonical_triple(s)?;
    Ok(RustRelease { major, minor, patch })
}

fn parse_python_release(s: &str) -> Result<PythonRelease, String> {
    let (major, minor, patch) = canonical_triple(s)?;
    Ok(PythonRelease { major, minor, patch })
}

fn parse_target(s: &str) -> Result<RustTargetTriple, String> {
    RustTargetTriple::ALL
        .iter()
        .copied()
        .find(|t| t.triple() == s)
        .ok_or_else(|| format!("unknown target triple {s:?}"))
}

fn sha256_from_hex(s: &str) -> Result<Sha256Digest, String> {
    Sha256Digest::from_hex(s).map_err(|e| format!("bad sha256 digest {s:?}: {e:?}"))
}

fn parse_artifact(lines: &[String]) -> Result<ArtifactDoc, String> {
    let mut c = Cursor::new(lines);
    c.expect_exact("BATPAK-TIER0-QUALIFICATION/2")?;

    // Source block, dispatched on source-kind.
    let source_kind = c.expect_key("source-kind")?;
    let source = match source_kind.as_str() {
        "git-checkout" => {
            let commit = GitCommitSha::from_hex(&c.expect_key("git-commit")?)
                .map_err(|e| format!("bad git-commit: {e:?}"))?;
            let tree = GitTreeSha::from_hex(&c.expect_key("git-tree")?)
                .map_err(|e| format!("bad git-tree: {e:?}"))?;
            let spec_manifest_digest = sha256_from_hex(&c.expect_key("spec-manifest-digest")?)?;
            let workflow_path = c.expect_key("workflow-path")?;
            let workflow_digest = sha256_from_hex(&c.expect_key("workflow-digest")?)?;
            ParsedSource::GitCheckout {
                commit,
                tree,
                spec_manifest_digest,
                workflow_path,
                workflow_digest,
            }
        }
        "frozen-export" => {
            let spec_manifest_digest = sha256_from_hex(&c.expect_key("spec-manifest-digest")?)?;
            let export_tree_digest = sha256_from_hex(&c.expect_key("export-tree-digest")?)?;
            ParsedSource::FrozenExport {
                spec_manifest_digest,
                export_tree_digest,
            }
        }
        other => return Err(format!("unknown source-kind {other:?}")),
    };

    // Toolchain block.
    let rustc_release = parse_release(&c.expect_key("rustc-release")?)?;
    let rustc_commit = ToolchainCommit::from_hex(&c.expect_key("rustc-commit")?)
        .map_err(|e| format!("bad rustc-commit: {e:?}"))?;
    let cargo_release = parse_release(&c.expect_key("cargo-release")?)?;
    let cargo_commit = ToolchainCommit::from_hex(&c.expect_key("cargo-commit")?)
        .map_err(|e| format!("bad cargo-commit: {e:?}"))?;
    let toolchain_file_digest = sha256_from_hex(&c.expect_key("toolchain-file-digest")?)?;
    let toolchain = ParsedToolchain {
        rustc_release,
        rustc_commit,
        cargo_release,
        cargo_commit,
        toolchain_file_digest,
    };

    // Bootstrap runtime block (v2): the CPython release the Python gates ran under.
    let python_release = parse_python_release(&c.expect_key("python-release")?)?;

    // Target and optional hosted-run block.
    let target = parse_target(&c.expect_key("target")?)?;
    let hosted_run = if c.peek() == Some("github-repository")
        || c.peek().map(|l| l.starts_with("github-repository: ")) == Some(true)
    {
        let repository = c.expect_key("github-repository")?;
        let workflow_path = c.expect_key("github-workflow-path")?;
        let run_id = c
            .expect_key("github-run-id")?
            .parse()
            .map_err(|_| "github-run-id is not a u64".to_owned())?;
        let run_attempt = c
            .expect_key("github-run-attempt")?
            .parse()
            .map_err(|_| "github-run-attempt is not a u32".to_owned())?;
        let runner_image_os = c.expect_key("runner-image-os")?;
        let runner_image_version = c.expect_key("runner-image-version")?;
        Some(ParsedHostedRun {
            repository,
            workflow_path,
            run_id,
            run_attempt,
            runner_image_os,
            runner_image_version,
        })
    } else {
        None
    };

    // Receipts block: one line per kind, in canonical order.
    let mut receipts = Vec::new();
    while let Some(line) = c.peek() {
        if !line.starts_with("receipt: ") {
            return Err(format!("unexpected line in receipts block: {line:?}"));
        }
        receipts.push(parse_receipt_line(line)?);
        c.pos += 1;
    }
    if !c.at_end() {
        return Err("trailing content after receipts".to_owned());
    }

    Ok(ArtifactDoc {
        source,
        toolchain,
        python_release,
        target,
        hosted_run,
        receipts,
    })
}

fn parse_receipt_line(line: &str) -> Result<ParsedReceipt, String> {
    let rest = line
        .strip_prefix("receipt: ")
        .ok_or_else(|| format!("not a receipt line: {line:?}"))?;
    let tokens: Vec<&str> = rest.split(' ').collect();
    let slug = *tokens.first().ok_or("receipt line has no slug")?;
    let kind = Tier0ReceiptKind::from_slug(slug)
        .ok_or_else(|| format!("unknown receipt slug {slug:?}"))?;

    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    for tok in &tokens[1..] {
        let (k, v) = tok
            .split_once('=')
            .ok_or_else(|| format!("receipt token {tok:?} is not key=value"))?;
        if fields.insert(k, v).is_some() {
            return Err(format!("duplicate receipt token key {k:?}"));
        }
    }

    let compilation = match req(&fields, "compiled")? {
        "not-attempted" => CompilationOutcome::NotAttempted,
        "failed" => CompilationOutcome::Failed,
        "succeeded" => CompilationOutcome::Succeeded,
        other => return Err(format!("bad compiled value {other:?}")),
    };
    let execution = match req(&fields, "execution")? {
        "not-attempted" => ExecutionAttempt::NotAttempted,
        "attempted" => ExecutionAttempt::Attempted,
        other => return Err(format!("bad execution value {other:?}")),
    };
    let outcome = match req(&fields, "outcome")? {
        "none" => None,
        "failed" => Some(ExecutionOutcome::Failed),
        "passed" => Some(ExecutionOutcome::Passed),
        other => return Err(format!("bad outcome value {other:?}")),
    };
    let evidence = match req(&fields, "evidence")? {
        "fixture-set" => Tier0ArtifactEvidence::FixtureSet {
            digest: sha256_from_hex(req(&fields, "digest")?)?,
        },
        "executable" => Tier0ArtifactEvidence::Executable {
            digest: sha256_from_hex(req(&fields, "digest")?)?,
        },
        "executable+output-tree" => Tier0ArtifactEvidence::ExecutableAndOutputTree {
            executable_digest: sha256_from_hex(req(&fields, "executable-digest")?)?,
            output_tree_digest: sha256_from_hex(req(&fields, "output-tree-digest")?)?,
        },
        other => return Err(format!("bad evidence policy {other:?}")),
    };

    Ok(ParsedReceipt {
        kind,
        compilation,
        execution,
        outcome,
        evidence,
    })
}

fn req<'a>(fields: &BTreeMap<&str, &'a str>, key: &str) -> Result<&'a str, String> {
    fields
        .get(key)
        .copied()
        .ok_or_else(|| format!("receipt is missing token {key:?}"))
}

// ===========================================================================
// Independent recomputation and comparison
// ===========================================================================

fn recompute_and_compare(doc: &ArtifactDoc, paths: &VerifyPaths) -> Result<(), String> {
    // Source-block digests.
    match &doc.source {
        ParsedSource::GitCheckout {
            spec_manifest_digest,
            workflow_path,
            workflow_digest,
            ..
        } => {
            compare_file_digest(
                &paths.root.join("SPEC.sha256"),
                *spec_manifest_digest,
                "spec-manifest-digest",
            )?;
            compare_file_digest(
                &paths.root.join(workflow_path),
                *workflow_digest,
                "workflow-digest",
            )?;
        }
        ParsedSource::FrozenExport {
            spec_manifest_digest,
            export_tree_digest,
        } => {
            compare_file_digest(
                &paths.root.join("SPEC.sha256"),
                *spec_manifest_digest,
                "spec-manifest-digest",
            )?;
            let actual = tree_digest(&paths.root)?;
            if actual != *export_tree_digest {
                return Err(digest_mismatch("export-tree-digest", *export_tree_digest, actual));
            }
        }
    }

    // Toolchain digests and identities.
    compare_file_digest(
        &paths.root.join("rust-toolchain.toml"),
        doc.toolchain.toolchain_file_digest,
        "toolchain-file-digest",
    )?;
    let (rustc_release, rustc_commit) = probe_tool("rustc", &["-vV"])?;
    if rustc_release != doc.toolchain.rustc_release {
        return Err(format!(
            "rustc-release mismatch: artifact {}, actual {}",
            doc.toolchain.rustc_release.render(),
            rustc_release.render()
        ));
    }
    if rustc_commit != doc.toolchain.rustc_commit {
        return Err(format!(
            "rustc-commit mismatch: artifact {}, actual {}",
            doc.toolchain.rustc_commit.render(),
            rustc_commit.render()
        ));
    }
    let (cargo_release, cargo_commit) = probe_tool("cargo", &["-Vv"])?;
    if cargo_release != doc.toolchain.cargo_release {
        return Err(format!(
            "cargo-release mismatch: artifact {}, actual {}",
            doc.toolchain.cargo_release.render(),
            cargo_release.render()
        ));
    }
    if cargo_commit != doc.toolchain.cargo_commit {
        return Err(format!(
            "cargo-commit mismatch: artifact {}, actual {}",
            doc.toolchain.cargo_commit.render(),
            cargo_commit.render()
        ));
    }

    // Bootstrap runtime: probe the EXACT interpreter that ran the Python gates
    // (never a search) and require CPython at the artifact's claimed release.
    let (python_impl, python_release) = probe_python(&paths.python_executable)?;
    if python_impl != "cpython" {
        return Err(format!(
            "python-implementation mismatch: artifact requires cpython, actual {python_impl:?}"
        ));
    }
    if python_release != doc.python_release {
        return Err(format!(
            "python-release mismatch: artifact {}, actual {}",
            doc.python_release.render(),
            python_release.render()
        ));
    }

    // Per-receipt artifact evidence.
    for r in &doc.receipts {
        match r.evidence {
            Tier0ArtifactEvidence::FixtureSet { digest } => {
                let manifest = paths.evidence.join("law-fixtures.manifest");
                validate_fixture_manifest(&manifest)?;
                compare_file_digest(&manifest, digest, "fixture-set digest")?;
            }
            Tier0ArtifactEvidence::Executable { digest } => {
                compare_file_digest(
                    &executable_path(&paths.evidence, r.kind),
                    digest,
                    "executable digest",
                )?;
            }
            Tier0ArtifactEvidence::ExecutableAndOutputTree {
                executable_digest,
                output_tree_digest,
            } => {
                compare_file_digest(
                    &executable_path(&paths.evidence, r.kind),
                    executable_digest,
                    "executable digest",
                )?;
                let actual = tree_digest(&paths.evidence.join("gate0-candidate"))?;
                if actual != output_tree_digest {
                    return Err(digest_mismatch(
                        "materializer output-tree digest",
                        output_tree_digest,
                        actual,
                    ));
                }
            }
        }
    }

    Ok(())
}

fn executable_path(evidence: &Path, kind: Tier0ReceiptKind) -> PathBuf {
    let base = evidence.join("executables").join(kind.slug());
    // The bundle carries a bare name on unix and a `.exe` on Windows; prefer
    // whichever exists so the verifier is host-agnostic.
    let exe = base.with_extension("exe");
    if exe.is_file() { exe } else { base }
}

fn compare_file_digest(path: &Path, claimed: Sha256Digest, label: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let actual = Sha256Digest::from_bytes(sha256(&bytes));
    if actual != claimed {
        return Err(digest_mismatch(label, claimed, actual));
    }
    Ok(())
}

fn digest_mismatch(label: &str, claimed: Sha256Digest, actual: Sha256Digest) -> String {
    format!(
        "{label} mismatch: artifact claims {}, actual {}",
        claimed.render(),
        actual.render()
    )
}

/// A fixture manifest must be non-empty, carry six pipe-separated fields per
/// row, and never repeat a fixture identity. A row that is not terminal (wrong
/// field count) is a started-without-completion failure.
fn validate_fixture_manifest(path: &Path) -> Result<(), String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read fixture manifest: {e}"))?;
    let rows: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    if rows.is_empty() {
        return Err("fixture manifest is empty".to_owned());
    }
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for row in rows {
        let fields: Vec<&str> = row.split('|').map(str::trim).collect();
        if fields.len() != 6 {
            return Err(format!(
                "fixture manifest row is not a terminal six-field row: {row:?}"
            ));
        }
        let identity = fields[0].to_owned();
        if seen.insert(identity.clone(), ()).is_some() {
            return Err(format!("duplicate fixture identity {identity:?}"));
        }
    }
    Ok(())
}

/// Probe `rustc -vV` / `cargo -Vv` and parse the `release:` and `commit-hash:`
/// lines into typed values.
fn probe_tool(tool: &str, args: &[&str]) -> Result<(RustRelease, ToolchainCommit), String> {
    let out = Command::new(tool)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {tool} {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{tool} {args:?} exited nonzero"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut release: Option<RustRelease> = None;
    let mut commit: Option<ToolchainCommit> = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("release: ") {
            release = Some(parse_release(v.trim())?);
        }
        if let Some(v) = line.strip_prefix("commit-hash: ") {
            commit = ToolchainCommit::from_hex(v.trim()).ok();
        }
    }
    let release = release.ok_or_else(|| format!("{tool} reported no release"))?;
    let commit = commit.ok_or_else(|| format!("{tool} reported no usable commit-hash"))?;
    Ok((release, commit))
}

/// Probe the EXACT interpreter path (never a `python`/`python3` search): its
/// implementation name and release (5.5E6c2). receiptcheck asks the interpreter
/// selftest / setup-python selected, so the verifier and the producer agree.
fn probe_python(python: &Path) -> Result<(String, PythonRelease), String> {
    let out = Command::new(python)
        .args([
            "-c",
            "import sys; print(sys.implementation.name); \
             print(sys.version_info.major, sys.version_info.minor, sys.version_info.micro)",
        ])
        .output()
        .map_err(|e| format!("cannot run python {}: {e}", python.display()))?;
    if !out.status.success() {
        return Err(format!("python {} exited nonzero", python.display()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let implementation = lines
        .next()
        .ok_or("python reported no implementation")?
        .trim()
        .to_owned();
    let version_line = lines.next().ok_or("python reported no version")?;
    let parts: Vec<&str> = version_line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!(
            "python version line {version_line:?} is not MAJOR MINOR MICRO"
        ));
    }
    let major = parts[0].parse().map_err(|_| "bad python major".to_owned())?;
    let minor = parts[1].parse().map_err(|_| "bad python minor".to_owned())?;
    let patch = parts[2].parse().map_err(|_| "bad python micro".to_owned())?;
    Ok((implementation, PythonRelease { major, minor, patch }))
}

// ===========================================================================
// Exact evidence-bundle shape (5.5E6c2): no unmanifested material
// ===========================================================================

/// Enforce the exact admitted evidence-bundle shape. The bundle carries EXACTLY
/// the admitted files — no compiler scratch (`.pdb`/`.lib`), no unmanifested
/// material, no symlink or special file, no case-folded duplicate.
/// `run-metadata.t0` is CONDITIONAL: forbidden in the core/supplemental posture,
/// required in the upload-ready hosted posture. `gate0-candidate/` internals stay
/// bound by the materializer output-tree digest.
fn check_bundle_shape(evidence: &Path, doc: &ArtifactDoc, upload_ready: bool) -> Result<(), String> {
    let hosted =
        doc.hosted_run.is_some() && matches!(doc.source, ParsedSource::GitCheckout { .. });
    let run_metadata_required = upload_ready && hosted;
    let allowed_files = ["qualification.t0", "law-fixtures.manifest"];
    let allowed_dirs = ["executables", "gate0-candidate"];

    let mut seen_lower: BTreeMap<String, String> = BTreeMap::new();
    let mut saw_run_metadata = false;
    for entry in fs::read_dir(evidence)
        .map_err(|e| format!("cannot read evidence bundle {}: {e}", evidence.display()))?
    {
        let entry = entry.map_err(|e| format!("bundle entry error: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry
            .file_type()
            .map_err(|e| format!("cannot stat bundle entry {name}: {e}"))?;
        if ft.is_symlink() {
            return Err(format!("evidence bundle contains a symlink: {name}"));
        }
        if let Some(prev) = seen_lower.insert(name.to_lowercase(), name.clone()) {
            return Err(format!("evidence bundle case-folds {prev:?} and {name:?}"));
        }
        if ft.is_dir() {
            if !allowed_dirs.contains(&name.as_str()) {
                return Err(format!("unadmitted directory in evidence bundle: {name}"));
            }
        } else if ft.is_file() {
            if name == "run-metadata.t0" {
                saw_run_metadata = true;
                if !run_metadata_required {
                    return Err(
                        "a core/supplemental evidence bundle must not carry run-metadata.t0"
                            .to_owned(),
                    );
                }
            } else if !allowed_files.contains(&name.as_str()) {
                return Err(format!("unmanifested file in evidence bundle: {name}"));
            }
        } else {
            return Err(format!("evidence bundle entry {name} is neither file nor dir"));
        }
    }
    if run_metadata_required && !saw_run_metadata {
        return Err("upload-ready hosted bundle is missing run-metadata.t0".to_owned());
    }

    // executables/ holds exactly one host-suffixed exe per non-fixture-set kind
    // (derived from Tier0ReceiptKind::ALL, never a hardcoded list).
    let exe_dir = evidence.join("executables");
    let admitted_exes: Vec<&str> = Tier0ReceiptKind::ALL
        .iter()
        .filter(|k| k.artifact_policy() != bq::Tier0ArtifactPolicy::FixtureSet)
        .map(|k| k.slug())
        .collect();
    let mut seen_exe_lower: BTreeMap<String, String> = BTreeMap::new();
    for entry in fs::read_dir(&exe_dir)
        .map_err(|e| format!("cannot read executables/ {}: {e}", exe_dir.display()))?
    {
        let entry = entry.map_err(|e| format!("executables/ entry error: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry
            .file_type()
            .map_err(|e| format!("cannot stat executables/ entry {name}: {e}"))?;
        if ft.is_symlink() {
            return Err(format!("executables/ contains a symlink: {name}"));
        }
        if !ft.is_file() {
            return Err(format!("executables/ contains a non-file entry: {name}"));
        }
        if let Some(prev) = seen_exe_lower.insert(name.to_lowercase(), name.clone()) {
            return Err(format!("executables/ case-folds {prev:?} and {name:?}"));
        }
        // Accept `<slug>` or `<slug>.exe`; refuse a stray `.pdb`/`.lib`, an extra
        // exe, or any other scratch output.
        let stem = name.strip_suffix(".exe").unwrap_or(&name);
        if !admitted_exes.contains(&stem) {
            return Err(format!("unmanifested file in executables/: {name}"));
        }
    }
    for slug in &admitted_exes {
        let base = exe_dir.join(slug);
        if !base.is_file() && !base.with_extension("exe").is_file() {
            return Err(format!("executables/ is missing the {slug} gate binary"));
        }
    }
    Ok(())
}

// ===========================================================================
// Deterministic tree digest (sorted paths, entry types, content hashes)
// ===========================================================================

fn tree_digest(root: &Path) -> Result<Sha256Digest, String> {
    let mut rows: Vec<String> = Vec::new();
    collect_tree(root, root, &mut rows)?;
    rows.sort();
    let mut joined = String::new();
    for row in rows {
        joined.push_str(&row);
        joined.push('\n');
    }
    Ok(Sha256Digest::from_bytes(sha256(joined.as_bytes())))
}

fn collect_tree(base: &Path, dir: &Path, rows: &mut Vec<String>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("cannot read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|_| "path escaped tree base".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry
            .file_type()
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if file_type.is_dir() {
            rows.push(format!("{rel}\tdir\t-"));
            collect_tree(base, &path, rows)?;
        } else if file_type.is_file() {
            let bytes =
                fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let content = render_hex(&sha256(&bytes));
            rows.push(format!("{rel}\tfile\t{content}"));
        } else {
            return Err(format!("tree entry {} is neither file nor dir", path.display()));
        }
    }
    Ok(())
}

fn render_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

// ===========================================================================
// SHA-256 (pure std), self-checked against standard vectors before use
// ===========================================================================

fn self_check_sha256() -> Result<(), String> {
    let empty = render_hex(&sha256(b""));
    if empty != "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" {
        return Err("internal SHA-256 failed the empty-string vector".to_owned());
    }
    let abc = render_hex(&sha256(b"abc"));
    if abc != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err("internal SHA-256 failed the \"abc\" vector".to_owned());
    }
    Ok(())
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

use spec::bootstrap_qualification::{self as bq, GitCommitSha, SourceBinding, Tier0ArtifactEvidence, Tier0ReceiptKind, VerifiedTier0Qualification};
use spec::tier0_cross_run::{compare_runs, confirm_promotion, CrossRunComparison};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::artifact::{parse_artifact, strict_lines, Cursor};
use crate::bundle::check_bundle_shape;
use crate::digests::recompute_and_compare;
use crate::hashing::self_check_sha256;

pub(crate) fn mode_policy() -> Result<(), String> {
    for &kind in Tier0ReceiptKind::ALL {
        println!("{} {}", kind.slug(), kind.artifact_policy().slug());
    }
    Ok(())
}

// ===========================================================================
// verify: parse strict artifact, recompute digests, call typed verify()
// ===========================================================================

pub(crate) struct VerifyPaths {
    artifact: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) evidence: PathBuf,
    /// The exact interpreter that ran the Python gates (setup-python's
    /// `python-path` / `sys.executable`) — probed, never searched for (5.5E6c2).
    pub(crate) python_executable: PathBuf,
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

pub(crate) fn mode_verify(args: &[String]) -> Result<(), String> {
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

pub(crate) fn mode_compare(args: &[String]) -> Result<(), String> {
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


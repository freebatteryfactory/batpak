use spec::bootstrap_qualification::{BootstrapRuntimeBinding, CompilationOutcome, ExecutionAttempt, ExecutionOutcome, GitCommitSha, GitHubActionsRunBinding, GitTreeSha, PythonRelease, Sha256Digest, SourceBinding, Tier0ArtifactEvidence, Tier0QualificationObservation, Tier0ReceiptKind, Tier0ReceiptObservation, ToolchainBinding, ToolchainCommit};
use spec::toolchain::{RustRelease, RustTargetTriple};
use std::collections::BTreeMap;

pub(crate) fn strict_lines(raw: &[u8]) -> Result<Vec<String>, String> {
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

pub(crate) enum ParsedSource {
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

pub(crate) struct ParsedToolchain {
    pub(crate) rustc_release: RustRelease,
    pub(crate) rustc_commit: ToolchainCommit,
    pub(crate) cargo_release: RustRelease,
    pub(crate) cargo_commit: ToolchainCommit,
    pub(crate) toolchain_file_digest: Sha256Digest,
}

pub(crate) struct ParsedHostedRun {
    repository: String,
    workflow_path: String,
    run_id: u64,
    run_attempt: u32,
    runner_image_os: String,
    runner_image_version: String,
}

pub(crate) struct ParsedReceipt {
    pub(crate) kind: Tier0ReceiptKind,
    compilation: CompilationOutcome,
    execution: ExecutionAttempt,
    outcome: Option<ExecutionOutcome>,
    pub(crate) evidence: Tier0ArtifactEvidence,
}

pub(crate) struct ArtifactDoc {
    pub(crate) source: ParsedSource,
    pub(crate) toolchain: ParsedToolchain,
    pub(crate) python_release: PythonRelease,
    target: RustTargetTriple,
    pub(crate) hosted_run: Option<ParsedHostedRun>,
    pub(crate) receipts: Vec<ParsedReceipt>,
}

impl ArtifactDoc {
    pub(crate) fn to_observation(&self) -> Tier0QualificationObservation {
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
pub(crate) struct Cursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(lines: &'a [String]) -> Self {
        Cursor { lines, pos: 0 }
    }

    pub(crate) fn expect_exact(&mut self, literal: &str) -> Result<(), String> {
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
    pub(crate) fn expect_key(&mut self, key: &str) -> Result<String, String> {
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

    pub(crate) fn at_end(&self) -> bool {
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

pub(crate) fn parse_release(s: &str) -> Result<RustRelease, String> {
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

pub(crate) fn parse_artifact(lines: &[String]) -> Result<ArtifactDoc, String> {
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


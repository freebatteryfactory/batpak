//! `receiptcheck e7-verify` — the independent verifier of the
//! BATPAK-E7-UNDERWRITING/1 opening-matrix artifact (E7-F).
//!
//! The artifact is a strict line-oriented document (the `qualification.t0`
//! idiom): magic, twelve binding lines, exactly twenty `zero <token> <n>`
//! rows in the frozen TL-5 order, the `architect-semantic-row
//! reserved-to-architect` line, and `end`. This mode recomputes every
//! binding from the bytes on disk, refuses any nonzero row by name,
//! RE-EXECUTES the campaign verification core in-process (the same
//! `campaign-verify` law, so the campaign-derived rows are re-proven, never
//! trusted — its PASS banner prints before this mode's own), and recomputes
//! row 20 from the nursery receipts (TL-6: escalation receipts counted
//! against the single expected rehearsal-fixture escalation, identified by
//! the one LawChanging candidate manifest).
//!
//! Documented digest bases, byte-identical with the Python producer:
//! * `source-tree`: for each SPEC.sha256 row the file's content hash is
//!   recomputed from disk and rendered back as `<sha256>  <relpath>` in
//!   manifest order; the digest is the SHA-256 of that text.
//! * `package-graph`: SHA-256 over `<relpath>\t<sha256>` rows (sorted by
//!   relpath, LF-terminated) of every non-root Cargo.toml inside the bound
//!   Tier 0 bundle's `gate0-candidate` tree; the set must equal the
//!   compiled `spec::architecture::PACKAGES` workspace paths exactly.
//! * `toolchain` / `python` are cross-checked against the bound campaign
//!   bundle's toolchain section and the bound Tier 0 artifact's
//!   `python-release` line — the carrier idioms already in evidence.
//!
//! The final semantic row is the ARCHITECT's: this verifier proves the
//! mechanical rows only and never announces semantic completeness.

use crate::artifact::{parse_release, strict_lines};
use crate::hashing::{sha256, tree_digest};
use spec::architecture::PACKAGES;
use spec::bootstrap_qualification::Sha256Digest;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const E7_MAGIC: &str = "BATPAK-E7-UNDERWRITING/1";
const ARCHITECT_LINE: &str = "architect-semantic-row reserved-to-architect";

/// TL-5: the architect's twenty matrix rows, kebab-cased, frozen order.
const ZERO_TOKENS: [&str; 20] = [
    "active-proof-rows-without-claim-or-plan",
    "adopter-less-verification-variants",
    "variants-lacking-fresh-executed-adopter-evidence",
    "generated-models-claiming-independence",
    "unbounded-candidate-searches",
    "incomplete-candidate-lineage",
    "unresolved-evidence-references",
    "search-holdout-overlaps",
    "runtime-monitors-with-rewrite-authority",
    "lawchanging-candidates-in-preserving-lanes",
    "qualified-nursery-records-treated-as-promoted",
    "external-tools-acting-as-semantic-owners",
    "python-dynamic-semantics-maps",
    "unregistered-generated-views",
    "release-seal-omissions",
    "package-topology-changes",
    "unclassified-dec-seed-proof-rows",
    "scaffolds-counted-realized",
    "later-green-above-unresolved-red",
    "unresolved-architect-required-findings",
];

struct E7Args {
    artifact: PathBuf,
    root: PathBuf,
    tier0: PathBuf,
    campaign_bundle: PathBuf,
    judge_root: PathBuf,
    envelope: PathBuf,
    source_commit: String,
    nursery_root: PathBuf,
    evidence_root: PathBuf,
}

fn parse_args(args: &[String]) -> Result<E7Args, String> {
    let artifact = args
        .first()
        .ok_or("e7-verify requires an artifact path")?
        .clone();
    let mut m: std::collections::BTreeMap<&str, String> = std::collections::BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            flag @ ("--root" | "--tier0-bundle" | "--campaign-bundle" | "--judge-root"
            | "--envelope" | "--source-commit" | "--nursery-root" | "--evidence-root") => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone();
                if m.insert(flag, value).is_some() {
                    return Err(format!("duplicate e7-verify flag {flag}"));
                }
                i += 2;
            }
            other => return Err(format!("unknown e7-verify flag {other}")),
        }
    }
    let get = |key: &str| -> Result<String, String> {
        m.get(key)
            .cloned()
            .ok_or_else(|| format!("e7-verify requires {key}"))
    };
    Ok(E7Args {
        artifact: PathBuf::from(artifact),
        root: PathBuf::from(get("--root")?),
        tier0: PathBuf::from(get("--tier0-bundle")?),
        campaign_bundle: PathBuf::from(get("--campaign-bundle")?),
        judge_root: PathBuf::from(get("--judge-root")?),
        envelope: PathBuf::from(get("--envelope")?),
        source_commit: get("--source-commit")?,
        nursery_root: PathBuf::from(get("--nursery-root")?),
        evidence_root: PathBuf::from(get("--evidence-root")?),
    })
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256Digest::from_bytes(sha256(bytes)).render()
}

fn require_hex64(value: &str, what: &str) -> Result<(), String> {
    Sha256Digest::from_hex(value)
        .map(|_| ())
        .map_err(|e| format!("e7: bad {what} {value:?}: {e:?}"))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("e7: cannot read {}: {e}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|_| format!("e7: {} is not UTF-8", path.display()))
}

fn compare_binding(claimed: &str, actual: &str, what: &str) -> Result<(), String> {
    if claimed != actual {
        return Err(format!(
            "e7: {what} mismatch: artifact claims {claimed}, recomputed {actual}"
        ));
    }
    Ok(())
}

/// A strict cursor over `key value` lines in EXACT declared order (the
/// artifact.rs idiom, adapted to the space-separated E7 grammar).
struct E7Cursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> E7Cursor<'a> {
    fn expect_exact(&mut self, literal: &str) -> Result<(), String> {
        let line = self
            .lines
            .get(self.pos)
            .ok_or_else(|| format!("e7: artifact ended early; expected {literal:?}"))?;
        if line != literal {
            return Err(format!("e7: expected line {literal:?}, found {line:?}"));
        }
        self.pos += 1;
        Ok(())
    }

    fn take(&mut self, key: &str) -> Result<String, String> {
        let line = self
            .lines
            .get(self.pos)
            .ok_or_else(|| format!("e7: artifact ended early; expected key {key:?}"))?;
        let value = line
            .strip_prefix(&format!("{key} "))
            .ok_or_else(|| format!("e7: expected key {key:?} in order, found {line:?}"))?;
        if value.is_empty() {
            return Err(format!("e7: key {key:?} has an empty value"));
        }
        self.pos += 1;
        Ok(value.to_owned())
    }

    fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }
}

/// Collect every non-root Cargo.toml under the gate0-candidate tree as
/// (forward-slash relpath, content sha256) — the package-graph basis.
fn collect_manifests(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("e7: cannot read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("e7: dir entry error: {e}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("e7: cannot stat {}: {e}", path.display()))?;
        if file_type.is_dir() {
            collect_manifests(base, &path, out)?;
        } else if file_type.is_file() && entry.file_name() == "Cargo.toml" {
            let rel = path
                .strip_prefix(base)
                .map_err(|_| "e7: path escaped the gate0-candidate tree".to_owned())?
                .to_string_lossy()
                .replace('\\', "/");
            if rel != "Cargo.toml" {
                out.push((rel, sha_hex(&read_bytes(&path)?)));
            }
        }
    }
    Ok(())
}

/// Row 20 recompute (TL-6) from the nursery bytes alone: LawChanging
/// candidate manifests and escalation receipts, counted against the single
/// expected rehearsal fixture.
fn recompute_architect_row(nursery_root: &Path) -> Result<u64, String> {
    let mut law_changing: Vec<String> = Vec::new();
    let mut escalations: Vec<String> = Vec::new();
    let entries = fs::read_dir(nursery_root)
        .map_err(|e| format!("e7: cannot read nursery {}: {e}", nursery_root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("e7: nursery entry error: {e}"))?;
        if !entry
            .file_type()
            .map_err(|e| format!("e7: cannot stat nursery entry: {e}"))?
            .is_dir()
        {
            return Err(format!(
                "e7: nursery carries a non-directory entry {:?}",
                entry.file_name()
            ));
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let manifest = entry.path().join("manifest");
        if manifest.is_file()
            && read_text(&manifest)?
                .lines()
                .any(|line| line == "change-class LawChanging")
        {
            law_changing.push(id.clone());
        }
        let receipts_dir = entry.path().join("receipts");
        if !receipts_dir.is_dir() {
            continue;
        }
        let receipts = fs::read_dir(&receipts_dir)
            .map_err(|e| format!("e7: cannot read {}: {e}", receipts_dir.display()))?;
        for receipt in receipts {
            let receipt = receipt.map_err(|e| format!("e7: receipt entry error: {e}"))?;
            if read_text(&receipt.path())?
                .lines()
                .any(|line| line == "kind escalation")
            {
                escalations.push(id.clone());
            }
        }
    }
    let planted = if law_changing.len() == 1 {
        Some(law_changing[0].as_str())
    } else {
        None
    };
    let mut violations = law_changing.len().saturating_sub(1) as u64;
    violations += escalations
        .iter()
        .filter(|owner| Some(owner.as_str()) != planted)
        .count() as u64;
    Ok(violations)
}

pub(crate) fn mode_e7_verify(args: &[String]) -> Result<(), String> {
    let a = parse_args(args)?;
    let raw = read_bytes(&a.artifact)?;
    let artifact_digest = sha_hex(&raw);
    let lines = strict_lines(&raw)?;

    // Strict, order-sensitive parse of the frozen grammar.
    let mut c = E7Cursor { lines: &lines, pos: 0 };
    c.expect_exact(E7_MAGIC)?;
    let source_commit = c.take("source-commit")?;
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(format!(
            "e7: source-commit {source_commit:?} is not 40 lowercase hex"
        ));
    }
    if source_commit != a.source_commit {
        return Err(format!(
            "e7: artifact source-commit {source_commit} does not match this \
             checkout's {}",
            a.source_commit
        ));
    }
    let source_tree = c.take("source-tree")?;
    require_hex64(&source_tree, "source-tree")?;
    let spec_manifest = c.take("spec-manifest")?;
    require_hex64(&spec_manifest, "spec-manifest")?;
    let workflow_digest = c.take("workflow-digest")?;
    require_hex64(&workflow_digest, "workflow-digest")?;
    let toolchain = c.take("toolchain")?;
    let mut toolchain_tokens = toolchain.split(' ');
    parse_release(toolchain_tokens.next().unwrap_or(""))
        .map_err(|e| format!("e7: toolchain line carries no rustc release: {e}"))?;
    if toolchain_tokens.next().is_none() {
        return Err("e7: toolchain line carries no host triple".to_owned());
    }
    let python = c.take("python")?;
    parse_release(&python).map_err(|e| format!("e7: bad python release: {e}"))?;
    let tier0_bundle = c.take("tier0-bundle")?;
    require_hex64(&tier0_bundle, "tier0-bundle")?;
    let campaign_bundle = c.take("campaign-bundle")?;
    require_hex64(&campaign_bundle, "campaign-bundle")?;
    let materializer = c.take("materializer-output-tree")?;
    require_hex64(&materializer, "materializer-output-tree")?;
    let package_graph = c.take("package-graph")?;
    require_hex64(&package_graph, "package-graph")?;
    let registry = c.take("generated-view-registry")?;
    require_hex64(&registry, "generated-view-registry")?;
    let release_envelope = c.take("release-envelope")?;
    require_hex64(&release_envelope, "release-envelope")?;
    let mut zeros: Vec<(String, u64)> = Vec::new();
    for expected in ZERO_TOKENS {
        let value = c.take("zero")?;
        let (token, count) = value
            .split_once(' ')
            .ok_or_else(|| format!("e7: zero row {value:?} is not `<token> <n>`"))?;
        if token != expected {
            return Err(format!(
                "e7: zero row {token:?} out of order; the frozen TL-5 order \
                 expects {expected:?}"
            ));
        }
        let count: u64 = count
            .parse()
            .map_err(|_| format!("e7: zero row {token} count {count:?} is not a u64"))?;
        zeros.push((token.to_owned(), count));
    }
    c.expect_exact(ARCHITECT_LINE)?;
    c.expect_exact("end")?;
    if !c.at_end() {
        return Err("e7: trailing content after end".to_owned());
    }

    // Bindings, each recomputed from the bytes on disk.
    let spec_manifest_bytes = read_bytes(&a.root.join("SPEC.sha256"))?;
    compare_binding(&spec_manifest, &sha_hex(&spec_manifest_bytes), "spec-manifest")?;
    compare_binding(
        &workflow_digest,
        &sha_hex(&read_bytes(
            &a.root.join(".github/workflows/msvc-qualification.yml"),
        )?),
        "workflow-digest",
    )?;
    compare_binding(
        &registry,
        &sha_hex(&read_bytes(&a.root.join("spec/generated_views/registry.rs"))?),
        "generated-view-registry",
    )?;
    let manifest_text = String::from_utf8(spec_manifest_bytes)
        .map_err(|_| "e7: SPEC.sha256 is not UTF-8".to_owned())?;
    let mut tree_rows = String::new();
    for line in manifest_text.lines() {
        let (_claimed, rel) = line
            .split_once("  ")
            .ok_or_else(|| format!("e7: SPEC.sha256 row {line:?} is not `<hex>  <rel>`"))?;
        tree_rows.push_str(&sha_hex(&read_bytes(&a.root.join(rel))?));
        tree_rows.push_str("  ");
        tree_rows.push_str(rel);
        tree_rows.push('\n');
    }
    compare_binding(&source_tree, &sha_hex(tree_rows.as_bytes()), "source-tree")?;
    compare_binding(
        &tier0_bundle,
        &tree_digest(&a.tier0)?.render(),
        "tier0-bundle tree digest",
    )?;
    let bundle_bytes = read_bytes(&a.campaign_bundle)?;
    compare_binding(&campaign_bundle, &sha_hex(&bundle_bytes), "campaign-bundle")?;
    let bundle_text = String::from_utf8(bundle_bytes)
        .map_err(|_| "e7: campaign bundle is not UTF-8".to_owned())?;
    let rustc_release = bundle_text
        .lines()
        .find_map(|line| line.strip_prefix("rustc-release "))
        .ok_or("e7: the campaign bundle binds no rustc-release")?;
    let bundle_target = bundle_text
        .lines()
        .find_map(|line| line.strip_prefix("target "))
        .ok_or("e7: the campaign bundle binds no target")?;
    compare_binding(
        &toolchain,
        &format!("{rustc_release} {bundle_target}"),
        "toolchain (campaign bundle carrier)",
    )?;
    let qual_text = read_text(&a.tier0.join("qualification.t0"))?;
    let python_release = qual_text
        .lines()
        .find_map(|line| line.strip_prefix("python-release: "))
        .ok_or("e7: the bound Tier 0 artifact binds no python-release")?;
    compare_binding(&python, python_release, "python (tier0 artifact carrier)")?;
    let gate0 = a.tier0.join("gate0-candidate");
    compare_binding(
        &materializer,
        &tree_digest(&gate0)?.render(),
        "materializer-output-tree",
    )?;
    let mut manifests: Vec<(String, String)> = Vec::new();
    collect_manifests(&gate0, &gate0, &mut manifests)?;
    let expected: BTreeSet<String> = PACKAGES
        .iter()
        .map(|p| format!("{}/Cargo.toml", p.id.workspace_path()))
        .collect();
    let got: BTreeSet<String> = manifests.iter().map(|(rel, _)| rel.clone()).collect();
    if got != expected {
        return Err(format!(
            "e7: the gate0-candidate package manifests {got:?} do not equal the \
             typed spec::architecture::PACKAGES workspace census {expected:?}"
        ));
    }
    manifests.sort();
    let mut graph_rows = String::new();
    for (rel, digest) in &manifests {
        graph_rows.push_str(rel);
        graph_rows.push('\t');
        graph_rows.push_str(digest);
        graph_rows.push('\n');
    }
    compare_binding(&package_graph, &sha_hex(graph_rows.as_bytes()), "package-graph")?;
    compare_binding(
        &release_envelope,
        &sha_hex(&read_bytes(&a.envelope)?),
        "release-envelope",
    )?;

    // The opening matrix: every row must be zero, refused by name.
    for (token, count) in &zeros {
        if *count != 0 {
            return Err(format!(
                "e7: zero row {token} is {count}, not zero -- the opening matrix \
                 is not zero"
            ));
        }
    }

    // Re-execute the campaign verification core in-process: the same
    // campaign-verify law this crate owns, over the six bound inputs, so the
    // campaign-derived rows are independently re-proven, never trusted.
    let campaign_args: Vec<String> = vec![
        a.campaign_bundle.display().to_string(),
        "--judge-root".to_owned(),
        a.judge_root.display().to_string(),
        "--envelope".to_owned(),
        a.envelope.display().to_string(),
        "--source-commit".to_owned(),
        a.source_commit.clone(),
        "--nursery-root".to_owned(),
        a.nursery_root.display().to_string(),
        "--evidence-root".to_owned(),
        a.evidence_root.display().to_string(),
    ];
    crate::campaign::mode_campaign_verify(&campaign_args)
        .map_err(|e| format!("e7: independent campaign re-verification refused: {e}"))?;

    // Row 20 (TL-6), recomputed from the nursery receipts alone.
    let recomputed = recompute_architect_row(&a.nursery_root)?;
    let claimed = zeros[19].1;
    if recomputed != claimed {
        return Err(format!(
            "e7: zero row {} recomputes {recomputed} from the nursery receipts, \
             but the artifact claims {claimed}",
            ZERO_TOKENS[19]
        ));
    }

    println!("receiptcheck: PASS");
    println!("mode: e7-verify");
    println!("grammar: {E7_MAGIC}");
    println!("artifact: {artifact_digest}");
    println!("e7: PASS");
    Ok(())
}

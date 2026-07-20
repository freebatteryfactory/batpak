//! `receiptcheck e7-verify` / `receiptcheck e7-open` — the independent
//! verifier and opening-eligibility printer for the
//! BATPAK-E7-UNDERWRITING/2 opening-matrix artifact (E7 closeout, CL-8).
//!
//! The artifact is a strict line-oriented document (the `qualification.t0`
//! idiom): magic, twelve binding lines, exactly twenty
//! `zero <token> <n> owner <owner> receipt <64hex>` rows in the frozen TL-5
//! order, the `cross-run-stability pending` line, the `architect-semantic-row
//! reserved-to-architect` line, and `end`. `e7-verify` recomputes every
//! binding from the bytes on disk, refuses any nonzero row by name, VERIFIES
//! each row's INDEPENDENTLY PRODUCED owner receipt (no longer accepting a
//! literal zero without a receipt-backed owner), RE-EXECUTES the campaign
//! verification core in-process (the same `campaign-verify` law — its PASS
//! banner prints before this mode's own), and recomputes row 20 from the
//! nursery receipts (TL-6).
//!
//! Owner receipts (CL-8): each row names its enforcing owner. A `tier0` row
//! is proved by the bound Tier 0 bundle — its receipt is the sha256 of that
//! bundle's `qualification.t0`, recomputed here. An `audit` / `project-check`
//! row is proved by a BATPAK-OWNER-RECEIPT/1 in the artifact's `receipts`
//! dir (derived as `<artifact-dir>/receipts`); the receipt is content-
//! addressed (its filename is the sha256 of its bytes), binds this checkout's
//! source-commit, states `outcome pass`, and its `tool` digest is RECOMPUTED
//! from the owner package bytes under `--root`. A `campaign-verify` row's
//! receipt binds the built receiptcheck exe (a per-run digest this verifier
//! does NOT recompute); its assurance is the in-process campaign re-run.
//!
//! `e7-open` is the SOLE printer of `phase6-opening-eligible: yes`. Given the
//! cross-run stability receipt and the two runs' artifacts, it recomputes the
//! receipt's four digests against the actual files, independently re-runs the
//! authoritative comparison (bindings minus the two per-run keys + every zero
//! row's (token,count) RESULT + `cross-run-stability pending` in BOTH),
//! requires every zero row 0 with owner+receipt present in both, and only
//! then prints the banner. Any failure is a named finding with no banner.
//!
//! The final semantic row is the ARCHITECT's: these modes prove the
//! mechanical rows only and never announce semantic completeness.

use crate::artifact::{parse_release, strict_lines};
use crate::hashing::{sha256, tree_digest};
use spec::architecture::PACKAGES;
use spec::bootstrap_qualification::Sha256Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const E7_MAGIC: &str = "BATPAK-E7-UNDERWRITING/2";
const ARCHITECT_LINE: &str = "architect-semantic-row reserved-to-architect";
const CROSSRUN_PENDING: &str = "cross-run-stability pending";
const OWNER_RECEIPT_MAGIC: &str = "BATPAK-OWNER-RECEIPT/1";
const STABILITY_MAGIC: &str = "BATPAK-CROSSRUN-STABILITY-RECEIPT/1";

/// The twelve binding keys, in wire order.
const BIND_KEYS: [&str; 12] = [
    "source-commit", "source-tree", "spec-manifest", "workflow-digest",
    "toolchain", "python", "tier0-bundle", "campaign-bundle",
    "materializer-output-tree", "package-graph", "generated-view-registry",
    "release-envelope",
];

/// The two PER-RUN binding keys excluded from cross-run authoritative equality.
const PER_RUN_KEYS: [&str; 2] = ["tier0-bundle", "campaign-bundle"];

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

/// CL-8: the enforcing owner backing each row's zero, in frozen TL-5 order.
const ROW_OWNERS: [&str; 20] = [
    "tier0", "tier0", "campaign-verify", "audit", "campaign-verify",
    "campaign-verify", "campaign-verify", "campaign-verify", "tier0",
    "campaign-verify", "campaign-verify", "audit", "tier0", "project-check",
    "campaign-verify", "audit", "audit", "campaign-verify", "campaign-verify",
    "campaign-verify",
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

fn compare_binding(
    claimed: impl AsRef<str>,
    actual: impl AsRef<str>,
    what: &str,
) -> Result<(), String> {
    let (claimed, actual) = (claimed.as_ref(), actual.as_ref());
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

/// The parsed /2 artifact: the twelve bindings by key and the twenty rows.
struct ParsedE7 {
    bindings: BTreeMap<String, String>,
    rows: Vec<(String, u64, String, String)>,
}

/// Strict, order-sensitive parse of the frozen BATPAK-E7-UNDERWRITING/2
/// grammar, shared by `e7-verify` and `e7-open`. Enforces the twelve binding
/// keys in order, the twenty `zero <token> <n> owner <owner> receipt <64hex>`
/// rows (owner MUST equal the census map for that row), the
/// `cross-run-stability pending` line (a non-`pending` line is a time-travel
/// refusal), then the architect line and `end`.
fn parse_e7_artifact(lines: &[String]) -> Result<ParsedE7, String> {
    let mut c = E7Cursor { lines, pos: 0 };
    c.expect_exact(E7_MAGIC)?;
    let mut bindings: BTreeMap<String, String> = BTreeMap::new();
    for key in BIND_KEYS {
        let value = c.take(key)?;
        bindings.insert(key.to_owned(), value);
    }
    let mut rows: Vec<(String, u64, String, String)> = Vec::new();
    for (token, expected_owner) in ZERO_TOKENS.iter().zip(ROW_OWNERS.iter()) {
        let value = c.take("zero")?;
        let parts: Vec<&str> = value.split(' ').collect();
        if parts.len() != 6
            || parts[0] != *token
            || parts[2] != "owner"
            || parts[3] != *expected_owner
            || parts[4] != "receipt"
        {
            return Err(format!(
                "e7: zero row {value:?} breaks the frozen `zero {token} <n> \
                 owner {expected_owner} receipt <64hex>` grammar"
            ));
        }
        let count: u64 = parts[1]
            .parse()
            .map_err(|_| format!("e7: zero row {token} count {:?} is not a u64", parts[1]))?;
        require_hex64(parts[5], "zero row receipt")?;
        rows.push((token.to_string(), count, parts[3].to_owned(), parts[5].to_owned()));
    }
    c.expect_exact(CROSSRUN_PENDING).map_err(|_| {
        format!("e7: the cross-run-stability line is not {CROSSRUN_PENDING:?}; a \
                 single run never claims cross-run stability (no time travel)")
    })?;
    c.expect_exact(ARCHITECT_LINE)?;
    c.expect_exact("end")?;
    if !c.at_end() {
        return Err("e7: trailing content after end".to_owned());
    }
    Ok(ParsedE7 { bindings, rows })
}

/// The owner package file set whose bytes define an audit / project owner
/// tool digest: the shim plus every non-`__pycache__` file under its package.
fn collect_owner_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("e7: cannot read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("e7: dir entry error: {e}"))?;
        let path = entry.path();
        let ft = entry
            .file_type()
            .map_err(|e| format!("e7: cannot stat {}: {e}", path.display()))?;
        if ft.is_dir() {
            if entry.file_name() == "__pycache__" {
                continue;
            }
            collect_owner_files(&path, out)?;
        } else if ft.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Recompute the `tool` digest an audit / project owner receipt must bind:
/// sha256 over the sorted `<rel>\t<sha256>` rows of the owner package bytes.
fn owner_tool_digest(root: &Path, owner: &str) -> Result<String, String> {
    let (shim, pkg) = match owner {
        "audit" => ("bootstrap/audit.py", "bootstrap/audit"),
        "project-check" => ("bootstrap/project.py", "bootstrap/project"),
        _ => return Err(format!("e7: owner {owner:?} has no recomputable tool digest")),
    };
    let mut files = vec![root.join(shim)];
    collect_owner_files(&root.join(pkg), &mut files)?;
    let mut rows: Vec<String> = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .map_err(|_| "e7: owner package file escaped the root".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        rows.push(format!("{rel}\t{}", sha_hex(&read_bytes(f)?)));
    }
    rows.sort();
    let mut text = String::new();
    for r in &rows {
        text.push_str(r);
        text.push('\n');
    }
    Ok(sha_hex(text.as_bytes()))
}

/// Verify one INDEPENDENTLY PRODUCED BATPAK-OWNER-RECEIPT/1 backing an
/// audit / project / campaign-verify row: it resolves by digest in the
/// artifact's receipts dir, is content-addressed (its filename equals the
/// sha256 of its bytes), declares the row's owner, binds this checkout's
/// source-commit, states `outcome pass`, and — for audit/project — its `tool`
/// recomputes from the owner package bytes under `--root`. A campaign-verify
/// receipt's `tool` is the per-run receiptcheck exe, NOT recomputed here; its
/// assurance is the in-process campaign re-run below.
fn verify_owner_receipt(
    receipts_dir: &Path,
    owner: &str,
    receipt: &str,
    root: &Path,
    source_commit: &str,
) -> Result<(), String> {
    let path = receipts_dir.join(format!("{receipt}.receipt"));
    let raw = fs::read(&path).map_err(|e| {
        format!("e7: {owner} owner receipt {receipt} is unresolvable at {}: {e}", path.display())
    })?;
    if sha_hex(&raw) != receipt {
        return Err(format!(
            "e7: owner receipt at {receipt}.receipt is not content-addressed; its \
             bytes hash to {}",
            sha_hex(&raw)
        ));
    }
    let lines = strict_lines(&raw)?;
    let mut c = E7Cursor { lines: &lines, pos: 0 };
    c.expect_exact(OWNER_RECEIPT_MAGIC)?;
    let got_owner = c.take("owner")?;
    if got_owner != owner {
        return Err(format!(
            "e7: owner receipt {receipt} declares owner {got_owner:?}, the row claims {owner:?}"
        ));
    }
    let sc = c.take("source-commit")?;
    if sc != source_commit {
        return Err(format!(
            "e7: owner receipt {receipt} binds source-commit {sc}, not this checkout's {source_commit}"
        ));
    }
    let tool = c.take("tool")?;
    require_hex64(&tool, "owner receipt tool")?;
    c.expect_exact("outcome pass")?;
    let output = c.take("output")?;
    require_hex64(&output, "owner receipt output")?;
    c.expect_exact("end")?;
    if !c.at_end() {
        return Err(format!("e7: owner receipt {receipt} has trailing content after end"));
    }
    if owner == "audit" || owner == "project-check" {
        let recomputed = owner_tool_digest(root, owner)?;
        if recomputed != tool {
            return Err(format!(
                "e7: owner receipt {receipt} tool digest {tool} does not match the \
                 {owner} package bytes under --root (recomputed {recomputed})"
            ));
        }
    }
    Ok(())
}

pub(crate) fn mode_e7_verify(args: &[String]) -> Result<(), String> {
    let a = parse_args(args)?;
    let raw = read_bytes(&a.artifact)?;
    let artifact_digest = sha_hex(&raw);
    let lines = strict_lines(&raw)?;

    // Strict, order-sensitive parse of the frozen grammar.
    let parsed = parse_e7_artifact(&lines)?;
    let source_commit = parsed.bindings["source-commit"].clone();
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
    let source_tree = &parsed.bindings["source-tree"];
    require_hex64(source_tree, "source-tree")?;
    let spec_manifest = &parsed.bindings["spec-manifest"];
    require_hex64(spec_manifest, "spec-manifest")?;
    let workflow_digest = &parsed.bindings["workflow-digest"];
    require_hex64(workflow_digest, "workflow-digest")?;
    let toolchain = &parsed.bindings["toolchain"];
    let mut toolchain_tokens = toolchain.split(' ');
    parse_release(toolchain_tokens.next().unwrap_or(""))
        .map_err(|e| format!("e7: toolchain line carries no rustc release: {e}"))?;
    if toolchain_tokens.next().is_none() {
        return Err("e7: toolchain line carries no host triple".to_owned());
    }
    let python = &parsed.bindings["python"];
    parse_release(python).map_err(|e| format!("e7: bad python release: {e}"))?;
    let tier0_bundle = &parsed.bindings["tier0-bundle"];
    require_hex64(tier0_bundle, "tier0-bundle")?;
    let campaign_bundle = &parsed.bindings["campaign-bundle"];
    require_hex64(campaign_bundle, "campaign-bundle")?;
    let materializer = &parsed.bindings["materializer-output-tree"];
    require_hex64(materializer, "materializer-output-tree")?;
    let package_graph = &parsed.bindings["package-graph"];
    require_hex64(package_graph, "package-graph")?;
    let registry = &parsed.bindings["generated-view-registry"];
    require_hex64(registry, "generated-view-registry")?;
    let release_envelope = &parsed.bindings["release-envelope"];
    require_hex64(release_envelope, "release-envelope")?;

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

    // The opening matrix: every row must be zero AND backed by its
    // independently produced owner receipt -- a literal zero without a
    // receipt-backed owner is no longer accepted (CL-8). The receipts dir is
    // derived as `<artifact-dir>/receipts`.
    let receipts_dir = a
        .artifact
        .parent()
        .ok_or("e7: the artifact path has no parent dir for its receipts")?
        .join("receipts");
    let tier0_qual_digest = sha_hex(&read_bytes(&a.tier0.join("qualification.t0"))?);
    for (i, (token, count, owner, receipt)) in parsed.rows.iter().enumerate() {
        if *count != 0 {
            return Err(format!(
                "e7: zero row {token} is {count}, not zero -- the opening matrix \
                 is not zero"
            ));
        }
        if owner != ROW_OWNERS[i] {
            return Err(format!(
                "e7: zero row {token} names owner {owner}, the census fixes {}",
                ROW_OWNERS[i]
            ));
        }
        match owner.as_str() {
            "tier0" => {
                if *receipt != tier0_qual_digest {
                    return Err(format!(
                        "e7: row {token} tier0 receipt {receipt} does not equal the \
                         sha256 of the bound Tier 0 bundle's qualification.t0 \
                         ({tier0_qual_digest})"
                    ));
                }
            }
            "audit" | "project-check" | "campaign-verify" => {
                verify_owner_receipt(&receipts_dir, owner, receipt, &a.root, &a.source_commit)?;
            }
            other => return Err(format!("e7: row {token} names unknown owner {other:?}")),
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
    let claimed = parsed.rows[19].1;
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

struct E7OpenArgs {
    stability: PathBuf,
    own: PathBuf,
    candidate: PathBuf,
    source_commit: String,
}

fn parse_open_args(args: &[String]) -> Result<E7OpenArgs, String> {
    let stability = args
        .first()
        .ok_or("e7-open requires a stability-receipt path")?
        .clone();
    let mut m: BTreeMap<&str, String> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            flag @ ("--own-artifact" | "--candidate-artifact" | "--source-commit") => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone();
                if m.insert(flag, value).is_some() {
                    return Err(format!("duplicate e7-open flag {flag}"));
                }
                i += 2;
            }
            other => return Err(format!("unknown e7-open flag {other}")),
        }
    }
    let get = |key: &str| -> Result<String, String> {
        m.get(key).cloned().ok_or_else(|| format!("e7-open requires {key}"))
    };
    Ok(E7OpenArgs {
        stability: PathBuf::from(stability),
        own: PathBuf::from(get("--own-artifact")?),
        candidate: PathBuf::from(get("--candidate-artifact")?),
        source_commit: get("--source-commit")?,
    })
}

/// A parsed BATPAK-CROSSRUN-STABILITY-RECEIPT/1.
struct StabilityReceipt {
    source_commit: String,
    own_e7_artifact: String,
    candidate_e7_artifact: String,
    own_campaign_bundle: String,
    candidate_campaign_bundle: String,
}

fn parse_stability_receipt(lines: &[String]) -> Result<StabilityReceipt, String> {
    let mut c = E7Cursor { lines, pos: 0 };
    c.expect_exact(STABILITY_MAGIC)?;
    let source_commit = c.take("source-commit")?;
    let own_e7_artifact = c.take("own-e7-artifact")?;
    let candidate_e7_artifact = c.take("candidate-e7-artifact")?;
    let own_campaign_bundle = c.take("own-campaign-bundle")?;
    let candidate_campaign_bundle = c.take("candidate-campaign-bundle")?;
    for hex in [
        &own_e7_artifact,
        &candidate_e7_artifact,
        &own_campaign_bundle,
        &candidate_campaign_bundle,
    ] {
        require_hex64(hex, "stability receipt digest")?;
    }
    c.expect_exact("campaign-authoritative identical")?;
    c.expect_exact("e7-authoritative identical")?;
    c.expect_exact("end")?;
    if !c.at_end() {
        return Err("e7: stability receipt has trailing content after end".to_owned());
    }
    Ok(StabilityReceipt {
        source_commit,
        own_e7_artifact,
        candidate_e7_artifact,
        own_campaign_bundle,
        candidate_campaign_bundle,
    })
}

/// The SOLE printer of `phase6-opening-eligible: yes`. Given the cross-run
/// stability receipt and the two runs' /2 artifacts, recompute the receipt's
/// four digests against the actual files, independently re-run the
/// authoritative comparison in Rust (bindings minus the per-run keys + every
/// zero row's (token,count) RESULT + `cross-run-stability pending` in BOTH),
/// require every zero row 0 with owner+receipt present in both, and only then
/// print the banner. Any failure is a named finding with no banner.
pub(crate) fn mode_e7_open(args: &[String]) -> Result<(), String> {
    let a = parse_open_args(args)?;

    let receipt_raw = read_bytes(&a.stability)?;
    let receipt_lines = strict_lines(&receipt_raw)?;
    let receipt = parse_stability_receipt(&receipt_lines)?;
    if receipt.source_commit != a.source_commit {
        return Err(format!(
            "e7-open: stability receipt binds source-commit {}, not the passed {}",
            receipt.source_commit, a.source_commit
        ));
    }

    let own_raw = read_bytes(&a.own)?;
    let candidate_raw = read_bytes(&a.candidate)?;
    let own = parse_e7_artifact(&strict_lines(&own_raw)?)?;
    let candidate = parse_e7_artifact(&strict_lines(&candidate_raw)?)?;

    // Both artifacts bind the passed source-commit.
    for (parsed, label) in [(&own, "own"), (&candidate, "candidate")] {
        if parsed.bindings["source-commit"] != a.source_commit {
            return Err(format!(
                "e7-open: the {label} artifact binds source-commit {}, not the passed {}",
                parsed.bindings["source-commit"], a.source_commit
            ));
        }
    }

    // The receipt's four digests, recomputed against the actual files: the two
    // e7-artifact digests directly, the two campaign-bundle digests from each
    // artifact's own (already receiptcheck-verified) `campaign-bundle` binding.
    compare_binding(&receipt.own_e7_artifact, sha_hex(&own_raw), "stability own-e7-artifact")?;
    compare_binding(
        &receipt.candidate_e7_artifact,
        sha_hex(&candidate_raw),
        "stability candidate-e7-artifact",
    )?;
    compare_binding(
        &receipt.own_campaign_bundle,
        &own.bindings["campaign-bundle"],
        "stability own-campaign-bundle vs own artifact binding",
    )?;
    compare_binding(
        &receipt.candidate_campaign_bundle,
        &candidate.bindings["campaign-bundle"],
        "stability candidate-campaign-bundle vs candidate artifact binding",
    )?;

    // Independently re-run the authoritative comparison: the bindings minus the
    // two per-run keys, and every zero row's (token, count) RESULT. Owner and
    // receipt are per-run provenance (the campaign-verify receipt binds the
    // per-run receiptcheck exe), verified WITHIN each run by e7-verify.
    for key in BIND_KEYS {
        if PER_RUN_KEYS.contains(&key) {
            continue;
        }
        if own.bindings[key] != candidate.bindings[key] {
            return Err(format!(
                "e7-open: authoritative binding {key} diverges across runs: {} vs {}",
                own.bindings[key], candidate.bindings[key]
            ));
        }
    }
    for ((token, count_a, _oa, _ra), (_t, count_b, _ob, _rb)) in
        own.rows.iter().zip(candidate.rows.iter())
    {
        if count_a != count_b {
            return Err(format!(
                "e7-open: authoritative zero row {token} diverges across runs: {count_a} vs {count_b}"
            ));
        }
    }

    // Every zero row 0 with owner+receipt present (well-formed by parse) in
    // BOTH artifacts; the `cross-run-stability pending` line is asserted by the
    // parse of each (no time travel).
    for parsed in [&own, &candidate] {
        for (token, count, _owner, _receipt) in &parsed.rows {
            if *count != 0 {
                return Err(format!(
                    "e7-open: zero row {token} is {count}, not zero -- the opening \
                     matrix is not zero"
                ));
            }
        }
    }

    println!("receiptcheck: PASS");
    println!("mode: e7-open");
    println!("grammar: {STABILITY_MAGIC}");
    println!("own-artifact: {}", sha_hex(&own_raw));
    println!("candidate-artifact: {}", sha_hex(&candidate_raw));
    println!("phase6-opening-eligible: yes");
    Ok(())
}

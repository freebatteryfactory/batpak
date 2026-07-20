//! `receiptcheck campaign-verify` — the independent verifier of the campaign
//! evidence perimeter (E7, R4: independent recompute is this binary's
//! charter; the Rust witness stays scoped to the semantic relation).
//!
//! The mode dispatches on the bundle's own first line. A
//! `BATPAK-CAMPAIGN-EVIDENCE/1` bundle routes to the retained historical arm
//! (`campaign_v1.rs`, TL-7) under the original four inputs; a
//! `BATPAK-CAMPAIGN-EVIDENCE/2` bundle is verified HERE and requires the full
//! six-flag perimeter: `--judge-root --envelope --source-commit
//! --nursery-root --evidence-root`.
//!
//! From the bytes on disk alone the V2 arm validates the V1-era laws that
//! remain lawful (the recomputed frozen-judge tree digest, the source-commit
//! binding, search-budget and fuzz policy, evaluation-set disjointness
//! recompute, disposition completeness with the activated-and-killed mutant,
//! the 20-field release envelope walk, and the ArchitectRequired presence —
//! now read from receipts) PLUS the E7-C exact perimeter (14 rows): the
//! bundle<->nursery bijection; nursery-manifest byte identity with the
//! bundle-embedded manifests; `candidate-id-preimage/2` and
//! `reuse-key-preimage/2` recompute from the parsed manifests alone; evidence
//! and receipt reference resolution to exactly one regular file whose bytes
//! hash to the address; receipt grammar, kind, candidate, judge, and
//! source-frontier binding; perimeter hygiene (no symlink, special file,
//! case-fold twin, compiler scratch, or unreferenced authority-looking file);
//! typed receipts for every terminal event; reuse and invalidation edge
//! resolution; parent/dependency read-back from manifests; nursery
//! byte-immutability across this pass; evidence-root append-only posture; and
//! the four-state frontier recompute against the bundle's own frontier
//! section — with the thirteen ruled coherence refusals (TL-4) each a named
//! finding.
//!
//! Every refusal is a named finding; the mode fails closed.

use crate::artifact::strict_lines;
use crate::hashing::{sha256, tree_digest};
use spec::bootstrap_qualification::Sha256Digest;
use spec::campaign::{
    CampaignClosureEdgeKind, CampaignReceiptKind, CampaignTerminal, EvidenceFreshness,
    FrontierState, MINI_SUPERNOVA_PROFILE,
};
use spec::proof::{ProofRowState, PROOF_ROWS};
use spec::release::{EmptySetPosture, RELEASE_SEAL_FIELDS};
use spec::sprouting::{
    CANDIDATE_CHANGE_CLASSES, CANDIDATE_ORIGIN_KINDS, REALIZATION_POSTURES,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_MAGIC: &str = "BATPAK-CAMPAIGN-EVIDENCE/2";
const BUNDLE_MAGIC_V1: &str = "BATPAK-CAMPAIGN-EVIDENCE/1";
const ENVELOPE_MAGIC: &str = "BATPAK-CAMPAIGN-ENVELOPE/1";
const MANIFEST_MAGIC: &str = "BATPAK-CANDIDATE-MANIFEST/2";
const RECEIPT_MAGIC: &str = "BATPAK-CAMPAIGN-RECEIPT/2";
const ID_PREIMAGE_DOMAIN: &str = "candidate-id-preimage/2";
const REUSE_PREIMAGE_DOMAIN: &str = "reuse-key-preimage/2";
/// The bundle's own resting place inside the evidence root (TL-9); it is the
/// one non-content-addressed file the flat CAS lawfully carries.
const BUNDLE_BASENAME: &str = "campaign-evidence.bundle";
/// Compiler-scratch extensions no evidence perimeter may carry (E7-C row 7).
const SCRATCH_EXTENSIONS: [&str; 6] = ["pdb", "exp", "lib", "d", "o", "rlib"];
const SECTIONS: [&str; 9] = [
    "judge", "source", "toolchain", "policy", "manifests", "candidates",
    "dispositions", "frontier", "closure",
];

struct CampaignArgs {
    bundle: PathBuf,
    judge_root: PathBuf,
    envelope: PathBuf,
    source_commit: String,
    nursery_root: Option<PathBuf>,
    evidence_root: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<CampaignArgs, String> {
    let bundle = args
        .first()
        .ok_or("campaign-verify requires a bundle path")?
        .clone();
    let mut m: BTreeMap<&str, String> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            flag @ ("--judge-root" | "--envelope" | "--source-commit" | "--nursery-root"
            | "--evidence-root") => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?
                    .clone();
                if m.insert(flag, value).is_some() {
                    return Err(format!("duplicate campaign-verify flag {flag}"));
                }
                i += 2;
            }
            other => return Err(format!("unknown campaign-verify flag {other}")),
        }
    }
    let get = |key: &str| -> Result<String, String> {
        m.get(key)
            .cloned()
            .ok_or_else(|| format!("campaign-verify requires {key}"))
    };
    Ok(CampaignArgs {
        bundle: PathBuf::from(bundle),
        judge_root: PathBuf::from(get("--judge-root")?),
        envelope: PathBuf::from(get("--envelope")?),
        source_commit: get("--source-commit")?,
        nursery_root: m.get("--nursery-root").map(PathBuf::from),
        evidence_root: m.get("--evidence-root").map(PathBuf::from),
    })
}

/// The mode entry: read the bundle once and dispatch on its own version line.
pub(crate) fn mode_campaign_verify(args: &[String]) -> Result<(), String> {
    let a = parse_args(args)?;
    let raw = fs::read(&a.bundle)
        .map_err(|e| format!("campaign: cannot read bundle {}: {e}", a.bundle.display()))?;
    let lines = strict_lines(&raw)?;
    match lines.first().map(String::as_str) {
        Some(BUNDLE_MAGIC_V1) => {
            if a.nursery_root.is_some() || a.evidence_root.is_some() {
                return Err(
                    "campaign: --nursery-root/--evidence-root name a V2 perimeter, but this \
                     bundle is BATPAK-CAMPAIGN-EVIDENCE/1 -- historical F5 evidence has no \
                     nursery or evidence perimeter to verify; drop the V2 flags"
                        .to_owned(),
                );
            }
            crate::campaign_v1::mode_campaign_verify_historical(
                &a.bundle,
                &a.judge_root,
                &a.envelope,
                &a.source_commit,
            )
        }
        Some(BUNDLE_MAGIC) => {
            let nursery_root = a
                .nursery_root
                .ok_or("campaign: a BATPAK-CAMPAIGN-EVIDENCE/2 bundle requires --nursery-root")?;
            let evidence_root = a
                .evidence_root
                .ok_or("campaign: a BATPAK-CAMPAIGN-EVIDENCE/2 bundle requires --evidence-root")?;
            verify_v2(
                &lines,
                &a.bundle,
                &a.judge_root,
                &a.envelope,
                &a.source_commit,
                &nursery_root,
                &evidence_root,
            )
        }
        other => Err(format!(
            "campaign: bundle magic is neither {BUNDLE_MAGIC:?} nor {BUNDLE_MAGIC_V1:?} \
             (found {other:?})"
        )),
    }
}

// ===========================================================================
// Bundle sections (shared shape with V1: nine sections, judge binding)
// ===========================================================================

struct BundleDoc {
    sections: BTreeMap<String, Vec<String>>,
    judge_root_digest: String,
}

fn split_sections(lines: &[String]) -> Result<BundleDoc, String> {
    let mut sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    for line in &lines[1..] {
        if let Some(name) = line.strip_prefix("section: ") {
            if sections.contains_key(name) {
                return Err(format!("campaign: duplicate section {name:?}"));
            }
            order.push(name.to_owned());
            sections.insert(name.to_owned(), Vec::new());
            current = Some(name.to_owned());
        } else {
            let name = current
                .as_ref()
                .ok_or_else(|| format!("campaign: line before any section: {line:?}"))?;
            sections
                .get_mut(name)
                .expect("section vector exists for the current section name")
                .push(line.clone());
        }
    }
    let expected: Vec<String> = SECTIONS.iter().map(|s| (*s).to_owned()).collect();
    if order != expected {
        return Err(format!(
            "campaign: section order {order:?} is not the canonical {SECTIONS:?}"
        ));
    }
    let judge = &sections["judge"];
    let judge_root_digest = expect_kv(judge, 0, "judge-root-digest")?;
    let after = expect_kv(judge, 1, "judge-root-digest-after")?;
    if judge.len() != 2 {
        return Err("campaign: judge section carries unexpected lines".to_owned());
    }
    hex64(&judge_root_digest, "judge-root-digest")?;
    hex64(&after, "judge-root-digest-after")?;
    if judge_root_digest != after {
        return Err(
            "campaign: judge-root digest changed across the campaign; the evidence \
             is invalidated"
                .to_owned(),
        );
    }
    Ok(BundleDoc {
        sections,
        judge_root_digest,
    })
}

fn expect_kv(section: &[String], index: usize, key: &str) -> Result<String, String> {
    let line = section
        .get(index)
        .ok_or_else(|| format!("campaign: expected {key:?} line"))?;
    line.strip_prefix(&format!("{key} "))
        .map(str::to_owned)
        .ok_or_else(|| format!("campaign: expected {key:?}, found {line:?}"))
}

fn hex64(value: &str, what: &str) -> Result<Sha256Digest, String> {
    Sha256Digest::from_hex(value).map_err(|e| format!("campaign: bad {what} {value:?}: {e:?}"))
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256Digest::from_bytes(sha256(bytes)).render()
}

fn token_in(value: &str, inventory: &[String], axis: &str) -> Result<(), String> {
    if inventory.iter().any(|t| t == value) {
        return Ok(());
    }
    Err(format!("manifest {axis} token {value:?} is not in the typed inventory"))
}

fn debug_names<T: core::fmt::Debug>(inventory: &[T]) -> Vec<String> {
    inventory.iter().map(|v| format!("{v:?}")).collect()
}

// ===========================================================================
// The V2 candidate manifest: strict parse, lex enforcement, identity recompute
// ===========================================================================

/// One parsed V2 candidate manifest: mint-time facts ONLY (the immutable
/// definition carries no evidence, receipt, or terminal line). The full line
/// vector is retained because the identity preimage is those exact lines.
struct ParsedCandidate {
    id: String,
    manifest_lines: Vec<String>,
    proof_targets: Vec<String>,
    parents: Vec<String>,
    dependency_lines: Vec<String>,
    dependencies: Vec<String>,
    content_commitment: String,
    change_class: String,
    posture: String,
    reuse_key: String,
}

struct ManifestCursor<'a> {
    lines: &'a [String],
    pos: usize,
}

impl<'a> ManifestCursor<'a> {
    fn take(&mut self, key: &str) -> Result<String, String> {
        let line = self
            .lines
            .get(self.pos)
            .ok_or_else(|| format!("manifest ended early; expected {key:?}"))?;
        self.pos += 1;
        line.strip_prefix(&format!("{key} "))
            .map(str::to_owned)
            .ok_or_else(|| format!("manifest expected {key:?}, found {line:?}"))
    }

    fn take_count(&mut self, key: &str) -> Result<usize, String> {
        self.take(key)?
            .parse()
            .map_err(|_| format!("manifest {key:?} is not a count"))
    }

    fn take_repeated(&mut self, count_key: &str, key: &str) -> Result<Vec<String>, String> {
        let count = self.take_count(count_key)?;
        let mut out = Vec::new();
        for _ in 0..count {
            out.push(self.take(key)?);
        }
        Ok(out)
    }
}

/// Canonical order is MANDATORY in the wire grammar: every repeated section
/// is strictly ascending, which also refuses duplicates.
fn require_lex_order(values: &[String], what: &str) -> Result<(), String> {
    for pair in values.windows(2) {
        if pair[1] <= pair[0] {
            return Err(format!(
                "manifest {what} lines are out of canonical lexicographic order \
                 ({:?} follows {:?})",
                pair[1], pair[0]
            ));
        }
    }
    Ok(())
}

/// Resolve one named proof target against the spec proof-row catalog: the
/// row must EXIST and be Active — a retired or unknown name binds no living
/// obligation and admits nothing (TL-1).
fn require_active_proof_row(name: &str) -> Result<(), String> {
    for record in PROOF_ROWS {
        if record.id.raw() == name {
            return match record.state {
                ProofRowState::Active { .. } => Ok(()),
                ProofRowState::Retired { .. } => Err(format!(
                    "manifest proof-target {name:?} is a RETIRED proof row; its \
                     successors own the obligation and a candidate must name them"
                )),
            };
        }
    }
    Err(format!(
        "manifest proof-target {name:?} names no spec::proof row; a diagnostic \
         label is not authority"
    ))
}

fn parse_manifest(lines: &[String]) -> Result<ParsedCandidate, String> {
    if lines.first().map(String::as_str) != Some(MANIFEST_MAGIC) {
        return Err(format!(
            "embedded manifest magic is not {MANIFEST_MAGIC:?} (found {:?})",
            lines.first()
        ));
    }
    let mut c = ManifestCursor {
        lines,
        pos: 1,
    };
    let id = c.take("candidate-id")?;
    hex64(&id, "candidate-id")?;
    let proof_targets = c.take_repeated("proof-target-count", "proof-target")?;
    if proof_targets.is_empty() {
        return Err(format!(
            "manifest for {id} names no proof target; an empty set binds no \
             semantic purpose and admits nothing"
        ));
    }
    require_lex_order(&proof_targets, "proof-target")?;
    for target in &proof_targets {
        require_active_proof_row(target)?;
    }
    let parents = c.take_repeated("parent-count", "parent")?;
    for parent in &parents {
        hex64(parent, "parent")?;
    }
    require_lex_order(&parents, "parent")?;
    hex64(&c.take("source-frontier-commitment")?, "source-frontier-commitment")?;
    let dependency_lines = c.take_repeated("dependency-count", "dependency")?;
    let mut dependencies = Vec::new();
    for dep in &dependency_lines {
        let (dep_id, commitment) = dep
            .split_once(' ')
            .ok_or_else(|| format!("manifest dependency {dep:?} is not two addresses"))?;
        hex64(dep_id, "dependency candidate")?;
        hex64(commitment, "dependency commitment")?;
        dependencies.push(dep_id.to_owned());
    }
    require_lex_order(&dependency_lines, "dependency")?;
    let content_commitment = c.take("content-commitment")?;
    hex64(&content_commitment, "content-commitment")?;
    token_in(&c.take("origin")?, &debug_names(CANDIDATE_ORIGIN_KINDS), "origin")?;
    let change_class = c.take("change-class")?;
    token_in(&change_class, &debug_names(CANDIDATE_CHANGE_CLASSES), "change-class")?;
    let posture = c.take("realization-posture")?;
    token_in(&posture, &debug_names(REALIZATION_POSTURES), "realization-posture")?;
    let reuse_key = c.take("reuse-key")?;
    hex64(&reuse_key, "reuse-key")?;
    if c.pos != lines.len() {
        return Err(format!("manifest for {id} carries trailing lines"));
    }
    Ok(ParsedCandidate {
        id,
        manifest_lines: lines.to_vec(),
        proof_targets,
        parents,
        dependency_lines,
        dependencies,
        content_commitment,
        change_class,
        posture,
        reuse_key,
    })
}

/// E7-C row 3, identity half: `candidate-id-preimage/2` is the domain line
/// plus every manifest line AFTER the candidate-id line, in manifest order,
/// each LF-terminated. The id must recompute from the parsed manifest alone.
fn recompute_candidate_id(manifest_lines: &[String]) -> String {
    let mut preimage = String::new();
    preimage.push_str(ID_PREIMAGE_DOMAIN);
    preimage.push('\n');
    for line in &manifest_lines[2..] {
        preimage.push_str(line);
        preimage.push('\n');
    }
    sha_hex(preimage.as_bytes())
}

/// E7-C row 3, reuse half: `reuse-key-preimage/2` binds the proof-target
/// lines (lex), the content-commitment line, the dependency lines (lex), and
/// the compiled profile line — `profile <id> <64hex>` where the hex is the
/// SHA-256 of the profile's realized row names LF-joined with a trailing LF.
/// Rust consumes the COMPILED `spec::campaign::MINI_SUPERNOVA_PROFILE`; the
/// producer parses the same const independently, and no parser is shared.
fn recompute_reuse_key(candidate: &ParsedCandidate) -> String {
    let mut names = String::new();
    for row in MINI_SUPERNOVA_PROFILE.realized_rows {
        names.push_str(row.raw());
        names.push('\n');
    }
    let profile_digest = sha_hex(names.as_bytes());
    let mut preimage = String::new();
    preimage.push_str(REUSE_PREIMAGE_DOMAIN);
    preimage.push('\n');
    for target in &candidate.proof_targets {
        preimage.push_str("proof-target ");
        preimage.push_str(target);
        preimage.push('\n');
    }
    preimage.push_str("content-commitment ");
    preimage.push_str(&candidate.content_commitment);
    preimage.push('\n');
    for dep in &candidate.dependency_lines {
        preimage.push_str("dependency ");
        preimage.push_str(dep);
        preimage.push('\n');
    }
    preimage.push_str("profile ");
    preimage.push_str(MINI_SUPERNOVA_PROFILE.id);
    preimage.push(' ');
    preimage.push_str(&profile_digest);
    preimage.push('\n');
    sha_hex(preimage.as_bytes())
}

fn parse_candidates(section: &[String]) -> Result<Vec<ParsedCandidate>, String> {
    let declared: usize = expect_kv(section, 0, "candidate-count")?
        .parse()
        .map_err(|_| "campaign: candidate-count is not a count".to_owned())?;
    let mut out = Vec::new();
    let mut i = 1;
    while i < section.len() {
        let begin = section[i]
            .strip_prefix("candidate-begin ")
            .ok_or_else(|| format!("campaign: expected candidate-begin, found {:?}", section[i]))?;
        let begin_id = begin.split(' ').next().unwrap_or("");
        let mut body: Vec<String> = Vec::new();
        i += 1;
        loop {
            let line = section
                .get(i)
                .ok_or_else(|| format!("campaign: candidate {begin_id} has no candidate-end"))?;
            if let Some(end_id) = line.strip_prefix("candidate-end ") {
                if end_id != begin_id {
                    return Err(format!(
                        "campaign: candidate-end {end_id} does not close candidate-begin {begin_id}"
                    ));
                }
                i += 1;
                break;
            }
            body.push(line.clone());
            i += 1;
        }
        let parsed = parse_manifest(&body)?;
        if parsed.id != begin_id {
            return Err(format!(
                "campaign: embedded manifest id {} does not match its block id {begin_id}",
                parsed.id
            ));
        }
        // E7-C row 3: both content addresses must recompute exactly from the
        // parsed manifest and the normative preimage grammars.
        let recomputed_id = recompute_candidate_id(&parsed.manifest_lines);
        if recomputed_id != parsed.id {
            return Err(format!(
                "campaign: candidate {} does not recompute from candidate-id-preimage/2 \
                 (recomputed {recomputed_id}); the identity-bearing fields do not \
                 produce the claimed id",
                parsed.id
            ));
        }
        let recomputed_reuse = recompute_reuse_key(&parsed);
        if recomputed_reuse != parsed.reuse_key {
            return Err(format!(
                "campaign: reuse-key for candidate {} does not recompute from \
                 reuse-key-preimage/2 (claimed {}, recomputed {recomputed_reuse})",
                parsed.id, parsed.reuse_key
            ));
        }
        out.push(parsed);
    }
    if out.len() != declared {
        return Err(format!(
            "campaign: candidate-count {declared} does not match {} embedded manifests",
            out.len()
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for candidate in &out {
        if !seen.insert(candidate.id.as_str()) {
            return Err(format!("campaign: duplicate candidate {}", candidate.id));
        }
    }
    // The lineage closure is complete: every dependency names a candidate in
    // THIS bundle (E7-C row 11; an unresolved edge is incomplete lineage).
    let ids: BTreeSet<&str> = out.iter().map(|c| c.id.as_str()).collect();
    for candidate in &out {
        for dep in &candidate.dependencies {
            if !ids.contains(dep.as_str()) {
                return Err(format!(
                    "campaign: candidate {} depends on unknown {dep}; the bundle's \
                     lineage closure is incomplete",
                    candidate.id
                ));
            }
        }
    }
    Ok(out)
}

// ===========================================================================
// V1-era checks that remain lawful under V2 (policy, evaluation sets,
// dispositions, envelope)
// ===========================================================================

fn check_policy(section: &[String]) -> Result<(), String> {
    let axes = [
        "max-candidates", "max-logical-work", "max-memory-bytes", "max-monotonic-ticks",
    ];
    for (i, axis) in axes.iter().enumerate() {
        let value = expect_kv(section, i, &format!("search-budget {axis}"))?;
        let (declared, actual) = value
            .split_once(' ')
            .ok_or_else(|| format!("campaign: search-budget {axis} is not two tokens"))?;
        let declared: u64 = declared
            .strip_prefix("declared=")
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("campaign: bad declared bound in {value:?}"))?;
        let actual: u64 = actual
            .strip_prefix("actual=")
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("campaign: bad actual use in {value:?}"))?;
        // The BoundedResponse law, independently recomputed: the receipt
        // must show termination WITHIN the declared budget.
        if actual > declared {
            return Err(format!(
                "campaign: search overran its declared budget on {axis} \
                 (declared {declared}, actual {actual})"
            ));
        }
    }
    let fuzz = expect_kv(section, axes.len(), "fuzz")?;
    for key in ["seed=", "traces=", "max-ops="] {
        if !fuzz.split(' ').any(|tok| {
            tok.strip_prefix(key)
                .is_some_and(|v| v.parse::<u64>().is_ok())
        }) {
            return Err(format!(
                "campaign: fuzz policy does not bind {key}<n> (seed and bounds are \
                 the reproducibility contract)"
            ));
        }
    }
    Ok(())
}

fn check_manifest_section(section: &[String], judge_root: &Path) -> Result<(), String> {
    let roles = ["search", "qualification", "holdout", "regression"];
    let mut texts: BTreeMap<&str, String> = BTreeMap::new();
    for (i, role) in roles.iter().enumerate() {
        let value = expect_kv(section, i, &format!("evaluation-set {role}"))?;
        let digest = value
            .split(' ')
            .find_map(|tok| tok.strip_prefix("digest="))
            .ok_or_else(|| format!("campaign: evaluation-set {role} binds no digest"))?;
        let path = judge_root.join(format!("{role}.vectors"));
        let bytes = fs::read(&path)
            .map_err(|e| format!("campaign: cannot read {}: {e}", path.display()))?;
        let recomputed = sha_hex(&bytes);
        if recomputed != digest {
            return Err(format!(
                "campaign: evaluation-set {role} digest does not match the frozen \
                 judge's vector file"
            ));
        }
        texts.insert(role, String::from_utf8_lossy(&bytes).into_owned());
    }
    if section.get(roles.len()).map(String::as_str) != Some("search-holdout-disjoint yes") {
        return Err("campaign: the search-holdout-disjoint attestation is absent".to_owned());
    }
    // Independent recompute of the disjointness the bundle attests.
    let canon = |text: &str| -> BTreeSet<String> {
        text.split("----")
            .map(|t| {
                t.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<&str>>()
                    .join("\n")
            })
            .filter(|t| !t.is_empty())
            .collect()
    };
    let search = canon(&texts["search"]);
    let holdout = canon(&texts["holdout"]);
    if search.intersection(&holdout).next().is_some() {
        return Err(
            "campaign: search and holdout sets OVERLAP; holdout evidence cannot \
             reuse search inputs"
                .to_owned(),
        );
    }
    Ok(())
}

fn check_dispositions(section: &[String]) -> Result<(), String> {
    let models = section
        .iter()
        .filter(|l| l.starts_with("model-disposition "))
        .count();
    if models == 0 {
        return Err("campaign: no model-verification dispositions".to_owned());
    }
    let runtime = section
        .iter()
        .filter(|l| l.starts_with("runtime-conformance-disposition "))
        .count();
    if runtime == 0 {
        return Err("campaign: no runtime-conformance dispositions".to_owned());
    }
    for line in section {
        if line.starts_with("runtime-conformance-disposition ")
            && !line.contains("conformant-for-observed-history")
        {
            return Err(format!(
                "campaign: runtime conformance concluded without offline replay: {line:?}"
            ));
        }
    }
    let mutant = section
        .iter()
        .find(|l| l.starts_with("mutant "))
        .ok_or("campaign: the planted mutant has no disposition line")?;
    if !mutant.contains("activated=yes") || !mutant.contains("killed=yes") {
        return Err(format!(
            "campaign: the mutant disposition is not activated-and-killed: {mutant:?}"
        ));
    }
    Ok(())
}

fn check_envelope(path: &Path) -> Result<(), String> {
    let raw = fs::read(path)
        .map_err(|e| format!("campaign: cannot read envelope {}: {e}", path.display()))?;
    let lines = strict_lines(&raw)?;
    if lines.first().map(String::as_str) != Some(ENVELOPE_MAGIC) {
        return Err(format!(
            "campaign: envelope magic is not {ENVELOPE_MAGIC:?} (found {:?})",
            lines.first()
        ));
    }
    let mut pos = 1;
    for &field in RELEASE_SEAL_FIELDS {
        let name = format!("{field:?}");
        let line = lines
            .get(pos)
            .ok_or_else(|| format!("campaign: envelope omits seal field {name} (a missing \
                                    field is an incomplete envelope, not an empty set)"))?;
        pos += 1;
        let rest = line
            .strip_prefix(&format!("seal {name} "))
            .ok_or_else(|| format!("campaign: expected seal field {name} in canonical \
                                    order, found {line:?}"))?;
        match field.empty_set_posture() {
            EmptySetPosture::NotSetValued => {
                let value = rest
                    .strip_prefix("commitment ")
                    .ok_or_else(|| format!("campaign: non-set field {name} must bind one \
                                            commitment, found {rest:?}"))?;
                hex64(value, &format!("{name} commitment"))?;
            }
            EmptySetPosture::ExplicitEvenWhenEmpty => {
                let count: usize = rest
                    .strip_prefix("rows ")
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| format!("campaign: set-valued field {name} must state \
                                            its explicit row count, found {rest:?}"))?;
                for _ in 0..count {
                    let row = lines
                        .get(pos)
                        .ok_or_else(|| format!("campaign: envelope ended inside {name} rows"))?;
                    if !row.starts_with(&format!("row {name} ")) {
                        return Err(format!(
                            "campaign: expected `row {name} ...`, found {row:?}"
                        ));
                    }
                    pos += 1;
                }
                // The three fields the rehearsal must POPULATE.
                if count == 0
                    && matches!(
                        name.as_str(),
                        "ModelDispositions"
                            | "RuntimeConformanceDispositions"
                            | "CandidatePromotionSet"
                    )
                {
                    return Err(format!(
                        "campaign: rehearsal envelope field {name} is empty; the \
                         rehearsal must carry its dispositions"
                    ));
                }
            }
        }
    }
    if pos != lines.len() {
        return Err("campaign: envelope carries trailing lines".to_owned());
    }
    Ok(())
}

// ===========================================================================
// Perimeter walk: snapshot, hygiene (E7-C rows 7, 12, 13)
// ===========================================================================

/// One walked perimeter root: every regular file's content digest and every
/// directory, by forward-slash relative path. Taken ONCE at the start and
/// re-taken at the end — byte drift during the pass is a refusal.
struct PerimeterSnapshot {
    files: BTreeMap<String, String>,
    dirs: BTreeSet<String>,
}

fn walk_root(root: &Path, what: &str) -> Result<PerimeterSnapshot, String> {
    let mut snapshot = PerimeterSnapshot {
        files: BTreeMap::new(),
        dirs: BTreeSet::new(),
    };
    walk_dir(root, root, what, &mut snapshot)?;
    // Case-fold twins: two entries whose names collide under case folding are
    // one entry on a case-insensitive filesystem and a forgery lane on a
    // case-sensitive one; the perimeter admits neither (row 7).
    let mut folded: BTreeMap<String, &String> = BTreeMap::new();
    let dir_names: Vec<String> = snapshot.dirs.iter().cloned().collect();
    let all: Vec<&String> = snapshot.files.keys().chain(dir_names.iter()).collect();
    for rel in all {
        if let Some(twin) = folded.insert(rel.to_lowercase(), rel) {
            return Err(format!(
                "campaign: perimeter: case-fold twins {twin:?} and {rel:?} under the \
                 {what} root"
            ));
        }
    }
    Ok(snapshot)
}

fn walk_dir(
    base: &Path,
    dir: &Path,
    what: &str,
    snapshot: &mut PerimeterSnapshot,
) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("campaign: cannot read {what} root dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("campaign: {what} root dir entry error: {e}"))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|_| "campaign: path escaped the perimeter root".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry
            .file_type()
            .map_err(|e| format!("campaign: cannot stat {}: {e}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "campaign: perimeter: {rel} under the {what} root is a symlink; the \
                 perimeter admits only regular files and directories"
            ));
        }
        if file_type.is_dir() {
            snapshot.dirs.insert(rel.clone());
            walk_dir(base, &path, what, snapshot)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&path)
                .map_err(|e| format!("campaign: cannot read {}: {e}", path.display()))?;
            snapshot.files.insert(rel, sha_hex(&bytes));
        } else {
            return Err(format!(
                "campaign: perimeter: {rel} under the {what} root is neither a regular \
                 file nor a directory"
            ));
        }
    }
    Ok(())
}

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn has_scratch_extension(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| SCRATCH_EXTENSIONS.iter().any(|s| *s == ext))
}

fn looks_authority_bearing(name: &str) -> bool {
    name.ends_with(".receipt") || name.ends_with(".manifest") || name == "manifest"
        || name.ends_with(".bundle") || name.ends_with(".t0")
}

/// Nursery layout hygiene (row 7): only `<id>/manifest` and
/// `<id>/receipts/<64hex>.receipt` are lawful entries. Anything else is a
/// scratch, authority-looking, or foreign file and is refused by name.
fn check_nursery_hygiene(snapshot: &PerimeterSnapshot) -> Result<(), String> {
    for rel in snapshot.dirs.iter() {
        let parts: Vec<&str> = rel.split('/').collect();
        let lawful = match parts.as_slice() {
            [id] => is_hex64(id),
            [id, sub] => is_hex64(id) && *sub == "receipts",
            _ => false,
        };
        if !lawful {
            return Err(format!(
                "campaign: perimeter: foreign directory {rel} under the nursery root \
                 (only <id>/ and <id>/receipts/ are lawful)"
            ));
        }
    }
    for rel in snapshot.files.keys() {
        if has_scratch_extension(rel) {
            return Err(format!(
                "campaign: perimeter: compiler-scratch file {rel} under the nursery root"
            ));
        }
        let parts: Vec<&str> = rel.split('/').collect();
        let lawful = match parts.as_slice() {
            [id, name] => is_hex64(id) && *name == "manifest",
            [id, sub, name] => {
                is_hex64(id)
                    && *sub == "receipts"
                    && name
                        .strip_suffix(".receipt")
                        .is_some_and(is_hex64)
            }
            _ => false,
        };
        if !lawful {
            let name = parts.last().copied().unwrap_or(rel.as_str());
            if looks_authority_bearing(name) {
                return Err(format!(
                    "campaign: perimeter: unreferenced authority-looking file {rel} under \
                     the nursery root (only <id>/manifest and \
                     <id>/receipts/<sha256>.receipt are lawful)"
                ));
            }
            return Err(format!(
                "campaign: perimeter: foreign file {rel} under the nursery root"
            ));
        }
    }
    Ok(())
}

/// Evidence-root layout hygiene (row 7): a FLAT content-addressed store of
/// `<64hex>.<suffix>` artifacts plus the bundle itself. Receipts and
/// manifests live in the nursery, never here.
fn check_evidence_hygiene(
    snapshot: &PerimeterSnapshot,
    exempt: &BTreeSet<String>,
) -> Result<(), String> {
    if let Some(dir) = snapshot.dirs.iter().next() {
        return Err(format!(
            "campaign: perimeter: directory {dir} under the evidence root; the \
             evidence root is a flat content-addressed store"
        ));
    }
    for rel in snapshot.files.keys() {
        if exempt.contains(rel) {
            continue;
        }
        if has_scratch_extension(rel) {
            return Err(format!(
                "campaign: perimeter: compiler-scratch file {rel} under the evidence root"
            ));
        }
        if rel.ends_with(".receipt") || rel.ends_with(".manifest") {
            return Err(format!(
                "campaign: perimeter: unreferenced authority-looking file {rel} under \
                 the evidence root (receipts and manifests live in the nursery)"
            ));
        }
        let addressed = rel
            .split_once('.')
            .is_some_and(|(stem, suffix)| is_hex64(stem) && !suffix.is_empty());
        if !addressed {
            return Err(format!(
                "campaign: perimeter: foreign file {rel} under the evidence root \
                 (artifacts are content-addressed as <sha256>.<suffix>)"
            ));
        }
    }
    Ok(())
}

// ===========================================================================
// Receipts (rows 4, 5, 6): typed parse, address recompute, reference collection
// ===========================================================================

/// One parsed nursery receipt: the typed facts the perimeter and coherence
/// checks consume.
struct ParsedReceipt {
    rel: String,
    kind: String,
    reused: Option<String>,
    reuse_key: Option<String>,
    dependency_lines: Vec<String>,
}

/// Everything the receipt walk learned about one candidate's receipt store.
struct ReceiptStore {
    kinds: BTreeSet<String>,
    receipts: Vec<ParsedReceipt>,
}

fn parse_receipt(
    rel: &str,
    bytes: &[u8],
    dir_id: &str,
    judge_digest: &str,
    evidence_refs: &mut Vec<(String, String)>,
) -> Result<ParsedReceipt, String> {
    let stem = rel
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix(".receipt"))
        .unwrap_or("");
    let actual = sha_hex(bytes);
    if actual != stem {
        return Err(format!(
            "campaign: receipt {rel} does not hash to its content address \
             (bytes are {actual})"
        ));
    }
    let lines =
        strict_lines(bytes).map_err(|e| format!("campaign: receipt {rel} is malformed: {e}"))?;
    if lines.first().map(String::as_str) != Some(RECEIPT_MAGIC) {
        return Err(format!(
            "campaign: receipt {rel} magic is not {RECEIPT_MAGIC:?} (found {:?})",
            lines.first()
        ));
    }
    let mut c = ManifestCursor {
        lines: &lines,
        pos: 1,
    };
    let kind = c.take("kind").map_err(|e| format!("campaign: receipt {rel}: {e}"))?;
    let kinds: Vec<String> = CampaignReceiptKind::ALL
        .iter()
        .map(|k| k.spelling().to_owned())
        .collect();
    if !kinds.iter().any(|k| *k == kind) {
        return Err(format!(
            "campaign: receipt {rel} kind token {kind:?} is not in the typed \
             CampaignReceiptKind inventory"
        ));
    }
    let candidate = c
        .take("candidate")
        .map_err(|e| format!("campaign: receipt {rel}: {e}"))?;
    hex64(&candidate, "receipt candidate")?;
    if candidate != dir_id {
        return Err(format!(
            "campaign: receipt {rel} names candidate {candidate} but lives in \
             {dir_id}'s receipt store"
        ));
    }
    let judge = c.take("judge").map_err(|e| format!("campaign: receipt {rel}: {e}"))?;
    if judge != judge_digest {
        return Err(format!(
            "campaign: receipt {rel} binds judge {judge}, not the recomputed frozen \
             judge {judge_digest}"
        ));
    }
    let source_frontier = c
        .take("source-frontier")
        .map_err(|e| format!("campaign: receipt {rel}: {e}"))?;
    hex64(&source_frontier, "receipt source-frontier")?;
    let mut reused = None;
    let mut reuse_key = None;
    let mut dependency_lines = Vec::new();
    if kind == "reuse" {
        // A key alone never licenses reuse: the receipt binds the reused id,
        // the reuse key, dependency commitments, and fresh requalification
        // evidence, in the wire order (row 9).
        let r = c.take("reused").map_err(|e| format!("campaign: reuse receipt {rel}: {e}"))?;
        hex64(&r, "reused candidate")?;
        reused = Some(r);
        let k = c
            .take("reuse-key")
            .map_err(|e| format!("campaign: reuse receipt {rel}: {e}"))?;
        hex64(&k, "receipt reuse-key")?;
        reuse_key = Some(k);
        while c
            .lines
            .get(c.pos)
            .is_some_and(|l| l.starts_with("dependency "))
        {
            let dep = c.take("dependency")?;
            let (dep_id, commitment) = dep
                .split_once(' ')
                .ok_or_else(|| format!("campaign: reuse receipt {rel} dependency {dep:?} is \
                                        not two addresses"))?;
            hex64(dep_id, "reuse dependency candidate")?;
            hex64(commitment, "reuse dependency commitment")?;
            dependency_lines.push(dep);
        }
        let requalified = c
            .take("requalified-evidence")
            .map_err(|e| format!("campaign: reuse receipt {rel}: {e}"))?;
        hex64(&requalified, "requalified-evidence")?;
        evidence_refs.push((requalified, rel.to_owned()));
    }
    let mut stale_named = false;
    for line in &lines[c.pos..] {
        if let Some(value) = line.strip_prefix("evidence ") {
            hex64(value, "receipt evidence reference")?;
            evidence_refs.push((value.to_owned(), rel.to_owned()));
        }
        if line.starts_with("stale-coordinate ") {
            stale_named = true;
        }
    }
    if kind == "invalidation" && !stale_named {
        return Err(format!(
            "campaign: invalidation receipt {rel} names no stale coordinate; an \
             invalidation records exactly which bound coordinate changed"
        ));
    }
    Ok(ParsedReceipt {
        rel: rel.to_owned(),
        kind,
        reused,
        reuse_key,
        dependency_lines,
    })
}

/// The receipt-derived terminal spelling, if any terminal event is receipted.
/// A later loss of standing outranks the event it superseded: invalidation,
/// then escalation, then refusal, then promotion.
fn derived_terminal(kinds: &BTreeSet<String>) -> Option<&'static str> {
    if kinds.contains("invalidation") {
        Some("Invalidated")
    } else if kinds.contains("escalation") {
        Some("ArchitectRequired")
    } else if kinds.contains("refusal") {
        Some("Refused")
    } else if kinds.contains("promotion") {
        Some("Promoted")
    } else {
        None
    }
}

/// The receipt kind that PROVES a terminal (row 8): exhaustive over the
/// closed terminal vocabulary, so a new terminal must choose its receipt.
fn receipt_kind_for(terminal: CampaignTerminal) -> CampaignReceiptKind {
    match terminal {
        CampaignTerminal::Promoted => CampaignReceiptKind::Promotion,
        CampaignTerminal::Refused => CampaignReceiptKind::Refusal,
        CampaignTerminal::Invalidated => CampaignReceiptKind::Invalidation,
        CampaignTerminal::ArchitectRequired => CampaignReceiptKind::Escalation,
    }
}

// ===========================================================================
// Frontier and closure sections
// ===========================================================================

struct FrontierClaims {
    state: BTreeMap<String, String>,
    freshness: BTreeMap<String, String>,
}

fn parse_frontier(section: &[String], candidates: &[ParsedCandidate]) -> Result<FrontierClaims, String> {
    let states: Vec<String> = FrontierState::ALL
        .iter()
        .map(|s| s.spelling().to_owned())
        .collect();
    let freshness_tokens: Vec<String> = EvidenceFreshness::ALL
        .iter()
        .map(|f| f.spelling().to_owned())
        .collect();
    let mut claims = FrontierClaims {
        state: BTreeMap::new(),
        freshness: BTreeMap::new(),
    };
    for line in section {
        let rest = line
            .strip_prefix("frontier ")
            .ok_or_else(|| format!("campaign: unexpected frontier line {line:?}"))?;
        let tokens: Vec<&str> = rest.split(' ').collect();
        let id = *tokens.first().ok_or("campaign: frontier line has no candidate")?;
        let state = tokens
            .iter()
            .find_map(|t| t.strip_prefix("state="))
            .ok_or_else(|| format!("campaign: frontier line binds no state: {line:?}"))?;
        token_in(state, &states, "frontier state").map_err(|e| format!("campaign: {e}"))?;
        let fresh = tokens
            .iter()
            .find_map(|t| t.strip_prefix("freshness="))
            .ok_or_else(|| format!("campaign: frontier line binds no freshness: {line:?}"))?;
        token_in(fresh, &freshness_tokens, "freshness").map_err(|e| format!("campaign: {e}"))?;
        if claims
            .state
            .insert(id.to_owned(), state.to_owned())
            .is_some()
        {
            return Err(format!("campaign: duplicate frontier line for {id}"));
        }
        claims.freshness.insert(id.to_owned(), fresh.to_owned());
    }
    for candidate in candidates {
        if !claims.state.contains_key(candidate.id.as_str()) {
            return Err(format!(
                "campaign: candidate {} has no frontier line",
                candidate.id
            ));
        }
    }
    if claims.state.len() != candidates.len() {
        return Err("campaign: frontier lines do not cover exactly the candidates".to_owned());
    }
    Ok(claims)
}

struct ClosureClaims {
    terminal: BTreeMap<String, String>,
    frontier: BTreeMap<String, String>,
    edges: Vec<(String, String, String)>,
}

fn parse_closure(section: &[String], candidates: &[ParsedCandidate]) -> Result<ClosureClaims, String> {
    let ids: BTreeSet<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    let kinds: Vec<String> = CampaignClosureEdgeKind::ALL
        .iter()
        .map(|k| k.spelling().to_owned())
        .collect();
    let mut claims = ClosureClaims {
        terminal: BTreeMap::new(),
        frontier: BTreeMap::new(),
        edges: Vec::new(),
    };
    for line in section {
        if let Some(rest) = line.strip_prefix("node ") {
            let tokens: Vec<&str> = rest.split(' ').collect();
            let id = *tokens.first().unwrap_or(&"");
            if !ids.contains(id) {
                return Err(format!("campaign: closure node {id} is not a candidate"));
            }
            let terminal = tokens
                .iter()
                .find_map(|t| t.strip_prefix("terminal="))
                .ok_or_else(|| format!("campaign: closure node binds no terminal: {line:?}"))?;
            let frontier = tokens
                .iter()
                .find_map(|t| t.strip_prefix("frontier="))
                .ok_or_else(|| format!("campaign: closure node binds no frontier: {line:?}"))?;
            if terminal != "none" {
                let known: Vec<String> = CampaignTerminal::ALL
                    .iter()
                    .map(|t| t.spelling().to_owned())
                    .collect();
                token_in(terminal, &known, "closure terminal")
                    .map_err(|e| format!("campaign: {e}"))?;
            }
            if claims
                .terminal
                .insert(id.to_owned(), terminal.to_owned())
                .is_some()
            {
                return Err(format!("campaign: duplicate closure node for {id}"));
            }
            claims.frontier.insert(id.to_owned(), frontier.to_owned());
        } else if let Some(rest) = line.strip_prefix("edge ") {
            let tokens: Vec<&str> = rest.split(' ').collect();
            let [from, to, kind] = tokens.as_slice() else {
                return Err(format!("campaign: closure edge is not `from to kind`: {line:?}"));
            };
            token_in(kind, &kinds, "closure edge kind").map_err(|e| format!("campaign: {e}"))?;
            if !ids.contains(*from) || !ids.contains(*to) {
                return Err(format!("campaign: closure edge endpoint unknown: {line:?}"));
            }
            claims
                .edges
                .push(((*from).to_owned(), (*to).to_owned(), (*kind).to_owned()));
        } else {
            return Err(format!("campaign: unexpected closure line {line:?}"));
        }
    }
    if claims.terminal.len() != candidates.len() {
        return Err("campaign: closure nodes do not cover exactly the candidates".to_owned());
    }
    Ok(claims)
}

/// E7-C row 8: every terminal event the bundle claims is represented by a
/// typed receipt of the MATCHING kind, and the claim equals the
/// receipt-derived terminal — a terminal without its receipt is an assertion.
fn check_terminal_representation(
    candidates: &[ParsedCandidate],
    stores: &BTreeMap<String, ReceiptStore>,
    closure: &ClosureClaims,
) -> Result<(), String> {
    for candidate in candidates {
        let id = candidate.id.as_str();
        let store = &stores[id];
        let claimed = closure.terminal[id].as_str();
        if claimed != "none" {
            let terminal = CampaignTerminal::ALL
                .iter()
                .find(|t| t.spelling() == claimed)
                .expect("closure terminal tokens were checked against the inventory");
            let required = receipt_kind_for(*terminal).spelling();
            if !store.kinds.contains(required) {
                return Err(format!(
                    "campaign: terminal {claimed} for candidate {id} is not represented \
                     by a {required} receipt in its nursery receipt store"
                ));
            }
        }
        match derived_terminal(&store.kinds) {
            Some(derived) => {
                if claimed == "none" {
                    return Err(format!(
                        "campaign: candidate {id} carries a receipted terminal \
                         ({derived}) but its closure node claims no terminal"
                    ));
                }
                if claimed != derived {
                    return Err(format!(
                        "campaign: closure node terminal {claimed} for candidate {id} \
                         does not match the receipt-derived terminal {derived}"
                    ));
                }
            }
            None => {
                if claimed != "none" {
                    return Err(format!(
                        "campaign: terminal {claimed} for candidate {id} has no \
                         receipted terminal event behind it"
                    ));
                }
            }
        }
    }
    Ok(())
}

// ===========================================================================
// The thirteen coherence refusals (TL-4: the architect's twelve, plus the
// derived thirteenth) — each a named finding
// ===========================================================================

fn check_coherence(
    candidates: &[ParsedCandidate],
    stores: &BTreeMap<String, ReceiptStore>,
    frontier: &FrontierClaims,
) -> Result<(), String> {
    for candidate in candidates {
        let id = candidate.id.as_str();
        let store = &stores[id];
        let state = frontier.state[id].as_str();
        let freshness = frontier.freshness[id].as_str();
        let derived = derived_terminal(&store.kinds);
        let deps_all_qualified = candidate
            .dependencies
            .iter()
            .all(|dep| frontier.state.get(dep.as_str()).map(String::as_str) == Some("Qualified"));
        // 1. Scaffold+Qualified.
        if candidate.posture == "Scaffold" && state == "Qualified" {
            return Err(format!(
                "campaign: coherence: Scaffold candidate {id} is declared Qualified; a \
                 scaffold realizes nothing and never enters the trusted frontier"
            ));
        }
        // 2. Missing+Qualified.
        if candidate.posture == "Missing" && state == "Qualified" {
            return Err(format!(
                "campaign: coherence: Missing candidate {id} is declared Qualified; an \
                 unrealized obligation cannot enter the trusted frontier"
            ));
        }
        // 3. Refused-terminal+Qualified.
        if derived == Some("Refused") && state == "Qualified" {
            return Err(format!(
                "campaign: coherence: refused candidate {id} is declared Qualified; a \
                 refusal on own content is not admission"
            ));
        }
        // 4. ArchitectRequired+Qualified.
        if derived == Some("ArchitectRequired") && state == "Qualified" {
            return Err(format!(
                "campaign: coherence: ArchitectRequired candidate {id} is declared \
                 Qualified; the campaign stopped for the architect and no machine \
                 admits it"
            ));
        }
        // 5. ArchitectRequired+BlockedByDependency.
        if derived == Some("ArchitectRequired") && state == "BlockedByDependency" {
            return Err(format!(
                "campaign: coherence: ArchitectRequired candidate {id} is declared \
                 BlockedByDependency; the block is the architect's, not a dependency's"
            ));
        }
        // 6. Own-content refusal hiding behind BlockedByDependency.
        if store.kinds.contains("refusal") && state == "BlockedByDependency" && deps_all_qualified
        {
            return Err(format!(
                "campaign: coherence: candidate {id} was refused on its own content yet \
                 is declared BlockedByDependency with every dependency Qualified"
            ));
        }
        // 7. BlockedByDependency with nothing blocking.
        if state == "BlockedByDependency" && deps_all_qualified {
            return Err(format!(
                "campaign: coherence: candidate {id} is declared BlockedByDependency \
                 but every dependency is Qualified; nothing upstream blocks it"
            ));
        }
        // 8. Qualified above a non-Qualified dependency (dependency-first:
        //    later green cannot bless earlier red).
        if state == "Qualified" {
            for dep in &candidate.dependencies {
                let dep_state = frontier
                    .state
                    .get(dep.as_str())
                    .map(String::as_str)
                    .unwrap_or("absent");
                if dep_state != "Qualified" {
                    return Err(format!(
                        "campaign: qualified candidate {id} sits above {dep} whose \
                         frontier state is {dep_state} -- dependency-first violated"
                    ));
                }
            }
        }
        // 9. Invalidated without an exact invalidation receipt.
        if state == "Invalidated" && !store.kinds.contains("invalidation") {
            return Err(format!(
                "campaign: coherence: candidate {id} is declared Invalidated without an \
                 exact invalidation receipt"
            ));
        }
        // 10. Promoted-terminal without a promotion receipt is refused by the
        //     terminal-representation walk (row 8) before this point.
        // 11. Promoted with an unbacked staleness claim: within one verify
        //     pass the judge digest is recomputed and matches, so a
        //     StaleByJudgeChange/StaleByDependencyChange claim must be backed
        //     by an invalidation receipt recording the divergence.
        if derived == Some("Promoted")
            && freshness != "Fresh"
            && !store.kinds.contains("invalidation")
        {
            return Err(format!(
                "campaign: coherence: promoted candidate {id} claims {freshness} but no \
                 digest divergence or invalidation receipt backs the staleness claim"
            ));
        }
        // 12. LawChanging without ArchitectRequired authority.
        if candidate.change_class == "LawChanging" && !store.kinds.contains("escalation") {
            return Err(format!(
                "campaign: coherence: LawChanging candidate {id} carries no \
                 ArchitectRequired escalation receipt; a law change cannot ride \
                 machine authority"
            ));
        }
        // 13 (derived, TL-4): in flight means not yet admitted anywhere.
        if derived.is_none() && state != "Unqualified" {
            return Err(format!(
                "campaign: coherence: candidate {id} is in flight (no terminal receipt) \
                 yet its frontier is {state}, not Unqualified"
            ));
        }
    }
    Ok(())
}

// ===========================================================================
// Closure edges (rows 9, 10, 11) and the four-state recompute (row 14)
// ===========================================================================

fn check_closure_edges(
    candidates: &[ParsedCandidate],
    stores: &BTreeMap<String, ReceiptStore>,
    closure: &ClosureClaims,
) -> Result<(), String> {
    let by_id: BTreeMap<&str, &ParsedCandidate> =
        candidates.iter().map(|c| (c.id.as_str(), c)).collect();
    for (from, to, kind) in &closure.edges {
        let owner = by_id[from.as_str()];
        // Lineage relations are DERIVED from required record fields: a
        // Parent/Dependency edge must be read back out of the manifest.
        if kind == "Parent" && !owner.parents.iter().any(|p| p == to) {
            return Err(format!(
                "campaign: Parent edge {from} -> {to} is not carried by the \
                 manifest's parent field"
            ));
        }
        if kind == "Dependency" && !owner.dependencies.iter().any(|d| d == to) {
            return Err(format!(
                "campaign: Dependency edge {from} -> {to} is not carried by a \
                 dependency commitment"
            ));
        }
        // Row 9: a reuse edge resolves to a reuse receipt binding the reused
        // id, the reuse key, the current judge (bound at receipt parse), the
        // dependency commitments, and fresh requalification evidence.
        if kind == "Reuse" {
            let store = &stores[from.as_str()];
            let receipt = store
                .receipts
                .iter()
                .find(|r| r.kind == "reuse" && r.reused.as_deref() == Some(to.as_str()))
                .ok_or_else(|| {
                    format!(
                        "campaign: Reuse edge {from} -> {to} resolves to no reuse \
                         receipt in {from}'s receipt store"
                    )
                })?;
            let reused_key = by_id[to.as_str()].reuse_key.as_str();
            if receipt.reuse_key.as_deref() != Some(reused_key) {
                return Err(format!(
                    "campaign: reuse receipt {} binds reuse-key {:?} but the reused \
                     candidate {to} carries {reused_key}",
                    receipt.rel,
                    receipt.reuse_key.as_deref().unwrap_or("none")
                ));
            }
            for dep in &receipt.dependency_lines {
                if !owner.dependency_lines.iter().any(|d| d == dep) {
                    return Err(format!(
                        "campaign: reuse receipt {} binds dependency {dep:?} that the \
                         manifest of {from} does not carry",
                        receipt.rel
                    ));
                }
            }
        }
        // Row 10: an invalidation edge resolves to an invalidation receipt
        // (the receipt's stale coordinate is enforced at receipt parse).
        if kind == "Invalidation" && !stores[from.as_str()].kinds.contains("invalidation") {
            return Err(format!(
                "campaign: Invalidation edge {from} -> {to} resolves to no invalidation \
                 receipt naming the stale coordinate"
            ));
        }
    }
    Ok(())
}

/// E7-C row 14: the frontier section is DERIVABLE. Recompute each
/// candidate's four-state answer from receipts + manifests alone and refuse
/// any divergence from the bundle's own frontier section.
fn recompute_frontier(
    candidates: &[ParsedCandidate],
    stores: &BTreeMap<String, ReceiptStore>,
    frontier: &FrontierClaims,
) -> Result<(), String> {
    let by_id: BTreeMap<&str, &ParsedCandidate> =
        candidates.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut memo: BTreeMap<String, String> = BTreeMap::new();
    for candidate in candidates {
        let mut visiting: BTreeSet<String> = BTreeSet::new();
        recompute_state(candidate.id.as_str(), &by_id, stores, frontier, &mut memo, &mut visiting)?;
    }
    for candidate in candidates {
        let id = candidate.id.as_str();
        let declared = frontier.state[id].as_str();
        let recomputed = memo[id].as_str();
        if declared != recomputed {
            return Err(format!(
                "campaign: frontier state for candidate {id} is declared {declared} but \
                 recomputes {recomputed} under the four-state law"
            ));
        }
    }
    Ok(())
}

fn recompute_state(
    id: &str,
    by_id: &BTreeMap<&str, &ParsedCandidate>,
    stores: &BTreeMap<String, ReceiptStore>,
    frontier: &FrontierClaims,
    memo: &mut BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
) -> Result<String, String> {
    if let Some(done) = memo.get(id) {
        return Ok(done.clone());
    }
    if !visiting.insert(id.to_owned()) {
        return Err(format!(
            "campaign: dependency cycle through {id}; the lineage closure is not \
             well-founded"
        ));
    }
    let candidate = by_id[id];
    let store = &stores[id];
    let state = match derived_terminal(&store.kinds) {
        Some("Invalidated") => "Invalidated".to_owned(),
        Some("Promoted") => {
            if candidate.posture != "Candidate" {
                // A promoted scaffold is refused by coherence row 1 before
                // this recompute runs; keep the recompute honest anyway.
                "Unqualified".to_owned()
            } else if frontier.freshness[id] != "Fresh" {
                // Backed staleness carries an invalidation receipt and lands
                // in the Invalidated arm; an unbacked claim was already
                // refused by coherence row 11.
                "Invalidated".to_owned()
            } else {
                let mut blocked = false;
                for dep in &candidate.dependencies {
                    let dep_state = recompute_state(dep, by_id, stores, frontier, memo, visiting)?;
                    if dep_state != "Qualified" {
                        blocked = true;
                    }
                }
                if blocked {
                    "BlockedByDependency".to_owned()
                } else {
                    "Qualified".to_owned()
                }
            }
        }
        // Refused, ArchitectRequired, and in-flight all answer the one
        // narrow frontier question the same way: not admitted.
        Some(_) | None => "Unqualified".to_owned(),
    };
    visiting.remove(id);
    memo.insert(id.to_owned(), state.clone());
    Ok(state)
}

// ===========================================================================
// The V2 verification pass
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn verify_v2(
    lines: &[String],
    bundle: &Path,
    judge_root: &Path,
    envelope: &Path,
    source_commit: &str,
    nursery_root: &Path,
    evidence_root: &Path,
) -> Result<(), String> {
    let doc = split_sections(lines)?;

    // Frozen-judge binding: RECOMPUTE the judge root's tree digest.
    let recomputed = tree_digest(judge_root)?;
    if recomputed.render() != doc.judge_root_digest {
        return Err(format!(
            "campaign: judge-root digest mismatch -- bundle claims {}, recomputed {} \
             (evidence bound to a mutated or foreign judge cannot verify)",
            doc.judge_root_digest,
            recomputed.render()
        ));
    }

    // Source-commit binding.
    let claimed = expect_kv(&doc.sections["source"], 0, "source-commit")?;
    if claimed.len() != 40 || !claimed.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err(format!("campaign: source-commit {claimed:?} is not 40 lowercase hex"));
    }
    if claimed != source_commit {
        return Err(format!(
            "campaign: bundle source-commit {claimed} does not match this checkout's \
             {source_commit}"
        ));
    }

    // V1-era laws that remain lawful under V2.
    check_policy(&doc.sections["policy"])?;
    check_manifest_section(&doc.sections["manifests"], judge_root)?;
    check_dispositions(&doc.sections["dispositions"])?;
    check_envelope(envelope)?;

    // The V2 candidates: strict manifests, active proof targets, and the
    // exact identity recompute (row 3).
    let candidates = parse_candidates(&doc.sections["candidates"])?;

    // Rows 7/12/13 setup: snapshot BOTH perimeter roots before touching any
    // referenced byte. The end-of-pass re-walk proves nursery
    // byte-immutability (row 12) and, applied to the evidence root, the
    // one-pass half of the append-only posture (row 13): the producer's own
    // snapshot proves appends across the campaign; this pass proves no byte
    // moved while it looked.
    let nursery_before = walk_root(nursery_root, "nursery")?;
    let evidence_before = walk_root(evidence_root, "evidence")?;
    check_nursery_hygiene(&nursery_before)?;
    let mut evidence_exempt: BTreeSet<String> = BTreeSet::new();
    for target in [bundle, envelope] {
        if let (Ok(canon_root), Ok(canon_target)) =
            (fs::canonicalize(evidence_root), fs::canonicalize(target))
        {
            if let Ok(rel) = canon_target.strip_prefix(&canon_root) {
                evidence_exempt.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    evidence_exempt.insert(BUNDLE_BASENAME.to_owned());
    check_evidence_hygiene(&evidence_before, &evidence_exempt)?;

    // Rows 1 and 2: the bundle<->nursery bijection, then byte identity
    // between each persisted manifest and its bundle-embedded copy.
    let ids: BTreeSet<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    let top_dirs: BTreeSet<&str> = nursery_before
        .dirs
        .iter()
        .filter(|d| !d.contains('/'))
        .map(String::as_str)
        .collect();
    for dir in &top_dirs {
        if !ids.contains(dir) {
            return Err(format!(
                "campaign: nursery record {dir} has no candidate in the bundle \
                 (bundle<->nursery bijection broken)"
            ));
        }
    }
    for candidate in &candidates {
        let id = candidate.id.as_str();
        let manifest_rel = format!("{id}/manifest");
        if !top_dirs.contains(id) || !nursery_before.files.contains_key(&manifest_rel) {
            return Err(format!(
                "campaign: candidate {id} has no nursery manifest at \
                 nursery/{id}/manifest (bundle<->nursery bijection broken)"
            ));
        }
        let mut embedded_text = String::new();
        for line in &candidate.manifest_lines {
            embedded_text.push_str(line);
            embedded_text.push('\n');
        }
        let manifest_path = nursery_root.join(id).join("manifest");
        let nursery_bytes = fs::read(&manifest_path)
            .map_err(|e| format!("campaign: cannot read {}: {e}", manifest_path.display()))?;
        let nursery_text = String::from_utf8_lossy(&nursery_bytes).into_owned();
        if nursery_text != embedded_text {
            return Err(format!(
                "campaign: nursery manifest for candidate {id} does not match its \
                 bundle-embedded manifest byte for byte"
            ));
        }
    }

    // Rows 4, 5, 6: walk every receipt store, recompute every content
    // address, parse every receipt against the typed grammar, and collect
    // the evidence references for resolution.
    let mut stores: BTreeMap<String, ReceiptStore> = BTreeMap::new();
    let mut evidence_refs: Vec<(String, String)> = Vec::new();
    let mut receipt_count = 0usize;
    for candidate in &candidates {
        let id = candidate.id.as_str();
        let prefix = format!("{id}/receipts/");
        let mut store = ReceiptStore {
            kinds: BTreeSet::new(),
            receipts: Vec::new(),
        };
        for rel in nursery_before.files.keys() {
            if !rel.starts_with(&prefix) {
                continue;
            }
            let path = nursery_root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            let bytes = fs::read(&path)
                .map_err(|e| format!("campaign: cannot read {}: {e}", path.display()))?;
            let parsed = parse_receipt(rel, &bytes, id, &doc.judge_root_digest, &mut evidence_refs)?;
            store.kinds.insert(parsed.kind.clone());
            store.receipts.push(parsed);
            receipt_count += 1;
        }
        if store.kinds.contains("promotion") && store.kinds.contains("refusal") {
            return Err(format!(
                "campaign: candidate {id} carries both a promotion and a refusal \
                 receipt; one candidate cannot be both admitted and refused"
            ));
        }
        stores.insert(id.to_owned(), store);
    }

    // The ArchitectRequired presence law, adapted to receipts: the
    // denominator must show the escalation lane exercised, not absent.
    if !stores.values().any(|s| s.kinds.contains("escalation")) {
        return Err(
            "campaign: no ArchitectRequired escalation receipt is visible in the \
             denominator"
                .to_owned(),
        );
    }

    // Rows 4 and 5 for evidence: every reference resolves to exactly one
    // regular file whose bytes hash to the address.
    let mut by_stem: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for rel in evidence_before.files.keys() {
        if evidence_exempt.contains(rel) {
            continue;
        }
        if let Some((stem, _)) = rel.split_once('.') {
            by_stem.entry(stem).or_default().push(rel);
        }
    }
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for (address, source) in &evidence_refs {
        let matches: &[&str] = by_stem
            .get(address.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        match matches {
            [] => {
                return Err(format!(
                    "campaign: evidence reference {address} (from receipt {source}) \
                     resolves to no regular file under the evidence root"
                ));
            }
            [one] => {
                let path = evidence_root.join(one);
                let bytes = fs::read(&path)
                    .map_err(|e| format!("campaign: cannot read {}: {e}", path.display()))?;
                let actual = sha_hex(&bytes);
                if actual != *address {
                    return Err(format!(
                        "campaign: evidence file {one} does not hash to its content \
                         address (bytes are {actual})"
                    ));
                }
                referenced.insert((*one).to_owned());
            }
            many => {
                return Err(format!(
                    "campaign: evidence reference {address} (from receipt {source}) \
                     resolves to {} files; exactly one regular file is required",
                    many.len()
                ));
            }
        }
    }

    // The frontier and closure sections, then the ruled coherence walk.
    let frontier = parse_frontier(&doc.sections["frontier"], &candidates)?;
    let closure = parse_closure(&doc.sections["closure"], &candidates)?;
    check_terminal_representation(&candidates, &stores, &closure)?;
    check_coherence(&candidates, &stores, &frontier)?;
    for candidate in &candidates {
        let id = candidate.id.as_str();
        if closure.frontier[id] != frontier.state[id] {
            return Err(format!(
                "campaign: closure node frontier {} for candidate {id} does not match \
                 the frontier section's {}",
                closure.frontier[id], frontier.state[id]
            ));
        }
    }
    check_closure_edges(&candidates, &stores, &closure)?;
    recompute_frontier(&candidates, &stores, &frontier)?;

    // Row 13's orphan half: the evidence root carries referenced artifacts
    // and the bundle, nothing else. Append-only ACROSS the run is the
    // producer's snapshot law; append-only across THIS pass is the row-12
    // idiom applied below to the evidence root.
    for rel in evidence_before.files.keys() {
        if evidence_exempt.contains(rel) || referenced.contains(rel) {
            continue;
        }
        return Err(format!(
            "campaign: evidence file {rel} is unreferenced; the append-only evidence \
             root carries no orphan artifacts"
        ));
    }

    // Rows 12 and 13: re-walk both roots; any byte that moved while this
    // pass was looking voids the pass.
    let nursery_after = walk_root(nursery_root, "nursery")?;
    if nursery_after.files != nursery_before.files || nursery_after.dirs != nursery_before.dirs {
        return Err(
            "campaign: nursery root drifted during verification; persisted records are \
             byte-immutable"
                .to_owned(),
        );
    }
    let evidence_after = walk_root(evidence_root, "evidence")?;
    if evidence_after.files != evidence_before.files || evidence_after.dirs != evidence_before.dirs
    {
        return Err(
            "campaign: evidence root drifted during verification; the append-only \
             posture admits no mutation while the verifier looks"
                .to_owned(),
        );
    }

    let mut terminals: BTreeMap<&str, usize> = BTreeMap::new();
    let mut in_flight = 0usize;
    for candidate in &candidates {
        match derived_terminal(&stores[candidate.id.as_str()].kinds) {
            Some(t) => *terminals.entry(t).or_insert(0) += 1,
            None => in_flight += 1,
        }
    }
    println!("receiptcheck: PASS");
    println!("mode: campaign-verify");
    println!("grammar: {BUNDLE_MAGIC}");
    println!("judge-root-digest: {}", doc.judge_root_digest);
    println!("source-commit: {claimed}");
    println!("candidates: {}", candidates.len());
    println!("nursery-receipts: {receipt_count}");
    println!(
        "evidence-artifacts: {}",
        evidence_before.files.len().saturating_sub(
            evidence_before
                .files
                .keys()
                .filter(|rel| evidence_exempt.contains(*rel))
                .count()
        )
    );
    print!("terminals:");
    for (terminal, count) in &terminals {
        print!(" {terminal}={count}");
    }
    print!(" in-flight={in_flight}");
    println!();
    Ok(())
}

//! The HISTORICAL `BATPAK-CAMPAIGN-EVIDENCE/1` verifier arm (E7, TL-7).
//!
//! Why this arm exists: V1 grammars are NOT silently revised. The F5
//! mini-supernova produced its evidence under the `/1` wire grammar, and that
//! evidence remains real, verifiable history — so when E7 changed the grammar
//! it BUMPED the version (`/2`, DEC-064) rather than rewriting V1 under the
//! same magic. This module carries the original F5 verification logic intact:
//! a V1 bundle still verifies honestly (a tampered V1 bundle is still
//! refused, with the same named findings), but a V1 PASS is reclassified —
//! the bundle is HISTORICAL F5 evidence and is not admissible for Phase 6
//! opening. The live V2 verifier is `campaign.rs`; no V1 renderer survives in
//! spec.
//!
//! From the bytes on disk alone it validates: the `BATPAK-CAMPAIGN-EVIDENCE/1`
//! magic and exact section grammar; the frozen-judge digest binding (the judge
//! root's tree digest is RECOMPUTED and compared, and the before/after digests
//! must agree); the source-commit binding; every embedded
//! `BATPAK-CANDIDATE-MANIFEST/1` per the historical grammar (exact key order,
//! count-consistent repeats, 64-lowercase-hex addresses through the sealed
//! `Sha256Digest` parser, axis tokens matched against the typed sprouting
//! inventories, terminals against `CampaignTerminal::ALL`); terminal
//! denominator completeness; frontier consistency; closure-node/edge
//! well-formedness; and the rehearsal release-envelope completeness.
//!
//! Every refusal is a named finding; the arm fails closed.

use crate::artifact::strict_lines;
use crate::hashing::tree_digest;
use spec::campaign::{CampaignClosureEdgeKind, CampaignTerminal, EvidenceFreshness, FrontierState};
use spec::release::{EmptySetPosture, RELEASE_SEAL_FIELDS};
use spec::sprouting::{
    CANDIDATE_CHANGE_CLASSES, CANDIDATE_ORIGIN_KINDS, REALIZATION_POSTURES,
};
use spec::bootstrap_qualification::Sha256Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const BUNDLE_MAGIC: &str = "BATPAK-CAMPAIGN-EVIDENCE/1";
const ENVELOPE_MAGIC: &str = "BATPAK-CAMPAIGN-ENVELOPE/1";
const MANIFEST_MAGIC: &str = "BATPAK-CANDIDATE-MANIFEST/1";
const SECTIONS: [&str; 9] = [
    "judge", "source", "toolchain", "policy", "manifests", "candidates",
    "dispositions", "frontier", "closure",
];

struct BundleDoc {
    sections: BTreeMap<String, Vec<String>>,
    judge_root_digest: String,
}

fn split_sections(lines: &[String]) -> Result<BundleDoc, String> {
    if lines.first().map(String::as_str) != Some(BUNDLE_MAGIC) {
        return Err(format!(
            "campaign: bundle magic is not {BUNDLE_MAGIC:?} (found {:?})",
            lines.first()
        ));
    }
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

/// One parsed embedded candidate manifest: the facts the campaign checks
/// need. Parsing is strict and order-sensitive per the historical V1 grammar.
struct ParsedCandidate {
    id: String,
    parents: Vec<String>,
    dependencies: Vec<String>,
    terminal: String,
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

fn token_in(value: &str, inventory: &[String], axis: &str) -> Result<(), String> {
    if inventory.iter().any(|t| t == value) {
        return Ok(());
    }
    Err(format!("manifest {axis} token {value:?} is not in the typed inventory"))
}

fn debug_names<T: core::fmt::Debug>(inventory: &[T]) -> Vec<String> {
    inventory.iter().map(|v| format!("{v:?}")).collect()
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
    let parents = c.take_repeated("parent-count", "parent")?;
    for parent in &parents {
        hex64(parent, "parent")?;
    }
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
    hex64(&c.take("content-commitment")?, "content-commitment")?;
    token_in(&c.take("origin")?, &debug_names(CANDIDATE_ORIGIN_KINDS), "origin")?;
    token_in(
        &c.take("change-class")?,
        &debug_names(CANDIDATE_CHANGE_CLASSES),
        "change-class",
    )?;
    token_in(
        &c.take("realization-posture")?,
        &debug_names(REALIZATION_POSTURES),
        "realization-posture",
    )?;
    for evidence in c.take_repeated("evidence-count", "evidence")? {
        hex64(&evidence, "evidence")?;
    }
    for r in c.take_repeated("qualification-receipt-count", "qualification-receipt")? {
        hex64(&r, "qualification-receipt")?;
    }
    for r in c.take_repeated("holdout-receipt-count", "holdout-receipt")? {
        hex64(&r, "holdout-receipt")?;
    }
    hex64(&c.take("reuse-key")?, "reuse-key")?;
    let terminal_value = c.take("terminal")?;
    if c.pos != lines.len() {
        return Err(format!("manifest for {id} carries trailing lines"));
    }
    // Terminal denominator completeness: every admitted candidate has a
    // receipted terminal; `terminal none` is an in-flight record and cannot
    // stand in a CLOSED campaign bundle.
    if terminal_value == "none" {
        return Err(format!(
            "candidate {id} has no terminal; the campaign denominator is incomplete"
        ));
    }
    let (spelling, receipt) = terminal_value
        .split_once(' ')
        .ok_or_else(|| format!("terminal {terminal_value:?} is not `<spelling> <receipt>`"))?;
    let known: Vec<String> = CampaignTerminal::ALL
        .iter()
        .map(|t| t.spelling().to_owned())
        .collect();
    token_in(spelling, &known, "terminal")?;
    hex64(receipt, "terminal receipt")?;
    Ok(ParsedCandidate {
        id,
        parents,
        dependencies,
        terminal: spelling.to_owned(),
    })
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
    Ok(out)
}

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
        let recomputed = Sha256Digest::from_bytes(crate::hashing::sha256(&bytes)).render();
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

fn check_frontier(
    section: &[String],
    candidates: &[ParsedCandidate],
) -> Result<(), String> {
    let states: Vec<String> = FrontierState::ALL
        .iter()
        .map(|s| s.spelling().to_owned())
        .collect();
    let freshness: Vec<String> = EvidenceFreshness::ALL
        .iter()
        .map(|f| f.spelling().to_owned())
        .collect();
    let mut by_id: BTreeMap<&str, String> = BTreeMap::new();
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
        token_in(state, &states, "frontier state")
            .map_err(|e| format!("campaign: {e}"))?;
        let fresh = tokens
            .iter()
            .find_map(|t| t.strip_prefix("freshness="))
            .ok_or_else(|| format!("campaign: frontier line binds no freshness: {line:?}"))?;
        token_in(fresh, &freshness, "freshness").map_err(|e| format!("campaign: {e}"))?;
        if by_id.insert(id, state.to_owned()).is_some() {
            return Err(format!("campaign: duplicate frontier line for {id}"));
        }
    }
    for candidate in candidates {
        let state = by_id.get(candidate.id.as_str()).ok_or_else(|| {
            format!("campaign: candidate {} has no frontier line", candidate.id)
        })?;
        // Dependency-first law: later green cannot bless earlier red. A
        // Qualified candidate above a non-Qualified dependency is refused.
        if state == "Qualified" {
            for dep in &candidate.dependencies {
                match by_id.get(dep.as_str()) {
                    Some(dep_state) if dep_state == "Qualified" => {}
                    Some(dep_state) => {
                        return Err(format!(
                            "campaign: qualified candidate {} sits above {dep} whose \
                             frontier state is {dep_state} -- dependency-first violated",
                            candidate.id
                        ));
                    }
                    None => {
                        return Err(format!(
                            "campaign: qualified candidate {} depends on unknown {dep}",
                            candidate.id
                        ));
                    }
                }
            }
        }
    }
    if by_id.len() != candidates.len() {
        return Err("campaign: frontier lines do not cover exactly the candidates".to_owned());
    }
    Ok(())
}

fn check_closure(section: &[String], candidates: &[ParsedCandidate]) -> Result<(), String> {
    let ids: BTreeSet<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    let kinds: Vec<String> = CampaignClosureEdgeKind::ALL
        .iter()
        .map(|k| k.spelling().to_owned())
        .collect();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for line in section {
        if let Some(rest) = line.strip_prefix("node ") {
            let id = rest.split(' ').next().unwrap_or("");
            if !ids.contains(id) {
                return Err(format!("campaign: closure node {id} is not a candidate"));
            }
            nodes.insert(id);
            if rest.contains("terminal=none") {
                return Err(format!("campaign: closure node {id} carries no terminal"));
            }
        } else if let Some(rest) = line.strip_prefix("edge ") {
            let tokens: Vec<&str> = rest.split(' ').collect();
            let [from, to, kind] = tokens.as_slice() else {
                return Err(format!("campaign: closure edge is not `from to kind`: {line:?}"));
            };
            token_in(kind, &kinds, "closure edge kind").map_err(|e| format!("campaign: {e}"))?;
            if !ids.contains(*from) || !ids.contains(*to) {
                return Err(format!("campaign: closure edge endpoint unknown: {line:?}"));
            }
            // Lineage relations are DERIVED from required record fields: a
            // Parent/Dependency edge must be read back out of the manifest.
            let owner = candidates
                .iter()
                .find(|c| c.id == *from)
                .expect("edge endpoints were checked against the candidate set");
            if *kind == "Parent" && !owner.parents.iter().any(|p| p == to) {
                return Err(format!(
                    "campaign: Parent edge {from} -> {to} is not carried by the \
                     manifest's parent field"
                ));
            }
            if *kind == "Dependency" && !owner.dependencies.iter().any(|d| d == to) {
                return Err(format!(
                    "campaign: Dependency edge {from} -> {to} is not carried by a \
                     dependency commitment"
                ));
            }
        } else {
            return Err(format!("campaign: unexpected closure line {line:?}"));
        }
    }
    if nodes.len() != candidates.len() {
        return Err("campaign: closure nodes do not cover exactly the candidates".to_owned());
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

/// Verify one `BATPAK-CAMPAIGN-EVIDENCE/1` bundle as HISTORICAL F5 evidence.
/// The original four inputs; the original refusal behavior (a tampered V1
/// bundle still fails closed with its named finding); the PASS banner alone
/// is reclassified as historical.
pub(crate) fn mode_campaign_verify_historical(
    bundle: &Path,
    judge_root: &Path,
    envelope: &Path,
    source_commit: &str,
) -> Result<(), String> {
    let raw = fs::read(bundle)
        .map_err(|e| format!("campaign: cannot read bundle {}: {e}", bundle.display()))?;
    let lines = strict_lines(&raw)?;
    let doc = split_sections(&lines)?;

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

    check_policy(&doc.sections["policy"])?;
    check_manifest_section(&doc.sections["manifests"], judge_root)?;
    let candidates = parse_candidates(&doc.sections["candidates"])?;
    let architect = candidates
        .iter()
        .filter(|c| c.terminal == "ArchitectRequired")
        .count();
    if architect == 0 {
        return Err(
            "campaign: no ArchitectRequired terminal is visible in the denominator"
                .to_owned(),
        );
    }
    check_dispositions(&doc.sections["dispositions"])?;
    check_frontier(&doc.sections["frontier"], &candidates)?;
    check_closure(&doc.sections["closure"], &candidates)?;
    check_envelope(envelope)?;

    let mut terminals: BTreeMap<&str, usize> = BTreeMap::new();
    for candidate in &candidates {
        *terminals.entry(candidate.terminal.as_str()).or_insert(0) += 1;
    }
    println!("receiptcheck: PASS");
    println!("mode: campaign-verify");
    println!("grammar: {BUNDLE_MAGIC} (historical arm)");
    println!(
        "classification: this bundle is verified as HISTORICAL F5 evidence under the \
         retired /1 grammar; it is not admissible for Phase 6 opening"
    );
    println!("judge-root-digest: {}", doc.judge_root_digest);
    println!("source-commit: {claimed}");
    println!("candidates: {}", candidates.len());
    print!("terminals:");
    for (terminal, count) in &terminals {
        print!(" {terminal}={count}");
    }
    println!();
    Ok(())
}

use spec::{architecture, guarantees, invariants, proof};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use crate::corpus::guarantee_ref_resolves;

/// The proof-identity catalog is the living census (5.5E2j). Executed laws:
/// no identity is both active and retired (or declared twice at all); both
/// lifecycle states are constructed; every retirement names at least one
/// successor and never itself; every successor resolves INSIDE the catalog;
/// and the catalog's active side equals the structurally parsed canonical
/// A libtest count proves every currently declared test executed; this
/// catalog proves no required proof identity disappeared. Since 5.5E4d the
/// typed catalog is the one membership authority — different axes, each with
/// its own owner.
pub(crate) fn check_proof_rows(_root: &Path, findings: &mut Vec<String>) {
    use proof::{ProofRowState, PROOF_ROWS};
    let mut active: BTreeSet<&str> = BTreeSet::new();
    let mut retired: BTreeSet<&str> = BTreeSet::new();
    for record in PROOF_ROWS {
        // Exhaustive: a new state must be classified here, not defaulted.
        let side = match record.state {
            ProofRowState::Active { .. } => &mut active,
            ProofRowState::Retired { .. } => &mut retired,
        };
        if !side.insert(record.id.raw()) {
            findings.push(format!(
                "{} is declared twice in the proof-identity catalog",
                record.id.raw()
            ));
        }
    }
    for both in active.intersection(&retired) {
        findings.push(format!(
            "{both} is both active and retired; one identity carries one lifecycle"
        ));
    }
    if active.is_empty() {
        findings.push("the proof-identity catalog constructs no Active row; the census was deleted".into());
    }
    if retired.is_empty() {
        findings.push("the proof-identity catalog constructs no Retired row; the retirement ledger was deleted".into());
    }
    for record in PROOF_ROWS {
        if let ProofRowState::Retired { successors } = record.state {
            if successors.is_empty() {
                findings.push(format!(
                    "{} is retired with no successor; retirement is supersession \
                     with a forwarding address, never deletion",
                    record.id.raw()
                ));
            }
            for successor in successors {
                if successor.raw() == record.id.raw() {
                    findings.push(format!("{} names itself as its successor", record.id.raw()));
                } else if !active.contains(successor.raw()) && !retired.contains(successor.raw())
                {
                    findings.push(format!(
                        "{} names successor {}, which resolves to no typed catalog \
                         identity",
                        record.id.raw(),
                        successor.raw()
                    ));
                }
            }
        }
    }
    // Succession TERMINATES (E3 preflight, a permanent re-entry guard). The
    // per-edge laws above cannot see a two-node cycle: retired A naming
    // retired B while B names A satisfies existence and non-self-succession
    // while owning no living obligation. Every retirement path must reach at
    // least one Active identity, and the retired-to-retired succession graph
    // must be acyclic.
    let mut successors_of: Vec<(&str, &[proof::ProofRowId])> = Vec::new();
    for record in PROOF_ROWS {
        if let ProofRowState::Retired { successors } = record.state {
            successors_of.push((record.id.raw(), successors));
        }
    }
    let edges = |id: &str| -> &[proof::ProofRowId] {
        successors_of
            .iter()
            .find(|(from, _)| *from == id)
            .map(|(_, s)| *s)
            .unwrap_or(&[])
    };
    for (start, _) in &successors_of {
        let mut frontier = vec![*start];
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut reaches_active = false;
        let mut cyclic = false;
        while let Some(id) = frontier.pop() {
            for successor in edges(id) {
                let s = successor.raw();
                if active.contains(s) {
                    reaches_active = true;
                } else if s == *start {
                    cyclic = true;
                } else if seen.insert(s) {
                    frontier.push(s);
                }
            }
        }
        if cyclic {
            findings.push(format!(
                "retirement succession is cyclic: {start} participates in a cycle"
            ));
        }
        if !reaches_active {
            findings.push(format!(
                "{start} retirement path terminates in no Active identity"
            ));
        }
    }
    // Membership parity against docs/24 fences retired in 5.5E4d: the typed
    // catalog is the ONE membership authority, docs/24 owns meaning only, and
    // audit.py proves the meaning entries cover every active row.
}

/// Typed witness citations RESOLVE (5.5E2). A witness names WHICH owned
/// evidence obligation a law depends on; a citation of an undeclared
/// guarantee, an unknown contract id, or a tool that does not exist in the
/// checked tree is refused at runtime, every run. The note carries the human
/// reading and carries no law.
/// The declared contract ids, read from the same inventory the front-matter
/// law walks. Shared by the witness-citation and identity-catalog laws.
pub(crate) fn declared_contract_ids(root: &Path) -> BTreeSet<String> {
    let mut contract_ids = BTreeSet::new();
    for relative in architecture::REQUIRED_DOCS {
        if !relative.ends_with(".md") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(root.join(relative)) {
            for line in text.lines().take(16) {
                if let Some(id) = line.strip_prefix("contract_id:") {
                    contract_ids.insert(id.trim().to_string());
                }
            }
        }
    }
    contract_ids
}

/// contract_id -> the authored text of its authoritative document, with
/// generated projection blocks removed (5.5E3d2). The owner must AUTHOR the
/// term it claims, and a generated block can never notarize its own owner
/// claim.
pub(crate) fn contract_authored_texts(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for relative in architecture::REQUIRED_DOCS {
        if !relative.ends_with(".md") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(root.join(relative)) {
            let id = text.lines().take(16).find_map(|line| {
                line.strip_prefix("contract_id:").map(|s| s.trim().to_string())
            });
            if let Some(id) = id {
                out.insert(id, strip_generated_blocks(&text));
            }
        }
    }
    out
}

/// Remove `<!-- NAME:BEGIN ... --> ... <!-- NAME:END -->` regions, which are
/// line-delimited in every authoritative document.
fn strip_generated_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for line in text.lines() {
        if !skipping && line.starts_with("<!-- ") && line.contains(":BEGIN") {
            skipping = true;
            continue;
        }
        if skipping {
            if line.starts_with("<!-- ") && line.contains(":END -->") {
                skipping = false;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Exact token occurrence: the neighbors of a match may not be identifier
/// characters, so prose "id" and longer identifiers mint nothing.
pub(crate) fn authors_token(text: &str, term: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(pos) = text[start..].find(term) {
        let i = start + pos;
        let j = i + term.len();
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        if (i == 0 || !word(bytes[i - 1])) && (j >= bytes.len() || !word(bytes[j])) {
            return true;
        }
        start = i + 1;
    }
    false
}

pub(crate) fn check_witness_citations(root: &Path, findings: &mut Vec<String>) {
    use guarantees::WitnessRef;
    let contract_ids = declared_contract_ids(root);
    for row in invariants::INVARIANTS {
        // SEED's witness posture is RowDeclared: an empty citation list is an
        // omission, never a policy.
        if row.witnesses.is_empty() {
            findings.push(format!("{} declares no witness citation", row.id));
        }
        for witness in row.witnesses {
            // Exhaustive: a new witness kind must be resolved here.
            match witness {
                WitnessRef::Guarantee(reference) => {
                    if !guarantee_ref_resolves(*reference) {
                        findings.push(format!(
                            "{} witness references {reference:?}, which no declared row owns",
                            row.id
                        ));
                    }
                }
                WitnessRef::Contract(id) => {
                    if !contract_ids.contains(id.raw()) {
                        findings.push(format!(
                            "{} witness cites contract {}, which no document declares",
                            row.id,
                            id.raw()
                        ));
                    }
                }
                WitnessRef::BootstrapTool(tool) => {
                    if !root.join(tool.path()).is_file() {
                        findings.push(format!(
                            "{} witness cites {}, which does not exist at {}",
                            row.id,
                            tool.display(),
                            tool.path()
                        ));
                    }
                }
            }
        }
    }
}

/// SEED-AUDITED-DENOMINATOR (5.5E1): the proof-terminal vocabulary is typed
/// and only Passed is the positive semantic terminal -- the fence is executed
/// through the real is_positive_semantic_terminal() (renamed from
/// counts_green in 5.5F2), so reclassifying a terminal reddens the running
/// binary. The positive terminal is one qualification input, never the
/// verdict: requirement qualification is spec/verification/ law.
pub(crate) fn check_proof_terminals(findings: &mut Vec<String>) {
    use proof::ProofUnitTerminal as T;
    let mut seen = BTreeSet::new();
    for t in proof::PROOF_UNIT_TERMINALS {
        // Exhaustive: a new terminal must be classified here, not defaulted.
        match t {
            T::Passed | T::Failed | T::Refused | T::Unsupported
            | T::SkippedWithAuthority | T::Expired | T::Superseded => {}
        }
        if !seen.insert(format!("{t:?}")) {
            findings.push(format!("proof terminal {t:?} is listed twice"));
        }
    }
    if seen.len() != 7 {
        findings.push(format!(
            "a proof terminal is missing from PROOF_UNIT_TERMINALS ({} of 7)", seen.len()));
    }
    if !T::Passed.is_positive_semantic_terminal() {
        findings.push("Passed is no longer the positive semantic terminal; the denominator is vacuous".into());
    }
    for t in [T::Failed, T::Refused, T::Unsupported, T::SkippedWithAuthority,
              T::Expired, T::Superseded] {
        if t.is_positive_semantic_terminal() {
            findings.push(format!(
                "proof terminal {t:?} is positive; only Passed may be (SEED-AUDITED-DENOMINATOR)"));
        }
    }
}


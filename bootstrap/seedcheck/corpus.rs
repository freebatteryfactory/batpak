use spec::{architecture, contracts, dispositions, gates, guarantees, invariants, legacy_obligations, toolchain};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use crate::proof::{authors_token, contract_authored_texts, declared_contract_ids};

/// The admitted compiler-assumption kinds execute their own law (5.5E3h):
/// spellings unique and authored by the semantic owner, one admission basis
/// per kind that NAMES the kind, only UnsafeMemoryContract intrinsically
/// requiring the SAFETY-CONTRACT marker, classification at G3, and release
/// qualification at G9.
pub(crate) fn check_compiler_assumptions(root: &Path, findings: &mut Vec<String>) {
    use spec::compiler_assumptions::CompilerAssumptionKind;
    let contract_ids = declared_contract_ids(root);
    let owner_texts = contract_authored_texts(root);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for kind in CompilerAssumptionKind::ALL {
        let s = kind.spelling();
        if s.trim().is_empty() {
            findings.push("an assumption kind projects an empty spelling".to_string());
        }
        if !seen.insert(s) {
            findings.push(format!("assumption-kind spelling {s} is claimed twice"));
        }
        let owner = kind.semantic_owner();
        if !contract_ids.contains(owner.raw()) {
            findings.push(format!(
                "assumption kind {s} cites owner {}, which no document declares",
                owner.raw()
            ));
        } else if !owner_texts
            .get(owner.raw())
            .is_some_and(|text| authors_token(text, s))
        {
            findings.push(format!(
                "assumption kind {s} cites owner {}, whose authoritative document \
                 does not author the spelling",
                owner.raw()
            ));
        }
        match kind.admission_basis() {
            guarantees::GuaranteeRef::Decision(decision) => {
                match dispositions::DECISIONS.iter().find(|d| d.id == decision.raw()) {
                    None => findings.push(format!(
                        "assumption kind {s} cites admission basis {}, which no \
                         declared decision owns",
                        decision.raw()
                    )),
                    Some(d) => {
                        let named = authors_token(d.subject, s)
                            || authors_token(d.successor, s)
                            || d.replacement_contract.is_some_and(|rc| authors_token(rc, s));
                        if !named {
                            findings.push(format!(
                                "assumption kind {s} cites admission basis {}, whose \
                                 forward-policy fields do not name the kind",
                                decision.raw()
                            ));
                        }
                    }
                }
            }
            other => findings.push(format!(
                "assumption kind {s} cites a non-decision admission basis {other:?}"
            )),
        }
        let is_unsafe_contract = matches!(kind, CompilerAssumptionKind::UnsafeMemoryContract);
        if kind.requires_safety_contract_marker() != is_unsafe_contract {
            findings.push(if is_unsafe_contract {
                "UnsafeMemoryContract must intrinsically require the SAFETY-CONTRACT \
                 marker"
                    .to_string()
            } else {
                format!(
                    "only UnsafeMemoryContract intrinsically requires the \
                     SAFETY-CONTRACT marker; {s} claims it"
                )
            });
        }
        if kind.classification_gate() != gates::GateId::G3 {
            findings.push(format!(
                "assumption kind {s} does not classify at G3; the ledger boundary is \
                 TestPak's"
            ));
        }
        if kind.release_qualification_gate() != gates::GateId::G9 {
            findings.push(format!(
                "assumption kind {s} does not qualify for release at G9; the release \
                 seal consumes the ledger"
            ));
        }
    }
}

/// The corpus reconciliation epoch executes its own law (5.5E3g): the
/// CURRENT selection is a declared variant, spellings are unique and
/// nonempty, the constitutional owner resolves, and the admission basis
/// resolves to a declared seed row. Frontmatter traversal stays audit and
/// projector work — seedcheck grows no Markdown parser to imitate them.
pub(crate) fn check_corpus(root: &Path, findings: &mut Vec<String>) {
    use spec::corpus::{ReconciliationEpoch, CURRENT_RECONCILIATION_EPOCH};
    let contract_ids = declared_contract_ids(root);
    if !ReconciliationEpoch::ALL
        .iter()
        .any(|e| *e == CURRENT_RECONCILIATION_EPOCH)
    {
        findings.push(
            "CURRENT_RECONCILIATION_EPOCH is not declared in ReconciliationEpoch::ALL"
                .to_string(),
        );
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for epoch in ReconciliationEpoch::ALL {
        let s = epoch.spelling();
        if s.trim().is_empty() {
            findings.push("a corpus epoch projects an empty spelling".to_string());
        }
        if !seen.insert(s) {
            findings.push(format!(
                "epoch spelling {s} is claimed twice; two epochs cannot share a name"
            ));
        }
        let owner = epoch.semantic_owner();
        if !contract_ids.contains(owner.raw()) {
            findings.push(format!(
                "epoch {s} cites owner {}, which no document declares",
                owner.raw()
            ));
        }
        match epoch.admission_basis() {
            guarantees::GuaranteeRef::Seed(seed) => {
                if !invariants::INVARIANTS.iter().any(|r| r.id == seed.raw()) {
                    findings.push(format!(
                        "epoch {s} cites admission basis {}, which no declared seed \
                         row owns",
                        seed.raw()
                    ));
                }
            }
            other => findings.push(format!(
                "epoch {s} cites a non-seed admission basis {other:?}; corpus \
                 membership is document-status law"
            )),
        }
    }
}

/// The mutation vocabulary executes its own law (5.5E3f): the lane facts
/// and result classifications are total const functions, so seedcheck runs
/// the REAL functions and asserts the semantic fences — no shadow table can
/// drift because none exists.
pub(crate) fn check_mutation(root: &Path, findings: &mut Vec<String>) {
    use spec::mutation::{MutationLane, MutationResult};
    let contract_ids = declared_contract_ids(root);
    let owner_texts = contract_authored_texts(root);
    let mut lane_seen = BTreeSet::new();
    for lane in MutationLane::ALL {
        let s = lane.spelling();
        if s.trim().is_empty() {
            findings.push("a mutation lane projects an empty spelling".to_string());
        }
        if !lane_seen.insert(s) {
            findings.push(format!("lane spelling {s} is claimed twice"));
        }
        let owner = lane.semantic_owner();
        if !contract_ids.contains(owner.raw()) {
            findings.push(format!(
                "lane {s} cites owner {}, which no document declares",
                owner.raw()
            ));
        } else if !owner_texts
            .get(owner.raw())
            .is_some_and(|text| authors_token(text, s))
        {
            findings.push(format!(
                "lane {s} cites owner {}, whose authoritative document does not \
                 author the spelling",
                owner.raw()
            ));
        }
        let basis = lane.admission_basis();
        match dispositions::DECISIONS.iter().find(|d| d.id == basis.raw()) {
            None => findings.push(format!(
                "lane {s} cites admission basis {}, which no declared decision owns",
                basis.raw()
            )),
            Some(d) => {
                let named = authors_token(d.subject, s)
                    || authors_token(d.successor, s)
                    || d.replacement_contract.is_some_and(|rc| authors_token(rc, s));
                if !named {
                    findings.push(format!(
                        "{} forward-policy fields do not name lane {s}; the \
                         admission boundary must name what it admits",
                        basis.raw()
                    ));
                }
            }
        }
        if !lane.requires_activation_evidence() {
            findings.push(format!(
                "lane {s} does not require activation evidence; a green test with \
                 a dormant mutant is not evidence"
            ));
        }
        if lane.permits_production_profile_slots() {
            findings.push(format!("lane {s} permits production-profile mutation slots"));
        }
        if !lane.requires_independent_evidence_route() {
            findings.push(format!("lane {s} does not require an independent evidence route"));
        }
        if lane.gates() != [gates::GateId::G3] {
            findings.push(format!("lane {s} gates are not exactly G3"));
        }
    }
    if MutationLane::SemanticIr.requires_real_rustc_semantics() {
        findings.push(
            "SemanticIr claims real rustc semantics; the reference interpreter \
             lane must not"
                .to_string(),
        );
    }
    if MutationLane::SemanticIr.requires_per_candidate_rust_compile() {
        findings.push(
            "SemanticIr requires a per-candidate Rust compile; the semantic lane \
             compiles nothing"
                .to_string(),
        );
    }
    if !MutationLane::SelectableCompiled.requires_real_rustc_semantics() {
        findings.push("SelectableCompiled must run under real rustc semantics".to_string());
    }
    if MutationLane::SelectableCompiled.requires_per_candidate_rust_compile() {
        findings.push(
            "SelectableCompiled must not require a per-candidate Rust compile; \
             one shard compiles once"
                .to_string(),
        );
    }
    if !MutationLane::CompilerBacked.requires_per_candidate_rust_compile() {
        findings.push("CompilerBacked must require a per-candidate Rust compile".to_string());
    }
    if !MutationLane::CompilerBacked.requires_real_rustc_semantics() {
        findings.push("CompilerBacked must run under real rustc semantics".to_string());
    }
    let mut result_seen = BTreeSet::new();
    for result in MutationResult::ALL {
        let s = result.spelling();
        if s.trim().is_empty() {
            findings.push("a mutation result projects an empty spelling".to_string());
        }
        if !result_seen.insert(s) {
            findings.push(format!("result spelling {s} is claimed twice"));
        }
        let owner = result.semantic_owner();
        if !contract_ids.contains(owner.raw()) {
            findings.push(format!(
                "result {s} cites owner {}, which no document declares",
                owner.raw()
            ));
        } else if !owner_texts
            .get(owner.raw())
            .is_some_and(|text| authors_token(text, s))
        {
            findings.push(format!(
                "result {s} cites owner {}, whose authoritative document does not \
                 author the spelling",
                owner.raw()
            ));
        }
        let is_killed = matches!(result, MutationResult::Killed);
        let is_survived = matches!(result, MutationResult::Survived);
        if result.counts_as_kill() != is_killed {
            findings.push(if is_killed {
                "Killed must count as a kill".to_string()
            } else {
                format!("only Killed counts as a kill; {s} claims one")
            });
        }
        if result.counts_as_survival() != is_survived {
            findings.push(if is_survived {
                "Survived must count as survival".to_string()
            } else {
                format!("only Survived counts as survival; {s} claims one")
            });
        }
        if !result.appears_in_denominator() {
            findings.push(format!(
                "{s} leaves the denominator; nothing exits silently to improve a score"
            ));
        }
    }
}

/// The admitted contract kinds are coherent (5.5E3c): every variant is
/// classified, spellings are unique and nonempty, and every kind's
/// admitting guarantee RESOLVES through the same law the relations and
/// witnesses use. Lifecycle law: a DELETED or DANGLING basis refuses the
/// kind; a lawfully closed, superseded, or historically retained basis
/// still resolves and the kind remains admitted.
pub(crate) fn check_contract_kinds(findings: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for kind in contracts::ContractKind::ALL {
        // Exhaustive: a new kind must be classified here, must join ALL, and
        // must cite the guarantee that admits it — not default.
        match kind {
            contracts::ContractKind::Error
            | contracts::ContractKind::Event
            | contracts::ContractKind::SchemaCodec
            | contracts::ContractKind::Projection
            | contracts::ContractKind::Subscription
            | contracts::ContractKind::OperationEffect
            | contracts::ContractKind::Process
            | contracts::ContractKind::Composition => {}
        }
        if kind.spelling().trim().is_empty() {
            findings.push(format!("{kind:?} projects an empty spelling"));
        }
        if !seen.insert(kind.spelling()) {
            findings.push(format!(
                "two contract kinds project the spelling {}",
                kind.spelling()
            ));
        }
        let basis = kind.admission_basis();
        if !guarantee_ref_resolves(basis) {
            findings.push(format!(
                "{kind:?} cites admission basis {basis:?}, which resolves \
                 to no declared row"
            ));
        }
    }
}

/// Whether a typed reference names a DECLARED row of its family. The type
/// carries the family; this function carries existence. Exhaustive: a new
/// family must be resolved here, not defaulted.
pub(crate) fn guarantee_ref_resolves(reference: guarantees::GuaranteeRef) -> bool {
    match reference {
        guarantees::GuaranteeRef::Seed(id) => {
            invariants::INVARIANTS.iter().any(|r| r.id == id.raw())
        }
        guarantees::GuaranteeRef::Legacy(id) => {
            legacy_obligations::OBLIGATIONS.iter().any(|r| r.id == id.raw())
        }
        guarantees::GuaranteeRef::Decision(id) => {
            dispositions::DECISIONS.iter().any(|r| r.id == id.raw())
        }
        guarantees::GuaranteeRef::Architecture(id) => {
            architecture::PACKAGES.iter().any(|p| p.id == id.package())
        }
        guarantees::GuaranteeRef::Qualification(id) => architecture::QUALIFICATION_PROFILES
            .iter()
            .any(|q| q.package == id.package() && q.profile == id.profile()),
    }
}

/// The typed toolchain owner is coherent and its tracked projection is
/// byte-exact (5.5E3a, type-baked in 5.5E3a1). The string-shape validators
/// died with the strings: a malformed edition or resolver has no spelling
/// now. What remains executable is what construction cannot say — the
/// exact-release/floor relation, component uniqueness, the tracked
/// projection's byte equality, and the environment spellings staying
/// capability profiles rather than target triples. Every refusal names the
/// violated field, the typed owner, the observed and required values, and
/// the repair direction.
pub(crate) fn check_toolchain(root: &Path, findings: &mut Vec<String>) {
    let t = toolchain::TOOLCHAIN;
    let refuse = |findings: &mut Vec<String>, field: &str, observed: &str, required: &str,
                  repair: &str| {
        findings.push(format!(
            "toolchain {field} violated (owner: spec/toolchain/types.rs ToolchainProfile;              observed: {observed}; required: {required}; repair: {repair})"
        ));
    };
    // The exact qualifying release and the MSRV floor answer DIFFERENT
    // questions. The law is exact >= floor — a newer qualifying compiler
    // that preserves the floor is ordinary; one below the floor would
    // qualify a foundation its own consumers may lawfully refuse.
    if !t.exact_rust_release.satisfies_floor(t.rust_version_floor) {
        refuse(
            findings,
            "exact_rust_release",
            &t.exact_rust_release.render(),
            &format!("at least the declared MSRV floor {}", t.rust_version_floor.render()),
            "qualify with a compiler at or above the floor the generated              workspace claims",
        );
    }
    if toolchain::RustupComponent::ALL.is_empty() {
        refuse(findings, "RustupComponent::ALL", "[]", "at least one component",
               "author the components qualification depends on");
    }
    let mut seen_components = BTreeSet::new();
    for component in toolchain::RustupComponent::ALL {
        // Exhaustive: a new component variant must join this classification
        // (and the auditor separately requires it to join ALL), not default.
        match component {
            toolchain::RustupComponent::Clippy | toolchain::RustupComponent::Rustfmt => {}
        }
        if !seen_components.insert(component.spelling()) {
            refuse(findings, "RustupComponent::ALL", component.spelling(),
                   "each component once",
                   "a duplicated component is a copy, not a requirement");
        }
    }
    // The tracked root selection is a PROJECTION: byte-equal or refused.
    let want = t.tracked_root_toolchain_toml();
    match fs::read_to_string(root.join("rust-toolchain.toml")) {
        Ok(tracked) if tracked == want => {}
        Ok(_) => refuse(
            findings,
            "tracked rust-toolchain.toml",
            "hand-edited or stale bytes",
            "the deterministic projection of ToolchainProfile",
            "regenerate the tracked file from the typed owner; never edit it by hand",
        ),
        Err(_) => refuse(
            findings,
            "tracked rust-toolchain.toml",
            "absent",
            "the deterministic projection of ToolchainProfile",
            "the tracked toolchain projection selects the compiler before the              spec can compile; restore it from the typed owner",
        ),
    }
    // Environment membership dissolved into construction (a triple has no
    // QualificationEnvironment spelling). What remains checkable is the
    // authored SPELLINGS themselves: a capability profile must never be
    // renamed into a target triple.
    for environment in architecture::QualificationEnvironment::ALL {
        let spelling = environment.spelling();
        if spelling.contains('-') && spelling.split('-').count() >= 3 {
            refuse(findings, "QualificationEnvironment::spelling", spelling,
                   "a capability environment, never a target triple",
                   "qualification environments answer WHAT holds, not WHERE it ran");
        }
    }
}



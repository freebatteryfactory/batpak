#![deny(warnings)]

// The typed specification is a LIBRARY (spec/lib.rs, 5.5E2): this binary
// links it instead of textually mounting its modules, so no tracked
// suppression exists and every pub spec item is API by construction.
use spec::{
    architecture, commands, contracts, dispositions, identities, gates, guarantees,
    invariants, legacy_invariant_coverage, legacy_obligations, operators, pakvm_isa,
    proof, reconciliation, syncbat_firewall, toolchain,
};

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let root = env::args().nth(1).map_or_else(|| PathBuf::from("."), PathBuf::from);
    let findings = inspect(&root);
    if findings.is_empty() {
        let production = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::Production).count();
        let adapters = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::BinaryAdapter).count();
        let dev_only = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::DevOnly).count();
        let examples = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::Example).count();
        println!(
            "seedcheck: PASS {} v{} train {} ({} packages: {} production, {} adapter, {} dev-only, {} example; {} edges; {} qualification profiles; {} invariants; {} decisions; {} legacy obligations; {} legacy invariant dispositions; {} operators)",
            architecture::REPOSITORY_NAME,
            architecture::SPEC_VERSION,
            architecture::WORKSPACE_VERSION,
            architecture::PACKAGES.len(),
            production,
            adapters,
            dev_only,
            examples,
            architecture::EDGES.len(),
            architecture::QUALIFICATION_PROFILES.len(),
            invariants::INVARIANTS.len(),
            dispositions::DECISIONS.len(),
            legacy_obligations::OBLIGATIONS.len(),
            legacy_invariant_coverage::COVERAGE.len(),
            operators::OPERATORS.len(),
        );
        return;
    }
    eprintln!("seedcheck: FAIL ({} finding(s))", findings.len());
    for finding in findings { eprintln!("- {finding}"); }
    std::process::exit(1);
}

fn inspect(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for relative in architecture::REQUIRED_DOCS {
        if !root.join(relative).is_file() { findings.push(format!("missing required file {relative}")); }
    }
    for relative in architecture::FORBIDDEN_TARGET_PATHS {
        if root.join(relative).exists() { findings.push(format!("forbidden target path exists: {relative}")); }
    }
    check_graph(&mut findings);
    check_profiles(&mut findings);
    check_authenticated_history(&mut findings);
    check_version(&mut findings);
    check_pakvm_isa(&mut findings);
    check_syncbat_firewall(&mut findings);
    check_syncbat_origin_law(&mut findings);
    check_unique_ids(&mut findings);
    check_candidate_containment(&mut findings);
    check_reconciliation(&mut findings);
    check_release_seal(&mut findings);
    check_proof_terminals(&mut findings);
    check_guarantee_admission(&mut findings);
    check_frontmatter(root, &mut findings);
    check_witness_citations(root, &mut findings);
    check_proof_rows(root, &mut findings);
    check_toolchain(root, &mut findings);
    check_contract_kinds(&mut findings);
    check_identity_catalogs(root, &mut findings);
    check_commands(root, &mut findings);
    check_mutation(root, &mut findings);
    check_corpus(root, &mut findings);
    check_compiler_assumptions(root, &mut findings);
    check_promotion(root, &mut findings);
    check_syncbat_shape(root, &mut findings);
    check_source_debt(root, &mut findings);
    findings
}

/// The cross-family guarantee admission law, EXECUTED (5.5E2 bake). Until
/// this check existed, `spec::guarantees::admit` had Rust vocabulary and a
/// Python reconstruction but no Rust execution path: every native row across
/// all five families now passes through the real sealed admission, and the
/// refusals are exercised with hostile sources so the reachable-refusal
/// question has an executed answer, not a plausible one.
fn check_guarantee_admission(findings: &mut Vec<String>) {
    use guarantees::{admit, GuaranteeAdmissionRule, GuaranteeSource};
    let mut admitted = 0usize;
    let refuse = |findings: &mut Vec<String>, e: guarantees::GuaranteeAdmissionFailure| {
        findings.push(format!(
            "guarantee {:?} does not admit: {} (owner: {}; repair: {})",
            e.id,
            e.rule.law(),
            e.rule.owner(),
            e.rule.repair()
        ));
    };
    // Rows are carried WHOLE under their family variant (5.5E2c): the driver
    // no longer copies fields into an option bag, so it cannot mis-wire one,
    // and ARCH/QUAL identity is structural — the leaked "ARCH-{package}"
    // format strings died with the flat source.
    for row in invariants::INVARIANTS {
        match admit(GuaranteeSource::Seed(row)) {
            Ok(_) => admitted += 1,
            Err(e) => refuse(findings, e),
        }
    }
    for row in legacy_obligations::OBLIGATIONS {
        match admit(GuaranteeSource::Legacy(row)) {
            Ok(_) => admitted += 1,
            Err(e) => refuse(findings, e),
        }
    }
    for row in dispositions::DECISIONS {
        match admit(GuaranteeSource::Decision(row)) {
            Ok(_) => admitted += 1,
            Err(e) => refuse(findings, e),
        }
    }
    for pkg in architecture::PACKAGES {
        match admit(GuaranteeSource::Architecture(pkg)) {
            Ok(_) => admitted += 1,
            Err(e) => refuse(findings, e),
        }
    }
    for q in architecture::QUALIFICATION_PROFILES {
        match admit(GuaranteeSource::Qualification(q)) {
            Ok(_) => admitted += 1,
            Err(e) => refuse(findings, e),
        }
    }
    let expected = invariants::INVARIANTS.len()
        + legacy_obligations::OBLIGATIONS.len()
        + dispositions::DECISIONS.len()
        + architecture::PACKAGES.len()
        + architecture::QUALIFICATION_PROFILES.len();
    if admitted != expected {
        findings.push(format!(
            "guarantee admission admitted {admitted} of {expected} native rows"
        ));
    }
    // Typed relations RESOLVE (5.5E2). The reference type carries the family
    // — a decision cited as a legacy obligation has no spelling — and this
    // law carries existence: a reference to an undeclared row is refused at
    // runtime, every run, through the sealed accessors.
    for row in invariants::INVARIANTS {
        for (rel, refs) in [
            ("derives_from", row.derives_from),
            ("refines", row.refines),
            ("discharges", row.discharges),
            ("supersedes", row.supersedes),
        ] {
            for reference in refs {
                if !guarantee_ref_resolves(*reference) {
                    findings.push(format!(
                        "{} {rel} references {reference:?}, which no declared row owns",
                        row.id
                    ));
                }
            }
        }
    }
    // Reachable refusals, exercised through the production door. Each hostile
    // row must refuse for ITS OWN typed rule — `is_err()` alone would let an
    // unrelated earlier refusal turn the fixture green for free. The
    // UNKNOWN-family and agreeing-duplicate hostiles from the first bake are
    // gone because they became UNREPRESENTABLE: a GuaranteeSource is one of
    // five family variants, so an undeclared family or a second author for a
    // family-owned field no longer has a spelling.
    let hostile = |findings: &mut Vec<String>,
                       name: &str,
                       source: GuaranteeSource,
                       rule: GuaranteeAdmissionRule| {
        match admit(source) {
            Ok(_) => findings.push(format!("hostile row was admitted: {name}")),
            Err(e) if e.rule != rule => findings.push(format!(
                "the {name} refusal fired for the wrong rule: {} (wanted: {})",
                e.rule.law(),
                rule.law()
            )),
            Err(_) => {}
        }
    };
    static GATELESS_ENFORCEMENT_DEC: dispositions::DecisionSpec = dispositions::DecisionSpec {
        id: "DEC-000-HOSTILE",
        class: dispositions::DecisionClass::Enforcement,
        gates: &[],
        disposition: dispositions::Disposition::Lock,
        subject: "hostile",
        successor: "hostile",
        stale_aliases: &[],
        stale_allowed_contexts: &[],
        replacement_contract: None,
    };
    hostile(
        findings,
        "gateless implementation-bearing decision",
        GuaranteeSource::Decision(&GATELESS_ENFORCEMENT_DEC),
        GuaranteeAdmissionRule::ImplementationBearingDecisionNamesGate,
    );
    // The TARGETLESS_QUAL hostile died in 5.5E3a1: a QualificationProfile
    // carries a closed QualificationEnvironment, so a missing or empty
    // environment has no spelling — the refusal it exercised became a
    // compile error, which is the strongest form of the fixture.
    static OWNERLESS_LEG: legacy_obligations::LegacyObligation =
        legacy_obligations::LegacyObligation {
            id: "LEG-000-HOSTILE",
            law: "hostile",
            clean_owner: "  ",
            gates: &[gates::GateId::G2],
            compatibility_disposition: legacy_obligations::CompatibilityDisposition::None,
            deletion_condition: legacy_obligations::DeletionCondition::Never,
            active_or_closed_status: legacy_obligations::ObligationStatus::Active,
        };
    hostile(
        findings,
        "legacy obligation with a blank owner",
        GuaranteeSource::Legacy(&OWNERLESS_LEG),
        GuaranteeAdmissionRule::NonEmptyOwner,
    );
    static UNSCHEDULED_SEED: invariants::InvariantSpec = invariants::InvariantSpec {
        id: "SEED-HOSTILE",
        statement: "hostile",
        kind: guarantees::GuaranteeKind::SemanticLaw,
        lifetime: guarantees::GuaranteeLifetime::Permanent,
        owner: "docs/00_CONSTITUTION.md",
        gates: &[],
        witnesses: &[],
        witness_note: "hostile",
        failure_disposition: "hostile",
        derives_from: &[],
        refines: &[],
        discharges: &[],
        supersedes: &[],
    };
    hostile(
        findings,
        "seed row scheduling no gate",
        GuaranteeSource::Seed(&UNSCHEDULED_SEED),
        GuaranteeAdmissionRule::RowNamesScheduledGates,
    );
}

/// The four identity catalogs are coherent (5.5E3d). One term, ONE axis:
/// spellings are unique across identity, generation, binding, and version —
/// so ContentDigest classified as an object identity has no spelling.
/// Every entry's owner resolves against the declared contract ids, the
/// chronology/navigation exclusion is executed law, and each catalog's
/// variants are exhaustively classified.
fn check_identity_catalogs(root: &Path, findings: &mut Vec<String>) {
    let contract_ids = declared_contract_ids(root);
    let owner_texts = contract_authored_texts(root);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut entries: Vec<(String, identities::CatalogEntry)> = Vec::new();
    for kind in identities::IdentityKind::ALL {
        entries.push((format!("identity {kind:?}"), kind.entry()));
    }
    for kind in identities::GenerationKind::ALL {
        entries.push((format!("generation {kind:?}"), kind.entry()));
    }
    for kind in identities::BindingKind::ALL {
        entries.push((format!("binding {kind:?}"), kind.entry()));
    }
    for kind in identities::VersionIdentityKind::ALL {
        entries.push((format!("version {kind:?}"), kind.entry()));
    }
    for (label, entry) in &entries {
        if entry.spelling.trim().is_empty() {
            findings.push(format!("{label} projects an empty spelling"));
        }
        if !seen.insert(entry.spelling) {
            findings.push(format!(
                "{label} spells {}, which another catalog entry already owns; \
                 one term answers one question",
                entry.spelling
            ));
        }
        if !contract_ids.contains(entry.owner.raw()) {
            findings.push(format!(
                "{label} names owner {}, which no document declares",
                entry.owner.raw()
            ));
        } else if !owner_texts
            .get(entry.owner.raw())
            .is_some_and(|text| authors_token(text, entry.spelling))
        {
            findings.push(format!(
                "{label} names owner {}, whose authoritative document does not \
                 author {}",
                entry.owner.raw(),
                entry.spelling
            ));
        }
        if identities::EXCLUDED_CHRONOLOGY_AND_NAVIGATION.contains(&entry.spelling) {
            findings.push(format!(
                "{label} admits {}, which is chronology/order/navigation \
                 vocabulary owned outside the identity catalogs",
                entry.spelling
            ));
        }
        if identities::EXISTING_TYPED_OWNER_SPELLINGS.contains(&entry.spelling) {
            findings.push(format!(
                "{label} duplicates {}, which already has a typed spec owner; \
                 the catalog may reference it but never re-admit it",
                entry.spelling
            ));
        }
    }
    // The residue half of the permanent corpus denominator: a non-cataloged
    // term may not ALSO be admitted, an owned-elsewhere term needs a live
    // contract owner, and a rejected term needs a live standing decision —
    // reopening happens by amending the decision, never by a quiet variant.
    let mut residue_seen: BTreeSet<&str> = BTreeSet::new();
    for residue in identities::NON_CATALOGED_IDENTITY_TERMS {
        if !residue_seen.insert(residue.term) {
            findings.push(format!(
                "{} is declared twice in the residue table; one term, one \
                 path also means one row on that path",
                residue.term
            ));
        }
        if seen.contains(residue.term) {
            findings.push(format!(
                "{} is both cataloged and listed as non-cataloged; one term \
                 resolves through exactly one path",
                residue.term
            ));
        }
        // Exhaustive: a new disposition must be classified here, not defaulted.
        match residue.disposition {
            identities::IdentityTermDisposition::OwnedElsewhere(owner) => {
                if !contract_ids.contains(owner.raw()) {
                    findings.push(format!(
                        "{} is owned elsewhere by {}, which no document declares",
                        residue.term,
                        owner.raw()
                    ));
                } else if !owner_texts
                    .get(owner.raw())
                    .is_some_and(|text| authors_token(text, residue.term))
                {
                    findings.push(format!(
                        "{} is owned elsewhere by {}, which does not author the term",
                        residue.term,
                        owner.raw()
                    ));
                }
            }
            identities::IdentityTermDisposition::NotYetAdmittedBy(decision) => {
                match dispositions::DECISIONS.iter().find(|d| d.id == decision.raw()) {
                    None => findings.push(format!(
                        "{} is not yet admitted by {}, which no declared decision owns",
                        residue.term,
                        decision.raw()
                    )),
                    // Forward policy authority is required: the one lawful
                    // lifetime derivation says Supersede and RetainAsEvidence
                    // rows are historical coverage only, and a historical
                    // archive does not issue future visas.
                    Some(d) if matches!(
                        d.disposition.guarantee_lifetime(),
                        guarantees::GuaranteeLifetime::HistoricalCoverageOnly
                    ) =>
                    {
                        findings.push(format!(
                            "{} is not yet admitted by {}, which is retained only as \
                             historical coverage and cannot own a future admission barrier",
                            residue.term,
                            decision.raw()
                        ));
                    }
                    // Permanent is not enough (5.5E3d1): Kill is Permanent
                    // like Lock, but it is a dead passport, not a pending
                    // application. The typed predicate — not the lifetime —
                    // decides which dispositions own a future entry path.
                    Some(d) if !d.disposition.may_own_not_yet_admitted_identity() => {
                        findings.push(format!(
                            "{} is not yet admitted by {}, whose {:?} disposition is \
                             not a standing future-entry policy; only Lock and Defer \
                             own pending applications",
                            residue.term,
                            decision.raw(),
                            d.disposition
                        ));
                    }
                    // Decision coherence (5.5E3d2): the cited decision must
                    // NAME the pending term in its forward-policy fields —
                    // subject, successor, or replacement contract. A live
                    // Lock decision about something else is an unrelated
                    // authority, and stale_aliases retire vocabulary rather
                    // than granting future admission.
                    Some(d) => {
                        let named = authors_token(d.subject, residue.term)
                            || authors_token(d.successor, residue.term)
                            || d.replacement_contract
                                .is_some_and(|rc| authors_token(rc, residue.term));
                        if !named {
                            findings.push(format!(
                                "{} is not yet admitted by {}, whose forward-policy \
                                 fields do not name the term",
                                residue.term,
                                decision.raw()
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// The three command namespaces are coherent (5.5E3e). Tokens are nonempty
/// and unique WITHIN each namespace (the same token across namespaces is
/// lawful); ASK/DO never enter a CLI namespace; a Direct owner resolves and
/// authors its token; a Composite's composition owner resolves and authors
/// its token while owning only orchestration — delegates are nonempty,
/// unique, live, and never include the composition owner. Delegate contracts
/// are NOT required to author the CLI token: they own the invoked semantic
/// operation, not the adapter spelling.
fn check_commands(root: &Path, findings: &mut Vec<String>) {
    let contract_ids = declared_contract_ids(root);
    let owner_texts = contract_authored_texts(root);
    let mut check_entry =
        |ns: &str, token: &str, auth: commands::CommandAuthority, seen: &mut BTreeSet<String>| {
            if token.trim().is_empty() {
                findings.push(format!("{ns} projects an empty token"));
            }
            if !seen.insert(token.to_string()) {
                findings.push(format!(
                    "{ns} token {token} is claimed twice; tokens are unique within \
                     their namespace"
                ));
            }
            if ns != "BatQlSourceMode" && token.eq_ignore_ascii_case("ask")
                || ns != "BatQlSourceMode" && token.eq_ignore_ascii_case("do")
            {
                findings.push(format!(
                    "{ns} admits {token}: ASK and DO are language modes, never CLI \
                     verbs, and DO is never a top-level command"
                ));
            }
            fn owner_check(
                findings: &mut Vec<String>,
                contract_ids: &BTreeSet<String>,
                owner_texts: &BTreeMap<String, String>,
                ns: &str,
                token: &str,
                role: &str,
                owner: guarantees::ContractId,
            ) {
                if !contract_ids.contains(owner.raw()) {
                    findings.push(format!(
                        "{ns} {token} cites {role} {}, which no document declares",
                        owner.raw()
                    ));
                } else if !owner_texts
                    .get(owner.raw())
                    .is_some_and(|text| authors_token(text, token))
                {
                    findings.push(format!(
                        "{ns} {token} cites {role} {}, whose authoritative document \
                         does not author the token",
                        owner.raw()
                    ));
                }
            }
            match auth {
                commands::CommandAuthority::Direct(owner) => {
                    if owner.raw() == "BP-COMMAND-PLANE-1" {
                        findings.push(format!(
                            "{ns} {token} is Direct-owned by BP-COMMAND-PLANE-1, which \
                             may own command composition but never the command-level \
                             meaning"
                        ));
                    }
                    owner_check(findings, &contract_ids, &owner_texts, ns, token, "owner", owner);
                }
                commands::CommandAuthority::Composite { composition_owner, delegates } => {
                    owner_check(
                        findings,
                        &contract_ids,
                        &owner_texts,
                        ns,
                        token,
                        "composition owner",
                        composition_owner,
                    );
                    if delegates.is_empty() {
                        findings.push(format!(
                            "{ns} {token} is a composite with no delegates; an \
                             orchestration boundary with nothing to route to is refused"
                        ));
                    }
                    let mut seen_delegates = BTreeSet::new();
                    for delegate in delegates {
                        if !seen_delegates.insert(delegate.raw()) {
                            findings.push(format!(
                                "{ns} {token} repeats delegate {}; each delegate \
                                 retains its own law exactly once",
                                delegate.raw()
                            ));
                        }
                        if delegate.raw() == composition_owner.raw() {
                            findings.push(format!(
                                "{ns} {token} lists its composition owner as its own \
                                 delegate; composition never transfers ownership"
                            ));
                        }
                        if !contract_ids.contains(delegate.raw()) {
                            findings.push(format!(
                                "{ns} {token} delegates to {}, which no document \
                                 declares",
                                delegate.raw()
                            ));
                        }
                    }
                }
            }
        };
    let mut product_seen = BTreeSet::new();
    for command in commands::ProductCommand::ALL {
        check_entry("ProductCommand", command.token(), command.authority(), &mut product_seen);
    }
    let mut mode_seen = BTreeSet::new();
    for mode in commands::BatQlSourceMode::ALL {
        check_entry("BatQlSourceMode", mode.keyword(), mode.authority(), &mut mode_seen);
    }
    let mut testpak_seen = BTreeSet::new();
    for command in commands::TestPakCommand::ALL {
        check_entry("TestPakCommand", command.token(), command.authority(), &mut testpak_seen);
    }
}

/// The typed promotion denominator executes its own law (5.5E3i): ALL is
/// conjunctive and complete, spellings are unique and authored by the
/// owner, every admission basis resolves AND names its requirement, the
/// policy surface is CandidatePromotion, the change basis is DEC-074, and
/// the gate bindings are G3 enforcement with G9 release visibility.
fn check_promotion(root: &Path, findings: &mut Vec<String>) {
    use spec::promotion::{
        PromotionRequirement, PROMOTION_CHANGE_BASIS, PROMOTION_ENFORCEMENT_GATE,
        PROMOTION_POLICY_SURFACE, PROMOTION_RELEASE_VISIBILITY_GATE,
    };
    let contract_ids = declared_contract_ids(root);
    let owner_texts = contract_authored_texts(root);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for requirement in PromotionRequirement::ALL {
        let s = requirement.spelling();
        if s.trim().is_empty() {
            findings.push("a promotion requirement projects an empty spelling".to_string());
        }
        if !seen.insert(s) {
            findings.push(format!("promotion-requirement spelling {s} is claimed twice"));
        }
        let owner = requirement.semantic_owner();
        if !contract_ids.contains(owner.raw()) {
            findings.push(format!(
                "promotion requirement {s} cites owner {}, which no document declares",
                owner.raw()
            ));
        } else if !owner_texts
            .get(owner.raw())
            .is_some_and(|text| authors_token(text, s))
        {
            findings.push(format!(
                "promotion requirement {s} cites owner {}, whose authoritative \
                 document does not author the spelling",
                owner.raw()
            ));
        }
        match requirement.admission_basis() {
            guarantees::GuaranteeRef::Decision(decision) => {
                match dispositions::DECISIONS.iter().find(|d| d.id == decision.raw()) {
                    None => findings.push(format!(
                        "promotion requirement {s} cites admission basis {}, which no \
                         declared decision owns",
                        decision.raw()
                    )),
                    Some(d) => {
                        let named = authors_token(d.subject, s)
                            || authors_token(d.successor, s)
                            || d.replacement_contract.is_some_and(|rc| authors_token(rc, s));
                        if !named {
                            findings.push(format!(
                                "promotion requirement {s} cites admission basis {}, \
                                 whose forward-policy fields do not name the requirement",
                                decision.raw()
                            ));
                        }
                    }
                }
            }
            other => findings.push(format!(
                "promotion requirement {s} cites a non-decision admission basis {other:?}"
            )),
        }
    }
    if !matches!(
        PROMOTION_POLICY_SURFACE,
        architecture::ProofPolicySurface::CandidatePromotion
    ) {
        findings.push(
            "the promotion policy surface is not ProofPolicySurface::CandidatePromotion"
                .to_string(),
        );
    }
    match PROMOTION_CHANGE_BASIS {
        guarantees::GuaranteeRef::Decision(decision) if decision.raw() == "DEC-074" => {}
        other => findings.push(format!(
            "the promotion policy-change basis is {other:?}, not DEC-074; requirement \
             admission and change classification are different laws"
        )),
    }
    if PROMOTION_ENFORCEMENT_GATE != gates::GateId::G3 {
        findings.push("candidate-promotion policy is not enforced at G3".to_string());
    }
    if PROMOTION_RELEASE_VISIBILITY_GATE != gates::GateId::G9 {
        findings.push(
            "promotion policy changes are not release-visibly qualified at G9".to_string(),
        );
    }
}

/// The admitted compiler-assumption kinds execute their own law (5.5E3h):
/// spellings unique and authored by the semantic owner, one admission basis
/// per kind that NAMES the kind, only UnsafeMemoryContract intrinsically
/// requiring the SAFETY-CONTRACT marker, classification at G3, and release
/// qualification at G9.
fn check_compiler_assumptions(root: &Path, findings: &mut Vec<String>) {
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
fn check_corpus(root: &Path, findings: &mut Vec<String>) {
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
fn check_mutation(root: &Path, findings: &mut Vec<String>) {
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
fn check_contract_kinds(findings: &mut Vec<String>) {
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
fn guarantee_ref_resolves(reference: guarantees::GuaranteeRef) -> bool {
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
fn check_toolchain(root: &Path, findings: &mut Vec<String>) {
    let t = toolchain::TOOLCHAIN;
    let refuse = |findings: &mut Vec<String>, field: &str, observed: &str, required: &str,
                  repair: &str| {
        findings.push(format!(
            "toolchain {field} violated (owner: spec/toolchain.rs ToolchainProfile;              observed: {observed}; required: {required}; repair: {repair})"
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
    let want = t.root_toolchain_toml();
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

/// The canonical ACTIVE proof-row identities docs/24 declares, parsed
/// STRUCTURALLY: only an id line inside the ```text fence that immediately
/// follows a "Required witnesses (proof owner ...)" header counts. A name
/// appearing in a migration note, an expectation paragraph, or any other
/// prose is NOT an active row — the substring search this replaces would
/// bless exactly the deletion the retention denominator exists to catch.
fn docs24_active_rows(doc: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    let mut armed = false;
    let mut in_fence = false;
    for line in doc.lines() {
        if in_fence {
            if line.trim_end() == "```" {
                in_fence = false;
                continue;
            }
            let id = line.trim();
            if !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                out.insert(id);
            }
            continue;
        }
        if armed {
            if line.trim().is_empty() {
                continue;
            }
            armed = false;
            if line.trim_end() == "```text" {
                in_fence = true;
            }
            continue;
        }
        if line.contains("Required witnesses (proof owner ") && line.trim_end().ends_with(':') {
            armed = true;
        }
    }
    out
}

/// The proof-identity catalog is the living census (5.5E2j). Executed laws:
/// no identity is both active and retired (or declared twice at all); both
/// lifecycle states are constructed; every retirement names at least one
/// successor and never itself; every successor resolves INSIDE the catalog;
/// and the catalog's active side equals the structurally parsed canonical
/// docs/24 rows exactly, in both directions. A libtest count proves every
/// currently declared test executed; this catalog proves no required proof
/// identity disappeared — different axes, each with its own owner.
fn check_proof_rows(root: &Path, findings: &mut Vec<String>) {
    use proof::{ProofRowState, PROOF_ROWS};
    let mut active: BTreeSet<&str> = BTreeSet::new();
    let mut retired: BTreeSet<&str> = BTreeSet::new();
    for record in PROOF_ROWS {
        // Exhaustive: a new state must be classified here, not defaulted.
        let side = match record.state {
            ProofRowState::Active => &mut active,
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
    let doc = fs::read_to_string(root.join("docs/24_GAUNTLET.md")).unwrap_or_default();
    let canonical = docs24_active_rows(&doc);
    for missing in canonical.iter().filter(|id| !active.contains(**id)) {
        findings.push(format!(
            "docs/24 declares active proof row {missing}, which the typed catalog \
             never learned"
        ));
    }
    for phantom in active.iter().filter(|id| !canonical.contains(**id)) {
        findings.push(format!(
            "typed Active identity {phantom} appears as no canonical docs/24 \
             active row"
        ));
    }
}

/// Typed witness citations RESOLVE (5.5E2). A witness names WHICH owned
/// evidence obligation a law depends on; a citation of an undeclared
/// guarantee, an unknown contract id, or a tool that does not exist in the
/// checked tree is refused at runtime, every run. The note carries the human
/// reading and carries no law.
/// The declared contract ids, read from the same inventory the front-matter
/// law walks. Shared by the witness-citation and identity-catalog laws.
fn declared_contract_ids(root: &Path) -> BTreeSet<String> {
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
fn contract_authored_texts(root: &Path) -> BTreeMap<String, String> {
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
fn authors_token(text: &str, term: &str) -> bool {
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

fn check_witness_citations(root: &Path, findings: &mut Vec<String>) {
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
/// and only Passed counts green -- the fence is executed through the real
/// counts_green(), so reclassifying a terminal reddens the running binary.
fn check_proof_terminals(findings: &mut Vec<String>) {
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
    if !T::Passed.counts_green() {
        findings.push("Passed no longer counts green; the denominator is vacuous".into());
    }
    for t in [T::Failed, T::Refused, T::Unsupported, T::SkippedWithAuthority,
              T::Expired, T::Superseded] {
        if t.counts_green() {
            findings.push(format!(
                "proof terminal {t:?} counts green; only Passed may (SEED-AUDITED-DENOMINATOR)"));
        }
    }
}

/// DEC-058 (5.5E1): the release seal binds one typed inventory. Three
/// hand-authored lists disagreed about kernel receipts and SBOM evidence
/// until this enum existed; the projections now derive from it.
fn check_release_seal(findings: &mut Vec<String>) {
    use architecture::ReleaseSealField as F;
    let mut seen = BTreeSet::new();
    for field in architecture::RELEASE_SEAL_FIELDS {
        // Exhaustive: a new field must be classified here, not defaulted.
        match field {
            F::SourceTree | F::Toolchain | F::DependencyGraph | F::GeneratedFacts
            | F::CompatibilityCorpus | F::TestDispositions | F::MutationDispositions
            | F::FuzzDispositions | F::BenchmarkDispositions | F::CompilerAssumptionLedger
            | F::DependencyLedger | F::KernelQualificationSet | F::PackageContents
            | F::PublicApi | F::Sbom | F::LicenseEvidence | F::ProofFreshness => {}
        }
        if !seen.insert(format!("{field:?}")) {
            findings.push(format!("release-seal field {field:?} is listed twice"));
        }
    }
    if seen.len() != 17 {
        findings.push(format!(
            "a release-seal field is missing from RELEASE_SEAL_FIELDS ({} of 17)", seen.len()));
    }
    if !seen.contains("KernelQualificationSet") {
        findings.push("the kernel qualification set left the seal; an empty set \
                       states 'no kernels admitted', it never disappears".into());
    }
}

/// DEC-075: the reconciliation composition stays closed, total, and honest.
/// The fence assertions at the end are the law's teeth: they exercise the
/// real `admissible()` the way production would, so weakening the
/// classification reddens an executed check, not a comment.
fn check_reconciliation(findings: &mut Vec<String>) {
    use reconciliation::{ReconciliationRole, RetrySignal};
    let mut roles = BTreeSet::new();
    for c in reconciliation::RECONCILIATION_COORDINATES {
        // Exhaustive: a new role must be classified here, not defaulted.
        match c.role {
            ReconciliationRole::DurableOrderWitness
            | ReconciliationRole::ChronologyWitness
            | ReconciliationRole::LogicalIdentity
            | ReconciliationRole::PhysicalIdentity
            | ReconciliationRole::BalancingEvidence => {}
        }
        if !roles.insert(format!("{:?}", c.role)) {
            findings.push(format!("reconciliation role {:?} is bound twice", c.role));
        }
        if c.carriers.is_empty() {
            findings.push(format!("reconciliation role {:?} names no carrier", c.role));
        }
        if c.law.trim().is_empty() {
            findings.push(format!("reconciliation role {:?} states no law", c.role));
        }
    }
    if roles.len() != 5 {
        findings.push(format!("a reconciliation role is unbound ({} of 5)", roles.len()));
    }
    let mut axes = BTreeSet::new();
    for a in reconciliation::DOUBLE_ENTRY_AXES {
        if !axes.insert(format!("{a:?}")) {
            findings.push(format!("double-entry axis {a:?} is listed twice"));
        }
    }
    if axes.len() != 10 {
        findings.push(format!("a double-entry axis is missing from DOUBLE_ENTRY_AXES ({} of 10)", axes.len()));
    }
    let mut sigs = BTreeSet::new();
    for s in reconciliation::RETRY_SIGNALS {
        if !sigs.insert(format!("{s:?}")) {
            findings.push(format!("retry signal {s:?} is listed twice"));
        }
    }
    if sigs.len() != 11 {
        findings.push(format!("a retry signal is missing from RETRY_SIGNALS ({} of 11)", sigs.len()));
    }
    if !reconciliation::RETRY_SIGNALS.iter().any(|s| s.admissible()) {
        findings.push("no retry signal is admissible; retry would be impossible".into());
    }
    if RetrySignal::ElapsedWallTime.admissible() {
        findings.push("elapsed wall time may not authorize retry (DEC-075)".into());
    }
    if RetrySignal::ProcessDeath.admissible() {
        findings.push("process death may not authorize retry (DEC-075)".into());
    }
    if RetrySignal::MissingAcknowledgement.admissible() {
        findings.push("a missing acknowledgement requires reconciliation, not retry (DEC-075)".into());
    }
    if RetrySignal::MissingInMemoryWaiter.admissible() {
        findings.push("a missing in-memory waiter may not authorize retry (DEC-075)".into());
    }
}

/// Candidate material stays disposable (spec/architecture.rs): the declared
/// output root must live in the untracked target/ tree and may never sit under
/// a surface candidates are forbidden to write. The two constants state one
/// law, and until this check existed no executing code consumed the output
/// root at all — the containment relation between them was never enforced.
fn check_candidate_containment(findings: &mut Vec<String>) {
    let out = architecture::CANDIDATE_OUTPUT_ROOT;
    if !out.starts_with("target/") {
        findings.push(format!("candidate output root {out} escapes the untracked target/ tree"));
    }
    for forbidden in architecture::CANDIDATE_FORBIDDEN_WRITE_ROOTS {
        if out.starts_with(forbidden) {
            findings.push(format!("candidate output root {out} sits under forbidden write root {forbidden}"));
        }
    }
}

/// The authenticated-history claim contract (DEC-071).
///
/// Structural only: this performs no signature, accumulator, witness, freshness,
/// or rollback verification of any kind, and proves nothing about cryptography.
/// It proves the CONTRACT cannot be weakened.
fn check_authenticated_history(findings: &mut Vec<String>) {
    use architecture::{
        AuthenticatedHistoryProfile, AuthenticityClaim, FreshnessClaim, IntegrityClaim,
        RollbackResistanceClaim, WitnessDisposition, WitnessPolicy,
    };
    // 5.5E2: the profile table dissolved into const fns on the profile enum,
    // and most of the ladder policing that stood here dissolved with it —
    // duplicate rows, row counts, an always-true bool, and per-row gate
    // presence have no spelling once every fact is a total function of the
    // variant. What remains is the AUTHORED semantics, executed through the
    // real fns so a mutated arm reddens the running binary.
    for profile in AuthenticatedHistoryProfile::ALL {
        let profile = *profile;
        // Exhaustive: a new profile must be classified here, not defaulted.
        match profile {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => {}
        }
        for policy in profile.permitted_witness_policies() {
            match policy {
                WitnessPolicy::None | WitnessPolicy::Optional | WitnessPolicy::Required => {}
            }
            if *policy == WitnessPolicy::Required
                && profile != AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            {
                findings.push(format!(
                    "{profile:?} permits WitnessPolicy::Required outside ExternallyAnchoredHistory"
                ));
            }
        }
        // A success bundle states all four axes. Every success verifies
        // integrity, and freshness never drifts from rollback resistance.
        for bundle in [
            profile.unanchored_success_claims(),
            profile.verified_witness_success_claims(),
        ]
        .into_iter()
        .flatten()
        {
            match bundle.integrity {
                IntegrityClaim::InternalConsistencyVerified => {}
            }
            match bundle.authenticity {
                AuthenticityClaim::NotClaimed | AuthenticityClaim::SignedHistoryVerified => {}
            }
            match bundle.freshness {
                FreshnessClaim::NotClaimed | FreshnessClaim::WitnessedGenerationVerified => {}
            }
            match bundle.rollback_resistance {
                RollbackResistanceClaim::Unavailable
                | RollbackResistanceClaim::ScopedToVerifiedWitness => {}
            }
            let fresh = bundle.freshness == FreshnessClaim::WitnessedGenerationVerified;
            let scoped =
                bundle.rollback_resistance == RollbackResistanceClaim::ScopedToVerifiedWitness;
            if fresh != scoped {
                findings.push(format!(
                    "{profile:?} lets freshness and rollback resistance drift apart"
                ));
            }
        }
        // An unanchored success never claims freshness or scoped rollback
        // resistance: a restored older validly signed history satisfies it.
        if let Some(bundle) = profile.unanchored_success_claims() {
            if bundle.freshness != FreshnessClaim::NotClaimed
                || bundle.rollback_resistance != RollbackResistanceClaim::Unavailable
            {
                findings.push(format!(
                    "{profile:?} unanchored success claims freshness or rollback resistance"
                ));
            }
            if profile == AuthenticatedHistoryProfile::InternalConsistency
                && bundle.authenticity != AuthenticityClaim::NotClaimed
            {
                findings.push("InternalConsistency unanchored success claims signed authenticity".into());
            }
        }
        // `None` here means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED.
        if profile == AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            && profile.unanchored_success_claims().is_some()
        {
            findings.push(
                "ExternallyAnchoredHistory admits an unanchored success bundle; an absent or invalid \
                 required witness must refuse, not fall back to a weaker success"
                    .into(),
            );
        }
        if profile != AuthenticatedHistoryProfile::InternalConsistency
            && profile.verified_witness_success_claims().is_none()
        {
            findings.push(format!("{profile:?} admits no witnessed success bundle"));
        }
        // The requirement fns and the policy matrix may not disagree: a
        // profile requiring an independent witness admits only Required, and
        // a profile requiring signed history is never InternalConsistency.
        if profile.requires_independent_witness_verification()
            != matches!(
                profile.permitted_witness_policies(),
                [WitnessPolicy::Required]
            )
        {
            findings.push(format!(
                "{profile:?} witness requirement disagrees with its permitted policies"
            ));
        }
        if !profile.requires_local_commitment_verification() {
            findings.push(format!(
                "{profile:?} does not require local commitment verification"
            ));
        }
        if profile.requires_signed_history_verification()
            != (profile != AuthenticatedHistoryProfile::InternalConsistency)
        {
            findings.push(format!(
                "{profile:?} signed-history requirement disagrees with the frozen family"
            ));
        }
        if profile.implementation_gates().is_empty()
            || profile.release_qualification_gates().is_empty()
        {
            findings.push(format!("{profile:?} names no implementation or release gate"));
        }
    }
    // A required witness fails closed on every frozen failure class, including
    // one that was never supplied.
    for disposition in architecture::REQUIRED_WITNESS_FAILURE_SET {
        match disposition {
            WitnessDisposition::NotApplicable | WitnessDisposition::Verified => {
                findings.push(format!(
                    "REQUIRED_WITNESS_FAILURE_SET contains the non-failure {disposition:?}"
                ));
            }
            _ => {}
        }
    }
    if !architecture::REQUIRED_WITNESS_FAILURE_SET.contains(&WitnessDisposition::NotProvided) {
        findings.push("a required witness that was never supplied is not a refusal".into());
    }
    // Optional: optional to supply, mandatory to validate once supplied.
    if architecture::OPTIONAL_WITNESS_REFUSAL_SET.contains(&WitnessDisposition::NotProvided) {
        findings.push("OPTIONAL_WITNESS_REFUSAL_SET refuses an absent optional witness".into());
    }
    for disposition in architecture::REQUIRED_WITNESS_FAILURE_SET {
        if *disposition != WitnessDisposition::NotProvided
            && !architecture::OPTIONAL_WITNESS_REFUSAL_SET.contains(disposition)
        {
            findings.push(format!(
                "a supplied {disposition:?} optional witness may degrade to absence or success"
            ));
        }
    }
    if architecture::REFUSAL_PARTIAL_CLAIM_LAW.trim().is_empty() {
        findings.push("no refusal/partial-evidence law is stated".into());
    }
}

fn check_version(findings: &mut Vec<String>) {
    // Reject any workspace version below the signed 1.0 implementation train so
    // a template cannot regress the family to a pre-1.0 line (e.g. 0.1.0).
    let version = architecture::WORKSPACE_VERSION;
    let major = version.split('.').next().and_then(|part| part.parse::<u64>().ok());
    match major {
        Some(major) if major >= 1 => {}
        _ => findings.push(format!("workspace version {version} is below the signed 1.0 train")),
    }
}

fn check_graph(findings: &mut Vec<String>) {
    // The package CATALOG law (5.5E3b): the variant is the identity, and the
    // declared inventory equals the authored rows exactly, in canonical
    // order. Counts are derived from the typed inventory, never hardcoded.
    let ids: Vec<architecture::PackageId> =
        architecture::PACKAGES.iter().map(|p| p.id).collect();
    if ids.as_slice() != architecture::PackageId::ALL {
        findings.push(
            "PACKAGES does not declare exactly PackageId::ALL in canonical order".into(),
        );
    }
    for id in architecture::PackageId::ALL {
        // Exhaustive: a new package variant must be classified here, must
        // join ALL, and must gain a PackageSpec row — not default.
        match id {
            architecture::PackageId::MacBatCompiler
            | architecture::PackageId::MacBat
            | architecture::PackageId::BatPak
            | architecture::PackageId::SyncBat
            | architecture::PackageId::BatQl
            | architecture::PackageId::NetBat
            | architecture::PackageId::TestPak
            | architecture::PackageId::BatPakCli
            | architecture::PackageId::BatPakExamples => {}
        }
        if id.cargo_name().trim().is_empty() {
            findings.push(format!("{id:?} projects an empty cargo name"));
        }
        if id.display_name().trim().is_empty() {
            findings.push(format!("{id:?} projects an empty display name"));
        }
        let path = id.workspace_path();
        if path.trim().is_empty()
            || path.starts_with('/')
            || path.contains(':')
            || path.split('/').any(|part| part == "..")
        {
            findings.push(format!(
                "{id:?} projects workspace path {path:?}, which is not relative and contained"
            ));
        }
    }
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.id.cargo_name()).collect();
    let layers: BTreeMap<&str, u8> = architecture::PACKAGES.iter().map(|p| (p.id.cargo_name(), p.layer)).collect();
    if packages.len() != architecture::PACKAGES.len() { findings.push("two package identities project the same cargo name".into()); }
    let paths: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.id.workspace_path()).collect();
    if paths.len() != architecture::PACKAGES.len() { findings.push("two package identities project the same workspace path".into()); }
    for package in architecture::PACKAGES {
        if package.role.trim().is_empty() { findings.push(format!("empty role for {}", package.id.cargo_name())); }
    }
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in architecture::EDGES {
        if !packages.contains(edge.importer.cargo_name()) { findings.push(format!("unknown importer {}", edge.importer.cargo_name())); }
        if !packages.contains(edge.importee.cargo_name()) { findings.push(format!("unknown importee {}", edge.importee.cargo_name())); }
        if edge.importer == edge.importee { findings.push(format!("self dependency {}", edge.importer.cargo_name())); }
        if edge.profile.is_empty() { findings.push(format!("edge has empty profile {} -> {}", edge.importer.cargo_name(), edge.importee.cargo_name())); }
        if let (Some(importer_layer), Some(importee_layer)) = (layers.get(edge.importer.cargo_name()), layers.get(edge.importee.cargo_name())) {
            if importer_layer <= importee_layer {
                findings.push(format!("dependency direction violation {}(L{}) -> {}(L{})", edge.importer.cargo_name(), importer_layer, edge.importee.cargo_name(), importee_layer));
            }
        }
        if edge.importer == architecture::PackageId::TestPak && edge.class != architecture::EdgeClass::DevOnly {
            findings.push(format!("testpak edge must be dev-only: {}", edge.importee.cargo_name()));
        }
        if edge.importee == architecture::PackageId::BatPakExamples {
            findings.push(format!("nothing may depend on the examples package: {} -> batpak-examples", edge.importer.cargo_name()));
        }
        if edge.importer == architecture::PackageId::BatPakExamples && edge.importee == architecture::PackageId::TestPak {
            findings.push("batpak-examples must not depend on dev tooling (testpak)".to_string());
        }
        if edge.importer == architecture::PackageId::BatPakCli && edge.class == architecture::EdgeClass::DevOnly {
            findings.push(format!("CLI edge cannot be dev-only: {}", edge.importee.cargo_name()));
        }
        graph.entry(edge.importer.cargo_name()).or_default().push(edge.importee.cargo_name());
    }
    for package in architecture::PACKAGES {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if cycle(package.id.cargo_name(), &graph, &mut visiting, &mut visited) {
            findings.push(format!("dependency cycle reaches {}", package.id.cargo_name()));
        }
    }
}

fn check_profiles(findings: &mut Vec<String>) {
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.id.cargo_name()).collect();
    let mut identities = BTreeSet::new();
    for profile in architecture::QUALIFICATION_PROFILES {
        if !packages.contains(profile.package.cargo_name()) {
            findings.push(format!("unknown qualification package {}", profile.package.cargo_name()));
        }
        if profile.profile.trim().is_empty() || profile.requirement.trim().is_empty() {
            findings.push(format!("incomplete qualification profile {}:{}", profile.package.cargo_name(), profile.profile));
        }
        if !identities.insert((profile.package.cargo_name(), profile.profile)) {
            findings.push(format!("duplicate qualification profile {}:{}", profile.package.cargo_name(), profile.profile));
        }
    }
    for package in [architecture::PackageId::BatPak, architecture::PackageId::SyncBat] {
        if !architecture::QUALIFICATION_PROFILES.iter().any(|p| p.package == package && p.profile == "semantic" && p.environment == architecture::QualificationEnvironment::NoStdAlloc) {
            findings.push(format!("missing no_std + alloc semantic profile for {}", package.cargo_name()));
        }
    }
}

fn cycle<'a>(node: &'a str, graph: &BTreeMap<&'a str, Vec<&'a str>>, visiting: &mut BTreeSet<&'a str>, visited: &mut BTreeSet<&'a str>) -> bool {
    if visited.contains(node) { return false; }
    if !visiting.insert(node) { return true; }
    if let Some(next) = graph.get(node) {
        for child in next { if cycle(child, graph, visiting, visited) { return true; } }
    }
    visiting.remove(node);
    visited.insert(node);
    false
}

fn check_unique_ids(findings: &mut Vec<String>) {
    let invariant_ids: BTreeSet<&str> = invariants::INVARIANTS.iter().map(|v| v.id).collect();
    if invariant_ids.len() != invariants::INVARIANTS.len() { findings.push("duplicate invariant ID".into()); }
    // The one gate identity. The inventory must be complete, unique, and in
    // canonical (declaration) order; every typed gate reference resolves through
    // it, so no fact can name a gate that does not exist.
    let gate_ids: Vec<gates::GateId> = gates::GATES.iter().map(|g| g.id).collect();
    let gate_id_set: BTreeSet<&str> = gates::GATES.iter().map(|g| g.id.token()).collect();
    let gate_tokens: BTreeSet<&str> = gates::GATES.iter().map(|g| g.token).collect();
    if gate_id_set.len() != gates::GATES.len() {
        findings.push("duplicate GateId in the gate inventory".into());
    }
    if gate_tokens.len() != gates::GATES.len() {
        findings.push("one gate token claimed by two GateIds".into());
    }
    for value in gates::GATES {
        if value.token.trim().is_empty() || value.title.trim().is_empty() {
            findings.push(format!("incomplete gate {}", value.token));
        }
        if value.token != value.id.token() {
            findings.push(format!("gate {} token disagrees with GateId::token()", value.token));
        }
        // Exhaustive: a new gate variant must be classified here, not defaulted.
        match value.id {
            gates::GateId::G0
            | gates::GateId::G1
            | gates::GateId::G2
            | gates::GateId::G3
            | gates::GateId::G4
            | gates::GateId::G5
            | gates::GateId::G6
            | gates::GateId::G7
            | gates::GateId::G8
            | gates::GateId::G9 => {}
        }
    }
    let mut sorted_gate_ids = gate_ids.clone();
    sorted_gate_ids.sort();
    if gate_ids != sorted_gate_ids {
        findings.push("gate inventory is not in canonical GateId order".into());
    }
    // Canonical, duplicate-free, resolvable gate lists on every gate-bearing fact.
    //
    // Emptiness is NOT judged here. Whether a fact must name a gate is a question
    // its own family answers: a DecisionClass with requires_gate() == false is
    // gate-independent by authored claim, not by omission. This rule once flagged
    // every empty list, which directly contradicted the class-aware rule three
    // lines below it and made DEC-001, DEC-002, and DEC-048 permanently red. Two
    // rules in one file disagreeing about one fact is the same defect as two
    // scripts carrying one constant.
    let check_gates = |ident: &str, list: &[gates::GateId], findings: &mut Vec<String>| {
        let unique: BTreeSet<&str> = list.iter().map(|g| g.token()).collect();
        if unique.len() != list.len() {
            findings.push(format!("{} names a duplicate GateId", ident));
        }
        let mut sorted = list.to_vec();
        sorted.sort();
        if list.to_vec() != sorted {
            findings.push(format!("{} gate list is not in canonical order", ident));
        }
        for gate in list {
            if !gates::GATES.iter().any(|g| g.id == *gate) {
                findings.push(format!("{} names a gate outside the inventory", ident));
            }
        }
    };

    for value in invariants::INVARIANTS {
        if value.statement.trim().is_empty() { findings.push(format!("empty invariant statement {}", value.id)); }
        if value.owner.trim().is_empty() || value.witnesses.is_empty() {
            findings.push(format!("unclassified invariant {} (missing owner or witness)", value.id));
        }
        match value.kind {
            guarantees::GuaranteeKind::SemanticLaw
            | guarantees::GuaranteeKind::ArchitectureConstraint
            | guarantees::GuaranteeKind::BootstrapAssertion
            | guarantees::GuaranteeKind::LegacyObligation
            | guarantees::GuaranteeKind::QualificationRequirement
            | guarantees::GuaranteeKind::Decision => {}
        }
        match value.lifetime {
            guarantees::GuaranteeLifetime::Permanent
            | guarantees::GuaranteeLifetime::UntilGate
            | guarantees::GuaranteeLifetime::UntilCompatibilityExpiry
            | guarantees::GuaranteeLifetime::UntilSuccessor
            | guarantees::GuaranteeLifetime::HistoricalCoverageOnly
            | guarantees::GuaranteeLifetime::ClosedEvidence => {}
        }
    }
    let decision_ids: BTreeSet<&str> = dispositions::DECISIONS.iter().map(|v| v.id).collect();
    if decision_ids.len() != dispositions::DECISIONS.len() { findings.push("duplicate decision ID".into()); }
    for value in dispositions::DECISIONS {
        if value.subject.trim().is_empty() || value.successor.trim().is_empty() {
            findings.push(format!("incomplete decision {}", value.id));
        }
        // The class -- not the title, ID range, document section, or keyword --
        // decides whether the row must name a gate (DEC-072).
        check_gates(value.id, value.gates, findings);
        if value.class.requires_gate() && value.gates.is_empty() {
            findings.push(format!(
                "{} is implementation-bearing and names no gate",
                value.id
            ));
        }
        // Exhaustive: a new class must be classified here, not defaulted.
        match value.class {
            dispositions::DecisionClass::Architecture
            | dispositions::DecisionClass::Capability
            | dispositions::DecisionClass::Compatibility
            | dispositions::DecisionClass::Enforcement
            | dispositions::DecisionClass::HistoricalReceipt
            | dispositions::DecisionClass::Naming
            | dispositions::DecisionClass::ImplementationPosture => {}
        }
        match value.disposition {
            dispositions::Disposition::Keep
            | dispositions::Disposition::Lock
            | dispositions::Disposition::Kill
            | dispositions::Disposition::Supersede
            | dispositions::Disposition::Demote
            | dispositions::Disposition::Defer
            | dispositions::Disposition::OpenImplementation
            | dispositions::Disposition::RetainAsEvidence => {}
        }
        // The stale-context vocabulary is closed in both directions (5.5E2):
        // the scanner's two DEFAULT contexts are declared variants, and a row
        // may never allow-list them — that would exempt production source or
        // ordinary authoritative prose from the inline STALE-REF law
        // wholesale. Executed through the real permissive(), so reclassifying
        // a default as permissive reddens the running binary.
        for context in value.stale_allowed_contexts {
            // Exhaustive: a new context must be classified here, not defaulted.
            match context {
                dispositions::StaleContext::DecisionLedger
                | dispositions::StaleContext::RejectionRecord
                | dispositions::StaleContext::SupersessionGuide
                | dispositions::StaleContext::LegacyEvidence
                | dispositions::StaleContext::MigrationCompatibility
                | dispositions::StaleContext::ProductionSource
                | dispositions::StaleContext::OrdinaryAuthoritative => {}
            }
            if !context.permissive() {
                findings.push(format!(
                    "{} allow-lists the default context {context:?}; a stale term in \
                     production source or ordinary authoritative material always \
                     requires an inline STALE-REF",
                    value.id
                ));
            }
        }
    }
    for value in invariants::INVARIANTS {
        check_gates(value.id, value.gates, findings);
        // SEED declares no gate-independent class, so an empty list here is an
        // omission rather than a claim. Stated explicitly now that check_gates no
        // longer judges emptiness: dropping the rule silently would have weakened
        // SEED to buy DEC's correctness.
        if value.gates.is_empty() {
            findings.push(format!("{} names no gate", value.id));
        }
    }
    for value in legacy_obligations::OBLIGATIONS {
        // Emptiness is judged below, with the rest of the obligation's
        // completeness.
        check_gates(value.id, value.gates, findings);
    }
    let legacy_ids: BTreeSet<&str> = legacy_obligations::OBLIGATIONS.iter().map(|v| v.id).collect();
    if legacy_ids.len() != legacy_obligations::OBLIGATIONS.len() { findings.push("duplicate legacy obligation ID".into()); }
    for value in legacy_obligations::OBLIGATIONS {
        if value.law.trim().is_empty() || value.clean_owner.trim().is_empty() || value.gates.is_empty() {
            findings.push(format!("incomplete legacy obligation {}", value.id));
        }
        match value.compatibility_disposition {
            legacy_obligations::CompatibilityDisposition::None
            | legacy_obligations::CompatibilityDisposition::ReadOlderAccepted
            | legacy_obligations::CompatibilityDisposition::CanonicalOrTypedRefusal
            | legacy_obligations::CompatibilityDisposition::FrozenCanonicalIdentity => {}
        }
        match value.deletion_condition {
            legacy_obligations::DeletionCondition::Never
            | legacy_obligations::DeletionCondition::OnSuccessorGateClosure
            | legacy_obligations::DeletionCondition::OnCompatibilityWindowExpiry => {}
        }
        match value.active_or_closed_status {
            legacy_obligations::ObligationStatus::Active
            | legacy_obligations::ObligationStatus::Closed => {}
        }
    }
    let coverage_ids: BTreeSet<&str> = legacy_invariant_coverage::COVERAGE.iter().map(|v| v.legacy_id).collect();
    if coverage_ids.len() != legacy_invariant_coverage::COVERAGE.len() { findings.push("duplicate legacy invariant coverage ID".into()); }
    if legacy_invariant_coverage::COVERAGE.len() != legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS {
        findings.push(format!("legacy invariant coverage row count is {}, expected {}", legacy_invariant_coverage::COVERAGE.len(), legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS));
    }
    if legacy_invariant_coverage::LEGACY_SOURCE_COMMIT.len() != 40 {
        findings.push("legacy invariant source commit is not a full SHA".into());
    }
    let manifest_ids: BTreeSet<&str> = legacy_invariant_coverage::SOURCE_INVARIANT_IDS.iter().copied().collect();
    if manifest_ids.len() != legacy_invariant_coverage::SOURCE_INVARIANT_IDS.len() {
        findings.push("duplicate source invariant declaration id".into());
    }
    if legacy_invariant_coverage::SOURCE_INVARIANT_IDS.len() != legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS {
        findings.push(format!(
            "source invariant manifest has {} ids, expected {}",
            legacy_invariant_coverage::SOURCE_INVARIANT_IDS.len(),
            legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS
        ));
    }
    for declared in &manifest_ids {
        if !coverage_ids.contains(declared) {
            findings.push(format!("declared legacy invariant {declared} has no coverage row"));
        }
    }
    for covered in &coverage_ids {
        if !manifest_ids.contains(covered) {
            findings.push(format!("coverage row {covered} is not a declared source invariant"));
        }
    }
    for value in legacy_invariant_coverage::COVERAGE {
        if value.legacy_id.trim().is_empty() || value.successor.trim().is_empty() || value.rationale.trim().is_empty() {
            findings.push(format!("incomplete legacy invariant coverage {}", value.legacy_id));
        }
        match value.disposition {
            legacy_invariant_coverage::CoverageDisposition::Preserve
            | legacy_invariant_coverage::CoverageDisposition::Supersede
            | legacy_invariant_coverage::CoverageDisposition::Demote
            | legacy_invariant_coverage::CoverageDisposition::Kill
            | legacy_invariant_coverage::CoverageDisposition::Requalify => {}
        }
    }
    let operator_ids: BTreeSet<&str> = operators::OPERATORS.iter().map(|o| o.id).collect();
    if operator_ids.len() != operators::OPERATORS.len() {
        findings.push("duplicate operator ID".into());
    }
    let mut surface_owner: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for op in operators::OPERATORS {
        if op.id.trim().is_empty()
            || op.word_surface.trim().is_empty()
            || op.semantic_op.trim().is_empty()
            || op.input_sorts.trim().is_empty()
            || op.result_sort.trim().is_empty()
            || op.overflow.trim().is_empty()
            || op.exception.trim().is_empty()
            || op.formatting.trim().is_empty()
            || op.spoken.trim().is_empty()
            || op.mutation_classes.trim().is_empty()
        {
            findings.push(format!("incomplete operator {}", op.id));
        }
        let fixity = match op.fixity {
            operators::Fixity::Prefix => "Prefix",
            operators::Fixity::Infix => "Infix",
        };
        for surface in [op.word_surface, op.symbol_surface] {
            if surface.is_empty() {
                continue;
            }
            if let Some(prev) = surface_owner.insert((surface, fixity), op.id) {
                if prev != op.id {
                    findings.push(format!("operator token {surface} fixity {fixity} claimed by {prev} and {}", op.id));
                }
            }
        }
        match op.class {
            operators::OperatorClass::Arithmetic
            | operators::OperatorClass::Comparison
            | operators::OperatorClass::Logical => {}
        }
        // Typed legality rules (5.5E1): placement is law. A wall-observation
        // difference anywhere but subtraction would leak wall arithmetic past
        // the TimeDelta fence (docs/16); a rate difference IS subtraction.
        if op.typing.is_empty() {
            findings.push(format!("operator {} declares no typing rules", op.id));
        }
        for rule in op.typing {
            // Exhaustive: a new rule must be classified here, not defaulted.
            match rule {
                operators::OperatorTypingRule::WallObservationDifference
                | operators::OperatorTypingRule::PercentDifference => {
                    if op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {} claims a difference typing rule but is not subtraction", op.id));
                    }
                }
                operators::OperatorTypingRule::PercentAdjustment => {
                    if op.semantic_op != "add" && op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {} claims PercentAdjustment outside add/subtract", op.id));
                    }
                }
                operators::OperatorTypingRule::SameSortComparison => {
                    if !matches!(op.class, operators::OperatorClass::Comparison) {
                        findings.push(format!(
                            "operator {} claims SameSortComparison outside the comparison class", op.id));
                    }
                }
                operators::OperatorTypingRule::TruthUnary
                | operators::OperatorTypingRule::TruthBinary => {
                    if !matches!(op.class, operators::OperatorClass::Logical) {
                        findings.push(format!(
                            "operator {} claims a Truth typing rule outside the logical class", op.id));
                    }
                }
                operators::OperatorTypingRule::SameUnit
                | operators::OperatorTypingRule::DimensionalByDimensionless
                | operators::OperatorTypingRule::LikeDimensionRatio => {
                    if !matches!(op.class, operators::OperatorClass::Arithmetic) {
                        findings.push(format!(
                            "operator {} claims an arithmetic typing rule outside the arithmetic class", op.id));
                    }
                }
            }
        }
        match op.arity {
            operators::Arity::Unary | operators::Arity::Binary => {}
        }
        match op.associativity {
            operators::Associativity::Left
            | operators::Associativity::Right
            | operators::Associativity::NonAssociative => {}
        }
        match op.exactness {
            operators::Exactness::Exact | operators::Exactness::NotApplicable => {}
        }
        match op.numeric_support {
            operators::NumericSupport::ExactSupported
            | operators::NumericSupport::QualifiedProfileOnly
            | operators::NumericSupport::Unsupported
            | operators::NumericSupport::NotApplicable => {}
        }
    }
}

fn check_frontmatter(root: &Path, findings: &mut Vec<String>) {
    for relative in architecture::REQUIRED_DOCS {
        if !relative.ends_with(".md") { continue; }
        let path=root.join(relative);
        if !path.is_file() { continue; }
        match fs::read_to_string(&path) {
            Ok(text) => {
                if !text.starts_with("---\n") { findings.push(format!("missing frontmatter: {relative}")); }
                // A GENERATED projection is not an authored contract and carries
                // no contract_id, supersedes, or last_reconciled: it names what
                // produced it instead. Demanding the authored set from a derived
                // index would be demanding it claim an authority it must not have.
                // This rule predated generated documents and never learned them,
                // so docs/GUARANTEE_GRAPH.generated.md has been red since 5.5C1.
                let generated = text.contains("status: GENERATED");
                let required: &[&str] = if generated {
                    &["status:", "authority_scope:", "generated_by:", "generated_from:", "do_not_edit:"]
                } else {
                    &["status:", "contract_id:", "authority_scope:", "supersedes:", "last_reconciled:"]
                };
                for key in required {
                    if !text.contains(key) { findings.push(format!("missing {key} in {relative}")); }
                }
            }
            Err(error) => findings.push(format!("cannot read {relative}: {error}")),
        }
    }
}

fn check_syncbat_shape(root: &Path, findings: &mut Vec<String>) {
    let base=root.join("crates/syncbat");
    if !base.exists() { return; }
    for relative in architecture::SYNCBAT_REQUIRED_PLANES {
        if !base.join(relative).exists() { findings.push(format!("syncbat missing required plane {relative}")); }
    }
}

fn check_source_debt(root: &Path, findings: &mut Vec<String>) {
    for base in [root.join("crates"), root.join("apps"), root.join("examples")] {
        if !base.is_dir() { continue; }
        walk(&base, root, findings);
    }
}

// Conservative, path-based test ownership. The bootstrap oracle never parses
// arbitrary Rust test bodies; TestPak's AST gate owns the precise distinction.
fn is_test_owned(rel: &Path) -> bool {
    for component in rel.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(text) = name.to_str() {
                if matches!(text, "tests" | "benches" | "fixtures" | "corpus" | "fuzz") {
                    return true;
                }
            }
        }
    }
    false
}

fn walk(path: &Path, root: &Path, findings: &mut Vec<String>) {
    let Ok(entries)=fs::read_dir(path) else { findings.push(format!("cannot read {}", path.display())); return; };
    for entry in entries.flatten() {
        let p=entry.path();
        if p.is_dir() { walk(&p, root, findings); continue; }
        if p.extension().is_some_and(|e| e == "rs") {
            let Ok(text)=fs::read_to_string(&p) else { findings.push(format!("cannot read {}", p.display())); continue; };
            let rel_path = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
            let rel=rel_path.display();
            let code = sanitize_rust(&text);
            for banned in ["#[allow", "#![allow", "#[expect", "#![expect", "todo!", "unimplemented!"] {
                if code.contains(banned) { findings.push(format!("banned source token {banned:?} in {rel}")); }
            }
            // Production-source lexical debt (defense in depth). Test-owned paths
            // are exempt so contextual .expect() in tests remains legal.
            if !is_test_owned(&rel_path) {
                for banned in [".unwrap(", ".expect(", "panic!", "dbg!"] {
                    if code.contains(banned) { findings.push(format!("banned production token {banned:?} in {rel}")); }
                }
            }
            let tokens = rust_tokens(&code);
            let has_unsafe = tokens.iter().any(|token| token.text == "unsafe");
            if has_unsafe {
                let location = rel.to_string();
                let admitted_path = location.contains("kernel") || location.contains("adapter") || location.contains("ffi");
                if !admitted_path { findings.push(format!("unsafe code outside kernel/adapter/ffi path in {rel}")); }
                if !text.contains("SAFETY-CONTRACT:") { findings.push(format!("unsafe code lacks SAFETY-CONTRACT record in {rel}")); }
            }
            for line in function_local_type_lines(&tokens) {
                findings.push(format!("function-local named type at {rel}:{line}"));
            }
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
struct RustToken {
    text: String,
    line: usize,
}

fn sanitize_rust(text: &str) -> String {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State { Code, LineComment, BlockComment, String, Char, RawString }

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut state = State::Code;
    let mut block_depth = 0usize;
    let mut raw_hashes = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();
        match state {
            State::Code => {
                if byte == b'/' && next == Some(b'/') {
                    out.push_str("  ");
                    index += 2;
                    state = State::LineComment;
                    continue;
                }
                if byte == b'/' && next == Some(b'*') {
                    out.push_str("  ");
                    index += 2;
                    block_depth = 1;
                    state = State::BlockComment;
                    continue;
                }
                if byte == b'"' {
                    out.push(' ');
                    index += 1;
                    state = State::String;
                    continue;
                }
                if byte == b'\'' {
                    let has_close = bytes[index + 1..bytes.len().min(index + 8)]
                        .iter()
                        .position(|candidate| *candidate == b'\'');
                    if has_close.is_some() {
                        out.push(' ');
                        index += 1;
                        state = State::Char;
                        continue;
                    }
                }
                if byte == b'r' {
                    let mut cursor = index + 1;
                    while bytes.get(cursor) == Some(&b'#') { cursor += 1; }
                    if bytes.get(cursor) == Some(&b'"') {
                        raw_hashes = cursor - index - 1;
                        for _ in index..=cursor { out.push(' '); }
                        index = cursor + 1;
                        state = State::RawString;
                        continue;
                    }
                }
                out.push(char::from(byte));
                index += 1;
            }
            State::LineComment => {
                if byte == b'\n' { out.push('\n'); state = State::Code; } else { out.push(' '); }
                index += 1;
            }
            State::BlockComment => {
                if byte == b'/' && next == Some(b'*') {
                    out.push_str("  "); index += 2; block_depth += 1; continue;
                }
                if byte == b'*' && next == Some(b'/') {
                    out.push_str("  "); index += 2; block_depth -= 1;
                    if block_depth == 0 { state = State::Code; }
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
            State::String => {
                if byte == b'\\' {
                    out.push(' ');
                    if index + 1 < bytes.len() { out.push(if bytes[index + 1] == b'\n' { '\n' } else { ' ' }); }
                    index += 2;
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
                if byte == b'"' { state = State::Code; }
            }
            State::Char => {
                if byte == b'\\' {
                    out.push(' ');
                    if index + 1 < bytes.len() { out.push(if bytes[index + 1] == b'\n' { '\n' } else { ' ' }); }
                    index += 2;
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
                if byte == b'\'' { state = State::Code; }
            }
            State::RawString => {
                if byte == b'"' {
                    let end = index + 1 + raw_hashes;
                    if end <= bytes.len() && bytes[index + 1..end].iter().all(|candidate| *candidate == b'#') {
                        for _ in index..end { out.push(' '); }
                        index = end;
                        state = State::Code;
                        continue;
                    }
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
        }
    }
    out
}

fn rust_tokens(code: &str) -> Vec<RustToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_line = 1usize;
    let mut line = 1usize;
    let flush = |tokens: &mut Vec<RustToken>, current: &mut String, current_line: usize| {
        if !current.is_empty() {
            tokens.push(RustToken { text: std::mem::take(current), line: current_line });
        }
    };
    for character in code.chars() {
        if character == '\n' {
            flush(&mut tokens, &mut current, current_line);
            line += 1;
            continue;
        }
        if character.is_ascii_alphanumeric() || character == '_' {
            if current.is_empty() { current_line = line; }
            current.push(character);
        } else {
            flush(&mut tokens, &mut current, current_line);
            if matches!(character, '{' | '}' | ';') {
                tokens.push(RustToken { text: character.to_string(), line });
            }
        }
    }
    flush(&mut tokens, &mut current, current_line);
    tokens
}

fn function_local_type_lines(tokens: &[RustToken]) -> BTreeSet<usize> {
    let mut contexts: Vec<bool> = Vec::new();
    let mut pending_function = false;
    let mut findings = BTreeSet::new();
    for token in tokens {
        match token.text.as_str() {
            "fn" => pending_function = true,
            "{" => {
                contexts.push(pending_function);
                pending_function = false;
            }
            ";" => pending_function = false,
            "}" => {
                let _ = contexts.pop();
                pending_function = false;
            }
            "struct" | "enum" | "trait" | "union" | "type"
                if contexts.iter().any(|is_function| *is_function) => {
                    findings.insert(token.line);
                }
            _ => {}
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::{is_test_owned, sanitize_rust};
    use std::path::Path;

    fn production_debt(code: &str) -> Vec<String> {
        let sanitized = sanitize_rust(code);
        let mut hits = Vec::new();
        for token in [".unwrap(", ".expect(", "panic!", "dbg!"] {
            if sanitized.contains(token) { hits.push(token.to_string()); }
        }
        hits
    }

    #[test]
    fn production_panic_is_rejected() {
        assert!(production_debt("fn f() { panic!(\"boom\"); }").iter().any(|h| h == "panic!"));
    }

    #[test]
    fn production_unwrap_is_rejected() {
        assert!(production_debt("fn f() { let _ = x.unwrap(); }").iter().any(|h| h == ".unwrap("));
    }

    #[test]
    fn production_expect_is_rejected() {
        assert!(production_debt("fn f() { let _ = x.expect(\"why\"); }").iter().any(|h| h == ".expect("));
    }

    #[test]
    fn production_dbg_is_rejected() {
        assert!(production_debt("fn f() { dbg!(x); }").iter().any(|h| h == "dbg!"));
    }

    #[test]
    fn commented_and_string_tokens_are_ignored() {
        assert!(production_debt("// panic!\nlet s = \".unwrap(\";").is_empty());
    }

    #[test]
    fn test_path_expect_is_allowed() {
        assert!(is_test_owned(Path::new("crates/batpak/tests/recovery.rs")));
        assert!(is_test_owned(Path::new("crates/testpak/fixtures/x.rs")));
        assert!(!is_test_owned(Path::new("crates/batpak/src/event.rs")));
    }

    #[test]
    fn bootstrap_detector_fixture_does_not_grade_itself() {
        assert!(production_debt("let banned = [r#\"panic!\"#, r\".unwrap(\"];").is_empty());
    }
}

/// The semantic ISA admits every authored node, and admits nothing else.
///
/// This runs the SAME `pakvm_isa::admit` the specification declares. It does not
/// re-derive the answer here: a checker that recomputed the policy would be a
/// second owner of it, which is the defect this whole pass exists to remove.
fn check_pakvm_isa(findings: &mut Vec<String>) {
    use pakvm_isa::*;
    let mut admitted = 0usize;
    for &node in PAKVM_NODES {
        match admit(node) {
            PakVmAdmission::Admitted(spec) => {
                admitted += 1;
                // A projector may serialize an admitted spec; it may never
                // complete one. Every string field must therefore already say
                // something.
                for (field, value) in [
                    ("authored name", spec.authored_name),
                    ("operand sorts", spec.operand_sorts),
                    ("result sorts", spec.result_sorts),
                    ("source origin", spec.source_origin),
                ] {
                    if value.is_empty() {
                        findings.push(format!("PakVM node {:?} admits with an empty {field}", node));
                    }
                }
                if spec.work_formula.units().is_empty() {
                    findings.push(format!(
                        "PakVM node {:?} admits with a work formula accounting in no unit", node));
                }
            }
            PakVmAdmission::Refused(why) => {
                findings.push(format!("PakVM node {:?} does not admit: {why}", node));
            }
        }
    }
    if admitted != PAKVM_NODES.len() {
        findings.push(format!(
            "PakVM semantic ISA admitted {admitted} of {} authored nodes", PAKVM_NODES.len()));
    }
    // Identity is symbolic and unique. A duplicate id or a duplicate authored
    // name would let two meanings share one node.
    for i in 0..PAKVM_NODES.len() {
        for j in (i + 1)..PAKVM_NODES.len() {
            if PAKVM_NODES[i] == PAKVM_NODES[j] {
                findings.push(format!("PakVM node {:?} is listed twice", PAKVM_NODES[i]));
            }
            if PAKVM_NODES[i].authored_name() == PAKVM_NODES[j].authored_name() {
                findings.push(format!(
                    "PakVM nodes {:?} and {:?} share the authored name {:?}",
                    PAKVM_NODES[i], PAKVM_NODES[j], PAKVM_NODES[i].authored_name()));
            }
        }
    }
    // Every authored work unit is claimed by some node family. A unit nothing
    // accounts in means either the unit list or the node inventory is wrong, and
    // silently carrying it would hide which.
    for unit in [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                 WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches, WorkUnit::Outputs,
                 WorkUnit::Artifacts, WorkUnit::Effects, WorkUnit::CallDepth] {
        let claimed = PAKVM_NODES.iter().any(|&n| match admit(n) {
            PakVmAdmission::Admitted(s) => s.work_formula.units().contains(&unit),
            PakVmAdmission::Refused(_) => false,
        });
        if !claimed {
            findings.push(format!(
                "authored work unit {:?} is accounted by no PakVM node family", unit));
        }
    }
}

/// The SyncBat authority firewall is complete and closed.
///
/// Runs the same `admit_crossing` the specification declares. It does not
/// recompute the legal-crossing table here: a checker carrying its own copy would
/// be a second owner of the law, and two copies of one guess agree perfectly.
fn check_syncbat_firewall(findings: &mut Vec<String>) {
    use architecture::{SyncBatPlane, SYNCBAT_PLANES};
    use syncbat_firewall::*;

    // Every authority has exactly one owning plane, and every plane owns at
    // least one authority. A plane owning nothing is not a plane; an authority
    // owned by nobody has no one to refuse its misuse.
    for &authority in SYNCBAT_AUTHORITIES {
        let owner = authority.owner();
        if !SYNCBAT_PLANES.contains(&owner) {
            findings.push(format!("{authority:?} is owned by a plane outside SyncBat"));
        }
    }
    for &plane in SYNCBAT_PLANES {
        if !SYNCBAT_AUTHORITIES.iter().any(|a| a.owner() == plane) {
            findings.push(format!("SyncBat plane {plane:?} owns no authority"));
        }
        if plane.package() != architecture::PackageId::SyncBat {
            findings.push(format!("SyncBat plane {plane:?} claims package {}", plane.package().cargo_name()));
        }
        if plane.authored_ownership().is_empty() {
            findings.push(format!("SyncBat plane {plane:?} names no authored ownership"));
        }
    }
    for i in 0..SYNCBAT_AUTHORITIES.len() {
        for j in (i + 1)..SYNCBAT_AUTHORITIES.len() {
            if SYNCBAT_AUTHORITIES[i] == SYNCBAT_AUTHORITIES[j] {
                findings.push(format!("authority {:?} is listed twice", SYNCBAT_AUTHORITIES[i]));
            }
        }
    }

    // Every declared crossing must itself be lawful: it must move an authority
    // its sender owns, between two distinct planes, and admit through the very
    // function production calls. A table entry that its own admission refuses
    // would be law nobody could obey.
    for crossing in SYNCBAT_LEGAL_CROSSINGS {
        if crossing.law.is_empty() {
            findings.push(format!(
                "the {:?} -> {:?} crossing names no authored law", crossing.from, crossing.to));
        }
        if crossing.carries.owner() != crossing.from {
            findings.push(format!(
                "the declared {:?} -> {:?} crossing carries {:?}, which {:?} does not own",
                crossing.from, crossing.to, crossing.carries, crossing.from));
            continue;
        }
        let origin = if crossing.carries.requires_semantic_origin() {
            Some(semantic_origin_for(crossing.carries))
        } else {
            None
        };
        match admit_crossing(crossing.from, crossing.to, crossing.carries, origin) {
            CrossingAdmission::Admitted(_) => {}
            CrossingAdmission::Refused(why) => findings.push(format!(
                "the declared {:?} -> {:?} crossing does not admit: {why}",
                crossing.from, crossing.to)),
        }
    }

    // A semantic authority names its PakVM origin BECAUSE it leaves the plane
    // that owns the ISA. One that no crossing carries would require an origin
    // for a value no other plane can ever receive: the requirement becomes
    // ceremony and the execution route the ISA depends on stops existing.
    //
    // This is the only rule that notices a lawful crossing being deleted. Every
    // loop above iterates the whitelist, so removing an entry does not fail
    // them -- it simply gives them less to say, which is exactly how a law
    // disappears while its proofs stay green.
    // The carrier must be REQUIRED, not merely present. A semantic route that is
    // deleted OR quietly downgraded to Optional both fail here: the ISA depends
    // on the route existing in the accepted machine, and Optional means "may be
    // absent".
    for &authority in SYNCBAT_AUTHORITIES {
        if authority.requires_semantic_origin()
            && !SYNCBAT_LEGAL_CROSSINGS.iter().any(|c| {
                c.carries == authority && c.posture == CrossingPosture::Required
            })
        {
            findings.push(format!(
                "{authority:?} must name a PakVM origin but no required crossing carries it off \
                 {:?}, so admitted node meaning has no lawful route out of its own plane",
                authority.owner()));
        }
    }

    check_syncbat_requiredness(findings);

    // docs/08 names five forbidden crossings by example. Each must be refused
    // through the production API, and the refusal must be the intended one.
    for (what, from, to, carries, want) in [
        // "PakVM advancing a durable checkpoint"
        ("pakvm advances a durable checkpoint", SyncBatPlane::PakVm, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
        // "Bvisor minting semantic restart authorization"
        ("bvisor mints semantic restart authority", SyncBatPlane::Bvisor, SyncBatPlane::Runtime,
         SyncBatAuthority::RetryRestartAuthority,
         "the sending plane does not own the authority it is exercising"),
        // "runtime bypassing Bvisor to execute a program"
        ("runtime bypasses bvisor to execute", SyncBatPlane::Runtime, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
        // "PakVM calling filesystem/network/clock directly"
        ("pakvm reaches the host directly", SyncBatPlane::PakVm, SyncBatPlane::Port,
         SyncBatAuthority::TypedEffectRequest,
         "no lawful crossing carries this authority between these planes"),
        // "world redefining BatPak-owned IDs or schema law"
        ("world redefines owned identity", SyncBatPlane::World, SyncBatPlane::PakVm,
         SyncBatAuthority::SemanticNodeInterpretation,
         "the sending plane does not own the authority it is exercising"),
        // attempt evidence must not satisfy a logical receipt
        ("attempt evidence satisfies a semantic receipt", SyncBatPlane::Bvisor,
         SyncBatPlane::Runtime, SyncBatAuthority::SemanticReceipt,
         "the sending plane does not own the authority it is exercising"),
        // runtime must not fabricate attempt evidence
        ("runtime fabricates attempt evidence", SyncBatPlane::Runtime, SyncBatPlane::PakVm,
         SyncBatAuthority::AttemptEvidence,
         "the sending plane does not own the authority it is exercising"),
        // world must not become a service locator for host capability
        ("world wires around the port", SyncBatPlane::World, SyncBatPlane::Port,
         SyncBatAuthority::PhysicalAttempt,
         "the sending plane does not own the authority it is exercising"),
    ] {
        let origin = if carries.requires_semantic_origin() {
            Some(semantic_origin_for(carries))
        } else {
            None
        };
        match admit_crossing(from, to, carries, origin) {
            CrossingAdmission::Admitted(_) => {
                findings.push(format!("the firewall admits a forbidden crossing: {what}"))
            }
            CrossingAdmission::Refused(why) if why != want => findings.push(format!(
                "{what} is refused, but for the wrong reason: {why:?} rather than {want:?}")),
            CrossingAdmission::Refused(_) => {}
        }
    }
}

/// Requiredness as connectivity, recomputed rather than trusted.
///
/// A Required crossing must be needed, and "needed" is not a stamp: it is that
/// the accepted turn cannot complete without it. The turn is a round trip. The
/// logical decider (the plane owning `LogicalResult`) authorizes work outward to
/// the execution planes and every plane returns its result or evidence back to
/// that same decider. So two obligations, both derived from authored ownership
/// rather than from any second policy table:
///
///   - every plane can reach the logical root by Required crossings (its work
///     returns), and
///   - the logical root can reach every plane except the composition source (the
///     plane owning `CompositionAndInstanceIdentity`, which precedes the turn).
///
/// Deleting a Required crossing, or downgrading one to Optional, drops it from
/// the Required graph and severs one of those reaches. Optional crossings are
/// excluded on purpose: "may be absent" means the machine must stand without
/// them, so relying on one for connectivity would be the very confusion this
/// posture exists to prevent.
fn check_syncbat_requiredness(findings: &mut Vec<String>) {
    use architecture::{SyncBatPlane, SYNCBAT_PLANES};
    use syncbat_firewall::*;

    let owner_of = |a: SyncBatAuthority| a.owner();
    let root = owner_of(SyncBatAuthority::LogicalResult);
    let source = owner_of(SyncBatAuthority::CompositionAndInstanceIdentity);

    // Reachability over Required crossings only. Small fixed graph; a linear
    // relaxation to a fixed point needs no allocation beyond a bitset.
    let reaches = |start: SyncBatPlane, goal: SyncBatPlane| -> bool {
        let mut seen = [false; 5];
        let idx = |p: SyncBatPlane| SYNCBAT_PLANES.iter().position(|&q| q == p).unwrap();
        seen[idx(start)] = true;
        let mut changed = true;
        while changed {
            changed = false;
            for c in SYNCBAT_LEGAL_CROSSINGS {
                // Only legal required edges carry connectivity: an illegal
                // substitute is flagged elsewhere and must not reconnect a graph
                // a deleted required route left broken.
                if c.posture == CrossingPosture::Required
                    && c.carries.owner() == c.from
                    && seen[idx(c.from)]
                    && !seen[idx(c.to)]
                {
                    seen[idx(c.to)] = true;
                    changed = true;
                }
            }
        }
        seen[idx(goal)]
    };

    for &plane in SYNCBAT_PLANES {
        if !reaches(plane, root) {
            findings.push(format!(
                "SyncBat plane {plane:?} cannot reach the logical root {root:?} by required \
                 crossings, so its result or evidence has no lawful route home; a required \
                 route is missing or was downgraded to optional"));
        }
        if plane != source && !reaches(root, plane) {
            findings.push(format!(
                "the logical root {root:?} cannot reach SyncBat plane {plane:?} by required \
                 crossings, so it cannot authorize the work that plane owns; a required route \
                 is missing or was downgraded to optional"));
        }
    }

    // An Optional crossing must be genuinely optional: there must be an alternate
    // required route delivering the same authority. If it is the sole carrier,
    // its absence would sever the machine, so "Optional" is a misclassification.
    for c in SYNCBAT_LEGAL_CROSSINGS {
        if c.posture == CrossingPosture::Optional
            && !SYNCBAT_LEGAL_CROSSINGS.iter().any(|o| {
                !core::ptr::eq(o, c)
                    && o.carries == c.carries
                    && o.posture == CrossingPosture::Required
            })
        {
            findings.push(format!(
                "the {:?} -> {:?} crossing is optional but is the sole route carrying {:?}; \
                 deleting it would sever the machine, so it cannot be optional",
                c.from, c.to, c.carries));
        }
    }
}

/// Origin law, exercised where the origin must be chosen rather than derived.
///
/// The forbidden-example loop above computes each origin from
/// `requires_semantic_origin()`, so it can never propose a MISMATCHED one. These
/// cases are exactly the mismatches, and they are the ones that matter: a pure
/// node emitting an effect request is an effect appearing inside a program the
/// validator cleared as pure.
fn check_syncbat_origin_law(findings: &mut Vec<String>) {
    use architecture::SyncBatPlane;
    use pakvm_isa::PakVmNodeId;
    use syncbat_firewall::*;

    for (what, from, to, carries, origin, want) in [
        // A pure node may not emit an effect request. Literal computes a value
        // and admits as Pure, so the effect posture is the only thing refusing.
        ("a pure node emits an effect request", SyncBatPlane::PakVm, SyncBatPlane::Bvisor,
         SyncBatAuthority::TypedEffectRequest, Some(PakVmNodeId::Literal),
         "a node that does not admit as Effectful emits an effect request"),
        // A semantic authority must carry its origin, not travel anonymously.
        ("an interpretation names no origin", SyncBatPlane::PakVm, SyncBatPlane::Runtime,
         SyncBatAuthority::SemanticNodeInterpretation, None,
         "a semantic authority names no PakVM origin"),
        // A physical authority cannot borrow an origin to look semantic.
        ("attempt evidence wears a semantic origin", SyncBatPlane::Bvisor,
         SyncBatPlane::Runtime, SyncBatAuthority::AttemptEvidence, Some(PakVmNodeId::Append),
         "a non-semantic authority names a PakVM origin it cannot have"),
    ] {
        match admit_crossing(from, to, carries, origin) {
            CrossingAdmission::Admitted(_) => {
                findings.push(format!("the firewall admits a forbidden crossing: {what}"))
            }
            CrossingAdmission::Refused(why) if why != want => findings.push(format!(
                "{what} is refused, but for the wrong reason: {why:?} rather than {want:?}")),
            CrossingAdmission::Refused(_) => {}
        }
    }
}

/// A node whose admitted posture suits an authority, for exercising the firewall.
/// Effect requests need an effectful node; interpretation takes any admitted one.
fn semantic_origin_for(authority: syncbat_firewall::SyncBatAuthority) -> pakvm_isa::PakVmNodeId {
    use pakvm_isa::PakVmNodeId;
    use syncbat_firewall::SyncBatAuthority;
    match authority {
        SyncBatAuthority::TypedEffectRequest => PakVmNodeId::Append,
        _ => PakVmNodeId::Compare,
    }
}

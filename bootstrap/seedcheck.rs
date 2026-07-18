#![deny(warnings)]

// The typed specification is a LIBRARY (spec/lib.rs, 5.5E2): this binary
// links it instead of textually mounting its modules, so no tracked
// suppression exists and every pub spec item is API by construction.
use spec::{
    architecture, bootstrap_output, bootstrap_qualification, commands, contracts, dispositions,
    generated_views, identities, gates, guarantees, invariants, legacy_invariant_coverage,
    legacy_obligations, operators, pakvm_isa, proof, reconciliation, syncbat_firewall,
    tier0_cross_run, toolchain,
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
    check_proof_relations(root, &mut findings);
    check_toolchain(root, &mut findings);
    check_contract_kinds(&mut findings);
    check_identity_catalogs(root, &mut findings);
    check_operators(root, &mut findings);
    check_inventory_classes(&mut findings);
    check_ledger_vocabulary(&mut findings);
    check_generated_views(root, &mut findings);
    check_commands(root, &mut findings);
    check_mutation(root, &mut findings);
    check_corpus(root, &mut findings);
    check_compiler_assumptions(root, &mut findings);
    check_promotion(root, &mut findings);
    check_syncbat_shape(root, &mut findings);
    check_bootstrap_output(&mut findings);
    check_bootstrap_qualification(&mut findings);
    check_tier0_cross_run(&mut findings);
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
            legacy_evidence: "hostile evidence",
            clean_owner: "  ",
            mechanism_disposition: "hostile",
            witness_requirement: legacy_obligations::LegacyWitnessRequirement::Planned("hostile route"),
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

/// The documentary inventory classes, EXECUTED (5.5E4b). The typed owners
/// already contain the truth the generated package/edge/profile projections
/// render; seedcheck proves the class inventories, spellings, row parity, and
/// gate discipline on the real Rust values. It parses no generated Markdown
/// and no Python Tier 0 rows.
fn check_inventory_classes(findings: &mut Vec<String>) {
    let mut pkg_spellings: BTreeSet<&str> = BTreeSet::new();
    for class in architecture::PackageClass::ALL {
        // Exhaustive: a new class must be classified here, not defaulted.
        match class {
            architecture::PackageClass::Production
            | architecture::PackageClass::BinaryAdapter
            | architecture::PackageClass::DevOnly
            | architecture::PackageClass::Example => {}
        }
        let spelling = class.spelling();
        if spelling.is_empty() {
            findings.push("empty PackageClass spelling".into());
        }
        if !pkg_spellings.insert(spelling) {
            findings.push(format!("duplicate PackageClass spelling {spelling}"));
        }
    }
    for class in [
        architecture::PackageClass::Production,
        architecture::PackageClass::BinaryAdapter,
        architecture::PackageClass::DevOnly,
        architecture::PackageClass::Example,
    ] {
        if !architecture::PackageClass::ALL.contains(&class) {
            findings.push(format!("PackageClass {:?} is missing from PackageClass::ALL", class));
        }
    }
    let mut edge_spellings: BTreeSet<&str> = BTreeSet::new();
    for class in architecture::EdgeClass::ALL {
        match class {
            architecture::EdgeClass::Required
            | architecture::EdgeClass::OptionalProfile
            | architecture::EdgeClass::DevOnly => {}
        }
        let spelling = class.spelling();
        if spelling.is_empty() {
            findings.push("empty EdgeClass spelling".into());
        }
        if !edge_spellings.insert(spelling) {
            findings.push(format!("duplicate EdgeClass spelling {spelling}"));
        }
    }
    for class in [
        architecture::EdgeClass::Required,
        architecture::EdgeClass::OptionalProfile,
        architecture::EdgeClass::DevOnly,
    ] {
        if !architecture::EdgeClass::ALL.contains(&class) {
            findings.push(format!("EdgeClass {:?} is missing from EdgeClass::ALL", class));
        }
    }
    if architecture::PACKAGES.len() != architecture::PackageId::ALL.len() {
        findings.push("PACKAGES does not equal PackageId::ALL".into());
    }
    for (index, package) in architecture::PACKAGES.iter().enumerate() {
        if architecture::PackageId::ALL.get(index) != Some(&package.id) {
            findings.push(format!(
                "PACKAGES row {} is out of PackageId::ALL order", package.id.cargo_name()));
        }
        if !architecture::PackageClass::ALL.contains(&package.class) {
            findings.push(format!(
                "package {} carries a class outside PackageClass::ALL",
                package.id.cargo_name()));
        }
    }
    for edge in architecture::EDGES {
        if !architecture::EdgeClass::ALL.contains(&edge.class) {
            findings.push(format!(
                "edge {} -> {} carries a class outside EdgeClass::ALL",
                edge.importer.cargo_name(), edge.importee.cargo_name()));
        }
        for endpoint in [edge.importer, edge.importee] {
            if !architecture::PackageId::ALL.contains(&endpoint) {
                findings.push(format!(
                    "edge endpoint {} is outside PackageId::ALL", endpoint.cargo_name()));
            }
        }
    }
    for profile in architecture::QUALIFICATION_PROFILES {
        if !architecture::PackageId::ALL.contains(&profile.package) {
            findings.push(format!(
                "qualification profile {} names a package outside PackageId::ALL",
                profile.profile));
        }
        if !architecture::QualificationEnvironment::ALL.contains(&profile.environment) {
            findings.push(format!(
                "qualification profile {}:{} names an unadmitted environment",
                profile.package.cargo_name(), profile.profile));
        }
        if profile.profile.trim().is_empty() {
            findings.push("empty qualification profile name".into());
        }
        if profile.gates.is_empty() {
            findings.push(format!(
                "qualification profile {}:{} declares no gates",
                profile.package.cargo_name(), profile.profile));
        }
        let mut last_index = None;
        for gate in profile.gates {
            let position = gates::GATES.iter().position(|g| g.id == *gate);
            if position.is_none() {
                findings.push(format!(
                    "qualification profile {}:{} names an undeclared gate",
                    profile.package.cargo_name(), profile.profile));
            }
            if position < last_index {
                findings.push(format!(
                    "qualification profile {}:{} gate list is out of canonical order",
                    profile.package.cargo_name(), profile.profile));
            }
            last_index = position;
        }
    }
}

/// The exact-ledger vocabularies, EXECUTED (5.5E4c). Disposition,
/// DecisionClass, and CoverageDisposition carry the documentary spellings and
/// meanings the generated ledgers render; seedcheck proves the inventories,
/// uniqueness, admission, and canonical gate order on the real Rust values.
/// It parses no Markdown and no DEC prose.
fn check_ledger_vocabulary(findings: &mut Vec<String>) {
    let mut disp_spellings: BTreeSet<&str> = BTreeSet::new();
    for disposition in dispositions::Disposition::ALL {
        match disposition {
            dispositions::Disposition::Keep
            | dispositions::Disposition::Lock
            | dispositions::Disposition::Kill
            | dispositions::Disposition::Supersede
            | dispositions::Disposition::Demote
            | dispositions::Disposition::Defer
            | dispositions::Disposition::OpenImplementation
            | dispositions::Disposition::RetainAsEvidence => {}
        }
        if disposition.spelling().is_empty() || disposition.meaning().is_empty() {
            findings.push(format!(
                "Disposition {:?} carries an empty spelling or meaning", disposition));
        }
        if !disp_spellings.insert(disposition.spelling()) {
            findings.push(format!(
                "duplicate Disposition spelling {}", disposition.spelling()));
        }
    }
    let mut class_spellings: BTreeSet<&str> = BTreeSet::new();
    for class in dispositions::DecisionClass::ALL {
        match class {
            dispositions::DecisionClass::Architecture
            | dispositions::DecisionClass::Capability
            | dispositions::DecisionClass::Compatibility
            | dispositions::DecisionClass::Enforcement
            | dispositions::DecisionClass::HistoricalReceipt
            | dispositions::DecisionClass::Naming
            | dispositions::DecisionClass::ImplementationPosture => {}
        }
        if class.spelling().is_empty() {
            findings.push(format!("DecisionClass {:?} carries an empty spelling", class));
        }
        if !class_spellings.insert(class.spelling()) {
            findings.push(format!("duplicate DecisionClass spelling {}", class.spelling()));
        }
    }
    for row in dispositions::DECISIONS {
        if !dispositions::Disposition::ALL.contains(&row.disposition) {
            findings.push(format!("{} carries a disposition outside Disposition::ALL", row.id));
        }
        if !dispositions::DecisionClass::ALL.contains(&row.class) {
            findings.push(format!("{} carries a class outside DecisionClass::ALL", row.id));
        }
        let mut last_index = None;
        for gate in row.gates {
            let position = gates::GATES.iter().position(|g| g.id == *gate);
            if position.is_none() {
                findings.push(format!("{} names an undeclared gate", row.id));
            }
            if position < last_index {
                findings.push(format!("{} gate list is out of canonical order", row.id));
            }
            last_index = position;
        }
    }
    let mut cov_spellings: BTreeSet<&str> = BTreeSet::new();
    for disposition in legacy_invariant_coverage::CoverageDisposition::ALL {
        match disposition {
            legacy_invariant_coverage::CoverageDisposition::Preserve
            | legacy_invariant_coverage::CoverageDisposition::Supersede
            | legacy_invariant_coverage::CoverageDisposition::Demote
            | legacy_invariant_coverage::CoverageDisposition::Kill
            | legacy_invariant_coverage::CoverageDisposition::Requalify => {}
        }
        if disposition.spelling().is_empty() || disposition.meaning().is_empty() {
            findings.push(format!(
                "CoverageDisposition {:?} carries an empty spelling or meaning", disposition));
        }
        if !cov_spellings.insert(disposition.spelling()) {
            findings.push(format!(
                "duplicate CoverageDisposition spelling {}", disposition.spelling()));
        }
    }
    for row in legacy_invariant_coverage::COVERAGE {
        if !legacy_invariant_coverage::CoverageDisposition::ALL.contains(&row.disposition) {
            findings.push(format!(
                "coverage row {} carries a disposition outside CoverageDisposition::ALL",
                row.legacy_id));
        }
    }
}

/// The generated-view registry, EXECUTED (5.5E4a). Seedcheck runs the real
/// `GeneratedView` values: inventory integrity, complete specs, surface/target
/// shape agreement, per-target marker uniqueness, generator identity, path
/// existence through the repository root, and the registry's own presence.
/// It parses no Markdown block bodies — that reconstruction is audit.py's.
fn check_generated_views(root: &Path, findings: &mut Vec<String>) {
    use generated_views::{GeneratedView, GeneratedViewSurface, GeneratedViewTarget};
    let mut names: BTreeSet<&str> = BTreeSet::new();
    let mut target_markers: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut registry_present = false;
    for view in GeneratedView::ALL {
        let name = view.name();
        if name.trim().is_empty() {
            findings.push("empty GeneratedView name".into());
        }
        if !names.insert(name) {
            findings.push(format!("duplicate GeneratedView {name} in ALL"));
        }
        if *view == GeneratedView::GeneratedViewRegistry {
            registry_present = true;
        }
        let spec = view.spec();
        if spec.authority_sources.is_empty() {
            findings.push(format!("generated view {name} names no authority source"));
        }
        for source in spec.authority_sources {
            if source.trim().is_empty() {
                findings.push(format!("generated view {name} names an empty authority source"));
            } else if !root.join(source).is_file() {
                findings.push(format!(
                    "generated view {name} names authority source {source}, which does not exist"));
            }
        }
        // Exhaustive on generator: a new bootstrap generator must be
        // classified here, never defaulted into legitimacy.
        match spec.generator {
            guarantees::BootstrapToolId::ProjectPy => {}
            other => findings.push(format!(
                "generated view {name} names generator {other:?}; every current \
                 generator is ProjectPy")),
        }
        let static_targets: &[&str] = match spec.target {
            GeneratedViewTarget::Static(targets) => {
                if targets.is_empty() {
                    findings.push(format!("generated view {name} declares an empty static target list"));
                }
                for target in targets {
                    if !root.join(target).is_file() {
                        findings.push(format!(
                            "generated view {name} targets {target}, which does not exist"));
                    }
                }
                targets
            }
            GeneratedViewTarget::EligibleMarkdownCorpus => &[],
        };
        // Surface/target shape agreement, exhaustive on both axes.
        match (spec.surface, spec.target) {
            (GeneratedViewSurface::EmbeddedBlock, GeneratedViewTarget::Static(_)) => {
                match spec.marker {
                    Some(marker) if !marker.trim().is_empty() => {
                        for target in static_targets {
                            if !target_markers.insert((target, marker)) {
                                findings.push(format!(
                                    "generated views claim marker {marker} in {target} twice; \
                                     one target carries one instance of one marker"));
                            }
                        }
                    }
                    _ => findings.push(format!(
                        "embedded generated view {name} carries no marker")),
                }
            }
            (GeneratedViewSurface::EmbeddedBlock, GeneratedViewTarget::EligibleMarkdownCorpus) => {
                findings.push(format!(
                    "embedded generated view {name} must name static targets"));
            }
            (GeneratedViewSurface::StandaloneFile, GeneratedViewTarget::Static(targets)) => {
                if spec.marker.is_some() {
                    findings.push(format!(
                        "standalone generated view {name} carries an embedded marker"));
                }
                if targets.len() != 1 {
                    findings.push(format!(
                        "standalone generated view {name} must name exactly one target"));
                }
            }
            (GeneratedViewSurface::StandaloneFile, GeneratedViewTarget::EligibleMarkdownCorpus) => {
                findings.push(format!(
                    "standalone generated view {name} must name one static target"));
            }
            (GeneratedViewSurface::CorpusFrontmatter, GeneratedViewTarget::EligibleMarkdownCorpus) => {
                if spec.marker.is_some() {
                    findings.push(format!(
                        "corpus-frontmatter generated view {name} carries an embedded marker"));
                }
            }
            (GeneratedViewSurface::CorpusFrontmatter, GeneratedViewTarget::Static(_)) => {
                findings.push(format!(
                    "corpus-frontmatter generated view {name} may not name static targets"));
            }
        }
    }
    if !registry_present {
        findings.push(
            "GeneratedView::ALL omits GeneratedViewRegistry; the registry must include itself"
                .into());
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


/// The proof-identity catalog is the living census (5.5E2j). Executed laws:
/// no identity is both active and retired (or declared twice at all); both
/// lifecycle states are constructed; every retirement names at least one
/// successor and never itself; every successor resolves INSIDE the catalog;
/// and the catalog's active side equals the structurally parsed canonical
/// A libtest count proves every currently declared test executed; this
/// catalog proves no required proof identity disappeared. Since 5.5E4d the
/// typed catalog is the one membership authority — different axes, each with
/// its own owner.
fn check_proof_rows(_root: &Path, findings: &mut Vec<String>) {
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
}

/// The typed operator surface law, EXECUTED (5.5E3j). Seedcheck runs the real
/// Rust values: identity/row parity in `OperatorId::ALL` order, token
/// discipline on both closed surface inventories, owner and basis resolution,
/// surface adoption, and the class/shape law. It parses no Markdown and no
/// decision prose; those reconstructions belong to audit.py.
fn check_operators(root: &Path, findings: &mut Vec<String>) {
    let owner_texts = contract_authored_texts(root);
    let decision_ids: BTreeSet<&str> = dispositions::DECISIONS.iter().map(|d| d.id).collect();
    let mut spellings: BTreeSet<&str> = BTreeSet::new();
    for id in operators::OperatorId::ALL {
        let spelling = id.spelling();
        if spelling.trim().is_empty() {
            findings.push("empty OperatorId spelling".into());
        }
        if !spellings.insert(spelling) {
            findings.push(format!("duplicate OperatorId spelling {spelling}"));
        }
        let rows = operators::OPERATORS.iter().filter(|o| o.id == *id).count();
        if rows != 1 {
            findings.push(format!(
                "OperatorId {spelling} has {rows} OperatorSpec rows; exactly one is lawful"));
        }
        if !owner_texts.contains_key(id.semantic_owner().raw()) {
            findings.push(format!(
                "operator {spelling} names owner {}, which no document declares",
                id.semantic_owner().raw()));
        }
        if !decision_ids.contains(id.admission_basis().raw()) {
            findings.push(format!(
                "operator {spelling} names admission basis {}, which no declared decision owns",
                id.admission_basis().raw()));
        }
    }
    if operators::OPERATORS.len() != operators::OperatorId::ALL.len() {
        findings.push(format!(
            "OPERATORS declares {} rows for {} OperatorId variants",
            operators::OPERATORS.len(),
            operators::OperatorId::ALL.len()));
    }
    for (index, op) in operators::OPERATORS.iter().enumerate() {
        if operators::OperatorId::ALL.get(index) != Some(&op.id) {
            findings.push(format!(
                "OPERATORS row {} is out of OperatorId::ALL order", op.id.spelling()));
        }
    }
    // The closed word inventory: canonical uppercase word grammar, unique
    // nonempty tokens, resolved owner and basis, exactly one adopting row.
    let mut word_tokens: BTreeSet<&str> = BTreeSet::new();
    for word in operators::OperatorWordSurface::ALL {
        let token = word.token();
        let uppercase_words = !token.is_empty()
            && !token.starts_with(' ')
            && !token.ends_with(' ')
            && !token.contains("  ")
            && token.bytes().all(|b| b.is_ascii_uppercase() || b == b' ');
        if !uppercase_words {
            findings.push(format!(
                "word surface token {token:?} violates the canonical uppercase word grammar"));
        }
        if !word_tokens.insert(token) {
            findings.push(format!("duplicate word surface token {token:?}"));
        }
        if !owner_texts.contains_key(word.semantic_owner().raw()) {
            findings.push(format!(
                "word surface {token:?} names owner {}, which no document declares",
                word.semantic_owner().raw()));
        }
        if !decision_ids.contains(word.admission_basis().raw()) {
            findings.push(format!(
                "word surface {token:?} names admission basis {}, which no declared decision owns",
                word.admission_basis().raw()));
        }
        let adopters = operators::OPERATORS
            .iter()
            .filter(|o| o.syntax.canonical_word() == Some(*word))
            .count();
        if adopters != 1 {
            findings.push(format!(
                "word surface {token:?} is adopted by {adopters} OperatorSpec rows; exactly one is lawful"));
        }
    }
    // The closed symbol inventory: nonempty ASCII punctuation, unique tokens,
    // resolved owner and basis, exactly one adopting row — an alias stays
    // attached to its one canonical OperatorId.
    let mut symbol_tokens: BTreeSet<&str> = BTreeSet::new();
    for symbol in operators::OperatorSymbolSurface::ALL {
        let token = symbol.token();
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_punctuation()) {
            findings.push(format!(
                "symbol surface token {token:?} is not nonempty ASCII punctuation"));
        }
        if !symbol_tokens.insert(token) {
            findings.push(format!("duplicate symbol surface token {token:?}"));
        }
        if !owner_texts.contains_key(symbol.semantic_owner().raw()) {
            findings.push(format!(
                "symbol surface {token:?} names owner {}, which no document declares",
                symbol.semantic_owner().raw()));
        }
        if !decision_ids.contains(symbol.admission_basis().raw()) {
            findings.push(format!(
                "symbol surface {token:?} names admission basis {}, which no declared decision owns",
                symbol.admission_basis().raw()));
        }
        let adopters = operators::OPERATORS
            .iter()
            .filter(|o| {
                o.syntax.canonical_symbol() == Some(*symbol)
                    || o.syntax.symbol_alias() == Some(*symbol)
            })
            .count();
        if adopters != 1 {
            findings.push(format!(
                "symbol surface {token:?} is adopted by {adopters} OperatorSpec rows; exactly one is lawful"));
        }
    }
    let mut surface_owner: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for op in operators::OPERATORS {
        let spelling = op.id.spelling();
        if op.syntax.canonical_token().is_empty() {
            findings.push(format!("operator {spelling} has an empty canonical token"));
        }
        if op.semantic_op.trim().is_empty()
            || op.input_sorts.trim().is_empty()
            || op.result_sort.trim().is_empty()
            || op.overflow.trim().is_empty()
            || op.exception.trim().is_empty()
            || op.spoken.trim().is_empty()
            || op.mutation_classes.trim().is_empty()
        {
            findings.push(format!("incomplete operator {spelling}"));
        }
        // The class/shape law is total: arithmetic is symbol-only, comparison
        // is word-with-symbol-alias, logical is word-only. Exhaustive on both
        // axes so a new class or shape must be classified here, not defaulted.
        let lawful_shape = match (op.class, op.syntax) {
            (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::SymbolOnly(_)) => true,
            (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::WordOnly(_))
            | (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => false,
            (operators::OperatorClass::Comparison, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => true,
            (operators::OperatorClass::Comparison, operators::OperatorSyntax::SymbolOnly(_))
            | (operators::OperatorClass::Comparison, operators::OperatorSyntax::WordOnly(_)) => false,
            (operators::OperatorClass::Logical, operators::OperatorSyntax::WordOnly(_)) => true,
            (operators::OperatorClass::Logical, operators::OperatorSyntax::SymbolOnly(_))
            | (operators::OperatorClass::Logical, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => false,
        };
        if !lawful_shape {
            findings.push(format!(
                "operator {spelling} violates the class/shape law: arithmetic is symbol-only, \
                 comparison is word-with-symbol-alias, logical is word-only"));
        }
        let fixity = match op.fixity {
            operators::Fixity::Prefix => "Prefix",
            operators::Fixity::Infix => "Infix",
        };
        let alias_token = match op.syntax.symbol_alias() {
            Some(symbol) => symbol.token(),
            None => "",
        };
        for surface in [op.syntax.canonical_token(), alias_token] {
            if surface.is_empty() {
                continue;
            }
            if let Some(prev) = surface_owner.insert((surface, fixity), spelling) {
                if prev != spelling {
                    findings.push(format!(
                        "operator token {surface} fixity {fixity} claimed by {prev} and {spelling}"));
                }
            }
        }
        // Typed legality rules (5.5E1): placement is law. A wall-observation
        // difference anywhere but subtraction would leak wall arithmetic past
        // the TimeDelta fence (docs/16); a rate difference IS subtraction.
        if op.typing.is_empty() {
            findings.push(format!("operator {spelling} declares no typing rules"));
        }
        for rule in op.typing {
            // Exhaustive: a new rule must be classified here, not defaulted.
            match rule {
                operators::OperatorTypingRule::WallObservationDifference
                | operators::OperatorTypingRule::PercentDifference => {
                    if op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {spelling} claims a difference typing rule but is not subtraction"));
                    }
                }
                operators::OperatorTypingRule::PercentAdjustment => {
                    if op.semantic_op != "add" && op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {spelling} claims PercentAdjustment outside add/subtract"));
                    }
                }
                operators::OperatorTypingRule::SameSortComparison => {
                    if !matches!(op.class, operators::OperatorClass::Comparison) {
                        findings.push(format!(
                            "operator {spelling} claims SameSortComparison outside the comparison class"));
                    }
                }
                operators::OperatorTypingRule::TruthUnary
                | operators::OperatorTypingRule::TruthBinary => {
                    if !matches!(op.class, operators::OperatorClass::Logical) {
                        findings.push(format!(
                            "operator {spelling} claims a Truth typing rule outside the logical class"));
                    }
                }
                operators::OperatorTypingRule::SameUnit
                | operators::OperatorTypingRule::DimensionalByDimensionless
                | operators::OperatorTypingRule::LikeDimensionRatio => {
                    if !matches!(op.class, operators::OperatorClass::Arithmetic) {
                        findings.push(format!(
                            "operator {spelling} claims an arithmetic typing rule outside the arithmetic class"));
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

/// The typed proof relations and complete legacy witness routes, EXECUTED
/// (5.5E4d). Seedcheck runs the real values: nonempty supplemental fields,
/// exactly one witness route per obligation with the matching relation
/// posture, resolvable currently-active guarantees, nonempty duplicate-free
/// resolvable projection contracts, active retired-successors, and the
/// LEG/DEC-only guarantee family law. It parses no docs/24 meaning prose and
/// no generated Markdown.
fn check_proof_relations(root: &Path, findings: &mut Vec<String>) {
    use proof::{ProofRowState, PROOF_ROWS};
    let owner_texts = contract_authored_texts(root);
    let mut leg_active: BTreeMap<&str, usize> = BTreeMap::new();
    let active_ids: BTreeSet<&str> = PROOF_ROWS
        .iter()
        .filter(|r| matches!(r.state, ProofRowState::Active { .. }))
        .map(|r| r.id.raw())
        .collect();
    for record in PROOF_ROWS {
        match record.state {
            ProofRowState::Active { guarantee, projection_contracts } => {
                let raw = match guarantee {
                    guarantees::GuaranteeRef::Legacy(id) => {
                        let raw = id.raw();
                        *leg_active.entry(raw).or_insert(0) += 1;
                        let row = legacy_obligations::OBLIGATIONS.iter().find(|o| o.id == raw);
                        match row {
                            None => findings.push(format!(
                                "proof row {} binds {}, which no legacy obligation declares",
                                record.id.raw(), raw)),
                            Some(o) if !matches!(o.active_or_closed_status,
                                legacy_obligations::ObligationStatus::Active) => findings.push(
                                format!("proof row {} binds {}, which is not currently active",
                                    record.id.raw(), raw)),
                            _ => {}
                        }
                        raw
                    }
                    guarantees::GuaranteeRef::Decision(id) => {
                        let raw = id.raw();
                        match dispositions::DECISIONS.iter().find(|d| d.id == raw) {
                            None => findings.push(format!(
                                "proof row {} binds {}, which no decision declares",
                                record.id.raw(), raw)),
                            Some(d) if matches!(d.disposition.guarantee_lifetime(),
                                guarantees::GuaranteeLifetime::HistoricalCoverageOnly) =>
                                findings.push(format!(
                                    "proof row {} binds {}, which is historical coverage only",
                                    record.id.raw(), raw)),
                            _ => {}
                        }
                        raw
                    }
                    _ => {
                        findings.push(format!(
                            "proof row {} binds a non-LEG/DEC guarantee family; a new                              family enters when a real active row needs it",
                            record.id.raw()));
                        ""
                    }
                };
                let _ = raw;
                if projection_contracts.is_empty() {
                    findings.push(format!(
                        "active proof row {} names no projection contract", record.id.raw()));
                }
                let mut seen: BTreeSet<&str> = BTreeSet::new();
                for contract in projection_contracts {
                    if !seen.insert(contract.raw()) {
                        findings.push(format!(
                            "active proof row {} repeats projection contract {}",
                            record.id.raw(), contract.raw()));
                    }
                    if !owner_texts.contains_key(contract.raw()) {
                        findings.push(format!(
                            "active proof row {} names projection contract {}, which no                              authored document declares",
                            record.id.raw(), contract.raw()));
                    }
                }
            }
            ProofRowState::Retired { successors } => {
                for successor in successors {
                    if !active_ids.contains(successor.raw()) {
                        findings.push(format!(
                            "retired proof row {} names successor {}, which is not active",
                            record.id.raw(), successor.raw()));
                    }
                }
            }
        }
    }
    for obligation in legacy_obligations::OBLIGATIONS {
        if obligation.legacy_evidence.trim().is_empty() {
            findings.push(format!("{} carries no legacy evidence pointer", obligation.id));
        }
        if obligation.mechanism_disposition.trim().is_empty() {
            findings.push(format!("{} carries no mechanism disposition", obligation.id));
        }
        let active = leg_active.get(obligation.id).copied().unwrap_or(0);
        match obligation.witness_requirement {
            legacy_obligations::LegacyWitnessRequirement::CanonicalProofRows => {
                if active == 0 {
                    findings.push(format!(
                        "{} claims CanonicalProofRows with no active typed relation",
                        obligation.id));
                }
            }
            legacy_obligations::LegacyWitnessRequirement::Planned(text) => {
                if text.trim().is_empty() {
                    findings.push(format!("{} carries an empty planned witness", obligation.id));
                }
                if active > 0 {
                    findings.push(format!(
                        "{} carries a planned witness AND active typed relations",
                        obligation.id));
                }
            }
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

/// The Gate-0 materializer output shape, EXECUTED from the typed plan
/// (5.5E5). Seedcheck expands the same closed inventories the materializer
/// and the independent selftest oracle consume; it renders no TOML and
/// inspects no filesystem output -- the plan's internal lawfulness is the
/// subject here, and the isolated qualification owns the published tree.
fn check_bootstrap_output(findings: &mut Vec<String>) {
    use bootstrap_output as bo;

    // Each closed vocabulary is generated with its ALL inventory from one
    // variant list (5.5E5a), so enum-to-ALL divergence is unrepresentable and
    // no cardinality is asserted here. What remains executable: the inventory
    // is nonempty and carries no duplicate. A duplicate would already be a
    // compile error in the enum, so this is belt-and-braces evidence, count
    // free.
    for (name, len, dup) in [
        ("Gate0RootArtifact", bo::Gate0RootArtifact::ALL.len(), {
            let a = bo::Gate0RootArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PackageArtifact", bo::Gate0PackageArtifact::ALL.len(), {
            let a = bo::Gate0PackageArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PackageTargetKind", bo::Gate0PackageTargetKind::ALL.len(), {
            let a = bo::Gate0PackageTargetKind::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PlaneArtifact", bo::Gate0PlaneArtifact::ALL.len(), {
            let a = bo::Gate0PlaneArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0OutputDisposition", bo::Gate0OutputDisposition::ALL.len(), {
            let a = bo::Gate0OutputDisposition::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
    ] {
        if len == 0 {
            findings.push(format!("{name}::ALL is empty"));
        }
        if dup {
            findings.push(format!("{name}::ALL lists a variant twice"));
        }
    }

    // Workspace metadata is nonempty.
    if bo::WORKSPACE_LICENSE.trim().is_empty() {
        findings.push("bootstrap-output workspace license is empty".into());
    }
    if bo::WORKSPACE_REPOSITORY.trim().is_empty() {
        findings.push("bootstrap-output workspace repository is empty".into());
    }

    // Expand the complete plan's PATHS exactly as the materializer does:
    // roots, then three files per package, then two files per plane. Every
    // path must be nonempty, relative, traversal-free, and unique.
    let mut planned: Vec<String> = Vec::new();
    for &artifact in bo::Gate0RootArtifact::ALL {
        planned.push(artifact.relative_path().to_owned());
    }
    let mut roots: BTreeSet<&str> = BTreeSet::new();
    for &artifact in bo::Gate0RootArtifact::ALL {
        if !roots.insert(artifact.relative_path()) {
            findings.push(format!("duplicate Gate0 root path {}", artifact.relative_path()));
        }
    }
    for &package in architecture::PackageId::ALL {
        let base = package.workspace_path();
        // one target kind per package (total function), and a binary target
        // name exactly where the kind requires one
        let kind = bo::target_kind(package);
        let named = bo::binary_target(package).is_some();
        let needs_name = matches!(
            kind,
            bo::Gate0PackageTargetKind::Binary | bo::Gate0PackageTargetKind::ExampleBinary
        );
        if needs_name && !named {
            findings.push(format!("{} is a binary-kind package with no binary target name", package.cargo_name()));
        }
        if !needs_name && named {
            findings.push(format!("{} is a library-kind package carrying a binary target name", package.cargo_name()));
        }
        if bo::source_suffix(package).trim().is_empty() {
            findings.push(format!("{} has an empty source door", package.cargo_name()));
        }
        // exactly one path per artifact family
        for &family in bo::Gate0PackageArtifact::ALL {
            let suffix = match family {
                bo::Gate0PackageArtifact::Manifest => "Cargo.toml",
                bo::Gate0PackageArtifact::Readme => "README.md",
                bo::Gate0PackageArtifact::SourceDoor => bo::source_suffix(package),
            };
            planned.push(format!("{base}/{suffix}"));
        }
    }
    for &plane in architecture::SyncBatPlane::ALL {
        let base = architecture::PackageId::SyncBat.workspace_path();
        let module = plane.module_name();
        if module.trim().is_empty() {
            findings.push(format!("SyncBat plane {plane:?} has an empty module name"));
        }
        for &family in bo::Gate0PlaneArtifact::ALL {
            planned.push(match family {
                bo::Gate0PlaneArtifact::Module => format!("{base}/src/{module}.rs"),
                bo::Gate0PlaneArtifact::DirectoryReadme => format!("{base}/src/{module}/README.md"),
            });
        }
    }
    // Every expanded path obeys the one portable-path law, and no two collide
    // under case folding (the authoritative host is case-insensitive).
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut folded: BTreeSet<String> = BTreeSet::new();
    for path in &planned {
        if !bo::is_portable_gate0_relative_path(path) {
            findings.push(format!("expanded Gate0 path {path} is not a portable relative path"));
        }
        if !folded.insert(path.to_ascii_lowercase()) {
            findings.push(format!("expanded Gate0 path {path} case-fold collides in the plan"));
        }
        if !seen.insert(path) {
            findings.push(format!("expanded Gate0 path {path} is planned twice"));
        }
    }
}

/// The typed Tier 0 qualification algebra, EXECUTED (5.5E6a; rebuilt for the
/// evidence-verification algebra in 5.5E6b). Seedcheck runs the real `verify`
/// over an honest whole qualification and over a perturbation for EVERY rule in
/// `Tier0VerificationRule::ALL` — no rule is a fire alarm whose battery was
/// never installed. The concrete digest recomputation lives in
/// bootstrap/receiptcheck.rs; here we exercise the typed shape/denominator/
/// target/source/hosted-run laws.
fn check_bootstrap_qualification(findings: &mut Vec<String>) {
    use bootstrap_qualification as bq;
    use toolchain::{RustRelease, RustTargetTriple, AUTHORITATIVE_TARGET};

    // Every kind has a nonempty unique slug and a total artifact policy.
    let mut slugs: Vec<&str> = Vec::new();
    for &kind in bq::Tier0ReceiptKind::ALL {
        if kind.slug().is_empty() {
            findings.push(format!("Tier0ReceiptKind {kind:?} has an empty slug"));
        }
        if slugs.contains(&kind.slug()) {
            findings.push(format!("Tier0 slug {} is claimed twice", kind.slug()));
        }
        slugs.push(kind.slug());
        let _ = kind.artifact_policy();
    }

    // Honest evidence exactly matching a kind's policy.
    fn evidence(kind: bq::Tier0ReceiptKind) -> bq::Tier0ArtifactEvidence {
        match kind.artifact_policy() {
            bq::Tier0ArtifactPolicy::FixtureSet => bq::Tier0ArtifactEvidence::FixtureSet {
                digest: bq::Sha256Digest::from_bytes([0u8; 32]),
            },
            bq::Tier0ArtifactPolicy::Executable => bq::Tier0ArtifactEvidence::Executable {
                digest: bq::Sha256Digest::from_bytes([1u8; 32]),
            },
            bq::Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                bq::Tier0ArtifactEvidence::ExecutableAndOutputTree {
                    executable_digest: bq::Sha256Digest::from_bytes([2u8; 32]),
                    output_tree_digest: bq::Sha256Digest::from_bytes([3u8; 32]),
                }
            }
        }
    }
    fn receipt(
        kind: bq::Tier0ReceiptKind,
        target: RustTargetTriple,
    ) -> bq::Tier0ReceiptObservation {
        bq::Tier0ReceiptObservation {
            kind,
            target,
            available: true,
            compilation: bq::CompilationOutcome::Succeeded,
            execution: bq::ExecutionAttempt::Attempted,
            outcome: Some(bq::ExecutionOutcome::Passed),
            artifact: Some(evidence(kind)),
        }
    }

    let git_source = || bq::SourceBinding::GitCheckout {
        commit: bq::GitCommitSha::from_bytes([0u8; 20]),
        tree: bq::GitTreeSha::from_bytes([0u8; 20]),
        spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
        workflow_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let export_source = || bq::SourceBinding::FrozenExport {
        spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
        export_tree_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let toolchain = bq::ToolchainBinding {
        rustc_release: RustRelease { major: 1, minor: 97, patch: 0 },
        rustc_commit: bq::ToolchainCommit::from_bytes([0u8; 20]),
        cargo_release: RustRelease { major: 1, minor: 97, patch: 0 },
        cargo_commit: bq::ToolchainCommit::from_bytes([0u8; 20]),
        toolchain_file_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let runtime = bq::BootstrapRuntimeBinding {
        python_release: bq::AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE,
    };
    let hosted = || {
        Some(bq::GitHubActionsRunBinding {
            repository: "freebatteryfactory/batpak".to_owned(),
            workflow_path: ".github/workflows/msvc-qualification.yml".to_owned(),
            run_id: 1,
            run_attempt: 1,
            runner_image_os: "Windows".to_owned(),
            runner_image_version: "0".to_owned(),
        })
    };
    let build = |target: RustTargetTriple,
                 hosted_run: Option<bq::GitHubActionsRunBinding>,
                 source: bq::SourceBinding|
     -> bq::Tier0QualificationObservation {
        bq::Tier0QualificationObservation {
            source,
            toolchain,
            bootstrap_runtime: runtime,
            target,
            hosted_run,
            receipts: bq::Tier0ReceiptKind::ALL
                .iter()
                .map(|&k| receipt(k, target))
                .collect(),
        }
    };

    // The honest MSVC qualification verifies; the honest supplemental GNU
    // qualification (frozen export, no hosted run) also verifies.
    if let Err(errs) = bq::verify(build(AUTHORITATIVE_TARGET, hosted(), git_source())) {
        let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
        findings.push(format!("an honest authoritative qualification was refused: {rules:?}"));
    }
    if let Err(errs) = bq::verify(build(
        RustTargetTriple::X86_64PcWindowsGnu,
        None,
        export_source(),
    )) {
        let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
        findings.push(format!("an honest supplemental GNU qualification was refused: {rules:?}"));
    }

    // Perturb the honest MSVC qualification once per rule; verify must refuse
    // AND report that specific rule.
    let want = |mut obs: bq::Tier0QualificationObservation,
                rule: bq::Tier0VerificationRule,
                label: &str,
                mutate: &dyn Fn(&mut bq::Tier0QualificationObservation),
                findings: &mut Vec<String>| {
        mutate(&mut obs);
        match bq::verify(obs) {
            Ok(_) => findings.push(format!("{label}: an incoherent qualification verified")),
            Err(errs) => {
                if !errs.iter().any(|e| e.rule() == rule) {
                    let got: Vec<_> = errs.iter().map(|e| e.rule()).collect();
                    findings.push(format!("{label}: refused, but not for {rule:?}: {got:?}"));
                }
            }
        }
    };
    let base = || build(AUTHORITATIVE_TARGET, hosted(), git_source());
    use bq::Tier0VerificationRule as R;
    want(base(), R::ReceiptNotAvailable, "not_available",
         &|o| o.receipts[0].available = false, findings);
    want(base(), R::ExecutionOutcomeWithoutAttempt, "outcome_without_attempt",
         &|o| o.receipts[0].execution = bq::ExecutionAttempt::NotAttempted, findings);
    want(base(), R::ExecutionAttemptWithoutOutcome, "attempt_without_outcome",
         &|o| o.receipts[0].outcome = None, findings);
    want(base(), R::ExecutionAfterFailedCompilation, "execution_after_failed_compilation",
         &|o| o.receipts[0].compilation = bq::CompilationOutcome::Failed, findings);
    want(base(), R::ReceiptNotPassed, "not_passed",
         &|o| o.receipts[0].outcome = Some(bq::ExecutionOutcome::Failed), findings);
    want(base(), R::ArtifactMissing, "artifact_missing",
         &|o| o.receipts[0].artifact = None, findings);
    want(base(), R::ArtifactPolicyMismatch, "artifact_policy_mismatch",
         &|o| o.receipts[1].artifact = Some(bq::Tier0ArtifactEvidence::FixtureSet {
             digest: bq::Sha256Digest::from_bytes([9u8; 32]),
         }), findings);
    want(base(), R::ReceiptTargetMismatch, "receipt_target_mismatch",
         &|o| o.receipts[0].target = RustTargetTriple::X86_64PcWindowsGnu, findings);
    want(base(), R::MissingRequiredReceipt, "missing_required_receipt",
         &|o| { o.receipts.pop(); }, findings);
    want(base(), R::DuplicateReceipt, "duplicate_receipt",
         &|o| { let last = o.receipts[o.receipts.len() - 1]; o.receipts.push(last); }, findings);
    want(base(), R::ReceiptsOutOfCanonicalOrder, "out_of_canonical_order",
         &|o| o.receipts.reverse(), findings);
    want(base(), R::SourceCannotQualifyAuthoritativeTarget, "export_cannot_qualify_authoritative",
         &|o| o.source = bq::SourceBinding::FrozenExport {
             spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
             export_tree_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
         }, findings);
    want(base(), R::AuthoritativeTargetWithoutHostedRun, "authoritative_without_hosted_run",
         &|o| o.hosted_run = None, findings);
    want(base(), R::AuthoritativeBootstrapRuntimeMismatch, "authoritative_wrong_python_runtime",
         &|o| o.bootstrap_runtime = bq::BootstrapRuntimeBinding {
             python_release: bq::PythonRelease { major: 3, minor: 11, patch: 9 },
         }, findings);

    // Every rule names a nonempty law, a repair, and one of the three declared
    // owners — no rule is unowned or unexplained.
    for &rule in bq::Tier0VerificationRule::ALL {
        if rule.law().is_empty() {
            findings.push(format!("verification rule {rule:?} has an empty law"));
        }
        if rule.repair().is_empty() {
            findings.push(format!("verification rule {rule:?} has an empty repair"));
        }
        let owner = rule.owner();
        let known = owner == bq::TIER0_RECEIPT_ALGEBRA_OWNER
            || owner == bq::TIER0_DENOMINATOR_OWNER
            || owner == bq::TIER0_HOSTED_QUALIFICATION_OWNER;
        if !known {
            findings.push(format!("verification rule {rule:?} names an unknown owner {owner:?}"));
        }
    }
}

/// The cross-run same-source comparator, EXECUTED (5.5E6c). Seedcheck builds
/// verified qualifications through the sealed `verify` and runs `compare_runs`
/// across every outcome: two independent runs of one source prove SameSource;
/// each source-identity coordinate, perturbed alone, is named as a divergence;
/// a frozen export, a missing hosted run, and a run compared to itself are each
/// refused as NotComparable; and a differing toolchain or target is RECORDED as
/// agreement strength while the same-source verdict still holds. The design is
/// grounded in real evidence: two hosted MSVC runs of one commit differ in their
/// executable digests but agree on every source coordinate, so executable
/// digests are deliberately varied here and never treated as a divergence.
fn check_tier0_cross_run(findings: &mut Vec<String>) {
    use bootstrap_qualification as bq;
    use tier0_cross_run as xr;
    use toolchain::{RustRelease, RustTargetTriple, AUTHORITATIVE_TARGET};

    // A controllable qualification: each source-identity coordinate, the
    // toolchain, the run identity, the target, the source posture, and the
    // (same-source-irrelevant) executable digest is a dial.
    #[derive(Clone, Copy)]
    struct Spec {
        commit: u8,
        tree: u8,
        manifest: u8,
        workflow: u8,
        otree: u8,
        tc: u8,
        run_id: u64,
        target: RustTargetTriple,
        repository: &'static str,
        workflow_path: &'static str,
        git: bool,
        hosted: bool,
        exe: u8,
        python: (u16, u16, u16),
    }

    let evidence = |kind: bq::Tier0ReceiptKind, exe: u8, otree: u8| -> bq::Tier0ArtifactEvidence {
        match kind.artifact_policy() {
            bq::Tier0ArtifactPolicy::FixtureSet => bq::Tier0ArtifactEvidence::FixtureSet {
                digest: bq::Sha256Digest::from_bytes([exe; 32]),
            },
            bq::Tier0ArtifactPolicy::Executable => bq::Tier0ArtifactEvidence::Executable {
                digest: bq::Sha256Digest::from_bytes([exe; 32]),
            },
            bq::Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                bq::Tier0ArtifactEvidence::ExecutableAndOutputTree {
                    executable_digest: bq::Sha256Digest::from_bytes([exe; 32]),
                    output_tree_digest: bq::Sha256Digest::from_bytes([otree; 32]),
                }
            }
        }
    };

    let verified = |s: &Spec,
                    label: &str,
                    findings: &mut Vec<String>|
     -> Option<bq::VerifiedTier0Qualification> {
        let source = if s.git {
            bq::SourceBinding::GitCheckout {
                commit: bq::GitCommitSha::from_bytes([s.commit; 20]),
                tree: bq::GitTreeSha::from_bytes([s.tree; 20]),
                spec_manifest_digest: bq::Sha256Digest::from_bytes([s.manifest; 32]),
                workflow_digest: bq::Sha256Digest::from_bytes([s.workflow; 32]),
            }
        } else {
            bq::SourceBinding::FrozenExport {
                spec_manifest_digest: bq::Sha256Digest::from_bytes([s.manifest; 32]),
                export_tree_digest: bq::Sha256Digest::from_bytes([s.tree; 32]),
            }
        };
        let hosted_run = if s.hosted {
            Some(bq::GitHubActionsRunBinding {
                repository: s.repository.to_owned(),
                workflow_path: s.workflow_path.to_owned(),
                run_id: s.run_id,
                run_attempt: 1,
                runner_image_os: "Windows".to_owned(),
                runner_image_version: "0".to_owned(),
            })
        } else {
            None
        };
        let toolchain = bq::ToolchainBinding {
            rustc_release: RustRelease { major: 1, minor: 97, patch: 0 },
            rustc_commit: bq::ToolchainCommit::from_bytes([s.tc; 20]),
            cargo_release: RustRelease { major: 1, minor: 97, patch: 0 },
            cargo_commit: bq::ToolchainCommit::from_bytes([s.tc; 20]),
            toolchain_file_digest: bq::Sha256Digest::from_bytes([s.tc; 32]),
        };
        let obs = bq::Tier0QualificationObservation {
            source,
            toolchain,
            bootstrap_runtime: bq::BootstrapRuntimeBinding {
                python_release: bq::PythonRelease {
                    major: s.python.0,
                    minor: s.python.1,
                    patch: s.python.2,
                },
            },
            target: s.target,
            hosted_run,
            receipts: bq::Tier0ReceiptKind::ALL
                .iter()
                .map(|&k| bq::Tier0ReceiptObservation {
                    kind: k,
                    target: s.target,
                    available: true,
                    compilation: bq::CompilationOutcome::Succeeded,
                    execution: bq::ExecutionAttempt::Attempted,
                    outcome: Some(bq::ExecutionOutcome::Passed),
                    artifact: Some(evidence(k, s.exe, s.otree)),
                })
                .collect(),
        };
        match bq::verify(obs) {
            Ok(v) => Some(v),
            Err(errs) => {
                let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
                findings.push(format!("cross-run fixture {label} did not verify: {rules:?}"));
                None
            }
        }
    };

    let base = Spec {
        commit: 1,
        tree: 2,
        manifest: 3,
        workflow: 4,
        otree: 5,
        tc: 6,
        run_id: 100,
        target: AUTHORITATIVE_TARGET,
        repository: "owner/repo",
        workflow_path: bq::AUTHORITATIVE_WORKFLOW_PATH,
        git: true,
        hosted: true,
        exe: 7,
        python: (3, 12, 10),
    };

    let compare = |right: &Spec,
                   label: &str,
                   findings: &mut Vec<String>|
     -> Option<xr::CrossRunComparison> {
        let l = verified(&base, label, findings)?;
        let r = verified(right, label, findings)?;
        Some(xr::compare_runs(&l, &r))
    };

    // Two independent runs of one source: SameSource, retaining the common
    // coordinates and recording identical toolchain and target. The executable
    // digest differs (exe 77 vs base 7) and MUST NOT block the verdict.
    let right = Spec { run_id: 200, exe: 77, ..base };
    if let Some(cmp) = compare(&right, "same-source", findings) {
        match cmp {
            xr::CrossRunComparison::SameSource(p) => {
                if p.materializer_output_tree() != bq::Sha256Digest::from_bytes([5u8; 32]) {
                    findings.push("cross-run same-source retained the wrong output tree".to_owned());
                }
                if !matches!(p.toolchain_agreement(), xr::ToolchainAgreement::Identical(_)) {
                    findings.push("cross-run same-source did not record identical toolchain".to_owned());
                }
                if !matches!(p.target_agreement(), xr::TargetAgreement::SameTarget(_)) {
                    findings.push("cross-run same-source did not record the same target".to_owned());
                }
                if p.left_run().run_id == p.right_run().run_id {
                    findings.push("cross-run same-source retained one run identity twice".to_owned());
                }
            }
            other => findings.push(format!("cross-run same-source expected SameSource, got {other:?}")),
        }
    }

    // Each source-identity coordinate, perturbed ALONE (with a distinct run so
    // the independence precondition passes), is named as its own divergence.
    let divergence_cases: &[(Spec, &str)] = &[
        (Spec { run_id: 200, commit: 9, ..base }, "commit"),
        (Spec { run_id: 200, tree: 9, ..base }, "tree"),
        (Spec { run_id: 200, manifest: 9, ..base }, "spec-manifest"),
        (Spec { run_id: 200, workflow: 9, ..base }, "workflow"),
        (Spec { run_id: 200, otree: 9, ..base }, "output-tree"),
    ];
    for (right, coord) in divergence_cases {
        if let Some(cmp) = compare(right, coord, findings) {
            match cmp {
                xr::CrossRunComparison::DifferentSource(divs) => {
                    let named = divs.iter().any(|d| match (coord, d) {
                        (&"commit", xr::SourceDivergence::SourceCommit { .. }) => true,
                        (&"tree", xr::SourceDivergence::SourceTree { .. }) => true,
                        (&"spec-manifest", xr::SourceDivergence::SpecManifestDigest { .. }) => true,
                        (&"workflow", xr::SourceDivergence::WorkflowDigest { .. }) => true,
                        (&"output-tree", xr::SourceDivergence::MaterializerOutputTree { .. }) => true,
                        _ => false,
                    });
                    if !named {
                        findings.push(format!(
                            "cross-run {coord} divergence was not named: {divs:?}"
                        ));
                    }
                }
                other => findings.push(format!(
                    "cross-run {coord} divergence expected DifferentSource, got {other:?}"
                )),
            }
        }
    }

    // A frozen export cannot be compared: no committed source coordinates.
    let frozen = Spec {
        run_id: 200,
        git: false,
        hosted: false,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        ..base
    };
    match compare(&frozen, "frozen-export", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::FrozenExportSource {
            which: xr::Side::Right,
        })) => {}
        Some(other) => findings.push(format!(
            "cross-run frozen export expected NotComparable::FrozenExportSource, got {other:?}"
        )),
        None => {}
    }

    // A git checkout with no hosted run is not one of two hosted runs.
    let no_run = Spec {
        run_id: 200,
        hosted: false,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        ..base
    };
    match compare(&no_run, "missing-hosted-run", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::MissingHostedRun {
            which: xr::Side::Right,
        })) => {}
        Some(other) => findings.push(format!(
            "cross-run missing hosted run expected NotComparable::MissingHostedRun, got {other:?}"
        )),
        None => {}
    }

    // One run compared to itself (same repository, run id, and attempt) is not
    // two independent witnesses.
    let same_run = base;
    match compare(&same_run, "same-hosted-run", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::SameHostedRun)) => {}
        Some(other) => findings.push(format!(
            "cross-run same hosted run expected NotComparable::SameHostedRun, got {other:?}"
        )),
        None => {}
    }

    // A differing toolchain does not weaken the same-source verdict; it is
    // recorded as divergent agreement strength.
    let other_toolchain = Spec { run_id: 200, tc: 99, ..base };
    match compare(&other_toolchain, "toolchain-divergent", findings) {
        Some(xr::CrossRunComparison::SameSource(p)) => {
            if !matches!(p.toolchain_agreement(), xr::ToolchainAgreement::Divergent { .. }) {
                findings.push("cross-run differing toolchain was not recorded as divergent".to_owned());
            }
        }
        Some(other) => findings.push(format!(
            "cross-run differing toolchain expected SameSource, got {other:?}"
        )),
        None => {}
    }

    // The same source qualified on a second target is still SameSource, recorded
    // as a cross-target pair. A GNU git checkout WITH a hosted run is comparable,
    // and a supplemental lane may run under a different (non-authoritative)
    // CPython release — recorded as a divergent bootstrap-runtime agreement.
    let cross_target = Spec {
        run_id: 200,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        exe: 55,
        python: (3, 11, 9),
        ..base
    };
    match compare(&cross_target, "cross-target", findings) {
        Some(xr::CrossRunComparison::SameSource(p)) => {
            if !matches!(p.target_agreement(), xr::TargetAgreement::CrossTarget { .. }) {
                findings.push("cross-run cross-target pair was not recorded as cross-target".to_owned());
            }
            if !matches!(
                p.bootstrap_runtime_agreement(),
                xr::BootstrapRuntimeAgreement::Divergent { .. }
            ) {
                findings.push("cross-run differing bootstrap runtime was not recorded as divergent".to_owned());
            }
        }
        Some(other) => findings.push(format!(
            "cross-run cross-target expected SameSource, got {other:?}"
        )),
        None => {}
    }

    // Promotion confirmation (5.5E6c1), EXECUTED: strictly stronger than
    // same-source. The canonical candidate/cleanroom posture confirms; each
    // missing posture requirement refuses even where same-source still holds.
    let confirm = |right: &Spec,
                   label: &str,
                   findings: &mut Vec<String>|
     -> Option<Result<xr::PromotionConfirmationProof, xr::PromotionConfirmationError>> {
        let c = verified(&base, label, findings)?;
        let r = verified(right, label, findings)?;
        Some(xr::confirm_promotion(&c, &r))
    };

    // Two authoritative, same-source, identical-toolchain runs in the same
    // repository and canonical workflow confirm the promotion. The executable
    // digest still differs (exe 77 vs base 7).
    let promote_ok = Spec { run_id: 200, exe: 77, ..base };
    match confirm(&promote_ok, "confirm-ok", findings) {
        Some(Ok(p)) => {
            if !p.target().is_authoritative() {
                findings.push("promotion confirmation did not bind the authoritative target".to_owned());
            }
            if p.candidate_run().run_id == p.cleanroom_run().run_id {
                findings.push("promotion confirmation retained one run identity twice".to_owned());
            }
        }
        Some(Err(e)) => findings.push(format!("promotion confirmation refused an honest pair: {e:?}")),
        None => {}
    }

    // One run confirmed against itself is not two witnesses; promotion
    // confirmation inherits the same-source refusal.
    match confirm(&base, "confirm-same-run", findings) {
        Some(Err(xr::PromotionConfirmationError::NotSameSource(
            xr::CrossRunComparison::NotComparable(xr::NotComparable::SameHostedRun),
        ))) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation of one run against itself expected NotSameSource/SameHostedRun, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair on a non-authoritative (supplemental GNU) target does
    // not confirm a promotion.
    let promote_gnu = Spec {
        run_id: 200,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        exe: 55,
        ..base
    };
    match confirm(&promote_gnu, "confirm-non-authoritative", findings) {
        Some(Err(xr::PromotionConfirmationError::NonAuthoritativeTarget {
            which: xr::Side::Right,
            ..
        })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation on a non-authoritative target expected NonAuthoritativeTarget, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair on a divergent toolchain does not confirm.
    let promote_tc = Spec { run_id: 200, tc: 99, ..base };
    match confirm(&promote_tc, "confirm-toolchain", findings) {
        Some(Err(xr::PromotionConfirmationError::ToolchainDivergent { .. })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation on a divergent toolchain expected ToolchainDivergent, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair in a different repository does not confirm.
    let promote_repo = Spec { run_id: 200, repository: "other/repo", ..base };
    match confirm(&promote_repo, "confirm-repository", findings) {
        Some(Err(xr::PromotionConfirmationError::RepositoryMismatch { .. })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation across repositories expected RepositoryMismatch, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair whose workflow path is not the canonical authoritative
    // workflow does not confirm.
    let promote_wf = Spec {
        run_id: 200,
        workflow_path: ".github/workflows/not-canonical.yml",
        ..base
    };
    match confirm(&promote_wf, "confirm-workflow-path", findings) {
        Some(Err(xr::PromotionConfirmationError::NonCanonicalWorkflowPath {
            which: xr::Side::Right,
            ..
        })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation with a non-canonical workflow path expected NonCanonicalWorkflowPath, got {other:?}"
        )),
        None => {}
    }
}

fn check_syncbat_shape(root: &Path, findings: &mut Vec<String>) {
    // The signed seed carries no generated workspace (5.5E5): plane files are
    // checked only where a materialized candidate is present, and their paths
    // derive from SyncBatPlane::ALL, never from a raw path table.
    let base = root.join("crates/syncbat");
    if !base.exists() { return; }
    for &plane in architecture::SyncBatPlane::ALL {
        let module = plane.module_name();
        if !base.join(format!("src/{module}.rs")).exists() {
            findings.push(format!("syncbat missing required plane src/{module}.rs"));
        }
        if !base.join(format!("src/{module}")).exists() {
            findings.push(format!("syncbat missing required plane src/{module}"));
        }
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
    use architecture::SyncBatPlane;
    use syncbat_firewall::*;

    // Every authority has exactly one owning plane, and every plane owns at
    // least one authority. A plane owning nothing is not a plane; an authority
    // owned by nobody has no one to refuse its misuse.
    for &authority in SYNCBAT_AUTHORITIES {
        let owner = authority.owner();
        if !SyncBatPlane::ALL.contains(&owner) {
            findings.push(format!("{authority:?} is owned by a plane outside SyncBat"));
        }
    }
    for &plane in SyncBatPlane::ALL {
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
    use architecture::SyncBatPlane;
    use syncbat_firewall::*;

    let owner_of = |a: SyncBatAuthority| a.owner();
    let root = owner_of(SyncBatAuthority::LogicalResult);
    let source = owner_of(SyncBatAuthority::CompositionAndInstanceIdentity);

    // Reachability over Required crossings only. Small fixed graph; a linear
    // relaxation to a fixed point needs no allocation beyond a bitset.
    let reaches = |start: SyncBatPlane, goal: SyncBatPlane| -> bool {
        let mut seen = [false; 5];
        let idx = |p: SyncBatPlane| SyncBatPlane::ALL.iter().position(|&q| q == p).unwrap();
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

    for &plane in SyncBatPlane::ALL {
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

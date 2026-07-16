#![deny(warnings)]

#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/gates.rs"]
mod gates;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/architecture.rs"]
mod architecture;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/guarantees.rs"]
mod guarantees;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/invariants.rs"]
mod invariants;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/dispositions.rs"]
mod dispositions;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/legacy_obligations.rs"]
mod legacy_obligations;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/legacy_invariant_coverage.rs"]
mod legacy_invariant_coverage;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/operators.rs"]
mod operators;
#[allow(dead_code)] // declarative spec surface; not this binary's program
#[path = "../spec/pakvm_isa.rs"]
mod pakvm_isa;

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
    check_unique_ids(&mut findings);
    check_frontmatter(root, &mut findings);
    check_syncbat_shape(root, &mut findings);
    check_source_debt(root, &mut findings);
    findings
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
    let profiles = architecture::AUTHENTICATED_HISTORY_PROFILES;
    if profiles.len() != 3 {
        findings.push(format!("expected 3 authenticated-history profiles, found {}", profiles.len()));
    }
    let mut seen = BTreeSet::new();
    for spec in profiles {
        // Exhaustive: a new profile must be classified here, not defaulted.
        match spec.profile {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => {}
        }
        if !seen.insert(format!("{:?}", spec.profile)) {
            findings.push(format!("duplicate authenticated-history profile {:?}", spec.profile));
        }
        if !spec.requires_local_commitment_verification {
            findings.push(format!("{:?} does not require local commitment verification", spec.profile));
        }
        for policy in spec.permitted_witness_policies {
            match policy {
                WitnessPolicy::None | WitnessPolicy::Optional | WitnessPolicy::Required => {}
            }
            if *policy == WitnessPolicy::Required
                && spec.profile != AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            {
                findings.push(format!(
                    "{:?} permits WitnessPolicy::Required outside ExternallyAnchoredHistory",
                    spec.profile
                ));
            }
        }
        if spec.implementation_gates.is_empty() {
            findings.push(format!("{:?} names no implementation gate", spec.profile));
        }
        if spec.release_qualification_gates.is_empty() {
            findings.push(format!("{:?} names no release qualification gate", spec.profile));
        }
        // A success bundle states all four axes. Every success verifies
        // integrity, and freshness never drifts from rollback resistance.
        for bundle in [spec.unanchored_success_claims, spec.verified_witness_success_claims]
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
            let scoped = bundle.rollback_resistance == RollbackResistanceClaim::ScopedToVerifiedWitness;
            if fresh != scoped {
                findings.push(format!(
                    "{:?} lets freshness and rollback resistance drift apart",
                    spec.profile
                ));
            }
        }
        // An unanchored success never claims freshness or scoped rollback
        // resistance: a restored older validly signed history satisfies it.
        if let Some(bundle) = spec.unanchored_success_claims {
            if bundle.freshness != FreshnessClaim::NotClaimed
                || bundle.rollback_resistance != RollbackResistanceClaim::Unavailable
            {
                findings.push(format!(
                    "{:?} unanchored success claims freshness or rollback resistance",
                    spec.profile
                ));
            }
            if spec.profile == AuthenticatedHistoryProfile::InternalConsistency
                && bundle.authenticity != AuthenticityClaim::NotClaimed
            {
                findings.push("InternalConsistency unanchored success claims signed authenticity".into());
            }
        }
        // `None` here means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED.
        if spec.profile == AuthenticatedHistoryProfile::ExternallyAnchoredHistory
            && spec.unanchored_success_claims.is_some()
        {
            findings.push(
                "ExternallyAnchoredHistory admits an unanchored success bundle; an absent or invalid \
                 required witness must refuse, not fall back to a weaker success"
                    .into(),
            );
        }
        if spec.profile != AuthenticatedHistoryProfile::InternalConsistency
            && spec.verified_witness_success_claims.is_none()
        {
            findings.push(format!("{:?} admits no witnessed success bundle", spec.profile));
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
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.package).collect();
    let layers: BTreeMap<&str, u8> = architecture::PACKAGES.iter().map(|p| (p.package, p.layer)).collect();
    if packages.len() != architecture::PACKAGES.len() { findings.push("duplicate package name".into()); }
    let paths: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.path).collect();
    if paths.len() != architecture::PACKAGES.len() { findings.push("duplicate package path".into()); }
    for package in architecture::PACKAGES {
        if package.role.trim().is_empty() { findings.push(format!("empty role for {}", package.package)); }
        if package.path.trim().is_empty() { findings.push(format!("empty path for {}", package.package)); }
    }
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in architecture::EDGES {
        if !packages.contains(edge.importer) { findings.push(format!("unknown importer {}", edge.importer)); }
        if !packages.contains(edge.importee) { findings.push(format!("unknown importee {}", edge.importee)); }
        if edge.importer == edge.importee { findings.push(format!("self dependency {}", edge.importer)); }
        if edge.profile.is_empty() { findings.push(format!("edge has empty profile {} -> {}", edge.importer, edge.importee)); }
        if let (Some(importer_layer), Some(importee_layer)) = (layers.get(edge.importer), layers.get(edge.importee)) {
            if importer_layer <= importee_layer {
                findings.push(format!("dependency direction violation {}(L{}) -> {}(L{})", edge.importer, importer_layer, edge.importee, importee_layer));
            }
        }
        if edge.importer == "testpak" && edge.class != architecture::EdgeClass::DevOnly {
            findings.push(format!("testpak edge must be dev-only: {}", edge.importee));
        }
        if edge.importee == "batpak-examples" {
            findings.push(format!("nothing may depend on the examples package: {} -> batpak-examples", edge.importer));
        }
        if edge.importer == "batpak-examples" && edge.importee == "testpak" {
            findings.push("batpak-examples must not depend on dev tooling (testpak)".to_string());
        }
        if edge.importer == "batpak-cli" && edge.class == architecture::EdgeClass::DevOnly {
            findings.push(format!("CLI edge cannot be dev-only: {}", edge.importee));
        }
        graph.entry(edge.importer).or_default().push(edge.importee);
    }
    for package in architecture::PACKAGES {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if cycle(package.package, &graph, &mut visiting, &mut visited) {
            findings.push(format!("dependency cycle reaches {}", package.package));
        }
    }
}

fn check_profiles(findings: &mut Vec<String>) {
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.package).collect();
    let mut identities = BTreeSet::new();
    for profile in architecture::QUALIFICATION_PROFILES {
        if !packages.contains(profile.package) {
            findings.push(format!("unknown qualification package {}", profile.package));
        }
        if profile.profile.trim().is_empty() || profile.target.trim().is_empty() || profile.requirement.trim().is_empty() {
            findings.push(format!("incomplete qualification profile {}:{}", profile.package, profile.profile));
        }
        if !identities.insert((profile.package, profile.profile)) {
            findings.push(format!("duplicate qualification profile {}:{}", profile.package, profile.profile));
        }
    }
    for package in ["batpak", "syncbat"] {
        if !architecture::QUALIFICATION_PROFILES.iter().any(|p| p.package == package && p.profile == "semantic" && p.target == "no_std + alloc") {
            findings.push(format!("missing no_std + alloc semantic profile for {package}"));
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
            | gates::GateId::G9
            | gates::GateId::GJ => {}
        }
    }
    let mut sorted_gate_ids = gate_ids.clone();
    sorted_gate_ids.sort();
    if gate_ids != sorted_gate_ids {
        findings.push("gate inventory is not in canonical GateId order".into());
    }
    // Canonical, duplicate-free, resolvable gate lists on every gate-bearing fact.
    let check_gates = |ident: &str, list: &[gates::GateId], findings: &mut Vec<String>| {
        if list.is_empty() {
            findings.push(format!("{} names no gate", ident));
        }
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
        if value.owner.trim().is_empty() || value.witness.trim().is_empty() {
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
    }
    for value in invariants::INVARIANTS {
        check_gates(value.id, value.gates, findings);
    }
    for value in legacy_obligations::OBLIGATIONS {
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
                for key in ["status:", "contract_id:", "authority_scope:", "supersedes:", "last_reconciled:"] {
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

#[cfg(test)]
mod pakvm_isa_tests {
    use crate::pakvm_isa::*;

    fn refusal(a: PakVmAdmission) -> &'static str {
        match a {
            PakVmAdmission::Refused(why) => why,
            PakVmAdmission::Admitted(_) => "ADMITTED",
        }
    }

    // The premise every probe below depends on: the node exists, its real policy
    // admits, and each mutation therefore changes a live outcome rather than a
    // dead one. A red light from an already-broken node would prove nothing.
    #[test]
    fn the_probe_targets_admit_before_mutation() {
        for &node in PAKVM_NODES {
            assert!(matches!(admit(node), PakVmAdmission::Admitted(_)), "{node:?}");
        }
    }

    #[test]
    fn an_internal_lowering_identity_is_not_a_semantic_node() {
        // A SpecializedPlan micro-op wearing a semantic node's identity.
        let ap = PakVmAlgebraPolicy {
            lowering: PakVmRule::AlgebraConstant(CandidateLoweringPosture::InternalLoweringIdentity),
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "an internal lowering identity is not a semantic node");
    }

    #[test]
    fn two_owners_for_one_field_is_refused() {
        // The algebra fixes the effect posture AND the class declares one.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            effect: Some(EffectPosture::ObservationalOnly),
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture has no single owner");
    }

    #[test]
    fn no_owner_for_one_field_is_refused() {
        // The algebra delegates and the class stays silent. Admission must fail
        // rather than let a downstream projector pick a value.
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::ClassDeclared,
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture has no single owner");
    }

    #[test]
    fn an_effect_node_claiming_purity_is_refused() {
        // Would slip past the validator that rejects Effect nodes in a pure
        // query image (docs/07, DEC-050).
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::AlgebraConstant(EffectPosture::Pure),
            capability: PakVmRule::AlgebraConstant(CapabilityRequirement::None),
            ..*algebra_policy(PakVmAlgebra::Effect).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::DurableAppend).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "effect posture and algebra disagree");
    }

    #[test]
    fn a_query_node_claiming_effectfulness_is_refused() {
        // The other direction: smuggling an effect into the query algebra.
        let ap = PakVmAlgebraPolicy {
            effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),
            ..*algebra_policy(PakVmAlgebra::QueryDataflow).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::RowTransform).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "effect posture and algebra disagree");
    }

    #[test]
    fn an_effect_without_its_declared_capability_is_refused() {
        let ap = PakVmAlgebraPolicy {
            capability: PakVmRule::AlgebraConstant(CapabilityRequirement::ReadOnlySourceEnvelope),
            ..*algebra_policy(PakVmAlgebra::Effect).unwrap()
        };
        let cp = *class_policy(PakVmNodeClass::DurableAppend).unwrap();
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "effect capability is required by a node that declares no effect");
    }

    #[test]
    fn a_pure_node_requiring_a_capability_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            capability: Some(CapabilityRequirement::ReadOnlySourceEnvelope),
            ..*class_policy(PakVmNodeClass::ScalarComputation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Compare, &ap, &cp)),
                   "a pure node requires a capability");
    }

    #[test]
    fn a_cost_law_disagreeing_with_iteration_is_refused() {
        // The mutation must stay ON its algebra's work plane, or the plane rule
        // fires first and this probe stops testing the rule it names: a red light
        // from the wrong severed wire is not evidence the alarm works.
        //
        // KernelCostContract is on the formula/decision plane and carries the
        // mandatory Instructions unit, so it clears every earlier check. Giving it
        // to ScalarComputation, whose boundedness is ConstantWork, leaves a node
        // that claims one interpreted step while costing a kernel's contract.
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::KernelCostContract,
            ..*class_policy(PakVmNodeClass::ScalarComputation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Compare, &ap, &cp)),
                   "boundedness posture and work formula disagree");
    }

    #[test]
    fn an_iterating_node_costing_one_instruction_is_refused() {
        // A scan declaring constant cost would make bounded traversal
        // unmeasurable: the budget could never bite. Two independent laws reject
        // it, and the work plane is the one that reaches it first — Instructions
        // is not a query/dataflow unit at all.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::ConstantInstruction,
            ..*class_policy(PakVmNodeClass::SourceTraversal).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Scan, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_class_policy_filed_under_a_foreign_algebra_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            algebra: PakVmAlgebra::Effect,
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "node class policy names a different algebra");
    }

    #[test]
    fn every_authored_work_unit_is_accounted_by_some_family() {
        for unit in [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                     WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches,
                     WorkUnit::Outputs, WorkUnit::Artifacts, WorkUnit::Effects,
                     WorkUnit::CallDepth] {
            assert!(PAKVM_NODES.iter().any(|&n| match admit(n) {
                PakVmAdmission::Admitted(s) => s.work_formula.units().contains(&unit),
                PakVmAdmission::Refused(_) => false,
            }), "{unit:?} is accounted by no node family");
        }
    }

    #[test]
    fn the_authored_algebra_populations_match_docs_07() {
        let count = |a: PakVmAlgebra| PAKVM_NODES.iter().filter(|n| n.algebra() == a).count();
        assert_eq!(count(PakVmAlgebra::FormulaDecision), 16);
        assert_eq!(count(PakVmAlgebra::QueryDataflow), 14);
        assert_eq!(count(PakVmAlgebra::Effect), 6);
        assert_eq!(PAKVM_NODES.len(), 36);
    }

    // --- work-formula LINEAGE, not mere coverage -------------------------------
    // Every one of these mutations ADMITTED before the work-plane law existed.
    // "All ten units are claimed" was satisfied in each case: the wrong family
    // still claims its unit. Coverage proves no bucket is empty, never that
    // anything is in the right bucket.

    #[test]
    fn windowing_accounted_by_returned_output_is_refused() {
        // The exact defect LEG-028's page_limit_bounds_discovery_work_not_only_output
        // exists to reject: bounding the returned vector instead of discovery.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::EmittedOutputs,
            ..*class_policy(PakVmNodeClass::Windowing).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Page, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn an_effect_node_given_a_query_cost_law_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::Effect).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::Rows,
            ..*class_policy(PakVmNodeClass::DurableAppend).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn an_effect_node_given_a_pure_query_family_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::Effect).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::ConstantInstruction,
            ..*class_policy(PakVmNodeClass::DurableAppend).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Append, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_query_node_given_an_unrelated_but_real_unit_is_refused() {
        // Artifacts exist in the authored vocabulary — StageArtifact produces
        // them — so this mutation is plausible and coverage-clean.
        let ap = *algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::StagedArtifacts,
            ..*class_policy(PakVmNodeClass::RowTransform).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::Filter, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn a_formula_node_given_a_row_cost_law_is_refused() {
        let ap = *algebra_policy(PakVmAlgebra::FormulaDecision).unwrap();
        let cp = PakVmNodeClassPolicy {
            work_formula: WorkFormulaFamily::Rows,
            ..*class_policy(PakVmNodeClass::KernelInvocation).unwrap()
        };
        assert_eq!(refusal(admit_candidate_policy(PakVmNodeId::KernelCall, &ap, &cp)),
                   "work formula accounts in a unit outside its algebra's work plane");
    }

    #[test]
    fn the_three_work_planes_partition_the_authored_vocabulary() {
        // Each unit belongs to exactly one algebra. If a unit sat on two planes,
        // the plane rule would admit a family in the wrong one; if it sat on
        // none, no family could ever account in it.
        let all = [WorkUnit::Instructions, WorkUnit::Rows, WorkUnit::DecodedBytes,
                   WorkUnit::TileBytes, WorkUnit::Groups, WorkUnit::Matches,
                   WorkUnit::Outputs, WorkUnit::Artifacts, WorkUnit::Effects,
                   WorkUnit::CallDepth];
        let planes = [PakVmAlgebra::FormulaDecision, PakVmAlgebra::QueryDataflow,
                      PakVmAlgebra::Effect];
        for unit in all {
            let owners = planes.iter().filter(|a| a.work_plane().contains(&unit)).count();
            assert_eq!(owners, 1, "{unit:?} is claimed by {owners} work planes");
        }
        let total: usize = planes.iter().map(|a| a.work_plane().len()).sum();
        assert_eq!(total, all.len());
    }

    #[test]
    fn canonical_tables_cannot_produce_the_forbidden_state() {
        // The seam proves refusals fire; this proves the real specification never
        // needs them. Every canonical algebra policy proposes a public identity.
        for algebra in [PakVmAlgebra::FormulaDecision, PakVmAlgebra::QueryDataflow,
                        PakVmAlgebra::Effect] {
            let ap = algebra_policy(algebra).unwrap();
            assert!(matches!(ap.lowering,
                PakVmRule::AlgebraConstant(CandidateLoweringPosture::PublicSemanticIdentity)),
                "{algebra:?} proposes a non-public lowering identity");
        }
    }

    #[test]
    fn removing_the_seam_would_not_change_production_admission() {
        // admit() must agree with the seam fed the canonical tables. If it did
        // not, the fixtures would be proving something production never runs.
        for &node in PAKVM_NODES {
            let ap = algebra_policy(node.algebra()).unwrap();
            let cp = class_policy(node.class()).unwrap();
            assert_eq!(admit(node), admit_candidate_policy(node, ap, cp), "{node:?}");
        }
    }
}

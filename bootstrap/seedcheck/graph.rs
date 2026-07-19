use spec::{architecture, dispositions, gates, guarantees, invariants, legacy_invariant_coverage, legacy_obligations};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn check_version(findings: &mut Vec<String>) {
    // Reject any workspace version below the signed 1.0 implementation train so
    // a template cannot regress the family to a pre-1.0 line (e.g. 0.1.0).
    let version = architecture::WORKSPACE_VERSION;
    let major = version.split('.').next().and_then(|part| part.parse::<u64>().ok());
    match major {
        Some(major) if major >= 1 => {}
        _ => findings.push(format!("workspace version {version} is below the signed 1.0 train")),
    }
}

pub(crate) fn check_graph(findings: &mut Vec<String>) {
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

pub(crate) fn check_profiles(findings: &mut Vec<String>) {
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

pub(crate) fn check_unique_ids(findings: &mut Vec<String>) {
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


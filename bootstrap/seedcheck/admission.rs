use spec::{architecture, dispositions, gates, guarantees, identities, invariants, legacy_invariant_coverage, legacy_obligations};
use std::collections::BTreeSet;
use std::path::Path;
use crate::corpus::guarantee_ref_resolves;
use crate::proof::{authors_token, contract_authored_texts, declared_contract_ids};

/// The cross-family guarantee admission law, EXECUTED (5.5E2 bake). Until
/// this check existed, `spec::guarantees::admit` had Rust vocabulary and a
/// Python reconstruction but no Rust execution path: every native row across
/// all five families now passes through the real sealed admission, and the
/// refusals are exercised with hostile sources so the reachable-refusal
/// question has an executed answer, not a plausible one.
pub(crate) fn check_guarantee_admission(findings: &mut Vec<String>) {
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
pub(crate) fn check_identity_catalogs(root: &Path, findings: &mut Vec<String>) {
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
pub(crate) fn check_inventory_classes(findings: &mut Vec<String>) {
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
pub(crate) fn check_ledger_vocabulary(findings: &mut Vec<String>) {
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


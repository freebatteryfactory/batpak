use spec::{architecture, commands, dispositions, generated_views, gates, guarantees};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use crate::proof::{authors_token, contract_authored_texts, declared_contract_ids};

/// The generated-view registry, EXECUTED (5.5E4a). Seedcheck runs the real
/// `GeneratedView` values: inventory integrity, complete specs, surface/target
/// shape agreement, per-target marker uniqueness, generator identity, path
/// existence through the repository root, and the registry's own presence.
/// It parses no Markdown block bodies — that reconstruction is audit.py's.
pub(crate) fn check_generated_views(root: &Path, findings: &mut Vec<String>) {
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
pub(crate) fn check_commands(root: &Path, findings: &mut Vec<String>) {
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
pub(crate) fn check_promotion(root: &Path, findings: &mut Vec<String>) {
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


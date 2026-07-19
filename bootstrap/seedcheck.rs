#![deny(warnings)]

// The typed specification is a LIBRARY (spec/lib.rs, 5.5E2): this binary
// links it instead of textually mounting its modules, so no tracked
// suppression exists and every pub spec item is API by construction.

use spec::{architecture, dispositions, guarantees, invariants, legacy_invariant_coverage, legacy_obligations, operators};
use std::env;
use std::path::{Path, PathBuf};

#[path = "seedcheck/admission.rs"] mod admission;
#[path = "seedcheck/views.rs"] mod views;
#[path = "seedcheck/corpus.rs"] mod corpus;
#[path = "seedcheck/proof.rs"] mod proof;
#[path = "seedcheck/verification.rs"] mod verification;
#[path = "seedcheck/sprouting.rs"] mod sprouting;
#[path = "seedcheck/graph.rs"] mod graph;
#[path = "seedcheck/operators.rs"] mod operator_checks;
#[path = "seedcheck/qualification.rs"] mod qualification;
#[path = "seedcheck/tier0.rs"] mod tier0;
#[path = "seedcheck/tokens.rs"] mod tokens;
#[path = "seedcheck/isa.rs"] mod isa;
#[cfg(test)]
#[path = "seedcheck/tests.rs"]
mod tests;

use crate::admission::*;
use crate::views::*;
use crate::corpus::*;
use crate::proof::*;
use crate::verification::*;
use crate::sprouting::*;
use crate::graph::*;
use crate::operator_checks::*;
use crate::qualification::*;
use crate::tier0::*;
use crate::tokens::*;
use crate::isa::*;

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
    // The closed bootstrap-tool inventory is the factory's COMPLETE tool set:
    // every variant's path must exist, cited by a witness or not (Wave-2
    // BootstrapToolId ruling).
    for tool in guarantees::BootstrapToolId::ALL {
        if !root.join(tool.path()).is_file() {
            findings.push(format!("bootstrap tool {} does not exist", tool.path()));
        }
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
    check_verification(&mut findings);
    check_sprouting(&mut findings);
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

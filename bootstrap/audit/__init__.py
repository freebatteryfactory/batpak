"""Independent standard-library audit for the clean-room specification.

Package aggregation and CLI. The audit is decomposed into cohesive submodules
(corpus, batql, guarantees, architecture, proof, verification, ledgers,
catalogs, tier0); this package re-exports their full public surface, owns the
cross-cutting check aggregators, and provides the ``main`` entry point. The
thin shim bootstrap/audit.py loads this package and re-exports it.
"""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

from .architecture import (
    A_AH_FN_ARM,
    A_CLAIM_AXES,
    A_CLAIM_BUNDLE,
    A_CLAIM_LADDER_NAMES,
    A_FROZEN_BUNDLES,
    A_FROZEN_MATRIX,
    A_FROZEN_PROFILE_BUNDLES,
    A_FW_ARCH,
    A_FW_DOC,
    A_FW_SRC,
    A_ISA_DOC,
    A_ISA_SIGS,
    A_ISA_SRC,
    A_ISA_TABLE,
    A_RECON_DOC,
    A_RECON_ROW,
    A_RECON_SRC,
    A_REQUIRED_FAILURES,
    A_RETIRED_CLAIM_VOCAB,
    A_WITNESS_DISPOSITIONS,
    A_WITNESS_VARIANT,
    DOC_LAW_MARKERS,
    FirewallAdmissionFailure,
    IsaAdmissionFailure,
    ah_fn_arms,
    authenticated_history_findings,
    authenticated_history_rows,
    check_document_law,
    claim_axis_findings,
    claim_bundles,
    pakvm_isa_findings,
    pakvm_isa_views,
    proof_terminal_findings,
    recon_findings,
    recon_views,
    release_seal_findings,
    retired_claim_vocabulary_findings,
    syncbat_firewall_findings,
    syncbat_firewall_views,
)
from .batql import (
    A_OPERATOR_DOCTRINE,
    BATQL_CLASS_SHAPE,
    BATQL_OP_ROW,
    BATQL_OPERATOR_BLOCKS,
    BATQL_OPERATOR_DOC_DENIALS,
    BATQL_OPERATOR_DOC_DOCTRINE,
    BATQL_OPERATOR_PROVENANCE,
    BATQL_PROOF_DISPOSITIONS,
    BATQL_ROW_FIELDS,
    BATQL_SHAPE_DISPLAY,
    BATQL_SYMBOL_GRAMMAR,
    BATQL_WORD_GRAMMAR,
    NUMERIC_INTERVAL_OPERATORS,
    NUMERIC_LAW_MARKERS,
    NUMERIC_ROUNDING_MODES,
    NUMERIC_SHARED_SORTS,
    PHANTOM_SORTS,
    batql_grammar_closure_findings,
    batql_grammar_fences,
    batql_language_change_findings,
    batql_operator_authority_findings,
    batql_operator_corpus_findings,
    batql_operator_doc_findings,
    batql_operator_fact_findings,
    batql_operator_model_findings,
    batql_parse_operator_model,
    batql_proof_disposition_findings,
    batql_proof_disposition_states,
    batql_render_catalog,
    batql_render_grammar,
    batql_render_numeric,
    batql_render_projection,
    batql_render_surfaces,
    batql_render_typing,
    batql_resolve_operator_rows,
    check_batql,
    check_numeric,
    numeric_law_findings,
    phantom_sort_findings,
)
from .catalogs import (
    A_COMMAND_NAMESPACES,
    A_CONTRACT_KIND_HYBRID_FENCE,
    A_IDENT_DISPOSITION,
    A_IDENT_TERM,
    A_IDENTITY_AXES,
    A_LANE_ALIAS,
    A_PROMOTION_DISQUALIFIED,
    A_PROMOTION_DOCTRINE,
    command_catalog_findings,
    compiler_assumption_findings,
    contract_kind_findings,
    corpus_findings,
    identity_catalog_findings,
    mutation_findings,
    promotion_findings,
)
from .corpus import (
    A_DISPOSITION_VARIANT,
    A_E4D_ACTIVE,
    A_E4D_AUTOMATIC,
    A_E4D_LEG_FULL,
    A_E4D_PLANNED_TEXT,
    A_GENERATED_BLOCK,
    CONTROL_ALLOWED,
    CONTROL_TEXT_NAMES,
    CONTROL_TEXT_SUFFIXES,
    EXCLUDE_DIRS,
    EXCLUDE_SUFFIXES,
    G_DEC_ROW,
    G_LEG_ROW,
    G_QUAL_ROW,
    GENERATED_FRONT,
    PLACEHOLDERS,
    PRODUCTION_SOURCE_DIRS,
    REQUIRED_FRONT,
    RETIRING_DISPOSITIONS,
    STALE_CONTEXT_BY_PATH,
    STALE_DEFAULT_CONTEXTS,
    STALE_PERMISSIVE_CONTEXTS,
    STALE_PROJECTION_DOCS,
    STALE_REF_RE,
    STALE_VOCAB_BLOCK_RE,
    STATUSES,
    _bootstrap_rust_source,
    _spec_module_source,
    _uncomment,
    batql_extract_block,
    casefold_collisions,
    check_stale_vocabulary,
    control_character_findings,
    declared_contract_ids,
    frontmatter,
    frozen_files,
    legacy_manifest_findings,
    markdown_links,
    parse_stale_vocabulary,
    scan_stale_occurrences,
    stale_context_for,
    string_array,
)
from .guarantees import (
    A_DEC_CLASS_ROW,
    A_DEC_CLASSES,
    A_DEC_GATE_OPTIONAL,
    A_DEC_GATE_REQUIRED,
    A_FAM_ROW,
    A_GATE_ENUM,
    A_GATE_SPEC,
    A_GATE_VARIANT,
    D4B3B0_UNGATED_FAMILIES,
    G_ENVIRONMENT_SPELLING,
    G_FAMILY_RANK,
    G_PKG_ROW,
    G_REF,
    G_REL_CONSTRUCTOR,
    G_REL_TAG_PREFIX,
    G_SEED_ROW,
    G_WITNESS_TAGGED,
    G_WITNESS_TOOL,
    G_WITNESS_TOOL_NAMES,
    GATE_BLOCK_RE,
    GUARANTEE_FAMILIES,
    GUARANTEE_KINDS,
    GUARANTEE_LIFETIMES,
    PERMANENT_WITNESS_KINDS,
    AdmissionFailure,
    _md_section_rows,
    admit_view,
    decision_class_findings,
    decision_lifetime_map,
    decision_rows,
    g_package_projection,
    g_witness_render,
    gate_doc_findings,
    gate_enum_variants,
    gate_findings,
    gate_inventory,
    gate_inventory_findings,
    gate_list,
    gate_optional_decision_classes,
    gate_reference_findings,
    gate_render,
    guarantee_admission_findings,
    guarantee_classification_findings,
    guarantee_derive,
    guarantee_family_policies,
    guarantee_gates,
    guarantee_index,
    guarantee_leg_meta,
    guarantee_lifetime_findings,
    guarantee_relation_findings,
    guarantee_seed_rows,
    guarantee_typed_relation_findings,
    guarantee_typed_witness_findings,
)
from .ledgers import (
    A_AUTHORED_FENCES,
    A_COV_FULL_ROW,
    A_DEC_FULL_ROW,
    A_E4B_DOCTRINE,
    A_E4B_RETIRED,
    A_EDGE_ROW,
    A_GENERATED_VIEWS_SRC,
    A_MARKER_BEGIN,
    A_MARKER_END,
    A_MARKER_PROVENANCE,
    A_PKG_ROW_FULL,
    A_QUAL_FULL,
    A_TIER0_ROWS,
    A_VERSION_ALIAS_BANS,
    A_VERSION_DOCTRINE,
    A_VIEW_ARM,
    A_VIEW_SURFACES,
    a_parse_generated_views,
    exact_ledger_findings,
    generated_view_findings,
    inventory_mirror_findings,
    proof_relation_findings,
)
from .proof import (
    A_PROOF_RECORD,
    D1_LEG_CLAUSES,
    D2_FORBIDDEN_PUBLIC_SYNTAX,
    D2_KEY_COMPONENTS,
    D3_BANNED_RESULT_WORDS,
    D3_CHANGE_CLASSES,
    D3_FORBIDDEN_CANDIDATE_ROOTS,
    D3_LANES,
    D3_NEVER_KILLED,
    D3_POLICY_SURFACES,
    D3_RESULTS,
    D4B2A_LEG019_CLAUSES,
    D4B2A_ROWS,
    D4B2B_DEFERRED,
    D4B2B_MEANING_CLAUSES,
    D4B2B_ROWS,
    D4B2B_VAGUE,
    D4B2C_COVERAGE,
    D4B2C_D21,
    D4B2C_D24,
    D4B2C_D35,
    D4B2C_D35_PROJECTION,
    D4B2C_LEG,
    D4B2C_MATRIX,
    D4B2C_MEANING_CLAUSES,
    D4B2C_MIGRATION_NOTE,
    D4B2C_ROWS,
    D4B3B0_CAND_ID,
    D4B3B0_FENCE,
    D4B3B0_PENDING_EXPECTATION_CEILING,
    D4B3B0_VAGUE,
    D4B_LEG023_CLAUSES,
    D4B_LEG023_WITNESSES,
    W_ANY_TARGET_BLOCK,
    W_CLAUSE,
    W_ID,
    W_INDENT,
    W_MEANING_BLOCK,
    W_OWNER_BLOCK,
    candidate_fences,
    candidate_summary,
    coverage_matrix,
    d1_substrate_findings,
    deferred_posture_findings,
    derived_material_findings,
    docs21_witness_refs,
    integrity_witness_findings,
    leg081_authority_findings,
    leg_gates,
    migration_note_retired_ids,
    proof_meaning_findings,
    proof_policy_findings,
    proof_row_catalog,
    proof_row_catalog_findings,
    proof_target_findings,
    proof_target_grammar_findings,
    retired_proof_row_findings,
    retired_proof_rows,
    specialization_findings,
    specialization_key_findings,
    witness_expectations,
    witness_meanings,
    witness_reference_findings,
    witness_rows,
)
from .tier0 import (
    A_BO_ENUMS,
    A_BO_SPEC,
    A_BQ_SPEC,
    A_TC_COMPONENT_SPELLING,
    A_TC_PROFILE_SPELLING,
    A_TRIPLE_SHAPE,
    A_XR_SPEC,
    bootstrap_output_findings,
    bootstrap_qualification_findings,
    bootstrap_topology_findings,
    python_tooling_findings,
    tier0_cross_run_findings,
    toolchain_findings,
    toolchain_profile,
    workflow_pinning_findings,
)
from .verification import (
    A_SPROUTING_AXES,
    A_SPROUTING_BUDGET_FIELDS,
    A_SPROUTING_FORBIDDEN,
    A_SPROUTING_PROMOTION_REQUIREMENTS,
    A_VERIFICATION_AXES,
    A_VERIFICATION_PLAN,
    A_VERIFICATION_REQ,
    A_VERIFICATION_TOOL_NAMES,
    sprouting_findings,
    verification_findings,
)

__all__ = [
    "A_AH_FN_ARM",
    "A_AUTHORED_FENCES",
    "A_BO_ENUMS",
    "A_BO_SPEC",
    "A_BQ_SPEC",
    "A_CLAIM_AXES",
    "A_CLAIM_BUNDLE",
    "A_CLAIM_LADDER_NAMES",
    "A_COMMAND_NAMESPACES",
    "A_CONTRACT_KIND_HYBRID_FENCE",
    "A_COV_FULL_ROW",
    "A_DEC_CLASSES",
    "A_DEC_CLASS_ROW",
    "A_DEC_FULL_ROW",
    "A_DEC_GATE_OPTIONAL",
    "A_DEC_GATE_REQUIRED",
    "A_DISPOSITION_VARIANT",
    "A_E4B_DOCTRINE",
    "A_E4B_RETIRED",
    "A_E4D_ACTIVE",
    "A_E4D_AUTOMATIC",
    "A_E4D_LEG_FULL",
    "A_E4D_PLANNED_TEXT",
    "A_EDGE_ROW",
    "A_FAM_ROW",
    "A_FROZEN_BUNDLES",
    "A_FROZEN_MATRIX",
    "A_FROZEN_PROFILE_BUNDLES",
    "A_FW_ARCH",
    "A_FW_DOC",
    "A_FW_SRC",
    "A_GATE_ENUM",
    "A_GATE_SPEC",
    "A_GATE_VARIANT",
    "A_GENERATED_BLOCK",
    "A_GENERATED_VIEWS_SRC",
    "A_IDENTITY_AXES",
    "A_IDENT_DISPOSITION",
    "A_IDENT_TERM",
    "A_ISA_DOC",
    "A_ISA_SIGS",
    "A_ISA_SRC",
    "A_ISA_TABLE",
    "A_LANE_ALIAS",
    "A_MARKER_BEGIN",
    "A_MARKER_END",
    "A_MARKER_PROVENANCE",
    "A_OPERATOR_DOCTRINE",
    "A_PKG_ROW_FULL",
    "A_PROMOTION_DISQUALIFIED",
    "A_PROMOTION_DOCTRINE",
    "A_PROOF_RECORD",
    "A_QUAL_FULL",
    "A_RECON_DOC",
    "A_RECON_ROW",
    "A_RECON_SRC",
    "A_REQUIRED_FAILURES",
    "A_RETIRED_CLAIM_VOCAB",
    "A_SPROUTING_AXES",
    "A_SPROUTING_BUDGET_FIELDS",
    "A_SPROUTING_FORBIDDEN",
    "A_SPROUTING_PROMOTION_REQUIREMENTS",
    "A_TC_COMPONENT_SPELLING",
    "A_TC_PROFILE_SPELLING",
    "A_TIER0_ROWS",
    "A_TRIPLE_SHAPE",
    "A_VERIFICATION_AXES",
    "A_VERIFICATION_PLAN",
    "A_VERIFICATION_REQ",
    "A_VERIFICATION_TOOL_NAMES",
    "A_VERSION_ALIAS_BANS",
    "A_VERSION_DOCTRINE",
    "A_VIEW_ARM",
    "A_VIEW_SURFACES",
    "A_WITNESS_DISPOSITIONS",
    "A_WITNESS_VARIANT",
    "A_XR_SPEC",
    "BATQL_CLASS_SHAPE",
    "BATQL_OPERATOR_BLOCKS",
    "BATQL_OPERATOR_DOC_DENIALS",
    "BATQL_OPERATOR_DOC_DOCTRINE",
    "BATQL_OPERATOR_PROVENANCE",
    "BATQL_OP_ROW",
    "BATQL_PROOF_DISPOSITIONS",
    "BATQL_ROW_FIELDS",
    "BATQL_SHAPE_DISPLAY",
    "BATQL_SYMBOL_GRAMMAR",
    "BATQL_WORD_GRAMMAR",
    "CONTROL_ALLOWED",
    "CONTROL_TEXT_NAMES",
    "CONTROL_TEXT_SUFFIXES",
    "D1_LEG_CLAUSES",
    "D2_FORBIDDEN_PUBLIC_SYNTAX",
    "D2_KEY_COMPONENTS",
    "D3_BANNED_RESULT_WORDS",
    "D3_CHANGE_CLASSES",
    "D3_FORBIDDEN_CANDIDATE_ROOTS",
    "D3_LANES",
    "D3_NEVER_KILLED",
    "D3_POLICY_SURFACES",
    "D3_RESULTS",
    "D4B2A_LEG019_CLAUSES",
    "D4B2A_ROWS",
    "D4B2B_DEFERRED",
    "D4B2B_MEANING_CLAUSES",
    "D4B2B_ROWS",
    "D4B2B_VAGUE",
    "D4B2C_COVERAGE",
    "D4B2C_D21",
    "D4B2C_D24",
    "D4B2C_D35",
    "D4B2C_D35_PROJECTION",
    "D4B2C_LEG",
    "D4B2C_MATRIX",
    "D4B2C_MEANING_CLAUSES",
    "D4B2C_MIGRATION_NOTE",
    "D4B2C_ROWS",
    "D4B3B0_CAND_ID",
    "D4B3B0_FENCE",
    "D4B3B0_PENDING_EXPECTATION_CEILING",
    "D4B3B0_UNGATED_FAMILIES",
    "D4B3B0_VAGUE",
    "D4B_LEG023_CLAUSES",
    "D4B_LEG023_WITNESSES",
    "DOC_LAW_MARKERS",
    "EXCLUDE_DIRS",
    "EXCLUDE_SUFFIXES",
    "GATE_BLOCK_RE",
    "GENERATED_FRONT",
    "GUARANTEE_FAMILIES",
    "GUARANTEE_KINDS",
    "GUARANTEE_LIFETIMES",
    "G_DEC_ROW",
    "G_ENVIRONMENT_SPELLING",
    "G_FAMILY_RANK",
    "G_LEG_ROW",
    "G_PKG_ROW",
    "G_QUAL_ROW",
    "G_REF",
    "G_REL_CONSTRUCTOR",
    "G_REL_TAG_PREFIX",
    "G_SEED_ROW",
    "G_WITNESS_TAGGED",
    "G_WITNESS_TOOL",
    "G_WITNESS_TOOL_NAMES",
    "NUMERIC_INTERVAL_OPERATORS",
    "NUMERIC_LAW_MARKERS",
    "NUMERIC_ROUNDING_MODES",
    "NUMERIC_SHARED_SORTS",
    "PERMANENT_WITNESS_KINDS",
    "PHANTOM_SORTS",
    "PLACEHOLDERS",
    "PRODUCTION_SOURCE_DIRS",
    "REQUIRED_FRONT",
    "RETIRING_DISPOSITIONS",
    "STALE_CONTEXT_BY_PATH",
    "STALE_DEFAULT_CONTEXTS",
    "STALE_PERMISSIVE_CONTEXTS",
    "STALE_PROJECTION_DOCS",
    "STALE_REF_RE",
    "STALE_VOCAB_BLOCK_RE",
    "STATUSES",
    "W_ANY_TARGET_BLOCK",
    "W_CLAUSE",
    "W_ID",
    "W_INDENT",
    "W_MEANING_BLOCK",
    "W_OWNER_BLOCK",
    "AdmissionFailure",
    "FirewallAdmissionFailure",
    "IsaAdmissionFailure",
    "a_parse_generated_views",
    "admit_view",
    "ah_fn_arms",
    "authenticated_history_findings",
    "authenticated_history_rows",
    "batql_extract_block",
    "batql_grammar_closure_findings",
    "batql_grammar_fences",
    "batql_language_change_findings",
    "batql_operator_authority_findings",
    "batql_operator_corpus_findings",
    "batql_operator_doc_findings",
    "batql_operator_fact_findings",
    "batql_operator_model_findings",
    "batql_parse_operator_model",
    "batql_proof_disposition_findings",
    "batql_proof_disposition_states",
    "batql_render_catalog",
    "batql_render_grammar",
    "batql_render_numeric",
    "batql_render_projection",
    "batql_render_surfaces",
    "batql_render_typing",
    "batql_resolve_operator_rows",
    "bootstrap_output_findings",
    "bootstrap_qualification_findings",
    "bootstrap_topology_findings",
    "candidate_fences",
    "candidate_summary",
    "casefold_collisions",
    "check_architecture",
    "check_batql",
    "check_document_law",
    "check_guarantees",
    "check_numeric",
    "check_seed_ids",
    "check_stale_vocabulary",
    "claim_axis_findings",
    "claim_bundles",
    "command_catalog_findings",
    "compiler_assumption_findings",
    "contract_kind_findings",
    "control_character_findings",
    "corpus_findings",
    "coverage_matrix",
    "d1_substrate_findings",
    "decision_class_findings",
    "decision_lifetime_map",
    "decision_rows",
    "declared_contract_ids",
    "deferred_posture_findings",
    "derived_material_findings",
    "docs21_witness_refs",
    "exact_ledger_findings",
    "frontmatter",
    "frozen_files",
    "g_package_projection",
    "g_witness_render",
    "gate_doc_findings",
    "gate_enum_variants",
    "gate_findings",
    "gate_inventory",
    "gate_inventory_findings",
    "gate_list",
    "gate_optional_decision_classes",
    "gate_reference_findings",
    "gate_render",
    "generated_view_findings",
    "guarantee_admission_findings",
    "guarantee_classification_findings",
    "guarantee_derive",
    "guarantee_family_policies",
    "guarantee_gates",
    "guarantee_index",
    "guarantee_leg_meta",
    "guarantee_lifetime_findings",
    "guarantee_relation_findings",
    "guarantee_seed_rows",
    "guarantee_typed_relation_findings",
    "guarantee_typed_witness_findings",
    "identity_catalog_findings",
    "integrity_witness_findings",
    "inventory_mirror_findings",
    "leg081_authority_findings",
    "leg_gates",
    "legacy_manifest_findings",
    "main",
    "markdown_links",
    "migration_note_retired_ids",
    "mutation_findings",
    "numeric_law_findings",
    "pakvm_isa_findings",
    "pakvm_isa_views",
    "parse_stale_vocabulary",
    "phantom_sort_findings",
    "promotion_findings",
    "proof_meaning_findings",
    "proof_policy_findings",
    "proof_relation_findings",
    "proof_row_catalog",
    "proof_row_catalog_findings",
    "proof_target_findings",
    "proof_target_grammar_findings",
    "proof_terminal_findings",
    "python_tooling_findings",
    "recon_findings",
    "recon_views",
    "release_seal_findings",
    "retired_claim_vocabulary_findings",
    "retired_proof_row_findings",
    "retired_proof_rows",
    "scan_stale_occurrences",
    "specialization_findings",
    "specialization_key_findings",
    "sprouting_findings",
    "stale_context_for",
    "string_array",
    "syncbat_firewall_findings",
    "syncbat_firewall_views",
    "tier0_cross_run_findings",
    "toolchain_findings",
    "toolchain_profile",
    "verification_findings",
    "witness_expectations",
    "witness_meanings",
    "witness_reference_findings",
    "witness_rows",
    "workflow_pinning_findings",
]


def check_guarantees(root: Path, findings: list[str]) -> None:
    if not (root / "spec/guarantees/mod.rs").is_file():
        findings.append("missing spec/guarantees/mod.rs")
        return
    seed_rows = guarantee_seed_rows(root)
    findings.extend(guarantee_typed_relation_findings(seed_rows))
    findings.extend(guarantee_typed_witness_findings(root, seed_rows))
    nodes, edges, admission = guarantee_derive(root)
    findings.extend(admission)
    leg_meta = guarantee_leg_meta(root)
    node_ids = {n["id"] for n in nodes}
    # A witness's guarantee citation resolves against the derived node set:
    # the type carries the family, this law carries existence on the Python
    # side (seedcheck executes the same law through the sealed accessors).
    for r in seed_rows:
        for tag, ident in G_WITNESS_TAGGED.findall(r.get("witness_raw", "")):
            if tag in ("leg", "dec") and ident not in node_ids:
                findings.append(
                    f"{r['id']} witness references {ident}, which no declared row owns")
    findings.extend(gate_inventory_findings(root))
    findings.extend(gate_doc_findings(root))
    findings.extend(gate_reference_findings(root))
    findings.extend(decision_class_findings(root))
    findings.extend(authenticated_history_findings(root))
    findings.extend(retired_claim_vocabulary_findings(root))
    findings.extend(retired_proof_row_findings(root))
    findings.extend(proof_row_catalog_findings(root))
    findings.extend(contract_kind_findings(root))
    findings.extend(generated_view_findings(root))
    findings.extend(inventory_mirror_findings(root))
    findings.extend(exact_ledger_findings(root))
    findings.extend(proof_relation_findings(root))
    findings.extend(verification_findings(root))
    findings.extend(sprouting_findings(root))
    findings.extend(identity_catalog_findings(root))
    findings.extend(command_catalog_findings(root))
    findings.extend(mutation_findings(root))
    findings.extend(corpus_findings(root))
    findings.extend(compiler_assumption_findings(root))
    findings.extend(promotion_findings(root))
    findings.extend(pakvm_isa_findings(root))
    findings.extend(recon_findings(root))
    findings.extend(release_seal_findings(root))
    findings.extend(proof_terminal_findings(root))
    findings.extend(syncbat_firewall_findings(root))
    findings.extend(bootstrap_output_findings(root))
    findings.extend(bootstrap_qualification_findings(root))
    findings.extend(tier0_cross_run_findings(root))
    findings.extend(workflow_pinning_findings(root))
    findings.extend(python_tooling_findings(root))
    findings.extend(bootstrap_topology_findings(root))
    findings.extend(guarantee_classification_findings(seed_rows))
    findings.extend(guarantee_relation_findings(node_ids, edges))
    findings.extend(guarantee_lifetime_findings(nodes, leg_meta, edges))
    if len(node_ids) != len(nodes):
        findings.append("duplicate guarantee id across source families")
    # duplicate-prose structural smell (SEED only; exact normalized text + owner, no relation)
    seed_src = (root / "spec/invariants/inventory.rs").read_text(encoding="utf-8")
    stmts = re.findall(r'id: "([^"]+)", statement: "((?:[^"\\]|\\.)*)"', seed_src)
    seen_text: dict[tuple[str, str], str] = {}
    rel_by_id = {r["id"]: r["rel"] for r in seed_rows}
    for sid, stmt in stmts:
        owner = next((r["owner"] for r in seed_rows if r["id"] == sid), "")
        key = (re.sub(r"\s+", " ", stmt).strip(), owner)
        if key in seen_text:
            other = seen_text[key]
            rels = rel_by_id.get(sid, {})
            if not any(rels.values()):
                findings.append(f"guarantee duplicate-prose smell: {sid} and {other} share text and owner without a relation")
        seen_text[key] = sid
    # comparison: generated SEED block and Guarantee Graph must equal derived facts
    seed_doc = (root / "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")
    seed_block = batql_extract_block(seed_doc.read_text(encoding="utf-8"), "SEED-CLASSIFICATION") if seed_doc.is_file() else None
    if seed_block is None:
        findings.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: missing SEED-CLASSIFICATION block")
    else:
        block_rows = [
            tuple(c.strip() for c in ln.split("|")[1:-1])[:6]
            for ln in seed_block.splitlines() if ln.strip().startswith("| SEED-")
        ]
        derived_rows = [(r["id"], r["kind"], r["lifetime"], r["owner"],
                         gate_render(r["gate_names"], root), r["witness"]) for r in seed_rows]
        if block_rows != derived_rows:
            findings.append("docs/23 SEED-CLASSIFICATION block does not equal spec/invariants/inventory.rs")
    graph_path = root / "docs/GUARANTEE_GRAPH.generated.md"
    if not graph_path.is_file():
        findings.append("missing docs/GUARANTEE_GRAPH.generated.md")
        return
    graph = graph_path.read_text(encoding="utf-8")
    node_rows = _md_section_rows(graph, "Nodes")
    # Every row must be full width. A width change that silently drops rows would
    # make this comparison pass against an empty set.
    malformed = [r for r in node_rows if len(r) != 9]
    if malformed or not node_rows:
        findings.append(
            f"Guarantee Graph node table has {len(malformed)} malformed row(s) of {len(node_rows)}"
        )
    file_nodes = {tuple(r[:9]) for r in node_rows if len(r) == 9}
    derived_nodes = {
        (n["id"], n["family"], n["kind"], n["lifetime"], n.get("dclass", "-"), n["owner"],
         n["gate_posture"], n["target"] or "-", n["witness"])
        for n in nodes
    }
    if len(file_nodes) != len(nodes):
        findings.append(
            f"Guarantee Graph node table has {len(file_nodes)} rows, derived {len(nodes)}"
        )
    if file_nodes != derived_nodes:
        findings.append("Guarantee Graph node table does not equal the derived source facts")
    file_edges = {tuple(r) for r in _md_section_rows(graph, "Edges") if len(r) == 3}
    if file_edges != {(s, k, t) for s, k, t in edges}:
        findings.append("Guarantee Graph edge table does not equal the derived relations")


def check_architecture(root: Path, findings: list[str]) -> None:
    path = root / "spec/architecture/mod.rs"
    if not path.is_file():
        findings.append("missing spec/architecture/mod.rs")
        return
    findings.extend(toolchain_findings(root))
    source = _spec_module_source(root, "architecture")
    # The variant is the identity (5.5E3b); cargo names and workspace paths
    # are projections this auditor reconstructs from the owning const fns.
    cargo = g_package_projection(root, "cargo_name")
    wpaths = g_package_projection(root, "workspace_path")
    enum_body = re.search(r"pub enum PackageId \{(.*?)\n\}", _uncomment(source), re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];", source, re.S)
    inventory = re.findall(r"PackageId::(\w+)", all_body.group(1)) if all_body else []
    for missing in sorted(set(variants) - set(inventory)):
        findings.append(f"spec/architecture/types.rs: PackageId::{missing} is omitted "
                        "from PackageId::ALL, the one package inventory")
    for phantom in sorted(set(inventory) - set(variants)):
        findings.append(f"spec/architecture/types.rs: PackageId::ALL names {phantom}, "
                        "which the enum does not declare")
    package_rows = re.findall(
        r"PackageSpec\s*\{\s*id:\s*PackageId::(\w+),\s*role:\s*\"([^\"]+)\",\s*class:\s*PackageClass::([A-Za-z]+),\s*layer:\s*([0-9]+),\s*\}",
        source,
        re.S,
    )
    if not package_rows:
        findings.append("spec/architecture/inventory.rs: no package rows parsed")
        return
    if [row[0] for row in package_rows] != inventory:
        findings.append("spec/architecture/inventory.rs: PACKAGES does not declare exactly "
                        "PackageId::ALL in canonical order")
    packages = [cargo.get(row[0], "") for row in package_rows]
    paths = [wpaths.get(row[0], "") for row in package_rows]
    layers = {cargo.get(row[0], ""): int(row[3]) for row in package_rows}
    if len(packages) != len(set(packages)):
        findings.append("spec/architecture/types.rs: two package identities project the same cargo name")
    if len(paths) != len(set(paths)):
        findings.append("spec/architecture/types.rs: two package identities project the same workspace path")
    for variant, role, package_class, layer in package_rows:
        wp = wpaths.get(variant, "")
        if (not cargo.get(variant) or not wp or wp.startswith("/") or ".." in wp.split("/")
                or not role or package_class not in {"Production", "BinaryAdapter", "DevOnly", "Example"}
                or not layer.isdigit()):
            findings.append(f"spec/architecture/inventory.rs: incomplete package {variant}")

    edge_rows = re.findall(
        r"EdgeSpec\s*\{\s*importer:\s*PackageId::(\w+),\s*importee:\s*PackageId::(\w+),\s*class:\s*EdgeClass::([A-Za-z]+),\s*profile:\s*\"([^\"]+)\"\s*\}",
        source,
        re.S,
    )
    edge_rows = [(cargo.get(a, a), cargo.get(b, b), c, p) for a, b, c, p in edge_rows]
    graph: dict[str, list[str]] = {package: [] for package in packages}
    package_set = set(packages)
    for importer, importee, edge_class, profile in edge_rows:
        if importer not in package_set or importee not in package_set:
            findings.append(f"spec/architecture/inventory.rs: unknown package in edge {importer} -> {importee}")
            continue
        if importer == importee:
            findings.append(f"spec/architecture/inventory.rs: self edge {importer}")
        if edge_class not in {"Required", "OptionalProfile", "DevOnly"} or not profile:
            findings.append(f"spec/architecture/inventory.rs: incomplete edge {importer} -> {importee}")
        if importer in layers and importee in layers and layers[importer] <= layers[importee]:
            findings.append(f"spec/architecture/inventory.rs: dependency direction violation {importer}(L{layers[importer]}) -> {importee}(L{layers[importee]})")
        graph[importer].append(importee)

    visiting: set[str] = set()
    visited: set[str] = set()

    def cycle(node: str) -> bool:
        if node in visited:
            return False
        if node in visiting:
            return True
        visiting.add(node)
        for child in graph.get(node, []):
            if cycle(child):
                return True
        visiting.remove(node)
        visited.add(node)
        return False

    for package in packages:
        visiting.clear()
        if cycle(package):
            findings.append(f"spec/architecture/inventory.rs: dependency cycle reaches {package}")
            break

    profile_rows = re.findall(
        r"QualificationProfile\s*\{\s*package:\s*PackageId::(\w+),\s*profile:\s*\"([^\"]+)\","
        r"\s*environment:\s*QualificationEnvironment::(\w+),"
        r"\s*gates:\s*&\[([^\]]*)\],\s*requirement:\s*\"([^\"]+)\",\s*\}",
        source,
        re.S,
    )
    profile_rows = [(cargo.get(v, v), *rest) for v, *rest in profile_rows]
    identities: set[tuple[str, str]] = set()
    for package, profile, environment, gates, requirement in profile_rows:
        identity = (package, profile)
        if package not in package_set or not profile or not environment or not requirement or not gates:
            findings.append(f"spec/architecture/inventory.rs: incomplete qualification {package}:{profile}")
        if identity in identities:
            findings.append(f"spec/architecture/inventory.rs: duplicate qualification {package}:{profile}")
        identities.add(identity)
    for package in ("batpak", "syncbat"):
        if (package, "semantic") not in identities or not any(
            row[0] == package and row[1] == "semantic" and row[2] == "NoStdAlloc" for row in profile_rows
        ):
            findings.append(f"spec/architecture/inventory.rs: missing no_std + alloc semantic profile for {package}")

    # The materializer selects behavior by PackageId, never by raw name: a
    # cargo-name string comparison is identity laundering (5.5E3b).
    mat = _bootstrap_rust_source(root, "materialize")
    if mat:
        for name in sorted(set(cargo.values())):
            if re.search(r'== "' + re.escape(name) + '"', mat):
                findings.append(
                    f"bootstrap/materialize.rs selects behavior by raw package "
                    f"name {name!r}; PackageId is the identity")

    for relative in string_array(source, "REQUIRED_DOCS"):
        if not (root / relative).is_file():
            findings.append(f"spec/architecture/inventory.rs: required file missing {relative}")
    for relative in string_array(source, "FORBIDDEN_TARGET_PATHS"):
        if (root / relative).exists():
            findings.append(f"spec/architecture/inventory.rs: forbidden path exists {relative}")
    syncbat_root = root / "crates/syncbat"
    if syncbat_root.exists():
        # Plane files derive from the typed inventory (5.5E5); the raw
        # SYNCBAT_REQUIRED_PLANES path table retired with it.
        arm = re.search(r"pub const fn module_name\(self\)[^{]*\{(.*?)\n    \}",
                        source, re.S)
        for module in re.findall(r'=>\s*"([^"]+)"', arm.group(1)) if arm else []:
            for relative in (f"src/{module}/mod.rs", f"src/{module}/types.rs"):
                if not (syncbat_root / relative).exists():
                    findings.append(f"syncbat source missing declared plane {relative}")


def check_seed_ids(root: Path, findings: list[str]) -> None:
    sources = {
        "invariant": (root / "spec/invariants/inventory.rs", r'id:\s*"(SEED-[A-Z0-9-]+)"'),
        "decision": (root / "spec/dispositions/inventory.rs", r'id:\s*"(DEC-[0-9]+)"'),
        "legacy": (root / "spec/legacy_obligations/inventory.rs", r'id:\s*"(LEG-[0-9]+)"'),
        "legacy_coverage": (root / "spec/legacy_invariant_coverage/inventory.rs", r'legacy_id:\s*"(INV-[A-Z0-9-]+)"'),
    }
    parsed: dict[str, list[str]] = {}
    for kind, (path, pattern) in sources.items():
        if not path.is_file():
            findings.append(f"missing {path.relative_to(root)}")
            continue
        values = re.findall(pattern, path.read_text(encoding="utf-8"))
        if len(values) != len(set(values)):
            findings.append(f"duplicate {kind} ID in {path.relative_to(root)}")
        parsed[kind] = values

    decision_doc = (root / "docs/30_DECISION_AND_REJECTION_LEDGER.md").read_text(encoding="utf-8")
    decision_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in decision_doc.splitlines():
        if not line.startswith("| DEC-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 6:
            findings.append(f"decision ledger malformed row: {line}")
            continue
        ident, tag, dclass, dgates, subject, successor = cells
        def clean(value):
            return re.sub(r"`([^`]*)`", r"\1", value)
        decision_doc_rows[ident] = (tag, clean(subject), clean(successor), dclass, dgates)
    disposition_tags = {
        "Keep": "KEEP",
        "Lock": "LOCK",
        "Kill": "KILL",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Defer": "DEFER",
        "OpenImplementation": "OPEN-IMPLEMENTATION",
        "RetainAsEvidence": "RETAIN-AS-EVIDENCE",
    }
    decision_source = (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")
    decision_seed_rows = {
        ident: (disposition_tags[variant], subject, successor, dclass,
                gate_render(gate_list(dgates), root))
        for ident, dclass, dgates, variant, subject, successor in re.findall(
            r'DecisionSpec \{ id: "([^"]+)", class: DecisionClass::(\w+), gates: &\[([^\]]*)\], '
            r'disposition: Disposition::([A-Za-z]+), subject: "([^"]+)", successor: "([^"]+)"',
            decision_source,
        )
    }
    for value in parsed.get("decision", []):
        if value not in decision_doc_rows:
            findings.append(f"decision seed {value} absent from docs/30_DECISION_AND_REJECTION_LEDGER.md")
        elif decision_seed_rows.get(value) != decision_doc_rows[value]:
            findings.append(f"decision seed/document mismatch for {value}")
    legacy_doc = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    legacy_doc_rows: dict[str, tuple[str, str, str, str, str, str]] = {}
    for line in legacy_doc.splitlines():
        if not line.startswith("| LEG-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 10:
            findings.append(f"legacy ledger malformed row: {line}")
            continue
        ident, law, _evidence, owner, _mechanism, _witness, gate, compat, deletion, status = cells
        def clean(value):
            return re.sub(r"`([^`]*)`", r"\1", value)
        legacy_doc_rows[ident] = (
            clean(law), clean(owner), clean(gate), clean(compat), clean(deletion), clean(status)
        )
    legacy_source = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    legacy_seed_rows = {
        ident: (law, owner, gate_render(gate_list(gates), root), compat, deletion, status)
        for ident, law, owner, gates, compat, deletion, status in re.findall(
            r'LegacyObligation \{ id: "([^"]+)", law: "([^"]+)", '
            r'legacy_evidence: "(?:[^"\\]|\\.)*", clean_owner: "([^"]+)", '
            r'mechanism_disposition: "(?:[^"\\]|\\.)*", witness_requirement: '
            r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
            r'gates: &\[([^\]]*)\], '
            r"compatibility_disposition: CompatibilityDisposition::(\w+), "
            r"deletion_condition: DeletionCondition::(\w+), "
            r"active_or_closed_status: ObligationStatus::(\w+) \}",
            legacy_source,
        )
    }
    for value in parsed.get("legacy", []):
        if value not in legacy_doc_rows:
            findings.append(f"legacy seed {value} absent from docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md")
        elif legacy_seed_rows.get(value) != legacy_doc_rows[value]:
            findings.append(f"legacy seed/document mismatch for {value}")

    coverage_doc = (root / "docs/34_LEGACY_INVARIANT_COVERAGE.md").read_text(encoding="utf-8")
    coverage_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in coverage_doc.splitlines():
        if not line.startswith("| `INV-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 4:
            findings.append(f"legacy invariant coverage malformed row: {line}")
            continue
        ident, tag, successor, rationale = cells
        def clean(value):
            return re.sub(r"`([^`]*)`", r"\1", value)
        coverage_doc_rows[clean(ident)] = (tag, clean(successor), rationale)
    coverage_tags = {
        "Preserve": "PRESERVE",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Kill": "KILL",
        "Requalify": "REQUALIFY",
    }
    coverage_source = (root / "spec/legacy_invariant_coverage/inventory.rs").read_text(encoding="utf-8")
    coverage_seed_rows = {
        ident: (coverage_tags[variant], successor, rationale)
        for ident, variant, successor, rationale in re.findall(
            r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", disposition: CoverageDisposition::([A-Za-z]+), successor: "([^"]+)", rationale: "([^"]+)" \}',
            coverage_source,
        )
    }
    expected_count_match = re.search(r"EXPECTED_COVERAGE_ROWS:\s*usize\s*=\s*([0-9]+)", coverage_source)
    expected_count = int(expected_count_match.group(1)) if expected_count_match else -1
    if expected_count != len(parsed.get("legacy_coverage", [])):
        findings.append(f"legacy invariant coverage count is {len(parsed.get('legacy_coverage', []))}, expected {expected_count}")
    source_commit = re.search(r'LEGACY_SOURCE_COMMIT:\s*&str\s*=\s*"([0-9a-f]+)"', coverage_source)
    if source_commit is None or len(source_commit.group(1)) != 40:
        findings.append("legacy invariant coverage source commit is missing or not a full SHA")
    manifest_ids = string_array(coverage_source, "SOURCE_INVARIANT_IDS")
    if not manifest_ids:
        findings.append("spec/legacy_invariant_coverage/inventory.rs: missing SOURCE_INVARIANT_IDS manifest")
    else:
        findings.extend(
            legacy_manifest_findings(manifest_ids, parsed.get("legacy_coverage", []), expected_count)
        )
    for value in parsed.get("legacy_coverage", []):
        if value not in coverage_doc_rows:
            findings.append(f"legacy invariant coverage seed {value} absent from docs/34_LEGACY_INVARIANT_COVERAGE.md")
        elif coverage_seed_rows.get(value) != coverage_doc_rows[value]:
            findings.append(f"legacy invariant coverage seed/document mismatch for {value}")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    findings: list[str] = []
    markdown = sorted(root.rglob("*.md"))
    contract_ids: dict[str, Path] = {}
    for path in markdown:
        relative_path = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in relative_path.parts):
            continue
        rel = relative_path.as_posix()
        text = path.read_text(encoding="utf-8")
        meta = frontmatter(text)
        required = GENERATED_FRONT if meta.get("status") == "GENERATED" else REQUIRED_FRONT
        missing = required - meta.keys()
        if missing:
            findings.append(f"{rel}: missing frontmatter keys {sorted(missing)}")
        elif meta["status"] not in STATUSES:
            findings.append(f"{rel}: unknown status {meta['status']!r}")
        cid = meta.get("contract_id")
        if cid:
            if cid in contract_ids:
                findings.append(f"duplicate contract_id {cid}: {rel} and {contract_ids[cid]}")
            contract_ids[cid] = path
        if meta.get("status") == "AUTHORITATIVE":
            upper = text.upper()
            for token in PLACEHOLDERS:
                if token in upper:
                    findings.append(f"{rel}: normative placeholder {token}")
        for target in markdown_links(text):
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root)
            except ValueError:
                findings.append(f"{rel}: link escapes bundle: {target}")
                continue
            if not resolved.exists():
                findings.append(f"{rel}: broken relative link: {target}")

    check_architecture(root, findings)
    check_seed_ids(root, findings)
    check_stale_vocabulary(root, findings)
    check_batql(root, findings)
    check_numeric(root, findings)
    check_guarantees(root, findings)
    check_document_law(root, findings)
    findings.extend(d1_substrate_findings(root))
    findings.extend(specialization_findings(root))
    findings.extend(specialization_key_findings(root))
    findings.extend(proof_policy_findings(root))
    findings.extend(witness_reference_findings(root))
    findings.extend(integrity_witness_findings(root))
    findings.extend(derived_material_findings(root))
    findings.extend(deferred_posture_findings(root))
    findings.extend(proof_target_grammar_findings(root))
    findings.extend(proof_target_findings(root))
    findings.extend(leg081_authority_findings(root))
    findings.extend(proof_meaning_findings(root))
    findings.extend(control_character_findings(root))

    manifest = root / "SPEC.sha256"
    actual_files = frozen_files(root)
    for earlier, later in casefold_collisions(actual_files.keys()):
        findings.append(f"case-folded path collision (not Windows-portable): {earlier} and {later}")
    manifest_rows: dict[str, str] = {}
    if not manifest.is_file():
        findings.append("missing SPEC.sha256")
    else:
        for line_no, line in enumerate(manifest.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.startswith("#"):
                continue
            try:
                expected, rel = line.split("  ", 1)
            except ValueError:
                findings.append(f"SPEC.sha256:{line_no}: malformed row")
                continue
            if rel in manifest_rows:
                findings.append(f"SPEC.sha256:{line_no}: duplicate row {rel}")
            manifest_rows[rel] = expected
            path = actual_files.get(rel)
            if path is None:
                findings.append(f"SPEC.sha256:{line_no}: missing or excluded {rel}")
                continue
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != expected:
                findings.append(f"SPEC.sha256:{line_no}: digest mismatch {rel}")
        missing_rows = sorted(set(actual_files) - set(manifest_rows))
        extra_rows = sorted(set(manifest_rows) - set(actual_files))
        for rel in missing_rows:
            findings.append(f"SPEC.sha256: unrecorded file {rel}")
        for rel in extra_rows:
            findings.append(f"SPEC.sha256: row has no frozen file {rel}")

    if findings:
        print(f"audit: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    # The structurally discovered candidate set and the transitional expectation
    # debt are reported every run, not asserted as constants here. Both must reach
    # zero by D4d; a number that only ever appears in a report cannot rot into a
    # registry that every normalization commit has to ceremonially update.
    cand_ids, cand_blocks, pending = candidate_summary(root)
    print(
        f"audit: PASS ({len(markdown)} markdown documents, {len(contract_ids)} contract IDs, "
        f"{len(actual_files)} frozen files; {cand_ids} unbound proof candidates in "
        f"{cand_blocks} blocks; {pending} rows pending an expectation clause)"
    )
    return 0


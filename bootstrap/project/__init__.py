"""Deterministic projection generator for the clean-room specification.

Every projection plan is built by iterating the typed generated-view registry
(`spec/generated_views.rs` GeneratedView::ALL, 5.5E4a): embedded marker blocks
across the authored corpus, the standalone generated files — including the
Guarantee Graph (docs/GUARANTEE_GRAPH.generated.md) and the tracked
rust-toolchain.toml, both of which THIS tool generates — and the mechanical
corpus-frontmatter epoch convergence. The registry owns which views exist;
each renderer here owns only how an admitted authority source is serialized,
and no manual literal plan may bypass the registry.

`project.py --write`  rewrites every registered generated view in place.
`project.py --check`  regenerates and fails if any view has drifted.

This is the GENERATOR half of the generator/auditor pair. `bootstrap/audit.py`
is the read-only AUDITOR: it recomputes the same projections independently and
compares. The two share no parsing, normalization, projection-building, or
comparison logic — only the inert marker names, whose values are independently
asserted on both sides. project.py never generates decision text itself.

Standard library only. All writes are UTF-8 with literal LF endings.
"""
from __future__ import annotations

import sys

# Self-register so the package's relative imports resolve even under a loader
# that execs this module without first inserting it into sys.modules (the
# selftest project-growth sandbox does exactly that). The shim and
# selftest.load() register first, so this branch is a no-op there.
if __name__ not in sys.modules:  # pragma: no cover - loader-dependent
    import types as _types
    _self_module = _types.ModuleType(__name__)
    _self_module.__path__ = __path__
    sys.modules[__name__] = _self_module

import re
from pathlib import Path

from .operators import (
    COMPANION,
    OPERATOR_ROW,
    typing_rules,
    parse_operators,
    render_catalog,
    render_surfaces,
    render_grammar,
    render_projection,
    render_numeric,
    render_typing,
)
from .guarantees import (
    GUARANTEE_DOC,
    SEED_DOC,
    FAMILY_RANK,
    GUARANTEE_FRONT,
    gate_inventory,
    gate_tokens,
    guarantee_seed,
    family_policies,
    disposition_lifetimes,
    gate_optional_classes,
    Unadmitted,
    admit,
    guarantee_nodes,
    guarantee_edges,
    render_seed_classification,
    render_guarantee_graph,
    render_gate_inventory,
    render_decision_ledger,
    render_legacy_coverage,
)
from .registry import (
    block_pattern,
    GENERATED_VIEWS_SRC,
    parse_generated_views,
    canonical_block,
)
from .repository import (
    NUMERIC_DOC,
    GATES_DOC,
    CONTRACT_DOC,
    parse_contract_kinds,
    render_contract_kinds,
    parse_toolchain,
    render_root_toolchain,
    IDENTITY_DOC,
    IDENTITY_AXES,
    parse_identity_catalog,
    parse_identity_residue,
    render_identity_axis,
    render_identity_residue,
    SECURITY_DOC,
    parse_compiler_assumptions,
    render_compiler_assumptions,
    parse_corpus_epoch,
    render_corpus_epoch,
    converge_epoch_frontmatter,
    MUTATION_DOC,
    parse_mutation_lanes,
    parse_mutation_results,
    render_mutation_lanes,
    render_mutation_results,
    COMMANDS_DOC,
    COMPANION_DOC,
    COMMAND_NAMESPACES,
    parse_command_namespace,
    render_command_namespace,
    STALE_DOCS,
    stale_aliases,
    render_stale_vocabulary,
    RELEASE_DOC,
    release_seal_fields,
    render_release_seal,
    render_tracked_toolchain,
    render_package_inventory,
    render_package_edges,
    render_qualification_profiles,
    render_bundle_inventory,
    render_tier0_receipts,
)
from .domains import (
    parse_promotion,
    render_promotion,
    SYSTEM_DOC,
    RECON_SPEC,
    recon_coordinates,
    render_recon_coordinates,
    recon_retry,
    render_recon_retry,
    TESTPAK_DOC,
    proof_terminals,
    render_proof_terminals,
    ISA_SPEC,
    ISA_DOC,
    P_ISA_ARM,
    P_ISA_NODES,
    P_ISA_VARIANT,
    P_ISA_FIELD,
    pakvm_specs,
    render_pakvm_isa,
    render_pakvm_signatures,
    FIREWALL_SPEC,
    ARCH_SPEC,
    SYNCBAT_DOC,
    P_COMMENT,
    syncbat_firewall_facts,
    render_syncbat_plane_ownership,
    render_syncbat_authorities,
    render_syncbat_crossings,
    BO_SPEC,
    gate0_plan_facts,
    render_gate0_plan,
    render_verification_plans,
    parse_proof_relations,
    render_legacy_obligation_ledger,
    render_proof_relations,
    render_generated_view_registry,
    render_sprouting_vocabulary,
    render_specialized_plan_policy,
    _proof_requirements_renderer,
)


VIEW_RENDERERS = {
    "OperatorsCatalog": lambda root: render_catalog(parse_operators(root)),
    "OperatorsSurfaces": lambda root: render_surfaces(parse_operators(root)),
    "OperatorsGrammar": lambda root: render_grammar(parse_operators(root)),
    "OperatorsProjection": lambda root: render_projection(parse_operators(root)),
    "OperatorsTyping": lambda root: render_typing(parse_operators(root)),
    "OperatorsNumeric": lambda root: render_numeric(parse_operators(root)),
    "SeedClassification": render_seed_classification,
    "GuaranteeGraph": render_guarantee_graph,
    "PakVmSemanticIsa": render_pakvm_isa,
    "PakVmSignatures": render_pakvm_signatures,
    "SyncBatPlaneOwnership": render_syncbat_plane_ownership,
    "SyncBatAuthorities": render_syncbat_authorities,
    "SyncBatCrossings": render_syncbat_crossings,
    "ReconciliationCoordinates": render_recon_coordinates,
    "ReconciliationRetry": render_recon_retry,
    "ReleaseSeal": render_release_seal,
    "ProofTerminals": render_proof_terminals,
    "StaleVocabulary": render_stale_vocabulary,
    "GateInventory": render_gate_inventory,
    "ContractKinds": render_contract_kinds,
    "RustToolchain": render_tracked_toolchain,
    "IdentityCatalog": lambda root: render_identity_axis(root, "IdentityKind"),
    "GenerationCatalog": lambda root: render_identity_axis(root, "GenerationKind"),
    "BindingCatalog": lambda root: render_identity_axis(root, "BindingKind"),
    "VersionCatalog": lambda root: render_identity_axis(root, "VersionIdentityKind"),
    "IdentityResidue": render_identity_residue,
    "ProductCommands": lambda root: render_command_namespace(root, "ProductCommand"),
    "TestPakCommands": lambda root: render_command_namespace(root, "TestPakCommand"),
    "BatQlSourceModes": lambda root: render_command_namespace(root, "BatQlSourceMode"),
    "MutationLanes": render_mutation_lanes,
    "MutationResults": render_mutation_results,
    "PromotionRequirements": render_promotion,
    "CompilerAssumptionKinds": render_compiler_assumptions,
    "CorpusReconciliationEpoch": render_corpus_epoch,
    "CorpusEpochMembership": lambda root: parse_corpus_epoch(root)["spelling"],
    "PackageInventory": render_package_inventory,
    "PackageEdges": render_package_edges,
    "QualificationProfiles": render_qualification_profiles,
    "BundleInventory": render_bundle_inventory,
    "Tier0ReceiptDenominator": render_tier0_receipts,
    "DecisionLedger": render_decision_ledger,
    "LegacyInvariantCoverage": render_legacy_coverage,
    "LegacyObligationLedger": render_legacy_obligation_ledger,
    "GauntletProofRelations": render_proof_relations,
    "SystemModelProofRequirements": _proof_requirements_renderer("SystemModelProofRequirements"),
    "RepositoryProofRequirements": _proof_requirements_renderer("RepositoryProofRequirements"),
    "TypeSystemProofRequirements": _proof_requirements_renderer("TypeSystemProofRequirements"),
    "StorageProofRequirements": _proof_requirements_renderer("StorageProofRequirements"),
    "SyncBatProofRequirements": _proof_requirements_renderer("SyncBatProofRequirements"),
    "BvisorProofRequirements": _proof_requirements_renderer("BvisorProofRequirements"),
    "WorldPortsProofRequirements": _proof_requirements_renderer("WorldPortsProofRequirements"),
    "TestPakProofRequirements": _proof_requirements_renderer("TestPakProofRequirements"),
    "IdentityTimeProofRequirements": _proof_requirements_renderer("IdentityTimeProofRequirements"),
    "MigrationProofRequirements": _proof_requirements_renderer("MigrationProofRequirements"),
    "SecretAuthorityProofRequirements": _proof_requirements_renderer("SecretAuthorityProofRequirements"),
    "Gate0MaterializationPlan": render_gate0_plan,
    "VerificationPlans": render_verification_plans,
    "SproutingVocabulary": render_sprouting_vocabulary,
    "SpecializedPlanCandidatePolicy": render_specialized_plan_policy,
    "SproutingProofRequirements": _proof_requirements_renderer("SproutingProofRequirements"),
    "GeneratedViewRegistry": render_generated_view_registry,
}


def build_plans(root: Path, findings: list[str]):
    """Every projection plan, built by iterating the typed registry. The
    renderer-key set must equal the registry in both directions: a registered
    view with no renderer refuses, and a renderer with no registered view
    refuses."""
    views = parse_generated_views(root)
    names = [v["name"] for v in views]
    for missing in [n for n in names if n not in VIEW_RENDERERS]:
        findings.append(f"registered generated view {missing} has no projector renderer")
    for phantom in sorted(set(VIEW_RENDERERS) - set(names)):
        findings.append(f"projector renderer {phantom} serializes no registered view")
    texts: dict[str, list] = {}

    def load(rel: str) -> list:
        if rel not in texts:
            path = root / rel
            original = path.read_text(encoding="utf-8") if path.is_file() else ""
            texts[rel] = [path, original, original]
        return texts[rel]

    epoch_spelling = None
    for view in views:
        name = view["name"]
        render = VIEW_RENDERERS.get(name)
        if render is None:
            continue
        if view["surface"] == "EmbeddedBlock":
            if not view["marker"] or not view["sources"] or not view["targets"]:
                findings.append(
                    f"embedded generated view {name} lacks a marker, authority "
                    f"source, or static target")
                continue
            try:
                body = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
                continue
            pattern = block_pattern(view["marker"])
            replacement = canonical_block(view["marker"], "; ".join(view["sources"]), body)
            for rel in view["targets"]:
                entry = load(rel)
                if not entry[2]:
                    findings.append(f"{rel}: missing generated-view target file")
                    continue
                if not pattern.search(entry[2]):
                    findings.append(
                        f"{rel}: missing generated block markers for {view['marker']}")
                    continue
                entry[2] = pattern.sub(lambda m, r=replacement: r, entry[2])
        elif view["surface"] == "StandaloneFile":
            if len(view["targets"]) != 1:
                findings.append(
                    f"standalone generated view {name} must name exactly one target")
                continue
            try:
                content = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
                continue
            load(view["targets"][0])[2] = content
        elif view["surface"] == "CorpusFrontmatter":
            try:
                epoch_spelling = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
        else:
            findings.append(
                f"generated view {name} carries unknown surface {view['surface']!r}")
    corpus_bindings = 0
    if epoch_spelling is not None:
        for path in sorted(root.rglob("*.md")):
            rel_parts = path.relative_to(root).parts
            if any(part in (".git", "target", "__pycache__") for part in rel_parts):
                continue
            entry = load(path.relative_to(root).as_posix())
            entry[2] = converge_epoch_frontmatter(entry[2], epoch_spelling)
            if re.search(r"^(?:source_)?reconciliation_epoch:", entry[2], re.M):
                corpus_bindings += 1
    plans = [(entry[0], entry[1], entry[2]) for entry in texts.values()]
    return views, plans, corpus_bindings


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in ("--write", "--check"):
        print("usage: project.py --write|--check [root]", file=sys.stderr)
        return 2
    mode = sys.argv[1]
    root = Path(sys.argv[2] if len(sys.argv) > 2 else ".").resolve()
    findings: list[str] = []
    views, plans, corpus_bindings = build_plans(root, findings)
    static_instances = sum(len(view["targets"]) for view in views)
    if findings:
        print(f"project: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    if mode == "--write":
        changed = sum(1 for _path, original, rewritten in plans if rewritten != original)
        for path, original, rewritten in plans:
            if rewritten != original:
                path.write_bytes(rewritten.encode("utf-8"))
        verb = "WROTE" if changed else "OK"
        print(f"project: {verb} {len(views)} registered generated views "
              f"({static_instances} static target instances; "
              f"{corpus_bindings} corpus-frontmatter bindings)")
        return 0
    # --check
    stale = [path.name for path, original, rewritten in plans if rewritten != original]
    if stale:
        print(f"project: FAIL (registered generated views stale in {stale}; "
              "run project.py --write)", file=sys.stderr)
        return 1
    print(f"project: PASS ({len(views)} registered generated views current; "
          f"{static_instances} static target instances; "
          f"{corpus_bindings} corpus-frontmatter bindings)")
    return 0

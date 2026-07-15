#!/usr/bin/env python3
"""Independent portability self-test for the bootstrap freeze/audit tools.

Turns the first executable finding (host-dependent manifest ordering) into a
permanent guarantee. Standard library only; operates on synthetic temporary
trees and leaves no repository artifacts.

Proven properties:

1. Manifest ordering derives ONLY from the UTF-8 bytes of the POSIX relative
   path, never from filesystem enumeration or the host locale/case behavior.
2. audit rejects a case-folded path collision (e.g. ``Foo.md`` vs ``foo.md``)
   that a case-insensitive filesystem could not materialize, while leaving
   genuinely distinct paths untouched.
"""
from __future__ import annotations
import importlib.util
import sys
import shutil
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_manifest_ordering(freeze) -> list[str]:
    findings: list[str] = []
    names = ["A.md", "a-child.md", "Z.md", "nested/Á.md", "nested/z.md"]
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for name in names:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode("utf-8"))
        rendered = freeze.render(root).decode("utf-8").splitlines()
    order = [line.split("  ", 1)[1] for line in rendered]
    expected = sorted(names, key=lambda value: value.encode("utf-8"))
    if order != expected:
        findings.append(f"ordering is not UTF-8-byte canonical: {order} != {expected}")
    return findings


def test_casefold_collision(audit) -> list[str]:
    findings: list[str] = []
    collisions = audit.casefold_collisions(["Foo.md", "foo.md", "bar.md"])
    if [("Foo.md", "foo.md")] != collisions:
        findings.append(f"expected Foo.md/foo.md collision, got {collisions}")
    unrelated = audit.casefold_collisions(["a.md", "b.md", "nested/a.md"])
    if unrelated:
        findings.append(f"false collision reported on distinct paths: {unrelated}")
    return findings


def test_stale_vocabulary(audit) -> list[str]:
    """Hostile fixtures for the decision-derived stale-vocabulary matcher."""
    findings: list[str] = []
    amap = {"filebat": ("DEC-003", frozenset({"DecisionLedger", "LegacyEvidence"}), "FileBat")}

    def scan(text: str, context: str) -> list[str]:
        out: list[str] = []
        audit.scan_stale_occurrences("synthetic.md", text, context, amap, out)
        return out

    if not scan("FileBat is superseded", "OrdinaryAuthoritative"):
        findings.append("stale_alias_in_authoritative_doc_is_rejected FAILED")
    if not scan("let handle = FileBat::open();", "ProductionSource"):
        findings.append("stale_alias_in_production_source_is_rejected FAILED")
    if not scan("FileBat [STALE-REF: DEC-011]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_wrong_decision_reference_is_rejected FAILED")
    if scan("FileBat [STALE-REF: DEC-003]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_killed_decision_reference_is_allowed FAILED")
    if not scan("plain text [STALE-REF: DEC-999]", "OrdinaryAuthoritative"):
        findings.append("unknown_stale_reference_is_rejected FAILED")
    if scan("FileBat appears in the ledger", "DecisionLedger"):
        findings.append("stale_alias_in_allowed_context_passes FAILED")
    return findings


def test_stale_derivation(audit, root) -> list[str]:
    """The matcher is derived from spec/dispositions.rs, not a hand list."""
    findings: list[str] = []
    problems: list[str] = []
    amap = audit.parse_stale_vocabulary(root, problems)
    if problems:
        findings.append(f"parse_stale_vocabulary reported problems on the real seed: {problems}")
    if len(amap) < 20:
        findings.append(f"generated_stale_table_must_match_decisions: derived only {len(amap)} aliases")
    owner = amap.get("filebat")
    if not owner or owner[0] != "DEC-003":
        findings.append("FileBat alias is not derived to its killing decision DEC-003")
    if "made-up-stale-term" in amap:
        findings.append("alias_removed_from_decision_disappears_from_all_views FAILED")
    return findings


def test_legacy_manifest_parity(audit) -> list[str]:
    """Hostile fixtures for the D-1 declaration-parity denominator.

    The denominator is measured against declared invariant IDs, so a structural
    key that merely resembles an invariant name (the generated INV-CATALOG
    marker) must never be counted as a declaration.
    """
    findings: list[str] = []
    base = ["INV-A", "INV-B", "INV-C"]
    check = audit.legacy_manifest_findings

    if check(base, base, 3):
        findings.append("exact_declaration_parity_should_pass FAILED")
    if not check(base + ["INV-CATALOG"], base, 3):
        findings.append("structural_INV_token_counted_as_declaration_is_rejected FAILED")
    if not check(base, ["INV-A", "INV-B"], 3):
        findings.append("declared_invariant_without_coverage_is_rejected FAILED")
    if not check(base, base + ["INV-GHOST"], 3):
        findings.append("coverage_for_undeclared_invariant_is_rejected FAILED")
    if not check(base, base, 4):
        findings.append("denominator_count_mismatch_is_rejected FAILED")
    if not check(base + ["INV-A"], base, 3):
        findings.append("duplicate_declaration_id_is_rejected FAILED")
    return findings


def _sample_operators() -> list[dict[str, str]]:
    base = {
        "semantic_op": "s", "input_sorts": "a", "result_sort": "b", "overflow": "o",
        "exception": "e", "mutation_classes": "m",
    }
    return [
        {**base, "id": "OP-ADD", "class": "Arithmetic", "word": "+", "symbol": "",
         "arity": "Binary", "fixity": "Infix", "precedence": "60",
         "associativity": "Left", "exactness": "Exact", "formatting": "+", "spoken": "plus"},
        {**base, "id": "OP-EQ", "class": "Comparison", "word": "IS", "symbol": "=",
         "arity": "Binary", "fixity": "Infix", "precedence": "50",
         "associativity": "NonAssociative", "exactness": "Exact", "formatting": "IS", "spoken": "is"},
    ]


def test_batql(audit, project) -> list[str]:
    """Named hostile fixtures for the BatQL operator/grammar/proof surfaces."""
    findings: list[str] = []
    ops = _sample_operators()
    companion = (HERE.parent / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # 1 & 2: proof-disposition axis must keep the exact five states
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("missing_legacy_weak_on_authoritative_surface_is_rejected")
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("legacy_weak_collapsed_by_projection_is_rejected")

    # 3 & 4: grammar reference closure and unique ownership
    undefined, _, _ = audit.batql_grammar_closure_findings(["foo\n  := bar baz"])
    if not undefined:
        fail("undefined_grammar_leaf_is_rejected")
    dup, _, _ = audit.batql_grammar_closure_findings(['foo\n  := "x"\n\nfoo\n  := "y"'])
    if not any("more than one owning" in item for item in dup):
        fail("duplicate_grammar_owner_is_rejected")

    # 5: MATCH requires an initial step plus at least one THEN step
    if "match_step (THEN match_step)+" not in companion or "match_step (THEN match_step)*" in companion:
        fail("one_step_match_is_rejected")

    # 6: kernel declaration clauses appear once, in canonical order
    order = ["FROM KERNEL kernel_ref", "purity_clause", "determinism_clause", "capability_clause", "cost_clause"]
    positions = [companion.find(token) for token in order]
    if -1 in positions or positions != sorted(positions):
        fail("reordered_or_duplicated_kernel_clauses_are_rejected")

    # 7 & 8 & 9: operator fact discipline
    if not any("duplicate OperatorId" in x for x in audit.batql_operator_fact_findings(ops + [dict(ops[0])])):
        fail("duplicate_operator_id_is_rejected")
    clash = {**ops[0], "id": "OP-ADD-CLONE"}
    if not any("claimed by" in x for x in audit.batql_operator_fact_findings([ops[0], clash])):
        fail("ambiguous_token_plus_fixity_ownership_is_rejected")
    missing_field = {**ops[0], "spoken": ""}
    if not any("missing field spoken" in x for x in audit.batql_operator_fact_findings([missing_field])):
        fail("missing_required_operatorspec_field_is_rejected")

    # 10, 11, 12: projection parity is sensitive to alteration, missing, and extra rows
    expected = audit.batql_render_catalog(ops)
    if expected == expected.replace("Exact", "EXACT", 1):
        fail("altered_generated_operator_projection_is_rejected")
    if expected == audit.batql_render_catalog([ops[0]]):
        fail("missing_generated_operator_row_is_rejected")
    if expected == audit.batql_render_catalog(ops + [{**ops[0], "id": "OP-XTRA", "word": "~"}]):
        fail("extra_generated_operator_row_is_rejected")

    # 13: canonical order is independent of source array order
    if audit.batql_render_catalog(ops) != audit.batql_render_catalog(list(reversed(ops))):
        fail("source_fact_reorder_leaves_generated_output_unchanged")

    # 14: the independent generator and auditor projections must agree
    if (
        project.render_catalog(ops) != audit.batql_render_catalog(ops)
        or project.render_grammar(ops) != audit.batql_render_grammar(ops)
        or project.render_projection(ops) != audit.batql_render_projection(ops)
    ):
        fail("generator_auditor_disagreement_turns_gate_red")

    # 15: FLOOR/CEILING without the language-change record turns the gate red
    modes = "rounding_mode\n  := HALF_EVEN | HALF_UP | DOWN | UP | FLOOR | CEILING\n"
    if audit.batql_language_change_findings(modes + "\nLanguage-change record (BatQL V1)\n"):
        fail("language_change_record_present_passes")
    if not audit.batql_language_change_findings(modes):
        fail("floor_ceiling_without_language_change_record_is_rejected")

    return findings


def test_numeric(audit) -> list[str]:
    """Named hostile fixtures for the numeric authority contract (docs/37)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    markers = [marker for _name, marker in audit.NUMERIC_LAW_MARKERS]
    if audit.numeric_law_findings("\n".join(markers)):
        fail("all_numeric_laws_present_passes")
    for name, marker in audit.NUMERIC_LAW_MARKERS:
        reduced = "\n".join(m for m in markers if m != marker)
        if not any(name in item for item in audit.numeric_law_findings(reduced)):
            fail(f"numeric_law_{name}_absence_is_rejected")

    # OperatorSpec arithmetic row lacking an approximate-support posture is rejected
    arith_no_posture = [{
        "id": "OP-ADD", "class": "Arithmetic", "word": "+", "symbol": "",
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix", "precedence": "60",
        "associativity": "Left", "input_sorts": "a", "result_sort": "b", "exactness": "Exact",
        "overflow": "o", "exception": "e", "formatting": "+", "spoken": "plus",
        "mutation_classes": "m", "numeric_support": "NotApplicable",
    }]
    if not any("lacks an approximate-support posture" in x for x in audit.batql_operator_fact_findings(arith_no_posture)):
        fail("operatorspec_arithmetic_row_lacking_approx_support_is_rejected")

    # An operator that admits raw approximate operands (QualifiedProfileOnly) with no
    # qualified numeric profile in V1 is rejected; ExactSupported never admits them.
    admits_raw_approx = [{**arith_no_posture[0], "numeric_support": "QualifiedProfileOnly"}]
    if not any("no qualified numeric profile exists" in x for x in audit.batql_operator_fact_findings(admits_raw_approx)):
        fail("operator_admitting_raw_approximate_without_profile_is_rejected")

    return findings


def test_guarantees(audit, project) -> list[str]:
    """Named hostile fixtures for the Guarantee Graph (DEC-070)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def seed(sid, kind="SemanticLaw", life="Permanent", owner="o", witness="w"):
        return {"id": sid, "kind": kind, "lifetime": life, "owner": owner, "gates": "",
                "witness": witness, "failure": "f",
                "rel": {"DerivesFrom": [], "Refines": [], "Discharges": [], "Supersedes": []}}

    def node(nid, family="SEED", kind="SemanticLaw", life="Permanent", owner="o", gates="", witness="w"):
        return {"id": nid, "family": family, "kind": kind, "lifetime": life,
                "owner": owner, "gates": gates, "witness": witness}

    good = [seed(f"SEED-{i}") for i in range(25)]
    if audit.guarantee_classification_findings(good):
        fail("valid_classification_passes")
    if not audit.guarantee_classification_findings(good[:24]):
        fail("missing_seed_row_is_rejected")
    if not audit.guarantee_classification_findings(good + [seed("SEED-25")]):
        fail("extra_seed_row_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "kind": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_kind_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "lifetime": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_lifetime_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "owner": ""}] + good[1:]):
        fail("unclassified_seed_row_is_rejected")

    ids = {"A", "B", "C"}
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "Z")]):
        fail("dangling_relation_target_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "A")]):
        fail("self_edge_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "B"), ("A", "DerivesFrom", "B")]):
        fail("duplicate_edge_is_rejected")
    for fam in ("DerivesFrom", "Refines", "Supersedes", "Discharges"):
        if not any("cycle" in x for x in audit.guarantee_relation_findings(ids, [("A", fam, "B"), ("B", fam, "A")])):
            fail(f"{fam.lower()}_cycle_is_rejected")

    if not any("Permanent SemanticLaw names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("SEED-X", witness="")], {}, [])):
        fail("permanent_semanticlaw_without_witness_is_rejected")
    if not any("Permanent ArchitectureConstraint names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("A", kind="ArchitectureConstraint", witness="")], {}, [])):
        fail("permanent_architecture_without_witness_is_rejected")
    if audit.guarantee_lifetime_findings([node("D", kind="Decision", witness="")], {}, []):
        fail("permanent_decision_witness_deferred_to_c2")
    if audit.guarantee_lifetime_findings([node("Q", kind="QualificationRequirement", witness="")], {}, []):
        fail("permanent_qualification_is_self_witnessing")
    if not any("UntilGate names no gate" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilGate", gates="")], {}, [])):
        fail("untilgate_without_gate_is_rejected")
    if not any("HistoricalCoverageOnly cannot gate" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="HistoricalCoverageOnly", gates="G2")], {}, [])):
        fail("historicalcoverageonly_carrying_active_gate_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor", owner="batpak::store", gates="G2")], {}, [])):
        fail("untilsuccessor_without_typed_successor_is_rejected")
    if audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor")], {}, [("Y", "Supersedes", "X")]):
        fail("untilsuccessor_with_resolvable_successor_passes")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "G2")]):
        fail("gate_name_cannot_become_successor_edge")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "batpak::store")]):
        fail("clean_owner_text_cannot_become_successor_edge")

    def legnode(life, gates="G2"):
        return node("LEG-1", family="LEG", kind="LegacyObligation", life=life, gates=gates, witness="")

    active_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Active"}}
    if not any("must not force Permanent" in x for x in audit.guarantee_lifetime_findings([legnode("Permanent")], active_never, [])):
        fail("deletion_never_forcing_permanent_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([legnode("UntilSuccessor")], active_never, [])):
        fail("gated_leg_without_typed_successor_cannot_project_untilsuccessor")
    closed_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Closed"}}
    if audit.guarantee_lifetime_findings([legnode("ClosedEvidence", gates="")], closed_never, []):
        fail("deletion_never_closed_leg_projects_closedevidence")
    if not any("Active LEG projects as ClosedEvidence" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], active_never, [])):
        fail("closedevidence_marked_active_is_rejected")
    closed = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "OnSuccessorGateClosure", "status": "Closed"}}
    if not any("still names an implementation gate" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], closed, [])):
        fail("closed_leg_still_gating_is_rejected")

    both = {"SEED-X", "LEG-1"}
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Discharges", "LEG-1")])):
        fail("bare_law_discharges_active_leg_is_rejected")
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Closes", "LEG-1")])):
        fail("bare_law_closes_active_leg_is_rejected")
    if audit.guarantee_relation_findings(both, [("SEED-X", "Refines", "LEG-1")]):
        fail("refines_active_leg_passes")

    root = HERE.parent
    proj_nodes = [(n["id"], n["family"], n["kind"], n["lifetime"], n["owner"], n["gates"]) for n in project.guarantee_nodes(root)]
    audit_nodes = [(n["id"], n["family"], n["kind"], n["lifetime"], n["owner"], n["gates"]) for n in audit.guarantee_derive(root)[0]]
    if proj_nodes != audit_nodes:
        fail("generator_auditor_guarantee_disagreement")
    nodes, edges = audit.guarantee_derive(root)
    if nodes != sorted(nodes, key=lambda n: (audit.G_FAMILY_RANK[n["family"]], n["id"])):
        fail("source_reorder_changes_guarantee_order")
    if any(not src.startswith("SEED-") for src, _k, _t in edges):
        fail("non_seed_source_guarantee_edge_present")

    return findings


# --- Guarded mutation: a claimed negative test is itself a claim -------------
# A red light caused by the wrong severed wire is not evidence that the alarm
# works. Every mutation below proves four things: the target existed, the bytes
# actually changed, the intended validator ran, and the EXPECTED finding class
# appeared -- never merely that something somewhere returned nonzero.
class ProbeError(AssertionError):
    """The probe itself is broken: absent target or inert mutation."""


def must_replace(text: str, old: str, new: str, what: str) -> str:
    if old not in text:
        raise ProbeError(f"probe target absent, mutation never applied: {what}")
    out = text.replace(old, new, 1)
    if out == text:
        raise ProbeError(f"probe mutation changed no bytes: {what}")
    return out


def must_remove(text: str, old: str, what: str) -> str:
    return must_replace(text, old, "", what)


def gate_sandbox(edits):
    """A repo copy with guarded edits applied; returns the sandbox root."""
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp())
    for sub in ("spec", "docs", "companion"):
        src = root / sub
        if src.is_dir():
            shutil.copytree(src, tmp / sub)
    for rel, old, new in edits:
        path = tmp / rel
        text = path.read_text(encoding="utf-8")
        path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
    return tmp


def test_gates(audit, project) -> list[str]:
    """Named hostile fixtures for the one gate identity (5.5C2a)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        # The expected finding class must appear, not merely some error.
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def guarantee_findings(tmp):
        out: list[str] = []
        audit.check_guarantees(tmp, out)
        return out

    # --- typed gate list rules ---
    expect("unknown_gateid_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G99"]), "unknown GateId")
    expect("duplicate_gateid_in_one_fact_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G2", "G2"]), "duplicate GateId")
    expect("noncanonical_gate_order_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G5", "G2"]), "not in canonical order")
    if audit.gate_findings(root, "SEED", "X", ["G2", "G5"]):
        fail("canonical_gate_list_passes")

    # --- inventory integrity ---
    probe("gateid_missing_from_inventory_is_rejected",
          [("spec/gates.rs",
            '    GateSpec { id: GateId::GJ, token: "GJ", title: "Integrated final tree" },',
            "")],
          audit.gate_inventory_findings, "missing from the GATES inventory")
    probe("inventory_token_duplicated_across_two_gateids_is_rejected",
          [("spec/gates.rs",
            'GateSpec { id: GateId::G1, token: "G1", title: "MacBat" }',
            'GateSpec { id: GateId::G1, token: "G0", title: "MacBat" }')],
          audit.gate_inventory_findings, "one token under two GateIds")

    # --- migration conservation: a SEED/LEG gate set may not move ---
    def gates_of(tmp, ident):
        return {n["id"]: n["gates"] for n in audit.guarantee_derive(tmp)[0]}[ident]

    base_seed = gates_of(root, "SEED-PAKVM-NAME")
    tmp = gate_sandbox([("spec/invariants.rs",
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5]',
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0]')])
    if gates_of(tmp, "SEED-PAKVM-NAME") == base_seed:
        fail("seed_gate_dropped_during_migration_is_detected")
    shutil.rmtree(tmp, ignore_errors=True)

    tmp = gate_sandbox([("spec/invariants.rs",
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5]',
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5, GateId::G7]')])
    if gates_of(tmp, "SEED-PAKVM-NAME") == base_seed:
        fail("seed_gate_added_during_migration_is_detected")
    shutil.rmtree(tmp, ignore_errors=True)

    base_leg = gates_of(root, "LEG-001")
    tmp = gate_sandbox([("spec/legacy_obligations.rs",
                         'clean_owner: "batpak::store", gates: &[GateId::G2], compatibility_disposition: '
                         "CompatibilityDisposition::None, deletion_condition: "
                         "DeletionCondition::OnSuccessorGateClosure",
                         'clean_owner: "batpak::store", gates: &[], compatibility_disposition: '
                         "CompatibilityDisposition::None, deletion_condition: "
                         "DeletionCondition::OnSuccessorGateClosure")])
    if gates_of(tmp, "LEG-001") == base_leg:
        fail("leg_gate_dropped_during_migration_is_detected")
    expect("emptied_gate_list_is_rejected",
           audit.gate_reference_findings(tmp), "names no gate")
    shutil.rmtree(tmp, ignore_errors=True)

    probe("leg_gate_added_during_migration_is_rejected",
          [("spec/legacy_obligations.rs",
            'clean_owner: "batpak::event", gates: &[GateId::G2]',
            'clean_owner: "batpak::event", gates: &[GateId::G2, GateId::G2]')],
          audit.gate_reference_findings, "duplicate GateId")

    # --- the retired string representation may not survive anywhere ---
    probe("old_string_gate_field_in_seed_is_rejected",
          [("spec/invariants.rs", "pub gates: &'static [GateId],", 'pub gates: "G0",')],
          audit.gate_reference_findings, "still carries a string gate field")
    probe("old_string_gate_field_in_leg_is_rejected",
          [("spec/legacy_obligations.rs", "pub gates: &'static [GateId],", 'pub gate: "G2",')],
          audit.gate_reference_findings, "still carries a string gate field")

    # --- generated projections may not drift from the typed authority ---
    probe("docs25_gate_inventory_drift_is_rejected",
          [("docs/25_IMPLEMENTATION_GATES.md",
            "| G4 | G4 | BatQL compiler |", "| G4 | G4 | BatQL compilerr |")],
          audit.gate_doc_findings, "does not equal spec/gates.rs")
    probe("docs23_rendered_seed_gate_drift_is_rejected",
          [("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
            "| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 |",
            "| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0 |")],
          guarantee_findings, "SEED-CLASSIFICATION block does not equal")
    probe("guarantee_graph_gate_metadata_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| LEG-001 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 |",
            "| LEG-001 | LEG | LegacyObligation | UntilGate | - | batpak::store | G7 |")],
          guarantee_findings, "node table does not equal")
    probe("docs21_rendered_leg_gate_drift_is_rejected",
          [("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "| concurrency model + duplicate-writer refusal | G2 |",
            "| concurrency model + duplicate-writer refusal | G3 |")],
          lambda tmp: [f for f in (lambda o: (audit.check_seed_ids(tmp, o), o)[1])([])],
          "legacy seed/document mismatch")

    # --- generator and auditor agree; source order does not change bytes ---
    if [(n["id"], n["gates"]) for n in project.guarantee_nodes(root)] != \
       [(n["id"], n["gates"]) for n in audit.guarantee_derive(root)[0]]:
        fail("generator_auditor_gate_disagreement")
    if [g[0] for g in project.gate_inventory(root)] != [g[0] for g in audit.gate_inventory(root)]:
        fail("generator_auditor_inventory_disagreement")
    inv = [g[0] for g in audit.gate_inventory(root)]
    if inv != sorted(inv, key=audit.gate_enum_variants(root).index):
        fail("gate_inventory_is_canonically_ordered")

    # --- the probe harness itself fails closed ---
    try:
        must_replace("abc", "zzz", "q", "self-check")
    except ProbeError:
        pass
    else:
        fail("must_replace_fails_closed_on_absent_target")
    try:
        must_replace("abc", "abc", "abc", "self-check")
    except ProbeError:
        pass
    else:
        fail("must_replace_fails_closed_on_inert_mutation")

    return findings


def test_decisions(audit, project) -> list[str]:
    """Named hostile fixtures for DecisionClass and gate binding (DEC-072)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # A row losing its class is a parse-visible defect, not a silent default.
    probe("missing_decision_class_is_rejected",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0], ',
            'DecisionSpec { id: "DEC-016", ')],
          audit.decision_class_findings, "carry no DecisionClass")
    probe("unknown_decision_class_is_rejected",
          [("spec/dispositions.rs", "class: DecisionClass::Enforcement, gates: &[GateId::G0], "
                                    'disposition: Disposition::Keep, subject: "Status/supersession metadata"',
            "class: DecisionClass::Bogus, gates: &[GateId::G0], "
            'disposition: Disposition::Keep, subject: "Status/supersession metadata"')],
          audit.decision_class_findings, "unknown DecisionClass")

    # Each implementation-bearing class must name a gate. One representative row
    # per class, so a class-specific hole cannot hide behind its neighbours.
    for cls, did, gates in (
        ("Architecture", "DEC-007", "&[GateId::G0]"),
        ("Capability", "DEC-017", "&[GateId::G6]"),
        ("Compatibility", "DEC-020", "&[GateId::G2]"),
        ("Enforcement", "DEC-016", "&[GateId::G0]"),
        ("ImplementationPosture", "DEC-033", "&[GateId::G8]"),
    ):
        probe(f"{cls.lower()}_decision_without_gate_is_rejected",
              [("spec/dispositions.rs",
                f'id: "{did}", class: DecisionClass::{cls}, gates: {gates}',
                f'id: "{did}", class: DecisionClass::{cls}, gates: &[]')],
              audit.decision_class_findings, "names no implementation or qualification gate")

    probe("unknown_gateid_on_a_decision_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0]',
            'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G12]')],
          audit.decision_class_findings, "unknown GateId")
    probe("duplicate_gateid_on_a_decision_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0]',
            'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0, GateId::G0]')],
          audit.decision_class_findings, "duplicate GateId")
    probe("noncanonical_decision_gate_order_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-072", class: DecisionClass::Enforcement, gates: &[GateId::G0, GateId::G9]',
            'id: "DEC-072", class: DecisionClass::Enforcement, gates: &[GateId::G9, GateId::G0]')],
          audit.decision_class_findings, "not in canonical order")

    # An ungated Naming/HistoricalReceipt row is lawful and must stay green.
    if audit.decision_class_findings(root):
        fail("ungated_naming_and_historical_receipt_rows_pass")

    def seed_id_findings(tmp):
        out: list[str] = []
        audit.check_seed_ids(tmp, out)
        return out

    probe("docs30_decision_class_mismatch_is_rejected",
          [("docs/30_DECISION_AND_REJECTION_LEDGER.md",
            "| DEC-016 | KEEP | Enforcement | G0 |", "| DEC-016 | KEEP | Capability | G0 |")],
          seed_id_findings, "decision seed/document mismatch for DEC-016")
    probe("docs30_decision_gate_mismatch_is_rejected",
          [("docs/30_DECISION_AND_REJECTION_LEDGER.md",
            "| DEC-016 | KEEP | Enforcement | G0 |", "| DEC-016 | KEEP | Enforcement | G5 |")],
          seed_id_findings, "decision seed/document mismatch for DEC-016")

    def guarantee_findings(tmp):
        out: list[str] = []
        audit.check_guarantees(tmp, out)
        return out

    probe("guarantee_graph_losing_decision_class_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| DEC-016 | DEC | Decision | Permanent | Enforcement |",
            "| DEC-016 | DEC | Decision | Permanent | - |")],
          guarantee_findings, "node table does not equal")
    probe("guarantee_graph_losing_a_decision_gate_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| DEC-072 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0/G9 |",
            "| DEC-072 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 |")],
          guarantee_findings, "node table does not equal")

    # Decision gates are metadata. They never become edges, and no gate or
    # profile node is synthesized to hold them.
    nodes, edges = audit.guarantee_derive(root)
    families = {n["family"] for n in nodes}
    if families != {"SEED", "LEG", "DEC", "ARCH", "QUAL"}:
        fail(f"unexpected guarantee family present: {sorted(families)}")
    node_ids = {n["id"] for n in nodes}
    inv_tokens = {g[1] for g in audit.gate_inventory(root)}
    if node_ids & inv_tokens:
        fail("synthetic gate node present in the Guarantee Graph")
    for s, k, t in edges:
        if s in inv_tokens or t in inv_tokens:
            fail(f"synthetic gate edge present: {s} {k} {t}")
    for profile in ("InternalConsistency", "SignedHistory", "ExternallyAnchoredHistory"):
        if profile in node_ids:
            fail(f"authenticated-history profile added as a graph node: {profile}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 72:
        fail("decision node count is not 72")

    if [(n["id"], n.get("dclass"), n["gates"]) for n in project.guarantee_nodes(root) if n["family"] == "DEC"] != \
       [(n["id"], n.get("dclass"), n["gates"]) for n in audit.guarantee_derive(root)[0] if n["family"] == "DEC"]:
        fail("generator_auditor_decision_metadata_disagreement")
    return findings


def test_authenticated_history(audit) -> list[str]:
    """Named hostile fixtures for the authenticated-history contract (DEC-071).

    Structural only. Bootstrap performs no signature, accumulator, witness,
    freshness, or rollback verification, and these fixtures do not pretend
    otherwise: they prove the CONTRACT cannot be weakened, not that crypto works.
    """
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, old, new, needle, validator=None):
        tmp = gate_sandbox([("spec/architecture.rs", old, new)])
        try:
            expect(name, (validator or audit.authenticated_history_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.authenticated_history_findings(root):
        fail("frozen_matrix_and_bundles_pass")

    # --- the four axes are independently representable ---------------------
    # This is the defect C2c corrects: one mutually exclusive posture could not
    # state integrity AND rollback-resistance at once.
    bundles = audit.claim_bundles(root)
    want = {
        "INTERNAL_CONSISTENCY_SUCCESS":
            ("InternalConsistencyVerified", "NotClaimed", "NotClaimed", "Unavailable"),
        "SIGNED_UNANCHORED_SUCCESS":
            ("InternalConsistencyVerified", "SignedHistoryVerified", "NotClaimed", "Unavailable"),
        "WITNESSED_SUCCESS":
            ("InternalConsistencyVerified", "SignedHistoryVerified",
             "WitnessedGenerationVerified", "ScopedToVerifiedWitness"),
    }
    for name, axes in want.items():
        got = bundles.get(name)
        if got is None:
            fail(f"{name.lower()}_bundle_exists")
        elif got != axes:
            fail(f"{name.lower()}_encodes_all_four_axes (wanted {axes}, got {got})")
    # Each axis must be independently readable, not a summary of the others.
    if bundles.get("INTERNAL_CONSISTENCY_SUCCESS", ("",))[0] != "InternalConsistencyVerified" or \
       bundles.get("INTERNAL_CONSISTENCY_SUCCESS", ("", "", "", ""))[3] != "Unavailable":
        fail("internal_consistency_success_encodes_integrity_and_rollback_unavailable_together")

    probe("history_claim_posture_reintroduced_is_rejected",
          "pub enum IntegrityClaim {", "pub enum HistoryClaimPosture {",
          "retired claim vocabulary HistoryClaimPosture")
    probe("claim_posture_field_reintroduced_is_rejected",
          "    pub integrity: IntegrityClaim,", "    pub claim_posture: IntegrityClaim,",
          "retired claim vocabulary claim_posture")
    probe("security_ladder_is_rejected",
          "pub enum IntegrityClaim {", "pub enum SecurityLevel {",
          "generic security ladder SecurityLevel")

    # --- no fallback success for a required witness ------------------------
    probe("externally_anchored_given_unanchored_success_is_rejected",
          "        // No successful unanchored result is admitted. An absent or invalid\n"
          "        // required witness refuses; it never falls back to a weaker success.\n"
          "        unanchored_success_claims: None,\n"
          "        verified_witness_success_claims: Some(WITNESSED_SUCCESS),",
          "        unanchored_success_claims: Some(SIGNED_UNANCHORED_SUCCESS),\n"
          "        verified_witness_success_claims: Some(WITNESSED_SUCCESS),",
          "must refuse, not fall back to a weaker success")

    # --- per-axis claim ceilings -------------------------------------------
    probe("internal_consistency_claiming_signed_authenticity_is_rejected",
          "pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::NotClaimed,",
          "pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,",
          "!= frozen bundle")
    probe("internal_consistency_claiming_freshness_is_rejected",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "drift apart")
    probe("internal_consistency_claiming_scoped_rollback_resistance_is_rejected",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "drift apart")
    probe("unanchored_signed_history_claiming_freshness_is_rejected",
          "pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,\n"
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "unanchored success claims freshness or rollback resistance")

    # --- freshness and rollback resistance may not drift apart -------------
    probe("freshness_verified_while_rollback_unavailable_is_rejected",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "drift apart")
    probe("rollback_scoped_while_freshness_not_claimed_is_rejected",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "drift apart")
    probe("success_bundle_without_verified_integrity_is_rejected",
          "pub enum IntegrityClaim {\n    InternalConsistencyVerified,\n}",
          "pub enum IntegrityClaim {\n    InternalConsistencyVerified,\n    NotClaimed,\n}",
          "!= frozen")

    # --- invalid profile/policy pairs are refused, never normalized --------
    probe("internal_consistency_with_optional_witness_is_rejected",
          "        permitted_witness_policies: &[WitnessPolicy::None],\n"
          "        requires_local_commitment_verification: true,\n"
          "        requires_signed_history_verification: false,",
          "        permitted_witness_policies: &[WitnessPolicy::None, WitnessPolicy::Optional],\n"
          "        requires_local_commitment_verification: true,\n"
          "        requires_signed_history_verification: false,",
          "frozen matrix says")
    probe("signed_history_with_required_witness_is_rejected",
          "        permitted_witness_policies: &[WitnessPolicy::None, WitnessPolicy::Optional],",
          "        permitted_witness_policies: &[WitnessPolicy::Required],",
          "permits WitnessPolicy::Required outside ExternallyAnchoredHistory")
    probe("externally_anchored_with_optional_witness_is_rejected",
          "        permitted_witness_policies: &[WitnessPolicy::Required],",
          "        permitted_witness_policies: &[WitnessPolicy::Optional],",
          "permits a non-Required witness policy")

    # --- witness axes stay distinct ---------------------------------------
    for variant in ("NotProvided", "Stale", "Conflicting", "Unverifiable",
                    "CryptographicallyInvalid", "LineageMismatch", "GenerationMismatch",
                    "AccumulatorMismatch"):
        probe(f"required_witness_{variant.lower()}_dropped_from_failure_set_is_rejected",
              f"    WitnessDisposition::{variant},\n", "", "!= frozen failure set")
    probe("witness_disposition_collapsed_into_valid_invalid_is_rejected",
          "    /// Supplied and contradicts the observed history.\n    Conflicting,\n", "",
          "collapses required distinctions")
    probe("optional_witness_absent_incorrectly_failing_is_rejected",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n    WitnessDisposition::Stale,",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n"
          "    WitnessDisposition::NotProvided,\n    WitnessDisposition::Stale,",
          "refuses an absent optional witness")
    probe("present_invalid_optional_witness_treated_as_absent_is_rejected",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n    WitnessDisposition::Stale,\n",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n",
          "may degrade to absence or success")
    probe("refusal_partial_evidence_law_removed_is_rejected",
          "pub const REFUSAL_PARTIAL_CLAIM_LAW", "pub const REFUSAL_PARTIAL_CLAIM_LAW_RENAMED",
          "states no refusal/partial-evidence law")
    return findings


def test_claim_receipt_law(audit) -> list[str]:
    """docs/14 must carry each claim axis separately (DEC-071)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    cases = [
        ("docs14_missing_integrity_claim_is_rejected", "IntegrityClaim",
         "receipt preserves the integrity claim"),
        ("docs14_missing_authenticity_claim_is_rejected", "AuthenticityClaim",
         "receipt preserves the authenticity claim"),
        ("docs14_missing_freshness_claim_is_rejected", "FreshnessClaim",
         "receipt preserves the freshness claim"),
        ("docs14_missing_rollback_resistance_claim_is_rejected", "RollbackResistanceClaim",
         "receipt preserves the rollback-resistance claim"),
        ("docs14_omitting_success_versus_refusal_is_rejected", "success or refusal outcome",
         "receipt states success versus refusal"),
        ("docs14_allowing_axis_inference_from_profile_name_is_rejected",
         "may infer an omitted claim axis", "no axis inferred from a profile name"),
        ("docs14_refusal_forced_into_success_bundle_is_rejected",
         "A refusal is never forced into one", "a refusal is not a weaker success"),
        ("docs14_partial_evidence_claiming_freshness_is_rejected",
         "never carry a freshness or scoped rollback-resistance claim after witness verification failed",
         "partial evidence never claims freshness after witness failure"),
        ("docs14_required_witness_fallback_success_is_rejected",
         "no successful unanchored result at all",
         "no successful unanchored result for a required witness"),
        ("docs14_optional_absent_and_invalid_rendering_identically_is_rejected",
         "An absent optional witness is a successful unanchored result",
         "absent and invalid optional witnesses never render identically"),
    ]
    for name, old, needle in cases:
        tmp = gate_sandbox([("docs/14_RECEIPTS_AND_EXPLANATION.md", old, "")])
        try:
            produced = doc_findings(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_delivery_notes(audit) -> list[str]:
    """The delivery inventory must not keep a stale decision count."""
    findings: list[str] = []
    text = (HERE.parent / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    if "70 architectural decision/disposition rows" in text:
        findings.append("delivery_notes_remaining_at_70_decisions FAILED")
    if "72 architectural decision/disposition rows" not in text:
        findings.append("delivery_notes_states_72_decisions FAILED")
    return findings


def test_document_law(audit) -> list[str]:
    """The rollback threat and the anti-flattening axes must stay in the docs."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    for name, rel, old, needle in (
        ("whole_store_rollback_threat_removed_is_rejected",
         "docs/19_SECURITY_MODEL.md",
         "may restore an older complete generation whose internal commitments and signatures remain valid",
         "whole-store rollback threat"),
        ("integrity_authenticity_freshness_collapsed_is_rejected",
         "docs/19_SECURITY_MODEL.md",
         "rollback resistance  restoring an older valid generation is detectable",
         "four axes"),
        ("receipt_losing_exact_profile_is_rejected",
         "docs/14_RECEIPTS_AND_EXPLANATION.md", "selected AuthenticatedHistoryProfile",
         "receipt preserves the exact profile"),
        ("receipt_losing_exact_witness_disposition_is_rejected",
         "docs/14_RECEIPTS_AND_EXPLANATION.md",
         "selected WitnessPolicy\nexact WitnessDisposition",
         "receipt preserves the exact witness disposition"),
    ):
        tmp = gate_sandbox([(rel, old, "")])
        try:
            produced = doc_findings(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_control_characters(audit) -> list[str]:
    """An invisible byte impersonating a regular expression must be rejected."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # The exact 5.5C2a defect: a backspace where a word boundary belongs.
    tmp = Path(tempfile.mkdtemp())
    (tmp / "spec").mkdir()
    (tmp / "spec" / "poisoned.rs").write_bytes(
        b'if re.search(r\'\x08gates?:\\s*"\', src):\n'
    )
    produced = audit.control_character_findings(tmp)
    if not any("forbidden control character U+0008" in f for f in produced):
        fail(f"backspace_in_a_validator_regex_is_rejected (got {produced!r})")
    shutil.rmtree(tmp, ignore_errors=True)

    for name, payload, code in (
        ("nul_byte_is_rejected", b"a\x00b\n", "U+0000"),
        ("vertical_tab_is_rejected", b"a\x0bb\n", "U+000B"),
        ("form_feed_is_rejected", b"a\x0cb\n", "U+000C"),
        ("escape_byte_is_rejected", b"a\x1bb\n", "U+001B"),
        ("carriage_return_is_rejected", b"a\r\nb\n", "U+000D"),
        ("delete_byte_is_rejected", b"a\x7fb\n", "U+007F"),
    ):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "spec").mkdir()
        (tmp / "spec" / "x.rs").write_bytes(payload)
        produced = audit.control_character_findings(tmp)
        if not any(code in f for f in produced):
            fail(f"{name} (wanted {code}, got {produced!r})")
        shutil.rmtree(tmp, ignore_errors=True)

    # LF and TAB are lawful and must not be reported.
    tmp = Path(tempfile.mkdtemp())
    (tmp / "spec").mkdir()
    (tmp / "spec" / "x.rs").write_bytes(b"fn main() {\n\tlet x = 1;\n}\n")
    if audit.control_character_findings(tmp):
        fail("lf_and_tab_are_permitted")
    shutil.rmtree(tmp, ignore_errors=True)

    # The real tree stays clean.
    if audit.control_character_findings(root):
        fail(f"repository_text_is_control_character_clean ({audit.control_character_findings(root)[:3]})")
    return findings


def test_substrate(audit) -> list[str]:
    """Named hostile fixtures for the 5.5D1 substrate primitives.

    Structural only. Bootstrap performs no storage commit, traversal, network
    call, generated dispatch, cancellation, or reconciliation, and these fixtures
    do not claim executable semantics were tested.
    """
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    def probe(name, rel, old, needle, new="", validator=None):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, (validator or doc_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    LEG = "spec/legacy_obligations.rs"

    # --- publication algebra: the three axes stay orthogonal ---------------
    probe("post_commit_result_changed_to_generic_failure_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "committed                        is NOT failed",
          "post-commit publication is not an ordinary failure")
    probe("committed_receipt_incomplete_collapsed_into_failure_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "committed, receipt incomplete    is NOT failed, and is NOT outcome unknown",
          "committed with an incomplete receipt is not failure or unknown")
    probe("outcome_unknown_collapsed_into_not_committed_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "outcome unknown                  is NOT proof of non-commit",
          "outcome unknown is not proof of non-commit")
    probe("cancelled_after_admission_treated_as_non_commit_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "cancelled after admission        is NOT proof of non-commit",
          "cancelled after admission is not proof of non-commit")
    probe("reconciliation_rewriting_durable_evidence_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "It never rewrites the original durable event",
          "reconciliation appends evidence and never rewrites durable history")
    probe("commit_knowledge_axis_removed_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "commit knowledge        KnownAbsent | KnownCommitted | Unknown",
          "commit knowledge is distinct from receipt completeness and reconciliation")
    probe("generic_kind_plus_bytes_publication_made_canonical_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "There is no `GenericPublicationCommand { kind: String, payload: Vec<u8> }`",
          "no generic untyped publication envelope")

    # --- typed batch atomicity --------------------------------------------
    probe("atomic_batch_allowing_partial_durable_publication_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md", "partial durable subset   forbidden",
          "an atomic group never publishes a partial durable subset")
    probe("arbitrary_cross_port_atomic_transaction_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "not an arbitrary distributed transaction across unrelated ports or services",
          "a typed batch is not a distributed transaction")
    probe("batch_receipt_order_decoupled_from_input_order_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md", "Input order and receipt order correspond exactly",
          "input order and receipt order correspond")
    probe("leg037_capability_bound_atomicity_weakened_is_rejected",
          LEG, "only where the admitting capability owns their common authority boundary",
          "LEG-037 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg037_partial_durable_subset_permitted_is_rejected",
          LEG, "never publishes a partial durable subset",
          "LEG-037 law no longer states", validator=audit.d1_substrate_findings)

    # --- logical operation versus physical attempt ------------------------
    probe("logical_operation_identity_and_attemptid_collapsed_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "LogicalOperationId   stable across retries of the same requested operation",
          "logical operation identity is stable across retries")
    probe("attempt_state_and_commit_knowledge_merged_into_one_outcome_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "never collapse into one `Outcome`, boolean, or success/failure field",
          "attempt state, logical outcome, commit knowledge, and receipt completeness stay separate")
    probe("cancelled_before_admission_distinction_removed_is_rejected",
          "docs/09_BVISOR.md", "CancelledBeforeAdmission   no physical execution was admitted",
          "cancellation before admission is distinct")
    probe("leg042_cancellation_split_weakened_is_rejected",
          LEG, "distinguishes cancellation before admission from cancellation after admission",
          "LEG-042 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg042_retry_erasing_prior_attempt_evidence_is_rejected",
          LEG, "receives a new AttemptId without erasing prior attempt evidence",
          "LEG-042 law no longer states", validator=audit.d1_substrate_findings)

    # --- bounded traversal --------------------------------------------------
    probe("cursor_losing_generation_binding_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
          "A cursor binds to its traversal identity, including store lineage, generation",
          "the cursor binds generation")
    probe("cursor_losing_source_cut_binding_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "source cut, direction, selector, and filters",
          "the cursor binds the source cut")
    probe("cursor_transplant_across_selector_accepted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
          "fails closed rather than resuming against a traversal it never described",
          "a transplanted cursor fails closed")
    probe("page_completeness_omitted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "page completeness or partiality posture",
          "the page carries a completeness posture")
    probe("work_observation_omitted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "WorkObservation",
          "the page carries a source stamp and work observation")
    probe("limit_applied_after_decode_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "constrain discovery **before** unbounded decode",
          "the limit applies before decode")
    probe("scan_then_truncate_called_bounded_is_rejected",
          "docs/13_BATQL_CONTRACT.md",
          "scan everything -> decode everything -> truncate the returned vector",
          "scan-then-truncate is not bounded traversal")
    probe("leg028_limit_before_decode_weakened_is_rejected",
          LEG, "constrain discovery before decode",
          "LEG-028 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg029_one_shot_equivalence_weakened_is_rejected",
          LEG, "concatenated valid pages equal a one-shot reference traversal",
          "LEG-029 law no longer states", validator=audit.d1_substrate_findings)

    # --- lowerings stay distinct -------------------------------------------
    probe("get_lowered_to_scan_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "GET           -> bounded seek",
          "GET lowers to a bounded seek")
    probe("children_of_lowered_to_subtree_scan_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "CHILDREN OF   -> bounded one-level traversal",
          "CHILDREN OF lowers to one level")
    probe("scan_under_lowered_to_one_level_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "SCAN UNDER    -> bounded subtree traversal",
          "SCAN UNDER lowers to a bounded subtree")

    # --- generated typed effect routing ------------------------------------
    probe("application_raw_effect_dispatcher_made_canonical_is_rejected",
          "docs/06_MACBAT.md",
          "Applications do not hand-author raw `kind integer + byte payload + switch` dispatch",
          "hand-authored raw dispatch is not the canonical path")
    probe("missing_or_duplicate_generated_route_accepted_is_rejected",
          "docs/06_MACBAT.md", "Generated routing fails closed when a declared variant has no route",
          "generated routing fails closed")
    probe("ambient_route_discovery_accepted_is_rejected",
          "docs/06_MACBAT.md", "never ambient linker registration, startup constructors",
          "route discovery is never ambient")
    probe("leg046_closed_exhaustive_routing_weakened_is_rejected",
          LEG, "closed and exhaustive", "LEG-046 law no longer states",
          validator=audit.d1_substrate_findings)
    probe("leg046_router_absorbing_domain_logic_is_rejected",
          LEG, "owns structure and glue rather than application domain transition logic",
          "LEG-046 law no longer states", validator=audit.d1_substrate_findings)

    # --- absolute monotonic deadline ---------------------------------------
    probe("relative_inactivity_timeout_called_absolute_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md",
          "A relative inactivity timeout is not an overall operation deadline",
          "a relative inactivity timeout is not a deadline")
    probe("retry_resets_overall_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md", "retry does not reset the overall deadline",
          "retry does not reset the overall deadline")
    probe("per_attempt_deadline_exceeding_overall_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md", "remains bounded by the overall operation deadline",
          "a per-attempt deadline stays bounded by the overall deadline")
    probe("lower_repeating_mechanism_outliving_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md",
          "The lowest mechanism capable of internally extending work enforces the remaining absolute deadline",
          "the lowest extending mechanism enforces the remaining deadline")
    probe("leg066_deadline_meaning_removed_is_rejected",
          LEG, "a relative inactivity timeout never substitutes for an absolute monotonic deadline",
          "LEG-066 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg066_retry_resetting_deadline_permitted_is_rejected",
          LEG, "a retry never resets the overall deadline",
          "LEG-066 law no longer states", validator=audit.d1_substrate_findings)

    return findings


def main() -> int:
    freeze = load("freeze")
    audit = load("audit")
    project = load("project")
    findings: list[str] = []
    findings += test_manifest_ordering(freeze)
    findings += test_casefold_collision(audit)
    findings += test_stale_vocabulary(audit)
    findings += test_stale_derivation(audit, HERE.parent)
    findings += test_legacy_manifest_parity(audit)
    findings += test_batql(audit, project)
    findings += test_numeric(audit)
    findings += test_guarantees(audit, project)
    findings += test_gates(audit, project)
    findings += test_decisions(audit, project)
    findings += test_authenticated_history(audit)
    findings += test_document_law(audit)
    findings += test_claim_receipt_law(audit)
    findings += test_delivery_notes(audit)
    findings += test_substrate(audit)
    findings += test_control_characters(audit)
    if findings:
        print(f"selftest: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("selftest: PASS (portability + stale-vocabulary + BatQL + numeric + guarantee + gate + decision + authenticated-history + control-character + substrate hostile fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

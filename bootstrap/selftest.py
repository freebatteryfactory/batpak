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
import contextlib
import hashlib
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
    """A repo copy with guarded edits applied; returns the sandbox root.

    The canonical tracked workspace is never the mutation target (5.5D4a).
    Callers discard the copy in a finally block; `isolated_tree` is the
    context-managed form that cleans up on every exit path.
    """
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
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
    if len([n for n in nodes if n["family"] == "DEC"]) != 74:
        fail("decision node count is not 74")

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


def test_specialization(audit, project) -> list[str]:
    """Named hostile fixtures for adaptive residual specialization (DEC-073).

    Structural only. Bootstrap does not execute PakVM, specialize a program, run
    a scan, measure a benchmark, test SIMD, compare residual results, or validate
    machine code, and these fixtures claim none of that.
    """
    findings: list[str] = []
    root = HERE.parent

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

    ISA = "docs/07_PAKVM_ISA.md"
    ECS = "docs/18_DATA_ORIENTED_ECS.md"
    DISP = "spec/dispositions.rs"

    # --- DEC-073 identity ---------------------------------------------------
    if audit.specialization_findings(root) or audit.specialization_key_findings(root):
        fail("dec073_contract_passes")
    probe("dec073_wrong_class_is_rejected", DISP,
          'id: "DEC-073", class: DecisionClass::Capability',
          "DEC-073 class Enforcement is not Capability",
          'id: "DEC-073", class: DecisionClass::Enforcement',
          validator=audit.specialization_findings)
    probe("dec073_wrong_gates_are_rejected", DISP,
          "gates: &[GateId::G4, GateId::G5, GateId::G8, GateId::G9], disposition: Disposition::Lock, "
          'subject: "Adaptive residual specialization"',
          "!= [G4, G5, G8, G9]",
          "gates: &[GateId::G4], disposition: Disposition::Lock, "
          'subject: "Adaptive residual specialization"',
          validator=audit.specialization_findings)

    # --- one probe per key component: a broad marker would prove several at once
    for component in audit.D2_KEY_COMPONENTS:
        probe(f"specialization_key_missing_{component.split()[0].lower()}_is_rejected",
              ISA, component, f"specialization key omits {component}",
              validator=audit.specialization_key_findings)

    # --- reference authority ------------------------------------------------
    probe("reference_interpreter_demoted_from_semantic_authority_is_rejected",
          ISA, "The reference interpreter is the executable semantic authority for admitted programs",
          "the reference interpreter is the executable semantic authority")
    probe("specialized_plan_made_program_authority_is_rejected",
          ISA, "It is never program authority, canonical source, a new `ProgramImage`",
          "SpecializedPlan is derived, never authority")
    probe("residual_made_sole_oracle_is_rejected",
          ISA, "the residual path is never the sole oracle", "the residual is never the sole oracle")
    probe("plan_reuse_under_a_different_key_permitted_is_rejected",
          ISA, "SpecializedPlan reused under a different key", "reuse under a different key is illegal")

    # --- cache and fallback -------------------------------------------------
    probe("cache_deletion_permitted_to_change_meaning_is_rejected",
          ISA, "Deleting every specialization cache changes performance and `WorkObservation` only",
          "cache deletion changes performance only")
    probe("cache_key_mismatch_accepted_is_rejected",
          ISA, "it discards, refuses reuse, and falls back to reference execution",
          "cache mismatch discards and falls back")
    probe("corrupt_residual_reused_is_rejected",
          ISA, "An old residual is never reinterpreted under a new key",
          "an old residual is never reinterpreted under a new key")
    probe("specialization_failure_without_reference_fallback_is_rejected",
          ISA, "because reference execution remains complete",
          "reference execution remains complete without a cache")

    # --- bounded first use --------------------------------------------------
    probe("first_use_specialization_without_budget_or_deadline_is_rejected",
          ISA, "an explicit work budget, an absolute monotonic deadline, a bounded memory posture",
          "first use is budgeted and deadline-bound")
    probe("specialization_attempt_resets_logical_deadline_is_rejected",
          ISA, "The specialization attempt never resets the logical operation deadline",
          "specialization never resets the logical deadline")

    # --- safety boundary ----------------------------------------------------
    probe("specializer_executes_an_effect_is_rejected",
          ISA, "It may not pre-execute durable effects, port effects, wall-clock reads",
          "specialization pre-executes no effects")
    probe("specializer_bypasses_bvisor_admission_is_rejected",
          ISA, "It may not eliminate or bypass Bvisor admission",
          "specialization cannot bypass Bvisor admission")
    probe("specializer_mints_a_capability_is_rejected",
          ISA, "PakVM specialization cannot mint capabilities.", "specialization cannot mint capabilities")
    probe("specializer_advances_a_durable_checkpoint_is_rejected",
          ISA, "PakVM specialization cannot advance a durable checkpoint.",
          "specialization cannot advance a durable checkpoint")
    probe("specializer_authorizes_a_physical_effect_is_rejected",
          ISA, "PakVM specialization cannot authorize a physical effect.",
          "specialization cannot authorize a physical effect")
    probe("check_discharged_because_it_usually_passes_is_rejected",
          ISA, "A check is not eliminated merely because the current implementation usually passes it",
          "a check is not discharged because it usually passes")
    probe("dynamic_fact_bound_as_static_is_rejected",
          ISA, "wall-clock time, file descriptor numbers, socket state",
          "dynamic facts are not bound as static")
    probe("key_component_silently_omitted_is_rejected",
          ISA, "It may never be silently omitted", "a key component is never silently omitted")

    # --- V1 requirement boundary -------------------------------------------
    probe("native_jit_or_executable_memory_made_a_v1_requirement_is_rejected",
          ISA, "machine-code JIT, executable-memory allocation, self-modifying code",
          "no machine-code JIT or executable memory in V1")
    probe("nightly_unsafe_or_simd_made_mandatory_is_rejected",
          ECS, "V1 does not require nightly Rust, unsafe target intrinsics, a portable-SIMD dependency",
          "nightly, unsafe intrinsics, and SIMD are not required")

    # --- result parity ------------------------------------------------------
    probe("specialized_execution_omits_proof_parity_is_rejected",
          ISA, "preserve semantic value, `Availability`, `Truth`, `Decision`, `Completeness`, `ProofDisposition`",
          "result axes are preserved across paths")
    probe("faster_path_emitting_less_proof_is_rejected",
          ISA, "A faster path never emits less proof, explanation, or receipt material",
          "a faster path emits no less proof")
    probe("work_observation_execution_path_omitted_is_rejected",
          ISA, "FallbackAfterSpecializationRefusal", "WorkObservation names the execution path")

    # --- scalar seam --------------------------------------------------------
    probe("scalar_scan_kernel_removed_is_rejected",
          ECS, "The scalar `ScanKernel` is mandatory and is the reference behavior",
          "the scalar scan kernel is the reference behavior")
    probe("optimized_kernel_made_sole_oracle_is_rejected",
          ECS, "No optimized kernel is ever the sole oracle", "no optimized kernel is the sole oracle")
    probe("optimized_kernel_accepted_without_qualification_is_rejected",
          ECS, "An unavailable optimized kernel falls back to scalar reference behavior",
          "an optimized kernel requires qualification and fallback")
    probe("selection_mask_frozen_to_64_rows_is_rejected",
          ECS, "The physical representation is not frozen", "the mask width is not frozen")
    probe("aosoa64_made_canonical_is_rejected",
          ECS, "Nothing here constitutionalizes `u64`, 64 rows, `AoSoA64`, or one tile width forever",
          "AoSoA64 and fixed 64-row masks are not constitutional")
    probe("mask_ops_across_row_domains_is_rejected",
          ECS, "legal only for masks over the same row domain, logical length, source cut",
          "mask algebra is domain bound")
    probe("mask_not_selecting_unused_high_bits_is_rejected",
          ECS, "unused high bits cannot select nonexistent rows", "NOT is bounded by the logical row length")
    probe("column_tile_made_authority_is_rejected",
          ECS, "`ColumnTile` remains an earned physical derived layout", "ColumnTile remains derived")
    probe("late_materialization_reorders_rows_is_rejected",
          ECS, "preserving source row order, row identity", "late materialization preserves row order")
    probe("unbounded_materialization_before_selection_is_rejected",
          ECS, "It may not materialize an unbounded result and then claim the mask saved work",
          "no unbounded materialization before selection")
    probe("work_observation_omitted_from_scan_seam_is_rejected",
          ECS, "the frozen obligation is that work remains observable", "WorkObservation stays observable")

    # --- graph conservation -------------------------------------------------
    nodes, edges = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-073" not in ids:
        fail("guarantee_graph_missing_dec073")
    for synthetic in ("SpecializedPlan", "SelectionMask", "ScanKernel", "Column", "ColumnTile",
                      "SpecializationKey", "WorkObservation"):
        if synthetic in ids:
            fail(f"synthetic specialization node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 74:
        fail("decision_node_count_is_not_74")
    if len(nodes) != 197 or len(edges) != 9:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    return findings


def test_delivery_notes_d2(audit) -> list[str]:
    """The delivery inventory tracks the live decision count."""
    findings: list[str] = []
    text = (HERE.parent / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    if "73 architectural decision/disposition rows" in text:
        findings.append("delivery_notes_remaining_at_73_decisions FAILED")
    if "74 architectural decision/disposition rows" not in text:
        findings.append("delivery_notes_states_74_decisions FAILED")
    return findings


def test_proof_policy(audit) -> list[str]:
    """Named hostile fixtures for mutation lanes and proof-policy anti-weakening.

    Structural only. Bootstrap compiles no mutant, activates no slot, runs no
    nextest, invokes no rustc, compares no cargo-mutants run, kills nothing,
    proves no equivalence, promotes nothing, and classifies no real diff
    semantically. These fixtures claim none of that.
    """
    findings: list[str] = []
    root = HERE.parent

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

    ARCH = "spec/architecture.rs"
    DISP = "spec/dispositions.rs"
    TP = "docs/12_TESTPAK.md"
    GA = "docs/24_GAUNTLET.md"
    pp = audit.proof_policy_findings

    if pp(root):
        fail("proof_policy_contract_passes")

    # --- DEC-074 identity ---------------------------------------------------
    probe("dec074_missing_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement',
          "DEC-074 is absent", 'id: "DEC-0740", class: DecisionClass::Enforcement', validator=pp)
    probe("dec074_wrong_class_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement',
          "is not Enforcement", 'id: "DEC-074", class: DecisionClass::Naming', validator=pp)
    probe("dec074_missing_g3_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3, GateId::G9]',
          "DEC-074 does not name gate G3",
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G9]', validator=pp)
    probe("dec074_missing_g9_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3, GateId::G9]',
          "DEC-074 does not name gate G9",
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3]', validator=pp)
    probe("dec015_losing_testpak_confinement_is_rejected", DISP,
          "Mutation testing only inside TestPak", "no longer confines mutation testing to TestPak",
          "Mutation testing anywhere", validator=pp)

    # --- lane and result vocabularies ---------------------------------------
    probe("fourth_mutation_lane_is_rejected", ARCH,
          "    CompilerBacked,\n}", "MutationLane variants", "    CompilerBacked,\n    NativeLane,\n}",
          validator=pp)
    probe("removed_mutation_lane_is_rejected", ARCH,
          "    /// Implementation-sensitive material whose truth depends on real compiler\n"
          "    /// and platform behavior. Runs under real rustc semantics.\n    CompilerBacked,\n",
          "MutationLane variants", "", validator=pp)
    for variant in ("NotActivated", "Refused", "TimedOut", "InfrastructureFailure", "EquivalentCandidate"):
        probe(f"mutation_result_{variant.lower()}_removed_is_rejected", ARCH,
              f"    {variant},\n", "MutationResult variants", "", validator=pp)
    probe("never_killed_dropping_unbuildable_is_rejected", ARCH,
          "pub const NEVER_KILLED: &[MutationResult] = &[\n    MutationResult::NotActivated,",
          "NEVER_KILLED", "pub const NEVER_KILLED: &[MutationResult] = &[", validator=pp)
    probe("never_survived_dropping_notactivated_is_rejected", ARCH,
          "pub const NEVER_SURVIVED: &[MutationResult] = &[\n    MutationResult::NotActivated,",
          "NEVER_SURVIVED", "pub const NEVER_SURVIVED: &[MutationResult] = &[", validator=pp)
    probe("denominator_dropping_a_terminal_result_is_rejected", ARCH,
          "pub const TERMINAL_MUTATION_RESULTS: &[MutationResult] = &[\n    MutationResult::Killed,",
          "!= all eight categories",
          "pub const TERMINAL_MUTATION_RESULTS: &[MutationResult] = &[", validator=pp)
    probe("change_class_collapsed_is_rejected", ARCH,
          "    Neutral,\n", "ProofPolicyChangeClass variants", "", validator=pp)
    probe("proof_policy_surface_removed_is_rejected", ARCH,
          "    WaiverLogic,\n", "ProofPolicySurface variants", "", validator=pp)

    # --- candidate and promotion boundary -----------------------------------
    for rootdir in ("src/", "tests/", "spec/", "docs/", "companion/"):
        probe(f"candidate_write_into_{rootdir.strip('/')}_permitted_is_rejected", ARCH,
              f'"{rootdir}"', f"not forbidden from writing {rootdir}", '"__none__"', validator=pp)
    probe("candidate_root_moved_into_tracked_source_is_rejected", ARCH,
          "target/muterprater/candidates/", "no candidate output root is declared", "tests/generated/",
          validator=pp)
    for needed, token in (("oracle", "independent evidence or oracle identity"),
                          ("named invariant", "named invariant, guarantee, obligation, or documented proof gap"),
                          ("killed real semantic mutant", "killed real semantic mutant or equivalent hostile evidence"),
                          ("receipt", "auditable proof and promotion receipt")):
        probe(f"promotion_without_{needed.split()[0]}_is_rejected", ARCH,
              f'    "{token}",\n', f"promotion requirements omit {needed}", "", validator=pp)

    # --- documentary law ----------------------------------------------------
    for name, rel, old, needle in (
        ("survived_without_activation_is_rejected", TP,
         "A `Survived` verdict without activation evidence is invalid", "Survived requires activation"),
        ("killed_without_baseline_is_rejected", TP,
         "A `Killed` verdict without a qualified baseline and activation evidence is invalid",
         "Killed requires baseline and activation"),
        ("timeout_counted_as_kill_is_rejected", TP, "A timeout is not a kill", "a timeout is not a kill"),
        ("unbuildable_counted_as_kill_is_rejected", TP,
         "a compiler error produced by an invalid mutation is not evidence",
         "an unbuildable candidate is not a kill"),
        ("equivalent_candidate_without_witness_is_rejected", TP,
         "a classification pending its witness, never equivalence proof",
         "EquivalentCandidate is not equivalence proof"),
        ("denominator_silently_shrunk_is_rejected", TP,
         "Nothing silently leaves the denominator", "nothing leaves the denominator silently"),
        ("score_without_distribution_is_rejected", TP,
         "A mutation score exposes the full result distribution",
         "the full result distribution is published"),
        ("lane_a_requiring_rustc_is_rejected", TP, "No per-candidate Rust compile. The reference interpreter",
         "Lane A needs no per-candidate Rust compile"),
        ("lane_b_slots_in_production_is_rejected", TP, "ordinary production artifact. A release artifact",
         "Lane B slots are test-profile only"),
        ("lane_b_without_activation_is_rejected", TP, "Activation is evidence, never an assumption",
         "Lane B records activation"),
        ("lane_c_replacing_rustc_is_rejected", TP, "never claims a homegrown Rust evaluator equals rustc",
         "no homegrown evaluator equals rustc"),
        ("nextest_as_semantic_authority_is_rejected", TP, "Nextest executes; it is never semantic authority",
         "nextest executes and is not semantic authority"),
        ("differential_tool_retired_by_prose_is_rejected", TP, "No coronation by architecture prose",
         "compiler-backed tooling is not retired by prose"),
        ("muterprater_as_standalone_product_is_rejected", TP,
         "It is not a standalone product package, a second semantic authority",
         "Muterprater is not a standalone product"),
        ("production_code_as_own_oracle_is_rejected", TP,
         "passing production evaluator against itself -> not an independent oracle",
         "self-calling production code is not an oracle"),
        ("second_pakvm_mandated_is_rejected", TP,
         "Building a second full PakVM merely to call it independent is not independence",
         "no second full PakVM is required for independence"),
        ("direct_tracked_write_path_is_rejected", TP,
         "It may not write candidates directly into `src/`, `tests/`, `spec/`, `docs/`, or `companion/`",
         "candidates land outside tracked source"),
        ("generation_and_promotion_merged_is_rejected", TP,
         "Generation and promotion are two different operations with two different receipts",
         "generation and promotion are separate"),
        ("unclassified_policy_change_accepted_is_rejected", TP,
         "An unclassified proof-policy change is refused",
         "an unclassified proof-policy change is refused"),
        ("neutral_without_parity_is_rejected", TP,
         "and documentary and receipt meaning. Requires parity evidence:",
         "Neutral requires parity evidence"),
        ("narrowed_domain_escaping_weakening_is_rejected", TP,
         "narrows the tested domain or silently removes denominator units is Weakening",
         "a narrowed domain is Weakening"),
        ("weakening_without_hostile_qualification_is_rejected", TP,
         "hostile qualification proving the new boundary is enforced",
         "a Weakening carries the full authority package"),
        ("issue_link_as_authority_is_rejected", TP, "A generic issue link is not decision authority",
         "an issue link is not authority"),
        ("commit_message_as_receipt_is_rejected", TP, "A commit message is not a weakening receipt",
         "a commit message is not a receipt"),
        ("gate_disabling_itself_as_cleanup_is_rejected", TP,
         "A proof-policy gate that can be disabled as unclassified cleanup is not an anti-weakening gate",
         "a disableable gate is not a gate"),
        ("bootstrap_claiming_diff_inference_is_rejected", TP,
         "Bootstrap does not understand arbitrary semantic diffs and never claims to",
         "bootstrap does not infer semantic diffs"),
        ("d1_publication_proof_ownership_removed_is_rejected", GA,
         "committed is never reported as an ordinary operation failure",
         "D1 publication proof family is owned"),
        ("d1_attempt_proof_ownership_removed_is_rejected", GA,
         "CancelledAfterAdmission does not prove non-commit", "D1 attempt proof family is owned"),
        ("d1_traversal_proof_ownership_removed_is_rejected", GA,
         "valid pages concatenate to the one-shot reference traversal",
         "D1 traversal proof family is owned"),
        ("d1_routing_proof_ownership_removed_is_rejected", GA,
         "every declared route is present exactly once", "D1 routing proof family is owned"),
        ("d1_deadline_proof_ownership_removed_is_rejected", GA,
         "retry does not reset the overall deadline", "D1 deadline proof family is owned"),
        ("d2_specialization_square_removed_is_rejected", GA,
         "Specialization proof square (DEC-073)", "the D2 specialization square is assigned"),
        ("one_lane_made_sole_proof_route_is_rejected", GA, "No lane is the sole proof route",
         "no lane is the sole proof route"),
    ):
        probe(name, rel, old, needle)

    # --- graph conservation -------------------------------------------------
    nodes, edges = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-074" not in ids:
        fail("guarantee_graph_missing_dec074")
    for synthetic in ("SemanticIr", "SelectableCompiled", "CompilerBacked", "MutationLane",
                      "MutationResult", "ProofPolicySurface", "Killed", "Survived"):
        if synthetic in ids:
            fail(f"synthetic mutation node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 74:
        fail("decision_node_count_is_not_74")
    if len(nodes) != 197 or len(edges) != 9:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    text = (root / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    if "74 architectural decision/disposition rows" not in text:
        fail("delivery_notes_states_74_decisions")
    return findings


# --- Hostile-probe isolation (5.5D4a) ---------------------------------------
# A probe timeout once left a disabled rule inside the canonical bootstrap/audit.py.
# Manual restoration worked, but a guard that depends on a later cleanup step is a
# guard that can be left asleep with its eyes painted open. The canonical tracked
# workspace is now never a mutation target: probes mutate an isolated copy, the
# copy is discarded on every exit path including timeout, exception, and
# cancellation, and the suite proves canonical validator bytes are unchanged.
CANONICAL_VALIDATORS = ("audit.py", "project.py", "freeze.py", "selftest.py")


def _commit(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_commitments() -> dict[str, str]:
    """Content commitments for every canonical validator, before any probe runs."""
    return {name: _commit(HERE / name) for name in CANONICAL_VALIDATORS if (HERE / name).is_file()}


def canonical_drift(before: dict[str, str]) -> list[str]:
    """Any canonical validator whose bytes moved is a residue finding, not a pass."""
    out = []
    for name, want in sorted(before.items()):
        got = _commit(HERE / name)
        if got != want:
            out.append(f"canonical validator {name} was modified by the hostile-probe suite")
    return out


@contextlib.contextmanager
def isolated_tree(subdirs=("spec", "docs", "companion")):
    """An isolated copy of the tracked material. Discarded on EVERY exit path:
    success, exception, timeout, assertion failure, or cancellation."""
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
    try:
        for sub in subdirs:
            src = root / sub
            if src.is_dir():
                shutil.copytree(src, tmp / sub)
        yield tmp
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


@contextlib.contextmanager
def neutered_validator(name: str, target: str, replacement: str = "if False:"):
    """Load a COPY of a canonical validator with one rule neutered.

    This replaces the retired pattern of patching bootstrap/audit.py in place and
    restoring afterwards. The canonical file is never opened for writing, so a
    timeout mid-probe cannot leave a disabled rule behind.
    """
    canonical = HERE / f"{name}.py"
    before = _commit(canonical)
    source = canonical.read_text(encoding="utf-8")
    if target not in source:
        raise ProbeError(f"probe target absent in {name}.py: {target!r}")
    mutated = source.replace(target, replacement, 1)
    if mutated == source:
        raise ProbeError(f"probe mutation changed no bytes in {name}.py: {target!r}")
    tmp = Path(tempfile.mkdtemp(prefix="batpak-neuter-"))
    try:
        path = tmp / f"{name}.py"
        path.write_text(mutated, encoding="utf-8")
        spec = importlib.util.spec_from_file_location(f"{name}_neutered", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        yield module
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
        if _commit(canonical) != before:
            raise ProbeError(f"canonical {name}.py was modified during a probe")


def test_probe_harness(audit) -> list[str]:
    """The harness cannot leave residue in the canonical workspace (5.5D4a)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    before = canonical_commitments()

    # 1 & 2: an exception or an abort mid-probe still cleans up, and the
    # canonical validator is untouched because it was never the target.
    leaked: list[Path] = []
    try:
        with isolated_tree() as tmp:
            leaked.append(tmp)
            raise RuntimeError("simulated probe explosion")
    except RuntimeError:
        pass
    if leaked and leaked[0].exists():
        fail("probe_exception_cannot_leave_validator_modified")
    if canonical_drift(before):
        fail("probe_exception_cannot_leave_validator_modified")

    leaked = []
    try:
        with isolated_tree() as tmp:
            leaked.append(tmp)
            # a timeout or cancellation surfaces as a BaseException, not Exception
            raise KeyboardInterrupt("simulated probe timeout/abort")
    except KeyboardInterrupt:
        pass
    if leaked and leaked[0].exists():
        fail("probe_abort_cannot_leave_validator_modified")
    if canonical_drift(before):
        fail("probe_abort_cannot_leave_validator_modified")

    # A neutered validator runs from a copy; the canonical file never moves.
    with neutered_validator("audit", 'if dec["class"] != "Capability":') as neutered:
        if neutered.specialization_findings(root):
            fail("neutered_validator_runs_from_a_copy")
    if canonical_drift(before):
        fail("neutered_validator_cannot_modify_canonical_bytes")

    # An abort inside a neutered probe still leaves canonical bytes intact.
    try:
        with neutered_validator("audit", 'if dec["class"] != "Capability":'):
            raise KeyboardInterrupt("simulated timeout while a rule is disabled")
    except KeyboardInterrupt:
        pass
    if canonical_drift(before):
        fail("probe_abort_cannot_leave_validator_modified")

    # 3 & 4: the harness fails closed on a broken probe rather than passing.
    try:
        with neutered_validator("audit", "zzz-absent-target-zzz"):
            pass
    except ProbeError:
        pass
    else:
        fail("absent_probe_target_is_rejected")
    try:
        must_replace("abc", "abc", "abc", "self-check")
    except ProbeError:
        pass
    else:
        fail("inert_probe_mutation_is_rejected")

    # 5: a probe that fires the WRONG diagnostic is not a pass. A red light from
    # the wrong severed wire is not evidence that the alarm works.
    with neutered_validator("audit", 'if dec["class"] != "Capability":') as neutered:
        produced = neutered.decision_class_findings(root)
        if any("unknown DecisionClass" in f for f in produced):
            fail("wrong_diagnostic_class_is_rejected")

    # 6: the canonical workspace is never a probe target.
    with isolated_tree() as tmp:
        if tmp == root or str(tmp).startswith(str(root)):
            fail("canonical_workspace_is_not_a_probe_target")
        (tmp / "spec" / "gates.rs").write_text("poisoned", encoding="utf-8")
        if (root / "spec" / "gates.rs").read_text(encoding="utf-8") == "poisoned":
            fail("canonical_workspace_is_not_a_probe_target")
    if canonical_drift(before):
        fail("canonical_workspace_is_not_a_probe_target")

    findings.extend(canonical_drift(before))
    return findings


def main() -> int:
    freeze = load("freeze")
    audit = load("audit")
    project = load("project")
    # Canonical validator bytes are committed before any hostile probe runs and
    # proven unchanged after the suite. A probe that dies mid-mutation cannot
    # leave a disabled rule behind and still report PASS.
    canonical_before = canonical_commitments()
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
    findings += test_substrate(audit)
    findings += test_specialization(audit, project)
    findings += test_delivery_notes_d2(audit)
    findings += test_proof_policy(audit)
    findings += test_probe_harness(audit)
    findings += canonical_drift(canonical_before)
    findings += test_control_characters(audit)
    if findings:
        print(f"selftest: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("selftest: PASS (portability + stale-vocabulary + BatQL + numeric + guarantee + gate + decision + authenticated-history + control-character + substrate + specialization + proof-policy + probe-isolation hostile fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

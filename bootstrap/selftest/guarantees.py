"""Numeric authority, guarantee classification, and guarantee-admission
authority hostile fixtures."""
from __future__ import annotations
from .core import (
    HERE,
    canonical_commitments,
    canonical_drift,
    isolated_tree,
    must_replace,
)


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
        "id": "OP-ADD", "class": "Arithmetic", "shape": "SymbolOnly",
        "canonical": "+", "alias": "",
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix", "precedence": "60",
        "associativity": "Left", "input_sorts": "a", "result_sort": "b", "exactness": "Exact",
        "overflow": "o", "exception": "e", "spoken": "plus",
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

    def node(nid, family="SEED", kind="SemanticLaw", life="Permanent", owner="o", gates="",
             witness="RowDeclared"):
        # Post-5.5D4b node shape: an admitted view carries a typed gate posture and
        # a witness posture, never a bare gate string plus free witness text.
        return {"id": nid, "family": family, "kind": kind, "lifetime": life,
                "owner": owner, "gate_posture": gates, "target": "", "witness": witness}

    good = [seed(f"SEED-{i}") for i in range(32)]
    if audit.guarantee_classification_findings(good):
        fail("valid_classification_passes")
    if not audit.guarantee_classification_findings(good[:31]):
        fail("missing_seed_row_is_rejected")
    if not audit.guarantee_classification_findings(good + [seed("SEED-32")]):
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
               for x in audit.guarantee_lifetime_findings([node("SEED-X", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_semanticlaw_without_witness_is_rejected")
    if not any("Permanent ArchitectureConstraint names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("A", kind="ArchitectureConstraint", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_architecture_without_witness_is_rejected")
    if audit.guarantee_lifetime_findings([node("D", kind="Decision", witness="NoFamilyWitness")], {}, []):
        fail("permanent_decision_witness_deferred_to_c2")
    if audit.guarantee_lifetime_findings([node("Q", kind="QualificationRequirement", witness="NoFamilyWitness")], {}, []):
        fail("permanent_qualification_is_self_witnessing")
    if not any("UntilGate names no active gate schedule" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilGate", gates="")], {}, [])):
        fail("untilgate_without_gate_is_rejected")
    if not any("HistoricalCoverageOnly cannot be actively scheduled" in x
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
    def shape(n):
        return (n["id"], n["family"], n["kind"], n["lifetime"], n["owner"],
                n["gate_posture"], n["target"], n["witness"])

    proj_nodes = [shape(n) for n in project.guarantee_nodes(root)]
    audit_nodes = [shape(n) for n in audit.guarantee_derive(root)[0]]
    if proj_nodes != audit_nodes:
        fail("generator_auditor_guarantee_disagreement")
    nodes, edges, _adm = audit.guarantee_derive(root)
    if nodes != sorted(nodes, key=lambda n: (audit.G_FAMILY_RANK[n["family"]], n["id"])):
        fail("source_reorder_changes_guarantee_order")
    if any(not src.startswith("SEED-") for src, _k, _t in edges):
        fail("non_seed_source_guarantee_edge_present")

    return findings


def test_guarantee_authority(audit, project) -> list[str]:
    """Guarantee admission and projection authority (5.5D4b closure).

    These are the adversaries that would have caught the C1 defect. The old suite
    attacked mutation and parity, and both are blind to common-mode invention: two
    scripts carrying the same unauthorized constant agree perfectly.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    GU = "spec/guarantees.rs"
    DI = "spec/dispositions.rs"
    AR = "spec/architecture.rs"

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new,
                                         f"{rel}: {old[:44]!r}"), encoding="utf-8")
            expect(name, (validator or audit.guarantee_admission_findings)(tmp), needle)

    if audit.guarantee_admission_findings(root):
        fail("every_node_passes_typed_admission")

    # 1. No semantic literal may live in both bootstrap paths. A constant in two
    #    "independent" scripts is common-mode invention, not evidence.
    import ast as _ast

    def lits(rel):
        tree = _ast.parse((HERE / rel).read_text(encoding="utf-8"))
        return {n.value for n in _ast.walk(tree)
                if isinstance(n, _ast.Constant) and isinstance(n.value, str)}

    shared = lits("project.py") & lits("audit.py")
    banned = {"Permanent", "docs/30_DECISION_AND_REJECTION_LEDGER.md", "spec/architecture.rs",
              "LegacyObligation", "Decision", "ArchitectureConstraint", "QualificationRequirement",
              "spec/architecture.rs; audit.py architecture checks"}
    pj = (HERE / "project.py").read_text(encoding="utf-8")
    au = (HERE / "audit.py").read_text(encoding="utf-8")
    for lit in sorted(shared & banned):
        for dim in ("lifetime", "owner", "kind", "gates", "witness"):
            asn = f'"{dim}": "{lit}"'
            if asn in pj and asn in au:
                fail(f"common_mode_semantic_constant_{dim}_{lit[:24]}_present_in_both_paths")

    # 2. Every family policy is declared; nothing is left to a projector default.
    pol = audit.guarantee_family_policies(root)
    if set(pol) != {"SEED", "LEG", "DEC", "ARCH", "QUAL"}:
        fail(f"all_five_families_declare_typed_policy (got {sorted(pol)})")
    if project.family_policies(root) != pol:
        fail("generator_and_auditor_read_the_same_typed_family_policy")
    if audit.decision_lifetime_map(root) != project.disposition_lifetimes(root):
        fail("generator_and_auditor_read_the_same_typed_disposition_map")

    # 3. The DEC lifetime map is owned by the typed source, not by either script.
    dm = audit.decision_lifetime_map(root)
    for disp, life in (("Lock", "Permanent"), ("Keep", "Permanent"), ("Kill", "Permanent"),
                       ("Defer", "Permanent"), ("OpenImplementation", "UntilGate"),
                       ("Demote", "UntilCompatibilityExpiry"),
                       ("Supersede", "HistoricalCoverageOnly"),
                       ("RetainAsEvidence", "HistoricalCoverageOnly")):
        if dm.get(disp) != life:
            fail(f"disposition_{disp}_maps_to_{life} (got {dm.get(disp)!r})")
    probe("illegal_disposition_to_lifetime_mapping_is_rejected", DI,
          "Disposition::OpenImplementation => GuaranteeLifetime::UntilGate,",
          "has no typed lifetime", "", validator=audit.guarantee_admission_findings)

    # 4. A projector may not supply a semantic default when policy is absent.
    probe("family_policy_removal_fails_closed", GU,
          '        family: "ARCH",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),',
          "no declared family policy for ARCH",
          '        family: "ARCH-RENAMED",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),')
    probe("family_constant_without_a_value_fails_closed", GU,
          'owner: OwnerRule::FamilyConstant("docs/30_DECISION_AND_REJECTION_LEDGER.md"),',
          "owner is FamilyConstant but names no value",
          'owner: OwnerRule::FamilyConstant(""),')

    # 5. A qualification target may never enter a gate field.
    probe("qualification_target_in_a_gate_field_is_rejected", AR,
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::G0, GateId::G5],",
          "names a value that is not a declared GateId",
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::std],")
    if any(audit.guarantee_gates(root, f"QUAL-{p}") in ("std", "no_std + alloc", "wasm32 host")
           for p in ("batpak-native", "batpak-semantic", "syncbat-browser")):
        fail("qualification_target_cannot_be_read_as_a_gate_posture")

    # 6. Missing metadata is not gate-independence, and neither is an empty list.
    probe("qual_row_without_a_typed_schedule_fails_closed", AR,
          "        gates: &[GateId::G0, GateId::G5],\n        requirement: \"contracts, schemas",
          "gate posture is RowDeclared but the row declares none",
          "        gates: &[],\n        requirement: \"contracts, schemas")
    probe("dec_requiring_a_gate_without_one_fails_closed", DI,
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[GateId::G2]',
          "requires a gate but the row declares none",
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[]')
    # An ungated HistoricalReceipt is GateIndependent by an AUTHORED class claim.
    idx = {n["id"]: n for n in audit.guarantee_derive(root)[0]}
    if idx["DEC-001"]["gate_posture"] != "GateIndependent":
        fail("ungated_historical_receipt_is_gate_independent_by_class")
    if idx["DEC-002"]["lifetime"] != "HistoricalCoverageOnly":
        fail("retain_as_evidence_is_not_projected_as_permanent")
    if idx["DEC-005"]["gate_posture"] != "historical:G0":
        fail("requires_gate_is_a_floor_not_a_ceiling")
    # A retired decision keeps its authored GateIds as provenance, and provenance
    # is never an active schedule. No family is exempt from the invariant.
    for did, want in (("DEC-009", "historical:G0"), ("DEC-010", "historical:G5")):
        if idx[did]["gate_posture"] != want:
            fail(f"{did}_retains_its_gate_reference_as_historical_association")
    def hist_node(posture):
        return {"id": "D", "family": "DEC", "kind": "Decision",
                "lifetime": "HistoricalCoverageOnly", "owner": "o",
                "gate_posture": posture, "target": "", "witness": "NoFamilyWitness"}

    if audit.guarantee_lifetime_findings([hist_node("historical:G0")], {}, []):
        fail("historical_association_is_not_an_active_schedule")
    if not any("cannot be actively scheduled" in x
               for x in audit.guarantee_lifetime_findings([hist_node("G0")], {}, [])):
        fail("historical_lifetime_with_an_active_schedule_is_rejected_for_every_family")

    # 7. ARCH loses its declared G0 schedule.
    probe("arch_row_losing_g0_scheduling_is_rejected", GU,
          "gate_posture: GatePostureRule::FamilyConstant(GatePosture::Scheduled(&[GateId::G0])),",
          "gate posture is RowDeclared but the row declares none",
          "gate_posture: GatePostureRule::RowDeclared,")

    # 7b. Relations are typed constructors (5.5E2): the auditor refuses a bare
    # string, a mistagged family, and an undeclared constructor. Existence of
    # the referenced row is seedcheck's executed law, proven in the Tier 0
    # fixtures with a dangling reference.
    IN = "spec/invariants.rs"

    def rel_validator(tmp):
        return audit.guarantee_typed_relation_findings(audit.guarantee_seed_rows(tmp))

    probe("bare_string_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          "not a typed GuaranteeRef constructor",
          'derives_from: &["DEC-068"]', validator=rel_validator)
    probe("mistagged_relation_family_is_rejected", IN,
          'refines: &[GuaranteeRef::leg("LEG-080")]',
          "whose family owns the LEG- prefix",
          'refines: &[GuaranteeRef::leg("DEC-080")]', validator=rel_validator)
    probe("undeclared_relation_constructor_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-065")]',
          "undeclared relation constructor GuaranteeRef::arch",
          'derives_from: &[GuaranteeRef::arch("DEC-065")]', validator=rel_validator)

    # 7c. Witness citations are typed and resolve (5.5E2): a bare string, an
    # unknown contract id, and a mistagged family are each refused by the
    # auditor; the dangling-guarantee case is seedcheck's executed law and is
    # proven in the Tier 0 fixtures.
    def wit_validator(tmp):
        return audit.guarantee_typed_witness_findings(tmp, audit.guarantee_seed_rows(tmp))

    probe("bare_string_witness_is_rejected", IN,
          'witnesses: &[WitnessRef::dec("DEC-015")]',
          "not a canonical typed WitnessRef",
          'witnesses: &["DEC-015"]', validator=wit_validator)
    probe("unknown_contract_witness_is_rejected", IN,
          'WitnessRef::contract("BP-GAUNTLET-1")',
          "which no document declares",
          'WitnessRef::contract("BP-NOPE-1")', validator=wit_validator)
    probe("mistagged_witness_family_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]',
          "whose family owns the LEG- prefix",
          'witnesses: &[WitnessRef::leg("DEC-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]', validator=wit_validator)

    # 8. Parity passes while provenance fails: mutate the SAME constant in both
    #    paths. This is the exact hole -- before the closure the graph regenerated,
    #    parity passed, and every gate stayed green.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        gu = tmp / GU
        gu.write_text(must_replace(gu.read_text(encoding="utf-8"),
                      "lifetime: LifetimeRule::FromDecisionDisposition,",
                      "lifetime: LifetimeRule::FamilyConstant(GuaranteeLifetime::Permanent),",
                      "common-mode lifetime"), encoding="utf-8")
        lives = {n["id"]: n["lifetime"] for n in audit.guarantee_derive(tmp)[0]}
        if lives.get("DEC-002") != "Permanent":
            fail("common_mode_mutation_probe_did_not_take_effect")
        # The typed source now says Permanent, so BOTH paths agree and parity is
        # silent. Only the typed owner changed -- which is the point: policy lives
        # in one place and a reviewer sees it in the diff, not in two scripts.
        proj = {n["id"]: n["lifetime"] for n in project.guarantee_nodes(tmp)}
        if proj.get("DEC-002") != lives.get("DEC-002"):
            fail("policy_change_is_visible_to_both_paths_from_one_owner")

    findings.extend(canonical_drift(before))
    return findings

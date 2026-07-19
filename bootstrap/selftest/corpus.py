"""Gate identity, decision ledger, authenticated history, document and
claim-receipt law, control characters, substrate, specialization, delivery
notes, and the probe-harness isolation fixtures."""
from __future__ import annotations
import shutil
import tempfile
from pathlib import Path
from .core import (
    HERE,
    ProbeError,
    canonical_commitments,
    canonical_drift,
    gate_sandbox,
    isolated_tree,
    must_replace,
    neutered_validator,
)


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
            '    GateSpec { id: GateId::G9, token: "G9", title: "Self-hosting and release seal" },',
            "")],
          audit.gate_inventory_findings, "missing from the GATES inventory")
    probe("inventory_token_duplicated_across_two_gateids_is_rejected",
          [("spec/gates.rs",
            'GateSpec { id: GateId::G1, token: "G1", title: "MacBat" }',
            'GateSpec { id: GateId::G1, token: "G0", title: "MacBat" }')],
          audit.gate_inventory_findings, "one token under two GateIds")

    # --- migration conservation: a SEED/LEG gate set may not move ---
    # None means the row no longer passes typed admission (5.5D4b). Dropping a
    # required gate list is still detected -- more sharply than before, because
    # the row cannot be admitted at all rather than projecting an empty string.
    def gates_of(tmp, ident):
        return {n["id"]: n["gate_posture"] for n in audit.guarantee_derive(tmp)[0]}.get(ident)

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
                         'Planned("concurrency model + duplicate-writer refusal"), gates: &[GateId::G2]',
                         'Planned("concurrency model + duplicate-writer refusal"), gates: &[]')])
    if gates_of(tmp, "LEG-001") == base_leg:
        fail("leg_gate_dropped_during_migration_is_detected")
    expect("emptied_gate_list_is_rejected",
           audit.gate_reference_findings(tmp), "names no gate")
    shutil.rmtree(tmp, ignore_errors=True)

    probe("leg_gate_added_during_migration_is_rejected",
          [("spec/legacy_obligations.rs",
            'Planned("historical vectors + future-version refusal"), gates: &[GateId::G2]',
            'Planned("historical vectors + future-version refusal"), gates: &[GateId::G2, GateId::G2]')],
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
    if [(n["id"], n["gate_posture"]) for n in project.guarantee_nodes(root)] != \
       [(n["id"], n["gate_posture"]) for n in audit.guarantee_derive(root)[0]]:
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
    nodes, edges, _adm = audit.guarantee_derive(root)
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
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision node count is not 82")

    if [(n["id"], n.get("dclass"), n["gate_posture"]) for n in project.guarantee_nodes(root) if n["family"] == "DEC"] != \
       [(n["id"], n.get("dclass"), n["gate_posture"]) for n in audit.guarantee_derive(root)[0] if n["family"] == "DEC"]:
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
    # The profile table dissolved (5.5E2): the mutation now attacks the arm of
    # the const fn that owns the fact, exactly where a real weakening would land.
    probe("externally_anchored_given_unanchored_success_is_rejected",
          "            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),\n"
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => None,",
          "            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),\n"
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => Some(SIGNED_UNANCHORED_SUCCESS),",
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
          "            AuthenticatedHistoryProfile::InternalConsistency => &[WitnessPolicy::None],",
          "            AuthenticatedHistoryProfile::InternalConsistency => "
          "&[WitnessPolicy::None, WitnessPolicy::Optional],",
          "frozen matrix says")
    probe("signed_history_with_required_witness_is_rejected",
          "            AuthenticatedHistoryProfile::SignedHistory => {\n"
          "                &[WitnessPolicy::None, WitnessPolicy::Optional]\n"
          "            }",
          "            AuthenticatedHistoryProfile::SignedHistory => &[WitnessPolicy::Required],",
          "permits WitnessPolicy::Required outside ExternallyAnchoredHistory")
    probe("externally_anchored_with_optional_witness_is_rejected",
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Required],",
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Optional],",
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
    nodes, edges, _adm = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-073" not in ids:
        fail("guarantee_graph_missing_dec073")
    for synthetic in ("SpecializedPlan", "SelectionMask", "ScanKernel", "Column", "ColumnTile",
                      "SpecializationKey", "WorkObservation"):
        if synthetic in ids:
            fail(f"synthetic specialization node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision_node_count_is_not_82")
    if len(nodes) != 216 or len(edges) != 24:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    return findings


def test_delivery_notes_d2(audit) -> list[str]:
    """The delivery inventory tracks the live decision count — through the
    generated BUNDLE-INVENTORY derivation (5.5E4b), never an authored number."""
    findings: list[str] = []
    root = HERE.parent
    text = (root / "docs/28_SELF_EXPLAINING_REPOSITORY.md").read_text(encoding="utf-8")
    live = len(audit.G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8")))
    if f"| decision rows | {live} |" not in text:
        findings.append("delivery_notes_tracks_live_decision_count FAILED")
    return findings


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

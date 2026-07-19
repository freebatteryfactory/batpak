"""Proof-relation, proof-policy, integrity/derived/deferred witness, LEG-081
authority, and proof-target-resolver hostile fixtures."""
from __future__ import annotations

import shutil

from .core import (
    HERE,
    _regen_operator_blocks,
    _resolve_edit_carrier,
    canonical_commitments,
    canonical_drift,
    gate_sandbox,
    isolated_tree,
    must_replace,
)


def test_proof_relations(audit, project) -> list[str]:
    """Named hostile fixtures for 5.5E4d: the complete legacy-obligation owner,
    typed proof relations, docs/24 meaning laws, domain projections, and the
    final proof/document mirror sweep."""
    findings: list[str] = []
    root = HERE.parent
    LO, PR = "spec/legacy_obligations.rs", "spec/proof.rs"
    D21, D24 = ("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md", "docs/24_GAUNTLET.md")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, needle, validator=None):
        tmp = gate_sandbox(edits)
        try:
            expect(name, (validator or audit.proof_relation_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.proof_relation_findings(root):
        fail(f"proof_relations_pass_on_the_real_seed "
             f"({audit.proof_relation_findings(root)!r})")

    LEDGER = "LEGACY-OBLIGATION-LEDGER does not match its typed derivation"
    RELS = "PROOF-RELATIONS does not match its typed derivation"
    DOMAIN = "PROOF-REQUIREMENTS does not match its typed derivation"
    ROW23 = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
             'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
             'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }')
    for name, rel, old, new, needle in (
        # The complete legacy owner.
        ("legacy_obligation_missing_evidence_is_rejected", LO,
         'legacy_evidence: "03_INVARIANTS.md, store writer runtime"',
         'legacy_evidence: "  "', "LEG-001 carries no legacy evidence"),
        ("legacy_obligation_missing_mechanism_disposition_is_rejected", LO,
         'mechanism_disposition: "Reimplement cleanly"',
         'mechanism_disposition: "  "', "LEG-001 carries no mechanism disposition"),
        ("planned_legacy_witness_cannot_be_empty", LO,
         'LegacyWitnessRequirement::Planned("concurrency model + duplicate-writer refusal")',
         'LegacyWitnessRequirement::Planned("  ")',
         "LEG-001 carries an empty planned witness"),
        ("canonical_legacy_witness_requires_active_proof_rows", LO,
         'id: "LEG-001", law: "One live writer and one physical commit order per journal", legacy_evidence: "03_INVARIANTS.md, store writer runtime", clean_owner: "batpak::store", mechanism_disposition: "Reimplement cleanly", witness_requirement: LegacyWitnessRequirement::Planned("concurrency model + duplicate-writer refusal")',
         'id: "LEG-001", law: "One live writer and one physical commit order per journal", legacy_evidence: "03_INVARIANTS.md, store writer runtime", clean_owner: "batpak::store", mechanism_disposition: "Reimplement cleanly", witness_requirement: LegacyWitnessRequirement::CanonicalProofRows',
         "LEG-001 claims CanonicalProofRows with no active typed relation"),
        ("planned_legacy_witness_cannot_also_have_active_proof_rows", LO,
         'id: "LEG-023"', 'id: "LEG-023X"',
         "a proof relation binds LEG-023, which no obligation row declares"),
        ("proof_row_cannot_bind_an_unknown_legacy_obligation", PR,
         'GuaranteeRef::leg("LEG-023")', 'GuaranteeRef::leg("LEG-993")',
         "a proof relation binds LEG-993, which no obligation row declares"),
        # Ledger drift.
        ("legacy_obligation_ledger_missing_row_is_rejected", D21,
         "| LEG-001 | One live writer", "| One live writer", LEDGER),
        ("legacy_obligation_ledger_order_drift_is_rejected", D21,
         "| LEG-001 |", "| LEG-001B |", LEDGER),
        ("legacy_obligation_law_drift_is_rejected", D21,
         "One live writer and one physical commit order per journal",
         "Two live writers per journal", LEDGER),
        ("legacy_obligation_evidence_drift_is_rejected", D21,
         "03_INVARIANTS.md, store writer runtime",
         "04_INVARIANTS.md, store writer runtime", LEDGER),
        ("legacy_obligation_owner_drift_is_rejected", D21,
         "| batpak::store | Reimplement cleanly |",
         "| batpak::journal | Reimplement cleanly |", LEDGER),
        ("legacy_obligation_mechanism_drift_is_rejected", D21,
         "| Reimplement cleanly |", "| Rewrite casually |", LEDGER),
        ("legacy_obligation_witness_drift_is_rejected", D21,
         "concurrency model + duplicate-writer refusal",
         "vibes-based concurrency", LEDGER),
        ("legacy_obligation_status_drift_is_rejected", D21,
         "| OnSuccessorGateClosure | Active |", "| OnSuccessorGateClosure | Closed |",
         LEDGER),
        # Proof relations.
        ("active_proof_row_requires_a_projection_contract", PR,
         'guarantee: GuaranteeRef::leg("LEG-023"), projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }',
         'guarantee: GuaranteeRef::leg("LEG-023"), projection_contracts: &[], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }',
         "names no projection contract"),
        ("active_proof_row_with_unknown_projection_contract_is_rejected", PR,
         'ContractId("BP-BVISOR-1")', 'ContractId("BP-NOBODY-1")',
         "which no authored document declares"),
        ("active_proof_row_with_duplicate_projection_contract_is_rejected", PR,
         'projection_contracts: &[ContractId("BP-BVISOR-1")]',
         'projection_contracts: &[ContractId("BP-BVISOR-1"), ContractId("BP-BVISOR-1")]',
         "repeats a projection contract"),
        ("generated_document_cannot_consume_active_proof_relation", PR,
         'ContractId("BP-MIGRATION-1")', 'ContractId("BP-GAUNTLET-1")',
         "lists automatic consumer BP-GAUNTLET-1"),
        ("proof_relation_summary_missing_row_is_rejected", D24,
         "| LEG-023 | BP-STORAGE-TILES-1 |", "| LEG-023X | BP-STORAGE-TILES-1 |",
         RELS),
        ("proof_relation_guarantee_drift_is_rejected", D24,
         "| DEC-075 | BP-SYSTEM-MODEL-1 |", "| DEC-074 | BP-SYSTEM-MODEL-1 |", RELS),
        ("proof_relation_target_drift_is_rejected", D24,
         "| LEG-043 | BP-BVISOR-1 |", "| LEG-043 | BP-SYNCBAT-1 |", RELS),
        # Domain projections.
        ("domain_proof_projection_missing_row_is_rejected",
         "docs/09_BVISOR.md",
         "descriptor_postcondition_failure_is_not_reported_applied; ", "", DOMAIN),
        ("domain_proof_projection_extra_row_is_rejected", "docs/22_MIGRATION_AND_CUTOVER.md",
         "| LEG-074 | close_reopen_reimport_returns_zero_new_events |",
         "| LEG-074 | close_reopen_reimport_returns_zero_new_events; rogue_row |",
         DOMAIN),
        ("domain_proof_projection_guarantee_drift_is_rejected",
         "docs/02_SYSTEM_MODEL.md",
         "| DEC-075 |", "| DEC-074 |", DOMAIN),
        ("docs03_hash_map_row_must_move_to_docs04", "docs/03_REPOSITORY_AND_PACKAGES.md",
         "browser_and_native_profiles_preserve_program_semantics",
         "browser_and_native_profiles_preserve_program_semantics; hash_map_iteration_cannot_influence_canonical_observables",
         "docs/03 carries the hash-map proof row"),
        # Meaning and mirror sweep.
        ("docs24_cannot_claim_proof_identity_ownership", D24,
         "`spec/proof/` owns proof-row identity, lifecycle, succession, guarantee binding, and projection membership; this document owns",
         "This document owns proof-row identity and executable meaning; it also owns",
         "may not claim proof-row identity ownership"),
        ("documentary_convergence_temporary_wording_cannot_return", D24,
         "## Typed proof ownership",
         "The receipt details stay in docs/24 until the documentary convergence pass.\n\n## Typed proof ownership",
         "temporary documentary-convergence wording may not return"),
        ("docs24_required_witness_inventory_cannot_return", D24,
         "## Typed proof ownership",
         "Required witnesses (proof owner TestPak; gates G2), also carried by `LEG-999`:\n\n## Typed proof ownership",
         "authored required-witnesses inventory may not return"),
        ("authored_domain_proof_inventory_cannot_return", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "## Required proof rows" if False else "`spec/proof/` owns proof-row identity and membership.",
         "Required proof rows, projected from docs/24 (LEG-023):\n\n```text\nmiddle_event_deletion_is_rejected\n```\n\n`spec/proof/` owns proof-row identity and membership.",
         "authored required-proof-rows inventory may not return"),
        ("authored_legacy_obligation_row_cannot_return", D21,
         "## Closure rule",
         "| LEG-999 | Rogue law | e | o | m | w | G2 | None | Never | Active |\n\n## Closure rule",
         "authored legacy-obligation row may not return"),
        ("proof_spec_cannot_absorb_expectation_prose", PR,
         "pub struct ProofRowRecord {",
         "pub struct ProofRowRecord {\n    pub expectation: &'static str,",
         "expectation prose field may not enter the typed catalog"),
    ):
        probe(name, [(rel, old, new)], needle)

    # Retired lifecycle: successor must be active (seedcheck-side law also
    # covers it; the audit-side catalog law keeps the census honest).
    probe("retired_proof_successor_must_be_active",
          [(PR, 'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
            'successors: &[ProofRowId("pre_shred_keyset_restore_is_rejected")]')],
          "retirement path terminates in no Active identity",
          validator=audit.proof_row_catalog_findings)

    # GREEN: planned obligation growth.
    tmp = gate_sandbox([
        (LO, "pub const OBLIGATIONS: &[LegacyObligation] = &[",
         "pub const OBLIGATIONS: &[LegacyObligation] = &[\n"
         '    LegacyObligation { id: "LEG-990", law: "Sandbox growth law", '
         'legacy_evidence: "sandbox evidence", clean_owner: "batpak::sandbox", '
         'mechanism_disposition: "Sandbox only", witness_requirement: '
         'LegacyWitnessRequirement::Planned("structural count-freedom evidence only"), '
         "gates: &[GateId::G2], compatibility_disposition: CompatibilityDisposition::None, "
         "deletion_condition: DeletionCondition::OnSuccessorGateClosure, "
         "active_or_closed_status: ObligationStatus::Active },")])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.proof_relation_findings(tmp)
        d21 = (tmp / D21).read_text(encoding="utf-8")
        if grown or "| LEG-990 | Sandbox growth law | sandbox evidence |" not in d21:
            fail(f"planned_obligation_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: canonical proof growth — a new active relation flows into
    # docs/21, docs/24 relations, and the bound domain document.
    tmp = gate_sandbox([
        (PR, ROW23 + ",", ROW23 + ",\n    "
         'ProofRowRecord { id: ProofRowId("sandbox_growth_row_is_lawful"), '
         'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
         'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.proof_relation_findings(tmp)
        ok = ("sandbox_growth_row_is_lawful" in (tmp / D21).read_text(encoding="utf-8")
              and "sandbox_growth_row_is_lawful" in
              (tmp / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8"))
        # The new row lacks a docs/24 meaning entry — exactly the law that
        # makes Planned->Canonical atomic with meaning admission.
        meaning_gap = any("has no authoritative meaning" in f
                          for f in audit.witness_reference_findings(tmp))
        if grown or not ok or not meaning_gap:
            fail(f"canonical_proof_growth_is_lawful ({grown!r}, projected={ok}, "
                 f"meaning_law={meaning_gap})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: retired succession — a retired row leaves the projections while
    # its successor carries the relation.
    tmp = gate_sandbox([
        (PR, ROW23,
         'ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
         'state: ProofRowState::Retired { successors: '
         '&[ProofRowId("event_reorder_is_rejected")] } }')])
    try:
        _regen_operator_blocks(project, tmp)
        d05 = (tmp / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8")
        if "middle_event_deletion_is_rejected" in audit.batql_extract_block(
                d05, "PROOF-REQUIREMENTS"):
            fail("retired_succession_leaves_current_projections")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

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

    PPOL = "spec/proof/policy.rs"
    SPRT = "spec/sprouting/inventory.rs"
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

    # --- lane and result vocabularies (typed owner: spec/mutation/types.rs) --
    MUT = "spec/mutation.rs"
    probe("fourth_mutation_lane_is_rejected", MUT,
          "    CompilerBacked,\n}", "MutationLane variants", "    CompilerBacked,\n    NativeLane,\n}",
          validator=pp)
    probe("removed_mutation_lane_is_rejected", MUT,
          "    /// Implementation-sensitive material whose truth depends on real\n"
          "    /// compiler and platform behavior. Runs under real rustc semantics.\n"
          "    CompilerBacked,\n}",
          "MutationLane variants", "}", validator=pp)
    for variant in ("NotActivated", "Refused", "TimedOut", "InfrastructureFailure", "EquivalentCandidate"):
        probe(f"mutation_result_{variant.lower()}_removed_is_rejected", MUT,
              f"    {variant},\n", "MutationResult variants", "", validator=pp)
    probe("change_class_collapsed_is_rejected", PPOL,
          "    Neutral,\n", "ProofPolicyChangeClass variants", "", validator=pp)
    probe("proof_policy_surface_removed_is_rejected", PPOL,
          "    WaiverLogic,\n", "ProofPolicySurface variants", "", validator=pp)

    # --- candidate and promotion boundary -----------------------------------
    for rootdir in ("src/", "tests/", "spec/", "docs/", "companion/"):
        probe(f"candidate_write_into_{rootdir.strip('/')}_permitted_is_rejected", SPRT,
              f'"{rootdir}"', f"not forbidden from writing {rootdir}", '"__none__"', validator=pp)
    probe("candidate_root_moved_into_tracked_source_is_rejected", SPRT,
          "target/muterprater/candidates/", "no candidate output root is declared", "tests/generated/",
          validator=pp)
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
    nodes, edges, _adm = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-074" not in ids:
        fail("guarantee_graph_missing_dec074")
    for synthetic in ("SemanticIr", "SelectableCompiled", "CompilerBacked", "MutationLane",
                      "MutationResult", "ProofPolicySurface", "Killed", "Survived"):
        if synthetic in ids:
            fail(f"synthetic mutation node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision_node_count_is_not_82")
    # 25 edges since the Source Grammar Bake: SEED-CONCEPT-SPINE additionally
    # derives from DEC-007 (the amended recursive domain-directory grammar).
    if len(nodes) != 216 or len(edges) != 25:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    text = (root / "docs/28_SELF_EXPLAINING_REPOSITORY.md").read_text(encoding="utf-8")
    live_decisions = len(audit.G_DEC_ROW.findall(
        (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")))
    if f"| decision rows | {live_decisions} |" not in text:
        fail("bundle_inventory_tracks_live_decision_count")
    return findings


def test_integrity_witnesses(audit) -> list[str]:
    """LEG-023 hostile integrity witnesses and the witness-reference resolver.

    Structural only. Bootstrap executes no journal verification, reads no real
    chain, detects no real event loss, and runs no SIDX recovery. These fixtures
    prove the witness identities, owner bindings, meanings, and cross-surface
    agreement survive -- not that a chain was verified.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    iw = audit.integrity_witness_findings

    if audit.witness_reference_findings(root) or iw(root):
        fail("leg023_witness_contract_passes")

    # Each of the seven witness identities must survive in the typed catalog
    # (5.5E4d: spec/proof/inventory.rs owns membership; docs/24 owns meaning only).
    for wid in audit.D4B_LEG023_WITNESSES:
        probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
              f'id: ProofRowId("{wid}")', f"{wid} is absent from docs/24",
              f'id: ProofRowId("{wid}_gone")', validator=iw)

    # The resolver: unknown, duplicated, misowned, missing, extra.
    probe("unknown_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "references unknown proof-row id ghost_witness",
          "ghost_witness; middle_event_deletion_is_rejected;")
    probe("duplicate_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "projects a duplicate witness reference",
          "middle_event_deletion_is_rejected; middle_event_deletion_is_rejected;")
    probe("witness_owned_by_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("cross_lane_predecessor_is_rejected"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")',
          "LEG-023 witness cross_lane_predecessor_is_rejected is bound to LEG-081",
          'ProofRowId("cross_lane_predecessor_is_rejected"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081")',
          validator=iw)
    probe("docs21_missing_owned_witness_is_rejected", D21,
          "; midstream_genesis_is_rejected", "omits owned proof row midstream_genesis_is_rejected")
    probe("docs21_extra_witness_is_rejected", D21,
          "middle_event_deletion_is_rejected;",
          "references unknown proof-row id unknown_leg_proof_row_for_fixture",
          "unknown_leg_proof_row_for_fixture; middle_event_deletion_is_rejected;")
    ROW23F = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
              'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("duplicate_docs24_proof_row_id_is_rejected", "spec/proof.rs", ROW23F,
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          ROW23F + "\n    " + ROW23F)
    probe("witness_meaning_removal_is_rejected", GA,
          "midstream_genesis_is_rejected\n    A genesis marker cannot reset an already-started stream.",
          "has no authoritative meaning", "")

    # Weakening the law itself: each impersonation the law forbids.
    for clause, label in (
        ("never proves EventCommitment equality", "content_digest_substituting_for_event_commitment_is_rejected"),
        ("an event appearing somewhere in a verified set is not thereby the immediate predecessor",
         "set_membership_accepted_as_predecessor_is_rejected"),
        ("legal genesis occurs only at the lawful stream head", "midstream_genesis_allowed_is_rejected"),
        ("may not select the expected authority bytes and then use those selected bytes to authenticate its own selection",
         "forged_index_selecting_and_authenticating_is_rejected"),
        ("requires the exact immediate predecessor", "inexact_predecessor_allowed_is_rejected"),
        ("lane isolation is owned by LEG-050", "leg023_absorbing_leg050_is_rejected"),
        ("visible linearization by LEG-067", "leg023_absorbing_leg067_is_rejected"),
    ):
        probe(label, LEG, clause, f"LEG-023 law no longer states: {clause}", "", validator=iw)

    findings.extend(canonical_drift(before))
    return findings


def test_derived_material_witnesses(audit) -> list[str]:
    """LEG-019 duals plus the LEG-020/LEG-028 bounded-work rows (5.5D4b-2a).

    Structural only. Bootstrap runs no SIDX recovery, detects no real loss,
    performs no traversal, and measures no allocation.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    dm = audit.derived_material_findings

    if dm(root) or audit.witness_reference_findings(root):
        fail("d4b2a_witness_contract_passes")

    # every new proof-row identity survives in the typed catalog (5.5E4d)
    for leg, ids in audit.D4B2A_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
                  f'id: ProofRowId("{wid}")',
                  f"{leg} witness {wid} is absent from docs/24",
                  f'id: ProofRowId("{wid}_gone")', validator=dm)

    # the law's load-bearing clauses -- both duals must stay
    for clause, label in (
        ("cannot prove safety and cannot prove loss", "sidx_proving_loss_or_safety_is_rejected"),
        ("proves only that exact row-to-authoritative-frame relationship",
         "corroborated_row_authenticating_siblings_count_order_or_tail_is_rejected"),
        ("never an authoritative proven-loss or proven-safety conclusion",
         "derived_table_reaching_an_authoritative_conclusion_is_rejected"),
        ("no trusted-local SIDX authority", "trusted_local_sidx_authority_is_rejected"),
        ("cannot authenticate themselves", "derived_material_self_authentication_is_rejected"),
    ):
        probe(label, LEG, clause, f"LEG-019 law no longer states: {clause}", "", validator=dm)

    # bounded discovery and bounded allocation keep distinct owners
    probe("bounded_output_accepted_as_bounded_discovery_work_is_rejected", "spec/proof.rs",
          'id: ProofRowId("page_limit_bounds_discovery_work_not_only_output")',
          "LEG-028 witness page_limit_bounds_discovery_work_not_only_output is absent",
          'id: ProofRowId("page_limit_bounds_discovery_work_not_only_output_gone")',
          validator=dm)
    probe("full_matched_set_allocation_accepted_is_rejected", "spec/proof.rs",
          'id: ProofRowId("allocation_does_not_scale_with_full_matched_set")',
          "LEG-020 witness allocation_does_not_scale_with_full_matched_set is absent",
          'id: ProofRowId("allocation_does_not_scale_with_full_matched_set_gone")',
          validator=dm)
    probe("discovery_and_allocation_sharing_one_owner_is_rejected", "spec/proof.rs",
          'ProofRowId("allocation_does_not_scale_with_full_matched_set"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-020")',
          "share one qualification target",
          'ProofRowId("allocation_does_not_scale_with_full_matched_set"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028")',
          validator=dm)

    # resolver coverage for the new rows
    probe("d4b2a_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("forged_sibling_cannot_cause_false_loss"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-019")',
          "which docs/24 binds to LEG-023",
          'ProofRowId("forged_sibling_cannot_cause_false_loss"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")')
    probe("docs21_missing_a_d4b2a_row_is_rejected", D21,
          "; derived_row_cannot_prove_absence_or_loss",
          "omits owned proof row derived_row_cannot_prove_absence_or_loss")
    probe("docs21_extra_d4b2a_row_is_rejected", D21,
          "allocation_does_not_scale_with_full_matched_set",
          "references derived_row_cannot_authenticate_order, which docs/24 binds to LEG-019",
          "derived_row_cannot_authenticate_order")
    probe("duplicate_row_across_d4b1_and_d4b2a_is_rejected", "spec/proof.rs",
          'id: ProofRowId("forged_sibling_cannot_cause_false_loss")',
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          'id: ProofRowId("middle_event_deletion_is_rejected")')
    probe("d4b2a_meaning_removed_is_rejected", GA,
          "derived_row_cannot_authenticate_order\n    Corroborating row contents does not establish table ordering.",
          "has no authoritative meaning", "")

    findings.extend(canonical_drift(before))
    return findings


def test_deferred_witnesses(audit) -> list[str]:
    """LEG-043 deferred native rows and the LEG-074 reimport row (5.5D4b-2b,
    rebased in 5.5E4d): the typed catalog owns membership; the fence-header
    posture prose retired with the membership mirrors, and execution evidence
    lives in run receipts. What survives here is membership and meaning."""
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    dp = audit.deferred_posture_findings

    if dp(root) or audit.witness_reference_findings(root):
        fail("d4b2b_witness_contract_passes")

    # every new proof-row identity survives in the typed catalog
    for leg, ids in audit.D4B2B_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
                  f'id: ProofRowId("{wid}")',
                  f"{leg} witness {wid} is absent from docs/24",
                  f'id: ProofRowId("{wid}_gone")', validator=dp)
    probe("leg043_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-043")',
          "LEG-043 witness fcntl_getfd_failure_fails_closed is bound to LEG-074",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-074")',
          validator=dp)

    # meanings carry the law they exist to defend
    # A proof row's MEANING is what a future implementation qualifies against.
    # Removing a clause from it is a weakening even though the ID survives.
    for frag, label, needle in (
        ("not reported as established",
         "descriptor_postcondition_reported_applied_after_failure_is_rejected",
         "meaning no longer states: not reported as established"),
        ("Failure to read descriptor flags cannot produce a verified close-on-exec",
         "fcntl_read_failure_reported_verified_is_rejected",
         "meaning no longer states: Failure to read descriptor flags"),
        ("Failure to apply descriptor flags cannot produce an applied or verified",
         "fcntl_write_failure_reported_applied_is_rejected",
         "meaning no longer states: Failure to apply descriptor flags"),
        ("imports zero new events", "reimport_creating_new_events_is_rejected",
         "meaning no longer states: imports zero new events"),
        ("already-established authority outcome", "reimport_inventing_an_outcome_is_rejected",
         "meaning no longer states: already-established authority outcome"),
        ("witness binds stable import identity", "reimport_losing_bound_identities_is_rejected",
         "meaning no longer states: witness binds stable import identity"),
    ):
        probe(label, GA, frag, needle, "", validator=audit.proof_meaning_findings)

    # resolver coverage for the new rows, on the typed catalog (5.5E4d)
    probe("d4b2b_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("close_reopen_reimport_returns_zero_new_events"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-074")',
          "which docs/24 binds to LEG-023",
          'ProofRowId("close_reopen_reimport_returns_zero_new_events"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")')
    probe("duplicate_row_across_prior_d4b_passes_is_rejected", "spec/proof.rs",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state',
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          'ProofRowId("middle_event_deletion_is_rejected"), state')

    findings.extend(canonical_drift(before))
    return findings


def test_leg081_authority(audit) -> list[str]:
    """LEG-081 proof-row authority migration (5.5D4b-2c).

    Structural only. Bootstrap never shredded a key, never called a key backend,
    never crashed a store, and never inspected key material. These fixtures prove
    that the nine proof contracts are owned in one place, projected exactly
    elsewhere, and that every clause of the typed law carries a witness.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    D35 = "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md"
    la = audit.leg081_authority_findings
    pm = audit.proof_meaning_findings
    LEAK = "travel only through an explicit, independent ceremony."

    if la(root) or audit.witness_reference_findings(root) or pm(root):
        fail("d4b2c_leg081_contract_passes")

    # The retarget premise: the old fixture id is now owned, and its replacement
    # is genuinely unknown. Without both, the retargeted probe would go red for
    # the wrong severed wire.
    rows = audit.witness_rows(root)
    if rows.get("shred_ack_waits_for_backend_durability", {}).get("leg") != "LEG-081":
        fail("retired_fixture_id_is_now_owned_by_leg081")
    if "unknown_leg_proof_row_for_fixture" in rows:
        fail("replacement_fixture_id_is_genuinely_unknown")

    # Every canonical identity survives in the typed catalog (5.5E4d).
    for wid in audit.D4B2C_ROWS:
        probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
              f'id: ProofRowId("{wid}")',
              f"LEG-081 canonical proof row {wid} is absent from docs/24",
              f'id: ProofRowId("{wid}_gone")', validator=la)

    # Retired vocabulary cannot return outside the historical migration note, in
    # ANY document: one registry, one rule. `stale_or_pre_shred_keyset_restore_is_rejected`
    # contains a retired id as a substring; the clean tree passing above proves the
    # boundary rule does not fire on it, and these prove it does fire on the bare
    # retired id. docs/35 is the plant site precisely because it owns none of these
    # rows now: a rule scoped to a phase's own documents would miss the leak.
    rp = audit.retired_proof_row_findings
    for old in audit.retired_proof_rows(HERE.parent):
        probe(f"retired_{old}_reintroduced_is_rejected", D35, LEAK,
              f"reintroduces retired proof-row id {old} outside the historical migration note",
              LEAK + f" See {old}.", validator=rp)

    # The catalog is TYPED and must agree with the migration notes in both
    # directions (5.5E2). The Python-dict era shipped exactly this defect: a
    # docs/24 note retired an id the dict never learned, so its scanner
    # guarded five of six retirements while claiming to guard them all.
    PR = "spec/proof.rs"
    probe("registry_missing_a_noted_retirement_is_rejected", PR,
          '    ProofRowRecord { id: ProofRowId("test_local_fixture_type_is_classified_correctly"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")] } },\n',
          "PROOF_ROWS never learned",
          "", validator=rp)
    probe("registry_retirement_without_a_note_is_rejected", PR,
          'ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"),',
          "no docs/24 migration note recording the retirement",
          'ProofRowRecord { id: ProofRowId("phantom_row_never_noted"),', validator=rp)

    # The catalog is the LIVING CENSUS (5.5E2j): active identities equal the
    # structurally parsed canonical docs/24 rows in both directions, one
    # identity carries one lifecycle, and both lifecycle states are
    # constructed. A name surviving in a migration note or prose is not a row.
    cat = audit.proof_row_catalog_findings
    if cat(HERE.parent):
        fail("proof_row_catalog_passes_on_the_real_seed")
    # The typed-vs-docs/24 membership parity retired in 5.5E4d: the typed
    # catalog is the one membership authority, and the meaning-coverage laws
    # (witness_reference_findings) own the docs/24 side.
    ROW23C = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
              'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("same_identity_cannot_be_active_and_retired", PR, ROW23C,
          "is declared twice in the proof-identity catalog",
          ROW23C + '\n    ProofRowRecord { id: '
          'ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("event_reorder_is_rejected")] } },', validator=cat)
    # E3 preflight: succession terminates. A two-node cycle satisfies
    # existence and non-self-succession while owning no living obligation;
    # a multi-hop chain through a retired row that reaches an Active identity
    # is lawful.
    RETIRED_PAIR = (
        '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },\n'
        '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct")] } },\n')
    RETIRED_CYCLE = (
        '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_and_keyset_missing_remain_distinct")] } },\n'
        '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("pre_shred_keyset_restore_is_rejected")] } },\n')
    probe("two_retired_rows_cannot_succeed_each_other_cyclically", PR,
          RETIRED_PAIR, "retirement succession is cyclic",
          RETIRED_CYCLE, validator=cat)
    probe("retirement_component_with_no_active_leaf_is_rejected", PR,
          RETIRED_PAIR, "retirement path terminates in no Active identity",
          RETIRED_CYCLE, validator=cat)
    # A GREEN fixture: the lawful multi-hop chain must produce no finding.
    tmp = gate_sandbox([(PR,
        'ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },',
        'ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_and_keyset_missing_remain_distinct")] } },')])
    try:
        got = cat(tmp)
        if got:
            fail(f"multi_hop_retirement_chain_terminates_in_active_identity (got {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    probe("proof_row_state_variants_are_constructed", PR,
          '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },\n'
          '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct")] } },\n'
          '    ProofRowRecord { id: ProofRowId("snapshot_and_fork_exclude_keys_by_default"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys")] } },\n'
          '    ProofRowRecord { id: ProofRowId("hash_map_iteration_cannot_change_canonical_bytes"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("hash_map_iteration_cannot_influence_canonical_observables")] } },\n'
          '    ProofRowRecord { id: ProofRowId("attempt_receipt_cannot_cross_invocation_classes"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("entrypoint_receipt_cannot_satisfy_query_program_execution"), '
          'ProofRowId("query_program_receipt_cannot_satisfy_entrypoint_invocation")] } },\n'
          '    ProofRowRecord { id: ProofRowId("test_local_fixture_type_is_classified_correctly"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")] } },\n',
          "constructs no Retired row",
          "", validator=cat)

    # docs/21 projects exactly the nine.
    probe("docs21_leg081_missing_projected_id_is_rejected", D21,
          "; foreign_keyset_generation_is_rejected",
          "omits owned proof row foreign_keyset_generation_is_rejected")
    probe("docs21_leg081_extra_projected_id_is_rejected", D21,
          "shred_ack_waits_for_backend_durability;",
          "references middle_event_deletion_is_rejected, which docs/24 binds to LEG-023",
          "middle_event_deletion_is_rejected; shred_ack_waits_for_backend_durability;")

    # docs/35 projects exactly the nine and reclaims nothing.
    # docs/35's hand list is a REGISTERED generated view since 5.5E4d
    # (SecretAuthorityProofRequirements): the universal census and domain
    # projection parity own its membership now. What survives is the reclaim
    # fence: docs/35 may never again author ownership surfaces.
    probe("docs35_reclaims_authoritative_ownership_is_rejected", D35, LEAK,
          "docs/35 reclaims authoritative proof-row ownership",
          LEAK + "\n\nRequired witnesses (proof owner TestPak; gates G2/G3), also carried by "
          "`LEG-081`:\n\n```text\nshred_ack_waits_for_backend_durability\n```\n",
          validator=la)
    probe("docs35_reclaims_authoritative_witness_meaning_is_rejected", D35, LEAK,
          "docs/35 reclaims per-ID authoritative witness meaning",
          LEAK + "\n\nAuthoritative meanings:\n\n```text\n"
          "shred_ack_waits_for_backend_durability\n    A local restatement.\n```\n",
          validator=la)

    # docs/24 row posture, owner, gates, target, meaning, uniqueness.
    probe("docs24_leg081_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("shred_ack_waits_for_backend_durability"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081")',
          "LEG-081 canonical proof row shred_ack_waits_for_backend_durability is bound to LEG-020",
          'ProofRowId("shred_ack_waits_for_backend_durability"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-020")',
          validator=la)
    probe("docs24_leg081_meaning_removal_is_rejected", GA,
          "shred_transition_binding_mismatch_is_rejected\n    The shred transition binds StoreId,"
          " AuthorityGeneration, KeyGeneration, key\n",
          "LEG-081 proof row shred_transition_binding_mismatch_is_rejected has no authoritative meaning",
          "", validator=la)
    ROW81F = ('ProofRowRecord { id: ProofRowId("foreign_keyset_generation_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081"), '
              'projection_contracts: &[ContractId("BP-CRYPTO-SECRET-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("duplicate_leg081_proof_row_id_is_rejected", "spec/proof.rs", ROW81F,
          "docs/24 binds LEG-081 proof row foreign_keyset_generation_is_rejected more than once",
          ROW81F + "\n    " + ROW81F, validator=la)
    probe("leg081_proof_row_id_rename_is_rejected", "spec/proof.rs",
          'ProofRowId("foreign_keyset_generation_is_rejected")',
          "docs/24 binds unexpected proof row foreign_keyset_generation_renamed to LEG-081",
          'ProofRowId("foreign_keyset_generation_renamed")', validator=la)

    # Meanings still carry the law they exist to defend.
    for frag, label in (
        ("established destruction of the relevant key authority.",
         "shred_ack_durability_clause_removal_is_rejected"),
        ("acknowledgement or a successful final shred result.",
         "crash_recovery_posture_clause_removal_is_rejected"),
        ("scope, and shred-transition identity.",
         "binding_components_clause_removal_is_rejected"),
        ("projection or formatter may collapse one into another.",
         "state_distinction_clause_removal_is_rejected"),
        ("not export usable raw secret-key bytes",
         "raw_key_export_clause_removal_is_rejected"),
        ("never semantics.",
         "external_backend_parity_clause_removal_is_rejected"),
    ):
        probe(label, GA, frag, "meaning no longer states", "", validator=pm)

    # Every law clause keeps an owned executable witness.
    probe("coverage_clause_missing_is_rejected", GA,
          "external backend semantic parity\n    external_key_backend_preserves_shred_semantics\n",
          "LEG-081 coverage matrix omits law clause: external backend semantic parity",
          "", validator=la)
    probe("coverage_clause_with_no_proof_row_is_rejected", GA,
          "external backend semantic parity\n    external_key_backend_preserves_shred_semantics",
          "LEG-081 coverage clause names no proof row: external backend semantic parity",
          "external backend semantic parity", validator=la)
    probe("canonical_row_omitted_from_coverage_matrix_is_rejected", GA,
          "    reopen_after_ack_cannot_recover_shredded_plaintext\n",
          "LEG-081 canonical proof row reopen_after_ack_cannot_recover_shredded_plaintext"
          " appears in no coverage clause", "", validator=la)
    probe("unknown_proof_row_in_coverage_matrix_is_rejected", GA,
          "    external_key_backend_preserves_shred_semantics\n",
          "LEG-081 coverage matrix names unknown proof row ghost_coverage_row",
          "    external_key_backend_preserves_shred_semantics\n    ghost_coverage_row\n",
          validator=la)
    probe("coverage_matrix_removed_is_rejected", GA, "LEG-081 proof-coverage matrix:",
          "docs/24 states no LEG-081 proof-coverage matrix",
          "LEG-081 proof-coverage notes:", validator=la)

    findings.extend(canonical_drift(before))
    return findings


def test_proof_target_resolver(audit) -> list[str]:
    """Generic source-qualified GuaranteeRef resolution (5.5D4b-3b0).

    Structural only. Bootstrap executes no proof row of any family. These
    fixtures prove the resolver accepts every guarantee family through one row
    format, reads gates from the typed target rather than the source block, and
    fails closed on an unknown or non-guarantee target.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def silent(name: str, produced) -> None:
        if produced:
            fail(f"{name} (expected no finding, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.proof_target_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    pt = audit.proof_target_findings
    pg = audit.proof_target_grammar_findings
    # LEG-028 owns exactly one row, so retargeting its block cannot collide.
    W28 = "page_limit_bounds_discovery_work_not_only_output"

    if pt(root) or pg(root) or audit.witness_reference_findings(root):
        fail("d4b3b0_resolver_contract_passes")

    # The index resolves every declared guarantee, and only those.
    idx = audit.guarantee_index(root)
    if len(idx) != 216:
        fail(f"guarantee_index_covers_every_guarantee (got {len(idx)})")
    for ref, want in (("LEG-081", "G2/G3"), ("DEC-065", "G0/G5"), ("SEED-FBAT-CORE", "G2")):
        if audit.guarantee_gates(root, ref) != want:
            fail(f"{ref}_gates_resolve_from_typed_target")
    # ARCH and QUAL now resolve typed gate postures (5.5D4b): ARCH from its
    # declared family policy, QUAL from its own row. Before the authority closure
    # both returned None and a qualification target sat in the gate field.
    for ref, want in (("ARCH-batpak", "G0"), ("QUAL-batpak-native", "G0/G5"),
                      ("QUAL-syncbat-browser", "G0/G5")):
        if audit.guarantee_gates(root, ref) != want:
            fail(f"{ref}_resolves_its_typed_gate_posture "
                 f"(wanted {want}, got {audit.guarantee_gates(root, ref)!r})")
    # A qualification target must never be readable as a gate.
    if audit.guarantee_gates(root, "QUAL-batpak-semantic") in ("no_std + alloc", "std", "wasm32 host"):
        fail("qualification_target_cannot_be_read_as_a_gate_posture")
    # LEG resolution is unchanged by the widening.
    if audit.leg_gates(root, "LEG-043") != "G5" or audit.leg_gates(root, "LEG-081") != "G2/G3":
        fail("leg_gates_behaviour_unchanged_by_widening")
    if audit.leg_gates(root, "DEC-065") != "":
        fail("leg_gates_refuses_a_non_leg_ref")

    # Target selection is TYPED since 5.5E4d: the fence-header retarget,
    # multi-target, family-grammar, and block-gate-override laws became
    # structural (an Active row carries exactly one GuaranteeRef and no gate
    # text). What survives executable here: a dangling member fails closed.
    probe("unknown_guarantee_member_fails_closed", "spec/proof.rs",
          'guarantee: GuaranteeRef::dec("DEC-050")',
          "targets DEC-999, which resolves to no existing DEC guarantee",
          'guarantee: GuaranteeRef::dec("DEC-999")')
    probe("dangling_legacy_guarantee_fails_closed", "spec/proof.rs",
          'guarantee: GuaranteeRef::leg("LEG-028")',
          "targets LEG-998, which resolves to no existing LEG guarantee",
          'guarantee: GuaranteeRef::leg("LEG-998")')

    # Structural discovery: heading text is not the parser API.
    #
    # These plant synthetic blocks rather than asserting against whichever
    # unnormalized block still happens to exist. The earlier form read the label
    # of a real docs/12 candidate, so normalizing that block would have broken the
    # probe -- the test measured the debt instead of the parser, and would have
    # had nothing to stand on once D4d drives the candidate count to zero.
    PLANT = "docs/12_TESTPAK.md"
    IDS = "planted_fixture_alpha_is_rejected\nplanted_fixture_beta_is_rejected"

    def plant(label: str) -> list[dict]:
        with isolated_tree() as tmp:
            p = tmp / PLANT
            p.write_text(p.read_text(encoding="utf-8")
                         + f"\n\n{label}\n\n```text\n{IDS}\n```\n", encoding="utf-8")
            return [c for c in audit.candidate_fences(tmp) if c["doc"] == PLANT]

    base = len([c for c in audit.candidate_fences(root) if c["doc"] == PLANT])
    # A block naming a proof owner is a candidate whatever noun its label uses.
    for label in ("Named hostile fixtures (proof owner TestPak; gates G3):",
                  "Hostile fixture obligations (proof owner: TestPak; gates G3)",
                  "Completely different authored label (proof owner TestPak; gates G3):"):
        got = plant(label)
        if len(got) != base + 1:
            fail(f"planted_block_is_discovered_structurally ({label!r})")
        if got[-1]["kind"] != "UnboundCandidate":
            fail(f"planted_block_with_a_proof_owner_is_an_unbound_candidate ({label!r})")
    # A gate token alone is executable metadata: no proof-owner noun required.
    if plant("Implemented at G3:")[-1]["kind"] != "UnboundCandidate":
        fail("planted_block_naming_only_a_gate_is_an_unbound_candidate")
    # Prose with no executable metadata is evidence, never a promotable obligation.
    if plant("Illustrative scenarios:")[-1]["kind"] != "DescriptiveEvidence":
        fail("fence_without_executable_metadata_is_descriptive_not_promoted")

    # Universal expectation-clause enforcement. The transitional ceiling is
    # spent (D4d): every canonical row carries the clause, the baseline count is
    # zero, and ANY row without one -- newly promoted or newly stripped -- is a
    # finding. The probes therefore corrupt the authored clause on the LEG-028
    # row instead of granting it one.
    ids, blocks, pending = audit.candidate_summary(root)
    if (ids, blocks, pending) != (0, 0, 0):
        fail(f"candidate_summary_reports_current_state (got {(ids, blocks, pending)})")
    W28E = ("    expects: discovery under a page limit and work budget observes decode,\n"
            "      allocation, and materialization work bounded by the declared budget\n"
            "      while the matched set grows far beyond the page")
    W28D = ("    disposition: a page of results with within-budget work evidence, or a\n"
            "      typed budget refusal; a full-set scan truncated at output fails the\n"
            "      witness")
    W28C = W28E + "\n" + W28D
    probe("expectation_clause_without_disposition_is_rejected", GA, W28C,
          f"proof row {W28} expectation clause states no disposition", W28E)
    probe("expectation_clause_without_predicate_is_rejected", GA, W28C,
          f"proof row {W28} expectation clause states no expects", W28D)
    probe("unfalsifiable_expectation_clause_is_rejected", GA, W28E,
          "expectation clause expects is not falsifiable",
          "    expects: TBD")
    ROW28 = ('ProofRowRecord { id: ProofRowId("page_limit_bounds_discovery_work_not_only_output"), '
             'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028"), '
             'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::ResourceEnvelope, verification: PLAN_COMPLEXITY } },')
    probe("newly_promoted_row_without_an_expectation_clause_is_rejected", "spec/proof.rs",
          ROW28,
          "proof rows carry no expectation clause, above the transitional ceiling of 0",
          ROW28 + '\n    ProofRowRecord { id: '
          'ProofRowId("newly_promoted_row_without_a_clause"), '
          'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028"), '
          'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::ResourceEnvelope, verification: PLAN_COMPLEXITY } },')
    # Enforcement is universal, not grandfathered: stripping the clause from a
    # long-migrated row is refused exactly like omitting it on a new one.
    probe("stripping_a_migrated_clause_is_rejected", GA, W28C,
          "proof rows carry no expectation clause, above the transitional ceiling of 0")
    # A clause may wrap, so the scan must read the folded value. Before the fold
    # existed, this mutation passed: the vague qualifier sits on the continuation
    # line, and a first-line-only parser never saw it.
    probe("unfalsifiable_expectation_clause_continuation_is_rejected", GA, W28E,
          "expectation clause expects is not falsifiable",
          W28E + ",\n      as appropriate")
    # The authored clause is itself wrapped: the scan must return the whole
    # folded value, not the first line it happens to see.
    got = audit.witness_expectations(root).get(W28, {}).get("expects", "")
    if got != ("discovery under a page limit and work budget observes decode, "
               "allocation, and materialization work bounded by the declared budget "
               "while the matched set grows far beyond the page"):
        fail(f"expectation_clause_continuation_folds_into_its_value (got {got!r})")

    findings.extend(canonical_drift(before))
    return findings

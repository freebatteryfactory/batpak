"""Domain fixtures: PakVM semantic ISA, SyncBat authority firewall and
crossing requiredness, and DEC-075 reconciliation."""
from __future__ import annotations

from .core import (
    HERE,
    _resolve_edit_carrier,
    canonical_commitments,
    canonical_drift,
    isolated_tree,
    must_replace,
    neutered_validator,
)


# Multi-line Rust fragments the ISA probes mutate. Named once: repeating them
# inline invites a transcription drift that would make a probe miss its target
# and pass for the wrong reason.
def _isa_class_row(cls: str, algebra: str, family: str) -> str:
    return (
        f"        class: PakVmNodeClass::{cls},\n"
        f"        algebra: PakVmAlgebra::{algebra},\n"
        f"        work_formula: WorkFormulaFamily::{family},"
    )


WINDOWING_ROWS = _isa_class_row("Windowing", "QueryDataflow", "Rows")
WINDOWING_OUTPUTS = _isa_class_row("Windowing", "QueryDataflow", "EmittedOutputs")
QD_EVIDENCE = ("        evidence: PakVmRule::AlgebraConstant"
               "(EvidenceClass::ReferenceInterpreterModel),\n")
PUBLIC_LOWERING = ("        lowering: PakVmRule::AlgebraConstant"
                   "(CandidateLoweringPosture::PublicSemanticIdentity),")
INTERNAL_LOWERING = ("        lowering: PakVmRule::AlgebraConstant"
                     "(CandidateLoweringPosture::InternalLoweringIdentity),")


def test_pakvm_semantic_isa(audit, project) -> list[str]:
    """PakVM semantic ISA lineage, projection, and provenance (5.5D4c1).

    Every probe must reach and trip the refusal it names. A probe that goes red
    on an earlier unrelated rule proves the wrong wire was cut, so each mutation
    below is chosen to clear the checks that precede its target.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    ISA = "spec/pakvm_isa.rs"
    D07 = "docs/07_PAKVM_ISA.md"
    pf = audit.pakvm_isa_findings

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            # SW5: an edit aimed at a concept-door spec file resolves to the
            # submodule that carries the needle (the ISA probes below name the
            # pakvm_isa door; their needles live in vocab/policies/nodes).
            path = tmp / _resolve_edit_carrier(root, rel, old)
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, name), encoding="utf-8")
            produced = (validator or pf)(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    # The premise: the ISA admits cleanly today, through BOTH independent
    # derivations. Without this every probe below could be red for free.
    if pf(root):
        fail(f"pakvm_isa_contract_passes (got {pf(root)!r})")
    try:
        auditor = audit.pakvm_isa_views(root)
        generator = project.pakvm_specs(root)
    except Exception as exc:
        auditor, generator = [], []
        fail(f"both_derivations_admit_the_isa ({exc!r})")
    if len(auditor) != 36 or len(generator) != 36:
        fail(f"both_derivations_admit_36_nodes (auditor {len(auditor)}, generator {len(generator)})")
    # Independence is only worth something if the two agree on the ANSWER while
    # sharing no code to compute it.
    if [v["node"] for v in auditor] != [s["node"] for s in generator]:
        fail("independent_derivations_agree_on_the_node_inventory")
    for a, g in zip(auditor, generator, strict=False):
        if (a["algebra"], a["class"], a["effect"], a["capability"], a["work_formula"],
                a["units"], a["evidence"], a["boundedness"]) != (
                g["algebra"], g["class"], g["effect"], g["capability"], g["work_formula"],
                g["units"], g["evidence"], g["boundedness"]):
            fail(f"independent_derivations_agree_on_{a['node']}")
            break

    # --- admission is complete or the node is not in the ISA -------------------
    probe("node_with_no_authored_class_does_not_admit", ISA,
          "            PakVmNodeId::Require => PakVmNodeClass::DecisionGuard,",
          "Require: class() states no authored family",
          "")
    probe("field_with_two_owners_does_not_admit", ISA,
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,\n        boundedness: BoundednessPosture::BoundedIteration,\n        effect: None,",
          "effect is declared by both its algebra and its class",
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,\n        boundedness: BoundednessPosture::BoundedIteration,\n        effect: Some(EffectPosture::ObservationalOnly),")
    probe("field_with_no_owner_does_not_admit", ISA,
          "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
          "effect is declared by neither its algebra nor its class",
          "        effect: PakVmRule::ClassDeclared,")

    # --- authoring surface: every node names a lawful producer (5.5E1) --------
    # Join is the one V1 node the frozen grammar cannot express; deleting its
    # authoring arm must refuse admission (rustc separately refuses the
    # non-exhaustive match under tier0-seedcheck), and smuggling 'join' back
    # into docs/13's member list must be caught as language re-admission.
    probe("a_node_with_no_lawful_producer_is_refused", ISA,
          "            PakVmNodeId::Join => NodeAuthoringSurface::CanonicalProgramImageOnly,\n",
          "Join: authoring_surface() names no lawful producer",
          "")
    probe("join_reentering_the_batql_member_list_is_rejected",
          "docs/13_BATQL_CONTRACT.md",
          "Source, subject, snapshot, time range, filter, fold, match, partition, "
          "group, aggregate, order, page, project, proof request.",
          "as a BatQL semantic-algebra member",
          "Source, subject, snapshot, time range, filter, fold, match, partition, "
          "group, aggregate, join, order, page, project, proof request.")

    # --- work-formula lineage, not coverage -----------------------------------
    # Coverage cannot catch these: the wrong family still claims its unit.
    probe("windowing_accounted_by_returned_output_does_not_admit", ISA,
          "        class: PakVmNodeClass::Windowing,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,",
          "accounts in Outputs, off the QueryDataflow plane",
          "        class: PakVmNodeClass::Windowing,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::EmittedOutputs,")
    probe("effect_node_with_a_query_cost_law_does_not_admit", ISA,
          "        class: PakVmNodeClass::DurableAppend,\n        algebra: PakVmAlgebra::Effect,\n        work_formula: WorkFormulaFamily::DeclaredEffects,",
          "accounts in Rows, off the Effect plane",
          "        class: PakVmNodeClass::DurableAppend,\n        algebra: PakVmAlgebra::Effect,\n        work_formula: WorkFormulaFamily::Rows,")
    probe("query_node_with_an_unrelated_real_unit_does_not_admit", ISA,
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,",
          "accounts in Artifacts, off the QueryDataflow plane",
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::StagedArtifacts,")
    # A unit on two planes would let a family launder cost through whichever
    # plane it likes; a unit on none could never be accounted at all.
    probe("a_work_unit_on_two_planes_is_rejected", ISA,
          "            PakVmAlgebra::Effect => &[WorkUnit::Effects, WorkUnit::Artifacts, WorkUnit::Outputs],",
          "sits on more than one algebra plane",
          "            PakVmAlgebra::Effect => &[WorkUnit::Effects, WorkUnit::Artifacts, WorkUnit::Outputs, WorkUnit::Rows],")

    # --- the effect-posture biconditional --------------------------------------
    # These mutate the ISA and then REGENERATE the projection before auditing.
    # Auditing the stale projection would only prove drift detection works, which
    # is what this rule's absence already looked like: before it existed, an
    # effectful query algebra was reported as a drifted row and vanished entirely
    # the moment someone reran the generator.
    def law_probe(name, old, new, needle):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, ISA, old)
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            d07 = tmp / D07
            text = d07.read_text(encoding="utf-8")
            for marker, render in (("PAKVM-SEMANTIC-ISA", project.render_pakvm_isa),
                                   ("PAKVM-SIGNATURES", project.render_pakvm_signatures)):
                pat = project.block_pattern(marker)
                text = pat.sub(lambda m, render=render: m.group(1) + render(tmp) + m.group(3), text)
            d07.write_text(text, encoding="utf-8")
            produced = pf(tmp)
            if any("drifted" in f for f in produced):
                fail(f"{name} (regeneration left drift; the probe is testing the wrong rule)")
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    law_probe("an_effectful_query_algebra_is_rejected_after_regeneration",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),",
              "admits as Effectful outside the Effect algebra")
    law_probe("a_pure_effect_algebra_is_rejected_after_regeneration",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),\n        capability",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),\n        capability",
              "does not admit as Effectful, so it would pass the pure-query validator")

    # --- the semantic layer never acquires an encoding -------------------------
    probe("an_explicit_discriminant_is_rejected", ISA,
          "pub enum PakVmNodeId {\n    // Formula and decision (docs/07: 16)\n    Literal,",
          "assigns an explicit discriminant",
          "pub enum PakVmNodeId {\n    // Formula and decision (docs/07: 16)\n    Literal = 0,")
    probe("a_repr_on_the_semantic_node_is_rejected", ISA,
          "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum PakVmNodeId {",
          "fixes a semantic node's representation",
          "#[repr(u8)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum PakVmNodeId {")

    # --- an internal lowering identity is not a semantic node ------------------
    # The lowering line is byte-identical in all three algebra policies, so the
    # anchor carries the unique line above it. must_replace rewrites the first
    # match: an ambiguous anchor would mutate whichever policy happens to come
    # first and the probe would drift silently the day the order changes.
    probe("an_internal_lowering_identity_does_not_admit", ISA,
          QD_EVIDENCE + PUBLIC_LOWERING,
          "proposes the non-semantic lowering InternalLoweringIdentity",
          QD_EVIDENCE + INTERNAL_LOWERING)

    # --- provenance ------------------------------------------------------------
    probe("a_node_with_an_empty_authored_name_does_not_admit", ISA,
          '            PakVmNodeId::Literal => "literal",',
          "states an empty name",
          '            PakVmNodeId::Literal => "",')
    probe("a_node_with_no_source_origin_does_not_admit", ISA,
          '            PakVmAlgebra::Effect => "docs/07 Three instruction algebras: Effects",',
          "names no source origin",
          '            PakVmAlgebra::Effect => "",')

    # --- projection drift ------------------------------------------------------
    probe("a_projected_row_edited_by_hand_is_rejected", D07,
          "| literal | FormulaDecision | ScalarComputation | Pure |",
          "drifted from its admitted spec",
          "| literal | FormulaDecision | ScalarComputation | Effectful |")
    probe("a_projected_row_deleted_by_hand_is_rejected", D07,
          "| binding | FormulaDecision | ScalarComputation | Pure | None | ConstantWork | ConstantInstruction | Instructions | ReferenceInterpreterModel |\n",
          "projects 35 PakVM nodes, 36 admit",
          "")
    probe("a_signature_deleted_by_hand_is_rejected", D07,
          "    operands  none; a constant-pool reference",
          "signature projection omits the operand sorts",
          "    operands  ")

    # --- a test injection cannot enter the projection or gain provenance -------
    with isolated_tree() as tmp:
        d07 = tmp / D07
        d07.write_text(d07.read_text(encoding="utf-8").replace(
            "| literal | FormulaDecision |",
            "| injected_micro_op | FormulaDecision | ScalarComputation | Pure | None | "
            "ConstantWork | ConstantInstruction | Instructions | ReferenceInterpreterModel |\n"
            "| literal | FormulaDecision |", 1), encoding="utf-8")
        if not any("PakVM nodes" in f or "drifted" in f for f in pf(tmp)):
            fail("an_injected_projection_row_is_rejected")

    # --- the generator refuses to project an unadmitted node -------------------
    # A projector that completed a missing field would reintroduce the exact
    # defect this pass exists to remove.
    with isolated_tree() as tmp:
        _gen_refusal_old = "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),"
        path = tmp / _resolve_edit_carrier(root, ISA, _gen_refusal_old)
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            _gen_refusal_old,
            "        effect: PakVmRule::ClassDeclared,",
            "generator refusal"), encoding="utf-8")
        try:
            project.pakvm_specs(tmp)
            fail("the_generator_refuses_to_project_an_unadmitted_node")
        except project.Unadmitted:
            pass

    # --- detector neutering ----------------------------------------------------
    # Silencing the plane rule must not leave a green suite. Uses the canonical
    # neutered_validator helper: the real audit.py is never opened for writing, so
    # a probe that dies mid-mutation cannot leave a disabled rule behind.
    with isolated_tree() as tmp:
        p2 = tmp / _resolve_edit_carrier(root, ISA, WINDOWING_ROWS)
        p2.write_text(must_replace(
            p2.read_text(encoding="utf-8"),
            WINDOWING_ROWS, WINDOWING_OUTPUTS,
            "cross-plane mutation"), encoding="utf-8")
        # the live rule sees it...
        if not any("off the QueryDataflow plane" in f for f in pf(tmp)):
            fail("the_plane_rule_bites_before_it_is_neutered")
        # ...and the neutered rule does not, which is what makes it a real rule.
        with neutered_validator("audit", "        stray = [u for u in units if u not in plane]",
                                "        stray = []") as neutered:
            if any("off the QueryDataflow plane" in f
                   for f in neutered.pakvm_isa_findings(tmp)):
                fail("neutering_the_plane_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings


def test_syncbat_firewall(audit, project) -> list[str]:
    """SyncBat authority firewall: ownership, crossing legality, projection (5.5D4c2).

    Every probe runs the full sequence: mutate the authoritative source, prove
    the bytes moved, REGENERATE the projection, refuse a drift alibi, then
    require the exact finding the rule names. Auditing a stale projection would
    only prove drift detection works -- and drift detection working is precisely
    what the PakVM effect biconditional looked like right before it turned out to
    be enforcing nothing at all.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    FW = "spec/syncbat_firewall.rs"
    D08 = "docs/08_SYNCBAT_RUNTIME.md"
    ff = audit.syncbat_firewall_findings

    OWNS_COMPOSITION = (
        "            SyncBatAuthority::CompositionAndInstanceIdentity => SyncBatPlane::World,")
    STEALS_COMPOSITION = (
        "            SyncBatAuthority::CompositionAndInstanceIdentity => SyncBatPlane::Port,")
    RETRY_ARM = (
        "            | SyncBatAuthority::RetryRestartAuthority => SyncBatPlane::Runtime,")
    OWNERLESS_ARM = "            => SyncBatPlane::Runtime,"
    EFFECT_ROUTE = (
        "    SyncBatCrossing {\n"
        "        from: SyncBatPlane::PakVm,\n"
        "        to: SyncBatPlane::Bvisor,\n"
        "        carries: SyncBatAuthority::TypedEffectRequest,\n"
        "        posture: CrossingPosture::Required,\n"
        '        law: "an effectful node emits a typed request for physical admission",\n'
        "    },\n")

    def regenerate(tmp) -> bool:
        """Make docs/08 current with the mutated spec. False when the generator
        refuses, which is itself lawful: a projector that completed a broken fact
        would be the defect. The caller then knows drift is expected."""
        d = tmp / D08
        text = d.read_text(encoding="utf-8")
        try:
            for marker, render in (
                    ("SYNCBAT-PLANE-OWNERSHIP", project.render_syncbat_plane_ownership),
                    ("SYNCBAT-AUTHORITIES", project.render_syncbat_authorities),
                    ("SYNCBAT-CROSSINGS", project.render_syncbat_crossings)):
                body = render(tmp)
                text = project.block_pattern(marker).sub(
                    lambda m, body=body: m.group(1) + body + m.group(3), text)
        except project.Unadmitted:
            return False
        d.write_text(text, encoding="utf-8")
        return True

    def probe(name, rel, old, needle, new="", regen=True):
        with isolated_tree() as tmp:
            path = tmp / _resolve_edit_carrier(root, rel, old)
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            regenerated = regenerate(tmp) if regen else False
            produced = ff(tmp)
            if regenerated and any("drifted" in f for f in produced):
                fail(f"{name} (regeneration left drift; the probe is testing the wrong rule)")
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    # The premise. Without it every probe below could be red for free.
    if ff(root):
        fail(f"syncbat_firewall_contract_passes (got {ff(root)!r})")
    try:
        auditor = audit.syncbat_firewall_views(root)
        generator = project.syncbat_firewall_facts(root)
    except Exception as exc:
        auditor, generator = {}, {}
        fail(f"both_derivations_admit_the_firewall ({exc!r})")
    if auditor and generator:
        # Independence is only worth something if the two agree on the ANSWER
        # while sharing no code to compute it.
        if auditor["listed_planes"] != generator["order"]:
            fail("independent_derivations_agree_on_the_plane_inventory")
        if auditor["listed"] != [a["authority"] for a in generator["authorities"]]:
            fail("independent_derivations_agree_on_the_authority_inventory")
        for a in generator["authorities"]:
            if auditor["owners"].get(a["authority"]) != a["owner"]:
                fail(f"independent_derivations_agree_on_the_owner_of_{a['authority']}")
                break
        if ([(c["from"], c["to"], c["carries"]) for c in auditor["crossings"]]
                != [(c["from"], c["to"], c["carries"]) for c in generator["crossings"]]):
            fail("independent_derivations_agree_on_the_crossing_whitelist")
        # A rule quantified over an empty set is vacuously true forever. No count
        # is asserted here; the sets must merely be inhabited.
        if not auditor["crossings"] or not auditor["semantic"] or not auditor["planes"]:
            fail("the_firewall_is_inhabited")

    # --- one owner per authority, never two and never none --------------------
    probe("an_authority_with_two_owners_does_not_admit", FW,
          "            SyncBatAuthority::CapabilityAdmission\n"
          "            | SyncBatAuthority::PhysicalAdmission",
          "owner() answers twice for LogicalLegality",
          "            SyncBatAuthority::LogicalLegality\n"
          "            | SyncBatAuthority::CapabilityAdmission\n"
          "            | SyncBatAuthority::PhysicalAdmission")
    probe("an_ownerless_authority_does_not_admit", FW,
          RETRY_ARM,
          "authority RetryRestartAuthority names no owning plane",
          OWNERLESS_ARM)
    # The reading rustc cannot perform. An authority left out of the listing
    # compiles perfectly and vanishes from every law that walks the listing:
    # seedcheck goes on proving things about the authorities it was handed and
    # never mentions the one it was not.
    probe("an_authority_missing_from_the_listing_is_found", FW,
          "    SyncBatAuthority::RetryRestartAuthority,\n"
          "    SyncBatAuthority::SemanticNodeInterpretation,",
          "is authored but SYNCBAT_AUTHORITIES omits it",
          "    SyncBatAuthority::SemanticNodeInterpretation,")

    # --- crossing legality, surviving regeneration ----------------------------
    # docs/08's own forbidden example: "Bvisor minting semantic restart
    # authorization". The projection is made perfectly current first; the finding
    # must still be there, because the rule reads the spec and not the document.
    probe("a_forbidden_crossing_is_rejected_after_regeneration", FW,
          "pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[\n",
          "lets Bvisor exercise RetryRestartAuthority, which Runtime owns",
          "pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[\n"
          "    SyncBatCrossing {\n"
          "        from: SyncBatPlane::Bvisor,\n"
          "        to: SyncBatPlane::Runtime,\n"
          "        carries: SyncBatAuthority::RetryRestartAuthority,\n"
          "        posture: CrossingPosture::Required,\n"
          '        law: "bvisor mints semantic restart authorization",\n'
          "    },\n")
    # Deleting a lawful crossing is the quiet one. Every other rule iterates the
    # whitelist, so a removal does not fail them -- it gives them less to say.
    # Only the carrier rule notices that an authority which must name a PakVM
    # origin now has nowhere to carry it.
    probe("a_deleted_semantic_route_is_rejected_after_regeneration", FW,
          EFFECT_ROUTE,
          "TypedEffectRequest must name a PakVM origin but no required crossing carries it off PakVm",
          "")
    probe("a_noncanonical_plane_identity_is_rejected", FW,
          "        from: SyncBatPlane::World,\n        to: SyncBatPlane::Runtime,",
          "names 'Kernel', which is no SyncBat plane",
          "        from: SyncBatPlane::Kernel,\n        to: SyncBatPlane::Runtime,")

    # --- a stale projection is not an alibi, and a fresh one is not absolution -
    # The same mutation, audited both ways. Before regeneration the semantic
    # finding must already be present rather than hiding behind a drift report;
    # after regeneration it must still be present rather than vanishing with it.
    for name, regen in (("a_stale_projection_does_not_mask_an_ownership_transfer", False),
                        ("a_regenerated_projection_does_not_launder_an_ownership_transfer", True)):
        probe(name, FW, OWNS_COMPOSITION,
              "lets World exercise CompositionAndInstanceIdentity, which Port owns",
              STEALS_COMPOSITION, regen=regen)

    # --- a hand-authored copy is a second answer ------------------------------
    probe("a_hand_authored_inventory_competing_with_the_projection_is_found", D08,
          "Forbidden examples:",
          "is also hand-authored outside the generated block",
          "```text\nruntime owns logical legality\n```\n\nForbidden examples:")

    # --- the generator refuses; it never completes -----------------------------
    # A projector that could supply a missing owner would be a second authority
    # wearing a serializer's name.
    with isolated_tree() as tmp:
        path = tmp / FW
        path.write_text(must_replace(path.read_text(encoding="utf-8"), RETRY_ARM,
                                     OWNERLESS_ARM, "generator refusal"), encoding="utf-8")
        try:
            project.syncbat_firewall_facts(tmp)
            fail("the_generator_refuses_to_project_an_ownerless_authority")
        except project.Unadmitted:
            pass

    # --- detector neutering ----------------------------------------------------
    # Silencing the ownership rule must not leave a green suite. Without this the
    # probes above prove only that SOMETHING returned a string.
    with isolated_tree() as tmp:
        p2 = tmp / FW
        p2.write_text(must_replace(p2.read_text(encoding="utf-8"), OWNS_COMPOSITION,
                                   STEALS_COMPOSITION, "ownership mutation"), encoding="utf-8")
        if not any("which Port owns" in f for f in ff(tmp)):
            fail("the_ownership_rule_bites_before_it_is_neutered")
        with neutered_validator("audit", "        if owner and owner != frm:",
                                "        if False:") as neutered:
            if any("which Port owns" in f for f in neutered.syncbat_firewall_findings(tmp)):
                fail("neutering_the_ownership_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings


def test_reconciliation(audit, project) -> list[str]:
    """DEC-075 single-writer dual-axis reconciliation: the auditor's half
    (5.5E1d). The four-signal retry fence itself is executed Rust inside
    seedcheck; here the composition's structure, carriers, and projections are
    attacked through the auditor.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    if audit.recon_findings(root):
        fail(f"reconciliation_contract_passes (got {audit.recon_findings(root)!r})")
    RS = "spec/reconciliation.rs"
    # A role bound twice is one name owning two authorities -- the HlcPoint
    # disease this decision exists to prevent.
    with isolated_tree() as tmp:
        p = tmp / RS
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "        role: ReconciliationRole::PhysicalIdentity,",
            "        role: ReconciliationRole::LogicalIdentity,",
            "rebind a role"), encoding="utf-8")
        got = audit.recon_findings(tmp)
        if not any("five distinct roles" in f for f in got):
            fail(f"a_rebound_reconciliation_role_is_refused (got {got!r})")
    # A hand-edited projection is drift, not authority.
    with isolated_tree() as tmp:
        p = tmp / "docs/02_SYSTEM_MODEL.md"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "| PhysicalIdentity | AttemptId |",
            "| PhysicalIdentity | AttemptId, Hlc |",
            "drift the coordinates block"), encoding="utf-8")
        if not any("drifted" in f for f in audit.recon_findings(tmp)):
            fail("a_drifted_reconciliation_block_is_caught")
    # A chronology carrier the time contract does not own would be a second
    # time authority minted by the composition.
    with isolated_tree() as tmp:
        p = tmp / RS
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            '"Hlc", "ObservedWallTime"', '"Hlc", "WallStamp"',
            "invent a carrier"), encoding="utf-8")
        got = audit.recon_findings(tmp)
        if not any("not owned by docs/16" in f for f in got):
            fail(f"an_invented_chronology_carrier_is_refused (got {got!r})")
    # DEC-058's seal inventory (5.5E1e): one owner, projected, never restated.
    if audit.release_seal_findings(root):
        fail(f"release_seal_contract_passes (got {audit.release_seal_findings(root)!r})")
    with isolated_tree() as tmp:
        p = tmp / "spec/architecture/policy.rs"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "    ReleaseSealField::KernelQualificationSet,\n", "",
            "drop the kernel set"), encoding="utf-8")
        got = audit.release_seal_findings(tmp)
        if not any("never disappears" in f for f in got):
            fail(f"dropping_the_kernel_set_from_the_seal_is_refused (got {got!r})")
    with isolated_tree() as tmp:
        p = tmp / "docs/36_PUBLIC_API_CI_AND_RELEASE.md"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"), "\nSbom\n", "\n",
            "hand-trim the projection"), encoding="utf-8")
        if not any("drifted" in f for f in audit.release_seal_findings(tmp)):
            fail("a_trimmed_seal_projection_is_caught")
    with isolated_tree() as tmp:
        p = tmp / "spec/dispositions.rs"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "binds every field of the typed ReleaseSealField inventory",
            "binds source tree toolchain and SBOM",
            "restate the list"), encoding="utf-8")
        got = audit.release_seal_findings(tmp)
        if not any("instead of naming the typed" in f for f in got):
            fail(f"a_decision_restating_the_seal_list_is_refused (got {got!r})")
    findings.extend(canonical_drift(before))
    return findings


def test_syncbat_requiredness(audit, project) -> list[str]:
    """SyncBat crossing requiredness: liveness, not just safety (5.5D4c2f).

    Safety proves no unauthorized door exists. Requiredness proves the authorized
    doors have not been bricked over. A stricter firewall that deletes a required
    execution path preserves safety by destroying liveness, and that is still a
    specification failure. Every probe mutates the authoritative source, proves
    the bytes moved, REGENERATES the projection, refuses a drift alibi, and
    requires the exact semantic finding -- deletion of a required route must
    survive a perfectly current document.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    FW = "spec/syncbat_firewall.rs"
    D08 = "docs/08_SYNCBAT_RUNTIME.md"
    ff = audit.syncbat_firewall_findings

    import re as _re
    fwsrc = (root / FW).read_text(encoding="utf-8")
    const = fwsrc[fwsrc.index("pub const SYNCBAT_LEGAL_CROSSINGS"):]
    const = const[: const.index("\n];") + 3]
    rows = _re.findall(r"    SyncBatCrossing \{.*?\n    \},\n", const, _re.S)
    if len(rows) != 7:
        fail(f"the_firewall_declares_the_expected_crossing_rows (found {len(rows)})")
    crossings = []
    for b in rows:
        crossings.append({
            "block": b,
            "from": _re.search(r"from: SyncBatPlane::(\w+)", b).group(1),
            "to": _re.search(r"to: SyncBatPlane::(\w+)", b).group(1),
            "carries": _re.search(r"carries: SyncBatAuthority::(\w+)", b).group(1),
        })

    def regenerate(tmp) -> bool:
        d = tmp / D08
        text = d.read_text(encoding="utf-8")
        try:
            for marker, render in (
                    ("SYNCBAT-PLANE-OWNERSHIP", project.render_syncbat_plane_ownership),
                    ("SYNCBAT-AUTHORITIES", project.render_syncbat_authorities),
                    ("SYNCBAT-CROSSINGS", project.render_syncbat_crossings)):
                body = render(tmp)
                text = project.block_pattern(marker).sub(
                    lambda m, body=body: m.group(1) + body + m.group(3), text)
        except project.Unadmitted:
            return False
        d.write_text(text, encoding="utf-8")
        return True

    def semantic_probe(name, new_fwsrc, needle):
        """Apply a whole-file firewall mutation, regenerate, require a NON-drift
        finding matching needle."""
        with isolated_tree() as tmp:
            path = tmp / FW
            cur = path.read_text(encoding="utf-8")
            if new_fwsrc == cur:
                fail(f"{name} (mutation changed no bytes)")
                return
            path.write_text(new_fwsrc, encoding="utf-8")
            regenerate(tmp)
            produced = ff(tmp)
            semantic = [f for f in produced if "drifted" not in f]
            if not any(needle in f for f in semantic):
                fail(f"{name} (wanted non-drift {needle!r}, got {produced!r})")

    # The premise: today the firewall admits and every route is Required.
    if ff(root):
        fail(f"syncbat_requiredness_contract_passes (got {ff(root)!r})")

    # --- 1 & 2: deleting each Required crossing is a semantic finding, not drift
    for c in crossings:
        semantic_probe(
            f"deleting_the_{c['carries']}_route_is_a_semantic_finding",
            fwsrc.replace(c["block"], "", 1),
            "required")  # every deletion message names a missing/downgraded required route

    # --- 3: Required -> Optional is caught when no alternate route exists -------
    for c in crossings:
        downgraded = c["block"].replace(
            "posture: CrossingPosture::Required,", "posture: CrossingPosture::Optional,")
        semantic_probe(
            f"downgrading_the_{c['carries']}_route_to_optional_is_caught",
            fwsrc.replace(c["block"], downgraded, 1),
            "required")

    # --- 5: swapping a crossing's direction is refused -------------------------
    # World -> Runtime becomes Runtime -> World; World no longer owns what it now
    # purports to send, and the ownership rule refuses it.
    swap = crossings[0]
    swapped = (swap["block"]
               .replace(f"from: SyncBatPlane::{swap['from']}", "from: SyncBatPlane::__TMP__")
               .replace(f"to: SyncBatPlane::{swap['to']}", f"to: SyncBatPlane::{swap['from']}")
               .replace("from: SyncBatPlane::__TMP__", f"from: SyncBatPlane::{swap['to']}"))
    semantic_probe("swapping_a_crossing_direction_is_refused",
                   fwsrc.replace(swap["block"], swapped, 1),
                   f"lets {swap['to']} exercise {swap['carries']}")

    # --- 6: keeping the endpoints but changing the carried authority is refused -
    c1 = crossings[1]  # Runtime -> PakVm, SemanticAuthorization
    reauth = c1["block"].replace(
        "carries: SyncBatAuthority::SemanticAuthorization,",
        "carries: SyncBatAuthority::AttemptEvidence,")
    semantic_probe("changing_the_carried_authority_is_refused",
                   fwsrc.replace(c1["block"], reauth, 1),
                   "lets Runtime exercise AttemptEvidence")

    # --- 7: a required route cannot be laundered by a similar-looking receipt ---
    # Delete the AttemptEvidence return, then add a Bvisor -> Runtime crossing
    # carrying SemanticReceipt -- a superficially similar "receipt". Bvisor does
    # not own SemanticReceipt, so the substitute is refused AND the connectivity
    # graph counts only legal edges, so it cannot reconnect Bvisor to the root.
    evidence = next(c for c in crossings if c["carries"] == "AttemptEvidence")
    substitute = (
        "    SyncBatCrossing {\n"
        "        from: SyncBatPlane::Bvisor,\n"
        "        to: SyncBatPlane::Runtime,\n"
        "        carries: SyncBatAuthority::SemanticReceipt,\n"
        "        posture: CrossingPosture::Required,\n"
        '        law: "a receipt that looks like evidence",\n'
        "    },\n")
    laundered = fwsrc.replace(evidence["block"], substitute, 1)
    with isolated_tree() as tmp:
        (tmp / FW).write_text(laundered, encoding="utf-8")
        regenerate(tmp)
        produced = ff(tmp)
        if not any("lets Bvisor exercise SemanticReceipt" in f for f in produced):
            fail(f"a_similar_receipt_cannot_launder_a_required_route:substitute_refused "
                 f"(got {produced!r})")
        if not any("Bvisor cannot reach the logical root" in f for f in produced):
            fail(f"a_similar_receipt_cannot_launder_a_required_route:route_still_missing "
                 f"(got {produced!r})")

    # --- 8: neutering the requiredness detector must not leave a green suite ----
    deleted = fwsrc.replace(
        next(c for c in crossings if c["carries"] == "PhysicalAttempt")["block"], "", 1)
    with isolated_tree() as tmp:
        (tmp / FW).write_text(deleted, encoding="utf-8")
        regenerate(tmp)
        if not any("cannot reach plane Port" in f for f in ff(tmp)):
            fail("the_requiredness_rule_bites_before_it_is_neutered")
        with neutered_validator(
                "audit",
                "            if plane != source and not reaches(logical_root, plane):",
                "            if False:") as neutered:
            if any("cannot reach plane Port" in f
                   for f in neutered.syncbat_firewall_findings(tmp)):
                fail("neutering_the_requiredness_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings

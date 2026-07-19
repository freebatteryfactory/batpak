"""Verification-plane and candidate-sprouting audits (5.5F2/F3).

Independently reconstructs the frozen verification axis vocabularies, the
shape-based anti-ladder guard, plan admissibility, route-kind closure, and the
candidate-sprouting plane (frozen vocabularies, bounded search budget,
promotion-plan admission). Shares no parser with bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .corpus import (
    A_E4D_ACTIVE,
    _bootstrap_rust_source,
    _bootstrap_source,
    _spec_module_source,
    batql_extract_block,
)

# 5.5F2 (DEC-077/DEC-078, docs/38): the verification plane's frozen axis
# vocabularies. Distinct typed axes, never one ordered ladder — the guard
# below is SHAPE-based, so a synonym ladder (ConfidenceTier, TrustGrade)
# walks into the same refusal as the named ones.
A_VERIFICATION_AXES = {
    "VerificationBasis": {"ContractProjection", "IndependentReference",
                          "DirectBoundary", "RuntimeObservation"},
    "VerificationMethod": {"StructuralRule", "CompileRefusal", "PropertySequence",
                           "BoundedStateExploration", "ScheduleExploration",
                           "DeterministicSimulation", "DifferentialExecution",
                           "TranslationValidation", "FaultInjection", "CrashRecovery",
                           "Fuzzing", "Mutation", "ComplexityContract",
                           "BenchmarkEnvelope", "HistoryReplay"},
    "VerificationClaimKind": {"Safety", "Liveness", "BoundedResponse", "Convergence",
                              "Stability", "NonOscillation", "Determinism",
                              "Refinement", "Conformance", "ResourceEnvelope"},
    "VerificationCoverage": {"Sampled", "Bounded", "ExhaustiveWithinDeclaredModel",
                             "ObservedHistory"},
    "VerificationEnforcementPosture": {"Blocking", "Quarantine", "Advisory"},
    "VerificationLane": {"PullRequestFast", "Merge", "Scheduled", "Release", "Runtime"},
    "IndependentEvidenceRouteKind": {"DifferentialImplementation", "HostileBoundary",
                                     "IndependentHistoryReplay"},
    "RuntimeVerificationMode": {"PreventiveGuard", "InBandObservation", "OfflineReplay"},
    "RuntimeVerificationDisposition": {"GuardAdmitted", "GuardRefused",
                                       "NoDivergenceObserved",
                                       "ConformantForObservedHistory", "Divergent",
                                       "Incomplete", "Stale", "Unsupported"},
}
# Tool names are receipts, never methods: the one lawful tool enum names the
# repository's own bootstrap tools, and a verification tool vocabulary is a
# ladder rung wearing a lab coat.
A_VERIFICATION_TOOL_NAMES = ("VerificationToolId", "LoomMethod", "KaniMethod",
                             "TlaMethod", "StaterightMethod")
A_VERIFICATION_PLAN = re.compile(
    r'pub const (PLAN_[A-Z0-9_]+): &\[VerificationRequirement\] = &\[(.*?)\];',
    re.S)
A_VERIFICATION_REQ = re.compile(
    r'VerificationRequirement \{ method: VerificationMethod::(\w+), '
    r'basis: (VerificationBasis::\w+(?: \{ route: (?:None|Some\(IndependentEvidenceRouteKind::\w+\)|IndependentEvidenceRouteKind::\w+) \})?), '
    r'coverage: VerificationCoverage::(\w+), lane: VerificationLane::(\w+), '
    r'enforcement: VerificationEnforcementPosture::(\w+) \}')


def _a_basis_variant(basis: str) -> str:
    """The bare VerificationBasis variant name from a full basis literal."""
    return re.match(r"VerificationBasis::(\w+)", basis).group(1)


def _a_basis_route(basis: str) -> str | None:
    """The route kind carried inside a VerificationBasis payload, or None."""
    m = re.search(r"IndependentEvidenceRouteKind::(\w+)", basis)
    return m.group(1) if m else None


def _a_render_basis(basis: str) -> str:
    """The audit's independent basis cell render, byte-identical to
    bootstrap/project.py's _render_basis: the route folds into the variant."""
    variant = _a_basis_variant(basis)
    route = _a_basis_route(basis)
    return f"{variant}({route})" if route else variant


def verification_findings(root: Path) -> list[str]:
    """The typed verification plane, independently reconstructed: frozen axis
    vocabularies, the shape-based anti-ladder guard, tool-neutral methods,
    plan admissibility, and route kinds closed by adoption."""
    out: list[str] = []
    vsrc = _spec_module_source(root, "verification")
    psrc = (root / "spec/proof.rs").read_text(encoding="utf-8")
    # Frozen axes: each enum exists with exactly its ruled variants. The
    # anchor accepts both payload-free variants (`Variant,`) and the payload
    # variants VerificationBasis now carries (`Variant { route: .. },`).
    for axis, want in A_VERIFICATION_AXES.items():
        m = re.search(r"pub enum " + re.escape(axis) + r" \{(.*?)\n\}", vsrc, re.S)
        got = set(re.findall(r"^\s{4}(\w+)(?:,| \{)", m.group(1), re.M)) if m else set()
        if not got:
            out.append(f"spec/verification.rs declares no {axis}")
        elif got != want:
            out.append(f"{axis} variants {sorted(got)} != frozen {sorted(want)}")
    # Shape guard: no ordering, ranking, scoring, or numeric conversion on
    # any verification axis or aggregate.
    for pattern, law in (
        (r"derive\(([^)]*\b(?:PartialOrd|Ord)\b[^)]*)\)",
         "derives an ordering on a verification type"),
        (r"impl\s+(?:PartialOrd|Ord)\b", "implements an ordering on a verification type"),
        (r"fn\s+(?:rank|level|assurance_score|score)\b",
         "exposes a rank/level/score on the verification plane"),
        (r"\bas\s+(?:u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|f32|f64)\b",
         "numerically converts a verification axis"),
    ):
        if re.search(pattern, vsrc):
            out.append(f"spec/verification.rs {law}; the axes are not a ladder (DEC-077)")
    # Tool-neutral methods: a tool-name vocabulary may not exist anywhere in
    # the spec as a declaration.
    for rel in sorted((root / "spec").rglob("*.rs")):
        src_i = rel.read_text(encoding="utf-8")
        for name in A_VERIFICATION_TOOL_NAMES:
            if re.search(r"^\s*pub (?:enum|struct|type|const) " + name + r"\b", src_i, re.M):
                out.append(f"{rel.name} declares tool vocabulary {name}; methods stay "
                           "tool-neutral and tools are receipts")
    # The retired spelling stays retired.
    tool_sources = {
        "bootstrap/seedcheck.rs": _bootstrap_rust_source(root, "seedcheck"),
        "bootstrap/audit.py": _bootstrap_source(root, "audit"),
        "spec/proof.rs": (root / "spec/proof.rs").read_text(encoding="utf-8")
            if (root / "spec/proof.rs").is_file() else "",
    }
    for rel in ("spec/proof.rs", "bootstrap/seedcheck.rs", "bootstrap/audit.py"):
        if re.search(r"\bcounts_green\s*\(", tool_sources[rel]):
            out.append(f"{rel} still calls counts_green; the seam is "
                       "is_positive_semantic_terminal (5.5F2)")
    # Named plans: parse, admit, and require nonempty duplicate-free tuples.
    plans: dict[str, list[tuple]] = {}
    for name, body in A_VERIFICATION_PLAN.findall(psrc):
        reqs = A_VERIFICATION_REQ.findall(body)
        raw_count = body.count("VerificationRequirement {")
        if raw_count != len(reqs):
            out.append(f"plan {name} carries a requirement the reconstruction cannot parse")
        if not reqs:
            out.append(f"plan {name} is empty; an active row cannot ride an empty plan")
        if len(reqs) != len(set(reqs)):
            out.append(f"plan {name} repeats a requirement")
        for method, basis, coverage, lane, enforcement in reqs:
            basis_variant = _a_basis_variant(basis)
            route = _a_basis_route(basis)
            for axis, value in (("VerificationMethod", method),
                                ("VerificationBasis", basis_variant),
                                ("VerificationCoverage", coverage),
                                ("VerificationLane", lane),
                                ("VerificationEnforcementPosture", enforcement)):
                if value not in A_VERIFICATION_AXES[axis]:
                    out.append(f"plan {name} names unknown {axis}::{value}")
            if route is not None and route not in \
                    A_VERIFICATION_AXES["IndependentEvidenceRouteKind"]:
                out.append(f"plan {name} names unknown route kind {route}")
            # Route/basis coherence (5.5F3): the route now lives INSIDE the
            # basis payload, and admission refuses every mismatch. An
            # independent reference rides differential implementation or
            # independent history replay; a direct boundary carries only the
            # hostile-boundary route; contract projection and runtime
            # observation structurally carry no route at all.
            if basis_variant == "IndependentReference":
                if route not in ("DifferentialImplementation",
                                 "IndependentHistoryReplay"):
                    out.append(f"plan {name} gives IndependentReference the route "
                               f"{route}; an independent reference rides only "
                               "differential implementation or independent history "
                               "replay")
            elif basis_variant == "DirectBoundary":
                if route is not None and route != "HostileBoundary":
                    out.append(f"plan {name} gives DirectBoundary the route {route}; "
                               "a direct boundary carries only the hostile-boundary "
                               "route")
            elif route is not None:
                out.append(f"plan {name} gives {basis_variant} a route; contract "
                           "projection and runtime observation carry no route")
            if basis_variant == "RuntimeObservation" and coverage != "ObservedHistory":
                out.append(f"plan {name} pairs runtime observation with {coverage}; "
                           "runtime observation covers observed history only")
            # ObservedHistory coverage is lawful ONLY with runtime observation
            # or an independent history replay (5.5F3 admission law).
            if coverage == "ObservedHistory" and not (
                    basis_variant == "RuntimeObservation"
                    or (basis_variant == "IndependentReference"
                        and route == "IndependentHistoryReplay")):
                out.append(f"plan {name} claims observed-history coverage without "
                           "runtime observation or independent history replay")
        plans[name] = reqs
    # Every active row references a declared plan and a ruled claim; route
    # kinds are closed by adoption.
    adopted_routes: set[str] = set()
    for m in A_E4D_ACTIVE.finditer(psrc):
        wid, claim, plan = m.group(1), m.group(5), m.group(6)
        if claim not in A_VERIFICATION_AXES["VerificationClaimKind"]:
            out.append(f"active proof row {wid} claims unknown kind {claim}")
        if plan not in plans:
            out.append(f"active proof row {wid} references undeclared plan {plan}")
        else:
            for _m, basis, _c, _l, _e in plans[plan]:
                route = _a_basis_route(basis)
                if route is not None:
                    adopted_routes.add(route)
    for kind in sorted(A_VERIFICATION_AXES["IndependentEvidenceRouteKind"] - adopted_routes):
        out.append(f"IndependentEvidenceRouteKind::{kind} has no active proof-row "
                   "adopter; a route kind arrives with its adopter, never before")
    # The runtime matrix in the typed source is the ruled matrix: the three
    # refused pairings must not appear in the admitting arms.
    matrix = re.search(r"pub const fn admits\(.*?\n    \}", vsrc, re.S)
    if not matrix:
        out.append("spec/verification.rs declares no admits() runtime matrix")
    else:
        arm = matrix.group(0)
        guard = re.search(r"PreventiveGuard => matches!\(\s*disposition,\s*([^)]*)\)", arm)
        inband = re.search(r"InBandObservation => matches!\(\s*disposition,\s*([^)]*)\)", arm)
        replay = re.search(r"OfflineReplay => matches!\(\s*disposition,\s*([^)]*)\)", arm)
        if not (guard and inband and replay):
            out.append("the admits() matrix does not classify all three runtime modes")
        else:
            if "ConformantForObservedHistory" in guard.group(1) + inband.group(1):
                out.append("a guard or in-band mode admits ConformantForObservedHistory; "
                           "only independent offline replay concludes conformance")
            if "NoDivergenceObserved" in replay.group(1):
                out.append("offline replay admits NoDivergenceObserved; replay concludes "
                           "for observed history, it does not narrate a live trace")
            if "GuardRefused" not in guard.group(1):
                out.append("the preventive guard cannot refuse; a guard needs no replay "
                           "before refusing")
    # The product verification plane never imports the Tier 0 cross-run
    # comparator as authority (naming it in prose as an analogy is lawful;
    # a use declaration is not).
    if re.search(r"use\s+(?:crate|spec)::tier0_cross_run", vsrc):
        out.append("spec/verification.rs imports tier0_cross_run; the Tier 0 law is an "
                   "analogy, never product verification authority")
    # The route now lives inside the VerificationBasis payload; the retired
    # parallel `independent_route` field may not return anywhere in the plane.
    if re.search(r"\bindependent_route\b", vsrc):
        out.append("spec/verification.rs reintroduces an independent_route field; the "
                   "route now lives inside the VerificationBasis payload (5.5F3)")
    # Enforcement is authored policy, never observed: the evidence-observation
    # struct carries no enforcement field.
    obs = re.search(r"pub struct RequirementEvidenceObservation \{(.*?)\n\}", vsrc, re.S)
    if not obs:
        out.append("spec/verification.rs declares no RequirementEvidenceObservation")
    elif re.search(r"^\s{4}pub enforcement:", obs.group(1), re.M):
        out.append("RequirementEvidenceObservation carries an enforcement field; "
                   "enforcement is authored policy, never observed (5.5F3)")
    # The renamed observation/assessment/runtime seams are present, and the
    # retired spellings stay retired across the spec crate.
    for fn in ("admit_observation", "assess_plan", "admit_runtime_verification"):
        if not re.search(r"\bfn " + fn + r"\s*\(", vsrc):
            out.append(f"spec/verification.rs declares no {fn}; the renamed 5.5F3 "
                       "verification seam is absent")
    for rel in sorted((root / "spec").rglob("*.rs")):
        src_i = rel.read_text(encoding="utf-8")
        for pattern, spelling in (
            (r"\bfn qualify\s*\(", "fn qualify"),
            (r"\bfn qualify_row\s*\(", "fn qualify_row"),
            (r"\bExecutedRequirementEvidence\b", "ExecutedRequirementEvidence"),
        ):
            if re.search(pattern, src_i):
                out.append(f"{rel.name} reintroduces retired spelling {spelling}; 5.5F3 "
                           "renamed the seam to admit_observation/assess_plan/"
                           "RequirementEvidenceObservation")
    # Runtime admission body law (5.5F3): the offline-replay arm requires the
    # independent history-replay route and observed history, and its conformant
    # disposition additionally requires a fresh, complete history. Verified by
    # regex over the admit_runtime_verification function body.
    admit_rt = re.search(
        r"pub const fn admit_runtime_verification\(.*?\n\}", vsrc, re.S)
    if not admit_rt:
        out.append("spec/verification.rs declares no admit_runtime_verification body")
    else:
        rt = admit_rt.group(0)
        for token, law in (
            ("IndependentHistoryReplay",
             "the offline-replay arm no longer requires the independent "
             "history-replay route"),
            ("ReplayRequiresObservedHistory",
             "the offline-replay arm no longer requires observed history"),
            ("ConformanceRequiresFreshHistory",
             "the conformant disposition no longer requires a fresh history"),
            ("ConformanceRequiresCompleteHistory",
             "the conformant disposition no longer requires a complete history"),
        ):
            if token not in rt:
                out.append(f"admit_runtime_verification: {law} (5.5F3 runtime law)")
    # VERIFICATION-PLANS parity: independently reconstruct the docs/38 block
    # exactly as bootstrap/project.py renders it, with the route folded into
    # the basis cell. No parser is shared with the generator.
    want_vp = ["| Plan | Method | Basis | Coverage | Lane | Enforcement |",
               "| --- | --- | --- | --- | --- | --- |"]
    for name, reqs in plans.items():
        for method, basis, coverage, lane, enforcement in reqs:
            want_vp.append(f"| {name} | {method} | {_a_render_basis(basis)} | "
                           f"{coverage} | {lane} | {enforcement} |")
    want_vp.append("")
    want_vp.append("| Active proof row | Claim | Plan |")
    want_vp.append("| --- | --- | --- |")
    for m in A_E4D_ACTIVE.finditer(psrc):
        want_vp.append(f"| {m.group(1)} | {m.group(5)} | {m.group(6)} |")
    d38 = (root / "docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md").read_text(
        encoding="utf-8")
    vp_body = batql_extract_block(d38, "VERIFICATION-PLANS")
    if vp_body is None:
        out.append("docs/38: missing generated block VERIFICATION-PLANS")
    elif vp_body != "\n".join(want_vp):
        out.append("docs/38: generated block VERIFICATION-PLANS does not match its "
                   "typed derivation")
    return out


# 5.5F3 (DEC-079..082/DEC-073, docs/39): the candidate-sprouting plane,
# independently reconstructed. Frozen candidate vocabularies, the four-field
# bounded search budget, the promotion-plan admission body law, the DEC-073
# specialized-plan policy, the Candidate identity registration, and the
# forbidden lineage/lifecycle/invalidation/manifest-version sweep. This
# auditor shares no parser with bootstrap/project.py.
A_SPROUTING_AXES = {
    "CandidateOriginKind": ("CANDIDATE_ORIGIN_KINDS",
        {"DeterministicGeneration", "BoundedSearch", "TransferReuse",
         "HumanAuthored", "MachineAssistedSynthesis", "RepairOfCandidate"}),
    "CandidateChangeClass": ("CANDIDATE_CHANGE_CLASSES",
        {"RealizationPreserving", "LawChanging"}),
    "EvaluationSetRole": ("EVALUATION_SET_ROLES",
        {"Search", "Qualification", "Holdout", "Regression"}),
    "RealizationPosture": ("REALIZATION_POSTURES",
        {"Missing", "Scaffold", "Candidate"}),
    "RepairAuthority": ("REPAIR_AUTHORITIES",
        {"Mechanical", "BoundedSearch", "ArchitectRequired"}),
}
A_SPROUTING_BUDGET_FIELDS = {"max_candidates", "max_logical_work",
                             "max_memory_bytes", "max_monotonic_ticks"}
A_SPROUTING_PROMOTION_REQUIREMENTS = {
    "IndependentEvidenceRoute", "NamedProofTarget", "QualifiedHostileEvidence",
    "AuditablePromotionReceipt"}
# Lineage and lifecycle are DERIVED from required manifest and receipt fields,
# never a parallel edge enum, lifecycle ladder, invalidation registry, or a
# manifest-version type minted before a serialized schema lands.
A_SPROUTING_FORBIDDEN = ("CandidateEvidenceRelation", "CandidateLifecycle",
                         "Invalidates", "CandidateManifestVersion")


def sprouting_findings(root: Path) -> list[str]:
    """The candidate-sprouting plane (BP-SPROUTING-1, docs/39), independently
    reconstructed: frozen vocabularies, the search budget's four NonZeroU64
    fields, the promotion-plan admission body law, the DEC-073 specialized-plan
    policy, the Candidate identity registration, and the forbidden-vocabulary
    sweep."""
    out: list[str] = []
    path = root / "spec/sprouting.rs"
    if not path.is_file():
        return ["missing spec/sprouting.rs"]
    src = path.read_text(encoding="utf-8")
    # Frozen candidate vocabularies: each enum and its all-const inventory
    # carry exactly the ruled variants. The line-anchored variant scan matches
    # the verification/claim-axis reconstructions and skips per-variant doc
    # comments.
    for kind, (const_name, want) in A_SPROUTING_AXES.items():
        m = re.search(r"pub enum " + kind + r" \{(.*?)\n\}", src, re.S)
        got = set(re.findall(r"^\s{4}(\w+)(?:,| \{)", m.group(1), re.M)) if m else set()
        if not got:
            out.append(f"spec/sprouting.rs declares no {kind}")
        elif got != want:
            out.append(f"{kind} variants {sorted(got)} != frozen {sorted(want)}")
        all_body = re.search(
            r"pub const " + const_name + r":[^=]*= &\[(.*?)\];", src, re.S)
        inv = set(re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1))) \
            if all_body else set()
        if not inv:
            out.append(f"spec/sprouting.rs declares no {const_name} inventory")
        elif inv != want:
            out.append(f"{const_name} inventory {sorted(inv)} != frozen {sorted(want)}")
    # The search budget carries exactly its four NonZeroU64 fields — no more,
    # no fewer, and none downgraded off NonZeroU64.
    budget = re.search(r"pub struct CandidateSearchBudget \{(.*?)\n\}", src, re.S)
    if not budget:
        out.append("spec/sprouting.rs declares no CandidateSearchBudget")
    else:
        nz = set(re.findall(r"pub (\w+): NonZeroU64", budget.group(1)))
        allf = set(re.findall(r"pub (\w+):", budget.group(1)))
        if nz != A_SPROUTING_BUDGET_FIELDS:
            out.append(f"CandidateSearchBudget NonZeroU64 fields {sorted(nz)} != "
                       f"frozen {sorted(A_SPROUTING_BUDGET_FIELDS)}")
        if allf != A_SPROUTING_BUDGET_FIELDS:
            out.append(f"CandidateSearchBudget declares fields {sorted(allf)}; the "
                       "budget carries exactly the four bounded NonZeroU64 fields")
    # admit_promotion_plan body law: measured against the conjunctive
    # PromotionRequirement::ALL denominator (no missing, no duplicate, no
    # unknown set), LawChanging forced onto ArchitectRequired, and
    # RealizationPreserving refusing ArchitectRequired.
    admit = re.search(r"pub (?:const )?fn admit_promotion_plan\(.*?\n\}", src, re.S)
    if not admit:
        out.append("spec/sprouting.rs declares no admit_promotion_plan body")
    else:
        body = admit.group(0)
        for token, law in (
            ("PromotionRequirement::ALL",
             "no longer measures the plan against the conjunctive "
             "PromotionRequirement::ALL denominator"),
            ("MissingRequirement", "no longer refuses a missing requirement"),
            ("DuplicateRequirement", "no longer refuses a duplicate requirement"),
            ("LawChangingRequiresArchitect",
             "no longer forces a law-changing candidate onto the "
             "architect-required authority path"),
            ("RealizationPreservingCannotRequireArchitect",
             "no longer refuses architect authority for a realization-"
             "preserving candidate"),
        ):
            if token not in body:
                out.append(f"admit_promotion_plan {law} (5.5F3 promotion-plan law)")
    # The DEC-073 specialized-plan candidate policy, reconstructed field by
    # field from the typed const.
    policy = re.search(
        r"pub const SPECIALIZED_PLAN_CANDIDATE_POLICY: SpecializedPlanCandidatePolicy =\s*"
        r"SpecializedPlanCandidatePolicy \{(.*?)\n\s*\};", src, re.S)
    if not policy:
        out.append("spec/sprouting.rs declares no SPECIALIZED_PLAN_CANDIDATE_POLICY")
    else:
        pbody = policy.group(1)
        owner = re.search(r'semantic_owner: ContractId\("([^"]+)"\)', pbody)
        basis = re.search(r'admission_basis: GuaranteeRef::dec\("([^"]+)"\)', pbody)
        change = re.search(r"change_class: CandidateChangeClass::(\w+)", pbody)
        route = re.search(
            r"independent_route: IndependentEvidenceRouteKind::(\w+)", pbody)
        if not owner or owner.group(1) != "BP-PAKVM-ISA-1":
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY does not name semantic "
                       "owner BP-PAKVM-ISA-1")
        if not basis or basis.group(1) != "DEC-073":
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY does not admit on DEC-073")
        if not change or change.group(1) != "RealizationPreserving":
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY is not RealizationPreserving")
        if not route or route.group(1) != "DifferentialImplementation":
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY does not select the "
                       "DifferentialImplementation route")
        origins = set(re.findall(r"CandidateOriginKind::(\w+)", pbody))
        if not origins and re.search(r"\bCANDIDATE_ORIGIN_KINDS\b", pbody):
            origins = set(A_SPROUTING_AXES["CandidateOriginKind"][1])
        if origins != A_SPROUTING_AXES["CandidateOriginKind"][1]:
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY does not admit all six "
                       "candidate origins")
        reqs = set(re.findall(r"PromotionRequirement::(\w+)", pbody))
        if reqs != {"ALL"} and reqs != A_SPROUTING_PROMOTION_REQUIREMENTS:
            out.append("SPECIALIZED_PLAN_CANDIDATE_POLICY does not carry the full "
                       "conjunctive PromotionRequirement::ALL denominator")
    # No lineage/lifecycle/invalidation/manifest-version vocabulary is DECLARED
    # anywhere in the spec crate.
    for rel in sorted((root / "spec").rglob("*.rs")):
        src_i = rel.read_text(encoding="utf-8")
        for name in A_SPROUTING_FORBIDDEN:
            if re.search(r"^\s*pub (?:enum|struct|type|const) " + name + r"\b",
                         src_i, re.M):
                out.append(f"{rel.name} declares {name}; candidate lineage and "
                           "lifecycle are derived from required manifest and receipt "
                           "fields, never a parallel vocabulary (DEC-079/DEC-080)")
    # The Candidate lineage identity is registered with owner BP-SPROUTING-1.
    isrc = (root / "spec/identities.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[IdentityKind\] = &\[(.*?)\];", isrc, re.S)
    inventory = re.findall(r"\bIdentityKind::(\w+)", all_body.group(1)) \
        if all_body else []
    if "Candidate" not in inventory:
        out.append("spec/identities.rs omits IdentityKind::Candidate from "
                   "IdentityKind::ALL")
    arm = re.search(
        r"\bIdentityKind::Candidate => \{?\s*entry!\(\s*\"([^\"]+)\",\s*\"([^\"]+)\"\s*\)",
        isrc)
    if not arm:
        out.append("spec/identities.rs declares no IdentityKind::Candidate catalog "
                   "entry")
    else:
        if arm.group(1) != "CandidateId":
            out.append(f"IdentityKind::Candidate spells {arm.group(1)}, not CandidateId")
        if arm.group(2) != "BP-SPROUTING-1":
            out.append(f"IdentityKind::Candidate names owner {arm.group(2)}, not "
                       "BP-SPROUTING-1")
    return out


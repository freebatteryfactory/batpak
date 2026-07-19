"""Verification-plane, sprouting-plane, and workflow-pinning fixtures."""
from __future__ import annotations
import shutil
import tempfile
from pathlib import Path
from .core import (
    HERE,
    load,
)


def test_verification_plane() -> list[str]:
    """audit.verification_findings: the frozen axes, the shape-based
    anti-ladder guard, tool-neutral methods, plan admissibility, route kinds
    closed by adoption, the runtime matrix, and the retired counts_green
    spelling (5.5F2). Each hostile mutates a sandbox copy of the real spec and
    expects its detector to fire; the unmutated copy must yield zero findings
    so no detector over-fires."""
    findings: list[str] = []
    audit = load("audit")
    root = HERE.parent
    base = Path(tempfile.mkdtemp(prefix="batpak-verif-"))
    try:
        (base / "spec").mkdir()
        (base / "bootstrap").mkdir()
        (base / "docs").mkdir()
        for rel in sorted((root / "spec").glob("*.rs")):
            shutil.copyfile(rel, base / "spec" / rel.name)
        for rel in ("bootstrap/seedcheck.rs", "bootstrap/audit.py",
                    "docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md",
                    "docs/39_SPROUTING_NURSERY_AND_PROMOTION.md",
                    "docs/07_PAKVM_ISA.md"):
            shutil.copyfile(root / rel, base / rel)
        if (root / "bootstrap/audit").is_dir():
            shutil.copytree(root / "bootstrap/audit", base / "bootstrap/audit")
        clean = audit.verification_findings(base)
        if clean:
            findings.append(f"verification_plane baseline is not clean: {clean[:3]}")

        vpath = base / "spec" / "verification.rs"
        ppath = base / "spec" / "proof.rs"
        vsrc = vpath.read_text(encoding="utf-8")
        psrc = ppath.read_text(encoding="utf-8")

        def expect(name: str, needle: str) -> None:
            got = audit.verification_findings(base)
            if not any(needle in f for f in got):
                findings.append(f"{name} FAILED (no finding containing {needle!r})")
            vpath.write_text(vsrc, encoding="utf-8", newline="\n")
            ppath.write_text(psrc, encoding="utf-8", newline="\n")

        def mutate(path: Path, source: str, old: str, new: str, what: str) -> bool:
            if old not in source:
                findings.append(f"{what}: probe target absent, mutation never applied")
                return False
            path.write_text(source.replace(old, new, 1), encoding="utf-8", newline="\n")
            return True

        if mutate(vpath, vsrc, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum VerificationCoverage",
                  "#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]\npub enum VerificationCoverage",
                  "ordered_coverage"):
            expect("verification_axis_ordering_is_rejected", "derives an ordering")
        vpath.write_text(vsrc + "\npub fn score(_x: bool) {}\n", encoding="utf-8", newline="\n")
        expect("verification_score_fn_is_rejected", "rank/level/score")
        vpath.write_text(
            vsrc + "\npub fn strength(c: VerificationCoverage) -> u8 { c as u8 }\n",
            encoding="utf-8", newline="\n")
        expect("verification_numeric_conversion_is_rejected", "numerically converts")
        vpath.write_text(vsrc + "\npub enum VerificationToolId { Placeholder }\n",
                         encoding="utf-8", newline="\n")
        expect("verification_tool_vocabulary_is_rejected", "tool vocabulary VerificationToolId")
        vpath.write_text(vsrc + "\nuse crate::tier0_cross_run as _t0;\n",
                         encoding="utf-8", newline="\n")
        expect("tier0_import_into_verification_is_rejected", "never product verification authority")
        if mutate(vpath, vsrc, "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n}",
                  "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n    Extra,\n}",
                  "extra_coverage_variant"):
            expect("verification_axis_variant_drift_is_rejected", "!= frozen")
        if mutate(vpath, vsrc,
                  "D::NoDivergenceObserved | D::Divergent | D::Incomplete | D::Stale | D::Unsupported",
                  "D::NoDivergenceObserved | D::ConformantForObservedHistory | D::Divergent | D::Incomplete | D::Stale | D::Unsupported",
                  "inband_conformant"):
            expect("in_band_conformance_is_rejected", "only independent offline replay")
        ppath.write_text(
            psrc + "\n// fn _retired() { let _ = ProofUnitTerminal::Passed.counts_green(); }\n",
            encoding="utf-8", newline="\n")
        expect("counts_green_reintroduction_is_rejected", "still calls counts_green")
        hostile_plan = ("pub const PLAN_HOSTILE_BOUNDARY: &[VerificationRequirement] = "
                        "&[VerificationRequirement { method: VerificationMethod::PropertySequence, "
                        "basis: VerificationBasis::DirectBoundary")
        if mutate(ppath, psrc, hostile_plan,
                  hostile_plan.replace("VerificationBasis::DirectBoundary",
                                       "VerificationBasis::ContractProjection"),
                  "projection_route_plan"):
            expect("projection_supplied_route_in_plan_is_rejected",
                   "gives ContractProjection a route")
        if mutate(ppath, psrc,
                  "route: IndependentEvidenceRouteKind::IndependentHistoryReplay }",
                  "route: IndependentEvidenceRouteKind::DifferentialImplementation }",
                  "orphan_replay_route"):
            expect("route_kind_without_adopter_is_rejected",
                   "IndependentEvidenceRouteKind::IndependentHistoryReplay has no active")
        row = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), state: '
               'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
               'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], '
               'claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }')
        if mutate(ppath, psrc, row,
                  row.replace("VerificationClaimKind::Safety", "VerificationClaimKind::Bogus"),
                  "bogus_claim"):
            expect("unknown_claim_kind_is_rejected", "claims unknown kind Bogus")
        if mutate(ppath, psrc, row,
                  row.replace("verification: PLAN_HOSTILE_BOUNDARY",
                              "verification: PLAN_NONEXISTENT"),
                  "undeclared_plan"):
            expect("undeclared_plan_reference_is_rejected", "undeclared plan PLAN_NONEXISTENT")
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return findings


def test_sprouting_plane() -> list[str]:
    """audit.sprouting_findings + audit.verification_findings: the frozen
    candidate-lineage vocabularies, the NonZeroU64 budget law, the
    admit_promotion_plan body law, the SPECIALIZED_PLAN_CANDIDATE_POLICY
    reconstruction, the forbidden candidate-evidence-relation vocabulary, the
    IdentityKind::Candidate registration, and the verification normalization
    laws (no independent_route field, no observation enforcement field, the
    OfflineReplay runtime replay-route law, and the retired qualify spelling)
    (5.5F3). Each hostile mutates a sandbox copy of the real spec and expects
    its detector to fire; the unmutated copy must yield zero findings so no
    detector over-fires."""
    findings: list[str] = []
    audit = load("audit")
    root = HERE.parent
    base = Path(tempfile.mkdtemp(prefix="batpak-sprout-"))
    try:
        (base / "spec").mkdir()
        (base / "bootstrap").mkdir()
        (base / "docs").mkdir()
        for rel in sorted((root / "spec").glob("*.rs")):
            shutil.copyfile(rel, base / "spec" / rel.name)
        for rel in ("bootstrap/seedcheck.rs", "bootstrap/audit.py",
                    "docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md",
                    "docs/39_SPROUTING_NURSERY_AND_PROMOTION.md",
                    "docs/07_PAKVM_ISA.md"):
            shutil.copyfile(root / rel, base / rel)
        if (root / "bootstrap/audit").is_dir():
            shutil.copytree(root / "bootstrap/audit", base / "bootstrap/audit")
        clean_s = audit.sprouting_findings(base)
        if clean_s:
            findings.append(f"sprouting_plane sprouting baseline is not clean: {clean_s[:3]}")
        clean_v = audit.verification_findings(base)
        if clean_v:
            findings.append(f"sprouting_plane verification baseline is not clean: {clean_v[:3]}")

        spath = base / "spec" / "sprouting.rs"
        vpath = base / "spec" / "verification.rs"
        ipath = base / "spec" / "identities.rs"
        ssrc = spath.read_text(encoding="utf-8")
        vsrc = vpath.read_text(encoding="utf-8")
        isrc = ipath.read_text(encoding="utf-8")

        def restore() -> None:
            spath.write_text(ssrc, encoding="utf-8", newline="\n")
            vpath.write_text(vsrc, encoding="utf-8", newline="\n")
            ipath.write_text(isrc, encoding="utf-8", newline="\n")

        def expect_s(name: str, needle: str) -> None:
            got = audit.sprouting_findings(base)
            if not any(needle in f for f in got):
                findings.append(f"{name} FAILED (no sprouting finding containing {needle!r})")
            restore()

        def expect_v(name: str, needle: str) -> None:
            got = audit.verification_findings(base)
            if not any(needle in f for f in got):
                findings.append(f"{name} FAILED (no verification finding containing {needle!r})")
            restore()

        def mutate(path: Path, source: str, old: str, new: str, what: str) -> bool:
            if old not in source:
                findings.append(f"{what}: probe target absent, mutation never applied")
                return False
            path.write_text(source.replace(old, new, 1), encoding="utf-8", newline="\n")
            return True

        # 1. A seventh candidate-origin variant drifts the frozen set.
        if mutate(spath, ssrc, "    RepairOfCandidate,\n",
                  "    RepairOfCandidate,\n    MischievousSeventhOrigin,\n",
                  "seventh_origin"):
            expect_s("seventh_candidate_origin_is_rejected", "CandidateOriginKind")
        # 2. A budget ceiling relaxed to a plain u64 loses the NonZero law.
        if mutate(spath, ssrc, "max_candidates: NonZeroU64", "max_candidates: u64", "budget_u64"):
            expect_s("candidate_budget_without_nonzero_is_rejected", "NonZeroU64")
        # 3. The admit_promotion_plan LawChanging->architect coupling neutered.
        if mutate(spath, ssrc, "PromotionPlanError::LawChangingRequiresArchitect",
                  "PromotionPlanError::RealizationPreservingCannotRequireArchitect",
                  "lawchanging_neutered"):
            expect_s("lawchanging_requires_architect_law_is_enforced", "law-changing")
        # 4. The policy const route flipped away from the differential route.
        if mutate(spath, ssrc,
                  "independent_route: IndependentEvidenceRouteKind::DifferentialImplementation",
                  "independent_route: IndependentEvidenceRouteKind::HostileBoundary",
                  "policy_route_flip"):
            expect_s("specialized_plan_policy_route_is_reconstructed", "DifferentialImplementation")
        # 5. A generic candidate-evidence-relation vocabulary is forbidden.
        spath.write_text(ssrc + "\npub enum CandidateEvidenceRelation { Placeholder }\n",
                         encoding="utf-8", newline="\n")
        expect_s("candidate_evidence_relation_vocabulary_is_rejected", "CandidateEvidenceRelation")
        # 6. The IdentityKind::Candidate registration removed (spelling mangled).
        if mutate(ipath, isrc, '"CandidateId"', '"RemovedCandidateId"', "candidate_identity_removed"):
            expect_s("candidate_identity_registration_is_required", "CandidateId")
        # 7. An independent_route field reintroduced into verification.rs.
        vpath.write_text(
            vsrc + "\npub struct _HostileRoute { pub independent_route: "
            "Option<IndependentEvidenceRouteKind> }\n",
            encoding="utf-8", newline="\n")
        expect_v("independent_route_field_reintroduction_is_rejected", "independent_route")
        # 8. An enforcement field added to the observation struct.
        if mutate(vpath, vsrc, "pub counterexample_outstanding: bool,",
                  "pub counterexample_outstanding: bool,\n    pub enforcement: "
                  "VerificationEnforcementPosture,",
                  "observation_enforcement"):
            expect_v("observation_enforcement_field_is_rejected", "enforcement")
        # 9. The OfflineReplay runtime arm loses its replay-route requirement.
        replay_guard = "\n".join([
            "            if !matches!(",
            "                observation.basis,",
            "                VerificationBasis::IndependentReference {",
            "                    route: IndependentEvidenceRouteKind::IndependentHistoryReplay",
            "                }",
            "            ) {",
            "                return Err(RuntimeVerificationError::ReplayRequiresIndependentHistoryReplay);",
            "            }",
            "",
        ])
        if mutate(vpath, vsrc, replay_guard, "", "replay_route_neutered"):
            expect_v("offline_replay_requires_replay_route", "independent history-replay")
        # 10. The retired qualify spelling reintroduced.
        vpath.write_text(vsrc + "\npub fn qualify() {}\n", encoding="utf-8", newline="\n")
        expect_v("retired_qualify_spelling_is_rejected", "qualify")
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return findings


def test_workflow_pinning() -> list[str]:
    """audit.workflow_pinning_findings refuses movable/abbreviated/docker-untagged
    action references and accepts local `./` actions and full-SHA pins (5.5E6c2)."""
    findings: list[str] = []
    audit = load("audit")
    work = Path(tempfile.mkdtemp(prefix="batpak-wf-pin-"))
    try:
        wf = work / ".github" / "workflows"
        wf.mkdir(parents=True)
        sha = "3" * 40

        def one(uses: str) -> bool:
            (wf / "x.yml").write_text(
                "on: push\njobs:\n  j:\n    runs-on: ubuntu-latest\n    steps:\n"
                f"      - uses: {uses}\n", encoding="utf-8", newline="\n")
            return any("x.yml" in f for f in audit.workflow_pinning_findings(work))

        cases = [
            (f"actions/checkout@{sha}", False),
            ("actions/checkout@v4", True),
            ("actions/checkout@main", True),
            (f"actions/checkout@{'3' * 10}", True),
            ("./.github/actions/local", False),
            ("docker://alpine:3.14", True),
            (f"docker://alpine@sha256:{'a' * 64}", False),
        ]
        for uses, want in cases:
            got = one(uses)
            if got != want:
                findings.append(f"workflow_pinning {uses!r}: expected finding={want}, got {got}")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return findings

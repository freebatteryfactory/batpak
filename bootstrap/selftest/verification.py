"""Verification-plane, sprouting-plane, and workflow-pinning fixtures."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from .core import (
    HERE,
    gate_sandbox,
    load,
    neutered_validator,
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
        for rel in sorted((root / "spec").rglob("*.rs")):
            dst = base / "spec" / rel.relative_to(root / "spec")
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(rel, dst)
        for rel in ("bootstrap/seedcheck.rs", "bootstrap/audit.py",
                    "docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md",
                    "docs/39_SPROUTING_NURSERY_AND_PROMOTION.md",
                    "docs/07_PAKVM_ISA.md"):
            shutil.copyfile(root / rel, base / rel)
        if (root / "bootstrap/audit").is_dir():
            shutil.copytree(root / "bootstrap/audit", base / "bootstrap/audit")
        for tool in ("seedcheck", "materialize", "receiptcheck"):
            if (root / "bootstrap" / tool).is_dir():
                shutil.copytree(root / "bootstrap" / tool, base / "bootstrap" / tool)
        clean = audit.verification_findings(base)
        if clean:
            findings.append(f"verification_plane baseline is not clean: {clean[:3]}")

        # Source-grammar bake: append-shaped probes land in the domain facade
        # (mod.rs -- the audit joins every file under the domain directory),
        # and probes that mutate an existing item target the carrier that owns
        # it (types.rs for axis vocabulary, proof/inventory.rs for plan/row
        # facts).
        vpath = base / "spec" / "verification" / "mod.rs"
        axpath = base / "spec" / "verification" / "types.rs"
        ppath = base / "spec" / "proof" / "inventory.rs"
        vsrc = vpath.read_text(encoding="utf-8")
        axsrc = axpath.read_text(encoding="utf-8")
        psrc = ppath.read_text(encoding="utf-8")

        def expect(name: str, needle: str) -> None:
            got = audit.verification_findings(base)
            if not any(needle in f for f in got):
                findings.append(f"{name} FAILED (no finding containing {needle!r})")
            vpath.write_text(vsrc, encoding="utf-8", newline="\n")
            axpath.write_text(axsrc, encoding="utf-8", newline="\n")
            ppath.write_text(psrc, encoding="utf-8", newline="\n")

        def mutate(path: Path, source: str, old: str, new: str, what: str) -> bool:
            if old not in source:
                findings.append(f"{what}: probe target absent, mutation never applied")
                return False
            path.write_text(source.replace(old, new, 1), encoding="utf-8", newline="\n")
            return True

        if mutate(axpath, axsrc, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum VerificationCoverage",
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
        if mutate(axpath, axsrc, "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n}",
                  "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n    Extra,\n}",
                  "extra_coverage_variant"):
            expect("verification_axis_variant_drift_is_rejected", "!= frozen")
        if mutate(axpath, axsrc,
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
        for rel in sorted((root / "spec").rglob("*.rs")):
            dst = base / "spec" / rel.relative_to(root / "spec")
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(rel, dst)
        for rel in ("bootstrap/seedcheck.rs", "bootstrap/audit.py",
                    "docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md",
                    "docs/39_SPROUTING_NURSERY_AND_PROMOTION.md",
                    "docs/07_PAKVM_ISA.md"):
            shutil.copyfile(root / rel, base / rel)
        if (root / "bootstrap/audit").is_dir():
            shutil.copytree(root / "bootstrap/audit", base / "bootstrap/audit")
        for tool in ("seedcheck", "materialize", "receiptcheck"):
            if (root / "bootstrap" / tool).is_dir():
                shutil.copytree(root / "bootstrap" / tool, base / "bootstrap" / tool)
        clean_s = audit.sprouting_findings(base)
        if clean_s:
            findings.append(f"sprouting_plane sprouting baseline is not clean: {clean_s[:3]}")
        clean_v = audit.verification_findings(base)
        if clean_v:
            findings.append(f"sprouting_plane verification baseline is not clean: {clean_v[:3]}")

        # Source-grammar bake: each probe targets the carrier file that owns
        # its needle (types.rs for frozen vocabulary and struct declarations,
        # admission.rs for the algorithm body, inventory.rs for the authored
        # policy instance); append-shaped probes land in a domain carrier the
        # audit joins via its directory walk.
        spath = base / "spec" / "sprouting" / "types.rs"
        sadmpath = base / "spec" / "sprouting" / "admission.rs"
        sinvpath = base / "spec" / "sprouting" / "inventory.rs"
        vpath = base / "spec" / "verification" / "mod.rs"
        ipath = base / "spec" / "identities" / "types.rs"
        opath = base / "spec" / "verification" / "observation.rs"
        rpath = base / "spec" / "verification" / "runtime.rs"
        ssrc = spath.read_text(encoding="utf-8")
        sadmsrc = sadmpath.read_text(encoding="utf-8")
        sinvsrc = sinvpath.read_text(encoding="utf-8")
        vsrc = vpath.read_text(encoding="utf-8")
        isrc = ipath.read_text(encoding="utf-8")
        osrc = opath.read_text(encoding="utf-8")
        rsrc = rpath.read_text(encoding="utf-8")

        def restore() -> None:
            spath.write_text(ssrc, encoding="utf-8", newline="\n")
            sadmpath.write_text(sadmsrc, encoding="utf-8", newline="\n")
            sinvpath.write_text(sinvsrc, encoding="utf-8", newline="\n")
            vpath.write_text(vsrc, encoding="utf-8", newline="\n")
            ipath.write_text(isrc, encoding="utf-8", newline="\n")
            opath.write_text(osrc, encoding="utf-8", newline="\n")
            rpath.write_text(rsrc, encoding="utf-8", newline="\n")

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
        if mutate(sadmpath, sadmsrc, "PromotionPlanError::LawChangingRequiresArchitect",
                  "PromotionPlanError::RealizationPreservingCannotRequireArchitect",
                  "lawchanging_neutered"):
            expect_s("lawchanging_requires_architect_law_is_enforced", "law-changing")
        # 4. The policy const route flipped away from the differential route.
        if mutate(sinvpath, sinvsrc,
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
        if mutate(opath, osrc, "pub counterexample_outstanding: bool,",
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
        if mutate(rpath, rsrc, replay_guard, "", "replay_route_neutered"):
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


def test_python_tooling(audit) -> list[str]:
    """audit.python_tooling_findings: the uv.lock requirement and
    lock-to-manifest parity (F4 prelude). A deleted lock and a dev tool
    declared but not locked are refused; neutering the lock-existence guard
    silences the refusal; the assembled fixture tree is clean."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    root = HERE.parent
    work = Path(tempfile.mkdtemp(prefix="batpak-pytool-"))
    try:
        (work / "spec").mkdir()
        # SGB source grammar: the domain is a directory; a missing directory is
        # a copytree refusal, never a silently thinner fixture.
        shutil.copytree(root / "spec/bootstrap_qualification",
                        work / "spec/bootstrap_qualification")
        for rel in (".python-version", "pyproject.toml", "uv.lock"):
            shutil.copyfile(root / rel, work / rel)
        if audit.python_tooling_findings(work):
            fail("python_tooling_fixture_baseline_is_clean")
        (work / "uv.lock").unlink()
        if not any("uv.lock: missing" in f for f in audit.python_tooling_findings(work)):
            fail("deleted_uv_lock_is_refused")
        with neutered_validator("audit", "    if not lock_path.is_file():",
                                "    if False:") as neutered:
            if any("uv.lock: missing" in f for f in neutered.python_tooling_findings(work)):
                fail("neutering_the_lock_requirement_silences_it")
        shutil.copyfile(root / "uv.lock", work / "uv.lock")
        pp = (root / "pyproject.toml").read_text(encoding="utf-8")
        if '"ruff' not in pp:
            fail("python_tooling_fixture_expects_ruff_declared")
        (work / "pyproject.toml").write_text(
            pp.replace('"ruff', '"ruffian', 1), encoding="utf-8", newline="\n")
        if not any("lock-to-manifest parity" in f
                   for f in audit.python_tooling_findings(work)):
            fail("unlocked_declared_dev_tool_is_refused")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return findings


def test_isolated_execution(audit) -> list[str]:
    """The bootstrap gates run under `python -I` (isolated interpreter). The
    canary proves a PYTHONPATH stdlib-shadowing poison is POTENT under a plain
    interpreter and INVISIBLE under -I; the three read-only gates then run as
    real `python -I` subprocesses against the canonical tree inside that
    poisoned environment and still pass, proving accidental site-package
    access can neither rescue nor sabotage a gate; the workflow mutation
    proves the audit refuses a qualification workflow that drops -I from a
    gate invocation; the neuter proves that refusal is live, not decorative."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    poison = Path(tempfile.mkdtemp(prefix="batpak-iso-poison-"))
    work = Path(tempfile.mkdtemp(prefix="batpak-iso-wf-"))
    try:
        # Canary: a module shadowing stdlib pathlib that detonates on import.
        (poison / "pathlib.py").write_text(
            "raise RuntimeError('site poison reached the interpreter')\n",
            encoding="utf-8", newline="\n")
        env = dict(os.environ)
        env["PYTHONPATH"] = str(poison)
        probe = "import pathlib; print(pathlib.Path('.'))"
        plain = subprocess.run([sys.executable, "-c", probe],
                               env=env, capture_output=True, text=True)
        if plain.returncode == 0:
            fail("canary_poison_is_potent_under_plain_python")
        isolated = subprocess.run([sys.executable, "-I", "-c", probe],
                                  env=env, capture_output=True, text=True)
        if isolated.returncode != 0:
            fail("canary_poison_is_invisible_under_isolated_python")

        # The real read-only gates as `python -I` subprocesses in the poisoned
        # environment, invoked exactly as the qualification workflow invokes
        # them (no root argument; cwd is the tree).
        root = HERE.parent
        for name, argv in (
            ("project_check_under_isolation", ("bootstrap/project.py", "--check")),
            ("audit_under_isolation", ("bootstrap/audit.py",)),
            ("freeze_check_under_isolation", ("bootstrap/freeze.py", "--check")),
        ):
            r = subprocess.run([sys.executable, "-I", *argv], cwd=root,
                               env=env, capture_output=True, text=True)
            if r.returncode != 0:
                tail = (r.stderr or r.stdout).strip().splitlines()[-1:] or ["<no output>"]
                fail(f"{name} (rc={r.returncode}: {tail[0][:160]})")

        # A qualification workflow that drops -I from a gate invocation is
        # refused; restoring -I silences the finding; neutering the guard
        # silences the refusal itself.
        wf_dir = work / ".github" / "workflows"
        wf_dir.mkdir(parents=True)
        wf = wf_dir / "msvc-qualification.yml"
        needle = "without isolated mode"
        wf.write_text("jobs:\n  q:\n    steps:\n"
                      "      - run: python bootstrap/audit.py\n",
                      encoding="utf-8", newline="\n")
        if not any(needle in f for f in audit.workflow_pinning_findings(work)):
            fail("plain_python_gate_invocation_is_refused")
        with neutered_validator(
                "audit",
                r"            if not re.search(r'\bpython(?:3)?\s+-I\b', line):",
                "            if False:",
        ) as neutered:
            if any(needle in f for f in neutered.workflow_pinning_findings(work)):
                fail("neutering_the_isolation_rule_silences_it")
        wf.write_text("jobs:\n  q:\n    steps:\n"
                      "      - run: python -I bootstrap/audit.py\n",
                      encoding="utf-8", newline="\n")
        if any(needle in f for f in audit.workflow_pinning_findings(work)):
            fail("isolated_gate_invocation_is_lawful")
    finally:
        shutil.rmtree(poison, ignore_errors=True)
        shutil.rmtree(work, ignore_errors=True)
    return findings


def test_bootstrap_topology(audit) -> list[str]:
    """audit.bootstrap_topology_findings: the eight-rule AST topology law over
    the bootstrap Python plane. h0 proves the canonical tree is clean; h1..h11
    each mutate a sandbox copy of bootstrap/ and prove the matching rule bites;
    n1..n6 neuter the rule's own source line and prove the detector goes silent.
    h8..h11 harden the AST matcher against four architect-verified bypasses: a
    non-comparison `if __name__:` guard, an impure assert, a grandfathered name
    rebound to an impure value, and a sys.path mutation reached through a
    `from sys import path as p` alias. A claimed negative test is itself a claim,
    so every hostile asserts its specific needle and every neuter asserts that
    same needle disappears."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # h0: the canonical tree carries no topology findings (run FIRST).
    if audit.bootstrap_topology_findings(HERE.parent) != []:
        fail("h0 canonical-clean")

    # Each positive hostile appends to a fresh sandbox copy (gate_sandbox copies
    # the whole bootstrap/ tree; the append mutates only the copy) and asserts
    # its needle fires. h1/h3/h5 and the four hardening hostiles h9/h10/h11
    # sandboxes are kept for the neuter re-runs.
    hostiles = [
        ("h1", "bootstrap/audit/corpus.py", "\nfrom project import registry\n",
         "static import of sibling bootstrap tool 'project'"),
        ("h2", "bootstrap/project/registry.py", "\nimport sys\nsys.path.append('x')\n",
         "sys.path mutation"),
        ("h3", "bootstrap/selftest/domains.py", "\nimport requests\n",
         "non-stdlib import 'requests'"),
        ("h4", "bootstrap/audit/batql.py", "\nprint('boot')\n",
         "import-time effect"),
        ("h5", "bootstrap/project/registry.py", "# pad\n" * 1000,
         "exceeds its ratchet ceiling"),
        ("h6", "bootstrap/freeze.py", "# pad\n" * 60,
         "entry shim has"),
        ("h7", "bootstrap/project/registry.py", "\nfrom .domains import PLAN_DOMAINS\n",
         "import cycle"),
        # H1: a bare `if __name__:` is not the exact Compare(==, "__main__")
        # guard, so it is an import-time effect, not a lawful main guard.
        ("h8", "bootstrap/audit/batql.py", "\nif __name__:\n    print('runs')\n",
         "import-time effect"),
        # H2: an assert whose test performs an impure call runs at import time.
        ("h9", "bootstrap/audit/batql.py", "\nassert open('side-effect.txt', 'w')\n",
         "import-time effect"),
        # H3: a grandfathered name rebound to an impure value has a different
        # value-dump digest, so the exemption lapses and the effect fires.
        ("h10", "bootstrap/selftest/core.py", "\nTOOLCHAIN_EDITION = open('x').read()\n",
         "import-time effect"),
        # H4: a sys.path mutation reached through a `from sys import path as p`
        # alias is still a sys.path mutation.
        ("h11", "bootstrap/project/registry.py",
         "\nfrom sys import path as p\np.append('ambient')\n",
         "sys.path mutation"),
    ]
    kept: dict[str, Path] = {}
    try:
        for name, rel, addition, needle in hostiles:
            sandbox = gate_sandbox([])
            path = sandbox / rel
            path.write_text(path.read_text(encoding="utf-8") + addition, encoding="utf-8")
            if not any(needle in f for f in audit.bootstrap_topology_findings(sandbox)):
                fail(f"{name} did not fire ({needle!r})")
            if name in ("h1", "h3", "h5", "h9", "h10", "h11"):
                kept[name] = sandbox
            else:
                shutil.rmtree(sandbox, ignore_errors=True)

        # Neuter proof: replacing the rule's own guard line with "if False:"
        # makes its hostile go silent. Each target is a unique single line in
        # bootstrap/audit/tier0.py (verified across the shim + package). n4/n5/n6
        # neuter the three hardening guards: the assert-impurity check, the
        # grandfather digest comparison, and the sys.path-alias mutation check.
        neuters = [
            ("n1", "h1", "if pkg is not None and first in _TOPOLOGY_TOOLS:",
             "static import of sibling bootstrap tool 'project'"),
            ("n2", "h3", "elif first not in sys.stdlib_module_names:",
             "non-stdlib import 'requests'"),
            ("n3", "h5", "if count > ceiling:",
             "exceeds its ratchet ceiling"),
            ("n4", "h9", "if _topology_impure(stmt.test):",
             "import-time effect"),
            ("n5", "h10", "if value_digest != exempt_digest:",
             "import-time effect"),
            ("n6", "h11", "if is_path_alias(recv) and node.func.attr in _TOPOLOGY_PATH_METHODS:",
             "sys.path mutation"),
        ]
        for name, hkey, target, needle in neuters:
            with neutered_validator("audit", target) as neutered:
                if any(needle in f for f in neutered.bootstrap_topology_findings(kept[hkey])):
                    fail(f"{name} did not silence ({needle!r})")
    finally:
        for sandbox in kept.values():
            shutil.rmtree(sandbox, ignore_errors=True)
    return findings

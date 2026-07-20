"""Hostile battery for the mini-supernova campaign rehearsal (docs/24).

Every refusal law in the rehearsal harness is proven live here, and the
independent verifier is proven two-sided (a tampered bundle is refused; a
blinded verifier goes silent, which the battery catches). Split from
supernova.py under the ratified split-on-next-edit size rule; the laws
themselves stay owned by supernova.py -- this module only exercises them.
"""
from __future__ import annotations

import shutil
from pathlib import Path

from . import supernova_bundle as sb
from . import supernova_subject as subj


def hostiles(paths: dict, work: Path, *, admit_failure_evidence,
             judge_freshness, promote_guard, validate_disjoint) -> list[str]:
    """The refusal laws are passed in by supernova.py (their owner) so the
    module graph stays a DAG: this battery depends only on the bundle and
    subject carriers, never back on the harness."""
    findings: list[str] = []

    def expect_refusal(name: str, produced: str | None, needle: str) -> None:
        if produced is None or needle not in produced:
            findings.append(f"{name} FAILED (wanted {needle!r}, got {produced!r})")

    # Judge mutation invalidates evidence (and prior evidence stays intact).
    judge_copy = work / "judge-mutant"
    shutil.copytree(paths["judge"], judge_copy)
    frozen = sb.tree_digest(judge_copy)
    if judge_freshness(judge_copy, frozen) is not None:
        findings.append("judge_freshness misfired on an unmutated judge")
    policy = judge_copy / "policy.txt"
    policy.write_text(policy.read_text(encoding="utf-8") + "# mutated\n",
                      encoding="utf-8", newline="\n")
    expect_refusal("judge_mutation_invalidates_evidence",
                   judge_freshness(judge_copy, frozen), "StaleByJudgeChange")

    # Search/holdout overlap is refused before evaluation.
    expect_refusal("search_holdout_overlap_is_refused",
                   validate_disjoint(subj.SEARCH_VECTORS,
                                     subj.SEARCH_VECTORS.split("----")[0]
                                     + "----\n" + subj.HOLDOUT_VECTORS),
                   "search/holdout overlap")

    # A scaffold cannot realize/promote.
    scaffold = {"posture": "Scaffold", "change_class": "RealizationPreserving",
                "dependencies": []}
    expect_refusal("scaffold_cannot_close_realization",
                   promote_guard(scaffold, set()), "scaffold cannot close realization")

    # A fabricated red status is refused: the failure must carry a real
    # compiler diagnostic from a real nonzero boundary.
    expect_refusal("fake_status_red_is_refused",
                   admit_failure_evidence("status = red", 1),
                   "no compiler diagnostic")
    expect_refusal("zero_exit_failure_is_refused",
                   admit_failure_evidence("error[E0308]: mismatched types", 0),
                   "exit code zero")
    if admit_failure_evidence("error[E0308]: mismatched types", 1) is not None:
        findings.append("a genuine compiler failure was refused as evidence")

    # The bundle verifier is two-sided: a tampered judge root is refused by
    # the REAL receiptcheck with a named finding, and a receiptcheck whose
    # judge-digest guard is blinded admits the identical tamper -- silence
    # the battery must catch.
    rc, err = sb.build_receiptcheck(paths["rustc"], paths["target_triple"], work)
    if rc is None:
        findings.append(f"hostile verifier battery unavailable: {err}")
        return findings
    tampered = work / "judge-tampered"
    shutil.copytree(paths["judge"], tampered)
    victim = tampered / "policy.txt"
    victim.write_text(victim.read_text(encoding="utf-8") + "# tampered\n",
                      encoding="utf-8", newline="\n")
    ok, out = sb.campaign_verify(rc, paths["bundle"], tampered, paths["envelope"],
                                 paths["source_commit"])
    if ok:
        findings.append("receiptcheck ADMITTED a bundle over a tampered judge root")
    elif "judge-root digest" not in out:
        findings.append("tampered judge refused for the wrong reason: "
                        f"{out.strip()[:160]}")
    neutered, err = sb.neutered_receiptcheck(
        paths["rustc"], paths["target_triple"], work,
        "if recomputed.render() != doc.judge_root_digest {",
        "if false {")
    if neutered is None:
        findings.append(f"verifier neuter did not build: {err}")
    else:
        ok, _out = sb.campaign_verify(neutered, paths["bundle"], tampered,
                                      paths["envelope"], paths["source_commit"])
        if not ok:
            findings.append("the blinded verifier still refused; the neuter "
                            "proves nothing about the judge-digest guard")
    return findings

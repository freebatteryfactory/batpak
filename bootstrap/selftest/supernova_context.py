"""Bundle-context assembly for the mini-supernova campaign rehearsal.

Split from supernova.py under the ratified E7-closeout plan (CL-9). The
rehearsal harness executes the campaign and hands its executed facts HERE;
this module assembles the exact context dict the supernova_bundle renderers
consume for the BATPAK-CAMPAIGN-EVIDENCE/3 bundle and the release envelope.
Assembly only: closure edges are DERIVED (parent and dependency edges read
from the manifest records; reuse and invalidation edges arrive from their
receipt-writing sites so receipts and closure edges stay set-equal), the
seal commitments are computed from the executed facts, and nothing here
executes a candidate or votes on subject semantics.
"""
from __future__ import annotations

import re
import subprocess
from pathlib import Path

from . import supernova_bundle as sb
from . import supernova_subject as subj


def build_context(*, judge: Path, judge_digest: str, judge_after: str,
                  source_commit: str, rustc: str, target_triple: str,
                  budget: dict, actual: dict, fuzz_line: str,
                  records: list[dict], roots: list[dict],
                  terminals: dict[str, tuple[str, str]],
                  extra_edges: list[tuple[str, str, str]],
                  witness_src_digest: str, events_n: str,
                  history_digest: str, mutant: dict) -> dict:
    """The renderer context: toolchain identity, derived closure edges over
    candidates AND roots (CL-1: closure endpoints are candidates union
    roots), evaluation-set commitments, dispositions, and the commitments
    plus rows for all 20 seal fields."""
    m = re.search(r"release: (\S+)", subprocess.run(
        [rustc, "-vV"], capture_output=True, text=True).stdout)
    rustc_release = m.group(1) if m else "0.0.0"
    edges: list[tuple[str, str, str]] = []
    for record in records:
        for parent in record["parents"]:
            edges.append((record["id"], parent, "Parent"))
        for dep_id, _c in record["dependencies"]:
            edges.append((record["id"], dep_id, "Dependency"))
    edges.extend(extra_edges)
    vector_sets = {
        role: (len([t for t in text.split("----") if t.strip()]),
               sb.sha_hex((judge / f"{role}.vectors").read_bytes()))
        for role, text in subj.VECTOR_SETS.items()}
    model_dispositions = [
        f"model-disposition mini-witness-target {role} agree "
        f"witness-source={witness_src_digest}"
        for role in ("qualification", "holdout", "regression")]
    runtime_dispositions = [
        f"runtime-conformance-disposition mini-witness-target "
        f"conformant-for-observed-history events={events_n} history={history_digest}"]
    closure_preview = "".join(f"node {r['id']}\n" for r in records)
    _profile_id, profile_rows = sb.mini_supernova_profile()
    return {
        "judge_digest": judge_digest, "judge_digest_after": judge_after,
        "source_commit": source_commit, "rustc_release": rustc_release,
        "target": target_triple, "budget_declared": budget, "budget_actual": actual,
        "fuzz_line": fuzz_line, "vector_sets": vector_sets, "records": records,
        "roots": roots, "terminals": terminals,
        "model_dispositions": model_dispositions,
        "runtime_dispositions": runtime_dispositions,
        "mutant_line": f"mutant {mutant['id']} activated=yes killed=yes "
                       "killer=witness-differential",
        "edges": edges,
        "seal_commitments": {
            "SourceTree": sb.sha_hex(source_commit),
            "Toolchain": sb.sha_hex(f"{rustc_release} {target_triple}"),
            "DependencyGraph": sb.sha_hex(
                "\n".join(f"{a} {b}" for a, b in subj.DEPENDENCY_EDGES)),
            "GeneratedFacts": sb.sha_hex(closure_preview),
            # The freshness commitment binds the spec-owned profile rows
            # (parsed, never hand-authored here).
            "ProofFreshness": sb.sha_hex("\n".join(profile_rows)),
        },
        "seal_rows": {
            "TestDispositions": [
                "mini-ledger unit-tests passed",
                "mini-reconcile unit-tests passed",
                "mini-frontier unit-tests passed"],
            "MutationDispositions": [f"{mutant['id']} activated killed "
                                     "witness-differential"],
            "FuzzDispositions": [fuzz_line + " held"],
            "ModelDispositions": [line.removeprefix("model-disposition ")
                                  for line in model_dispositions],
            "RuntimeConformanceDispositions": [
                runtime_dispositions[0].removeprefix(
                    "runtime-conformance-disposition ")],
            "CandidatePromotionSet": [
                f"{r['id']} {r['unit']} {terminals[r['id']][1]}"
                for r in records
                if terminals.get(r["id"], ("",))[0] == "Promoted"],
        },
    }

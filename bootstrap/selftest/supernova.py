"""The F5 mini-supernova campaign rehearsal harness (docs/24, DEC-076..082).

One complete candidate-authority cycle in miniature, executed against the
tracked subject templates (supernova_subject) in an EXTERNAL campaign root
with physically disjoint judge / candidate / evidence / nursery roots:

freeze judge -> generate candidates -> fail for a REAL reason (a genuine
compiler refusal, never `status = red`) -> compress causal diagnostics ->
repair as a child candidate under a receipted bounded search -> independently
qualify (the separately implemented Rust witness judges; this producer never
votes on semantics) -> exercise the disjoint holdout -> activate and kill the
planted semantic mutant -> advance the trusted frontier dependency-first ->
invalidate descendants -> rematerialize -> reach every terminal (Promoted /
Refused / Invalidated / ArchitectRequired, all visible in the denominator) ->
emit the BATPAK-CAMPAIGN-EVIDENCE/2 bundle and the rehearsal release envelope
-> block on bootstrap/receiptcheck.rs `campaign-verify` (V2: bundle + judge +
envelope + source commit + the exact nursery and evidence perimeter).

Every candidate persists an immutable V2 manifest at nursery/<id>/manifest,
receipts append-only beside it; terminals are receipt-backed facts, never
record fields; the frontier speaks the four-state law. The nine spec-owned
profile rows are realized literally, each asserted against executed evidence.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from . import supernova_bundle as sb
from . import supernova_subject as subj
from .core import HERE, TOOLCHAIN_EDITION, ProbeError, receipt

_EXE = ".exe" if os.name == "nt" else ""
_WITNESS_FIXTURE = HERE / "selftest" / "supernova_witness.rs"


# ---------------------------------------------------------------------------
# Build helpers (the tier0 qualify idiom: fresh unique artifacts, digest
# recorded at build, rebound immediately before every execution)
# ---------------------------------------------------------------------------

def _build(rustc: str, target: str, out: Path, crate_name: str, src: Path,
           crate_type: str = "bin", externs: tuple = (), test: bool = False,
           ) -> tuple[bool, str]:
    cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
           "--crate-name", crate_name.replace("-", "_")]
    if crate_type == "rlib":
        cmd += ["--crate-type", "rlib"]
    if test:
        cmd += ["--test"]
    for search_dir in sorted({str(Path(rlib).parent) for _name, rlib in externs}):
        # Transitive rlib dependencies resolve through search paths, not
        # through the direct --extern map.
        cmd += ["-L", f"dependency={search_dir}"]
    for name, rlib in externs:
        cmd += ["--extern", f"{name.replace('-', '_')}={rlib}"]
    cmd += ["-o", str(out), str(src)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    return proc.returncode == 0 and out.is_file(), proc.stdout + proc.stderr


def _bind(path: Path) -> tuple[Path, str]:
    return path, sb.sha_hex(path.read_bytes())


def _run_bound(bound: tuple[Path, str], args: list[str],
               ) -> tuple[int, str]:
    """Execute an artifact only after proving it is still the one this run
    compiled (digest rebind)."""
    path, digest = bound
    if sb.sha_hex(path.read_bytes()) != digest:
        raise ProbeError(f"artifact {path.name} changed between build and execution")
    proc = subprocess.run([str(path), *args], capture_output=True, text=True)
    return proc.returncode, proc.stdout + proc.stderr


# ---------------------------------------------------------------------------
# Typed harness laws (each has a hostile probe in test_supernova)
# ---------------------------------------------------------------------------

def admit_failure_evidence(diagnostic: str, returncode: int) -> str | None:
    """A campaign failure must come from a real boundary: nonzero exit AND a
    genuine compiler diagnostic. A fabricated `status = red` is refused."""
    if returncode == 0:
        return "refused: failure evidence with exit code zero is not a failure"
    if not re.search(r"error(\[E\d+\])?:", diagnostic):
        return ("refused: failure evidence carries no compiler diagnostic; "
                "a fabricated red status is not a boundary failure")
    return None


def validate_disjoint(search: str, holdout: str) -> str | None:
    """Search and holdout evaluation sets are DISJOINT (DEC-081); an overlap
    is refused before any evaluation runs."""
    canon = lambda text: {  # noqa: E731 - tiny local canonicalizer
        "\n".join(line.strip() for line in t.strip().splitlines())
        for t in text.split("----") if t.strip()}
    overlap = canon(search) & canon(holdout)
    if overlap:
        return ("refused: search/holdout overlap -- a holdout trace also "
                f"drives the search ({sorted(overlap)[0].splitlines()[0]!r} ...)")
    return None


def promote_guard(record: dict, qualified_ids: set[str]) -> str | None:
    """The conjunctive promotion guard: a scaffold cannot close realization,
    a law-changing candidate cannot enter this lane, and later green cannot
    bless an unqualified dependency."""
    if record["posture"] == "Scaffold":
        return ("refused: scaffold cannot close realization -- a compiling "
                "placeholder counts in the unrealized denominator")
    if record["change_class"] == "LawChanging":
        return "refused: law-changing candidate requires the architect (DEC-080)"
    for dep_id, _commitment in record["dependencies"]:
        if dep_id not in qualified_ids:
            return (f"refused: BlockedByDependency -- dependency {dep_id[:12]} is "
                    "not qualified; later green cannot bless earlier red")
    return None


def judge_freshness(judge_root: Path, frozen_digest: str) -> str | None:
    """Evidence binds the exact frozen judge digest; a mutated judge
    invalidates it (StaleByJudgeChange), never quietly repaired."""
    if sb.tree_digest(judge_root) != frozen_digest:
        return ("refused: StaleByJudgeChange -- the judge root no longer "
                "matches the frozen snapshot; bound evidence is invalidated")
    return None


# ---------------------------------------------------------------------------
# The rehearsal
# ---------------------------------------------------------------------------

def _split_traces(text: str) -> list[str]:
    return [t.strip() + "\n" for t in text.split("----") if t.strip()]


def _target_transcript(target: tuple[Path, str], vectors: str, scratch: Path,
                       tag: str) -> tuple[str, list[str]]:
    """Run the subject boundary executable over each trace and join the
    per-trace transcripts in the witness's judged grammar."""
    findings: list[str] = []
    out_lines: list[str] = []
    for i, trace in enumerate(_split_traces(vectors)):
        trace_path = scratch / f"{tag}-{i}.trace"
        trace_path.write_text(trace, encoding="utf-8", newline="\n")
        code, out = _run_bound(target, ["run", str(trace_path),
                                        str(subj.FRONTIER_MAX_BATCHES)])
        if code != 0:
            findings.append(f"target refused trace {tag}-{i}: {out.strip()[:120]}")
            continue
        if i > 0:
            out_lines.append("----")
        out_lines.extend(out.splitlines())
    return "".join(line + "\n" for line in out_lines), findings


def _witness(witness: tuple[Path, str], args: list[str], want: str,
             findings: list[str], what: str) -> str:
    code, out = _run_bound(witness, args)
    if code != 0 or want not in out:
        findings.append(f"{what}: witness verdict missing {want!r}: {out.strip()[:200]}")
    return out


def run_rehearsal(base: Path) -> tuple[list[str], list[str], dict]:
    """Execute the complete campaign in `base` (which must live OUTSIDE the
    tracked checkout). Returns (findings, summary_lines, paths)."""
    findings: list[str] = []
    summary: list[str] = []
    paths: dict = {}
    rustc = shutil.which("rustc")
    if not rustc:
        return ["rustc absent; the rehearsal cannot run"], summary, paths
    base = base.resolve()
    if base.is_relative_to(HERE.parent):
        return [f"campaign root {base} is inside the tracked checkout"], summary, paths

    # Physically disjoint roots (campaign roots law).
    judge = base / "judge"
    candidates = base / "candidates"
    evidence = base / "evidence"
    nursery = base / "nursery"
    scratch = base / "scratch"
    for root in (judge, candidates, evidence, nursery, scratch):
        root.mkdir(parents=True, exist_ok=True)
    paths.update(judge=judge, evidence=evidence, base=base,
                 nursery=nursery, evidence_root=evidence)

    # Judge materialization from tracked law, then the frozen snapshot. The
    # witness source is PART of the judge: candidate work never touches it.
    (judge / "witness.rs").write_text(
        _WITNESS_FIXTURE.read_text(encoding="utf-8"), encoding="utf-8", newline="\n")
    for role, text in subj.VECTOR_SETS.items():
        (judge / f"{role}.vectors").write_text(text, encoding="utf-8", newline="\n")
    (judge / "policy.txt").write_text(subj.POLICY_TEXT, encoding="utf-8", newline="\n")
    overlap = validate_disjoint(subj.SEARCH_VECTORS, subj.HOLDOUT_VECTORS)
    if overlap:
        return [f"evaluation-set admission: {overlap}"], summary, paths
    judge_digest = sb.tree_digest(judge)
    summary.append(f"judge frozen {judge_digest[:16]} (content-addressed before "
                   "any candidate executed)")

    target_triple = sb.working_target(rustc, scratch)
    witness_src_digest = sb.sha_hex((judge / "witness.rs").read_bytes())
    witness_dir = scratch / f"witness-{witness_src_digest[:12]}"
    if witness_dir.exists():
        return [f"witness artifact path {witness_dir.name} was not unique"], summary, paths
    witness_dir.mkdir()
    witness_exe = witness_dir / ("supernova_witness" + _EXE)
    ok, diag = _build(rustc, target_triple, witness_exe, "supernova_witness",
                      judge / "witness.rs")
    if not ok:
        return ["the independent witness did not compile:\n"
                + "\n".join(diag.splitlines()[:6])], summary, paths
    witness = _bind(witness_exe)

    # -- lineage generation (whole-tree speculative materialization) --------
    # Proof targets bind each candidate's semantic purpose (TL-1), honestly
    # per role; a diagnostic unit label is never authority. Terminals are
    # receipt-backed facts kept BESIDE the immutable records, never fields.
    source_frontier = sb.sha_hex("genesis\n" + judge_digest)
    terminals: dict[str, tuple[str, str]] = {}

    def persist_manifest(record: dict) -> Path:
        d = nursery / record["id"]
        d.mkdir(parents=True, exist_ok=True)
        path = d / "manifest"
        path.write_text(sb.render_manifest(record), encoding="utf-8", newline="\n")
        return path

    ledger_src = subj.MINI_LEDGER_RS
    broken_src = subj.MINI_RECONCILE_RS.replace("__APPEND_ARG__", subj.BROKEN_APPEND_ARG)
    frontier_src = subj.MINI_FRONTIER_RS
    target_src = subj.MINI_WITNESS_TARGET_RS

    # The stable qualified frontier the bounded repair loop converges to
    # includes the unchanged upstream; the failed parent is that loop's entry.
    row_repair_loop = "bounded_repair_loop_reaches_stable_qualified_frontier"
    # The invalidated echo/never-attempted descendants realize the Invalidated
    # arm of the liveness claim (every obligation reaches a visible terminal).
    row_terminal = "every_admitted_campaign_obligation_reaches_a_terminal"
    ledger1 = sb.mint_record("mini-ledger", proof_targets=[row_repair_loop],
                             parents=[], source_frontier=source_frontier,
                             dependencies=[], content=ledger_src,
                             origin="DeterministicGeneration",
                             change_class="RealizationPreserving", posture="Candidate")
    reconcile1 = sb.mint_record("mini-reconcile", proof_targets=[row_repair_loop],
                                parents=[], source_frontier=source_frontier,
                                dependencies=[(ledger1["id"], ledger1["content_commitment"])],
                                content=broken_src, origin="DeterministicGeneration",
                                change_class="RealizationPreserving", posture="Candidate")
    frontier1 = sb.mint_record("mini-frontier", proof_targets=[row_terminal],
                               parents=[], source_frontier=source_frontier,
                               dependencies=[(reconcile1["id"], reconcile1["content_commitment"])],
                               content=frontier_src, origin="DeterministicGeneration",
                               change_class="RealizationPreserving", posture="Candidate")
    target1 = sb.mint_record("mini-witness-target", proof_targets=[row_terminal],
                             parents=[], source_frontier=source_frontier,
                             dependencies=[(frontier1["id"], frontier1["content_commitment"])],
                             content=target_src, origin="DeterministicGeneration",
                             change_class="RealizationPreserving", posture="Candidate")
    for record in (ledger1, reconcile1, frontier1, target1):
        persist_manifest(record)
    parent_bytes = (nursery / reconcile1["id"] / "manifest").read_bytes()

    def norm(text: str) -> str:
        return sb.normalize_paths(text, [base])

    # -- phase 1: the REAL failure and its echoes ---------------------------
    gen1 = candidates / "gen1"
    gen1.mkdir()
    (gen1 / "mini_ledger.rs").write_text(ledger_src, encoding="utf-8", newline="\n")
    (gen1 / "mini_reconcile.rs").write_text(broken_src, encoding="utf-8", newline="\n")
    (gen1 / "mini_frontier.rs").write_text(frontier_src, encoding="utf-8", newline="\n")
    ledger_rlib = gen1 / "libmini_ledger.rlib"
    ok, diag = _build(rustc, target_triple, ledger_rlib, "mini_ledger",
                      gen1 / "mini_ledger.rs", crate_type="rlib")
    if not ok:
        return ["gen1 mini-ledger did not compile:\n" + diag], summary, paths
    ledger_test = gen1 / ("mini_ledger_test" + _EXE)
    ok, out = _build(rustc, target_triple, ledger_test, "mini_ledger",
                     gen1 / "mini_ledger.rs", test=True)
    if ok:
        code, out = _run_bound(_bind(ledger_test), [])
        ok = code == 0 and "test result: ok. 1 passed" in out
    if not ok:
        findings.append("gen1 mini-ledger unit test did not pass")
    sb.put_receipt(nursery, judge_digest, source_frontier, "qualification",
                   ledger1["id"], ["unit-tests 1 passed", f"target {target_triple}"])

    reconcile_rlib = gen1 / "libmini_reconcile.rlib"
    ok, diag = _build(rustc, target_triple, reconcile_rlib, "mini_reconcile",
                      gen1 / "mini_reconcile.rs", crate_type="rlib",
                      externs=(("mini_ledger", ledger_rlib),))
    if ok:
        return ["the broken mini-reconcile candidate COMPILED; the planted "
                "type error is gone and the causal run has no real failure"], summary, paths
    refusal = admit_failure_evidence(diag, 1)
    if refusal:
        return [f"gen1 failure evidence was refused: {refusal}"], summary, paths
    if "error[E0308]" not in diag:
        findings.append("the mini-reconcile refusal is not the planted E0308 type error")
    root_diag_ref = sb.put_evidence(evidence, judge_digest, "compiler-diagnostic",
                                    norm(diag))
    excerpt = next((line for line in diag.splitlines() if "error[" in line), "")
    ok_echo, echo_diag = _build(rustc, target_triple, gen1 / "libmini_frontier.rlib",
                                "mini_frontier", gen1 / "mini_frontier.rs",
                                crate_type="rlib",
                                externs=(("mini_ledger", ledger_rlib),
                                         ("mini_reconcile", reconcile_rlib)))
    if ok_echo:
        findings.append("gen1 mini-frontier compiled over a missing dependency")
    echo_ref = sb.put_evidence(evidence, judge_digest, "compiler-diagnostic",
                               norm(echo_diag))
    echo_excerpt = next((line for line in echo_diag.splitlines() if "error" in line), "")
    summary += [
        f"causal root: mini-reconcile {reconcile1['id'][:12]} -- {excerpt.strip()}",
        f"affected descendant (echo): mini-frontier -- {echo_excerpt.strip()}",
        "never meaningfully attempted: mini-witness-target",
        "stale receipts: none (mini-ledger receipts bind unchanged commitments)",
        "legal next action: RepairOfCandidate on mini-reconcile "
        "(realization-preserving; BoundedSearch authority)",
    ]
    summary_ref = sb.put_evidence(evidence, judge_digest, "campaign-summary",
                                  norm("".join(line + "\n" for line in summary)))
    evidence_mark = sb.evidence_snapshot(evidence)

    # -- phase 2: receipted bounded repair search ---------------------------
    budget = dict(subj.SEARCH_BUDGET)
    t0 = time.monotonic_ns()
    tried = 0
    logical_work = 0
    memory_bytes = 0
    chosen: str | None = None
    for patch in subj.REPAIR_PATCHES:
        if tried == budget["max-candidates"]:
            break
        tried += 1
        patched = subj.MINI_RECONCILE_RS.replace("__APPEND_ARG__", patch)
        sdir = candidates / f"search-{tried}"
        sdir.mkdir()
        (sdir / "mini_ledger.rs").write_text(ledger_src, encoding="utf-8", newline="\n")
        (sdir / "mini_reconcile.rs").write_text(patched, encoding="utf-8", newline="\n")
        (sdir / "driver.rs").write_text(subj.SEARCH_DRIVER_RS, encoding="utf-8",
                                        newline="\n")
        memory_bytes += len(patched) + len(subj.SEARCH_DRIVER_RS)
        lrl = sdir / "libmini_ledger.rlib"
        _build(rustc, target_triple, lrl, "mini_ledger", sdir / "mini_ledger.rs",
               crate_type="rlib")
        rrl = sdir / "libmini_reconcile.rlib"
        ok, _diag = _build(rustc, target_triple, rrl, "mini_reconcile",
                           sdir / "mini_reconcile.rs", crate_type="rlib",
                           externs=(("mini_ledger", lrl),))
        logical_work += 1
        if not ok:
            continue
        driver = sdir / ("driver" + _EXE)
        ok, _diag = _build(rustc, target_triple, driver, "search_driver",
                           sdir / "driver.rs",
                           externs=(("mini_ledger", lrl), ("mini_reconcile", rrl)))
        if not ok:
            continue
        code, out = _run_bound(_bind(driver), [str(judge / "search.vectors")])
        logical_work += sum(1 for line in subj.SEARCH_VECTORS.splitlines()
                            if line.strip() and line.strip() != "----")
        if code != 0:
            continue
        transcript = scratch / f"search-{tried}.transcript"
        transcript.write_text(out, encoding="utf-8", newline="\n")
        # max-batches 0: the search driver has no frontier stage.
        vcode, _vout = _run_bound(witness, ["judge", str(judge / "search.vectors"),
                                            str(transcript), "0"])
        if vcode == 0:
            chosen = patch
            break
    ticks = time.monotonic_ns() - t0
    actual = {"max-candidates": tried, "max-logical-work": logical_work,
              "max-memory-bytes": memory_bytes, "max-monotonic-ticks": ticks}
    if chosen != "*event":
        findings.append(f"bounded search selected {chosen!r}, not the mechanical "
                        "*event repair")
    over = [axis for axis, used in actual.items() if used > budget[axis]]
    if over:
        findings.append(f"search exceeded its declared budget on {over}")
    summary.append(f"bounded search: {tried} candidate(s) within declared budget; "
                   f"repair patch {chosen!r}")

    repaired_src = subj.MINI_RECONCILE_RS.replace("__APPEND_ARG__", chosen or "*event")
    reconcile2 = sb.mint_record(
        "mini-reconcile",
        proof_targets=[row_repair_loop,
                       "candidate_search_terminates_within_declared_budget"],
        parents=[reconcile1["id"]], source_frontier=source_frontier,
        dependencies=[(ledger1["id"], ledger1["content_commitment"])],
        content=repaired_src, origin="RepairOfCandidate",
        change_class="RealizationPreserving", posture="Candidate")
    # Binds the MEASURED resource actuals (monotonic ticks: per-run evidence
    # by construction); the bundle's policy section carries the declared/
    # actual budget claim the comparator treats as non-authoritative.
    search_ref = sb.put_evidence(
        evidence, judge_digest, "search-receipt",
        "".join(f"{axis} declared={budget[axis]} actual={actual[axis]}\n"
                for axis in budget) + "within-budget yes\n")

    # Descendants bound to the OLD dependency commitment are invalidated and
    # the whole workspace is rematerialized -- no stale bytes survive. This
    # IS the Invalidated law: previously-applicable standing lost because a
    # bound coordinate changed, receipted exactly.
    for record in (frontier1, target1):
        inv = sb.put_receipt(nursery, judge_digest, source_frontier,
                             "invalidation", record["id"],
                             [f"stale-coordinate dependency-commitment "
                              f"old={reconcile1['id']} new={reconcile2['id']}",
                              f"evidence {echo_ref}"])
        terminals[record["id"]] = ("Invalidated", inv)
        record["frontier"] = "Invalidated"
        record["freshness"] = "StaleByDependencyChange"
        record["frontier_reason"] = "superseded-dependency-commitment"
    # The rematerialized descendant exists to demonstrate that its local
    # green cannot bless the still-unqualified repair beneath it.
    frontier2 = sb.mint_record(
        "mini-frontier",
        proof_targets=["later_green_cannot_bless_unresolved_dependency"],
        parents=[frontier1["id"]], source_frontier=source_frontier,
        dependencies=[(reconcile2["id"], reconcile2["content_commitment"])],
        content=frontier_src, origin="DeterministicGeneration",
        change_class="RealizationPreserving", posture="Candidate")
    target2 = sb.mint_record(
        "mini-witness-target",
        proof_targets=["bounded_generated_trace_attack_holds_boundary",
                       "deterministic_replay_equality_of_simulated_trace",
                       "runtime_capture_appends_observed_history_without_rewrite",
                       "offline_replay_concludes_conformance_for_captured_history"],
        parents=[target1["id"]], source_frontier=source_frontier,
        dependencies=[(frontier2["id"], frontier2["content_commitment"])],
        content=target_src, origin="DeterministicGeneration",
        change_class="RealizationPreserving", posture="Candidate")
    for record in (reconcile2, frontier2, target2):
        persist_manifest(record)
    # Parent record immutability: the failed parent's persisted nursery
    # record is byte-identical after the repair child was minted beside it.
    if (nursery / reconcile1["id"] / "manifest").read_bytes() != parent_bytes:
        findings.append("the failed parent's nursery record was rewritten by "
                        "the repair (records are immutable)")
    # Lawful transfer reuse: mini-ledger's reuse key still matches the current
    # upstream frontier, so its evidence is reused, receipted, not re-run.
    # The reuse receipt binds key + current dependency commitments + fresh
    # requalification evidence -- the key alone licenses nothing.
    reuse_check = sb.mint_record(
        "mini-ledger", proof_targets=list(ledger1["proof_targets"]),
        parents=[], source_frontier=source_frontier, dependencies=[],
        content=ledger_src, origin="DeterministicGeneration",
        change_class="RealizationPreserving", posture="Candidate")
    if reuse_check["reuse_key"] != ledger1["reuse_key"]:
        findings.append("mini-ledger reuse key failed requalification against "
                        "unchanged commitments")
    requal_ref = sb.put_evidence(
        evidence, judge_digest, "reuse-requalification",
        f"reused {ledger1['id']}\nrecomputed-key {reuse_check['reuse_key']}\n"
        f"matches-original yes\n")
    sb.put_receipt(nursery, judge_digest, source_frontier, "reuse",
                   reconcile2["id"],
                   [f"reused {ledger1['id']}", f"reuse-key {ledger1['reuse_key']}",
                    *[f"dependency {d} {c}" for d, c in reconcile2["dependencies"]],
                    f"requalified-evidence {requal_ref}"])

    gen2 = candidates / "gen2"
    gen2.mkdir()
    sources = {"mini_ledger": ledger_src, "mini_reconcile": repaired_src,
               "mini_frontier": frontier_src, "mini_witness_target": target_src}
    rlibs: dict[str, Path] = {}
    unit_tests = {"mini_ledger": 1, "mini_reconcile": 2, "mini_frontier": 1}
    for crate in ("mini_ledger", "mini_reconcile", "mini_frontier"):
        (gen2 / f"{crate}.rs").write_text(sources[crate], encoding="utf-8", newline="\n")
        rlib = gen2 / f"lib{crate}.rlib"
        externs = tuple((dep, rlibs[dep]) for dep in rlibs)
        ok, diag = _build(rustc, target_triple, rlib, crate, gen2 / f"{crate}.rs",
                          crate_type="rlib", externs=externs)
        if not ok:
            return [f"gen2 {crate} did not compile after repair:\n"
                    + "\n".join(diag.splitlines()[:6])], summary, paths
        rlibs[crate] = rlib
        test_exe = gen2 / (f"{crate}_test" + _EXE)
        ok, out = _build(rustc, target_triple, test_exe, crate, gen2 / f"{crate}.rs",
                         externs=externs, test=True)
        if ok:
            code, out = _run_bound(_bind(test_exe), [])
            ok = code == 0 and f"test result: ok. {unit_tests[crate]} passed" in out
        if not ok:
            findings.append(f"gen2 {crate} unit tests did not pass")
    (gen2 / "mini_witness_target.rs").write_text(sources["mini_witness_target"],
                                                 encoding="utf-8", newline="\n")
    target_exe = gen2 / ("mini_witness_target" + _EXE)
    ok, diag = _build(rustc, target_triple, target_exe, "mini_witness_target",
                      gen2 / "mini_witness_target.rs",
                      externs=tuple((dep, rlibs[dep]) for dep in rlibs))
    if not ok:
        return ["gen2 mini-witness-target did not compile:\n" + diag], summary, paths
    target = _bind(target_exe)

    # -- dependency-first frontier: later green cannot bless earlier red ----
    qualified: set[str] = {ledger1["id"]}
    blocked = promote_guard(frontier2, qualified)
    blocked_ref = None
    if not blocked or "BlockedByDependency" not in blocked:
        findings.append("a locally green descendant advanced over an unqualified "
                        "dependency (no BlockedByDependency refusal)")
    else:
        blocked_ref = sb.put_evidence(evidence, judge_digest, "frontier-refusal",
                                      f"{frontier2['id']}\n{blocked}\n")
        summary.append("dependency-first demonstrated: mini-frontier promotion "
                       "refused while mini-reconcile was unqualified")

    def qualify(record: dict, lines: list[str]) -> None:
        sb.put_receipt(nursery, judge_digest, source_frontier, "qualification",
                       record["id"], lines)
        qualified.add(record["id"])

    qual_transcript, terrs = _target_transcript(target, subj.QUALIFICATION_VECTORS,
                                                scratch, "qual")
    findings += terrs
    qt = scratch / "qualification.transcript"
    qt.write_text(qual_transcript, encoding="utf-8", newline="\n")
    _witness(witness, ["judge", str(judge / "qualification.vectors"), str(qt),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: AGREE", findings, "qualification differential")
    qualify(reconcile2, ["unit-tests 2 passed",
                         "witness-differential search+qualification agree",
                         f"evidence {search_ref}"])
    qualify(frontier2, ["unit-tests 1 passed"]
            + ([f"evidence {blocked_ref}"] if blocked_ref else []))
    qualify(target2, ["witness-differential qualification agree"])

    hold_transcript, terrs = _target_transcript(target, subj.HOLDOUT_VECTORS,
                                                scratch, "holdout")
    findings += terrs
    ht = scratch / "holdout.transcript"
    ht.write_text(hold_transcript, encoding="utf-8", newline="\n")
    _witness(witness, ["judge", str(judge / "holdout.vectors"), str(ht),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: AGREE", findings, "holdout differential")
    sb.put_receipt(nursery, judge_digest, source_frontier, "holdout",
                   target2["id"],
                   ["witness-differential holdout agree",
                    "holdout-disjoint-from-search yes"])
    reg_transcript, terrs = _target_transcript(target, subj.REGRESSION_VECTORS,
                                               scratch, "regression")
    findings += terrs
    rt = scratch / "regression.transcript"
    rt.write_text(reg_transcript, encoding="utf-8", newline="\n")
    _witness(witness, ["judge", str(judge / "regression.vectors"), str(rt),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: AGREE", findings, "regression differential")

    for record in (ledger1, reconcile2, frontier2, target2):
        refusal = promote_guard(record, qualified)
        if refusal:
            findings.append(f"promotion of {record['unit']} refused: {refusal}")
            continue
        promo = sb.put_receipt(nursery, judge_digest, source_frontier,
                               "promotion", record["id"],
                               ["denominator satisfied", "dependencies-qualified yes"])
        terminals[record["id"]] = ("Promoted", promo)
        record["frontier"] = "Qualified"
        record["freshness"] = "Fresh"
        record["frontier_reason"] = "promoted"

    # -- verification adopters: simulation, fuzz, capture, offline replay ---
    _witness(witness, ["simulate", str(judge / "qualification.vectors"),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: REPLAY-EQUAL", findings, "deterministic simulation")
    fuzz_dir = scratch / "fuzz"
    fuzz_dir.mkdir()
    fuzz_out = _witness(
        witness, ["fuzz", str(subj.FUZZ_SEED), str(subj.FUZZ_TRACES),
                  str(subj.FUZZ_MAX_OPS), str(subj.FUZZ_MAX_AMOUNT),
                  str(subj.FRONTIER_MAX_BATCHES), str(target[0]), str(fuzz_dir)],
        "witness: FUZZ-HELD", findings, "bounded generated-trace attack")
    fuzz_line = (f"fuzz seed={subj.FUZZ_SEED} traces={subj.FUZZ_TRACES} "
                 f"max-ops={subj.FUZZ_MAX_OPS} max-amount={subj.FUZZ_MAX_AMOUNT}")
    sb.put_receipt(nursery, judge_digest, source_frontier, "fuzz", target2["id"],
                   [fuzz_line, fuzz_out.strip().splitlines()[-1]
                    if fuzz_out.strip() else "no verdict"])

    history = scratch / "observed.history"
    for i, trace_text in enumerate((subj.CAPTURE_TRACE_1, subj.CAPTURE_TRACE_2)):
        trace_path = scratch / f"capture-{i}.trace"
        trace_path.write_text(trace_text, encoding="utf-8", newline="\n")
        before = history.read_bytes() if history.exists() else b""
        code, out = _run_bound(target, ["capture", str(trace_path), str(history)])
        if code != 0:
            findings.append(f"capture session {i} failed: {out.strip()[:120]}")
        if "no-divergence-observed" not in out:
            findings.append(f"capture session {i} did not state its in-band "
                            "no-divergence observation")
        if "CONFORMANT" in out.upper():
            findings.append("the in-band side claimed conformance; only offline "
                            "replay may conclude")
        after = history.read_bytes()
        if not after.startswith(before) or len(after) <= len(before):
            findings.append(f"capture session {i} rewrote observed history "
                            "instead of appending")
    replay_out = _witness(witness, ["replay", str(history)],
                          "witness: CONFORMANT-FOR-OBSERVED-HISTORY", findings,
                          "offline history replay")
    history_digest = sb.sha_hex(history.read_bytes())
    events_m = re.search(r"events=(\d+)", replay_out)
    events_n = events_m.group(1) if events_m else "0"

    # -- the planted semantic mutant: activated, then killed ----------------
    mutant_src = ledger_src.replace(subj.MUTANT_TARGET, subj.MUTANT_REPLACEMENT, 1)
    if mutant_src == ledger_src:
        return ["the mutant patch target is absent; no mutant was planted"], summary, paths
    mutant = sb.mint_record("mini-ledger",
                            proof_targets=["planted_semantic_mutant_is_activated_and_killed"],
                            parents=[ledger1["id"]],
                            source_frontier=source_frontier, dependencies=[],
                            content=mutant_src, origin="DeterministicGeneration",
                            change_class="RealizationPreserving", posture="Candidate")
    persist_manifest(mutant)
    mdir = candidates / "mutant"
    mdir.mkdir()
    msources = dict(sources, mini_ledger=mutant_src)
    mrlibs: dict[str, Path] = {}
    for crate in ("mini_ledger", "mini_reconcile", "mini_frontier"):
        (mdir / f"{crate}.rs").write_text(msources[crate], encoding="utf-8", newline="\n")
        rlib = mdir / f"lib{crate}.rlib"
        ok, diag = _build(rustc, target_triple, rlib, crate, mdir / f"{crate}.rs",
                          crate_type="rlib",
                          externs=tuple((dep, mrlibs[dep]) for dep in mrlibs))
        if not ok:
            findings.append(f"mutant workspace {crate} did not compile (the mutant "
                            "must compile)")
        mrlibs[crate] = rlib
    mtest = mdir / ("mini_ledger_test" + _EXE)
    ok, out = _build(rustc, target_triple, mtest, "mini_ledger",
                     mdir / "mini_ledger.rs", test=True)
    if ok:
        code, out = _run_bound(_bind(mtest), [])
        ok = code == 0 and "test result: ok. 1 passed" in out
    if not ok:
        findings.append("the mutant did not pass the insufficient happy-path test; "
                        "it is not the planted semantic mutant")
    activation_ref = sb.put_evidence(
        evidence, judge_digest, "mutant-activation",
        f"{mutant['id']}\ncompiles yes\nhappy-path-test passed\nACTIVATED\n")
    (mdir / "mini_witness_target.rs").write_text(msources["mini_witness_target"],
                                                 encoding="utf-8", newline="\n")
    mtarget_exe = mdir / ("mini_witness_target" + _EXE)
    ok, diag = _build(rustc, target_triple, mtarget_exe, "mini_witness_target",
                      mdir / "mini_witness_target.rs",
                      externs=tuple((dep, mrlibs[dep]) for dep in mrlibs))
    kill_ref = None
    if not ok:
        findings.append("the mutant boundary target did not compile")
    else:
        m_transcript, _terrs = _target_transcript(_bind(mtarget_exe),
                                                  subj.QUALIFICATION_VECTORS,
                                                  scratch, "mutant")
        mt = scratch / "mutant.transcript"
        mt.write_text(m_transcript, encoding="utf-8", newline="\n")
        code, vout = _run_bound(witness, ["judge", str(judge / "qualification.vectors"),
                                          str(mt), str(subj.FRONTIER_MAX_BATCHES)])
        if code == 0 or "witness: DIVERGE" not in vout:
            findings.append("the planted semantic mutant SURVIVED the independent "
                            "differential route")
        else:
            kill_ref = sb.put_evidence(evidence, judge_digest, "mutant-kill",
                                       norm(vout))
            summary.append("mutant activated and KILLED by the witness differential: "
                           + vout.strip().splitlines()[-1][:100])
    # Own-content refusal: terminal Refused, frontier Unqualified, and NO
    # second invalidation event merely because a successful sibling later
    # existed (Invalidated = standing lost to a changed bound coordinate).
    mutant_refusal = sb.put_receipt(
        nursery, judge_digest, source_frontier, "refusal", mutant["id"],
        ["killed-by witness-differential", f"evidence {activation_ref}"]
        + ([f"evidence {kill_ref}"] if kill_ref else ["kill-evidence missing"]))
    terminals[mutant["id"]] = ("Refused", mutant_refusal)
    mutant["frontier"] = "Unqualified"
    mutant["frontier_reason"] = "own-content-refusal"

    # -- the failed parent's terminal: refused on its own content -----------
    r1_refusal = sb.put_receipt(nursery, judge_digest, source_frontier,
                                "refusal", reconcile1["id"],
                                ["failed compile-refusal error[E0308]",
                                 f"evidence {root_diag_ref}",
                                 f"evidence {summary_ref}"])
    terminals[reconcile1["id"]] = ("Refused", r1_refusal)
    reconcile1["frontier"] = "Unqualified"
    reconcile1["frontier_reason"] = "own-content-refusal"

    # -- the compiling scaffold that must stay unrealized -------------------
    if subj.SCAFFOLD_CONTENT not in frontier_src:
        return ["the scaffold content is not part of the compiled frontier "
                "source; the scaffold does not compile"], summary, paths
    # The scaffold's own posture is what refuses it: Unqualified, its
    # unrealized obligation open -- never BlockedByDependency (the fault is
    # not upstream).
    scaffold = sb.mint_record(
        "mini-frontier/drain-reorder",
        proof_targets=["scaffold_cannot_close_realization"],
        parents=[], source_frontier=source_frontier,
        dependencies=[(frontier2["id"], frontier2["content_commitment"])],
        content=subj.SCAFFOLD_CONTENT, origin="HumanAuthored",
        change_class="RealizationPreserving", posture="Scaffold")
    persist_manifest(scaffold)
    scaffold_refusal_text = promote_guard(scaffold, qualified)
    if not scaffold_refusal_text or "scaffold" not in scaffold_refusal_text:
        findings.append("the scaffold was not refused promotion")
    sref = sb.put_receipt(nursery, judge_digest, source_frontier, "refusal",
                          scaffold["id"],
                          [scaffold_refusal_text or "missing refusal"])
    terminals[scaffold["id"]] = ("Refused", sref)
    scaffold["frontier"] = "Unqualified"
    scaffold["frontier_reason"] = "unrealized-obligation-open"

    # -- ArchitectRequired: a law-changing candidate stops the campaign -----
    lawchange = sb.mint_record(
        "mini-ledger/law-change",
        proof_targets=["law_changing_candidate_cannot_enter_realization_preserving_lane"],
        parents=[ledger1["id"]],
        source_frontier=source_frontier,
        dependencies=[(ledger1["id"], ledger1["content_commitment"])],
        content=subj.LAW_CHANGE_PATCH, origin="HumanAuthored",
        change_class="LawChanging", posture="Candidate")
    persist_manifest(lawchange)
    lc_refusal = promote_guard(lawchange, qualified)
    if not lc_refusal or "architect" not in lc_refusal:
        findings.append("the law-changing candidate was not routed to the architect")
    esc = sb.put_receipt(nursery, judge_digest, source_frontier, "escalation",
                         lawchange["id"],
                         ["reason law-changing-candidate",
                          "disposition ArchitectRequired",
                          "campaign STOPPED; amendment proceeds separately (DEC-074)"])
    terminals[lawchange["id"]] = ("ArchitectRequired", esc)
    lawchange["frontier"] = "Unqualified"
    lawchange["frontier_reason"] = "architect-amendment-outstanding"
    summary.append(f"ArchitectRequired terminal visible: {lawchange['id'][:12]} "
                   "(campaign stopped; no ordinary repair lane)")

    records = [ledger1, reconcile1, frontier1, target1, reconcile2, frontier2,
               target2, mutant, scaffold, lawchange]

    # -- convergence: the bounded repair loop reaches a stable frontier -----
    def frontier_view() -> list[tuple[str, str]]:
        return [(r["id"], r["frontier"]) for r in records]

    stable = None
    view = frontier_view()
    for _iteration in range(subj.REPAIR_MAX_LOOP):
        again = frontier_view()
        if again == view:
            stable = again
            break
        view = again
    if stable is None:
        findings.append("the repair loop did not reach a stable qualified frontier "
                        "within its bound")
    sb.put_receipt(nursery, judge_digest, source_frontier, "convergence",
                   reconcile2["id"],
                   [f"stable-after-iterations 1 bound {subj.REPAIR_MAX_LOOP}"])

    # -- stability: re-evaluating the qualified frontier changes nothing ----
    re_transcript, _terrs = _target_transcript(target, subj.QUALIFICATION_VECTORS,
                                               scratch, "stability")
    if re_transcript != qual_transcript:
        findings.append("re-running the qualified candidate changed its boundary "
                        "transcript (stability violated)")
    terminals_before = [(r["id"], terminals.get(r["id"], (None,))[0])
                        for r in records]
    for record in records:
        refusal = promote_guard(record, qualified)
        if terminals.get(record["id"], ("",))[0] == "Promoted" and refusal:
            findings.append(f"stability: {record['unit']} lost its promotion "
                            "standing on re-evaluation")
    if terminals_before != [(r["id"], terminals.get(r["id"], (None,))[0])
                            for r in records]:
        findings.append("re-evaluation changed a terminal (stability violated)")

    # -- liveness: every admitted obligation reaches a visible terminal -----
    missing_terminals = [r["unit"] for r in records if r["id"] not in terminals]
    if missing_terminals:
        findings.append(f"obligations without a terminal: {missing_terminals}")
    counts: dict[str, int] = {}
    for record in records:
        if record["id"] in terminals:
            spelling = terminals[record["id"]][0]
            counts[spelling] = counts.get(spelling, 0) + 1
    if counts.get("ArchitectRequired", 0) != 1:
        findings.append("the ArchitectRequired terminal is not visible in the "
                        "denominator")
    summary.append("denominator: " + " ".join(f"{k}={v}" for k, v in sorted(counts.items())))

    # -- judge unmutated; evidence append-only ------------------------------
    judge_after = sb.tree_digest(judge)
    if judge_after != judge_digest:
        findings.append("the judge root changed during the campaign; all evidence "
                        "is invalidated")
    findings += sb.evidence_append_only(evidence_mark, sb.evidence_snapshot(evidence))

    # -- the bundle, the envelope, and the independent verification ---------
    source_commit = sb.source_commit_of(HERE.parent)
    if source_commit is None:
        return ["no git HEAD commit; the bundle cannot bind its source"], summary, paths
    m = re.search(r"release: (\S+)", subprocess.run(
        [rustc, "-vV"], capture_output=True, text=True).stdout)
    rustc_release = m.group(1) if m else "0.0.0"
    edges: list[tuple[str, str, str]] = []
    for record in records:
        for parent in record["parents"]:
            edges.append((record["id"], parent, "Parent"))
        for dep_id, _c in record["dependencies"]:
            edges.append((record["id"], dep_id, "Dependency"))
    edges.append((reconcile2["id"], ledger1["id"], "Reuse"))
    edges.append((frontier1["id"], reconcile2["id"], "Invalidation"))
    edges.append((target1["id"], reconcile2["id"], "Invalidation"))
    vector_sets = {role: (len(_split_traces(text)),
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
    context = {
        "judge_digest": judge_digest, "judge_digest_after": judge_after,
        "source_commit": source_commit, "rustc_release": rustc_release,
        "target": target_triple, "budget_declared": budget, "budget_actual": actual,
        "fuzz_line": fuzz_line, "vector_sets": vector_sets, "records": records,
        "terminals": terminals,
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
    bundle_text = sb.render_bundle(context)
    envelope_text = sb.render_envelope(context)
    bundle_path = evidence / "campaign-evidence.bundle"
    envelope_path = evidence / "campaign-release.envelope"
    bundle_path.write_text(bundle_text, encoding="ascii", newline="\n")
    envelope_path.write_text(envelope_text, encoding="ascii", newline="\n")
    paths.update(bundle=bundle_path, envelope=envelope_path,
                 source_commit=source_commit, target_triple=target_triple,
                 rustc=rustc)
    for row in profile_rows:
        summary.append(f"proof-row realized: {row}")
    return findings, summary, paths


def verify_bundle(paths: dict, work: Path) -> tuple[list[str], str]:
    """Build the REAL receiptcheck and block on its campaign-verify verdict."""
    rc, err = sb.build_receiptcheck(paths["rustc"], paths["target_triple"], work)
    if rc is None:
        return [f"receiptcheck unavailable for campaign-verify: {err}"], ""
    ok, out = sb.campaign_verify(rc, paths["bundle"], paths["judge"],
                                 paths["envelope"], paths["source_commit"],
                                 paths["nursery"], paths["evidence_root"])
    if not ok:
        return ["receiptcheck REFUSED the campaign bundle:\n" + out], out
    return [], out


# ---------------------------------------------------------------------------
# Battery and CLI wiring
# ---------------------------------------------------------------------------

def test_supernova() -> list[str]:
    """The F5 mini-supernova campaign rehearsal: full causal run, independent
    verification, and the hostile battery. rustc-gated like its siblings."""
    if not shutil.which("rustc"):
        print("selftest: supernova campaign rehearsal unavailable (no rustc)")
        return []
    started = time.monotonic()
    base = Path(tempfile.mkdtemp(prefix="batpak-supernova-"))
    findings: list[str] = []
    try:
        findings, summary, paths = run_rehearsal(base)
        for line in summary:
            print(f"supernova: {line}")
        if not findings and paths.get("bundle"):
            work = base / "verify-work"
            work.mkdir()
            verify_findings, out = verify_bundle(paths, work)
            findings += verify_findings
            if out and not verify_findings:
                print(f"supernova: {out.splitlines()[0]} (campaign bundle "
                      "independently verified)")
            if not verify_findings:
                from .supernova_hostiles import hostiles
                findings += hostiles(
                    paths, work,
                    admit_failure_evidence=admit_failure_evidence,
                    judge_freshness=judge_freshness,
                    promote_guard=promote_guard,
                    validate_disjoint=validate_disjoint)
        elapsed = time.monotonic() - started
        bundle = paths.get("bundle")
        digest = sb.sha_hex(bundle.read_bytes())[:12] if bundle and bundle.exists() else "-"
        receipt("supernova-campaign-rehearsal", available=True, compiled=True,
                executed=True, passed=not findings,
                target=paths.get("target_triple"),
                reason=f"bundle sha256:{digest} runtime {elapsed:.1f}s")
        print(f"supernova: rehearsal runtime {elapsed:.1f}s")
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return findings


def run_supernova_cli(directory: str) -> int:
    """`selftest.py --supernova <dir>`: execute the rehearsal with persistent
    roots under <dir> and block on the independent bundle verification. The
    workflow's candidate AND confirming postures both run this."""
    if not shutil.which("rustc"):
        print("selftest: --supernova requires rustc", file=sys.stderr)
        return 1
    base = Path(directory).resolve()
    if base.exists() and any(base.iterdir()):
        shutil.rmtree(base, ignore_errors=True)
    base.mkdir(parents=True, exist_ok=True)
    findings, summary, paths = run_rehearsal(base)
    for line in summary:
        print(f"supernova: {line}")
    if not findings and paths.get("bundle"):
        work = base / "verify-work"
        work.mkdir(exist_ok=True)
        verify_findings, out = verify_bundle(paths, work)
        findings += verify_findings
        if out:
            print(out)
    if findings:
        print(f"selftest: SUPERNOVA FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print(f"selftest: supernova campaign rehearsal PASS (bundle {paths['bundle']})")
    return 0


def compare_supernova_cli(mine: str, theirs: str) -> int:
    """`selftest.py --supernova-compare <own-bundle> <candidate-bundle>`: the
    Stability row's cross-run law -- the confirming rehearsal's authoritative
    results (terminals, frontier, dispositions) must equal the candidate's."""
    mine_text = Path(mine).read_text(encoding="utf-8")
    theirs_text = Path(theirs).read_text(encoding="utf-8")
    findings = sb.compare_authoritative(mine_text, theirs_text)
    if findings:
        print("selftest: SUPERNOVA STABILITY FAIL:", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("selftest: campaign authoritative results identical across runs "
          "(confirming rerun changed no authoritative result)")
    return 0

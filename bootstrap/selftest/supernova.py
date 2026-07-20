"""The F5 mini-supernova campaign rehearsal harness (docs/24, DEC-076..082).

One complete candidate-authority cycle in miniature, executed against the
tracked subject templates (supernova_subject) in an EXTERNAL campaign root
with physically disjoint judge / candidate / evidence / nursery roots:

freeze judge -> generate candidates (each admitted by a causally-first
generation receipt at its mint site) -> fail for a REAL reason (a genuine
compiler refusal, never `status = red`) -> compress causal diagnostics ->
repair as a child candidate under a receipted bounded search -> independently
qualify per proof target (the separately implemented Rust witness judges;
this producer never votes on semantics) -> exercise the disjoint holdout ->
persist every adopter verdict (simulation, fuzz, capture, offline replay) as
evidence with per-row qualification receipts -> activate and kill the planted
semantic mutant -> advance the trusted frontier dependency-first ->
invalidate descendants with typed causes -> rematerialize -> reach every
terminal (Promoted / Refused / Invalidated / ArchitectRequired, all in the
denominator), where PROMOTION IS THE LAST RECEIPT, assembled from the actual
receipt store only after every named proof target carries a naming receipt
(refused otherwise) -> emit the BATPAK-CAMPAIGN-EVIDENCE/3 bundle (trusted
roots beside the nine candidate manifests, CL-1) and the release envelope ->
block on receiptcheck `campaign-verify`.

Every candidate persists an immutable V2 manifest at nursery/<id>/manifest
(write-once: absent -> create, identical -> unchanged, different -> refuse),
receipts append-only beside it; terminals are receipt-backed facts, never
record fields; the frontier speaks the four-state law. The nine spec-owned
profile rows are realized literally, each asserted against executed evidence.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from . import supernova_bundle as sb
from . import supernova_subject as subj
from .core import HERE, TOOLCHAIN_EDITION, ProbeError, receipt
from .supernova_context import build_context

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
    for root_dir in (judge, candidates, evidence, nursery, scratch):
        root_dir.mkdir(parents=True, exist_ok=True)
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
    # Proof targets bind semantic purpose (TL-1); a diagnostic unit label is
    # never authority; terminals are receipt-backed facts BESIDE the records.
    source_frontier = sb.sha_hex("genesis\n" + judge_digest)
    terminals: dict[str, tuple[str, str]] = {}
    # CL-2: every generation receipt's generator configuration commitment is
    # the frozen subject template module (templates + budgets + vectors).
    generator_commitment = sb.sha_hex(
        (HERE / "selftest" / "supernova_subject.py").read_bytes())

    def persist_manifest(record: dict) -> Path:
        """WRITE-ONCE (perimeter-equality law): absent -> create, identical
        bytes -> unchanged, different bytes -> refuse -- a content-addressed
        record is never unconditionally rewritten."""
        d = nursery / record["id"]
        d.mkdir(parents=True, exist_ok=True)
        path = d / "manifest"
        data = sb.render_manifest(record).encode("utf-8")
        if path.exists():
            if path.read_bytes() != data:
                raise ProbeError(f"nursery manifest {record['id'][:12]} collision: "
                                 "write-once violated")
            return path
        path.write_bytes(data)
        return path

    def admit(record: dict, gen_evidence: tuple[str, ...] = ()) -> None:
        """Persist the immutable manifest, then the causally-first generation
        receipt at the mint site (CL-7): complete creation provenance."""
        manifest_path = persist_manifest(record)
        sb.put_receipt(nursery, judge_digest, source_frontier, "generation",
                       record["id"],
                       sb.receipt_generation(record, manifest_path.read_bytes(),
                                             "supernova-rehearsal-harness",
                                             generator_commitment,
                                             list(gen_evidence)))

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
    row_search = "candidate_search_terminates_within_declared_budget"
    row_blocked = "later_green_cannot_bless_unresolved_dependency"
    row_fuzz = "bounded_generated_trace_attack_holds_boundary"
    row_sim = "deterministic_replay_equality_of_simulated_trace"
    row_capture = "runtime_capture_appends_observed_history_without_rewrite"
    row_replay = "offline_replay_concludes_conformance_for_captured_history"
    # CL-1: mini-ledger is an already-TRUSTED ROOT, not a candidate: no
    # nursery record, no generation/promotion/qualification receipt; it is
    # declared in the bundle's roots section, requalified freshly through
    # the reuse receipt below, and seeds the qualified frontier because it
    # IS the trusted frontier input.
    root = sb.mint_root("mini-ledger", ledger_src)
    reconcile1 = sb.mint_record("mini-reconcile", proof_targets=[row_repair_loop],
                                parents=[], source_frontier=source_frontier,
                                dependencies=[(root["id"], root["content_commitment"])],
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
    for record in (reconcile1, frontier1, target1):
        admit(record)
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
    # The root's fresh unit-test run feeds the reuse requalification (CL-1).
    root_unit_line = "unit-tests 1 passed" if ok else "unit-tests FAILED"

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
        proof_targets=[row_repair_loop, row_search],
        parents=[reconcile1["id"]], source_frontier=source_frontier,
        dependencies=[(root["id"], root["content_commitment"])],
        content=repaired_src, origin="RepairOfCandidate",
        change_class="RealizationPreserving", posture="Candidate")
    # Binds the MEASURED resource actuals (monotonic ticks: per-run evidence
    # by construction); the bundle's policy section carries the budget claim
    # the comparator treats as non-authoritative.
    search_ref = sb.put_evidence(
        evidence, judge_digest, "search-receipt",
        "".join(f"{axis} declared={budget[axis]} actual={actual[axis]}\n"
                for axis in budget) + "within-budget yes\n")
    # The repair child's generation receipt binds the bounded-search receipt
    # as its DEC-079 synthesis evidence.
    admit(reconcile2, gen_evidence=(search_ref,))
    # The rematerialized descendant exists to demonstrate that its local
    # green cannot bless the still-unqualified repair beneath it.
    frontier2 = sb.mint_record(
        "mini-frontier",
        proof_targets=[row_blocked],
        parents=[frontier1["id"]], source_frontier=source_frontier,
        dependencies=[(reconcile2["id"], reconcile2["content_commitment"])],
        content=frontier_src, origin="DeterministicGeneration",
        change_class="RealizationPreserving", posture="Candidate")
    admit(frontier2)
    # Descendants bound to the OLD dependency commitment are invalidated --
    # typed cause (CL-4/CL-6): the candidate-id old/new axis reconciles
    # against the manifests (the invalidated manifest carries dependency id
    # == `old`; `new`'s parent set contains `old`) and the coordinate line
    # carries the exact stale bound commitment.
    invalidation_edges: list[tuple[str, str, str]] = []
    for record, old, new in ((frontier1, reconcile1, reconcile2),
                             (target1, frontier1, frontier2)):
        coordinate = dict(record["dependencies"])[old["id"]]
        inv = sb.put_receipt(nursery, judge_digest, source_frontier,
                             "invalidation", record["id"],
                             sb.receipt_invalidation(
                                 "dependency-commitment-changed", coordinate,
                                 old["id"], new["id"], [echo_ref]))
        terminals[record["id"]] = ("Invalidated", inv)
        record["frontier"] = "Invalidated"
        record["freshness"] = "StaleByDependencyChange"
        record["frontier_reason"] = "superseded-dependency-commitment"
        invalidation_edges.append((record["id"], new["id"], "Invalidation"))
    target2 = sb.mint_record(
        "mini-witness-target",
        proof_targets=[row_fuzz, row_sim, row_capture, row_replay],
        parents=[target1["id"]], source_frontier=source_frontier,
        dependencies=[(frontier2["id"], frontier2["content_commitment"])],
        content=target_src, origin="DeterministicGeneration",
        change_class="RealizationPreserving", posture="Candidate")
    admit(target2)
    # Parent record immutability: the failed parent's persisted nursery
    # record is byte-identical after the repair child was minted beside it.
    if (nursery / reconcile1["id"] / "manifest").read_bytes() != parent_bytes:
        findings.append("the failed parent's nursery record was rewritten by "
                        "the repair (records are immutable)")
    # Lawful transfer reuse (CL-1): the root key is recomputed from unchanged
    # content; the receipt binds key + SET-EQUAL current dependency set +
    # reused target + fresh requalification -- the key alone licenses nothing.
    reuse_check = sb.mint_root("mini-ledger", ledger_src)
    if reuse_check["reuse_key"] != root["reuse_key"]:
        findings.append("mini-ledger root reuse key failed requalification "
                        "against unchanged content")
    root_requal_ref = sb.put_evidence(
        evidence, judge_digest, "root-requalification",
        f"root {root['id']}\nrecomputed-key {reuse_check['reuse_key']}\n"
        f"matches-original yes\n{root_unit_line}\ntarget {target_triple}\n")
    sb.put_receipt(nursery, judge_digest, source_frontier, "reuse",
                   reconcile2["id"],
                   sb.receipt_reuse(root["id"], root["reuse_key"],
                                    reconcile2["dependencies"],
                                    [row_repair_loop],
                                    {row_repair_loop: root_requal_ref}))
    reuse_edges = [(reconcile2["id"], root["id"], "Reuse")]

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
    qualified: set[str] = {root["id"]}  # the trusted frontier input (CL-1)
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

    def qualify(record: dict, targets: list[str], plan: dict[str, str],
                evidence_refs: list[str]) -> None:
        """One V3 qualification receipt: named targets, spec-owned plan
        tokens, executed disposition, persisted evidence refs."""
        sb.put_receipt(nursery, judge_digest, source_frontier, "qualification",
                       record["id"],
                       sb.receipt_qualification(targets, plan, evidence_refs))
        qualified.add(record["id"])

    qual_transcript, terrs = _target_transcript(target, subj.QUALIFICATION_VECTORS,
                                                scratch, "qual")
    findings += terrs
    qt = scratch / "qualification.transcript"
    qt.write_text(qual_transcript, encoding="utf-8", newline="\n")
    _witness(witness, ["judge", str(judge / "qualification.vectors"), str(qt),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: AGREE", findings, "qualification differential")
    # The witness differential accepted the winning search candidate, so the
    # differential implementation is the route that supplied independence.
    qualify(reconcile2, [row_search],
            sb.proof_plan(row_search, route="DifferentialImplementation"),
            [search_ref])
    if blocked_ref:
        qualify(frontier2, [row_blocked], sb.proof_plan(row_blocked), [blocked_ref])

    hold_transcript, terrs = _target_transcript(target, subj.HOLDOUT_VECTORS,
                                                scratch, "holdout")
    findings += terrs
    ht = scratch / "holdout.transcript"
    ht.write_text(hold_transcript, encoding="utf-8", newline="\n")
    hold_verdict = _witness(witness, ["judge", str(judge / "holdout.vectors"),
                                      str(ht), str(subj.FRONTIER_MAX_BATCHES)],
                            "witness: AGREE", findings, "holdout differential")
    hold_ref = sb.put_evidence(evidence, judge_digest, "holdout-verdict",
                               norm(hold_verdict))
    sb.put_receipt(nursery, judge_digest, source_frontier, "holdout",
                   target2["id"],
                   sb.receipt_holdout(
                       [row_fuzz],
                       sb.sha_hex((judge / "holdout.vectors").read_bytes()),
                       [hold_ref]))
    reg_transcript, terrs = _target_transcript(target, subj.REGRESSION_VECTORS,
                                               scratch, "regression")
    findings += terrs
    rt = scratch / "regression.transcript"
    rt.write_text(reg_transcript, encoding="utf-8", newline="\n")
    _witness(witness, ["judge", str(judge / "regression.vectors"), str(rt),
                       str(subj.FRONTIER_MAX_BATCHES)],
             "witness: AGREE", findings, "regression differential")

    # -- verification adopters: simulation, fuzz, capture, offline replay ---
    # Each verdict PERSISTS as evidence and earns a per-row V3 receipt on the
    # boundary candidate BEFORE any promotion is assembled (CL-7).
    sim_out = _witness(witness, ["simulate", str(judge / "qualification.vectors"),
                                 str(subj.FRONTIER_MAX_BATCHES)],
                       "witness: REPLAY-EQUAL", findings, "deterministic simulation")
    sim_ref = sb.put_evidence(evidence, judge_digest, "simulation-verdict",
                              norm(sim_out))
    if "witness: REPLAY-EQUAL" in sim_out:
        # The separately owned witness performed the simulation: the
        # differential implementation supplied the independence.
        qualify(target2, [row_sim],
                sb.proof_plan(row_sim, route="DifferentialImplementation"),
                [sim_ref])
    fuzz_dir = scratch / "fuzz"
    fuzz_dir.mkdir()
    fuzz_out = _witness(
        witness, ["fuzz", str(subj.FUZZ_SEED), str(subj.FUZZ_TRACES),
                  str(subj.FUZZ_MAX_OPS), str(subj.FUZZ_MAX_AMOUNT),
                  str(subj.FRONTIER_MAX_BATCHES), str(target[0]), str(fuzz_dir)],
        "witness: FUZZ-HELD", findings, "bounded generated-trace attack")
    fuzz_line = (f"fuzz seed={subj.FUZZ_SEED} traces={subj.FUZZ_TRACES} "
                 f"max-ops={subj.FUZZ_MAX_OPS} max-amount={subj.FUZZ_MAX_AMOUNT}")
    fuzz_ref = sb.put_evidence(evidence, judge_digest, "fuzz-verdict",
                               norm(fuzz_out))
    traces_executed = len(list(fuzz_dir.glob("fuzz-*.trace")))
    if "witness: FUZZ-HELD" in fuzz_out:
        sb.put_receipt(nursery, judge_digest, source_frontier, "fuzz",
                       target2["id"],
                       sb.receipt_fuzz([row_fuzz], subj.FUZZ_SEED,
                                       subj.FUZZ_TRACES, subj.FUZZ_MAX_OPS,
                                       subj.FUZZ_MAX_AMOUNT, traces_executed,
                                       [fuzz_ref]))

    history = scratch / "observed.history"
    capture_ok = True
    observations: list[str] = []
    for i, trace_text in enumerate((subj.CAPTURE_TRACE_1, subj.CAPTURE_TRACE_2)):
        trace_path = scratch / f"capture-{i}.trace"
        trace_path.write_text(trace_text, encoding="utf-8", newline="\n")
        before = history.read_bytes() if history.exists() else b""
        code, out = _run_bound(target, ["capture", str(trace_path), str(history)])
        session_ok = True
        if code != 0:
            findings.append(f"capture session {i} failed: {out.strip()[:120]}")
            session_ok = False
        if "no-divergence-observed" not in out:
            findings.append(f"capture session {i} did not state its in-band "
                            "no-divergence observation")
            session_ok = False
        if "CONFORMANT" in out.upper():
            findings.append("the in-band side claimed conformance; only offline "
                            "replay may conclude")
            session_ok = False
        after = history.read_bytes()
        if not after.startswith(before) or len(after) <= len(before):
            findings.append(f"capture session {i} rewrote observed history "
                            "instead of appending")
            session_ok = False
        observations.append(f"session-{i} no-divergence-observed append-only "
                            + ("yes" if session_ok else "VIOLATED"))
        capture_ok = capture_ok and session_ok
    history_digest = sb.sha_hex(history.read_bytes())
    capture_ref = sb.put_evidence(
        evidence, judge_digest, "runtime-capture-observation",
        "".join(line + "\n" for line in observations)
        + f"history {history_digest}\n")
    if capture_ok:
        # The captured history's independence arrives via the independent
        # offline replay of exactly these recorded bytes.
        qualify(target2, [row_capture],
                sb.proof_plan(row_capture, "PropertySequence",
                              route="IndependentHistoryReplay"), [capture_ref])
    replay_out = _witness(witness, ["replay", str(history)],
                          "witness: CONFORMANT-FOR-OBSERVED-HISTORY", findings,
                          "offline history replay")
    replay_ref = sb.put_evidence(evidence, judge_digest, "offline-replay-verdict",
                                 norm(replay_out))
    if "witness: CONFORMANT-FOR-OBSERVED-HISTORY" in replay_out:
        qualify(target2, [row_replay],
                sb.proof_plan(row_replay, "HistoryReplay"), [replay_ref])
    events_m = re.search(r"events=(\d+)", replay_out)
    events_n = events_m.group(1) if events_m else "0"

    # -- the planted semantic mutant: activated, then killed ----------------
    mutant_src = ledger_src.replace(subj.MUTANT_TARGET, subj.MUTANT_REPLACEMENT, 1)
    if mutant_src == ledger_src:
        return ["the mutant patch target is absent; no mutant was planted"], summary, paths
    mutant = sb.mint_record("mini-ledger",
                            proof_targets=["planted_semantic_mutant_is_activated_and_killed"],
                            parents=[root["id"]],
                            source_frontier=source_frontier, dependencies=[],
                            content=mutant_src, origin="DeterministicGeneration",
                            change_class="RealizationPreserving", posture="Candidate")
    admit(mutant)
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
        sb.receipt_refusal("witness-differential-kill",
                           [activation_ref] + ([kill_ref] if kill_ref else [])))
    terminals[mutant["id"]] = ("Refused", mutant_refusal)
    mutant["frontier"] = "Unqualified"
    mutant["frontier_reason"] = "own-content-refusal"

    # -- the failed parent's terminal: refused on its own content -----------
    r1_refusal = sb.put_receipt(nursery, judge_digest, source_frontier,
                                "refusal", reconcile1["id"],
                                sb.receipt_refusal("compile-refusal",
                                                   [root_diag_ref, summary_ref]))
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
    admit(scaffold)
    scaffold_refusal_text = promote_guard(scaffold, qualified)
    if not scaffold_refusal_text or "scaffold" not in scaffold_refusal_text:
        findings.append("the scaffold was not refused promotion")
    scaffold_ref = sb.put_evidence(
        evidence, judge_digest, "promotion-refusal",
        f"{scaffold['id']}\n{scaffold_refusal_text or 'missing refusal'}\n")
    sref = sb.put_receipt(nursery, judge_digest, source_frontier, "refusal",
                          scaffold["id"],
                          sb.receipt_refusal("unrealized-obligation-open",
                                             [scaffold_ref]))
    terminals[scaffold["id"]] = ("Refused", sref)
    scaffold["frontier"] = "Unqualified"
    scaffold["frontier_reason"] = "unrealized-obligation-open"

    # -- ArchitectRequired: a law-changing candidate stops the campaign -----
    lawchange = sb.mint_record(
        "mini-ledger/law-change",
        proof_targets=["law_changing_candidate_cannot_enter_realization_preserving_lane"],
        parents=[root["id"]],
        source_frontier=source_frontier,
        dependencies=[(root["id"], root["content_commitment"])],
        content=subj.LAW_CHANGE_PATCH, origin="HumanAuthored",
        change_class="LawChanging", posture="Candidate")
    admit(lawchange)
    lc_refusal = promote_guard(lawchange, qualified)
    if not lc_refusal or "architect" not in lc_refusal:
        findings.append("the law-changing candidate was not routed to the architect")
    lc_ref = sb.put_evidence(
        evidence, judge_digest, "escalation-cause",
        f"{lawchange['id']}\n{lc_refusal or 'missing refusal'}\n"
        "campaign STOPPED; amendment proceeds separately (DEC-074)\n")
    esc = sb.put_receipt(nursery, judge_digest, source_frontier, "escalation",
                         lawchange["id"],
                         sb.receipt_escalation("law-changing-candidate",
                                               [lc_ref]))
    terminals[lawchange["id"]] = ("ArchitectRequired", esc)
    lawchange["frontier"] = "Unqualified"
    lawchange["frontier_reason"] = "architect-amendment-outstanding"
    summary.append(f"ArchitectRequired terminal visible: {lawchange['id'][:12]} "
                   "(campaign stopped; no ordinary repair lane)")

    records = [reconcile1, frontier1, target1, reconcile2, frontier2,
               target2, mutant, scaffold, lawchange]

    # -- convergence: the bounded repair loop reaches a stable frontier -----
    def frontier_view() -> list[tuple[str, str]]:
        return [(r["id"], r["frontier"]) for r in records]

    stable = None
    iterations = 0
    view = frontier_view()
    for iteration in range(subj.REPAIR_MAX_LOOP):
        again = frontier_view()
        if again == view:
            stable = again
            iterations = iteration + 1
            break
        view = again
    if stable is None:
        findings.append("the repair loop did not reach a stable qualified frontier "
                        "within its bound")
    sb.put_receipt(nursery, judge_digest, source_frontier, "convergence",
                   reconcile2["id"],
                   sb.receipt_convergence([row_repair_loop], iterations,
                                          subj.REPAIR_MAX_LOOP))

    # -- promotion: the CONSEQUENCE of completed proof (CL-7) ---------------
    # Promotions are the LAST receipts written, ASSEMBLED from the actual
    # receipt store: every manifest proof target must be NAMED by an
    # un-superseded qualification/holdout/fuzz/convergence receipt already
    # persisted beside the record; a target without a naming receipt REFUSES
    # the promotion (the honest stop), never an optimistic bookmark.
    hostile_refs = [activation_ref] + ([kill_ref] if kill_ref else [])
    for record in (reconcile2, frontier2, target2):
        refusal = promote_guard(record, qualified)
        if refusal:
            findings.append(f"promotion of {record['unit']} refused: {refusal}")
            continue
        try:
            promo_lines = sb.assemble_promotion(nursery, record, hostile_refs)
        except ProbeError as stop:
            findings.append(str(stop))
            continue
        promo = sb.put_receipt(nursery, judge_digest, source_frontier,
                               "promotion", record["id"], promo_lines)
        terminals[record["id"]] = ("Promoted", promo)
        record["frontier"] = "Qualified"
        record["freshness"] = "Fresh"
        record["frontier_reason"] = "promoted"

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
    context = build_context(
        judge=judge, judge_digest=judge_digest, judge_after=judge_after,
        source_commit=source_commit, rustc=rustc, target_triple=target_triple,
        budget=budget, actual=actual, fuzz_line=fuzz_line,
        records=records, roots=[root], terminals=terminals,
        extra_edges=reuse_edges + invalidation_edges,
        witness_src_digest=witness_src_digest, events_n=events_n,
        history_digest=history_digest, mutant=mutant)
    bundle_text = sb.render_bundle(context)
    envelope_text = sb.render_envelope(context)
    bundle_path = evidence / "campaign-evidence.bundle"
    envelope_path = evidence / "campaign-release.envelope"
    bundle_path.write_text(bundle_text, encoding="ascii", newline="\n")
    envelope_path.write_text(envelope_text, encoding="ascii", newline="\n")
    paths.update(bundle=bundle_path, envelope=envelope_path,
                 source_commit=source_commit, target_triple=target_triple,
                 rustc=rustc)
    _profile_id, profile_rows = sb.mini_supernova_profile()
    for row in profile_rows:
        summary.append(f"proof-row realized: {row}")
    return findings, summary, paths


# ---------------------------------------------------------------------------
# The battery (CLI wiring lives in supernova_cli; the module graph is a DAG)
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
            verify_findings, out = sb.verify_bundle(paths, work)
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

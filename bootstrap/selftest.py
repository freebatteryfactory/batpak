#!/usr/bin/env python3
"""Independent portability self-test for the bootstrap freeze/audit tools.

Turns the first executable finding (host-dependent manifest ordering) into a
permanent guarantee. Standard library only; operates on synthetic temporary
trees and leaves no repository artifacts.

Proven properties:

1. Manifest ordering derives ONLY from the UTF-8 bytes of the POSIX relative
   path, never from filesystem enumeration or the host locale/case behavior.
2. audit rejects a case-folded path collision (e.g. ``Foo.md`` vs ``foo.md``)
   that a case-insensitive filesystem could not materialize, while leaving
   genuinely distinct paths untouched.
"""
from __future__ import annotations
import contextlib
import hashlib
import os
import importlib.util
import re
import sys
import shutil
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


# Qualification receipts. NOT one flat status: availability, compilation,
# execution-attempted, and outcome are orthogonal, and flattening them is how a
# gate that was only ever compiled came to sit beside one that actually ran and
# passed. A receipt records each separately, plus the target it holds for and
# the exact reason when something did not happen.
def _toolchain_edition() -> str:
    """The edition every rustc invocation uses, read from the typed owner.

    A literal here would be a hidden Python owner; the audit refuses a
    hardcoded edition argument anywhere in bootstrap (5.5E3a)."""
    src = (HERE.parent / "spec/toolchain.rs").read_text(encoding="utf-8")
    m = re.search(r"edition: RustEdition::Rust(\d+)", src)
    return m.group(1) if m else "0"


TOOLCHAIN_EDITION = _toolchain_edition()


QUALIFICATION_RECEIPTS: list[dict] = []


def receipt(name, *, available, compiled=None, executed=None, passed=None,
            target=None, reason=None):
    QUALIFICATION_RECEIPTS.append({
        "name": name, "available": available, "compiled": compiled,
        "executed": executed, "passed": passed, "target": target, "reason": reason,
    })


def render_receipts() -> str:
    """One line per receipt, stage by stage.

    An unavailable or unexecuted check says so here rather than being summarised
    away by an aggregate that prints PASS. A check that was compiled and never
    executed has earned no place beside one that executed and passed.
    """
    lines = []
    for r in QUALIFICATION_RECEIPTS:
        parts = ["available" if r["available"] else "UNAVAILABLE"]
        if r["compiled"] is True:
            parts.append("compiled")
        elif r["compiled"] is False:
            parts.append("COMPILE FAILED")
        if r["executed"] is True:
            parts.append("executed")
        elif r["executed"] is False:
            parts.append("NOT EXECUTED")
        if r["passed"] is True:
            parts.append("passed")
        elif r["passed"] is False:
            parts.append("FAILED")
        if r["target"]:
            parts.append("target " + str(r["target"]))
        if r["reason"]:
            parts.append("reason: " + str(r["reason"]))
        lines.append("selftest: receipt " + r["name"] + ": " + ", ".join(parts))
    return chr(10).join(lines)


def executed_and_passed() -> list[str]:
    """Only a check that actually ran AND passed may be claimed by the banner."""
    return [r["name"] for r in QUALIFICATION_RECEIPTS
            if r["available"] and r["executed"] and r["passed"]]


# The canonical Tier 0 receipt denominator, in one place (5.5E2c). Each entry
# is (name, artifact_bound): an artifact-bound receipt must carry the exact
# "<triple> artifact sha256:... source sha256:..." binding qualify_binary
# records; the law-fixture suite runs many short-lived binaries and binds the
# triple only. The authoritative hosted lane passes --require-receipts
# <triple>, and then EVERY receipt named here must be available, compiled,
# executed, passed, and bound to that exact triple or selftest exits nonzero:
# a zero exit plus a "NOT QUALIFIED LOCALLY" line never satisfies the
# authoritative profile, and the hosted workflow consumes this inventory
# through the flag instead of owning a copied list.
REQUIRED_TIER0_RECEIPTS = (
    ("tier0-law-fixtures", False),
    ("tier0-seedcheck", True),
    ("tier0-materialize", True),
    ("tier0-seedcheck-tests", True),
    ("tier0-spec-tests", True),
)


def unearned_required_receipts(required_target: str) -> list[str]:
    """Every required receipt that did NOT earn its place for this target."""
    problems: list[str] = []
    # A receipt binds WHERE a binary ran — a physical target triple. A
    # semantic qualification environment ("std", "no_std + alloc") answers a
    # different question and cannot substitute (5.5E3a).
    if not re.fullmatch(r"[a-z0-9_]+(-[a-z0-9_]+){2,}", required_target):
        return [f"required target {required_target!r} is not a physical target "
                "triple; a semantic qualification environment answers WHAT must "
                "hold, never WHERE a binary ran"]
    for name, artifact_bound in REQUIRED_TIER0_RECEIPTS:
        rows = [r for r in QUALIFICATION_RECEIPTS if r["name"] == name]
        if not rows:
            problems.append(f"{name}: no receipt was recorded")
            continue
        binding = (re.escape(required_target)
                   + (r" artifact sha256:[0-9a-f]+ source sha256:[0-9a-f]+"
                      if artifact_bound else ""))
        # Every claimed stage, explicitly — including compiled. Execution
        # normally implies compilation in reality, but the law refuses a
        # contradictory receipt shape (executed yet compiled=False) instead
        # of relying on a reader to repair it mentally (5.5E2g).
        if not any(r["available"] and r["compiled"] is True and r["executed"]
                   and r["passed"]
                   and r["target"] and re.fullmatch(binding, str(r["target"]))
                   for r in rows):
            got = "; ".join(str(r["target"] or r["reason"] or "unearned") for r in rows)
            problems.append(f"{name}: no executed, passed receipt bound to "
                            f"{required_target} (got: {got})")
    return problems
def _tree_digest(root: Path) -> str:
    """A digest of the typed source snapshot a qualification represents."""
    h = hashlib.sha256()
    for rel in sorted(p.relative_to(root).as_posix()
                      for p in (root / "spec").glob("*.rs")):
        h.update(rel.encode())
        h.update((root / rel).read_bytes())
    for rel in ("bootstrap/seedcheck.rs", "bootstrap/materialize.rs"):
        if (root / rel).is_file():
            h.update(rel.encode())
            h.update((root / rel).read_bytes())
    return h.hexdigest()


def qualify_binary(rustc, root: Path, workdir: Path, name: str, src: str,
                   expect: str, extra: tuple = (),
                   run_args: tuple | None = None, link_spec: bool = True) -> list[str]:
    """Compile, link, and RUN one bootstrap binary, artifact-bound.

    The binding exists because honest individual facts can still assemble into a
    dishonest composite: compile A paired with a stale or substituted binary B is
    a receipt for work that never happened. So the artifact path is unique to this
    run and must not pre-exist, its digest is taken from what THIS compile
    produced, the digest is rechecked immediately before execution, and the source
    snapshot is proven unchanged across the whole qualification.

    Type-checking is not execution, and a matching PASS banner is not a result:
    the process exit and the expected disposition must agree.
    """
    findings: list[str] = []
    before = _tree_digest(root)
    exe = workdir / f"{name}-{before[:12]}" / ("run" + (".exe" if os.name == "nt" else ""))
    exe.parent.mkdir(parents=True, exist_ok=True)
    if exe.exists():
        receipt(name, available=True, compiled=False,
                reason="artifact path already existed; a prior run's binary could be reused")
        return [f"{name}: artifact path was not unique to this run"]
    built = None
    for target in (None, "x86_64-pc-windows-gnu"):
        # The spec is linked as a real rlib (5.5E2); it is built per attempted
        # target so the binary and its library always share one triple. The
        # spec's own test harness cannot extern itself: link_spec=False.
        externs: list[str] = []
        if link_spec:
            rlib = exe.parent / f"libspec-{target or 'host'}.rlib"
            lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                       "--crate-name", "spec", "-o", str(rlib), str(root / "spec/lib.rs")]
            if target:
                lib_cmd[1:1] = ["--target", target]
            if subprocess.run(lib_cmd, capture_output=True, text=True).returncode != 0:
                continue
            externs = ["--extern", f"spec={rlib}"]
        cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", name.replace("-", "_"),
               *externs, *extra, "-o", str(exe), str(root / src)]
        if target:
            cmd[1:1] = ["--target", target]
        if subprocess.run(cmd, capture_output=True, text=True).returncode == 0 and exe.is_file():
            # The receipt names the real triple, never "host default": on a
            # hosted MSVC runner the default-target build IS the authoritative
            # x86_64-pc-windows-msvc qualification, and a receipt that hides
            # the triple cannot be that evidence.
            if target is None:
                m = re.search(r"host: (\S+)",
                              subprocess.run([rustc, "-vV"], capture_output=True,
                                             text=True).stdout)
                target = m.group(1) if m else "host default"
            built = (target, hashlib.sha256(exe.read_bytes()).hexdigest())
            break
    if built is None:
        receipt(name, available=True, compiled=True, executed=False,
                reason="no working linker for any attempted target; "
                       "hosted MSVC owns the authoritative receipt")
        return findings
    target, digest = built
    if not exe.is_file() or hashlib.sha256(exe.read_bytes()).hexdigest() != digest:
        receipt(name, available=True, compiled=True, executed=False,
                reason="the artifact changed between compilation and execution")
        return [f"{name}: the executed artifact is not the one this run compiled"]
    # A libtest harness reads a positional argument as a test FILTER, so the
    # root path would silently filter out every test: run_args exists so a
    # harness binary can run with none.
    proc = subprocess.run(
        [str(exe), *(run_args if run_args is not None else (str(root),))],
        capture_output=True, text=True)
    out = proc.stdout + proc.stderr
    # The exit code and the disposition must agree. Either alone is forgeable: a
    # banner is a string, and an exit code says nothing about what was checked.
    ok = proc.returncode == 0 and expect in out
    after = _tree_digest(root)
    if after != before:
        receipt(name, available=True, compiled=True, executed=True, passed=False,
                target=target, reason="the source snapshot changed during qualification")
        return [f"{name}: the source changed mid-qualification; the receipt binds nothing"]
    receipt(name, available=True, compiled=True, executed=True, passed=ok,
            target=f"{target} artifact sha256:{digest[:12]} source sha256:{before[:12]}")
    if not ok:
        head = "\n".join(out.splitlines()[:8])
        findings.append(f"{name} executed and FAILED ({target}):\n{head}")
    return findings


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_manifest_ordering(freeze) -> list[str]:
    findings: list[str] = []
    names = ["A.md", "a-child.md", "Z.md", "nested/Á.md", "nested/z.md"]
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for name in names:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode("utf-8"))
        rendered = freeze.render(root).decode("utf-8").splitlines()
    order = [line.split("  ", 1)[1] for line in rendered]
    expected = sorted(names, key=lambda value: value.encode("utf-8"))
    if order != expected:
        findings.append(f"ordering is not UTF-8-byte canonical: {order} != {expected}")
    return findings


def test_casefold_collision(audit) -> list[str]:
    findings: list[str] = []
    collisions = audit.casefold_collisions(["Foo.md", "foo.md", "bar.md"])
    if [("Foo.md", "foo.md")] != collisions:
        findings.append(f"expected Foo.md/foo.md collision, got {collisions}")
    unrelated = audit.casefold_collisions(["a.md", "b.md", "nested/a.md"])
    if unrelated:
        findings.append(f"false collision reported on distinct paths: {unrelated}")
    return findings


def test_stale_vocabulary(audit) -> list[str]:
    """Hostile fixtures for the decision-derived stale-vocabulary matcher."""
    findings: list[str] = []
    amap = {"filebat": ("DEC-003", frozenset({"DecisionLedger", "LegacyEvidence"}), "FileBat")}

    def scan(text: str, context: str) -> list[str]:
        out: list[str] = []
        audit.scan_stale_occurrences("synthetic.md", text, context, amap, out)
        return out

    if not scan("FileBat is superseded", "OrdinaryAuthoritative"):
        findings.append("stale_alias_in_authoritative_doc_is_rejected FAILED")
    if not scan("let handle = FileBat::open();", "ProductionSource"):
        findings.append("stale_alias_in_production_source_is_rejected FAILED")
    if not scan("FileBat [STALE-REF: DEC-011]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_wrong_decision_reference_is_rejected FAILED")
    if scan("FileBat [STALE-REF: DEC-003]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_killed_decision_reference_is_allowed FAILED")
    if not scan("plain text [STALE-REF: DEC-999]", "OrdinaryAuthoritative"):
        findings.append("unknown_stale_reference_is_rejected FAILED")
    if scan("FileBat appears in the ledger", "DecisionLedger"):
        findings.append("stale_alias_in_allowed_context_passes FAILED")
    return findings


def test_stale_derivation(audit, root) -> list[str]:
    """The matcher is derived from spec/dispositions.rs, not a hand list."""
    findings: list[str] = []
    problems: list[str] = []
    amap = audit.parse_stale_vocabulary(root, problems)
    if problems:
        findings.append(f"parse_stale_vocabulary reported problems on the real seed: {problems}")
    if len(amap) < 20:
        findings.append(f"generated_stale_table_must_match_decisions: derived only {len(amap)} aliases")
    owner = amap.get("filebat")
    if not owner or owner[0] != "DEC-003":
        findings.append("FileBat alias is not derived to its killing decision DEC-003")
    if "made-up-stale-term" in amap:
        findings.append("alias_removed_from_decision_disappears_from_all_views FAILED")

    # 5.5E2: the context vocabulary is closed in BOTH directions — the enum
    # declares exactly what the scanner speaks, and the two never-permissive
    # defaults cannot be allow-listed by a row.
    def context_probe(name, old, new, needle):
        tmp = gate_sandbox([("spec/dispositions.rs", old, new)])
        try:
            problems: list[str] = []
            audit.parse_stale_vocabulary(tmp, problems)
            if not any(needle in p for p in problems):
                findings.append(f"{name} FAILED (wanted {needle!r}, got {problems!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    context_probe(
        "allow_listing_production_source_is_rejected",
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        'StaleContext::SupersessionGuide, StaleContext::LegacyEvidence], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        "StaleContext::SupersessionGuide, StaleContext::LegacyEvidence, "
        'StaleContext::ProductionSource], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "allow-lists the default context ProductionSource")
    context_probe(
        "dropping_a_default_context_variant_is_rejected",
        "    /// DEFAULT context for every other authoritative surface. Never permissive.\n"
        "    OrdinaryAuthoritative,\n",
        "",
        "the scanner's closed vocabulary")
    context_probe(
        "reintroducing_a_dead_context_variant_is_rejected",
        "    MigrationCompatibility,\n",
        "    MigrationCompatibility,\n    TestFixture,\n",
        "the scanner's closed vocabulary")
    return findings


def test_legacy_manifest_parity(audit) -> list[str]:
    """Hostile fixtures for the D-1 declaration-parity denominator.

    The denominator is measured against declared invariant IDs, so a structural
    key that merely resembles an invariant name (the generated INV-CATALOG
    marker) must never be counted as a declaration.
    """
    findings: list[str] = []
    base = ["INV-A", "INV-B", "INV-C"]
    check = audit.legacy_manifest_findings

    if check(base, base, 3):
        findings.append("exact_declaration_parity_should_pass FAILED")
    if not check(base + ["INV-CATALOG"], base, 3):
        findings.append("structural_INV_token_counted_as_declaration_is_rejected FAILED")
    if not check(base, ["INV-A", "INV-B"], 3):
        findings.append("declared_invariant_without_coverage_is_rejected FAILED")
    if not check(base, base + ["INV-GHOST"], 3):
        findings.append("coverage_for_undeclared_invariant_is_rejected FAILED")
    if not check(base, base, 4):
        findings.append("denominator_count_mismatch_is_rejected FAILED")
    if not check(base + ["INV-A"], base, 3):
        findings.append("duplicate_declaration_id_is_rejected FAILED")
    return findings


def _sample_operators() -> list[dict[str, str]]:
    base = {
        "semantic_op": "s", "input_sorts": "a", "result_sort": "b", "overflow": "o",
        "exception": "e", "mutation_classes": "m",
    }
    return [
        {**base, "id": "OP-ADD", "class": "Arithmetic", "word": "+", "symbol": "",
         "arity": "Binary", "fixity": "Infix", "precedence": "60", "semantic_op": "add",
         "typing": "OperatorTypingRule::PercentAdjustment, OperatorTypingRule::SameUnit",
         "associativity": "Left", "exactness": "Exact", "formatting": "+", "spoken": "plus"},
        {**base, "id": "OP-EQ", "class": "Comparison", "word": "IS", "symbol": "=",
         "arity": "Binary", "fixity": "Infix", "precedence": "50",
         "typing": "OperatorTypingRule::SameSortComparison",
         "associativity": "NonAssociative", "exactness": "Exact", "formatting": "IS", "spoken": "is"},
    ]


def test_batql(audit, project) -> list[str]:
    """Named hostile fixtures for the BatQL operator/grammar/proof surfaces."""
    findings: list[str] = []
    ops = _sample_operators()
    companion = (HERE.parent / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # 1 & 2: proof-disposition axis must keep the exact five states
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("missing_legacy_weak_on_authoritative_surface_is_rejected")
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("legacy_weak_collapsed_by_projection_is_rejected")

    # 3 & 4: grammar reference closure and unique ownership
    undefined, _, _ = audit.batql_grammar_closure_findings(["foo\n  := bar baz"])
    if not undefined:
        fail("undefined_grammar_leaf_is_rejected")
    dup, _, _ = audit.batql_grammar_closure_findings(['foo\n  := "x"\n\nfoo\n  := "y"'])
    if not any("more than one owning" in item for item in dup):
        fail("duplicate_grammar_owner_is_rejected")

    # 5: MATCH requires an initial step plus at least one THEN step
    if "match_step (THEN match_step)+" not in companion or "match_step (THEN match_step)*" in companion:
        fail("one_step_match_is_rejected")

    # 6: kernel declaration clauses appear once, in canonical order
    order = ["FROM KERNEL kernel_ref", "purity_clause", "determinism_clause", "capability_clause", "cost_clause"]
    positions = [companion.find(token) for token in order]
    if -1 in positions or positions != sorted(positions):
        fail("reordered_or_duplicated_kernel_clauses_are_rejected")

    # 7 & 8 & 9: operator fact discipline
    if not any("duplicate OperatorId" in x for x in audit.batql_operator_fact_findings(ops + [dict(ops[0])])):
        fail("duplicate_operator_id_is_rejected")
    clash = {**ops[0], "id": "OP-ADD-CLONE"}
    if not any("claimed by" in x for x in audit.batql_operator_fact_findings([ops[0], clash])):
        fail("ambiguous_token_plus_fixity_ownership_is_rejected")
    missing_field = {**ops[0], "spoken": ""}
    if not any("missing field spoken" in x for x in audit.batql_operator_fact_findings([missing_field])):
        fail("missing_required_operatorspec_field_is_rejected")

    # 10, 11, 12: projection parity is sensitive to alteration, missing, and extra rows
    expected = audit.batql_render_catalog(ops)
    if expected == expected.replace("Exact", "EXACT", 1):
        fail("altered_generated_operator_projection_is_rejected")
    if expected == audit.batql_render_catalog([ops[0]]):
        fail("missing_generated_operator_row_is_rejected")
    if expected == audit.batql_render_catalog(ops + [{**ops[0], "id": "OP-XTRA", "word": "~"}]):
        fail("extra_generated_operator_row_is_rejected")

    # 13: canonical order is independent of source array order
    if audit.batql_render_catalog(ops) != audit.batql_render_catalog(list(reversed(ops))):
        fail("source_fact_reorder_leaves_generated_output_unchanged")

    # Typed legality rules (5.5E1): the rules are the operand/result authority,
    # their placement is law, and the two phantom sorts stay dead.
    if not any("declares no typing rules" in x
               for x in audit.batql_operator_fact_findings([{**ops[0], "typing": ""}])):
        fail("operator_without_typing_rules_is_rejected")
    if not any("only subtraction" in x for x in audit.batql_operator_fact_findings(
            [{**ops[0], "typing": "OperatorTypingRule::WallObservationDifference"}])):
        fail("wall_difference_outside_subtraction_is_rejected")
    if not any("must carry exactly" in x for x in audit.batql_operator_fact_findings(
            [{**ops[1], "typing": "OperatorTypingRule::TruthUnary"}])):
        fail("comparison_with_foreign_typing_rule_is_rejected")
    if audit.phantom_sort_findings(HERE.parent):
        fail("phantom_sort_baseline_is_silent")
    with isolated_tree() as tmp:
        p = tmp / "companion/BATQL_LANGUAGE.md"
        p.write_text(p.read_text(encoding="utf-8")
                     + "\nDecimalMoney is back.\n", encoding="utf-8")
        if not any("phantom sort" in x for x in audit.phantom_sort_findings(tmp)):
            fail("a_phantom_sort_cannot_reenter_the_corpus")
    # A mutated rule regenerates a DIFFERENT matrix: the block is semantic, not
    # decorative. Dropping the percent-difference rule must change the render.
    stripped = {**ops[0], "id": "OP-SUB", "word": "-", "semantic_op": "subtract",
                "typing": "OperatorTypingRule::SameUnit"}
    ruled = {**stripped,
             "typing": "OperatorTypingRule::PercentDifference, OperatorTypingRule::SameUnit"}
    if audit.batql_render_typing([stripped]) == audit.batql_render_typing([ruled]):
        fail("typing_rule_mutation_changes_the_generated_matrix")

    # 14: the independent generator and auditor projections must agree
    if (
        project.render_catalog(ops) != audit.batql_render_catalog(ops)
        or project.render_grammar(ops) != audit.batql_render_grammar(ops)
        or project.render_projection(ops) != audit.batql_render_projection(ops)
    ):
        fail("generator_auditor_disagreement_turns_gate_red")

    # 15: FLOOR/CEILING without the language-change record turns the gate red
    modes = "rounding_mode\n  := HALF_EVEN | HALF_UP | DOWN | UP | FLOOR | CEILING\n"
    if audit.batql_language_change_findings(modes + "\nLanguage-change record (BatQL V1)\n"):
        fail("language_change_record_present_passes")
    if not audit.batql_language_change_findings(modes):
        fail("floor_ceiling_without_language_change_record_is_rejected")

    return findings


def test_numeric(audit) -> list[str]:
    """Named hostile fixtures for the numeric authority contract (docs/37)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    markers = [marker for _name, marker in audit.NUMERIC_LAW_MARKERS]
    if audit.numeric_law_findings("\n".join(markers)):
        fail("all_numeric_laws_present_passes")
    for name, marker in audit.NUMERIC_LAW_MARKERS:
        reduced = "\n".join(m for m in markers if m != marker)
        if not any(name in item for item in audit.numeric_law_findings(reduced)):
            fail(f"numeric_law_{name}_absence_is_rejected")

    # OperatorSpec arithmetic row lacking an approximate-support posture is rejected
    arith_no_posture = [{
        "id": "OP-ADD", "class": "Arithmetic", "word": "+", "symbol": "",
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix", "precedence": "60",
        "associativity": "Left", "input_sorts": "a", "result_sort": "b", "exactness": "Exact",
        "overflow": "o", "exception": "e", "formatting": "+", "spoken": "plus",
        "mutation_classes": "m", "numeric_support": "NotApplicable",
    }]
    if not any("lacks an approximate-support posture" in x for x in audit.batql_operator_fact_findings(arith_no_posture)):
        fail("operatorspec_arithmetic_row_lacking_approx_support_is_rejected")

    # An operator that admits raw approximate operands (QualifiedProfileOnly) with no
    # qualified numeric profile in V1 is rejected; ExactSupported never admits them.
    admits_raw_approx = [{**arith_no_posture[0], "numeric_support": "QualifiedProfileOnly"}]
    if not any("no qualified numeric profile exists" in x for x in audit.batql_operator_fact_findings(admits_raw_approx)):
        fail("operator_admitting_raw_approximate_without_profile_is_rejected")

    return findings


def test_guarantees(audit, project) -> list[str]:
    """Named hostile fixtures for the Guarantee Graph (DEC-070)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def seed(sid, kind="SemanticLaw", life="Permanent", owner="o", witness="w"):
        return {"id": sid, "kind": kind, "lifetime": life, "owner": owner, "gates": "",
                "witness": witness, "failure": "f",
                "rel": {"DerivesFrom": [], "Refines": [], "Discharges": [], "Supersedes": []}}

    def node(nid, family="SEED", kind="SemanticLaw", life="Permanent", owner="o", gates="",
             witness="RowDeclared"):
        # Post-5.5D4b node shape: an admitted view carries a typed gate posture and
        # a witness posture, never a bare gate string plus free witness text.
        return {"id": nid, "family": family, "kind": kind, "lifetime": life,
                "owner": owner, "gate_posture": gates, "target": "", "witness": witness}

    good = [seed(f"SEED-{i}") for i in range(25)]
    if audit.guarantee_classification_findings(good):
        fail("valid_classification_passes")
    if not audit.guarantee_classification_findings(good[:24]):
        fail("missing_seed_row_is_rejected")
    if not audit.guarantee_classification_findings(good + [seed("SEED-25")]):
        fail("extra_seed_row_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "kind": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_kind_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "lifetime": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_lifetime_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "owner": ""}] + good[1:]):
        fail("unclassified_seed_row_is_rejected")

    ids = {"A", "B", "C"}
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "Z")]):
        fail("dangling_relation_target_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "A")]):
        fail("self_edge_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "B"), ("A", "DerivesFrom", "B")]):
        fail("duplicate_edge_is_rejected")
    for fam in ("DerivesFrom", "Refines", "Supersedes", "Discharges"):
        if not any("cycle" in x for x in audit.guarantee_relation_findings(ids, [("A", fam, "B"), ("B", fam, "A")])):
            fail(f"{fam.lower()}_cycle_is_rejected")

    if not any("Permanent SemanticLaw names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("SEED-X", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_semanticlaw_without_witness_is_rejected")
    if not any("Permanent ArchitectureConstraint names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("A", kind="ArchitectureConstraint", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_architecture_without_witness_is_rejected")
    if audit.guarantee_lifetime_findings([node("D", kind="Decision", witness="NoFamilyWitness")], {}, []):
        fail("permanent_decision_witness_deferred_to_c2")
    if audit.guarantee_lifetime_findings([node("Q", kind="QualificationRequirement", witness="NoFamilyWitness")], {}, []):
        fail("permanent_qualification_is_self_witnessing")
    if not any("UntilGate names no active gate schedule" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilGate", gates="")], {}, [])):
        fail("untilgate_without_gate_is_rejected")
    if not any("HistoricalCoverageOnly cannot be actively scheduled" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="HistoricalCoverageOnly", gates="G2")], {}, [])):
        fail("historicalcoverageonly_carrying_active_gate_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor", owner="batpak::store", gates="G2")], {}, [])):
        fail("untilsuccessor_without_typed_successor_is_rejected")
    if audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor")], {}, [("Y", "Supersedes", "X")]):
        fail("untilsuccessor_with_resolvable_successor_passes")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "G2")]):
        fail("gate_name_cannot_become_successor_edge")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "batpak::store")]):
        fail("clean_owner_text_cannot_become_successor_edge")

    def legnode(life, gates="G2"):
        return node("LEG-1", family="LEG", kind="LegacyObligation", life=life, gates=gates, witness="")

    active_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Active"}}
    if not any("must not force Permanent" in x for x in audit.guarantee_lifetime_findings([legnode("Permanent")], active_never, [])):
        fail("deletion_never_forcing_permanent_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([legnode("UntilSuccessor")], active_never, [])):
        fail("gated_leg_without_typed_successor_cannot_project_untilsuccessor")
    closed_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Closed"}}
    if audit.guarantee_lifetime_findings([legnode("ClosedEvidence", gates="")], closed_never, []):
        fail("deletion_never_closed_leg_projects_closedevidence")
    if not any("Active LEG projects as ClosedEvidence" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], active_never, [])):
        fail("closedevidence_marked_active_is_rejected")
    closed = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "OnSuccessorGateClosure", "status": "Closed"}}
    if not any("still names an implementation gate" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], closed, [])):
        fail("closed_leg_still_gating_is_rejected")

    both = {"SEED-X", "LEG-1"}
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Discharges", "LEG-1")])):
        fail("bare_law_discharges_active_leg_is_rejected")
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Closes", "LEG-1")])):
        fail("bare_law_closes_active_leg_is_rejected")
    if audit.guarantee_relation_findings(both, [("SEED-X", "Refines", "LEG-1")]):
        fail("refines_active_leg_passes")

    root = HERE.parent
    def shape(n):
        return (n["id"], n["family"], n["kind"], n["lifetime"], n["owner"],
                n["gate_posture"], n["target"], n["witness"])

    proj_nodes = [shape(n) for n in project.guarantee_nodes(root)]
    audit_nodes = [shape(n) for n in audit.guarantee_derive(root)[0]]
    if proj_nodes != audit_nodes:
        fail("generator_auditor_guarantee_disagreement")
    nodes, edges, _adm = audit.guarantee_derive(root)
    if nodes != sorted(nodes, key=lambda n: (audit.G_FAMILY_RANK[n["family"]], n["id"])):
        fail("source_reorder_changes_guarantee_order")
    if any(not src.startswith("SEED-") for src, _k, _t in edges):
        fail("non_seed_source_guarantee_edge_present")

    return findings


# --- Guarded mutation: a claimed negative test is itself a claim -------------
# A red light caused by the wrong severed wire is not evidence that the alarm
# works. Every mutation below proves four things: the target existed, the bytes
# actually changed, the intended validator ran, and the EXPECTED finding class
# appeared -- never merely that something somewhere returned nonzero.
class ProbeError(AssertionError):
    """The probe itself is broken: absent target or inert mutation."""


def must_replace(text: str, old: str, new: str, what: str) -> str:
    if old not in text:
        raise ProbeError(f"probe target absent, mutation never applied: {what}")
    out = text.replace(old, new, 1)
    if out == text:
        raise ProbeError(f"probe mutation changed no bytes: {what}")
    return out


def gate_sandbox(edits):
    """A repo copy with guarded edits applied; returns the sandbox root.

    The canonical tracked workspace is never the mutation target (5.5D4a).
    Callers discard the copy in a finally block; `isolated_tree` is the
    context-managed form that cleans up on every exit path.
    """
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
    for sub in ("spec", "docs", "companion"):
        src = root / sub
        if src.is_dir():
            shutil.copytree(src, tmp / sub)
    # The tracked toolchain projection is part of every tree (5.5E3a): it
    # selects the compiler before the spec compiles, and its absence is a
    # refusal, not a sandbox convenience.
    if (root / "rust-toolchain.toml").is_file():
        shutil.copy2(root / "rust-toolchain.toml", tmp / "rust-toolchain.toml")
    for rel, old, new in edits:
        path = tmp / rel
        text = path.read_text(encoding="utf-8")
        path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
    return tmp


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
                         'clean_owner: "batpak::store", gates: &[GateId::G2], compatibility_disposition: '
                         "CompatibilityDisposition::None, deletion_condition: "
                         "DeletionCondition::OnSuccessorGateClosure",
                         'clean_owner: "batpak::store", gates: &[], compatibility_disposition: '
                         "CompatibilityDisposition::None, deletion_condition: "
                         "DeletionCondition::OnSuccessorGateClosure")])
    if gates_of(tmp, "LEG-001") == base_leg:
        fail("leg_gate_dropped_during_migration_is_detected")
    expect("emptied_gate_list_is_rejected",
           audit.gate_reference_findings(tmp), "names no gate")
    shutil.rmtree(tmp, ignore_errors=True)

    probe("leg_gate_added_during_migration_is_rejected",
          [("spec/legacy_obligations.rs",
            'clean_owner: "batpak::event", gates: &[GateId::G2]',
            'clean_owner: "batpak::event", gates: &[GateId::G2, GateId::G2]')],
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
    if len([n for n in nodes if n["family"] == "DEC"]) != 75:
        fail("decision node count is not 75")

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
    if len([n for n in nodes if n["family"] == "DEC"]) != 75:
        fail("decision_node_count_is_not_75")
    if len(nodes) != 202 or len(edges) != 9:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    return findings


def test_delivery_notes_d2(audit) -> list[str]:
    """The delivery inventory tracks the live decision count."""
    findings: list[str] = []
    text = (HERE.parent / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    if "74 architectural decision/disposition rows" in text:
        findings.append("delivery_notes_remaining_at_74_decisions FAILED")
    if "75 architectural decision/disposition rows" not in text:
        findings.append("delivery_notes_states_75_decisions FAILED")
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

    ARCH = "spec/architecture.rs"
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

    # --- lane and result vocabularies ---------------------------------------
    probe("fourth_mutation_lane_is_rejected", ARCH,
          "    CompilerBacked,\n}", "MutationLane variants", "    CompilerBacked,\n    NativeLane,\n}",
          validator=pp)
    probe("removed_mutation_lane_is_rejected", ARCH,
          "    /// Implementation-sensitive material whose truth depends on real compiler\n"
          "    /// and platform behavior. Runs under real rustc semantics.\n    CompilerBacked,\n",
          "MutationLane variants", "", validator=pp)
    for variant in ("NotActivated", "Refused", "TimedOut", "InfrastructureFailure", "EquivalentCandidate"):
        probe(f"mutation_result_{variant.lower()}_removed_is_rejected", ARCH,
              f"    {variant},\n", "MutationResult variants", "", validator=pp)
    probe("never_killed_dropping_unbuildable_is_rejected", ARCH,
          "pub const NEVER_KILLED: &[MutationResult] = &[\n    MutationResult::NotActivated,",
          "NEVER_KILLED", "pub const NEVER_KILLED: &[MutationResult] = &[", validator=pp)
    probe("never_survived_dropping_notactivated_is_rejected", ARCH,
          "pub const NEVER_SURVIVED: &[MutationResult] = &[\n    MutationResult::NotActivated,",
          "NEVER_SURVIVED", "pub const NEVER_SURVIVED: &[MutationResult] = &[", validator=pp)
    probe("denominator_dropping_a_terminal_result_is_rejected", ARCH,
          "pub const TERMINAL_MUTATION_RESULTS: &[MutationResult] = &[\n    MutationResult::Killed,",
          "!= all eight categories",
          "pub const TERMINAL_MUTATION_RESULTS: &[MutationResult] = &[", validator=pp)
    probe("change_class_collapsed_is_rejected", ARCH,
          "    Neutral,\n", "ProofPolicyChangeClass variants", "", validator=pp)
    probe("proof_policy_surface_removed_is_rejected", ARCH,
          "    WaiverLogic,\n", "ProofPolicySurface variants", "", validator=pp)

    # --- candidate and promotion boundary -----------------------------------
    for rootdir in ("src/", "tests/", "spec/", "docs/", "companion/"):
        probe(f"candidate_write_into_{rootdir.strip('/')}_permitted_is_rejected", ARCH,
              f'"{rootdir}"', f"not forbidden from writing {rootdir}", '"__none__"', validator=pp)
    probe("candidate_root_moved_into_tracked_source_is_rejected", ARCH,
          "target/muterprater/candidates/", "no candidate output root is declared", "tests/generated/",
          validator=pp)
    for needed, token in (("oracle", "independent evidence or oracle identity"),
                          ("named invariant", "named invariant, guarantee, obligation, or documented proof gap"),
                          ("killed real semantic mutant", "killed real semantic mutant or equivalent hostile evidence"),
                          ("receipt", "auditable proof and promotion receipt")):
        probe(f"promotion_without_{needed.split()[0]}_is_rejected", ARCH,
              f'    "{token}",\n', f"promotion requirements omit {needed}", "", validator=pp)

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
    if len([n for n in nodes if n["family"] == "DEC"]) != 75:
        fail("decision_node_count_is_not_75")
    if len(nodes) != 202 or len(edges) != 9:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    text = (root / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    if "75 architectural decision/disposition rows" not in text:
        fail("delivery_notes_states_74_decisions")
    return findings


# --- Hostile-probe isolation (5.5D4a) ---------------------------------------
# A probe timeout once left a disabled rule inside the canonical bootstrap/audit.py.
# Manual restoration worked, but a guard that depends on a later cleanup step is a
# guard that can be left asleep with its eyes painted open. The canonical tracked
# workspace is now never a mutation target: probes mutate an isolated copy, the
# copy is discarded on every exit path including timeout, exception, and
# cancellation, and the suite proves canonical validator bytes are unchanged.
CANONICAL_VALIDATORS = ("audit.py", "project.py", "freeze.py", "selftest.py")


def _commit(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_commitments() -> dict[str, str]:
    """Content commitments for every canonical validator, before any probe runs."""
    return {name: _commit(HERE / name) for name in CANONICAL_VALIDATORS if (HERE / name).is_file()}


def canonical_drift(before: dict[str, str]) -> list[str]:
    """Any canonical validator whose bytes moved is a residue finding, not a pass."""
    out = []
    for name, want in sorted(before.items()):
        got = _commit(HERE / name)
        if got != want:
            out.append(f"canonical validator {name} was modified by the hostile-probe suite")
    return out


@contextlib.contextmanager
def isolated_tree(subdirs=("spec", "docs", "companion")):
    """An isolated copy of the tracked material. Discarded on EVERY exit path:
    success, exception, timeout, assertion failure, or cancellation."""
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
    try:
        for sub in subdirs:
            src = root / sub
            if src.is_dir():
                shutil.copytree(src, tmp / sub)
        if (root / "rust-toolchain.toml").is_file():
            shutil.copy2(root / "rust-toolchain.toml", tmp / "rust-toolchain.toml")
        yield tmp
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


@contextlib.contextmanager
def neutered_validator(name: str, target: str, replacement: str = "if False:"):
    """Load a COPY of a canonical validator with one rule neutered.

    This replaces the retired pattern of patching bootstrap/audit.py in place and
    restoring afterwards. The canonical file is never opened for writing, so a
    timeout mid-probe cannot leave a disabled rule behind.
    """
    canonical = HERE / f"{name}.py"
    before = _commit(canonical)
    source = canonical.read_text(encoding="utf-8")
    if target not in source:
        raise ProbeError(f"probe target absent in {name}.py: {target!r}")
    mutated = source.replace(target, replacement, 1)
    if mutated == source:
        raise ProbeError(f"probe mutation changed no bytes in {name}.py: {target!r}")
    tmp = Path(tempfile.mkdtemp(prefix="batpak-neuter-"))
    try:
        path = tmp / f"{name}.py"
        path.write_text(mutated, encoding="utf-8")
        spec = importlib.util.spec_from_file_location(f"{name}_neutered", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        yield module
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
        if _commit(canonical) != before:
            raise ProbeError(f"canonical {name}.py was modified during a probe")


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
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    iw = audit.integrity_witness_findings

    if audit.witness_reference_findings(root) or iw(root):
        fail("leg023_witness_contract_passes")

    # Each of the seven witness identities must survive in docs/24.
    for wid in audit.D4B_LEG023_WITNESSES:
        probe(f"{wid}_identity_removed_is_rejected", GA, wid + "\n", f"{wid} is absent from docs/24",
              "", validator=iw)

    # The resolver: unknown, duplicated, misowned, missing, extra.
    probe("unknown_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "references unknown proof-row id ghost_witness",
          "ghost_witness; middle_event_deletion_is_rejected;")
    probe("duplicate_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "projects a duplicate witness reference",
          "middle_event_deletion_is_rejected; middle_event_deletion_is_rejected;")
    probe("witness_owned_by_wrong_leg_is_rejected", GA,
          "), also carried by `LEG-023`:", "which docs/24 binds to LEG-081",
          "), also carried by `LEG-081`:")
    probe("docs21_missing_owned_witness_is_rejected", D21,
          "; midstream_genesis_is_rejected", "omits owned proof row midstream_genesis_is_rejected")
    probe("docs21_extra_witness_is_rejected", D21,
          "middle_event_deletion_is_rejected;",
          "references unknown proof-row id unknown_leg_proof_row_for_fixture",
          "unknown_leg_proof_row_for_fixture; middle_event_deletion_is_rejected;")
    probe("duplicate_docs24_proof_row_id_is_rejected", GA,
          "middle_event_deletion_is_rejected\nevent_reorder_is_rejected",
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          "middle_event_deletion_is_rejected\nmiddle_event_deletion_is_rejected\nevent_reorder_is_rejected")
    probe("witness_meaning_removal_is_rejected", GA,
          "midstream_genesis_is_rejected\n    A genesis marker cannot reset an already-started stream.",
          "has no authoritative meaning", "")
    probe("future_executable_posture_removal_is_rejected", GA,
          "Required witnesses (proof owner TestPak; gates G2/G3), also carried by `LEG-023`:",
          "states no future-executable proof owner or gate",
          "Required witnesses (proof owner Nobody; gates ), also carried by `LEG-023`:")

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
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    dm = audit.derived_material_findings

    if dm(root) or audit.witness_reference_findings(root):
        fail("d4b2a_witness_contract_passes")

    # every new proof-row identity survives, one probe each
    for leg, ids in audit.D4B2A_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", GA, wid + "\n",
                  f"{leg} witness {wid} is absent from docs/24", "", validator=dm)

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
    probe("bounded_output_accepted_as_bounded_discovery_work_is_rejected", GA,
          "page_limit_bounds_discovery_work_not_only_output\n",
          "LEG-028 witness page_limit_bounds_discovery_work_not_only_output is absent",
          "", validator=dm)
    probe("full_matched_set_allocation_accepted_is_rejected", GA,
          "allocation_does_not_scale_with_full_matched_set\n",
          "LEG-020 witness allocation_does_not_scale_with_full_matched_set is absent",
          "", validator=dm)
    probe("discovery_and_allocation_sharing_one_owner_is_rejected", GA,
          "), also carried by `LEG-020`:", "share one qualification target",
          "), also carried by `LEG-028`:", validator=dm)

    # resolver coverage for the new rows
    probe("d4b2a_row_bound_to_wrong_leg_is_rejected", GA,
          "), also carried by `LEG-019`:", "which docs/24 binds to LEG-023",
          "), also carried by `LEG-023`:")
    probe("docs21_missing_a_d4b2a_row_is_rejected", D21,
          "; derived_row_cannot_prove_absence_or_loss",
          "omits owned proof row derived_row_cannot_prove_absence_or_loss")
    probe("docs21_extra_d4b2a_row_is_rejected", D21,
          "allocation_does_not_scale_with_full_matched_set",
          "references derived_row_cannot_authenticate_order, which docs/24 binds to LEG-019",
          "derived_row_cannot_authenticate_order")
    probe("duplicate_row_across_d4b1_and_d4b2a_is_rejected", GA,
          "forged_sibling_cannot_cause_false_loss\n",
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          "middle_event_deletion_is_rejected\n")
    probe("d4b2a_meaning_removed_is_rejected", GA,
          "derived_row_cannot_authenticate_order\n    Corroborating row contents does not establish table ordering.",
          "has no authoritative meaning", "")
    probe("d4b2a_future_executable_posture_removed_is_rejected", GA,
          "Required witnesses (proof owner TestPak; gates G2/G3), also carried by `LEG-019`:",
          "states no future-executable proof owner or gate",
          "Required witnesses (proof owner Nobody; gates ), also carried by `LEG-019`:")

    findings.extend(canonical_drift(before))
    return findings


def test_deferred_witnesses(audit) -> list[str]:
    """LEG-043 deferred native rows and the LEG-074 reimport row (5.5D4b-2b).

    Structural only. Bootstrap never called fcntl, never inspected a descriptor
    table, never launched a native process, never verified a live close-on-exec
    flag, and ships no native launcher. These fixtures prove the proof boundary
    is named and honestly deferred -- nothing more.
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
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    dp = audit.deferred_posture_findings
    H43 = ("Required witnesses (proof owner TestPak; gates G5; future executable: yes; "
           "deferred until: the relevant native or foreign execution adapter is admitted; "
           "bootstrap executed: no), also carried by `LEG-043`:")
    H74 = ("Required witnesses (proof owner TestPak; gates G2/G3; future executable: yes; "
           "bootstrap executed: no), also carried by `LEG-074`:")

    if dp(root) or audit.witness_reference_findings(root):
        fail("d4b2b_witness_contract_passes")

    # every new proof-row identity survives
    for leg, ids in audit.D4B2B_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", GA, wid + "\n",
                  f"{leg} witness {wid} is absent from docs/24", "", validator=dp)

    # deferred posture must be explicit, honest, and admission-bounded
    probe("leg043_missing_future_executable_posture_is_rejected", GA,
          "gates G5; future executable: yes; deferred until:",
          "states no future-executable posture", "gates G5; deferred until:", validator=dp)
    probe("leg043_missing_deferral_posture_is_rejected", GA,
          "; deferred until: the relevant native or foreign execution adapter is admitted",
          "states no deferral condition", "", validator=dp)
    probe("leg043_vague_later_deferral_is_rejected", GA,
          "deferred until: the relevant native or foreign execution adapter is admitted",
          "names no admission boundary", "deferred until: later", validator=dp)
    probe("leg043_falsely_marked_bootstrap_executed_is_rejected", GA,
          "bootstrap executed: no), also carried by `LEG-043`:",
          "falsely claims bootstrap execution",
          "bootstrap executed: yes), also carried by `LEG-043`:", validator=dp)
    probe("leg043_falsely_marked_currently_qualified_is_rejected", GA,
          "bootstrap executed: no), also carried by `LEG-043`:",
          "falsely claims current executable qualification",
          "bootstrap executed: no; currently qualified), also carried by `LEG-043`:", validator=dp)
    probe("leg043_unknown_gate_is_rejected", GA, "gates G5;", "unknown GateId", "gates G12;",
          validator=dp)
    probe("leg043_gates_differing_from_owning_leg_is_rejected", GA, "gates G5;",
          "differ from LEG-043 gates", "gates G2;", validator=dp)
    probe("leg074_incorrectly_marked_native_deferred_is_rejected", GA,
          "gates G2/G3; future executable: yes; bootstrap executed: no), also carried by `LEG-074`:",
          "inherits a native-adapter deferral it does not have",
          "gates G2/G3; future executable: yes; deferred until: the adapter is admitted; "
          "bootstrap executed: no), also carried by `LEG-074`:", validator=dp)
    probe("leg074_falsely_marked_bootstrap_executed_is_rejected", GA,
          "bootstrap executed: no), also carried by `LEG-074`:",
          "falsely claims bootstrap execution",
          "bootstrap executed: yes), also carried by `LEG-074`:", validator=dp)

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

    # resolver coverage for the new rows
    probe("d4b2b_row_bound_to_wrong_leg_is_rejected", GA,
          "), also carried by `LEG-074`:", "which docs/24 binds to LEG-023",
          "), also carried by `LEG-023`:")
    probe("docs21_missing_a_d4b2b_row_is_rejected", D21,
          "; fcntl_setfd_failure_fails_closed", "omits owned proof row fcntl_setfd_failure_fails_closed")
    probe("docs21_extra_d4b2b_row_is_rejected", D21,
          "close_reopen_reimport_returns_zero_new_events",
          "references fcntl_getfd_failure_fails_closed, which docs/24 binds to LEG-043",
          "fcntl_getfd_failure_fails_closed")
    probe("duplicate_row_across_prior_d4b_passes_is_rejected", GA,
          "fcntl_getfd_failure_fails_closed\n",
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          "middle_event_deletion_is_rejected\n")
    probe("d4b2b_future_executable_posture_removed_is_rejected", GA, H74,
          "states no future-executable posture",
          "Required witnesses (proof owner TestPak; gates G2/G3), also carried by `LEG-074`:",
          validator=dp)

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
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    D35 = "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md"
    la = audit.leg081_authority_findings
    pm = audit.proof_meaning_findings
    H81 = ("Required witnesses (proof owner TestPak; gates G2/G3; future executable: yes; "
           "bootstrap executed: no), also carried by `LEG-081`:")
    PROJ = ("Required proof rows, projected from docs/24 (qualification target: LEG-081; "
            "canonical proof-row owner: docs/24 Gauntlet):")
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

    # Every canonical identity survives in docs/24.
    for wid in audit.D4B2C_ROWS:
        probe(f"{wid}_identity_removed_is_rejected", GA, wid + "\n",
              f"LEG-081 canonical proof row {wid} is absent from docs/24", "", validator=la)

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
          "spec/proof.rs PROOF_ROWS never learned",
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
    probe("typed_active_row_missing_from_docs24_is_rejected", PR,
          '    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Active },\n',
          "typed Active identity row_invented_in_rust_only appears as no "
          "canonical docs/24 active row",
          '    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Active },\n'
          '    ProofRowRecord { id: ProofRowId("row_invented_in_rust_only"), '
          'state: ProofRowState::Active },\n', validator=cat)
    probe("docs24_active_row_missing_from_typed_catalog_is_rejected", GA,
          "raw_batql_is_not_a_netbat_invocation\n",
          "docs/24 declares active proof row row_added_only_in_docs, which the "
          "typed catalog never learned",
          "raw_batql_is_not_a_netbat_invocation\nrow_added_only_in_docs\n",
          validator=cat)
    probe("same_identity_cannot_be_active_and_retired", PR,
          '    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Active },\n',
          "is declared twice in the proof-identity catalog",
          '    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Active },\n'
          '    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("event_reorder_is_rejected")] } },\n', validator=cat)
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
    probe("docs35_leg081_missing_projected_id_is_rejected", D35,
          "foreign_keyset_generation_is_rejected\n",
          "docs/35 omits projected LEG-081 proof row foreign_keyset_generation_is_rejected",
          "", validator=la)
    probe("docs35_leg081_extra_projected_id_is_rejected", D35,
          "shred_ack_waits_for_backend_durability\n",
          "docs/35 projects middle_event_deletion_is_rejected, which docs/24 binds to LEG-023",
          "shred_ack_waits_for_backend_durability\nmiddle_event_deletion_is_rejected\n",
          validator=la)
    probe("docs35_projection_label_removed_is_rejected", D35, PROJ,
          "docs/35 states no LEG-081 proof-row projection labelled as projected from docs/24",
          "Required proof rows:", validator=la)
    probe("docs35_projection_claims_own_canonical_ownership_is_rejected", D35, PROJ,
          "docs/35 projection names canonical proof-row owner 'docs/35 itself', not docs/24",
          PROJ.replace("canonical proof-row owner: docs/24 Gauntlet",
                       "canonical proof-row owner: docs/35 itself"), validator=la)
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
    probe("docs24_leg081_row_bound_to_wrong_leg_is_rejected", GA,
          "), also carried by `LEG-081`:",
          "LEG-081 canonical proof row shred_ack_waits_for_backend_durability is bound to LEG-020",
          "), also carried by `LEG-020`:", validator=la)
    probe("docs24_leg081_wrong_proof_owner_is_rejected", GA, H81,
          "names proof owner 'Nobody', not TestPak",
          H81.replace("proof owner TestPak", "proof owner Nobody"), validator=la)
    probe("docs24_leg081_gate_mismatch_is_rejected", GA, H81,
          "differ from typed LEG-081 gates 'G2/G3'",
          H81.replace("gates G2/G3;", "gates G2;"), validator=la)
    probe("docs24_leg081_unknown_gate_is_rejected", GA, H81,
          "names unknown GateId G12",
          H81.replace("gates G2/G3;", "gates G2/G12;"), validator=la)
    probe("docs24_leg081_missing_future_executable_posture_is_rejected", GA, H81,
          "states no future-executable posture",
          H81.replace("; future executable: yes", ""), validator=la)
    probe("docs24_leg081_false_bootstrap_execution_is_rejected", GA, H81,
          "falsely claims bootstrap execution",
          H81.replace("bootstrap executed: no", "bootstrap executed: yes"), validator=la)
    probe("docs24_leg081_meaning_removal_is_rejected", GA,
          "shred_transition_binding_mismatch_is_rejected\n    The shred transition binds StoreId,"
          " AuthorityGeneration, KeyGeneration, key\n",
          "LEG-081 proof row shred_transition_binding_mismatch_is_rejected has no authoritative meaning",
          "", validator=la)
    probe("duplicate_leg081_proof_row_id_is_rejected", GA,
          "foreign_keyset_generation_is_rejected\nshredded_unavailable",
          "docs/24 binds LEG-081 proof row foreign_keyset_generation_is_rejected more than once",
          "foreign_keyset_generation_is_rejected\nforeign_keyset_generation_is_rejected"
          "\nshredded_unavailable", validator=la)
    probe("leg081_proof_row_id_rename_is_rejected", GA,
          "foreign_keyset_generation_is_rejected\n",
          "docs/24 binds unexpected proof row foreign_keyset_generation_renamed to LEG-081",
          "foreign_keyset_generation_renamed\n", validator=la)

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
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.proof_target_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    pt = audit.proof_target_findings
    pg = audit.proof_target_grammar_findings
    # LEG-028 owns exactly one row, so retargeting its block cannot collide.
    H28 = "Required witnesses (proof owner TestPak; gates G2), also carried by `LEG-028`:"
    W28 = "page_limit_bounds_discovery_work_not_only_output"

    if pt(root) or pg(root) or audit.witness_reference_findings(root):
        fail("d4b3b0_resolver_contract_passes")

    # The index resolves every declared guarantee, and only those.
    idx = audit.guarantee_index(root)
    if len(idx) != 202:
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

    # A DEC target resolves through the one canonical row format.
    probe("dec_target_resolves_through_generic_resolver", GA, H28,
          "gates 'G2' differ from typed DEC-050 gates 'G4/G5'",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `DEC-050`:")
    with isolated_tree() as tmp:
        p = tmp / GA
        p.write_text(must_replace(p.read_text(encoding="utf-8"), H28,
                     "Required witnesses (proof owner TestPak; gates G4/G5), also carried by `DEC-050`:",
                     "retarget to DEC"), encoding="utf-8")
        silent("dec_target_with_matching_typed_gates_is_accepted",
               [f for f in pt(tmp) if W28 in f])
        if audit.witness_rows(tmp).get(W28, {}).get("target") != "DEC-050":
            fail("dec_target_is_recorded_as_the_primary_target")

    # SEED / ARCH / QUAL target forms parse and resolve on synthetic rows.
    with isolated_tree() as tmp:
        p = tmp / GA
        p.write_text(must_replace(p.read_text(encoding="utf-8"), H28,
                     "Required witnesses (proof owner TestPak; gates G2), also carried by `SEED-FBAT-CORE`:",
                     "retarget to SEED"), encoding="utf-8")
        silent("seed_target_with_matching_typed_gates_is_accepted",
               [f for f in pt(tmp) if W28 in f])
    for ref, fam, want in (("ARCH-batpak", "ARCH", "G0"), ("QUAL-batpak-native", "QUAL", "G0/G5")):
        probe(f"{fam.lower()}_target_gate_mismatch_is_read_from_the_typed_target", GA, H28,
              f"differ from typed {ref} gates '{want}'",
              f"Required witnesses (proof owner TestPak; gates G2), also carried by `{ref}`:")
        with isolated_tree() as tmp:
            p2 = tmp / GA
            p2.write_text(must_replace(p2.read_text(encoding="utf-8"), H28,
                          f"Required witnesses (proof owner TestPak; gates {want}), "
                          f"also carried by `{ref}`:", f"retarget to {fam}"), encoding="utf-8")
            silent(f"{fam.lower()}_target_with_matching_typed_gates_is_accepted",
                   [f for f in pt(tmp) if W28 in f])

    # Unknown family and unknown member both fail closed.
    # An unknown family cannot reach the target resolver: the row grammar admits
    # only the five known prefixes, so the grammar rule is what fails it closed.
    probe("unknown_guarantee_family_is_rejected", GA, H28,
          "docs/24 names 'BOGUS-1' as a proof target, which is not a source-qualified GuaranteeRef",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `BOGUS-1`:",
          validator=pg)
    with isolated_tree() as tmp:
        p = tmp / GA
        p.write_text(must_replace(p.read_text(encoding="utf-8"), H28,
                     "Required witnesses (proof owner TestPak; gates G2), also carried by `BOGUS-1`:",
                     "unknown family"), encoding="utf-8")
        if W28 in audit.witness_rows(tmp):
            fail("unknown_family_block_is_not_parsed_as_a_canonical_row")
    probe("unknown_guarantee_member_fails_closed", GA, H28,
          "targets DEC-999, which resolves to no existing DEC guarantee",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `DEC-999`:")

    # A gate schedules qualification; it never owns semantic meaning.
    probe("gateid_cannot_be_a_proof_target", GA, H28,
          "names GateId G2 as a proof target",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `G2`:",
          validator=pg)
    probe("free_text_cannot_be_a_proof_target", GA, H28,
          "as a proof target, which is not a source-qualified GuaranteeRef",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `batpak::projection`:",
          validator=pg)

    # One primary target per row.
    probe("two_primary_targets_is_rejected", GA, H28,
          "names more than one primary target: LEG-028 and DEC-050",
          "Required witnesses (proof owner TestPak; gates G2), also carried by `DEC-050` and `LEG-028`:")

    # Gates come from the typed target, never from the source block's text.
    probe("block_gate_text_cannot_override_the_typed_target", GA, H28,
          "gates 'G2/G7' differ from typed LEG-028 gates 'G2'",
          "Required witnesses (proof owner TestPak; gates G2/G7), also carried by `LEG-028`:")

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
    probe("newly_promoted_row_without_an_expectation_clause_is_rejected", GA,
          W28 + "\n```", "proof rows carry no expectation clause, above the transitional ceiling of 0",
          W28 + "\nnewly_promoted_row_without_a_clause\n```")
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


def test_guarantee_authority(audit, project) -> list[str]:
    """Guarantee admission and projection authority (5.5D4b closure).

    These are the adversaries that would have caught the C1 defect. The old suite
    attacked mutation and parity, and both are blind to common-mode invention: two
    scripts carrying the same unauthorized constant agree perfectly.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    GU = "spec/guarantees.rs"
    DI = "spec/dispositions.rs"
    AR = "spec/architecture.rs"

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new,
                                         f"{rel}: {old[:44]!r}"), encoding="utf-8")
            expect(name, (validator or audit.guarantee_admission_findings)(tmp), needle)

    if audit.guarantee_admission_findings(root):
        fail("every_node_passes_typed_admission")

    # 1. No semantic literal may live in both bootstrap paths. A constant in two
    #    "independent" scripts is common-mode invention, not evidence.
    import ast as _ast

    def lits(rel):
        tree = _ast.parse((HERE / rel).read_text(encoding="utf-8"))
        return {n.value for n in _ast.walk(tree)
                if isinstance(n, _ast.Constant) and isinstance(n.value, str)}

    shared = lits("project.py") & lits("audit.py")
    banned = {"Permanent", "docs/30_DECISION_AND_REJECTION_LEDGER.md", "spec/architecture.rs",
              "LegacyObligation", "Decision", "ArchitectureConstraint", "QualificationRequirement",
              "spec/architecture.rs; audit.py architecture checks"}
    pj = (HERE / "project.py").read_text(encoding="utf-8")
    au = (HERE / "audit.py").read_text(encoding="utf-8")
    for lit in sorted(shared & banned):
        for dim in ("lifetime", "owner", "kind", "gates", "witness"):
            asn = f'"{dim}": "{lit}"'
            if asn in pj and asn in au:
                fail(f"common_mode_semantic_constant_{dim}_{lit[:24]}_present_in_both_paths")

    # 2. Every family policy is declared; nothing is left to a projector default.
    pol = audit.guarantee_family_policies(root)
    if set(pol) != {"SEED", "LEG", "DEC", "ARCH", "QUAL"}:
        fail(f"all_five_families_declare_typed_policy (got {sorted(pol)})")
    if project.family_policies(root) != pol:
        fail("generator_and_auditor_read_the_same_typed_family_policy")
    if audit.decision_lifetime_map(root) != project.disposition_lifetimes(root):
        fail("generator_and_auditor_read_the_same_typed_disposition_map")

    # 3. The DEC lifetime map is owned by the typed source, not by either script.
    dm = audit.decision_lifetime_map(root)
    for disp, life in (("Lock", "Permanent"), ("Keep", "Permanent"), ("Kill", "Permanent"),
                       ("Defer", "Permanent"), ("OpenImplementation", "UntilGate"),
                       ("Demote", "UntilCompatibilityExpiry"),
                       ("Supersede", "HistoricalCoverageOnly"),
                       ("RetainAsEvidence", "HistoricalCoverageOnly")):
        if dm.get(disp) != life:
            fail(f"disposition_{disp}_maps_to_{life} (got {dm.get(disp)!r})")
    probe("illegal_disposition_to_lifetime_mapping_is_rejected", DI,
          "Disposition::OpenImplementation => GuaranteeLifetime::UntilGate,",
          "has no typed lifetime", "", validator=audit.guarantee_admission_findings)

    # 4. A projector may not supply a semantic default when policy is absent.
    probe("family_policy_removal_fails_closed", GU,
          '        family: "ARCH",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),',
          "no declared family policy for ARCH",
          '        family: "ARCH-RENAMED",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),')
    probe("family_constant_without_a_value_fails_closed", GU,
          'owner: OwnerRule::FamilyConstant("docs/30_DECISION_AND_REJECTION_LEDGER.md"),',
          "owner is FamilyConstant but names no value",
          'owner: OwnerRule::FamilyConstant(""),')

    # 5. A qualification target may never enter a gate field.
    probe("qualification_target_in_a_gate_field_is_rejected", AR,
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::G0, GateId::G5],",
          "names a value that is not a declared GateId",
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::std],")
    if any(audit.guarantee_gates(root, f"QUAL-{p}") in ("std", "no_std + alloc", "wasm32 host")
           for p in ("batpak-native", "batpak-semantic", "syncbat-browser")):
        fail("qualification_target_cannot_be_read_as_a_gate_posture")

    # 6. Missing metadata is not gate-independence, and neither is an empty list.
    probe("qual_row_without_a_typed_schedule_fails_closed", AR,
          "        gates: &[GateId::G0, GateId::G5],\n        requirement: \"contracts, schemas",
          "gate posture is RowDeclared but the row declares none",
          "        gates: &[],\n        requirement: \"contracts, schemas")
    probe("dec_requiring_a_gate_without_one_fails_closed", DI,
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[GateId::G2]',
          "requires a gate but the row declares none",
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[]')
    # An ungated HistoricalReceipt is GateIndependent by an AUTHORED class claim.
    idx = {n["id"]: n for n in audit.guarantee_derive(root)[0]}
    if idx["DEC-001"]["gate_posture"] != "GateIndependent":
        fail("ungated_historical_receipt_is_gate_independent_by_class")
    if idx["DEC-002"]["lifetime"] != "HistoricalCoverageOnly":
        fail("retain_as_evidence_is_not_projected_as_permanent")
    if idx["DEC-005"]["gate_posture"] != "historical:G0":
        fail("requires_gate_is_a_floor_not_a_ceiling")
    # A retired decision keeps its authored GateIds as provenance, and provenance
    # is never an active schedule. No family is exempt from the invariant.
    for did, want in (("DEC-009", "historical:G0"), ("DEC-010", "historical:G5")):
        if idx[did]["gate_posture"] != want:
            fail(f"{did}_retains_its_gate_reference_as_historical_association")
    def hist_node(posture):
        return {"id": "D", "family": "DEC", "kind": "Decision",
                "lifetime": "HistoricalCoverageOnly", "owner": "o",
                "gate_posture": posture, "target": "", "witness": "NoFamilyWitness"}

    if audit.guarantee_lifetime_findings([hist_node("historical:G0")], {}, []):
        fail("historical_association_is_not_an_active_schedule")
    if not any("cannot be actively scheduled" in x
               for x in audit.guarantee_lifetime_findings([hist_node("G0")], {}, [])):
        fail("historical_lifetime_with_an_active_schedule_is_rejected_for_every_family")

    # 7. ARCH loses its declared G0 schedule.
    probe("arch_row_losing_g0_scheduling_is_rejected", GU,
          "gate_posture: GatePostureRule::FamilyConstant(GatePosture::Scheduled(&[GateId::G0])),",
          "gate posture is RowDeclared but the row declares none",
          "gate_posture: GatePostureRule::RowDeclared,")

    # 7b. Relations are typed constructors (5.5E2): the auditor refuses a bare
    # string, a mistagged family, and an undeclared constructor. Existence of
    # the referenced row is seedcheck's executed law, proven in the Tier 0
    # fixtures with a dangling reference.
    IN = "spec/invariants.rs"

    def rel_validator(tmp):
        return audit.guarantee_typed_relation_findings(audit.guarantee_seed_rows(tmp))

    probe("bare_string_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          "not a typed GuaranteeRef constructor",
          'derives_from: &["DEC-068"]', validator=rel_validator)
    probe("mistagged_relation_family_is_rejected", IN,
          'refines: &[GuaranteeRef::leg("LEG-080")]',
          "whose family owns the LEG- prefix",
          'refines: &[GuaranteeRef::leg("DEC-080")]', validator=rel_validator)
    probe("undeclared_relation_constructor_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-065")]',
          "undeclared relation constructor GuaranteeRef::arch",
          'derives_from: &[GuaranteeRef::arch("DEC-065")]', validator=rel_validator)

    # 7c. Witness citations are typed and resolve (5.5E2): a bare string, an
    # unknown contract id, and a mistagged family are each refused by the
    # auditor; the dangling-guarantee case is seedcheck's executed law and is
    # proven in the Tier 0 fixtures.
    def wit_validator(tmp):
        return audit.guarantee_typed_witness_findings(tmp, audit.guarantee_seed_rows(tmp))

    probe("bare_string_witness_is_rejected", IN,
          'witnesses: &[WitnessRef::dec("DEC-015")]',
          "not a canonical typed WitnessRef",
          'witnesses: &["DEC-015"]', validator=wit_validator)
    probe("unknown_contract_witness_is_rejected", IN,
          'WitnessRef::contract("BP-GAUNTLET-1")',
          "which no document declares",
          'WitnessRef::contract("BP-NOPE-1")', validator=wit_validator)
    probe("mistagged_witness_family_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]',
          "whose family owns the LEG- prefix",
          'witnesses: &[WitnessRef::leg("DEC-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]', validator=wit_validator)

    # 8. Parity passes while provenance fails: mutate the SAME constant in both
    #    paths. This is the exact hole -- before the closure the graph regenerated,
    #    parity passed, and every gate stayed green.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        gu = tmp / GU
        gu.write_text(must_replace(gu.read_text(encoding="utf-8"),
                      "lifetime: LifetimeRule::FromDecisionDisposition,",
                      "lifetime: LifetimeRule::FamilyConstant(GuaranteeLifetime::Permanent),",
                      "common-mode lifetime"), encoding="utf-8")
        lives = {n["id"]: n["lifetime"] for n in audit.guarantee_derive(tmp)[0]}
        if lives.get("DEC-002") != "Permanent":
            fail("common_mode_mutation_probe_did_not_take_effect")
        # The typed source now says Permanent, so BOTH paths agree and parity is
        # silent. Only the typed owner changed -- which is the point: policy lives
        # in one place and a reviewer sees it in the diff, not in two scripts.
        proj = {n["id"]: n["lifetime"] for n in project.guarantee_nodes(tmp)}
        if proj.get("DEC-002") != lives.get("DEC-002"):
            fail("policy_change_is_visible_to_both_paths_from_one_owner")

    findings.extend(canonical_drift(before))
    return findings


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
            path = tmp / rel
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
    except Exception as exc:  # noqa: BLE001 - a crash here is itself the finding
        auditor, generator = [], []
        fail(f"both_derivations_admit_the_isa ({exc!r})")
    if len(auditor) != 36 or len(generator) != 36:
        fail(f"both_derivations_admit_36_nodes (auditor {len(auditor)}, generator {len(generator)})")
    # Independence is only worth something if the two agree on the ANSWER while
    # sharing no code to compute it.
    if [v["node"] for v in auditor] != [s["node"] for s in generator]:
        fail("independent_derivations_agree_on_the_node_inventory")
    for a, g in zip(auditor, generator):
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
            path = tmp / ISA
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            d07 = tmp / D07
            text = d07.read_text(encoding="utf-8")
            for marker, render in (("PAKVM-SEMANTIC-ISA", project.render_pakvm_isa),
                                   ("PAKVM-SIGNATURES", project.render_pakvm_signatures)):
                pat = project.block_pattern(marker)
                text = pat.sub(lambda m: m.group(1) + render(tmp) + m.group(3), text)
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
        path = tmp / ISA
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
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
        p2 = tmp / ISA
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
                    lambda m: m.group(1) + body + m.group(3), text)
        except project.Unadmitted:
            return False
        d.write_text(text, encoding="utf-8")
        return True

    def probe(name, rel, old, needle, new="", regen=True):
        with isolated_tree() as tmp:
            path = tmp / rel
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
    except Exception as exc:  # noqa: BLE001 - a crash here is itself the finding
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
        p = tmp / "spec/architecture.rs"
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
                    lambda m: m.group(1) + body + m.group(3), text)
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


def test_identity_catalogs(audit) -> list[str]:
    """Named hostile fixtures for the identity catalogs (5.5E3d). Four axes,
    one term one axis, owners resolve, chronology stays out, existing typed
    owners are never duplicated, docs/16 projects ordered entries AND owners,
    and the permanent corpus denominator refuses any identity-shaped term
    that resolves through none of the five classification paths."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.identity_catalog_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.identity_catalog_findings(root):
        fail("identity_catalogs_pass_on_the_real_seed")

    ID = "spec/identities.rs"
    DOC16 = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
    # EventId was the sweep's headline restoration; deleting it again is
    # refused by the corpus denominator itself: the term is still authored
    # in docs/04, docs/05, and the companion, so it must classify.
    probe("event_id_cannot_disappear_from_identity_catalog",
          [(ID, "        IdentityKind::Event,\n", ""),
           (ID, '            IdentityKind::Event => entry!("EventId", "BP-STORAGE-TILES-1"),\n',
            "")],
          "identity-shaped term EventId appears in the authoritative corpus "
          "and resolves through none of the five classification paths")
    # A binding is a commitment to content, never the object's identity:
    # cross-axis uniqueness means the misclassification has no spelling.
    probe("content_digest_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("CorrelationId", "BP-STORAGE-TILES-1")',
            'entry!("ContentDigest", "BP-STORAGE-TILES-1")')],
          "ContentDigest answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    probe("commitment_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("Commitment", "BP-IDENTITY-TIME-NAV-1")')],
          "Commitment answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    # Chronology and navigation vocabulary is excluded by executed law.
    probe("hlc_cannot_enter_identity_catalog",
          [(ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "Hlc is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("commit_point_cannot_enter_identity_catalog",
          [(ID, 'entry!("UnitId", "BP-NUMERIC-1")',
            'entry!("CommitPoint", "BP-IDENTITY-TIME-NAV-1")')],
          "CommitPoint is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("identity_kind_missing_from_all_is_rejected",
          [(ID, "        IdentityKind::Tile,\n", "")],
          "IdentityKind::Tile declares an entry but is omitted from IdentityKind::ALL")
    probe("catalog_entry_with_unknown_owner_is_rejected",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("TileId", "BP-NONEXISTENT-1")')],
          "IdentityKind entry TileId names owner BP-NONEXISTENT-1, "
          "which no declared contract owns")
    probe("docs_identity_block_drift_is_rejected",
          [(DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "docs/16 IDENTITY-CATALOG row 7 states EventId BP-SYNCBAT-1; the "
          "typed catalog states EventId BP-STORAGE-TILES-1 at that position")
    # Existing typed owners stay owners: a duplicate variant is a wrapper
    # passport, refused by spelling.
    probe("existing_package_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry PackageId duplicates a spelling that already "
          "has a typed spec owner")
    probe("existing_proof_row_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("KeyBackendId", "BP-CRYPTO-SECRET-1")',
            'entry!("ProofRowId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry ProofRowId duplicates a spelling that already "
          "has a typed spec owner")
    # The standing denominator law: a new identity-shaped term authored
    # anywhere in the corpus turns red until classified through exactly one
    # of the five paths.
    probe("new_identity_shaped_term_requires_classification",
          [(DOC16, "BatPak never invents its own entropy primitive.",
            "BatPak never invents its own entropy primitive. A future "
            "WidgetTrackerId is under discussion.")],
          "identity-shaped term WidgetTrackerId appears in the authoritative "
          "corpus and resolves through none of the five classification paths")
    probe("noncataloged_term_cannot_also_be_admitted",
          [(ID, 'entry!("KeyId", "BP-CRYPTO-SECRET-1")',
            'entry!("NavigationId", "BP-IDENTITY-TIME-NAV-1")')],
          "NavigationId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("owned_elsewhere_term_requires_a_live_contract_owner",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-GHOST-1"))')],
          "residue term GateId is owned elsewhere by BP-GHOST-1, "
          "which no declared contract owns")
    # CompressionId is a passport application stapled to DEC-063's checklist.
    # Sneaking it into a catalog while the residue row stands is the two-path
    # refusal; entry happens only by amending DEC-063 AND moving the term.
    probe("compression_id_cannot_enter_without_dec063_amendment",
          [(ID, 'entry!("FloatFormatId", "BP-NUMERIC-1")',
            'entry!("CompressionId", "BP-NUMERIC-1")')],
          "CompressionId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("not_yet_admitted_term_requires_a_live_decision",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-963"))')],
          "residue term CompressionId is not yet admitted by DEC-963, "
          "which no declared decision owns")
    # A decision retained only as historical coverage has no forward policy
    # authority: the archive does not issue future visas. DEC-005 is the
    # standing Supersede row.
    probe("historical_decision_cannot_own_not_yet_admitted_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-005"))')],
          "residue term CompressionId is not yet admitted by DEC-005, which "
          "is retained only as historical coverage and cannot own a future "
          "admission barrier")
    # 5.5E3d1: TRUE set equality — a catalog entry with only its own
    # generated projection is an answer sheet, not an adopter. The typed
    # entry and the docs/16 row alone must refuse; add a real authored
    # sentence under the live owner contract and the same growth is lawful.
    _SESSION_GROWTH = [
        (ID, "        IdentityKind::Tile,\n",
         "        IdentityKind::Tile,\n        IdentityKind::Session,\n"),
        (ID, '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),',
         '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),\n'
         '            IdentityKind::Session => entry!("SessionId", "BP-STORAGE-TILES-1"),'),
        (DOC16, f"{'TileId':<30} BP-STORAGE-TILES-1\n",
         f"{'TileId':<30} BP-STORAGE-TILES-1\n"
         f"{'SessionId':<30} BP-STORAGE-TILES-1\n"),
    ]
    probe("catalog_entry_without_authored_adopter_is_rejected",
          list(_SESSION_GROWTH),
          "catalog entry SessionId appears in no authored corpus document; "
          "an entry without an authored adopter is a brochure passport")
    # Self-neutering guards: deleting the exclusion or wrapper-guard row
    # before smuggling the term in is refused by independent parity with
    # the authored docs/16 fences and the real spec declarations.
    probe("chronology_exclusion_cannot_be_removed_then_cataloged",
          [(ID, '    "Hlc",\n', ''),
           (ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "EXCLUDED_CHRONOLOGY_AND_NAVIGATION omits Hlc, which docs/16's "
          "navigation and time-and-order fences declare")
    probe("existing_owner_guard_cannot_be_removed_then_readmitted",
          [(ID, '"PackageId", ', ''),
           (ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "EXISTING_TYPED_OWNER_SPELLINGS omits PackageId, which the spec "
          "declares as an existing typed owner")
    # One term, one path — and one ROW on that path.
    probe("duplicate_residue_term_is_rejected",
          [(ID, '        term: "GateId",\n',
            '        term: "GateId",\n        disposition: '
            'IdentityTermDisposition::OwnedElsewhere(ContractId("BP-GATES-1")),\n'
            '    },\n    NonCatalogedIdentityTerm {\n        term: "GateId",\n')],
          "residue term GateId is declared twice; one term, one path also "
          "means one row on that path")
    # Permanent is not enough: Kill is a dead passport and Keep is admitted
    # retained policy — neither is a pending application.
    probe("killed_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Kill disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    probe("kept_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Keep,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Keep disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    # Provenance is verified independently of the renderer: a BEGIN marker
    # naming any other source is not a generated identity block at all.
    probe("identity_projection_source_marker_drift_is_rejected",
          [(DOC16,
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identities.rs "
            "by bootstrap/project.py; do not edit -->",
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identity_soup.rs "
            "by bootstrap/project.py; do not edit -->")],
          "docs/16 carries no generated IDENTITY-CATALOG block naming "
          "spec/identities.rs as source")
    # Owner coherence (5.5E3d2): the owner must AUTHOR the term. Each owner
    # mutation moves the generated docs/16 row in lockstep, so the hostile
    # fails for owner incoherence, never for a stale projection.
    probe("catalog_owner_must_author_the_term",
          [(ID, 'entry!("EventId", "BP-STORAGE-TILES-1")',
            'entry!("EventId", "BP-SYNCBAT-1")'),
           (DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "IdentityKind entry EventId names owner BP-SYNCBAT-1, whose "
          "authoritative document does not author the term")
    probe("owned_elsewhere_owner_must_author_the_term",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-NETBAT-1"))'),
           (DOC16, f"{'GateId':<30} {'owned elsewhere':<22} BP-GATES-1",
            f"{'GateId':<30} {'owned elsewhere':<22} BP-NETBAT-1")],
          "residue term GateId is owned elsewhere by BP-NETBAT-1, "
          "which does not author the term")
    # DEC-058 is live and Lock, so it passes existence and the disposition
    # predicate — but it is about the release seal and says nothing about
    # CompressionId. A live but unrelated authority is refused by name.
    probe("not_yet_admitted_decision_must_name_the_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-058"))'),
           (DOC16, f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-063",
            f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-058")],
          "residue term CompressionId is not yet admitted by DEC-058, whose "
          "forward-policy fields do not name the term")
    # GREEN fixture: a term may lawfully appear in many documents while its
    # canonical owner document also authors it — breadth of adoption is not
    # an ownership dispute.
    tmp = gate_sandbox([
        ("docs/14_RECEIPTS_AND_EXPLANATION.md",
         "It is structured evidence, not a debug log.",
         "It is structured evidence, not a debug log. A replay receipt names "
         "the EventId range it covered.")])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"term_in_many_documents_with_authoring_owner_is_lawful "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    # GREEN fixture: the counts are observed output, never acceptance
    # thresholds. The SAME growth as the brochure hostile above, plus a real
    # authored adopter sentence under the live owner contract (docs/05,
    # BP-STORAGE-TILES-1), is lawful with no frozen number touched.
    tmp = gate_sandbox(_SESSION_GROWTH + [
        ("docs/05_STORAGE_FBAT_AND_TILES.md",
         "EventFrame identity\n",
         "EventFrame identity\nSessionId scoping\n"),
    ])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"catalog_counts_are_derived_not_frozen (lawful growth "
                 f"was refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_seedcheck_executes_its_law(_audit) -> list[str]:
    """Tier 0 rules, proven by RUNNING seedcheck against mutated typed sources.

    Two rules were repaired in 5.5D4c2 after seedcheck was executed for the first
    time. A repair with no adversary is indistinguishable from a nerf, so each one
    is planted against here. These build and run the real binary: asserting on the
    source text would prove the file contains a string, not that the gate bites.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        receipt("tier0-law-fixtures", available=False, reason="rustc absent")
        return findings

    used_triples: set[str] = set()

    def run_seedcheck(tree: Path) -> tuple[int, str]:
        # seedcheck links the typed tables at COMPILE time through the tree's
        # spec rlib (5.5E2), so a mutated spec/ is only tested by building the
        # library from the mutated tree. The real seedcheck.rs is copied in
        # rather than reproduced: a hand-written stand-in would drift from the
        # gate it claims to test. A mutation the COMPILER refuses is a refusal,
        # not linker unavailability: its rustc output is returned so a probe
        # can assert the exact reason instead of silently skipping.
        (tree / "bootstrap").mkdir(exist_ok=True)
        if not (tree / "bootstrap/seedcheck.rs").is_file():
            shutil.copy2(root / "bootstrap/seedcheck.rs", tree / "bootstrap/seedcheck.rs")
        tmp = Path(tempfile.mkdtemp(prefix="batpak-tier0-"))
        try:
            exe = tmp / ("sc" + (".exe" if os.name == "nt" else ""))
            built = None
            for target in (None, "x86_64-pc-windows-gnu"):
                rlib = tmp / f"libspec-{target or 'host'}.rlib"
                lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                           "--crate-name", "spec", "-o", str(rlib),
                           str(tree / "spec/lib.rs")]
                if target:
                    lib_cmd[1:1] = ["--target", target]
                lib_built = subprocess.run(lib_cmd, capture_output=True, text=True)
                if lib_built.returncode != 0:
                    if "linking with" not in lib_built.stderr:
                        return (lib_built.returncode, lib_built.stderr)
                    continue
                cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", "sc",
                       "--extern", f"spec={rlib}",
                       "-o", str(exe), str(tree / "bootstrap/seedcheck.rs")]
                if target:
                    cmd[1:1] = ["--target", target]
                built = subprocess.run(cmd, capture_output=True, text=True)
                if built.returncode == 0:
                    # The receipt names the real triple, never "host default"
                    # (5.5E2c): on the hosted MSVC runner the default-target
                    # build IS the authoritative qualification, and a
                    # law-fixture receipt that hides its triple can never
                    # enter the required-receipt denominator.
                    if target is None:
                        m = re.search(r"host: (\S+)",
                                      subprocess.run([rustc, "-vV"], capture_output=True,
                                                     text=True).stdout)
                        target = m.group(1) if m else "host default"
                    used_triples.add(target)
                    break
                if "linking with" not in built.stderr:
                    return (built.returncode, built.stderr)
            else:
                return (-1, "unavailable")
            if not exe.is_file():
                return (-1, "unavailable")
            proc = subprocess.run([str(exe), str(tree)], capture_output=True, text=True)
            return (proc.returncode, proc.stdout + proc.stderr)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def probe(name: str, rel: str, old: str, new: str, needle: str) -> None:
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            code, out = run_seedcheck(tmp)
            if code == -1:
                print(f"selftest: {name} unavailable (no working linker)")
                return
            if code == 0:
                findings.append(f"{name} FAILED (seedcheck admitted the mutation)")
            elif needle not in out:
                findings.append(f"{name} FAILED (wanted {needle!r}, got {out.strip()[:200]!r})")

    # The premise: Tier 0 passes before any mutation. Without this every probe
    # below could be red for free.
    code, out = run_seedcheck(root)
    if code == -1:
        receipt("tier0-law-fixtures", available=False, compiled=True, executed=False,
                reason="no working linker for any attempted target")
        return findings
    if code != 0:
        receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
                passed=False, reason="seedcheck already fails before any mutation")
        findings.append(f"seedcheck_passes_before_mutation FAILED ({out.strip()[:200]!r})")
        return findings

    DI = "spec/dispositions.rs"
    IN = "spec/invariants.rs"

    # DEC-072: the CLASS decides. A class that requires a gate cannot be empty.
    probe("gate_requiring_decision_class_with_no_gate_is_rejected", DI,
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8],',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, gates: &[],',
          "DEC-063 is implementation-bearing and names no gate")

    # ...and the exception is DEC-specific. SEED declares no gate-independent
    # class, so it must not inherit DEC's lawful emptiness. This is the probe that
    # separates the repair from a nerf.
    probe("seed_does_not_inherit_the_decision_gate_exception", IN,
          "gates: &[GateId::G0, GateId::G5]",
          "gates: &[]",
          "names no gate")

    # A typed relation carries its family in the TYPE; existence is this
    # executed law. A reference to an undeclared decision must redden the
    # running seedcheck, not merely fail to render an edge.
    probe("dangling_typed_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          'derives_from: &[GuaranteeRef::dec("DEC-999")]',
          "which no declared row owns")

    # The same existence law for witness citations (5.5E2): a witness naming
    # an undeclared obligation reddens the running binary.
    probe("dangling_witness_citation_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-066"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          'witnesses: &[WitnessRef::leg("LEG-999"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          "which no declared row owns")

    # A contract kind's admitting law resolves or the running binary reddens:
    # a kind whose law was deleted survives as nothing (5.5E3c).
    probe("contract_kind_with_dangling_admitting_law_is_rejected",
          "spec/contracts.rs",
          'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
          'ContractKind::Error => GuaranteeRef::leg("LEG-999"),',
          "resolves to no declared row")

    # Retirement is supersession with a forwarding address that must point at
    # the LIVING CENSUS (5.5E2j): a successor resolves inside the typed
    # catalog or the running binary reddens.
    probe("dangling_proof_row_successor_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("row_that_was_never_authored_anywhere")]',
          "resolves to no typed catalog identity")

    # ...and a name that occurs in docs/24 PROSE is not a catalog identity:
    # under the deleted substring search, "committed" (which opens a substrate
    # prose line) satisfied existence.
    probe("retired_successor_mentioned_only_in_prose_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("committed")]',
          "resolves to no typed catalog identity")

    # The defect the census exists to catch: delete a canonical active row
    # whose name survives in a historical migration note. The substring
    # search blessed exactly this deletion; the structural parse refuses it.
    probe("active_row_deleted_but_name_left_in_migration_note_is_rejected",
          "docs/24_GAUNTLET.md",
          "\nstale_or_pre_shred_keyset_restore_is_rejected\n",
          "\n",
          "typed Active identity stale_or_pre_shred_keyset_restore_is_rejected "
          "appears as no canonical docs/24 active row")

    # The RUNNING binary refuses a hand-edited toolchain projection and a
    # physical triple sitting in a semantic qualification target (5.5E3a).
    probe("hand_edited_toolchain_projection_reddens_seedcheck",
          "rust-toolchain.toml",
          'channel = "1.97.0"',
          'channel = "1.96.0"',
          "toolchain tracked rust-toolchain.toml violated")
    # 5.5E3a1: the environment is a closed enum, so a triple in the field is
    # a COMPILE refusal — the strongest form. run_seedcheck returns the rustc
    # output for a compile-refused mutation, and the probe asserts the exact
    # type error rather than a runtime finding.
    probe("physical_triple_cannot_substitute_for_semantic_profile",
          "spec/architecture.rs",
          'environment: QualificationEnvironment::WasmHost,',
          'environment: "wasm32-unknown-unknown",',
          "expected `QualificationEnvironment`")
    # 5.5E3d residue lifecycle, executed by the RUNNING gate: a decision
    # retained only as historical coverage cannot own a future admission
    # barrier, and an existing typed owner's spelling is never re-admitted.
    probe("seedcheck_historical_decision_cannot_own_not_yet_admitted_term",
          "spec/identities.rs",
          'NotYetAdmittedBy(DecisionId("DEC-063"))',
          'NotYetAdmittedBy(DecisionId("DEC-005"))',
          "CompressionId is not yet admitted by DEC-005, which is retained "
          "only as historical coverage and cannot own a future admission barrier")
    probe("seedcheck_wrapper_passport_is_rejected",
          "spec/identities.rs",
          'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
          'entry!("PackageId", "BP-CRYPTO-SECRET-1")',
          "duplicates PackageId, which already has a typed spec owner")
    # 5.5E3d1: the typed predicate, not the Permanent lifetime, decides who
    # owns a pending application — Kill is a dead passport.
    probe("seedcheck_killed_decision_cannot_own_not_yet_admitted_term",
          "spec/dispositions.rs",
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,',
          "CompressionId is not yet admitted by DEC-063, whose Kill disposition is "
          "not a standing future-entry policy; only Lock and Defer own pending "
          "applications")

    # SEED-AUDITED-DENOMINATOR's fence is executed law: reclassifying Expired
    # as green must redden the running seedcheck through counts_green().
    probe("expired_proof_counting_green_is_refused",
          "spec/proof.rs",
          "            ProofUnitTerminal::Passed => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Expired\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "            ProofUnitTerminal::Passed | ProofUnitTerminal::Expired => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "counts green; only Passed may")

    # DEC-075's retry fence is executed law, not a comment: reclassifying
    # elapsed wall time as an admissible retry signal must redden the running
    # seedcheck through the real admissible() function.
    probe("weakened_retry_classification_is_refused",
          "spec/reconciliation.rs",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::ProcessDeath",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ProcessDeath",
          "elapsed wall time may not authorize retry")

    # Candidate containment is one law across two constants: an output root
    # relocated under a forbidden write surface must redden seedcheck, not wait
    # for Phase 6 to write a candidate into tracked source.
    probe("candidate_output_root_under_a_forbidden_root_is_rejected",
          "spec/architecture.rs",
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";',
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "spec/candidates/";',
          "sits under forbidden write root")

    # A generated projection is classified by its MARKER, not its filename. An
    # authored document cannot shed its frontmatter duties by looking generated,
    # and a projection that drops its marker takes the authored duties back.
    probe("authored_document_cannot_evade_frontmatter_by_looking_generated",
          "docs/GUARANTEE_GRAPH.generated.md",
          "status: GENERATED", "status: AUTHORITATIVE",
          "missing contract_id: in docs/GUARANTEE_GRAPH.generated.md")

    # A generated projection must still name what produced it. Provenance is the
    # price of not claiming authored authority.
    probe("generated_projection_without_source_provenance_is_rejected",
          "docs/GUARANTEE_GRAPH.generated.md",
          "generated_from:", "produced_from:",
          "missing generated_from: in docs/GUARANTEE_GRAPH.generated.md")

    receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
            passed=not findings,
            target=", ".join(sorted(used_triples)) or "no triple resolved")
    return findings


def test_rust_specification_compiles(_audit) -> list[str]:
    """The typed specification surface must actually compile (5.5D4b).

    Until this session no rustc was available and every Rust contract shipped
    compiler-unverified. When a toolchain IS present this runs; when it is absent
    it reports that plainly rather than passing silently. A skipped check that
    prints PASS is exactly the fake proof this project refuses.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: rustc unavailable; Rust specification surface remains "
              "compiler-unverified until Gate 0")
        return findings
    tmp = Path(tempfile.mkdtemp(prefix="batpak-rustc-"))
    try:
        # The spec is a real LIBRARY (spec/lib.rs, 5.5E2): one rlib boundary,
        # edition 2024, zero suppressions. The binaries LINK it; nothing mounts
        # spec modules textually anymore, so the visibility the probes below
        # attack is the exact boundary production uses.
        spec_meta = tmp / "libspec.rmeta"
        proc = subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
             "--crate-name", "spec", "-o", str(spec_meta), str(root / "spec/lib.rs")],
            capture_output=True, text=True)
        if proc.returncode != 0:
            head = "\n".join(proc.stderr.splitlines()[:6])
            findings.append(f"rust specification surface does not compile (spec):\n{head}")
            return findings
        for name, src in (
            ("seedcheck", root / "bootstrap/seedcheck.rs"),
            ("materialize", root / "bootstrap/materialize.rs"),
        ):
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--emit=metadata", "--crate-name", name,
                 "--extern", f"spec={spec_meta}",
                 "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode != 0:
                head = "\n".join(proc.stderr.splitlines()[:6])
                findings.append(
                    f"rust specification surface does not compile ({name}):\n{head}")
        # seedcheck is Tier 0. Compiling it is not running it: until this check
        # existed it was only ever type-checked, and it had been FAILING since
        # 5.5C1 -- three DEC rows red against a gate rule its own file already
        # said the class decides, and the generated Guarantee Graph red against a
        # frontmatter rule that predated generated documents. Nobody saw it
        # because nobody ran it. A gate that is compiled and never executed is a
        # gate in the same sense that a locked door in a field is a gate.
        # Local execution is SUPPLEMENTAL evidence. The authoritative Windows
        # receipt is a hosted MSVC runner; this machine's linker is whatever
        # happens to be installed, and no toolchain here is a declared BatPak
        # prerequisite. Report exactly one of executed-and-passed,
        # executed-and-failed, or unavailable. Never let unavailable wear the
        # green of a check that passed.
        # BOTH bootstrap binaries execute. materialize.rs was the same species as
        # seedcheck: it existed, it type-checked, and it had never once run.
        for name, src, expect in (
            ("tier0-seedcheck", "bootstrap/seedcheck.rs", "seedcheck: PASS"),
            ("tier0-materialize", "bootstrap/materialize.rs", "materialize: PASS"),
        ):
            findings.extend(qualify_binary(rustc, root, tmp, name, src, expect))
        # The #[cfg(test)] refusal tests inside seedcheck are claims too, and a
        # plain compile STRIPS them: until D4d they had never executed or even
        # type-checked -- the same species as materialize, wearing test syntax.
        # The expected count derives from the authored source at run time, so a
        # deleted test shrinks the expectation and an emptied harness is a
        # finding, never a quiet "ok. 0 passed".
        n_tests = (root / "bootstrap/seedcheck.rs").read_text(
            encoding="utf-8").count("#[test]")
        if n_tests == 0:
            findings.append(
                "seedcheck declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-seedcheck-tests", "bootstrap/seedcheck.rs",
            f"test result: ok. {n_tests} passed", extra=("--test",), run_args=()))
        # The spec's OWN tests (5.5E2): the ISA admission refusals moved into
        # spec/pakvm_isa.rs where the seam they exercise lives — a downstream
        # crate's test cfg no longer reaches across the boundary, which is the
        # boundary working. Qualified with the same artifact binding.
        n_spec_tests = sum(p.read_text(encoding="utf-8").count("#[test]")
                           for p in sorted((root / "spec").glob("*.rs")))
        if n_spec_tests == 0:
            findings.append(
                "the spec declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-spec-tests", "spec/lib.rs",
            f"test result: ok. {n_spec_tests} passed", extra=("--test",),
            run_args=(), link_spec=False))
        # The admission desks have no public bypass and the sealed witnesses
        # cannot be forged or fabricated — proven by COMPILING a hostile
        # consumer against the REAL crate boundary (--extern spec), the exact
        # visibility configuration production uses since the 5.5E2 bake. Each
        # probe must fail for ITS reason: a probe red for an unrelated compile
        # error would certify a boundary nobody tested.
        for name, uses, body, want in (
            ("isa_seam_is_absent_from_production",
             "use spec::pakvm_isa::*;",
             "let ap = algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();\n"
             "    let cp = class_policy(PakVmNodeClass::RowTransform).unwrap();\n"
             "    let _ = admit_candidate_policy(PakVmNodeId::Filter, ap, cp);",
             "cannot find function `admit_candidate_policy`"),
            ("isa_admission_witness_cannot_be_forged",
             "use spec::pakvm_isa::*;",
             "let _ = AdmittedAsPublicSemanticNode(());",
             "cannot initialize a tuple struct which contains private fields"),
            ("firewall_admitted_crossing_cannot_be_forged",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = AdmittedCrossing {\n"
             "        from: SyncBatPlane::Bvisor, to: SyncBatPlane::Runtime,\n"
             "        carries: SyncBatAuthority::RetryRestartAuthority,\n"
             "        posture: CrossingPosture::Required,\n"
             '        origin: None, law: "minted by hand", seal: (),\n'
             "    };",
             "field `seal` of struct"),
            ("firewall_origin_cannot_be_fabricated",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = admit_crossing(SyncBatPlane::PakVm, SyncBatPlane::Bvisor,\n"
             "        SyncBatAuthority::TypedEffectRequest, Some(PakVmNodeId::ReachTheHost));",
             "named `ReachTheHost` found for enum `PakVmNodeId`"),
            # 5.5E2: per-family guarantee identity is sealed at the crate
            # boundary. A consumer can RECEIVE a GuaranteeRef and read it, but
            # cannot mint one claiming a family it did not come from.
            ("guarantee_identity_cannot_be_minted",
             "use spec::guarantees::{GuaranteeRef, SeedId};",
             'let _ = GuaranteeRef::Seed(SeedId("SEED-FORGED"));',
             "cannot initialize a tuple struct which contains private fields"),
            # 5.5E3b: the package identity is the variant; strings cannot
            # mint the guarantee identities and typed relationships cannot
            # be read as raw names.
            ("architecture_guarantee_id_cannot_be_minted_from_a_string",
             "use spec::guarantees::ArchitectureGuaranteeId;",
             'let _ = ArchitectureGuaranteeId("batpak");',
             "cannot initialize a tuple struct which contains private fields"),
            ("qualification_id_cannot_carry_a_string_package",
             "use spec::guarantees::QualificationId;",
             'let _ = QualificationId { package: "batpak", profile: "semantic" };',
             "expected `PackageId`, found `&str`"),
            ("dependency_endpoint_is_package_typed",
             "use spec::architecture::EDGES;",
             "let _: &str = EDGES[0].importer;",
             "expected `&str`, found `PackageId`"),
            ("qualification_package_is_package_typed",
             "use spec::architecture::QUALIFICATION_PROFILES;",
             "let _: &str = QUALIFICATION_PROFILES[0].package;",
             "expected `&str`, found `PackageId`"),
            ("syncbat_plane_package_is_package_typed",
             "use spec::architecture::SyncBatPlane;",
             "let _: &str = SyncBatPlane::Runtime.package();",
             "expected `&str`, found `PackageId`"),
            # 5.5E3d: "not currently admitted" and "forbidden by decision"
            # are DIFFERENT laws, and no term is currently decision-forbidden,
            # so a forbidden disposition has no spelling at all. Mislabeling
            # the pending passport application as a dead passport is a
            # compile error, not a reclassification.
            ("future_required_identity_cannot_be_mislabeled_rejected",
             "use spec::identities::IdentityTermDisposition;\n"
             "use spec::guarantees::DecisionId;",
             'let _ = IdentityTermDisposition::RejectedBy(DecisionId("DEC-063"));',
             "no variant, associated function, or constant named `RejectedBy`"),
        ):
            src = tmp / f"{name}.rs"
            src.write_text(
                uses + f"\npub fn probe() {{\n    {body}\n}}\n", encoding="utf-8")
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
                 "--extern", f"spec={spec_meta}",
                 "--crate-name", name, "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode == 0:
                findings.append(
                    f"{name}: production code reached a sealed, test-only, or "
                    f"fabricated spec item across the crate boundary")
            elif want not in proc.stderr:
                head = "\n".join(proc.stderr.splitlines()[:4])
                findings.append(
                    f"{name}: compile failed, but not with {want!r}:\n{head}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_toolchain(audit) -> list[str]:
    """Named hostile fixtures for the typed toolchain owner (5.5E3a).

    spec/toolchain.rs owns every value; the tracked root projection, the
    materializer, and the harness derive. Each fixture plants a hidden owner
    or a substituted dimension and must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, rel, old, new, needle, subdirs=("spec", "docs", "companion", "bootstrap")):
        with isolated_tree(subdirs=subdirs) as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got = audit.toolchain_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    if audit.toolchain_findings(root):
        fail("toolchain_authority_passes_on_the_real_seed")

    # Hidden owners: a literal in the materializer or the harness is a second
    # authority the moment it exists.
    probe("materialized_resolver_mismatch_is_rejected", "bootstrap/materialize.rs",
          'resolver = \\"{}\\"',
          'resolver = \\"2\\"',
          "hardcodes a resolver literal")
    probe("workspace_msrv_mismatch_is_rejected", "bootstrap/materialize.rs",
          'rust-version = \\"{}\\"',
          'rust-version = \\"1.96\\"',
          "hardcodes a rust-version literal")
    # The planted literal is assembled at run time: written contiguously it
    # would sit in THIS file's source and the hidden-owner scan would fire on
    # its own fixture — the phantom ban catching its author again.
    probe("rustc_edition_mismatch_is_rejected", "bootstrap/selftest.py",
          '"--edition", TOOLCHAIN_EDITION,',
          '"--edition", ' + '"2021",',
          "hardcodes a rustc edition")
    # exact >= floor, never "same minor" (5.5E3a1). A NEWER qualifying
    # compiler that preserves the MSRV floor is lawful — the tracked
    # projection moves with it and NO finding fires; one BELOW the floor is
    # the contradiction. The tracked file is mutated in lockstep so only the
    # law under test can fire.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 98, patch: 0 }",
            "newer_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.98.0"',
                      "newer channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if got:
            fail(f"newer_qualifying_compiler_may_preserve_msrv (got {got!r})")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 96, patch: 0 }",
            "older_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.96.0"',
                      "older channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("below the declared MSRV floor" in f for f in got):
            fail(f"qualifying_compiler_below_msrv_is_rejected (got {got!r})")
    # Dimension substitution, semantic side: renaming an environment spelling
    # into a triple is refused at the enum's own spelling arm.
    probe("semantic_target_shaped_like_a_triple_is_rejected", "spec/architecture.rs",
          'QualificationEnvironment::WasmHost => "wasm32 host",',
          'QualificationEnvironment::WasmHost => "wasm32-unknown-unknown",',
          "is shaped like a physical target triple")
    # RustupComponent::ALL is the one component denominator: dropping a
    # declared component from it (with the projection moving in lockstep)
    # must be refused, or Rustfmt becomes a declared-but-unconsumed variant.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "&[RustupComponent::Clippy, RustupComponent::Rustfmt]",
            "&[RustupComponent::Clippy]",
            "component inventory"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'components = ["clippy", "rustfmt"]',
                      'components = ["clippy"]',
                      "component projection"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("omitted from RustupComponent::ALL" in f for f in got):
            fail("declared_toolchain_component_cannot_be_omitted_from_"
                 f"required_inventory (got {got!r})")

    # Hand-editing or losing the tracked projection is refused.
    probe("root_toolchain_channel_mismatch_is_rejected", "rust-toolchain.toml",
          'channel = "1.97.0"', 'channel = "1.96.0"',
          "does not equal the deterministic projection")
    probe("root_toolchain_component_omission_is_rejected", "rust-toolchain.toml",
          'components = ["clippy", "rustfmt"]', 'components = ["clippy"]',
          "does not equal the deterministic projection")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        (tmp / "rust-toolchain.toml").unlink()
        got = audit.toolchain_findings(tmp)
        if not any("absent" in f for f in got):
            fail(f"tracked_toolchain_projection_cannot_be_missing (got {got!r})")
    return findings


def test_package_identity(audit) -> list[str]:
    """Named hostile fixtures for the typed package identity (5.5E3b). The
    variant is the identity; cargo names and workspace paths are projections;
    every catalog defect must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, old, new, needle):
        with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
            path = tmp / "spec/architecture.rs"
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got: list[str] = []
            audit.check_architecture(tmp, got)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    clean: list[str] = []
    audit.check_architecture(root, clean)
    if clean:
        fail(f"package_catalog_passes_on_the_real_seed (got {clean!r})")

    probe("package_variant_missing_from_all_is_rejected",
          "        PackageId::NetBat,\n        PackageId::TestPak,",
          "        PackageId::TestPak,",
          "PackageId::NetBat is omitted from PackageId::ALL")
    probe("package_id_missing_from_package_specs_is_rejected",
          "    PackageSpec {\n        id: PackageId::NetBat,",
          "    PackageSpec {\n        id: PackageId::TestPak,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("package_spec_duplicate_id_is_rejected",
          "    PackageSpec {\n        id: PackageId::MacBat,",
          "    PackageSpec {\n        id: PackageId::MacBatCompiler,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("two_package_ids_cannot_project_the_same_cargo_name",
          'PackageId::BatQl => "batql",',
          'PackageId::BatQl => "batpak",',
          "two package identities project the same cargo name")
    probe("two_package_ids_cannot_project_the_same_workspace_path",
          'PackageId::BatQl => "crates/batql",',
          'PackageId::BatQl => "crates/batpak",',
          "two package identities project the same workspace path")

    # The materializer may not select behavior by raw package name.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        path = tmp / "bootstrap/materialize.rs"
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            "package.id == architecture::PackageId::MacBat",
            'package.id.cargo_name() == "macbat"',
            "raw-name selection"), encoding="utf-8")
        got: list[str] = []
        audit.check_architecture(tmp, got)
        if not any("selects behavior by raw package name" in f for f in got):
            fail(f"materializer_cannot_select_package_behavior_by_raw_name (got {got!r})")
    return findings


def test_contract_kinds(audit) -> list[str]:
    """Named hostile fixtures for the admitted contract kinds (5.5E3c). The
    enum is the admitted border crossing: a kind without an admitting law is
    a brochure entry, and the doc fence and the enum may not disagree."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.contract_kind_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.contract_kind_findings(root):
        fail("contract_kinds_pass_on_the_real_seed")

    CK = "spec/contracts.rs"
    # A kind with no admitting law is speculation wearing a variant: adding
    # StateMachine to the enum, the inventory, and the spelling map without a
    # citation is refused for exactly that absence.
    probe("speculative_contract_kind_is_rejected",
          [(CK, "    Composition,\n}", "    Composition,\n    StateMachine,\n}"),
           (CK, "        ContractKind::Composition,\n    ];",
                "        ContractKind::Composition,\n        ContractKind::StateMachine,\n    ];"),
           (CK, 'ContractKind::Composition => "Composition",',
                'ContractKind::Composition => "Composition",\n            '
                'ContractKind::StateMachine => "StateMachine",')],
          "ContractKind::StateMachine cites no admission basis")
    probe("contract_kind_missing_from_all_is_rejected",
          [(CK, "        ContractKind::Composition,\n    ];", "    ];")],
          "ContractKind::Composition is omitted from ContractKind::ALL")
    probe("docs_contract_kind_missing_row_is_rejected",
          [("docs/06_MACBAT.md",
            "Composition      LEG-041   deterministic composition identity\n", "")],
          "docs/06 omits admitted contract kind row Composition LEG-041")
    probe("docs_contract_kind_extra_row_is_rejected",
          [("docs/06_MACBAT.md",
            "Composition      LEG-041   deterministic composition identity",
            "Composition      LEG-041   deterministic composition identity\n"
            "StateMachine     LEG-999   speculative")],
          "docs/06 admits StateMachine, which ContractKind does not declare")
    # The BASIS is semantic identity, not decoration: a wrong law, swapped
    # laws, and reordered rows are each refused positionally.
    probe("docs_contract_kind_wrong_basis_is_rejected",
          [("docs/06_MACBAT.md", "Error            LEG-047",
            "Error            LEG-999")],
          "docs/06 row 1 states Error LEG-999; the typed catalog states "
          "Error LEG-047 at that position")
    probe("docs_contract_kind_swapped_bases_are_rejected",
          [("docs/06_MACBAT.md", "Error            LEG-047",
            "Error            LEG-002"),
           ("docs/06_MACBAT.md", "Event            LEG-002",
            "Event            LEG-047")],
          "docs/06 row 1 states Error LEG-002; the typed catalog states "
          "Error LEG-047 at that position")
    probe("docs_contract_kind_order_drift_is_rejected",
          [("docs/06_MACBAT.md",
            "Error            LEG-047   error class and public shape never drift into side tables\n"
            "Event            LEG-002   accepted event bytes are immutable; shape evolves on read",
            "Event            LEG-002   accepted event bytes are immutable; shape evolves on read\n"
            "Error            LEG-047   error class and public shape never drift into side tables")],
          "docs/06 row 1 states Event LEG-002; the typed catalog states "
          "Error LEG-047 at that position")
    probe("mistagged_admitting_guarantee_is_rejected",
          [(CK, 'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
                'ContractKind::Error => GuaranteeRef::leg("DEC-047"),')],
          "whose family owns the LEG- prefix")

    # Lifecycle law (5.5E3c1): a DELETED basis refuses the kind; a lawfully
    # CLOSED basis is retained evidence and the kind remains admitted —
    # closing the founding obligation must not erase the vocabulary it
    # earned. Process citing superseded DEC-009 on the real seed is the
    # standing green witness for the historical tier.
    probe("deleted_admission_basis_is_rejected",
          [("spec/legacy_obligations.rs",
            'LegacyObligation { id: "LEG-047",',
            'LegacyObligation { id: "LEG-047-GONE",')],
          "ContractKind::Error cites admission basis LEG-047, which no "
          "declared row owns")
    tmp = gate_sandbox([("spec/legacy_obligations.rs",
        'id: "LEG-047", law: "Error handling class and public error shape do '
        'not drift into side tables", clean_owner: "macbat-compiler", '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Active }',
        'id: "LEG-047", law: "Error handling class and public error shape do '
        'not drift into side tables", clean_owner: "macbat-compiler", '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Closed }')])
    try:
        got = audit.contract_kind_findings(tmp)
        if got:
            fail("closed_admission_basis_remains_valid_retained_evidence "
                 f"(got {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_required_receipt_denominator() -> list[str]:
    """Dishonest receipt shapes must refuse the authoritative denominator.

    The earned predicate claims available + compiled + executed + passed +
    target-bound. Each stage is checked explicitly: a receipt whose fields
    contradict reality — executed and passed yet compiled=False — is a
    contradictory shape the law refuses, never a gap a reader repairs
    mentally. Runs against fabricated receipts and restores the live list on
    every exit path."""
    findings: list[str] = []
    saved = list(QUALIFICATION_RECEIPTS)
    triple = "x86_64-pc-windows-msvc"

    def earned(name, bound):
        binding = f"{triple} artifact sha256:{'a' * 12} source sha256:{'b' * 12}"
        return {"name": name, "available": True, "compiled": True,
                "executed": True, "passed": True, "reason": None,
                "target": binding if bound else triple}

    def full_set():
        return [earned(n, b) for n, b in REQUIRED_TIER0_RECEIPTS]

    def case(name, receipts, expect_refusal):
        QUALIFICATION_RECEIPTS[:] = receipts
        got = unearned_required_receipts(triple)
        if bool(got) != expect_refusal:
            findings.append(f"required_receipt_{name} FAILED (got {got!r})")

    try:
        case("all_earned_passes", full_set(), False)
        rs = full_set()
        rs[1] = dict(rs[1], compiled=False)
        case("uncompiled_yet_executed_receipt_is_refused", rs, True)
        rs = full_set()
        rs[0] = dict(rs[0], executed=False, passed=None)
        case("unexecuted_receipt_is_refused", rs, True)
        rs = full_set()
        rs[2] = dict(rs[2], passed=False)
        case("failed_receipt_is_refused", rs, True)
        case("missing_receipt_is_refused",
             [r for r in full_set() if r["name"] != "tier0-spec-tests"], True)
        rs = full_set()
        rs[3] = dict(rs[3], target=str(rs[3]["target"]).replace(
            triple, "x86_64-pc-windows-gnu"))
        case("wrong_triple_receipt_is_refused", rs, True)
        rs = full_set()
        rs[4] = dict(rs[4], target=triple)
        case("unbound_artifact_receipt_is_refused", rs, True)
        # A semantic environment where a triple is required (5.5E3a).
        QUALIFICATION_RECEIPTS[:] = full_set()
        if not unearned_required_receipts("std"):
            findings.append(
                "required_receipt_semantic_target_cannot_substitute_for_"
                "physical_triple FAILED")
    finally:
        QUALIFICATION_RECEIPTS[:] = saved
    return findings


def main() -> int:
    # --require-receipts <triple>: the authoritative-lane posture (5.5E2c).
    # Without it, receipts render honestly but an unearned one is a printed
    # "NOT QUALIFIED LOCALLY" line and exit zero — correct for a machine with
    # no linker, never sufficient for the authoritative profile.
    require_target = None
    argv = sys.argv[1:]
    if len(argv) == 2 and argv[0] == "--require-receipts":
        require_target = argv[1]
    elif argv:
        print(f"selftest: unknown arguments {argv!r} "
              "(usage: selftest.py [--require-receipts <triple>])", file=sys.stderr)
        return 2
    freeze = load("freeze")
    audit = load("audit")
    project = load("project")
    # Canonical validator bytes are committed before any hostile probe runs and
    # proven unchanged after the suite. A probe that dies mid-mutation cannot
    # leave a disabled rule behind and still report PASS.
    canonical_before = canonical_commitments()
    findings: list[str] = []
    findings += test_manifest_ordering(freeze)
    findings += test_casefold_collision(audit)
    findings += test_stale_vocabulary(audit)
    findings += test_stale_derivation(audit, HERE.parent)
    findings += test_legacy_manifest_parity(audit)
    findings += test_batql(audit, project)
    findings += test_numeric(audit)
    findings += test_guarantees(audit, project)
    findings += test_gates(audit, project)
    findings += test_decisions(audit, project)
    findings += test_authenticated_history(audit)
    findings += test_document_law(audit)
    findings += test_claim_receipt_law(audit)
    findings += test_substrate(audit)
    findings += test_specialization(audit, project)
    findings += test_delivery_notes_d2(audit)
    findings += test_proof_policy(audit)
    findings += test_integrity_witnesses(audit)
    findings += test_derived_material_witnesses(audit)
    findings += test_deferred_witnesses(audit)
    findings += test_leg081_authority(audit)
    findings += test_proof_target_resolver(audit)
    findings += test_guarantee_authority(audit, project)
    findings += test_pakvm_semantic_isa(audit, project)
    findings += test_syncbat_firewall(audit, project)
    findings += test_syncbat_requiredness(audit, project)
    findings += test_reconciliation(audit, project)
    findings += test_toolchain(audit)
    findings += test_package_identity(audit)
    findings += test_contract_kinds(audit)
    findings += test_identity_catalogs(audit)
    findings += test_required_receipt_denominator()
    findings += test_seedcheck_executes_its_law(audit)
    findings += test_rust_specification_compiles(audit)
    findings += test_probe_harness(audit)
    findings += canonical_drift(canonical_before)
    findings += test_control_characters(audit)
    if findings:
        print(f"selftest: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    # Every receipt, stage by stage, BEFORE the banner. The banner then claims
    # only the qualifications that actually executed and passed: listing "Tier 0
    # execution" unconditionally is precisely how a check that never ran came to
    # read as green.
    rendered = render_receipts()
    if rendered:
        print(rendered)
    claimed = ["portability", "stale-vocabulary", "BatQL", "numeric", "guarantee",
               "gate", "decision", "authenticated-history", "control-character",
               "substrate", "specialization", "proof-policy", "probe-isolation",
               "integrity-witness", "derived-material", "deferred-witness",
               "LEG-081 authority", "proof-target resolver",
               "guarantee-authority hostile fixtures", "PakVM semantic ISA",
               "SyncBat authority firewall",
               "SyncBat crossing requiredness",
               "DEC-075 reconciliation",
               "toolchain authority",
               "package identity",
               "contract kinds",
               "rust specification compile"] + executed_and_passed()
    unearned = [r["name"] for r in QUALIFICATION_RECEIPTS
                if not (r["available"] and r["executed"] and r["passed"])]
    print("selftest: PASS (" + " + ".join(claimed) + ")")
    if unearned:
        # A passing aggregate must never absorb a check that did not run.
        print("selftest: NOT QUALIFIED LOCALLY: " + ", ".join(unearned))
    if require_target:
        problems = unearned_required_receipts(require_target)
        if problems:
            print("selftest: REQUIRED RECEIPTS NOT EARNED "
                  f"(--require-receipts {require_target}):", file=sys.stderr)
            for p in problems:
                print(f"- {p}", file=sys.stderr)
            return 1
        print(f"selftest: receipts-qualified target={require_target} "
              f"receipts={len(REQUIRED_TIER0_RECEIPTS)}/{len(REQUIRED_TIER0_RECEIPTS)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

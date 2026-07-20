"""Independent portability self-test for the bootstrap freeze/audit tools.

Thin package aggregation entry: it self-registers for loader paths that exec the
package without inserting it into sys.modules, imports the full test surface from
the cohesive submodules, and drives them from main(). Standard library only;
operates on synthetic temporary trees and leaves no repository artifacts. The
argv contract is preserved byte-exactly.
"""
from __future__ import annotations

import sys

if __name__ not in sys.modules:  # pragma: no cover - loader-dependent
    import types as _types
    _self_module = _types.ModuleType(__name__)
    _self_module.__path__ = __path__
    sys.modules[__name__] = _self_module

import shutil
import tempfile
from pathlib import Path

from .catalogs import (
    test_commands,
    test_compiler_assumptions,
    test_contract_kinds,
    test_corpus_epoch,
    test_identity_catalogs,
    test_mutation,
    test_package_identity,
    test_promotion,
    test_rust_specification_compiles,
    test_seedcheck_executes_its_law,
    test_toolchain,
)
from .core import (
    HERE,
    QUALIFICATION_RECEIPTS,
    canonical_commitments,
    canonical_drift,
    executed_and_passed,
    load,
    render_receipts,
)
from .corpus import (
    test_authenticated_history,
    test_bundle_inventory_decision_count,
    test_claim_receipt_law,
    test_control_characters,
    test_decisions,
    test_document_law,
    test_gates,
    test_probe_harness,
    test_specialization,
    test_substrate,
)
from .domains import (
    test_pakvm_semantic_isa,
    test_reconciliation,
    test_syncbat_firewall,
    test_syncbat_requiredness,
)
from .e7 import (
    compare_cli as e7_compare_cli,
)
from .e7 import (
    underwrite_cli as e7_underwrite_cli,
)
from .grammar import (
    test_source_grammar,
)
from .guarantees import (
    test_guarantee_authority,
    test_guarantees,
    test_numeric,
)
from .inventory import (
    test_batql,
    test_casefold_collision,
    test_exact_ledgers,
    test_generated_views,
    test_inventory_mirrors,
    test_legacy_manifest_parity,
    test_manifest_ordering,
    test_operator_surfaces,
    test_stale_derivation,
    test_stale_vocabulary,
)
from .proof import (
    test_deferred_witnesses,
    test_derived_material_witnesses,
    test_integrity_witnesses,
    test_leg081_authority,
    test_proof_policy,
    test_proof_relations,
    test_proof_target_resolver,
)
from .supernova import (
    test_supernova,
)
from .supernova_cli import (
    compare_supernova_cli,
    run_supernova_cli,
)
from .tier0 import (
    _T0_GNU,
    _T0_MSVC,
    _confirm_promotion,
    _emit_run_metadata,
    _verify_bundle,
    produce_tier0_evidence,
    test_bootstrap_output,
    test_bootstrap_qualification,
    test_receiptcheck_bundle_perimeter,
    test_receiptcheck_refuses_dishonest_artifacts,
    verify_tier0_evidence,
)
from .verification import (
    test_bootstrap_topology,
    test_isolated_execution,
    test_python_tooling,
    test_sprouting_plane,
    test_verification_plane,
    test_workflow_pinning,
)


def main() -> int:
    # --emit-evidence <dir>: produce the Tier 0 evidence bundle and hand it to
    # the independent verifier (5.5E6b). A standalone mode: it builds the real
    # gates against a clean export, writes qualification.t0, and reports
    # receiptcheck's verdict — no admission predicate of its own.
    argv0 = sys.argv[1:]
    if argv0[:1] == ["--emit-run-metadata"] and len(argv0) >= 2:
        return _emit_run_metadata(argv0[1], argv0[2:])
    if argv0[:1] == ["--confirm-promotion"] and len(argv0) == 6:
        return _confirm_promotion(*argv0[1:])
    if argv0[:1] == ["--verify-bundle"] and len(argv0) == 3:
        return _verify_bundle(argv0[1], argv0[2])
    # --supernova <dir>: execute the F5 mini-supernova campaign rehearsal with
    # persistent roots under <dir> (OUTSIDE the checkout) and block on the
    # independent receiptcheck campaign-verify. Both hosted postures run it.
    if argv0[:1] == ["--supernova"] and len(argv0) == 2:
        return run_supernova_cli(argv0[1])
    # --supernova-compare <own-bundle> <candidate-bundle>: the Stability law --
    # the confirming rehearsal's authoritative results must be identical.
    if argv0[:1] == ["--supernova-compare"] and len(argv0) == 3:
        return compare_supernova_cli(argv0[1], argv0[2])
    # --e7-underwrite <out> --campaign-root <dir> --tier0-bundle <dir>: the
    # E7-F opening-matrix producer -- compute every binding and all twenty
    # zero rows from named bases, then block on receiptcheck e7-verify.
    if argv0[:1] == ["--e7-underwrite"] and len(argv0) == 6 \
            and argv0[2] == "--campaign-root" and argv0[4] == "--tier0-bundle":
        return e7_underwrite_cli(argv0[1], argv0[3], argv0[5])
    # --e7-compare <own-artifact> <candidate-artifact>: the E7 stability law
    # (TL-12) -- authoritative-result equality, then the mechanical-only
    # Phase 6 opening-eligibility banner.
    if argv0[:1] == ["--e7-compare"] and len(argv0) == 3:
        return e7_compare_cli(argv0[1], argv0[2])
    if len(argv0) == 2 and argv0[0] == "--emit-evidence":
        bundle = Path(argv0[1]).resolve() / "tier0-evidence"
        artifact, source_root, problems = produce_tier0_evidence(bundle, _T0_GNU)
        if problems:
            print("selftest: TIER0 EVIDENCE INCOMPLETE:", file=sys.stderr)
            for p in problems:
                print(f"- {p}", file=sys.stderr)
            return 1
        ok, out = verify_tier0_evidence(bundle, artifact, source_root, _T0_GNU)
        print(out)
        if not ok:
            print("selftest: receiptcheck REFUSED the produced evidence", file=sys.stderr)
            return 1
        print(f"selftest: tier0 evidence verified by receiptcheck ({artifact})")
        return 0

    # --require-receipts <triple>: the authoritative-lane posture (5.5E2c).
    # Without it, receipts render honestly but an unearned one is a printed
    # "NOT QUALIFIED LOCALLY" line and exit zero — correct for a machine with
    # no linker, never sufficient for the authoritative profile.
    require_target = None
    require_emit = None
    argv = sys.argv[1:]
    if len(argv) >= 2 and argv[0] == "--require-receipts":
        require_target = argv[1]
        rest = argv[2:]
        if rest[:1] == ["--emit"] and len(rest) == 2:
            # Persist the produced bundle at <dir>/tier0-evidence for upload
            # (5.5E6c1), instead of a temp dir the run discards.
            require_emit = rest[1]
        elif rest:
            print(f"selftest: unknown --require-receipts arguments {rest!r} "
                  "(usage: --require-receipts <triple> [--emit <dir>])", file=sys.stderr)
            return 2
    elif argv:
        print(f"selftest: unknown arguments {argv!r} "
              "(usage: selftest.py [--require-receipts <triple> [--emit <dir>]])",
              file=sys.stderr)
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
    findings += test_operator_surfaces(audit, project)
    findings += test_generated_views(audit, project)
    findings += test_inventory_mirrors(audit, project)
    findings += test_exact_ledgers(audit, project)
    findings += test_proof_relations(audit, project)
    findings += test_bootstrap_output(audit, project)
    findings += test_bootstrap_qualification(audit)
    findings += test_numeric(audit)
    findings += test_guarantees(audit, project)
    findings += test_gates(audit, project)
    findings += test_decisions(audit, project)
    findings += test_authenticated_history(audit)
    findings += test_document_law(audit)
    findings += test_claim_receipt_law(audit)
    findings += test_substrate(audit)
    findings += test_specialization(audit, project)
    findings += test_bundle_inventory_decision_count(audit)
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
    findings += test_commands(audit)
    findings += test_mutation(audit)
    findings += test_corpus_epoch(audit)
    findings += test_compiler_assumptions(audit)
    findings += test_promotion(audit)
    findings += test_receiptcheck_refuses_dishonest_artifacts()
    findings += test_receiptcheck_bundle_perimeter()
    findings += test_workflow_pinning()
    findings += test_verification_plane()
    findings += test_sprouting_plane()
    findings += test_seedcheck_executes_its_law(audit)
    findings += test_source_grammar(audit)
    findings += test_rust_specification_compiles(audit)
    findings += test_probe_harness(audit)
    findings += test_bootstrap_topology(audit)
    findings += test_isolated_execution(audit)
    findings += test_python_tooling(audit)
    findings += test_supernova()
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
               "isolated materializer output",
               "typed Tier 0 receipt policy",
               "rust specification compile",
               "bootstrap_topology",
               "isolated-interpreter execution",
               "python-tooling lock"] + executed_and_passed()
    unearned = [r["name"] for r in QUALIFICATION_RECEIPTS
                if not (r["available"] and r["executed"] and r["passed"])]
    print("selftest: PASS (" + " + ".join(claimed) + ")")
    if unearned:
        # A passing aggregate must never absorb a check that did not run.
        print("selftest: NOT QUALIFIED LOCALLY: " + ", ".join(unearned))
    if require_target:
        # Compat surface (5.5E6b): the flag no longer runs a Python admission
        # predicate. It produces concrete evidence for the requested target and
        # delegates the verdict to the independent verifier. The authoritative
        # MSVC lane rides a git-checkout with a hosted-run binding (the hosted
        # runner); the supplemental GNU lane rides a clean frozen export.
        if require_target not in (_T0_MSVC, _T0_GNU):
            print(f"selftest: --require-receipts {require_target} is not a Tier 0 "
                  f"qualification target ({_T0_MSVC} or {_T0_GNU})", file=sys.stderr)
            return 1
        if require_emit:
            bundle = Path(require_emit).resolve() / "tier0-evidence"
        else:
            bundle = Path(tempfile.mkdtemp(prefix="batpak-tier0-evidence-")) / "tier0-evidence"
        try:
            artifact, source_root, problems = produce_tier0_evidence(bundle, require_target)
            if problems:
                print("selftest: REQUIRED RECEIPTS NOT EARNED "
                      f"(--require-receipts {require_target}):", file=sys.stderr)
                for p in problems:
                    print(f"- {p}", file=sys.stderr)
                return 1
            ok, out = verify_tier0_evidence(bundle, artifact, source_root, require_target)
            if not ok:
                print("selftest: receiptcheck REFUSED the produced evidence "
                      f"(--require-receipts {require_target}):", file=sys.stderr)
                print(out, file=sys.stderr)
                return 1
            print(out)
            print(f"selftest: receipts-qualified target={require_target} "
                  "(verified by bootstrap/receiptcheck.rs)")
            if require_emit:
                print(f"selftest: tier0 evidence bundle persisted ({bundle})")
        finally:
            # Persisted bundles (for upload) survive; temp bundles are discarded.
            if not require_emit:
                shutil.rmtree(bundle.parent, ignore_errors=True)
    return 0

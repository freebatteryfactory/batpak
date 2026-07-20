"""CLI wiring for the mini-supernova campaign rehearsal (E7-closeout split).

`selftest.py --supernova <dir>` executes the complete rehearsal with
persistent roots under <dir> (OUTSIDE the checkout) and blocks on the
independent receiptcheck campaign-verify verdict. The cross-run authoritative
comparison (the Stability row's law) is no longer a separate entry here: the
single --e7-crossrun comparison authority (selftest/e7.py) absorbed it (CL-3),
calling supernova_bundle.compare_authoritative directly. Wiring only: the
rehearsal lives in supernova.py, the grammars and the verifier delegation in
supernova_bundle.py, and the module graph stays a DAG (this module imports
the harness, never the reverse).
"""
from __future__ import annotations

import shutil
import sys
from pathlib import Path

from .supernova import run_rehearsal
from .supernova_bundle import verify_bundle

__all__ = ["run_supernova_cli", "verify_bundle"]


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

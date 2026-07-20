"""E7 cross-run stability: the --e7-crossrun producer and the --e7-open shim.

Split from selftest/e7.py (the underwrite + verify-side concern) to keep each
file under its size ratchet, mirroring the campaign split (supernova_cli.py).
--e7-crossrun ABSORBS the retired --supernova-compare (CL-3): ONE comparison
authority performs BOTH the campaign authoritative comparison
(supernova_bundle.compare_authoritative) AND the E7 authoritative comparison
(bindings minus the two per-run keys + every zero row's (token, count) RESULT
+ both artifacts carrying `cross-run-stability pending`), prints BOTH ruled
literals, and writes the BATPAK-CROSSRUN-STABILITY-RECEIPT/1 content-addressed
into the own run's e7 receipts dir. The zero-row comparison is over the RESULT
only, NOT the owner/receipt provenance: the campaign-verify owner receipt
binds the per-run receiptcheck exe, so its digest is per-run evidence (like the
tier0/campaign bindings), verified WITHIN each run by e7-verify. --e7-crossrun
NEVER prints the eligibility banner; only `receiptcheck e7-open` does (via the
--e7-open shim below), keeping the workflow on one entrypoint.
"""
from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from . import supernova_bundle as sb
from .core import ProbeError
from .e7 import (
    ARCHITECT_LINE,
    ARTIFACT_NAME,
    AUTHORITATIVE_KEYS,
    BIND_KEYS,
    CROSSRUN_PENDING,
    E7_MAGIC,
    ROW_OWNERS,
    STABILITY_MAGIC,
    ZERO_TOKENS,
)

__all__ = ["crossrun_cli", "open_cli"]


def _parse_artifact(path: Path) -> dict:
    raw = path.read_bytes()
    if b"\r" in raw or not raw.endswith(b"\n"):
        raise ProbeError(f"{path}: not a strict LF-terminated artifact")
    try:
        lines = raw.decode("ascii").split("\n")[:-1]
    except UnicodeDecodeError as exc:
        raise ProbeError(f"{path}: not ASCII: {exc}") from None
    pos = 0

    def take(expect_key: str) -> str:
        nonlocal pos
        if pos >= len(lines) or not lines[pos].startswith(expect_key + " "):
            raise ProbeError(f"{path}: expected {expect_key!r} at line "
                             f"{pos + 1}, found "
                             f"{lines[pos] if pos < len(lines) else '<end>'!r}")
        value = lines[pos].removeprefix(expect_key + " ")
        pos += 1
        return value

    if not lines or lines[0] != E7_MAGIC:
        raise ProbeError(f"{path}: magic is not {E7_MAGIC}")
    pos = 1
    bindings = {key: take(key) for key in BIND_KEYS}
    rows: list[tuple[str, int, str, str]] = []
    for token, expected_owner in zip(ZERO_TOKENS, ROW_OWNERS, strict=True):
        value = take("zero")
        parts = value.split(" ")
        if len(parts) != 6 or parts[0] != token or not parts[1].isdigit() \
                or parts[2] != "owner" or parts[3] != expected_owner \
                or parts[4] != "receipt" \
                or not re.fullmatch(r"[0-9a-f]{64}", parts[5]):
            raise ProbeError(f"{path}: zero row {value!r} breaks the frozen "
                             f"`zero {token} <n> owner {expected_owner} "
                             "receipt <64hex>` grammar")
        rows.append((token, int(parts[1]), parts[3], parts[5]))
    if pos >= len(lines) or lines[pos] != CROSSRUN_PENDING:
        raise ProbeError(f"{path}: the cross-run-stability line is not "
                         f"{CROSSRUN_PENDING!r} (no time travel)")
    pos += 1
    if pos >= len(lines) or lines[pos] != ARCHITECT_LINE:
        raise ProbeError(f"{path}: the architect-semantic-row line is absent")
    pos += 1
    if pos >= len(lines) or lines[pos] != "end" or pos + 1 != len(lines):
        raise ProbeError(f"{path}: no exact `end` terminator")
    return {"bindings": bindings, "rows": rows}


def _e7_authoritative_diff(mine: dict, theirs: dict) -> list[str]:
    """Every AUTHORITATIVE divergence between two independently produced /2
    artifacts: the bindings minus the two per-run keys, and each zero row's
    (token, count) RESULT (both parses already assert `cross-run-stability
    pending`)."""
    findings: list[str] = []
    for key in AUTHORITATIVE_KEYS:
        if mine["bindings"][key] != theirs["bindings"][key]:
            findings.append(f"authoritative line {key} diverges: "
                            f"{mine['bindings'][key]} vs "
                            f"{theirs['bindings'][key]}")
    for (token, count_a, _oa, _ra), (_t, count_b, _ob, _rb) in zip(
            mine["rows"], theirs["rows"], strict=True):
        if count_a != count_b:
            findings.append(f"authoritative zero row {token} diverges: "
                            f"{count_a} vs {count_b}")
    return findings


def _render_stability(source_commit: str, own_art: str, cand_art: str,
                      own_bundle: str, cand_bundle: str) -> str:
    lines = [STABILITY_MAGIC, f"source-commit {source_commit}",
             f"own-e7-artifact {own_art}",
             f"candidate-e7-artifact {cand_art}",
             f"own-campaign-bundle {own_bundle}",
             f"candidate-campaign-bundle {cand_bundle}",
             "campaign-authoritative identical",
             "e7-authoritative identical", "end"]
    return "".join(line + "\n" for line in lines)


def crossrun_cli(own_e7_dir: str, cand_e7_dir: str, own_bundle: str,
                 cand_bundle: str) -> int:
    """`selftest.py --e7-crossrun <own-e7-dir> <candidate-e7-dir> <own-bundle>
    <candidate-bundle>`: the single cross-run comparison authority (CL-3).
    Compare BOTH the campaign bundles' and the two e7 artifacts' authoritative
    results; on identity, print both ruled literals and write the stability
    receipt into the own run's e7 receipts dir. Never prints the eligibility
    banner; exits nonzero on any divergence."""
    own_art_path = Path(own_e7_dir).resolve() / ARTIFACT_NAME
    cand_art_path = Path(cand_e7_dir).resolve() / ARTIFACT_NAME
    own_bundle_path = Path(own_bundle).resolve()
    cand_bundle_path = Path(cand_bundle).resolve()
    try:
        mine = _parse_artifact(own_art_path)
        theirs = _parse_artifact(cand_art_path)
    except ProbeError as exc:
        print(f"selftest: E7 CROSSRUN REFUSED: {exc}", file=sys.stderr)
        return 1
    own_bundle_text = own_bundle_path.read_text(encoding="utf-8")
    cand_bundle_text = cand_bundle_path.read_text(encoding="utf-8")
    campaign_findings = sb.compare_authoritative(own_bundle_text, cand_bundle_text)
    e7_findings: list[str] = []
    if mine["bindings"]["source-commit"] != theirs["bindings"]["source-commit"]:
        e7_findings.append("the two runs bind different source commits")
    e7_findings += _e7_authoritative_diff(mine, theirs)
    if campaign_findings or e7_findings:
        print(f"selftest: E7 CROSSRUN STABILITY FAIL "
              f"({len(campaign_findings) + len(e7_findings)} finding(s))",
              file=sys.stderr)
        for finding in campaign_findings + e7_findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("campaign authoritative results identical across runs "
          "(confirming rerun changed no authoritative result)")
    print("E7 authoritative results identical across runs")
    receipt_text = _render_stability(
        mine["bindings"]["source-commit"],
        sb.sha_hex(own_art_path.read_bytes()),
        sb.sha_hex(cand_art_path.read_bytes()),
        sb.sha_hex(own_bundle_path.read_bytes()),
        sb.sha_hex(cand_bundle_path.read_bytes()))
    receipts_dir = Path(own_e7_dir).resolve() / "receipts"
    receipts_dir.mkdir(parents=True, exist_ok=True)
    digest = sb.sha_hex(receipt_text.encode("ascii"))
    (receipts_dir / f"{digest}.receipt").write_text(
        receipt_text, encoding="ascii", newline="\n")
    print(f"selftest: cross-run stability receipt written "
          f"({receipts_dir / (digest + '.receipt')})")
    return 0


def open_cli(stability_receipt: str, own_artifact: str, candidate_artifact: str,
             source_commit: str) -> int:
    """`selftest.py --e7-open <stability-receipt> <own-artifact>
    <candidate-artifact> <source-commit>`: a THIN shim that builds the real
    receiptcheck and delegates to its `e7-open` mode (the sole printer of
    phase6-opening-eligible). Python never re-implements the opening verdict."""
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: --e7-open requires rustc", file=sys.stderr)
        return 1
    work = Path(tempfile.mkdtemp(prefix="batpak-e7-open-"))
    try:
        target = sb.working_target(rustc, work)
        rc_exe, err = sb.build_receiptcheck(rustc, target, work)
        if rc_exe is None:
            print(f"selftest: receiptcheck unavailable for --e7-open: {err}",
                  file=sys.stderr)
            return 1
        proc = subprocess.run(
            [str(rc_exe), "e7-open", str(Path(stability_receipt).resolve()),
             "--own-artifact", str(Path(own_artifact).resolve()),
             "--candidate-artifact", str(Path(candidate_artifact).resolve()),
             "--source-commit", source_commit],
            capture_output=True, text=True)
        out = (proc.stdout + proc.stderr).strip()
        if out:
            print(out)
        if proc.returncode != 0:
            print("selftest: receiptcheck REFUSED the opening receipt",
                  file=sys.stderr)
            return 1
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)

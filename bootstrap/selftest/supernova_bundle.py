"""Campaign records, receipts, and the versioned campaign-evidence grammars.

The RECORD side of the F5 mini-supernova rehearsal: content-addressed
candidate records rendered as BATPAK-CANDIDATE-MANIFEST/1 (the spec/campaign/
grammar, re-implemented here as the PRODUCER; bootstrap/receiptcheck.rs
re-parses it independently), append-only judge-bound evidence and receipt
artifacts, the BATPAK-CAMPAIGN-EVIDENCE/1 bundle, the BATPAK-CAMPAIGN-
ENVELOPE/1 rehearsal release envelope (all 20 seal fields; explicit empty is
stated, never omitted), the authoritative-result comparison the Stability row
demands, and the delegation to the independent receiptcheck campaign verifier.

Nothing here votes on subject semantics: verdicts come from the witness, and
the bundle's admission comes from receiptcheck's independent recompute.
"""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
from pathlib import Path

from .core import HERE, TOOLCHAIN_EDITION, ProbeError

BUNDLE_MAGIC = "BATPAK-CAMPAIGN-EVIDENCE/1"
ENVELOPE_MAGIC = "BATPAK-CAMPAIGN-ENVELOPE/1"
MANIFEST_MAGIC = "BATPAK-CANDIDATE-MANIFEST/1"
RECEIPT_MAGIC = "BATPAK-CAMPAIGN-RECEIPT/1"

# The 20 release seal fields in canonical order with their typed empty-set
# posture, mirrored from spec/release/ (receiptcheck re-reads the typed owner
# independently, so drift between this producer copy and the spec reddens the
# independent verification).
SEAL_FIELDS = (
    ("SourceTree", "commitment"),
    ("Toolchain", "commitment"),
    ("DependencyGraph", "commitment"),
    ("GeneratedFacts", "commitment"),
    ("CompatibilityCorpus", "rows"),
    ("TestDispositions", "rows"),
    ("MutationDispositions", "rows"),
    ("FuzzDispositions", "rows"),
    ("BenchmarkDispositions", "rows"),
    ("ModelDispositions", "rows"),
    ("RuntimeConformanceDispositions", "rows"),
    ("CompilerAssumptionLedger", "rows"),
    ("DependencyLedger", "rows"),
    ("KernelQualificationSet", "rows"),
    ("CandidatePromotionSet", "rows"),
    ("PackageContents", "rows"),
    ("PublicApi", "rows"),
    ("Sbom", "rows"),
    ("LicenseEvidence", "rows"),
    ("ProofFreshness", "commitment"),
)

# The nine docs/24 mini-supernova proof-row identities this rehearsal
# realizes literally; the ProofFreshness commitment binds them.
REALIZED_PROOF_ROWS = (
    "planted_semantic_mutant_is_activated_and_killed",
    "bounded_generated_trace_attack_holds_boundary",
    "deterministic_replay_equality_of_simulated_trace",
    "runtime_capture_appends_observed_history_without_rewrite",
    "offline_replay_concludes_conformance_for_captured_history",
    "every_admitted_campaign_obligation_reaches_a_terminal",
    "candidate_search_terminates_within_declared_budget",
    "bounded_repair_loop_reaches_stable_qualified_frontier",
    "confirming_rerun_changes_no_authoritative_result",
)


def sha_hex(data: bytes | str) -> str:
    if isinstance(data, str):
        data = data.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def tree_digest(root: Path) -> str:
    """Entry-aware tree digest, row-identical to receiptcheck's hashing.rs
    (`<rel>\\tdir\\t-` / `<rel>\\tfile\\t<sha256>`; sorted; sha256 the join)
    so the frozen-judge binding is recomputable by the independent verifier."""
    rows: list[str] = []
    for path in root.rglob("*"):
        rel = path.relative_to(root).as_posix()
        if path.is_symlink() or (not path.is_dir() and not path.is_file()):
            raise ProbeError(f"judge tree entry {rel} is neither file nor dir")
        if path.is_dir():
            rows.append(f"{rel}\tdir\t-")
        else:
            rows.append(f"{rel}\tfile\t{sha_hex(path.read_bytes())}")
    rows.sort()
    return sha_hex("".join(r + "\n" for r in rows))


def normalize_paths(text: str, roots: list[Path]) -> str:
    """Replace campaign-root absolute paths (both separator spellings) so
    diagnostics-derived evidence is content-addressed identically across
    reruns of the same source."""
    for root in roots:
        s = str(root)
        for spelling in (s, s.replace("\\", "/"), s.replace("/", "\\")):
            text = text.replace(spelling, "<campaign>")
    return text.replace("\r\n", "\n")


# ---------------------------------------------------------------------------
# Candidate records and the BATPAK-CANDIDATE-MANIFEST/1 render
# ---------------------------------------------------------------------------

def mint_record(unit: str, *, parents: list[str], source_frontier: str,
                dependencies: list[tuple[str, str]], content: str, origin: str,
                change_class: str, posture: str) -> dict:
    """One immutable nursery lineage record. The id is content-addressed over
    the identity preimage; the reuse key over unit + content + dependencies
    (requalified against upstream churn before any reuse)."""
    content_commitment = sha_hex(content)
    preimage = "\n".join([
        "candidate-id-preimage/1", unit, *parents, source_frontier,
        content_commitment, origin,
    ])
    reuse_preimage = "\n".join([
        "reuse-key-preimage/1", unit, content_commitment,
        *[f"{d} {c}" for d, c in sorted(dependencies)],
    ])
    return {
        "unit": unit,
        "id": sha_hex(preimage),
        "parents": list(parents),
        "source_frontier": source_frontier,
        "dependencies": list(dependencies),
        "content_commitment": content_commitment,
        "origin": origin,
        "change_class": change_class,
        "posture": posture,
        "evidence": [],
        "qualification_receipts": [],
        "holdout_receipts": [],
        "reuse_key": sha_hex(reuse_preimage),
        "terminal": None,          # (spelling, receipt_ref) once receipted
        "frontier": "BlockedByDependency",  # the frontier has not reached it
        "freshness": "Fresh",      # EvidenceFreshness spelling
        "frontier_reason": "in-flight",
    }


def render_manifest(record: dict) -> str:
    """The canonical BATPAK-CANDIDATE-MANIFEST/1 line grammar, field for
    field the spec/campaign/types.rs order."""
    lines = [MANIFEST_MAGIC, f"candidate-id {record['id']}",
             f"parent-count {len(record['parents'])}"]
    lines.extend(f"parent {p}" for p in record["parents"])
    lines.append(f"source-frontier-commitment {record['source_frontier']}")
    lines.append(f"dependency-count {len(record['dependencies'])}")
    lines.extend(f"dependency {d} {c}" for d, c in record["dependencies"])
    lines.append(f"content-commitment {record['content_commitment']}")
    lines.append(f"origin {record['origin']}")
    lines.append(f"change-class {record['change_class']}")
    lines.append(f"realization-posture {record['posture']}")
    lines.append(f"evidence-count {len(record['evidence'])}")
    lines.extend(f"evidence {e}" for e in record["evidence"])
    lines.append(f"qualification-receipt-count {len(record['qualification_receipts'])}")
    lines.extend(f"qualification-receipt {r}" for r in record["qualification_receipts"])
    lines.append(f"holdout-receipt-count {len(record['holdout_receipts'])}")
    lines.extend(f"holdout-receipt {r}" for r in record["holdout_receipts"])
    lines.append(f"reuse-key {record['reuse_key']}")
    if record["terminal"] is None:
        lines.append("terminal none")
    else:
        spelling, receipt = record["terminal"]
        lines.append(f"terminal {spelling} {receipt}")
    return "".join(line + "\n" for line in lines)


# ---------------------------------------------------------------------------
# Append-only, judge-bound evidence and receipts
# ---------------------------------------------------------------------------

def put_artifact(evidence_dir: Path, suffix: str, text: str) -> str:
    """Content-address one artifact into the evidence root, append-only: a
    path collision with DIFFERENT bytes is a refusal, never an overwrite."""
    data = text.encode("utf-8")
    digest = sha_hex(data)
    path = evidence_dir / f"{digest}.{suffix}"
    if path.exists():
        if path.read_bytes() != data:
            raise ProbeError(f"evidence collision at {path.name}: append-only violated")
        return digest
    path.write_bytes(data)
    return digest


def put_evidence(evidence_dir: Path, judge_digest: str, kind: str, payload: str) -> str:
    text = (f"BATPAK-CAMPAIGN-EVIDENCE-ARTIFACT/1\nkind {kind}\n"
            f"judge {judge_digest}\n--\n{payload}")
    if not text.endswith("\n"):
        text += "\n"
    return put_artifact(evidence_dir, "evidence", text)


def put_receipt(evidence_dir: Path, judge_digest: str, kind: str,
                candidate_id: str, lines: list[str]) -> str:
    text = "".join(
        line + "\n"
        for line in [RECEIPT_MAGIC, f"kind {kind}", f"candidate {candidate_id}",
                     f"judge {judge_digest}", *lines])
    return put_artifact(evidence_dir, "receipt", text)


def evidence_snapshot(evidence_dir: Path) -> dict[str, str]:
    return {p.name: sha_hex(p.read_bytes())
            for p in sorted(evidence_dir.iterdir()) if p.is_file()}


def evidence_append_only(before: dict[str, str], after: dict[str, str]) -> list[str]:
    """Prior evidence bytes may never move; later observation only appends."""
    out = []
    for name, digest in sorted(before.items()):
        if after.get(name) != digest:
            out.append(f"evidence artifact {name} was rewritten or removed")
    return out


# ---------------------------------------------------------------------------
# The bundle and the rehearsal release envelope
# ---------------------------------------------------------------------------

def render_bundle(c: dict) -> str:
    """BATPAK-CAMPAIGN-EVIDENCE/1: sections judge/source/toolchain/policy/
    manifests/candidates/dispositions/frontier/closure, LF-only ASCII."""
    lines: list[str] = [BUNDLE_MAGIC, "section: judge",
                        f"judge-root-digest {c['judge_digest']}",
                        f"judge-root-digest-after {c['judge_digest_after']}",
                        "section: source", f"source-commit {c['source_commit']}",
                        "section: toolchain", f"rustc-release {c['rustc_release']}",
                        f"target {c['target']}", "section: policy"]
    for axis, declared in c["budget_declared"].items():
        actual = c["budget_actual"][axis]
        lines.append(f"search-budget {axis} declared={declared} actual={actual}")
    lines.append(c["fuzz_line"])
    lines.append("section: manifests")
    for role in ("search", "qualification", "holdout", "regression"):
        count, digest = c["vector_sets"][role]
        lines.append(f"evaluation-set {role} count={count} digest={digest}")
    lines.append("search-holdout-disjoint yes")
    lines.append("section: candidates")
    lines.append(f"candidate-count {len(c['records'])}")
    for record in c["records"]:
        lines.append(f"candidate-begin {record['id']} unit={record['unit']}")
        lines.extend(render_manifest(record).splitlines())
        lines.append(f"candidate-end {record['id']}")
    lines.append("section: dispositions")
    lines.extend(c["model_dispositions"])
    lines.extend(c["runtime_dispositions"])
    lines.append(c["mutant_line"])
    lines.append("section: frontier")
    for record in c["records"]:
        lines.append(f"frontier {record['id']} state={record['frontier']} "
                     f"freshness={record['freshness']} reason={record['frontier_reason']}")
    lines.append("section: closure")
    for record in c["records"]:
        spelling = record["terminal"][0] if record["terminal"] else "none"
        lines.append(f"node {record['id']} unit={record['unit']} "
                     f"posture={record['posture']} frontier={record['frontier']} "
                     f"terminal={spelling} freshness={record['freshness']}")
    for src, dst, kind in c["edges"]:
        lines.append(f"edge {src} {dst} {kind}")
    return "".join(line + "\n" for line in lines)


def render_envelope(c: dict) -> str:
    """The rehearsal release envelope: ALL 20 seal fields in canonical order.
    A NotSetValued field binds one commitment; a set-valued field states its
    row count even when zero -- explicit emptiness, never omission. The three
    new fields carry the model dispositions (witness verdicts), the
    runtime-conformance dispositions (capture + offline replay), and the
    candidate promotion receipts."""
    rows: dict[str, list[str]] = c["seal_rows"]
    commitments: dict[str, str] = c["seal_commitments"]
    lines = [ENVELOPE_MAGIC]
    for field, shape in SEAL_FIELDS:
        if shape == "commitment":
            lines.append(f"seal {field} commitment {commitments[field]}")
        else:
            field_rows = rows.get(field, [])
            lines.append(f"seal {field} rows {len(field_rows)}")
            lines.extend(f"row {field} {row}" for row in field_rows)
    return "".join(line + "\n" for line in lines)


# ---------------------------------------------------------------------------
# Authoritative-result comparison (the Stability row): terminals, frontier
# state, and dispositions from the confirming rehearsal must equal the
# candidate rehearsal's. Receipt-content hashes that bind measured resource
# actuals (the search receipt's monotonic ticks) are per-run evidence, not
# authoritative results, so the comparison reads the authoritative sections.
# ---------------------------------------------------------------------------

def _sections(text: str) -> dict[str, list[str]]:
    lines = text.splitlines()
    if not lines or lines[0] != BUNDLE_MAGIC:
        raise ProbeError("not a BATPAK-CAMPAIGN-EVIDENCE/1 bundle")
    out: dict[str, list[str]] = {}
    current: str | None = None
    for line in lines[1:]:
        if line.startswith("section: "):
            current = line[len("section: "):]
            out[current] = []
        elif current is None:
            raise ProbeError(f"bundle line outside any section: {line!r}")
        else:
            out[current].append(line)
    return out


def _candidate_authority(section: list[str]) -> list[str]:
    """Per-candidate authoritative tuple: id, unit, origin, change class,
    posture, terminal spelling -- receipt hashes excluded (per-run)."""
    out: list[str] = []
    current: dict[str, str] = {}
    for line in section:
        if line.startswith("candidate-begin "):
            current = {"begin": line[len("candidate-begin "):]}
        for key in ("origin ", "change-class ", "realization-posture ", "terminal "):
            if line.startswith(key):
                current[key.strip()] = line[len(key):]
        if line.startswith("candidate-end "):
            terminal = current.get("terminal", "none").split(" ")[0]
            out.append(f"{current.get('begin')} origin={current.get('origin')} "
                       f"change-class={current.get('change-class')} "
                       f"posture={current.get('realization-posture')} "
                       f"terminal={terminal}")
    return out


def compare_authoritative(mine: str, theirs: str) -> list[str]:
    """Every authoritative divergence between two campaign bundles."""
    a, b = _sections(mine), _sections(theirs)
    findings: list[str] = []
    for name in ("judge", "source", "manifests", "dispositions", "frontier", "closure"):
        if a.get(name) != b.get(name):
            sa, sb = a.get(name, []), b.get(name, [])
            diff = [f"-{x}" for x in sa if x not in sb] + [f"+{x}" for x in sb if x not in sa]
            findings.append(f"authoritative section {name} diverges: {diff[:4]!r}")
    if _candidate_authority(a.get("candidates", [])) != \
            _candidate_authority(b.get("candidates", [])):
        findings.append("candidate authoritative tuples diverge "
                        "(id/origin/change-class/posture/terminal)")
    return findings


# ---------------------------------------------------------------------------
# Delegation to the independent verifier (R4): build the REAL receiptcheck
# and run its campaign-verify mode. Python never re-implements the verdict.
# ---------------------------------------------------------------------------

def working_target(rustc: str, work: Path) -> str:
    probe = work / "sn-probe.rs"
    probe.write_text("fn main() {}\n", encoding="utf-8")
    for target in (None, "x86_64-pc-windows-gnu"):
        exe = work / ("sn-probe" + (".exe" if os.name == "nt" else ""))
        if exe.exists():
            exe.unlink()
        flags = ["--target", target] if target else []
        if subprocess.run([rustc, "--edition", TOOLCHAIN_EDITION, *flags, "-o",
                           str(exe), str(probe)],
                          capture_output=True, text=True).returncode == 0 and exe.is_file():
            if target:
                return target
            m = re.search(r"host: (\S+)", subprocess.run(
                [rustc, "-vV"], capture_output=True, text=True).stdout)
            return m.group(1) if m else "x86_64-pc-windows-gnu"
    return "x86_64-pc-windows-gnu"


def build_receiptcheck(rustc: str, target: str, work: Path,
                       source_root: Path | None = None) -> tuple[Path | None, str]:
    """Compile the spec rlib and receiptcheck for `target` from the given
    source root (default: the tracked checkout)."""
    root = source_root if source_root is not None else HERE.parent
    rlib = work / "libspec.rlib"
    if not rlib.is_file():
        proc = subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
             "--crate-type", "rlib", "--crate-name", "spec", "-o", str(rlib),
             str(root / "spec/lib.rs")], capture_output=True, text=True)
        if proc.returncode != 0:
            return None, "spec rlib did not build: " + "\n".join(
                proc.stderr.splitlines()[:4])
    exe = work / ("sn-receiptcheck" + (".exe" if os.name == "nt" else ""))
    proc = subprocess.run(
        [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
         "--crate-name", "receiptcheck", "--extern", f"spec={rlib}",
         "-o", str(exe), str(root / "bootstrap/receiptcheck.rs")],
        capture_output=True, text=True)
    if proc.returncode != 0 or not exe.is_file():
        return None, "receiptcheck did not build: " + "\n".join(
            proc.stderr.splitlines()[:6])
    return exe, ""


def campaign_verify(rc_exe: Path, bundle: Path, judge_root: Path,
                    envelope: Path, source_commit: str) -> tuple[bool, str]:
    proc = subprocess.run(
        [str(rc_exe), "campaign-verify", str(bundle),
         "--judge-root", str(judge_root), "--envelope", str(envelope),
         "--source-commit", source_commit],
        capture_output=True, text=True)
    return proc.returncode == 0, (proc.stdout + proc.stderr).strip()


def source_commit_of(root: Path) -> str | None:
    proc = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"],
                          capture_output=True, text=True)
    out = proc.stdout.strip()
    return out if proc.returncode == 0 and re.fullmatch(r"[0-9a-f]{40}", out) else None


def neutered_receiptcheck(rustc: str, target: str, work: Path,
                          needle: str, replacement: str) -> tuple[Path | None, str]:
    """Build receiptcheck from a COPY with one campaign guard blinded (the
    two-sided verifier battery). The canonical tree is never written."""
    root = HERE.parent
    copy = work / "rc-neutered-src"
    (copy / "bootstrap" / "receiptcheck").mkdir(parents=True, exist_ok=True)
    (copy / "spec").mkdir(exist_ok=True)
    shutil.copyfile(root / "bootstrap/receiptcheck.rs", copy / "bootstrap/receiptcheck.rs")
    for p in sorted((root / "bootstrap" / "receiptcheck").glob("*.rs")):
        shutil.copyfile(p, copy / "bootstrap" / "receiptcheck" / p.name)
    carrier = None
    for p in sorted((copy / "bootstrap" / "receiptcheck").glob("*.rs")):
        if needle in p.read_text(encoding="utf-8"):
            carrier = p
            break
    if carrier is None:
        return None, f"neuter needle absent from receiptcheck package: {needle!r}"
    src = carrier.read_text(encoding="utf-8")
    mutated = src.replace(needle, replacement, 1)
    if mutated == src:
        return None, "neuter mutation changed no bytes"
    carrier.write_text(mutated, encoding="utf-8", newline="\n")
    # The copy reuses the tracked spec (the guard under neuter is
    # receiptcheck's own, not the spec's).
    for rel in sorted((root / "spec").rglob("*.rs")):
        dst = copy / "spec" / rel.relative_to(root / "spec")
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(rel, dst)
    neuter_work = work / "rc-neutered-build"
    neuter_work.mkdir(exist_ok=True)
    return build_receiptcheck(rustc, target, neuter_work, source_root=copy)

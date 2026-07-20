"""Campaign records, receipts, and the versioned campaign-evidence grammars.

The RECORD side of the mini-supernova rehearsal: content-addressed candidate
records rendered as BATPAK-CANDIDATE-MANIFEST/2 (the spec/campaign/ grammar,
re-implemented here as the PRODUCER; bootstrap/receiptcheck.rs re-parses it
independently and no parser is shared), append-only judge-bound evidence
artifacts, per-candidate BATPAK-CAMPAIGN-RECEIPT/2 receipts beside the
immutable records, the BATPAK-CAMPAIGN-EVIDENCE/2 bundle, the BATPAK-
CAMPAIGN-ENVELOPE/1 rehearsal release envelope (all 20 seal fields; explicit
empty is stated, never omitted), the authoritative-result comparison the
Stability row demands, and the delegation to the independent receiptcheck
campaign verifier.

E7 removed the Python dynamic-semantics maps: the seal-field order/posture
and the realized proof-row profile are PARSED from their spec owners at run
time, never hand-authored here. Nothing here votes on subject semantics:
verdicts come from the witness, and the bundle's admission comes from
receiptcheck's independent recompute.
"""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
from pathlib import Path

from .core import HERE, TOOLCHAIN_EDITION, ProbeError

BUNDLE_MAGIC = "BATPAK-CAMPAIGN-EVIDENCE/2"
ENVELOPE_MAGIC = "BATPAK-CAMPAIGN-ENVELOPE/1"
MANIFEST_MAGIC = "BATPAK-CANDIDATE-MANIFEST/2"
RECEIPT_MAGIC = "BATPAK-CAMPAIGN-RECEIPT/2"

_SEAL_FIELDS_CACHE: tuple[tuple[str, str], ...] | None = None
_PROFILE_CACHE: tuple[str, tuple[str, ...]] | None = None


def seal_fields() -> tuple[tuple[str, str], ...]:
    """The 20 release seal fields in canonical order with their envelope
    shape, parsed INDEPENDENTLY from the two typed owners (E7-D: the
    hand-authored Python mirror is removed): spec/release/inventory.rs owns
    the field order, spec/release/types.rs owns the empty-set posture
    (NotSetValued -> one commitment line; ExplicitEvenWhenEmpty -> explicit
    rows). The two parses must agree in membership; receiptcheck consumes the
    compiled consts, so producer and verifier stay independent."""
    global _SEAL_FIELDS_CACHE
    if _SEAL_FIELDS_CACHE is not None:
        return _SEAL_FIELDS_CACHE
    root = HERE.parent
    inventory = (root / "spec/release/inventory.rs").read_text(encoding="utf-8")
    body = re.search(
        r"pub const RELEASE_SEAL_FIELDS: &\[ReleaseSealField\] = &\[(.*?)\];",
        inventory, re.S)
    if not body:
        raise ProbeError("spec/release/inventory.rs: RELEASE_SEAL_FIELDS unreadable")
    order = re.findall(r"ReleaseSealField::(\w+),", body.group(1))
    types_src = (root / "spec/release/types.rs").read_text(encoding="utf-8")
    matrix = re.search(
        r"pub const fn empty_set_posture\(self\) -> EmptySetPosture \{\s*"
        r"match self \{(.*?)\n        \}", types_src, re.S)
    if not matrix:
        raise ProbeError("spec/release/types.rs: empty_set_posture matrix unreadable")
    posture = dict(re.findall(
        r"ReleaseSealField::(\w+) => EmptySetPosture::(\w+),", matrix.group(1)))
    shape = {"NotSetValued": "commitment", "ExplicitEvenWhenEmpty": "rows"}
    if not order or not posture or len(order) != len(set(order)) \
            or set(order) != set(posture) \
            or any(p not in shape for p in posture.values()):
        raise ProbeError("release seal parses disagree: inventory order vs "
                         "empty-set posture matrix")
    _SEAL_FIELDS_CACHE = tuple((field, shape[posture[field]]) for field in order)
    return _SEAL_FIELDS_CACHE


def mini_supernova_profile() -> tuple[str, tuple[str, ...]]:
    """(profile id, realized proof-row names in profile order), parsed from
    the spec-owned campaign evidence profile (TL-10): spec/campaign/
    inventory.rs MINI_SUPERNOVA_PROFILE. This producer parses the const's
    source text; receiptcheck consumes the compiled const -- the two sides
    stay independent. An empty or duplicated row set is refused."""
    global _PROFILE_CACHE
    if _PROFILE_CACHE is not None:
        return _PROFILE_CACHE
    src = (HERE.parent / "spec/campaign/inventory.rs").read_text(encoding="utf-8")
    const = re.search(
        r"pub const MINI_SUPERNOVA_PROFILE: CampaignEvidenceProfile =\s*"
        r"CampaignEvidenceProfile \{(.*?)\n\};", src, re.S)
    if not const:
        raise ProbeError("spec/campaign/inventory.rs: MINI_SUPERNOVA_PROFILE unreadable")
    profile_id = re.search(r'id: "([^"]+)"', const.group(1))
    rows = re.findall(r'ProofRowId\("(\w+)"\)', const.group(1))
    if not profile_id or not rows:
        raise ProbeError("MINI_SUPERNOVA_PROFILE names no id or no realized rows")
    if len(rows) != len(set(rows)):
        raise ProbeError("MINI_SUPERNOVA_PROFILE names a realized row twice")
    _PROFILE_CACHE = (profile_id.group(1), tuple(rows))
    return _PROFILE_CACHE


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
# Candidate records and the BATPAK-CANDIDATE-MANIFEST/2 render
# ---------------------------------------------------------------------------

def _manifest_body(record: dict) -> str:
    """Every manifest line AFTER the candidate-id line, LF-terminated, in the
    frozen V2 field order with every repeated section canonicalized to
    lexicographic order -- byte-identical to spec/campaign/types.rs render().
    The candidate-id preimage is the /2 domain line plus exactly these bytes,
    so the id stays recomputable from the parsed manifest alone."""
    lines = [f"proof-target-count {len(record['proof_targets'])}"]
    lines.extend(f"proof-target {t}" for t in sorted(record["proof_targets"]))
    lines.append(f"parent-count {len(record['parents'])}")
    lines.extend(f"parent {p}" for p in sorted(record["parents"]))
    lines.append(f"source-frontier-commitment {record['source_frontier']}")
    lines.append(f"dependency-count {len(record['dependencies'])}")
    lines.extend(f"dependency {d} {c}" for d, c in sorted(record["dependencies"]))
    lines.append(f"content-commitment {record['content_commitment']}")
    lines.append(f"origin {record['origin']}")
    lines.append(f"change-class {record['change_class']}")
    lines.append(f"realization-posture {record['posture']}")
    lines.append(f"reuse-key {record['reuse_key']}")
    return "".join(line + "\n" for line in lines)


def mint_record(unit: str, *, proof_targets: list[str], parents: list[str],
                source_frontier: str, dependencies: list[tuple[str, str]],
                content: str, origin: str, change_class: str,
                posture: str) -> dict:
    """One immutable V2 nursery lineage record: mint-time facts ONLY --
    evidence, receipts, and terminals are append-only campaign receipts
    beside the record, never fields of it. The id is the sha256 of the
    candidate-id-preimage/2 (the domain line plus the post-id manifest body);
    the reuse key binds proof targets + content + dependency commitments +
    the spec-owned profile, requalified against upstream churn before reuse.
    `unit` is a diagnostic label, never authority (proof targets are)."""
    if not proof_targets:
        raise ProbeError(f"candidate {unit} names no proof target (manifest law)")
    targets = sorted(proof_targets)
    if len(targets) != len(set(targets)):
        raise ProbeError(f"candidate {unit} names a duplicate proof target")
    content_commitment = sha_hex(content)
    profile_id, profile_rows = mini_supernova_profile()
    profile_hex = sha_hex("".join(row + "\n" for row in profile_rows))
    reuse_preimage = (
        "reuse-key-preimage/2\n"
        + "".join(f"proof-target {t}\n" for t in targets)
        + f"content-commitment {content_commitment}\n"
        + "".join(f"dependency {d} {c}\n" for d, c in sorted(dependencies))
        + f"profile {profile_id} {profile_hex}\n")
    record = {
        "unit": unit,
        "id": "",
        "proof_targets": targets,
        "parents": sorted(parents),
        "source_frontier": source_frontier,
        "dependencies": sorted(dependencies),
        "content_commitment": content_commitment,
        "origin": origin,
        "change_class": change_class,
        "posture": posture,
        "reuse_key": sha_hex(reuse_preimage),
        "frontier": "Unqualified",  # in flight: not admitted on its own yet
        "freshness": "Fresh",       # EvidenceFreshness spelling
        "frontier_reason": "in-flight",
    }
    record["id"] = sha_hex("candidate-id-preimage/2\n" + _manifest_body(record))
    return record


def render_manifest(record: dict) -> str:
    """The canonical BATPAK-CANDIDATE-MANIFEST/2 line grammar: the magic, the
    candidate-id line, then exactly the id-preimage body -- no evidence,
    receipt, or terminal line exists in the V2 grammar."""
    return (f"{MANIFEST_MAGIC}\ncandidate-id {record['id']}\n"
            + _manifest_body(record))


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


def put_receipt(nursery: Path, judge_digest: str, source_frontier: str,
                kind: str, candidate_id: str, lines: list[str]) -> str:
    """One BATPAK-CAMPAIGN-RECEIPT/2 persisted BESIDE the candidate's
    immutable record at nursery/<candidate>/receipts/<sha256-of-bytes>.receipt
    (TL-9), append-only: an existing identical file is lawful, differing
    content at the same address is refused. Every receipt binds the frozen
    judge digest and the current source frontier before its kind-specific and
    free evidence lines."""
    text = "".join(
        line + "\n"
        for line in [RECEIPT_MAGIC, f"kind {kind}", f"candidate {candidate_id}",
                     f"judge {judge_digest}", f"source-frontier {source_frontier}",
                     *lines])
    data = text.encode("utf-8")
    digest = sha_hex(data)
    receipts = nursery / candidate_id / "receipts"
    receipts.mkdir(parents=True, exist_ok=True)
    path = receipts / f"{digest}.receipt"
    if path.exists():
        if path.read_bytes() != data:
            raise ProbeError(f"receipt collision at {path.name}: append-only violated")
        return digest
    path.write_bytes(data)
    return digest


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
    """BATPAK-CAMPAIGN-EVIDENCE/2: sections judge/source/toolchain/policy/
    manifests/candidates/dispositions/frontier/closure, LF-only ASCII. The
    candidates section carries V2 manifests (mint-time facts only); terminals
    appear in the closure section from the receipt-backed terminal map; the
    frontier section speaks the four-state law."""
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
        terminal = c["terminals"].get(record["id"])
        spelling = terminal[0] if terminal else "none"
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
    for field, shape in seal_fields():
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
# candidate rehearsal's. V2 manifests carry mint-time facts only -- no
# receipt hashes -- so the candidates section is FULLY authoritative and is
# compared byte for byte. The policy section stays excluded: its search-
# budget `actual=` values bind measured monotonic ticks, per-run evidence by
# construction. Receipt digests are EXCLUDED for the same reason, measured
# empirically (two same-commit runs diffed): the no-orphan evidence law makes
# a receipt reference the search-receipt artifact, whose bytes bind those
# measured ticks -- so at least one receipt address is per-run evidence, not
# an authoritative result. Receiptcheck independently verifies each run's
# receipt store against its own nursery/evidence perimeter instead.
# ---------------------------------------------------------------------------

def _sections(text: str) -> dict[str, list[str]]:
    lines = text.splitlines()
    if not lines or lines[0] != BUNDLE_MAGIC:
        raise ProbeError("not a BATPAK-CAMPAIGN-EVIDENCE/2 bundle")
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


def compare_authoritative(mine: str, theirs: str) -> list[str]:
    """Every authoritative divergence between two campaign bundles."""
    a, b = _sections(mine), _sections(theirs)
    findings: list[str] = []
    for name in ("judge", "source", "manifests", "candidates", "dispositions",
                 "frontier", "closure"):
        if a.get(name) != b.get(name):
            sa, sb = a.get(name, []), b.get(name, [])
            diff = [f"-{x}" for x in sa if x not in sb] + [f"+{x}" for x in sb if x not in sa]
            findings.append(f"authoritative section {name} diverges: {diff[:4]!r}")
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
                    envelope: Path, source_commit: str, nursery: Path,
                    evidence_root: Path) -> tuple[bool, str]:
    """R4: the verifier receives the bundle AND the exact nursery + evidence
    perimeter it must independently prove (all six coordinates required for a
    V2 bundle)."""
    proc = subprocess.run(
        [str(rc_exe), "campaign-verify", str(bundle),
         "--judge-root", str(judge_root), "--envelope", str(envelope),
         "--source-commit", source_commit, "--nursery-root", str(nursery),
         "--evidence-root", str(evidence_root)],
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

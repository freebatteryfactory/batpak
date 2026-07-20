"""Campaign records, receipts, and the versioned campaign-evidence grammars.

The RECORD side of the mini-supernova rehearsal: content-addressed candidate
records rendered as BATPAK-CANDIDATE-MANIFEST/2 (the spec/campaign/ grammar,
re-implemented here as the PRODUCER; bootstrap/receiptcheck.rs re-parses it
independently and no parser is shared), append-only judge-bound evidence
artifacts, per-candidate BATPAK-CAMPAIGN-RECEIPT/3 receipts beside the
immutable records (the strict ten-kind grammar: counted lex-ordered
sections, typed causes, explicit supersession, no free-form trailing line),
trusted frontier roots (CL-1: a root IS its content commitment), the
BATPAK-CAMPAIGN-EVIDENCE/3 bundle, the BATPAK-CAMPAIGN-ENVELOPE/1 rehearsal
release envelope (all 20 seal fields; explicit empty is stated, never
omitted), the authoritative-result comparison the Stability row demands, and
the delegation to the independent receiptcheck campaign verifier.

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

BUNDLE_MAGIC = "BATPAK-CAMPAIGN-EVIDENCE/3"
ENVELOPE_MAGIC = "BATPAK-CAMPAIGN-ENVELOPE/1"
MANIFEST_MAGIC = "BATPAK-CANDIDATE-MANIFEST/2"
RECEIPT_MAGIC = "BATPAK-CAMPAIGN-RECEIPT/3"

_SEAL_FIELDS_CACHE: tuple[tuple[str, str], ...] | None = None
_PROFILE_CACHE: tuple[str, tuple[str, ...]] | None = None
_PROMOTION_REQ_CACHE: tuple[str, ...] | None = None
_PROOF_PLAN_CACHE: dict[str, list[dict[str, str]]] | None = None


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


def promotion_requirements() -> tuple[str, ...]:
    """The PromotionRequirement spellings in inventory (ALL) order, parsed
    from spec/promotion/types.rs source text (the E7 idiom: Rust owns the
    vocabulary, this producer parses the const, receiptcheck consumes the
    compiled const -- no shared parser, no hand-authored Python mirror). The
    promotion receipt's denominator line states every member =satisfied and
    the verifier RECOMPUTES all four, never trusts them as asserted."""
    global _PROMOTION_REQ_CACHE
    if _PROMOTION_REQ_CACHE is not None:
        return _PROMOTION_REQ_CACHE
    src = (HERE.parent / "spec/promotion/types.rs").read_text(encoding="utf-8")
    body = re.search(
        r"pub const ALL: &'static \[PromotionRequirement\] = &\[(.*?)\];", src, re.S)
    if not body:
        raise ProbeError("spec/promotion/types.rs: PromotionRequirement::ALL unreadable")
    order = re.findall(r"PromotionRequirement::(\w+),", body.group(1))
    spellings = dict(re.findall(r'PromotionRequirement::(\w+) => "(\w+)"', src))
    if not order or len(order) != len(set(order)) or set(order) - set(spellings):
        raise ProbeError("spec/promotion/types.rs: ALL order vs spelling arms disagree")
    _PROMOTION_REQ_CACHE = tuple(spellings[variant] for variant in order)
    return _PROMOTION_REQ_CACHE


def proof_plan(row: str, method: str | None = None,
               route: str | None = None) -> dict[str, str]:
    """The verification-plan tokens a V3 qualification receipt binds for one
    Active proof row: plan name, method, independent route, coverage, and
    lane -- parsed from spec/proof/inventory.rs (PROOF_ROWS plus the named
    PLAN_* consts), never hand-authored here; receiptcheck cross-checks the
    same tokens against the compiled consts. A composite plan (several
    requirements) is selected by `method`. When the requirement's basis
    names an independent route the receipt binds exactly that one (a
    contradicting `route` refuses); when it names none, the caller MUST name
    the admitted route that genuinely supplied independence."""
    global _PROOF_PLAN_CACHE
    if _PROOF_PLAN_CACHE is None:
        src = (HERE.parent / "spec/proof/inventory.rs").read_text(encoding="utf-8")
        plans: dict[str, list[dict[str, str]]] = {}
        for m in re.finditer(
                r"pub const (PLAN_\w+): &\[VerificationRequirement\] = &\[(.*?)\];",
                src, re.S):
            requirements: list[dict[str, str]] = []
            for chunk in m.group(2).split("VerificationRequirement {")[1:]:
                axes = {axis: re.search(rf"{axis}: Verification{owner}::(\w+)", chunk)
                        for axis, owner in (("method", "Method"),
                                            ("coverage", "Coverage"),
                                            ("lane", "Lane"))}
                if not all(axes.values()):
                    raise ProbeError(
                        f"spec/proof/inventory.rs: {m.group(1)} requirement unreadable")
                basis_route = re.search(r"IndependentEvidenceRouteKind::(\w+)",
                                        chunk.split(", coverage:")[0])
                requirements.append({
                    "plan": m.group(1),
                    "method": axes["method"].group(1),
                    "route": basis_route.group(1) if basis_route else "none",
                    "coverage": axes["coverage"].group(1),
                    "lane": axes["lane"].group(1)})
            plans[m.group(1)] = requirements
        rows = re.findall(
            r'ProofRowRecord \{ id: ProofRowId\("(\w+)"\), '
            r"state: ProofRowState::Active \{[^}]*verification: (\w+) \}", src)
        cache = {name: plans[plan] for name, plan in rows if plan in plans}
        if not cache:
            raise ProbeError("spec/proof/inventory.rs: no Active row/plan pair parsed")
        _PROOF_PLAN_CACHE = cache
    requirements = _PROOF_PLAN_CACHE.get(row)
    if not requirements:
        raise ProbeError(f"proof row {row} names no parsed verification plan")
    if method is None and len(requirements) != 1:
        raise ProbeError(f"proof row {row} carries a composite plan; name the method")
    chosen = next((r for r in requirements
                   if method is None or r["method"] == method), None)
    if chosen is None:
        raise ProbeError(f"proof row {row} plan carries no {method} requirement")
    if chosen["route"] == "none":
        if route is None:
            raise ProbeError(
                f"proof row {row} plan names no route; the receipt must name the "
                "admitted route that supplied independence")
        return dict(chosen, route=route)
    if route is not None and route != chosen["route"]:
        raise ProbeError(
            f"proof row {row} plan requires route {chosen['route']}, not {route}")
    return chosen


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


def mint_root(unit: str, content: str) -> dict:
    """One already-trusted frontier root (CL-1): a trusted root IS its
    content, so the root id is the content commitment -- no new preimage
    machinery. Roots are DECLARED in the bundle's roots section and are
    never candidates: no nursery record, no manifest, no generation or
    promotion receipt, no proof-target rows (the architect vetoed decorative
    proof rows); fresh root standing is earned through the reuse receipt's
    requalification evidence. The reuse key binds reuse-key-preimage/2 with
    ZERO proof-target lines, the content commitment, ZERO dependency lines,
    and the profile line."""
    content_commitment = sha_hex(content)
    profile_id, profile_rows = mini_supernova_profile()
    profile_hex = sha_hex("".join(row + "\n" for row in profile_rows))
    reuse_preimage = (
        "reuse-key-preimage/2\n"
        + f"content-commitment {content_commitment}\n"
        + f"profile {profile_id} {profile_hex}\n")
    return {"unit": unit, "id": content_commitment,
            "content_commitment": content_commitment,
            "reuse_key": sha_hex(reuse_preimage)}


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


_TERMINAL_RECEIPT_KINDS = frozenset({"promotion", "refusal", "invalidation",
                                     "escalation"})


def put_receipt(nursery: Path, judge_digest: str, source_frontier: str,
                kind: str, candidate_id: str, lines: list[str],
                supersedes: str | None = None) -> str:
    """One BATPAK-CAMPAIGN-RECEIPT/3 persisted BESIDE the candidate's
    immutable record at nursery/<candidate>/receipts/<sha256-of-bytes>.receipt
    (TL-9), append-only: an existing identical file is lawful, differing
    content at the same address is refused. The strict common header --
    magic, kind, candidate, judge, source-frontier (a CHECKED binding the
    verifier compares against the manifest's source-frontier commitment) --
    then the OPTIONAL supersedes-receipt line (terminal kinds only: explicit
    supersession, never implicit precedence), then EXACTLY the kind-specific
    line set. No free evidence line exists in the /3 grammar."""
    if supersedes is not None and kind not in _TERMINAL_RECEIPT_KINDS:
        raise ProbeError(
            f"supersedes-receipt is lawful only on terminal kinds, not {kind}")
    header = [RECEIPT_MAGIC, f"kind {kind}", f"candidate {candidate_id}",
              f"judge {judge_digest}", f"source-frontier {source_frontier}"]
    if supersedes is not None:
        header.append(f"supersedes-receipt {supersedes}")
    text = "".join(line + "\n" for line in [*header, *lines])
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


# ---------------------------------------------------------------------------
# BATPAK-CAMPAIGN-RECEIPT/3 kind-specific line sets (CL-6): counted sections
# with entries in canonical lexicographic order, explicit empty (`0` count),
# typed causes/reasons, and NOTHING ELSE -- the verifier refuses any unknown
# trailing line, so a /3 receipt is never "a receipt plus notes".
# ---------------------------------------------------------------------------

def _counted(key: str, values: list[str]) -> list[str]:
    return [f"{key}-count {len(values)}"] + [f"{key} {v}" for v in sorted(values)]


def receipt_generation(record: dict, manifest_bytes: bytes, generator: str,
                       generator_commitment: str, evidence: list[str]) -> list[str]:
    """Creation provenance (DEC-079): the causally-first mint receipt,
    exactly one per candidate, never superseded. Binds the digest of the
    exact persisted nursery manifest bytes, the manifest-equal origin /
    content commitment / parent set, the generator identity, and the CL-2
    generator configuration commitment."""
    if record["origin"] == "RepairOfCandidate" and not record["parents"]:
        raise ProbeError("a RepairOfCandidate generation receipt names no parent")
    return ([f"manifest-digest {sha_hex(manifest_bytes)}",
             f"origin {record['origin']}",
             f"generator {generator}",
             f"generator-commitment {generator_commitment}",
             f"content-commitment {record['content_commitment']}"]
            + _counted("parent", record["parents"])
            + _counted("evidence", evidence))


def receipt_qualification(targets: list[str], plan: dict[str, str],
                          evidence: list[str]) -> list[str]:
    """Per-target qualifying evidence: the named proof rows, the spec-owned
    plan tokens (proof_plan -- never hand-authored), the executed
    disposition, freshness, and the persisted evidence artifacts."""
    if not targets or not evidence:
        raise ProbeError("a qualification receipt names >=1 target and >=1 evidence ref")
    return (_counted("proof-target", targets)
            + [f"plan {plan['plan']}", f"method {plan['method']}",
               f"route {plan['route']}", f"coverage {plan['coverage']}",
               f"lane {plan['lane']}",
               "execution compiled=succeeded execution=attempted outcome=passed",
               "freshness Fresh"]
            + _counted("evidence", evidence))


def receipt_holdout(targets: list[str], holdout_set: str,
                    evidence: list[str]) -> list[str]:
    """Holdout evidence on the search-disjoint set (DEC-081), binding the
    exact holdout-set digest and the witness verdict."""
    return (_counted("proof-target", targets)
            + [f"holdout-set {holdout_set}",
               "holdout-disjoint-from-search yes",
               "verdict witness-differential-agree"]
            + _counted("evidence", evidence))


def receipt_fuzz(targets: list[str], seed: int, traces: int, max_ops: int,
                 max_amount: int, traces_executed: int,
                 evidence: list[str]) -> list[str]:
    """The seeded bounded generated-trace attack (R2): exact seed and bounds
    so the confirming run reproduces the identical attack, plus the MEASURED
    executed-trace count."""
    return (_counted("proof-target", targets)
            + [f"seed {seed}", f"traces {traces}", f"max-ops {max_ops}",
               f"max-amount {max_amount}", "held yes",
               f"traces-executed {traces_executed}"]
            + _counted("evidence", evidence))


def receipt_convergence(targets: list[str], iterations: int, bound: int) -> list[str]:
    """The bounded repair loop's convergence to a stable qualified frontier."""
    return (_counted("proof-target", targets)
            + [f"stable-after-iterations {iterations}", f"bound {bound}"])


def receipt_refusal(cause: str, evidence: list[str]) -> list[str]:
    """An own-content refusal with its typed CampaignRefusalCause token."""
    if cause not in ("compile-refusal", "witness-differential-kill",
                     "unrealized-obligation-open"):
        raise ProbeError(f"unknown refusal cause {cause!r}")
    return [f"cause {cause}"] + _counted("evidence", evidence)


def receipt_invalidation(cause: str, coordinate: str, old: str, new: str,
                         evidence: list[str]) -> list[str]:
    """Typed invalidation (CL-4/CL-6): the changed-coordinate cause, the
    exact stale bound coordinate (the dependency commitment the invalidated
    manifest carried), and the candidate-id old/new axis the verifier
    reconciles against the manifests and current frontier."""
    if cause not in ("dependency-commitment-changed", "judge-changed"):
        raise ProbeError(f"unknown invalidation cause {cause!r}")
    return ([f"cause {cause}", f"coordinate {coordinate}",
             f"old {old}", f"new {new}"]
            + _counted("evidence", evidence))


def receipt_escalation(reason: str, evidence: list[str]) -> list[str]:
    """An escalation stopping the campaign; the disposition is ALWAYS
    ArchitectRequired and the reason token is typed (CL-6)."""
    if reason not in ("law-changing-candidate", "judge-defect",
                      "proof-policy-conflict"):
        raise ProbeError(f"unknown escalation reason {reason!r}")
    return ([f"reason {reason}", "disposition ArchitectRequired"]
            + _counted("evidence", evidence))


def receipt_reuse(reused: str, reuse_key: str,
                  dependencies: list[tuple[str, str]], targets: list[str],
                  requalified: dict[str, str]) -> list[str]:
    """A reuse authorization (architect strengthening): binds the reused
    root, its key, the SET-EQUAL current dependency set of the reusing
    manifest, the reused targets, and fresh TARGET-SPECIFIC requalification
    evidence -- the key alone licenses nothing."""
    if not targets or set(requalified) != set(targets):
        raise ProbeError("a reuse receipt requalifies exactly its reused targets")
    return ([f"reused {reused}", f"reuse-key {reuse_key}",
             f"dependency-count {len(dependencies)}"]
            + [f"dependency {d} {c}" for d, c in sorted(dependencies)]
            + _counted("proof-target", targets)
            + [f"requalified-evidence {name} {requalified[name]}"
               for name in sorted(requalified)])


def receipt_promotion(record: dict, target_evidence: dict[str, list[str]],
                      route: str, hostile_evidence: list[str]) -> list[str]:
    """The promotion receipt is the CONSEQUENCE of completed proof, never an
    optimistic bookmark: the SET-EQUAL manifest proof-target set, one
    target-evidence line per target listing every in-store receipt that
    NAMES it, the independent evidence route, the qualified hostile evidence,
    the SET-EQUAL dependency snapshot, and the denominator stated over every
    PromotionRequirement in inventory order. HONEST STOP: a target without a
    naming receipt REFUSES the promotion -- nothing here fabricates."""
    targets = record["proof_targets"]
    missing = sorted(t for t in targets if not target_evidence.get(t))
    foreign = sorted(set(target_evidence) - set(targets))
    if missing or foreign:
        raise ProbeError(
            f"promotion of {record['unit']} refused: proof target(s) without a "
            f"naming receipt {missing}; foreign target-evidence {foreign}")
    if not hostile_evidence:
        raise ProbeError(
            f"promotion of {record['unit']} refused: no qualified hostile evidence")
    lines = _counted("proof-target", targets)
    lines += [f"target-evidence {t} " + " ".join(sorted(target_evidence[t]))
              for t in sorted(targets)]
    lines.append(f"route {route}")
    lines += _counted("hostile-evidence", hostile_evidence)
    lines.append(f"dependency-count {len(record['dependencies'])}")
    lines += [f"dependency {d} {c}" for d, c in sorted(record["dependencies"])]
    lines.append("denominator " + " ".join(
        f"{req}=satisfied" for req in promotion_requirements()))
    return lines


def assemble_promotion(nursery: Path, record: dict,
                       hostile_evidence: list[str]) -> list[str]:
    """Assemble one promotion receipt FROM THE CANDIDATE'S ACTUAL STORE
    (CL-7): collect the un-superseded qualification/holdout/fuzz/convergence
    receipts persisted beside the record, map each manifest proof target to
    the receipt addresses NAMING it, and read the independent route back
    from the referenced receipts -- never asserted freely. A target without
    a naming receipt, or a store whose referenced receipts name no
    independent route, REFUSES (the honest stop)."""
    superseded: set[str] = set()
    parsed: list[tuple[str, dict[str, list[str]]]] = []
    for path in sorted((nursery / record["id"] / "receipts").glob("*.receipt")):
        fields: dict[str, list[str]] = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            key, _sep, value = line.partition(" ")
            fields.setdefault(key, []).append(value)
        superseded.update(fields.get("supersedes-receipt", []))
        parsed.append((path.stem, fields))
    naming: dict[str, list[str]] = {}
    routes: dict[str, str] = {}
    for digest, fields in parsed:
        if digest in superseded or fields.get("kind", [""])[0] not in (
                "qualification", "holdout", "fuzz", "convergence"):
            continue
        if fields.get("route"):
            routes[digest] = fields["route"][0]
        for name in fields.get("proof-target", []):
            naming.setdefault(name, []).append(digest)
    per_target = {t: naming.get(t, []) for t in record["proof_targets"]}
    named_routes = sorted({routes[d] for refs in per_target.values()
                           for d in refs if d in routes})
    if not named_routes:
        raise ProbeError(f"promotion of {record['unit']} refused: no referenced "
                         "receipt names an independent route")
    return receipt_promotion(record, per_target, named_routes[0], hostile_evidence)


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
    """BATPAK-CAMPAIGN-EVIDENCE/3: sections judge/source/toolchain/policy/
    manifests/roots/candidates/dispositions/frontier/closure (ten), LF-only
    ASCII. The roots section DECLARES the trusted frontier inputs (CL-1:
    root id = content commitment; the reuse receipt carries the proof); the
    candidates section carries V2 manifests (mint-time facts only);
    terminals appear in the closure section from the receipt-backed terminal
    map; the frontier section speaks the four-state law."""
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
    lines.append("section: roots")
    lines.append(f"root-count {len(c['roots'])}")
    for root in c["roots"]:
        lines.append(f"root {root['id']} content-commitment "
                     f"{root['content_commitment']} reuse-key {root['reuse_key']}")
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
        raise ProbeError("not a BATPAK-CAMPAIGN-EVIDENCE/3 bundle")
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
    for name in ("judge", "source", "manifests", "roots", "candidates",
                 "dispositions", "frontier", "closure"):
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


def verify_bundle(paths: dict, work: Path) -> tuple[list[str], str]:
    """Build the REAL receiptcheck and block on its campaign-verify verdict
    (shared by the battery and the CLI; the module graph stays a DAG)."""
    rc, err = build_receiptcheck(paths["rustc"], paths["target_triple"], work)
    if rc is None:
        return [f"receiptcheck unavailable for campaign-verify: {err}"], ""
    ok, out = campaign_verify(rc, paths["bundle"], paths["judge"],
                              paths["envelope"], paths["source_commit"],
                              paths["nursery"], paths["evidence_root"])
    if not ok:
        return ["receiptcheck REFUSED the campaign bundle:\n" + out], out
    return [], out


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

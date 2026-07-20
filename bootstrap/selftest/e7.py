"""The E7 underwriting artifact: producer, self-verification, and cross-run.

BATPAK-E7-UNDERWRITING/2 (E7 closeout, CL-8) is the executable opening
matrix: one versioned artifact, deliberately OUTSIDE the Tier 0 receipt
denominator, produced fresh by every run. It binds the exact source (commit,
manifested tree, SPEC.sha256, workflow, toolchain, Python runtime) and this
run's fresh evidence (the Tier 0 qualification bundle and the
CampaignEvidenceV3 bundle), and carries the twenty named zero rows (TL-5:
kebab-case, frozen order). Every zero is COMPUTED from a named basis -- an
independent Python parse of the spec sources, a fresh execution in this run,
or the independent `receiptcheck campaign-verify` recompute -- never
asserted. A nonzero count is rendered honestly and fails verification by
name; a row that cannot be derived stops the underwrite instead of printing
an uncomputed zero.

CL-8 moves every zero row behind an INDEPENDENTLY RECEIPTED OWNER: each row
renders `zero <token> <n> owner <owner> receipt <64hex>`. The owner is the
enforcing authority (per the census map): `tier0` for the rows the Tier 0
qualification bundle already proves (receipt = sha256 of that bundle's
`qualification.t0`), `audit` / `project-check` for the rows the freshly
executed audit / project gates own (receipt = sha256 of the produced
BATPAK-OWNER-RECEIPT/1 beside the artifact), and `campaign-verify` for the
rows the independent receiptcheck campaign recompute owns. The producer runs
audit / project / campaign-verify FRESH here and WRAPS those executions into
owner receipts (zero owner-file edits); the Rust `e7-verify` verifies the
exact independently produced receipts and no longer accepts a literal zero
without a receipt-backed owner. The artifact carries `cross-run-stability
pending` in BOTH the candidate and confirming runs (no time travel); the
Phase-6 eligibility banner is spoken ONLY by `receiptcheck e7-open`, gated on
both artifacts plus the cross-run stability receipt.

Documented digest bases (receiptcheck e7-verify recomputes both):
* source-tree: for each SPEC.sha256 row the file's content hash is
  RECOMPUTED from the bytes on disk and rendered back as
  ``<sha256>  <relpath>`` in manifest order; the digest is the SHA-256 of
  that text -- the manifested file set, bound by bytes, not by claim.
* package-graph: SHA-256 over ``<relpath>\\t<sha256>`` rows (sorted by
  relpath, LF-terminated each) of every non-root Cargo.toml inside the bound
  Tier 0 bundle's gate0-candidate tree -- the nine materialized package
  manifests. The Rust side additionally pins the set to the compiled
  spec::architecture::PACKAGES workspace paths.

PER-RUN bindings: tier0-bundle and campaign-bundle. The Tier 0 bundle embeds
run metadata and freshly linked executables; the campaign bundle's search
receipt binds measured monotonic ticks. Each is verified within its own run
and never compared across runs; every other line is AUTHORITATIVE and the
comparator refuses any divergence.

TL-6 (row 20): escalation receipts across the campaign nursery are counted
against the single expected rehearsal-fixture escalation, identified by the
one LawChanging candidate manifest; any other escalation receipt, or any
additional LawChanging candidate, counts.

TL-12: Python announces NO semantic completeness. The artifact's
architect-semantic-row line stays ``reserved-to-architect``; Python never
prints ``phase6-opening-eligible`` at all. That banner belongs to
``receiptcheck e7-open`` (the sole printer), speaks only for the twenty
mechanical rows across two runs, and the final semantic disposition is the
architect's alone.
"""
from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

from . import supernova_bundle as sb
from .core import HERE, ProbeError
from .supernova_bundle import _sections as bundle_sections

E7_MAGIC = "BATPAK-E7-UNDERWRITING/2"
ARTIFACT_NAME = "e7-underwriting.t0"
ARCHITECT_LINE = "architect-semantic-row reserved-to-architect"
CROSSRUN_PENDING = "cross-run-stability pending"
OWNER_RECEIPT_MAGIC = "BATPAK-OWNER-RECEIPT/1"
STABILITY_MAGIC = "BATPAK-CROSSRUN-STABILITY-RECEIPT/1"
# The owner receipt owners the census grammar admits (crossrun-compare is
# reserved for the cross-run stability receipt's producer; the twenty zero
# rows use only the first three plus the receipt-less `tier0` owner).
OWNER_TOKENS = ("audit", "project-check", "campaign-verify", "crossrun-compare")

# TL-5: the architect's twenty matrix rows, kebab-cased, in this frozen order.
ZERO_TOKENS = (
    "active-proof-rows-without-claim-or-plan",
    "adopter-less-verification-variants",
    "variants-lacking-fresh-executed-adopter-evidence",
    "generated-models-claiming-independence",
    "unbounded-candidate-searches",
    "incomplete-candidate-lineage",
    "unresolved-evidence-references",
    "search-holdout-overlaps",
    "runtime-monitors-with-rewrite-authority",
    "lawchanging-candidates-in-preserving-lanes",
    "qualified-nursery-records-treated-as-promoted",
    "external-tools-acting-as-semantic-owners",
    "python-dynamic-semantics-maps",
    "unregistered-generated-views",
    "release-seal-omissions",
    "package-topology-changes",
    "unclassified-dec-seed-proof-rows",
    "scaffolds-counted-realized",
    "later-green-above-unresolved-red",
    "unresolved-architect-required-findings",
)

# Artifact binding lines, in wire order.
BIND_KEYS = ("source-commit", "source-tree", "spec-manifest", "workflow-digest",
             "toolchain", "python", "tier0-bundle", "campaign-bundle",
             "materializer-output-tree", "package-graph",
             "generated-view-registry", "release-envelope")

# Cross-run classification: everything is AUTHORITATIVE except the two
# per-run evidence bindings named above the imports.
PER_RUN_KEYS = ("tier0-bundle", "campaign-bundle")
AUTHORITATIVE_KEYS = tuple(k for k in BIND_KEYS if k not in PER_RUN_KEYS)

# CL-8 owner map: for each of the twenty rows (0-based, frozen TL-5 order),
# the enforcing owner whose independently produced receipt backs the zero.
# `tier0` rows are proved by the bound Tier 0 qualification bundle (receipt =
# sha256 of its qualification.t0); `audit` / `project-check` / `campaign-verify`
# rows are proved by the freshly executed gate wrapped into a
# BATPAK-OWNER-RECEIPT/1 beside the artifact.
ROW_OWNERS = (
    "tier0",           # 1  active-proof-rows-without-claim-or-plan
    "tier0",           # 2  adopter-less-verification-variants
    "campaign-verify", # 3  variants-lacking-fresh-executed-adopter-evidence
    "audit",           # 4  generated-models-claiming-independence
    "campaign-verify", # 5  unbounded-candidate-searches
    "campaign-verify", # 6  incomplete-candidate-lineage
    "campaign-verify", # 7  unresolved-evidence-references
    "campaign-verify", # 8  search-holdout-overlaps
    "tier0",           # 9  runtime-monitors-with-rewrite-authority
    "campaign-verify", # 10 lawchanging-candidates-in-preserving-lanes
    "campaign-verify", # 11 qualified-nursery-records-treated-as-promoted
    "audit",           # 12 external-tools-acting-as-semantic-owners
    "tier0",           # 13 python-dynamic-semantics-maps
    "project-check",   # 14 unregistered-generated-views
    "campaign-verify", # 15 release-seal-omissions
    "audit",           # 16 package-topology-changes
    "audit",           # 17 unclassified-dec-seed-proof-rows
    "campaign-verify", # 18 scaffolds-counted-realized
    "campaign-verify", # 19 later-green-above-unresolved-red
    "campaign-verify", # 20 unresolved-architect-required-findings
)

# The single profile row with no candidate carrier. It is realized at the
# CROSS-RUN level: its carrier is the `cross-run-stability` line itself, which
# both runs render `pending` and which `receiptcheck e7-open` closes only
# after the two runs' authoritative results are proven identical (there is no
# time travel inside one run). It is therefore excluded from row-3 scope, not
# silently skipped for lack of a carrier -- see _zero_fresh_adopters.
CAMPAIGN_LEVEL_ROW = "confirming_rerun_changes_no_authoritative_result"

_BUDGET_AXES = ("max-candidates", "max-logical-work", "max-memory-bytes",
                "max-monotonic-ticks")


# ---------------------------------------------------------------------------
# Independent spec-source parses (basis a)
# ---------------------------------------------------------------------------

def _read(rel: str) -> str:
    return (HERE.parent / rel).read_text(encoding="utf-8")


def _const_block(src: str, name: str, where: str) -> str:
    body = re.search(rf"pub const {name}: [^=]+= &\[(.*?)\];", src, re.S)
    if not body:
        raise ProbeError(f"{where}: const {name} unreadable")
    return body.group(1)


def _proof_rows() -> tuple[int, dict[str, str], dict[str, str]]:
    """Row 1 and the raw material for row 2: every PROOF_ROWS record parsed
    independently. Returns (violations, active row -> plan name, active row
    -> claim kind); an Active row missing its claim or its verification plan
    is a violation, never a default."""
    body = _const_block(_read("spec/proof/inventory.rs"), "PROOF_ROWS",
                        "spec/proof/inventory.rs")
    entries = re.findall(
        r'ProofRowRecord \{ id: ProofRowId\("(\w+)"\), '
        r"state: ProofRowState::(\w+) \{(.*?)\} \},", body)
    if not entries:
        raise ProbeError("spec/proof/inventory.rs: no PROOF_ROWS records parsed")
    violations = 0
    plans: dict[str, str] = {}
    claims: dict[str, str] = {}
    for row, state, fields in entries:
        if state != "Active":
            continue
        claim = re.search(r"claim: VerificationClaimKind::(\w+)", fields)
        plan = re.search(r"verification: (PLAN_\w+)", fields)
        if not claim or not plan:
            violations += 1
            continue
        plans[row] = plan.group(1)
        claims[row] = claim.group(1)
    return violations, plans, claims


def _verification_vocab() -> dict[str, list[str]]:
    src = _read("spec/verification/types.rs")
    out: dict[str, list[str]] = {}
    for axis, const, ty in (
            ("methods", "VERIFICATION_METHODS", "VerificationMethod"),
            ("coverages", "VERIFICATION_COVERAGES", "VerificationCoverage"),
            ("lanes", "VERIFICATION_LANES", "VerificationLane"),
            ("claims", "VERIFICATION_CLAIM_KINDS", "VerificationClaimKind"),
            ("routes", "INDEPENDENT_EVIDENCE_ROUTE_KINDS",
             "IndependentEvidenceRouteKind")):
        names = re.findall(rf"{ty}::(\w+)",
                           _const_block(src, const, "spec/verification/types.rs"))
        if not names:
            raise ProbeError(f"spec/verification/types.rs: {const} names no variant")
        out[axis] = names
    return out


def _plan_axes() -> dict[str, dict[str, set[str]]]:
    src = _read("spec/proof/inventory.rs")
    out: dict[str, dict[str, set[str]]] = {}
    for name, body in re.findall(
            r"pub const (PLAN_\w+): &\[VerificationRequirement\] = &\[(.*?)\];",
            src, re.S):
        out[name] = {
            "methods": set(re.findall(r"VerificationMethod::(\w+)", body)),
            "coverages": set(re.findall(r"VerificationCoverage::(\w+)", body)),
            "lanes": set(re.findall(r"VerificationLane::(\w+)", body)),
            "routes": set(re.findall(r"IndependentEvidenceRouteKind::(\w+)", body)),
        }
    if not out:
        raise ProbeError("spec/proof/inventory.rs: no verification plans parsed")
    return out


def _zero_adopterless(plans: dict[str, str], claims: dict[str, str]) -> int:
    """Row 2 (basis a): the seedcheck adoption loops, mirrored in regex form.
    Every method / coverage / lane / route variant must be consumed by at
    least one Active row's plan; every claim kind must be some Active row's
    primary claim."""
    vocab = _verification_vocab()
    axes = _plan_axes()
    adopted: dict[str, set[str]] = {"methods": set(), "coverages": set(),
                                    "lanes": set(), "routes": set()}
    for row, plan in plans.items():
        if plan not in axes:
            raise ProbeError(f"active row {row} names unparsed plan {plan}")
        for axis in adopted:
            adopted[axis] |= axes[plan][axis]
    violations = 0
    for axis in ("methods", "coverages", "lanes", "routes"):
        violations += sum(1 for v in vocab[axis] if v not in adopted[axis])
    violations += sum(1 for v in vocab["claims"] if v not in set(claims.values()))
    return violations


def _registry_rows() -> tuple[int, int]:
    """Rows 4 and 12 (basis a): every GeneratedView arm declares nonempty
    authority_sources (a generated model claiming independence declares
    none), and every declared source points into spec/ (an external tool
    acting as a semantic owner points elsewhere)."""
    src = _read("spec/generated_views/registry.rs")
    arms = src.count("=> GeneratedViewSpec {")
    blocks = re.findall(r"authority_sources: &\[(.*?)\]", src, re.S)
    if arms == 0 or len(blocks) != arms:
        raise ProbeError("spec/generated_views/registry.rs: authority_sources "
                         f"blocks ({len(blocks)}) do not cover the {arms} arms")
    empty = 0
    foreign = 0
    for block in blocks:
        sources = re.findall(r'"([^"]+)"', block)
        if not sources:
            empty += 1
        foreign += sum(1 for s in sources if not s.startswith("spec/"))
    return empty, foreign


def _zero_monitor_rewrite() -> int:
    """Row 9 (basis a): runtime monitors are observe-only by type. The
    in-band observation arm of the admits matrix may state observations only;
    a disposition that concludes conformance or admits/refuses (guard
    authority) is rewrite authority in a monitor's hands."""
    src = _read("spec/verification/types.rs")
    arm = re.search(
        r"RuntimeVerificationMode::InBandObservation => matches!\((.*?)\)",
        src, re.S)
    if not arm:
        raise ProbeError("spec/verification/types.rs: the InBandObservation "
                         "admits arm is unreadable")
    admitted = set(re.findall(
        r"(?:D|RuntimeVerificationDisposition)::(\w+)", arm.group(1)))
    if not admitted:
        raise ProbeError("spec/verification/types.rs: the InBandObservation "
                         "arm admits no parsed disposition")
    authority = {"ConformantForObservedHistory", "GuardAdmitted", "GuardRefused"}
    return len(admitted & authority)


def _zero_python_maps() -> int:
    """Row 13 (basis a): the retired dynamic-semantics map shapes must not
    reappear anywhere under bootstrap/. The needles are assembled from parts
    so this scanner never matches its own source."""
    seal = "SEAL_" "FIELDS"
    realized = "REALIZED_" "PROOF_ROWS"
    pattern = re.compile(
        rf"^\s*(?:{seal}|{realized})\s*(?::[^=\n]+)?=\s*[(\[]", re.M)
    hits = 0
    for path in sorted(HERE.rglob("*.py")):
        if "__pycache__" in path.parts:
            continue
        hits += len(pattern.findall(path.read_text(encoding="utf-8")))
    return hits


def _package_census() -> int:
    body = _const_block(_read("spec/architecture/inventory.rs"), "PACKAGES",
                        "spec/architecture/inventory.rs")
    census = len(re.findall(r"id: PackageId::(\w+),", body))
    if census == 0:
        raise ProbeError("spec/architecture/inventory.rs: PACKAGES parsed empty")
    return census


# ---------------------------------------------------------------------------
# Campaign perimeter parses (raw material for the bundle-derived rows)
# ---------------------------------------------------------------------------

def _one_field(body: list[str], key: str, cid: str) -> str:
    values = [line.removeprefix(key + " ")
              for line in body if line.startswith(key + " ")]
    if len(values) != 1:
        raise ProbeError(f"candidate {cid[:12]} manifest has {len(values)} "
                         f"{key} lines, not one")
    return values[0]


def _candidate_blocks(section: list[str]) -> list[dict]:
    if not section or not section[0].startswith("candidate-count "):
        raise ProbeError("bundle candidates section has no candidate-count")
    declared = int(section[0].removeprefix("candidate-count "))
    out: list[dict] = []
    i = 1
    while i < len(section):
        if not section[i].startswith("candidate-begin "):
            raise ProbeError(f"expected candidate-begin, found {section[i]!r}")
        cid = section[i].removeprefix("candidate-begin ").split(" ")[0]
        i += 1
        body: list[str] = []
        while i < len(section) and not section[i].startswith("candidate-end "):
            body.append(section[i])
            i += 1
        if i == len(section):
            raise ProbeError(f"candidate {cid[:12]} has no candidate-end")
        i += 1
        if body[:2] != [sb.MANIFEST_MAGIC, f"candidate-id {cid}"]:
            raise ProbeError(f"candidate {cid[:12]} embeds a malformed manifest")
        out.append({
            "id": cid,
            "lines": body,
            "targets": [line.removeprefix("proof-target ")
                        for line in body if line.startswith("proof-target ")],
            "deps": [tuple(line.split(" ")[1:3])
                     for line in body if line.startswith("dependency ")],
            "content": _one_field(body, "content-commitment", cid),
            "change_class": _one_field(body, "change-class", cid),
            "posture": _one_field(body, "realization-posture", cid),
            "reuse_key": _one_field(body, "reuse-key", cid),
        })
    if len(out) != declared:
        raise ProbeError(f"candidate-count {declared} does not match "
                         f"{len(out)} embedded manifests")
    return out


def _root_ids(section: list[str]) -> set[str]:
    """CL-1: the trusted frontier roots declared in the bundle's `roots`
    section. A root IS its content commitment; it is Qualified by definition
    and carries no nursery record, so the E7 recomputations resolve
    dependency and dependency-first laws against candidate ids UNION roots."""
    out: set[str] = set()
    for line in section:
        if line.startswith("root ") and not line.startswith("root-count "):
            out.add(line.split(" ")[1])
    return out


def _frontier_map(section: list[str]) -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    for line in section:
        tokens = line.split(" ")
        if tokens[0] != "frontier" or len(tokens) < 3:
            raise ProbeError(f"unexpected frontier line {line!r}")
        out[tokens[1]] = dict(t.split("=", 1) for t in tokens[2:] if "=" in t)
    return out


def _closure_terminals(section: list[str]) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in section:
        if not line.startswith("node "):
            continue
        tokens = line.split(" ")
        kv = dict(t.split("=", 1) for t in tokens[2:] if "=" in t)
        out[tokens[1]] = kv.get("terminal", "none")
    return out


def _nursery_records(nursery: Path) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for entry in sorted(nursery.iterdir()):
        if not entry.is_dir():
            raise ProbeError(f"nursery carries a non-directory entry {entry.name}")
        manifest_path = entry / "manifest"
        manifest = (manifest_path.read_text(encoding="utf-8")
                    if manifest_path.is_file() else None)
        receipts: list[dict] = []
        receipts_dir = entry / "receipts"
        if receipts_dir.is_dir():
            for f in sorted(receipts_dir.glob("*.receipt")):
                lines = f.read_text(encoding="utf-8").splitlines()
                kind = lines[1].removeprefix("kind ") if len(lines) > 1 else ""
                # RECEIPT/3: `evidence <64hex>` and the reuse receipt's
                # `requalified-evidence <name> <64hex>` both carry the artifact
                # digest as their LAST token.
                refs = [line.split(" ")[-1] for line in lines
                        if line.startswith(("evidence ", "requalified-evidence "))]
                targets = [line.removeprefix("proof-target ") for line in lines
                           if line.startswith("proof-target ")]
                receipts.append({"kind": kind, "refs": refs, "targets": targets})
        out[entry.name] = {"manifest": manifest, "receipts": receipts}
    return out


# ---------------------------------------------------------------------------
# The campaign-derived zero rows (bases b and c on top of the independent
# receiptcheck campaign-verify PASS the underwrite blocks on first)
# ---------------------------------------------------------------------------

# The executed-evidence receipt kinds a fresh adopter row rides on: a
# candidate that carries a proof target and was ADOPTED (promoted into the
# trusted frontier) must have one of these NAMING that target. Generation is
# mint provenance; the terminal kinds (refusal/invalidation/escalation) and
# the promotion receipt itself are the terminal being justified, not the
# fresh executed evidence -- so they never satisfy row 3.
_FRESH_ADOPTER_KINDS = frozenset({"qualification", "holdout", "fuzz",
                                  "convergence", "reuse"})


def _zero_fresh_adopters(cands: list[dict], recs: dict[str, dict],
                         sections: dict[str, list[str]], qual_text: str,
                         profile_rows: tuple[str, ...],
                         terminals: dict[str, str]) -> int:
    """Row 3: the E7-D stronger closure, scoped to this run's fresh
    executions and tightened by Commit B's per-target naming law. Chain:
    declared variant -> active ADOPTER row -> fresh executed receipt NAMING
    that target. The first hop is executed law in the bound Tier 0 bundle
    (seedcheck runs the adoption loops; its executed+passed receipt is
    required here) and independently recomputed as row 2. The second hop is
    the closeout repair of the old any-receipt bug: a target counts as
    freshly executed only when a receipt in an ADOPTED carrier's store NAMES
    it (a qualification/holdout/fuzz/convergence/reuse receipt carrying a
    matching proof-target line) -- ANY receipt no longer suffices, and a
    refusal/invalidation/escalation/promotion/generation receipt no longer
    counts. A carrier is adopted when its closure terminal is Promoted; a row
    whose carriers were refused, invalidated, or escalated is realized by
    those visible terminals (the closure completeness sweep below), not by
    fresh adopter evidence, so it is not a fresh-adopter obligation.

    The one ruled campaign-level row (CAMPAIGN_LEVEL_ROW) is EXCLUDED from
    scope, not skipped for lack of a carrier: its carrier is the cross-run-
    stability line, closed by receiptcheck e7-open across two runs, never by
    a single-run receipt. Its presence in the spec-owned profile is still
    required (a missing profile entry is one violation). The rehearsal's
    disposition loci (mutant, fuzz, model, runtime conformance, closure
    terminals) must each be present in the bundle."""
    violations = 0
    carriers: dict[str, list[str]] = {}
    for c in cands:
        for target in c["targets"]:
            carriers.setdefault(target, []).append(c["id"])
    scope = list(dict.fromkeys(list(profile_rows) + sorted(carriers)))
    if CAMPAIGN_LEVEL_ROW not in profile_rows:
        violations += 1

    def names_target(cid: str, row: str) -> bool:
        return any(r["kind"] in _FRESH_ADOPTER_KINDS and row in r["targets"]
                   for r in recs.get(cid, {}).get("receipts", []))

    for row in scope:
        if row == CAMPAIGN_LEVEL_ROW:
            continue
        adopted = [cid for cid in carriers.get(row, [])
                   if terminals.get(cid) == "Promoted"]
        # Only an ADOPTED variant owes fresh executed adopter evidence; a
        # non-adopted carrier is realized by its terminal disposition below.
        if adopted and not any(names_target(cid, row) for cid in adopted):
            violations += 1
    disp = sections["dispositions"]
    if not any(line.startswith("mutant ") and "activated=yes" in line
               and "killed=yes" in line for line in disp):
        violations += 1
    if not any(line.startswith("model-disposition ") for line in disp):
        violations += 1
    if not any(line.startswith("runtime-conformance-disposition ")
               and "conformant-for-observed-history" in line for line in disp):
        violations += 1
    if not any(line.startswith("fuzz ") for line in sections["policy"]):
        violations += 1
    violations += sum(1 for line in sections["closure"]
                      if line.startswith("node ") and "terminal=none" in line)
    for slug in ("tier0-seedcheck", "tier0-seedcheck-tests", "tier0-spec-tests"):
        if not re.search(rf"^receipt: {slug} compiled=succeeded "
                         r"execution=attempted outcome=passed", qual_text, re.M):
            violations += 1
    return violations


def _zero_unbounded(policy: list[str]) -> int:
    violations = 0
    for axis in _BUDGET_AXES:
        line = next((ln for ln in policy
                     if ln.startswith(f"search-budget {axis} ")), None)
        bound = line and re.search(r"declared=(\d+) actual=(\d+)$", line)
        if not bound or int(bound.group(2)) > int(bound.group(1)):
            violations += 1
    fuzz = next((ln for ln in policy if ln.startswith("fuzz ")), "")
    for key in ("seed=", "traces=", "max-ops="):
        if not re.search(rf"{key}\d+", fuzz):
            violations += 1
    return violations


def _zero_lineage(cands: list[dict], recs: dict[str, dict],
                  profile: tuple[str, tuple[str, ...]],
                  root_ids: set[str]) -> int:
    """Row 6: bundle<->nursery bijection, candidate-id and reuse-key
    recompute from the normative /2 preimages, and dependency closure --
    each divergence one violation (unknown dependency = incomplete lineage;
    parent edges stay derived by the verifier, mirroring its law). CL-1: a
    dependency on a declared trusted root is complete lineage, so the closure
    resolves against candidate ids UNION roots; nursery records stay
    candidates only (a root carries none)."""
    violations = 0
    ids = {c["id"] for c in cands}
    dep_targets = ids | root_ids
    profile_id, profile_rows = profile
    profile_hex = sb.sha_hex("".join(row + "\n" for row in profile_rows))
    for c in cands:
        rec = recs.get(c["id"])
        rendered = "".join(line + "\n" for line in c["lines"])
        if not rec or rec["manifest"] != rendered:
            violations += 1
        body = "".join(line + "\n" for line in c["lines"][2:])
        if sb.sha_hex("candidate-id-preimage/2\n" + body) != c["id"]:
            violations += 1
        preimage = ("reuse-key-preimage/2\n"
                    + "".join(f"proof-target {t}\n" for t in c["targets"])
                    + f"content-commitment {c['content']}\n"
                    + "".join(f"dependency {a} {b}\n" for a, b in c["deps"])
                    + f"profile {profile_id} {profile_hex}\n")
        if sb.sha_hex(preimage) != c["reuse_key"]:
            violations += 1
        violations += sum(1 for dep, _c in c["deps"] if dep not in dep_targets)
    violations += sum(1 for name in recs if name not in ids)
    return violations


def _zero_evidence_refs(recs: dict[str, dict], evidence: Path) -> int:
    by_stem: dict[str, list[Path]] = {}
    for path in sorted(evidence.iterdir()):
        if path.is_file():
            by_stem.setdefault(path.name.split(".", 1)[0], []).append(path)
    violations = 0
    for rec in recs.values():
        for receipt in rec["receipts"]:
            for ref in receipt["refs"]:
                matches = by_stem.get(ref, [])
                if len(matches) != 1 \
                        or sb.sha_hex(matches[0].read_bytes()) != ref:
                    violations += 1
    return violations


def _zero_overlap(judge: Path) -> int:
    def canon(text: str) -> set[str]:
        return {"\n".join(line.strip() for line in t.strip().splitlines())
                for t in text.split("----") if t.strip()}
    search = canon((judge / "search.vectors").read_text(encoding="utf-8"))
    holdout = canon((judge / "holdout.vectors").read_text(encoding="utf-8"))
    return len(search & holdout)


def _zero_lawchanging(cands: list[dict], recs: dict[str, dict],
                      frontier: dict[str, dict[str, str]]) -> int:
    violations = 0
    for c in cands:
        if c["change_class"] != "LawChanging":
            continue
        kinds = {r["kind"] for r in recs.get(c["id"], {}).get("receipts", [])}
        if "escalation" not in kinds:
            violations += 1
        if "promotion" in kinds:
            violations += 1
        if frontier.get(c["id"], {}).get("state") != "Unqualified":
            violations += 1
    return violations


def _zero_qualified_promoted(cands: list[dict], recs: dict[str, dict],
                             frontier: dict[str, dict[str, str]],
                             terminals: dict[str, str]) -> int:
    violations = 0
    for c in cands:
        kinds = {r["kind"] for r in recs.get(c["id"], {}).get("receipts", [])}
        if terminals.get(c["id"]) == "Promoted" and "promotion" not in kinds:
            violations += 1
        if frontier.get(c["id"], {}).get("state") == "Qualified" \
                and "promotion" not in kinds:
            violations += 1
    return violations


def _zero_envelope(envelope: Path) -> int:
    """Row 15: the 20-field envelope walk against the spec-parsed field order
    and empty-set postures (the independent campaign-verify re-proves the
    same walk from the compiled consts)."""
    lines = envelope.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != sb.ENVELOPE_MAGIC:
        return 1
    violations = 0
    pos = 1
    for field, shape in sb.seal_fields():
        if pos >= len(lines) or not lines[pos].startswith(f"seal {field} "):
            return violations + 1
        rest = lines[pos].removeprefix(f"seal {field} ")
        pos += 1
        if shape == "commitment":
            if not re.fullmatch(r"commitment [0-9a-f]{64}", rest):
                violations += 1
            continue
        rows = re.fullmatch(r"rows (\d+)", rest)
        if not rows:
            return violations + 1
        for _ in range(int(rows.group(1))):
            if pos >= len(lines) or not lines[pos].startswith(f"row {field} "):
                return violations + 1
            pos += 1
    if pos != len(lines):
        violations += 1
    return violations


def _zero_scaffolds(cands: list[dict], frontier: dict[str, dict[str, str]],
                    terminals: dict[str, str]) -> int:
    violations = 0
    for c in cands:
        if c["posture"] != "Scaffold":
            continue
        if frontier.get(c["id"], {}).get("state") == "Qualified":
            violations += 1
        if terminals.get(c["id"]) == "Promoted":
            violations += 1
    return violations


def _zero_dependency_first(cands: list[dict],
                           frontier: dict[str, dict[str, str]],
                           root_ids: set[str]) -> int:
    violations = 0
    for c in cands:
        if frontier.get(c["id"], {}).get("state") != "Qualified":
            continue
        # CL-1: a declared trusted root is Qualified by definition.
        violations += sum(
            1 for dep, _c in c["deps"]
            if dep not in root_ids
            and frontier.get(dep, {}).get("state") != "Qualified")
    return violations


def _zero_architect(recs: dict[str, dict]) -> int:
    """Row 20 (TL-6): the single expected rehearsal-fixture escalation is the
    one on the single LawChanging candidate; any other escalation receipt,
    or any additional LawChanging candidate, counts."""
    law_changing = [cid for cid, rec in recs.items()
                    if rec["manifest"]
                    and re.search(r"^change-class LawChanging$",
                                  rec["manifest"], re.M)]
    planted = law_changing[0] if len(law_changing) == 1 else None
    violations = max(0, len(law_changing) - 1)
    for cid, rec in recs.items():
        violations += sum(1 for r in rec["receipts"]
                          if r["kind"] == "escalation" and cid != planted)
    return violations


# ---------------------------------------------------------------------------
# Fresh gate executions (basis b)
# ---------------------------------------------------------------------------

def _fresh_gate(args: list[str]) -> tuple[int, str]:
    proc = subprocess.run([sys.executable, "-I", *args], capture_output=True,
                          text=True, cwd=str(HERE.parent))
    return proc.returncode, (proc.stdout + proc.stderr).strip()


# ---------------------------------------------------------------------------
# Owner receipts (CL-8): each freshly executed gate is WRAPPED into an
# independently verifiable BATPAK-OWNER-RECEIPT/1 beside the artifact, with
# ZERO edits to the owner tools themselves. `tool` binds the enforcing tool's
# bytes (audit/project: the sha256 over the sorted `<rel>\t<sha256>` rows of
# the owner package files; campaign-verify: the built receiptcheck exe -- a
# per-run digest the verifier does not recompute, its assurance coming from
# the in-process campaign re-run instead). `output` binds the sha256 of the
# captured stdout with campaign-root paths normalized.
# ---------------------------------------------------------------------------

def _owner_package_files(root: Path, owner: str) -> list[Path]:
    """The exact file set whose bytes define an audit / project owner tool:
    the shim plus every non-__pycache__ file under its package."""
    shim, pkg = {
        "audit": ("bootstrap/audit.py", "bootstrap/audit"),
        "project-check": ("bootstrap/project.py", "bootstrap/project"),
    }[owner]
    files = [root / shim]
    for path in sorted((root / pkg).rglob("*")):
        if path.is_file() and "__pycache__" not in path.parts:
            files.append(path)
    return files


def _owner_tool_digest(root: Path, owner: str) -> str:
    rows = sorted(f"{p.relative_to(root).as_posix()}\t{sb.sha_hex(p.read_bytes())}"
                  for p in _owner_package_files(root, owner))
    return sb.sha_hex("".join(r + "\n" for r in rows))


def _write_owner_receipt(receipts_dir: Path, owner: str, source_commit: str,
                         tool: str, output: str) -> str:
    """Persist one BATPAK-OWNER-RECEIPT/1 content-addressed under the
    artifact's receipts dir; append-only (identical bytes lawful, differing
    bytes at the same address refused). Returns its sha256-of-bytes digest --
    the value the zero row references."""
    lines = [OWNER_RECEIPT_MAGIC, f"owner {owner}",
             f"source-commit {source_commit}", f"tool {tool}",
             "outcome pass", f"output {output}", "end"]
    data = "".join(line + "\n" for line in lines).encode("ascii")
    digest = sb.sha_hex(data)
    receipts_dir.mkdir(parents=True, exist_ok=True)
    path = receipts_dir / f"{digest}.receipt"
    if path.exists():
        if path.read_bytes() != data:
            raise ProbeError(f"owner receipt collision at {path.name}")
    else:
        path.write_bytes(data)
    return digest


# ---------------------------------------------------------------------------
# The underwrite: bindings, rows, artifact, independent verification
# ---------------------------------------------------------------------------

def _render(bindings: dict[str, str],
            rows: list[tuple[str, int, str, str]]) -> str:
    lines = [E7_MAGIC]
    lines.extend(f"{key} {bindings[key]}" for key in BIND_KEYS)
    lines.extend(f"zero {token} {count} owner {owner} receipt {receipt}"
                 for token, count, owner, receipt in rows)
    lines.append(CROSSRUN_PENDING)
    lines.append(ARCHITECT_LINE)
    lines.append("end")
    return "".join(line + "\n" for line in lines)


def underwrite(out_dir: str | Path, campaign_root: str | Path,
               tier0_bundle: str | Path) -> list[str]:
    """Produce the BATPAK-E7-UNDERWRITING/1 artifact in `out_dir` from the
    campaign roots and the Tier 0 evidence bundle, then block on the
    independent `receiptcheck e7-verify` recompute of every binding and
    every row. Returns findings; empty means underwritten and verified."""
    out = Path(out_dir).resolve()
    campaign = Path(campaign_root).resolve()
    tier0 = Path(tier0_bundle).resolve()
    judge = campaign / "judge"
    nursery = campaign / "nursery"
    evidence = campaign / "evidence"
    bundle_path = evidence / "campaign-evidence.bundle"
    envelope_path = evidence / "campaign-release.envelope"
    qual_path = tier0 / "qualification.t0"
    gate0 = tier0 / "gate0-candidate"
    for path, what in ((judge, "judge root"), (nursery, "nursery root"),
                       (evidence, "evidence root"), (gate0, "gate0 candidate")):
        if not path.is_dir():
            return [f"{what} absent at {path}"]
    for path, what in ((bundle_path, "campaign bundle"),
                       (envelope_path, "release envelope"),
                       (qual_path, "tier0 qualification artifact")):
        if not path.is_file():
            return [f"{what} absent at {path}"]
    rustc = shutil.which("rustc")
    if not rustc:
        return ["rustc absent; the independent verifier cannot be built"]
    source_commit = sb.source_commit_of(HERE.parent)
    if source_commit is None:
        return ["no git HEAD commit; the artifact cannot bind its source"]

    # The independent verifier FIRST (basis c): the campaign-derived rows are
    # honest zeros only above a receiptcheck campaign-verify PASS.
    work = out.parent / (out.name + "-work")
    out.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    target = sb.working_target(rustc, work)
    rc_exe, err = sb.build_receiptcheck(rustc, target, work)
    if rc_exe is None:
        return [f"receiptcheck unavailable: {err}"]
    ok, cv_out = sb.campaign_verify(rc_exe, bundle_path, judge, envelope_path,
                                    source_commit, nursery, evidence)
    if not ok:
        return ["receiptcheck REFUSED the bound campaign bundle; the "
                "campaign-derived rows cannot be underwritten:\n" + cv_out]
    print(f"e7: {cv_out.splitlines()[0]} (campaign bundle independently verified)")

    # CL-8: wrap this fresh campaign-verify PASS into its owner receipt. `tool`
    # is the built receiptcheck exe (per-run bytes; the verifier re-runs the
    # campaign core rather than recomputing this digest); `output` is the
    # normalized captured stdout. tier0/audit/project receipts follow below.
    # CL-11: this exe hash MAY stay per-run -- cross-run equality is over the
    # verified authoritative RESULT + source/toolchain coordinates, never the
    # native exe bytes; e7_crossrun.py and receiptcheck e7-open both compare
    # each zero row's (token, count) RESULT only (owner+receipt are ignored).
    receipts_dir = out / "receipts"
    owner_receipts: dict[str, str] = {}
    owner_receipts["campaign-verify"] = _write_owner_receipt(
        receipts_dir, "campaign-verify", source_commit,
        sb.sha_hex(rc_exe.read_bytes()),
        sb.sha_hex(sb.normalize_paths(cv_out, [campaign])))

    findings: list[str] = []
    root = HERE.parent
    bundle_text = bundle_path.read_text(encoding="utf-8")
    qual_text = qual_path.read_text(encoding="utf-8")
    sections = bundle_sections(bundle_text)
    cands = _candidate_blocks(sections["candidates"])
    frontier = _frontier_map(sections["frontier"])
    terminals = _closure_terminals(sections["closure"])
    root_ids = _root_ids(sections.get("roots", []))
    recs = _nursery_records(nursery)
    profile = sb.mini_supernova_profile()

    # Bindings.
    spec_manifest_bytes = (root / "SPEC.sha256").read_bytes()
    tree_rows: list[str] = []
    for line in spec_manifest_bytes.decode("utf-8").splitlines():
        _claimed, _, rel = line.partition("  ")
        if not rel:
            raise ProbeError(f"SPEC.sha256 row {line!r} is not `<hex>  <rel>`")
        tree_rows.append(f"{sb.sha_hex((root / rel).read_bytes())}  {rel}")
    toolchain_lines = sections["toolchain"]
    rustc_release = next((ln.removeprefix("rustc-release ")
                          for ln in toolchain_lines
                          if ln.startswith("rustc-release ")), None)
    bundle_target = next((ln.removeprefix("target ")
                          for ln in toolchain_lines
                          if ln.startswith("target ")), None)
    if not rustc_release or not bundle_target:
        raise ProbeError("bundle toolchain section binds no rustc-release/target")
    python_release = re.search(r"^python-release: (\S+)$", qual_text, re.M)
    if not python_release:
        raise ProbeError("qualification.t0 binds no python-release")
    vi = sys.version_info
    if python_release.group(1) != f"{vi.major}.{vi.minor}.{vi.micro}":
        raise ProbeError(
            f"the bound Tier 0 evidence rode CPython {python_release.group(1)}, "
            "not this underwriter's runtime")
    mat_tree = sb.tree_digest(gate0)
    claimed_tree = re.search(r"output-tree-digest=([0-9a-f]{64})", qual_text)
    if not claimed_tree or claimed_tree.group(1) != mat_tree:
        raise ProbeError("the gate0-candidate tree does not match the bound "
                         "tier0-materialize output-tree-digest")
    manifests = sorted(
        (p.relative_to(gate0).as_posix(), sb.sha_hex(p.read_bytes()))
        for p in gate0.rglob("Cargo.toml")
        if p.relative_to(gate0).as_posix() != "Cargo.toml")
    bindings = {
        "source-commit": source_commit,
        "source-tree": sb.sha_hex("".join(r + "\n" for r in tree_rows)),
        "spec-manifest": sb.sha_hex(spec_manifest_bytes),
        "workflow-digest": sb.sha_hex(
            (root / ".github/workflows/msvc-qualification.yml").read_bytes()),
        "toolchain": f"{rustc_release} {bundle_target}",
        "python": python_release.group(1),
        "tier0-bundle": sb.tree_digest(tier0),
        "campaign-bundle": sb.sha_hex(bundle_path.read_bytes()),
        "materializer-output-tree": mat_tree,
        "package-graph": sb.sha_hex(
            "".join(f"{rel}\t{digest}\n" for rel, digest in manifests)),
        "generated-view-registry": sb.sha_hex(
            (root / "spec/generated_views/registry.rs").read_bytes()),
        "release-envelope": sb.sha_hex(envelope_path.read_bytes()),
    }

    # The twenty rows, each from its named basis.
    row1, plans, claims = _proof_rows()
    registry_empty, registry_foreign = _registry_rows()
    # The audit and project gates run FRESH here and each is wrapped into its
    # owner receipt (CL-8). A refused gate cannot honestly back a pass receipt,
    # so it is a hard stop before any owner receipt or artifact is produced.
    audit_rc, audit_out = _fresh_gate([str(HERE / "audit.py")])
    if audit_rc != 0:
        return ["fresh audit refused; its rows have no owner receipt:\n"
                + audit_out[-2000:]]
    project_rc, project_out = _fresh_gate([str(HERE / "project.py"), "--check"])
    if project_rc != 0:
        return ["fresh project --check refused; its rows have no owner "
                "receipt:\n" + project_out[-2000:]]
    owner_receipts["audit"] = _write_owner_receipt(
        receipts_dir, "audit", source_commit,
        _owner_tool_digest(root, "audit"),
        sb.sha_hex(sb.normalize_paths(audit_out, [campaign])))
    owner_receipts["project-check"] = _write_owner_receipt(
        receipts_dir, "project-check", source_commit,
        _owner_tool_digest(root, "project-check"),
        sb.sha_hex(sb.normalize_paths(project_out, [campaign])))
    # tier0 rows are proved by the bound Tier 0 bundle itself: the receipt is
    # the sha256 of its qualification.t0 (no separate owner receipt file).
    receipt_for = dict(owner_receipts, tier0=sb.sha_hex(qual_path.read_bytes()))
    census = _package_census()
    values = [
        row1,
        _zero_adopterless(plans, claims),
        _zero_fresh_adopters(cands, recs, sections, qual_text, profile[1],
                             terminals),
        registry_empty,
        _zero_unbounded(sections["policy"]),
        _zero_lineage(cands, recs, profile, root_ids),
        _zero_evidence_refs(recs, evidence),
        _zero_overlap(judge),
        _zero_monitor_rewrite(),
        _zero_lawchanging(cands, recs, frontier),
        _zero_qualified_promoted(cands, recs, frontier, terminals),
        registry_foreign,
        _zero_python_maps(),
        0 if project_rc == 0 else 1,
        _zero_envelope(envelope_path),
        int(census != 9) + int(len(manifests) != 9) + int(audit_rc != 0),
        0 if audit_rc == 0 else 1,
        _zero_scaffolds(cands, frontier, terminals),
        _zero_dependency_first(cands, frontier, root_ids),
        _zero_architect(recs),
    ]
    rows = [(token, count, owner, receipt_for[owner])
            for (token, count), owner in zip(
                zip(ZERO_TOKENS, values, strict=True), ROW_OWNERS, strict=True)]
    for token, count, owner, receipt in rows:
        print(f"e7: zero {token} {count} owner {owner} receipt {receipt[:12]}")
        if count != 0:
            findings.append(f"zero row {token} computed {count}, not zero")

    # Render, persist, then INDEPENDENTLY verify the artifact. A nonzero row
    # is rendered honestly -- the artifact never lies, and the verifier
    # refuses it by name.
    artifact_path = out / ARTIFACT_NAME
    artifact_path.write_text(_render(bindings, rows), encoding="ascii",
                             newline="\n")
    proc = subprocess.run(
        [str(rc_exe), "e7-verify", str(artifact_path),
         "--root", str(root), "--tier0-bundle", str(tier0),
         "--campaign-bundle", str(bundle_path), "--judge-root", str(judge),
         "--envelope", str(envelope_path), "--source-commit", source_commit,
         "--nursery-root", str(nursery), "--evidence-root", str(evidence)],
        capture_output=True, text=True)
    verify_out = (proc.stdout + proc.stderr).strip()
    if proc.returncode != 0 or "e7: PASS" not in verify_out:
        findings.append("receiptcheck REFUSED the produced e7 artifact:\n"
                        + verify_out)
    else:
        print(verify_out)
    return findings


def underwrite_cli(out_dir: str, campaign_root: str, tier0_bundle: str) -> int:
    """`selftest.py --e7-underwrite <out> --campaign-root <dir>
    --tier0-bundle <dir>`: the E7-F opening-matrix producer, blocking on the
    independent receiptcheck e7-verify."""
    try:
        findings = underwrite(out_dir, campaign_root, tier0_bundle)
    except ProbeError as exc:
        print(f"selftest: E7 UNRESOLVED: {exc}", file=sys.stderr)
        return 1
    if findings:
        print(f"selftest: E7 UNDERWRITING FAIL ({len(findings)} finding(s))",
              file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("selftest: e7 underwriting PASS "
          f"({Path(out_dir).resolve() / ARTIFACT_NAME})")
    return 0

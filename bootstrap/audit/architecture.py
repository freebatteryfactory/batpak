"""Architecture and spec-structure audits.

Owns the DEC-075 reconciliation composition, release-seal and proof-terminal
denominators, authenticated-history claims (DEC-071), document-law survival,
the PakVM semantic ISA lineage, and the SyncBat firewall. Each half reparses
the typed owner independently of bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .corpus import _const_variants, _enum_variants, _spec_module_source, _uncomment
from .guarantees import gate_findings, gate_list

# --- DEC-075 reconciliation composition (5.5E1d) ----------------------------
# AUDITOR half: independent parse, independent render, plus the cross-document
# laws the generator cannot see. Shares no parsing with project.py.
A_RECON_SRC = "spec/reconciliation.rs"
A_RECON_DOC = "docs/02_SYSTEM_MODEL.md"
A_RECON_ROW = re.compile(
    r"ReconciliationCoordinate \{\s*role: ReconciliationRole::(\w+),\s*"
    r"carriers: &\[([^\]]*)\],\s*law: \"((?:[^\"\\]|\\.)*)\"", re.S)


def recon_views(root: Path) -> dict:
    src = (root / A_RECON_SRC).read_text(encoding="utf-8")
    start = src.index("pub const RECONCILIATION_COORDINATES")
    coords = [
        {"role": m.group(1),
         "carriers": re.findall(r'"([^"]+)"', m.group(2)),
         "law": re.sub(r"\\\n\s*", "", m.group(3))}
        for m in A_RECON_ROW.finditer(src[start: src.index("\n];", start)])
    ]
    fn = src[src.index("pub const fn admissible"):]
    fn = fn[: fn.index("\n    }")]
    adm: list[str] = []
    inadm: list[str] = []
    for arm, verdict in re.findall(r"((?:RetrySignal::\w+\s*\|?\s*)+)=>\s*(true|false)", fn):
        (adm if verdict == "true" else inadm).extend(re.findall(r"RetrySignal::(\w+)", arm))
    listing = re.findall(r"RetrySignal::(\w+)",
                         src[src.index("pub const RETRY_SIGNALS"):])
    return {"coords": coords, "admissible": adm, "inadmissible": inadm,
            "listing": listing}


def recon_findings(root: Path) -> list[str]:
    out: list[str] = []
    try:
        v = recon_views(root)
    except (ValueError, KeyError) as exc:
        return [f"spec/reconciliation.rs is unreadable: {exc}"]
    roles = [c["role"] for c in v["coords"]]
    if sorted(set(roles)) != sorted(roles) or len(roles) != 5:
        out.append(f"reconciliation coordinates do not bind five distinct roles ({roles})")
    for c in v["coords"]:
        if not c["carriers"]:
            out.append(f"reconciliation role {c['role']} names no carrier")
    if not v["admissible"] or not v["inadmissible"]:
        out.append("retry classification does not partition the signals")
    classified = set(v["admissible"]) | set(v["inadmissible"])
    if set(v["listing"]) != classified:
        out.append("RETRY_SIGNALS and the admissible() classification disagree "
                   f"({sorted(set(v['listing']) ^ classified)})")
    # Chronology carriers must be vocabulary docs/16 owns: a chronology witness
    # this composition invents would be a second time authority.
    d16 = (root / "docs/16_IDENTITY_TIME_AND_NAVIGATION.md").read_text(encoding="utf-8")
    for c in v["coords"]:
        if c["role"] in ("ChronologyWitness", "DurableOrderWitness"):
            for carrier in c["carriers"]:
                if not re.search(r"\b" + re.escape(carrier) + r"\b", d16):
                    out.append(f"reconciliation carrier {carrier} is not owned by docs/16")
    # Projection parity: the generated docs/02 blocks must match this parse.
    doc = (root / A_RECON_DOC).read_text(encoding="utf-8")
    m = re.search(r"RECONCILIATION-COORDINATES:BEGIN[^>]*-->\n(.*?)\n<!-- "
                  r"RECONCILIATION-COORDINATES:END", doc, re.S)
    if m is None:
        out.append("docs/02 carries no generated reconciliation-coordinates block")
    else:
        want = ["| Role | Carriers | Law |", "| --- | --- | --- |"] + [
            f"| {c['role']} | {', '.join(c['carriers'])} | {c['law']} |"
            for c in v["coords"]]
        if m.group(1) != "\n".join(want):
            out.append("docs/02 reconciliation-coordinates block drifted from spec/reconciliation.rs")
    m = re.search(r"RECONCILIATION-RETRY:BEGIN[^>]*-->\n(.*?)\n<!-- "
                  r"RECONCILIATION-RETRY:END", doc, re.S)
    if m is None:
        out.append("docs/02 carries no generated reconciliation-retry block")
    else:
        want = (["```text", "admissible for retry, resume, compensation, or refusal:"]
                + [f"    {n}" for n in v["admissible"]]
                + ["", "never sufficient on their own:"]
                + [f"    {n}" for n in v["inadmissible"]] + ["```"])
        if m.group(1) != "\n".join(want):
            out.append("docs/02 reconciliation-retry block drifted from spec/reconciliation.rs")
    return out


def proof_terminal_findings(root: Path) -> list[str]:
    """SEED-AUDITED-DENOMINATOR (5.5E1): the proof-terminal vocabulary has one
    typed owner; docs/12 projects it. The only-Passed-counts-green fence is
    seedcheck's executed law; the auditor checks structure and parity."""
    out: list[str] = []
    src = (root / "spec/proof.rs").read_text(encoding="utf-8")
    start = src.find("pub const PROOF_UNIT_TERMINALS")
    if start < 0:
        return ["spec/proof.rs declares no PROOF_UNIT_TERMINALS"]
    names = re.findall(r"ProofUnitTerminal::(\w+)", src[start: src.index("\n];", start)])
    if len(names) != len(set(names)):
        out.append("a proof terminal is listed twice")
    fn = src[src.index("pub const fn is_positive_semantic_terminal"):]
    fn = fn[: fn.index("\n    }")]
    green = [n for arm, verdict in re.findall(
        r"((?:ProofUnitTerminal::\w+\s*\|?\s*)+)=>\s*(true|false)", fn)
        for n in re.findall(r"ProofUnitTerminal::(\w+)", arm) if verdict == "true"]
    if not green or set(green) >= set(names):
        out.append("the green classification is degenerate: the denominator "
                   "would be vacuous or unreachable")
    doc = (root / "docs/12_TESTPAK.md").read_text(encoding="utf-8")
    m = re.search(r"PROOF-TERMINALS:BEGIN[^>]*-->\n(.*?)\n<!-- PROOF-TERMINALS:END",
                  doc, re.S)
    if m is None:
        out.append("docs/12 carries no generated proof-terminals block")
    elif m.group(1) != "\n".join(["```text"] + names + ["```"]):
        out.append("docs/12 proof-terminals block drifted from spec/proof.rs")
    return out


def release_seal_findings(root: Path) -> list[str]:
    """DEC-058's seal binds ONE typed inventory (5.5E1). The auditor re-derives
    the field list independently, checks the docs/36 projection, and refuses a
    decision row that restates the list instead of naming the owner."""
    out: list[str] = []
    src = _spec_module_source(root, "architecture")
    start = src.find("pub const RELEASE_SEAL_FIELDS")
    if start < 0:
        return ["spec/architecture.rs declares no RELEASE_SEAL_FIELDS"]
    fields = re.findall(r"ReleaseSealField::(\w+)", src[start: src.index("\n];", start)])
    if len(fields) != len(set(fields)):
        out.append("a release-seal field is listed twice")
    if "KernelQualificationSet" not in fields:
        out.append("the kernel qualification set left the release seal; an empty "
                   "set states 'no kernels admitted', it never disappears")
    doc = (root / "docs/36_PUBLIC_API_CI_AND_RELEASE.md").read_text(encoding="utf-8")
    m = re.search(r"RELEASE-SEAL:BEGIN[^>]*-->\n(.*?)\n<!-- RELEASE-SEAL:END", doc, re.S)
    if m is None:
        out.append("docs/36 carries no generated release-seal block")
    elif m.group(1) != "\n".join(["```text"] + fields + ["```"]):
        out.append("docs/36 release-seal block drifted from the typed inventory")
    disp = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    row = re.search(r'DecisionSpec \{ id: "DEC-058".*?successor: "([^"]*)"', disp, re.S)
    if row is None:
        out.append("DEC-058 is missing from the decision ledger")
    elif "ReleaseSealField" not in row.group(1):
        out.append("DEC-058 restates a seal list instead of naming the typed "
                   "ReleaseSealField owner")
    return out


# --- Authenticated history (DEC-071) ----------------------------------------
# Structural contract checks only. Bootstrap performs no signature, accumulator,
# witness, freshness, or cryptographic verification and must never claim to.
# The profile TABLE dissolved in 5.5E2: every authenticated-history fact is a
# const fn on AuthenticatedHistoryProfile, so the auditor parses the match arms
# of the fn that owns each fact — the same posture as decision_lifetime_map.
A_AH_FN_ARM = re.compile(
    r"((?:AuthenticatedHistoryProfile::\w+\s*\|?\s*)+)=>\s*"
    r"(\{.*?\}|&\[[^\]]*\]|\w+(?:\([^)]*\))?)",
    re.S,
)
A_WITNESS_VARIANT = re.compile(r'WitnessPolicy::(\w+)')
A_CLAIM_BUNDLE = re.compile(
    r'pub const (\w+): AuthenticatedHistoryClaims = AuthenticatedHistoryClaims \{\s*'
    r'integrity: IntegrityClaim::(\w+),\s*'
    r'authenticity: AuthenticityClaim::(\w+),\s*'
    r'freshness: FreshnessClaim::(\w+),\s*'
    r'rollback_resistance: RollbackResistanceClaim::(\w+),\s*\}',
    re.S,
)

A_FROZEN_MATRIX = {
    "InternalConsistency": ["None"],
    "SignedHistory": ["None", "Optional"],
    "ExternallyAnchoredHistory": ["Required"],
}
# The three canonical success bundles, four axes each. Independent axes, not an
# ordered ladder: an InternalConsistency success asserts verified integrity AND
# unavailable rollback resistance simultaneously, which one enum could not say.
A_FROZEN_BUNDLES = {
    "INTERNAL_CONSISTENCY_SUCCESS":
        ("InternalConsistencyVerified", "NotClaimed", "NotClaimed", "Unavailable"),
    "SIGNED_UNANCHORED_SUCCESS":
        ("InternalConsistencyVerified", "SignedHistoryVerified", "NotClaimed", "Unavailable"),
    "WITNESSED_SUCCESS":
        ("InternalConsistencyVerified", "SignedHistoryVerified",
         "WitnessedGenerationVerified", "ScopedToVerifiedWitness"),
}
# profile -> (unanchored success bundle, verified-witness success bundle).
# None means NO SUCCESSFUL RESULT IS ADMITTED, never "unknown".
A_FROZEN_PROFILE_BUNDLES = {
    "InternalConsistency": ("Some(INTERNAL_CONSISTENCY_SUCCESS)", "None"),
    "SignedHistory": ("Some(SIGNED_UNANCHORED_SUCCESS)", "Some(WITNESSED_SUCCESS)"),
    "ExternallyAnchoredHistory": ("None", "Some(WITNESSED_SUCCESS)"),
}
A_CLAIM_AXES = {
    "IntegrityClaim": {"InternalConsistencyVerified"},
    "AuthenticityClaim": {"NotClaimed", "SignedHistoryVerified"},
    "FreshnessClaim": {"NotClaimed", "WitnessedGenerationVerified"},
    "RollbackResistanceClaim": {"Unavailable", "ScopedToVerifiedWitness"},
}
A_REQUIRED_FAILURES = {
    "NotProvided", "Stale", "Conflicting", "Unverifiable",
    "CryptographicallyInvalid", "LineageMismatch", "GenerationMismatch",
    "AccumulatorMismatch",
}
A_WITNESS_DISPOSITIONS = {
    "NotApplicable", "NotProvided", "Verified", "Stale", "Conflicting",
    "Unverifiable", "CryptographicallyInvalid", "LineageMismatch",
    "GenerationMismatch", "AccumulatorMismatch",
}
# Retired by 5.5C2c: one mutually exclusive posture could not represent four
# orthogonal claims, and its non-Option unanchored field gave
# ExternallyAnchoredHistory the shape of a fallback success.
A_RETIRED_CLAIM_VOCAB = ("HistoryClaimPosture", "claim_posture", "unanchored_claim_posture")
A_CLAIM_LADDER_NAMES = ("SecurityPosture", "VerificationLevel", "AssuranceLevel", "SecurityLevel")


def ah_fn_arms(root: Path, name: str) -> dict[str, str]:
    """AuthenticatedHistoryProfile variant -> that profile's value expression,
    parsed from the const fn that owns the fact."""
    src = _uncomment(_spec_module_source(root, "architecture"))
    body = re.search(
        r"pub const fn " + re.escape(name) + r"\(self\)[^{]*\{\s*match self \{(.*?)\n    \}",
        src, re.S)
    out: dict[str, str] = {}
    if not body:
        return out
    for arm in A_AH_FN_ARM.finditer(body.group(1)):
        value = arm.group(2).strip()
        if value.startswith("{") and value.endswith("}"):
            value = value[1:-1].strip()
        for p in re.findall(r"AuthenticatedHistoryProfile::(\w+)", arm.group(1)):
            out[p] = value
    return out


def authenticated_history_rows(root: Path) -> list[dict]:
    """One derived row per profile, joined across the owning const fns. A
    profile absent from ANY fn simply lacks that key's fact and is reported by
    the findings, never defaulted."""
    facts = {
        "policies": {p: A_WITNESS_VARIANT.findall(v)
                     for p, v in ah_fn_arms(root, "permitted_witness_policies").items()},
        "local": {p: v == "true"
                  for p, v in ah_fn_arms(root, "requires_local_commitment_verification").items()},
        "signed": {p: v == "true"
                   for p, v in ah_fn_arms(root, "requires_signed_history_verification").items()},
        "witness": {p: v == "true"
                    for p, v in ah_fn_arms(root, "requires_independent_witness_verification").items()},
        "impl_gates": {p: gate_list(v)
                       for p, v in ah_fn_arms(root, "implementation_gates").items()},
        "release_gates": {p: gate_list(v)
                          for p, v in ah_fn_arms(root, "release_qualification_gates").items()},
        "unanchored": dict(ah_fn_arms(root, "unanchored_success_claims")),
        "verified_witness": dict(ah_fn_arms(root, "verified_witness_success_claims")),
    }
    profiles = sorted(set().union(*[set(m) for m in facts.values()]) if any(facts.values()) else set())
    return [{"profile": p, **{k: m.get(p) for k, m in facts.items()}} for p in profiles]


def claim_bundles(root: Path) -> dict[str, tuple[str, str, str, str]]:
    src = _spec_module_source(root, "architecture")
    return {m[0]: (m[1], m[2], m[3], m[4]) for m in A_CLAIM_BUNDLE.findall(src)}


def claim_axis_findings(root: Path) -> list[str]:
    """Four independently representable claim axes, no ladder, no flattening."""
    out: list[str] = []
    src = _spec_module_source(root, "architecture")
    for term in A_RETIRED_CLAIM_VOCAB:
        if term in src:
            out.append(f"spec/architecture.rs reintroduces retired claim vocabulary {term}")
    for name in A_CLAIM_LADDER_NAMES:
        # A DECLARATION, not a doc comment naming what must never be built.
        if re.search(r"^\s*pub (?:enum|struct|type|const) " + name + r"\b", src, re.M):
            out.append(f"spec/architecture.rs introduces a generic security ladder {name}")
    for axis, want in A_CLAIM_AXES.items():
        got = _enum_variants(root, axis)
        if not got:
            out.append(f"spec/architecture.rs declares no {axis}")
        elif got != want:
            out.append(f"{axis} variants {sorted(got)} != frozen {sorted(want)}")
    fields = re.search(r"pub struct AuthenticatedHistoryClaims \{(.*?)\n\}", src, re.S)
    if not fields:
        out.append("spec/architecture.rs declares no AuthenticatedHistoryClaims")
    else:
        for want in ("integrity: IntegrityClaim", "authenticity: AuthenticityClaim",
                     "freshness: FreshnessClaim", "rollback_resistance: RollbackResistanceClaim"):
            if want not in fields.group(1):
                out.append(f"AuthenticatedHistoryClaims omits {want}")
    bundles = claim_bundles(root)
    if set(bundles) != set(A_FROZEN_BUNDLES):
        out.append(f"claim bundles {sorted(bundles)} != frozen {sorted(A_FROZEN_BUNDLES)}")
    for name, want in A_FROZEN_BUNDLES.items():
        got = bundles.get(name)
        if got and got != want:
            out.append(f"{name} claims {got} != frozen bundle {want}")
    # Axis coherence, for these three profiles and this witness model.
    for name, got in bundles.items():
        integrity, _authenticity, freshness, rollback = got
        if integrity != "InternalConsistencyVerified":
            out.append(f"{name} success bundle does not verify integrity")
        if (freshness == "WitnessedGenerationVerified") != (rollback == "ScopedToVerifiedWitness"):
            out.append(f"{name} lets freshness and rollback resistance drift apart")
    if not re.search(r"^pub const REFUSAL_PARTIAL_CLAIM_LAW: &str =", src, re.M):
        out.append("spec/architecture.rs states no refusal/partial-evidence law")
    return out


def authenticated_history_findings(root: Path) -> list[str]:
    rows = authenticated_history_rows(root)
    out: list[str] = list(claim_axis_findings(root))
    if not rows:
        return out + ["spec/architecture.rs derives no authenticated-history profile facts"]
    names = [r["profile"] for r in rows]
    if set(names) != set(A_FROZEN_MATRIX):
        out.append(f"authenticated-history profile set {sorted(names)} != frozen {sorted(A_FROZEN_MATRIX)}")
    for r in rows:
        # Every owning fn classifies every profile; a missing arm is a
        # missing fact, never a default.
        missing = [k for k, v in r.items() if v is None]
        if missing:
            out.append(f"{r['profile']} has no authored fact for {missing}")
            continue
        want = A_FROZEN_MATRIX.get(r["profile"])
        if want is None:
            continue
        if r["policies"] != want:
            out.append(f"{r['profile']} permits witness policies {r['policies']}, frozen matrix says {want}")
        if "Required" in r["policies"] and r["profile"] != "ExternallyAnchoredHistory":
            out.append(f"{r['profile']} permits WitnessPolicy::Required outside ExternallyAnchoredHistory")
        if r["profile"] == "ExternallyAnchoredHistory" and (
            "None" in r["policies"] or "Optional" in r["policies"]
        ):
            out.append("ExternallyAnchoredHistory permits a non-Required witness policy")
        # Which success bundles each profile may reach.
        want_un, want_wit = A_FROZEN_PROFILE_BUNDLES[r["profile"]]
        if r["unanchored"] != want_un:
            if r["profile"] == "ExternallyAnchoredHistory" and r["unanchored"] != "None":
                out.append(
                    "ExternallyAnchoredHistory admits an unanchored success bundle; an absent or "
                    "invalid required witness must refuse, not fall back to a weaker success"
                )
            else:
                out.append(f"{r['profile']} unanchored_success_claims {r['unanchored']} != frozen {want_un}")
        if r["verified_witness"] != want_wit:
            out.append(
                f"{r['profile']} verified_witness_success_claims {r['verified_witness']} != frozen {want_wit}"
            )
        # Capability/claim agreement.
        if r["profile"] == "InternalConsistency" and (r["signed"] or r["witness"]):
            out.append("InternalConsistency requires signed-history or witness verification")
        if r["profile"] == "SignedHistory":
            if not r["signed"]:
                out.append("SignedHistory does not require signed-history verification")
            if r["witness"]:
                out.append("SignedHistory requires an independent witness; that is ExternallyAnchoredHistory")
        if r["profile"] == "ExternallyAnchoredHistory" and not (r["signed"] and r["witness"]):
            out.append("ExternallyAnchoredHistory does not require signed history plus an independent witness")
        if not r["local"]:
            out.append(f"{r['profile']} does not require local commitment verification")
        # An unanchored success never claims freshness or scoped rollback resistance.
        bundles = claim_bundles(root)
        m = re.match(r"Some\((\w+)\)", r["unanchored"])
        if m and m.group(1) in bundles:
            _i, authenticity, freshness, rollback = bundles[m.group(1)]
            if freshness != "NotClaimed" or rollback != "Unavailable":
                out.append(f"{r['profile']} unanchored success claims freshness or rollback resistance")
            if r["profile"] == "InternalConsistency" and authenticity != "NotClaimed":
                out.append("InternalConsistency unanchored success claims signed authenticity")
            if r["profile"] == "SignedHistory" and authenticity != "SignedHistoryVerified":
                out.append("SignedHistory unanchored success does not claim signed authenticity")
        out.extend(gate_findings(root, "profile", r["profile"] + " implementation", r["impl_gates"]))
        out.extend(gate_findings(root, "profile", r["profile"] + " release", r["release_gates"]))
        if not r["impl_gates"]:
            out.append(f"{r['profile']} names no implementation gate")
        if not r["release_gates"]:
            out.append(f"{r['profile']} names no release qualification gate")
    declared = _enum_variants(root, "WitnessDisposition")
    if not declared:
        out.append("spec/architecture.rs declares no WitnessDisposition")
    else:
        missing = A_WITNESS_DISPOSITIONS - declared
        if missing:
            out.append(f"WitnessDisposition collapses required distinctions: missing {sorted(missing)}")
    required = set(_const_variants(root, "REQUIRED_WITNESS_FAILURE_SET"))
    if required != A_REQUIRED_FAILURES:
        out.append(
            f"REQUIRED_WITNESS_FAILURE_SET {sorted(required)} != frozen failure set {sorted(A_REQUIRED_FAILURES)}"
        )
    optional = set(_const_variants(root, "OPTIONAL_WITNESS_REFUSAL_SET"))
    if "NotProvided" in optional:
        out.append("OPTIONAL_WITNESS_REFUSAL_SET refuses an absent optional witness")
    if optional != A_REQUIRED_FAILURES - {"NotProvided"}:
        out.append("a supplied invalid optional witness may degrade to absence or success")
    return out


def retired_claim_vocabulary_findings(root: Path) -> list[str]:
    """The flattened claim type may not return through any authoritative surface."""
    out: list[str] = []
    _decomposed = {"spec/architecture.rs": "architecture", "spec/guarantees.rs": "guarantees"}
    for rel in ("spec/architecture.rs", "spec/dispositions.rs", "spec/guarantees.rs",
                "docs/14_RECEIPTS_AND_EXPLANATION.md", "docs/19_SECURITY_MODEL.md",
                "docs/05_STORAGE_FBAT_AND_TILES.md", "docs/31_FINAL_CONTRADICTION_AUDIT.md"):
        if rel in _decomposed:
            text = _spec_module_source(root, _decomposed[rel])
        else:
            path = root / rel
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
        for line in text.splitlines():
            if "[RETIRED-CLAIM-QUOTE]" in line:
                continue  # an explicitly marked historical quotation
            for term in A_RETIRED_CLAIM_VOCAB:
                if term in line:
                    out.append(f"{rel}: retired claim vocabulary {term} reintroduced")
                    break
    return out


# --- Document law: the claims that must not quietly vanish -------------------
# Whitespace-insensitive markers: prose wraps, law does not.
DOC_LAW_MARKERS = {
    "docs/19_SECURITY_MODEL.md": [
        ("whole-store rollback threat",
         "may restore an older complete generation whose internal commitments and signatures remain valid"),
        ("whole-store rollback threat",
         "Neither alone proves that it is the newest generation ever acknowledged"),
        ("freshness requires an independent monotonic witness",
         "requires an independent monotonic witness"),
        ("four axes stay distinct",
         "rollback resistance  restoring an older valid generation is detectable"),
        ("four axes stay distinct", "authenticity         an authenticated signer produced this history"),
        ("optional witness is mandatory to validate once supplied",
         "Once supplied it is mandatory to validate"),
    ],
    "docs/07_PAKVM_ISA.md": [
        ("the reference interpreter is the executable semantic authority",
         "The reference interpreter is the executable semantic authority for admitted programs"),
        ("the residual is never the sole oracle", "the residual path is never the sole oracle"),
        ("SpecializedPlan is derived, never authority",
         "It is never program authority, canonical source, a new `ProgramImage`"),
        ("reuse under a different key is illegal", "SpecializedPlan reused under a different key"),
        ("only two specialization modes", "bounded first-use residual specialization"),
        ("no machine-code JIT or executable memory in V1",
         "machine-code JIT, executable-memory allocation, self-modifying code"),
        ("a key component is never silently omitted", "It may never be silently omitted"),
        ("dynamic facts are not bound as static", "wall-clock time, file descriptor numbers, socket state"),
        ("specialization pre-executes no effects",
         "It may not pre-execute durable effects, port effects, wall-clock reads"),
        ("specialization cannot bypass Bvisor admission",
         "It may not eliminate or bypass Bvisor admission"),
        ("specialization cannot mint capabilities", "PakVM specialization cannot mint capabilities."),
        ("specialization cannot advance a durable checkpoint",
         "PakVM specialization cannot advance a durable checkpoint."),
        ("specialization cannot authorize a physical effect",
         "PakVM specialization cannot authorize a physical effect."),
        ("a check is not discharged because it usually passes",
         "A check is not eliminated merely because the current implementation usually passes it"),
        ("cache mismatch discards and falls back",
         "it discards, refuses reuse, and falls back to reference execution"),
        ("an old residual is never reinterpreted under a new key",
         "An old residual is never reinterpreted under a new key"),
        ("cache deletion changes performance only",
         "Deleting every specialization cache changes performance and `WorkObservation` only"),
        ("reference execution remains complete without a cache",
         "because reference execution remains complete"),
        ("first use is budgeted and deadline-bound",
         "an explicit work budget, an absolute monotonic deadline, a bounded memory posture"),
        ("specialization never resets the logical deadline",
         "The specialization attempt never resets the logical operation deadline"),
        ("result axes are preserved across paths",
         "preserve semantic value, `Availability`, `Truth`, `Decision`, `Completeness`, `ProofDisposition`"),
        ("a faster path emits no less proof",
         "A faster path never emits less proof, explanation, or receipt material"),
        ("WorkObservation names the execution path", "FallbackAfterSpecializationRefusal"),
    ],
    "docs/18_DATA_ORIENTED_ECS.md": [
        ("the scalar scan kernel is the reference behavior",
         "The scalar `ScanKernel` is mandatory and is the reference behavior"),
        ("no optimized kernel is the sole oracle",
         "No optimized kernel is ever the sole oracle"),
        ("the mask width is not frozen", "The physical representation is not frozen"),
        ("AoSoA64 and fixed 64-row masks are not constitutional",
         "Nothing here constitutionalizes `u64`, 64 rows, `AoSoA64`, or one tile width forever"),
        ("mask algebra is domain bound",
         "legal only for masks over the same row domain, logical length, source cut"),
        ("NOT is bounded by the logical row length",
         "unused high bits cannot select nonexistent rows"),
        ("ColumnTile remains derived", "`ColumnTile` remains an earned physical derived layout"),
        ("late materialization preserves row order", "preserving source row order, row identity"),
        ("no unbounded materialization before selection",
         "It may not materialize an unbounded result and then claim the mask saved work"),
        ("nightly, unsafe intrinsics, and SIMD are not required",
         "V1 does not require nightly Rust, unsafe target intrinsics, a portable-SIMD dependency"),
        ("an optimized kernel requires qualification and fallback",
         "An unavailable optimized kernel falls back to scalar reference behavior"),
        ("speed alone does not qualify a kernel", "Wall-clock speed alone is insufficient"),
        ("WorkObservation stays observable", "the frozen obligation is that work remains observable"),
    ],
    "docs/14_RECEIPTS_AND_EXPLANATION.md": [
        ("receipt preserves the exact profile", "selected AuthenticatedHistoryProfile"),
        ("receipt preserves the exact witness policy", "selected WitnessPolicy"),
        ("receipt preserves the exact witness disposition",
         "selected WitnessPolicy exact WitnessDisposition"),
        ("the specialization manifest names the complete key", "complete SpecializationKey"),
        ("the manifest separates retained from discharged checks",
         "checks discharged from proven static facts"),
        ("the manifest is evidence, not authority",
         "It does not upgrade the residual into authority"),
        ("receipt preserves the integrity claim", "IntegrityClaim"),
        ("receipt preserves the authenticity claim", "AuthenticityClaim"),
        ("receipt preserves the freshness claim", "FreshnessClaim"),
        ("receipt preserves the rollback-resistance claim", "RollbackResistanceClaim"),
        ("receipt states success versus refusal", "success or refusal outcome"),
        ("no axis inferred from a profile name", "may infer an omitted claim axis"),
        ("a refusal is not a weaker success", "A refusal is never forced into one"),
        ("partial evidence never claims freshness after witness failure",
         "never carry a freshness or scoped rollback-resistance claim after witness verification failed"),
        ("no successful unanchored result for a required witness",
         "no successful unanchored result at all"),
        ("receipt preserves the proof disposition",
         "ProofDisposition                      passed through, never upgraded or collapsed"),
        ("absent and invalid optional witnesses never render identically",
         "An absent optional witness is a successful unanchored result"),
        ("no formatter upgrades or collapses a claim", "may upgrade or collapse a claim"),
    ],
    "docs/05_STORAGE_FBAT_AND_TILES.md": [
        ("commit knowledge is distinct from receipt completeness and reconciliation",
         "commit knowledge        KnownAbsent | KnownCommitted | Unknown"),
        ("receipt completeness axis", "receipt completeness    Complete | Incomplete"),
        ("reconciliation posture axis", "reconciliation posture  NotRequired | Pending"),
        ("post-commit publication is not an ordinary failure", "committed                        is NOT failed"),
        ("committed with an incomplete receipt is not failure or unknown",
         "committed, receipt incomplete    is NOT failed, and is NOT outcome unknown"),
        ("outcome unknown is not proof of non-commit",
         "outcome unknown                  is NOT proof of non-commit"),
        ("cancelled after admission is not proof of non-commit",
         "cancelled after admission        is NOT proof of non-commit"),
        ("reconciliation appends evidence and never rewrites durable history",
         "It never rewrites the original durable event"),
        ("no generic untyped publication envelope",
         "There is no `GenericPublicationCommand { kind: String, payload: Vec<u8> }`"),

        ("storage owns segment seals", "segment seal"),
        ("storage owns the history commitment", "global append accumulator"),
        ("coherence is not freshness", "Coherence is not freshness"),
    ],
    "docs/08_SYNCBAT_RUNTIME.md": [
        ("typed batch atomicity is capability-bounded",
         "only where the owning capability explicitly admits joint atomic publication"),
        ("a typed batch is not a distributed transaction",
         "not an arbitrary distributed transaction across unrelated ports or services"),
        ("input order and receipt order correspond",
         "Input order and receipt order correspond exactly"),
        ("an atomic group never publishes a partial durable subset",
         "partial durable subset   forbidden"),
        ("logical operation identity is stable across retries",
         "LogicalOperationId   stable across retries of the same requested operation"),
        ("AttemptId is unique per physical admission",
         "AttemptId            unique for every physical admission attempt"),
        ("attempt state, logical outcome, commit knowledge, and receipt completeness stay separate",
         "never collapse into one `Outcome`, boolean, or success/failure field"),
    ],
    "docs/09_BVISOR.md": [
        ("cancellation before admission is distinct",
         "CancelledBeforeAdmission   no physical execution was admitted"),
        ("cancellation after admission is not proof of non-commit",
         "`CancelledAfterAdmission` is never proof of non-commit"),
    ],
    "docs/10_WORLD_IMAGES_AND_PORTS.md": [
        ("port deadlines are absolute and monotonic",
         "accepts an absolute monotonic deadline"),
        ("a relative inactivity timeout is not a deadline",
         "A relative inactivity timeout is not an overall operation deadline"),
        ("the lowest extending mechanism enforces the remaining deadline",
         "The lowest mechanism capable of internally extending work enforces the remaining absolute deadline"),
        ("retry does not reset the overall deadline", "retry does not reset the overall deadline"),
        ("a per-attempt deadline stays bounded by the overall deadline",
         "remains bounded by the overall operation deadline"),
    ],
    "docs/16_IDENTITY_TIME_AND_NAVIGATION.md": [
        ("the cursor binds generation", "A cursor binds to its traversal identity, including store lineage, generation"),
        ("the cursor binds the source cut", "source cut, direction, selector, and filters"),
        ("a transplanted cursor fails closed", "fails closed rather than resuming against a traversal it never described"),
        ("the page carries a completeness posture", "page completeness or partiality posture"),
        ("the page carries a source stamp and work observation", "WorkObservation"),
    ],
    "docs/13_BATQL_CONTRACT.md": [
        ("GET lowers to a bounded seek", "GET           -> bounded seek"),
        ("CHILDREN OF lowers to one level", "CHILDREN OF   -> bounded one-level traversal"),
        ("SCAN UNDER lowers to a bounded subtree", "SCAN UNDER    -> bounded subtree traversal"),
        ("the limit applies before decode", "constrain discovery **before** unbounded decode"),
        ("scan-then-truncate is not bounded traversal",
         "scan everything -> decode everything -> truncate the returned vector"),
    ],
    "docs/06_MACBAT.md": [
        ("generated routing is exhaustive and typed", "exhaustive type-id dispatch"),
        ("generated routing fails closed", "Generated routing fails closed when a declared variant has no route"),
        ("route discovery is never ambient", "never ambient linker registration, startup constructors"),
        ("the application owns domain transition logic", "domain transition logic"),
        ("hand-authored raw dispatch is not the canonical path",
         "Applications do not hand-author raw `kind integer + byte payload + switch` dispatch"),
    ],
    "docs/12_TESTPAK.md": [
        ("Muterprater is not a standalone product",
         "It is not a standalone product package, a second semantic authority"),
        ("Lane A needs no per-candidate Rust compile", "No per-candidate Rust compile"),
        ("Lane B slots are test-profile only",
         "never enter an ordinary production artifact"),
        ("Lane B records activation", "Activation is evidence, never an assumption"),
        ("Lane C runs under real rustc semantics",
         "Candidates execute under real rustc semantics"),
        ("no homegrown evaluator equals rustc",
         "never claims a homegrown Rust evaluator equals rustc"),
        ("nextest executes and is not semantic authority",
         "Nextest executes; it is never semantic authority"),
        ("compiler-backed tooling is not retired by prose",
         "No coronation by architecture prose"),
        ("Survived requires activation", "A `Survived` verdict without activation evidence is invalid"),
        ("Killed requires baseline and activation",
         "A `Killed` verdict without a qualified baseline and activation evidence is invalid"),
        ("a timeout is not a kill", "A timeout is not a kill"),
        ("an unbuildable candidate is not a kill",
         "a compiler error produced by an invalid mutation is not evidence"),
        ("EquivalentCandidate is not equivalence proof",
         "a classification pending its witness, never equivalence proof"),
        ("nothing leaves the denominator silently",
         "Nothing silently leaves the denominator"),
        ("the full result distribution is published",
         "A mutation score exposes the full result distribution"),
        ("activation evidence distinguishes reached from selected",
         "mutant reached, mutant changed the executed semantic path"),
        ("an independent evidence route is required",
         "no independent evidence route          -> no semantic mutation qualification"),
        ("self-calling production code is not an oracle",
         "passing production evaluator against itself -> not an independent oracle"),
        ("no second full PakVM is required for independence",
         "Building a second full PakVM merely to call it independent is not independence"),
        ("candidates land outside tracked source",
         "It may not write candidates directly into `src/`, `tests/`, `spec/`, `docs/`, or `companion/`"),
        ("generation and promotion are separate",
         "Generation and promotion are two different operations with two different receipts"),
        ("the typed promotion denominator is conjunctive",
         "a missing member\nrefuses promotion"),
        ("an unclassified proof-policy change is refused",
         "An unclassified proof-policy change is refused"),
        ("Neutral requires parity evidence",
         'Requires parity evidence: "refactor" is not automatically Neutral'),
        ("a narrowed domain is Weakening",
         "narrows the tested domain or silently removes denominator units is Weakening"),
        ("a Weakening carries the full authority package",
         "hostile qualification proving the new boundary is enforced"),
        ("an issue link is not authority", "A generic issue link is not decision authority"),
        ("a commit message is not a receipt", "A commit message is not a weakening receipt"),
        ("the gate defends itself", "or the anti-weakening gate itself"),
        ("a disableable gate is not a gate",
         "A proof-policy gate that can be disabled as unclassified cleanup is not an anti-weakening gate"),
        ("bootstrap does not infer semantic diffs",
         "Bootstrap does not understand arbitrary semantic diffs and never claims to"),
    ],
    "docs/24_GAUNTLET.md": [
        ("D1 publication proof family is owned",
         "committed is never reported as an ordinary operation failure"),
        ("D1 attempt proof family is owned", "CancelledAfterAdmission does not prove non-commit"),
        ("D1 traversal proof family is owned",
         "valid pages concatenate to the one-shot reference traversal"),
        ("D1 routing proof family is owned", "every declared route is present exactly once"),
        ("D1 deadline proof family is owned", "retry does not reset the overall deadline"),
        ("the D2 specialization square is assigned", "Specialization proof square (DEC-073)"),
        ("the square routes to the lanes", "semantic mutation                                    Lane SemanticIr"),
        ("no lane is the sole proof route", "No lane is the sole proof route"),
    ],
    "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md": [
        ("docs/35 does not own whole-store history",
         "It does not own whole-store authenticated history"),
    ],
}


def check_document_law(root: Path, findings: list[str]) -> None:
    """Each named claim survives in its owning document."""
    for rel, markers in DOC_LAW_MARKERS.items():
        path = root / rel
        if not path.is_file():
            findings.append(f"missing {rel}")
            continue
        text = re.sub(r"\s+", " ", path.read_text(encoding="utf-8"))
        for label, marker in markers:
            if re.sub(r"\s+", " ", marker) not in text:
                findings.append(f"{rel}: {label} is absent")


# --- PakVM semantic ISA lineage audit (D4c1) --------------------------------
# AUDITOR half. This re-derives the admitted node specs from spec/pakvm_isa.rs
# INDEPENDENTLY of bootstrap/project.py and compares against the generated
# docs/07 projection. It shares no parsing, admission, rendering, or comparison
# code with the generator, and it authors no node semantics of its own: a policy
# constant living in both scripts would be common-mode invention, and parity
# between two copies of the same guess is not evidence.

A_ISA_SRC = "spec/pakvm_isa.rs"
A_ISA_DOC = "docs/07_PAKVM_ISA.md"
A_ISA_TABLE = re.compile(
    r"<!-- PAKVM-SEMANTIC-ISA:BEGIN[^>]*-->\n(.*?)\n<!-- PAKVM-SEMANTIC-ISA:END -->", re.S)
A_ISA_SIGS = re.compile(
    r"<!-- PAKVM-SIGNATURES:BEGIN[^>]*-->\n(.*?)\n<!-- PAKVM-SIGNATURES:END -->", re.S)


class IsaAdmissionFailure(Exception):
    pass


def _a_isa_method(src: str, method: str) -> str:
    head = src.find("pub const fn " + method + "(")
    if head < 0:
        raise IsaAdmissionFailure(f"spec/pakvm_isa.rs declares no {method}()")
    tail = src.find("\n    pub const fn ", head + 1)
    return src[head:] if tail < 0 else src[head:tail]


def _a_isa_dispatch(src: str, method: str, value_re: str) -> dict[str, str]:
    """node variant -> arm value, for a total match over PakVmNodeId."""
    text = _a_isa_method(src, method)
    table: dict[str, str] = {}
    for arm in re.finditer(r"((?:PakVmNodeId::\w+(?:\s*\|\s*)?\s*)+)=>\s*\{?\s*(" + value_re + ")",
                           text, re.S):
        for node in re.findall(r"PakVmNodeId::(\w+)", arm.group(1)):
            if node in table:
                raise IsaAdmissionFailure(f"{node} is matched twice by {method}()")
            table[node] = arm.group(2)
    return table


def _a_isa_block(src: str, marker: str) -> str:
    start = src.find(marker)
    if start < 0:
        raise IsaAdmissionFailure(f"spec/pakvm_isa.rs declares no {marker}")
    end = src.find("\n];", start)
    return src[start:end]


def _a_isa_policy(src: str, const: str, key: str) -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    for entry in re.split(r"\n    \w+Policy \{", "\n" + _a_isa_block(src, const)):
        fields = dict(re.findall(r"^\s{8}(\w+):\s*(.+?),?$", entry, re.M))
        name = fields.get(key, "")
        if name:
            out[name.split("::")[-1]] = fields
    return out


def _a_isa_rule(rule: str, declared: str, node: str, field: str) -> str:
    """One owner, never zero and never two. The auditor decides this from the
    same authored text the generator reads, by its own reading."""
    fixed = re.search(r"AlgebraConstant\(\s*\w+::(\w+)\s*\)", rule or "")
    given = re.search(r"Some\(\s*\w+::(\w+)\s*\)", declared or "")
    if fixed and given:
        raise IsaAdmissionFailure(f"{node}: {field} is declared by both its algebra and its class")
    if fixed:
        return fixed.group(1)
    if given:
        return given.group(1)
    raise IsaAdmissionFailure(f"{node}: {field} is declared by neither its algebra nor its class")


def pakvm_isa_views(root: Path) -> list[dict]:
    """Independently admitted PakVM node views, or IsaAdmissionFailure."""
    src = _spec_module_source(root, "pakvm_isa")
    listing = _a_isa_block(src, "pub const PAKVM_NODES")
    order = re.findall(r"PakVmNodeId::(\w+)", listing)
    if not order:
        raise IsaAdmissionFailure("spec/pakvm_isa.rs lists no PakVM nodes")
    algebras = _a_isa_dispatch(src, "algebra", r"PakVmAlgebra::\w+")
    classes = _a_isa_dispatch(src, "class", r"PakVmNodeClass::\w+")
    names = _a_isa_dispatch(src, "authored_name", r'"(?:[^"\\]|\\.)*"')
    operands = _a_isa_dispatch(src, "operand_sorts", r'"(?:[^"\\]|\\.)*"')
    results = _a_isa_dispatch(src, "result_sorts", r'"(?:[^"\\]|\\.)*"')
    surfaces = _a_isa_dispatch(src, "authoring_surface", r"NodeAuthoringSurface::\w+")
    algebra_pol = _a_isa_policy(src, "pub const PAKVM_ALGEBRA_POLICIES", "algebra")
    class_pol = _a_isa_policy(src, "pub const PAKVM_NODE_CLASS_POLICIES", "class")
    family_units: dict[str, list[str]] = {}
    for fam, arm in re.findall(r"WorkFormulaFamily::(\w+)\s*=>\s*\{?\s*&\[([^\]]*)\]",
                               _a_isa_block(src, "pub const fn units(self)") + "\n];", re.S):
        family_units[fam] = re.findall(r"WorkUnit::(\w+)", arm)
    plane_src = src[src.find("pub const fn work_plane"): src.find("pub const fn mandatory_work_unit")]
    planes = {alg: re.findall(r"WorkUnit::(\w+)", arm) for alg, arm in
              re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*&\[([^\]]*)\]", plane_src, re.S)}
    mand_src = _a_isa_method(src, "mandatory_work_unit")
    mandatory = {alg: (re.search(r"WorkUnit::(\w+)", val).group(1)
                       if "WorkUnit::" in val else None)
                 for alg, val in re.findall(
                     r"PakVmAlgebra::(\w+)\s*=>\s*(Some\([^)]*\)|None)", mand_src)}
    origins = dict(re.findall(r'PakVmAlgebra::(\w+)\s*=>\s*"([^"]*)"',
                              _a_isa_method(src, "source_origin")))
    views: list[dict] = []
    for node in order:
        alg = algebras.get(node, "").split("::")[-1]
        cls = classes.get(node, "").split("::")[-1]
        if not alg:
            raise IsaAdmissionFailure(f"{node}: algebra() states no authored algebra")
        if not cls:
            raise IsaAdmissionFailure(f"{node}: class() states no authored family")
        ap = algebra_pol.get(alg)
        cp = class_pol.get(cls)
        if ap is None:
            raise IsaAdmissionFailure(f"{node}: algebra {alg} declares no policy")
        if cp is None:
            raise IsaAdmissionFailure(f"{node}: class {cls} declares no policy")
        if cp.get("algebra", "").split("::")[-1] != alg:
            raise IsaAdmissionFailure(f"{node}: class {cls} is filed under a foreign algebra")
        view = {"node": node, "algebra": alg, "class": cls}
        for field in ("effect", "capability", "evidence"):
            view[field] = _a_isa_rule(ap.get(field, ""), cp.get(field, "None"), node, field)
        lowering = _a_isa_rule(ap.get("lowering", ""), "None", node, "lowering")
        if lowering != "PublicSemanticIdentity":
            raise IsaAdmissionFailure(f"{node}: proposes the non-semantic lowering {lowering}")
        view["lowering"] = lowering
        family = cp.get("work_formula", "").split("::")[-1]
        units = family_units.get(family)
        if not units:
            raise IsaAdmissionFailure(f"{node}: work family {family} accounts in no authored unit")
        plane = planes.get(alg, [])
        stray = [u for u in units if u not in plane]
        if stray:
            raise IsaAdmissionFailure(
                f"{node}: work family {family} accounts in {stray[0]}, off the {alg} plane")
        need = mandatory.get(alg)
        if need and need not in units:
            raise IsaAdmissionFailure(f"{node}: work family {family} omits {need}")
        view["work_formula"] = family
        view["units"] = units
        view["boundedness"] = cp.get("boundedness", "").split("::")[-1]
        for field, table in (("name", names), ("operands", operands), ("results", results)):
            raw = table.get(node)
            if raw is None:
                raise IsaAdmissionFailure(f"{node}: states no authored {field}")
            text = raw[1:-1]
            if not text.strip():
                raise IsaAdmissionFailure(f"{node}: states an empty {field}")
            view[field] = text
        view["origin"] = origins.get(alg, "")
        if not view["origin"]:
            raise IsaAdmissionFailure(f"{node}: {alg} names no source origin")
        surface = surfaces.get(node, "").split("::")[-1]
        if not surface:
            raise IsaAdmissionFailure(f"{node}: authoring_surface() names no lawful producer")
        view["surface"] = surface
        views.append(view)
    return views


def pakvm_isa_findings(root: Path) -> list[str]:
    """Lineage, partition, and projection-drift findings. Never a crash: a
    hostile edit must produce a diagnostic, not a traceback."""
    out: list[str] = []
    try:
        views = pakvm_isa_views(root)
    except IsaAdmissionFailure as exc:
        return [f"spec/pakvm_isa.rs: PakVM node does not admit: {exc}"]
    except (ValueError, KeyError, AttributeError) as exc:
        return [f"spec/pakvm_isa.rs: PakVM semantic ISA is unreadable: {exc}"]
    ids = [v["node"] for v in views]
    if len(set(ids)) != len(ids):
        out.append("spec/pakvm_isa.rs: a PakVM node id is listed more than once")
    seen: dict[str, str] = {}
    for v in views:
        if v["name"] in seen:
            out.append(f"spec/pakvm_isa.rs: PakVM nodes {seen[v['name']]} and {v['node']} "
                       f"share the authored name {v['name']!r}")
        seen[v["name"]] = v["node"]
    # A node whose only lawful producer is canonical ProgramImage authoring is
    # not part of the frozen BatQL V1 language. Its authored name appearing in
    # docs/13's semantic-algebra MEMBER LISTS (the first line under each ###
    # heading; explanatory prose below may lawfully discuss it) would re-admit
    # it as source-language law without the language-amendment process.
    d13 = (root / "docs/13_BATQL_CONTRACT.md").read_text(encoding="utf-8")
    m = re.search(r"## Semantic algebras\n(.*?)\n## ", d13, re.S)
    if m is None:
        out.append("docs/13 declares no semantic-algebras inventory")
    else:
        members = " ".join(h.group(1) for h in re.finditer(r"###[^\n]*\n+([^\n]+)", m.group(1)))
        for v in views:
            if v["surface"] != "BatQlLowered" and re.search(
                    r"\b" + re.escape(v["name"]) + r"\b", members):
                out.append(
                    f"docs/13 lists {v['name']!r} as a BatQL semantic-algebra member, but its "
                    f"authoring surface is {v['surface']} (spec/pakvm_isa.rs): source-language "
                    f"membership enters through the language-amendment law, never to make "
                    f"inventories agree")
    # The work planes partition the authored unit vocabulary. Derived from the
    # spec's own plane declarations: this states no plane membership of its own.
    src = _spec_module_source(root, "pakvm_isa")
    plane_src = src[src.find("pub const fn work_plane"): src.find("pub const fn mandatory_work_unit")]
    planes = {alg: re.findall(r"WorkUnit::(\w+)", arm) for alg, arm in
              re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*&\[([^\]]*)\]", plane_src, re.S)}
    declared = re.findall(r"^    (\w+),$",
                          src[src.find("pub enum WorkUnit"): src.find("\n}", src.find("pub enum WorkUnit"))],
                          re.M)
    for unit in declared:
        owners = [a for a, us in planes.items() if unit in us]
        if len(owners) > 1:
            out.append(f"work unit {unit} sits on more than one algebra plane: {', '.join(sorted(owners))}")
        elif not owners:
            out.append(f"work unit {unit} sits on no algebra plane and no node can account in it")
    for unit in declared:
        if not any(unit in v["units"] for v in views):
            out.append(f"authored work unit {unit} is accounted by no admitted PakVM node")
    # The effect-posture partition, verified INDEPENDENTLY of the Rust admission
    # that also enforces it. Until this rule existed the biconditional lived only
    # in spec/pakvm_isa.rs: an algebra policy declaring an effectful query
    # dataflow was caught here as projection drift, and regenerating the
    # projection made the auditor accept it outright. Drift is not a law, and a
    # finding that disappears when someone reruns the generator was never
    # enforcing anything.
    #
    # This states no posture of its own. The typed ISA owns which posture each
    # algebra carries; the auditor checks that the biconditional it implies holds
    # and that both sides are inhabited.
    for v in views:
        effectful = v["effect"] == "Effectful"
        in_effect_algebra = v["algebra"] == "Effect"
        if effectful and not in_effect_algebra:
            out.append(f"PakVM node {v['node']} admits as Effectful outside the Effect "
                       f"algebra, so an effect could enter a pure query image")
        if in_effect_algebra and not effectful:
            out.append(f"PakVM node {v['node']} is an Effect-algebra node that does not "
                       f"admit as Effectful, so it would pass the pure-query validator")
    # Binding a predicate to a typed set only means something if the set is
    # inhabited. A rule quantified over an empty set is vacuously true and can
    # never fire, so query_program_with_effect_instruction_is_rejected (DEC-050)
    # would pass forever while proving nothing. Both sides must exist; neither
    # count is policy, and no number is asserted here.
    if not [v for v in views if v["effect"] == "Effectful"]:
        out.append("no PakVM node admits with the Effectful posture, so "
                   "query_program_with_effect_instruction_is_rejected is vacuous")
    if not [v for v in views if v["effect"] != "Effectful"]:
        out.append("every PakVM node admits as Effectful, so a pure query image "
                   "could contain no node at all")
    # An opcode never enters the semantic layer, by any route.
    if re.search(r"#\[repr\(", src):
        out.append("spec/pakvm_isa.rs: a #[repr] fixes a semantic node's representation")
    body = src[src.find("pub enum PakVmNodeId"): src.find("\n}", src.find("pub enum PakVmNodeId"))]
    if re.search(r"=\s*-?\d+", body):
        out.append("spec/pakvm_isa.rs: PakVmNodeId assigns an explicit discriminant")
    if re.search(r"PakVmNodeId[^\n]*\bas\s+[iu](8|16|32|64|size)\b", src):
        out.append("spec/pakvm_isa.rs: a PakVmNodeId is cast to an integer")
    # The generated projection carries exactly the admitted views.
    doc = (root / A_ISA_DOC).read_text(encoding="utf-8")
    table = A_ISA_TABLE.search(doc)
    if not table:
        out.append(f"{A_ISA_DOC}: no generated PAKVM-SEMANTIC-ISA block")
    else:
        rows = [r for r in table.group(1).splitlines() if r.startswith("| ") and "| ---" not in r]
        rows = [r for r in rows if not r.startswith("| Node ")]
        if len(rows) != len(views):
            out.append(f"{A_ISA_DOC}: projects {len(rows)} PakVM nodes, {len(views)} admit")
        for row, v in zip(rows, views, strict=False):
            cells = [c.strip() for c in row.strip().strip("|").split("|")]
            want = [v["name"], v["algebra"], v["class"], v["effect"], v["capability"],
                    v["boundedness"], v["work_formula"], "/".join(v["units"]), v["evidence"]]
            if cells != want:
                out.append(f"{A_ISA_DOC}: projected row for {v['name']!r} drifted from its admitted spec")
                break
    sigs = A_ISA_SIGS.search(doc)
    if not sigs:
        out.append(f"{A_ISA_DOC}: no generated PAKVM-SIGNATURES block")
    else:
        text = sigs.group(1)
        for v in views:
            if f"    operands  {v['operands']}" not in text:
                out.append(f"{A_ISA_DOC}: signature projection omits the operand sorts of {v['name']!r}")
                break
            if f"    results   {v['results']}" not in text:
                out.append(f"{A_ISA_DOC}: signature projection omits the result sorts of {v['name']!r}")
                break
    return out


A_FW_SRC = "spec/syncbat_firewall.rs"
A_FW_ARCH = "spec/architecture.rs"
A_FW_DOC = "docs/08_SYNCBAT_RUNTIME.md"


def _a_fw_block(doc: str, name: str) -> str | None:
    m = re.search(r"<!-- " + name + r":BEGIN[^>]*-->\n(.*?)\n<!-- " + name + r":END -->", doc, re.S)
    return m.group(1) if m else None


class FirewallAdmissionFailure(Exception):
    pass


def _a_fw_variants(src: str, enum: str, where: str) -> list[str]:
    """The authored variants of one enum.

    This is the reading rustc cannot perform for us. A variant omitted from the
    `&[...]` listing below compiles perfectly and disappears from every law that
    iterates the listing: seedcheck would go on proving things about the
    authorities it was handed, and never mention the one it was not."""
    head = src.find(f"pub enum {enum} {{")
    if head < 0:
        raise FirewallAdmissionFailure(f"{where}: declares no {enum}")
    return re.findall(r"^\s{4}(\w+),$", src[head: src.find("\n}", head)], re.M)


def _a_fw_listing(src: str, const: str, variant: str, where: str) -> list[str]:
    head = src.find(f"pub const {const}")
    if head < 0:
        raise FirewallAdmissionFailure(f"{where}: declares no {const}")
    return re.findall(variant + r"::(\w+)", src[head: src.find("\n];", head)])


def _a_fw_method(src: str, name: str, where: str) -> str:
    head = src.find(f"pub const fn {name}(self)")
    if head < 0:
        raise FirewallAdmissionFailure(f"{where}: declares no {name}()")
    end = src.find("\n    }", head)
    return re.sub(r"//[^\n]*", "", src[head: end if end > 0 else len(src)])


def syncbat_firewall_views(root: Path) -> dict:
    """The firewall as this auditor reads it, from the typed owners alone.

    Independently derived: it shares no parsing, no table, and no constant with
    `project.py`. It carries no plane ownership map and no crossing whitelist of
    its own, because a checker holding its own copy of the answer agrees with
    itself and calls that verification."""
    arch = _spec_module_source(root, "architecture")
    src = (root / A_FW_SRC).read_text(encoding="utf-8")
    planes = _a_fw_variants(arch, "SyncBatPlane", A_FW_ARCH)
    if not planes:
        raise FirewallAdmissionFailure(f"{A_FW_ARCH}: SyncBatPlane authors no plane")
    m = re.search(r"pub const ALL: &'static \[SyncBatPlane\] = &\[(.*?)\];", arch, re.S)
    if not m:
        raise FirewallAdmissionFailure(f"{A_FW_ARCH}: declares no SyncBatPlane::ALL")
    listed_planes = re.findall(r"SyncBatPlane::(\w+)", m.group(1))
    sentences: dict[str, str] = {}
    for arm in _a_fw_method(arch, "authored_ownership", A_FW_ARCH).split("\n"):
        m = re.search(r'SyncBatPlane::(\w+)\s*=>\s*"([^"]*)"', arm)
        if m:
            sentences[m.group(1)] = m.group(2)
    authorities = _a_fw_variants(src, "SyncBatAuthority", A_FW_SRC)
    if not authorities:
        raise FirewallAdmissionFailure(f"{A_FW_SRC}: SyncBatAuthority authors no authority")
    listed = _a_fw_listing(src, "SYNCBAT_AUTHORITIES", "SyncBatAuthority", A_FW_SRC)
    owners: dict[str, str] = {}
    for arm in _a_fw_method(src, "owner", A_FW_SRC).split(","):
        if "=>" not in arm:
            continue
        left, right = arm.split("=>", 1)
        plane = re.search(r"SyncBatPlane::(\w+)", right)
        if not plane:
            continue
        for name in re.findall(r"SyncBatAuthority::(\w+)", left):
            if name in owners:
                raise FirewallAdmissionFailure(f"owner() answers twice for {name}")
            owners[name] = plane.group(1)
    semantic = set(re.findall(
        r"SyncBatAuthority::(\w+)",
        _a_fw_method(src, "requires_semantic_origin", A_FW_SRC)))
    head = src.find("pub const SYNCBAT_LEGAL_CROSSINGS")
    if head < 0:
        raise FirewallAdmissionFailure(f"{A_FW_SRC}: declares no SYNCBAT_LEGAL_CROSSINGS")
    crossings = []
    for chunk in src[head: src.find("\n];", head)].split("SyncBatCrossing {")[1:]:
        got = dict(re.findall(r"(\w+):\s*(?:\w+::)?(\w+|\"[^\"]*\")", chunk))
        crossings.append({
            "from": got.get("from", ""), "to": got.get("to", ""),
            "carries": got.get("carries", ""), "posture": got.get("posture", ""),
            "law": got.get("law", "").strip('"'),
        })
    return {"planes": planes, "listed_planes": listed_planes, "sentences": sentences,
            "authorities": authorities, "listed": listed, "owners": owners,
            "semantic": semantic, "crossings": crossings}


def syncbat_firewall_findings(root: Path) -> list[str]:
    """Firewall closure, ownership, crossing legality, and projection findings.

    Every rule below reads the typed owner. None reads the projection to decide
    a semantic question, which is why regenerating docs/08 cannot quiet one of
    them: that failure mode is not hypothetical here, it is the exact defect
    this campaign found in the PakVM effect biconditional, where an unlawful
    algebra policy was reported as drift and a rerun of the generator turned the
    auditor green."""
    out: list[str] = []
    try:
        v = syncbat_firewall_views(root)
    except FirewallAdmissionFailure as exc:
        return [f"{A_FW_SRC}: the SyncBat firewall does not admit: {exc}"]
    except (ValueError, KeyError, AttributeError, IndexError) as exc:
        return [f"{A_FW_SRC}: the SyncBat firewall is unreadable: {exc}"]
    planes, authorities = v["planes"], v["authorities"]
    # Closure. The listings must name exactly the authored variants: an omission
    # is invisible to rustc and silently narrows every law that iterates them.
    for name in planes:
        if name not in v["listed_planes"]:
            out.append(f"{A_FW_ARCH}: plane {name} is authored but SyncBatPlane::ALL omits it, "
                       f"so no law that walks the planes can see it")
    for name in v["listed_planes"]:
        if name not in planes:
            out.append(f"{A_FW_ARCH}: SyncBatPlane::ALL lists {name}, which SyncBatPlane does not author")
    for name in authorities:
        if name not in v["listed"]:
            out.append(f"{A_FW_SRC}: authority {name} is authored but SYNCBAT_AUTHORITIES omits it, "
                       f"so every law that walks the authorities skips it")
    for name in v["listed"]:
        if name not in authorities:
            out.append(f"{A_FW_SRC}: SYNCBAT_AUTHORITIES lists {name}, "
                       f"which SyncBatAuthority does not author")
    if len(set(v["listed"])) != len(v["listed"]):
        out.append(f"{A_FW_SRC}: SYNCBAT_AUTHORITIES lists an authority more than once")
    # Every plane authors its sentence, and every sentence is its own.
    for name in planes:
        if not v["sentences"].get(name):
            out.append(f"{A_FW_ARCH}: plane {name} authors no ownership sentence")
    said: dict[str, str] = {}
    for name in planes:
        text = v["sentences"].get(name, "")
        if text and text in said:
            out.append(f"{A_FW_ARCH}: planes {said[text]} and {name} author the same "
                       f"ownership sentence, so the firewall cannot tell them apart")
        said[text] = name
    # One owner per authority, and that owner is a plane.
    for name in authorities:
        owner = v["owners"].get(name)
        if not owner:
            out.append(f"{A_FW_SRC}: authority {name} names no owning plane, so nobody "
                       f"can refuse its misuse")
        elif owner not in planes:
            out.append(f"{A_FW_SRC}: authority {name} is owned by {owner}, which is no SyncBat plane")
    # Inhabitance. A plane that owns nothing is a name in a document, and its
    # ownership sentence enforces nothing. No count is asserted here.
    for name in planes:
        if not [a for a in authorities if v["owners"].get(a) == name]:
            out.append(f"{A_FW_SRC}: plane {name} owns no authority, so its authored "
                       f"ownership sentence constrains nothing")
    # Crossing legality, recomputed rather than consulted.
    seen: set[tuple[str, str, str]] = set()
    for c in v["crossings"]:
        frm, to, carries = c["from"], c["to"], c["carries"]
        where = f"the {frm} -> {to} crossing"
        for plane in (frm, to):
            if plane not in planes:
                out.append(f"{A_FW_SRC}: {where} names {plane!r}, which is no SyncBat plane")
        if frm == to:
            out.append(f"{A_FW_SRC}: {where} does not leave its plane; that is an internal transition")
        if carries not in authorities:
            out.append(f"{A_FW_SRC}: {where} carries {carries!r}, which is no authored authority")
            continue
        owner = v["owners"].get(carries)
        if owner and owner != frm:
            out.append(f"{A_FW_SRC}: {where} lets {frm} exercise {carries}, which {owner} owns")
        if not c["law"]:
            out.append(f"{A_FW_SRC}: {where} states no authored law")
        key = (frm, to, carries)
        if key in seen:
            out.append(f"{A_FW_SRC}: {where} carrying {carries} is declared twice")
        seen.add(key)
    if not v["crossings"]:
        out.append(f"{A_FW_SRC}: no lawful crossing is declared, so the firewall admits nothing "
                   f"and every crossing rule is vacuous")
    # ISA-only execution. Semantics has one owner, so every authority that must
    # name a PakVM origin answers to the same plane. Which plane that is stays
    # the spec's business: reading it out of the ownership prose would be the
    # archaeology this file refuses to do.
    origin_owners = {v["owners"].get(a) for a in v["semantic"] if a in authorities}
    if len(origin_owners) > 1:
        out.append(f"{A_FW_SRC}: authorities requiring a PakVM origin answer to more than one "
                   f"plane ({', '.join(sorted(str(o) for o in origin_owners))}), so semantic "
                   f"execution has no single owner")
    if not v["semantic"]:
        out.append(f"{A_FW_SRC}: no authority requires a PakVM origin, so ISA-only execution "
                   f"is a sentence rather than a condition on a value")
    for name in v["semantic"]:
        if name not in authorities:
            out.append(f"{A_FW_SRC}: requires_semantic_origin() names {name}, "
                       f"which is no authored authority")
    # A semantic authority must name its PakVM origin BECAUSE it leaves the plane
    # that owns the ISA. One that crosses nothing requires an origin for a value
    # no other plane can ever receive, which makes the requirement ceremony and
    # the carrier a dead end. This is the rule that notices a lawful crossing
    # being deleted: without it the whitelist can quietly shrink, the projection
    # regenerates to match, and every gate stays green while an execution route
    # the ISA depends on stops existing.
    for name in sorted(v["semantic"]):
        carried_required = [c for c in v["crossings"]
                            if c["carries"] == name and c["posture"] == "Required"]
        if name in authorities and not carried_required:
            out.append(f"{A_FW_SRC}: {name} must name a PakVM origin but no required crossing "
                       f"carries it off {v['owners'].get(name)}, so admitted node meaning has no "
                       f"lawful route out of the plane that owns it")
    # Requiredness as connectivity. Permission says a door may exist; requiredness
    # says the accepted machine cannot complete its turn if this door is bricked
    # over. The turn is a round trip: the logical decider authorizes work outward
    # and every plane returns its result or evidence back to that decider. Both
    # roots are DERIVED from authored ownership, never named here as policy.
    posture_values = {c["posture"] for c in v["crossings"]}
    if posture_values - {"Required", "Optional"}:
        out.append(f"{A_FW_SRC}: a crossing states no requiredness posture; absence is not "
                   f"silently optional")
    # Only LEGAL required edges carry connectivity. An illegal substitute -- a
    # crossing whose sender does not own what it carries -- is flagged by the
    # ownership rule and must not be allowed to launder a missing required route
    # by appearing to reconnect the graph.
    required = [c for c in v["crossings"]
                if c["posture"] == "Required" and v["owners"].get(c["carries"]) == c["from"]]
    logical_root = v["owners"].get("LogicalResult")
    source = v["owners"].get("CompositionAndInstanceIdentity")
    if not logical_root:
        out.append(f"{A_FW_SRC}: no plane owns LogicalResult, so the firewall has no logical "
                   f"root to require routes toward")
    elif not source:
        out.append(f"{A_FW_SRC}: no plane owns CompositionAndInstanceIdentity, so the firewall "
                   f"has no composition source to exempt from inbound authorization")
    else:
        def reaches(start: str, goal: str) -> bool:
            seen = {start}
            changed = True
            while changed:
                changed = False
                for c in required:
                    if c["from"] in seen and c["to"] not in seen:
                        seen.add(c["to"])
                        changed = True
            return goal in seen
        for plane in planes:
            if not reaches(plane, logical_root):
                out.append(f"{A_FW_SRC}: plane {plane} cannot reach the logical root "
                           f"{logical_root} by required crossings, so its result or evidence has "
                           f"no lawful route home; a required route is missing or was downgraded "
                           f"to optional")
            if plane != source and not reaches(logical_root, plane):
                out.append(f"{A_FW_SRC}: the logical root {logical_root} cannot reach plane "
                           f"{plane} by required crossings, so it cannot authorize the work that "
                           f"plane owns; a required route is missing or was downgraded to optional")
    # An Optional crossing must be genuinely optional: some alternate required
    # route must carry the same authority. The sole carrier of an authority can
    # never be optional, because its absence severs the delivery outright.
    for c in v["crossings"]:
        if c["posture"] == "Optional" and not [
                o for o in v["crossings"] if o is not c
                and o["carries"] == c["carries"] and o["posture"] == "Required"]:
            out.append(f"{A_FW_SRC}: the {c['from']} -> {c['to']} crossing is optional but is "
                       f"the sole route carrying {c['carries']}; deleting it would sever the "
                       f"machine, so it cannot be optional")
    # The projections carry exactly the admitted facts.
    doc = (root / A_FW_DOC).read_text(encoding="utf-8")
    own = _a_fw_block(doc, "SYNCBAT-PLANE-OWNERSHIP")
    if own is None:
        out.append(f"{A_FW_DOC}: no generated SYNCBAT-PLANE-OWNERSHIP block")
    else:
        want = ["```text"] + [v["sentences"].get(p, "") for p in v["listed_planes"]] + ["```"]
        if own.splitlines() != want:
            out.append(f"{A_FW_DOC}: the projected ownership sentences drifted from "
                       f"{A_FW_ARCH}")
    auth = _a_fw_block(doc, "SYNCBAT-AUTHORITIES")
    if auth is None:
        out.append(f"{A_FW_DOC}: no generated SYNCBAT-AUTHORITIES block")
    else:
        rows = [r for r in auth.splitlines() if r.startswith("| ")
                and "| ---" not in r and not r.startswith("| Authority ")]
        want = [f"| {a} | {v['owners'].get(a, '')} | {'yes' if a in v['semantic'] else 'no'} |"
                for a in v["listed"]]
        if rows != want:
            out.append(f"{A_FW_DOC}: the projected authority inventory drifted from {A_FW_SRC}")
    cross = _a_fw_block(doc, "SYNCBAT-CROSSINGS")
    if cross is None:
        out.append(f"{A_FW_DOC}: no generated SYNCBAT-CROSSINGS block")
    else:
        rows = [r for r in cross.splitlines() if r.startswith("| ")
                and "| ---" not in r and not r.startswith("| From ")]
        want = [f"| {c['from']} | {c['to']} | {c['carries']} | {c['posture']} | {c['law']} |"
                for c in v["crossings"]]
        if rows != want:
            out.append(f"{A_FW_DOC}: the projected crossing whitelist drifted from {A_FW_SRC}")
    # A hand-authored copy of a projected inventory is not documentation; it is a
    # second answer that no generator maintains and no reader can distinguish
    # from the first.
    outside = doc
    for name in ("SYNCBAT-PLANE-OWNERSHIP", "SYNCBAT-AUTHORITIES", "SYNCBAT-CROSSINGS"):
        outside = re.sub(r"<!-- " + name + r":BEGIN[^>]*-->\n.*?\n<!-- " + name + r":END -->",
                         "", outside, flags=re.S)
    for plane in planes:
        text = v["sentences"].get(plane, "")
        if text and text in outside:
            out.append(f"{A_FW_DOC}: the ownership sentence for {plane} is also hand-authored "
                       f"outside the generated block, competing with the projection")
    return out


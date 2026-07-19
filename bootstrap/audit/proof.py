"""Proof-row, witness, and proof-target audits.

Owns the substrate/specialization/proof-policy laws (5.5D1-D3), witness
reference resolution, the LEG-023/019/043/081 obligation proofs, the typed
proof-identity catalog and its retirement lineage, and the generic
proof-target resolution. docs/24 owns meaning; spec/proof/inventory.rs owns
membership.
"""
from __future__ import annotations

import re
from pathlib import Path

from .guarantees import G_REF, decision_rows, gate_inventory, guarantee_index

# --- 5.5D1 substrate law: sharpened LEG statements may not be weakened -------
# Each phrase is the load-bearing clause of its obligation. Losing one silently
# returns the substrate to the ambiguity higher layers had to work around.
D1_LEG_CLAUSES = {
    "LEG-028": [
        "constrain discovery before decode",
        "bind store lineage generation source cut direction selector and filters",
        "fails closed",
    ],
    "LEG-029": [
        "WorkObservation",
        "concatenated valid pages equal a one-shot reference traversal",
        "no page claims completeness merely by filling its requested size",
    ],
    "LEG-037": [
        "only where the admitting capability owns their common authority boundary",
        "input order and receipt order correspond exactly",
        "never publishes a partial durable subset",
        "not an arbitrary distributed transaction",
    ],
    "LEG-042": [
        "distinguishes cancellation before admission from cancellation after admission",
        "never read as proof of non-commit",
        "receives a new AttemptId without erasing prior attempt evidence",
    ],
    "LEG-046": [
        "closed and exhaustive",
        "fails closed",
        "never ambient linker registration startup constructors or process-global registries",
        "owns structure and glue rather than application domain transition logic",
    ],
    "LEG-066": [
        "deadline",
        "a relative inactivity timeout never substitutes for an absolute monotonic deadline",
        "the lowest mechanism able to extend work enforces the remaining absolute deadline",
        "a retry never resets the overall deadline",
    ],
}


def d1_substrate_findings(root: Path) -> list[str]:
    """The sharpened substrate clauses survive in their owning LEG rows."""
    src = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    laws = dict(re.findall(r'LegacyObligation \{ id: "(LEG-\d+)", law: "([^"]+)"', src))
    out = []
    for lid, clauses in D1_LEG_CLAUSES.items():
        law = laws.get(lid)
        if law is None:
            out.append(f"{lid} is absent from spec/legacy_obligations/inventory.rs")
            continue
        for clause in clauses:
            if clause not in law:
                out.append(f"{lid} law no longer states: {clause}")
    return out


# --- 5.5D2 adaptive residual specialization (DEC-073) -----------------------
# Bootstrap enforces the CONTRACT. It never executes PakVM, specializes a
# program, runs a scan, measures a benchmark, tests SIMD, compares residual
# results, or validates machine code, and must never claim to.
D2_KEY_COMPONENTS = [
    "ProgramImageId",
    "WorldImageId",
    "PakVM ISA version",
    "specializer implementation or semantic version identity",
    "residual-plan format identity",
    "schema identities",
    "layout identities",
    "numeric profile identities",
    "operator and qualified-kernel interface identities",
    "qualified-kernel implementation identities where pinned",
    "capability-envelope identity",
    "target profile",
    "qualified CPU-feature profile where relevant",
]
D2_FORBIDDEN_PUBLIC_SYNTAX = ("SPECIALIZE", "AoSoA", "SIMD")


def specialization_key_findings(root: Path) -> list[str]:
    """Every required key component is named. One finding per missing component,
    so removing one marker cannot accidentally prove several unrelated cases."""
    text = re.sub(r"\s+", " ", (root / "docs/07_PAKVM_ISA.md").read_text(encoding="utf-8"))
    out = []
    for component in D2_KEY_COMPONENTS:
        if re.sub(r"\s+", " ", component) not in text:
            out.append(f"specialization key omits {component}")
    return out


def specialization_findings(root: Path) -> list[str]:
    """DEC-073 structure: class, gates, and the no-public-syntax rule."""
    out: list[str] = []
    rows = {r["id"]: r for r in decision_rows(root)}
    dec = rows.get("DEC-073")
    if dec is None:
        out.append("DEC-073 is absent from spec/dispositions/inventory.rs")
        return out
    if dec["class"] != "Capability":
        out.append(f"DEC-073 class {dec['class']} is not Capability")
    if dec["gate_names"] != ["G4", "G5", "G8", "G9"]:
        out.append(f"DEC-073 gates {dec['gate_names']} != [G4, G5, G8, G9]")
    # The execution path never changes program meaning, so no public syntax.
    grammar = (root / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")
    for token in D2_FORBIDDEN_PUBLIC_SYNTAX:
        if re.search(r"^\s*\|?\s*\"" + token + r"\"", grammar, re.M):
            out.append(f"public BatQL syntax introduced for {token}")
    return out


# --- 5.5D3 proof-policy anti-weakening and mutation (DEC-015, DEC-074) ------
# Bootstrap proves typed and documentary structure. It compiles no mutant,
# activates no slot, runs no nextest, invokes no rustc, compares no
# cargo-mutants run, kills nothing, proves no equivalence, promotes nothing, and
# classifies no real diff semantically. It must never claim to.
D3_LANES = ["SemanticIr", "SelectableCompiled", "CompilerBacked"]
D3_RESULTS = [
    "Killed", "Survived", "NotActivated", "Refused", "Unbuildable",
    "TimedOut", "InfrastructureFailure", "EquivalentCandidate",
]
D3_CHANGE_CLASSES = ["Strengthening", "Neutral", "Weakening"]
D3_POLICY_SURFACES = [
    "MutationThreshold", "WaiverLogic", "EquivalentMutantRule", "HostileFixture",
    "CandidatePromotion", "ProofReceiptSchema", "ProofFreshness", "ReleaseBlocking",
    "TestSelection", "AssuranceRequirement", "GateQualification",
]
# Never a kill and never a survival: an invalid mutation that fails to compile
# proves nothing about the suite, and an unreached mutant was never tested.
D3_NEVER_KILLED = ["NotActivated", "Refused", "Unbuildable", "TimedOut", "InfrastructureFailure"]
D3_FORBIDDEN_CANDIDATE_ROOTS = ["src/", "tests/", "spec/", "docs/", "companion/"]
D3_BANNED_RESULT_WORDS = ("pass/fail", "killed/not-killed", "success/error")


def _arch_enum(root: Path, name: str, rel: str) -> list[str]:
    src = (root / rel).read_text(encoding="utf-8")
    m = re.search(r"pub enum " + re.escape(name) + r" \{(.*?)\n\}", src, re.S)
    return re.findall(r"^\s{4}(\w+),", m.group(1), re.M) if m else []


def proof_policy_findings(root: Path) -> list[str]:
    """DEC-074 identity plus the frozen mutation and policy vocabularies."""
    out: list[str] = []
    rows = {r["id"]: r for r in decision_rows(root)}
    dec = rows.get("DEC-074")
    if dec is None:
        out.append("DEC-074 is absent from spec/dispositions/inventory.rs")
    else:
        if dec["class"] != "Enforcement":
            out.append(f"DEC-074 class {dec['class']} is not Enforcement")
        for gate in ("G3", "G9"):
            if gate not in dec["gate_names"]:
                out.append(f"DEC-074 does not name gate {gate}")
    d15 = rows.get("DEC-015")
    if d15 is None or "Mutation testing only inside TestPak" not in d15["subject"] + str(d15):
        src = (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")
        if "Mutation testing only inside TestPak" not in src:
            out.append("DEC-015 no longer confines mutation testing to TestPak")
    # The lane and result vocabularies moved to their typed owner (5.5E3f);
    # their facts and classifications are reconstructed by mutation_findings.
    for name, want, rel in (
            ("MutationLane", D3_LANES, "spec/mutation/types.rs"),
            ("MutationResult", D3_RESULTS, "spec/mutation/types.rs"),
            ("ProofPolicyChangeClass", D3_CHANGE_CLASSES, "spec/proof/policy.rs"),
            ("ProofPolicySurface", D3_POLICY_SURFACES, "spec/proof/policy.rs")):
        got = _arch_enum(root, name, rel)
        if not got:
            out.append(f"{rel} declares no {name}")
        elif got != want:
            out.append(f"{name} variants {got} != frozen {want}")
    src = (root / "spec/sprouting/inventory.rs").read_text(encoding="utf-8")
    if "target/muterprater/candidates/" not in src:
        out.append("no candidate output root is declared outside tracked source")
    m = re.search(r"pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS:[^=]*=\s*&\[(.*?)\];", src, re.S)
    forbidden = re.findall(r'"([^"]+)"', m.group(1)) if m else []
    for rootdir in D3_FORBIDDEN_CANDIDATE_ROOTS:
        if rootdir not in forbidden:
            out.append(f"candidate generation is not forbidden from writing {rootdir}")
    # The promotion denominator moved to its typed owner (5.5E3i);
    # promotion_findings reconstructs it and refuses the raw array's return.
    # No YAML proof-policy language, no public BatQL mutation syntax.
    grammar = (root / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")
    for token in ("MUTATE", "PROMOTE"):
        if re.search(r"^\s*\|?\s*\"" + token + r"\"", grammar, re.M):
            out.append(f"public BatQL syntax introduced for {token}")
    return out


# --- Witness reference resolution (5.5D4b) ----------------------------------
# docs/24 owns proof-row identity and executable meaning. docs/21 projects the
# required witness IDs per legacy obligation. The projection is audited but is
# never a second authority: a meaning changes in docs/24 or nowhere.
#
# Ownership is read from the explicit binding in the live docs/24 block header,
# never inferred from witness-name keywords or nearby headings.
W_OWNER_BLOCK = re.compile(
    r"Required witnesses \(proof owner ([^;]+); gates ([^;)]*)((?:;[^)]*)?)\)"
    r"([^\n:]*?)`((?:LEG|DEC|SEED|ARCH|QUAL)-[A-Za-z0-9][A-Za-z0-9_-]*)`:"
    r"\s*\n+```text\n(.*?)\n```",
    re.S,
)
W_MEANING_BLOCK = re.compile(r"Authoritative meanings:\s*\n+```text\n(.*?)\n```", re.S)
W_ID = re.compile(r"^[a-z0-9_]+$")


def witness_rows(root: Path) -> dict[str, dict]:
    """proof-row id -> {owner guarantee}. Since 5.5E4d the TYPED catalog in
    spec/proof/inventory.rs is the one membership authority; docs/24 owns
    meaning only, and the retired fence headers are no longer parsed for
    anything."""
    src = (root / "spec/proof/inventory.rs").read_text(encoding="utf-8")
    out: dict[str, dict] = {}
    for wid, family, guarantee, _contracts, _claim, _plan in re.findall(
            r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
            r'ProofRowState::Active \{ guarantee: GuaranteeRef::(leg|dec)\("([^"]+)"\), '
            r'projection_contracts: &\[([^\]]*)\], '
            r'claim: VerificationClaimKind::(\w+), verification: ([A-Z0-9_]+) \} \}', src):
        if wid in out:
            out[wid]["duplicate"] = True
            continue
        out[wid] = {"leg": guarantee, "target": guarantee, "family": family,
                    "duplicate": False}
    return out


def witness_meanings(root: Path) -> set[str]:
    doc = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    named: set[str] = set()
    for body in W_MEANING_BLOCK.findall(doc):
        for line in body.splitlines():
            if line and not line.startswith(" ") and W_ID.match(line.strip()):
                named.add(line.strip())
    return named


def docs21_witness_refs(root: Path) -> dict[str, list[str]]:
    """LEG id -> the witness IDs its docs/21 cell projects."""
    doc = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    out: dict[str, list[str]] = {}
    for line in doc.splitlines():
        if not line.startswith("| LEG-"):
            continue
        cells = [c.strip() for c in line.split("|")[1:-1]]
        if len(cells) != 10:
            continue
        refs = [r.strip() for r in cells[5].split(";") if W_ID.match(r.strip())]
        if refs:
            out[cells[0]] = refs
    return out


def witness_reference_findings(root: Path) -> list[str]:
    """docs/21 projects exactly the witness IDs docs/24 binds to that LEG."""
    rows = witness_rows(root)
    meanings = witness_meanings(root)
    refs = docs21_witness_refs(root)
    out: list[str] = []
    for wid, row in sorted(rows.items()):
        if row["duplicate"]:
            out.append(f"docs/24 binds proof-row id {wid} more than once")
        if wid not in meanings:
            out.append(f"docs/24 proof row {wid} has no authoritative meaning")
    # every owned row is projected, and nothing extra is
    owned: dict[str, set[str]] = {}
    for wid, row in rows.items():
        owned.setdefault(row["leg"], set()).add(wid)
    for leg, want in sorted(owned.items()):
        got = refs.get(leg, [])
        if len(got) != len(set(got)):
            out.append(f"docs/21 {leg} projects a duplicate witness reference")
        got_set = set(got)
        if leg.startswith("LEG-"):
            for missing in sorted(want - got_set):
                out.append(f"docs/21 {leg} omits owned proof row {missing}")
        elif got:
            # docs/21 is the legacy-obligation document and projects LEG-owned
            # rows only; its parser admits no other family. A proof row may target
            # any GuaranteeRef, so a non-LEG target projects in the domain document
            # that owns it. Demanding a docs/21 cell for a DEC would be demanding
            # one that cannot exist; claiming a foreign row still fails below.
            out.append(
                f"docs/21 projects proof rows for {leg}, which is not a legacy obligation"
            )
        for extra in sorted(got_set - want):
            owner = rows.get(extra, {}).get("leg")
            if owner is None:
                out.append(f"docs/21 {leg} references unknown proof-row id {extra}")
            else:
                out.append(f"docs/21 {leg} references {extra}, which docs/24 binds to {owner}")
    # A LEG that owns nothing may still project references. Those must resolve
    # and must be bound to that LEG, or the projection is claiming another
    # owner's witness.
    for leg, got in sorted(refs.items()):
        if leg in owned:
            continue
        if len(got) != len(set(got)):
            out.append(f"docs/21 {leg} projects a duplicate witness reference")
        for extra in got:
            owner = rows.get(extra, {}).get("leg")
            if owner is None:
                out.append(f"docs/21 {leg} references unknown proof-row id {extra}")
            elif owner != leg:
                out.append(f"docs/21 {leg} references {extra}, which docs/24 binds to {owner}")
    return out


# LEG-023's law must keep the six integrity claims separate and refuse the two
# impersonations that make a digest match look like chain integrity.
D4B_LEG023_CLAUSES = [
    "never proves EventCommitment equality",
    "requires the exact immediate predecessor",
    "an event appearing somewhere in a verified set is not thereby the immediate predecessor",
    "legal genesis occurs only at the lawful stream head",
    "may not select the expected authority bytes and then use those selected bytes to authenticate its own selection",
    "lane isolation is owned by LEG-050",
    "visible linearization by LEG-067",
]
D4B_LEG023_WITNESSES = [
    "middle_event_deletion_is_rejected",
    "event_reorder_is_rejected",
    "duplicate_payload_splice_is_rejected",
    "cross_lane_predecessor_is_rejected",
    "cross_entity_predecessor_is_rejected",
    "midstream_genesis_is_rejected",
    "forged_index_row_cannot_choose_and_authenticate_bytes",
]


def integrity_witness_findings(root: Path) -> list[str]:
    src = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    m = re.search(r'id: "LEG-023", law: "([^"]+)"', src)
    out: list[str] = []
    if not m:
        return ["LEG-023 is absent from spec/legacy_obligations/inventory.rs"]
    law = m.group(1)
    for clause in D4B_LEG023_CLAUSES:
        if clause not in law:
            out.append(f"LEG-023 law no longer states: {clause}")
    rows = witness_rows(root)
    for wid in D4B_LEG023_WITNESSES:
        row = rows.get(wid)
        if row is None:
            out.append(f"LEG-023 witness {wid} is absent from docs/24")
        elif row["leg"] != "LEG-023":
            out.append(f"LEG-023 witness {wid} is bound to {row['leg']}")
    return out


# --- 5.5D4b-2a derived-material duals (LEG-019) ------------------------------
# The two directions stay equally visible. A table that agrees cannot prove
# safety, and a table that disagrees cannot prove loss.
D4B2A_LEG019_CLAUSES = [
    "cannot authenticate themselves",
    "proves only that exact row-to-authoritative-frame relationship",
    "cannot prove safety and cannot prove loss",
    "never an authoritative proven-loss or proven-safety conclusion",
    "no trusted-local SIDX authority",
]
D4B2A_ROWS = {
    "LEG-019": [
        "forged_sibling_cannot_cause_false_loss",
        "agreeing_truncated_table_cannot_cause_false_safety",
        "derived_row_cannot_authenticate_siblings",
        "derived_row_cannot_authenticate_table_count",
        "derived_row_cannot_authenticate_order",
        "derived_row_cannot_authenticate_tail_boundary",
        "derived_row_cannot_prove_absence_or_loss",
    ],
    "LEG-028": ["page_limit_bounds_discovery_work_not_only_output"],
    "LEG-020": ["allocation_does_not_scale_with_full_matched_set"],
}


def derived_material_findings(root: Path) -> list[str]:
    src = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    m = re.search(r'id: "LEG-019", law: "([^"]+)"', src)
    out: list[str] = []
    if not m:
        return ["LEG-019 is absent from spec/legacy_obligations/inventory.rs"]
    law = m.group(1)
    for clause in D4B2A_LEG019_CLAUSES:
        if clause not in law:
            out.append(f"LEG-019 law no longer states: {clause}")
    rows = witness_rows(root)
    for leg, ids in D4B2A_ROWS.items():
        for wid in ids:
            row = rows.get(wid)
            if row is None:
                out.append(f"{leg} witness {wid} is absent from docs/24")
            elif row["leg"] != leg:
                out.append(f"{leg} witness {wid} is bound to {row['leg']}")
    # The discovery-side and allocation-side claims keep one primary owner each.
    disc = rows.get("page_limit_bounds_discovery_work_not_only_output", {}).get("leg")
    alloc = rows.get("allocation_does_not_scale_with_full_matched_set", {}).get("leg")
    if disc == alloc:
        out.append("bounded discovery and bounded allocation share one qualification target")
    return out


# --- 5.5D4b-2b deferred proof posture ---------------------------------------
# "Deferred" must be an explicit, machine-audited fact. It is never inferred
# from the word fcntl, the witness ID, the owner name, or the document section.
# A deferred row records that the proof boundary exists; it never claims that
# bootstrap called a syscall, inspected a descriptor table, or shipped a
# launcher.
D4B2B_ROWS = {
    "LEG-043": [
        "descriptor_postcondition_failure_is_not_reported_applied",
        "fcntl_getfd_failure_fails_closed",
        "fcntl_setfd_failure_fails_closed",
    ],
    "LEG-074": ["close_reopen_reimport_returns_zero_new_events"],
}
D4B2B_DEFERRED = set(D4B2B_ROWS["LEG-043"])
D4B2B_VAGUE = ("later", "eventually", "someday", "tbd", "in future", "at some point")


def leg_gates(root: Path, leg: str) -> str:
    """The owning LEG's live typed gate list, rendered. Never invented.

    Delegates to the generic guarantee index (5.5D4b-3b0). LEG resolution is
    byte-for-byte what it was before the widening.
    """
    entry = guarantee_index(root).get(leg)
    return entry["gates"] if entry and entry["family"] == "LEG" else ""


# Load-bearing clauses inside a proof row's authoritative meaning. Presence of
# the ID is not enough: the meaning is what a future implementation qualifies
# against, so its claims must survive too.
D4B2B_MEANING_CLAUSES = {
    "descriptor_postcondition_failure_is_not_reported_applied": [
        "not reported as established",
        "when the required postcondition verification fails",
    ],
    "fcntl_getfd_failure_fails_closed": [
        "Failure to read descriptor flags cannot produce a verified close-on-exec",
    ],
    "fcntl_setfd_failure_fails_closed": [
        "Failure to apply descriptor flags cannot produce an applied or verified",
    ],
    "close_reopen_reimport_returns_zero_new_events": [
        "imports zero new events",
        "already-established authority outcome",
        "witness binds stable import identity",
        "source lineage",
        "destination",
        "reopened destination state",
        "idempotency",
        "new-event count of zero",
    ],
}


def proof_meaning_findings(root: Path) -> list[str]:
    """Each proof row's meaning still states the claims it exists to defend."""
    doc = re.sub(r"\s+", " ", (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8"))
    out = []
    for wid, clauses in {**D4B2B_MEANING_CLAUSES, **D4B2C_MEANING_CLAUSES}.items():
        for clause in clauses:
            if re.sub(r"\s+", " ", clause) not in doc:
                out.append(f"proof row {wid} meaning no longer states: {clause}")
    return out


def deferred_posture_findings(root: Path) -> list[str]:
    """The deferred-proof rows keep their typed membership (5.5E4d): the
    fence-header posture prose retired with the membership mirrors, and
    execution evidence lives in run receipts, never in timeless prose."""
    rows = witness_rows(root)
    out: list[str] = []
    for leg, ids in D4B2B_ROWS.items():
        for wid in ids:
            row = rows.get(wid)
            if row is None:
                out.append(f"{leg} witness {wid} is absent from docs/24")
            elif row["leg"] != leg:
                out.append(f"{leg} witness {wid} is bound to {row['leg']}")
    return out

# --- 5.5D4b-2c LEG-081 proof-row authority and coverage ---------------------
# docs/24 owns the nine canonical rows. docs/35 keeps the secret-authority law
# and projects the same nine IDs; it owns no per-row executable meaning. The
# eight pre-migration names were a preimplementation sketch: they left the
# shred-transition binding clause unwitnessed and three names claimed less scope
# than the clause they defended. Coverage is proven against the typed law's
# clauses, never inferred from surrounding prose.
D4B2C_LEG = "LEG-081"
D4B2C_D24 = "docs/24_GAUNTLET.md"
D4B2C_D35 = "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md"
D4B2C_D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
D4B2C_ROWS = [
    "shred_ack_waits_for_backend_durability",
    "crash_before_durable_key_delete_does_not_report_shred_success",
    "reopen_after_ack_cannot_recover_shredded_plaintext",
    "shred_transition_binding_mismatch_is_rejected",
    "stale_or_pre_shred_keyset_restore_is_rejected",
    "foreign_keyset_generation_is_rejected",
    "shredded_unavailable_and_keyset_missing_remain_distinct",
    "snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys",
    "external_key_backend_preserves_shred_semantics",
]
# Retired proof-row vocabulary: retired id -> the successors that replaced it.
# ONE registry for the whole specification, not one per phase that renames a row.
# A retired id may appear only inside a marked historical migration note; its
# successors are its only other names. The value is a tuple because a retirement
# is not always a rename: splitting one bidirectional row into two directional
# ones has two successors, and recording only one would lose half the migration.
# Note that `pre_shred_keyset_restore_is_rejected` is a proper substring of its
# own successor, so reintroduction is matched on identifier boundaries, never
# by `in`.
# The retired proof-row registry moved to its typed owner in 5.5E2 (today
# spec/proof/inventory.rs PROOF_ROWS, which absorbed the old
# PROOF_ROW_MIGRATIONS). The dict that stood here was already
# missing an entry (test_local_fixture_type_is_classified_correctly, retired
# by a docs/24 migration note the dict never learned) — a registry the
# auditor owns is a registry the auditor cannot be caught neglecting. This
# auditor now PARSES the typed registry and independently verifies it agrees
# with every historical migration note.
A_PROOF_RECORD = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("(\w+)"\), '
    r'state: ProofRowState::(Active|Retired \{ successors: &\[([^\]]*)\] \}) \},'
)


def proof_row_catalog(root: Path) -> list[tuple[str, str, tuple[str, ...]]]:
    """(id, state, successors) in declaration order, from the typed catalog.
    Active rows carry guarantee and projection contracts since 5.5E4d; this
    lifecycle view records the state name and any successors."""
    src = (root / "spec/proof/inventory.rs").read_text(encoding="utf-8")
    out: list[tuple[str, str, tuple[str, ...]]] = []
    for m in re.finditer(
            r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
            r'ProofRowState::(Active|Retired)(?: \{([^}]*(?:\}[^}]*)?)\})? \}', src):
        successors = tuple(re.findall(r'ProofRowId\("([^"]+)"\)', m.group(3) or ""))
        out.append((m.group(1), m.group(2), successors))
    return out


def retired_proof_rows(root: Path) -> dict[str, tuple[str, ...]]:
    """retired id -> successors, parsed from the typed catalog."""
    return {rid: succ for rid, state, succ in proof_row_catalog(root)
            if state == "Retired"}


def proof_row_catalog_findings(root: Path) -> list[str]:
    """The catalog is the living census: one lifecycle per identity, both
    states constructed, successors resolving INSIDE the catalog, and exact
    two-way equality between typed Active ids and the structurally parsed
    canonical docs/24 rows. A name surviving in a migration note or prose is
    not an active row."""
    out: list[str] = []
    catalog = proof_row_catalog(root)
    if not catalog:
        return ["spec/proof/inventory.rs declares no proof-identity catalog"]
    ids = [rid for rid, _, _ in catalog]
    for dup in sorted({i for i in ids if ids.count(i) > 1}):
        out.append(f"{dup} is declared twice in the proof-identity catalog; "
                   "one identity carries one lifecycle")
    active = {rid for rid, state, _ in catalog if state == "Active"}
    retired = {rid for rid, state, _ in catalog if state == "Retired"}
    if not active:
        out.append("the proof-identity catalog constructs no Active row")
    if not retired:
        out.append("the proof-identity catalog constructs no Retired row")
    for rid, state, succ in catalog:
        if state != "Retired":
            continue
        if not succ:
            out.append(f"retired proof-row id {rid} names no successor")
        for s in succ:
            if s == rid:
                out.append(f"{rid} names itself as its successor")
            elif s not in active and s not in retired:
                out.append(f"{rid} names successor {s}, which resolves to no "
                           "typed catalog identity")
    # Succession terminates (E3 preflight): the per-edge laws cannot see a
    # two-node cycle, which satisfies existence and non-self-succession while
    # owning no living obligation. Acyclic, and every path reaches Active.
    succ_map = {rid: succ for rid, state, succ in catalog if state == "Retired"}
    for start in sorted(succ_map):
        frontier, seen = [start], set()
        reaches_active = cyclic = False
        while frontier:
            for s in succ_map.get(frontier.pop(), ()):
                if s in active:
                    reaches_active = True
                elif s == start:
                    cyclic = True
                elif s not in seen:
                    seen.add(s)
                    frontier.append(s)
        if cyclic:
            out.append(f"retirement succession is cyclic: {start} participates in a cycle")
        if not reaches_active:
            out.append(f"{start} retirement path terminates in no Active identity")
    canonical = set(witness_rows(root))
    for missing in sorted(canonical - active):
        out.append(f"docs/24 declares active proof row {missing}, which the "
                   "typed catalog never learned")
    for phantom in sorted(active - canonical):
        out.append(f"typed Active identity {phantom} appears as no canonical "
                   "docs/24 active row")
    return out


def migration_note_retired_ids(root: Path) -> set[str]:
    """Every id a docs/24 historical migration note declares retired, across
    both authored formats (retired/successor tables and renamed/now pairs)."""
    text = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    ids: set[str] = set()
    for block in D4B2C_MIGRATION_NOTE.findall(text):
        ids.update(re.findall(r"(?m)^retired\s+(\w+)", block))
        ids.update(re.findall(r"(?m)^(\w+)\n\s+now\s+\w+", block))
    return ids
D4B2C_COVERAGE = {
    "durable destruction before acknowledgement": [
        "shred_ack_waits_for_backend_durability",
        "crash_before_durable_key_delete_does_not_report_shred_success",
        "reopen_after_ack_cannot_recover_shredded_plaintext",
    ],
    "shred-transition identity and authority binding": [
        "shred_transition_binding_mismatch_is_rejected",
    ],
    "stale, pre-shred, and foreign restore rejection": [
        "stale_or_pre_shred_keyset_restore_is_rejected",
        "foreign_keyset_generation_is_rejected",
    ],
    "Shredded / Unavailable / KeysetMissing distinction": [
        "shredded_unavailable_and_keyset_missing_remain_distinct",
    ],
    "raw-key exclusion from snapshot, fork, WorldImage, artifact, and ordinary receipt surfaces": [
        "snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys",
    ],
    "external backend semantic parity": [
        "external_key_backend_preserves_shred_semantics",
    ],
}
D4B2C_MEANING_CLAUSES = {
    "shred_ack_waits_for_backend_durability": [
        "only after the owning backend has durably established destruction",
        "An accepted request or an attempted delete is not sufficient",
    ],
    "crash_before_durable_key_delete_does_not_report_shred_success": [
        "cannot leave a successful shred acknowledgement",
        "Recovery preserves the incomplete or refused posture",
    ],
    "reopen_after_ack_cannot_recover_shredded_plaintext": [
        "cannot recover plaintext through the shredded key scope",
    ],
    "shred_transition_binding_mismatch_is_rejected": [
        "binds StoreId, AuthorityGeneration, KeyGeneration, key scope, and shred-transition identity",
        "fails closed and cannot produce an acknowledged, applied, or verified shred transition",
    ],
    "stale_or_pre_shred_keyset_restore_is_rejected": [
        "keyset from before the acknowledged shred transition",
    ],
    "foreign_keyset_generation_is_rejected": [
        "another store lineage, authority generation, key generation",
    ],
    "shredded_unavailable_and_keyset_missing_remain_distinct": [
        "remain distinct availability or authority outcomes",
        "No projection or formatter may collapse one into another",
    ],
    "snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys": [
        "exclude raw key material by default",
        "not export usable raw secret-key bytes",
    ],
    "external_key_backend_preserves_shred_semantics": [
        "the same acknowledgement durability",
        "Backend indirection may change mechanism, never semantics",
    ],
}
D4B2C_MIGRATION_NOTE = re.compile(
    r"<!-- HISTORICAL-MIGRATION:BEGIN -->(.*?)<!-- HISTORICAL-MIGRATION:END -->", re.S
)
D4B2C_MATRIX = re.compile(r"LEG-081 proof-coverage matrix:\s*\n+```text\n(.*?)\n```", re.S)
D4B2C_D35_PROJECTION = re.compile(
    r"Required proof rows, projected from docs/24 \(qualification target: (LEG-\d+); "
    r"canonical proof-row owner: ([^)]+)\):\s*\n+```text\n(.*?)\n```",
    re.S,
)


def retired_proof_row_findings(root: Path) -> list[str]:
    """A retired proof-row id returns nowhere but a marked historical note.

    Every document, not a per-phase list of the ones a rename happened to touch:
    a retired id leaking into docs/09 is the same defect as one leaking into
    docs/35, and only one of those was ever checked. The registry itself is the
    typed spec/proof/inventory.rs owner, and it must agree with the migration
    notes in
    BOTH directions — a note that retires an id the registry never learned is
    exactly the defect the Python dict era shipped.
    """
    out: list[str] = []
    registry = retired_proof_rows(root)
    noted = migration_note_retired_ids(root)
    for missing in sorted(noted - set(registry)):
        out.append(
            f"docs/24 retires proof-row id {missing} in a migration note that "
            "spec/proof/inventory.rs PROOF_ROWS never learned"
        )
    for phantom in sorted(set(registry) - noted):
        out.append(
            f"spec/proof/inventory.rs retires proof-row id {phantom} with no docs/24 "
            "migration note recording the retirement"
        )
    for old, successors in registry.items():
        if not successors:
            out.append(f"retired proof-row id {old} names no successor")
    for p in sorted((root / "docs").glob("*.md")):
        text = D4B2C_MIGRATION_NOTE.sub("", p.read_text(encoding="utf-8"))
        for old, successors in registry.items():
            if re.search(r"(?<![a-z0-9_])" + re.escape(old) + r"(?![a-z0-9_])", text):
                out.append(
                    f"docs/{p.name} reintroduces retired proof-row id {old} outside the "
                    f"historical migration note; it was replaced by "
                    f"{' and '.join(successors)}"
                )
    return out


def coverage_matrix(root: Path):
    """[(law clause, [proof-row id])] in document order, or None if absent."""
    m = D4B2C_MATRIX.search((root / D4B2C_D24).read_text(encoding="utf-8"))
    if not m:
        return None
    out: list[tuple[str, list[str]]] = []
    for line in m.group(1).splitlines():
        if not line.strip():
            continue
        if line.startswith(" "):
            if out:
                out[-1][1].append(line.strip())
        else:
            out.append((line.strip(), []))
    return out


def leg081_authority_findings(root: Path) -> list[str]:
    out: list[str] = []
    rows = witness_rows(root)
    meanings = witness_meanings(root)
    want = set(D4B2C_ROWS)

    # 1. docs/24 binds exactly the nine canonical rows to LEG-081
    owned = {w for w, r in rows.items() if r["leg"] == D4B2C_LEG}
    for wid in sorted(want - owned):
        other = rows.get(wid, {}).get("leg")
        if other:
            out.append(f"LEG-081 canonical proof row {wid} is bound to {other} in docs/24")
        else:
            out.append(f"LEG-081 canonical proof row {wid} is absent from docs/24")
    for wid in sorted(owned - want):
        out.append(f"docs/24 binds unexpected proof row {wid} to LEG-081")
    for wid in sorted(want & owned):
        row = rows[wid]
        if row["duplicate"]:
            out.append(f"docs/24 binds LEG-081 proof row {wid} more than once")
        # Fence-header owner/gate/posture prose retired with the membership
        # mirrors (5.5E4d); membership is typed and execution lives in receipts.
        if wid not in meanings:
            out.append(f"LEG-081 proof row {wid} has no authoritative meaning")

    # 2. docs/35 projects the same nine and reclaims no authority
    d35 = (root / D4B2C_D35).read_text(encoding="utf-8")
    if re.search(r"Required witnesses \(proof owner", d35):
        out.append("docs/35 reclaims authoritative proof-row ownership; docs/24 owns proof-row identity")
    if W_MEANING_BLOCK.search(d35):
        out.append("docs/35 reclaims per-ID authoritative witness meaning; docs/24 owns it")
    # The docs/35 required-list is a REGISTERED generated view since 5.5E4d
    # (SecretAuthorityProofRequirements); the universal census and domain
    # projection laws own its membership now.

    # 3. retired vocabulary is owned by retired_proof_row_findings, which sweeps
    #    every document rather than the three this rule happened to know about.

    # 4. every law clause carries an owned executable witness
    cov = coverage_matrix(root)
    if cov is None:
        out.append("docs/24 states no LEG-081 proof-coverage matrix")
    else:
        seen: list[str] = [c for c, _ in cov]
        covered: set[str] = set()
        for clause in D4B2C_COVERAGE:
            if clause not in seen:
                out.append(f"LEG-081 coverage matrix omits law clause: {clause}")
            elif seen.count(clause) != 1:
                out.append(f"LEG-081 coverage matrix names law clause {clause!r} more than once")
        for clause, ids in cov:
            if clause not in D4B2C_COVERAGE:
                out.append(f"LEG-081 coverage matrix names unknown law clause: {clause}")
                continue
            if not ids:
                out.append(f"LEG-081 coverage clause names no proof row: {clause}")
            for wid in ids:
                if wid in want:
                    covered.add(wid)
                else:
                    out.append(f"LEG-081 coverage matrix names unknown proof row {wid} for clause: {clause}")
            if set(ids) != set(D4B2C_COVERAGE[clause]):
                out.append(
                    f"LEG-081 coverage clause {clause!r} maps {sorted(ids)}, "
                    f"not {sorted(D4B2C_COVERAGE[clause])}"
                )
        for wid in sorted(want - covered):
            out.append(f"LEG-081 canonical proof row {wid} appears in no coverage clause")
    return out


# Expectation clause. Every canonical proof row states, inside its
# authoritative meaning:
#
#     expects: <source-native terminal result, refusal, or success predicate>
#     disposition: <terminal receipt or disposition path>
#
# The transitional ceiling is spent: D4d migrated the 29 rows that predated the
# grammar, so a ceiling of zero IS universal enforcement. A row without the
# clause -- newly promoted or newly stripped -- is a finding, and the ceiling
# never rises again without an architect ruling.
D4B3B0_PENDING_EXPECTATION_CEILING = 0
D4B3B0_VAGUE = ("tbd", "later", "eventually", "someday", "as appropriate",
                "as needed", "correctly", "something", "n/a")
# A clause may wrap. A continuation is a line indented DEEPER than the key that
# opened it and carrying no key of its own; it folds into that key's value. This
# is not cosmetic: a first-line-only parser would scan a truncated value, so a
# vague or self-contradicting qualifier could hide on the second line and pass.
W_CLAUSE = re.compile(r"^(\s+)(expects|disposition):\s*(.*)$")
W_INDENT = re.compile(r"^(\s*)(\S.*)$")


def witness_expectations(root: Path) -> dict[str, dict]:
    """proof-row id -> {expects, disposition} for rows carrying the clause."""
    doc = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    out: dict[str, dict] = {}
    for body in W_MEANING_BLOCK.findall(doc):
        cur = None
        key = None
        depth = 0
        for line in body.splitlines():
            s = line.strip()
            if not s:
                key = None
                continue
            if not line.startswith(" ") and W_ID.match(s):
                cur, key = s, None
                continue
            if cur is None:
                continue
            m = W_CLAUSE.match(line)
            if m:
                key, depth = m.group(2), len(m.group(1))
                out.setdefault(cur, {})[key] = m.group(3).strip()
                continue
            if key is not None and len(W_INDENT.match(line).group(1)) > depth:
                out[cur][key] = (out[cur][key] + " " + s).strip()
                continue
            key = None
    return out


# Grammar check reads the target slot before it is known to be a GuaranteeRef, so
# a GateId or free text in that slot produces its own diagnostic instead of
# silently unmatching the whole block.
W_ANY_TARGET_BLOCK = re.compile(
    r"Required witnesses \(proof owner [^)]*\)([^\n:]*?)`([^`\n]+)`:\s*\n+```text\n", re.S)


def proof_target_grammar_findings(root: Path) -> list[str]:
    doc = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    tokens = {g[1] for g in gate_inventory(root)}
    out: list[str] = []
    for _tail, target in W_ANY_TARGET_BLOCK.findall(doc):
        if G_REF.fullmatch(target):
            continue
        if target in tokens:
            out.append(
                f"docs/24 names GateId {target} as a proof target; a gate schedules "
                f"qualification and never owns semantic meaning"
            )
        else:
            out.append(f"docs/24 names {target!r} as a proof target, which is not a source-qualified GuaranteeRef")
    return out


def proof_target_findings(root: Path) -> list[str]:
    idx = guarantee_index(root)
    rows = witness_rows(root)
    exp = witness_expectations(root)
    out: list[str] = []
    for wid, row in sorted(rows.items()):
        # One primary target per row is now STRUCTURAL: the typed Active state
        # carries exactly one GuaranteeRef (5.5E4d).
        ref = row["target"]
        entry = idx.get(ref)
        if entry is None:
            # An unknown *family* never reaches here: W_OWNER_BLOCK only matches
            # the five known prefixes, so proof_target_grammar_findings owns that
            # diagnostic. This branch is for a well-formed ref naming no live
            # guarantee.
            out.append(
                f"proof row {wid} targets {ref}, which resolves to no existing "
                f"{ref.split('-', 1)[0]} guarantee"
            )
            continue
    for wid, e in sorted(exp.items()):
        if wid not in rows:
            out.append(f"expectation clause names unknown proof row {wid}")
            continue
        for field in ("expects", "disposition"):
            v = e.get(field, "")
            if not v:
                out.append(f"proof row {wid} expectation clause states no {field}")
            elif any(x in v.lower() for x in D4B3B0_VAGUE):
                out.append(f"proof row {wid} expectation clause {field} is not falsifiable: {v!r}")
    pending = sorted(w for w in rows if w not in exp)
    if len(pending) > D4B3B0_PENDING_EXPECTATION_CEILING:
        out.append(
            f"{len(pending)} proof rows carry no expectation clause, above the transitional "
            f"ceiling of {D4B3B0_PENDING_EXPECTATION_CEILING}; every canonical row must carry it"
        )
    return out


# Structural candidate discovery. Heading text is never the parser API: the 29/52
# undercount happened because a scan keyed on one English phrase missed
# "implemented at G3" and "Hostile fixture obligations". A fence is proof-bearing
# because its label carries executable metadata, not because it says a blessed
# phrase.
D4B3B0_FENCE = re.compile(r"```text\n(.*?)\n```", re.S)
D4B3B0_CAND_ID = re.compile(r"^[a-z][a-z0-9_]{6,}$")


def candidate_fences(root: Path) -> list[dict]:
    tokens = {g[1] for g in gate_inventory(root)}
    canon = set(witness_rows(root))
    out: list[dict] = []
    for p in sorted((root / "docs").glob("*.md")):
        text = p.read_text(encoding="utf-8")
        for m in D4B3B0_FENCE.finditer(text):
            ids = [ln.strip() for ln in m.group(1).splitlines() if ln.strip()]
            if not ids or not all(D4B3B0_CAND_ID.match(i) for i in ids):
                continue
            before = [ln for ln in text[: m.start()].splitlines() if ln.strip()]
            label = before[-1] if before else ""
            has_meta = (
                "proof owner" in label.lower()
                or G_REF.search(label) is not None
                or any(re.search(r"\b" + t + r"\b", label) for t in tokens)
            )
            rel = "docs/" + p.name
            if not has_meta:
                kind = "DescriptiveEvidence"
            elif rel == "docs/24_GAUNTLET.md":
                kind = "Authority"
            elif all(i in canon for i in ids):
                kind = "Projection"
            else:
                kind = "UnboundCandidate"
            out.append({"doc": rel, "line": text[: m.start()].count("\n") + 1,
                        "label": label, "ids": ids, "kind": kind})
    return out


def candidate_summary(root: Path) -> tuple[int, int, int]:
    """(unbound ids, unbound blocks, rows pending the expectation clause)."""
    unbound = [c for c in candidate_fences(root) if c["kind"] == "UnboundCandidate"]
    rows = witness_rows(root)
    exp = witness_expectations(root)
    return (sum(len(c["ids"]) for c in unbound), len(unbound),
            len([w for w in rows if w not in exp]))



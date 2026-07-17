#!/usr/bin/env python3
"""Independent standard-library audit for the clean-room specification."""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

STATUSES = {"AUTHORITATIVE", "GENERATED", "EVIDENCE-ONLY", "SUPERSEDED", "REJECTED"}
REQUIRED_FRONT = {"status", "contract_id", "authority_scope", "supersedes", "last_reconciled"}
GENERATED_FRONT = {"status", "authority_scope", "generated_by", "generated_from", "do_not_edit"}
PLACEHOLDERS = ("TBD", "TO BE DECIDED", "IMPLEMENTATION DECIDES", "FIXME")
# Stale vocabulary is DERIVED from spec/dispositions.rs (never a hand-kept list).
# Each retiring decision names its stale aliases and the contexts in which they
# may still appear. Ordinary authoritative material and production/config source
# are not permissive contexts: they must carry an inline [STALE-REF: DEC-...].
STALE_CONTEXT_BY_PATH = {
    "docs/30_DECISION_AND_REJECTION_LEDGER.md": "DecisionLedger",
    "docs/29_STATUS_AND_SUPERSESSION.md": "SupersessionGuide",
    "docs/33_AGENT_FINISH_LINE_CHECKLIST.md": "SupersessionGuide",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md": "RejectionRecord",
    "FINAL_RECONCILIATION.md": "RejectionRecord",
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md": "LegacyEvidence",
    "docs/34_LEGACY_INVARIANT_COVERAGE.md": "LegacyEvidence",
    "docs/22_MIGRATION_AND_CUTOVER.md": "MigrationCompatibility",
    "docs/15_SCHEMA_CODEC_AND_MIGRATION.md": "MigrationCompatibility",
    "docs/20_DEPENDENCY_SOVEREIGNTY.md": "MigrationCompatibility",
    "spec/dispositions.rs": "DecisionLedger",
}
STALE_PROJECTION_DOCS = ("docs/29_STATUS_AND_SUPERSESSION.md", "docs/33_AGENT_FINISH_LINE_CHECKLIST.md")
# The closed stale-context vocabulary (5.5E2): five permissive contexts a
# retiring decision may allow-list, plus the two DEFAULT contexts the scanner
# assigns to everything else. The defaults are never permissive. The typed
# enum in spec/dispositions.rs must declare exactly this set — before the
# completion the scanner spoke two context names the typed owner could not.
STALE_PERMISSIVE_CONTEXTS = frozenset({
    "DecisionLedger", "RejectionRecord", "SupersessionGuide",
    "LegacyEvidence", "MigrationCompatibility"})
STALE_DEFAULT_CONTEXTS = frozenset({"ProductionSource", "OrdinaryAuthoritative"})
STALE_REF_RE = re.compile(r"\[STALE-REF:\s*(DEC-\d+)\]")
# The block is generated from spec/dispositions.rs (5.5D4b). The marker carries
# its provenance, so the pattern must not pin the bare form.
STALE_VOCAB_BLOCK_RE = re.compile(r"<!-- STALE-VOCAB:BEGIN[^>]*-->(.*?)<!-- STALE-VOCAB:END -->", re.S)
RETIRING_DISPOSITIONS = {"Kill", "Supersede", "Demote", "Lock"}
PRODUCTION_SOURCE_DIRS = ("crates", "apps")
EXCLUDE_DIRS = {".git", "target", "__pycache__"}
EXCLUDE_SUFFIXES = {".zip", ".pyc"}


def frontmatter(text: str) -> dict[str, str]:
    if not text.startswith("---\n"):
        return {}
    end = text.find("\n---\n", 4)
    if end < 0:
        return {}
    out: dict[str, str] = {}
    for line in text[4:end].splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            out[key.strip()] = value.strip()
    return out


def markdown_links(text: str):
    for match in re.finditer(r"\[[^\]]+\]\(([^)]+)\)", text):
        target = match.group(1).split("#", 1)[0]
        if target and not re.match(r"^[a-z]+://", target) and not target.startswith("mailto:"):
            yield target


def frozen_files(root: Path) -> dict[str, Path]:
    # Do not rely on Path object ordering (case-sensitive on POSIX,
    # case-insensitive on Windows). Downstream checks are set/row-based, so
    # enumeration order does not matter here; portability ordering lives in
    # freeze.py and is proven by bootstrap/selftest.py.
    out: dict[str, Path] = {}
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        rel = relative.as_posix()
        if any(part in EXCLUDE_DIRS for part in relative.parts):
            continue
        if rel == "SPEC.sha256" or any(rel.endswith(suffix) for suffix in EXCLUDE_SUFFIXES):
            continue
        out[rel] = path
    return out


def casefold_collisions(rel_paths) -> list[tuple[str, str]]:
    """Distinct paths that collide under case-folding.

    Windows (and case-insensitive filesystems) cannot safely materialize both
    ``Foo.md`` and ``foo.md`` even where POSIX can. Ordering the survey by the
    UTF-8 relative-path bytes keeps the reported pair stable across hosts.
    """
    seen: dict[str, str] = {}
    collisions: list[tuple[str, str]] = []
    for rel in sorted(rel_paths, key=lambda value: value.encode("utf-8")):
        key = rel.casefold()
        if key in seen:
            collisions.append((seen[key], rel))
        else:
            seen[key] = rel
    return collisions


def string_array(source: str, name: str) -> list[str]:
    match = re.search(rf"pub const {re.escape(name)}:\s*&\[[^]]*\]\s*=\s*&\[(.*?)\];", source, re.S)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group(1))


def legacy_manifest_findings(manifest_ids: list[str], coverage_ids: list[str], expected_rows: int) -> list[str]:
    """Declaration-parity between the frozen source manifest and coverage rows.

    D-1: the legacy denominator is measured against declared invariant IDs, never
    a raw ``INV-`` token count. A structural key that resembles an invariant name
    (e.g. the generated ``INV-CATALOG`` block marker) is not a declaration and
    must not appear in the manifest. Every declared id has exactly one coverage
    row and vice versa.
    """
    out: list[str] = []
    if len(manifest_ids) != len(set(manifest_ids)):
        out.append("SOURCE_INVARIANT_IDS contains a duplicate declaration id")
    if len(manifest_ids) != expected_rows:
        out.append(f"SOURCE_INVARIANT_IDS has {len(manifest_ids)} ids, expected {expected_rows}")
    manifest = set(manifest_ids)
    coverage = set(coverage_ids)
    for missing in sorted(manifest - coverage):
        out.append(f"declared legacy invariant {missing} has no coverage row")
    for extra in sorted(coverage - manifest):
        out.append(f"coverage row {extra} is not a declared source invariant")
    return out


# --- BatQL: OperatorSpec parity, grammar closure, proof disposition ----------
# This is the AUDITOR half of the generator/auditor pair. It recomputes the
# operator projection INDEPENDENTLY of bootstrap/project.py (its own parser,
# its own renderers) and compares. The two share no parsing, normalization,
# projection-building, or comparison logic; only the inert marker names are
# common, and those are asserted on both sides.
BATQL_OP_ROW = re.compile(
    r'OperatorSpec \{ id: "([^"]*)", class: OperatorClass::(\w+), '
    r'word_surface: "([^"]*)", symbol_surface: "([^"]*)", semantic_op: "([^"]*)", '
    r'arity: Arity::(\w+), fixity: Fixity::(\w+), precedence: (\d+), '
    r'associativity: Associativity::(\w+), typing: &\[([^\]]*)\], input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", formatting: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
BATQL_OP_FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "typing", "input_sorts", "result_sort",
    "exactness", "overflow", "exception", "formatting", "spoken",
    "mutation_classes", "numeric_support",
)
BATQL_CLASS_RANK = {"Arithmetic": 0, "Comparison": 1, "Logical": 2}
BATQL_PROOF_DISPOSITIONS = ["VERIFIED", "LEGACY_WEAK", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]


def batql_parse_operators(root: Path) -> list[dict[str, str]]:
    source = (root / "spec/operators.rs").read_text(encoding="utf-8")
    return [dict(zip(BATQL_OP_FIELDS, match)) for match in BATQL_OP_ROW.findall(source)]


def _batql_canonical(ops: list[dict[str, str]]) -> list[dict[str, str]]:
    return sorted(ops, key=lambda o: (BATQL_CLASS_RANK.get(o["class"], 9), o["id"]))


def batql_render_catalog(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Class | Word | Symbol | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for op in _batql_canonical(ops):
        symbol = f"`{op['symbol']}`" if op["symbol"] else ""
        out.append(
            f"| {op['id']} | {op['class']} | `{op['word']}` | {symbol} | {op['arity']} | "
            f"{op['fixity']} | {op['precedence']} | {op['associativity']} | {op['result_sort']} | {op['exactness']} |"
        )
    return "\n".join(out)


def batql_render_grammar(ops: list[dict[str, str]]) -> str:
    ops = _batql_canonical(ops)
    comparison = [o["word"] for o in ops if o["class"] == "Comparison"]
    comparison += [f'"{o["symbol"]}"' for o in ops if o["class"] == "Comparison" and o["symbol"]]
    arithmetic = [f'"{o["word"]}"' for o in ops if o["class"] == "Arithmetic"]
    logical = [o["word"] for o in ops if o["class"] == "Logical" and o["fixity"] == "Infix"]
    return "\n".join([
        "```text",
        "comparison_operator",
        "  := " + " | ".join(comparison),
        "",
        "arithmetic_operator",
        "  := " + " | ".join(arithmetic),
        "",
        "logical_operator",
        "  := " + " | ".join(logical),
        "```",
    ])


def batql_render_projection(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Formatting | Spoken | Legal mutation classes |",
        "| --- | --- | --- | --- |",
    ]
    for op in _batql_canonical(ops):
        out.append(f"| {op['id']} | `{op['formatting']}` | {op['spoken']} | {op['mutation_classes']} |")
    return "\n".join(out)


def batql_render_numeric(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Class | Numeric support |",
        "| --- | --- | --- |",
    ]
    for op in _batql_canonical(ops):
        out.append(f"| {op['id']} | {op['class']} | {op['numeric_support']} |")
    return "\n".join(out)


def batql_extract_block(text: str, name: str) -> str | None:
    match = re.search(
        r"<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n(.*?)\n<!-- " + re.escape(name) + r":END -->",
        text,
        re.S,
    )
    return match.group(1) if match else None


def batql_operator_fact_findings(ops: list[dict[str, str]]) -> list[str]:
    out: list[str] = []
    ids = [o["id"] for o in ops]
    if len(ids) != len(set(ids)):
        out.append("spec/operators.rs: duplicate OperatorId")
    owner: dict[tuple[str, str], str] = {}
    for op in ops:
        for surface in (op["word"], op["symbol"]):
            if not surface:
                continue
            key = (surface, op["fixity"])
            if key in owner and owner[key] != op["id"]:
                out.append(
                    f"spec/operators.rs: token {surface!r} with fixity {op['fixity']} claimed by {owner[key]} and {op['id']}"
                )
            owner[key] = op["id"]
    required = [f for f in BATQL_OP_FIELDS if f != "symbol"]
    for op in ops:
        for field in required:
            if not op.get(field, "").strip():
                out.append(f"spec/operators.rs: operator {op.get('id', '?')} missing field {field}")
    valid_support = {"ExactSupported", "QualifiedProfileOnly", "Unsupported", "NotApplicable"}
    for op in ops:
        support = op.get("numeric_support", "")
        if support and support not in valid_support:
            out.append(f"spec/operators.rs: operator {op['id']} has unknown numeric_support {support}")
        if op.get("class") in ("Arithmetic", "Comparison") and support in ("", "NotApplicable"):
            out.append(f"spec/operators.rs: {op.get('class')} operator {op['id']} lacks an approximate-support posture")
        # V1 admits no raw approximate operands: no qualified numeric profile exists,
        # so no operator may carry QualifiedProfileOnly. ExactSupported is exact-only.
        if support == "QualifiedProfileOnly":
            out.append(
                f"spec/operators.rs: operator {op['id']} admits approximate operands (QualifiedProfileOnly) "
                f"but no qualified numeric profile exists in V1"
            )
    # The typed legality rules (5.5E1). Placement is law, not decoration: a
    # wall-observation difference anywhere but subtraction would let wall
    # arithmetic leak past the TimeDelta fence (docs/16), and a percent
    # difference is subtraction by meaning.
    for op in ops:
        rules = re.findall(r"OperatorTypingRule::(\w+)", op.get("typing", ""))
        if not rules:
            out.append(f"spec/operators.rs: operator {op['id']} declares no typing rules")
            continue
        sem = op.get("semantic_op", "")
        if "WallObservationDifference" in rules and sem != "subtract":
            out.append(f"spec/operators.rs: {op['id']} claims WallObservationDifference; "
                       f"only subtraction of two observations yields TimeDelta")
        if "PercentDifference" in rules and sem != "subtract":
            out.append(f"spec/operators.rs: {op['id']} claims PercentDifference; "
                       f"a rate difference is subtraction")
        if "PercentAdjustment" in rules and sem not in ("add", "subtract"):
            out.append(f"spec/operators.rs: {op['id']} claims PercentAdjustment "
                       f"outside addition and subtraction")
        if op["class"] == "Comparison" and rules != ["SameSortComparison"]:
            out.append(f"spec/operators.rs: comparison {op['id']} must carry exactly "
                       f"SameSortComparison")
        if op["class"] == "Logical" and not set(rules) <= {"TruthUnary", "TruthBinary"}:
            out.append(f"spec/operators.rs: logical {op['id']} carries a non-Truth typing rule")
        if op["class"] == "Arithmetic" and set(rules) & {
                "TruthUnary", "TruthBinary", "SameSortComparison"}:
            out.append(f"spec/operators.rs: arithmetic {op['id']} carries a "
                       f"comparison/truth typing rule")
    return out


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
    fn = src[src.index("pub const fn counts_green"):]
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
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
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


# Two phantom sorts were born in the hand-authored arithmetic matrix and owned
# by nothing: `DecimalMoney` (Money*Percent needs no new sort) and
# `SignedDuration` (the signed observation difference is `TimeDelta`, docs/16).
# Deleted by the 5.5E1 ruling; any tracked prose resurrecting either is a
# finding, not a style choice.
PHANTOM_SORTS = ("DecimalMoney", "SignedDuration")


def phantom_sort_findings(root: Path) -> list[str]:
    out: list[str] = []
    for rel in sorted(list((root / "docs").glob("*.md")) + list((root / "companion").glob("*.md"))):
        text = rel.read_text(encoding="utf-8")
        for sort in PHANTOM_SORTS:
            if re.search(r"\b" + sort + r"\b", text):
                out.append(f"{rel.relative_to(root).as_posix()}: names the phantom sort "
                           f"{sort}, which no authority owns (5.5E1)")
    return out


def _batql_typing_rows(rule: str, word: str) -> list[str]:
    """The auditor's own rendering of one typing rule's projected rows. Must
    agree byte-for-byte with the generator's block, by independent code."""
    if rule == "SameUnit":
        return [f"Money<USD> {word} Money<USD>       -> Money<USD>",
                f"Money<USD> {word} Money<EUR>       -> rejected"]
    if rule == "DimensionalByDimensionless":
        if word == "/":
            return ["dimensional / dimensionless      -> same dimension"]
        return [f"Money<USD> {word} Decimal          -> Money<USD>",
                f"Money<USD> {word} Percent          -> Money<USD>",
                f"Duration {word} Integer            -> Duration",
                f"Money {word} Money                 -> rejected"]
    if rule == "LikeDimensionRatio":
        return [f"Money<USD> {word} Money<USD>       -> Ratio",
                f"Duration {word} Duration           -> Ratio",
                f"Money<USD> {word} Money<EUR>       -> rejected"]
    if rule == "PercentDifference":
        return [f"Percent {word} Percent             -> PercentagePoints"]
    if rule == "PercentAdjustment":
        if word == "+":
            return ["Percent + PercentagePoints       -> Percent",
                    "PercentagePoints + Percent       -> Percent"]
        return [f"Percent {word} PercentagePoints    -> Percent"]
    if rule == "WallObservationDifference":
        return [f"ObservedWallTime {word} ObservedWallTime -> TimeDelta"]
    return [f"UNKNOWN TYPING RULE {rule}"]


def batql_render_typing(ops: list[dict[str, str]]) -> str:
    titles = {"OP-ADD": "Addition", "OP-SUB": "Subtraction",
              "OP-MUL": "Multiplication", "OP-DIV": "Division"}
    lines: list[str] = []
    for op in _batql_canonical(ops):
        if op["id"] not in titles:
            continue
        lines += [f"### {titles[op['id']]} (`{op['word']}`)", "", "```text"]
        for rule in re.findall(r"OperatorTypingRule::(\w+)", op.get("typing", "")):
            lines += _batql_typing_rows(rule, op["word"])
        lines += ["```", ""]
    lines += ["### Comparison and truth", "", "```text",
              "comparisons        exact same-sort pair -> Truth with TypedMargin",
              "NOT                Truth -> Truth (K3 total)",
              "AND / OR           Truth x Truth -> Truth (K3 total)", "```"]
    return "\n".join(lines)


def batql_grammar_fences(companion: str) -> list[str]:
    start = companion.find("\n# 15.")
    if start < 0:
        return []
    end = companion.find("\n# 16.", start + 1)
    region = companion[start: end if end >= 0 else len(companion)]
    fences = re.findall(r"```text\n(.*?)\n```", region, re.S)
    return [fence for fence in fences if "  :=" in fence]


def batql_grammar_closure_findings(fences: list[str]) -> tuple[list[str], int, int]:
    """Returns (findings, defined_count, undefined_count)."""
    defined: list[str] = []
    referenced: set[str] = set()
    for fence in fences:
        for line in fence.split("\n"):
            if re.match(r"^[a-z][a-z0-9_]*$", line):
                defined.append(line)
                continue
            stripped = line.strip()
            if stripped.startswith(":=") or stripped.startswith("|"):
                rhs = stripped[2:] if stripped.startswith(":=") else stripped[1:]
                rhs = re.sub(r"<[^>]*>", " ", rhs)
                rhs = re.sub(r'"[^"]*"', " ", rhs)
                for token in re.findall(r"[a-z][a-z0-9_]*", rhs):
                    referenced.add(token)
    out: list[str] = []
    seen: set[str] = set()
    for name in defined:
        if name in seen:
            out.append(f"grammar nonterminal {name} has more than one owning production")
        seen.add(name)
    undefined = sorted(referenced - set(defined))
    for name in undefined:
        out.append(f"grammar references undefined nonterminal {name}")
    return out, len(set(defined)), len(undefined)


def batql_proof_disposition_states(companion: str) -> list[str]:
    match = re.search(r"## 4\.5 Proof disposition\n\n```text\n(.*?)\n```", companion, re.S)
    if not match:
        return []
    return [line.strip() for line in match.group(1).split("\n") if line.strip()]


def batql_proof_disposition_findings(states: list[str]) -> list[str]:
    if states != BATQL_PROOF_DISPOSITIONS:
        return [f"BatQL proof-disposition axis is {states}, expected {BATQL_PROOF_DISPOSITIONS}"]
    return []


def batql_language_change_findings(companion: str) -> list[str]:
    """FLOOR and CEILING are additive BatQL V1 syntax, not grammar repair. If the
    rounding-mode surface exposes them, the language-change record must exist;
    removing the record while keeping the new spellings is a silent syntax
    addition and turns the gate red."""
    match = re.search(r"rounding_mode\n  := ([^\n]+)", companion)
    modes = match.group(1) if match else ""
    has_new_spellings = "FLOOR" in modes and "CEILING" in modes
    has_record = "Language-change record (BatQL V1)" in companion
    if has_new_spellings and not has_record:
        return ["companion/BATQL_LANGUAGE.md: FLOOR/CEILING rounding spellings present without a BatQL V1 language-change record"]
    return []


def check_batql(root: Path, findings: list[str]) -> None:
    companion_path = root / "companion/BATQL_LANGUAGE.md"
    if not companion_path.is_file():
        findings.append("missing companion/BATQL_LANGUAGE.md")
        return
    companion = companion_path.read_text(encoding="utf-8")
    ops = batql_parse_operators(root)
    if not ops:
        findings.append("spec/operators.rs: no OperatorSpec rows parsed")
    findings.extend(batql_operator_fact_findings(ops))
    findings.extend(phantom_sort_findings(root))
    block_specs = (
        ("OPERATORS-CATALOG", "companion/BATQL_LANGUAGE.md", batql_render_catalog(ops)),
        ("OPERATORS-GRAMMAR", "companion/BATQL_LANGUAGE.md", batql_render_grammar(ops)),
        ("OPERATORS-PROJECTION", "companion/BATQL_LANGUAGE.md", batql_render_projection(ops)),
        ("OPERATORS-NUMERIC", "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md", batql_render_numeric(ops)),
        ("OPERATORS-TYPING", "companion/BATQL_LANGUAGE.md", batql_render_typing(ops)),
    )
    block_text: dict[str, str] = {}
    for name, rel, expected in block_specs:
        if rel not in block_text:
            block_path = root / rel
            block_text[rel] = block_path.read_text(encoding="utf-8") if block_path.is_file() else ""
        actual = batql_extract_block(block_text[rel], name)
        if actual is None:
            findings.append(f"{rel}: missing generated block {name}")
        elif actual != expected:
            findings.append(f"{rel}: generated block {name} does not match the spec/operators.rs projection")
    grammar_findings, _defined, _undefined = batql_grammar_closure_findings(batql_grammar_fences(companion))
    findings.extend(grammar_findings)
    findings.extend(batql_proof_disposition_findings(batql_proof_disposition_states(companion)))
    findings.extend(batql_language_change_findings(companion))
    if "`REQUIRE VERIFIED` accepts only `VERIFIED`" not in companion:
        findings.append("companion/BATQL_LANGUAGE.md 4.5: REQUIRE VERIFIED-only law missing")
    if "may upgrade or collapse" not in companion:
        findings.append("companion/BATQL_LANGUAGE.md 4.5: proof-disposition no-upgrade/no-collapse law missing")
    receipts_path = root / "docs/14_RECEIPTS_AND_EXPLANATION.md"
    if receipts_path.is_file() and "LegacyWeak" not in receipts_path.read_text(encoding="utf-8"):
        findings.append("docs/14_RECEIPTS_AND_EXPLANATION.md: LegacyWeak proof state missing")


# --- Numeric semantics (DEC-069 / docs/37) -----------------------------------
# Bootstrap enforces the numeric CONTRACT only. It never evaluates, quantizes,
# fuzzes, or interval-checks actual numbers; those are future executable gates.
NUMERIC_LAW_MARKERS = (
    ("approximate_value_marked_ordinary_authority", "does not make its approximate numeric meaning an authority value"),
    ("implicit_approximate_to_fixed_coercion", "no direct cast"),
    ("quantize_without_receipt", "produces a `QuantizationReceipt`"),
    ("nan_collapsed_into_missing", "Known(ApproximateBinary::NaN)"),
    ("nan_collapsed_into_pending", "Approximate does not imply"),
    ("signed_zero_evidence_identity_collapsed", "PositiveZero and NegativeZero remain distinct"),
    ("raw_bits_normalized_on_observation", "must not normalize observed raw bits"),
    ("non_finite_declared_quantizable", "infinity-to-fixed quantize"),
    ("interval_lower_greater_than_upper", "lower > upper"),
    ("interval_decision_lacking_pending_overlap", "overlaps the decision boundary"),
    ("default_rounding_on_authority_conversion", "default rounding mode"),
    ("wide_exact_made_ordinary_inline_authority", "ordinary inline EventFrame values"),
    # 5.5B2a: separate exact forms and split point/interval quantization
    ("fixed128_scoped_to_fixeddecimal", "canonical inline representation of `FixedDecimal` only"),
    ("non_singleton_interval_returns_no_point", "never returns one authoritative point"),
    ("quantizepoint_named_product", "named product"),
    ("quantizeinterval_floor_lower_bound", "lower bound with `Floor`"),
    ("quantizeinterval_ceiling_upper_bound", "upper bound with `Ceiling`"),
    ("interval_enclosure_contains_all_source_values", "contains every value the source interval contained"),
    ("estimation_is_not_quantization", "is not quantization"),
    ("exact_supported_refuses_raw_approximate", "never admits raw ApproximateBinary"),
)
NUMERIC_ROUNDING_MODES = ("HalfEven", "HalfAwayFromZero", "TowardZero", "AwayFromZero", "Floor", "Ceiling")
NUMERIC_INTERVAL_OPERATORS = ("IS (equal)", "IS NOT", "IS LESS THAN", "IS AT MOST", "IS MORE THAN", "IS AT LEAST")
NUMERIC_SHARED_SORTS = (
    "ExactInteger", "FixedDecimal", "ExactRatio", "WideExact", "ApproximateBinary",
    "Interval", "ProofDisposition", "TypedMargin",
)


def numeric_law_findings(doc37: str) -> list[str]:
    # Match whitespace-insensitively so a law that wraps across lines still counts.
    flat = re.sub(r"\s+", " ", doc37)
    out: list[str] = []
    for name, marker in NUMERIC_LAW_MARKERS:
        if re.sub(r"\s+", " ", marker) not in flat:
            out.append(f"docs/37: numeric law absent ({name})")
    return out


def check_numeric(root: Path, findings: list[str]) -> None:
    doc_path = root / "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md"
    if not doc_path.is_file():
        findings.append("missing docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md")
        return
    doc37 = doc_path.read_text(encoding="utf-8")
    flat = re.sub(r"\s+", " ", doc37)
    findings.extend(numeric_law_findings(doc37))
    for mode in NUMERIC_ROUNDING_MODES:
        if mode not in flat:
            findings.append(f"docs/37: canonical rounding mode {mode} missing")
    for operator in NUMERIC_INTERVAL_OPERATORS:
        if operator not in flat:
            findings.append(f"docs/37: IntervalDecision table missing comparison operator {operator!r}")
    layout_path = root / "docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md"
    if layout_path.is_file():
        layout = layout_path.read_text(encoding="utf-8")
        for sort in NUMERIC_SHARED_SORTS:
            if sort not in layout:
                findings.append(f"docs/04: shared semantic sort {sort} does not resolve")
    companion_path = root / "companion/BATQL_LANGUAGE.md"
    if companion_path.is_file():
        companion = companion_path.read_text(encoding="utf-8")
        if "Type-vocabulary record (BatQL V1)" not in companion:
            findings.append("companion/BATQL_LANGUAGE.md: numeric Type-vocabulary record (BatQL V1) missing")


# --- Guarantee graph (DEC-070): INDEPENDENT derivation, comparison, rules -----
# This is the AUDITOR half. It derives guarantee nodes/edges from the source
# facts with its own parsers and rules, then compares against the generated
# artifacts. It shares no parsing, normalization, lifetime derivation, edge
# construction, cycle detection, serialization, or comparison logic with
# bootstrap/project.py.
# Independent gate identity reader. This deliberately does NOT share parsing,
# ordering, or rendering with bootstrap/project.py: the auditor extracts variants
# by pattern rather than by positional split, and builds its own canonical order
# from spec/gates.rs. The oracle cannot manufacture the evidence it grades.
A_GATE_SPEC = re.compile(r'GateSpec \{ id: GateId::(\w+), token: "([^"]*)", title: "([^"]*)" \}')
A_GATE_VARIANT = re.compile(r'GateId::(\w+)')
A_GATE_ENUM = re.compile(r'pub enum GateId \{([^}]*)\}', re.S)


def gate_inventory(root: Path) -> list[tuple[str, str, str]]:
    return A_GATE_SPEC.findall((root / "spec" / "gates.rs").read_text(encoding="utf-8"))


def gate_enum_variants(root: Path) -> list[str]:
    m = A_GATE_ENUM.search((root / "spec" / "gates.rs").read_text(encoding="utf-8"))
    if not m:
        return []
    return [v.strip().rstrip(",") for v in m.group(1).split() if v.strip().rstrip(",")]


def gate_list(expr: str) -> list[str]:
    """Variants named by a typed gate list, in source order."""
    return A_GATE_VARIANT.findall(expr)


def gate_render(names: list[str], root: Path) -> str:
    token_of = {g[0]: g[1] for g in gate_inventory(root)}
    return "/".join(token_of.get(n, f"<unknown:{n}>") for n in names)


def gate_findings(root: Path, label: str, ident: str, names: list[str]) -> list[str]:
    """Resolvable, duplicate-free, canonically ordered."""
    inv = [g[0] for g in gate_inventory(root)]
    out = []
    for n in names:
        if n not in inv:
            out.append(f"{label} {ident} names unknown GateId {n}")
    if len(names) != len(set(names)):
        out.append(f"{label} {ident} names a duplicate GateId")
    known = [n for n in names if n in inv]
    if known != sorted(known, key=inv.index):
        out.append(f"{label} {ident} gate list is not in canonical order")
    return out


G_FAMILY_RANK = {"SEED": 0, "LEG": 1, "DEC": 2, "ARCH": 3, "QUAL": 4}
G_SEED_ROW = re.compile(
    r'InvariantSpec \{ id: "([^"]+)", statement: "(?:[^"\\]|\\.)*", '
    r'kind: GuaranteeKind::(\w+), lifetime: GuaranteeLifetime::(\w+), '
    r'owner: "([^"]*)", gates: &\[([^\]]*)\], witnesses: &\[([^\]]*)\], '
    r'witness_note: "([^"]*)", '
    r'failure_disposition: "([^"]*)", derives_from: &\[([^\]]*)\], '
    r'refines: &\[([^\]]*)\], discharges: &\[([^\]]*)\], supersedes: &\[([^\]]*)\] \}'
)
# The canonical authored witness forms (5.5E2). This auditor recognizes ONLY
# these; anything else in a witnesses slice is a citation trying to bypass
# the typed vocabulary and is refused by guarantee_typed_witness_findings.
G_WITNESS_TAGGED = re.compile(r'WitnessRef::(leg|dec|contract)\("([^"]+)"\)')
G_WITNESS_TOOL = re.compile(r"WitnessRef::BootstrapTool\(BootstrapToolId::(\w+)\)")
G_WITNESS_TOOL_NAMES = {
    "ProjectPy": "project.py", "AuditPy": "audit.py", "FreezePy": "freeze.py",
    "SelftestPy": "selftest.py", "Seedcheck": "seedcheck", "Materialize": "materialize",
}


def g_witness_render(refs_raw: str, note: str) -> str:
    """The auditor's own rendering of a witness column, compared byte-for-byte
    against the generator's projection."""
    parts: list[str] = []
    for m in re.finditer(r"WitnessRef::\w+", refs_raw):
        tagged = G_WITNESS_TAGGED.match(refs_raw, m.start())
        if tagged:
            parts.append(tagged.group(2))
            continue
        tool = G_WITNESS_TOOL.match(refs_raw, m.start())
        if tool and tool.group(1) in G_WITNESS_TOOL_NAMES:
            parts.append(G_WITNESS_TOOL_NAMES[tool.group(1)])
    text = "; ".join(parts)
    return f"{text} -- {note}" if note else text
G_LEG_ROW = re.compile(
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", clean_owner: "([^"]+)", '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::(\w+), '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: ObligationStatus::(\w+) \}'
)
G_DEC_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+),'
)
G_PKG_ROW = re.compile(
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)


def g_package_projection(root: Path, fn: str) -> dict[str, str]:
    """PackageId variant -> projected value, parsed from the owning const fn
    (cargo_name or workspace_path). This auditor reconstructs the projections
    independently; it carries no hand-maintained package map."""
    src = _uncomment((root / "spec/architecture.rs").read_text(encoding="utf-8"))
    body = re.search(
        r"pub const fn " + re.escape(fn)
        + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        src, re.S)
    return dict(re.findall(r'PackageId::(\w+) => "([^"]+)",', body.group(1))) if body else {}


G_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "[^"]+",\s*\}', re.S)
# The auditor's own spelling of each environment variant, compared
# byte-for-byte against the generator's projection through the blocks.
G_ENVIRONMENT_SPELLING = {"NoStdAlloc": "no_std + alloc", "NativeStd": "std",
                          "WasmHost": "wasm32 host"}
GUARANTEE_KINDS = {"SemanticLaw", "ArchitectureConstraint", "BootstrapAssertion",
                   "LegacyObligation", "QualificationRequirement", "Decision"}
GUARANTEE_LIFETIMES = {"Permanent", "UntilGate", "UntilCompatibilityExpiry",
                       "UntilSuccessor", "HistoricalCoverageOnly", "ClosedEvidence"}


def _g_ids(raw: str) -> list[str]:
    return re.findall(r'"([^"]+)"', raw)


def guarantee_seed_rows(root: Path) -> list[dict]:
    src = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    rows = []
    for m in G_SEED_ROW.findall(src):
        rows.append({
            "id": m[0], "kind": m[1], "lifetime": m[2], "owner": m[3],
            "gate_names": gate_list(m[4]),
            "witness": g_witness_render(m[5], m[6]),
            "witness_raw": m[5], "failure": m[7],
            "rel": {"DerivesFrom": _g_ids(m[8]), "Refines": _g_ids(m[9]),
                    "Discharges": _g_ids(m[10]), "Supersedes": _g_ids(m[11])},
            "rel_raw": {"DerivesFrom": m[8], "Refines": m[9],
                        "Discharges": m[10], "Supersedes": m[11]},
        })
    return rows


# Relations are typed GuaranteeRef constructors (5.5E2): the family tag lives
# in the TYPE, and each tag owns exactly one id prefix. A bare string or a
# mistagged reference is a relation trying to bypass the typed identity.
G_REL_CONSTRUCTOR = re.compile(r"GuaranteeRef::(\w+)\(\"([^\"]+)\"\)")
G_REL_TAG_PREFIX = {"leg": "LEG-", "dec": "DEC-"}


def guarantee_typed_relation_findings(rows: list[dict]) -> list[str]:
    """Constructor-form laws for authored relations. Distinct from
    guarantee_relation_findings below, which owns the derived EDGE laws
    (dangling, self-reference, duplicates, cycles) on the graph."""
    out: list[str] = []
    for r in rows:
        for rel, raw in r.get("rel_raw", {}).items():
            if not raw.strip():
                continue
            typed = G_REL_CONSTRUCTOR.findall(raw)
            quoted = re.findall(r'"([^"]+)"', raw)
            if len(typed) != len(quoted):
                out.append(f"{r['id']} {rel} carries a relation that is not a "
                           "typed GuaranteeRef constructor")
                continue
            for tag, ident in typed:
                want = G_REL_TAG_PREFIX.get(tag)
                if want is None:
                    out.append(f"{r['id']} {rel} uses undeclared relation "
                               f"constructor GuaranteeRef::{tag}")
                elif not ident.startswith(want):
                    out.append(f"{r['id']} {rel} tags {ident!r} as "
                               f"GuaranteeRef::{tag}, whose family owns the "
                               f"{want} prefix")
    return out


def declared_contract_ids(root: Path) -> set[str]:
    """Every contract_id declared by a document's front matter."""
    out: set[str] = set()
    for path in sorted(root.rglob("*.md")):
        rel = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = path.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            out.add(cid)
    return out


def guarantee_typed_witness_findings(root: Path, rows: list[dict]) -> list[str]:
    """Witness citations are canonical typed constructors that RESOLVE: a
    bare string, a mistagged family, an unknown contract id, or an undeclared
    tool is a citation trying to bypass the typed vocabulary."""
    out: list[str] = []
    contract_ids = declared_contract_ids(root)
    for r in rows:
        raw = r.get("witness_raw", "")
        refs = re.findall(r"WitnessRef::\w+", raw)
        tagged = G_WITNESS_TAGGED.findall(raw)
        tools = G_WITNESS_TOOL.findall(raw)
        quoted = re.findall(r'"([^"]+)"', raw)
        # Precedence matters for refusal identity: content that is not a
        # canonical constructor (a bare string, a malformed call) is "not
        # canonical"; only a genuinely EMPTY slice is "no citation".
        if len(tagged) + len(tools) != len(refs) or len(quoted) != len(tagged):
            out.append(f"{r['id']} carries a witness that is not a canonical typed WitnessRef")
            continue
        if not refs:
            out.append(f"{r['id']} declares no typed witness citation")
            continue
        for tag, ident in tagged:
            if tag == "leg" and not ident.startswith("LEG-"):
                out.append(f"{r['id']} witness tags {ident!r} as WitnessRef::leg, "
                           "whose family owns the LEG- prefix")
            elif tag == "dec" and not ident.startswith("DEC-"):
                out.append(f"{r['id']} witness tags {ident!r} as WitnessRef::dec, "
                           "whose family owns the DEC- prefix")
            elif tag == "contract" and ident not in contract_ids:
                out.append(f"{r['id']} witness cites contract {ident}, "
                           "which no document declares")
        for tool in tools:
            if tool not in G_WITNESS_TOOL_NAMES:
                out.append(f"{r['id']} witness cites undeclared bootstrap tool {tool}")
    return out


def guarantee_leg_meta(root: Path) -> dict[str, dict]:
    leg = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    out = {}
    for lid, owner, gate, compat, deletion, status in G_LEG_ROW.findall(leg):  # noqa: E501
        out[lid] = {"owner": owner, "gate_names": gate_list(gate), "compat": compat,
                    "deletion": deletion, "status": status}
    return out


def _leg_lifetime(meta: dict) -> str:
    # A LEG row names a clean owner and gates but no typed successor GuaranteeRef,
    # so an active gate-closed obligation is UntilGate. UntilSuccessor requires a
    # resolved typed successor, which the LEG schema does not provide.
    if meta["status"] == "Closed":
        return "ClosedEvidence"
    if meta["deletion"] == "OnCompatibilityWindowExpiry":
        return "UntilCompatibilityExpiry"
    return "UntilGate"


# --- Typed guarantee admission, independently parsed (5.5D4b) ----------------
# This auditor no longer authors family semantics. It parses the SAME typed
# policy the projector parses -- spec/guarantees.rs GUARANTEE_FAMILY_POLICIES and
# the derivations owned by spec/dispositions.rs -- and verifies that every
# projected field traces to it. Independent parsers are useful; duplicate policy
# is not independence. The nine constants that used to live in both scripts are
# gone from both.
def _uncomment(src: str) -> str:
    return re.sub(r"//[^\n]*", "", src)


A_FAM_ROW = re.compile(
    r'GuaranteeFamilyPolicy \{\s*family: "(\w+)",\s*'
    r'kind: KindRule::(\w+)(?:\(GuaranteeKind::(\w+)\))?,\s*'
    r'owner: OwnerRule::(\w+)(?:\("([^"]*)"\))?,\s*'
    r'lifetime: LifetimeRule::(\w+)(?:\(GuaranteeLifetime::(\w+)\))?,\s*'
    r'gate_posture: GatePostureRule::(\w+)(?:\(GatePosture::Scheduled\(&\[([^\]]*)\]\)\))?,\s*'
    r'witness: WitnessPosture::(\w+),\s*\}',
    re.S,
)


def guarantee_family_policies(root: Path) -> dict[str, dict]:
    src = _uncomment((root / "spec/guarantees.rs").read_text(encoding="utf-8"))
    out: dict[str, dict] = {}
    for m in A_FAM_ROW.finditer(src):
        fam, krule, kconst, orule, oconst, lrule, lconst, grule, gconst, wit = m.groups()
        out[fam] = {"kind": (krule, kconst), "owner": (orule, oconst),
                    "lifetime": (lrule, lconst), "gate_posture": (grule, gconst),
                    "witness": wit}
    return out


def decision_lifetime_map(root: Path) -> dict[str, str]:
    """Disposition -> GuaranteeLifetime from the typed owner's match arms."""
    src = _uncomment((root / "spec/dispositions.rs").read_text(encoding="utf-8"))
    body = re.search(r"pub const fn guarantee_lifetime\(self\) -> GuaranteeLifetime \{(.*?)\n    \}",
                     src, re.S)
    out: dict[str, str] = {}
    if not body:
        return out
    for arm in re.finditer(r"((?:Disposition::\w+\s*\|?\s*)+)=>\s*\{?\s*GuaranteeLifetime::(\w+)",
                           body.group(1), re.S):
        for d in re.findall(r"Disposition::(\w+)", arm.group(1)):
            out[d] = arm.group(2)
    return out


def gate_optional_decision_classes(root: Path) -> set[str]:
    src = _uncomment((root / "spec/dispositions.rs").read_text(encoding="utf-8"))
    body = re.search(r"pub const fn requires_gate\(self\) -> bool \{(.*?)\n    \}", src, re.S)
    if not body:
        return set()
    arms = re.split(r"=>\s*true\s*,", body.group(1))
    if len(arms) < 2:
        return set()
    return set(re.findall(r"DecisionClass::(\w+)", arms[1]))


class AdmissionFailure(Exception):
    """A required GuaranteeView field has no authored source."""


def admit_view(root: Path, family: str, ident: str, row: dict, failure: str) -> dict:
    pol = guarantee_family_policies(root).get(family)
    if pol is None:
        raise AdmissionFailure(f"{ident}: no declared family policy for {family}")

    def pick(dim, row_value, derived=None):
        rule, const = pol[dim]
        if rule == "RowDeclared":
            if row_value in (None, ""):
                raise AdmissionFailure(f"{ident}: {dim} is RowDeclared but the row declares none")
            return row_value
        if rule == "FamilyConstant":
            if not const:
                raise AdmissionFailure(f"{ident}: {dim} is FamilyConstant but names no value")
            return const
        if derived is None:
            raise AdmissionFailure(f"{ident}: {dim} names derivation {rule} but none was supplied")
        return derived

    kind = pick("kind", row.get("kind"))
    owner = pick("owner", row.get("owner"))
    lrule = pol["lifetime"][0]
    derived = None
    if lrule == "FromDecisionDisposition":
        derived = decision_lifetime_map(root).get(row.get("disposition"))
        if derived is None:
            raise AdmissionFailure(f"{ident}: disposition {row.get('disposition')!r} has no typed lifetime")
    elif lrule == "FromLegacyStatusAndDeletion":
        derived = row.get("derived_lifetime")
    lifetime = pick("lifetime", row.get("lifetime"), derived)
    grule, gconst = pol["gate_posture"]
    if grule == "FamilyConstant":
        posture = gate_render(gate_list(gconst), root)
    elif grule == "FromDecisionDispositionClassAndGates":
        if row.get("gates"):
            # A retired decision keeps its authored GateIds as PROVENANCE: the
            # references are preserved, never deleted, never rendered as active.
            posture = (f"historical:{row['gates']}"
                       if lifetime == "HistoricalCoverageOnly" else row["gates"])
        elif row.get("class") in gate_optional_decision_classes(root):
            posture = "GateIndependent"
        else:
            raise AdmissionFailure(
                f"{ident}: class {row.get('class')} requires a gate but the row declares none")
    else:
        if row.get("gates") in (None, ""):
            raise AdmissionFailure(f"{ident}: gate posture is RowDeclared but the row declares none")
        posture = row["gates"]
    # Type domain: only a declared GateId may inhabit a gate posture. gate_render
    # marks an unresolvable variant rather than raising, so a qualification target
    # or any other free string reaching this field fails admission here.
    if "<unknown:" in posture:
        raise AdmissionFailure(
            f"{ident}: gate posture {posture!r} names a value that is not a declared GateId")
    return {"id": ident, "family": family, "kind": kind, "lifetime": lifetime, "owner": owner,
            "gate_posture": posture, "target": row.get("target", ""),
            "witness": pol["witness"], "failure": failure}


def guarantee_admission_findings(root: Path) -> list[str]:
    """Rows that cannot be admitted. Fail-closed means REPORTED, not raised: a
    crash is not a diagnostic, and a hostile probe that strips a required field
    must produce a finding a human can read."""
    return guarantee_derive(root)[2]


def guarantee_derive(root: Path) -> tuple[list[dict], list[tuple], list[str]]:
    nodes = []
    admission: list[str] = []

    def _admit(family, ident, row, failure):
        try:
            return admit_view(root, family, ident, row, failure)
        except AdmissionFailure as exc:
            admission.append(f"guarantee admission failed: {exc}")
            return None

    for s in guarantee_seed_rows(root):
        nodes.append(_admit("SEED", s["id"], {
            "kind": s["kind"], "owner": s["owner"], "lifetime": s["lifetime"],
            "gates": gate_render(s["gate_names"], root), "witness": s["witness"],
        }, f"Seed({s['failure']})"))
    for lid, meta in guarantee_leg_meta(root).items():
        nodes.append(_admit("LEG", lid, {
            "owner": meta["owner"], "gates": gate_render(meta["gate_names"], root),
            "derived_lifetime": _leg_lifetime(meta),
        }, "Legacy"))
    dec = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    for did, dcls, dgates, disp in G_DEC_ROW.findall(dec):
        # Decision gates are node metadata; they never become graph edges.
        node = _admit("DEC", did, {
            "gates": gate_render(gate_list(dgates), root), "disposition": disp, "class": dcls,
        }, f"Decision({disp})")
        if node is not None:
            node["dclass"] = dcls
        nodes.append(node)
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    cargo = g_package_projection(root, "cargo_name")
    for pkg, cls, layer in G_PKG_ROW.findall(arch):
        nodes.append(_admit("ARCH", f"ARCH-{cargo.get(pkg, '')}", {}, f"Architecture(L{layer} {cls})"))
    for pkg, profile, variant, qgates in G_QUAL_ROW.findall(arch):
        # environment is the SEMANTIC dimension; qgates is the SCHEDULE.
        # Never interchanged. The node projects the canonical spelling.
        spelling = G_ENVIRONMENT_SPELLING.get(variant, "")
        nodes.append(_admit("QUAL", f"QUAL-{cargo.get(pkg, '')}-{profile}", {
            "gates": gate_render(gate_list(qgates), root), "target": spelling,
        }, f"Qualification({spelling})"))
    nodes = [n for n in nodes if n is not None]
    nodes.sort(key=lambda n: (G_FAMILY_RANK[n["family"]], n["id"]))
    edges = []
    for s in guarantee_seed_rows(root):
        for kind in ("DerivesFrom", "Refines", "Discharges", "Supersedes"):
            for target in s["rel"][kind]:
                edges.append((s["id"], kind, target))
    edges.sort(key=lambda e: (e[0], e[1], e[2]))
    return nodes, edges, admission


def _g_has_cycle(adj: dict[str, list[str]]) -> bool:
    color = {}

    def visit(u):
        color[u] = 1
        for v in adj.get(u, []):
            c = color.get(v, 0)
            if c == 1 or (c == 0 and visit(v)):
                return True
        color[u] = 2
        return False

    return any(color.get(n, 0) == 0 and visit(n) for n in list(adj))


def guarantee_relation_findings(node_ids: set[str], edges: list[tuple]) -> list[str]:
    out = []
    seen = set()
    for s, k, t in edges:
        if s == t:
            out.append(f"guarantee self-edge {s} {k}")
        if (s, k, t) in seen:
            out.append(f"duplicate guarantee edge {s} {k} {t}")
        seen.add((s, k, t))
        if t not in node_ids:
            out.append(f"guarantee relation target {t} does not resolve ({s} {k})")
        if k in ("Discharges", "Closes"):
            out.append(f"guarantee {k} edge {s}->{t} requires qualifying evidence or a closure receipt, absent before Gate 0")
    for fam in ("DerivesFrom", "Refines", "Supersedes", "Discharges"):
        adj: dict[str, list[str]] = {}
        for s, k, t in edges:
            if k == fam:
                adj.setdefault(s, []).append(t)
        if _g_has_cycle(adj):
            out.append(f"guarantee {fam} relation has a cycle")
    return out


# Kind-aware Permanent-witness enforcement. Decision witnessing (gate binding) is
# deferred to 5.5C2; QualificationRequirement is itself a qualification contract.
PERMANENT_WITNESS_KINDS = {"SemanticLaw", "BootstrapAssertion", "ArchitectureConstraint"}


def _is_scheduled(posture: str) -> bool:
    """An ACTIVE gate schedule. `historical:` provenance and `GateIndependent`
    are not schedules; both are authored postures, not missing data."""
    return bool(posture.strip()) and posture != "GateIndependent"         and not posture.startswith("historical:")


def guarantee_lifetime_findings(nodes: list[dict], leg_meta: dict[str, dict], edges: list[tuple]) -> list[str]:
    node_ids = {n["id"] for n in nodes}
    # A guarantee is validly UntilSuccessor only if a resolvable typed successor
    # supersedes it. clean_owner text and gate names are never successor edges.
    superseded = {t for s, k, t in edges if k == "Supersedes" and t in node_ids}
    out = []
    for n in nodes:
        life, posture, kind = n["lifetime"], n["gate_posture"], n["kind"]
        # "Cannot gate" is about carrying an active implementation obligation. A
        # No family exemption. A retired decision keeps its authored GateIds as a
        # HistoricalAssociation, which is provenance rather than an active
        # schedule, so it satisfies "cannot gate" by construction instead of by
        # being excused from the rule. Before 5.5D4b no node was ever
        # HistoricalCoverageOnly, so this rule had never once fired.
        if life == "HistoricalCoverageOnly" and _is_scheduled(posture):
            out.append(f"{n['id']} HistoricalCoverageOnly cannot be actively scheduled")
        if life == "UntilGate" and not _is_scheduled(posture):
            out.append(f"{n['id']} UntilGate names no active gate schedule")
        if life == "UntilSuccessor" and n["id"] not in superseded:
            out.append(f"{n['id']} UntilSuccessor names no resolvable typed successor relation")
        if life == "Permanent" and kind in PERMANENT_WITNESS_KINDS \
                and n["witness"] == "NoFamilyWitness":
            out.append(f"{n['id']} Permanent {kind} names no witness")
        if n["family"] == "LEG":
            meta = leg_meta.get(n["id"], {})
            if meta.get("status") == "Active" and life == "ClosedEvidence":
                out.append(f"{n['id']} Active LEG projects as ClosedEvidence")
            if meta.get("status") == "Closed" and life != "ClosedEvidence":
                out.append(f"{n['id']} Closed LEG does not project as ClosedEvidence")
            if meta.get("status") == "Closed" and _is_scheduled(posture):
                out.append(f"{n['id']} Closed LEG still names an implementation gate")
            if life == "Permanent":
                out.append(f"{n['id']} LEG projects as Permanent; DeletionCondition::Never must not force Permanent")
            if life == "UntilCompatibilityExpiry" and meta.get("compat") == "None":
                out.append(f"{n['id']} UntilCompatibilityExpiry names no compatibility condition")
    return out


def guarantee_classification_findings(seed_rows: list[dict]) -> list[str]:
    out = []
    if len(seed_rows) != 25:
        out.append(f"SEED classification has {len(seed_rows)} rows, expected 25")
    ids = [r["id"] for r in seed_rows]
    if len(ids) != len(set(ids)):
        out.append("duplicate SEED guarantee id")
    for r in seed_rows:
        if r["kind"] not in GUARANTEE_KINDS:
            out.append(f"{r['id']} unknown guarantee kind {r['kind']}")
        if r["lifetime"] not in GUARANTEE_LIFETIMES:
            out.append(f"{r['id']} unknown guarantee lifetime {r['lifetime']}")
        if not r["owner"].strip():
            out.append(f"{r['id']} classified without an owner")
        if not r["witness"].strip():
            out.append(f"{r['id']} classified without a witness")
    return out


def _md_section_rows(text: str, title: str) -> list[list[str]]:
    start = text.find(f"\n## {title}\n")
    if start < 0:
        return []
    region = text[start + 1:]
    nxt = region.find("\n## ")
    region = region[:nxt] if nxt >= 0 else region
    rows = []
    for line in region.splitlines():
        line = line.strip()
        if line.startswith("| ") and not line.startswith("| ---") and "GuaranteeId" not in line and "Source |" not in line:
            rows.append([c.strip() for c in line.split("|")[1:-1]])
    return rows


def gate_inventory_findings(root: Path) -> list[str]:
    """The gate inventory is the one gate identity: complete, unique, ordered."""
    inv = gate_inventory(root)
    variants = gate_enum_variants(root)
    out = []
    if not inv:
        out.append("spec/gates.rs declares no GATES inventory")
        return out
    ids = [g[0] for g in inv]
    tokens = [g[1] for g in inv]
    for v in variants:
        if v not in ids:
            out.append(f"GateId::{v} is missing from the GATES inventory")
    for i in ids:
        if i not in variants:
            out.append(f"GATES names {i}, which is not a GateId variant")
    if len(ids) != len(set(ids)):
        out.append("GATES declares a duplicate GateId")
    if len(tokens) != len(set(tokens)):
        out.append("GATES declares one token under two GateIds")
    if ids != variants:
        out.append("GATES order does not equal GateId declaration order")
    for gid, token, title in inv:
        if not token.strip():
            out.append(f"gate {gid} has an empty token")
        if not title.strip():
            out.append(f"gate {gid} has an empty title")
    return out


def gate_reference_findings(root: Path) -> list[str]:
    """Every typed gate reference in every gate-bearing fact family resolves."""
    out = []
    for row in guarantee_seed_rows(root):
        out.extend(gate_findings(root, "SEED", row["id"], row["gate_names"]))
        if not row["gate_names"]:
            out.append(f"SEED {row['id']} names no gate")
    for lid, meta in guarantee_leg_meta(root).items():
        out.extend(gate_findings(root, "LEG", lid, meta["gate_names"]))
        if not meta["gate_names"]:
            out.append(f"LEG {lid} names no gate")
    src_inv = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    src_leg = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    if re.search(r'\bgates?:\s*"', src_inv):
        out.append("spec/invariants.rs still carries a string gate field")
    if re.search(r'\bgates?:\s*"', src_leg):
        out.append("spec/legacy_obligations.rs still carries a string gate field")
    return out


GATE_BLOCK_RE = re.compile(
    r"<!-- GATE-INVENTORY:BEGIN[^>]*-->\n(.*?)\n<!-- GATE-INVENTORY:END -->", re.S
)


def gate_doc_findings(root: Path) -> list[str]:
    """docs/25 gate inventory block equals spec/gates.rs, recomputed independently."""
    doc = (root / "docs/25_IMPLEMENTATION_GATES.md").read_text(encoding="utf-8")
    m = GATE_BLOCK_RE.search(doc)
    if not m:
        return ["docs/25_IMPLEMENTATION_GATES.md: missing GATE-INVENTORY block"]
    rows = []
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line.startswith("|") or set(line) <= set("|- "):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) == 3 and cells[0] != "GateId":
            rows.append(tuple(cells))
    want = [(g[0], g[1], g[2]) for g in gate_inventory(root)]
    if rows != want:
        return ["docs/25 GATE-INVENTORY block does not equal spec/gates.rs"]
    return []


# --- Decision classification and gate binding (DEC-072) ---------------------
# The AUDITOR half. Parses spec/dispositions.rs independently of
# bootstrap/project.py and re-derives every rule from the typed row. The class,
# never the title/ID range/document section/keyword, decides gate obligation.
A_DEC_CLASS_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+), subject: "([^"]*)"'
)
A_DEC_CLASSES = {
    "Architecture", "Capability", "Compatibility", "Enforcement",
    "HistoricalReceipt", "Naming", "ImplementationPosture",
}
# Implementation-bearing classes must name a gate. There is no vague
# architecture subclass that dodges gate binding.
A_DEC_GATE_REQUIRED = {
    "Architecture", "Capability", "Compatibility", "Enforcement", "ImplementationPosture",
}
A_DEC_GATE_OPTIONAL = {"HistoricalReceipt", "Naming"}


def decision_rows(root: Path) -> list[dict]:
    src = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    out = []
    for did, cls, gates, disp, subject in A_DEC_CLASS_ROW.findall(src):
        out.append({"id": did, "class": cls, "gate_names": gate_list(gates),
                    "disposition": disp, "subject": subject})
    return out


def decision_class_findings(root: Path) -> list[str]:
    rows = decision_rows(root)
    out: list[str] = []
    declared = len(re.findall(r'DecisionSpec \{ id: "DEC-', (root / "spec/dispositions.rs").read_text(encoding="utf-8")))
    if len(rows) != declared:
        out.append(f"{declared - len(rows)} DecisionSpec row(s) carry no DecisionClass")
    for r in rows:
        if r["class"] not in A_DEC_CLASSES:
            out.append(f"{r['id']} has unknown DecisionClass {r['class']}")
            continue
        out.extend(gate_findings(root, "DEC", r["id"], r["gate_names"]))
        if r["class"] in A_DEC_GATE_REQUIRED and not r["gate_names"]:
            out.append(f"{r['id']} is {r['class']} and names no implementation or qualification gate")
        if r["class"] not in A_DEC_GATE_REQUIRED and r["class"] not in A_DEC_GATE_OPTIONAL:
            out.append(f"{r['id']} class {r['class']} has no gate rule")
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
A_DISPOSITION_VARIANT = re.compile(r'WitnessDisposition::(\w+)')
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
    src = _uncomment((root / "spec/architecture.rs").read_text(encoding="utf-8"))
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
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    return {m[0]: (m[1], m[2], m[3], m[4]) for m in A_CLAIM_BUNDLE.findall(src)}


def _enum_variants(root: Path, name: str) -> set[str]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    m = re.search(r"pub enum " + re.escape(name) + r" \{(.*?)\n\}", src, re.S)
    return set(re.findall(r"^\s{4}(\w+),", m.group(1), re.M)) if m else set()


def _const_variants(root: Path, name: str) -> list[str]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    m = re.search(r"pub const " + re.escape(name) + r":[^=]*=\s*&\[(.*?)\];", src, re.S)
    return A_DISPOSITION_VARIANT.findall(m.group(1)) if m else []


def claim_axis_findings(root: Path) -> list[str]:
    """Four independently representable claim axes, no ladder, no flattening."""
    out: list[str] = []
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
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
        integrity, authenticity, freshness, rollback = got
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
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
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
    for rel in ("spec/architecture.rs", "spec/dispositions.rs", "spec/guarantees.rs",
                "docs/14_RECEIPTS_AND_EXPLANATION.md", "docs/19_SECURITY_MODEL.md",
                "docs/05_STORAGE_FBAT_AND_TILES.md", "docs/31_FINAL_CONTRADICTION_AUDIT.md"):
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


# --- Control-character integrity --------------------------------------------
# 5.5C2a shipped a rule containing an ASCII backspace impersonating a word
# boundary; grep and sed rendered it invisibly. Tracked specification text is
# LF-only and carries no other C0 controls.
CONTROL_TEXT_SUFFIXES = {".py", ".rs", ".md", ".sha256", ".toml"}
CONTROL_TEXT_NAMES = {".gitattributes", ".gitignore"}
CONTROL_ALLOWED = {0x09, 0x0A}  # TAB, LF. CR is rejected: the tree freezes LF.


def control_character_findings(root: Path) -> list[str]:
    out: list[str] = []
    for rel, path in sorted(frozen_files(root).items()):
        if path.suffix not in CONTROL_TEXT_SUFFIXES and path.name not in CONTROL_TEXT_NAMES:
            continue
        data = path.read_bytes()
        for offset, byte in enumerate(data):
            if byte < 0x20 and byte not in CONTROL_ALLOWED or byte == 0x7F:
                out.append(f"{rel}: byte {offset}: forbidden control character U+{byte:04X}")
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
        ("no oracle, no promotion", "No oracle, no promotion."),
        ("no named obligation, no promotion", "No invariant or named proof obligation, no promotion."),
        ("no killed mutant, no promotion",
         "No killed mutant or equivalent hostile evidence, no promotion."),
        ("no proof receipt, no trust", "No proof receipt, no trust."),
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
    src = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    laws = dict(re.findall(r'LegacyObligation \{ id: "(LEG-\d+)", law: "([^"]+)"', src))
    out = []
    for lid, clauses in D1_LEG_CLAUSES.items():
        law = laws.get(lid)
        if law is None:
            out.append(f"{lid} is absent from spec/legacy_obligations.rs")
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
        out.append("DEC-073 is absent from spec/dispositions.rs")
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


def _arch_enum(root: Path, name: str) -> list[str]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    m = re.search(r"pub enum " + re.escape(name) + r" \{(.*?)\n\}", src, re.S)
    return re.findall(r"^\s{4}(\w+),", m.group(1), re.M) if m else []


def proof_policy_findings(root: Path) -> list[str]:
    """DEC-074 identity plus the frozen mutation and policy vocabularies."""
    out: list[str] = []
    rows = {r["id"]: r for r in decision_rows(root)}
    dec = rows.get("DEC-074")
    if dec is None:
        out.append("DEC-074 is absent from spec/dispositions.rs")
    else:
        if dec["class"] != "Enforcement":
            out.append(f"DEC-074 class {dec['class']} is not Enforcement")
        for gate in ("G3", "G9"):
            if gate not in dec["gate_names"]:
                out.append(f"DEC-074 does not name gate {gate}")
    d15 = rows.get("DEC-015")
    if d15 is None or "Mutation testing only inside TestPak" not in d15["subject"] + str(d15):
        src = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
        if "Mutation testing only inside TestPak" not in src:
            out.append("DEC-015 no longer confines mutation testing to TestPak")
    for name, want in (("MutationLane", D3_LANES), ("MutationResult", D3_RESULTS),
                       ("ProofPolicyChangeClass", D3_CHANGE_CLASSES),
                       ("ProofPolicySurface", D3_POLICY_SURFACES)):
        got = _arch_enum(root, name)
        if not got:
            out.append(f"spec/architecture.rs declares no {name}")
        elif got != want:
            out.append(f"{name} variants {got} != frozen {want}")
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    lanes = re.findall(r"lane: MutationLane::(\w+),", src)
    if lanes != D3_LANES:
        out.append(f"MUTATION_LANES rows {lanes} != frozen three lanes {D3_LANES}")
    for const, want in (("NEVER_KILLED", D3_NEVER_KILLED), ("NEVER_SURVIVED", D3_NEVER_KILLED)):
        m = re.search(r"pub const " + const + r":[^=]*=\s*&\[(.*?)\];", src, re.S)
        got = re.findall(r"MutationResult::(\w+)", m.group(1)) if m else []
        if sorted(got) != sorted(want):
            out.append(f"{const} {sorted(got)} != frozen {sorted(want)}")
    m = re.search(r"pub const TERMINAL_MUTATION_RESULTS:[^=]*=\s*&\[(.*?)\];", src, re.S)
    terminal = re.findall(r"MutationResult::(\w+)", m.group(1)) if m else []
    if sorted(terminal) != sorted(D3_RESULTS):
        out.append(f"TERMINAL_MUTATION_RESULTS {sorted(terminal)} != all eight categories")
    if "target/muterprater/candidates/" not in src:
        out.append("no candidate output root is declared outside tracked source")
    m = re.search(r"pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS:[^=]*=\s*&\[(.*?)\];", src, re.S)
    forbidden = re.findall(r'"([^"]+)"', m.group(1)) if m else []
    for rootdir in D3_FORBIDDEN_CANDIDATE_ROOTS:
        if rootdir not in forbidden:
            out.append(f"candidate generation is not forbidden from writing {rootdir}")
    m = re.search(r"pub const PROMOTION_REQUIREMENTS:[^=]*=\s*&\[(.*?)\];", src, re.S)
    reqs = " ".join(re.findall(r'"([^"]+)"', m.group(1))) if m else ""
    for needed in ("oracle", "named invariant", "killed real semantic mutant", "receipt"):
        if needed not in reqs:
            out.append(f"promotion requirements omit {needed}")
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
    """proof-row id -> {owner LEG, proof owner, gates}. docs/24 is the authority."""
    doc = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    out: dict[str, dict] = {}
    for proof_owner, gates, posture, tail, leg, body in W_OWNER_BLOCK.findall(doc):
        for line in body.splitlines():
            wid = line.strip()
            if not wid or not W_ID.match(wid):
                continue
            if wid in out:
                out[wid]["duplicate"] = True
                continue
            out[wid] = {"leg": leg, "target": leg, "tail": tail,
                        "proof_owner": proof_owner.strip(),
                        "gates": gates.strip(), "posture": posture.strip(),
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
        if "TestPak" not in row["proof_owner"] or not row["gates"]:
            out.append(f"docs/24 proof row {wid} states no future-executable proof owner or gate")
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
    src = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    m = re.search(r'id: "LEG-023", law: "([^"]+)"', src)
    out: list[str] = []
    if not m:
        return ["LEG-023 is absent from spec/legacy_obligations.rs"]
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
    src = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    m = re.search(r'id: "LEG-019", law: "([^"]+)"', src)
    out: list[str] = []
    if not m:
        return ["LEG-019 is absent from spec/legacy_obligations.rs"]
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
    rows = witness_rows(root)
    out: list[str] = []
    for leg, ids in D4B2B_ROWS.items():
        want_gates = leg_gates(root, leg)
        for wid in ids:
            row = rows.get(wid)
            if row is None:
                out.append(f"{leg} witness {wid} is absent from docs/24")
                continue
            if row["leg"] != leg:
                out.append(f"{leg} witness {wid} is bound to {row['leg']}")
            posture = row["posture"].lower()
            if "future executable: yes" not in posture:
                out.append(f"proof row {wid} states no future-executable posture")
            if "bootstrap executed: no" not in posture:
                out.append(f"proof row {wid} does not state that bootstrap did not execute it")
            if "bootstrap executed: yes" in posture:
                out.append(f"proof row {wid} falsely claims bootstrap execution")
            if "currently qualified" in posture:
                out.append(f"proof row {wid} falsely claims current executable qualification")
            # the proof row qualifies at its owning obligation's gates
            if row["gates"] != want_gates:
                out.append(
                    f"proof row {wid} gates {row['gates']!r} differ from {leg} gates {want_gates!r}"
                )
            out.extend(gate_findings(root, "proof row", wid, gate_list(
                " ".join(f"GateId::{t}" for t in row["gates"].split("/") if t))))
            if wid in D4B2B_DEFERRED:
                m = re.search(r"deferred until:\s*([^;)]*)", row["posture"], re.I)
                if not m or not m.group(1).strip():
                    out.append(f"deferred proof row {wid} states no deferral condition")
                else:
                    cond = m.group(1).strip()
                    if any(v in cond.lower() for v in D4B2B_VAGUE) or "admitted" not in cond.lower():
                        out.append(
                            f"deferred proof row {wid} names no admission boundary: {cond!r}"
                        )
            else:
                # a non-deferred future row must not inherit the adapter deferral
                if "deferred until" in posture:
                    out.append(f"proof row {wid} inherits a native-adapter deferral it does not have")
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
# The retired proof-row registry moved to its typed owner in 5.5E2:
# spec/proof.rs PROOF_ROW_MIGRATIONS. The dict that stood here was already
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
    """[(id, 'Active'|'Retired', successors)] from the typed catalog."""
    src = _uncomment((root / "spec/proof.rs").read_text(encoding="utf-8"))
    out: list[tuple[str, str, tuple[str, ...]]] = []
    for m in A_PROOF_RECORD.finditer(src):
        state = "Active" if m.group(2) == "Active" else "Retired"
        succ = tuple(re.findall(r'ProofRowId\("(\w+)"\)', m.group(3) or ""))
        out.append((m.group(1), state, succ))
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
        return ["spec/proof.rs declares no proof-identity catalog"]
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
    src = (root / A_ISA_SRC).read_text(encoding="utf-8")
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
    src = (A_ISA_SRC and (root / A_ISA_SRC).read_text(encoding="utf-8"))
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
        for row, v in zip(rows, views):
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
    arch = (root / A_FW_ARCH).read_text(encoding="utf-8")
    src = (root / A_FW_SRC).read_text(encoding="utf-8")
    planes = _a_fw_variants(arch, "SyncBatPlane", A_FW_ARCH)
    if not planes:
        raise FirewallAdmissionFailure(f"{A_FW_ARCH}: SyncBatPlane authors no plane")
    listed_planes = _a_fw_listing(arch, "SYNCBAT_PLANES", "SyncBatPlane", A_FW_ARCH)
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
            out.append(f"{A_FW_ARCH}: plane {name} is authored but SYNCBAT_PLANES omits it, "
                       f"so no law that walks the planes can see it")
    for name in v["listed_planes"]:
        if name not in planes:
            out.append(f"{A_FW_ARCH}: SYNCBAT_PLANES lists {name}, which SyncBatPlane does not author")
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


def retired_proof_row_findings(root: Path) -> list[str]:
    """A retired proof-row id returns nowhere but a marked historical note.

    Every document, not a per-phase list of the ones a rename happened to touch:
    a retired id leaking into docs/09 is the same defect as one leaking into
    docs/35, and only one of those was ever checked. The registry itself is the
    typed spec/proof.rs owner, and it must agree with the migration notes in
    BOTH directions — a note that retires an id the registry never learned is
    exactly the defect the Python dict era shipped.
    """
    out: list[str] = []
    registry = retired_proof_rows(root)
    noted = migration_note_retired_ids(root)
    for missing in sorted(noted - set(registry)):
        out.append(
            f"docs/24 retires proof-row id {missing} in a migration note that "
            "spec/proof.rs PROOF_ROWS never learned"
        )
    for phantom in sorted(set(registry) - noted):
        out.append(
            f"spec/proof.rs retires proof-row id {phantom} with no docs/24 "
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
    want_gates = leg_gates(root, D4B2C_LEG)

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
        if row["proof_owner"] != "TestPak":
            out.append(f"LEG-081 proof row {wid} names proof owner {row['proof_owner']!r}, not TestPak")
        if row["gates"] != want_gates:
            out.append(
                f"LEG-081 proof row {wid} gates {row['gates']!r} differ from typed LEG-081 gates {want_gates!r}"
            )
        out.extend(gate_findings(root, "proof row", wid, gate_list(
            " ".join(f"GateId::{t}" for t in row["gates"].split("/") if t))))
        posture = row["posture"].lower()
        if "future executable: yes" not in posture:
            out.append(f"LEG-081 proof row {wid} states no future-executable posture")
        if "bootstrap executed: yes" in posture:
            out.append(f"LEG-081 proof row {wid} falsely claims bootstrap execution")
        elif "bootstrap executed: no" not in posture:
            out.append(f"LEG-081 proof row {wid} does not state that bootstrap did not execute it")
        if wid not in meanings:
            out.append(f"LEG-081 proof row {wid} has no authoritative meaning")

    # 2. docs/35 projects the same nine and reclaims no authority
    d35 = (root / D4B2C_D35).read_text(encoding="utf-8")
    if re.search(r"Required witnesses \(proof owner", d35):
        out.append("docs/35 reclaims authoritative proof-row ownership; docs/24 owns proof-row identity")
    if W_MEANING_BLOCK.search(d35):
        out.append("docs/35 reclaims per-ID authoritative witness meaning; docs/24 owns it")
    m = D4B2C_D35_PROJECTION.search(d35)
    if not m:
        out.append("docs/35 states no LEG-081 proof-row projection labelled as projected from docs/24")
    else:
        target, owner, body = m.groups()
        if target != D4B2C_LEG:
            out.append(f"docs/35 projection targets {target}, not LEG-081")
        if "docs/24" not in owner:
            out.append(f"docs/35 projection names canonical proof-row owner {owner.strip()!r}, not docs/24")
        got: list[str] = []
        for line in body.splitlines():
            s = line.strip()
            if not s:
                continue
            if line != s or not W_ID.match(s):
                out.append(f"docs/35 projection line is not a bare proof-row id: {line!r}")
                continue
            got.append(s)
        if len(got) != len(set(got)):
            out.append("docs/35 projects a duplicate LEG-081 proof-row id")
        for missing in sorted(want - set(got)):
            out.append(f"docs/35 omits projected LEG-081 proof row {missing}")
        for extra in sorted(set(got) - want):
            owner_leg = rows.get(extra, {}).get("leg")
            if owner_leg is None:
                out.append(f"docs/35 projects unknown proof-row id {extra}")
            else:
                out.append(f"docs/35 projects {extra}, which docs/24 binds to {owner_leg}")

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


# --- 5.5D4b-3b0 generic proof-target resolution -----------------------------
# A proof row targets any existing source-qualified GuaranteeRef. The first
# resolver was LEG-shaped only because LEG rows migrated first; that was never a
# law. There is one canonical row format for every family -- no second identity,
# no per-family parser, no LEG exemption.
#
# Every family below is read through the fact parser that already owns it. This
# module adds no new parser for spec/.
GUARANTEE_FAMILIES = ("LEG", "DEC", "SEED", "ARCH", "QUAL")
G_REF = re.compile(r"(?:LEG|DEC|SEED|ARCH|QUAL)-[A-Za-z0-9][A-Za-z0-9_-]*")


def guarantee_index(root: Path) -> dict[str, dict]:
    """GuaranteeRef -> {family, gates}.

    gates is None when the family declares no machine-resolvable gate list. None
    is never silently read as "no gates": a row targeting such a family fails
    closed rather than inheriting an invented empty list.
    """
    out: dict[str, dict] = {}
    for r in guarantee_seed_rows(root):
        out[r["id"]] = {"family": "SEED", "gates": gate_render(r["gate_names"], root)}
    for lid, meta in guarantee_leg_meta(root).items():
        out[lid] = {"family": "LEG", "gates": gate_render(meta["gate_names"], root)}
    for r in decision_rows(root):
        out[r["id"]] = {"family": "DEC", "gates": gate_render(r["gate_names"], root)}
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    # ARCH and QUAL now carry typed gate postures (5.5D4b): ARCH from its declared
    # family policy, QUAL from its own row. Neither invents a list, and a
    # qualification target never stands in for one.
    pol = guarantee_family_policies(root)
    arch_rule, arch_const = pol["ARCH"]["gate_posture"]
    arch_gates = gate_render(gate_list(arch_const), root) if arch_rule == "FamilyConstant" else None
    cargo = g_package_projection(root, "cargo_name")
    for pkg, _cls, _layer in G_PKG_ROW.findall(arch):
        out[f"ARCH-{cargo.get(pkg, '')}"] = {"family": "ARCH", "gates": arch_gates}
    for pkg, profile, _target, qgates in G_QUAL_ROW.findall(arch):
        out[f"QUAL-{cargo.get(pkg, '')}-{profile}"] = {"family": "QUAL",
                                        "gates": gate_render(gate_list(qgates), root)}
    return out


# Every accepted family now resolves a typed gate posture. A family that
# cannot is a finding, never an invented empty list.
D4B3B0_UNGATED_FAMILIES: tuple[str, ...] = ()


def guarantee_gates(root: Path, ref: str) -> str | None:
    entry = guarantee_index(root).get(ref)
    return entry["gates"] if entry else None


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
        ref = row["target"]
        # one primary target per row; supporting references live in the meaning
        extra = sorted({r for r in G_REF.findall(row["tail"]) if r != ref})
        if extra:
            out.append(f"proof row {wid} names more than one primary target: {ref} and {', '.join(extra)}")
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
        # gates resolve from the typed target. Source-block gate text is never authority.
        if entry["gates"] is None:
            out.append(
                f"proof row {wid} targets {ref}, whose {entry['family']} family declares no "
                f"machine-resolvable gate list"
            )
            continue
        if row["gates"] != entry["gates"]:
            out.append(
                f"proof row {wid} gates {row['gates']!r} differ from typed {ref} gates {entry['gates']!r}"
            )
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
            ids = [l.strip() for l in m.group(1).splitlines() if l.strip()]
            if not ids or not all(D4B3B0_CAND_ID.match(i) for i in ids):
                continue
            before = [l for l in text[: m.start()].splitlines() if l.strip()]
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


def check_guarantees(root: Path, findings: list[str]) -> None:
    if not (root / "spec/guarantees.rs").is_file():
        findings.append("missing spec/guarantees.rs")
        return
    seed_rows = guarantee_seed_rows(root)
    findings.extend(guarantee_typed_relation_findings(seed_rows))
    findings.extend(guarantee_typed_witness_findings(root, seed_rows))
    nodes, edges, admission = guarantee_derive(root)
    findings.extend(admission)
    leg_meta = guarantee_leg_meta(root)
    node_ids = {n["id"] for n in nodes}
    # A witness's guarantee citation resolves against the derived node set:
    # the type carries the family, this law carries existence on the Python
    # side (seedcheck executes the same law through the sealed accessors).
    for r in seed_rows:
        for tag, ident in G_WITNESS_TAGGED.findall(r.get("witness_raw", "")):
            if tag in ("leg", "dec") and ident not in node_ids:
                findings.append(
                    f"{r['id']} witness references {ident}, which no declared row owns")
    findings.extend(gate_inventory_findings(root))
    findings.extend(gate_doc_findings(root))
    findings.extend(gate_reference_findings(root))
    findings.extend(decision_class_findings(root))
    findings.extend(authenticated_history_findings(root))
    findings.extend(retired_claim_vocabulary_findings(root))
    findings.extend(retired_proof_row_findings(root))
    findings.extend(proof_row_catalog_findings(root))
    findings.extend(contract_kind_findings(root))
    findings.extend(identity_catalog_findings(root))
    findings.extend(pakvm_isa_findings(root))
    findings.extend(recon_findings(root))
    findings.extend(release_seal_findings(root))
    findings.extend(proof_terminal_findings(root))
    findings.extend(syncbat_firewall_findings(root))
    findings.extend(guarantee_classification_findings(seed_rows))
    findings.extend(guarantee_relation_findings(node_ids, edges))
    findings.extend(guarantee_lifetime_findings(nodes, leg_meta, edges))
    if len(node_ids) != len(nodes):
        findings.append("duplicate guarantee id across source families")
    # duplicate-prose structural smell (SEED only; exact normalized text + owner, no relation)
    seed_src = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    stmts = re.findall(r'id: "([^"]+)", statement: "((?:[^"\\]|\\.)*)"', seed_src)
    seen_text: dict[tuple[str, str], str] = {}
    rel_by_id = {r["id"]: r["rel"] for r in seed_rows}
    for sid, stmt in stmts:
        owner = next((r["owner"] for r in seed_rows if r["id"] == sid), "")
        key = (re.sub(r"\s+", " ", stmt).strip(), owner)
        if key in seen_text:
            other = seen_text[key]
            rels = rel_by_id.get(sid, {})
            if not any(rels.values()):
                findings.append(f"guarantee duplicate-prose smell: {sid} and {other} share text and owner without a relation")
        seen_text[key] = sid
    # comparison: generated SEED block and Guarantee Graph must equal derived facts
    seed_doc = (root / "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")
    seed_block = batql_extract_block(seed_doc.read_text(encoding="utf-8"), "SEED-CLASSIFICATION") if seed_doc.is_file() else None
    if seed_block is None:
        findings.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: missing SEED-CLASSIFICATION block")
    else:
        block_rows = [
            tuple(c.strip() for c in ln.split("|")[1:-1])[:6]
            for ln in seed_block.splitlines() if ln.strip().startswith("| SEED-")
        ]
        derived_rows = [(r["id"], r["kind"], r["lifetime"], r["owner"],
                         gate_render(r["gate_names"], root), r["witness"]) for r in seed_rows]
        if block_rows != derived_rows:
            findings.append("docs/23 SEED-CLASSIFICATION block does not equal spec/invariants.rs")
    graph_path = root / "docs/GUARANTEE_GRAPH.generated.md"
    if not graph_path.is_file():
        findings.append("missing docs/GUARANTEE_GRAPH.generated.md")
        return
    graph = graph_path.read_text(encoding="utf-8")
    node_rows = _md_section_rows(graph, "Nodes")
    # Every row must be full width. A width change that silently drops rows would
    # make this comparison pass against an empty set.
    malformed = [r for r in node_rows if len(r) != 9]
    if malformed or not node_rows:
        findings.append(
            f"Guarantee Graph node table has {len(malformed)} malformed row(s) of {len(node_rows)}"
        )
    file_nodes = {tuple(r[:9]) for r in node_rows if len(r) == 9}
    derived_nodes = {
        (n["id"], n["family"], n["kind"], n["lifetime"], n.get("dclass", "-"), n["owner"],
         n["gate_posture"], n["target"] or "-", n["witness"])
        for n in nodes
    }
    if len(file_nodes) != len(nodes):
        findings.append(
            f"Guarantee Graph node table has {len(file_nodes)} rows, derived {len(nodes)}"
        )
    if file_nodes != derived_nodes:
        findings.append("Guarantee Graph node table does not equal the derived source facts")
    file_edges = {tuple(r) for r in _md_section_rows(graph, "Edges") if len(r) == 3}
    if file_edges != {(s, k, t) for s, k, t in edges}:
        findings.append("Guarantee Graph edge table does not equal the derived relations")


# --- Toolchain authority (5.5E3a) --------------------------------------------
# spec/toolchain.rs owns every toolchain value. This auditor independently
# reconstructs the tracked projection's bytes and refuses any hidden owner: a
# hardcoded resolver/edition/MSRV/channel literal in the materializer or a
# hardcoded rustc edition in the harness is a second authority.
A_TRIPLE_SHAPE = re.compile(r"[a-z0-9_]+(-[a-z0-9_]+){2,}")


A_TC_PROFILE_SPELLING = {"Minimal": "minimal"}
A_TC_COMPONENT_SPELLING = {"Clippy": "clippy", "Rustfmt": "rustfmt"}


def toolchain_profile(root: Path) -> dict:
    src = _uncomment((root / "spec/toolchain.rs").read_text(encoding="utf-8"))
    release = re.search(
        r"exact_rust_release: RustRelease \{ major: (\d+), minor: (\d+), patch: (\d+) \}", src)
    floor = re.search(
        r"rust_version_floor: RustVersionFloor \{ major: (\d+), minor: (\d+) \}", src)
    edition = re.search(r"edition: RustEdition::Rust(\d+)", src)
    resolver = re.search(r"cargo_resolver: CargoResolver::V(\d+)", src)
    profile = re.search(r"rustup_profile: RustupProfile::(\w+)", src)
    comps = re.search(
        r"pub const ALL: &'static \[RustupComponent\] =\s*&\[([^\]]*)\]", src)
    enum_body = re.search(r"pub enum RustupComponent \{(.*?)\n\}", src, re.S)
    # The semantic environments live on the typed enum in architecture.rs;
    # the auditor reads the authored spelling arms, never a copied list.
    arch = _uncomment((root / "spec/architecture.rs").read_text(encoding="utf-8"))
    spellings = re.findall(
        r'QualificationEnvironment::(\w+) => "([^"]+)",', arch)
    return {
        "exact_parts": tuple(int(x) for x in release.groups()) if release else None,
        "floor_parts": tuple(int(x) for x in floor.groups()) if floor else None,
        "exact": ".".join(release.groups()) if release else "",
        "floor": ".".join(floor.groups()) if floor else "",
        "edition": edition.group(1) if edition else "",
        "resolver": resolver.group(1) if resolver else "",
        "profile": A_TC_PROFILE_SPELLING.get(profile.group(1), "") if profile else "",
        "components": [A_TC_COMPONENT_SPELLING.get(v, "")
                       for v in re.findall(r"RustupComponent::(\w+)", comps.group(1))]
                      if comps else [],
        "component_all": re.findall(r"RustupComponent::(\w+)", comps.group(1))
                         if comps else [],
        "component_variants": re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M)
                              if enum_body else [],
        "environment_spellings": dict(spellings),
    }


def toolchain_findings(root: Path) -> list[str]:
    t = toolchain_profile(root)
    out: list[str] = []
    for key in ("exact", "floor", "edition", "resolver", "profile"):
        if not t[key]:
            out.append(f"spec/toolchain.rs authors no toolchain {key}")
    if not t["components"]:
        out.append("spec/toolchain.rs authors no required components")
    # RustupComponent::ALL is the ONE component denominator: every declared
    # variant appears in it exactly once. Dropping a component from ALL while
    # the projection moves in lockstep would leave a declared-but-unconsumed
    # variant — the TestFixture defect wearing a toolbelt.
    for missing in sorted(set(t["component_variants"]) - set(t["component_all"])):
        out.append(f"declared toolchain component {missing} is omitted from "
                   "RustupComponent::ALL, the one component inventory")
    for phantom in sorted(set(t["component_all"]) - set(t["component_variants"])):
        out.append(f"RustupComponent::ALL names {phantom}, which the enum "
                   "does not declare")
    if len(t["component_all"]) != len(set(t["component_all"])):
        out.append("RustupComponent::ALL repeats a component")
    if not t["environment_spellings"]:
        out.append("spec/architecture.rs declares no qualification environments")
    # exact >= floor, never "same minor": a newer qualifying compiler that
    # preserves the MSRV floor is ordinary; one below the floor would qualify
    # a foundation its own consumers may lawfully refuse.
    if t["exact_parts"] and t["floor_parts"] and t["exact_parts"][:2] < t["floor_parts"]:
        out.append(
            f"exact_rust_release {t['exact']} is below the declared MSRV "
            f"floor {t['floor']}; the qualifying compiler must satisfy the "
            "floor the generated workspace claims")
    want = ('[toolchain]\nchannel = "{}"\nprofile = "{}"\ncomponents = [{}]\n'
            .format(t["exact"], t["profile"],
                    ", ".join(f'"{c}"' for c in t["components"])))
    tracked = root / "rust-toolchain.toml"
    if not tracked.is_file():
        out.append("tracked rust-toolchain.toml is absent; the toolchain "
                   "projection selects the compiler before the spec compiles")
    elif tracked.read_text(encoding="utf-8") != want:
        out.append("tracked rust-toolchain.toml does not equal the "
                   "deterministic projection of ToolchainProfile "
                   "(owner: spec/toolchain.rs; repair: regenerate, never hand-edit)")
    mat = (root / "bootstrap/materialize.rs").read_text(encoding="utf-8")
    for pattern, what in ((r'resolver = \\"[0-9]', "resolver"),
                          (r'edition = \\"[0-9]', "edition"),
                          (r'rust-version = \\"[0-9]', "rust-version"),
                          (r'channel = \\"[0-9]', "channel")):
        if re.search(pattern, mat):
            out.append(f"bootstrap/materialize.rs hardcodes a {what} literal; "
                       "spec/toolchain.rs ToolchainProfile is the owner")
    harness = (root / "bootstrap/selftest.py").read_text(encoding="utf-8")
    if re.search(r'"--edition", "[0-9]', harness):
        out.append("bootstrap/selftest.py hardcodes a rustc edition; "
                   "spec/toolchain.rs ToolchainProfile is the owner")
    # Environment membership dissolved into construction (5.5E3a1): a QUAL
    # row carries a closed QualificationEnvironment variant, so the auditor
    # checks the authored SPELLINGS and that rows name declared variants.
    for variant, spelling in t["environment_spellings"].items():
        if A_TRIPLE_SHAPE.fullmatch(spelling):
            out.append(f"QualificationEnvironment::{variant} spells {spelling!r}, "
                       "which is shaped like a physical target triple; "
                       "environments answer WHAT holds, never WHERE a binary ran")
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    for pkg, profile, variant, _gates in G_QUAL_ROW.findall(arch):
        if t["environment_spellings"] and variant not in t["environment_spellings"]:
            out.append(f"QualificationProfile {pkg}:{profile} names environment "
                       f"{variant}, which the QualificationEnvironment enum "
                       "does not declare")
    return out


# --- Admitted contract kinds (5.5E3c) ----------------------------------------
# spec/contracts.rs owns the admitted set; docs/06 projects it. A kind whose
# admitting citation dangles, and a doc fence that admits more or fewer kinds
# than the enum, are both refused.
A_CONTRACT_KIND_FENCE = re.compile(
    r"The admitted kinds, each with its admitting law:\s*\n+```text\n(.*?)\n```", re.S)


def contract_kind_findings(root: Path) -> list[str]:
    out: list[str] = []
    src_path = root / "spec/contracts.rs"
    if not src_path.is_file():
        return ["missing spec/contracts.rs"]
    src = _uncomment(src_path.read_text(encoding="utf-8"))
    enum_body = re.search(r"pub enum ContractKind \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(r"pub const ALL: &'static \[ContractKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"ContractKind::(\w+)", all_body.group(1)) if all_body else []
    if not variants:
        out.append("spec/contracts.rs declares no ContractKind variants")
    for missing in sorted(set(variants) - set(inventory)):
        out.append(f"ContractKind::{missing} is omitted from ContractKind::ALL")
    for phantom in sorted(set(inventory) - set(variants)):
        out.append(f"ContractKind::ALL names {phantom}, which the enum does not declare")
    # tag/prefix agreement and resolution against the declared tables
    tagged = re.findall(r'ContractKind::(\w+) => GuaranteeRef::(leg|dec)\("([^"]+)"\)', src)
    leg_ids = {m[0] for m in G_LEG_ROW.findall(
        (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8"))}
    dec_ids = {m[0] for m in G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8"))}
    for kind, tag, ident in tagged:
        if tag == "leg" and not ident.startswith("LEG-"):
            out.append(f"ContractKind::{kind} tags {ident!r} as leg, whose family "
                       "owns the LEG- prefix")
        elif tag == "dec" and not ident.startswith("DEC-"):
            out.append(f"ContractKind::{kind} tags {ident!r} as dec, whose family "
                       "owns the DEC- prefix")
        elif ident not in (leg_ids | dec_ids):
            out.append(f"ContractKind::{kind} cites admission basis {ident}, "
                       "which no declared row owns")
    for kind in set(variants) - {k for k, _, _ in tagged}:
        out.append(f"ContractKind::{kind} cites no admission basis")
    # docs/06 projects EXACTLY the ordered (kind, admission-basis) pairs, in
    # ContractKind::ALL order. The first two tokens of each fence line are
    # semantic identity the typed owner projects; the explanatory tail is
    # authored prose and carries none. Comparing sets of kind names alone
    # would let the doc confidently explain the wrong provenance.
    spelling_of = dict(re.findall(r'ContractKind::(\w+) => "(\w+)",', src))
    basis_of = {k: ident for k, _tag, ident in tagged}
    expected = [(spelling_of.get(v, ""), basis_of.get(v, "")) for v in inventory]
    doc = (root / "docs/06_MACBAT.md").read_text(encoding="utf-8")
    fence = A_CONTRACT_KIND_FENCE.search(doc)
    if not fence:
        out.append("docs/06 carries no admitted-kinds fence")
        return out
    listed = [tuple(line.split()[:2]) for line in fence.group(1).splitlines()
              if line.strip()]
    for i, want in enumerate(expected):
        if i >= len(listed):
            out.append(f"docs/06 omits admitted contract kind row {want[0]} {want[1]}")
        elif listed[i] != want:
            out.append(
                f"docs/06 row {i + 1} states {listed[i][0]} {listed[i][1] if len(listed[i]) > 1 else '?'}; "
                f"the typed catalog states {want[0]} {want[1]} at that position")
    for extra in listed[len(expected):]:
        out.append(f"docs/06 admits {extra[0]}, which ContractKind does not declare")
    return out


# --- Identity catalogs (5.5E3d) ----------------------------------------------
# spec/identities.rs owns four catalog axes plus the non-cataloged residue.
# This auditor independently parses the typed owner, reconstructs the five
# generated docs/16 blocks (ordered entries AND owners), and runs the standing
# corpus-sweep law: every identity-shaped term discovered in the AUTHORED
# corpus resolves through exactly one of five classification paths. Discovery
# deliberately excludes generated projection blocks and spec/identities.rs
# itself — the denominator must never prove itself by rereading its own
# answer sheet.
A_IDENTITY_AXES = (
    ("IDENTITY-CATALOG", "IdentityKind"),
    ("GENERATION-CATALOG", "GenerationKind"),
    ("BINDING-CATALOG", "BindingKind"),
    ("VERSION-CATALOG", "VersionIdentityKind"),
)
A_IDENT_DISPOSITION = {
    "OwnedElsewhere": "owned elsewhere",
    "NotYetAdmittedBy": "not yet admitted by",
}
# Exact case-sensitive identifier tokens ending in one of the six admitted
# suffixes, with token boundaries: "Valid", "provide", and prose containing
# lowercase "id" mint no surprise passports.
A_IDENT_TERM = re.compile(
    r"\b[A-Z][A-Za-z0-9]*(?:Id|Hash|Digest|Commitment|Generation|Version)\b")
A_GENERATED_BLOCK = re.compile(
    r"<!-- [A-Z0-9-]+:BEGIN[^>]*-->.*?<!-- [A-Z0-9-]+:END -->", re.S)


def _identity_block_body(name: str, doc: str):
    # The BEGIN marker is provenance, verified independently of the renderer
    # (5.5E3d1): it must name spec/identities.rs as the source authority
    # exactly, or the block is not a generated identity block at all.
    m = re.search(
        r"<!-- " + re.escape(name)
        + r":BEGIN generated from spec/identities\.rs by bootstrap/project\.py; "
        + r"do not edit -->\n```text\n(.*?)\n```\n<!-- "
        + re.escape(name) + r":END -->", doc, re.S)
    return m.group(1) if m else None


def identity_catalog_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/identities.rs"
    if not path.is_file():
        return ["missing spec/identities.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    dec_disposition = {m[0]: m[3] for m in G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8"))}
    # Spellings with existing typed spec owners are referenced, never
    # re-admitted: a duplicate variant is a wrapper passport.
    owned_body = re.search(
        r"pub const EXISTING_TYPED_OWNER_SPELLINGS: &\[&str\] = &\[(.*?)\];", src, re.S)
    existing_owned = set(re.findall(r'"(\w+)"', owned_body.group(1))) if owned_body else set()
    if not existing_owned:
        out.append("spec/identities.rs declares no EXISTING_TYPED_OWNER_SPELLINGS list")
    # Independent parse of the four axes, in Kind::ALL order. The \b anchor
    # keeps "VersionIdentityKind::..." from satisfying "IdentityKind::...".
    catalogs: dict[str, list[tuple[str, str]]] = {}
    axis_of: dict[str, str] = {}
    for marker, kind in A_IDENTITY_AXES:
        all_body = re.search(
            r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        arms = {v: (s, o) for v, s, o in re.findall(
            r"\b" + kind + r"::(\w+) => \{?\s*entry!\(\s*\"([^\"]+)\",\s*\"([^\"]+)\"\s*\)",
            src)}
        if not inventory:
            out.append(f"spec/identities.rs declares no {kind} inventory")
            continue
        entries: list[tuple[str, str]] = []
        for v in inventory:
            if v not in arms:
                out.append(f"{kind}::{v} is missing from ALL or declares no catalog entry")
                continue
            spelling, owner = arms[v]
            entries.append((spelling, owner))
            if spelling in axis_of:
                out.append(
                    f"identity spelling {spelling} answers two questions: it appears "
                    f"in both {axis_of[spelling]} and {kind}")
            else:
                axis_of[spelling] = kind
            if owner not in contract_ids:
                out.append(f"{kind} entry {spelling} names owner {owner}, "
                           "which no declared contract owns")
            if spelling in existing_owned:
                out.append(f"{kind} entry {spelling} duplicates a spelling that "
                           "already has a typed spec owner; the catalog may "
                           "reference it but never re-admit it")
        for phantom in sorted(set(arms) - set(inventory)):
            out.append(f"{kind}::{phantom} declares an entry but is omitted from {kind}::ALL")
        catalogs[marker] = entries
    # Chronology/order/topology/coordinate/cursor/navigation vocabulary is
    # excluded by executed law, not convention.
    excluded_body = re.search(
        r"pub const EXCLUDED_CHRONOLOGY_AND_NAVIGATION: &\[&str\] = &\[(.*?)\];",
        src, re.S)
    excluded = set(re.findall(r'"(\w+)"', excluded_body.group(1))) if excluded_body else set()
    if not excluded:
        out.append("spec/identities.rs declares no EXCLUDED_CHRONOLOGY_AND_NAVIGATION list")
    for term in sorted(excluded & set(axis_of)):
        out.append(f"{term} is chronology/navigation vocabulary and may not enter "
                   f"the {axis_of[term]} catalog")
    # The residue table: the denominator's non-admitted half.
    residue_body = re.search(
        r"pub const NON_CATALOGED_IDENTITY_TERMS: &\[NonCatalogedIdentityTerm\] = &\[(.*?)\n\];",
        src, re.S)
    residue = re.findall(
        r'term: "(\w+)",\s*disposition: IdentityTermDisposition::(\w+)\('
        r'(?:ContractId|DecisionId)\("([^"]+)"\)\)',
        residue_body.group(1)) if residue_body else []
    if not residue:
        out.append("spec/identities.rs declares no NON_CATALOGED_IDENTITY_TERMS rows")
    residue_seen: set[str] = set()
    for term, variant, ref in residue:
        if term in residue_seen:
            out.append(f"residue term {term} is declared twice; one term, one path "
                       "also means one row on that path")
        residue_seen.add(term)
        if term in axis_of:
            out.append(f"{term} resolves through two paths: the residue table and "
                       f"the {axis_of[term]} catalog")
        if variant == "OwnedElsewhere":
            if ref not in contract_ids:
                out.append(f"residue term {term} is owned elsewhere by {ref}, "
                           "which no declared contract owns")
        elif variant == "NotYetAdmittedBy":
            disp = dec_disposition.get(ref)
            if disp is None:
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           "which no declared decision owns")
            elif disp in ("Supersede", "RetainAsEvidence"):
                # Mirrors Disposition::guarantee_lifetime: these two derive
                # HistoricalCoverageOnly, and a decision retained only as
                # historical coverage has no forward policy authority to own
                # a future admission barrier.
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           "which is retained only as historical coverage and "
                           "cannot own a future admission barrier")
            elif disp not in ("Lock", "Defer"):
                # Mirrors Disposition::may_own_not_yet_admitted_identity:
                # Permanent is not enough — Kill is a permanent PROHIBITION
                # (a dead passport), Keep is admitted retained policy, and
                # neither expresses "excluded now, with a standing future
                # entry path". Only Lock and Defer do.
                out.append(f"residue term {term} is not yet admitted by {ref}, "
                           f"whose {disp} disposition is not a standing "
                           "future-entry policy; only Lock and Defer own "
                           "pending applications")
        else:
            out.append(f"residue term {term} carries unknown disposition {variant}")
    # docs/16 carries five generated blocks projecting ordered entries AND
    # owners; this auditor reconstructs each body independently.
    doc = (root / "docs/16_IDENTITY_TIME_AND_NAVIGATION.md").read_text(encoding="utf-8")
    for marker, _kind in A_IDENTITY_AXES:
        want = [f"{s:<30} {o}" for s, o in catalogs.get(marker, [])]
        _identity_block_parity(marker, want, doc, out)
    residue_want = [f"{t:<30} {A_IDENT_DISPOSITION.get(v, '?'):<22} {r}"
                    for t, v, r in residue]
    _identity_block_parity("IDENTITY-RESIDUE", residue_want, doc, out)
    # The standing corpus-sweep law, TRUE set equality (5.5E3d1). Authored
    # corpus: the authoritative docs and companion, minus every generated
    # projection block. Three refusal shapes: discovered but unclassified;
    # classified residue with no authored occurrence; classified catalog
    # entry with no authored adopter. The adopter direction uses exact
    # token presence rather than the sweep grammar because a spelling like
    # bare "Commitment" is lawful catalog vocabulary the prefix+suffix
    # grammar deliberately does not mint from prose.
    authored: list[str] = []
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        authored.append(A_GENERATED_BLOCK.sub("", text))
    blob = "\n".join(authored)
    discovered = set(A_IDENT_TERM.findall(blob))
    classified = set(axis_of) | {t for t, _v, _r in residue}
    for term in sorted(discovered - classified):
        out.append(f"identity-shaped term {term} appears in the authoritative corpus "
                   "and resolves through none of the five classification paths")
    for term in sorted(set(axis_of)):
        if not re.search(r"\b" + re.escape(term) + r"\b", blob):
            out.append(f"catalog entry {term} appears in no authored corpus document; "
                       "an entry without an authored adopter is a brochure passport")
    for term in sorted({t for t, _v, _r in residue}):
        if not re.search(r"\b" + re.escape(term) + r"\b", blob):
            out.append(f"residue term {term} no longer appears in the authored corpus; "
                       "a disposition for a term nobody authors is an expired application")
    # Self-neutering guards (5.5E3d1): the exclusion list must equal the
    # vocabulary the authored docs/16 navigation and time-and-order fences
    # declare, and the wrapper guard must equal the identity types the spec
    # actually declares elsewhere. Deleting a guard row before smuggling the
    # term in is refused by parity, not by the guard it deleted.
    doc16_authored = A_GENERATED_BLOCK.sub(
        "", (root / "docs/16_IDENTITY_TIME_AND_NAVIGATION.md").read_text(encoding="utf-8"))
    required_excluded: set[str] = set()
    for heading in ("Navigation types", "Time and order"):
        fence = re.search(r"## " + heading + r"\n+```text\n(.*?)\n```", doc16_authored, re.S)
        if not fence:
            out.append(f"docs/16 carries no authored {heading} fence to derive "
                       "the chronology/navigation exclusion from")
            continue
        required_excluded.update(
            line.split()[0] for line in fence.group(1).splitlines() if line.strip())
    for term in sorted(required_excluded - excluded):
        out.append(f"EXCLUDED_CHRONOLOGY_AND_NAVIGATION omits {term}, which docs/16's "
                   "navigation and time-and-order fences declare")
    for term in sorted(excluded - required_excluded):
        out.append(f"EXCLUDED_CHRONOLOGY_AND_NAVIGATION lists {term}, which docs/16's "
                   "navigation and time-and-order fences do not declare")
    required_owned: set[str] = set()
    for rel in ("spec/architecture.rs", "spec/proof.rs", "spec/gates.rs"):
        required_owned.update(re.findall(
            r"pub (?:struct|enum) (\w+Id)\b",
            _uncomment((root / rel).read_text(encoding="utf-8"))))
    for term in sorted(required_owned - existing_owned):
        out.append(f"EXISTING_TYPED_OWNER_SPELLINGS omits {term}, which the spec "
                   "declares as an existing typed owner")
    for term in sorted(existing_owned - required_owned):
        out.append(f"EXISTING_TYPED_OWNER_SPELLINGS lists {term}, which no spec "
                   "module declares as an existing typed owner")
    return out


def _identity_block_parity(marker: str, want: list[str], doc: str,
                           out: list[str]) -> None:
    body = _identity_block_body(marker, doc)
    if body is None:
        out.append(f"docs/16 carries no generated {marker} block naming "
                   "spec/identities.rs as source")
        return
    listed = [line.rstrip() for line in body.splitlines() if line.strip()]
    want = [line.rstrip() for line in want]
    for i, expect in enumerate(want):
        if i >= len(listed):
            out.append(f"docs/16 {marker} omits row {expect.split()[0]}")
        elif listed[i] != expect:
            out.append(f"docs/16 {marker} row {i + 1} states {' '.join(listed[i].split())}; "
                       f"the typed catalog states {' '.join(expect.split())} at that position")
    for extra in listed[len(want):]:
        out.append(f"docs/16 {marker} lists {extra.split()[0]}, "
                   "which the typed catalog does not project")


def check_architecture(root: Path, findings: list[str]) -> None:
    path = root / "spec/architecture.rs"
    if not path.is_file():
        findings.append("missing spec/architecture.rs")
        return
    findings.extend(toolchain_findings(root))
    source = path.read_text(encoding="utf-8")
    # The variant is the identity (5.5E3b); cargo names and workspace paths
    # are projections this auditor reconstructs from the owning const fns.
    cargo = g_package_projection(root, "cargo_name")
    wpaths = g_package_projection(root, "workspace_path")
    enum_body = re.search(r"pub enum PackageId \{(.*?)\n\}", _uncomment(source), re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];", source, re.S)
    inventory = re.findall(r"PackageId::(\w+)", all_body.group(1)) if all_body else []
    for missing in sorted(set(variants) - set(inventory)):
        findings.append(f"spec/architecture.rs: PackageId::{missing} is omitted "
                        "from PackageId::ALL, the one package inventory")
    for phantom in sorted(set(inventory) - set(variants)):
        findings.append(f"spec/architecture.rs: PackageId::ALL names {phantom}, "
                        "which the enum does not declare")
    package_rows = re.findall(
        r"PackageSpec\s*\{\s*id:\s*PackageId::(\w+),\s*role:\s*\"([^\"]+)\",\s*class:\s*PackageClass::([A-Za-z]+),\s*layer:\s*([0-9]+),\s*\}",
        source,
        re.S,
    )
    if not package_rows:
        findings.append("spec/architecture.rs: no package rows parsed")
        return
    if [row[0] for row in package_rows] != inventory:
        findings.append("spec/architecture.rs: PACKAGES does not declare exactly "
                        "PackageId::ALL in canonical order")
    packages = [cargo.get(row[0], "") for row in package_rows]
    paths = [wpaths.get(row[0], "") for row in package_rows]
    layers = {cargo.get(row[0], ""): int(row[3]) for row in package_rows}
    if len(packages) != len(set(packages)):
        findings.append("spec/architecture.rs: two package identities project the same cargo name")
    if len(paths) != len(set(paths)):
        findings.append("spec/architecture.rs: two package identities project the same workspace path")
    for variant, role, package_class, layer in package_rows:
        wp = wpaths.get(variant, "")
        if (not cargo.get(variant) or not wp or wp.startswith("/") or ".." in wp.split("/")
                or not role or package_class not in {"Production", "BinaryAdapter", "DevOnly", "Example"}
                or not layer.isdigit()):
            findings.append(f"spec/architecture.rs: incomplete package {variant}")

    edge_rows = re.findall(
        r"EdgeSpec\s*\{\s*importer:\s*PackageId::(\w+),\s*importee:\s*PackageId::(\w+),\s*class:\s*EdgeClass::([A-Za-z]+),\s*profile:\s*\"([^\"]+)\"\s*\}",
        source,
        re.S,
    )
    edge_rows = [(cargo.get(a, a), cargo.get(b, b), c, p) for a, b, c, p in edge_rows]
    graph: dict[str, list[str]] = {package: [] for package in packages}
    package_set = set(packages)
    for importer, importee, edge_class, profile in edge_rows:
        if importer not in package_set or importee not in package_set:
            findings.append(f"spec/architecture.rs: unknown package in edge {importer} -> {importee}")
            continue
        if importer == importee:
            findings.append(f"spec/architecture.rs: self edge {importer}")
        if edge_class not in {"Required", "OptionalProfile", "DevOnly"} or not profile:
            findings.append(f"spec/architecture.rs: incomplete edge {importer} -> {importee}")
        if importer in layers and importee in layers and layers[importer] <= layers[importee]:
            findings.append(f"spec/architecture.rs: dependency direction violation {importer}(L{layers[importer]}) -> {importee}(L{layers[importee]})")
        graph[importer].append(importee)

    visiting: set[str] = set()
    visited: set[str] = set()

    def cycle(node: str) -> bool:
        if node in visited:
            return False
        if node in visiting:
            return True
        visiting.add(node)
        for child in graph.get(node, []):
            if cycle(child):
                return True
        visiting.remove(node)
        visited.add(node)
        return False

    for package in packages:
        visiting.clear()
        if cycle(package):
            findings.append(f"spec/architecture.rs: dependency cycle reaches {package}")
            break

    profile_rows = re.findall(
        r"QualificationProfile\s*\{\s*package:\s*PackageId::(\w+),\s*profile:\s*\"([^\"]+)\","
        r"\s*environment:\s*QualificationEnvironment::(\w+),"
        r"\s*gates:\s*&\[([^\]]*)\],\s*requirement:\s*\"([^\"]+)\",\s*\}",
        source,
        re.S,
    )
    profile_rows = [(cargo.get(v, v), *rest) for v, *rest in profile_rows]
    identities: set[tuple[str, str]] = set()
    for package, profile, environment, gates, requirement in profile_rows:
        identity = (package, profile)
        if package not in package_set or not profile or not environment or not requirement or not gates:
            findings.append(f"spec/architecture.rs: incomplete qualification {package}:{profile}")
        if identity in identities:
            findings.append(f"spec/architecture.rs: duplicate qualification {package}:{profile}")
        identities.add(identity)
    for package in ("batpak", "syncbat"):
        if (package, "semantic") not in identities or not any(
            row[0] == package and row[1] == "semantic" and row[2] == "NoStdAlloc" for row in profile_rows
        ):
            findings.append(f"spec/architecture.rs: missing no_std + alloc semantic profile for {package}")

    # The materializer selects behavior by PackageId, never by raw name: a
    # cargo-name string comparison is identity laundering (5.5E3b).
    mat_path = root / "bootstrap/materialize.rs"
    if mat_path.is_file():
        mat = mat_path.read_text(encoding="utf-8")
        for name in sorted(set(cargo.values())):
            if re.search(r'== "' + re.escape(name) + '"', mat):
                findings.append(
                    f"bootstrap/materialize.rs selects behavior by raw package "
                    f"name {name!r}; PackageId is the identity")

    for relative in string_array(source, "REQUIRED_DOCS"):
        if not (root / relative).is_file():
            findings.append(f"spec/architecture.rs: required file missing {relative}")
    for relative in string_array(source, "FORBIDDEN_TARGET_PATHS"):
        if (root / relative).exists():
            findings.append(f"spec/architecture.rs: forbidden path exists {relative}")
    syncbat_root = root / "crates/syncbat"
    if syncbat_root.exists():
        for relative in string_array(source, "SYNCBAT_REQUIRED_PLANES"):
            if not (syncbat_root / relative).exists():
                findings.append(f"syncbat source missing declared plane {relative}")


def check_seed_ids(root: Path, findings: list[str]) -> None:
    sources = {
        "invariant": (root / "spec/invariants.rs", r'id:\s*"(SEED-[A-Z0-9-]+)"'),
        "decision": (root / "spec/dispositions.rs", r'id:\s*"(DEC-[0-9]+)"'),
        "legacy": (root / "spec/legacy_obligations.rs", r'id:\s*"(LEG-[0-9]+)"'),
        "legacy_coverage": (root / "spec/legacy_invariant_coverage.rs", r'legacy_id:\s*"(INV-[A-Z0-9-]+)"'),
    }
    parsed: dict[str, list[str]] = {}
    for kind, (path, pattern) in sources.items():
        if not path.is_file():
            findings.append(f"missing {path.relative_to(root)}")
            continue
        values = re.findall(pattern, path.read_text(encoding="utf-8"))
        if len(values) != len(set(values)):
            findings.append(f"duplicate {kind} ID in {path.relative_to(root)}")
        parsed[kind] = values

    decision_doc = (root / "docs/30_DECISION_AND_REJECTION_LEDGER.md").read_text(encoding="utf-8")
    decision_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in decision_doc.splitlines():
        if not line.startswith("| DEC-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 6:
            findings.append(f"decision ledger malformed row: {line}")
            continue
        ident, tag, dclass, dgates, subject, successor = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        decision_doc_rows[ident] = (tag, clean(subject), clean(successor), dclass, dgates)
    disposition_tags = {
        "Keep": "KEEP",
        "Lock": "LOCK",
        "Kill": "KILL",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Defer": "DEFER",
        "OpenImplementation": "OPEN-IMPLEMENTATION",
        "RetainAsEvidence": "RETAIN-AS-EVIDENCE",
    }
    decision_source = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    decision_seed_rows = {
        ident: (disposition_tags[variant], subject, successor, dclass,
                gate_render(gate_list(dgates), root))
        for ident, dclass, dgates, variant, subject, successor in re.findall(
            r'DecisionSpec \{ id: "([^"]+)", class: DecisionClass::(\w+), gates: &\[([^\]]*)\], '
            r'disposition: Disposition::([A-Za-z]+), subject: "([^"]+)", successor: "([^"]+)"',
            decision_source,
        )
    }
    for value in parsed.get("decision", []):
        if value not in decision_doc_rows:
            findings.append(f"decision seed {value} absent from docs/30_DECISION_AND_REJECTION_LEDGER.md")
        elif decision_seed_rows.get(value) != decision_doc_rows[value]:
            findings.append(f"decision seed/document mismatch for {value}")
    legacy_doc = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    legacy_doc_rows: dict[str, tuple[str, str, str, str, str, str]] = {}
    for line in legacy_doc.splitlines():
        if not line.startswith("| LEG-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 10:
            findings.append(f"legacy ledger malformed row: {line}")
            continue
        ident, law, _evidence, owner, _mechanism, _witness, gate, compat, deletion, status = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        legacy_doc_rows[ident] = (
            clean(law), clean(owner), clean(gate), clean(compat), clean(deletion), clean(status)
        )
    legacy_source = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    legacy_seed_rows = {
        ident: (law, owner, gate_render(gate_list(gates), root), compat, deletion, status)
        for ident, law, owner, gates, compat, deletion, status in re.findall(
            r'LegacyObligation \{ id: "([^"]+)", law: "([^"]+)", clean_owner: "([^"]+)", '
            r'gates: &\[([^\]]*)\], '
            r"compatibility_disposition: CompatibilityDisposition::(\w+), "
            r"deletion_condition: DeletionCondition::(\w+), "
            r"active_or_closed_status: ObligationStatus::(\w+) \}",
            legacy_source,
        )
    }
    for value in parsed.get("legacy", []):
        if value not in legacy_doc_rows:
            findings.append(f"legacy seed {value} absent from docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md")
        elif legacy_seed_rows.get(value) != legacy_doc_rows[value]:
            findings.append(f"legacy seed/document mismatch for {value}")

    coverage_doc = (root / "docs/34_LEGACY_INVARIANT_COVERAGE.md").read_text(encoding="utf-8")
    coverage_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in coverage_doc.splitlines():
        if not line.startswith("| `INV-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 4:
            findings.append(f"legacy invariant coverage malformed row: {line}")
            continue
        ident, tag, successor, rationale = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        coverage_doc_rows[clean(ident)] = (tag, clean(successor), rationale)
    coverage_tags = {
        "Preserve": "PRESERVE",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Kill": "KILL",
        "Requalify": "REQUALIFY",
    }
    coverage_source = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    coverage_seed_rows = {
        ident: (coverage_tags[variant], successor, rationale)
        for ident, variant, successor, rationale in re.findall(
            r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", disposition: CoverageDisposition::([A-Za-z]+), successor: "([^"]+)", rationale: "([^"]+)" \}',
            coverage_source,
        )
    }
    expected_count_match = re.search(r"EXPECTED_COVERAGE_ROWS:\s*usize\s*=\s*([0-9]+)", coverage_source)
    expected_count = int(expected_count_match.group(1)) if expected_count_match else -1
    if expected_count != len(parsed.get("legacy_coverage", [])):
        findings.append(f"legacy invariant coverage count is {len(parsed.get('legacy_coverage', []))}, expected {expected_count}")
    source_commit = re.search(r'LEGACY_SOURCE_COMMIT:\s*&str\s*=\s*"([0-9a-f]+)"', coverage_source)
    if source_commit is None or len(source_commit.group(1)) != 40:
        findings.append("legacy invariant coverage source commit is missing or not a full SHA")
    manifest_ids = string_array(coverage_source, "SOURCE_INVARIANT_IDS")
    if not manifest_ids:
        findings.append("spec/legacy_invariant_coverage.rs: missing SOURCE_INVARIANT_IDS manifest")
    else:
        findings.extend(
            legacy_manifest_findings(manifest_ids, parsed.get("legacy_coverage", []), expected_count)
        )
    for value in parsed.get("legacy_coverage", []):
        if value not in coverage_doc_rows:
            findings.append(f"legacy invariant coverage seed {value} absent from docs/34_LEGACY_INVARIANT_COVERAGE.md")
        elif coverage_seed_rows.get(value) != coverage_doc_rows[value]:
            findings.append(f"legacy invariant coverage seed/document mismatch for {value}")


def parse_stale_vocabulary(root: Path, findings: list[str]) -> dict[str, tuple[str, frozenset[str], str]]:
    """Derive the stale-alias matcher from spec/dispositions.rs.

    Returns alias_lower -> (owning decision id, allowed contexts, canonical alias).
    The matcher is a consequence of the decisions that retired vocabulary; there
    is no separately authored stale-term list.
    """
    path = root / "spec/dispositions.rs"
    if not path.is_file():
        findings.append("missing spec/dispositions.rs for stale-vocabulary derivation")
        return {}
    source = path.read_text(encoding="utf-8")
    # The typed enum declares EXACTLY the vocabulary this scanner speaks:
    # the five permissive contexts plus the two never-permissive defaults.
    enum_body = re.search(r"pub enum StaleContext \{(.*?)\n\}", _uncomment(source), re.S)
    declared = set(re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M)) if enum_body else set()
    want = STALE_PERMISSIVE_CONTEXTS | STALE_DEFAULT_CONTEXTS
    if declared != want:
        findings.append(
            f"spec/dispositions.rs StaleContext variants {sorted(declared)} != "
            f"the scanner's closed vocabulary {sorted(want)}")
    if not (set(STALE_CONTEXT_BY_PATH.values()) <= STALE_PERMISSIVE_CONTEXTS):
        findings.append("audit's stale-context path map names an undeclared or non-permissive context")
    alias_map: dict[str, tuple[str, frozenset[str], str]] = {}
    row = re.compile(
        r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::\w+, gates: &\[[^\]]*\], '
        r"disposition: Disposition::(\w+),.*?"
        r"stale_aliases: &\[([^\]]*)\], stale_allowed_contexts: &\[([^\]]*)\],"
    )
    for ident, disposition, aliases_raw, contexts_raw in row.findall(source):
        aliases = re.findall(r'"([^"]+)"', aliases_raw)
        contexts = frozenset(re.findall(r"StaleContext::(\w+)", contexts_raw))
        if aliases and disposition not in RETIRING_DISPOSITIONS:
            findings.append(f"spec/dispositions.rs: {ident} carries stale aliases but disposition {disposition} does not retire vocabulary")
        if aliases and not contexts:
            findings.append(f"spec/dispositions.rs: {ident} has stale aliases but no allowed contexts")
        for ctx in sorted(contexts & STALE_DEFAULT_CONTEXTS):
            findings.append(
                f"spec/dispositions.rs: {ident} allow-lists the default context {ctx}; "
                "production source and ordinary authoritative material always require "
                "an inline STALE-REF")
        for alias in aliases:
            key = alias.lower()
            if key in alias_map:
                findings.append(f"spec/dispositions.rs: stale alias {alias!r} claimed by {alias_map[key][0]} and {ident}")
            alias_map[key] = (ident, contexts, alias)
    return alias_map


def stale_context_for(rel: str, status: str | None) -> str:
    if rel in STALE_CONTEXT_BY_PATH:
        return STALE_CONTEXT_BY_PATH[rel]
    if status == "EVIDENCE-ONLY":
        return "LegacyEvidence"
    if rel.split("/", 1)[0] in PRODUCTION_SOURCE_DIRS:
        return "ProductionSource"
    return "OrdinaryAuthoritative"


def scan_stale_occurrences(rel: str, text: str, context: str, alias_map, findings: list[str]) -> None:
    lower = text.lower()
    known = {ident for ident, _, _ in alias_map.values()}
    for match in STALE_REF_RE.finditer(text):
        if match.group(1) not in known:
            findings.append(f"{rel}: STALE-REF names {match.group(1)}, which owns no stale alias")
    for key, (ident, contexts, alias) in alias_map.items():
        start = 0
        while True:
            pos = lower.find(key, start)
            if pos < 0:
                break
            start = pos + len(key)
            if context in contexts:
                continue
            line_start = text.rfind("\n", 0, pos) + 1
            line_end = text.find("\n", pos)
            line = text[line_start: line_end if line_end >= 0 else len(text)]
            if ident in STALE_REF_RE.findall(line):
                continue
            findings.append(f"{rel}: stale term {alias!r} ({ident}) in {context} context needs [STALE-REF: {ident}]")


def check_stale_vocabulary(root: Path, findings: list[str]) -> None:
    alias_map = parse_stale_vocabulary(root, findings)
    if not alias_map:
        return
    derived = {alias for _, _, alias in alias_map.values()}
    for path in sorted(root.rglob("*.md")):
        rel_path = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel_path.parts):
            continue
        rel = rel_path.as_posix()
        text = path.read_text(encoding="utf-8")
        context = stale_context_for(rel, frontmatter(text).get("status"))
        scan_stale_occurrences(rel, text, context, alias_map, findings)
    for base in PRODUCTION_SOURCE_DIRS:
        base_dir = root / base
        if not base_dir.is_dir():
            continue
        for path in sorted(base_dir.rglob("*")):
            if not path.is_file() or path.suffix not in (".rs", ".toml"):
                continue
            rel = path.relative_to(root).as_posix()
            scan_stale_occurrences(rel, path.read_text(encoding="utf-8"), "ProductionSource", alias_map, findings)
    for rel in STALE_PROJECTION_DOCS:
        path = root / rel
        if not path.is_file():
            findings.append(f"missing stale-vocabulary projection doc {rel}")
            continue
        block = STALE_VOCAB_BLOCK_RE.search(path.read_text(encoding="utf-8"))
        if not block:
            findings.append(f"{rel}: missing <!-- STALE-VOCAB:BEGIN/END --> projection block")
            continue
        listed = {
            line.strip()
            for line in block.group(1).splitlines()
            if line.strip() and not line.strip().startswith("```")
        }
        for alias in sorted(derived - listed):
            findings.append(f"{rel}: stale projection missing derived alias {alias!r}")
        for alias in sorted(listed - derived):
            findings.append(f"{rel}: stale projection lists {alias!r}, not derived from any decision")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    findings: list[str] = []
    markdown = sorted(root.rglob("*.md"))
    contract_ids: dict[str, Path] = {}
    for path in markdown:
        relative_path = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in relative_path.parts):
            continue
        rel = relative_path.as_posix()
        text = path.read_text(encoding="utf-8")
        meta = frontmatter(text)
        required = GENERATED_FRONT if meta.get("status") == "GENERATED" else REQUIRED_FRONT
        missing = required - meta.keys()
        if missing:
            findings.append(f"{rel}: missing frontmatter keys {sorted(missing)}")
        elif meta["status"] not in STATUSES:
            findings.append(f"{rel}: unknown status {meta['status']!r}")
        cid = meta.get("contract_id")
        if cid:
            if cid in contract_ids:
                findings.append(f"duplicate contract_id {cid}: {rel} and {contract_ids[cid]}")
            contract_ids[cid] = path
        if meta.get("status") == "AUTHORITATIVE":
            upper = text.upper()
            for token in PLACEHOLDERS:
                if token in upper:
                    findings.append(f"{rel}: normative placeholder {token}")
        for target in markdown_links(text):
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root)
            except ValueError:
                findings.append(f"{rel}: link escapes bundle: {target}")
                continue
            if not resolved.exists():
                findings.append(f"{rel}: broken relative link: {target}")

    check_architecture(root, findings)
    check_seed_ids(root, findings)
    check_stale_vocabulary(root, findings)
    check_batql(root, findings)
    check_numeric(root, findings)
    check_guarantees(root, findings)
    check_document_law(root, findings)
    findings.extend(d1_substrate_findings(root))
    findings.extend(specialization_findings(root))
    findings.extend(specialization_key_findings(root))
    findings.extend(proof_policy_findings(root))
    findings.extend(witness_reference_findings(root))
    findings.extend(integrity_witness_findings(root))
    findings.extend(derived_material_findings(root))
    findings.extend(deferred_posture_findings(root))
    findings.extend(proof_target_grammar_findings(root))
    findings.extend(proof_target_findings(root))
    findings.extend(leg081_authority_findings(root))
    findings.extend(proof_meaning_findings(root))
    findings.extend(control_character_findings(root))

    manifest = root / "SPEC.sha256"
    actual_files = frozen_files(root)
    for earlier, later in casefold_collisions(actual_files.keys()):
        findings.append(f"case-folded path collision (not Windows-portable): {earlier} and {later}")
    manifest_rows: dict[str, str] = {}
    if not manifest.is_file():
        findings.append("missing SPEC.sha256")
    else:
        for line_no, line in enumerate(manifest.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.startswith("#"):
                continue
            try:
                expected, rel = line.split("  ", 1)
            except ValueError:
                findings.append(f"SPEC.sha256:{line_no}: malformed row")
                continue
            if rel in manifest_rows:
                findings.append(f"SPEC.sha256:{line_no}: duplicate row {rel}")
            manifest_rows[rel] = expected
            path = actual_files.get(rel)
            if path is None:
                findings.append(f"SPEC.sha256:{line_no}: missing or excluded {rel}")
                continue
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != expected:
                findings.append(f"SPEC.sha256:{line_no}: digest mismatch {rel}")
        missing_rows = sorted(set(actual_files) - set(manifest_rows))
        extra_rows = sorted(set(manifest_rows) - set(actual_files))
        for rel in missing_rows:
            findings.append(f"SPEC.sha256: unrecorded file {rel}")
        for rel in extra_rows:
            findings.append(f"SPEC.sha256: row has no frozen file {rel}")

    if findings:
        print(f"audit: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    # The structurally discovered candidate set and the transitional expectation
    # debt are reported every run, not asserted as constants here. Both must reach
    # zero by D4d; a number that only ever appears in a report cannot rot into a
    # registry that every normalization commit has to ceremonially update.
    cand_ids, cand_blocks, pending = candidate_summary(root)
    print(
        f"audit: PASS ({len(markdown)} markdown documents, {len(contract_ids)} contract IDs, "
        f"{len(actual_files)} frozen files; {cand_ids} unbound proof candidates in "
        f"{cand_blocks} blocks; {pending} rows pending an expectation clause)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

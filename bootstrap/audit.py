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
STALE_REF_RE = re.compile(r"\[STALE-REF:\s*(DEC-\d+)\]")
STALE_VOCAB_BLOCK_RE = re.compile(r"<!-- STALE-VOCAB:BEGIN -->(.*?)<!-- STALE-VOCAB:END -->", re.S)
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
    r'associativity: Associativity::(\w+), input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", formatting: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
BATQL_OP_FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "input_sorts", "result_sort", "exactness",
    "overflow", "exception", "formatting", "spoken", "mutation_classes",
    "numeric_support",
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
    return out


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
    block_specs = (
        ("OPERATORS-CATALOG", "companion/BATQL_LANGUAGE.md", batql_render_catalog(ops)),
        ("OPERATORS-GRAMMAR", "companion/BATQL_LANGUAGE.md", batql_render_grammar(ops)),
        ("OPERATORS-PROJECTION", "companion/BATQL_LANGUAGE.md", batql_render_projection(ops)),
        ("OPERATORS-NUMERIC", "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md", batql_render_numeric(ops)),
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
    r'owner: "([^"]*)", gates: &\[([^\]]*)\], witness: "([^"]*)", '
    r'failure_disposition: "([^"]*)", derives_from: &\[([^\]]*)\], '
    r'refines: &\[([^\]]*)\], discharges: &\[([^\]]*)\], supersedes: &\[([^\]]*)\] \}'
)
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
    r'PackageSpec \{\s*package: "([^"]+)",\s*path: "[^"]+",\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
G_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: "([^"]+)",\s*profile: "([^"]+)",\s*'
    r'target: "([^"]+)",\s*requirement: "[^"]+",\s*\}', re.S)
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
            "gate_names": gate_list(m[4]), "witness": m[5], "failure": m[6],
            "rel": {"DerivesFrom": _g_ids(m[7]), "Refines": _g_ids(m[8]),
                    "Discharges": _g_ids(m[9]), "Supersedes": _g_ids(m[10])},
        })
    return rows


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


def guarantee_derive(root: Path) -> tuple[list[dict], list[tuple]]:
    nodes = []
    seed = guarantee_seed_rows(root)
    for s in seed:
        nodes.append({"id": s["id"], "family": "SEED", "kind": s["kind"], "lifetime": s["lifetime"],
                      "owner": s["owner"], "gates": gate_render(s["gate_names"], root),
                      "witness": s["witness"]})
    leg_meta = guarantee_leg_meta(root)
    for lid, meta in leg_meta.items():
        nodes.append({"id": lid, "family": "LEG", "kind": "LegacyObligation", "lifetime": _leg_lifetime(meta),
                      "owner": meta["owner"], "gates": gate_render(meta["gate_names"], root),
                      "witness": ""})
    dec = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    for did, dcls, dgates, _disp in G_DEC_ROW.findall(dec):
        # Decision gates are node metadata; they never become graph edges.
        nodes.append({"id": did, "family": "DEC", "kind": "Decision", "lifetime": "Permanent",
                      "owner": "docs/30_DECISION_AND_REJECTION_LEDGER.md",
                      "gates": gate_render(gate_list(dgates), root), "witness": "", "dclass": dcls})
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    for pkg, _cls, _layer in G_PKG_ROW.findall(arch):
        nodes.append({"id": f"ARCH-{pkg}", "family": "ARCH", "kind": "ArchitectureConstraint",
                      "lifetime": "Permanent", "owner": "spec/architecture.rs", "gates": "",
                      "witness": "spec/architecture.rs; audit.py architecture checks"})
    for pkg, profile, target in G_QUAL_ROW.findall(arch):
        nodes.append({"id": f"QUAL-{pkg}-{profile}", "family": "QUAL", "kind": "QualificationRequirement",
                      "lifetime": "Permanent", "owner": "spec/architecture.rs", "gates": target, "witness": ""})
    nodes.sort(key=lambda n: (G_FAMILY_RANK[n["family"]], n["id"]))
    edges = []
    for s in seed:
        for kind in ("DerivesFrom", "Refines", "Discharges", "Supersedes"):
            for target in s["rel"][kind]:
                edges.append((s["id"], kind, target))
    edges.sort(key=lambda e: (e[0], e[1], e[2]))
    return nodes, edges


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


def guarantee_lifetime_findings(nodes: list[dict], leg_meta: dict[str, dict], edges: list[tuple]) -> list[str]:
    node_ids = {n["id"] for n in nodes}
    # A guarantee is validly UntilSuccessor only if a resolvable typed successor
    # supersedes it. clean_owner text and gate names are never successor edges.
    superseded = {t for s, k, t in edges if k == "Supersedes" and t in node_ids}
    out = []
    for n in nodes:
        life, kind = n["lifetime"], n["kind"]
        if life == "HistoricalCoverageOnly" and n["gates"].strip():
            out.append(f"{n['id']} HistoricalCoverageOnly cannot gate")
        if life == "UntilGate" and not n["gates"].strip():
            out.append(f"{n['id']} UntilGate names no gate")
        if life == "UntilSuccessor" and n["id"] not in superseded:
            out.append(f"{n['id']} UntilSuccessor names no resolvable typed successor relation")
        if life == "Permanent" and kind in PERMANENT_WITNESS_KINDS and not n["witness"].strip():
            out.append(f"{n['id']} Permanent {kind} names no witness")
        if n["family"] == "LEG":
            meta = leg_meta.get(n["id"], {})
            if meta.get("status") == "Active" and life == "ClosedEvidence":
                out.append(f"{n['id']} Active LEG projects as ClosedEvidence")
            if meta.get("status") == "Closed" and life != "ClosedEvidence":
                out.append(f"{n['id']} Closed LEG does not project as ClosedEvidence")
            if meta.get("status") == "Closed" and n["gates"].strip():
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
A_AH_PROFILE_ROW = re.compile(
    r'AuthenticatedHistoryProfileSpec \{\s*'
    r'profile: AuthenticatedHistoryProfile::(\w+),\s*'
    r'permitted_witness_policies: &\[([^\]]*)\],\s*'
    r'requires_local_commitment_verification: (\w+),\s*'
    r'requires_signed_history_verification: (\w+),\s*'
    r'requires_independent_witness_verification: (\w+),\s*'
    r'implementation_gates: &\[([^\]]*)\],\s*'
    r'release_qualification_gates: &\[([^\]]*)\],\s*'
    r'claim_posture: HistoryClaimPosture::(\w+),\s*'
    r'unanchored_claim_posture: HistoryClaimPosture::(\w+),\s*\}',
    re.S,
)
A_WITNESS_VARIANT = re.compile(r'WitnessPolicy::(\w+)')
A_DISPOSITION_VARIANT = re.compile(r'WitnessDisposition::(\w+)')

# The frozen matrix. Anything outside it is refused, never normalized.
A_FROZEN_MATRIX = {
    "InternalConsistency": ["None"],
    "SignedHistory": ["None", "Optional"],
    "ExternallyAnchoredHistory": ["Required"],
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
A_CLAIM_POSTURES = {
    "InternalConsistencyVerified",
    "AuthenticatedHistoryVerifiedNoFreshnessClaim",
    "ExternallyAnchoredForThisWitnessedGeneration",
    "RollbackResistanceUnavailable",
}


def authenticated_history_rows(root: Path) -> list[dict]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    out = []
    for m in A_AH_PROFILE_ROW.findall(src):
        out.append({
            "profile": m[0],
            "policies": A_WITNESS_VARIANT.findall(m[1]),
            "local": m[2] == "true",
            "signed": m[3] == "true",
            "witness": m[4] == "true",
            "impl_gates": gate_list(m[5]),
            "release_gates": gate_list(m[6]),
            "claim": m[7],
            "unanchored": m[8],
        })
    return out


def _const_variants(root: Path, name: str) -> list[str]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    m = re.search(r"pub const " + re.escape(name) + r":[^=]*=\s*&\[(.*?)\];", src, re.S)
    return A_DISPOSITION_VARIANT.findall(m.group(1)) if m else []


def authenticated_history_findings(root: Path) -> list[str]:
    rows = authenticated_history_rows(root)
    out: list[str] = []
    if not rows:
        return ["spec/architecture.rs declares no AUTHENTICATED_HISTORY_PROFILES"]
    names = [r["profile"] for r in rows]
    if len(names) != len(set(names)):
        out.append("duplicate authenticated-history profile name")
    if set(names) != set(A_FROZEN_MATRIX):
        out.append(f"authenticated-history profile set {sorted(names)} != frozen {sorted(A_FROZEN_MATRIX)}")
    for r in rows:
        want = A_FROZEN_MATRIX.get(r["profile"])
        if want is None:
            continue
        if r["policies"] != want:
            out.append(
                f"{r['profile']} permits witness policies {r['policies']}, frozen matrix says {want}"
            )
        # Required exists only with ExternallyAnchoredHistory.
        if "Required" in r["policies"] and r["profile"] != "ExternallyAnchoredHistory":
            out.append(f"{r['profile']} permits WitnessPolicy::Required outside ExternallyAnchoredHistory")
        if r["profile"] == "ExternallyAnchoredHistory" and (
            "None" in r["policies"] or "Optional" in r["policies"]
        ):
            out.append("ExternallyAnchoredHistory permits a non-Required witness policy")
        # Claim ceilings: no profile may claim more than it verifies.
        if r["profile"] == "InternalConsistency":
            if r["signed"] or r["witness"]:
                out.append("InternalConsistency requires signed-history or witness verification")
            if r["claim"] != "InternalConsistencyVerified":
                out.append(f"InternalConsistency claims {r['claim']}, exceeding internal consistency")
            if r["unanchored"] != "RollbackResistanceUnavailable":
                out.append("InternalConsistency does not state rollback resistance unavailable")
        if r["profile"] == "SignedHistory":
            if not r["signed"]:
                out.append("SignedHistory does not require signed-history verification")
            if r["witness"]:
                out.append("SignedHistory requires an independent witness; that is ExternallyAnchoredHistory")
            if r["unanchored"] != "AuthenticatedHistoryVerifiedNoFreshnessClaim":
                out.append(
                    "SignedHistory without a verified witness claims freshness or rollback resistance"
                )
        if r["profile"] == "ExternallyAnchoredHistory":
            if not (r["signed"] and r["witness"]):
                out.append("ExternallyAnchoredHistory does not require signed history plus an independent witness")
            if r["unanchored"] != "RollbackResistanceUnavailable":
                out.append("ExternallyAnchoredHistory unanchored posture claims rollback resistance")
        if not r["local"]:
            out.append(f"{r['profile']} does not require local commitment verification")
        if r["claim"] not in A_CLAIM_POSTURES:
            out.append(f"{r['profile']} names unknown claim posture {r['claim']}")
        out.extend(gate_findings(root, "profile", r["profile"] + " implementation", r["impl_gates"]))
        out.extend(gate_findings(root, "profile", r["profile"] + " release", r["release_gates"]))
        if not r["impl_gates"]:
            out.append(f"{r['profile']} names no implementation gate")
        if not r["release_gates"]:
            out.append(f"{r['profile']} names no release qualification gate")
    # The witness axes must stay distinct.
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    enum_m = re.search(r"pub enum WitnessDisposition \{(.*?)\n\}", src, re.S)
    if not enum_m:
        out.append("spec/architecture.rs declares no WitnessDisposition")
    else:
        declared = set(re.findall(r"^\s{4}(\w+),", enum_m.group(1), re.M))
        missing = A_WITNESS_DISPOSITIONS - declared
        if missing:
            out.append(f"WitnessDisposition collapses required distinctions: missing {sorted(missing)}")
    posture_m = re.search(r"pub enum HistoryClaimPosture \{(.*?)\n\}", src, re.S)
    if not posture_m:
        out.append("spec/architecture.rs declares no HistoryClaimPosture")
    else:
        declared = set(re.findall(r"^\s{4}(\w+),", posture_m.group(1), re.M))
        missing = A_CLAIM_POSTURES - declared
        if missing:
            out.append(f"HistoryClaimPosture collapses required distinctions: missing {sorted(missing)}")
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
    "docs/14_RECEIPTS_AND_EXPLANATION.md": [
        ("receipt preserves the exact profile", "selected AuthenticatedHistoryProfile"),
        ("receipt preserves the exact witness policy", "selected WitnessPolicy"),
        ("receipt preserves the exact witness disposition", "exact WitnessDisposition"),
        ("receipt preserves the exact claim posture", "exact HistoryClaimPosture"),
        ("receipt preserves the proof disposition",
         "ProofDisposition                      passed through, never upgraded or collapsed"),
        ("absent and invalid optional witnesses never render identically",
         "An absent optional witness is a successful unanchored result"),
        ("no formatter upgrades a claim", "may upgrade a claim posture"),
    ],
    "docs/05_STORAGE_FBAT_AND_TILES.md": [
        ("storage owns segment seals", "segment seal"),
        ("storage owns the history commitment", "global append accumulator"),
        ("coherence is not freshness", "Coherence is not freshness"),
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


def check_guarantees(root: Path, findings: list[str]) -> None:
    if not (root / "spec/guarantees.rs").is_file():
        findings.append("missing spec/guarantees.rs")
        return
    seed_rows = guarantee_seed_rows(root)
    nodes, edges = guarantee_derive(root)
    leg_meta = guarantee_leg_meta(root)
    node_ids = {n["id"] for n in nodes}
    findings.extend(gate_inventory_findings(root))
    findings.extend(gate_doc_findings(root))
    findings.extend(gate_reference_findings(root))
    findings.extend(decision_class_findings(root))
    findings.extend(authenticated_history_findings(root))
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
    malformed = [r for r in node_rows if len(r) != 7]
    if malformed or not node_rows:
        findings.append(
            f"Guarantee Graph node table has {len(malformed)} malformed row(s) of {len(node_rows)}"
        )
    file_nodes = {(r[0], r[1], r[2], r[3], r[4], r[5], r[6]) for r in node_rows if len(r) == 7}
    derived_nodes = {
        (n["id"], n["family"], n["kind"], n["lifetime"], n.get("dclass", "-"), n["owner"], n["gates"])
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


def check_architecture(root: Path, findings: list[str]) -> None:
    path = root / "spec/architecture.rs"
    if not path.is_file():
        findings.append("missing spec/architecture.rs")
        return
    source = path.read_text(encoding="utf-8")
    package_rows = re.findall(
        r"PackageSpec\s*\{\s*package:\s*\"([^\"]+)\",\s*path:\s*\"([^\"]+)\",\s*role:\s*\"([^\"]+)\",\s*class:\s*PackageClass::([A-Za-z]+),\s*layer:\s*([0-9]+),\s*\}",
        source,
        re.S,
    )
    if not package_rows:
        findings.append("spec/architecture.rs: no package rows parsed")
        return
    packages = [row[0] for row in package_rows]
    paths = [row[1] for row in package_rows]
    layers = {row[0]: int(row[4]) for row in package_rows}
    if len(packages) != len(set(packages)):
        findings.append("spec/architecture.rs: duplicate package name")
    if len(paths) != len(set(paths)):
        findings.append("spec/architecture.rs: duplicate package path")
    for package, package_path, role, package_class, layer in package_rows:
        if not package_path or not role or package_class not in {"Production", "BinaryAdapter", "DevOnly", "Example"} or not layer.isdigit():
            findings.append(f"spec/architecture.rs: incomplete package {package}")

    edge_rows = re.findall(
        r"EdgeSpec\s*\{\s*importer:\s*\"([^\"]+)\",\s*importee:\s*\"([^\"]+)\",\s*class:\s*EdgeClass::([A-Za-z]+),\s*profile:\s*\"([^\"]+)\"\s*\}",
        source,
        re.S,
    )
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
        r"QualificationProfile\s*\{\s*package:\s*\"([^\"]+)\",\s*profile:\s*\"([^\"]+)\",\s*target:\s*\"([^\"]+)\",\s*requirement:\s*\"([^\"]+)\",\s*\}",
        source,
        re.S,
    )
    identities: set[tuple[str, str]] = set()
    for package, profile, target, requirement in profile_rows:
        identity = (package, profile)
        if package not in package_set or not profile or not target or not requirement:
            findings.append(f"spec/architecture.rs: incomplete qualification {package}:{profile}")
        if identity in identities:
            findings.append(f"spec/architecture.rs: duplicate qualification {package}:{profile}")
        identities.add(identity)
    for package in ("batpak", "syncbat"):
        if (package, "semantic") not in identities or not any(
            row[0] == package and row[1] == "semantic" and row[2] == "no_std + alloc" for row in profile_rows
        ):
            findings.append(f"spec/architecture.rs: missing no_std + alloc semantic profile for {package}")

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
    print(
        f"audit: PASS ({len(markdown)} markdown documents, {len(contract_ids)} contract IDs, "
        f"{len(actual_files)} frozen files)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

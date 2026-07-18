#!/usr/bin/env python3
"""Independent standard-library audit for the clean-room specification."""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

STATUSES = {"AUTHORITATIVE", "GENERATED", "EVIDENCE-ONLY", "SUPERSEDED", "REJECTED"}
REQUIRED_FRONT = {"status", "contract_id", "authority_scope", "supersedes", "last_reconciled",
                  "reconciliation_epoch"}
GENERATED_FRONT = {"status", "authority_scope", "generated_by", "generated_from", "do_not_edit",
                   "source_reconciliation_epoch"}
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
    r'OperatorSpec \{ id: OperatorId::(\w+), class: OperatorClass::(\w+), '
    r'syntax: OperatorSyntax::(\w+)\(([^)]*)\), semantic_op: "([^"]*)", '
    r'arity: Arity::(\w+), fixity: Fixity::(\w+), precedence: (\d+), '
    r'associativity: Associativity::(\w+), typing: &\[([^\]]*)\], input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
BATQL_PROOF_DISPOSITIONS = ["VERIFIED", "LEGACY_WEAK", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]
BATQL_SHAPE_DISPLAY = {"SymbolOnly": "symbol-only", "WordOnly": "word-only",
                       "WordWithSymbolAlias": "word-with-symbol"}
# The canonical uppercase word grammar and the nonempty-ASCII-punctuation
# symbol grammar. A word can never enter the symbol inventory and punctuation
# can never enter the word inventory.
BATQL_WORD_GRAMMAR = re.compile(r"^[A-Z]+(?: [A-Z]+)*$")
BATQL_SYMBOL_GRAMMAR = re.compile(r"^[!-/:-@\[-`{-~]+$")
# The class/shape law: arithmetic symbols are canonical source, comparison
# words are canonical with exact symbol aliases, logical operators are words
# in every source density.
BATQL_CLASS_SHAPE = {"Arithmetic": "SymbolOnly", "Comparison": "WordWithSymbolAlias",
                     "Logical": "WordOnly"}


def _batql_enum_variants(src: str, type_name: str) -> list[str]:
    body = re.search(r"pub enum " + type_name + r" \{(.*?)\n\}", src, re.S)
    if not body:
        return []
    return re.findall(r"\n    (\w+),", _uncomment(body.group(1)))


def _batql_all_inventory(src: str, type_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", src, re.S)
    # \b keeps a longer type name's variants from satisfying a shorter prefix.
    return re.findall(r"\b" + type_name + r"::(\w+)", body.group(1)) if body else []


def _batql_fn_arms(src: str, type_name: str, fn_name: str, value_re: str) -> dict[str, str]:
    """Variant -> value arms of one const fn, scoped to the type's impl block."""
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", src, re.S)
    if not impl:
        return {}
    fn = re.search(
        r"pub const fn " + fn_name + r"\(self\) -> [^{]+\{\s*match self \{(.*?)\n        \}",
        impl.group(1), re.S)
    if not fn:
        return {}
    return dict(re.findall(r"\b" + type_name + r"::(\w+) => " + value_re + ",", fn.group(1)))


def batql_parse_operator_model(root: Path) -> dict:
    """The auditor's own parse of the typed operator source: enum variants,
    ALL inventories, token/spelling arms, owner and basis arms, and raw
    OperatorSpec rows. Shares no code with the generator."""
    src = (root / "spec/operators.rs").read_text(encoding="utf-8")
    model: dict = {"src": src}
    for key, type_name, token_fn in (("id", "OperatorId", "spelling"),
                                     ("word", "OperatorWordSurface", "token"),
                                     ("symbol", "OperatorSymbolSurface", "token")):
        model[f"{key}_type"] = type_name
        model[f"{key}_variants"] = _batql_enum_variants(src, type_name)
        model[f"{key}_all"] = _batql_all_inventory(src, type_name)
        model[f"{key}_token"] = _batql_fn_arms(src, type_name, token_fn, r'"([^"]*)"')
        model[f"{key}_owner"] = _batql_fn_arms(
            src, type_name, "semantic_owner", r'ContractId\("([^"]*)"\)')
        model[f"{key}_basis"] = _batql_fn_arms(
            src, type_name, "admission_basis", r'DecisionId\("([^"]*)"\)')
    model["rows"] = [
        {"variant": m[0], "class": m[1], "shape": m[2], "inner": m[3],
         "semantic_op": m[4], "arity": m[5], "fixity": m[6], "precedence": m[7],
         "associativity": m[8], "typing": m[9], "input_sorts": m[10],
         "result_sort": m[11], "exactness": m[12], "overflow": m[13],
         "exception": m[14], "spoken": m[15], "mutation_classes": m[16],
         "numeric_support": m[17]}
        for m in BATQL_OP_ROW.findall(src)
    ]
    return model


def batql_resolve_operator_rows(model: dict, out: list[str]) -> list[dict[str, str]]:
    """Raw rows resolved through the typed token maps, in OperatorId::ALL
    order. An unresolvable variant is a finding and its row is dropped —
    there is no raw-string fallback."""
    resolved: dict[str, dict[str, str]] = {}
    for raw in model["rows"]:
        variant = raw["variant"]
        words = re.findall(r"OperatorWordSurface::(\w+)", raw["inner"])
        symbols = re.findall(r"OperatorSymbolSurface::(\w+)", raw["inner"])
        unknown = False
        if variant not in model["id_token"]:
            out.append(f"spec/operators.rs: OperatorSpec row names OperatorId::{variant}, "
                       "which declares no spelling arm")
            unknown = True
        for word in words:
            if word not in model["word_token"]:
                out.append(f"spec/operators.rs: OperatorSpec row {variant} uses "
                           f"OperatorWordSurface::{word}, which the word inventory does not declare")
                unknown = True
        for symbol in symbols:
            if symbol not in model["symbol_token"]:
                out.append(f"spec/operators.rs: OperatorSpec row {variant} uses "
                           f"OperatorSymbolSurface::{symbol}, which the symbol inventory does not declare")
                unknown = True
        if unknown:
            continue
        shape = raw["shape"]
        if shape == "SymbolOnly" and not words and len(symbols) == 1:
            canonical, alias = model["symbol_token"][symbols[0]], ""
        elif shape == "WordOnly" and len(words) == 1 and not symbols:
            canonical, alias = model["word_token"][words[0]], ""
        elif shape == "WordWithSymbolAlias" and len(words) == 1 and len(symbols) == 1:
            canonical, alias = model["word_token"][words[0]], model["symbol_token"][symbols[0]]
        else:
            out.append(f"spec/operators.rs: OperatorSpec row {variant} carries "
                       f"unresolvable syntax {shape}({raw['inner']})")
            continue
        if variant in resolved:
            out.append(f"spec/operators.rs: duplicate OperatorSpec row for OperatorId::{variant}")
            continue
        row = dict(raw)
        del row["inner"]
        row["id"] = model["id_token"][variant]
        row["canonical"] = canonical
        row["alias"] = alias
        resolved[variant] = row
    order = [v for v in model["id_all"] if v in resolved]
    order += [v for v in resolved if v not in model["id_all"]]
    return [resolved[v] for v in order]


def batql_operator_model_findings(model: dict) -> list[str]:
    """Structural laws on the typed source: enum/ALL parity, token discipline,
    surface adoption, row/identity parity and order, and the wildcard ban."""
    out: list[str] = []
    for key in ("id", "word", "symbol"):
        type_name = model[f"{key}_type"]
        variants, inventory = model[f"{key}_variants"], model[f"{key}_all"]
        if not inventory:
            out.append(f"spec/operators.rs declares no {type_name}::ALL inventory")
        for variant in variants:
            if variant not in inventory:
                out.append(f"spec/operators.rs: {type_name}::{variant} is declared "
                           f"but missing from {type_name}::ALL")
        for variant in inventory:
            if variant not in variants:
                out.append(f"spec/operators.rs: {type_name}::ALL names {variant}, "
                           "which the enum does not declare")
        tokens = model[f"{key}_token"]
        fn_name = "spelling" if key == "id" else "token"
        for variant in inventory:
            if variant not in tokens:
                out.append(f"spec/operators.rs: {type_name}::{variant} declares no {fn_name} arm")
            if variant not in model[f"{key}_owner"]:
                out.append(f"spec/operators.rs: {type_name}::{variant} declares no semantic_owner arm")
            if variant not in model[f"{key}_basis"]:
                out.append(f"spec/operators.rs: {type_name}::{variant} declares no admission_basis arm")
        seen: dict[str, str] = {}
        for variant, token in tokens.items():
            if not token:
                out.append(f"spec/operators.rs: {type_name}::{variant} declares an empty token")
                continue
            if token in seen:
                out.append(f"spec/operators.rs: duplicate {type_name} token {token!r} "
                           f"({seen[token]} and {variant})")
            seen[token] = variant
            if key == "word" and not BATQL_WORD_GRAMMAR.match(token):
                out.append(f"spec/operators.rs: word token {token!r} violates the "
                           "canonical uppercase word grammar; punctuation can never "
                           "enter the word inventory")
            if key == "symbol" and not BATQL_SYMBOL_GRAMMAR.match(token):
                out.append(f"spec/operators.rs: symbol token {token!r} is not nonempty "
                           "ASCII punctuation; a word can never enter the symbol inventory")
    if re.search(r"\n\s*_ =>", _uncomment(model["src"])):
        out.append("spec/operators.rs: a wildcard match arm may not stand in for "
                   "explicit per-variant law")
    # Row/identity parity in ALL order, and surface adoption in both directions.
    row_variants = [raw["variant"] for raw in model["rows"]]
    for variant in model["id_all"]:
        count = row_variants.count(variant)
        if count == 0:
            out.append(f"spec/operators.rs: OperatorId::{variant} has no OperatorSpec row")
        elif count > 1:
            out.append(f"spec/operators.rs: duplicate OperatorSpec row for OperatorId::{variant}")
    for variant in row_variants:
        if variant not in model["id_all"]:
            out.append(f"spec/operators.rs: OperatorSpec row {variant} is missing "
                       "from OperatorId::ALL")
    known_rows = [v for v in row_variants if v in model["id_all"]]
    expected = [v for v in model["id_all"] if v in row_variants]
    if known_rows != expected:
        out.append("spec/operators.rs: OPERATORS row order does not equal OperatorId::ALL")
    used_words = []
    used_symbols = []
    for raw in model["rows"]:
        used_words += re.findall(r"OperatorWordSurface::(\w+)", raw["inner"])
        used_symbols += re.findall(r"OperatorSymbolSurface::(\w+)", raw["inner"])
    for variant in model["word_all"]:
        if variant not in used_words:
            out.append(f"spec/operators.rs: word surface {variant} is adopted by no "
                       "OperatorSpec row; a declared surface without an adopter is an orphan")
    for variant in model["symbol_all"]:
        if variant not in used_symbols:
            out.append(f"spec/operators.rs: symbol surface {variant} is adopted by no "
                       "OperatorSpec row; a declared surface without an adopter is an orphan")
    for variant in set(used_words):
        if used_words.count(variant) > 1:
            out.append(f"spec/operators.rs: word surface {variant} is adopted by more "
                       "than one OperatorSpec row")
    for variant in set(used_symbols):
        if used_symbols.count(variant) > 1:
            out.append(f"spec/operators.rs: symbol surface {variant} is adopted by more "
                       "than one OperatorSpec row; an alias stays attached to one OperatorId")
    return out


def batql_operator_authority_findings(model: dict, contract_ids: set[str],
                                      decision_ids: set[str]) -> list[str]:
    """Every owner arm resolves to a declared contract and every basis arm to
    a declared decision, on all three typed families."""
    out: list[str] = []
    for key in ("id", "word", "symbol"):
        type_name = model[f"{key}_type"]
        for variant, owner in sorted(model[f"{key}_owner"].items()):
            if owner not in contract_ids:
                out.append(f"spec/operators.rs: {type_name}::{variant} names owner "
                           f"{owner}, which no declared contract owns")
        for variant, basis in sorted(model[f"{key}_basis"].items()):
            if basis not in decision_ids:
                out.append(f"spec/operators.rs: {type_name}::{variant} names admission "
                           f"basis {basis}, which no declared decision owns")
    return out


# 5.5E3j1: the documentary layer of spec/operators.rs may fence OPERATOR
# ADMISSION, never deny VALUE EXISTENCE. First-class Qualified Approximation
# is owned by DEC-069/docs/37 today; the ordinary operator parser strips
# comments, so this law reads the RAW source before any comment removal.
BATQL_OPERATOR_DOC_DENIALS = (
    re.compile(r"No `?Approximate`? value exists"),
    re.compile(r"arrives? with 5\.5B2"),
)
BATQL_OPERATOR_DOC_DOCTRINE = (
    ("value-existence", "already exist as typed values"),
    ("admission-fence", "admits raw approximate operands"),
    ("future-profile-law", "language-change record"),
)


def batql_operator_doc_findings(src: str) -> list[str]:
    """The value-existence/operator-admission distinction, executed over the
    raw source: approximate values exist (DEC-069, docs/37); what the module
    may deny is only ordinary-operator admission of raw approximate operands.
    Comment markers and line wraps are normalized away first, so a claim
    split across doc-comment lines neither hides a denial nor loses doctrine."""
    flat = re.sub(r"\s+", " ", src.replace("///", " ").replace("//!", " "))
    out: list[str] = []
    for pattern in BATQL_OPERATOR_DOC_DENIALS:
        match = pattern.search(flat)
        if match:
            out.append(
                "spec/operators.rs: approximate values exist; no current ordinary "
                "operator admits raw approximate operands — the documentary layer "
                f"may not deny value existence ({match.group(0)!r})")
    for name, fragment in BATQL_OPERATOR_DOC_DOCTRINE:
        if fragment not in flat:
            out.append(
                f"spec/operators.rs: approximate-authority doctrine absent ({name})")
    return out


def batql_render_catalog(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Class | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for op in ops:
        out.append(
            f"| {op['id']} | {op['class']} | {op['arity']} | "
            f"{op['fixity']} | {op['precedence']} | {op['associativity']} | {op['result_sort']} | {op['exactness']} |"
        )
    return "\n".join(out)


def batql_render_surfaces(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Shape | Canonical | Accepted alias |",
        "| --- | --- | --- | --- |",
    ]
    for op in ops:
        alias = f"`{op['alias']}`" if op["alias"] else "-"
        out.append(f"| {op['id']} | {BATQL_SHAPE_DISPLAY.get(op['shape'], '?')} | "
                   f"`{op['canonical']}` | {alias} |")
    return "\n".join(out)


def batql_render_grammar(ops: list[dict[str, str]]) -> str:
    comparison = [o["canonical"] for o in ops if o["class"] == "Comparison"]
    comparison += [f'"{o["alias"]}"' for o in ops if o["class"] == "Comparison" and o["alias"]]
    arithmetic = [f'"{o["canonical"]}"' for o in ops if o["class"] == "Arithmetic"]
    logical = [o["canonical"] for o in ops if o["class"] == "Logical" and o["fixity"] == "Infix"]
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
        "| OperatorId | Canonical formatting | Spoken | Legal mutation classes |",
        "| --- | --- | --- | --- |",
    ]
    for op in ops:
        out.append(f"| {op['id']} | `{op['canonical']}` | {op['spoken']} | {op['mutation_classes']} |")
    return "\n".join(out)


def batql_render_numeric(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Class | Numeric support |",
        "| --- | --- | --- |",
    ]
    for op in ops:
        out.append(f"| {op['id']} | {op['class']} | {op['numeric_support']} |")
    return "\n".join(out)


def batql_extract_block(text: str, name: str) -> str | None:
    match = re.search(
        r"<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n(.*?)\n<!-- " + re.escape(name) + r":END -->",
        text,
        re.S,
    )
    return match.group(1) if match else None


BATQL_ROW_FIELDS = (
    "id", "class", "shape", "canonical", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "typing", "input_sorts", "result_sort",
    "exactness", "overflow", "exception", "spoken", "mutation_classes",
    "numeric_support",
)


def batql_operator_fact_findings(ops: list[dict[str, str]]) -> list[str]:
    out: list[str] = []
    ids = [o["id"] for o in ops]
    if len(ids) != len(set(ids)):
        out.append("spec/operators.rs: duplicate OperatorId")
    owner: dict[tuple[str, str], str] = {}
    for op in ops:
        for surface in (op["canonical"], op.get("alias", "")):
            if not surface:
                continue
            key = (surface, op["fixity"])
            if key in owner and owner[key] != op["id"]:
                out.append(
                    f"spec/operators.rs: token {surface!r} with fixity {op['fixity']} claimed by {owner[key]} and {op['id']}"
                )
            owner[key] = op["id"]
    for op in ops:
        # The class/shape law is total: no fourth class, no fourth shape, no
        # exception path. A comparison symbol can never become canonical and a
        # logical operator can never gain a symbol alias.
        required_shape = BATQL_CLASS_SHAPE.get(op.get("class", ""))
        if required_shape is None:
            out.append(f"spec/operators.rs: operator {op.get('id', '?')} carries "
                       f"unknown class {op.get('class')!r}")
        elif op.get("shape") != required_shape:
            out.append(
                f"spec/operators.rs: {op.get('class', '?').lower()} operator "
                f"{op.get('id', '?')} carries {op.get('shape')!r} source syntax; "
                f"its class law requires {required_shape}")
        if op.get("shape") == "WordWithSymbolAlias" and not op.get("alias"):
            out.append(f"spec/operators.rs: operator {op.get('id', '?')} claims a "
                       "symbol alias shape but carries no alias token")
        if op.get("shape") in ("SymbolOnly", "WordOnly") and op.get("alias"):
            out.append(f"spec/operators.rs: operator {op.get('id', '?')} carries an "
                       "alias token outside the WordWithSymbolAlias shape")
    for op in ops:
        for field in BATQL_ROW_FIELDS:
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
    for op in ops:
        if op["id"] not in titles:
            continue
        lines += [f"### {titles[op['id']]} (`{op['canonical']}`)", "", "```text"]
        for rule in re.findall(r"OperatorTypingRule::(\w+)", op.get("typing", "")):
            lines += _batql_typing_rows(rule, op["canonical"])
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


# Every operator projection carries exact provenance: the BEGIN marker names
# the typed source and the generator, byte-exactly.
BATQL_OPERATOR_BLOCKS = (
    ("OPERATORS-CATALOG", "companion/BATQL_LANGUAGE.md"),
    ("OPERATORS-SURFACES", "companion/BATQL_LANGUAGE.md"),
    ("OPERATORS-GRAMMAR", "companion/BATQL_LANGUAGE.md"),
    ("OPERATORS-PROJECTION", "companion/BATQL_LANGUAGE.md"),
    ("OPERATORS-TYPING", "companion/BATQL_LANGUAGE.md"),
    ("OPERATORS-NUMERIC", "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md"),
)
BATQL_OPERATOR_PROVENANCE = "generated from spec/operators.rs by bootstrap/project.py; do not edit"
# The companion's authored operator-surface doctrine (5.5E3j). Each clause is
# executable law with its own hostile, checked against the authored body with
# every generated block removed.
A_OPERATOR_DOCTRINE = (
    ("arithmetic-canonical-symbols", "Arithmetic uses canonical symbols"),
    ("comparison-word-canonical", "Comparisons use canonical word phrases and accept exact symbolic aliases"),
    ("logical-words-every-density", "Logical operators remain canonical words in every source density"),
    ("one-typed-operator-id", "All accepted surfaces lower to one typed `OperatorId`"),
    ("formatter-canonical-only", "The default formatter emits only the canonical surface"),
    ("spoken-not-parser-syntax", "Spoken narration is not parser syntax"),
)


def _batql_authorities(root: Path) -> tuple[set[str], set[str]]:
    contract_ids: set[str] = set()
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        cid = frontmatter(rel.read_text(encoding="utf-8")).get("contract_id")
        if cid:
            contract_ids.add(cid)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    decision_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    return contract_ids, decision_ids


def batql_operator_corpus_findings(root: Path, model: dict,
                                   ops: list[dict[str, str]]) -> list[str]:
    """Cross-document operator surface laws: the DEC-060 admission boundary,
    the companion's authored doctrine and token authorship, projection
    provenance, the spoken fence, the retired raw-field boundary, and the
    typed-owner backing behind the OperatorId identity-residue row."""
    out: list[str] = []
    src = model["src"]
    # Retired raw surface fields and the empty-string sentinel may not return.
    for field in ("word_surface", "symbol_surface", "formatting"):
        if re.search(r"\b" + field + r"\s*:", _uncomment(src)):
            out.append(f"spec/operators.rs: retired raw surface field {field} may not return")
    if re.search(r':\s*""', _uncomment(src)):
        out.append("spec/operators.rs: an empty-string surface sentinel may not return")
    # The residue claim rests on a real public typed owner, not a raw string field.
    if not re.search(r"pub enum OperatorId\b", src):
        out.append("spec/operators.rs declares no public typed OperatorId owner; "
                   "the identity-residue claim may not rest on a raw string field")
    identities_src = (root / "spec/identities.rs").read_text(encoding="utf-8")
    if not re.search(
            r'term: "OperatorId",\s*disposition: IdentityTermDisposition::'
            r'OwnedElsewhere\(ContractId\("BP-BATQL-LANGUAGE-1"\)\)', identities_src):
        out.append("spec/identities.rs: the OperatorId owned-elsewhere residue row "
                   "(BP-BATQL-LANGUAGE-1) is missing")
    # DEC-060 is the admission basis: its forward-policy fields must name the
    # typed boundary and the exact V1 word/symbol mapping.
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    row = re.search(
        r'DecisionSpec \{ id: "DEC-060",.*?subject: "([^"]*)", successor: "([^"]*)",'
        r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
    fields = " ".join(g or "" for g in row.groups()) if row else ""
    if not row:
        out.append("DEC-060 is missing from the decision ledger")
    for token in ("OperatorId", "OperatorId::ALL", "OperatorWordSurface",
                  "OperatorWordSurface::ALL", "OperatorSymbolSurface",
                  "OperatorSymbolSurface::ALL", "OperatorSyntax", "spec/operators.rs"):
        if token not in fields:
            out.append(f"DEC-060 forward-policy fields do not name {token}; the basis "
                       "must name the operator surface boundary it admits")
    typed_pairs = [(op["canonical"], op["alias"]) for op in ops
                   if op["class"] == "Comparison"]
    named_pairs = re.findall(r"\b([A-Z]+(?: [A-Z]+)*) with (\S+?)[,;]", fields)
    if named_pairs != typed_pairs:
        out.append(f"DEC-060 names comparison pairs {named_pairs}, which do not equal "
                   f"the typed word/symbol mapping {typed_pairs}")
    arithmetic_symbols = " ".join(op["canonical"] for op in ops
                                  if op["class"] == "Arithmetic")
    if arithmetic_symbols and arithmetic_symbols not in fields:
        out.append(f"DEC-060 does not name the canonical arithmetic symbols "
                   f"{arithmetic_symbols}")
    word_only = " ".join(op["canonical"] for op in ops if op["class"] == "Logical")
    if word_only and f"{word_only} are word-only" not in fields:
        out.append(f"DEC-060 does not name {word_only} as word-only")
    # The companion's authored doctrine and token authorship: the owner's
    # authored body (generated blocks removed) authors every clause and every
    # canonical/alias token the typed source declares.
    companion_path = root / "companion/BATQL_LANGUAGE.md"
    authored = A_GENERATED_BLOCK.sub(
        "", companion_path.read_text(encoding="utf-8")) if companion_path.is_file() else ""
    for name, fragment in A_OPERATOR_DOCTRINE:
        if fragment not in authored:
            out.append(f"companion/BATQL_LANGUAGE.md 1.3: operator surface doctrine "
                       f"absent ({name})")
    for variant, token in sorted(model["word_token"].items()):
        if token and not re.search(r"\b" + re.escape(token) + r"\b", authored):
            out.append(f"companion/BATQL_LANGUAGE.md: the authored body does not "
                       f"author operator surface token {token!r}")
    for variant, token in sorted(model["symbol_token"].items()):
        if token and token not in authored:
            out.append(f"companion/BATQL_LANGUAGE.md: the authored body does not "
                       f"author operator surface token {token!r}")
    # Projection provenance: each BEGIN marker names the typed source and the
    # generator exactly.
    for name, rel in BATQL_OPERATOR_BLOCKS:
        path = root / rel
        text = path.read_text(encoding="utf-8") if path.is_file() else ""
        if re.search(r"<!-- " + re.escape(name) + r":BEGIN", text) and not re.search(
                r"<!-- " + re.escape(name) + r":BEGIN "
                + re.escape(BATQL_OPERATOR_PROVENANCE) + r" -->", text):
            out.append(f"{rel}: {name} BEGIN marker does not name its "
                       "spec/operators.rs + bootstrap/project.py provenance exactly")
    # The spoken projection is narration, never source: no spoken phrase may
    # surface as grammar, and the projection table formats every operator as
    # its canonical surface.
    companion_text = companion_path.read_text(encoding="utf-8") if companion_path.is_file() else ""
    grammar_body = batql_extract_block(companion_text, "OPERATORS-GRAMMAR") or ""
    for op in ops:
        spoken = op.get("spoken", "")
        if spoken and spoken not in (op["canonical"], op.get("alias", "")) \
                and re.search(r"\b" + re.escape(spoken) + r"\b", grammar_body):
            out.append(f"companion/BATQL_LANGUAGE.md: spoken projection {spoken!r} "
                       "appears in the operator grammar; narration is never source syntax")
    projection_body = batql_extract_block(companion_text, "OPERATORS-PROJECTION") or ""
    canonical_of = {op["id"]: op["canonical"] for op in ops}
    for op_id, cell in re.findall(r"^\| (\S+) \| `([^`]*)` \|", projection_body, re.M):
        if op_id in canonical_of and cell != canonical_of[op_id]:
            out.append(f"companion/BATQL_LANGUAGE.md: OPERATORS-PROJECTION formats "
                       f"{op_id} as {cell!r}, not its canonical surface "
                       f"{canonical_of[op_id]!r}")
    return out


def check_batql(root: Path, findings: list[str]) -> None:
    companion_path = root / "companion/BATQL_LANGUAGE.md"
    if not companion_path.is_file():
        findings.append("missing companion/BATQL_LANGUAGE.md")
        return
    companion = companion_path.read_text(encoding="utf-8")
    model = batql_parse_operator_model(root)
    findings.extend(batql_operator_model_findings(model))
    findings.extend(batql_operator_doc_findings(model["src"]))
    ops = batql_resolve_operator_rows(model, findings)
    if not ops:
        findings.append("spec/operators.rs: no OperatorSpec rows parsed")
    findings.extend(batql_operator_fact_findings(ops))
    contract_ids, decision_ids = _batql_authorities(root)
    findings.extend(batql_operator_authority_findings(model, contract_ids, decision_ids))
    findings.extend(batql_operator_corpus_findings(root, model, ops))
    findings.extend(phantom_sort_findings(root))
    block_specs = (
        ("OPERATORS-CATALOG", "companion/BATQL_LANGUAGE.md", batql_render_catalog(ops)),
        ("OPERATORS-SURFACES", "companion/BATQL_LANGUAGE.md", batql_render_surfaces(ops)),
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
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", '
    r'legacy_evidence: "(?:[^"\\]|\\.)*", clean_owner: "([^"]+)", '
    r'mechanism_disposition: "(?:[^"\\]|\\.)*", witness_requirement: '
    r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
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


def _arch_enum(root: Path, name: str, rel: str = "spec/architecture.rs") -> list[str]:
    src = (root / rel).read_text(encoding="utf-8")
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
    # The lane and result vocabularies moved to their typed owner (5.5E3f);
    # their facts and classifications are reconstructed by mutation_findings.
    for name, want, rel in (
            ("MutationLane", D3_LANES, "spec/mutation.rs"),
            ("MutationResult", D3_RESULTS, "spec/mutation.rs"),
            ("ProofPolicyChangeClass", D3_CHANGE_CLASSES, "spec/architecture.rs"),
            ("ProofPolicySurface", D3_POLICY_SURFACES, "spec/architecture.rs")):
        got = _arch_enum(root, name, rel)
        if not got:
            out.append(f"{rel} declares no {name}")
        elif got != want:
            out.append(f"{name} variants {got} != frozen {want}")
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
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
    spec/proof.rs is the one membership authority; docs/24 owns meaning only,
    and the retired fence headers are no longer parsed for anything."""
    src = (root / "spec/proof.rs").read_text(encoding="utf-8")
    out: dict[str, dict] = {}
    for wid, family, guarantee, _contracts in re.findall(
            r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
            r'ProofRowState::Active \{ guarantee: GuaranteeRef::(leg|dec)\("([^"]+)"\), '
            r'projection_contracts: &\[([^\]]*)\] \} \}', src):
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
    """(id, state, successors) in declaration order, from the typed catalog.
    Active rows carry guarantee and projection contracts since 5.5E4d; this
    lifecycle view records the state name and any successors."""
    src = (root / "spec/proof.rs").read_text(encoding="utf-8")
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


# --- 5.5E5 Gate-0 materializer output shape ----------------------------------
# spec/bootstrap_output.rs owns the candidate's file shape; the materializer
# and the selftest oracle both expand it. This auditor re-parses the typed
# owner independently and holds the STATIC posture laws: the dynamic isolated
# qualification (two output roots, seed snapshot, byte oracle, Cargo) lives in
# selftest, and audit never executes Cargo or materializes a tree.

A_BO_SPEC = "spec/bootstrap_output.rs"
A_BO_ENUMS = ("Gate0RootArtifact", "Gate0PackageArtifact", "Gate0PackageTargetKind",
              "Gate0PlaneArtifact", "Gate0OutputDisposition")


def bootstrap_output_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_BO_SPEC
    if not path.is_file():
        return [f"missing {A_BO_SPEC}"]
    raw = path.read_text(encoding="utf-8")
    src = _uncomment(raw)
    arch = _uncomment((root / "spec/architecture.rs").read_text(encoding="utf-8"))

    def enum_variants(text: str, enum: str) -> list[str]:
        # Indentation-robust brace match: the five closed enums live inside a
        # `gate0_closed_enum!` invocation (5.5E5a), so their variants sit two
        # indents deep and their `ALL` is macro-generated, not authored.
        head = text.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_BO_SPEC}: declares no {enum}")
            return []
        i = text.find("{", head)
        depth = 0
        body = ""
        for j in range(i, len(text)):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    body = text[i + 1: j]
                    break
        return re.findall(r"^\s+([A-Z]\w*),", body, re.M)

    def arm_pairs(text: str, fn: str, head_re: str, key: str) -> dict[str, str]:
        m = re.search(head_re, text, re.S)
        if not m:
            out.append(f"{A_BO_SPEC}: declares no {fn}()")
            return {}
        return dict(re.findall(key + r"::(\w+) => \"([^\"]*)\"", m.group(1)))

    # The five closed vocabularies are each declared through the one
    # `gate0_closed_enum!` primitive, which emits the enum AND its `ALL`
    # inventory from a single variant list (5.5E5a). Enum-to-`ALL` divergence
    # is therefore unrepresentable, so there is no authored-vs-listed
    # comparison and no cardinality here: the auditor verifies the CONSTRUCTION
    # instead. A hand-written `pub const ALL` for one of these enums is the
    # divergence surface the macro exists to close, so it is a finding.
    macro_enums = set(re.findall(
        r"gate0_closed_enum!\s*\{[^{]*?pub enum (\w+)", src))
    inventories: dict[str, list[str]] = {}
    for enum in A_BO_ENUMS:
        authored = enum_variants(src, enum)
        inventories[enum] = authored
        if enum not in macro_enums:
            out.append(f"{A_BO_SPEC}: {enum} is not declared through gate0_closed_enum!; "
                       "its inventory could diverge from the enum")
        if re.search(r"pub const ALL: &'static \[" + enum + r"\]", src):
            out.append(f"{A_BO_SPEC}: {enum} carries a hand-written ALL; the inventory "
                       "must come from gate0_closed_enum! so it cannot diverge")
        if len(set(authored)) != len(authored):
            out.append(f"{A_BO_SPEC}: {enum} lists a variant twice")

    # Root artifact paths: total, nonempty, unique; the RustToolchain artifact
    # must project the tracked root toolchain path.
    root_paths = arm_pairs(
        src, "relative_path",
        r"pub const fn relative_path\(self\)[^{]*\{(.*?)\n    \}", "Gate0RootArtifact")
    for name in inventories.get("Gate0RootArtifact", []):
        path_str = root_paths.get(name, "")
        if not path_str.strip():
            out.append(f"{A_BO_SPEC}: root artifact {name} projects no path")
        elif path_str.startswith("/") or ".." in path_str.split("/"):
            out.append(f"{A_BO_SPEC}: root artifact {name} path {path_str!r} is not "
                       "output-root-relative and traversal-free")
    if len(set(root_paths.values())) != len(root_paths):
        out.append(f"{A_BO_SPEC}: two root artifacts project one path")
    if root_paths.get("RustToolchain", "rust-toolchain.toml") != "rust-toolchain.toml":
        out.append(f"{A_BO_SPEC}: RustToolchain does not project the tracked root toolchain path")

    # Target kinds: every PackageId arm explicit, no wildcard, no class fallback.
    packages = enum_variants(arch, "PackageId")
    tk = re.search(r"pub const fn target_kind\(package: PackageId\)[^{]*\{(.*?)\n\}",
                   src, re.S)
    kinds: dict[str, str] = {}
    if not tk:
        out.append(f"{A_BO_SPEC}: declares no target_kind()")
    else:
        body = tk.group(1)
        kinds = dict(re.findall(r"PackageId::(\w+) => Gate0PackageTargetKind::(\w+)", body))
        if re.search(r"\n\s*_\s*=>", body) or "PackageClass" in body:
            out.append(f"{A_BO_SPEC}: target_kind() carries a wildcard or class fallback; "
                       "a future package must be classified explicitly")
        for package in packages:
            if package not in kinds:
                out.append(f"{A_BO_SPEC}: package {package} has no explicit target kind")
        for package, kind in kinds.items():
            if package not in packages:
                out.append(f"{A_BO_SPEC}: target_kind() names unknown package {package}")
            if kind not in inventories.get("Gate0PackageTargetKind", []):
                out.append(f"{A_BO_SPEC}: target_kind() names unknown kind {kind}")

    # Binary targets: named exactly where the kind requires one, nonempty, unique.
    bt = re.search(r"pub const fn binary_target\(package: PackageId\)[^{]*\{(.*?)\n\}",
                   src, re.S)
    named: dict[str, str] = {}
    if not bt:
        out.append(f"{A_BO_SPEC}: declares no binary_target()")
    else:
        named = dict(re.findall(r"PackageId::(\w+) => Some\(\"([^\"]+)\"\)", bt.group(1)))
        want = {p for p, k in kinds.items() if k in ("Binary", "ExampleBinary")}
        for package in sorted(want - set(named)):
            out.append(f"{A_BO_SPEC}: binary-kind package {package} names no binary target")
        for package in sorted(set(named) - want):
            out.append(f"{A_BO_SPEC}: library-kind package {package} names a binary target")
        if len(set(named.values())) != len(named):
            out.append(f"{A_BO_SPEC}: two packages claim one binary target name")

    # Source doors: every authored kind projects a nonempty door.
    door = re.search(r"pub const fn source_suffix\(package: PackageId\)[^{]*\{(.*?)\n\}",
                     src, re.S)
    doors: dict[str, str] = {}
    if not door:
        out.append(f"{A_BO_SPEC}: declares no source_suffix()")
    else:
        for pattern, suffix in re.findall(
                r"((?:Gate0PackageTargetKind::\w+\s*\|?\s*)+)=> \"([^\"]+)\"",
                door.group(1)):
            for kind in re.findall(r"Gate0PackageTargetKind::(\w+)", pattern):
                doors[kind] = suffix
        for kind in inventories.get("Gate0PackageTargetKind", []):
            if not doors.get(kind, "").strip():
                out.append(f"{A_BO_SPEC}: target kind {kind} projects no source door")

    # Plane modules: total over SyncBatPlane, nonempty, unique.
    modules = arm_pairs(
        arch, "module_name",
        r"pub const fn module_name\(self\)[^{]*\{(.*?)\n    \}", "SyncBatPlane")
    planes = enum_variants(arch, "SyncBatPlane")
    for plane in planes:
        if not modules.get(plane, "").strip():
            out.append(f"{A_BO_SPEC}: SyncBat plane {plane} projects no module name")
    if len(set(modules.values())) != len(modules):
        out.append(f"{A_BO_SPEC}: two SyncBat planes project one module name")

    # Workspace metadata is owned here and nonempty.
    # Parsed from the RAW text: _uncomment would truncate the // in a URL.
    for const in ("WORKSPACE_LICENSE", "WORKSPACE_REPOSITORY"):
        m = re.search(r"pub const " + const + r": &str = \"([^\"]*)\";", raw)
        if not m or not m.group(1).strip():
            out.append(f"{A_BO_SPEC}: {const} is missing or empty")

    # The docs/23 plan block equals this auditor's own expansion, path for
    # path, in canonical order: roots, packages, planes.
    expected_paths: list[str] = []
    for name in inventories.get("Gate0RootArtifact", []):
        expected_paths.append(root_paths.get(name, ""))
    package_paths = arm_pairs(
        arch, "workspace_path",
        r"pub const fn workspace_path\(self\)[^{]*\{(.*?)\n    \}", "PackageId")
    package_all = re.search(r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];",
                            arch, re.S)
    package_order = re.findall(r"PackageId::(\w+)", package_all.group(1)) if package_all else []
    for package in package_order:
        base = package_paths.get(package, "")
        door = doors.get(kinds.get(package, ""), "")
        for suffix in ("Cargo.toml", "README.md", door):
            expected_paths.append(f"{base}/{suffix}" if base and suffix else "")
    plane_all = re.search(r"pub const ALL: &'static \[SyncBatPlane\] = &\[(.*?)\];",
                          arch, re.S)
    plane_order = re.findall(r"SyncBatPlane::(\w+)", plane_all.group(1)) if plane_all else []
    syncbat_base = package_paths.get("SyncBat", "")
    for plane in plane_order:
        module = modules.get(plane, "")
        expected_paths.append(f"{syncbat_base}/src/{module}.rs")
        expected_paths.append(f"{syncbat_base}/src/{module}/README.md")
    doc23 = root / "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md"
    if doc23.is_file():
        text = doc23.read_text(encoding="utf-8")
        m = re.search(r"<!-- GATE0-MATERIALIZATION-PLAN:BEGIN[^>]*-->\n(.*?)\n"
                      r"<!-- GATE0-MATERIALIZATION-PLAN:END -->", text, re.S)
        if not m:
            out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: carries no "
                       "GATE0-MATERIALIZATION-PLAN block")
        else:
            got_paths = [line.split("|")[1].strip()
                         for line in m.group(1).splitlines()
                         if line.startswith("|") and not line.startswith("| ---")
                         and not line.startswith("| Path")]
            if got_paths != expected_paths:
                out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: the "
                           "GATE0-MATERIALIZATION-PLAN block does not match its "
                           "typed derivation")
    else:
        out.append("missing docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")

    # Every expanded plan path obeys one portable-path law, independently
    # reconstructed here (5.5E5a; rebuilt as a POSITIVE grammar in 5.5E6b). The
    # old negative filter admitted everything nobody forbade — a non-ASCII
    # component and the Windows-invalid set `< > " | ? *` slipped through. This
    # reconstruction spells out what a component MAY contain (ASCII
    # `[A-Za-z0-9._-]` only), so a byte that cannot be spelled cannot smuggle a
    # backslash separator, a drive colon, a wildcard, or a homoglyph.
    def portable_component(component: str) -> bool:
        if component in ("", ".", ".."):
            return False
        if not all(
            ("a" <= c <= "z") or ("A" <= c <= "Z") or ("0" <= c <= "9")
            or c in "._-"
            for c in component
        ):
            return False
        if component.endswith("."):
            return False
        stem = component.split(".")[0].upper()
        if stem in ("CON", "PRN", "AUX", "NUL"):
            return False
        if (len(stem) == 4 and stem[:3] in ("COM", "LPT")
                and stem[3].isdigit() and stem[3] != "0"):
            return False
        return True

    def portable(path: str) -> bool:
        if not path or path.startswith("/"):
            return False
        return all(portable_component(c) for c in path.split("/"))

    folded: dict[str, str] = {}
    for path in expected_paths:
        if not portable(path):
            out.append(f"{A_BO_SPEC}: expanded plan path {path!r} is not a portable "
                       "relative path")
        low = path.lower()
        if low in folded and folded[low] != path:
            out.append(f"{A_BO_SPEC}: plan paths {folded[low]!r} and {path!r} collide "
                       "under case folding")
        folded[low] = path

    # The typed path law is the one law: the materializer and seedcheck must
    # USE it, not carry their own slash-only reimplementation, and no
    # cardinality assertion may return in seedcheck.
    if "pub fn is_portable_gate0_relative_path" not in raw:
        out.append(f"{A_BO_SPEC}: the portable-path law is missing")
    sc_path = root / "bootstrap/seedcheck.rs"
    if sc_path.is_file():
        sc = sc_path.read_text(encoding="utf-8")
        if "is_portable_gate0_relative_path" not in sc:
            out.append("bootstrap/seedcheck.rs: does not use the typed portable-path law")
        if re.search(r"Gate0\w+::ALL\.len\(\)\s*!=\s*\d", sc):
            out.append("bootstrap/seedcheck.rs: a Gate0 ALL cardinality assertion "
                       "returned; the inventory is count-free by construction")

    # The parallel plane inventories cannot return.
    for token in ("SYNCBAT_REQUIRED_PLANES", "pub const SYNCBAT_PLANES"):
        if token in arch:
            out.append(f"spec/architecture.rs: the raw plane inventory {token.split()[-1]} "
                       "returned; SyncBatPlane::ALL is the one inventory")

    # Static materializer posture laws.
    mat_path = root / "bootstrap/materialize.rs"
    if not mat_path.is_file():
        out.append("missing bootstrap/materialize.rs")
    else:
        mat = mat_path.read_text(encoding="utf-8")
        if 'PathBuf::from(".")' in mat or "args().nth(1)" in mat:
            out.append("bootstrap/materialize.rs: an implicit current-directory or "
                       "one-root invocation route exists; both roots must be explicit")
        for flag in ("--seed", "--output"):
            if f'"{flag}"' not in mat:
                out.append(f"bootstrap/materialize.rs: the explicit {flag} boundary is gone")
        if "write_checked" in mat:
            out.append("bootstrap/materialize.rs: the per-file overwrite-or-repair "
                       "route returned; publication is a staged whole tree")
        if "fs::rename(" not in mat:
            out.append("bootstrap/materialize.rs: publication does not rename a "
                       "complete staged tree into place")
        if "plane_roles" in mat:
            out.append("bootstrap/materialize.rs: a materializer-local plane role "
                       "map returned; authored_ownership() is the one description")
        if re.search(r'"browser(?:-storage)?"', mat):
            out.append("bootstrap/materialize.rs: a handwritten qualification-feature "
                       "name returned; features derive from the profile ledger")
        if "is_portable_gate0_relative_path" not in mat:
            out.append("bootstrap/materialize.rs: does not use the typed portable-path "
                       "law; a hand-rolled path check can drift from the owner")
        # The candidate justfile is internally executable: it may not advertise
        # seed-only tools it does not contain.
        jf = re.search(r"fn justfile\(\)[^{]*\{(.*?)\n\}", mat, re.S)
        if jf and re.search(r"audit\.py|freeze\.py|seedcheck\.rs", jf.group(1)):
            out.append("bootstrap/materialize.rs: the candidate justfile references a "
                       "seed-only tool the candidate does not contain")
        # The binary is bound to the seed manifest it was compiled from.
        if "COMPILED_SEED_MANIFEST" not in mat or "include_str!" not in mat:
            out.append("bootstrap/materialize.rs: the binary is not bound to its "
                       "compiled SPEC.sha256 seed manifest")

    # The dedicated isolated qualification exists and the generic binary route
    # no longer carries tier0-materialize.
    st_path = root / "bootstrap/selftest.py"
    if st_path.is_file():
        st = st_path.read_text(encoding="utf-8")
        if "def qualify_materializer" not in st:
            out.append("bootstrap/selftest.py: no dedicated isolated materializer "
                       "qualification exists")
        if re.search(r'\("tier0-materialize",\s*"bootstrap/materialize\.rs"', st):
            out.append("bootstrap/selftest.py: tier0-materialize still rides the "
                       "generic one-root binary qualification")
        oracle = re.search(r"\ndef oracle_plan\(.*?(?=\ndef )", st, re.S)
        if oracle:
            block = oracle.group(0)
            cargo_names = re.findall(r'PackageId::\w+ => "([^"]+)"',
                                     re.search(r"pub const fn cargo_name\(self\)[^{]*\{(.*?)\n    \}",
                                               arch, re.S).group(1))
            for name in cargo_names:
                if f'"{name}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"package name {name!r}; names derive from spec/architecture.rs")
            for module in modules.values():
                if f'"{module}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"plane module name {module!r}; modules derive from the typed inventory")
            wasm_profiles = [prof for _, prof, env in re.findall(
                r'package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
                r"environment: QualificationEnvironment::(\w+)", arch) if env == "WasmHost"]
            for prof in wasm_profiles:
                if f'"{prof}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"feature name {prof!r}; features derive from the profile ledger")
    return out


# --- 5.5E6a typed Tier 0 receipt policy -------------------------------------
A_BQ_SPEC = "spec/bootstrap_qualification.rs"


def bootstrap_qualification_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_BQ_SPEC
    if not path.is_file():
        return [f"missing {A_BQ_SPEC}"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    tc = _uncomment((root / "spec/toolchain.rs").read_text(encoding="utf-8"))

    def variants(text: str, enum: str) -> list[str]:
        head = text.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_BQ_SPEC}: declares no {enum}")
            return []
        return re.findall(r"^\s{4}(\w+),$", text[head: text.find("\n}", head)], re.M)

    def all_of(text: str, enum: str) -> list[str]:
        m = re.search(r"pub const ALL: &'static \[" + enum + r"\] = &\[(.*?)\];", text, re.S)
        return re.findall(enum + r"::(\w+)", m.group(1)) if m else []

    # Closed vocabularies equal their ALL inventory, both directions.
    for text, enum in ((src, "Tier0ReceiptKind"), (src, "Tier0ArtifactPolicy"),
                       (tc, "RustTargetTriple")):
        authored = variants(text, enum)
        listed = all_of(text, enum)
        for name in authored:
            if name not in listed:
                out.append(f"{enum}::{name} is authored but ALL omits it")
        for name in listed:
            if name not in authored:
                out.append(f"{enum}::ALL lists {name}, which the enum does not author")
        if not listed:
            out.append(f"{enum}::ALL is empty")

    # Every Tier0ReceiptKind has a nonempty unique slug and a total policy arm.
    kinds = all_of(src, "Tier0ReceiptKind")
    slugs = dict(re.findall(r'Tier0ReceiptKind::(\w+) => "([^"]+)"', src))
    policy = dict(re.findall(r"Tier0ReceiptKind::(\w+) => Tier0ArtifactPolicy::(\w+)", src))
    seen: set[str] = set()
    for kind in kinds:
        s = slugs.get(kind, "")
        if not s:
            out.append(f"{A_BQ_SPEC}: {kind} projects no slug")
        elif s in seen:
            out.append(f"{A_BQ_SPEC}: slug {s} is claimed twice")
        seen.add(s)
        if kind not in policy:
            out.append(f"{A_BQ_SPEC}: {kind} names no artifact policy")

    # The authoritative target is the MSVC triple, distinct from the semantic
    # QualificationEnvironment.
    auth = re.search(r"pub const AUTHORITATIVE_TARGET: RustTargetTriple = "
                     r"RustTargetTriple::(\w+);", tc)
    if not auth or auth.group(1) != "X86_64PcWindowsMsvc":
        out.append("spec/toolchain.rs: AUTHORITATIVE_TARGET is not the MSVC triple")
    if "QualificationEnvironment" in src:
        out.append(f"{A_BQ_SPEC}: speaks QualificationEnvironment; a semantic environment "
                   "is not a physical target")

    # The denominator moved out of Python: selftest DERIVES it, never authors a
    # literal slug tuple, and the product ReceiptId cannot substitute.
    st = (root / "bootstrap/selftest.py").read_text(encoding="utf-8")
    # 5.5E6b: the Tier 0 admission authority left Python. selftest holds no
    # hand-authored denominator and no Python pass-predicate; it produces
    # concrete evidence and delegates the verdict to the independent verifier
    # bootstrap/receiptcheck.rs. A reintroduced slug tuple, or a selftest that
    # no longer delegates, is refused here.
    if re.search(r'REQUIRED_TIER0_RECEIPTS = \(\s*\("tier0-', st):
        out.append("bootstrap/selftest.py: REQUIRED_TIER0_RECEIPTS is a hand-authored "
                   "slug tuple; Tier0ReceiptKind::ALL is the one denominator")
    if "verify_tier0_evidence(" not in st:
        out.append("bootstrap/selftest.py: does not delegate Tier 0 qualification to "
                   "the independent verifier bootstrap/receiptcheck.rs")
    if "ReceiptId" in src:
        out.append(f"{A_BQ_SPEC}: reuses the product ReceiptId; Tier0ReceiptKind is the "
                   "bootstrap receipt identity")

    # The Delivery Notes denominator block equals the typed projection.
    dn = (root / "DELIVERY_NOTES.md")
    if dn.is_file():
        m = re.search(r"<!-- TIER0-RECEIPT-DENOMINATOR:BEGIN[^>]*-->\n(.*?)\n"
                      r"<!-- TIER0-RECEIPT-DENOMINATOR:END -->",
                      dn.read_text(encoding="utf-8"), re.S)
        if not m:
            out.append("DELIVERY_NOTES.md: carries no TIER0-RECEIPT-DENOMINATOR block")
        else:
            rows = [tuple(c.strip() for c in line.strip("|").split("|"))
                    for line in m.group(1).splitlines()
                    if line.startswith("|") and not line.startswith("| ---")
                    and not line.startswith("| Receipt")]
            want = [(slugs[k], policy[k]) for k in kinds]
            if rows != want:
                out.append("DELIVERY_NOTES.md: the Tier 0 denominator block does not "
                           "match its typed derivation")
    else:
        out.append("missing DELIVERY_NOTES.md")

    # === E6b: the evidence-verification algebra is internally coherent ======
    # Independent reconstruction (regex, no shared parse with the spec crate):
    # the rule inventory equals its ALL both directions; every rule carries an
    # owner, a law, and a repair; ownership resolves only to the three declared
    # owners; the artifact-evidence sum matches the policy vocabulary exactly;
    # and the concrete-digest and refusal primitives are present. This proves
    # the SHAPES are admissible; bootstrap/receiptcheck.rs proves the concrete
    # evidence matches them, and seedcheck executes verify() against each rule.
    rule_enum = "Tier0VerificationRule"
    rule_authored = variants(src, rule_enum)
    rule_listed = all_of(src, rule_enum)
    for name in rule_authored:
        if name not in rule_listed:
            out.append(f"{rule_enum}::{name} is authored but ALL omits it")
    for name in rule_listed:
        if name not in rule_authored:
            out.append(f"{rule_enum}::ALL lists {name}, which the enum does not author")
    if not rule_listed:
        out.append(f"{rule_enum}::ALL is empty")

    def arm_body(fn_sig: str) -> str:
        m = re.search(re.escape(fn_sig) + r"\s*\{(.*?)\n    \}", src, re.S)
        return m.group(1) if m else ""

    owner_body = arm_body("pub const fn owner(self) -> ContractId")
    law_body = arm_body("pub const fn law(self) -> &'static str")
    repair_body = arm_body("pub const fn repair(self) -> &'static str")
    for label, body in (("owner", owner_body), ("law", law_body),
                        ("repair", repair_body)):
        if not body:
            out.append(f"{A_BQ_SPEC}: {rule_enum} declares no {label}() arm")
    for name in rule_authored:
        token = f"{rule_enum}::{name}"
        if token not in owner_body:
            out.append(f"{A_BQ_SPEC}: rule {name} names no owning contract")
        if f"{token} =>" not in law_body:
            out.append(f"{A_BQ_SPEC}: rule {name} states no law")
        if f"{token} =>" not in repair_body:
            out.append(f"{A_BQ_SPEC}: rule {name} states no repair")
    # Every law and repair arm is a nonempty string literal.
    for label, body in (("law", law_body), ("repair", repair_body)):
        for name, text in re.findall(
                rule_enum + r"::(\w+) =>\s*\"((?:[^\"\\]|\\.)*)\"", body):
            if not text.strip():
                out.append(f"{A_BQ_SPEC}: rule {name} {label} is empty")

    # The three owners, and only those three, resolve rule ownership; each is
    # declared with its authoritative contract id.
    owner_decl = dict(re.findall(
        r'pub const (TIER0_\w+_OWNER): ContractId =\s*ContractId\("([^"]+)"\);', src))
    expected_owners = {
        "TIER0_RECEIPT_ALGEBRA_OWNER": "BP-RECEIPTS-1",
        "TIER0_DENOMINATOR_OWNER": "BP-BOOTSTRAP-1",
        "TIER0_HOSTED_QUALIFICATION_OWNER": "BP-PUBLIC-API-CI-RELEASE-1",
    }
    if owner_decl != expected_owners:
        out.append(f"{A_BQ_SPEC}: the three Tier 0 ownership constants are not "
                   "declared with their authoritative contract ids")
    owners_used = set(re.findall(r"TIER0_\w+_OWNER", owner_body))
    for used in owners_used:
        if used not in expected_owners:
            out.append(f"{A_BQ_SPEC}: owner() resolves to {used}, not a declared "
                       "Tier 0 ownership constant")
    for declared in expected_owners:
        if declared not in owners_used:
            out.append(f"{A_BQ_SPEC}: ownership constant {declared} owns no rule")

    # The artifact-evidence sum matches the policy vocabulary one-to-one, and
    # each evidence variant reports the same-named policy.
    ev_head = src.find("pub enum Tier0ArtifactEvidence {")
    ev_body = src[ev_head: src.find("\n}", ev_head)] if ev_head >= 0 else ""
    ev_variants = set(re.findall(r"^\s{4}(\w+)\s*(?:\{|,)", ev_body, re.M))
    policy_variants = set(variants(src, "Tier0ArtifactPolicy"))
    if not ev_variants or ev_variants != policy_variants:
        out.append(f"{A_BQ_SPEC}: Tier0ArtifactEvidence variants do not match the "
                   "Tier0ArtifactPolicy vocabulary")
    ev_policy = re.findall(
        r"Tier0ArtifactEvidence::(\w+) \{ \.\. \} => \{?\s*Tier0ArtifactPolicy::(\w+)",
        src)
    for variant, mapped in ev_policy:
        if variant != mapped:
            out.append(f"{A_BQ_SPEC}: evidence {variant} reports policy {mapped}")
    if {v for v, _ in ev_policy} != ev_variants:
        out.append(f"{A_BQ_SPEC}: Tier0ArtifactEvidence::policy() is not total")

    # A slug resolves back to its kind, and the refusal error names its rule.
    if "pub fn from_slug(" not in src:
        out.append(f"{A_BQ_SPEC}: Tier0ReceiptKind has no from_slug resolver")
    err_head = src.find("pub enum Tier0VerificationError {")
    err_body = src[err_head: src.find("\n}", err_head)] if err_head >= 0 else ""
    for arm in ("Receipt", "Denominator", "Qualification"):
        if f"{arm} {{" not in err_body:
            out.append(f"{A_BQ_SPEC}: Tier0VerificationError omits its {arm} arm")
    if "pub const fn rule(self) -> Tier0VerificationRule" not in src:
        out.append(f"{A_BQ_SPEC}: Tier0VerificationError projects no rule()")

    # The concrete-digest and hex-parse primitives are present and
    # lowercase-strict: a Tier 0 evidence digest is its own bootstrap type.
    for prim in ("Sha256Digest", "GitCommitSha", "GitTreeSha", "ToolchainCommit"):
        impl = re.search(r"impl " + prim + r" \{(.*?)\n\}", src, re.S)
        body = impl.group(1) if impl else ""
        if "fn from_hex(" not in body or "fn render(" not in body:
            out.append(f"{A_BQ_SPEC}: {prim} is not a from_hex/render digest primitive")
    hex_err = set(variants(src, "HexParseError"))
    if hex_err != {"WrongLength", "NonLowerHexDigit"}:
        out.append(f"{A_BQ_SPEC}: HexParseError does not distinguish a wrong length "
                   "from a non-lowercase-hex digit")

    # receiptcheck.rs is the authoritative independent computer, and it is wired
    # into the required-file denominator so it can never silently vanish.
    if "bootstrap/receiptcheck.rs" not in \
            (root / "spec/architecture.rs").read_text(encoding="utf-8"):
        out.append("spec/architecture.rs: REQUIRED_DOCS omits bootstrap/receiptcheck.rs")
    return out


A_XR_SPEC = "spec/tier0_cross_run.rs"


def tier0_cross_run_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_XR_SPEC
    if not path.is_file():
        return [f"missing {A_XR_SPEC}"]
    src = _uncomment(path.read_text(encoding="utf-8"))

    # Independent reconstruction (regex, no shared parse with the spec crate) of
    # the cross-run same-source comparator (5.5E6c). The verdict is EXECUTED by
    # seedcheck (compare_runs over verified qualifications); here we prove the
    # SHAPE is admissible: the outcomes, the preconditions, and — the load-
    # bearing property — that the source-identity coordinates are exactly the
    # deterministic ones and NO compiled-artifact digest is treated as a source
    # coordinate.
    def variants(enum: str) -> set[str]:
        head = src.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_XR_SPEC}: declares no {enum}")
            return set()
        body = src[head: src.find("\n}", head)]
        return set(re.findall(r"^\s{4}(\w+)\s*(?:\{|,|\()", body, re.M))

    # The comparison result names exactly its three outcomes.
    if variants("CrossRunComparison") != {"NotComparable", "DifferentSource", "SameSource"}:
        out.append(f"{A_XR_SPEC}: CrossRunComparison is not the three-outcome result "
                   "(NotComparable | DifferentSource | SameSource)")

    # Non-comparability names exactly its three precondition failures.
    if variants("NotComparable") != {"FrozenExportSource", "MissingHostedRun", "SameHostedRun"}:
        out.append(f"{A_XR_SPEC}: NotComparable does not name exactly the frozen-export, "
                   "missing-hosted-run, and same-hosted-run preconditions")

    # The source-identity coordinates are EXACTLY the five deterministic ones;
    # an executable/artifact coordinate here would be the dishonest-red mistake.
    expected_div = {"SourceCommit", "SourceTree", "SpecManifestDigest",
                    "WorkflowDigest", "MaterializerOutputTree"}
    div = variants("SourceDivergence")
    if div != expected_div:
        out.append(f"{A_XR_SPEC}: SourceDivergence is not exactly the five source-identity "
                   "coordinates (commit, tree, spec-manifest, workflow, materializer-output-tree)")
    if any(("xecutable" in v) or ("Artifact" in v) for v in div):
        out.append(f"{A_XR_SPEC}: SourceDivergence treats a compiled-artifact digest as a "
                   "source coordinate; executable digests legitimately differ across runs")

    # Side and the two agreement classifications.
    if variants("Side") != {"Left", "Right", "Both"}:
        out.append(f"{A_XR_SPEC}: Side is not Left/Right/Both")
    if variants("ToolchainAgreement") != {"Identical", "Divergent"}:
        out.append(f"{A_XR_SPEC}: ToolchainAgreement is not Identical/Divergent")
    if variants("TargetAgreement") != {"SameTarget", "CrossTarget"}:
        out.append(f"{A_XR_SPEC}: TargetAgreement is not SameTarget/CrossTarget")

    # compare_runs actually EMITS every declared outcome — no coordinate is
    # declared but never checked, and no precondition is unreachable — consumes
    # the load-bearing materializer output tree, and inspects no executable digest.
    fn_head = src.find("pub fn compare_runs(")
    # Bound compare_runs at the next top-level item (confirm_promotion) so the
    # executable-digest scan below does not reach into confirm_promotion or the
    # in-crate test module.
    fn_end = src.find("\npub fn confirm_promotion(", fn_head) if fn_head >= 0 else -1
    if fn_end < 0:
        fn_end = src.find("\n#[cfg(test)]", fn_head) if fn_head >= 0 else -1
    fn_body = src[fn_head: fn_end if fn_end >= 0 else len(src)] if fn_head >= 0 else ""
    if not fn_body:
        out.append(f"{A_XR_SPEC}: declares no compare_runs")
    else:
        for variant in sorted(expected_div):
            if f"SourceDivergence::{variant}" not in fn_body:
                out.append(f"{A_XR_SPEC}: compare_runs never emits the {variant} divergence")
        for variant in ("FrozenExportSource", "MissingHostedRun", "SameHostedRun"):
            if f"NotComparable::{variant}" not in fn_body:
                out.append(f"{A_XR_SPEC}: compare_runs never emits NotComparable::{variant}")
        if "materializer_output_tree" not in fn_body:
            out.append(f"{A_XR_SPEC}: compare_runs ignores the materializer output tree")
        if "executable_digest" in fn_body:
            out.append(f"{A_XR_SPEC}: compare_runs inspects an executable digest; compiled "
                       "artifacts legitimately differ across independent runs")

    # The proof is sealed (compare_runs is its sole constructor) and retains the
    # proven-common coordinates plus the two attesting run identities.
    proof_head = src.find("pub struct SameSourceProof {")
    proof_body = src[proof_head: src.find("\n}", proof_head)] if proof_head >= 0 else ""
    if "_seal: ()" not in proof_body:
        out.append(f"{A_XR_SPEC}: SameSourceProof is not sealed; compare_runs is not its "
                   "sole constructor")
    for field in ("commit", "tree", "spec_manifest_digest", "workflow_digest",
                  "materializer_output_tree", "left_run", "right_run"):
        if f"{field}:" not in proof_body:
            out.append(f"{A_XR_SPEC}: SameSourceProof retains no {field}")

    # Promotion confirmation (5.5E6c1) is STRICTLY STRONGER than same-source. Its
    # error algebra names exactly the posture requirements it adds over
    # same-source; its sealed proof retains the confirmed authority.
    expected_pc_err = {"NotSameSource", "NonAuthoritativeTarget", "CrossTarget",
                       "ToolchainDivergent", "RepositoryMismatch",
                       "NonCanonicalWorkflowPath"}
    if variants("PromotionConfirmationError") != expected_pc_err:
        out.append(f"{A_XR_SPEC}: PromotionConfirmationError is not exactly the six promotion "
                   "posture refusals (not-same-source, non-authoritative-target, cross-target, "
                   "toolchain-divergent, repository-mismatch, non-canonical-workflow-path)")

    pc_head = src.find("pub struct PromotionConfirmationProof {")
    pc_body = src[pc_head: src.find("\n}", pc_head)] if pc_head >= 0 else ""
    if "_seal: ()" not in pc_body:
        out.append(f"{A_XR_SPEC}: PromotionConfirmationProof is not sealed; confirm_promotion is "
                   "not its sole constructor")
    for field in ("same_source", "target", "toolchain", "candidate_run", "cleanroom_run"):
        if f"{field}:" not in pc_body:
            out.append(f"{A_XR_SPEC}: PromotionConfirmationProof retains no {field}")

    # confirm_promotion builds on the same-source proof, refuses on every added
    # posture requirement, requires the authoritative target, and pins the
    # canonical authoritative workflow — a promotion cannot be confirmed from a
    # branch name or a supplemental target.
    cp_head = src.find("pub fn confirm_promotion(")
    cp_end = src.find("\n#[cfg(test)]", cp_head) if cp_head >= 0 else -1
    cp_body = src[cp_head: cp_end if cp_end >= 0 else len(src)] if cp_head >= 0 else ""
    if not cp_body:
        out.append(f"{A_XR_SPEC}: declares no confirm_promotion")
    else:
        for variant in sorted(expected_pc_err):
            if f"PromotionConfirmationError::{variant}" not in cp_body:
                out.append(f"{A_XR_SPEC}: confirm_promotion never emits the {variant} refusal")
        if "compare_runs(" not in cp_body or "SameSource" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not build on the same-source proof")
        if "is_authoritative()" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not require the authoritative target")
        if "AUTHORITATIVE_WORKFLOW_PATH" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not pin the canonical authoritative "
                       "workflow path")

    # The retained binding the comparator consumes is exposed by the sealed E6b
    # qualification, the canonical workflow path is owned there, and seedcheck
    # executes both the comparator and the promotion confirmation end to end.
    bq = _uncomment((root / "spec/bootstrap_qualification.rs").read_text(encoding="utf-8"))
    if "fn materializer_output_tree(&self) -> Sha256Digest" not in bq:
        out.append("spec/bootstrap_qualification.rs: VerifiedTier0Qualification exposes no "
                   "materializer_output_tree() accessor for the cross-run comparator")
    if "AUTHORITATIVE_WORKFLOW_PATH" not in bq:
        out.append("spec/bootstrap_qualification.rs: declares no AUTHORITATIVE_WORKFLOW_PATH "
                   "constant for promotion confirmation")
    sc = (root / "bootstrap/seedcheck.rs").read_text(encoding="utf-8")
    if "check_tier0_cross_run" not in sc or "compare_runs(" not in sc:
        out.append("bootstrap/seedcheck.rs: does not execute the cross-run comparator")
    if "confirm_promotion(" not in sc:
        out.append("bootstrap/seedcheck.rs: does not execute the promotion confirmation")
    return out


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
    findings.extend(generated_view_findings(root))
    findings.extend(inventory_mirror_findings(root))
    findings.extend(exact_ledger_findings(root))
    findings.extend(proof_relation_findings(root))
    findings.extend(identity_catalog_findings(root))
    findings.extend(command_catalog_findings(root))
    findings.extend(mutation_findings(root))
    findings.extend(corpus_findings(root))
    findings.extend(compiler_assumption_findings(root))
    findings.extend(promotion_findings(root))
    findings.extend(pakvm_isa_findings(root))
    findings.extend(recon_findings(root))
    findings.extend(release_seal_findings(root))
    findings.extend(proof_terminal_findings(root))
    findings.extend(syncbat_firewall_findings(root))
    findings.extend(bootstrap_output_findings(root))
    findings.extend(bootstrap_qualification_findings(root))
    findings.extend(tier0_cross_run_findings(root))
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
    # The tracked root projection opens with its machine-readable provenance
    # line (5.5E4a): the comment is part of the exact projected bytes.
    want = ('# generated from spec/toolchain.rs by bootstrap/project.py; do not edit\n'
            '[toolchain]\nchannel = "{}"\nprofile = "{}"\ncomponents = [{}]\n'
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


# --- Generated-view registry (5.5E4a) ----------------------------------------
# spec/generated_views.rs is the closed denominator of every repository-
# generated view. This auditor parses the typed registry INDEPENDENTLY of
# bootstrap/project.py (never importing it or executing its parser), scans
# every tracked Markdown document for generated markers, and proves true set
# equality between registered embedded target instances and actual ones —
# plus provenance, pairing, dispatcher parity, and the projector's own
# documentary honesty. After 5.5E4a an unregistered generated block cannot
# hide in the wallpaper.
A_GENERATED_VIEWS_SRC = "spec/generated_views.rs"
A_VIEW_ARM = re.compile(
    r"GeneratedView::(\w+) => GeneratedViewSpec \{"
    r"(.*?)generator: BootstrapToolId::(\w+),", re.S)
A_VIEW_SURFACES = {"EmbeddedBlock", "StandaloneFile", "CorpusFrontmatter"}
A_MARKER_BEGIN = re.compile(r"<!-- ([A-Z0-9][A-Z0-9-]*):BEGIN([^>]*)-->")
A_MARKER_END = re.compile(r"<!-- ([A-Z0-9][A-Z0-9-]*):END -->")
# Multiple authority sources are joined in registry order by "; " — source
# order is provenance, and reordering without changing the set is drift.
A_MARKER_PROVENANCE = re.compile(r"\Agenerated from (.+?) by (\S+); do not edit\Z")
# Closed authored-fence allowlist: marker-shaped fences that are AUTHORED
# document structure, not generated views. HISTORICAL-MIGRATION is the
# docs/24 quarantine fence this auditor itself consumes for retired-ID scope.
A_AUTHORED_FENCES = {"HISTORICAL-MIGRATION"}


def a_parse_generated_views(root: Path) -> dict:
    """The auditor's own registry parse. Shares no code with the generator."""
    path = root / A_GENERATED_VIEWS_SRC
    if not path.is_file():
        return {"variants": [], "order": [], "views": {}}
    src = path.read_text(encoding="utf-8")
    enum_body = re.search(r"pub enum GeneratedView \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"\n    (\w+),", _uncomment(enum_body.group(1))) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[GeneratedView\] = &\[(.*?)\];", src, re.S)
    order = re.findall(r"GeneratedView::(\w+)", all_body.group(1)) if all_body else []
    views: dict[str, dict] = {}
    for name, body, generator in A_VIEW_ARM.findall(src):
        sources_m = re.search(r"authority_sources: &\[([^\]]*)\]", body)
        static_m = re.search(r"target: GeneratedViewTarget::Static\(&\[([^\]]*)\]\)", body)
        corpus = "GeneratedViewTarget::EligibleMarkdownCorpus" in body
        surface_m = re.search(r"surface: GeneratedViewSurface::(\w+)", body)
        marker_m = re.search(r'marker: Some\("([^"]+)"\)', body)
        views[name] = {
            "sources": re.findall(r'"([^"]+)"', sources_m.group(1)) if sources_m else [],
            "targets": re.findall(r'"([^"]+)"', static_m.group(1)) if static_m else [],
            "corpus": corpus,
            "surface": surface_m.group(1) if surface_m else "",
            "marker": marker_m.group(1) if marker_m else None,
            "generator": generator,
        }
    return {"variants": variants, "order": order, "views": views}


def _a_tracked_markdown(root: Path) -> list[Path]:
    out = []
    for path in sorted(root.rglob("*.md")):
        parts = path.relative_to(root).parts
        if any(part in (".git", "target", "__pycache__") for part in parts):
            continue
        out.append(path)
    return out


def generated_view_findings(root: Path) -> list[str]:
    out: list[str] = []
    reg = a_parse_generated_views(root)
    variants, order, views = reg["variants"], reg["order"], reg["views"]
    if not order:
        out.append("spec/generated_views.rs declares no GeneratedView::ALL inventory")
    for variant in variants:
        if variant not in order:
            out.append(f"spec/generated_views.rs: GeneratedView::{variant} is declared "
                       "but missing from GeneratedView::ALL")
    for variant in order:
        if variant not in variants:
            out.append(f"spec/generated_views.rs: GeneratedView::ALL names {variant}, "
                       "which the enum does not declare")
    for duplicate in sorted({v for v in order if order.count(v) > 1}):
        out.append(f"spec/generated_views.rs: GeneratedView::ALL repeats {duplicate}")
    for variant in order:
        if variant not in views:
            out.append(f"spec/generated_views.rs: GeneratedView::{variant} has no "
                       "parseable spec() arm")
    if "GeneratedViewRegistry" not in order:
        out.append("GeneratedView::ALL omits GeneratedViewRegistry; the registry "
                   "must include itself")
    membership = views.get("CorpusEpochMembership")
    if "CorpusEpochMembership" not in order or membership is None \
            or membership["surface"] != "CorpusFrontmatter" or not membership["corpus"]:
        out.append("CorpusEpochMembership may not leave the registry: the corpus "
                   "epoch frontmatter projection must stay registered and complete")
    claimed: dict[tuple[str, str], str] = {}
    for name in order:
        view = views.get(name)
        if view is None:
            continue
        if not view["sources"]:
            out.append(f"generated view {name} names no authority source")
        for source in view["sources"]:
            if not (root / source).is_file():
                out.append(f"generated view {name} names authority source {source}, "
                           "which does not exist")
        for target in view["targets"]:
            if not (root / target).is_file():
                out.append(f"generated view {name} targets {target}, which does not exist")
        surface = view["surface"]
        if surface not in A_VIEW_SURFACES:
            out.append(f"generated view {name} carries unknown surface {surface!r}")
        if surface == "EmbeddedBlock":
            if not view["marker"]:
                out.append(f"embedded generated view {name} carries no marker")
            if not view["targets"] or view["corpus"]:
                out.append(f"embedded generated view {name} must name static targets")
            for target in view["targets"]:
                key = (target, view["marker"] or "")
                if key in claimed:
                    out.append(f"generated views {claimed[key]} and {name} both claim "
                               f"marker {view['marker']} in {target}; one target plus "
                               "one marker names one view")
                claimed[key] = name
        elif surface == "StandaloneFile":
            if view["marker"] is not None:
                out.append(f"standalone generated view {name} carries an embedded marker")
            if len(view["targets"]) != 1 or view["corpus"]:
                out.append(f"standalone generated view {name} must name exactly one "
                           "static target")
        elif surface == "CorpusFrontmatter":
            if view["marker"] is not None:
                out.append(f"corpus-frontmatter generated view {name} carries an "
                           "embedded marker")
            if view["targets"] or not view["corpus"]:
                out.append(f"corpus-frontmatter generated view {name} must target the "
                           "eligible markdown corpus, never static paths")
    # The universal marker census: registered embedded instances == actual.
    registered = {(target, views[name]["marker"]) for name in order
                  if name in views and views[name]["surface"] == "EmbeddedBlock"
                  and views[name]["marker"] for target in views[name]["targets"]}
    marker_source = {marker: "; ".join(views[name]["sources"]) for name in order
                     if name in views and views[name]["marker"] and views[name]["sources"]
                     for marker in [views[name]["marker"]]}
    actual: set[tuple[str, str]] = set()
    for path in _a_tracked_markdown(root):
        rel = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        begins = A_MARKER_BEGIN.findall(text)
        ends = A_MARKER_END.findall(text)
        begin_names = [name for name, _tail in begins]
        for name in sorted(set(begin_names)):
            if begin_names.count(name) > ends.count(name):
                out.append(f"{rel}: generated marker {name} has a BEGIN with no "
                           "matching END")
        for name in sorted(set(ends)):
            if ends.count(name) > begin_names.count(name):
                out.append(f"{rel}: generated marker {name} has an END with no "
                           "matching BEGIN")
        for name, tail in begins:
            if name in marker_source:
                if (rel, name) in actual:
                    out.append(f"{rel}: generated marker {name} appears more than "
                               "once; one target carries one instance of one marker")
                actual.add((rel, name))
                prov = A_MARKER_PROVENANCE.match(tail.strip())
                if not prov:
                    out.append(f"{rel}: {name} BEGIN marker does not carry the "
                               "canonical machine-readable provenance")
                else:
                    if prov.group(1) != marker_source[name]:
                        out.append(f"{rel}: {name} BEGIN marker names source "
                                   f"{prov.group(1)}, not its registered authority "
                                   f"source {marker_source[name]}")
                    if prov.group(2) != "bootstrap/project.py":
                        out.append(f"{rel}: {name} BEGIN marker names generator "
                                   f"{prov.group(2)}, not bootstrap/project.py")
            elif "generated" in tail:
                out.append(f"{rel}: generated marker {name} is not a registered "
                           "generated view; a future view enters the registry "
                           "before it may land")
            elif name not in A_AUTHORED_FENCES:
                out.append(f"{rel}: marker {name} is neither a registered generated "
                           "view nor a declared authored fence")
    for target, marker in sorted(registered - actual):
        out.append(f"{target}: registered generated marker {marker} is absent "
                   "from its target")
    for rel, name in sorted(actual - registered):
        out.append(f"{rel}: generated marker {name} appears in a target the "
                   "registry does not claim")
    # Standalone provenance equals the registry row.
    graph = views.get("GuaranteeGraph")
    if graph:
        graph_path = root / graph["targets"][0] if graph["targets"] else None
        if graph_path and graph_path.is_file():
            front = frontmatter(graph_path.read_text(encoding="utf-8"))
            want_from = "; ".join(graph["sources"])
            if front.get("generated_from") != want_from:
                out.append(f"{graph['targets'][0]}: generated_from is "
                           f"{front.get('generated_from')!r}, not its registry row "
                           f"{want_from!r}")
            if front.get("generated_by") != "bootstrap/project.py":
                out.append(f"{graph['targets'][0]}: generated_by is "
                           f"{front.get('generated_by')!r}, not bootstrap/project.py")
    toolchain_view = views.get("RustToolchain")
    if toolchain_view and toolchain_view["targets"]:
        tc_path = root / toolchain_view["targets"][0]
        want_line = (f"# generated from {toolchain_view['sources'][0]} by "
                     "bootstrap/project.py; do not edit")
        if tc_path.is_file():
            first = tc_path.read_text(encoding="utf-8").splitlines()
            if not first or first[0] != want_line:
                out.append(f"{toolchain_view['targets'][0]}: missing or drifted "
                           "provenance comment; the comment is part of the exact "
                           "projected bytes")
    # The registry's own projection (docs/28) equals the auditor's independent
    # reconstruction, its own row included.
    reg_row = views.get("GeneratedViewRegistry")
    if reg_row and reg_row["targets"] and reg_row["marker"]:
        reg_target = root / reg_row["targets"][0]
        if reg_target.is_file():
            body = batql_extract_block(reg_target.read_text(encoding="utf-8"),
                                       reg_row["marker"])
            surface_display = {"EmbeddedBlock": "embedded-block",
                               "StandaloneFile": "standalone-file",
                               "CorpusFrontmatter": "corpus-frontmatter"}
            generator_display = {"ProjectPy": "bootstrap/project.py"}
            rows = ["| View | Surface | Authority source(s) | Target | Marker | Generator |",
                    "| --- | --- | --- | --- | --- | --- |"]
            for name in order:
                view = views.get(name)
                if view is None:
                    continue
                target_cell = "; ".join(view["targets"]) if view["targets"] \
                    else "eligible markdown corpus"
                rows.append(
                    f"| {name} | {surface_display.get(view['surface'], '?')} | "
                    f"{'; '.join(view['sources'])} | {target_cell} | "
                    f"{view['marker'] or '-'} | "
                    f"{generator_display.get(view['generator'], '?')} |")
            if body is not None and body != "\n".join(rows):
                out.append(f"{reg_row['targets'][0]}: generated block "
                           f"{reg_row['marker']} does not match the "
                           "spec/generated_views.rs projection")
    # Dispatcher parity: the projector's VIEW_RENDERERS key set equals the
    # registry, and no manual literal plan bypasses it. Parsed from source —
    # the auditor never imports or executes project.py. An absent projector
    # is a refusal, never a crash.
    proj_path = root / "bootstrap/project.py"
    if not proj_path.is_file():
        out.append("bootstrap/project.py is absent; the registered generated "
                   "views have no projector")
        return out
    proj_src = proj_path.read_text(encoding="utf-8")
    dispatch_body = re.search(r"\nVIEW_RENDERERS = \{(.*?)\n\}", proj_src, re.S)
    dispatch_keys = re.findall(r'\n    "(\w+)":', dispatch_body.group(1)) \
        if dispatch_body else []
    if not dispatch_keys:
        out.append("bootstrap/project.py declares no VIEW_RENDERERS dispatch map")
    for missing in [n for n in order if n not in dispatch_keys]:
        out.append(f"registered generated view {missing} has no projector "
                   "dispatcher entry")
    for phantom in sorted(set(dispatch_keys) - set(order)):
        out.append(f"projector dispatcher entry {phantom} serializes no "
                   "registered view")
    for literal in re.findall(r'block_pattern\(\s*"([^"]+)"', proj_src):
        out.append(f"bootstrap/project.py builds a manual projection plan for "
                   f"{literal!r}; no literal plan may bypass the registry")
    # The projector's own documentary honesty.
    if re.search(r"do(?:es)? not generate the Guarantee Graph", proj_src):
        out.append("bootstrap/project.py documentation denies generating the "
                   "Guarantee Graph, which its own plan builder generates")
    if "operator blocks current" in proj_src or "WROTE operator blocks" in proj_src:
        out.append("bootstrap/project.py status reports only operator blocks; "
                   "the banner must report the registered-view denominator")
    return out


# --- Repository inventory mirrors (5.5E4b) -----------------------------------
# The typed owners already contain the truth. This auditor reconstructs the
# package, edge, qualification-profile, bundle, and Tier 0 receipt projections
# INDEPENDENTLY (own regexes, own traversal, no shared results with the
# projector) and refuses every retired authored mirror. Three species stay
# distinct: semantic facts (typed spec), derived repository inventory
# (generated from denominators plus current tracked-tree membership), and run
# evidence (exact-SHA, exact-target receipts). A current count is not
# semantic authority; a denominator is not a claim a run passed.
A_PKG_ROW_FULL = re.compile(
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "([^"]*)",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
A_EDGE_ROW = re.compile(
    r'EdgeSpec \{ importer: PackageId::(\w+), importee: PackageId::(\w+), '
    r'class: EdgeClass::(\w+), profile: "([^"]*)" \}')
A_QUAL_FULL = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "([^"]+)",\s*\}', re.S)
A_TIER0_ROWS = re.compile(r'\("([^"]+)", (True|False)\)')
# Authored doctrine fragments the converged documents must keep authoring.
A_E4B_DOCTRINE = (
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "feature-coverage-distinction",
     "They are not additional QualificationProfile identities unless they enter"),
    ("DELIVERY_NOTES.md", "denominator-not-a-run-claim",
     "This block declares the denominator. It does not claim that the latest run passed."),
    ("DELIVERY_NOTES.md", "receipts-carry-outcomes",
     "exact-SHA, exact-target run receipts, not timeless prose"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "authored-metadata-epoch",
     "ReconciliationEpoch corpus membership"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "generated-metadata-sources",
     "generator, exact authority sources, do-not-edit posture"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "generated-needs-no-contract-id",
     "does not invent a contract ID or supersession claim"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "authored-cannot-claim-generated",
     "cannot claim generated status to evade the authored fields"),
)
# Retired authored mirrors, refused against the stripped authored body.
A_E4B_RETIRED = (
    ("README.md", "authored package table",
     re.compile(r"\|\s*`?[a-z-]+`?\s*\|\s*(?:production|dev-only|binary adapter|proc-macro) ")),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "authored layer inventory",
     re.compile(r"^L0\s+macbat-compiler", re.M)),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "ASCII dependency inventory",
     re.compile(r"[↑↑]")),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "five-profile target matrix",
     re.compile(r"^batpak semantic\s+no_std", re.M)),
    ("DELIVERY_NOTES.md", "authored bundle counts",
     re.compile(r"^\d+ (?:numbered|declared|retained|one-for-one|concrete|bootstrap|architectural|classified) ", re.M)),
    ("DELIVERY_NOTES.md", "volatile word count",
     re.compile(r"\d[\d,]*\+? words")),
    ("DELIVERY_NOTES.md", "107-row legacy claim",
     re.compile(r"\b107-row\b")),
    ("DELIVERY_NOTES.md", "static validation PASS table",
     re.compile(r"Validation completed in this delivery environment")),
    ("DELIVERY_NOTES.md", "historical no-rustc claim",
     re.compile(r"did not contain `?rustc`?|structurally inspected but not compiled")),
    ("DELIVERY_NOTES.md", "authored package topology",
     re.compile(r"Frozen package direction")),
    ("DELIVERY_NOTES.md", "current run id embedded as timeless law",
     re.compile(r"\b\d{10,}\b")),
)


def _a_gate_token_render(expr: str, root: Path) -> str:
    src = (root / "spec/gates.rs").read_text(encoding="utf-8")
    rows = re.findall(r'GateSpec \{ id: GateId::(\w+), token: "([^"]*)"', src)
    order = [r[0] for r in rows]
    token_of = dict(rows)
    names = [c.strip().split("::")[-1] for c in expr.split(",") if c.strip()]
    names.sort(key=lambda n: order.index(n) if n in order else 99)
    return "/".join(token_of.get(n, "?") for n in names)


def inventory_mirror_findings(root: Path) -> list[str]:
    out: list[str] = []
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    # Documentary class spellings: variants == ALL, spellings unique/nonempty.
    for type_name in ("PackageClass", "EdgeClass"):
        enum_body = re.search(r"pub enum " + type_name + r" \{(.*?)\n\}", arch, re.S)
        variants = re.findall(r"\n    (\w+),", _uncomment(enum_body.group(1))) \
            if enum_body else []
        all_body = re.search(
            r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", arch, re.S)
        inventory = re.findall(r"\b" + type_name + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"spec/architecture.rs: {type_name}::{missing} is declared "
                       f"but missing from {type_name}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"spec/architecture.rs: {type_name}::ALL names {phantom}, "
                       "which the enum does not declare")
        impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", arch, re.S)
        spellings = re.findall(r"\b" + type_name + r'::(\w+) => "([^"]*)",',
                               impl.group(1)) if impl else []
        seen: dict[str, str] = {}
        for variant, spelling in spellings:
            if not spelling:
                out.append(f"spec/architecture.rs: {type_name}::{variant} declares "
                           "an empty documentary spelling")
            elif spelling in seen:
                out.append(f"spec/architecture.rs: duplicate {type_name} spelling "
                           f"{spelling!r} ({seen[spelling]} and {variant})")
            seen[spelling] = variant
        for variant in [v for v in inventory if v not in dict(spellings)]:
            out.append(f"spec/architecture.rs: {type_name}::{variant} declares no "
                       "documentary spelling arm")
    # Independent projections of the typed rows.
    pkg_all = re.findall(r"\bPackageId::(\w+),", re.search(
        r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];", arch, re.S).group(1))
    cargo = g_package_projection(root, "cargo_name")
    wpath = g_package_projection(root, "workspace_path")
    pkg_class = dict(re.findall(
        r'PackageClass::(\w+) => "([^"]*)",',
        re.search(r"impl PackageClass \{(.*?)\n\}", arch, re.S).group(1)))
    edge_class = dict(re.findall(
        r'EdgeClass::(\w+) => "([^"]*)",',
        re.search(r"impl EdgeClass \{(.*?)\n\}", arch, re.S).group(1)))
    environments = dict(re.findall(r'QualificationEnvironment::(\w+) => "([^"]+)"', arch))
    pkg_rows = A_PKG_ROW_FULL.findall(arch)
    if [r[0] for r in pkg_rows] != pkg_all:
        out.append("spec/architecture.rs: PACKAGES does not equal PackageId::ALL "
                   "exactly and in order")
    want_pkg = [f"| {cargo.get(pid, '?')} | {pkg_class.get(cls, '?')} | {layer} | "
                f"{wpath.get(pid, '?')} | {role} |"
                for pid, role, cls, layer in pkg_rows]
    want_edges = [f"| {cargo.get(a, '?')} | {cargo.get(b, '?')} | "
                  f"{edge_class.get(cls, '?')} | {profile} |"
                  for a, b, cls, profile in A_EDGE_ROW.findall(arch)]
    want_quals = [f"| {cargo.get(pkg, '?')} | {profile} | "
                  f"{environments.get(env, '?')} | {_a_gate_token_render(gates, root)} | "
                  f"{req} |"
                  for pkg, profile, env, gates, req in A_QUAL_FULL.findall(arch)]
    # Tier 0 receipt denominator, from the typed owner it now lives in (5.5E6a).
    bq_path = root / "spec/bootstrap_qualification.rs"
    bq = bq_path.read_text(encoding="utf-8") if bq_path.is_file() else ""
    bq_order_body = re.search(
        r"pub const ALL: &'static \[Tier0ReceiptKind\] = &\[(.*?)\];", bq, re.S)
    bq_order = re.findall(r"Tier0ReceiptKind::(\w+),", bq_order_body.group(1)) \
        if bq_order_body else []
    bq_slugs = dict(re.findall(r'Tier0ReceiptKind::(\w+) => "([^"]+)"', bq))
    bq_policy = dict(re.findall(
        r"Tier0ReceiptKind::(\w+) => Tier0ArtifactPolicy::(\w+)", bq))
    want_tier0 = [f"| {bq_slugs[k]} | {bq_policy[k]} |" for k in bq_order
                  if k in bq_slugs and k in bq_policy]
    if bq and not want_tier0:
        out.append("spec/bootstrap_qualification.rs declares no parseable "
                   "Tier 0 receipt denominator")
    # Bundle inventory: every count independently derived.
    md_paths = _a_tracked_markdown(root)
    reg = a_parse_generated_views(root)
    seeds = len(re.findall(r'InvariantSpec \{ id: "', (root / "spec/invariants.rs")
                           .read_text(encoding="utf-8")))
    decisions = len(G_DEC_ROW.findall((root / "spec/dispositions.rs")
                                      .read_text(encoding="utf-8")))
    obligations = len(G_LEG_ROW.findall((root / "spec/legacy_obligations.rs")
                                        .read_text(encoding="utf-8")))
    coverage_src = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", coverage_src,
        re.S).group(1))
    if len(re.findall(r"LegacyInvariantCoverage \{\s*legacy_id:", coverage_src)) \
            != len(manifest):
        out.append("spec/legacy_invariant_coverage.rs: COVERAGE rows do not equal "
                   "SOURCE_INVARIANT_IDS")
    op_model = batql_parse_operator_model(root)
    if len(op_model["rows"]) != len(op_model["id_all"]):
        out.append("spec/operators.rs: OPERATORS rows do not equal OperatorId::ALL")
    static_instances = sum(len(reg["views"][n]["targets"]) for n in reg["order"]
                           if n in reg["views"])
    bindings = sum(1 for path in md_paths
                   if re.search(r"^(?:source_)?reconciliation_epoch:",
                                path.read_text(encoding="utf-8"), re.M))
    derived = [
        ("numbered architecture documents",
         len(list((root / "docs").glob("[0-9][0-9]_*.md"))),
         "current tracked docs matching docs/[0-9][0-9]_*.md"),
        ("Markdown documents", len(md_paths), "current eligible Markdown corpus"),
        ("Cargo packages", len(pkg_all), "PackageId::ALL with PACKAGES parity"),
        ("package edges", len(A_EDGE_ROW.findall(arch)), "EDGES"),
        ("qualification profiles", len(A_QUAL_FULL.findall(arch)),
         "QUALIFICATION_PROFILES"),
        ("SEED guarantees", seeds, "spec/invariants.rs SEED inventory"),
        ("decision rows", decisions, "spec/dispositions.rs DECISIONS"),
        ("legacy semantic obligations", obligations,
         "spec/legacy_obligations.rs OBLIGATIONS"),
        ("legacy invariant declarations", len(manifest),
         "SOURCE_INVARIANT_IDS with COVERAGE parity"),
        ("BatQL operators", len(op_model["id_all"]),
         "OperatorId::ALL with OPERATORS parity"),
        ("registered generated views", len(reg["order"]), "GeneratedView::ALL"),
        ("static generated target instances", static_instances,
         "expansion of every Static registry target"),
        ("corpus-frontmatter bindings", bindings,
         "the eligible Markdown corpus reached by CorpusEpochMembership"),
    ]
    want_bundle = [f"| {metric} | {count} | {derivation} |"
                   for metric, count, derivation in derived]
    # Positional block parity for all five projections.
    for rel, marker, want in (
        ("README.md", "PACKAGE-INVENTORY", want_pkg),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "PACKAGE-INVENTORY", want_pkg),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "PACKAGE-EDGES", want_edges),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "QUALIFICATION-PROFILES", want_quals),
        ("DELIVERY_NOTES.md", "BUNDLE-INVENTORY", want_bundle),
        ("DELIVERY_NOTES.md", "TIER0-RECEIPT-DENOMINATOR", want_tier0),
    ):
        path = root / rel
        body = batql_extract_block(path.read_text(encoding="utf-8"), marker) \
            if path.is_file() else None
        if body is None:
            out.append(f"{rel}: missing generated block {marker}")
            continue
        got = [line for line in body.splitlines()[2:] if line.strip()]
        if got != want:
            out.append(f"{rel}: generated block {marker} does not match its "
                       "typed derivation")
    # Retired mirrors stay retired, and the converged doctrine stays authored —
    # both judged with every generated block stripped.
    stripped: dict[str, str] = {}
    for rel in ("README.md", "docs/03_REPOSITORY_AND_PACKAGES.md",
                "DELIVERY_NOTES.md", "docs/29_STATUS_AND_SUPERSESSION.md"):
        path = root / rel
        stripped[rel] = A_GENERATED_BLOCK.sub("", path.read_text(encoding="utf-8")) \
            if path.is_file() else ""
    for rel, label, pattern in A_E4B_RETIRED:
        if pattern.search(stripped[rel]):
            out.append(f"{rel}: retired authored mirror may not return ({label})")
    for rel, name, fragment in A_E4B_DOCTRINE:
        if fragment not in stripped[rel]:
            out.append(f"{rel}: converged inventory doctrine absent ({name})")
    return out


# --- Version boundaries and exact-ledger mirrors (5.5E4c) --------------------
# Part one: the accepted version-identity vocabulary converges with DEC-064
# and every documentary consumer — the inventory derives from
# VersionIdentityKind::ALL, never a hardcoded count. Part two/three: docs/30
# and docs/34 are exact mirrors whose complete rows live in typed owners; the
# auditor reconstructs both generated bodies independently and refuses the
# authored mirrors' return. Part four: the docs/28 convergence doctrine.
A_DEC_FULL_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+), '
    r'subject: "((?:[^"\\]|\\.)*)", successor: "((?:[^"\\]|\\.)*)"')
A_COV_FULL_ROW = re.compile(
    r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", '
    r'disposition: CoverageDisposition::(\w+), successor: "((?:[^"\\]|\\.)*)", '
    r'rationale: "((?:[^"\\]|\\.)*)" \}')
A_VERSION_ALIAS_BANS = ("EventFrameVersion", "FbatFrameVersion")
A_VERSION_DOCTRINE = (
    ("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "frame-version-reservation",
     "FrameVersion is reserved for the EventFrame envelope"),
    ("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "shared-representation-fence",
     "Sharing an integer representation or a migration does not permit"),
    ("docs/05_STORAGE_FBAT_AND_TILES.md", "three-way-envelope-split",
     "FrameVersion versions the EventFrame envelope. FbatFormatVersion versions "
     "the surrounding .fbat container. BatTaggedRecordVersion versions a tagged "
     "payload record inside EncodedPayload"),
    ("docs/05_STORAGE_FBAT_AND_TILES.md", "coordinated-migration",
     "preserving three independent version identities"),
    ("docs/15_SCHEMA_CODEC_AND_MIGRATION.md", "payload-record-boundary",
     "BatTaggedRecordVersion versions the tagged payload-record envelope and "
     "codec. It does not version EventFrame or the .fbat container"),
    ("docs/02_SYSTEM_MODEL.md", "layout-schema-split",
     "LayoutVersion versions physical organization under LayoutId. SchemaVersion "
     "versions schema meaning. Neither substitutes for the other"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "frame-version-constants",
     "FrameVersion values and EventFrame envelope field IDs/order"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "container-constants",
     "FbatFormatVersion values and .fbat container magic/header"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "payload-record-constants",
     "BatTaggedRecordVersion values, field tags, and length encoding"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "layout-constants",
     "LayoutVersion values for selected physical layouts"),
    ("docs/24_GAUNTLET.md", "version-witness-consumes-all",
     "Every member of spec/identities.rs VersionIdentityKind::ALL denotes a "
     "distinct version type"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "exact-mirror-clause",
     "An equivalence checker babysitting two editable copies is not convergence"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "no-python-prosthetics-clause",
     "from conventions, neighbouring rows, or Python defaults"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "subset-relation-clause",
     "Parsing prose headings is not an authority relation"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "historical-narrative-clause",
     "cannot be read as the current inventory"),
    ("docs/34_LEGACY_INVARIANT_COVERAGE.md", "historical-framing",
     "retained as secondary historical evidence, not a competing denominator"),
)


def _a_str_arms(source: str, type_name: str, fn_name: str) -> dict[str, str]:
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", source, re.S)
    fn = re.search(
        r"pub const fn " + fn_name + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        impl.group(1), re.S) if impl else None
    return dict(re.findall(
        r"\b" + type_name + r'::(\w+) => \{?\s*"((?:[^"\\]|\\.)*)"', fn.group(1))) \
        if fn else {}


def _a_all_of(source: str, type_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", source, re.S)
    return re.findall(r"\b" + type_name + r"::(\w+)", body.group(1)) if body else []


def _a_enum_variants_of(source: str, type_name: str) -> list[str]:
    body = re.search(r"pub enum " + type_name + r" \{(.*?)\n\}", source, re.S)
    return re.findall(r"\n    (\w+),", _uncomment(body.group(1))) if body else []


def _a_spelling_laws(out, source, src_label, type_name, spelling_fn,
                     meaning_fn=None):
    variants = _a_enum_variants_of(source, type_name)
    inventory = _a_all_of(source, type_name)
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"{src_label}: {type_name}::{missing} is declared but missing "
                   f"from {type_name}::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"{src_label}: {type_name}::ALL names {phantom}, which the "
                   "enum does not declare")
    for fn_name, kind in ((spelling_fn, "spelling"),) + (
            ((meaning_fn, "meaning"),) if meaning_fn else ()):
        arms = _a_str_arms(source, type_name, fn_name)
        seen: dict[str, str] = {}
        for variant in inventory:
            value = arms.get(variant)
            if not value:
                out.append(f"{src_label}: {type_name}::{variant} declares no or an "
                           f"empty {kind}")
                continue
            if kind == "spelling" and value in seen:
                out.append(f"{src_label}: duplicate {type_name} spelling {value!r} "
                           f"({seen[value]} and {variant})")
            seen[value] = variant
    return inventory


def exact_ledger_findings(root: Path) -> list[str]:
    out: list[str] = []
    # Version-identity convergence.
    ident_src = (root / "spec/identities.rs").read_text(encoding="utf-8")
    version_all = _a_all_of(ident_src, "VersionIdentityKind")
    version_pairs = re.findall(
        r'\bVersionIdentityKind::(\w+) => \{?\s*entry!\(\s*"([^"]+)",\s*"([^"]+)"\s*\)',
        ident_src)
    version_arms = {v: s for v, s, _o in version_pairs}
    version_owner = {v: o for v, _s, o in version_pairs}
    spellings = [version_arms[v] for v in version_all if v in version_arms]
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    row = re.search(
        r'DecisionSpec \{ id: "DEC-064",.*?subject: "([^"]*)", successor: "([^"]*)",'
        r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
    fields = " ".join(g or "" for g in row.groups()) if row else ""
    if not row:
        out.append("DEC-064 is missing from the decision ledger")
    for spelling in spellings:
        if not re.search(r"\b" + re.escape(spelling) + r"\b", fields):
            out.append(f"DEC-064 forward-policy fields do not name {spelling}; "
                       "the version-separation law names its complete inventory")
    named = set(re.findall(r"\b([A-Z][A-Za-z]*Version)\b", fields)) - {"Version"}
    for phantom in sorted(named - set(spellings)):
        out.append(f"DEC-064 names {phantom}, which VersionIdentityKind::ALL "
                   "does not declare (phantom version identity)")
    frame_owner = version_owner.get("Frame")
    if frame_owner != "BP-STORAGE-TILES-1":
        out.append(f"FrameVersion is owned by {frame_owner!r}, not BP-STORAGE-TILES-1")
    doc05 = (root / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8")
    if "frame_version: FrameVersion" not in doc05:
        out.append("docs/05: EventFrameV2 does not carry frame_version: FrameVersion")
    for rel, name, fragment in A_VERSION_DOCTRINE:
        text = A_GENERATED_BLOCK.sub("", (root / rel).read_text(encoding="utf-8")) \
            if (root / rel).is_file() else ""
        if re.sub(r"\s+", " ", fragment) not in re.sub(r"\s+", " ", text):
            out.append(f"{rel}: version/convergence doctrine absent ({name})")
    doc24 = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    if re.search(r"ten declared version identities", doc24):
        out.append("docs/24: the version witness may not freeze a numeric count; "
                   "it consumes VersionIdentityKind::ALL")
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        stripped = A_GENERATED_BLOCK.sub("", rel.read_text(encoding="utf-8"))
        for alias in A_VERSION_ALIAS_BANS:
            if re.search(r"\b" + alias + r"\b", stripped):
                out.append(f"{rel.relative_to(root).as_posix()}: names {alias}, "
                           "an unadmitted version-identity alias")
    for spec_rel in sorted((root / "spec").glob("*.rs")):
        if re.search(r"pub (?:struct|enum) Version\b",
                     _uncomment(spec_rel.read_text(encoding="utf-8"))):
            out.append(f"{spec_rel.relative_to(root).as_posix()}: declares a "
                       "generic Version type; no generic Version crosses a "
                       "subsystem boundary")
    # E4d protections: the mixed ledgers stay authored until their missing
    # fields have real owners.
    for rel, label in (("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
                        "docs/21 carries evidence and witness meaning "
                        "LegacyObligation does not yet own"),
                       ("docs/24_GAUNTLET.md",
                        "docs/24 is the canonical semantic owner of proof-row "
                        "meaning, not an exact mirror")):
        front = frontmatter((root / rel).read_text(encoding="utf-8"))
        if front.get("status") == "GENERATED":
            out.append(f"{rel}: may not be marked GENERATED before its missing "
                       f"fields are owned ({label})")
    # Decision-ledger and coverage reconstruction, byte-exact.
    disp_all = _a_spelling_laws(out, dsrc, "spec/dispositions.rs", "Disposition",
                                "spelling", "meaning")
    _a_spelling_laws(out, dsrc, "spec/dispositions.rs", "DecisionClass", "spelling")
    disp_tags = _a_str_arms(dsrc, "Disposition", "spelling")
    disp_meanings = _a_str_arms(dsrc, "Disposition", "meaning")
    class_spellings = _a_str_arms(dsrc, "DecisionClass", "spelling")
    ledger_lines = ["| Tag | Meaning |", "| --- | --- |"]
    ledger_lines += [f"| {disp_tags.get(v, '?')} | {disp_meanings.get(v, '?')} |"
                     for v in disp_all]
    ledger_lines += ["", "| ID | Tag | Class | Gates | Subject | Final ruling |",
                     "| --- | --- | --- | --- | --- | --- |"]
    for dec_id, cls, gates, disp, subject, successor in A_DEC_FULL_ROW.findall(dsrc):
        if disp not in disp_tags:
            out.append(f"{dec_id} carries disposition {disp}, which "
                       "Disposition::ALL does not admit")
        if cls not in class_spellings:
            out.append(f"{dec_id} carries class {cls}, which DecisionClass::ALL "
                       "does not admit")
        ledger_lines.append(
            f"| {dec_id} | {disp_tags.get(disp, '?')} | "
            f"{class_spellings.get(cls, '?')} | {_a_gate_token_render(gates, root)} | "
            f"{subject} | {successor} |")
    cov_src = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    cov_all = _a_spelling_laws(out, cov_src, "spec/legacy_invariant_coverage.rs",
                               "CoverageDisposition", "spelling", "meaning")
    cov_tags = _a_str_arms(cov_src, "CoverageDisposition", "spelling")
    cov_meanings = _a_str_arms(cov_src, "CoverageDisposition", "meaning")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", cov_src,
        re.S).group(1))
    cov_rows = A_COV_FULL_ROW.findall(cov_src)
    cov_lines = ["| Disposition | Meaning |", "| --- | --- |"]
    cov_lines += [f"| {cov_tags.get(v, '?')} | {cov_meanings.get(v, '?')} |"
                  for v in cov_all]
    cov_lines += ["", f"The source declaration denominator is the {len(manifest)}"
                  "-declaration `SOURCE_INVARIANT_IDS` manifest, in exact set "
                  "parity with the coverage rows below.",
                  "", "| Legacy invariant | Disposition | Clean successor | Rationale |",
                  "| --- | --- | --- | --- |"]
    for inv, disp, successor, rationale in cov_rows:
        if disp not in cov_tags:
            out.append(f"coverage row {inv} carries disposition {disp}, which "
                       "CoverageDisposition::ALL does not admit")
        cov_lines.append(f"| `{inv}` | {cov_tags.get(disp, '?')} | `{successor}` | "
                         f"{rationale} |")
    for rel, marker, want in (
        ("docs/30_DECISION_AND_REJECTION_LEDGER.md", "DECISION-LEDGER",
         "\n".join(ledger_lines)),
        ("docs/34_LEGACY_INVARIANT_COVERAGE.md", "LEGACY-INVARIANT-COVERAGE",
         "\n".join(cov_lines)),
    ):
        text = (root / rel).read_text(encoding="utf-8") if (root / rel).is_file() else ""
        body = batql_extract_block(text, marker)
        if body is None:
            out.append(f"{rel}: missing generated block {marker}")
        elif body != want:
            out.append(f"{rel}: generated block {marker} does not match its "
                       "typed derivation")
        stripped = A_GENERATED_BLOCK.sub("", text)
        if marker == "DECISION-LEDGER":
            if re.search(r"^\| DEC-\d+ \|", stripped, re.M):
                out.append(f"{rel}: an authored decision row may not return "
                           "outside the generated ledger")
            if re.search(r"^KEEP\s{2,}retained", stripped, re.M):
                out.append(f"{rel}: an authored disposition inventory may not "
                           "return outside the generated ledger")
        else:
            if re.search(r"^\| `INV-", stripped, re.M):
                out.append(f"{rel}: an authored coverage row may not return "
                           "outside the generated matrix")
            if re.search(r"^PRESERVE\s{2,}semantic law", stripped, re.M):
                out.append(f"{rel}: an authored coverage disposition inventory "
                           "may not return outside the generated matrix")
    return out


# --- Proof relations and the complete legacy ledger (5.5E4d) -----------------
# spec/proof.rs is the one membership authority; docs/24 owns meaning; the
# domain documents own the pressured law; docs/21 is the generated human
# ledger. This auditor reconstructs every projection independently, proves
# the contract/target selection laws, and sweeps the retired mirrors.
A_E4D_ACTIVE = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
    r'ProofRowState::Active \{ guarantee: GuaranteeRef::(leg|dec)\("([^"]+)"\), '
    r'projection_contracts: &\[([^\]]*)\] \} \}')
A_E4D_LEG_FULL = re.compile(
    r'LegacyObligation \{ id: "(LEG-\d+)", law: "((?:[^"\\]|\\.)*)", '
    r'legacy_evidence: "((?:[^"\\]|\\.)*)", clean_owner: "([^"]*)", '
    r'mechanism_disposition: "((?:[^"\\]|\\.)*)", witness_requirement: '
    r'LegacyWitnessRequirement::(CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::(\w+), '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: '
    r'ObligationStatus::(\w+) \}')
A_E4D_PLANNED_TEXT = re.compile(r'Planned\("((?:[^"\\]|\\.)*)"\)')
# The automatic consumers never appear in projection_contracts: docs/24 and
# docs/21 receive every relation mechanically.
A_E4D_AUTOMATIC = {"BP-GAUNTLET-1", "BP-LEGACY-OBLIGATIONS-1"}


def proof_relation_findings(root: Path) -> list[str]:
    out: list[str] = []
    psrc = (root / "spec/proof.rs").read_text(encoding="utf-8")
    active = [{"id": m[0], "family": m[1], "guarantee": m[2],
               "contracts": re.findall(r'ContractId\("([^"]+)"\)', m[3])}
              for m in A_E4D_ACTIVE.findall(psrc)]
    contract_ids: dict[str, str] = {}
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        front = frontmatter(rel.read_text(encoding="utf-8"))
        if front.get("contract_id"):
            contract_ids[front["contract_id"]] = front.get("status", "")
    for row in active:
        if not row["contracts"]:
            out.append(f"active proof row {row['id']} names no projection contract")
        if len(row["contracts"]) != len(set(row["contracts"])):
            out.append(f"active proof row {row['id']} repeats a projection contract")
        for contract in row["contracts"]:
            if contract in A_E4D_AUTOMATIC:
                out.append(f"active proof row {row['id']} lists automatic consumer "
                           f"{contract}; docs/24 and docs/21 receive every relation "
                           "mechanically")
            elif contract not in contract_ids:
                out.append(f"active proof row {row['id']} names projection contract "
                           f"{contract}, which no authored document declares")
            elif contract_ids[contract] != "AUTHORITATIVE":
                out.append(f"active proof row {row['id']} names {contract}, whose "
                           "document is not an authoritative consumer")
    # PROOF-RELATIONS block parity (independent grouping).
    groups: dict[tuple, list[str]] = {}
    for row in active:
        groups.setdefault((row["guarantee"], tuple(row["contracts"])), []).append(row["id"])
    want_rel = ["| Guarantee | Projection contract(s) | Active proof rows |",
                "| --- | --- | --- |"]
    for (guarantee, contracts), ids in groups.items():
        want_rel.append(f"| {guarantee} | {'; '.join(contracts)} | {'; '.join(ids)} |")
    d24_text = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    body = batql_extract_block(d24_text, "PROOF-RELATIONS")
    if body is None:
        out.append("docs/24: missing generated block PROOF-RELATIONS")
    elif body != "\n".join(want_rel):
        out.append("docs/24: generated block PROOF-RELATIONS does not match its "
                   "typed derivation")
    # Domain projections: registered PROOF-REQUIREMENTS targets == the
    # contract set active rows project to, selected by frontmatter.
    reg = a_parse_generated_views(root)
    domain_targets = [reg["views"][n]["targets"][0] for n in reg["order"]
                      if n in reg["views"] and reg["views"][n].get("marker") == "PROOF-REQUIREMENTS"
                      and reg["views"][n]["targets"]]
    target_contracts: dict[str, str] = {}
    for rel in domain_targets:
        front = frontmatter((root / rel).read_text(encoding="utf-8")) \
            if (root / rel).is_file() else {}
        target_contracts[rel] = front.get("contract_id", "")
    used = {c for row in active for c in row["contracts"]}
    registered = set(target_contracts.values())
    for missing in sorted(used - registered):
        out.append(f"projection contract {missing} has no registered domain "
                   "proof view")
    for phantom in sorted(registered - used):
        out.append(f"registered domain proof view for {phantom} has no active "
                   "proof relation")
    for rel, contract in sorted(target_contracts.items()):
        text = (root / rel).read_text(encoding="utf-8") if (root / rel).is_file() else ""
        body = batql_extract_block(text, "PROOF-REQUIREMENTS")
        if body is None:
            out.append(f"{rel}: missing generated block PROOF-REQUIREMENTS")
            continue
        want_groups: dict[str, list[str]] = {}
        for row in active:
            if contract in row["contracts"]:
                want_groups.setdefault(row["guarantee"], []).append(row["id"])
        want = ["| Guarantee | Required proof rows |", "| --- | --- |"]
        for guarantee, ids in want_groups.items():
            want.append(f"| {guarantee} | {'; '.join(ids)} |")
        if body != "\n".join(want):
            out.append(f"{rel}: generated block PROOF-REQUIREMENTS does not match "
                       "its typed derivation")
    if "hash_map_iteration_cannot_influence_canonical_observables" in \
            (root / "docs/03_REPOSITORY_AND_PACKAGES.md").read_text(encoding="utf-8"):
        out.append("docs/03 carries the hash-map proof row; canonical collection "
                   "order is docs/04's law")
    # docs/21 ledger parity, including the witness route cells.
    lsrc = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    by_leg: dict[str, list[str]] = {}
    for row in active:
        if row["family"] == "leg":
            by_leg.setdefault(row["guarantee"], []).append(row["id"])
    want_ledger = ["| ID | Retained law | Legacy evidence | Clean owner | Mechanism "
                   "disposition | Required witness | Gates | Compatibility | "
                   "Deletion condition | Status |",
                   "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for m in A_E4D_LEG_FULL.finditer(lsrc):
        leg, law, evidence, owner, mech, route, gates, compat, deletion, status = m.groups()
        if not evidence.strip():
            out.append(f"{leg} carries no legacy evidence pointer")
        if not mech.strip():
            out.append(f"{leg} carries no mechanism disposition")
        if route == "CanonicalProofRows":
            witness = "; ".join(by_leg.get(leg, []))
            if not by_leg.get(leg):
                out.append(f"{leg} claims CanonicalProofRows with no active typed relation")
        else:
            witness = A_E4D_PLANNED_TEXT.search(route).group(1)
            if not witness.strip():
                out.append(f"{leg} carries an empty planned witness")
            if by_leg.get(leg):
                out.append(f"{leg} carries a planned witness AND active proof rows")
        want_ledger.append(f"| {leg} | {law} | {evidence} | {owner} | {mech} | "
                           f"{witness} | {_a_gate_token_render(gates, root)} | "
                           f"{compat} | {deletion} | {status} |")
    for guarantee in sorted(set(by_leg) - {m.group(1) for m in A_E4D_LEG_FULL.finditer(lsrc)}):
        out.append(f"a proof relation binds {guarantee}, which no obligation row declares")
    d21_text = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    body = batql_extract_block(d21_text, "LEGACY-OBLIGATION-LEDGER")
    if body is None:
        out.append("docs/21: missing generated block LEGACY-OBLIGATION-LEDGER")
    elif body != "\n".join(want_ledger):
        out.append("docs/21: generated block LEGACY-OBLIGATION-LEDGER does not "
                   "match its typed derivation")
    # The final mirror sweep: with generated blocks and historical migration
    # material removed, no membership inventory or retired ownership claim
    # survives anywhere in the corpus.
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        stripped = A_GENERATED_BLOCK.sub("", text)
        stripped = re.sub(r"<!-- HISTORICAL-MIGRATION:BEGIN -->.*?"
                          r"<!-- HISTORICAL-MIGRATION:END -->", "", stripped, flags=re.S)
        name = rel.relative_to(root).as_posix()
        if re.search(r"^\| LEG-\d+ \|", stripped, re.M):
            out.append(f"{name}: an authored legacy-obligation row may not return "
                       "outside the generated ledger")
        if re.search(r"Required witnesses \(proof owner", stripped):
            out.append(f"{name}: an authored required-witnesses inventory may not "
                       "return; spec/proof.rs owns membership")
        if re.search(r"Required proof rows[^\n]*:\s*\n+```text", stripped):
            out.append(f"{name}: an authored required-proof-rows inventory may not "
                       "return; the PROOF-REQUIREMENTS projection owns it")
        if re.search(r"until the documentary convergence pass", stripped):
            out.append(f"{name}: the temporary documentary-convergence wording may "
                       "not return")
        if re.search(r"docs/24 owns proof-row identity|owns proof-row identity and "
                     r"executable meaning|canonical proof-row owner: docs/24", stripped):
            out.append(f"{name}: docs/24 may not claim proof-row identity ownership; "
                       "spec/proof.rs owns identity, docs/24 owns meaning")
    if "until the documentary convergence pass" in psrc:
        out.append("spec/proof.rs: the temporary documentary-convergence wording "
                   "may not return")
    for field in ("expectation:", "disposition_text:", "meaning:"):
        if re.search(r"\b" + field.rstrip(":") + r":\s*&'static str", psrc):
            out.append(f"spec/proof.rs: a {field.rstrip(':')} prose field may not "
                       "enter the typed catalog; docs/24 owns meaning")
    # docs/24 doctrine presence.
    d24_stripped = A_GENERATED_BLOCK.sub("", d24_text)
    for name, fragment in (
        ("typed-ownership-split", "spec/proof.rs` owns ProofRowId identity"),
        ("meaning-stays", "docs/24 owns each active row's authoritative semantic summary"),
        ("relation-cannot-move-meaning", "A generated relation cannot change proof meaning"),
        ("prose-cannot-move-rows", "A prose meaning cannot silently move a proof row"),
    ):
        if fragment not in d24_stripped:
            out.append(f"docs/24: typed proof-ownership doctrine absent ({name})")
    return out


# --- Admitted contract kinds (5.5E3c, block form 5.5E4a) ---------------------
# spec/contracts.rs owns the admitted set; docs/06 projects it through the
# ordinary CONTRACT-KINDS generated block. A kind whose admitting citation
# dangles, and a block that admits more or fewer kinds than the enum, are both
# refused — and the retired HYBRID fence (a generator editing selected columns
# inside an authored fence) may not return.
A_CONTRACT_KIND_HYBRID_FENCE = re.compile(
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
    # ContractKind::ALL order, through the ordinary CONTRACT-KINDS generated
    # block. Comparing sets of kind names alone would let the doc confidently
    # explain the wrong provenance.
    spelling_of = dict(re.findall(r'ContractKind::(\w+) => "(\w+)",', src))
    basis_of = {k: ident for k, _tag, ident in tagged}
    expected = [(spelling_of.get(v, ""), basis_of.get(v, "")) for v in inventory]
    doc = (root / "docs/06_MACBAT.md").read_text(encoding="utf-8")
    if A_CONTRACT_KIND_HYBRID_FENCE.search(doc):
        out.append("docs/06 hybrid admitted-kinds fence may not return; "
                   "CONTRACT-KINDS is an ordinary generated block and no "
                   "generator edits selected columns inside an authored fence")
    body = batql_extract_block(doc, "CONTRACT-KINDS")
    if body is None:
        out.append("docs/06 carries no generated CONTRACT-KINDS block")
        return out
    listed = [tuple(cell.strip() for cell in line.strip().strip("|").split("|"))
              for line in body.splitlines()[2:] if line.strip()]
    for i, want in enumerate(expected):
        if i >= len(listed):
            out.append(f"docs/06 omits admitted contract kind row {want[0]} {want[1]}")
        elif listed[i] != want:
            out.append(
                f"docs/06 CONTRACT-KINDS row {i + 1} states {' '.join(listed[i])}; "
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
    # Owner coherence (5.5E3d2): the owner must AUTHOR the term. Map each
    # declared contract to its authoritative document's authored text —
    # generated blocks removed, so a projection can never notarize its own
    # owner claim. An occurrence in some other live contract's document does
    # not satisfy the claim: a live building cannot sign a passport merely
    # because the applicant exists somewhere else in town.
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)

    def owner_authors(cid: str, term: str) -> bool:
        return bool(re.search(r"\b" + re.escape(term) + r"\b", owner_text.get(cid, "")))

    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_disposition = {m[0]: m[3] for m in G_DEC_ROW.findall(dsrc)}
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
            elif not owner_authors(owner, spelling):
                out.append(f"{kind} entry {spelling} names owner {owner}, "
                           "whose authoritative document does not author the term")
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
            elif not owner_authors(ref, term):
                out.append(f"residue term {term} is owned elsewhere by {ref}, "
                           "which does not author the term")
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
                # Decision coherence (5.5E3d2): the cited decision must NAME
                # the pending term in its forward-policy fields — subject,
                # successor, or replacement contract. stale_aliases retire
                # vocabulary and carry no future admission authority.
                row = re.search(
                    r'DecisionSpec \{ id: "' + re.escape(ref)
                    + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                    + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
                fields = " ".join(g or "" for g in row.groups()) if row else ""
                if not re.search(r"\b" + re.escape(term) + r"\b", fields):
                    out.append(f"residue term {term} is not yet admitted by {ref}, "
                               "whose forward-policy fields do not name the term")
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
    for rel in ("spec/architecture.rs", "spec/proof.rs", "spec/gates.rs",
                "spec/operators.rs"):
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


# --- Promotion requirements (5.5E3i) -----------------------------------------
# spec/promotion.rs owns the conjunctive denominator: four required members,
# no optional flag, no waiver variant, no second list anywhere. docs/12 owns
# what each requirement means; docs/24 consumes Rule qualification; docs/31
# consumes the typed law. The authored doctrine clauses are themselves law:
# each fragment below has a hostile proving its deletion reds.
A_PROMOTION_DOCTRINE = (
    ("a missing member refuses promotion",
     "the requirements are conjunctive"),
    ("are not independence",
     "the evidence route cannot be self-grading"),
    ("A generic issue link, chat",
     "a generic issue is not a named proof target"),
    ("chat transcript, commit message, or vague",
     "a commit message is not a named proof target"),
    ("a stable name, a ContractId owner, the affected",
     "a documented proof gap requires a stable owner"),
    ("Rule qualification law",
     "qualified hostile evidence consumes the docs/24 Rule qualification law"),
    ("EquivalentCandidate without its independent equivalence witness",
     "an equivalent candidate is not a killed mutant"),
    ("a failure produced by the unchanged baseline",
     "a baseline failure cannot kill a candidate"),
    ("A commit message\nis not this receipt.",
     "a commit message is not the promotion receipt"),
    ("resulting tracked-content commitment",
     "the receipt binds the resulting tracked-content commitment"),
)
A_PROMOTION_DISQUALIFIED = (
    "Survived", "NotActivated", "Refused", "Unbuildable", "TimedOut",
    "InfrastructureFailure",
)


def promotion_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/promotion.rs"
    if not path.is_file():
        return ["missing spec/promotion.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    enum_body = re.search(r"pub enum PromotionRequirement \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[PromotionRequirement\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bPromotionRequirement::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"PromotionRequirement::{missing} is omitted from "
                   "PromotionRequirement::ALL; the denominator is conjunctive "
                   "and complete")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"PromotionRequirement::ALL names {phantom}, which the enum "
                   "does not declare")
    if re.search(r"\boptional\b|\bwaiver\b", src, re.I):
        out.append("spec/promotion.rs speaks of optional or waiver; the "
                   "denominator is conjunctive with no escape hatch")

    def fn_arms(name):
        body = re.search(
            r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        return dict(re.findall(r"PromotionRequirement::(\w+)\s*=>\s*([^,]+),",
                               body.group(1), re.S)) if body else {}

    spellings = fn_arms("spelling")
    owners = fn_arms("semantic_owner")
    bases = fn_arms("admission_basis")
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    seen: set[str] = set()
    rows: list[str] = []
    for v in inventory:
        s = spellings.get(v, "").strip().strip('"')
        if not s.strip():
            out.append(f"PromotionRequirement::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"promotion-requirement spelling {s} is claimed twice")
        seen.add(s)
        owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
        oid = owner.group(1) if owner else ""
        if oid not in contract_ids:
            out.append(f"promotion requirement {s} cites owner {oid}, which no "
                       "declared contract owns")
        elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
            out.append(f"promotion requirement {s} cites owner {oid}, whose "
                       "authoritative document does not author the spelling")
        basis = re.search(r'GuaranteeRef::dec\("([^"]+)"\)', bases.get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"promotion requirement {s} cites admission basis {bid}, "
                       "which no declared decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"promotion requirement {s} cites admission basis {bid}, "
                           "whose forward-policy fields do not name the requirement")
        rows.append(f"{s:<26} {oid:<14} {bid}")
    # The family facts answer four different questions.
    surface = re.search(
        r"PROMOTION_POLICY_SURFACE: ProofPolicySurface = ProofPolicySurface::(\w+);", src)
    if not surface or surface.group(1) != "CandidatePromotion":
        out.append("the promotion policy surface is not "
                   "ProofPolicySurface::CandidatePromotion")
    change = re.search(
        r'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec\("([^"]+)"\);', src)
    if not change or change.group(1) != "DEC-074":
        out.append("the promotion policy-change basis is not DEC-074; requirement "
                   "admission and change classification are different laws")
    egate = re.search(r"PROMOTION_ENFORCEMENT_GATE: GateId = GateId::(\w+);", src)
    if not egate or egate.group(1) != "G3":
        out.append("candidate-promotion policy is not enforced at G3")
    rgate = re.search(r"PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::(\w+);", src)
    if not rgate or rgate.group(1) != "G9":
        out.append("promotion policy changes are not release-visibly qualified at G9")
    # DEC-074 binds the typed change law without restating the inventory.
    row74 = re.search(
        r'DecisionSpec \{ id: "DEC-074",.*?successor: "([^"]*)"', dsrc)
    f74 = row74.group(1) if row74 else ""
    for token in ("PromotionRequirement", "ProofPolicySurface::CandidatePromotion",
                  "spec/promotion.rs"):
        if token not in f74:
            out.append(f"DEC-074 does not name {token}; the change law must bind "
                       "the typed boundary")
    # docs/12: authored doctrine clauses plus the generated projection.
    doc12 = (root / "docs/12_TESTPAK.md").read_text(encoding="utf-8")
    authored12 = A_GENERATED_BLOCK.sub("", doc12)
    flat12 = " ".join(authored12.split())
    for fragment, label in A_PROMOTION_DOCTRINE:
        if " ".join(fragment.split()) not in flat12:
            out.append(f"docs/12 promotion doctrine no longer states: {label}")
    disq = re.search(r"Not satisfied by:(.*?)(?:\n\n|`Auditable)", authored12, re.S)
    disq_text = disq.group(1) if disq else ""
    for token in A_PROMOTION_DISQUALIFIED:
        if not re.search(r"\b" + token + r"\b", disq_text):
            out.append(f"docs/12 no longer states that {token} cannot satisfy "
                       "QualifiedHostileEvidence")
    m = re.search(
        r"<!-- PROMOTION-REQUIREMENTS:BEGIN generated from spec/promotion\.rs by "
        r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
        r"PROMOTION-REQUIREMENTS:END -->", doc12, re.S)
    if not m:
        out.append("docs/12 carries no generated PROMOTION-REQUIREMENTS block "
                   "naming spec/promotion.rs as source")
    else:
        header = f"{'requirement':<26} {'owner':<14} admission basis"
        family = [f"{'policy surface':<26} {surface.group(1) if surface else '?'}",
                  f"{'policy-change basis':<26} {change.group(1) if change else '?'}",
                  f"{'enforcement gate':<26} {egate.group(1) if egate else '?'}",
                  f"{'release-visibility gate':<26} {rgate.group(1) if rgate else '?'}"]
        expected = [header] + rows + family
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/12 PROMOTION-REQUIREMENTS omits row "
                           f"{want.split()[0]}")
            elif listed[i] != want.rstrip():
                out.append(f"docs/12 PROMOTION-REQUIREMENTS row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/12 PROMOTION-REQUIREMENTS lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # docs/24 consumes without re-owning; the retired MutationLane owner path
    # cannot return.
    doc24 = A_GENERATED_BLOCK.sub(
        "", (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8"))
    if re.search(r"MutationLane`? in `?spec/architecture\.rs", doc24):
        out.append("docs/24 points MutationLane at spec/architecture.rs; the "
                   "typed owner moved to spec/mutation.rs and left no re-export")
    if "QualifiedHostileEvidence" not in doc24 or "Rule qualification" not in doc24:
        out.append("docs/24 no longer states that QualifiedHostileEvidence "
                   "consumes its Rule qualification law")
    if re.search(r"\bIndependentEvidenceRoute\b", doc24):
        out.append("docs/24 restates the promotion-requirement inventory; it "
                   "consumes Rule qualification without re-owning the denominator")
    # docs/31 consumes the typed law and owns no second list.
    doc31 = A_GENERATED_BLOCK.sub(
        "", (root / "docs/31_FINAL_CONTRADICTION_AUDIT.md").read_text(encoding="utf-8"))
    if "PromotionRequirement::ALL" not in doc31:
        out.append("docs/31 no longer consumes spec/promotion.rs "
                   "PromotionRequirement::ALL as the promotion condition")
    for s in ("IndependentEvidenceRoute", "NamedProofTarget",
              "QualifiedHostileEvidence", "AuditablePromotionReceipt"):
        if re.search(r"\b" + s + r"\b", doc31):
            out.append(f"docs/31 restates {s}; the audit consumer owns no second "
                       "promotion inventory")
    # The raw array stays dead, and the forbidden alternate owners neither
    # declare nor re-export the type. dispositions.rs lawfully NAMES it in
    # DEC-074's amended text; naming is not ownership.
    for rel in sorted((root / "spec").glob("*.rs")):
        rsrc = _uncomment(rel.read_text(encoding="utf-8"))
        if rel.name in ("architecture.rs", "mutation.rs", "proof.rs",
                        "reconciliation.rs") \
                and re.search(r"\bPromotionRequirement\b", rsrc):
            out.append(f"spec/{rel.name} speaks PromotionRequirement; the type has "
                       "one owner module")
        if re.search(r"pub const PROMOTION_REQUIREMENTS\b", rsrc) \
                and rel.name != "promotion.rs":
            out.append(f"spec/{rel.name} resurrects the raw PROMOTION_REQUIREMENTS "
                       "array; the napkin is retired")
    return out


# --- Compiler-assumption kinds (5.5E3h) --------------------------------------
# spec/compiler_assumptions.rs owns the admitted ledgerable kinds — two
# sharp instruments, never a junk drawer. Hard violations, test allowances,
# dependency mechanisms, and not-yet-earned ideas keep their existing
# owners; the borders are proven by hostiles, not by a second census table.
def compiler_assumption_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/compiler_assumptions.rs"
    if not path.is_file():
        return ["missing spec/compiler_assumptions.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    enum_body = re.search(r"pub enum CompilerAssumptionKind \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[CompilerAssumptionKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bCompilerAssumptionKind::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"CompilerAssumptionKind::{missing} is omitted from "
                   "CompilerAssumptionKind::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"CompilerAssumptionKind::ALL names {phantom}, which the enum "
                   "does not declare")

    def fn_arms(name):
        body = re.search(
            r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        arms: dict[str, str] = {}
        if not body:
            return arms
        for arm, value in re.findall(
                r"(CompilerAssumptionKind::\w+(?:\s*\|\s*CompilerAssumptionKind::\w+)*)"
                r"\s*=>\s*([^,]+),", body.group(1), re.S):
            for v in re.findall(r"\bCompilerAssumptionKind::(\w+)", arm):
                arms[v] = value.strip()
        return arms

    spellings = fn_arms("spelling")
    owners = fn_arms("semantic_owner")
    bases = fn_arms("admission_basis")
    markers = fn_arms("requires_safety_contract_marker")
    cgates = fn_arms("classification_gate")
    rgates = fn_arms("release_qualification_gate")
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    seen: set[str] = set()
    rows: list[str] = []
    for v in inventory:
        s = spellings.get(v, "").strip('"')
        if not s.strip():
            out.append(f"CompilerAssumptionKind::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"assumption-kind spelling {s} is claimed twice")
        seen.add(s)
        owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
        oid = owner.group(1) if owner else ""
        if oid not in contract_ids:
            out.append(f"assumption kind {s} cites owner {oid}, which no declared "
                       "contract owns")
        elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
            out.append(f"assumption kind {s} cites owner {oid}, whose authoritative "
                       "document does not author the spelling")
        basis = re.search(r'GuaranteeRef::dec\("([^"]+)"\)', bases.get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"assumption kind {s} cites admission basis {bid}, which no "
                       "declared decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"assumption kind {s} cites admission basis {bid}, whose "
                           "forward-policy fields do not name the kind")
        marker = markers.get(v, "")
        if v == "UnsafeMemoryContract" and marker != "true":
            out.append("UnsafeMemoryContract must intrinsically require the "
                       "SAFETY-CONTRACT marker")
        if v != "UnsafeMemoryContract" and marker == "true":
            out.append(f"only UnsafeMemoryContract intrinsically requires the "
                       f"SAFETY-CONTRACT marker; {s} claims it")
        cg = re.search(r"GateId::(\w+)", cgates.get(v, ""))
        if not cg or cg.group(1) != "G3":
            out.append(f"assumption kind {s} classifies at "
                       f"{cg.group(1) if cg else '?'}; the ledger boundary is "
                       "classified at G3")
        rg = re.search(r"GateId::(\w+)", rgates.get(v, ""))
        if not rg or rg.group(1) != "G9":
            out.append(f"assumption kind {s} qualifies for release at "
                       f"{rg.group(1) if rg else '?'}; the release seal consumes "
                       "the ledger at G9")
        rows.append(f"{s:<22} {bid:<9} {marker:<7} "
                    f"{cg.group(1) if cg else '?':<9} {rg.group(1) if rg else '?'}")
    # docs/19 carries the fact rows; the semantic owner projects, docs/12
    # only consumes.
    doc19 = (root / "docs/19_SECURITY_MODEL.md").read_text(encoding="utf-8")
    m = re.search(
        r"<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
        r"spec/compiler_assumptions\.rs by bootstrap/project\.py; do not edit -->"
        r"\n```text\n(.*?)\n```\n<!-- COMPILER-ASSUMPTION-KINDS:END -->", doc19, re.S)
    if not m:
        out.append("docs/19 carries no generated COMPILER-ASSUMPTION-KINDS block "
                   "naming spec/compiler_assumptions.rs as source")
    else:
        header = f"{'kind':<22} {'basis':<9} {'marker':<7} {'classify':<9} release"
        expected = [header] + rows
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS omits row "
                           f"{want.split()[0]}")
            elif listed[i] != want.rstrip():
                out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/19 COMPILER-ASSUMPTION-KINDS lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # No universal assumption identity, and no other module speaks the type.
    for rel in sorted((root / "spec").glob("*.rs")):
        rsrc = _uncomment(rel.read_text(encoding="utf-8"))
        if re.search(r"pub (?:struct|enum|type) (?:CompilerAssumptionId|AssumptionKind)\b",
                     rsrc):
            out.append(f"spec/{rel.name} declares a universal assumption identity; "
                       "only earned ledgerable kinds enter CompilerAssumptionKind")
        if rel.name in ("architecture.rs", "reconciliation.rs") and re.search(
                r"\bCompilerAssumptionKind\b", rsrc):
            out.append(f"spec/{rel.name} speaks CompilerAssumptionKind; the type has "
                       "one owner module")
    return out


# --- Corpus reconciliation epoch (5.5E3g) ------------------------------------
# spec/corpus.rs owns which corpus epochs exist and which is CURRENT. The
# epoch answers "which coherent semantic corpus does this document belong
# to" — never a date, release, digest, proof freshness, or the runtime
# reconciliation posture that spec/reconciliation.rs owns under DEC-075.
def corpus_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/corpus.rs"
    if not path.is_file():
        return ["missing spec/corpus.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    enum_body = re.search(r"pub enum ReconciliationEpoch \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[ReconciliationEpoch\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bReconciliationEpoch::(\w+)", all_body.group(1)) \
        if all_body else []
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"ReconciliationEpoch::{missing} is omitted from "
                   "ReconciliationEpoch::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"ReconciliationEpoch::ALL names {phantom}, which the enum "
                   "does not declare")
    cur = re.search(
        r"pub const CURRENT_RECONCILIATION_EPOCH:\s*ReconciliationEpoch = "
        r"ReconciliationEpoch::(\w+);", src)
    if not cur:
        out.append("spec/corpus.rs declares no CURRENT_RECONCILIATION_EPOCH")
        return out
    current = cur.group(1)
    if current not in inventory:
        out.append(f"CURRENT_RECONCILIATION_EPOCH names {current}, which "
                   "ReconciliationEpoch::ALL does not declare")
    spellings = dict(re.findall(r'ReconciliationEpoch::(\w+) => "([^"]*)",', src))
    owners = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => ContractId\("([^"]+)"\)', src))
    bases = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => \{?\s*GuaranteeRef::Seed\(SeedId\("([^"]+)"\)\)',
        src))
    inv_src = _uncomment((root / "spec/invariants.rs").read_text(encoding="utf-8"))
    seen: set[str] = set()
    for v in inventory:
        s = spellings.get(v, "")
        if not s.strip():
            out.append(f"ReconciliationEpoch::{v} projects an empty spelling")
            continue
        if s in seen:
            out.append(f"epoch spelling {s} is claimed twice")
        seen.add(s)
        oid = owners.get(v, "")
        if oid not in contract_ids:
            out.append(f"epoch {s} cites owner {oid}, which no declared contract owns")
        else:
            otext = None
            for p in sorted(root.rglob("*.md")):
                rel = p.relative_to(root)
                if any(part in EXCLUDE_DIRS for part in rel.parts):
                    continue
                text = p.read_text(encoding="utf-8")
                if frontmatter(text).get("contract_id") == oid:
                    # Authored BODY only: every document's frontmatter now
                    # carries the projected epoch spelling, and metadata may
                    # not notarize its own authorship claim.
                    body = re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)
                    otext = A_GENERATED_BLOCK.sub("", body)
                    break
            for token in ("ReconciliationEpoch", s):
                if otext is None or not re.search(
                        r"(?<![A-Za-z0-9_-])" + re.escape(token) + r"(?![A-Za-z0-9_-])",
                        otext):
                    out.append(f"epoch {s} cites owner {oid}, whose authoritative "
                               f"document does not author {token}")
        basis = bases.get(v, "")
        row = re.search(r'id: "' + re.escape(basis) + r'", statement: "([^"]*)"', inv_src)
        if not row:
            out.append(f"epoch {s} cites admission basis {basis}, which no declared "
                       "seed row owns")
        elif "ReconciliationEpoch" not in row.group(1):
            out.append(f"{basis} does not name ReconciliationEpoch in its statement; "
                       "the admission basis must name what it admits")
    # Every eligible document claims the CURRENT corpus epoch; standalone
    # generated documents bind their SOURCE epoch. A date never substitutes.
    current_spelling = spellings.get(current, "")
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        meta = frontmatter(p.read_text(encoding="utf-8"))
        key = ("source_reconciliation_epoch" if meta.get("status") == "GENERATED"
               else "reconciliation_epoch")
        got = meta.get(key)
        if got is None:
            out.append(f"{rel.as_posix()}: declares no {key}; every eligible "
                       "document claims the current corpus epoch")
            continue
        if key == "reconciliation_epoch" and "last_reconciled" not in meta:
            out.append(f"{rel.as_posix()}: last_reconciled is deleted; the epoch "
                       "answers WHICH corpus, the date answers WHEN reviewed, and "
                       "both stand")
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}", got):
            out.append(f"{rel.as_posix()}: a date ({got}) cannot substitute for a "
                       "corpus epoch")
        elif got != current_spelling:
            out.append(f"{rel.as_posix()}: claims corpus epoch {got}; the current "
                       f"corpus epoch is {current_spelling}")
    # spec/reconciliation.rs owns runtime execution reconciliation, never the
    # corpus epoch.
    recon = _uncomment((root / "spec/reconciliation.rs").read_text(encoding="utf-8"))
    if re.search(r"\bReconciliationEpoch\b", recon):
        out.append("spec/reconciliation.rs speaks ReconciliationEpoch; the runtime "
                   "reconciliation module does not own the corpus epoch")
    # The docs/00 fact block: exact provenance and ordered content.
    doc00 = (root / "docs/00_CONSTITUTION.md").read_text(encoding="utf-8")
    m = re.search(
        r"<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from spec/corpus\.rs by "
        r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
        r"CORPUS-RECONCILIATION-EPOCH:END -->", doc00, re.S)
    if not m:
        out.append("docs/00 carries no generated CORPUS-RECONCILIATION-EPOCH block "
                   "naming spec/corpus.rs as source")
    else:
        want = [f"{'current epoch':<18} {current_spelling}",
                f"{'semantic owner':<18} {owners.get(current, '')}",
                f"{'admission basis':<18} {bases.get(current, '')}"]
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        if listed != want:
            out.append(f"docs/00 CORPUS-RECONCILIATION-EPOCH states {listed}; the "
                       f"typed owner states {want}")
    return out


# --- Mutation vocabulary (5.5E3f) --------------------------------------------
# spec/mutation.rs owns the lanes and result classifications as total const
# functions. This auditor independently parses the fn arms, executes the
# semantic fences, reconstructs the two docs/12 fact tables, and refuses the
# lettered lane nicknames anywhere in authoritative prose.
A_LANE_ALIAS = re.compile(r"\bLane\s+[ABC]\b")


def _mutation_arms(src: str, enum: str, name: str) -> dict[str, str]:
    impl = re.search(r"impl " + enum + r" \{(.*?)\n\}", src, re.S)
    body = re.search(
        r"pub const fn " + name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
        impl.group(1) if impl else "", re.S)
    arms: dict[str, str] = {}
    if not body:
        return arms
    for arm, value in re.findall(
            r"(" + enum + r"::\w+(?:\s*\|\s*" + enum + r"::\w+)*)\s*=>\s*([^,]+),",
            body.group(1), re.S):
        for v in re.findall(r"\b" + enum + r"::(\w+)", arm):
            arms[v] = value.strip()
    return arms


def mutation_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/mutation.rs"
    if not path.is_file():
        return ["missing spec/mutation.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)
    dsrc = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    dec_ids = {m[0] for m in G_DEC_ROW.findall(dsrc)}
    inventories: dict[str, list[str]] = {}
    for enum in ("MutationLane", "MutationResult"):
        enum_body = re.search(r"pub enum " + enum + r" \{(.*?)\n\}", src, re.S)
        variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) \
            if enum_body else []
        all_body = re.search(
            r"pub const ALL: &'static \[" + enum + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + enum + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"{enum}::{missing} is omitted from {enum}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"{enum}::ALL names {phantom}, which the enum does not declare")
        inventories[enum] = inventory
        spellings = _mutation_arms(src, enum, "spelling")
        owners = _mutation_arms(src, enum, "semantic_owner")
        seen: set[str] = set()
        for v in inventory:
            s = spellings.get(v, "").strip('"')
            if not s.strip():
                out.append(f"{enum}::{v} projects an empty spelling")
                continue
            if s in seen:
                out.append(f"{enum} spelling {s} is claimed twice")
            seen.add(s)
            owner = re.search(r'ContractId\("([^"]+)"\)', owners.get(v, ""))
            oid = owner.group(1) if owner else ""
            if oid not in contract_ids:
                out.append(f"{enum} {s} cites owner {oid}, which no declared "
                           "contract owns")
            elif not re.search(r"\b" + re.escape(s) + r"\b", owner_text.get(oid, "")):
                out.append(f"{enum} {s} cites owner {oid}, whose authoritative "
                           "document does not author the spelling")
    # Lane facts: the semantic fences, executed against the parsed arms.
    lane_fields = {name: _mutation_arms(src, "MutationLane", name) for name in (
        "spelling", "admission_basis", "requires_per_candidate_rust_compile",
        "requires_real_rustc_semantics", "requires_activation_evidence",
        "permits_production_profile_slots", "requires_independent_evidence_route",
        "gates")}
    lane_rows: list[str] = []
    for v in inventories.get("MutationLane", []):
        s = lane_fields["spelling"].get(v, "").strip('"')
        basis = re.search(r'DecisionId\("([^"]+)"\)',
                          lane_fields["admission_basis"].get(v, ""))
        bid = basis.group(1) if basis else ""
        if bid not in dec_ids:
            out.append(f"lane {s} cites admission basis {bid}, which no declared "
                       "decision owns")
        else:
            row = re.search(
                r'DecisionSpec \{ id: "' + re.escape(bid)
                + r'",.*?subject: "([^"]*)", successor: "([^"]*)",'
                + r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
            fields = " ".join(g or "" for g in row.groups()) if row else ""
            if not re.search(r"\b" + re.escape(s) + r"\b", fields):
                out.append(f"{bid} forward-policy fields do not name lane {s}; "
                           "the admission boundary must name what it admits")
        pc = lane_fields["requires_per_candidate_rust_compile"].get(v, "")
        rr = lane_fields["requires_real_rustc_semantics"].get(v, "")
        act = lane_fields["requires_activation_evidence"].get(v, "")
        slots = lane_fields["permits_production_profile_slots"].get(v, "")
        ind = lane_fields["requires_independent_evidence_route"].get(v, "")
        gates = re.findall(r"GateId::(\w+)", lane_fields["gates"].get(v, ""))
        if act != "true":
            out.append(f"lane {s} does not require activation evidence; a green "
                       "test with a dormant mutant is not evidence")
        if slots != "false":
            out.append(f"lane {s} permits production-profile mutation slots")
        if ind != "true":
            out.append(f"lane {s} does not require an independent evidence route")
        if gates != ["G3"]:
            out.append(f"lane {s} gates {gates} are not exactly G3")
        if v == "SemanticIr":
            if rr != "false":
                out.append("SemanticIr claims real rustc semantics; the reference "
                           "interpreter lane must not")
            if pc != "false":
                out.append("SemanticIr requires a per-candidate Rust compile; the "
                           "semantic lane compiles nothing")
        if v == "SelectableCompiled":
            if rr != "true":
                out.append("SelectableCompiled must run under real rustc semantics")
            if pc != "false":
                out.append("SelectableCompiled must not require a per-candidate "
                           "Rust compile; one shard compiles once")
        if v == "CompilerBacked":
            if pc != "true":
                out.append("CompilerBacked must require a per-candidate Rust compile")
            if rr != "true":
                out.append("CompilerBacked must run under real rustc semantics")
        lane_rows.append(f"{s:<20} {pc:<7} {rr:<7} {act:<7} {slots:<7} {ind:<7} "
                         f"{' '.join(gates)}".rstrip())
    # Result classification: only Killed kills, only Survived survives, every
    # result stays in the denominator.
    result_fields = {name: _mutation_arms(src, "MutationResult", name) for name in (
        "spelling", "counts_as_kill", "counts_as_survival", "appears_in_denominator")}
    result_rows: list[str] = []
    for v in inventories.get("MutationResult", []):
        s = result_fields["spelling"].get(v, "").strip('"')
        kill = result_fields["counts_as_kill"].get(v, "")
        surv = result_fields["counts_as_survival"].get(v, "")
        denom = result_fields["appears_in_denominator"].get(v, "")
        if v == "Killed" and kill != "true":
            out.append("Killed must count as a kill")
        if v != "Killed" and kill == "true":
            out.append(f"only Killed counts as a kill; {s} claims one")
        if v == "Survived" and surv != "true":
            out.append("Survived must count as survival")
        if v != "Survived" and surv == "true":
            out.append(f"only Survived counts as survival; {s} claims one")
        if denom != "true":
            out.append(f"{s} leaves the denominator; nothing exits silently to "
                       "improve a score")
        result_rows.append(f"{s:<22} {kill:<7} {surv:<7} {denom}")
    # docs/12 fact tables: exact ordered content plus provenance.
    doc12 = (root / "docs/12_TESTPAK.md").read_text(encoding="utf-8")
    lane_header = (f"{'lane':<20} {'compile':<7} {'rustc':<7} {'activ':<7} "
                   f"{'slots':<7} {'indep':<7} gates")
    result_header = f"{'result':<22} {'kill':<7} {'surv':<7} denominator"
    for marker, expected in (("MUTATION-LANES", [lane_header] + lane_rows),
                             ("MUTATION-RESULTS", [result_header] + result_rows)):
        m = re.search(
            r"<!-- " + marker + r":BEGIN generated from spec/mutation\.rs by "
            r"bootstrap/project\.py; do not edit -->\n```text\n(.*?)\n```\n<!-- "
            + marker + r":END -->", doc12, re.S)
        if not m:
            out.append(f"docs/12 carries no generated {marker} block naming "
                       "spec/mutation.rs as source")
            continue
        listed = [line.rstrip() for line in m.group(1).splitlines() if line.strip()]
        expected = [line.rstrip() for line in expected]
        for i, want in enumerate(expected):
            if i >= len(listed):
                out.append(f"docs/12 {marker} omits row {want.split()[0]}")
            elif listed[i] != want:
                out.append(f"docs/12 {marker} row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(want.split())} at that position")
        for extra in listed[len(expected):]:
            out.append(f"docs/12 {marker} lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    # The lettered nicknames are dead in authoritative prose.
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        for m in A_LANE_ALIAS.finditer(A_GENERATED_BLOCK.sub("", text)):
            out.append(f"{rel.name} cites the letter alias {m.group(0)!r} in "
                       "authoritative prose; the lanes are SemanticIr, "
                       "SelectableCompiled, and CompilerBacked")
    return out


# --- Command namespaces (5.5E3e) ---------------------------------------------
# spec/commands.rs owns three separate closed namespaces and each entry's
# typed authority relation. A command's namespace identity and its invoked
# semantic authority are separate axes: Direct owners own the command-level
# operation; a Composite's composition owner owns ONLY orchestration and
# routing, and composition never transfers ownership.
A_COMMAND_NAMESPACES = (
    ("PRODUCT-COMMANDS", "ProductCommand", "docs/26_COMMAND_PLANE.md"),
    ("TESTPAK-COMMANDS", "TestPakCommand", "docs/26_COMMAND_PLANE.md"),
    ("BATQL-SOURCE-MODES", "BatQlSourceMode", "companion/BATQL_LANGUAGE.md"),
)


def _command_block_body(name: str, doc: str):
    m = re.search(
        r"<!-- " + re.escape(name)
        + r":BEGIN generated from spec/commands\.rs by bootstrap/project\.py; "
        + r"do not edit -->\n```text\n(.*?)\n```\n<!-- "
        + re.escape(name) + r":END -->", doc, re.S)
    return m.group(1) if m else None


def command_catalog_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / "spec/commands.rs"
    if not path.is_file():
        return ["missing spec/commands.rs"]
    src = _uncomment(path.read_text(encoding="utf-8"))
    contract_ids = declared_contract_ids(root)
    owner_text: dict[str, str] = {}
    for p in sorted(root.rglob("*.md")):
        rel = p.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = p.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            owner_text[cid] = A_GENERATED_BLOCK.sub("", text)

    def owner_authors(cid: str, term: str) -> bool:
        return bool(re.search(r"\b" + re.escape(term) + r"\b", owner_text.get(cid, "")))

    # No universal CommandId anywhere in the public spec: command identity
    # lives in three separate namespaces or nowhere.
    for rel in sorted((root / "spec").glob("*.rs")):
        if re.search(r"pub (?:struct|enum|type) CommandId\b",
                     _uncomment(rel.read_text(encoding="utf-8"))):
            out.append(f"spec/{rel.name} declares a universal CommandId; command "
                       "identity lives in three separate namespaces or nowhere")
    # The composition boundary is DECLARED in authored docs/26 prose, and the
    # typed authority must agree in both directions — a command cannot
    # silently collapse to a single sovereign or silently gain a composite.
    doc26 = (root / "docs/26_COMMAND_PLANE.md").read_text(encoding="utf-8")
    comp_sec = re.search(r"## Composite commands\n(.*?)(?:\n## |\Z)",
                         A_GENERATED_BLOCK.sub("", doc26), re.S)
    declared_composites: set[str] = set()
    if not comp_sec:
        out.append("docs/26 carries no authored Composite commands section "
                   "describing the composition boundary")
    else:
        declared_composites = set(re.findall(r"`(\w+)`", comp_sec.group(1)))
    companion = (root / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")
    docs_of = {"docs/26_COMMAND_PLANE.md": doc26,
               "companion/BATQL_LANGUAGE.md": companion}
    for marker, kind, docrel in A_COMMAND_NAMESPACES:
        all_body = re.search(
            r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
        inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        if not inventory:
            out.append(f"spec/commands.rs declares no {kind} inventory")
            continue
        enum_body = re.search(r"pub enum " + kind + r" \{(.*?)\n\}", src, re.S)
        variants = re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M) if enum_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"{kind}::{missing} is omitted from {kind}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"{kind}::ALL names {phantom}, which the enum does not declare")
        tokens: dict[str, str] = {}
        for arm, tok in re.findall(
                r"\b" + kind + r"::(\w+(?:\s*\|\s*" + kind + r"::\w+)*) => \"([^\"]+)\",",
                src):
            for v in re.split(r"\s*\|\s*", arm):
                tokens[v.split("::")[-1]] = tok
        authority: dict[str, tuple[str, str, list[str]]] = {}
        for arm_body, direct, comp in re.findall(
                r"(" + kind + r"::\w+(?:\s*\|\s*" + kind + r"::\w+)*)\s*=>\s*\{?\s*"
                r"(?:CommandAuthority::Direct\(ContractId\(\"([^\"]+)\"\)\)"
                r"|CommandAuthority::(Composite \{.*?\}))", src, re.S):
            if direct:
                entry = ("direct", direct, [])
            else:
                owner = re.search(r'composition_owner: ContractId\("([^"]+)"\)', comp)
                delegates = re.findall(r'ContractId\("([^"]+)"\)',
                                       comp.split("delegates:", 1)[-1])
                entry = ("composite", owner.group(1) if owner else "", delegates)
            for v in re.findall(r"\b" + kind + r"::(\w+)", arm_body):
                authority[v] = entry
        seen_tokens: set[str] = set()
        rows: list[str] = []
        for v in inventory:
            if v not in tokens or v not in authority:
                out.append(f"{kind}::{v} declares no token or no authority relation")
                continue
            token = tokens[v]
            shape, owner, delegates = authority[v]
            rows.append(f"{token:<10} {shape:<10} {owner:<28} "
                        f"{' '.join(delegates)}".rstrip())
            if not token.strip():
                out.append(f"{kind}::{v} projects an empty token")
            if token in seen_tokens:
                out.append(f"{kind} token {token} is claimed twice; tokens are "
                           "unique within their namespace")
            seen_tokens.add(token)
            if kind != "BatQlSourceMode" and token.lower() in ("ask", "do"):
                out.append(f"{kind} admits {token}: ASK and DO are language modes, "
                           "never CLI verbs, and DO is never a top-level command")
            if shape == "direct":
                if owner == "BP-COMMAND-PLANE-1":
                    out.append(f"{kind} {token} is Direct-owned by BP-COMMAND-PLANE-1, "
                               "which may own command composition but never the "
                               "command-level meaning")
                if owner not in contract_ids:
                    out.append(f"{kind} {token} cites owner {owner}, which no "
                               "declared contract owns")
                elif not owner_authors(owner, token):
                    out.append(f"{kind} {token} cites owner {owner}, whose "
                               "authoritative document does not author the token")
            else:
                if owner not in contract_ids:
                    out.append(f"{kind} {token} cites composition owner {owner}, "
                               "which no declared contract owns")
                elif not owner_authors(owner, token):
                    out.append(f"{kind} {token} cites composition owner {owner}, "
                               "whose authoritative document does not author the token")
                if not delegates:
                    out.append(f"{kind} {token} is a composite with no delegates; "
                               "an orchestration boundary with nothing to route "
                               "to is refused")
                if len(set(delegates)) != len(delegates):
                    out.append(f"{kind} {token} repeats a delegate; each delegate "
                               "retains its own law exactly once")
                if owner in delegates:
                    out.append(f"{kind} {token} lists its composition owner as its "
                               "own delegate; composition never transfers ownership")
                for d in delegates:
                    if d not in contract_ids:
                        out.append(f"{kind} {token} delegates to {d}, which no "
                                   "declared contract owns")
            if kind == "ProductCommand":
                if shape == "composite" and token not in declared_composites:
                    out.append(f"docs/26 does not declare {token} a composition "
                               "boundary; the typed authority may not silently "
                               "gain a composite")
                if shape == "direct" and token in declared_composites:
                    out.append(f"docs/26 declares {token} a composition boundary; "
                               f"a Direct owner would collapse it to a single "
                               "sovereign")
        body = _command_block_body(marker, docs_of[docrel])
        if body is None:
            out.append(f"{docrel} carries no generated {marker} block naming "
                       "spec/commands.rs as source")
            continue
        listed = [line.rstrip() for line in body.splitlines() if line.strip()]
        for i, expect in enumerate(rows):
            if i >= len(listed):
                out.append(f"{docrel} {marker} omits row {expect.split()[0]}")
            elif listed[i] != expect:
                out.append(f"{docrel} {marker} row {i + 1} states "
                           f"{' '.join(listed[i].split())}; the typed catalog states "
                           f"{' '.join(expect.split())} at that position")
        for extra in listed[len(rows):]:
            out.append(f"{docrel} {marker} lists {extra.split()[0]}, "
                       "which the typed catalog does not project")
    return out


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
        # Plane files derive from the typed inventory (5.5E5); the raw
        # SYNCBAT_REQUIRED_PLANES path table retired with it.
        arm = re.search(r"pub const fn module_name\(self\)[^{]*\{(.*?)\n    \}",
                        source, re.S)
        for module in re.findall(r'=>\s*"([^"]+)"', arm.group(1)) if arm else []:
            for relative in (f"src/{module}.rs", f"src/{module}"):
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
            r'LegacyObligation \{ id: "([^"]+)", law: "([^"]+)", '
            r'legacy_evidence: "(?:[^"\\]|\\.)*", clean_owner: "([^"]+)", '
            r'mechanism_disposition: "(?:[^"\\]|\\.)*", witness_requirement: '
            r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
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

"""BatQL operator surface and numeric-semantics audit.

The auditor half of the operator generator/auditor pair: it reparses
spec/operators.rs independently, checks grammar closure, proof disposition,
the numeric contract (DEC-069 / docs/37), and the phantom-sort ban. Shares no
parsing with bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .corpus import (
    A_GENERATED_BLOCK,
    G_DEC_ROW,
    _uncomment,
    batql_extract_block,
    frontmatter,
)


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




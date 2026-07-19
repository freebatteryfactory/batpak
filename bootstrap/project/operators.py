"""BatQL operator projections (5.5E3j) and the typing legality matrix (5.5E1).

Parses spec/operators.rs through the typed inventories and serializes the six
operator views plus the typed legality matrix. Standard library only; shares
no parsing with bootstrap/audit.py.
"""
from __future__ import annotations

import re

from .guarantees import Unadmitted


COMPANION = "companion/BATQL_LANGUAGE.md"

# Typed operator parsing (5.5E3j). Every OperatorSpec row is resolved through
# the typed OperatorId, word-surface, and symbol-surface inventories and their
# token arms. There is no Python token map, no Python class-ranking table, and
# no raw-string fallback: a variant the typed source does not declare fails
# generation instead of rendering itself. Canonical projection order comes
# from OperatorId::ALL — the one canonical order.
OPERATOR_ROW = re.compile(
    r'OperatorSpec \{ id: OperatorId::(\w+), class: OperatorClass::(\w+), '
    r'syntax: OperatorSyntax::(\w+)\(([^)]*)\), semantic_op: "([^"]*)", '
    r'arity: Arity::(\w+), fixity: Fixity::(\w+), precedence: (\d+), '
    r'associativity: Associativity::(\w+), typing: &\[([^\]]*)\], input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
_SHAPE_DISPLAY = {"SymbolOnly": "symbol-only", "WordOnly": "word-only",
                  "WordWithSymbolAlias": "word-with-symbol"}


def typing_rules(op: dict[str, str]) -> list[str]:
    """The operator's closed legality rules, in authored match order."""
    return re.findall(r"OperatorTypingRule::(\w+)", op["typing"])


def _operator_all(source: str, type_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", source, re.S)
    if not body:
        raise SystemExit(f"project: spec/operators.rs declares no {type_name}::ALL inventory")
    # \b keeps e.g. OperatorSymbolSurface::X from satisfying OperatorId::X-style
    # prefixes of longer type names.
    return re.findall(r"\b" + type_name + r"::(\w+)", body.group(1))


def _operator_token_arms(source: str, type_name: str, fn_name: str) -> dict[str, str]:
    """Variant -> token arms of one const fn, scoped to the type's impl block
    so another enum's same-named fn can never answer for it."""
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", source, re.S)
    body = impl.group(1) if impl else ""
    fn = re.search(
        r"pub const fn " + fn_name
        + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        body, re.S)
    if not fn:
        raise SystemExit(
            f"project: spec/operators.rs declares no {type_name}::{fn_name} arms")
    return dict(re.findall(r"\b" + type_name + r'::(\w+) => "([^"]*)",', fn.group(1)))


def parse_operators(root: Path) -> list[dict[str, str]]:
    """OperatorSpec rows resolved through the typed inventories, returned in
    OperatorId::ALL order."""
    source = (root / "spec/operators.rs").read_text(encoding="utf-8")
    id_all = _operator_all(source, "OperatorId")
    id_spelling = _operator_token_arms(source, "OperatorId", "spelling")
    word_token = _operator_token_arms(source, "OperatorWordSurface", "token")
    symbol_token = _operator_token_arms(source, "OperatorSymbolSurface", "token")
    rows: dict[str, dict[str, str]] = {}
    for match in OPERATOR_ROW.findall(source):
        (variant, cls, shape, inner, semantic_op, arity, fixity, precedence,
         associativity, typing, input_sorts, result_sort, exactness, overflow,
         exception, spoken, mutation_classes, numeric_support) = match
        if variant not in id_spelling:
            raise SystemExit(
                f"project: OperatorSpec row names unknown OperatorId::{variant}")
        if variant in rows:
            raise SystemExit(f"project: duplicate OperatorSpec row for OperatorId::{variant}")
        words = re.findall(r"OperatorWordSurface::(\w+)", inner)
        symbols = re.findall(r"OperatorSymbolSurface::(\w+)", inner)
        for word in words:
            if word not in word_token:
                raise SystemExit(
                    f"project: OperatorSpec row names unknown OperatorWordSurface::{word}")
        for symbol in symbols:
            if symbol not in symbol_token:
                raise SystemExit(
                    f"project: OperatorSpec row names unknown OperatorSymbolSurface::{symbol}")
        if shape == "SymbolOnly" and not words and len(symbols) == 1:
            canonical_token, alias = symbol_token[symbols[0]], ""
        elif shape == "WordOnly" and len(words) == 1 and not symbols:
            canonical_token, alias = word_token[words[0]], ""
        elif shape == "WordWithSymbolAlias" and len(words) == 1 and len(symbols) == 1:
            canonical_token, alias = word_token[words[0]], symbol_token[symbols[0]]
        else:
            raise SystemExit(
                f"project: OperatorSpec row for {variant} carries unresolvable "
                f"syntax {shape}({inner})")
        rows[variant] = {
            "id": id_spelling[variant], "class": cls, "shape": shape,
            "canonical": canonical_token, "alias": alias,
            "semantic_op": semantic_op, "arity": arity, "fixity": fixity,
            "precedence": precedence, "associativity": associativity,
            "typing": typing, "input_sorts": input_sorts,
            "result_sort": result_sort, "exactness": exactness,
            "overflow": overflow, "exception": exception, "spoken": spoken,
            "mutation_classes": mutation_classes,
            "numeric_support": numeric_support,
        }
    for variant in rows:
        if variant not in id_all:
            raise SystemExit(
                f"project: OperatorSpec row {variant} is missing from OperatorId::ALL")
    for variant in id_all:
        if variant not in rows:
            raise SystemExit(f"project: OperatorId::{variant} has no OperatorSpec row")
    return [rows[variant] for variant in id_all]


def render_catalog(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Class | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for op in ops:
        lines.append(
            f"| {op['id']} | {op['class']} | {op['arity']} | {op['fixity']} | "
            f"{op['precedence']} | {op['associativity']} | {op['result_sort']} | {op['exactness']} |"
        )
    return "\n".join(lines)


def render_surfaces(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Shape | Canonical | Accepted alias |",
        "| --- | --- | --- | --- |",
    ]
    for op in ops:
        alias = f"`{op['alias']}`" if op["alias"] else "-"
        lines.append(
            f"| {op['id']} | {_SHAPE_DISPLAY[op['shape']]} | `{op['canonical']}` | {alias} |"
        )
    return "\n".join(lines)


def render_grammar(ops: list[dict[str, str]]) -> str:
    comparison = [op["canonical"] for op in ops if op["class"] == "Comparison"]
    comparison += [f'"{op["alias"]}"' for op in ops
                   if op["class"] == "Comparison" and op["alias"]]
    # arithmetic canonical spelling is the symbol itself; quote it as a terminal
    arithmetic = [f'"{op["canonical"]}"' for op in ops if op["class"] == "Arithmetic"]
    logical = [op["canonical"] for op in ops
               if op["class"] == "Logical" and op["fixity"] == "Infix"]
    body = [
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
    ]
    return "\n".join(body)


def render_projection(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Canonical formatting | Spoken | Legal mutation classes |",
        "| --- | --- | --- | --- |",
    ]
    for op in ops:
        lines.append(
            f"| {op['id']} | `{op['canonical']}` | {op['spoken']} | {op['mutation_classes']} |")
    return "\n".join(lines)


def render_numeric(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Class | Numeric support |",
        "| --- | --- | --- |",
    ]
    for op in ops:
        lines.append(f"| {op['id']} | {op['class']} | {op['numeric_support']} |")
    return "\n".join(lines)


# The §5.2a legality matrix, projected from each operator's typed rules
# (OperatorTypingRule, 5.5E1). Rendering is keyed on (rule, surface): the rows
# an enum variant projects are fixed here, so a new legality shape requires a
# new typed rule, never a hand-authored exception line. `DecimalMoney` and
# `SignedDuration` were born in the hand matrix this replaces.
def _typing_rows(rule: str, word: str) -> list[str]:
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
    raise Unadmitted(f"no projection is defined for typing rule {rule}")


def render_typing(ops: list[dict[str, str]]) -> str:
    titles = {"OP-ADD": "Addition", "OP-SUB": "Subtraction",
              "OP-MUL": "Multiplication", "OP-DIV": "Division"}
    lines: list[str] = []
    for op in ops:
        if op["id"] not in titles:
            continue
        rules = typing_rules(op)
        if not rules:
            raise Unadmitted(f"{op['id']} declares no typing rules")
        lines += [f"### {titles[op['id']]} (`{op['canonical']}`)", "", "```text"]
        for rule in rules:
            lines += _typing_rows(rule, op["canonical"])
        lines += ["```", ""]
    lines += ["### Comparison and truth", "", "```text",
              "comparisons        exact same-sort pair -> Truth with TypedMargin",
              "NOT                Truth -> Truth (K3 total)",
              "AND / OR           Truth x Truth -> Truth (K3 total)", "```"]
    return "\n".join(lines)

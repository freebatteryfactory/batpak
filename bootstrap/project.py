#!/usr/bin/env python3
"""Deterministic projection generator for the clean-room specification.

In 5.5B1 this projects `spec/operators.rs` (OperatorSpec) into the generated
operator blocks in `companion/BATQL_LANGUAGE.md`:

    OPERATORS-CATALOG      precedence / fixity / associativity / sort table
    OPERATORS-GRAMMAR      comparison / arithmetic / logical operator productions
    OPERATORS-PROJECTION   formatting / spoken / mutation-class projection

`project.py --write`  rewrites the designated generated blocks in place.
`project.py --check`  regenerates and fails if any block has drifted.

This is the GENERATOR half of the generator/auditor pair. `bootstrap/audit.py`
is the read-only AUDITOR: it recomputes the same projection independently and
compares. The two share no parsing, normalization, projection-building, or
comparison logic — only the inert marker names, whose values are independently
asserted on both sides. project.py never generates DEC-060 itself, and does not
generate the Guarantee Graph (that arrives with 5.5C1).

Standard library only. All writes are UTF-8 with literal LF endings.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

COMPANION = "companion/BATQL_LANGUAGE.md"

OPERATOR_ROW = re.compile(
    r'OperatorSpec \{ id: "([^"]*)", class: OperatorClass::(\w+), '
    r'word_surface: "([^"]*)", symbol_surface: "([^"]*)", semantic_op: "([^"]*)", '
    r'arity: Arity::(\w+), fixity: Fixity::(\w+), precedence: (\d+), '
    r'associativity: Associativity::(\w+), input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", formatting: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)" \}'
)
FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "input_sorts", "result_sort", "exactness",
    "overflow", "exception", "formatting", "spoken", "mutation_classes",
)


CLASS_RANK = {"Arithmetic": 0, "Comparison": 1, "Logical": 2}


def canonical(ops: list[dict[str, str]]) -> list[dict[str, str]]:
    # Canonical projection order is (class, id), independent of source array
    # order, so reordering spec/operators.rs never changes generated output.
    return sorted(ops, key=lambda o: (CLASS_RANK.get(o["class"], 9), o["id"]))


def parse_operators(root: Path) -> list[dict[str, str]]:
    source = (root / "spec/operators.rs").read_text(encoding="utf-8")
    return [dict(zip(FIELDS, match)) for match in OPERATOR_ROW.findall(source)]


def render_catalog(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Class | Word | Symbol | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for op in canonical(ops):
        symbol = f"`{op['symbol']}`" if op["symbol"] else ""
        lines.append(
            f"| {op['id']} | {op['class']} | `{op['word']}` | {symbol} | {op['arity']} | "
            f"{op['fixity']} | {op['precedence']} | {op['associativity']} | {op['result_sort']} | {op['exactness']} |"
        )
    return "\n".join(lines)


def render_grammar(ops: list[dict[str, str]]) -> str:
    ops = canonical(ops)

    def words(cls: str) -> list[str]:
        return [op["word"] for op in ops if op["class"] == cls]

    def symbols(cls: str) -> list[str]:
        return [f'"{op["symbol"]}"' for op in ops if op["class"] == cls and op["symbol"]]

    comparison = words("Comparison") + symbols("Comparison")
    # arithmetic canonical spelling is the symbol itself; quote it as a terminal
    arithmetic = [f'"{op["word"]}"' for op in ops if op["class"] == "Arithmetic"]
    logical = [op["word"] for op in ops if op["class"] == "Logical" and op["fixity"] == "Infix"]
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
        "| OperatorId | Formatting | Spoken | Legal mutation classes |",
        "| --- | --- | --- | --- |",
    ]
    for op in canonical(ops):
        lines.append(f"| {op['id']} | `{op['formatting']}` | {op['spoken']} | {op['mutation_classes']} |")
    return "\n".join(lines)


BLOCKS = {
    "OPERATORS-CATALOG": render_catalog,
    "OPERATORS-GRAMMAR": render_grammar,
    "OPERATORS-PROJECTION": render_projection,
}


def block_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r"(<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n)(.*?)(\n<!-- " + re.escape(name) + r":END -->)",
        re.S,
    )


def apply_blocks(text: str, ops: list[dict[str, str]]) -> tuple[str, list[str]]:
    """Return (rewritten_text, missing_block_names)."""
    missing: list[str] = []
    for name, render in BLOCKS.items():
        pattern = block_pattern(name)
        body = render(ops)
        if not pattern.search(text):
            missing.append(name)
            continue
        text = pattern.sub(lambda m: m.group(1) + body + m.group(3), text)
    return text, missing


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in ("--write", "--check"):
        print("usage: project.py --write|--check [root]", file=sys.stderr)
        return 2
    mode = sys.argv[1]
    root = Path(sys.argv[2] if len(sys.argv) > 2 else ".").resolve()
    ops = parse_operators(root)
    findings: list[str] = []
    if not ops:
        findings.append("spec/operators.rs: no OperatorSpec rows parsed")
    path = root / COMPANION
    original = path.read_text(encoding="utf-8")
    rewritten, missing = apply_blocks(original, ops)
    for name in missing:
        findings.append(f"{COMPANION}: missing generated block markers for {name}")
    if findings:
        print(f"project: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    if mode == "--write":
        if rewritten != original:
            path.write_bytes(rewritten.encode("utf-8"))
            print(f"project: WROTE {len(BLOCKS)} operator blocks ({len(ops)} operators)")
        else:
            print(f"project: OK ({len(BLOCKS)} operator blocks already current)")
        return 0
    # --check
    if rewritten != original:
        print("project: FAIL (generated operator blocks are stale; run project.py --write)", file=sys.stderr)
        return 1
    print(f"project: PASS ({len(BLOCKS)} operator blocks current; {len(ops)} operators)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

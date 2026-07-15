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
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "input_sorts", "result_sort", "exactness",
    "overflow", "exception", "formatting", "spoken", "mutation_classes",
    "numeric_support",
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


def render_numeric(ops: list[dict[str, str]]) -> str:
    lines = [
        "| OperatorId | Class | Numeric support |",
        "| --- | --- | --- |",
    ]
    for op in canonical(ops):
        lines.append(f"| {op['id']} | {op['class']} | {op['numeric_support']} |")
    return "\n".join(lines)


# --- Guarantee graph (DEC-070) -----------------------------------------------
GUARANTEE_DOC = "docs/GUARANTEE_GRAPH.generated.md"
SEED_DOC = "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md"
FAMILY_RANK = {"SEED": 0, "LEG": 1, "DEC": 2, "ARCH": 3, "QUAL": 4}
GUARANTEE_FRONT = (
    "---\n"
    "status: GENERATED\n"
    "authority_scope: derived structural index only\n"
    "generated_by: bootstrap/project.py\n"
    "generated_from: typed SEED, LEG, DEC, architecture, and qualification fact families\n"
    "do_not_edit: true\n"
    "---\n"
)
_SEED_ROW = re.compile(
    r'InvariantSpec \{ id: "([^"]+)", statement: "((?:[^"\\]|\\.)*)", '
    r'kind: GuaranteeKind::(\w+), lifetime: GuaranteeLifetime::(\w+), '
    r'owner: "([^"]*)", gates: "([^"]*)", witness: "([^"]*)", '
    r'failure_disposition: "([^"]*)", derives_from: &\[([^\]]*)\], '
    r'refines: &\[([^\]]*)\], discharges: &\[([^\]]*)\], supersedes: &\[([^\]]*)\] \}'
)
_LEG_ROW = re.compile(
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", clean_owner: "([^"]+)", '
    r'gate: "([^"]+)", compatibility_disposition: CompatibilityDisposition::\w+, '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: ObligationStatus::(\w+) \}'
)
_DEC_ROW = re.compile(r'DecisionSpec \{ id: "(DEC-\d+)", disposition: Disposition::(\w+), subject: "[^"]+"')
_PKG_ROW = re.compile(
    r'PackageSpec \{\s*package: "([^"]+)",\s*path: "[^"]+",\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: "([^"]+)",\s*profile: "([^"]+)",\s*'
    r'target: "([^"]+)",\s*requirement: "[^"]+",\s*\}', re.S)


def _ids(raw):
    return re.findall(r'"([^"]+)"', raw)


def guarantee_seed(root):
    src = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    out = []
    for m in _SEED_ROW.findall(src):
        out.append({
            "id": m[0], "statement": m[1], "kind": m[2], "lifetime": m[3],
            "owner": m[4], "gates": m[5], "witness": m[6], "failure": m[7],
            "DerivesFrom": _ids(m[8]), "Refines": _ids(m[9]),
            "Discharges": _ids(m[10]), "Supersedes": _ids(m[11]),
        })
    return out


def guarantee_nodes(root):
    nodes = []
    for s in guarantee_seed(root):
        nodes.append({"id": s["id"], "family": "SEED", "kind": s["kind"], "lifetime": s["lifetime"],
                      "owner": s["owner"], "gates": s["gates"], "witness": s["witness"],
                      "failure": f"Seed({s['failure']})"})
    leg = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    for lid, owner, gate, deletion, status in _LEG_ROW.findall(leg):
        if status == "Closed":
            life = "ClosedEvidence"
        elif deletion == "OnCompatibilityWindowExpiry":
            life = "UntilCompatibilityExpiry"
        else:
            life = "UntilSuccessor"
        nodes.append({"id": lid, "family": "LEG", "kind": "LegacyObligation", "lifetime": life,
                      "owner": owner, "gates": gate, "witness": "", "failure": f"Legacy({gate})"})
    dec = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    for did, disp in _DEC_ROW.findall(dec):
        nodes.append({"id": did, "family": "DEC", "kind": "Decision", "lifetime": "Permanent",
                      "owner": "docs/30_DECISION_AND_REJECTION_LEDGER.md", "gates": "", "witness": "",
                      "failure": f"Decision({disp})"})
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    for pkg, cls, layer in _PKG_ROW.findall(arch):
        nodes.append({"id": f"ARCH-{pkg}", "family": "ARCH", "kind": "ArchitectureConstraint",
                      "lifetime": "Permanent", "owner": "spec/architecture.rs", "gates": "",
                      "witness": "", "failure": f"Architecture(L{layer} {cls})"})
    for pkg, profile, target in _QUAL_ROW.findall(arch):
        nodes.append({"id": f"QUAL-{pkg}-{profile}", "family": "QUAL", "kind": "QualificationRequirement",
                      "lifetime": "Permanent", "owner": "spec/architecture.rs", "gates": target,
                      "witness": "", "failure": f"Qualification({target})"})
    nodes.sort(key=lambda n: (FAMILY_RANK[n["family"]], n["id"]))
    return nodes


def guarantee_edges(root):
    edges = []
    for s in guarantee_seed(root):
        for kind in ("DerivesFrom", "Refines", "Discharges", "Supersedes"):
            for target in s[kind]:
                edges.append((s["id"], kind, target))
    edges.sort(key=lambda e: (e[0], e[1], e[2]))
    return edges


def _relations(s):
    parts = []
    for kind in ("DerivesFrom", "Refines", "Discharges", "Supersedes"):
        for target in s[kind]:
            parts.append(f"{kind} {target}")
    return "; ".join(parts) if parts else "-"


def render_seed_classification(root):
    seed = guarantee_seed(root)
    out = [
        "| GuaranteeId | Kind | Lifetime | Owner | Gates | Witness | Relations |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for s in seed:
        out.append(f"| {s['id']} | {s['kind']} | {s['lifetime']} | {s['owner']} | {s['gates']} | {s['witness']} | {_relations(s)} |")
    return "\n".join(out)


def _count(items):
    counts = {}
    for x in items:
        counts[x] = counts.get(x, 0) + 1
    return counts


def render_guarantee_graph(root):
    nodes = guarantee_nodes(root)
    edges = guarantee_edges(root)
    node_ids = {n["id"] for n in nodes}
    unresolved = sum(1 for _s, _k, t in edges if t not in node_ids)
    by_family = _count(n["family"] for n in nodes)
    by_kind = _count(n["kind"] for n in nodes)
    by_life = _count(n["lifetime"] for n in nodes)
    closed = by_life.get("ClosedEvidence", 0)
    historical = by_life.get("HistoricalCoverageOnly", 0)
    active = len(nodes) - closed - historical
    lines = [GUARANTEE_FRONT.rstrip("\n"), "",
             "# Guarantee Graph (generated)", "",
             "Derived structural index across the authored fact families (DEC-070). It is not",
             "normative: every law resolves to its owning fact. Displayed law text is copied for",
             "navigation only; the owning fact is authoritative. Do not edit.", "",
             "## Source-family inventory", "",
             "| Family | Nodes |", "| --- | --- |"]
    for fam in ("SEED", "LEG", "DEC", "ARCH", "QUAL"):
        lines.append(f"| {fam} | {by_family.get(fam, 0)} |")
    lines += ["", "## Totals", "", "```text",
              f"nodes: {len(nodes)}", f"edges: {len(edges)}", f"unresolved references: {unresolved}",
              "```", "", "## Counts by kind", "", "| GuaranteeKind | Nodes |", "| --- | --- |"]
    for kind in sorted(by_kind):
        lines.append(f"| {kind} | {by_kind[kind]} |")
    lines += ["", "## Counts by lifetime", "", "| GuaranteeLifetime | Nodes |", "| --- | --- |"]
    for life in sorted(by_life):
        lines.append(f"| {life} | {by_life[life]} |")
    lines += ["", "## Active versus closed", "", "```text",
              f"active (gating): {active}", f"closed evidence: {closed}",
              f"historical coverage only: {historical}", "```", "",
              "## Classified SEED", "", render_seed_classification(root), "",
              "## Nodes", "", "| GuaranteeId | Family | Kind | Lifetime | Owner | Gates |",
              "| --- | --- | --- | --- | --- | --- |"]
    for n in nodes:
        lines.append(f"| {n['id']} | {n['family']} | {n['kind']} | {n['lifetime']} | {n['owner']} | {n['gates']} |")
    lines += ["", "## Edges", "", "| Source | Relation | Target |", "| --- | --- | --- |"]
    for s, k, t in edges:
        lines.append(f"| {s} | {k} | {t} |")
    lines.append("")
    return "\n".join(lines)


NUMERIC_DOC = "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md"
BLOCK_RENDER = {
    "OPERATORS-CATALOG": render_catalog,
    "OPERATORS-GRAMMAR": render_grammar,
    "OPERATORS-PROJECTION": render_projection,
    "OPERATORS-NUMERIC": render_numeric,
}
BLOCK_FILE = {
    "OPERATORS-CATALOG": COMPANION,
    "OPERATORS-GRAMMAR": COMPANION,
    "OPERATORS-PROJECTION": COMPANION,
    "OPERATORS-NUMERIC": NUMERIC_DOC,
}


def block_pattern(name: str) -> re.Pattern[str]:
    return re.compile(
        r"(<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n)(.*?)(\n<!-- " + re.escape(name) + r":END -->)",
        re.S,
    )


def apply_blocks(text: str, ops: list[dict[str, str]], names: list[str]) -> tuple[str, list[str]]:
    """Return (rewritten_text, missing_block_names) for the named blocks."""
    missing: list[str] = []
    for name in names:
        pattern = block_pattern(name)
        body = BLOCK_RENDER[name](ops)
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
    by_file: dict[str, list[str]] = {}
    for name, rel in BLOCK_FILE.items():
        by_file.setdefault(rel, []).append(name)
    plans: list[tuple[Path, str, str]] = []
    for rel, names in sorted(by_file.items()):
        path = root / rel
        original = path.read_text(encoding="utf-8")
        rewritten, missing = apply_blocks(original, ops, names)
        for name in missing:
            findings.append(f"{rel}: missing generated block markers for {name}")
        plans.append((path, original, rewritten))
    # Guarantee classification: SEED marker block in docs/23 + the whole graph file
    seed_path = root / SEED_DOC
    seed_original = seed_path.read_text(encoding="utf-8")
    seed_pattern = block_pattern("SEED-CLASSIFICATION")
    if not seed_pattern.search(seed_original):
        findings.append(f"{SEED_DOC}: missing generated block markers for SEED-CLASSIFICATION")
        seed_rewritten = seed_original
    else:
        body = render_seed_classification(root)
        seed_rewritten = seed_pattern.sub(lambda m: m.group(1) + body + m.group(3), seed_original)
    plans.append((seed_path, seed_original, seed_rewritten))
    graph_path = root / GUARANTEE_DOC
    graph_original = graph_path.read_text(encoding="utf-8") if graph_path.is_file() else ""
    plans.append((graph_path, graph_original, render_guarantee_graph(root)))
    if findings:
        print(f"project: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    if mode == "--write":
        changed = sum(1 for path, original, rewritten in plans if rewritten != original)
        for path, original, rewritten in plans:
            if rewritten != original:
                path.write_bytes(rewritten.encode("utf-8"))
        verb = "WROTE" if changed else "OK"
        print(f"project: {verb} operator blocks, SEED classification, and the Guarantee Graph ({len(ops)} operators)")
        return 0
    # --check
    stale = [path.name for path, original, rewritten in plans if rewritten != original]
    if stale:
        print(f"project: FAIL (generated operator blocks stale in {stale}; run project.py --write)", file=sys.stderr)
        return 1
    print(f"project: PASS ({len(BLOCK_RENDER)} operator blocks current; {len(ops)} operators)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

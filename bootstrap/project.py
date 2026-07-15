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
_GATE_SPEC_ROW = re.compile(
    r'GateSpec \{ id: GateId::(\w+), token: "([^"]*)", title: "([^"]*)" \}'
)


def gate_inventory(root: Path) -> list[tuple[str, str, str]]:
    """The gate inventory in declaration order. Declaration order IS canonical."""
    src = (root / "spec" / "gates.rs").read_text(encoding="utf-8")
    return _GATE_SPEC_ROW.findall(src)


def gate_tokens(expr: str, root: Path) -> str:
    """Render a typed GateId list as canonical tokens joined with '/'.

    Splits the list positionally and maps each variant through the inventory,
    so an unknown variant raises rather than rendering itself.
    """
    order = [g[0] for g in gate_inventory(root)]
    token_of = {g[0]: g[1] for g in gate_inventory(root)}
    names = [c.strip().split("::")[-1] for c in expr.split(",") if c.strip()]
    for nm in names:
        if nm not in token_of:
            raise SystemExit(f"project: unknown GateId {nm} in {expr!r}")
    names.sort(key=order.index)
    return "/".join(token_of[nm] for nm in names)


_SEED_ROW = re.compile(
    r'InvariantSpec \{ id: "([^"]+)", statement: "((?:[^"\\]|\\.)*)", '
    r'kind: GuaranteeKind::(\w+), lifetime: GuaranteeLifetime::(\w+), '
    r'owner: "([^"]*)", gates: &\[([^\]]*)\], witness: "([^"]*)", '
    r'failure_disposition: "([^"]*)", derives_from: &\[([^\]]*)\], '
    r'refines: &\[([^\]]*)\], discharges: &\[([^\]]*)\], supersedes: &\[([^\]]*)\] \}'
)
_LEG_ROW = re.compile(
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", clean_owner: "([^"]+)", '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::\w+, '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: ObligationStatus::(\w+) \}'
)
_DEC_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+), subject: "[^"]+"'
)
_PKG_ROW = re.compile(
    r'PackageSpec \{\s*package: "([^"]+)",\s*path: "[^"]+",\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: "([^"]+)",\s*profile: "([^"]+)",\s*'
    r'target: "([^"]+)",\s*gates: &\[([^\]]*)\],\s*requirement: "[^"]+",\s*\}', re.S)


def _ids(raw):
    return re.findall(r'"([^"]+)"', raw)


def guarantee_seed(root):
    src = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    out = []
    for m in _SEED_ROW.findall(src):
        out.append({
            "id": m[0], "statement": m[1], "kind": m[2], "lifetime": m[3],
            "owner": m[4], "gates": gate_tokens(m[5], root), "witness": m[6], "failure": m[7],
            "DerivesFrom": _ids(m[8]), "Refines": _ids(m[9]),
            "Discharges": _ids(m[10]), "Supersedes": _ids(m[11]),
        })
    return out


# --- Typed guarantee admission (5.5D4b authority closure) --------------------
# Every GuaranteeView field comes from a direct authored value, a declared family
# policy in spec/guarantees.rs, or one named lawful derivation. This projector
# serializes admitted views. It does not complete them: the nine semantic
# constants that used to live here (Permanent, the docs/30 owner string, the
# family kinds, the empty gate/witness strings, and `gates: target`) are gone,
# because a projector that fills silence is an unauthorized author.
def _decomment(src):
    return re.sub(r"//[^\n]*", "", src)


_FAM_ROW = re.compile(
    r'GuaranteeFamilyPolicy \{\s*family: "(\w+)",\s*'
    r'kind: KindRule::(\w+)(?:\(GuaranteeKind::(\w+)\))?,\s*'
    r'owner: OwnerRule::(\w+)(?:\("([^"]*)"\))?,\s*'
    r'lifetime: LifetimeRule::(\w+)(?:\(GuaranteeLifetime::(\w+)\))?,\s*'
    r'gate_posture: GatePostureRule::(\w+)(?:\(GatePosture::Scheduled\(&\[([^\]]*)\]\)\))?,\s*'
    r'witness: WitnessPosture::(\w+),\s*\}',
    re.S,
)


def family_policies(root):
    """The typed family policy. Parsed, never authored here."""
    src = _decomment((root / "spec/guarantees.rs").read_text(encoding="utf-8"))
    out = {}
    for m in _FAM_ROW.finditer(src):
        fam, krule, kconst, orule, oconst, lrule, lconst, grule, gconst, wit = m.groups()
        out[fam] = {
            "kind": (krule, kconst), "owner": (orule, oconst),
            "lifetime": (lrule, lconst), "gate_posture": (grule, gconst),
            "witness": wit,
        }
    return out


def disposition_lifetimes(root):
    """Disposition -> GuaranteeLifetime, read from the typed owner's match arms."""
    src = _decomment((root / "spec/dispositions.rs").read_text(encoding="utf-8"))
    body = re.search(r"pub const fn guarantee_lifetime\(self\) -> GuaranteeLifetime \{(.*?)\n    \}",
                     src, re.S)
    out = {}
    if not body:
        return out
    for arm in re.finditer(r"((?:Disposition::\w+\s*\|?\s*)+)=>\s*\{?\s*GuaranteeLifetime::(\w+)",
                           body.group(1), re.S):
        for d in re.findall(r"Disposition::(\w+)", arm.group(1)):
            out[d] = arm.group(2)
    return out


def gate_optional_classes(root):
    """Classes whose typed `requires_gate()` is false. Read from the owner."""
    src = _decomment((root / "spec/dispositions.rs").read_text(encoding="utf-8"))
    body = re.search(r"pub const fn requires_gate\(self\) -> bool \{(.*?)\n    \}", src, re.S)
    if not body:
        return set()
    arms = re.split(r"=>\s*true\s*,", body.group(1))
    if len(arms) < 2:
        return set()
    return set(re.findall(r"DecisionClass::(\w+)", arms[1]))


class Unadmitted(Exception):
    """A required field has no authored source. Admission fails closed."""


def _resolve(policy, dim, row_value, derived=None):
    rule, const = policy[dim]
    if rule == "RowDeclared":
        if row_value in (None, ""):
            raise Unadmitted(f"{dim} is RowDeclared but the row declares none")
        return row_value
    if rule == "FamilyConstant":
        if not const:
            raise Unadmitted(f"{dim} is FamilyConstant but the policy names no value")
        return const
    if derived is None:
        raise Unadmitted(f"{dim} names derivation {rule} but none was supplied")
    return derived


def admit(root, family, ident, row, failure):
    """native row + family policy + lawful derivation -> complete GuaranteeView."""
    pol = family_policies(root).get(family)
    if pol is None:
        raise Unadmitted(f"{ident}: no declared family policy for {family}")
    kind = _resolve(pol, "kind", row.get("kind"))
    owner = _resolve(pol, "owner", row.get("owner"))
    lrule = pol["lifetime"][0]
    derived_life = None
    if lrule == "FromDecisionDisposition":
        derived_life = disposition_lifetimes(root).get(row.get("disposition"))
        if derived_life is None:
            raise Unadmitted(f"{ident}: disposition {row.get('disposition')!r} has no typed lifetime")
    elif lrule == "FromLegacyStatusAndDeletion":
        derived_life = row.get("derived_lifetime")
    lifetime = _resolve(pol, "lifetime", row.get("lifetime"), derived_life)
    grule, gconst = pol["gate_posture"]
    if grule == "FamilyConstant":
        posture = gate_tokens(gconst, root)
    elif grule == "FromDecisionClassAndRowGates":
        if row.get("gates"):
            posture = row["gates"]
        elif row.get("class") in gate_optional_classes(root):
            # The class authors gate-independence; absence is not the claim.
            posture = "GateIndependent"
        else:
            raise Unadmitted(
                f"{ident}: class {row.get('class')} requires a gate but the row declares none")
    else:
        if row.get("gates") in (None, ""):
            raise Unadmitted(f"{ident}: gate posture is RowDeclared but the row declares none")
        posture = row["gates"]
    return {
        "id": ident, "family": family, "kind": kind, "lifetime": lifetime, "owner": owner,
        "gate_posture": posture, "target": row.get("target", ""),
        "witness": pol["witness"], "failure": failure,
        "witness_text": row.get("witness", ""),
    }


def guarantee_nodes(root):
    nodes = []
    for s in guarantee_seed(root):
        nodes.append(admit(root, "SEED", s["id"], {
            "kind": s["kind"], "owner": s["owner"], "lifetime": s["lifetime"],
            "gates": s["gates"], "witness": s["witness"],
        }, f"Seed({s['failure']})"))
    leg = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    for lid, owner, gate_expr, deletion, status in _LEG_ROW.findall(leg):
        gate = gate_tokens(gate_expr, root)
        # A LEG row names a clean owner and gates but no typed successor
        # GuaranteeRef, so an active gate-closed obligation is UntilGate, not
        # UntilSuccessor. UntilSuccessor requires a resolved typed successor.
        if status == "Closed":
            life = "ClosedEvidence"
        elif deletion == "OnCompatibilityWindowExpiry":
            life = "UntilCompatibilityExpiry"
        else:
            life = "UntilGate"
        nodes.append(admit(root, "LEG", lid, {
            "owner": owner, "gates": gate, "derived_lifetime": life,
        }, f"Legacy({gate})"))
    dec = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    for did, dcls, dgates, disp in _DEC_ROW.findall(dec):
        # Decision gates are node METADATA. They never become graph edges, and no
        # gate or profile node is synthesized to hold them.
        node = admit(root, "DEC", did, {
            "gates": gate_tokens(dgates, root), "disposition": disp, "class": dcls,
        }, f"Decision({disp})")
        node["dclass"] = dcls
        nodes.append(node)
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    for pkg, cls, layer in _PKG_ROW.findall(arch):
        nodes.append(admit(root, "ARCH", f"ARCH-{pkg}", {}, f"Architecture(L{layer} {cls})"))
    for pkg, profile, target, qgates in _QUAL_ROW.findall(arch):
        # target is the ENVIRONMENT; qgates is the SCHEDULE. Never interchanged.
        nodes.append(admit(root, "QUAL", f"QUAL-{pkg}-{profile}", {
            "gates": gate_tokens(qgates, root), "target": target,
        }, f"Qualification({target})"))
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
              "## Nodes", "",
              "Gate posture schedules qualification. Qualification target names the environment",
              "a requirement is qualified against. They are different dimensions: a target never",
              "appears under gate posture. `-` means the dimension does not apply to that family,",
              "which is distinct from missing data -- a field with no authored source fails",
              "admission and never reaches this table.", "",
              "| GuaranteeId | Family | Kind | Lifetime | DecisionClass | Owner | GatePosture | "
              "QualificationTarget | WitnessPosture |",
              "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for n in nodes:
        lines.append(f"| {n['id']} | {n['family']} | {n['kind']} | {n['lifetime']} | "
                     f"{n.get('dclass', '-')} | {n['owner']} | {n['gate_posture']} | "
                     f"{n['target'] or '-'} | {n['witness']} |")
    lines += ["", "## Edges", "", "| Source | Relation | Target |", "| --- | --- | --- |"]
    for s, k, t in edges:
        lines.append(f"| {s} | {k} | {t} |")
    lines.append("")
    return "\n".join(lines)


NUMERIC_DOC = "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md"
GATES_DOC = "docs/25_IMPLEMENTATION_GATES.md"


# --- Stale-vocabulary projection (5.5D4b) ------------------------------------
# The alias set is a CONSEQUENCE of the decisions that retired vocabulary. It was
# duplicated by hand in two documents and guarded by an equality check -- two
# human-maintained copies that a checker babysat. One typed owner, two generated
# projections, no mirror.
STALE_DOCS = ("docs/29_STATUS_AND_SUPERSESSION.md", "docs/33_AGENT_FINISH_LINE_CHECKLIST.md")
_STALE_ROW = re.compile(
    r'DecisionSpec \{ id: "DEC-\d+", class: DecisionClass::\w+, gates: &\[[^\]]*\], '
    r'disposition: Disposition::\w+, subject: "[^"]*", successor: "[^"]*", '
    r'stale_aliases: &\[([^\]]*)\]', re.S)


def stale_aliases(root: Path) -> list[str]:
    """Every retired alias, in DEC declaration order. Declaration order IS
    canonical, so the projection stays grouped under the decision that retired
    each term rather than scattering them alphabetically."""
    src = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    out: list[str] = []
    for raw in _STALE_ROW.findall(src):
        for alias in re.findall(r'"([^"]+)"', raw):
            if alias not in out:
                out.append(alias)
    return out


def render_stale_vocabulary(root: Path) -> str:
    lines = ["```text"]
    lines += stale_aliases(root)
    lines.append("```")
    return "\n".join(lines)


def render_gate_inventory(root: Path) -> str:
    """The docs/25 gate inventory, projected from spec/gates.rs."""
    rows = ["| GateId | Token | Title |", "| --- | --- | --- |"]
    for gid, token, title in gate_inventory(root):
        rows.append(f"| {gid} | {token} | {title} |")
    return "\n".join(rows)


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
    gates_path = root / GATES_DOC
    gates_original = gates_path.read_text(encoding="utf-8")
    for rel in STALE_DOCS:
        sp = root / rel
        if not sp.is_file():
            findings.append(f"{rel}: missing stale-vocabulary projection doc")
            continue
        so = sp.read_text(encoding="utf-8")
        spat = block_pattern("STALE-VOCAB")
        if not spat.search(so):
            findings.append(f"{rel}: missing generated block markers for STALE-VOCAB")
        else:
            plans.append((sp, so, spat.sub(
                lambda m: m.group(1) + render_stale_vocabulary(root) + m.group(3), so)))
    gates_pattern = block_pattern("GATE-INVENTORY")
    if not gates_pattern.search(gates_original):
        findings.append(f"{GATES_DOC}: missing generated block markers for GATE-INVENTORY")
        gates_rewritten = gates_original
    else:
        body = render_gate_inventory(root)
        gates_rewritten = gates_pattern.sub(lambda m: m.group(1) + body + m.group(3), gates_original)
    plans.append((gates_path, gates_original, gates_rewritten))
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

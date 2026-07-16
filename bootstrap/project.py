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
    r'associativity: Associativity::(\w+), typing: &\[([^\]]*)\], input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", formatting: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)", numeric_support: NumericSupport::(\w+) \}'
)
FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "typing", "input_sorts", "result_sort",
    "exactness", "overflow", "exception", "formatting", "spoken",
    "mutation_classes", "numeric_support",
)


def typing_rules(op: dict[str, str]) -> list[str]:
    """The operator's closed legality rules, in authored match order."""
    return re.findall(r"OperatorTypingRule::(\w+)", op["typing"])


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
    elif grule == "FromDecisionDispositionClassAndGates":
        if row.get("gates"):
            # A retired decision keeps its authored GateIds as PROVENANCE. The
            # references are preserved, never deleted, and never rendered as an
            # active schedule.
            posture = (f"historical:{row['gates']}"
                       if lifetime == "HistoricalCoverageOnly" else row["gates"])
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


SYSTEM_DOC = "docs/02_SYSTEM_MODEL.md"
RECON_SPEC = "spec/reconciliation.rs"


def recon_coordinates(root: Path) -> list[tuple[str, list[str], str]]:
    """(role, carriers, law) rows from the typed DEC-075 composition owner.
    Serializes admitted facts only; invents nothing."""
    src = (root / RECON_SPEC).read_text(encoding="utf-8")
    start = src.index("pub const RECONCILIATION_COORDINATES")
    block = src[start: src.index("\n];", start) + 3]
    rows: list[tuple[str, list[str], str]] = []
    for m in re.finditer(
            r"ReconciliationCoordinate \{\s*role: ReconciliationRole::(\w+),\s*"
            r"carriers: &\[([^\]]*)\],\s*law: \"((?:[^\"\\]|\\.)*)\"", block, re.S):
        carriers = re.findall(r'"([^"]+)"', m.group(2))
        law = re.sub(r"\\\n\s*", "", m.group(3))
        if not carriers or not law.strip():
            raise Unadmitted(f"reconciliation role {m.group(1)} is incomplete")
        rows.append((m.group(1), carriers, law))
    if len(rows) != 5:
        raise Unadmitted(f"expected 5 reconciliation coordinates, parsed {len(rows)}")
    return rows


def render_recon_coordinates(root: Path) -> str:
    lines = ["| Role | Carriers | Law |", "| --- | --- | --- |"]
    for role, carriers, law in recon_coordinates(root):
        lines.append(f"| {role} | {', '.join(carriers)} | {law} |")
    return "\n".join(lines)


def recon_retry(root: Path) -> tuple[list[str], list[str]]:
    """(admissible, inadmissible) retry signals from the typed classification."""
    src = (root / RECON_SPEC).read_text(encoding="utf-8")
    start = src.index("pub const fn admissible")
    body = src[start: src.index("\n    }", start)]
    adm: list[str] = []
    inadm: list[str] = []
    for arm, verdict in re.findall(
            r"((?:RetrySignal::\w+\s*\|?\s*)+)=>\s*(true|false)", body):
        names = re.findall(r"RetrySignal::(\w+)", arm)
        (adm if verdict == "true" else inadm).extend(names)
    if not adm or not inadm:
        raise Unadmitted("retry classification does not partition the signals")
    return adm, inadm


def render_recon_retry(root: Path) -> str:
    adm, inadm = recon_retry(root)
    lines = ["```text", "admissible for retry, resume, compensation, or refusal:"]
    lines += [f"    {name}" for name in adm]
    lines += ["", "never sufficient on their own:"]
    lines += [f"    {name}" for name in inadm]
    lines.append("```")
    return "\n".join(lines)


RELEASE_DOC = "docs/36_PUBLIC_API_CI_AND_RELEASE.md"


def release_seal_fields(root: Path) -> list[str]:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    start = src.index("pub const RELEASE_SEAL_FIELDS")
    names = re.findall(r"ReleaseSealField::(\w+)",
                       src[start: src.index("\n];", start)])
    if not names:
        raise Unadmitted("RELEASE_SEAL_FIELDS declares no fields")
    return names


def render_release_seal(root: Path) -> str:
    lines = ["```text"] + release_seal_fields(root) + ["```"]
    return "\n".join(lines)


def render_gate_inventory(root: Path) -> str:
    """The docs/25 gate inventory, projected from spec/gates.rs."""
    rows = ["| GateId | Token | Title |", "| --- | --- | --- |"]
    for gid, token, title in gate_inventory(root):
        rows.append(f"| {gid} | {token} | {title} |")
    return "\n".join(rows)


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
    for op in canonical(ops):
        if op["id"] not in titles:
            continue
        rules = typing_rules(op)
        if not rules:
            raise Unadmitted(f"{op['id']} declares no typing rules")
        lines += [f"### {titles[op['id']]} (`{op['word']}`)", "", "```text"]
        for rule in rules:
            lines += _typing_rows(rule, op["word"])
        lines += ["```", ""]
    lines += ["### Comparison and truth", "", "```text",
              "comparisons        exact same-sort pair -> Truth with TypedMargin",
              "NOT                Truth -> Truth (K3 total)",
              "AND / OR           Truth x Truth -> Truth (K3 total)", "```"]
    return "\n".join(lines)


BLOCK_RENDER = {
    "OPERATORS-CATALOG": render_catalog,
    "OPERATORS-GRAMMAR": render_grammar,
    "OPERATORS-PROJECTION": render_projection,
    "OPERATORS-NUMERIC": render_numeric,
    "OPERATORS-TYPING": render_typing,
}
BLOCK_FILE = {
    "OPERATORS-CATALOG": COMPANION,
    "OPERATORS-GRAMMAR": COMPANION,
    "OPERATORS-PROJECTION": COMPANION,
    "OPERATORS-NUMERIC": NUMERIC_DOC,
    "OPERATORS-TYPING": COMPANION,
}


# --- PakVM semantic ISA projection (D4c1) -----------------------------------
# GENERATOR half. `bootstrap/audit.py` re-derives all of this independently and
# compares; the two share no parsing or admission logic. Neither may author node
# semantics: `spec/pakvm_isa.rs` owns them, this reads them, and a node whose
# spec does not admit is not projected at all.

ISA_SPEC = "spec/pakvm_isa.rs"
ISA_DOC = "docs/07_PAKVM_ISA.md"
P_ISA_ARM = re.compile(
    r"((?:\|?\s*PakVmNodeId::\w+\s*)+)=>\s*\{?\s*"
    r"(\"(?:[^\"\\]|\\.)*\"|[A-Za-z_][\w:]*)")
P_ISA_NODES = re.compile(r"pub const PAKVM_NODES: &\[PakVmNodeId\] = &\[(.*?)\n\];", re.S)
P_ISA_VARIANT = re.compile(r"PakVmNodeId::(\w+)")
P_ISA_FIELD = re.compile(r"(\w+):\s*([^,\n]+?),?\s*$")


def _const_fn_body(src: str, name: str) -> str:
    """The text of one `impl PakVmNodeId` method, start to the next method."""
    start = src.index(f"pub const fn {name}(")
    rest = src[start:]
    nxt = rest.find("\n    pub const fn ", 1)
    return rest if nxt < 0 else rest[:nxt]


def _isa_arms(src: str, name: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for pattern, value in P_ISA_ARM.findall(_const_fn_body(src, name)):
        for variant in P_ISA_VARIANT.findall(pattern):
            out[variant] = value[1:-1] if value.startswith('"') else value.split("::")[-1]
    return out


def _struct_rows(src: str, const: str, kind: str) -> list[dict[str, str]]:
    body = src[src.index(f"pub const {const}"):]
    body = body[: body.index("\n];")]
    rows = []
    for chunk in re.findall(kind + r"\s*\{(.*?)\n    \},", body + "\n    },", re.S):
        row: dict[str, str] = {}
        depth = 0
        field = ""
        for ch in chunk:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            if ch == "," and depth == 0:
                m = P_ISA_FIELD.search(field.strip())
                if m:
                    row[m.group(1)] = m.group(2).strip()
                field = ""
            else:
                field += ch
        m = P_ISA_FIELD.search(field.strip())
        if m:
            row[m.group(1)] = m.group(2).strip()
        if row:
            rows.append(row)
    return rows


def _isa_units(src: str) -> dict[str, list[str]]:
    """WorkFormulaFamily -> the work units it accounts in."""
    body = src[src.index("pub const fn units(self)"):]
    body = body[: body.index("\n}")]
    out: dict[str, list[str]] = {}
    for fam, units in re.findall(
            r"WorkFormulaFamily::(\w+)\s*=>\s*\{?\s*&\[([^\]]*)\]", body, re.S):
        out[fam] = re.findall(r"WorkUnit::(\w+)", units)
    return out


def _isa_planes(src: str) -> tuple[dict[str, list[str]], dict[str, str]]:
    body = src[src.index("pub const fn work_plane(self)"):]
    body = body[: body.index("pub const fn mandatory_work_unit")]
    planes = {a: re.findall(r"WorkUnit::(\w+)", u)
              for a, u in re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*&\[([^\]]*)\]", body, re.S)}
    body2 = src[src.index("pub const fn mandatory_work_unit(self)"):]
    body2 = body2[: body2.index("\n}")]
    mand = {}
    for alg, val in re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*(Some\(WorkUnit::\w+\)|None)", body2):
        m = re.search(r"WorkUnit::(\w+)", val)
        mand[alg] = m.group(1) if m else ""
    return planes, mand


class Unadmitted(Exception):
    pass


def pakvm_specs(root: Path) -> list[dict]:
    """Admitted PakVM node specs, derived independently of spec/pakvm_isa.rs's
    own `admit`. A node that does not admit raises: a projector serializes an
    admitted spec and may never complete one."""
    src = (root / ISA_SPEC).read_text(encoding="utf-8")
    m = P_ISA_NODES.search(src)
    nodes = P_ISA_VARIANT.findall(m.group(1)) if m else []
    algebra = _isa_arms(src, "algebra")
    klass = _isa_arms(src, "class")
    name = _isa_arms(src, "authored_name")
    operands = _isa_arms(src, "operand_sorts")
    results = _isa_arms(src, "result_sorts")
    units = _isa_units(src)
    planes, mandatory = _isa_planes(src)
    apol = {r["algebra"].split("::")[-1]: r
            for r in _struct_rows(src, "PAKVM_ALGEBRA_POLICIES", "PakVmAlgebraPolicy")}
    cpol = {r["class"].split("::")[-1]: r
            for r in _struct_rows(src, "PAKVM_NODE_CLASS_POLICIES", "PakVmNodeClassPolicy")}
    origins = {}
    body = _const_fn_body(src, "source_origin")
    for alg, val in re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*\"([^\"]*)\"", body):
        origins[alg] = val
    out = []
    for n in nodes:
        alg, cls = algebra.get(n), klass.get(n)
        if not alg or not cls:
            raise Unadmitted(f"{n}: no authored algebra or class")
        ap, cp = apol.get(alg), cpol.get(cls)
        if ap is None or cp is None:
            raise Unadmitted(f"{n}: {alg}/{cls} declares no policy")
        if cp["algebra"].split("::")[-1] != alg:
            raise Unadmitted(f"{n}: class policy names a different algebra")
        resolved = {}
        for field in ("effect", "capability", "evidence", "lowering"):
            rule = ap.get(field, "")
            declared = cp.get(field, "None") if field != "lowering" else "None"
            const = re.search(r"AlgebraConstant\((\w+::)?(\w+)\)", rule)
            got = re.search(r"Some\((\w+::)?(\w+)\)", declared)
            if const and got:
                raise Unadmitted(f"{n}: {field} has no single owner")
            if const:
                resolved[field] = const.group(2)
            elif got:
                resolved[field] = got.group(2)
            else:
                raise Unadmitted(f"{n}: {field} has no single owner")
        if resolved["lowering"] != "PublicSemanticIdentity":
            raise Unadmitted(f"{n}: an internal lowering identity is not a semantic node")
        fam = cp["work_formula"].split("::")[-1]
        fu = units.get(fam)
        if not fu:
            raise Unadmitted(f"{n}: work formula {fam} accounts in no unit")
        plane = planes.get(alg, [])
        for u in fu:
            if u not in plane:
                raise Unadmitted(f"{n}: work unit {u} is outside the {alg} work plane")
        need = mandatory.get(alg, "")
        if need and need not in fu:
            raise Unadmitted(f"{n}: work formula omits {need}")
        out.append({
            "node": n, "name": name.get(n, ""), "algebra": alg, "class": cls,
            "effect": resolved["effect"], "capability": resolved["capability"],
            "boundedness": cp["boundedness"].split("::")[-1], "work_formula": fam,
            "units": fu, "evidence": resolved["evidence"],
            "operands": operands.get(n, ""), "results": results.get(n, ""),
            "origin": origins.get(alg, ""),
        })
    return out


def render_pakvm_isa(root) -> str:
    specs = pakvm_specs(root)
    out = [
        "| Node | Algebra | Class | Effect | Capability | Boundedness | WorkFormula | WorkUnits | Evidence |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for s in specs:
        out.append(
            f"| {s['name']} | {s['algebra']} | {s['class']} | {s['effect']} | "
            f"{s['capability']} | {s['boundedness']} | {s['work_formula']} | "
            f"{'/'.join(s['units'])} | {s['evidence']} |")
    return "\n".join(out)


def render_pakvm_signatures(root) -> str:
    out = ["```text"]
    for s in pakvm_specs(root):
        out.append(s["name"])
        out.append(f"    operands  {s['operands']}")
        out.append(f"    results   {s['results']}")
        out.append("")
    return "\n".join(out).rstrip("\n") + "\n```"


FIREWALL_SPEC = "spec/syncbat_firewall.rs"
ARCH_SPEC = "spec/architecture.rs"
SYNCBAT_DOC = "docs/08_SYNCBAT_RUNTIME.md"
P_COMMENT = re.compile(r"//[^\n]*")


def _method_body(src: str, name: str, where: str) -> str:
    """One `fn name(self)` body, bounded by its own closing brace. Bounding by
    the brace rather than by the next method matters: the last method in an impl
    would otherwise swallow the rest of the file, and a table parsed from that
    text would read as if every constant below it were part of the match."""
    m = re.search(r"fn " + re.escape(name) + r"\(self\)[^{]*\{(.*?)\n    \}", src, re.S)
    if not m:
        raise Unadmitted(f"{where}: declares no {name}()")
    return P_COMMENT.sub("", m.group(1))


def _const_list(src: str, const: str, variant: str, where: str) -> list[str]:
    try:
        body = src[src.index(f"pub const {const}"):]
    except ValueError:
        raise Unadmitted(f"{where}: declares no {const}") from None
    return re.findall(variant + r"::(\w+)", body[: body.index("\n];")])


def syncbat_firewall_facts(root: Path) -> dict:
    """Admitted SyncBat firewall facts, read from their typed owners.

    This serializes. It states no plane ownership, no legal crossing, no
    semantic default, and completes nothing: a fact the spec does not author
    raises `Unadmitted` rather than being filled in here. That restraint is the
    whole point — a generator that can supply a missing policy is a second
    authority no matter what its docstring claims."""
    arch = (root / ARCH_SPEC).read_text(encoding="utf-8")
    src = (root / FIREWALL_SPEC).read_text(encoding="utf-8")
    # Plane identity and the ownership sentences: spec/architecture.rs owns both.
    order = _const_list(arch, "SYNCBAT_PLANES", "SyncBatPlane", ARCH_SPEC)
    if not order:
        raise Unadmitted(f"{ARCH_SPEC}: SYNCBAT_PLANES lists no plane")
    sentences = dict(re.findall(
        r'SyncBatPlane::(\w+)\s*=>\s*"([^"]*)"',
        _method_body(arch, "authored_ownership", ARCH_SPEC)))
    for plane in order:
        if not sentences.get(plane):
            raise Unadmitted(f"{ARCH_SPEC}: plane {plane} authors no ownership sentence")
    # Authority ownership: spec/syncbat_firewall.rs owns it.
    listed = _const_list(src, "SYNCBAT_AUTHORITIES", "SyncBatAuthority", FIREWALL_SPEC)
    if not listed:
        raise Unadmitted(f"{FIREWALL_SPEC}: SYNCBAT_AUTHORITIES lists no authority")
    owner_body = _method_body(src, "owner", FIREWALL_SPEC)
    owners: dict[str, str] = {}
    for pattern, plane in re.findall(
            r"((?:\|?\s*SyncBatAuthority::\w+\s*)+)=>\s*SyncBatPlane::(\w+)", owner_body):
        for variant in re.findall(r"SyncBatAuthority::(\w+)", pattern):
            if variant in owners:
                raise Unadmitted(f"{FIREWALL_SPEC}: owner() answers twice for {variant}")
            owners[variant] = plane
    semantic = set(re.findall(
        r"SyncBatAuthority::(\w+)",
        _method_body(src, "requires_semantic_origin", FIREWALL_SPEC)))
    authorities = []
    for name in listed:
        plane = owners.get(name)
        if not plane:
            raise Unadmitted(f"{FIREWALL_SPEC}: authority {name} names no owning plane")
        if plane not in order:
            raise Unadmitted(f"{FIREWALL_SPEC}: authority {name} names {plane}, which is no plane")
        authorities.append({"authority": name, "owner": plane, "origin": name in semantic})
    # The legal-crossing whitelist: spec/syncbat_firewall.rs owns it.
    crossings = []
    for row in _struct_rows(src, "SYNCBAT_LEGAL_CROSSINGS", "SyncBatCrossing"):
        frm = row.get("from", "").split("::")[-1]
        to = row.get("to", "").split("::")[-1]
        carries = row.get("carries", "").split("::")[-1]
        posture = row.get("posture", "").split("::")[-1]
        law = row.get("law", "").strip().strip('"')
        for plane in (frm, to):
            if plane not in order:
                raise Unadmitted(f"{FIREWALL_SPEC}: a crossing names {plane!r}, which is no plane")
        if carries not in listed:
            raise Unadmitted(f"{FIREWALL_SPEC}: a crossing carries {carries!r}, "
                             f"which SYNCBAT_AUTHORITIES does not author")
        # No default. An absent posture is not silently Optional; it is a refusal.
        if posture not in ("Required", "Optional"):
            raise Unadmitted(f"{FIREWALL_SPEC}: the {frm} -> {to} crossing states no "
                             f"requiredness posture (got {posture!r})")
        if not law:
            raise Unadmitted(f"{FIREWALL_SPEC}: the {frm} -> {to} crossing states no law")
        crossings.append({"from": frm, "to": to, "carries": carries,
                          "posture": posture, "law": law})
    if not crossings:
        raise Unadmitted(f"{FIREWALL_SPEC}: SYNCBAT_LEGAL_CROSSINGS lists no crossing")
    return {"order": order, "sentences": sentences,
            "authorities": authorities, "crossings": crossings}


def render_syncbat_plane_ownership(root) -> str:
    facts = syncbat_firewall_facts(root)
    return "\n".join(["```text"] + [facts["sentences"][p] for p in facts["order"]] + ["```"])


def render_syncbat_authorities(root) -> str:
    out = ["| Authority | Owning plane | Names a PakVM origin |", "| --- | --- | --- |"]
    for a in syncbat_firewall_facts(root)["authorities"]:
        out.append(f"| {a['authority']} | {a['owner']} | {'yes' if a['origin'] else 'no'} |")
    return "\n".join(out)


def render_syncbat_crossings(root) -> str:
    out = ["| From | To | Carries | Posture | Law |", "| --- | --- | --- | --- | --- |"]
    for c in syncbat_firewall_facts(root)["crossings"]:
        out.append(f"| {c['from']} | {c['to']} | {c['carries']} | {c['posture']} | {c['law']} |")
    return "\n".join(out)


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
    isa_path = root / ISA_DOC
    isa_original = isa_path.read_text(encoding="utf-8")
    isa_rewritten = isa_original
    for name, render in (("PAKVM-SEMANTIC-ISA", render_pakvm_isa),
                         ("PAKVM-SIGNATURES", render_pakvm_signatures)):
        pat = block_pattern(name)
        if not pat.search(isa_rewritten):
            findings.append(f"{ISA_DOC}: missing generated block markers for {name}")
            continue
        body = render(root)
        isa_rewritten = pat.sub(lambda m: m.group(1) + body + m.group(3), isa_rewritten)
    plans.append((isa_path, isa_original, isa_rewritten))
    fw_path = root / SYNCBAT_DOC
    fw_original = fw_path.read_text(encoding="utf-8")
    fw_rewritten = fw_original
    for name, render in (("SYNCBAT-PLANE-OWNERSHIP", render_syncbat_plane_ownership),
                         ("SYNCBAT-AUTHORITIES", render_syncbat_authorities),
                         ("SYNCBAT-CROSSINGS", render_syncbat_crossings)):
        pat = block_pattern(name)
        if not pat.search(fw_rewritten):
            findings.append(f"{SYNCBAT_DOC}: missing generated block markers for {name}")
            continue
        try:
            body = render(root)
        except Unadmitted as exc:
            # Fail closed and say why. The alternative — a traceback — is a
            # projector refusing to answer, which reads to a fixture exactly like
            # a projector that has nothing to report.
            findings.append(f"{SYNCBAT_DOC}: {name} does not admit: {exc}")
            continue
        fw_rewritten = pat.sub(lambda m: m.group(1) + body + m.group(3), fw_rewritten)
    plans.append((fw_path, fw_original, fw_rewritten))
    recon_path = root / SYSTEM_DOC
    recon_original = recon_path.read_text(encoding="utf-8")
    recon_rewritten = recon_original
    for name, render in (("RECONCILIATION-COORDINATES", render_recon_coordinates),
                         ("RECONCILIATION-RETRY", render_recon_retry)):
        pat = block_pattern(name)
        if not pat.search(recon_rewritten):
            findings.append(f"{SYSTEM_DOC}: missing generated block markers for {name}")
            continue
        try:
            body = render(root)
        except Unadmitted as exc:
            findings.append(f"{SYSTEM_DOC}: {name} does not admit: {exc}")
            continue
        recon_rewritten = pat.sub(lambda m: m.group(1) + body + m.group(3), recon_rewritten)
    plans.append((recon_path, recon_original, recon_rewritten))
    seal_path = root / RELEASE_DOC
    seal_original = seal_path.read_text(encoding="utf-8")
    seal_pattern = block_pattern("RELEASE-SEAL")
    if not seal_pattern.search(seal_original):
        findings.append(f"{RELEASE_DOC}: missing generated block markers for RELEASE-SEAL")
        seal_rewritten = seal_original
    else:
        try:
            body = render_release_seal(root)
            seal_rewritten = seal_pattern.sub(
                lambda m: m.group(1) + body + m.group(3), seal_original)
        except Unadmitted as exc:
            findings.append(f"{RELEASE_DOC}: RELEASE-SEAL does not admit: {exc}")
            seal_rewritten = seal_original
    plans.append((seal_path, seal_original, seal_rewritten))
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

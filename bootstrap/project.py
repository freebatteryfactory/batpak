#!/usr/bin/env python3
"""Deterministic projection generator for the clean-room specification.

Every projection plan is built by iterating the typed generated-view registry
(`spec/generated_views.rs` GeneratedView::ALL, 5.5E4a): embedded marker blocks
across the authored corpus, the standalone generated files — including the
Guarantee Graph (docs/GUARANTEE_GRAPH.generated.md) and the tracked
rust-toolchain.toml, both of which THIS tool generates — and the mechanical
corpus-frontmatter epoch convergence. The registry owns which views exist;
each renderer here owns only how an admitted authority source is serialized,
and no manual literal plan may bypass the registry.

`project.py --write`  rewrites every registered generated view in place.
`project.py --check`  regenerates and fails if any view has drifted.

This is the GENERATOR half of the generator/auditor pair. `bootstrap/audit.py`
is the read-only AUDITOR: it recomputes the same projections independently and
compares. The two share no parsing, normalization, projection-building, or
comparison logic — only the inert marker names, whose values are independently
asserted on both sides. project.py never generates decision text itself.

Standard library only. All writes are UTF-8 with literal LF endings.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

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


# --- Guarantee graph (DEC-070) -----------------------------------------------
GUARANTEE_DOC = "docs/GUARANTEE_GRAPH.generated.md"
SEED_DOC = "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md"
FAMILY_RANK = {"SEED": 0, "LEG": 1, "DEC": 2, "ARCH": 3, "QUAL": 4}
GUARANTEE_FRONT = (
    "---\n"
    "status: GENERATED\n"
    "authority_scope: derived structural index only\n"
    "generated_by: bootstrap/project.py\n"
    "generated_from: spec/invariants.rs; spec/legacy_obligations.rs; "
    "spec/dispositions.rs; spec/architecture.rs; spec/guarantees.rs; spec/gates.rs\n"
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
    r'owner: "([^"]*)", gates: &\[([^\]]*)\], witnesses: &\[([^\]]*)\], '
    r'witness_note: "([^"]*)", '
    r'failure_disposition: "([^"]*)", derives_from: &\[([^\]]*)\], '
    r'refines: &\[([^\]]*)\], discharges: &\[([^\]]*)\], supersedes: &\[([^\]]*)\] \}'
)
# Typed witness citations (5.5E2): the projector renders the canonical forms
# and never invents one. A citation it does not recognize renders as nothing,
# which the block-parity comparison then catches.
_WITNESS_REF = re.compile(
    r'WitnessRef::(?:leg|dec|contract)\("([^"]+)"\)'
    r'|WitnessRef::BootstrapTool\(BootstrapToolId::(\w+)\)')
_TOOL_DISPLAY = {"ProjectPy": "project.py", "AuditPy": "audit.py",
                 "FreezePy": "freeze.py", "SelftestPy": "selftest.py",
                 "Seedcheck": "seedcheck", "Materialize": "materialize"}


def _witness_column(refs_raw, note):
    parts = [_TOOL_DISPLAY.get(tool, "") if tool else ident
             for ident, tool in _WITNESS_REF.findall(refs_raw)]
    text = "; ".join(p for p in parts if p)
    return f"{text} -- {note}" if note else text
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
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)


def _package_cargo_names(root):
    """PackageId variant -> cargo name, parsed from the projection the typed
    owner authors. The projector renders through this map and never carries a
    hand-maintained copy of the package list."""
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    body = re.search(
        r"pub const fn cargo_name\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        src, re.S)
    return dict(re.findall(r'PackageId::(\w+) => "([^"]+)",', body.group(1))) if body else {}
_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "[^"]+",\s*\}', re.S)
# The documentary spelling of each environment variant, mirrored by the
# auditor's own map and compared byte-for-byte through the generated blocks.
_ENVIRONMENT_SPELLING = {"NoStdAlloc": "no_std + alloc", "NativeStd": "std",
                         "WasmHost": "wasm32 host"}


def _ids(raw):
    return re.findall(r'"([^"]+)"', raw)


def guarantee_seed(root):
    src = (root / "spec/invariants.rs").read_text(encoding="utf-8")
    out = []
    for m in _SEED_ROW.findall(src):
        out.append({
            "id": m[0], "statement": m[1], "kind": m[2], "lifetime": m[3],
            "owner": m[4], "gates": gate_tokens(m[5], root),
            "witness": _witness_column(m[6], m[7]), "failure": m[8],
            "DerivesFrom": _ids(m[9]), "Refines": _ids(m[10]),
            "Discharges": _ids(m[11]), "Supersedes": _ids(m[12]),
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
    # The rendered node id is a PROJECTION of the typed identity: the graph
    # keeps its byte-identical "ARCH-batpak" form, but the id is derived
    # through the cargo_name arms the typed owner authors.
    cargo = _package_cargo_names(root)
    for variant, cls, layer in _PKG_ROW.findall(arch):
        nodes.append(admit(root, "ARCH", f"ARCH-{cargo.get(variant, '')}", {},
                           f"Architecture(L{layer} {cls})"))
    for variant, profile, environment, qgates in _QUAL_ROW.findall(arch):
        # environment is the SEMANTIC dimension; qgates is the SCHEDULE.
        # Never interchanged. The node projects the canonical spelling.
        spelling = _ENVIRONMENT_SPELLING.get(environment, "")
        nodes.append(admit(root, "QUAL", f"QUAL-{cargo.get(variant, '')}-{profile}", {
            "gates": gate_tokens(qgates, root), "target": spelling,
        }, f"Qualification({spelling})"))
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
CONTRACT_DOC = "docs/06_MACBAT.md"


def parse_contract_kinds(root):
    """Ordered (spelling, admission-basis id) pairs, in ContractKind::ALL
    order, from the typed owner."""
    src = (root / "spec/contracts.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[ContractKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"ContractKind::(\w+)", all_body.group(1)) if all_body else []
    spelling = dict(re.findall(r'ContractKind::(\w+) => "(\w+)",', src))
    basis = dict(re.findall(
        r'ContractKind::(\w+) => GuaranteeRef::(?:leg|dec)\("([^"]+)"\)', src))
    return [(spelling.get(v, ""), basis.get(v, "")) for v in inventory]


def render_contract_kinds(root):
    """The docs/06 admitted-kinds table, an ORDINARY generated block (5.5E4a):
    inventory membership and canonical order from the typed owner, nothing
    else. The retired hybrid fence let this generator edit selected columns
    inside an authored fence; the explanatory prose now lives outside."""
    lines = ["| ContractKind | Admission basis |", "| --- | --- |"]
    for kind, basis in parse_contract_kinds(root):
        lines.append(f"| {kind} | {basis} |")
    return "\n".join(lines)


# --- Toolchain projection (5.5E3a) -------------------------------------------
# spec/toolchain.rs owns the values; this projector serializes the tracked
# root rust-toolchain.toml and never completes a missing field.
_TOOLCHAIN_PROFILE_SPELLING = {"Minimal": "minimal"}
_TOOLCHAIN_COMPONENT_SPELLING = {"Clippy": "clippy", "Rustfmt": "rustfmt"}


def parse_toolchain(root):
    src = (root / "spec/toolchain.rs").read_text(encoding="utf-8")
    release = re.search(
        r"exact_rust_release: RustRelease \{ major: (\d+), minor: (\d+), patch: (\d+) \}", src)
    floor = re.search(
        r"rust_version_floor: RustVersionFloor \{ major: (\d+), minor: (\d+) \}", src)
    edition = re.search(r"edition: RustEdition::Rust(\d+)", src)
    resolver = re.search(r"cargo_resolver: CargoResolver::V(\d+)", src)
    profile = re.search(r"rustup_profile: RustupProfile::(\w+)", src)
    comps = re.search(
        r"pub const ALL: &'static \[RustupComponent\] =\s*&\[([^\]]*)\]", src)
    return {
        "exact": ".".join(release.groups()) if release else "",
        "floor": ".".join(floor.groups()) if floor else "",
        "edition": edition.group(1) if edition else "",
        "resolver": resolver.group(1) if resolver else "",
        "profile": _TOOLCHAIN_PROFILE_SPELLING.get(profile.group(1), "") if profile else "",
        "components": [_TOOLCHAIN_COMPONENT_SPELLING.get(v, "")
                       for v in re.findall(r"RustupComponent::(\w+)", comps.group(1))]
                      if comps else [],
    }


def render_root_toolchain(tc):
    comps = ", ".join(f'"{c}"' for c in tc["components"])
    return (f'[toolchain]\nchannel = "{tc["exact"]}"\n'
            f'profile = "{tc["profile"]}"\ncomponents = [{comps}]\n')


# --- Identity catalog projection (5.5E3d) ------------------------------------
# spec/identities.rs owns four separate catalog axes plus the non-cataloged
# residue table; docs/16 carries five generated blocks projecting ordered
# entries AND owners. This projector renders; audit.py independently
# reconstructs; they share no classification map.
IDENTITY_DOC = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
IDENTITY_AXES = (
    ("IDENTITY-CATALOG", "IdentityKind"),
    ("GENERATION-CATALOG", "GenerationKind"),
    ("BINDING-CATALOG", "BindingKind"),
    ("VERSION-CATALOG", "VersionIdentityKind"),
)
_DISPOSITION_SPELLING = {
    "OwnedElsewhere": "owned elsewhere",
    "NotYetAdmittedBy": "not yet admitted by",
}


def parse_identity_catalog(root, kind):
    """Ordered (spelling, owner) entries in Kind::ALL order from the typed
    owner. An ALL variant with no entry() arm refuses rather than rendering
    a hole."""
    src = (root / "spec/identities.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) if all_body else []
    # \b matters: "VersionIdentityKind::Schema" must never satisfy an
    # unanchored "IdentityKind::..." pattern and overwrite identity arms.
    arms = {v: (s, o) for v, s, o in re.findall(
        r"\b" + kind + r"::(\w+) => \{?\s*entry!\("
        r"\s*\"([^\"]+)\",\s*\"([^\"]+)\"\s*\)", src)}
    missing = [v for v in inventory if v not in arms]
    if not inventory or missing:
        raise Unadmitted(
            f"spec/identities.rs: {kind} inventory unreadable or entries missing "
            f"for {missing}")
    return [arms[v] for v in inventory]


def parse_identity_residue(root):
    """Ordered (term, disposition variant, reference) rows from
    NON_CATALOGED_IDENTITY_TERMS."""
    src = (root / "spec/identities.rs").read_text(encoding="utf-8")
    body = re.search(
        r"pub const NON_CATALOGED_IDENTITY_TERMS: &\[NonCatalogedIdentityTerm\] = &\[(.*?)\n\];",
        src, re.S)
    if not body:
        raise Unadmitted("spec/identities.rs: NON_CATALOGED_IDENTITY_TERMS unreadable")
    rows = re.findall(
        r'term: "(\w+)",\s*disposition: IdentityTermDisposition::(\w+)\('
        r'(?:ContractId|DecisionId)\("([^"]+)"\)\)', body.group(1))
    bad = [v for _, v, _ in rows if v not in _DISPOSITION_SPELLING]
    if not rows or bad:
        raise Unadmitted(
            f"spec/identities.rs: residue rows unreadable or dispositions unknown: {bad}")
    return rows


def render_identity_axis(root, kind):
    entries = parse_identity_catalog(root, kind)
    rows = "\n".join(f"{s:<30} {o}" for s, o in entries)
    return "```text\n" + rows + "\n```"


def render_identity_residue(root):
    rows = "\n".join(
        f"{term:<30} {_DISPOSITION_SPELLING[v]:<22} {ref}"
        for term, v, ref in parse_identity_residue(root))
    return "```text\n" + rows + "\n```"


# --- Compiler-assumption kinds projection (5.5E3h) ---------------------------
# spec/compiler_assumptions.rs owns the admitted ledgerable kinds; docs/19,
# the semantic owner, carries the generated fact row per kind.
SECURITY_DOC = "docs/19_SECURITY_MODEL.md"


def parse_compiler_assumptions(root):
    src = (root / "spec/compiler_assumptions.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[CompilerAssumptionKind\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bCompilerAssumptionKind::(\w+)", all_body.group(1)) \
        if all_body else []
    fields = {}
    for name, pat in (
            ("spelling", r'"([^"]*)"'),
            ("basis", r'GuaranteeRef::dec\("([^"]+)"\)'),
            ("marker", r"(true|false)"),
            ("cgate", r"GateId::(\w+)"),
            ("rgate", r"GateId::(\w+)")):
        fields[name] = {}
    for fn_name, key in (("spelling", "spelling"), ("admission_basis", "basis"),
                         ("requires_safety_contract_marker", "marker"),
                         ("classification_gate", "cgate"),
                         ("release_qualification_gate", "rgate")):
        body = re.search(
            r"pub const fn " + fn_name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        if not body:
            continue
        for arm, value in re.findall(
                r"(CompilerAssumptionKind::\w+(?:\s*\|\s*CompilerAssumptionKind::\w+)*)"
                r"\s*=>\s*([^,]+),", body.group(1), re.S):
            for v in re.findall(r"\bCompilerAssumptionKind::(\w+)", arm):
                fields[key][v] = value.strip()
    rows = []
    for v in inventory:
        if any(v not in fields[k] for k in fields):
            raise Unadmitted(
                f"spec/compiler_assumptions.rs: CompilerAssumptionKind::{v} facts unreadable")
        rows.append((
            fields["spelling"][v].strip('"'),
            re.search(r'"([^"]+)"', fields["basis"][v]).group(1),
            fields["marker"][v],
            re.search(r"GateId::(\w+)", fields["cgate"][v]).group(1),
            re.search(r"GateId::(\w+)", fields["rgate"][v]).group(1),
        ))
    if not rows:
        raise Unadmitted("spec/compiler_assumptions.rs: inventory unreadable")
    return rows


def render_compiler_assumptions(root):
    header = (f"{'kind':<22} {'basis':<9} {'marker':<7} "
              f"{'classify':<9} release")
    rows = "\n".join(
        f"{s:<22} {basis:<9} {marker:<7} {cg:<9} {rg}"
        for s, basis, marker, cg, rg in parse_compiler_assumptions(root))
    return "```text\n" + header + "\n" + rows + "\n```"


# --- Promotion requirements projection (5.5E3i) ------------------------------
# spec/promotion.rs owns the conjunctive denominator; docs/12 carries the
# generated table. This projector parses the typed owner — never a Python
# list of the four requirements.
def parse_promotion(root):
    src = (root / "spec/promotion.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[PromotionRequirement\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bPromotionRequirement::(\w+)", all_body.group(1)) \
        if all_body else []
    arms = {}
    for fn_name in ("spelling", "semantic_owner", "admission_basis"):
        body = re.search(
            r"pub const fn " + fn_name + r"\(self\)[^{]*\{\s*match self \{(.*?)\n        \}",
            src, re.S)
        arms[fn_name] = dict(re.findall(
            r"PromotionRequirement::(\w+)\s*=>\s*([^,]+),", body.group(1), re.S)) \
            if body else {}
    rows = []
    for v in inventory:
        s = arms["spelling"].get(v, "").strip().strip('"')
        owner = re.search(r'ContractId\("([^"]+)"\)', arms["semantic_owner"].get(v, ""))
        basis = re.search(r'GuaranteeRef::dec\("([^"]+)"\)',
                          arms["admission_basis"].get(v, ""))
        if not s or not owner or not basis:
            raise Unadmitted(
                f"spec/promotion.rs: PromotionRequirement::{v} facts unreadable")
        rows.append((s, owner.group(1), basis.group(1)))
    surface = re.search(r"ProofPolicySurface::(\w+);", src)
    change = re.search(
        r'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec\("([^"]+)"\);', src)
    egate = re.search(r"PROMOTION_ENFORCEMENT_GATE: GateId = GateId::(\w+);", src)
    rgate = re.search(r"PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::(\w+);", src)
    if not rows or not surface or not change or not egate or not rgate:
        raise Unadmitted("spec/promotion.rs: family facts unreadable")
    return rows, surface.group(1), change.group(1), egate.group(1), rgate.group(1)


def render_promotion(root):
    rows, surface, change, egate, rgate = parse_promotion(root)
    header = f"{'requirement':<26} {'owner':<14} admission basis"
    body = "\n".join(f"{s:<26} {o:<14} {b}" for s, o, b in rows)
    family = (f"\n{'policy surface':<26} {surface}\n"
              f"{'policy-change basis':<26} {change}\n"
              f"{'enforcement gate':<26} {egate}\n"
              f"{'release-visibility gate':<26} {rgate}")
    return "```text\n" + header + "\n" + body + "\n" + family + "\n```"


# --- Corpus epoch projection (5.5E3g) ----------------------------------------
# spec/corpus.rs owns which corpus epochs exist and which one is CURRENT;
# this projector mechanically converges every eligible document's epoch
# frontmatter and renders the docs/00 fact block. It touches ONLY the epoch
# field — authored status, contract id, scope, supersession, and the
# last_reconciled date stay authored.
def parse_corpus_epoch(root):
    src = (root / "spec/corpus.rs").read_text(encoding="utf-8")
    cur = re.search(
        r"pub const CURRENT_RECONCILIATION_EPOCH:\s*ReconciliationEpoch = "
        r"ReconciliationEpoch::(\w+);", src)
    spellings = dict(re.findall(r'ReconciliationEpoch::(\w+) => "([^"]+)",', src))
    owners = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => ContractId\("([^"]+)"\)', src))
    bases = dict(re.findall(
        r'ReconciliationEpoch::(\w+) => \{?\s*GuaranteeRef::Seed\(SeedId\("([^"]+)"\)\)', src))
    if not cur or cur.group(1) not in spellings:
        raise Unadmitted("spec/corpus.rs: CURRENT_RECONCILIATION_EPOCH unreadable")
    v = cur.group(1)
    return {"variant": v, "spelling": spellings[v],
            "owner": owners.get(v, ""), "basis": bases.get(v, "")}


def render_corpus_epoch(root):
    e = parse_corpus_epoch(root)
    return ("```text\n"
            f"{'current epoch':<18} {e['spelling']}\n"
            f"{'semantic owner':<18} {e['owner']}\n"
            f"{'admission basis':<18} {e['basis']}\n"
            "```")


def converge_epoch_frontmatter(text, spelling):
    """Insert or update the epoch field in one document's frontmatter,
    touching nothing else. GENERATED documents bind their SOURCE epoch."""
    m = re.match(r"---\n(.*?\n)---\n", text, re.S)
    if not m:
        return text
    front = m.group(1)
    key = ("source_reconciliation_epoch"
           if re.search(r"^status: GENERATED$", front, re.M)
           else "reconciliation_epoch")
    line = f"{key}: {spelling}\n"
    if re.search(r"^" + key + r": .*$", front, re.M):
        new_front = re.sub(r"^" + key + r": .*$", line.rstrip("\n"), front, flags=re.M)
    elif re.search(r"^last_reconciled: .*$", front, re.M):
        new_front = re.sub(r"^(last_reconciled: .*)$", r"\1\n" + line.rstrip("\n"),
                           front, flags=re.M)
    else:
        new_front = front + line
    return "---\n" + new_front + "---\n" + text[m.end():]


# --- Mutation vocabulary projection (5.5E3f) ---------------------------------
# spec/mutation.rs owns the lanes and result classifications as TOTAL const
# functions; docs/12 carries two generated fact tables.
MUTATION_DOC = "docs/12_TESTPAK.md"


def _mutation_fn_arms(src, enum, name):
    # Scope to the enum's own impl block: both enums declare a spelling()
    # and semantic_owner(), and the first match in file order would answer
    # for the wrong type.
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


def parse_mutation_lanes(root):
    src = (root / "spec/mutation.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[MutationLane\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bMutationLane::(\w+)", all_body.group(1)) if all_body else []
    fields = {name: _mutation_fn_arms(src, "MutationLane", name) for name in (
        "spelling", "requires_per_candidate_rust_compile",
        "requires_real_rustc_semantics", "requires_activation_evidence",
        "permits_production_profile_slots", "requires_independent_evidence_route",
        "gates")}
    rows = []
    for v in inventory:
        if any(v not in fields[f] for f in fields):
            raise Unadmitted(f"spec/mutation.rs: MutationLane::{v} facts unreadable")
        rows.append((
            fields["spelling"][v].strip('"'),
            fields["requires_per_candidate_rust_compile"][v],
            fields["requires_real_rustc_semantics"][v],
            fields["requires_activation_evidence"][v],
            fields["permits_production_profile_slots"][v],
            fields["requires_independent_evidence_route"][v],
            " ".join(re.findall(r"GateId::(\w+)", fields["gates"][v])),
        ))
    if not rows:
        raise Unadmitted("spec/mutation.rs: MutationLane inventory unreadable")
    return rows


def parse_mutation_results(root):
    src = (root / "spec/mutation.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[MutationResult\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\bMutationResult::(\w+)", all_body.group(1)) if all_body else []
    fields = {name: _mutation_fn_arms(src, "MutationResult", name) for name in (
        "spelling", "counts_as_kill", "counts_as_survival", "appears_in_denominator")}
    rows = []
    for v in inventory:
        if any(v not in fields[f] for f in fields):
            raise Unadmitted(f"spec/mutation.rs: MutationResult::{v} facts unreadable")
        rows.append((fields["spelling"][v].strip('"'), fields["counts_as_kill"][v],
                     fields["counts_as_survival"][v], fields["appears_in_denominator"][v]))
    if not rows:
        raise Unadmitted("spec/mutation.rs: MutationResult inventory unreadable")
    return rows


def render_mutation_lanes(root):
    rows = "\n".join(
        f"{s:<20} {pc:<7} {rr:<7} {act:<7} {slots:<7} {ind:<7} {gates}"
        for s, pc, rr, act, slots, ind, gates in parse_mutation_lanes(root))
    header = (f"{'lane':<20} {'compile':<7} {'rustc':<7} {'activ':<7} "
              f"{'slots':<7} {'indep':<7} gates")
    return "```text\n" + header + "\n" + rows + "\n```"


def render_mutation_results(root):
    rows = "\n".join(
        f"{s:<22} {kill:<7} {surv:<7} {denom}"
        for s, kill, surv, denom in parse_mutation_results(root))
    header = f"{'result':<22} {'kill':<7} {'surv':<7} denominator"
    return "```text\n" + header + "\n" + rows + "\n```"


# --- Command namespace projection (5.5E3e) -----------------------------------
# spec/commands.rs owns three separate closed namespaces and each entry's
# typed authority relation. docs/26 projects the two CLI inventories; the
# BatQL companion projects the source modes.
COMMANDS_DOC = "docs/26_COMMAND_PLANE.md"
COMPANION_DOC = "companion/BATQL_LANGUAGE.md"
COMMAND_NAMESPACES = (
    ("PRODUCT-COMMANDS", "ProductCommand", COMMANDS_DOC),
    ("TESTPAK-COMMANDS", "TestPakCommand", COMMANDS_DOC),
    ("BATQL-SOURCE-MODES", "BatQlSourceMode", COMPANION_DOC),
)


def parse_command_namespace(root, kind):
    """Ordered (token, shape, owner, delegates) rows in Kind::ALL order from
    the typed owner."""
    src = (root / "spec/commands.rs").read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[" + kind + r"\] = &\[(.*?)\];", src, re.S)
    inventory = re.findall(r"\b" + kind + r"::(\w+)", all_body.group(1)) if all_body else []
    tokens: dict[str, str] = {}
    for arm, tok in re.findall(
            r"\b" + kind + r"::(\w+(?:\s*\|\s*" + kind + r"::\w+)*) => \"([^\"]+)\",", src):
        for v in re.split(r"\s*\|\s*", arm):
            tokens[v.split("::")[-1]] = tok
    impl = re.search(r"impl " + kind + r" \{(.*?)\n\}", src, re.S)
    authority: dict[str, tuple[str, str, list[str]]] = {}
    for arm_body, direct, comp in re.findall(
            r"(" + kind + r"::\w+(?:\s*\|\s*" + kind + r"::\w+)*)\s*=>\s*\{?\s*"
            r"(?:CommandAuthority::Direct\(ContractId\(\"([^\"]+)\"\)\)"
            r"|CommandAuthority::(Composite \{.*?\},?))",
            impl.group(1) if impl else "", re.S):
        variants = re.findall(r"\b" + kind + r"::(\w+)", arm_body)
        if direct:
            entry = ("direct", direct, [])
        else:
            owner = re.search(r'composition_owner: ContractId\("([^"]+)"\)', comp)
            delegates = re.findall(r'ContractId\("([^"]+)"\)', comp.split("delegates:", 1)[-1])
            entry = ("composite", owner.group(1) if owner else "", delegates)
        for v in variants:
            authority[v] = entry
    missing = [v for v in inventory if v not in tokens or v not in authority]
    if not inventory or missing:
        raise Unadmitted(
            f"spec/commands.rs: {kind} inventory unreadable or arms missing for {missing}")
    return [(tokens[v], *authority[v]) for v in inventory]


def render_command_namespace(root, kind):
    rows = []
    for token, shape, owner, delegates in parse_command_namespace(root, kind):
        row = f"{token:<10} {shape:<10} {owner:<28} {' '.join(delegates)}".rstrip()
        rows.append(row)
    return "```text\n" + "\n".join(rows) + "\n```"


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


TESTPAK_DOC = "docs/12_TESTPAK.md"


def proof_terminals(root: Path) -> list[str]:
    src = (root / "spec/proof.rs").read_text(encoding="utf-8")
    start = src.index("pub const PROOF_UNIT_TERMINALS")
    names = re.findall(r"ProofUnitTerminal::(\w+)",
                       src[start: src.index("\n];", start)])
    if not names:
        raise Unadmitted("PROOF_UNIT_TERMINALS declares no terminals")
    return names


def render_proof_terminals(root: Path) -> str:
    return "\n".join(["```text"] + proof_terminals(root) + ["```"])


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


# --- The generated-view registry (5.5E4a) ------------------------------------
# spec/generated_views.rs owns which generated views exist. This projector
# parses the typed registry independently, dispatches through VIEW_RENDERERS,
# and builds EVERY projection plan by iterating the registry — the scattered
# hand-built plan denominator is dead, and a manual literal plan cannot
# bypass the registry.
GENERATED_VIEWS_SRC = "spec/generated_views.rs"
_VIEW_SPEC_ARM = re.compile(
    r"GeneratedView::(\w+) => GeneratedViewSpec \{\s*"
    r"authority_sources: &\[([^\]]*)\],\s*"
    r"target: GeneratedViewTarget::(?:Static\(&\[([^\]]*)\]\)|(EligibleMarkdownCorpus)),\s*"
    r"surface: GeneratedViewSurface::(\w+),\s*"
    r'marker: (?:Some\("([^"]+)"\)|None),\s*'
    r"generator: BootstrapToolId::(\w+),", re.S)


def parse_generated_views(root: Path) -> list[dict]:
    """The typed registry in GeneratedView::ALL order. Unknown or incomplete
    arms fail generation; there is no raw fallback."""
    src = (root / GENERATED_VIEWS_SRC).read_text(encoding="utf-8")
    all_body = re.search(
        r"pub const ALL: &'static \[GeneratedView\] = &\[(.*?)\];", src, re.S)
    if not all_body:
        raise SystemExit("project: spec/generated_views.rs declares no GeneratedView::ALL")
    order = re.findall(r"GeneratedView::(\w+)", all_body.group(1))
    arms: dict[str, dict] = {}
    for m in _VIEW_SPEC_ARM.finditer(src):
        arms[m.group(1)] = {
            "name": m.group(1),
            "sources": re.findall(r'"([^"]+)"', m.group(2)),
            "targets": re.findall(r'"([^"]+)"', m.group(3) or ""),
            "corpus": bool(m.group(4)),
            "surface": m.group(5),
            "marker": m.group(6),
            "generator": m.group(7),
        }
    views = []
    for name in order:
        if name not in arms:
            raise SystemExit(f"project: GeneratedView::{name} has no parseable spec() arm")
        views.append(arms[name])
    return views


def canonical_block(marker: str, source: str, body: str) -> str:
    """One embedded view, marker AND body: the BEGIN provenance is generated
    mechanically from the registry, never preserved from the target. Multiple
    authority sources are joined in registry order by "; " — source order is
    provenance, and reordering without changing the set is drift."""
    return (f"<!-- {marker}:BEGIN generated from {source} by bootstrap/project.py; "
            f"do not edit -->\n{body}\n<!-- {marker}:END -->")


def render_tracked_toolchain(root: Path) -> str:
    """The tracked root rust-toolchain.toml: provenance line plus the bare
    selection; byte-equal to ToolchainProfile::tracked_root_toolchain_toml."""
    tc = parse_toolchain(root)
    if not all(tc.values()):
        raise Unadmitted("spec/toolchain.rs: incomplete ToolchainProfile; the projection refuses")
    return ("# generated from spec/toolchain.rs by bootstrap/project.py; do not edit\n"
            + render_root_toolchain(tc))


# --- Repository inventory projections (5.5E4b) -------------------------------
# The typed owners already contain the truth; these renderers parse them
# directly. No Python package-name, workspace-path, class-spelling, edge-class,
# environment, gate, or receipt-name map exists here.
_PKG_SPEC_ROW = re.compile(
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "([^"]*)",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
_EDGE_ROW = re.compile(
    r'EdgeSpec \{ importer: PackageId::(\w+), importee: PackageId::(\w+), '
    r'class: EdgeClass::(\w+), profile: "([^"]*)" \}')
_QUAL_PROFILE_ROW = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "([^"]+)",\s*\}', re.S)


def _class_spelling_arms(source: str, type_name: str) -> dict[str, str]:
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", source, re.S)
    return dict(re.findall(r"\b" + type_name + r'::(\w+) => "([^"]*)",',
                           impl.group(1))) if impl else {}


def _package_projection(source: str, fn_name: str) -> dict[str, str]:
    body = re.search(
        r"pub const fn " + fn_name + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        source, re.S)
    return dict(re.findall(r'PackageId::(\w+) => "([^"]+)",', body.group(1))) if body else {}


def _environment_spellings(source: str) -> dict[str, str]:
    return dict(re.findall(r'QualificationEnvironment::(\w+) => "([^"]+)",', source))


def render_package_inventory(root: Path) -> str:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    cargo = _package_projection(src, "cargo_name")
    wpath = _package_projection(src, "workspace_path")
    classes = _class_spelling_arms(src, "PackageClass")
    lines = ["| Package | Class | Layer | Workspace path | Role |",
             "| --- | --- | --- | --- | --- |"]
    for pid, role, cls, layer in _PKG_SPEC_ROW.findall(src):
        if pid not in cargo or pid not in wpath or cls not in classes:
            raise Unadmitted(f"package row {pid} does not resolve through the typed owner")
        lines.append(f"| {cargo[pid]} | {classes[cls]} | {layer} | {wpath[pid]} | {role} |")
    return "\n".join(lines)


def render_package_edges(root: Path) -> str:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    cargo = _package_projection(src, "cargo_name")
    classes = _class_spelling_arms(src, "EdgeClass")
    lines = ["| Importer | Importee | Class | Profile |",
             "| --- | --- | --- | --- |"]
    for importer, importee, cls, profile in _EDGE_ROW.findall(src):
        if importer not in cargo or importee not in cargo or cls not in classes:
            raise Unadmitted(f"edge {importer}->{importee} does not resolve through the typed owner")
        lines.append(f"| {cargo[importer]} | {cargo[importee]} | {classes[cls]} | {profile} |")
    return "\n".join(lines)


def render_qualification_profiles(root: Path) -> str:
    src = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    cargo = _package_projection(src, "cargo_name")
    environments = _environment_spellings(src)
    lines = ["| Package | Profile | Environment | Gates | Requirement |",
             "| --- | --- | --- | --- | --- |"]
    for pkg, profile, environment, gates, requirement in _QUAL_PROFILE_ROW.findall(src):
        if pkg not in cargo or environment not in environments:
            raise Unadmitted(f"qualification profile {pkg}:{profile} does not resolve")
        lines.append(f"| {cargo[pkg]} | {profile} | {environments[environment]} | "
                     f"{gate_tokens(gates, root)} | {requirement} |")
    return "\n".join(lines)


def _eligible_markdown(root: Path) -> list[Path]:
    out = []
    for path in sorted(root.rglob("*.md")):
        parts = path.relative_to(root).parts
        if any(part in (".git", "target", "__pycache__") for part in parts):
            continue
        out.append(path)
    return out


def render_bundle_inventory(root: Path) -> str:
    """Every count is DERIVED from an admitted typed denominator or the
    current tracked tree — never hardcoded, never substituted."""
    arch = (root / "spec/architecture.rs").read_text(encoding="utf-8")
    numbered = len(list((root / "docs").glob("[0-9][0-9]_*.md")))
    markdown = len(_eligible_markdown(root))
    packages = len(re.findall(r"\bPackageId::\w+,", re.search(
        r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];", arch, re.S).group(1)))
    if len(_PKG_SPEC_ROW.findall(arch)) != packages:
        raise Unadmitted("PACKAGES does not equal PackageId::ALL")
    edges = len(_EDGE_ROW.findall(arch))
    profiles = len(_QUAL_PROFILE_ROW.findall(arch))
    seeds = len(guarantee_seed(root))
    decisions = len(_DEC_ROW.findall((root / "spec/dispositions.rs").read_text(encoding="utf-8")))
    obligations = len(_LEG_ROW.findall(
        (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")))
    coverage_src = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", coverage_src,
        re.S).group(1))
    coverage_rows = re.findall(r"LegacyInvariantCoverage \{\s*legacy_id:", coverage_src)
    if len(coverage_rows) != len(manifest):
        raise Unadmitted("legacy coverage rows do not equal SOURCE_INVARIANT_IDS")
    operators = len(parse_operators(root))
    views = parse_generated_views(root)
    static_instances = sum(len(view["targets"]) for view in views)
    bindings = sum(1 for path in _eligible_markdown(root)
                   if re.search(r"^(?:source_)?reconciliation_epoch:",
                                path.read_text(encoding="utf-8"), re.M))
    rows = [
        ("numbered architecture documents", numbered,
         "current tracked docs matching docs/[0-9][0-9]_*.md"),
        ("Markdown documents", markdown, "current eligible Markdown corpus"),
        ("Cargo packages", packages, "PackageId::ALL with PACKAGES parity"),
        ("package edges", edges, "EDGES"),
        ("qualification profiles", profiles, "QUALIFICATION_PROFILES"),
        ("SEED guarantees", seeds, "spec/invariants.rs SEED inventory"),
        ("decision rows", decisions, "spec/dispositions.rs DECISIONS"),
        ("legacy semantic obligations", obligations,
         "spec/legacy_obligations.rs OBLIGATIONS"),
        ("legacy invariant declarations", len(manifest),
         "SOURCE_INVARIANT_IDS with COVERAGE parity"),
        ("BatQL operators", operators, "OperatorId::ALL with OPERATORS parity"),
        ("registered generated views", len(views), "GeneratedView::ALL"),
        ("static generated target instances", static_instances,
         "expansion of every Static registry target"),
        ("corpus-frontmatter bindings", bindings,
         "the eligible Markdown corpus reached by CorpusEpochMembership"),
    ]
    lines = ["| Metric | Count | Derivation |", "| --- | --- | --- |"]
    for metric, count, derivation in rows:
        lines.append(f"| {metric} | {count} | {derivation} |")
    return "\n".join(lines)


def render_tier0_receipts(root: Path) -> str:
    """The required Tier 0 receipt denominator, parsed from the harness that
    enforces it. This block declares WHAT is required; run receipts carry
    whether a particular run passed."""
    harness_path = root / "bootstrap/selftest.py"
    if not harness_path.is_file():
        raise Unadmitted("bootstrap/selftest.py is absent; the Tier 0 denominator refuses")
    src = harness_path.read_text(encoding="utf-8")
    body = re.search(r"REQUIRED_TIER0_RECEIPTS = \((.*?)\n\)", src, re.S)
    if not body:
        raise Unadmitted("bootstrap/selftest.py declares no REQUIRED_TIER0_RECEIPTS")
    rows = re.findall(r'\("([^"]+)", (True|False)\)', body.group(1))
    if not rows:
        raise Unadmitted("REQUIRED_TIER0_RECEIPTS parses to no rows")
    lines = ["| Receipt | Artifact-bound |", "| --- | --- |"]
    for name, bound in rows:
        lines.append(f"| {name} | {'yes' if bound == 'True' else 'no'} |")
    return "\n".join(lines)


# --- Exact-ledger projections (5.5E4c) ---------------------------------------
# docs/30 and docs/34 are exact mirrors whose complete rows already live in
# typed owners; both become generated views. No Python spelling, meaning,
# count, or ordering map — unknown variants refuse generation.
_FULL_DEC_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+), '
    r'subject: "((?:[^"\\]|\\.)*)", successor: "((?:[^"\\]|\\.)*)"')
_COVERAGE_FULL_ROW = re.compile(
    r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", '
    r'disposition: CoverageDisposition::(\w+), successor: "((?:[^"\\]|\\.)*)", '
    r'rationale: "((?:[^"\\]|\\.)*)" \}')


def _fn_str_arms(source: str, type_name: str, fn_name: str) -> dict[str, str]:
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", source, re.S)
    body = impl.group(1) if impl else ""
    fn = re.search(
        r"pub const fn " + fn_name + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        body, re.S)
    return dict(re.findall(
        r"\b" + type_name + r'::(\w+) => \{?\s*"((?:[^"\\]|\\.)*)"', fn.group(1))) \
        if fn else {}


def _all_variants(source: str, type_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", source, re.S)
    return re.findall(r"\b" + type_name + r"::(\w+)", body.group(1)) if body else []


def render_decision_ledger(root: Path) -> str:
    src = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    tags = _fn_str_arms(src, "Disposition", "spelling")
    meanings = _fn_str_arms(src, "Disposition", "meaning")
    classes = _fn_str_arms(src, "DecisionClass", "spelling")
    order = _all_variants(src, "Disposition")
    if not order:
        raise Unadmitted("spec/dispositions.rs declares no Disposition::ALL")
    lines = ["| Tag | Meaning |", "| --- | --- |"]
    for variant in order:
        if variant not in tags or variant not in meanings:
            raise Unadmitted(f"Disposition::{variant} lacks a spelling or meaning arm")
        lines.append(f"| {tags[variant]} | {meanings[variant]} |")
    lines += ["", "| ID | Tag | Class | Gates | Subject | Final ruling |",
              "| --- | --- | --- | --- | --- | --- |"]
    rows = _FULL_DEC_ROW.findall(src)
    if not rows:
        raise Unadmitted("spec/dispositions.rs declares no DecisionSpec rows")
    for dec_id, cls, gates, disp, subject, successor in rows:
        if disp not in tags or cls not in classes:
            raise Unadmitted(f"{dec_id} carries an unadmitted class or disposition")
        lines.append(f"| {dec_id} | {tags[disp]} | {classes[cls]} | "
                     f"{gate_tokens(gates, root)} | {subject} | {successor} |")
    return "\n".join(lines)


def render_legacy_coverage(root: Path) -> str:
    src = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    tags = _fn_str_arms(src, "CoverageDisposition", "spelling")
    meanings = _fn_str_arms(src, "CoverageDisposition", "meaning")
    order = _all_variants(src, "CoverageDisposition")
    if not order:
        raise Unadmitted("spec/legacy_invariant_coverage.rs declares no CoverageDisposition::ALL")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", src, re.S).group(1))
    rows = _COVERAGE_FULL_ROW.findall(src)
    if {r[0] for r in rows} != set(manifest) or len(rows) != len(manifest):
        raise Unadmitted("COVERAGE does not equal SOURCE_INVARIANT_IDS set parity")
    lines = ["| Disposition | Meaning |", "| --- | --- |"]
    for variant in order:
        if variant not in tags or variant not in meanings:
            raise Unadmitted(f"CoverageDisposition::{variant} lacks a spelling or meaning arm")
        lines.append(f"| {tags[variant]} | {meanings[variant]} |")
    lines += ["", f"The source declaration denominator is the {len(manifest)}-declaration "
              "`SOURCE_INVARIANT_IDS` manifest, in exact set parity with the coverage "
              "rows below.",
              "", "| Legacy invariant | Disposition | Clean successor | Rationale |",
              "| --- | --- | --- | --- |"]
    for inv, disp, successor, rationale in rows:
        if disp not in tags:
            raise Unadmitted(f"coverage row {inv} carries unadmitted disposition {disp}")
        lines.append(f"| `{inv}` | {tags[disp]} | `{successor}` | {rationale} |")
    return "\n".join(lines)


_VIEW_SURFACE_DISPLAY = {"EmbeddedBlock": "embedded-block",
                         "StandaloneFile": "standalone-file",
                         "CorpusFrontmatter": "corpus-frontmatter"}
_VIEW_GENERATOR_DISPLAY = {"ProjectPy": "bootstrap/project.py"}


def render_generated_view_registry(root: Path) -> str:
    """The registry's own projection (docs/28), including its own row."""
    lines = ["| View | Surface | Authority source(s) | Target | Marker | Generator |",
             "| --- | --- | --- | --- | --- | --- |"]
    for view in parse_generated_views(root):
        surface = _VIEW_SURFACE_DISPLAY.get(view["surface"])
        generator = _VIEW_GENERATOR_DISPLAY.get(view["generator"])
        if surface is None or generator is None:
            raise Unadmitted(
                f"generated view {view['name']} carries an unregistered surface "
                f"or generator spelling")
        target = "; ".join(view["targets"]) if view["targets"] else "eligible markdown corpus"
        lines.append(
            f"| {view['name']} | {surface} | {'; '.join(view['sources'])} | "
            f"{target} | {view['marker'] or '-'} | {generator} |")
    return "\n".join(lines)


VIEW_RENDERERS = {
    "OperatorsCatalog": lambda root: render_catalog(parse_operators(root)),
    "OperatorsSurfaces": lambda root: render_surfaces(parse_operators(root)),
    "OperatorsGrammar": lambda root: render_grammar(parse_operators(root)),
    "OperatorsProjection": lambda root: render_projection(parse_operators(root)),
    "OperatorsTyping": lambda root: render_typing(parse_operators(root)),
    "OperatorsNumeric": lambda root: render_numeric(parse_operators(root)),
    "SeedClassification": render_seed_classification,
    "GuaranteeGraph": render_guarantee_graph,
    "PakVmSemanticIsa": render_pakvm_isa,
    "PakVmSignatures": render_pakvm_signatures,
    "SyncBatPlaneOwnership": render_syncbat_plane_ownership,
    "SyncBatAuthorities": render_syncbat_authorities,
    "SyncBatCrossings": render_syncbat_crossings,
    "ReconciliationCoordinates": render_recon_coordinates,
    "ReconciliationRetry": render_recon_retry,
    "ReleaseSeal": render_release_seal,
    "ProofTerminals": render_proof_terminals,
    "StaleVocabulary": render_stale_vocabulary,
    "GateInventory": render_gate_inventory,
    "ContractKinds": render_contract_kinds,
    "RustToolchain": render_tracked_toolchain,
    "IdentityCatalog": lambda root: render_identity_axis(root, "IdentityKind"),
    "GenerationCatalog": lambda root: render_identity_axis(root, "GenerationKind"),
    "BindingCatalog": lambda root: render_identity_axis(root, "BindingKind"),
    "VersionCatalog": lambda root: render_identity_axis(root, "VersionIdentityKind"),
    "IdentityResidue": render_identity_residue,
    "ProductCommands": lambda root: render_command_namespace(root, "ProductCommand"),
    "TestPakCommands": lambda root: render_command_namespace(root, "TestPakCommand"),
    "BatQlSourceModes": lambda root: render_command_namespace(root, "BatQlSourceMode"),
    "MutationLanes": render_mutation_lanes,
    "MutationResults": render_mutation_results,
    "PromotionRequirements": render_promotion,
    "CompilerAssumptionKinds": render_compiler_assumptions,
    "CorpusReconciliationEpoch": render_corpus_epoch,
    "CorpusEpochMembership": lambda root: parse_corpus_epoch(root)["spelling"],
    "PackageInventory": render_package_inventory,
    "PackageEdges": render_package_edges,
    "QualificationProfiles": render_qualification_profiles,
    "BundleInventory": render_bundle_inventory,
    "Tier0ReceiptDenominator": render_tier0_receipts,
    "DecisionLedger": render_decision_ledger,
    "LegacyInvariantCoverage": render_legacy_coverage,
    "GeneratedViewRegistry": render_generated_view_registry,
}


def build_plans(root: Path, findings: list[str]):
    """Every projection plan, built by iterating the typed registry. The
    renderer-key set must equal the registry in both directions: a registered
    view with no renderer refuses, and a renderer with no registered view
    refuses."""
    views = parse_generated_views(root)
    names = [v["name"] for v in views]
    for missing in [n for n in names if n not in VIEW_RENDERERS]:
        findings.append(f"registered generated view {missing} has no projector renderer")
    for phantom in sorted(set(VIEW_RENDERERS) - set(names)):
        findings.append(f"projector renderer {phantom} serializes no registered view")
    texts: dict[str, list] = {}

    def load(rel: str) -> list:
        if rel not in texts:
            path = root / rel
            original = path.read_text(encoding="utf-8") if path.is_file() else ""
            texts[rel] = [path, original, original]
        return texts[rel]

    epoch_spelling = None
    for view in views:
        name = view["name"]
        render = VIEW_RENDERERS.get(name)
        if render is None:
            continue
        if view["surface"] == "EmbeddedBlock":
            if not view["marker"] or not view["sources"] or not view["targets"]:
                findings.append(
                    f"embedded generated view {name} lacks a marker, authority "
                    f"source, or static target")
                continue
            try:
                body = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
                continue
            pattern = block_pattern(view["marker"])
            replacement = canonical_block(view["marker"], "; ".join(view["sources"]), body)
            for rel in view["targets"]:
                entry = load(rel)
                if not entry[2]:
                    findings.append(f"{rel}: missing generated-view target file")
                    continue
                if not pattern.search(entry[2]):
                    findings.append(
                        f"{rel}: missing generated block markers for {view['marker']}")
                    continue
                entry[2] = pattern.sub(lambda m, r=replacement: r, entry[2])
        elif view["surface"] == "StandaloneFile":
            if len(view["targets"]) != 1:
                findings.append(
                    f"standalone generated view {name} must name exactly one target")
                continue
            try:
                content = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
                continue
            load(view["targets"][0])[2] = content
        elif view["surface"] == "CorpusFrontmatter":
            try:
                epoch_spelling = render(root)
            except Unadmitted as exc:
                findings.append(f"{name} does not admit: {exc}")
        else:
            findings.append(
                f"generated view {name} carries unknown surface {view['surface']!r}")
    corpus_bindings = 0
    if epoch_spelling is not None:
        for path in sorted(root.rglob("*.md")):
            rel_parts = path.relative_to(root).parts
            if any(part in (".git", "target", "__pycache__") for part in rel_parts):
                continue
            entry = load(path.relative_to(root).as_posix())
            entry[2] = converge_epoch_frontmatter(entry[2], epoch_spelling)
            if re.search(r"^(?:source_)?reconciliation_epoch:", entry[2], re.M):
                corpus_bindings += 1
    plans = [(entry[0], entry[1], entry[2]) for entry in texts.values()]
    return views, plans, corpus_bindings


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in ("--write", "--check"):
        print("usage: project.py --write|--check [root]", file=sys.stderr)
        return 2
    mode = sys.argv[1]
    root = Path(sys.argv[2] if len(sys.argv) > 2 else ".").resolve()
    findings: list[str] = []
    views, plans, corpus_bindings = build_plans(root, findings)
    static_instances = sum(len(view["targets"]) for view in views)
    if findings:
        print(f"project: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    if mode == "--write":
        changed = sum(1 for _path, original, rewritten in plans if rewritten != original)
        for path, original, rewritten in plans:
            if rewritten != original:
                path.write_bytes(rewritten.encode("utf-8"))
        verb = "WROTE" if changed else "OK"
        print(f"project: {verb} {len(views)} registered generated views "
              f"({static_instances} static target instances; "
              f"{corpus_bindings} corpus-frontmatter bindings)")
        return 0
    # --check
    stale = [path.name for path, original, rewritten in plans if rewritten != original]
    if stale:
        print(f"project: FAIL (registered generated views stale in {stale}; "
              "run project.py --write)", file=sys.stderr)
        return 1
    print(f"project: PASS ({len(views)} registered generated views current; "
          f"{static_instances} static target instances; "
          f"{corpus_bindings} corpus-frontmatter bindings)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

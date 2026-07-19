"""Gates, the derived Guarantee Graph (DEC-070), and the exact decision and
legacy-invariant-coverage ledgers.

Owns the shared admission exception (Unadmitted), the gate inventory and token
rendering, the SEED classification, the typed guarantee-view admission, and the
exact-mirror decision and coverage ledgers. Standard library only; shares no
derivation logic with bootstrap/audit.py.
"""
from __future__ import annotations

import re

from .registry import _spec_module_source


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
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", '
    r'legacy_evidence: "(?:[^"\\]|\\.)*", clean_owner: "([^"]+)", '
    r'mechanism_disposition: "(?:[^"\\]|\\.)*", witness_requirement: '
    r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
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
    src = _spec_module_source(root, "architecture")
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
    src = _decomment(_spec_module_source(root, "guarantees"))
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
    arch = _spec_module_source(root, "architecture")
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


def render_gate_inventory(root: Path) -> str:
    """The docs/25 gate inventory, projected from spec/gates.rs."""
    rows = ["| GateId | Token | Title |", "| --- | --- | --- |"]
    for gid, token, title in gate_inventory(root):
        rows.append(f"| {gid} | {token} | {title} |")
    return "\n".join(rows)


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

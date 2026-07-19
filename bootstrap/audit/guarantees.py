"""Guarantee graph, gate identity, and decision classification.

The auditor half of the guarantee graph (DEC-070): it derives guarantee nodes
and edges from the typed source facts with its own parsers, admits every
projected field through the family policy, owns gate-identity reading, decision
classification (DEC-072), and the generic guarantee index used by proof-target
resolution. Shares no derivation logic with bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .corpus import (
    G_DEC_ROW,
    G_LEG_ROW,
    G_QUAL_ROW,
    _spec_module_source,
    _uncomment,
    declared_contract_ids,
)

# --- Guarantee graph (DEC-070): INDEPENDENT derivation, comparison, rules -----
# This is the AUDITOR half. It derives guarantee nodes/edges from the source
# facts with its own parsers and rules, then compares against the generated
# artifacts. It shares no parsing, normalization, lifetime derivation, edge
# construction, cycle detection, serialization, or comparison logic with
# bootstrap/project.py.
# Independent gate identity reader. This deliberately does NOT share parsing,
# ordering, or rendering with bootstrap/project.py: the auditor extracts variants
# by pattern rather than by positional split, and builds its own canonical order
# from spec/gates/inventory.rs. The oracle cannot manufacture the evidence it
# grades.
A_GATE_SPEC = re.compile(r'GateSpec \{ id: GateId::(\w+), token: "([^"]*)", title: "([^"]*)" \}')
A_GATE_VARIANT = re.compile(r'GateId::(\w+)')
A_GATE_ENUM = re.compile(r'pub enum GateId \{([^}]*)\}', re.S)


def gate_inventory(root: Path) -> list[tuple[str, str, str]]:
    return A_GATE_SPEC.findall(
        (root / "spec/gates/inventory.rs").read_text(encoding="utf-8"))


def gate_enum_variants(root: Path) -> list[str]:
    m = A_GATE_ENUM.search((root / "spec/gates/types.rs").read_text(encoding="utf-8"))
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
    "Receiptcheck": "receiptcheck",
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
G_PKG_ROW = re.compile(
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "[^"]+",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)


def g_package_projection(root: Path, fn: str) -> dict[str, str]:
    """PackageId variant -> projected value, parsed from the owning const fn
    (cargo_name or workspace_path). This auditor reconstructs the projections
    independently; it carries no hand-maintained package map."""
    src = _uncomment(_spec_module_source(root, "architecture"))
    body = re.search(
        r"pub const fn " + re.escape(fn)
        + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        src, re.S)
    return dict(re.findall(r'PackageId::(\w+) => "([^"]+)",', body.group(1))) if body else {}


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
    src = (root / "spec/invariants/inventory.rs").read_text(encoding="utf-8")
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
G_REL_TAG_PREFIX = {"leg": "LEG-", "dec": "DEC-", "seed": "SEED-"}


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
    leg = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    out = {}
    for lid, owner, gate, compat, deletion, status in G_LEG_ROW.findall(leg):
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
# policy the projector parses -- spec/guarantees/policy.rs
# GUARANTEE_FAMILY_POLICIES and the derivations owned by
# spec/dispositions/types.rs -- and verifies that every
# projected field traces to it. Independent parsers are useful; duplicate policy
# is not independence. The nine constants that used to live in both scripts are
# gone from both.
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
    src = _uncomment(_spec_module_source(root, "guarantees"))
    out: dict[str, dict] = {}
    for m in A_FAM_ROW.finditer(src):
        fam, krule, kconst, orule, oconst, lrule, lconst, grule, gconst, wit = m.groups()
        out[fam] = {"kind": (krule, kconst), "owner": (orule, oconst),
                    "lifetime": (lrule, lconst), "gate_posture": (grule, gconst),
                    "witness": wit}
    return out


def decision_lifetime_map(root: Path) -> dict[str, str]:
    """Disposition -> GuaranteeLifetime from the typed owner's match arms."""
    src = _uncomment((root / "spec/dispositions/types.rs").read_text(encoding="utf-8"))
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
    src = _uncomment((root / "spec/dispositions/types.rs").read_text(encoding="utf-8"))
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
    dec = (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")
    for did, dcls, dgates, disp in G_DEC_ROW.findall(dec):
        # Decision gates are node metadata; they never become graph edges.
        node = _admit("DEC", did, {
            "gates": gate_render(gate_list(dgates), root), "disposition": disp, "class": dcls,
        }, f"Decision({disp})")
        if node is not None:
            node["dclass"] = dcls
        nodes.append(node)
    arch = _spec_module_source(root, "architecture")
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
    if len(seed_rows) != 32:
        out.append(f"SEED classification has {len(seed_rows)} rows, expected 32")
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
        out.append("spec/gates/inventory.rs declares no GATES inventory")
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
    src_inv = (root / "spec/invariants/inventory.rs").read_text(encoding="utf-8")
    src_leg = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    if re.search(r'\bgates?:\s*"', src_inv):
        out.append("spec/invariants/inventory.rs still carries a string gate field")
    if re.search(r'\bgates?:\s*"', src_leg):
        out.append("spec/legacy_obligations/inventory.rs still carries a string gate field")
    return out


GATE_BLOCK_RE = re.compile(
    r"<!-- GATE-INVENTORY:BEGIN[^>]*-->\n(.*?)\n<!-- GATE-INVENTORY:END -->", re.S
)


def gate_doc_findings(root: Path) -> list[str]:
    """docs/25 gate inventory block equals spec/gates/inventory.rs, recomputed
    independently."""
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
        return ["docs/25 GATE-INVENTORY block does not equal spec/gates/inventory.rs"]
    return []


# --- Decision classification and gate binding (DEC-072) ---------------------
# The AUDITOR half. Parses spec/dispositions/inventory.rs independently of
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
    src = (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")
    out = []
    for did, cls, gates, disp, subject in A_DEC_CLASS_ROW.findall(src):
        out.append({"id": did, "class": cls, "gate_names": gate_list(gates),
                    "disposition": disp, "subject": subject})
    return out


def decision_class_findings(root: Path) -> list[str]:
    rows = decision_rows(root)
    out: list[str] = []
    declared = len(re.findall(r'DecisionSpec \{ id: "DEC-', (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")))
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
    arch = _spec_module_source(root, "architecture")
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


"""Domain projections: PakVM ISA, SyncBat firewall, reconciliation, the Gate-0
materialization plan, promotion, verification plans, proof relations, and the
candidate-sprouting policy.

Each renderer parses its typed owner directly and serializes admitted facts
only; a fact the spec does not author raises Unadmitted rather than being filled
in here. Standard library only; shares no parser with bootstrap/audit.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .guarantees import Unadmitted, gate_tokens
from .registry import _spec_module_source, parse_generated_views


# --- Promotion requirements projection (5.5E3i) ------------------------------
# spec/promotion/types.rs owns the conjunctive denominator; docs/12 carries the
# generated table. This projector parses the typed owner — never a Python
# list of the four requirements.
def parse_promotion(root):
    src = (root / "spec/promotion/types.rs").read_text(encoding="utf-8")
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
                f"spec/promotion/types.rs: PromotionRequirement::{v} facts unreadable")
        rows.append((s, owner.group(1), basis.group(1)))
    surface = re.search(r"ProofPolicySurface::(\w+);", src)
    change = re.search(
        r'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec\("([^"]+)"\);', src)
    egate = re.search(r"PROMOTION_ENFORCEMENT_GATE: GateId = GateId::(\w+);", src)
    rgate = re.search(r"PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::(\w+);", src)
    if not rows or not surface or not change or not egate or not rgate:
        raise Unadmitted("spec/promotion/types.rs: family facts unreadable")
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


SYSTEM_DOC = "docs/02_SYSTEM_MODEL.md"
RECON_SPEC = "spec/reconciliation/inventory.rs"
RECON_TYPES = "spec/reconciliation/types.rs"


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
    src = (root / RECON_TYPES).read_text(encoding="utf-8")
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
    src = (root / "spec/proof/types.rs").read_text(encoding="utf-8")
    start = src.index("pub const PROOF_UNIT_TERMINALS")
    names = re.findall(r"ProofUnitTerminal::(\w+)",
                       src[start: src.index("\n];", start)])
    if not names:
        raise Unadmitted("PROOF_UNIT_TERMINALS declares no terminals")
    return names


def render_proof_terminals(root: Path) -> str:
    return "\n".join(["```text"] + proof_terminals(root) + ["```"])


# --- PakVM semantic ISA projection (D4c1) -----------------------------------
# GENERATOR half. `bootstrap/audit.py` re-derives all of this independently and
# compares; the two share no parsing or admission logic. Neither may author node
# semantics: the `spec/pakvm_isa/` domain owns them, this reads them, and a node
# whose spec does not admit is not projected at all.

ISA_SPEC = "spec/pakvm_isa/"
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


def pakvm_specs(root: Path) -> list[dict]:
    """Admitted PakVM node specs, derived independently of the pakvm_isa
    domain's own `admit` (spec/pakvm_isa/admission.rs). A node that does not
    admit raises: a projector serializes an admitted spec and may never
    complete one."""
    src = _spec_module_source(root, "pakvm_isa")
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
    body = _const_fn_body(src, "source_origin")
    origins = dict(re.findall(r"PakVmAlgebra::(\w+)\s*=>\s*\"([^\"]*)\"", body))
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


FIREWALL_SPEC = "spec/syncbat_firewall/types.rs"
FIREWALL_INVENTORY = "spec/syncbat_firewall/inventory.rs"
ARCH_SPEC = "spec/architecture/types.rs"
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
    arch = _spec_module_source(root, "architecture")
    src = (root / FIREWALL_SPEC).read_text(encoding="utf-8")
    inv = (root / FIREWALL_INVENTORY).read_text(encoding="utf-8")
    # Plane identity and the ownership sentences: spec/architecture/types.rs owns both.
    # The inventory is SyncBatPlane::ALL (5.5E5); the raw SYNCBAT_PLANES alias
    # retired with the isolated-materializer closure.
    m = re.search(r"pub const ALL: &'static \[SyncBatPlane\] = &\[(.*?)\];", arch, re.S)
    order = re.findall(r"SyncBatPlane::(\w+)", m.group(1)) if m else []
    if not order:
        raise Unadmitted(f"{ARCH_SPEC}: SyncBatPlane::ALL lists no plane")
    sentences = dict(re.findall(
        r'SyncBatPlane::(\w+)\s*=>\s*"([^"]*)"',
        _method_body(arch, "authored_ownership", ARCH_SPEC)))
    for plane in order:
        if not sentences.get(plane):
            raise Unadmitted(f"{ARCH_SPEC}: plane {plane} authors no ownership sentence")
    # Authority ownership: spec/syncbat_firewall/types.rs owns it.
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
    # The legal-crossing whitelist: spec/syncbat_firewall/inventory.rs owns it.
    crossings = []
    for row in _struct_rows(inv, "SYNCBAT_LEGAL_CROSSINGS", "SyncBatCrossing"):
        frm = row.get("from", "").split("::")[-1]
        to = row.get("to", "").split("::")[-1]
        carries = row.get("carries", "").split("::")[-1]
        posture = row.get("posture", "").split("::")[-1]
        law = row.get("law", "").strip().strip('"')
        for plane in (frm, to):
            if plane not in order:
                raise Unadmitted(f"{FIREWALL_INVENTORY}: a crossing names {plane!r}, which is no plane")
        if carries not in listed:
            raise Unadmitted(f"{FIREWALL_INVENTORY}: a crossing carries {carries!r}, "
                             f"which SYNCBAT_AUTHORITIES does not author")
        # No default. An absent posture is not silently Optional; it is a refusal.
        if posture not in ("Required", "Optional"):
            raise Unadmitted(f"{FIREWALL_INVENTORY}: the {frm} -> {to} crossing states no "
                             f"requiredness posture (got {posture!r})")
        if not law:
            raise Unadmitted(f"{FIREWALL_INVENTORY}: the {frm} -> {to} crossing states no law")
        crossings.append({"from": frm, "to": to, "carries": carries,
                          "posture": posture, "law": law})
    if not crossings:
        raise Unadmitted(f"{FIREWALL_INVENTORY}: SYNCBAT_LEGAL_CROSSINGS lists no crossing")
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


# --- The Gate-0 materialization plan (5.5E5) ---------------------------------
# spec/bootstrap_output/types.rs owns the output shape; spec/architecture/types.rs
# owns package and plane membership; spec/toolchain/types.rs owns the toolchain
# file the plan carries. This renderer expands the complete candidate file plan in
# canonical order -- roots, then packages, then SyncBat planes -- and derives
# every path from the typed owners. It carries no file table of its own.

BO_SPEC = "spec/bootstrap_output/types.rs"


def gate0_plan_facts(root: Path) -> list[dict]:
    bo = (root / BO_SPEC).read_text(encoding="utf-8")
    arch = _spec_module_source(root, "architecture")

    def enum_all(src: str, enum: str, where: str) -> list[str]:
        # A hand-written `pub const ALL` or, for the closed Gate-0 vocabularies,
        # the `gate0_closed_enum!` variant list that macro-generates the
        # identical inventory (5.5E5a).
        m = re.search(r"pub const ALL: &'static \[" + re.escape(enum) + r"\] = &\[(.*?)\];",
                      src, re.S)
        if m:
            got = re.findall(re.escape(enum) + r"::(\w+)", m.group(1))
        else:
            head = src.find(f"pub enum {enum} {{")
            got = []
            if head >= 0:
                i = src.find("{", head)
                depth = 0
                for j in range(i, len(src)):
                    if src[j] == "{":
                        depth += 1
                    elif src[j] == "}":
                        depth -= 1
                        if depth == 0:
                            got = re.findall(r"^\s+([A-Z]\w*),", src[i + 1: j], re.M)
                            break
        if not got:
            raise Unadmitted(f"{where}: {enum}::ALL lists nothing")
        return got

    def arm_map(src: str, fn: str, key: str, where: str) -> dict[str, str]:
        # impl methods close at four-space indent; matching a column-0 brace
        # would swallow every later method's arms into the same dict
        m = re.search(r"fn " + re.escape(fn) + r"\(self\)[^{]*\{(.*?)\n    \}", src, re.S)
        if not m:
            raise Unadmitted(f"{where}: declares no {fn}()")
        return dict(re.findall(re.escape(key) + r"::(\w+) => \"([^\"]*)\"", m.group(1)))

    roots = enum_all(bo, "Gate0RootArtifact", BO_SPEC)
    root_paths = arm_map(bo, "relative_path", "Gate0RootArtifact", BO_SPEC)
    package_families = enum_all(bo, "Gate0PackageArtifact", BO_SPEC)
    plane_families = enum_all(bo, "Gate0PlaneArtifact", BO_SPEC)
    packages = enum_all(arch, "PackageId", ARCH_SPEC)
    package_paths = arm_map(arch, "workspace_path", "PackageId", ARCH_SPEC)

    # target kinds and their source doors
    kind_m = re.search(r"pub const fn target_kind\(package: PackageId\)[^{]*\{(.*?)\n\}",
                       bo, re.S)
    if not kind_m:
        raise Unadmitted(f"{BO_SPEC}: declares no target_kind()")
    kinds = dict(re.findall(r"PackageId::(\w+) => Gate0PackageTargetKind::(\w+)",
                            kind_m.group(1)))
    door_m = re.search(r"pub const fn source_suffix\(package: PackageId\)[^{]*\{(.*?)\n\}",
                       bo, re.S)
    if not door_m:
        raise Unadmitted(f"{BO_SPEC}: declares no source_suffix()")
    doors: dict[str, str] = {}
    for pattern, suffix in re.findall(
            r"((?:Gate0PackageTargetKind::\w+\s*\|?\s*)+)=> \"([^\"]+)\"", door_m.group(1)):
        for kind in re.findall(r"Gate0PackageTargetKind::(\w+)", pattern):
            doors[kind] = suffix

    planes = enum_all(arch, "SyncBatPlane", ARCH_SPEC)
    modules = arm_map(arch, "module_name", "SyncBatPlane", ARCH_SPEC)
    syncbat_base = package_paths.get("SyncBat", "")

    rows: list[dict] = []
    for artifact in roots:
        path = root_paths.get(artifact, "")
        if not path:
            raise Unadmitted(f"{BO_SPEC}: root artifact {artifact} projects no path")
        rows.append({"path": path, "family": f"Root / {artifact}",
                     "derivation": f"Gate0RootArtifact::{artifact}"})
    for package in packages:
        base = package_paths.get(package)
        kind = kinds.get(package)
        if not base or not kind:
            raise Unadmitted(f"{BO_SPEC}: package {package} lacks a path or target kind")
        for family in package_families:
            suffix = {"Manifest": "Cargo.toml", "Readme": "README.md",
                      "SourceDoor": doors.get(kind, "")}[family]
            if not suffix:
                raise Unadmitted(f"{BO_SPEC}: target kind {kind} projects no source door")
            rows.append({"path": f"{base}/{suffix}", "family": f"Package / {family}",
                         "derivation": f"PackageId::{package} ({kind})"})
    for plane in planes:
        module = modules.get(plane)
        if not module or not syncbat_base:
            raise Unadmitted(f"{ARCH_SPEC}: plane {plane} projects no module name")
        for family in plane_families:
            # A plane is a directory under the domain-directory grammar:
            # mod.rs is the plane facade, types.rs the private noun carrier.
            suffix = {"ModuleDoor": f"{module}/mod.rs",
                      "TypesCarrier": f"{module}/types.rs"}.get(family)
            if not suffix:
                raise Unadmitted(
                    f"{BO_SPEC}: plane artifact family {family} projects no path")
            rows.append({"path": f"{syncbat_base}/src/{suffix}",
                         "family": f"SyncBat plane / {family}",
                         "derivation": f"SyncBatPlane::{plane}"})
    seen: set[str] = set()
    for row in rows:
        if row["path"] in seen:
            raise Unadmitted(f"{BO_SPEC}: the expanded plan lists {row['path']} twice")
        seen.add(row["path"])
    return rows


def render_gate0_plan(root) -> str:
    out = ["| Path | Artifact family | Derivation |", "| --- | --- | --- |"]
    for row in gate0_plan_facts(root):
        out.append(f"| {row['path']} | {row['family']} | {row['derivation']} |")
    return "\n".join(out)


# --- Verification plans (5.5F2, DEC-077/DEC-078) -----------------------------
# The typed owners already contain the truth; these regexes parse them
# directly. No Python claim, method, coverage, lane, enforcement, or route
# spelling map exists here.
_V_PLAN_ROW = re.compile(
    r'pub const (PLAN_[A-Z0-9_]+): &\[VerificationRequirement\] = &\[(.*?)\];', re.S)
_V_REQ_ROW = re.compile(
    r'VerificationRequirement \{ method: VerificationMethod::(\w+), '
    r'basis: (VerificationBasis::\w+(?: \{ route: (?:None|Some\(IndependentEvidenceRouteKind::\w+\)|IndependentEvidenceRouteKind::\w+) \})?), '
    r'coverage: VerificationCoverage::(\w+), lane: VerificationLane::(\w+), '
    r'enforcement: VerificationEnforcementPosture::(\w+) \}')


def _render_basis(basis: str) -> str:
    """Fold the route (now carried inside VerificationBasis) into a single
    basis cell: ContractProjection, RuntimeObservation, DirectBoundary,
    DirectBoundary(HostileBoundary), IndependentReference(DifferentialImplementation),
    IndependentReference(IndependentHistoryReplay)."""
    variant = re.match(r"VerificationBasis::(\w+)", basis).group(1)
    route = re.search(r"IndependentEvidenceRouteKind::(\w+)", basis)
    if route:
        return f"{variant}({route.group(1)})"
    return variant
_V_ACTIVE_ROW = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
    r'ProofRowState::Active \{ [^}]*claim: VerificationClaimKind::(\w+), '
    r'verification: ([A-Z0-9_]+) \} \}')


def render_verification_plans(root: Path) -> str:
    """docs/38 VERIFICATION-PLANS: the named plans' exact axis tuples and
    every active proof row's claim + plan, parsed from spec/proof/inventory.rs."""
    src = (root / "spec/proof/inventory.rs").read_text(encoding="utf-8")
    lines = ["| Plan | Method | Basis | Coverage | Lane | Enforcement |",
             "| --- | --- | --- | --- | --- | --- |"]
    for name, body in _V_PLAN_ROW.findall(src):
        for m in _V_REQ_ROW.finditer(body):
            lines.append(f"| {name} | {m.group(1)} | {_render_basis(m.group(2))} | "
                         f"{m.group(3)} | {m.group(4)} | {m.group(5)} |")
    lines.append("")
    lines.append("| Active proof row | Claim | Plan |")
    lines.append("| --- | --- | --- |")
    for wid, claim, plan in _V_ACTIVE_ROW.findall(src):
        lines.append(f"| {wid} | {claim} | {plan} |")
    return "\n".join(lines)


# --- Proof relations and the complete legacy ledger (5.5E4d) -----------------
# spec/proof/inventory.rs owns proof-row identity, lifecycle, succession,
# guarantee binding, and projection membership; spec/legacy_obligations/inventory.rs
# owns the complete obligation row. No Python relation map, no LEG witness map, no
# heading parser, no ContractId-to-path map: domain selection is SOLELY the
# target document's authored contract_id matching an active row's
# projection_contracts, with the target path coming from the registry.
_PROOF_ACTIVE_RE = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
    r'ProofRowState::Active \{ guarantee: GuaranteeRef::(leg|dec)\("([^"]+)"\), '
    r'projection_contracts: &\[([^\]]*)\], '
    r'claim: VerificationClaimKind::(\w+), verification: ([A-Z0-9_]+) \} \}')
_PROOF_RETIRED_RE = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
    r'ProofRowState::Retired \{ successors: &\[([^\]]*)\] \}')
_LEG_FULL_RE = re.compile(
    r'LegacyObligation \{ id: "(LEG-\d+)", law: "((?:[^"\\]|\\.)*)", '
    r'legacy_evidence: "((?:[^"\\]|\\.)*)", clean_owner: "([^"]*)", '
    r'mechanism_disposition: "((?:[^"\\]|\\.)*)", witness_requirement: '
    r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("((?:[^"\\]|\\.)*)"\)), '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::(\w+), '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: '
    r'ObligationStatus::(\w+) \}')


def parse_proof_relations(root: Path):
    src = (root / "spec/proof/inventory.rs").read_text(encoding="utf-8")
    active = [{"id": m[0], "family": m[1], "guarantee": m[2],
               "contracts": re.findall(r'ContractId\("([^"]+)"\)', m[3])}
              for m in _PROOF_ACTIVE_RE.findall(src)]
    retired = [{"id": m[0], "successors": re.findall(r'ProofRowId\("([^"]+)"\)', m[1])}
               for m in _PROOF_RETIRED_RE.findall(src)]
    for row in active:
        if not row["contracts"]:
            raise Unadmitted(f"active proof row {row['id']} names no projection contract")
    return active, retired


def _doc_contract_id(root: Path, rel: str) -> str:
    text = (root / rel).read_text(encoding="utf-8")
    m = re.match(r"---\n(.*?)\n---\n", text, re.S)
    front = dict(re.findall(r"^(\w+):\s*(.+)$", m.group(1), re.M)) if m else {}
    contract = front.get("contract_id", "")
    if not contract:
        raise Unadmitted(f"{rel} declares no contract_id frontmatter")
    return contract


def render_legacy_obligation_ledger(root: Path) -> str:
    src = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    active, _retired = parse_proof_relations(root)
    by_leg: dict[str, list[str]] = {}
    for row in active:
        if row["family"] == "leg":
            by_leg.setdefault(row["guarantee"], []).append(row["id"])
    rows = _LEG_FULL_RE.findall(src)
    if not rows:
        raise Unadmitted("spec/legacy_obligations/inventory.rs declares no complete obligation rows")
    known = {r[0] for r in rows}
    for guarantee in by_leg:
        if guarantee not in known:
            raise Unadmitted(f"a proof relation binds {guarantee}, which no obligation row declares")
    lines = ["| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition "
             "| Required witness | Gates | Compatibility | Deletion condition | Status |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for (leg, law, evidence, owner, mech, planned, gates, compat, deletion,
         status) in rows:
        canonical = "CanonicalProofRows" in re.search(
            r'id: "' + leg + r'".*?witness_requirement: (\S+?)[,(]', src, re.S).group(1)
        if canonical:
            if planned:
                raise Unadmitted(f"{leg} carries both witness routes")
            if not by_leg.get(leg):
                raise Unadmitted(f"{leg} claims CanonicalProofRows with no active relation")
            witness = "; ".join(by_leg[leg])
        else:
            if not planned:
                raise Unadmitted(f"{leg} has no witness route")
            if by_leg.get(leg):
                raise Unadmitted(f"{leg} carries a planned witness AND active proof rows")
            witness = planned
        lines.append(f"| {leg} | {law} | {evidence} | {owner} | {mech} | {witness} | "
                     f"{gate_tokens(gates, root)} | {compat} | {deletion} | {status} |")
    return "\n".join(lines)


def render_proof_relations(root: Path) -> str:
    active, _retired = parse_proof_relations(root)
    groups: dict[tuple, list[str]] = {}
    for row in active:
        key = (row["family"], row["guarantee"], tuple(row["contracts"]))
        groups.setdefault(key, []).append(row["id"])
    lines = ["| Guarantee | Projection contract(s) | Active proof rows |",
             "| --- | --- | --- |"]
    for (_family, guarantee, contracts), ids in groups.items():
        lines.append(f"| {guarantee} | {'; '.join(contracts)} | {'; '.join(ids)} |")
    return "\n".join(lines)


def _proof_requirements_renderer(view_name: str):
    def render(root: Path) -> str:
        views = {v["name"]: v for v in parse_generated_views(root)}
        view = views.get(view_name)
        if view is None or not view["targets"]:
            raise Unadmitted(f"{view_name} is not a registered domain proof view")
        contract = _doc_contract_id(root, view["targets"][0])
        active, _retired = parse_proof_relations(root)
        groups: dict[str, list[str]] = {}
        for row in active:
            if contract in row["contracts"]:
                groups.setdefault(row["guarantee"], []).append(row["id"])
        if not groups:
            raise Unadmitted(f"{view_name}: no active proof relation names {contract}")
        lines = ["| Guarantee | Required proof rows |", "| --- | --- |"]
        for guarantee, ids in groups.items():
            lines.append(f"| {guarantee} | {'; '.join(ids)} |")
        return "\n".join(lines)
    return render


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


# --- Sprouting vocabulary and specialized-plan policy (5.5F3, BP-SPROUTING-1) -
# spec/sprouting/types.rs owns the candidate-sprouting vocabulary;
# spec/sprouting/inventory.rs owns the DEC-073 specialized-plan candidate
# policy instance. These projectors parse the typed owners directly; no Python
# origin, class, role, posture, authority, or requirement spelling map exists
# here.
_SPROUTING_VOCAB = (
    ("Candidate origins", "CANDIDATE_ORIGIN_KINDS"),
    ("Candidate change classes", "CANDIDATE_CHANGE_CLASSES"),
    ("Evaluation set roles", "EVALUATION_SET_ROLES"),
    ("Realization postures", "REALIZATION_POSTURES"),
    ("Repair authorities", "REPAIR_AUTHORITIES"),
)


def _sprouting_const_variants(src: str, const_name: str) -> list[str]:
    body = re.search(
        r"pub const " + const_name + r":[^=]*= &\[(.*?)\];", src, re.S)
    if not body:
        raise Unadmitted(f"spec/sprouting/types.rs: {const_name} inventory unreadable")
    variants = re.findall(r"\w+::(\w+)", body.group(1))
    if not variants:
        raise Unadmitted(f"spec/sprouting/types.rs: {const_name} names no variants")
    return variants


def render_sprouting_vocabulary(root: Path) -> str:
    """docs/39 SPROUTING-VOCABULARY: the five frozen candidate-sprouting axes,
    each variant in its authored inventory order, parsed from spec/sprouting/types.rs."""
    src = (root / "spec/sprouting/types.rs").read_text(encoding="utf-8")
    sections = []
    for heading, const_name in _SPROUTING_VOCAB:
        variants = _sprouting_const_variants(src, const_name)
        sections.append(heading + "\n" + "\n".join(f"  {v}" for v in variants))
    return "```text\n" + "\n\n".join(sections) + "\n```"


def render_specialized_plan_policy(root: Path) -> str:
    """docs/07 SPECIALIZED-PLAN-CANDIDATE-POLICY: the DEC-073 candidate policy
    (owner, admission basis, change class, allowed origins, independent route,
    conjunctive promotion requirements), parsed from spec/sprouting/inventory.rs
    with the origin fallback resolved through spec/sprouting/types.rs."""
    src = (root / "spec/sprouting/inventory.rs").read_text(encoding="utf-8")
    m = re.search(
        r"pub const SPECIALIZED_PLAN_CANDIDATE_POLICY: SpecializedPlanCandidatePolicy =\s*"
        r"SpecializedPlanCandidatePolicy \{(.*?)\n\s*\};", src, re.S)
    if not m:
        raise Unadmitted("spec/sprouting/inventory.rs: SPECIALIZED_PLAN_CANDIDATE_POLICY unreadable")
    body = m.group(1)
    owner = re.search(r'semantic_owner: ContractId\("([^"]+)"\)', body)
    basis = re.search(r'admission_basis: GuaranteeRef::dec\("([^"]+)"\)', body)
    change = re.search(r"change_class: CandidateChangeClass::(\w+)", body)
    route = re.search(r"independent_route: IndependentEvidenceRouteKind::(\w+)", body)
    if not (owner and basis and change and route):
        raise Unadmitted("spec/sprouting/inventory.rs: specialized-plan policy facts unreadable")
    origins = re.findall(r"CandidateOriginKind::(\w+)", body)
    if not origins:
        types_src = (root / "spec/sprouting/types.rs").read_text(encoding="utf-8")
        origins = _sprouting_const_variants(types_src, "CANDIDATE_ORIGIN_KINDS")
    reqs = re.findall(r"PromotionRequirement::(\w+)", body)
    if not reqs or "ALL" in reqs:
        reqs = [s for s, _o, _b in parse_promotion(root)[0]]
    lines = [
        f"{'semantic owner':<22} {owner.group(1)}",
        f"{'admission basis':<22} {basis.group(1)}",
        f"{'change class':<22} {change.group(1)}",
        f"{'allowed origins':<22} {'; '.join(origins)}",
        f"{'independent route':<22} {route.group(1)}",
        f"{'promotion requirements':<22} {'; '.join(reqs)}",
    ]
    return "```text\n" + "\n".join(lines) + "\n```"

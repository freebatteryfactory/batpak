"""Repository, toolchain, and vocabulary-catalog projections (5.5E3 and 5.5E4b).

Serializes the typed owners for the package/edge/qualification inventory, the
bundle inventory, the Tier 0 receipt denominator, the tracked toolchain, the
release seal, and the identity/mutation/command/compiler/contract/corpus/stale
vocabulary catalogs. Standard library only; shares no parsing with
bootstrap/audit.py.
"""
from __future__ import annotations

import re

from .guarantees import Unadmitted, gate_tokens, guarantee_seed, _DEC_ROW, _LEG_ROW
from .operators import parse_operators
from .registry import _spec_module_source, parse_generated_views


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


RELEASE_DOC = "docs/36_PUBLIC_API_CI_AND_RELEASE.md"


def release_seal_fields(root: Path) -> list[str]:
    src = _spec_module_source(root, "architecture")
    start = src.index("pub const RELEASE_SEAL_FIELDS")
    names = re.findall(r"ReleaseSealField::(\w+)",
                       src[start: src.index("\n];", start)])
    if not names:
        raise Unadmitted("RELEASE_SEAL_FIELDS declares no fields")
    return names


def render_release_seal(root: Path) -> str:
    lines = ["```text"] + release_seal_fields(root) + ["```"]
    return "\n".join(lines)


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
    src = _spec_module_source(root, "architecture")
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
    src = _spec_module_source(root, "architecture")
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
    src = _spec_module_source(root, "architecture")
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
    arch = _spec_module_source(root, "architecture")
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
    """The required Tier 0 receipt denominator, parsed from the typed owner
    spec/bootstrap_qualification.rs (5.5E6a). This block declares WHAT is
    required and each receipt's artifact policy; run receipts carry whether a
    particular run passed. The denominator is no longer a Python authority."""
    src_path = root / "spec/bootstrap_qualification.rs"
    if not src_path.is_file():
        raise Unadmitted("spec/bootstrap_qualification.rs is absent; the Tier 0 denominator refuses")
    src = _spec_module_source(root, "bootstrap_qualification")
    order_body = re.search(r"pub const ALL: &'static \[Tier0ReceiptKind\] = &\[(.*?)\];",
                           src, re.S)
    if not order_body:
        raise Unadmitted("spec/bootstrap_qualification.rs declares no Tier0ReceiptKind::ALL")
    order = re.findall(r"Tier0ReceiptKind::(\w+),", order_body.group(1))
    slugs = dict(re.findall(r'Tier0ReceiptKind::(\w+) => "([^"]+)"', src))
    policy = dict(re.findall(r"Tier0ReceiptKind::(\w+) => Tier0ArtifactPolicy::(\w+)", src))
    if not order or not slugs or not policy:
        raise Unadmitted("spec/bootstrap_qualification.rs Tier 0 policy parses to nothing")
    lines = ["| Receipt | Artifact policy |", "| --- | --- |"]
    for kind in order:
        lines.append(f"| {slugs[kind]} | {policy[kind]} |")
    return "\n".join(lines)

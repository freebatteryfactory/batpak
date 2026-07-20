"""Generated-view registry and exact repository-inventory ledgers.

Independently reconstructs the generated-view denominator (5.5E4a), the
package/edge/qualification/bundle/Tier-0 inventory mirrors (5.5E4b), the
version-boundary and exact-ledger projections (5.5E4c), and the proof-relation
and complete legacy-obligation ledger (5.5E4d), refusing every retired authored
mirror. Shares no results with bootstrap/project.py.
"""
from __future__ import annotations

import re
from pathlib import Path

from .batql import batql_parse_operator_model
from .corpus import (
    A_E4D_ACTIVE,
    A_E4D_AUTOMATIC,
    A_E4D_LEG_FULL,
    A_E4D_PLANNED_TEXT,
    A_GENERATED_BLOCK,
    G_DEC_ROW,
    _bootstrap_source,
    _spec_module_source,
    _uncomment,
    batql_extract_block,
    frontmatter,
)
from .guarantees import G_LEG_ROW, g_package_projection

# --- Generated-view registry (5.5E4a) ----------------------------------------
# spec/generated_views/registry.rs is the closed denominator of every
# repository-
# generated view. This auditor parses the typed registry INDEPENDENTLY of
# bootstrap/project.py (never importing it or executing its parser), scans
# every tracked Markdown document for generated markers, and proves true set
# equality between registered embedded target instances and actual ones —
# plus provenance, pairing, dispatcher parity, and the projector's own
# documentary honesty. After 5.5E4a an unregistered generated block cannot
# hide in the wallpaper.
A_GENERATED_VIEWS_SRC = "spec/generated_views/registry.rs"
A_VIEW_ARM = re.compile(
    r"GeneratedView::(\w+) => GeneratedViewSpec \{"
    r"(.*?)generator: BootstrapToolId::(\w+),", re.S)
A_VIEW_SURFACES = {"EmbeddedBlock", "StandaloneFile", "CorpusFrontmatter"}
A_MARKER_BEGIN = re.compile(r"<!-- ([A-Z0-9][A-Z0-9-]*):BEGIN([^>]*)-->")
A_MARKER_END = re.compile(r"<!-- ([A-Z0-9][A-Z0-9-]*):END -->")
# Multiple authority sources are joined in registry order by "; " — source
# order is provenance, and reordering without changing the set is drift.
A_MARKER_PROVENANCE = re.compile(r"\Agenerated from (.+?) by (\S+); do not edit\Z")
# Closed authored-fence allowlist: marker-shaped fences that are AUTHORED
# document structure, not generated views. HISTORICAL-MIGRATION is the
# docs/24 quarantine fence this auditor itself consumes for retired-ID scope.
A_AUTHORED_FENCES = {"HISTORICAL-MIGRATION"}


def a_parse_generated_views(root: Path) -> dict:
    """The auditor's own registry parse. Shares no code with the generator."""
    path = root / A_GENERATED_VIEWS_SRC
    if not path.is_file():
        return {"variants": [], "order": [], "views": {}}
    src = path.read_text(encoding="utf-8")
    enum_body = re.search(r"pub enum GeneratedView \{(.*?)\n\}", src, re.S)
    variants = re.findall(r"\n    (\w+),", _uncomment(enum_body.group(1))) if enum_body else []
    all_body = re.search(
        r"pub const ALL: &'static \[GeneratedView\] = &\[(.*?)\];", src, re.S)
    order = re.findall(r"GeneratedView::(\w+)", all_body.group(1)) if all_body else []
    views: dict[str, dict] = {}
    for name, body, generator in A_VIEW_ARM.findall(src):
        sources_m = re.search(r"authority_sources: &\[([^\]]*)\]", body)
        static_m = re.search(r"target: GeneratedViewTarget::Static\(&\[([^\]]*)\]\)", body)
        corpus = "GeneratedViewTarget::EligibleMarkdownCorpus" in body
        surface_m = re.search(r"surface: GeneratedViewSurface::(\w+)", body)
        marker_m = re.search(r'marker: Some\("([^"]+)"\)', body)
        views[name] = {
            "sources": re.findall(r'"([^"]+)"', sources_m.group(1)) if sources_m else [],
            "targets": re.findall(r'"([^"]+)"', static_m.group(1)) if static_m else [],
            "corpus": corpus,
            "surface": surface_m.group(1) if surface_m else "",
            "marker": marker_m.group(1) if marker_m else None,
            "generator": generator,
        }
    return {"variants": variants, "order": order, "views": views}


def _a_tracked_markdown(root: Path) -> list[Path]:
    out = []
    for path in sorted(root.rglob("*.md")):
        parts = path.relative_to(root).parts
        if any(part in (".git", "target", "__pycache__") for part in parts):
            continue
        out.append(path)
    return out


def generated_view_findings(root: Path) -> list[str]:
    out: list[str] = []
    reg = a_parse_generated_views(root)
    variants, order, views = reg["variants"], reg["order"], reg["views"]
    if not order:
        out.append("spec/generated_views/registry.rs declares no GeneratedView::ALL inventory")
    for variant in variants:
        if variant not in order:
            out.append(f"spec/generated_views/registry.rs: GeneratedView::{variant} is declared "
                       "but missing from GeneratedView::ALL")
    for variant in order:
        if variant not in variants:
            out.append(f"spec/generated_views/registry.rs: GeneratedView::ALL names {variant}, "
                       "which the enum does not declare")
    for duplicate in sorted({v for v in order if order.count(v) > 1}):
        out.append(f"spec/generated_views/registry.rs: GeneratedView::ALL repeats {duplicate}")
    for variant in order:
        if variant not in views:
            out.append(f"spec/generated_views/registry.rs: GeneratedView::{variant} has no "
                       "parseable spec() arm")
    if "GeneratedViewRegistry" not in order:
        out.append("GeneratedView::ALL omits GeneratedViewRegistry; the registry "
                   "must include itself")
    membership = views.get("CorpusEpochMembership")
    if "CorpusEpochMembership" not in order or membership is None \
            or membership["surface"] != "CorpusFrontmatter" or not membership["corpus"]:
        out.append("CorpusEpochMembership may not leave the registry: the corpus "
                   "epoch frontmatter projection must stay registered and complete")
    claimed: dict[tuple[str, str], str] = {}
    for name in order:
        view = views.get(name)
        if view is None:
            continue
        if not view["sources"]:
            out.append(f"generated view {name} names no authority source")
        for source in view["sources"]:
            if not (root / source).is_file():
                out.append(f"generated view {name} names authority source {source}, "
                           "which does not exist")
        for target in view["targets"]:
            if not (root / target).is_file():
                out.append(f"generated view {name} targets {target}, which does not exist")
        surface = view["surface"]
        if surface not in A_VIEW_SURFACES:
            out.append(f"generated view {name} carries unknown surface {surface!r}")
        if surface == "EmbeddedBlock":
            if not view["marker"]:
                out.append(f"embedded generated view {name} carries no marker")
            if not view["targets"] or view["corpus"]:
                out.append(f"embedded generated view {name} must name static targets")
            for target in view["targets"]:
                key = (target, view["marker"] or "")
                if key in claimed:
                    out.append(f"generated views {claimed[key]} and {name} both claim "
                               f"marker {view['marker']} in {target}; one target plus "
                               "one marker names one view")
                claimed[key] = name
        elif surface == "StandaloneFile":
            if view["marker"] is not None:
                out.append(f"standalone generated view {name} carries an embedded marker")
            if len(view["targets"]) != 1 or view["corpus"]:
                out.append(f"standalone generated view {name} must name exactly one "
                           "static target")
        elif surface == "CorpusFrontmatter":
            if view["marker"] is not None:
                out.append(f"corpus-frontmatter generated view {name} carries an "
                           "embedded marker")
            if view["targets"] or not view["corpus"]:
                out.append(f"corpus-frontmatter generated view {name} must target the "
                           "eligible markdown corpus, never static paths")
    # The universal marker census: registered embedded instances == actual.
    registered = {(target, views[name]["marker"]) for name in order
                  if name in views and views[name]["surface"] == "EmbeddedBlock"
                  and views[name]["marker"] for target in views[name]["targets"]}
    marker_source = {marker: "; ".join(views[name]["sources"]) for name in order
                     if name in views and views[name]["marker"] and views[name]["sources"]
                     for marker in [views[name]["marker"]]}
    actual: set[tuple[str, str]] = set()
    for path in _a_tracked_markdown(root):
        rel = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        begins = A_MARKER_BEGIN.findall(text)
        ends = A_MARKER_END.findall(text)
        begin_names = [name for name, _tail in begins]
        for name in sorted(set(begin_names)):
            if begin_names.count(name) > ends.count(name):
                out.append(f"{rel}: generated marker {name} has a BEGIN with no "
                           "matching END")
        for name in sorted(set(ends)):
            if ends.count(name) > begin_names.count(name):
                out.append(f"{rel}: generated marker {name} has an END with no "
                           "matching BEGIN")
        for name, tail in begins:
            if name in marker_source:
                if (rel, name) in actual:
                    out.append(f"{rel}: generated marker {name} appears more than "
                               "once; one target carries one instance of one marker")
                actual.add((rel, name))
                prov = A_MARKER_PROVENANCE.match(tail.strip())
                if not prov:
                    out.append(f"{rel}: {name} BEGIN marker does not carry the "
                               "canonical machine-readable provenance")
                else:
                    if prov.group(1) != marker_source[name]:
                        out.append(f"{rel}: {name} BEGIN marker names source "
                                   f"{prov.group(1)}, not its registered authority "
                                   f"source {marker_source[name]}")
                    if prov.group(2) != "bootstrap/project.py":
                        out.append(f"{rel}: {name} BEGIN marker names generator "
                                   f"{prov.group(2)}, not bootstrap/project.py")
            elif "generated" in tail:
                out.append(f"{rel}: generated marker {name} is not a registered "
                           "generated view; a future view enters the registry "
                           "before it may land")
            elif name not in A_AUTHORED_FENCES:
                out.append(f"{rel}: marker {name} is neither a registered generated "
                           "view nor a declared authored fence")
    for target, marker in sorted(registered - actual):
        out.append(f"{target}: registered generated marker {marker} is absent "
                   "from its target")
    for rel, name in sorted(actual - registered):
        out.append(f"{rel}: generated marker {name} appears in a target the "
                   "registry does not claim")
    # Standalone provenance equals the registry row.
    graph = views.get("GuaranteeGraph")
    if graph:
        graph_path = root / graph["targets"][0] if graph["targets"] else None
        if graph_path and graph_path.is_file():
            front = frontmatter(graph_path.read_text(encoding="utf-8"))
            want_from = "; ".join(graph["sources"])
            if front.get("generated_from") != want_from:
                out.append(f"{graph['targets'][0]}: generated_from is "
                           f"{front.get('generated_from')!r}, not its registry row "
                           f"{want_from!r}")
            if front.get("generated_by") != "bootstrap/project.py":
                out.append(f"{graph['targets'][0]}: generated_by is "
                           f"{front.get('generated_by')!r}, not bootstrap/project.py")
    toolchain_view = views.get("RustToolchain")
    if toolchain_view and toolchain_view["targets"]:
        tc_path = root / toolchain_view["targets"][0]
        want_line = (f"# generated from {toolchain_view['sources'][0]} by "
                     "bootstrap/project.py; do not edit")
        if tc_path.is_file():
            first = tc_path.read_text(encoding="utf-8").splitlines()
            if not first or first[0] != want_line:
                out.append(f"{toolchain_view['targets'][0]}: missing or drifted "
                           "provenance comment; the comment is part of the exact "
                           "projected bytes")
    # The registry's own projection (docs/28) equals the auditor's independent
    # reconstruction, its own row included.
    reg_row = views.get("GeneratedViewRegistry")
    if reg_row and reg_row["targets"] and reg_row["marker"]:
        reg_target = root / reg_row["targets"][0]
        if reg_target.is_file():
            body = batql_extract_block(reg_target.read_text(encoding="utf-8"),
                                       reg_row["marker"])
            surface_display = {"EmbeddedBlock": "embedded-block",
                               "StandaloneFile": "standalone-file",
                               "CorpusFrontmatter": "corpus-frontmatter"}
            generator_display = {"ProjectPy": "bootstrap/project.py"}
            rows = ["| View | Surface | Authority source(s) | Target | Marker | Generator |",
                    "| --- | --- | --- | --- | --- | --- |"]
            for name in order:
                view = views.get(name)
                if view is None:
                    continue
                target_cell = "; ".join(view["targets"]) if view["targets"] \
                    else "eligible markdown corpus"
                rows.append(
                    f"| {name} | {surface_display.get(view['surface'], '?')} | "
                    f"{'; '.join(view['sources'])} | {target_cell} | "
                    f"{view['marker'] or '-'} | "
                    f"{generator_display.get(view['generator'], '?')} |")
            if body is not None and body != "\n".join(rows):
                out.append(f"{reg_row['targets'][0]}: generated block "
                           f"{reg_row['marker']} does not match the "
                           "spec/generated_views/registry.rs projection")
    # Dispatcher parity: the projector's VIEW_RENDERERS key set equals the
    # registry, and no manual literal plan bypasses it. Parsed from source —
    # the auditor never imports or executes project.py. An absent projector
    # is a refusal, never a crash.
    proj_src = _bootstrap_source(root, "project")
    if not proj_src:
        out.append("bootstrap/project.py is absent; the registered generated "
                   "views have no projector")
        return out
    dispatch_body = re.search(r"\nVIEW_RENDERERS = \{(.*?)\n\}", proj_src, re.S)
    dispatch_keys = re.findall(r'\n    "(\w+)":', dispatch_body.group(1)) \
        if dispatch_body else []
    if not dispatch_keys:
        out.append("bootstrap/project.py declares no VIEW_RENDERERS dispatch map")
    for missing in [n for n in order if n not in dispatch_keys]:
        out.append(f"registered generated view {missing} has no projector "
                   "dispatcher entry")
    for phantom in sorted(set(dispatch_keys) - set(order)):
        out.append(f"projector dispatcher entry {phantom} serializes no "
                   "registered view")
    for literal in re.findall(r'block_pattern\(\s*"([^"]+)"', proj_src):
        out.append(f"bootstrap/project.py builds a manual projection plan for "
                   f"{literal!r}; no literal plan may bypass the registry")
    # The projector's own documentary honesty.
    if re.search(r"do(?:es)? not generate the Guarantee Graph", proj_src):
        out.append("bootstrap/project.py documentation denies generating the "
                   "Guarantee Graph, which its own plan builder generates")
    if "operator blocks current" in proj_src or "WROTE operator blocks" in proj_src:
        out.append("bootstrap/project.py status reports only operator blocks; "
                   "the banner must report the registered-view denominator")
    return out


# --- Repository inventory mirrors (5.5E4b) -----------------------------------
# The typed owners already contain the truth. This auditor reconstructs the
# package, edge, qualification-profile, bundle, and Tier 0 receipt projections
# INDEPENDENTLY (own regexes, own traversal, no shared results with the
# projector) and refuses every retired authored mirror. Three species stay
# distinct: semantic facts (typed spec), derived repository inventory
# (generated from denominators plus current tracked-tree membership), and run
# evidence (exact-SHA, exact-target receipts). A current count is not
# semantic authority; a denominator is not a claim a run passed.
A_PKG_ROW_FULL = re.compile(
    r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "([^"]*)",\s*'
    r'class: PackageClass::(\w+),\s*layer: (\d+),\s*\}', re.S)
A_EDGE_ROW = re.compile(
    r'EdgeSpec \{ importer: PackageId::(\w+), importee: PackageId::(\w+), '
    r'class: EdgeClass::(\w+), profile: "([^"]*)" \}')
A_QUAL_FULL = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "([^"]+)",\s*\}', re.S)
A_TIER0_ROWS = re.compile(r'\("([^"]+)", (True|False)\)')
# Authored doctrine fragments the converged documents must keep authoring.
A_E4B_DOCTRINE = (
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "feature-coverage-distinction",
     "They are not additional QualificationProfile identities unless they enter"),
    ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "denominator-not-a-run-claim",
     "This block declares the denominator. It does not claim that the latest run passed."),
    ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "receipts-carry-outcomes",
     "exact-SHA, exact-target run receipts, not timeless prose"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "authored-metadata-epoch",
     "ReconciliationEpoch corpus membership"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "generated-metadata-sources",
     "generator, exact authority sources, do-not-edit posture"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "generated-needs-no-contract-id",
     "does not invent a contract ID or supersession claim"),
    ("docs/29_STATUS_AND_SUPERSESSION.md", "authored-cannot-claim-generated",
     "cannot claim generated status to evade the authored fields"),
)
# Retired authored mirrors, refused against the stripped authored body.
A_E4B_RETIRED = (
    ("README.md", "authored package table",
     re.compile(r"\|\s*`?[a-z-]+`?\s*\|\s*(?:production|dev-only|binary adapter|proc-macro) ")),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "authored layer inventory",
     re.compile(r"^L0\s+macbat-compiler", re.M)),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "ASCII dependency inventory",
     re.compile(r"[↑↑]")),
    ("docs/03_REPOSITORY_AND_PACKAGES.md", "five-profile target matrix",
     re.compile(r"^batpak semantic\s+no_std", re.M)),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "authored bundle counts",
     re.compile(r"^\d+ (?:numbered|declared|retained|one-for-one|concrete|bootstrap|architectural|classified) ", re.M)),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "volatile word count",
     re.compile(r"\d[\d,]*\+? words")),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "107-row legacy claim",
     re.compile(r"\b107-row\b")),
    ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "static validation PASS table",
     re.compile(r"Validation completed in this delivery environment")),
    ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "historical no-rustc claim",
     re.compile(r"did not contain `?rustc`?|structurally inspected but not compiled")),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "authored package topology",
     re.compile(r"Frozen package direction")),
    ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "current run id embedded as timeless law",
     re.compile(r"\b\d{10,}\b")),
)


def _a_gate_token_render(expr: str, root: Path) -> str:
    src = (root / "spec/gates/inventory.rs").read_text(encoding="utf-8")
    rows = re.findall(r'GateSpec \{ id: GateId::(\w+), token: "([^"]*)"', src)
    order = [r[0] for r in rows]
    token_of = dict(rows)
    names = [c.strip().split("::")[-1] for c in expr.split(",") if c.strip()]
    names.sort(key=lambda n: order.index(n) if n in order else 99)
    return "/".join(token_of.get(n, "?") for n in names)


def inventory_mirror_findings(root: Path) -> list[str]:
    out: list[str] = []
    arch = _spec_module_source(root, "architecture")
    # Documentary class spellings: variants == ALL, spellings unique/nonempty.
    for type_name in ("PackageClass", "EdgeClass"):
        enum_body = re.search(r"pub enum " + type_name + r" \{(.*?)\n\}", arch, re.S)
        variants = re.findall(r"\n    (\w+),", _uncomment(enum_body.group(1))) \
            if enum_body else []
        all_body = re.search(
            r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", arch, re.S)
        inventory = re.findall(r"\b" + type_name + r"::(\w+)", all_body.group(1)) \
            if all_body else []
        for missing in [v for v in variants if v not in inventory]:
            out.append(f"spec/architecture/types.rs: {type_name}::{missing} is declared "
                       f"but missing from {type_name}::ALL")
        for phantom in [v for v in inventory if v not in variants]:
            out.append(f"spec/architecture/types.rs: {type_name}::ALL names {phantom}, "
                       "which the enum does not declare")
        impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", arch, re.S)
        spellings = re.findall(r"\b" + type_name + r'::(\w+) => "([^"]*)",',
                               impl.group(1)) if impl else []
        seen: dict[str, str] = {}
        for variant, spelling in spellings:
            if not spelling:
                out.append(f"spec/architecture/types.rs: {type_name}::{variant} declares "
                           "an empty documentary spelling")
            elif spelling in seen:
                out.append(f"spec/architecture/types.rs: duplicate {type_name} spelling "
                           f"{spelling!r} ({seen[spelling]} and {variant})")
            seen[spelling] = variant
        for variant in [v for v in inventory if v not in dict(spellings)]:
            out.append(f"spec/architecture/types.rs: {type_name}::{variant} declares no "
                       "documentary spelling arm")
    # Independent projections of the typed rows.
    pkg_all = re.findall(r"\bPackageId::(\w+),", re.search(
        r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];", arch, re.S).group(1))
    cargo = g_package_projection(root, "cargo_name")
    wpath = g_package_projection(root, "workspace_path")
    pkg_class = dict(re.findall(
        r'PackageClass::(\w+) => "([^"]*)",',
        re.search(r"impl PackageClass \{(.*?)\n\}", arch, re.S).group(1)))
    edge_class = dict(re.findall(
        r'EdgeClass::(\w+) => "([^"]*)",',
        re.search(r"impl EdgeClass \{(.*?)\n\}", arch, re.S).group(1)))
    environments = dict(re.findall(r'QualificationEnvironment::(\w+) => "([^"]+)"', arch))
    pkg_rows = A_PKG_ROW_FULL.findall(arch)
    if [r[0] for r in pkg_rows] != pkg_all:
        out.append("spec/architecture/inventory.rs: PACKAGES does not equal PackageId::ALL "
                   "exactly and in order")
    want_pkg = [f"| {cargo.get(pid, '?')} | {pkg_class.get(cls, '?')} | {layer} | "
                f"{wpath.get(pid, '?')} | {role} |"
                for pid, role, cls, layer in pkg_rows]
    want_edges = [f"| {cargo.get(a, '?')} | {cargo.get(b, '?')} | "
                  f"{edge_class.get(cls, '?')} | {profile} |"
                  for a, b, cls, profile in A_EDGE_ROW.findall(arch)]
    want_quals = [f"| {cargo.get(pkg, '?')} | {profile} | "
                  f"{environments.get(env, '?')} | {_a_gate_token_render(gates, root)} | "
                  f"{req} |"
                  for pkg, profile, env, gates, req in A_QUAL_FULL.findall(arch)]
    # Tier 0 receipt denominator, from the typed owner it now lives in (5.5E6a).
    bq_path = root / "spec/bootstrap_qualification/types.rs"
    bq = bq_path.read_text(encoding="utf-8") if bq_path.is_file() else ""
    bq_order_body = re.search(
        r"pub const ALL: &'static \[Tier0ReceiptKind\] = &\[(.*?)\];", bq, re.S)
    bq_order = re.findall(r"Tier0ReceiptKind::(\w+),", bq_order_body.group(1)) \
        if bq_order_body else []
    bq_slugs = dict(re.findall(r'Tier0ReceiptKind::(\w+) => "([^"]+)"', bq))
    bq_policy = dict(re.findall(
        r"Tier0ReceiptKind::(\w+) => Tier0ArtifactPolicy::(\w+)", bq))
    want_tier0 = [f"| {bq_slugs[k]} | {bq_policy[k]} |" for k in bq_order
                  if k in bq_slugs and k in bq_policy]
    if bq and not want_tier0:
        out.append("spec/bootstrap_qualification/types.rs declares no parseable "
                   "Tier 0 receipt denominator")
    # Bundle inventory: every count independently derived.
    md_paths = _a_tracked_markdown(root)
    reg = a_parse_generated_views(root)
    seeds = len(re.findall(r'InvariantSpec \{ id: "', (root / "spec/invariants/inventory.rs")
                           .read_text(encoding="utf-8")))
    decisions = len(G_DEC_ROW.findall((root / "spec/dispositions/inventory.rs")
                                      .read_text(encoding="utf-8")))
    obligations = len(G_LEG_ROW.findall((root / "spec/legacy_obligations/inventory.rs")
                                        .read_text(encoding="utf-8")))
    coverage_src = (root / "spec/legacy_invariant_coverage/inventory.rs").read_text(encoding="utf-8")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", coverage_src,
        re.S).group(1))
    if len(re.findall(r"LegacyInvariantCoverage \{\s*legacy_id:", coverage_src)) \
            != len(manifest):
        out.append("spec/legacy_invariant_coverage/inventory.rs: COVERAGE rows do not equal "
                   "SOURCE_INVARIANT_IDS")
    op_model = batql_parse_operator_model(root)
    if len(op_model["rows"]) != len(op_model["id_all"]):
        out.append("spec/operators/inventory.rs: OPERATORS rows do not equal OperatorId::ALL")
    static_instances = sum(len(reg["views"][n]["targets"]) for n in reg["order"]
                           if n in reg["views"])
    bindings = sum(1 for path in md_paths
                   if re.search(r"^(?:source_)?reconciliation_epoch:",
                                path.read_text(encoding="utf-8"), re.M))
    derived = [
        ("numbered architecture documents",
         len(list((root / "docs").glob("[0-9][0-9]_*.md"))),
         "current tracked docs matching docs/[0-9][0-9]_*.md"),
        ("Markdown documents", len(md_paths), "current eligible Markdown corpus"),
        ("Cargo packages", len(pkg_all), "PackageId::ALL with PACKAGES parity"),
        ("package edges", len(A_EDGE_ROW.findall(arch)), "EDGES"),
        ("qualification profiles", len(A_QUAL_FULL.findall(arch)),
         "QUALIFICATION_PROFILES"),
        ("SEED guarantees", seeds, "spec/invariants/inventory.rs SEED inventory"),
        ("decision rows", decisions, "spec/dispositions/inventory.rs DECISIONS"),
        ("legacy semantic obligations", obligations,
         "spec/legacy_obligations/inventory.rs OBLIGATIONS"),
        ("legacy invariant declarations", len(manifest),
         "SOURCE_INVARIANT_IDS with COVERAGE parity"),
        ("BatQL operators", len(op_model["id_all"]),
         "OperatorId::ALL with OPERATORS parity"),
        ("registered generated views", len(reg["order"]), "GeneratedView::ALL"),
        ("static generated target instances", static_instances,
         "expansion of every Static registry target"),
        ("corpus-frontmatter bindings", bindings,
         "the eligible Markdown corpus reached by CorpusEpochMembership"),
    ]
    want_bundle = [f"| {metric} | {count} | {derivation} |"
                   for metric, count, derivation in derived]
    # Positional block parity for all five projections.
    for rel, marker, want in (
        ("README.md", "PACKAGE-INVENTORY", want_pkg),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "PACKAGE-INVENTORY", want_pkg),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "PACKAGE-EDGES", want_edges),
        ("docs/03_REPOSITORY_AND_PACKAGES.md", "QUALIFICATION-PROFILES", want_quals),
        ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "BUNDLE-INVENTORY", want_bundle),
        ("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md", "TIER0-RECEIPT-DENOMINATOR", want_tier0),
    ):
        path = root / rel
        body = batql_extract_block(path.read_text(encoding="utf-8"), marker) \
            if path.is_file() else None
        if body is None:
            out.append(f"{rel}: missing generated block {marker}")
            continue
        got = [line for line in body.splitlines()[2:] if line.strip()]
        if got != want:
            out.append(f"{rel}: generated block {marker} does not match its "
                       "typed derivation")
    # Retired mirrors stay retired, and the converged doctrine stays authored —
    # both judged with every generated block stripped.
    stripped: dict[str, str] = {}
    for rel in ("README.md", "docs/03_REPOSITORY_AND_PACKAGES.md",
                "docs/28_SELF_EXPLAINING_REPOSITORY.md",
                "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
                "docs/29_STATUS_AND_SUPERSESSION.md"):
        path = root / rel
        stripped[rel] = A_GENERATED_BLOCK.sub("", path.read_text(encoding="utf-8")) \
            if path.is_file() else ""
    for rel, label, pattern in A_E4B_RETIRED:
        if pattern.search(stripped[rel]):
            out.append(f"{rel}: retired authored mirror may not return ({label})")
    for rel, name, fragment in A_E4B_DOCTRINE:
        if fragment not in stripped[rel]:
            out.append(f"{rel}: converged inventory doctrine absent ({name})")
    return out


# --- Version boundaries and exact-ledger mirrors (5.5E4c) --------------------
# Part one: the accepted version-identity vocabulary converges with DEC-064
# and every documentary consumer — the inventory derives from
# VersionIdentityKind::ALL, never a hardcoded count. Part two/three: docs/30
# and docs/34 are exact mirrors whose complete rows live in typed owners; the
# auditor reconstructs both generated bodies independently and refuses the
# authored mirrors' return. Part four: the docs/28 convergence doctrine.
A_DEC_FULL_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+), '
    r'subject: "((?:[^"\\]|\\.)*)", successor: "((?:[^"\\]|\\.)*)"')
A_COV_FULL_ROW = re.compile(
    r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", '
    r'disposition: CoverageDisposition::(\w+), successor: "((?:[^"\\]|\\.)*)", '
    r'rationale: "((?:[^"\\]|\\.)*)" \}')
A_VERSION_ALIAS_BANS = ("EventFrameVersion", "FbatFrameVersion")
A_VERSION_DOCTRINE = (
    ("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "frame-version-reservation",
     "FrameVersion is reserved for the EventFrame envelope"),
    ("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "shared-representation-fence",
     "Sharing an integer representation or a migration does not permit"),
    ("docs/05_STORAGE_FBAT_AND_TILES.md", "three-way-envelope-split",
     "FrameVersion versions the EventFrame envelope. FbatFormatVersion versions "
     "the surrounding .fbat container. BatTaggedRecordVersion versions a tagged "
     "payload record inside EncodedPayload"),
    ("docs/05_STORAGE_FBAT_AND_TILES.md", "coordinated-migration",
     "preserving three independent version identities"),
    ("docs/15_SCHEMA_CODEC_AND_MIGRATION.md", "payload-record-boundary",
     "BatTaggedRecordVersion versions the tagged payload-record envelope and "
     "codec. It does not version EventFrame or the .fbat container"),
    ("docs/02_SYSTEM_MODEL.md", "layout-schema-split",
     "LayoutVersion versions physical organization under LayoutId. SchemaVersion "
     "versions schema meaning. Neither substitutes for the other"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "frame-version-constants",
     "FrameVersion values and EventFrame envelope field IDs/order"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "container-constants",
     "FbatFormatVersion values and .fbat container magic/header"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "payload-record-constants",
     "BatTaggedRecordVersion values, field tags, and length encoding"),
    ("docs/32_IMPLEMENTATION_CONSTANTS.md", "layout-constants",
     "LayoutVersion values for selected physical layouts"),
    ("docs/24_GAUNTLET.md", "version-witness-consumes-all",
     "Every member of spec/identities/types.rs VersionIdentityKind::ALL denotes a "
     "distinct version type"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "exact-mirror-clause",
     "An equivalence checker babysitting two editable copies is not convergence"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "no-python-prosthetics-clause",
     "from conventions, neighbouring rows, or Python defaults"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "subset-relation-clause",
     "Parsing prose headings is not an authority relation"),
    ("docs/28_SELF_EXPLAINING_REPOSITORY.md", "historical-narrative-clause",
     "cannot be read as the current inventory"),
    ("docs/34_LEGACY_INVARIANT_COVERAGE.md", "historical-framing",
     "retained as secondary historical evidence, not a competing denominator"),
)


def _a_str_arms(source: str, type_name: str, fn_name: str) -> dict[str, str]:
    impl = re.search(r"impl " + type_name + r" \{(.*?)\n\}", source, re.S)
    fn = re.search(
        r"pub const fn " + fn_name + r"\(self\) -> &'static str \{\s*match self \{(.*?)\n        \}",
        impl.group(1), re.S) if impl else None
    return dict(re.findall(
        r"\b" + type_name + r'::(\w+) => \{?\s*"((?:[^"\\]|\\.)*)"', fn.group(1))) \
        if fn else {}


def _a_all_of(source: str, type_name: str) -> list[str]:
    body = re.search(
        r"pub const ALL: &'static \[" + type_name + r"\] = &\[(.*?)\];", source, re.S)
    return re.findall(r"\b" + type_name + r"::(\w+)", body.group(1)) if body else []


def _a_enum_variants_of(source: str, type_name: str) -> list[str]:
    body = re.search(r"pub enum " + type_name + r" \{(.*?)\n\}", source, re.S)
    return re.findall(r"\n    (\w+),", _uncomment(body.group(1))) if body else []


def _a_spelling_laws(out, source, src_label, type_name, spelling_fn,
                     meaning_fn=None):
    variants = _a_enum_variants_of(source, type_name)
    inventory = _a_all_of(source, type_name)
    for missing in [v for v in variants if v not in inventory]:
        out.append(f"{src_label}: {type_name}::{missing} is declared but missing "
                   f"from {type_name}::ALL")
    for phantom in [v for v in inventory if v not in variants]:
        out.append(f"{src_label}: {type_name}::ALL names {phantom}, which the "
                   "enum does not declare")
    for fn_name, kind in ((spelling_fn, "spelling"),) + (
            ((meaning_fn, "meaning"),) if meaning_fn else ()):
        arms = _a_str_arms(source, type_name, fn_name)
        seen: dict[str, str] = {}
        for variant in inventory:
            value = arms.get(variant)
            if not value:
                out.append(f"{src_label}: {type_name}::{variant} declares no or an "
                           f"empty {kind}")
                continue
            if kind == "spelling" and value in seen:
                out.append(f"{src_label}: duplicate {type_name} spelling {value!r} "
                           f"({seen[value]} and {variant})")
            seen[value] = variant
    return inventory


def exact_ledger_findings(root: Path) -> list[str]:
    out: list[str] = []
    # Version-identity convergence.
    ident_src = (root / "spec/identities/types.rs").read_text(encoding="utf-8")
    version_all = _a_all_of(ident_src, "VersionIdentityKind")
    version_pairs = re.findall(
        r'\bVersionIdentityKind::(\w+) => \{?\s*entry!\(\s*"([^"]+)",\s*"([^"]+)"\s*\)',
        ident_src)
    version_arms = {v: s for v, s, _o in version_pairs}
    version_owner = {v: o for v, _s, o in version_pairs}
    spellings = [version_arms[v] for v in version_all if v in version_arms]
    dsrc = (root / "spec/dispositions/inventory.rs").read_text(encoding="utf-8")
    dtypes = (root / "spec/dispositions/types.rs").read_text(encoding="utf-8")
    row = re.search(
        r'DecisionSpec \{ id: "DEC-064",.*?subject: "([^"]*)", successor: "([^"]*)",'
        r'.*?replacement_contract: (?:None|Some\("([^"]*)"\))', dsrc)
    fields = " ".join(g or "" for g in row.groups()) if row else ""
    if not row:
        out.append("DEC-064 is missing from the decision ledger")
    for spelling in spellings:
        if not re.search(r"\b" + re.escape(spelling) + r"\b", fields):
            out.append(f"DEC-064 forward-policy fields do not name {spelling}; "
                       "the version-separation law names its complete inventory")
    named = set(re.findall(r"\b([A-Z][A-Za-z]*Version)\b", fields)) - {"Version"}
    for phantom in sorted(named - set(spellings)):
        out.append(f"DEC-064 names {phantom}, which VersionIdentityKind::ALL "
                   "does not declare (phantom version identity)")
    frame_owner = version_owner.get("Frame")
    if frame_owner != "BP-STORAGE-TILES-1":
        out.append(f"FrameVersion is owned by {frame_owner!r}, not BP-STORAGE-TILES-1")
    doc05 = (root / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8")
    if "frame_version: FrameVersion" not in doc05:
        out.append("docs/05: EventFrameV2 does not carry frame_version: FrameVersion")
    for rel, name, fragment in A_VERSION_DOCTRINE:
        text = A_GENERATED_BLOCK.sub("", (root / rel).read_text(encoding="utf-8")) \
            if (root / rel).is_file() else ""
        if re.sub(r"\s+", " ", fragment) not in re.sub(r"\s+", " ", text):
            out.append(f"{rel}: version/convergence doctrine absent ({name})")
    doc24 = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    if re.search(r"ten declared version identities", doc24):
        out.append("docs/24: the version witness may not freeze a numeric count; "
                   "it consumes VersionIdentityKind::ALL")
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        stripped = A_GENERATED_BLOCK.sub("", rel.read_text(encoding="utf-8"))
        for alias in A_VERSION_ALIAS_BANS:
            if re.search(r"\b" + alias + r"\b", stripped):
                out.append(f"{rel.relative_to(root).as_posix()}: names {alias}, "
                           "an unadmitted version-identity alias")
    for spec_rel in sorted((root / "spec").rglob("*.rs")):
        if re.search(r"pub (?:struct|enum) Version\b",
                     _uncomment(spec_rel.read_text(encoding="utf-8"))):
            out.append(f"{spec_rel.relative_to(root).as_posix()}: declares a "
                       "generic Version type; no generic Version crosses a "
                       "subsystem boundary")
    # E4d protections: the mixed ledgers stay authored until their missing
    # fields have real owners.
    for rel, label in (("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
                        "docs/21 carries evidence and witness meaning "
                        "LegacyObligation does not yet own"),
                       ("docs/24_GAUNTLET.md",
                        "docs/24 is the canonical semantic owner of proof-row "
                        "meaning, not an exact mirror")):
        front = frontmatter((root / rel).read_text(encoding="utf-8"))
        if front.get("status") == "GENERATED":
            out.append(f"{rel}: may not be marked GENERATED before its missing "
                       f"fields are owned ({label})")
    # Decision-ledger and coverage reconstruction, byte-exact.
    disp_all = _a_spelling_laws(out, dtypes, "spec/dispositions/types.rs", "Disposition",
                                "spelling", "meaning")
    _a_spelling_laws(out, dtypes, "spec/dispositions/types.rs", "DecisionClass", "spelling")
    disp_tags = _a_str_arms(dtypes, "Disposition", "spelling")
    disp_meanings = _a_str_arms(dtypes, "Disposition", "meaning")
    class_spellings = _a_str_arms(dtypes, "DecisionClass", "spelling")
    ledger_lines = ["| Tag | Meaning |", "| --- | --- |"]
    ledger_lines += [f"| {disp_tags.get(v, '?')} | {disp_meanings.get(v, '?')} |"
                     for v in disp_all]
    ledger_lines += ["", "| ID | Tag | Class | Gates | Subject | Final ruling |",
                     "| --- | --- | --- | --- | --- | --- |"]
    for dec_id, cls, gates, disp, subject, successor in A_DEC_FULL_ROW.findall(dsrc):
        if disp not in disp_tags:
            out.append(f"{dec_id} carries disposition {disp}, which "
                       "Disposition::ALL does not admit")
        if cls not in class_spellings:
            out.append(f"{dec_id} carries class {cls}, which DecisionClass::ALL "
                       "does not admit")
        ledger_lines.append(
            f"| {dec_id} | {disp_tags.get(disp, '?')} | "
            f"{class_spellings.get(cls, '?')} | {_a_gate_token_render(gates, root)} | "
            f"{subject} | {successor} |")
    cov_src = (root / "spec/legacy_invariant_coverage/inventory.rs").read_text(encoding="utf-8")
    cov_types = (root / "spec/legacy_invariant_coverage/types.rs").read_text(encoding="utf-8")
    cov_all = _a_spelling_laws(out, cov_types, "spec/legacy_invariant_coverage/types.rs",
                               "CoverageDisposition", "spelling", "meaning")
    cov_tags = _a_str_arms(cov_types, "CoverageDisposition", "spelling")
    cov_meanings = _a_str_arms(cov_types, "CoverageDisposition", "meaning")
    manifest = re.findall(r'"(INV-[^"]+)"', re.search(
        r"pub const SOURCE_INVARIANT_IDS: &\[&str\] = &\[(.*?)\];", cov_src,
        re.S).group(1))
    cov_rows = A_COV_FULL_ROW.findall(cov_src)
    cov_lines = ["| Disposition | Meaning |", "| --- | --- |"]
    cov_lines += [f"| {cov_tags.get(v, '?')} | {cov_meanings.get(v, '?')} |"
                  for v in cov_all]
    cov_lines += ["", f"The source declaration denominator is the {len(manifest)}"
                  "-declaration `SOURCE_INVARIANT_IDS` manifest, in exact set "
                  "parity with the coverage rows below.",
                  "", "| Legacy invariant | Disposition | Clean successor | Rationale |",
                  "| --- | --- | --- | --- |"]
    for inv, disp, successor, rationale in cov_rows:
        if disp not in cov_tags:
            out.append(f"coverage row {inv} carries disposition {disp}, which "
                       "CoverageDisposition::ALL does not admit")
        cov_lines.append(f"| `{inv}` | {cov_tags.get(disp, '?')} | `{successor}` | "
                         f"{rationale} |")
    for rel, marker, want in (
        ("docs/30_DECISION_AND_REJECTION_LEDGER.md", "DECISION-LEDGER",
         "\n".join(ledger_lines)),
        ("docs/34_LEGACY_INVARIANT_COVERAGE.md", "LEGACY-INVARIANT-COVERAGE",
         "\n".join(cov_lines)),
    ):
        text = (root / rel).read_text(encoding="utf-8") if (root / rel).is_file() else ""
        body = batql_extract_block(text, marker)
        if body is None:
            out.append(f"{rel}: missing generated block {marker}")
        elif body != want:
            out.append(f"{rel}: generated block {marker} does not match its "
                       "typed derivation")
        stripped = A_GENERATED_BLOCK.sub("", text)
        if marker == "DECISION-LEDGER":
            if re.search(r"^\| DEC-\d+ \|", stripped, re.M):
                out.append(f"{rel}: an authored decision row may not return "
                           "outside the generated ledger")
            if re.search(r"^KEEP\s{2,}retained", stripped, re.M):
                out.append(f"{rel}: an authored disposition inventory may not "
                           "return outside the generated ledger")
        else:
            if re.search(r"^\| `INV-", stripped, re.M):
                out.append(f"{rel}: an authored coverage row may not return "
                           "outside the generated matrix")
            if re.search(r"^PRESERVE\s{2,}semantic law", stripped, re.M):
                out.append(f"{rel}: an authored coverage disposition inventory "
                           "may not return outside the generated matrix")
    return out


# --- Proof relations and the complete legacy ledger (5.5E4d) -----------------
# spec/proof/inventory.rs is the one membership authority; docs/24 owns
# meaning; the
# domain documents own the pressured law; docs/21 is the generated human
# ledger. This auditor reconstructs every projection independently, proves
# the contract/target selection laws, and sweeps the retired mirrors.
def proof_relation_findings(root: Path) -> list[str]:
    out: list[str] = []
    psrc = _spec_module_source(root, "proof")
    active = [{"id": m[0], "family": m[1], "guarantee": m[2],
               "contracts": re.findall(r'ContractId\("([^"]+)"\)', m[3])}
              for m in A_E4D_ACTIVE.findall(psrc)]
    contract_ids: dict[str, str] = {}
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        front = frontmatter(rel.read_text(encoding="utf-8"))
        if front.get("contract_id"):
            contract_ids[front["contract_id"]] = front.get("status", "")
    for row in active:
        if not row["contracts"]:
            out.append(f"active proof row {row['id']} names no projection contract")
        if len(row["contracts"]) != len(set(row["contracts"])):
            out.append(f"active proof row {row['id']} repeats a projection contract")
        for contract in row["contracts"]:
            if contract in A_E4D_AUTOMATIC:
                out.append(f"active proof row {row['id']} lists automatic consumer "
                           f"{contract}; docs/24 and docs/21 receive every relation "
                           "mechanically")
            elif contract not in contract_ids:
                out.append(f"active proof row {row['id']} names projection contract "
                           f"{contract}, which no authored document declares")
            elif contract_ids[contract] != "AUTHORITATIVE":
                out.append(f"active proof row {row['id']} names {contract}, whose "
                           "document is not an authoritative consumer")
    # PROOF-RELATIONS block parity (independent grouping).
    groups: dict[tuple, list[str]] = {}
    for row in active:
        groups.setdefault((row["guarantee"], tuple(row["contracts"])), []).append(row["id"])
    want_rel = ["| Guarantee | Projection contract(s) | Active proof rows |",
                "| --- | --- | --- |"]
    for (guarantee, contracts), ids in groups.items():
        want_rel.append(f"| {guarantee} | {'; '.join(contracts)} | {'; '.join(ids)} |")
    d24_text = (root / "docs/24_GAUNTLET.md").read_text(encoding="utf-8")
    body = batql_extract_block(d24_text, "PROOF-RELATIONS")
    if body is None:
        out.append("docs/24: missing generated block PROOF-RELATIONS")
    elif body != "\n".join(want_rel):
        out.append("docs/24: generated block PROOF-RELATIONS does not match its "
                   "typed derivation")
    # Domain projections: registered PROOF-REQUIREMENTS targets == the
    # contract set active rows project to, selected by frontmatter.
    reg = a_parse_generated_views(root)
    domain_targets = [reg["views"][n]["targets"][0] for n in reg["order"]
                      if n in reg["views"] and reg["views"][n].get("marker") == "PROOF-REQUIREMENTS"
                      and reg["views"][n]["targets"]]
    target_contracts: dict[str, str] = {}
    for rel in domain_targets:
        front = frontmatter((root / rel).read_text(encoding="utf-8")) \
            if (root / rel).is_file() else {}
        target_contracts[rel] = front.get("contract_id", "")
    used = {c for row in active for c in row["contracts"]}
    registered = set(target_contracts.values())
    for missing in sorted(used - registered):
        out.append(f"projection contract {missing} has no registered domain "
                   "proof view")
    for phantom in sorted(registered - used):
        out.append(f"registered domain proof view for {phantom} has no active "
                   "proof relation")
    for rel, contract in sorted(target_contracts.items()):
        text = (root / rel).read_text(encoding="utf-8") if (root / rel).is_file() else ""
        body = batql_extract_block(text, "PROOF-REQUIREMENTS")
        if body is None:
            out.append(f"{rel}: missing generated block PROOF-REQUIREMENTS")
            continue
        want_groups: dict[str, list[str]] = {}
        for row in active:
            if contract in row["contracts"]:
                want_groups.setdefault(row["guarantee"], []).append(row["id"])
        want = ["| Guarantee | Required proof rows |", "| --- | --- |"]
        for guarantee, ids in want_groups.items():
            want.append(f"| {guarantee} | {'; '.join(ids)} |")
        if body != "\n".join(want):
            out.append(f"{rel}: generated block PROOF-REQUIREMENTS does not match "
                       "its typed derivation")
    if "hash_map_iteration_cannot_influence_canonical_observables" in \
            (root / "docs/03_REPOSITORY_AND_PACKAGES.md").read_text(encoding="utf-8"):
        out.append("docs/03 carries the hash-map proof row; canonical collection "
                   "order is docs/04's law")
    # docs/21 ledger parity, including the witness route cells.
    lsrc = (root / "spec/legacy_obligations/inventory.rs").read_text(encoding="utf-8")
    by_leg: dict[str, list[str]] = {}
    for row in active:
        if row["family"] == "leg":
            by_leg.setdefault(row["guarantee"], []).append(row["id"])
    want_ledger = ["| ID | Retained law | Legacy evidence | Clean owner | Mechanism "
                   "disposition | Required witness | Gates | Compatibility | "
                   "Deletion condition | Status |",
                   "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for m in A_E4D_LEG_FULL.finditer(lsrc):
        leg, law, evidence, owner, mech, route, gates, compat, deletion, status = m.groups()
        if not evidence.strip():
            out.append(f"{leg} carries no legacy evidence pointer")
        if not mech.strip():
            out.append(f"{leg} carries no mechanism disposition")
        if route == "CanonicalProofRows":
            witness = "; ".join(by_leg.get(leg, []))
            if not by_leg.get(leg):
                out.append(f"{leg} claims CanonicalProofRows with no active typed relation")
        else:
            witness = A_E4D_PLANNED_TEXT.search(route).group(1)
            if not witness.strip():
                out.append(f"{leg} carries an empty planned witness")
            if by_leg.get(leg):
                out.append(f"{leg} carries a planned witness AND active proof rows")
        want_ledger.append(f"| {leg} | {law} | {evidence} | {owner} | {mech} | "
                           f"{witness} | {_a_gate_token_render(gates, root)} | "
                           f"{compat} | {deletion} | {status} |")
    for guarantee in sorted(set(by_leg) - {m.group(1) for m in A_E4D_LEG_FULL.finditer(lsrc)}):
        out.append(f"a proof relation binds {guarantee}, which no obligation row declares")
    d21_text = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    body = batql_extract_block(d21_text, "LEGACY-OBLIGATION-LEDGER")
    if body is None:
        out.append("docs/21: missing generated block LEGACY-OBLIGATION-LEDGER")
    elif body != "\n".join(want_ledger):
        out.append("docs/21: generated block LEGACY-OBLIGATION-LEDGER does not "
                   "match its typed derivation")
    # The final mirror sweep: with generated blocks and historical migration
    # material removed, no membership inventory or retired ownership claim
    # survives anywhere in the corpus.
    for rel in sorted((root / "docs").glob("*.md")) + sorted((root / "companion").glob("*.md")):
        text = rel.read_text(encoding="utf-8")
        if frontmatter(text).get("status") == "GENERATED":
            continue
        stripped = A_GENERATED_BLOCK.sub("", text)
        stripped = re.sub(r"<!-- HISTORICAL-MIGRATION:BEGIN -->.*?"
                          r"<!-- HISTORICAL-MIGRATION:END -->", "", stripped, flags=re.S)
        name = rel.relative_to(root).as_posix()
        if re.search(r"^\| LEG-\d+ \|", stripped, re.M):
            out.append(f"{name}: an authored legacy-obligation row may not return "
                       "outside the generated ledger")
        if re.search(r"Required witnesses \(proof owner", stripped):
            out.append(f"{name}: an authored required-witnesses inventory may not "
                       "return; spec/proof/inventory.rs owns membership")
        if re.search(r"Required proof rows[^\n]*:\s*\n+```text", stripped):
            out.append(f"{name}: an authored required-proof-rows inventory may not "
                       "return; the PROOF-REQUIREMENTS projection owns it")
        if re.search(r"until the documentary convergence pass", stripped):
            out.append(f"{name}: the temporary documentary-convergence wording may "
                       "not return")
        if re.search(r"docs/24 owns proof-row identity|owns proof-row identity and "
                     r"executable meaning|canonical proof-row owner: docs/24", stripped):
            out.append(f"{name}: docs/24 may not claim proof-row identity ownership; "
                       "spec/proof/types.rs owns identity, docs/24 owns meaning")
    if "until the documentary convergence pass" in psrc:
        out.append("spec/proof/: the temporary documentary-convergence wording "
                   "may not return")
    for field in ("expectation:", "disposition_text:", "meaning:"):
        if re.search(r"\b" + field.rstrip(":") + r":\s*&'static str", psrc):
            out.append(f"spec/proof/: a {field.rstrip(':')} prose field may not "
                       "enter the typed catalog; docs/24 owns meaning")
    # docs/24 doctrine presence.
    d24_stripped = A_GENERATED_BLOCK.sub("", d24_text)
    for name, fragment in (
        ("typed-ownership-split", "spec/proof/` owns ProofRowId identity"),
        ("meaning-stays", "docs/24 owns each active row's authoritative semantic summary"),
        ("relation-cannot-move-meaning", "A generated relation cannot change proof meaning"),
        ("prose-cannot-move-rows", "A prose meaning cannot silently move a proof row"),
    ):
        if fragment not in d24_stripped:
            out.append(f"docs/24: typed proof-ownership doctrine absent ({name})")
    return out


# --- Authored domain-catalog membership parity (E7-E, TL-11) -----------------
# spec/lib.rs is the membership authority; the spec/README.md authored Domain
# catalog carries each domain's meaning. Descriptions remain authored —
# membership does not: every declared domain carries exactly one authored row
# and no authored row admits a domain the crate facade never declares. Judged
# with generated blocks stripped, so the generated module census can never
# stand in for the authored table.
A_DOMAIN_ROW = re.compile(r"^\| `(\w+)/` \|", re.M)


def domain_catalog_findings(root: Path) -> list[str]:
    out: list[str] = []
    lib = root / "spec/lib.rs"
    readme = root / "spec/README.md"
    if not lib.is_file() or not readme.is_file():
        return ["spec/lib.rs or spec/README.md is absent; the domain-catalog "
                "parity law has no denominator"]
    declared: list[str] = []
    for line in _uncomment(lib.read_text(encoding="utf-8")).splitlines():
        m = re.match(r"\s*(#\[cfg\(test\)\]\s*)?pub mod (\w+);", line)
        if m and not m.group(1):
            declared.append(m.group(2))
    rows = A_DOMAIN_ROW.findall(A_GENERATED_BLOCK.sub(
        "", readme.read_text(encoding="utf-8")))
    for dup in sorted({r for r in rows if rows.count(r) > 1}):
        out.append(f"spec/README.md authored domain catalog carries {dup}/ "
                   "twice; one domain, one row")
    for name in [d for d in declared if d not in rows]:
        out.append(f"spec/lib.rs declares pub mod {name}; but the spec/README.md "
                   "authored domain catalog carries no row for it; membership "
                   "derives from the crate facade, only the description is authored")
    for name in [r for r in rows if r not in declared]:
        out.append(f"spec/README.md authored domain catalog carries a row for "
                   f"{name}/, which spec/lib.rs never declares; an authored row "
                   "cannot admit a domain")
    return out


"""Shared corpus primitives for the clean-room audit package.

Owns the frontmatter parser, frozen-file and corpus-scan helpers, the
cross-module spec-source parse primitives (comment stripping, enum/const
variant readers, the bootstrap self-source concatenator, generated-block and
typed-row regexes), control-character integrity, and the derived
stale-vocabulary scanner. Every other submodule imports from here; this module
imports nothing from the audit package.
"""
from __future__ import annotations

import re
from pathlib import Path

STATUSES = {"AUTHORITATIVE", "GENERATED", "EVIDENCE-ONLY", "SUPERSEDED", "REJECTED"}
REQUIRED_FRONT = {"status", "contract_id", "authority_scope", "supersedes", "last_reconciled",
                  "reconciliation_epoch"}
GENERATED_FRONT = {"status", "authority_scope", "generated_by", "generated_from", "do_not_edit",
                   "source_reconciliation_epoch"}
PLACEHOLDERS = ("TBD", "TO BE DECIDED", "IMPLEMENTATION DECIDES", "FIXME")
# Stale vocabulary is DERIVED from spec/dispositions/inventory.rs (never a
# hand-kept list).
# Each retiring decision names its stale aliases and the contexts in which they
# may still appear. Ordinary authoritative material and production/config source
# are not permissive contexts: they must carry an inline [STALE-REF: DEC-...].
STALE_CONTEXT_BY_PATH = {
    "docs/30_DECISION_AND_REJECTION_LEDGER.md": "DecisionLedger",
    "docs/29_STATUS_AND_SUPERSESSION.md": "SupersessionGuide",
    "docs/33_AGENT_FINISH_LINE_CHECKLIST.md": "SupersessionGuide",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md": "RejectionRecord",
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md": "LegacyEvidence",
    "docs/34_LEGACY_INVARIANT_COVERAGE.md": "LegacyEvidence",
    "docs/22_MIGRATION_AND_CUTOVER.md": "MigrationCompatibility",
    "docs/15_SCHEMA_CODEC_AND_MIGRATION.md": "MigrationCompatibility",
    "docs/20_DEPENDENCY_SOVEREIGNTY.md": "MigrationCompatibility",
    "spec/dispositions/inventory.rs": "DecisionLedger",
}
STALE_PROJECTION_DOCS = ("docs/29_STATUS_AND_SUPERSESSION.md", "docs/33_AGENT_FINISH_LINE_CHECKLIST.md")
# The closed stale-context vocabulary (5.5E2): five permissive contexts a
# retiring decision may allow-list, plus the two DEFAULT contexts the scanner
# assigns to everything else. The defaults are never permissive. The typed
# enum in spec/dispositions/types.rs must declare exactly this set — before the
# completion the scanner spoke two context names the typed owner could not.
STALE_PERMISSIVE_CONTEXTS = frozenset({
    "DecisionLedger", "RejectionRecord", "SupersessionGuide",
    "LegacyEvidence", "MigrationCompatibility"})
STALE_DEFAULT_CONTEXTS = frozenset({"ProductionSource", "OrdinaryAuthoritative"})
STALE_REF_RE = re.compile(r"\[STALE-REF:\s*(DEC-\d+)\]")
# The block is generated from spec/dispositions/inventory.rs (5.5D4b). The
# marker carries
# its provenance, so the pattern must not pin the bare form.
STALE_VOCAB_BLOCK_RE = re.compile(r"<!-- STALE-VOCAB:BEGIN[^>]*-->(.*?)<!-- STALE-VOCAB:END -->", re.S)
RETIRING_DISPOSITIONS = {"Kill", "Supersede", "Demote", "Lock"}
PRODUCTION_SOURCE_DIRS = ("crates", "apps")
# Lockstep with freeze.py EXCLUDE_DIRS: .ruff_cache is derived lint-tool
# output (same class as __pycache__). Freeze excludes it, so an audit census
# that counted it would demand a row freeze can never record.
EXCLUDE_DIRS = {".git", "target", "__pycache__", ".ruff_cache"}
EXCLUDE_SUFFIXES = {".zip", ".pyc"}


def frontmatter(text: str) -> dict[str, str]:
    if not text.startswith("---\n"):
        return {}
    end = text.find("\n---\n", 4)
    if end < 0:
        return {}
    out: dict[str, str] = {}
    for line in text[4:end].splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            out[key.strip()] = value.strip()
    return out


def markdown_links(text: str):
    for match in re.finditer(r"\[[^\]]+\]\(([^)]+)\)", text):
        target = match.group(1).split("#", 1)[0]
        if target and not re.match(r"^[a-z]+://", target) and not target.startswith("mailto:"):
            yield target


def frozen_files(root: Path) -> dict[str, Path]:
    # Do not rely on Path object ordering (case-sensitive on POSIX,
    # case-insensitive on Windows). Downstream checks are set/row-based, so
    # enumeration order does not matter here; portability ordering lives in
    # freeze.py and is proven by bootstrap/selftest.py.
    out: dict[str, Path] = {}
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        rel = relative.as_posix()
        if any(part in EXCLUDE_DIRS for part in relative.parts):
            continue
        if rel == "SPEC.sha256" or any(rel.endswith(suffix) for suffix in EXCLUDE_SUFFIXES):
            continue
        out[rel] = path
    return out


def casefold_collisions(rel_paths) -> list[tuple[str, str]]:
    """Distinct paths that collide under case-folding.

    Windows (and case-insensitive filesystems) cannot safely materialize both
    ``Foo.md`` and ``foo.md`` even where POSIX can. Ordering the survey by the
    UTF-8 relative-path bytes keeps the reported pair stable across hosts.
    """
    seen: dict[str, str] = {}
    collisions: list[tuple[str, str]] = []
    for rel in sorted(rel_paths, key=lambda value: value.encode("utf-8")):
        key = rel.casefold()
        if key in seen:
            collisions.append((seen[key], rel))
        else:
            seen[key] = rel
    return collisions


def string_array(source: str, name: str) -> list[str]:
    match = re.search(rf"pub const {re.escape(name)}:\s*&\[[^]]*\]\s*=\s*&\[(.*?)\];", source, re.S)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group(1))


def legacy_manifest_findings(manifest_ids: list[str], coverage_ids: list[str], expected_rows: int) -> list[str]:
    """Declaration-parity between the frozen source manifest and coverage rows.

    D-1: the legacy denominator is measured against declared invariant IDs, never
    a raw ``INV-`` token count. A structural key that resembles an invariant name
    (e.g. the generated ``INV-CATALOG`` block marker) is not a declaration and
    must not appear in the manifest. Every declared id has exactly one coverage
    row and vice versa.
    """
    out: list[str] = []
    if len(manifest_ids) != len(set(manifest_ids)):
        out.append("SOURCE_INVARIANT_IDS contains a duplicate declaration id")
    if len(manifest_ids) != expected_rows:
        out.append(f"SOURCE_INVARIANT_IDS has {len(manifest_ids)} ids, expected {expected_rows}")
    manifest = set(manifest_ids)
    coverage = set(coverage_ids)
    for missing in sorted(manifest - coverage):
        out.append(f"declared legacy invariant {missing} has no coverage row")
    for extra in sorted(coverage - manifest):
        out.append(f"coverage row {extra} is not a declared source invariant")
    return out


def _uncomment(src: str) -> str:
    return re.sub(r"//[^\n]*", "", src)


def batql_extract_block(text: str, name: str) -> str | None:
    match = re.search(
        r"<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n(.*?)\n<!-- " + re.escape(name) + r":END -->",
        text,
        re.S,
    )
    return match.group(1) if match else None


def _bootstrap_source(root: Path, tool: str) -> str:
    """The complete source of one bootstrap tool: its entry file plus, when a
    package directory exists, every package file (Wave-2 SW3a). Detectors grep
    this so a decomposition cannot silently move a guarded symbol out of a
    detector's view."""
    pieces: list[str] = []
    entry = root / "bootstrap" / f"{tool}.py"
    if entry.is_file():
        pieces.append(entry.read_text(encoding="utf-8"))
    pkg = root / "bootstrap" / tool
    if pkg.is_dir():
        for rel in sorted(pkg.rglob("*.py")):
            pieces.append(rel.read_text(encoding="utf-8"))
    return "\n".join(pieces)


def _bootstrap_rust_source(root: Path, tool: str) -> str:
    """The complete source of one bootstrap Rust tool: its root file plus, when a
    module directory exists, every module file (Wave-2 SW4). Detectors grep this
    so a decomposition cannot silently move a guarded symbol out of a detector's
    view."""
    pieces: list[str] = []
    entry = root / "bootstrap" / f"{tool}.rs"
    if entry.is_file():
        pieces.append(entry.read_text(encoding="utf-8"))
    pkg = root / "bootstrap" / tool
    if pkg.is_dir():
        for rel in sorted(pkg.rglob("*.rs")):
            pieces.append(rel.read_text(encoding="utf-8"))
    return "\n".join(pieces)


def _spec_module_source(root: Path, name: str) -> str:
    """The complete source of one spec domain: every module file under the
    domain directory spec/<name>/ in sorted order (SGB source grammar). A
    sibling concept-door file spec/<name>.rs is an unlawful shape and is
    never consulted. Detectors grep this so an intra-domain decomposition
    cannot silently move a guarded symbol out of a detector's view."""
    pkg = root / "spec" / name
    if not pkg.is_dir():
        return ""
    return "\n".join(
        rel.read_text(encoding="utf-8") for rel in sorted(pkg.rglob("*.rs")))


def declared_contract_ids(root: Path) -> set[str]:
    """Every contract_id declared by a document's front matter."""
    out: set[str] = set()
    for path in sorted(root.rglob("*.md")):
        rel = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel.parts):
            continue
        text = path.read_text(encoding="utf-8")
        cid = frontmatter(text).get("contract_id")
        if cid:
            out.add(cid)
    return out


A_DISPOSITION_VARIANT = re.compile(r'WitnessDisposition::(\w+)')


# Both helpers read the authenticated-history claim algebra, which graduated
# from spec/architecture/ to its own domain in F4 (spec::authenticated_history).
def _enum_variants(root: Path, name: str) -> set[str]:
    src = _spec_module_source(root, "authenticated_history")
    m = re.search(r"pub enum " + re.escape(name) + r" \{(.*?)\n\}", src, re.S)
    return set(re.findall(r"^\s{4}(\w+),", m.group(1), re.M)) if m else set()


def _const_variants(root: Path, name: str) -> list[str]:
    src = _spec_module_source(root, "authenticated_history")
    m = re.search(r"pub const " + re.escape(name) + r":[^=]*=\s*&\[(.*?)\];", src, re.S)
    return A_DISPOSITION_VARIANT.findall(m.group(1)) if m else []


A_GENERATED_BLOCK = re.compile(
    r"<!-- [A-Z0-9-]+:BEGIN[^>]*-->.*?<!-- [A-Z0-9-]+:END -->", re.S)


G_LEG_ROW = re.compile(
    r'LegacyObligation \{ id: "([^"]+)", law: "[^"]+", '
    r'legacy_evidence: "(?:[^"\\]|\\.)*", clean_owner: "([^"]+)", '
    r'mechanism_disposition: "(?:[^"\\]|\\.)*", witness_requirement: '
    r'LegacyWitnessRequirement::(?:CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::(\w+), '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: ObligationStatus::(\w+) \}'
)
G_DEC_ROW = re.compile(
    r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::(\w+), '
    r'gates: &\[([^\]]*)\], disposition: Disposition::(\w+),'
)


G_QUAL_ROW = re.compile(
    r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
    r'environment: QualificationEnvironment::(\w+),\s*gates: &\[([^\]]*)\],\s*'
    r'requirement: "[^"]+",\s*\}', re.S)


A_E4D_ACTIVE = re.compile(
    r'ProofRowRecord \{ id: ProofRowId\("([a-z0-9_]+)"\), state: '
    r'ProofRowState::Active \{ guarantee: GuaranteeRef::(leg|dec)\("([^"]+)"\), '
    r'projection_contracts: &\[([^\]]*)\], '
    r'claim: VerificationClaimKind::(\w+), verification: ([A-Z0-9_]+) \} \}')
A_E4D_LEG_FULL = re.compile(
    r'LegacyObligation \{ id: "(LEG-\d+)", law: "((?:[^"\\]|\\.)*)", '
    r'legacy_evidence: "((?:[^"\\]|\\.)*)", clean_owner: "([^"]*)", '
    r'mechanism_disposition: "((?:[^"\\]|\\.)*)", witness_requirement: '
    r'LegacyWitnessRequirement::(CanonicalProofRows|Planned\("(?:[^"\\]|\\.)*"\)), '
    r'gates: &\[([^\]]*)\], compatibility_disposition: CompatibilityDisposition::(\w+), '
    r'deletion_condition: DeletionCondition::(\w+), active_or_closed_status: '
    r'ObligationStatus::(\w+) \}')
A_E4D_PLANNED_TEXT = re.compile(r'Planned\("((?:[^"\\]|\\.)*)"\)')
# The automatic consumers never appear in projection_contracts: docs/24 and
# docs/21 receive every relation mechanically.
A_E4D_AUTOMATIC = {"BP-GAUNTLET-1", "BP-LEGACY-OBLIGATIONS-1"}


# --- Control-character integrity --------------------------------------------
# 5.5C2a shipped a rule containing an ASCII backspace impersonating a word
# boundary; grep and sed rendered it invisibly. Tracked specification text is
# LF-only and carries no other C0 controls.
CONTROL_TEXT_SUFFIXES = {".py", ".rs", ".md", ".sha256", ".toml"}
CONTROL_TEXT_NAMES = {".gitattributes", ".gitignore"}
CONTROL_ALLOWED = {0x09, 0x0A}  # TAB, LF. CR is rejected: the tree freezes LF.


def control_character_findings(root: Path) -> list[str]:
    out: list[str] = []
    for rel, path in sorted(frozen_files(root).items()):
        if path.suffix not in CONTROL_TEXT_SUFFIXES and path.name not in CONTROL_TEXT_NAMES:
            continue
        data = path.read_bytes()
        for offset, byte in enumerate(data):
            if (byte < 0x20 and byte not in CONTROL_ALLOWED) or byte == 0x7F:
                out.append(f"{rel}: byte {offset}: forbidden control character U+{byte:04X}")
                break
    return out


def parse_stale_vocabulary(root: Path, findings: list[str]) -> dict[str, tuple[str, frozenset[str], str]]:
    """Derive the stale-alias matcher from the spec/dispositions domain
    (StaleContext vocabulary in types.rs, DECISIONS rows in inventory.rs).

    Returns alias_lower -> (owning decision id, allowed contexts, canonical alias).
    The matcher is a consequence of the decisions that retired vocabulary; there
    is no separately authored stale-term list.
    """
    source = _spec_module_source(root, "dispositions")
    if not source:
        findings.append("missing spec/dispositions/ for stale-vocabulary derivation")
        return {}
    # The typed enum declares EXACTLY the vocabulary this scanner speaks:
    # the five permissive contexts plus the two never-permissive defaults.
    enum_body = re.search(r"pub enum StaleContext \{(.*?)\n\}", _uncomment(source), re.S)
    declared = set(re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M)) if enum_body else set()
    want = STALE_PERMISSIVE_CONTEXTS | STALE_DEFAULT_CONTEXTS
    if declared != want:
        findings.append(
            f"spec/dispositions/types.rs StaleContext variants {sorted(declared)} != "
            f"the scanner's closed vocabulary {sorted(want)}")
    if not (set(STALE_CONTEXT_BY_PATH.values()) <= STALE_PERMISSIVE_CONTEXTS):
        findings.append("audit's stale-context path map names an undeclared or non-permissive context")
    alias_map: dict[str, tuple[str, frozenset[str], str]] = {}
    row = re.compile(
        r'DecisionSpec \{ id: "(DEC-\d+)", class: DecisionClass::\w+, gates: &\[[^\]]*\], '
        r"disposition: Disposition::(\w+),.*?"
        r"stale_aliases: &\[([^\]]*)\], stale_allowed_contexts: &\[([^\]]*)\],"
    )
    for ident, disposition, aliases_raw, contexts_raw in row.findall(source):
        aliases = re.findall(r'"([^"]+)"', aliases_raw)
        contexts = frozenset(re.findall(r"StaleContext::(\w+)", contexts_raw))
        if aliases and disposition not in RETIRING_DISPOSITIONS:
            findings.append(f"spec/dispositions/inventory.rs: {ident} carries stale aliases but disposition {disposition} does not retire vocabulary")
        if aliases and not contexts:
            findings.append(f"spec/dispositions/inventory.rs: {ident} has stale aliases but no allowed contexts")
        for ctx in sorted(contexts & STALE_DEFAULT_CONTEXTS):
            findings.append(
                f"spec/dispositions/inventory.rs: {ident} allow-lists the default context {ctx}; "
                "production source and ordinary authoritative material always require "
                "an inline STALE-REF")
        for alias in aliases:
            key = alias.lower()
            if key in alias_map:
                findings.append(f"spec/dispositions/inventory.rs: stale alias {alias!r} claimed by {alias_map[key][0]} and {ident}")
            alias_map[key] = (ident, contexts, alias)
    return alias_map


def stale_context_for(rel: str, status: str | None) -> str:
    if rel in STALE_CONTEXT_BY_PATH:
        return STALE_CONTEXT_BY_PATH[rel]
    if status == "EVIDENCE-ONLY":
        return "LegacyEvidence"
    if rel.split("/", 1)[0] in PRODUCTION_SOURCE_DIRS:
        return "ProductionSource"
    return "OrdinaryAuthoritative"


def scan_stale_occurrences(rel: str, text: str, context: str, alias_map, findings: list[str]) -> None:
    lower = text.lower()
    known = {ident for ident, _, _ in alias_map.values()}
    for match in STALE_REF_RE.finditer(text):
        if match.group(1) not in known:
            findings.append(f"{rel}: STALE-REF names {match.group(1)}, which owns no stale alias")
    for key, (ident, contexts, alias) in alias_map.items():
        start = 0
        while True:
            pos = lower.find(key, start)
            if pos < 0:
                break
            start = pos + len(key)
            if context in contexts:
                continue
            line_start = text.rfind("\n", 0, pos) + 1
            line_end = text.find("\n", pos)
            line = text[line_start: line_end if line_end >= 0 else len(text)]
            if ident in STALE_REF_RE.findall(line):
                continue
            findings.append(f"{rel}: stale term {alias!r} ({ident}) in {context} context needs [STALE-REF: {ident}]")


def check_stale_vocabulary(root: Path, findings: list[str]) -> None:
    alias_map = parse_stale_vocabulary(root, findings)
    if not alias_map:
        return
    derived = {alias for _, _, alias in alias_map.values()}
    for path in sorted(root.rglob("*.md")):
        rel_path = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in rel_path.parts):
            continue
        rel = rel_path.as_posix()
        text = path.read_text(encoding="utf-8")
        context = stale_context_for(rel, frontmatter(text).get("status"))
        scan_stale_occurrences(rel, text, context, alias_map, findings)
    for base in PRODUCTION_SOURCE_DIRS:
        base_dir = root / base
        if not base_dir.is_dir():
            continue
        for path in sorted(base_dir.rglob("*")):
            if not path.is_file() or path.suffix not in (".rs", ".toml"):
                continue
            rel = path.relative_to(root).as_posix()
            scan_stale_occurrences(rel, path.read_text(encoding="utf-8"), "ProductionSource", alias_map, findings)
    for rel in STALE_PROJECTION_DOCS:
        path = root / rel
        if not path.is_file():
            findings.append(f"missing stale-vocabulary projection doc {rel}")
            continue
        block = STALE_VOCAB_BLOCK_RE.search(path.read_text(encoding="utf-8"))
        if not block:
            findings.append(f"{rel}: missing <!-- STALE-VOCAB:BEGIN/END --> projection block")
            continue
        listed = {
            line.strip()
            for line in block.group(1).splitlines()
            if line.strip() and not line.strip().startswith("```")
        }
        for alias in sorted(derived - listed):
            findings.append(f"{rel}: stale projection missing derived alias {alias!r}")
        for alias in sorted(listed - derived):
            findings.append(f"{rel}: stale projection lists {alias!r}, not derived from any decision")


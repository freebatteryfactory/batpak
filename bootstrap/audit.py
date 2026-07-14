#!/usr/bin/env python3
"""Independent standard-library audit for the clean-room specification."""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

STATUSES = {"AUTHORITATIVE", "GENERATED", "EVIDENCE-ONLY", "SUPERSEDED", "REJECTED"}
REQUIRED_FRONT = {"status", "contract_id", "authority_scope", "supersedes", "last_reconciled"}
PLACEHOLDERS = ("TBD", "TO BE DECIDED", "IMPLEMENTATION DECIDES", "FIXME")
# Stale vocabulary is DERIVED from spec/dispositions.rs (never a hand-kept list).
# Each retiring decision names its stale aliases and the contexts in which they
# may still appear. Ordinary authoritative material and production/config source
# are not permissive contexts: they must carry an inline [STALE-REF: DEC-...].
STALE_CONTEXT_BY_PATH = {
    "docs/30_DECISION_AND_REJECTION_LEDGER.md": "DecisionLedger",
    "docs/29_STATUS_AND_SUPERSESSION.md": "SupersessionGuide",
    "docs/33_AGENT_FINISH_LINE_CHECKLIST.md": "SupersessionGuide",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md": "RejectionRecord",
    "FINAL_RECONCILIATION.md": "RejectionRecord",
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md": "LegacyEvidence",
    "docs/34_LEGACY_INVARIANT_COVERAGE.md": "LegacyEvidence",
    "docs/22_MIGRATION_AND_CUTOVER.md": "MigrationCompatibility",
    "docs/15_SCHEMA_CODEC_AND_MIGRATION.md": "MigrationCompatibility",
    "docs/20_DEPENDENCY_SOVEREIGNTY.md": "MigrationCompatibility",
    "spec/dispositions.rs": "DecisionLedger",
}
STALE_PROJECTION_DOCS = ("docs/29_STATUS_AND_SUPERSESSION.md", "docs/33_AGENT_FINISH_LINE_CHECKLIST.md")
STALE_REF_RE = re.compile(r"\[STALE-REF:\s*(DEC-\d+)\]")
STALE_VOCAB_BLOCK_RE = re.compile(r"<!-- STALE-VOCAB:BEGIN -->(.*?)<!-- STALE-VOCAB:END -->", re.S)
RETIRING_DISPOSITIONS = {"Kill", "Supersede", "Demote", "Lock"}
PRODUCTION_SOURCE_DIRS = ("crates", "apps")
EXCLUDE_DIRS = {".git", "target", "__pycache__"}
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


# --- BatQL: OperatorSpec parity, grammar closure, proof disposition ----------
# This is the AUDITOR half of the generator/auditor pair. It recomputes the
# operator projection INDEPENDENTLY of bootstrap/project.py (its own parser,
# its own renderers) and compares. The two share no parsing, normalization,
# projection-building, or comparison logic; only the inert marker names are
# common, and those are asserted on both sides.
BATQL_OP_ROW = re.compile(
    r'OperatorSpec \{ id: "([^"]*)", class: OperatorClass::(\w+), '
    r'word_surface: "([^"]*)", symbol_surface: "([^"]*)", semantic_op: "([^"]*)", '
    r'arity: Arity::(\w+), fixity: Fixity::(\w+), precedence: (\d+), '
    r'associativity: Associativity::(\w+), input_sorts: "([^"]*)", '
    r'result_sort: "([^"]*)", exactness: Exactness::(\w+), overflow: "([^"]*)", '
    r'exception: "([^"]*)", formatting: "([^"]*)", spoken: "([^"]*)", '
    r'mutation_classes: "([^"]*)" \}'
)
BATQL_OP_FIELDS = (
    "id", "class", "word", "symbol", "semantic_op", "arity", "fixity",
    "precedence", "associativity", "input_sorts", "result_sort", "exactness",
    "overflow", "exception", "formatting", "spoken", "mutation_classes",
)
BATQL_CLASS_RANK = {"Arithmetic": 0, "Comparison": 1, "Logical": 2}
BATQL_PROOF_DISPOSITIONS = ["VERIFIED", "LEGACY_WEAK", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]


def batql_parse_operators(root: Path) -> list[dict[str, str]]:
    source = (root / "spec/operators.rs").read_text(encoding="utf-8")
    return [dict(zip(BATQL_OP_FIELDS, match)) for match in BATQL_OP_ROW.findall(source)]


def _batql_canonical(ops: list[dict[str, str]]) -> list[dict[str, str]]:
    return sorted(ops, key=lambda o: (BATQL_CLASS_RANK.get(o["class"], 9), o["id"]))


def batql_render_catalog(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Class | Word | Symbol | Arity | Fixity | Precedence | Associativity | Result sort | Exactness |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for op in _batql_canonical(ops):
        symbol = f"`{op['symbol']}`" if op["symbol"] else ""
        out.append(
            f"| {op['id']} | {op['class']} | `{op['word']}` | {symbol} | {op['arity']} | "
            f"{op['fixity']} | {op['precedence']} | {op['associativity']} | {op['result_sort']} | {op['exactness']} |"
        )
    return "\n".join(out)


def batql_render_grammar(ops: list[dict[str, str]]) -> str:
    ops = _batql_canonical(ops)
    comparison = [o["word"] for o in ops if o["class"] == "Comparison"]
    comparison += [f'"{o["symbol"]}"' for o in ops if o["class"] == "Comparison" and o["symbol"]]
    arithmetic = [f'"{o["word"]}"' for o in ops if o["class"] == "Arithmetic"]
    logical = [o["word"] for o in ops if o["class"] == "Logical" and o["fixity"] == "Infix"]
    return "\n".join([
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
    ])


def batql_render_projection(ops: list[dict[str, str]]) -> str:
    out = [
        "| OperatorId | Formatting | Spoken | Legal mutation classes |",
        "| --- | --- | --- | --- |",
    ]
    for op in _batql_canonical(ops):
        out.append(f"| {op['id']} | `{op['formatting']}` | {op['spoken']} | {op['mutation_classes']} |")
    return "\n".join(out)


def batql_extract_block(text: str, name: str) -> str | None:
    match = re.search(
        r"<!-- " + re.escape(name) + r":BEGIN[^>]*-->\n(.*?)\n<!-- " + re.escape(name) + r":END -->",
        text,
        re.S,
    )
    return match.group(1) if match else None


def batql_operator_fact_findings(ops: list[dict[str, str]]) -> list[str]:
    out: list[str] = []
    ids = [o["id"] for o in ops]
    if len(ids) != len(set(ids)):
        out.append("spec/operators.rs: duplicate OperatorId")
    owner: dict[tuple[str, str], str] = {}
    for op in ops:
        for surface in (op["word"], op["symbol"]):
            if not surface:
                continue
            key = (surface, op["fixity"])
            if key in owner and owner[key] != op["id"]:
                out.append(
                    f"spec/operators.rs: token {surface!r} with fixity {op['fixity']} claimed by {owner[key]} and {op['id']}"
                )
            owner[key] = op["id"]
    required = [f for f in BATQL_OP_FIELDS if f != "symbol"]
    for op in ops:
        for field in required:
            if not op.get(field, "").strip():
                out.append(f"spec/operators.rs: operator {op.get('id', '?')} missing field {field}")
    return out


def batql_grammar_fences(companion: str) -> list[str]:
    start = companion.find("\n# 15.")
    if start < 0:
        return []
    end = companion.find("\n# 16.", start + 1)
    region = companion[start: end if end >= 0 else len(companion)]
    fences = re.findall(r"```text\n(.*?)\n```", region, re.S)
    return [fence for fence in fences if "  :=" in fence]


def batql_grammar_closure_findings(fences: list[str]) -> tuple[list[str], int, int]:
    """Returns (findings, defined_count, undefined_count)."""
    defined: list[str] = []
    referenced: set[str] = set()
    for fence in fences:
        for line in fence.split("\n"):
            if re.match(r"^[a-z][a-z0-9_]*$", line):
                defined.append(line)
                continue
            stripped = line.strip()
            if stripped.startswith(":=") or stripped.startswith("|"):
                rhs = stripped[2:] if stripped.startswith(":=") else stripped[1:]
                rhs = re.sub(r"<[^>]*>", " ", rhs)
                rhs = re.sub(r'"[^"]*"', " ", rhs)
                for token in re.findall(r"[a-z][a-z0-9_]*", rhs):
                    referenced.add(token)
    out: list[str] = []
    seen: set[str] = set()
    for name in defined:
        if name in seen:
            out.append(f"grammar nonterminal {name} has more than one owning production")
        seen.add(name)
    undefined = sorted(referenced - set(defined))
    for name in undefined:
        out.append(f"grammar references undefined nonterminal {name}")
    return out, len(set(defined)), len(undefined)


def batql_proof_disposition_states(companion: str) -> list[str]:
    match = re.search(r"## 4\.5 Proof disposition\n\n```text\n(.*?)\n```", companion, re.S)
    if not match:
        return []
    return [line.strip() for line in match.group(1).split("\n") if line.strip()]


def batql_proof_disposition_findings(states: list[str]) -> list[str]:
    if states != BATQL_PROOF_DISPOSITIONS:
        return [f"BatQL proof-disposition axis is {states}, expected {BATQL_PROOF_DISPOSITIONS}"]
    return []


def check_batql(root: Path, findings: list[str]) -> None:
    companion_path = root / "companion/BATQL_LANGUAGE.md"
    if not companion_path.is_file():
        findings.append("missing companion/BATQL_LANGUAGE.md")
        return
    companion = companion_path.read_text(encoding="utf-8")
    ops = batql_parse_operators(root)
    if not ops:
        findings.append("spec/operators.rs: no OperatorSpec rows parsed")
    findings.extend(batql_operator_fact_findings(ops))
    for name, expected in (
        ("OPERATORS-CATALOG", batql_render_catalog(ops)),
        ("OPERATORS-GRAMMAR", batql_render_grammar(ops)),
        ("OPERATORS-PROJECTION", batql_render_projection(ops)),
    ):
        actual = batql_extract_block(companion, name)
        if actual is None:
            findings.append(f"companion/BATQL_LANGUAGE.md: missing generated block {name}")
        elif actual != expected:
            findings.append(
                f"companion/BATQL_LANGUAGE.md: generated block {name} does not match the spec/operators.rs projection"
            )
    grammar_findings, _defined, _undefined = batql_grammar_closure_findings(batql_grammar_fences(companion))
    findings.extend(grammar_findings)
    findings.extend(batql_proof_disposition_findings(batql_proof_disposition_states(companion)))
    if "`REQUIRE VERIFIED` accepts only `VERIFIED`" not in companion:
        findings.append("companion/BATQL_LANGUAGE.md 4.5: REQUIRE VERIFIED-only law missing")
    if "may upgrade or collapse" not in companion:
        findings.append("companion/BATQL_LANGUAGE.md 4.5: proof-disposition no-upgrade/no-collapse law missing")
    receipts_path = root / "docs/14_RECEIPTS_AND_EXPLANATION.md"
    if receipts_path.is_file() and "LegacyWeak" not in receipts_path.read_text(encoding="utf-8"):
        findings.append("docs/14_RECEIPTS_AND_EXPLANATION.md: LegacyWeak proof state missing")


def check_architecture(root: Path, findings: list[str]) -> None:
    path = root / "spec/architecture.rs"
    if not path.is_file():
        findings.append("missing spec/architecture.rs")
        return
    source = path.read_text(encoding="utf-8")
    package_rows = re.findall(
        r"PackageSpec\s*\{\s*package:\s*\"([^\"]+)\",\s*path:\s*\"([^\"]+)\",\s*role:\s*\"([^\"]+)\",\s*class:\s*PackageClass::([A-Za-z]+),\s*layer:\s*([0-9]+),\s*\}",
        source,
        re.S,
    )
    if not package_rows:
        findings.append("spec/architecture.rs: no package rows parsed")
        return
    packages = [row[0] for row in package_rows]
    paths = [row[1] for row in package_rows]
    layers = {row[0]: int(row[4]) for row in package_rows}
    if len(packages) != len(set(packages)):
        findings.append("spec/architecture.rs: duplicate package name")
    if len(paths) != len(set(paths)):
        findings.append("spec/architecture.rs: duplicate package path")
    for package, package_path, role, package_class, layer in package_rows:
        if not package_path or not role or package_class not in {"Production", "BinaryAdapter", "DevOnly", "Example"} or not layer.isdigit():
            findings.append(f"spec/architecture.rs: incomplete package {package}")

    edge_rows = re.findall(
        r"EdgeSpec\s*\{\s*importer:\s*\"([^\"]+)\",\s*importee:\s*\"([^\"]+)\",\s*class:\s*EdgeClass::([A-Za-z]+),\s*profile:\s*\"([^\"]+)\"\s*\}",
        source,
        re.S,
    )
    graph: dict[str, list[str]] = {package: [] for package in packages}
    package_set = set(packages)
    for importer, importee, edge_class, profile in edge_rows:
        if importer not in package_set or importee not in package_set:
            findings.append(f"spec/architecture.rs: unknown package in edge {importer} -> {importee}")
            continue
        if importer == importee:
            findings.append(f"spec/architecture.rs: self edge {importer}")
        if edge_class not in {"Required", "OptionalProfile", "DevOnly"} or not profile:
            findings.append(f"spec/architecture.rs: incomplete edge {importer} -> {importee}")
        if importer in layers and importee in layers and layers[importer] <= layers[importee]:
            findings.append(f"spec/architecture.rs: dependency direction violation {importer}(L{layers[importer]}) -> {importee}(L{layers[importee]})")
        graph[importer].append(importee)

    visiting: set[str] = set()
    visited: set[str] = set()

    def cycle(node: str) -> bool:
        if node in visited:
            return False
        if node in visiting:
            return True
        visiting.add(node)
        for child in graph.get(node, []):
            if cycle(child):
                return True
        visiting.remove(node)
        visited.add(node)
        return False

    for package in packages:
        visiting.clear()
        if cycle(package):
            findings.append(f"spec/architecture.rs: dependency cycle reaches {package}")
            break

    profile_rows = re.findall(
        r"QualificationProfile\s*\{\s*package:\s*\"([^\"]+)\",\s*profile:\s*\"([^\"]+)\",\s*target:\s*\"([^\"]+)\",\s*requirement:\s*\"([^\"]+)\",\s*\}",
        source,
        re.S,
    )
    identities: set[tuple[str, str]] = set()
    for package, profile, target, requirement in profile_rows:
        identity = (package, profile)
        if package not in package_set or not profile or not target or not requirement:
            findings.append(f"spec/architecture.rs: incomplete qualification {package}:{profile}")
        if identity in identities:
            findings.append(f"spec/architecture.rs: duplicate qualification {package}:{profile}")
        identities.add(identity)
    for package in ("batpak", "syncbat"):
        if (package, "semantic") not in identities or not any(
            row[0] == package and row[1] == "semantic" and row[2] == "no_std + alloc" for row in profile_rows
        ):
            findings.append(f"spec/architecture.rs: missing no_std + alloc semantic profile for {package}")

    for relative in string_array(source, "REQUIRED_DOCS"):
        if not (root / relative).is_file():
            findings.append(f"spec/architecture.rs: required file missing {relative}")
    for relative in string_array(source, "FORBIDDEN_TARGET_PATHS"):
        if (root / relative).exists():
            findings.append(f"spec/architecture.rs: forbidden path exists {relative}")
    syncbat_root = root / "crates/syncbat"
    if syncbat_root.exists():
        for relative in string_array(source, "SYNCBAT_REQUIRED_PLANES"):
            if not (syncbat_root / relative).exists():
                findings.append(f"syncbat source missing declared plane {relative}")


def check_seed_ids(root: Path, findings: list[str]) -> None:
    sources = {
        "invariant": (root / "spec/invariants.rs", r'id:\s*"(SEED-[A-Z0-9-]+)"'),
        "decision": (root / "spec/dispositions.rs", r'id:\s*"(DEC-[0-9]+)"'),
        "legacy": (root / "spec/legacy_obligations.rs", r'id:\s*"(LEG-[0-9]+)"'),
        "legacy_coverage": (root / "spec/legacy_invariant_coverage.rs", r'legacy_id:\s*"(INV-[A-Z0-9-]+)"'),
    }
    parsed: dict[str, list[str]] = {}
    for kind, (path, pattern) in sources.items():
        if not path.is_file():
            findings.append(f"missing {path.relative_to(root)}")
            continue
        values = re.findall(pattern, path.read_text(encoding="utf-8"))
        if len(values) != len(set(values)):
            findings.append(f"duplicate {kind} ID in {path.relative_to(root)}")
        parsed[kind] = values

    decision_doc = (root / "docs/30_DECISION_AND_REJECTION_LEDGER.md").read_text(encoding="utf-8")
    decision_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in decision_doc.splitlines():
        if not line.startswith("| DEC-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 4:
            findings.append(f"decision ledger malformed row: {line}")
            continue
        ident, tag, subject, successor = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        decision_doc_rows[ident] = (tag, clean(subject), clean(successor))
    disposition_tags = {
        "Keep": "KEEP",
        "Lock": "LOCK",
        "Kill": "KILL",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Defer": "DEFER",
        "OpenImplementation": "OPEN-IMPLEMENTATION",
        "RetainAsEvidence": "RETAIN-AS-EVIDENCE",
    }
    decision_source = (root / "spec/dispositions.rs").read_text(encoding="utf-8")
    decision_seed_rows = {
        ident: (disposition_tags[variant], subject, successor)
        for ident, variant, subject, successor in re.findall(
            r'DecisionSpec \{ id: "([^"]+)", disposition: Disposition::([A-Za-z]+), subject: "([^"]+)", successor: "([^"]+)"',
            decision_source,
        )
    }
    for value in parsed.get("decision", []):
        if value not in decision_doc_rows:
            findings.append(f"decision seed {value} absent from docs/30_DECISION_AND_REJECTION_LEDGER.md")
        elif decision_seed_rows.get(value) != decision_doc_rows[value]:
            findings.append(f"decision seed/document mismatch for {value}")
    legacy_doc = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    legacy_doc_rows: dict[str, tuple[str, str, str, str, str, str]] = {}
    for line in legacy_doc.splitlines():
        if not line.startswith("| LEG-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 10:
            findings.append(f"legacy ledger malformed row: {line}")
            continue
        ident, law, _evidence, owner, _mechanism, _witness, gate, compat, deletion, status = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        legacy_doc_rows[ident] = (
            clean(law), clean(owner), clean(gate), clean(compat), clean(deletion), clean(status)
        )
    legacy_source = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    legacy_seed_rows = {
        ident: (law, owner, gate, compat, deletion, status)
        for ident, law, owner, gate, compat, deletion, status in re.findall(
            r'LegacyObligation \{ id: "([^"]+)", law: "([^"]+)", clean_owner: "([^"]+)", gate: "([^"]+)", '
            r"compatibility_disposition: CompatibilityDisposition::(\w+), "
            r"deletion_condition: DeletionCondition::(\w+), "
            r"active_or_closed_status: ObligationStatus::(\w+) \}",
            legacy_source,
        )
    }
    for value in parsed.get("legacy", []):
        if value not in legacy_doc_rows:
            findings.append(f"legacy seed {value} absent from docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md")
        elif legacy_seed_rows.get(value) != legacy_doc_rows[value]:
            findings.append(f"legacy seed/document mismatch for {value}")

    coverage_doc = (root / "docs/34_LEGACY_INVARIANT_COVERAGE.md").read_text(encoding="utf-8")
    coverage_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in coverage_doc.splitlines():
        if not line.startswith("| `INV-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 4:
            findings.append(f"legacy invariant coverage malformed row: {line}")
            continue
        ident, tag, successor, rationale = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        coverage_doc_rows[clean(ident)] = (tag, clean(successor), rationale)
    coverage_tags = {
        "Preserve": "PRESERVE",
        "Supersede": "SUPERSEDE",
        "Demote": "DEMOTE",
        "Kill": "KILL",
        "Requalify": "REQUALIFY",
    }
    coverage_source = (root / "spec/legacy_invariant_coverage.rs").read_text(encoding="utf-8")
    coverage_seed_rows = {
        ident: (coverage_tags[variant], successor, rationale)
        for ident, variant, successor, rationale in re.findall(
            r'LegacyInvariantCoverage \{ legacy_id: "([^"]+)", disposition: CoverageDisposition::([A-Za-z]+), successor: "([^"]+)", rationale: "([^"]+)" \}',
            coverage_source,
        )
    }
    expected_count_match = re.search(r"EXPECTED_COVERAGE_ROWS:\s*usize\s*=\s*([0-9]+)", coverage_source)
    expected_count = int(expected_count_match.group(1)) if expected_count_match else -1
    if expected_count != len(parsed.get("legacy_coverage", [])):
        findings.append(f"legacy invariant coverage count is {len(parsed.get('legacy_coverage', []))}, expected {expected_count}")
    source_commit = re.search(r'LEGACY_SOURCE_COMMIT:\s*&str\s*=\s*"([0-9a-f]+)"', coverage_source)
    if source_commit is None or len(source_commit.group(1)) != 40:
        findings.append("legacy invariant coverage source commit is missing or not a full SHA")
    manifest_ids = string_array(coverage_source, "SOURCE_INVARIANT_IDS")
    if not manifest_ids:
        findings.append("spec/legacy_invariant_coverage.rs: missing SOURCE_INVARIANT_IDS manifest")
    else:
        findings.extend(
            legacy_manifest_findings(manifest_ids, parsed.get("legacy_coverage", []), expected_count)
        )
    for value in parsed.get("legacy_coverage", []):
        if value not in coverage_doc_rows:
            findings.append(f"legacy invariant coverage seed {value} absent from docs/34_LEGACY_INVARIANT_COVERAGE.md")
        elif coverage_seed_rows.get(value) != coverage_doc_rows[value]:
            findings.append(f"legacy invariant coverage seed/document mismatch for {value}")


def parse_stale_vocabulary(root: Path, findings: list[str]) -> dict[str, tuple[str, frozenset[str], str]]:
    """Derive the stale-alias matcher from spec/dispositions.rs.

    Returns alias_lower -> (owning decision id, allowed contexts, canonical alias).
    The matcher is a consequence of the decisions that retired vocabulary; there
    is no separately authored stale-term list.
    """
    path = root / "spec/dispositions.rs"
    if not path.is_file():
        findings.append("missing spec/dispositions.rs for stale-vocabulary derivation")
        return {}
    source = path.read_text(encoding="utf-8")
    alias_map: dict[str, tuple[str, frozenset[str], str]] = {}
    row = re.compile(
        r'DecisionSpec \{ id: "(DEC-\d+)", disposition: Disposition::(\w+),.*?'
        r"stale_aliases: &\[([^\]]*)\], stale_allowed_contexts: &\[([^\]]*)\],"
    )
    for ident, disposition, aliases_raw, contexts_raw in row.findall(source):
        aliases = re.findall(r'"([^"]+)"', aliases_raw)
        contexts = frozenset(re.findall(r"StaleContext::(\w+)", contexts_raw))
        if aliases and disposition not in RETIRING_DISPOSITIONS:
            findings.append(f"spec/dispositions.rs: {ident} carries stale aliases but disposition {disposition} does not retire vocabulary")
        if aliases and not contexts:
            findings.append(f"spec/dispositions.rs: {ident} has stale aliases but no allowed contexts")
        for alias in aliases:
            key = alias.lower()
            if key in alias_map:
                findings.append(f"spec/dispositions.rs: stale alias {alias!r} claimed by {alias_map[key][0]} and {ident}")
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


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    findings: list[str] = []
    markdown = sorted(root.rglob("*.md"))
    contract_ids: dict[str, Path] = {}
    for path in markdown:
        relative_path = path.relative_to(root)
        if any(part in EXCLUDE_DIRS for part in relative_path.parts):
            continue
        rel = relative_path.as_posix()
        text = path.read_text(encoding="utf-8")
        meta = frontmatter(text)
        missing = REQUIRED_FRONT - meta.keys()
        if missing:
            findings.append(f"{rel}: missing frontmatter keys {sorted(missing)}")
        elif meta["status"] not in STATUSES:
            findings.append(f"{rel}: unknown status {meta['status']!r}")
        cid = meta.get("contract_id")
        if cid:
            if cid in contract_ids:
                findings.append(f"duplicate contract_id {cid}: {rel} and {contract_ids[cid]}")
            contract_ids[cid] = path
        if meta.get("status") == "AUTHORITATIVE":
            upper = text.upper()
            for token in PLACEHOLDERS:
                if token in upper:
                    findings.append(f"{rel}: normative placeholder {token}")
        for target in markdown_links(text):
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root)
            except ValueError:
                findings.append(f"{rel}: link escapes bundle: {target}")
                continue
            if not resolved.exists():
                findings.append(f"{rel}: broken relative link: {target}")

    check_architecture(root, findings)
    check_seed_ids(root, findings)
    check_stale_vocabulary(root, findings)
    check_batql(root, findings)

    manifest = root / "SPEC.sha256"
    actual_files = frozen_files(root)
    for earlier, later in casefold_collisions(actual_files.keys()):
        findings.append(f"case-folded path collision (not Windows-portable): {earlier} and {later}")
    manifest_rows: dict[str, str] = {}
    if not manifest.is_file():
        findings.append("missing SPEC.sha256")
    else:
        for line_no, line in enumerate(manifest.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.startswith("#"):
                continue
            try:
                expected, rel = line.split("  ", 1)
            except ValueError:
                findings.append(f"SPEC.sha256:{line_no}: malformed row")
                continue
            if rel in manifest_rows:
                findings.append(f"SPEC.sha256:{line_no}: duplicate row {rel}")
            manifest_rows[rel] = expected
            path = actual_files.get(rel)
            if path is None:
                findings.append(f"SPEC.sha256:{line_no}: missing or excluded {rel}")
                continue
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != expected:
                findings.append(f"SPEC.sha256:{line_no}: digest mismatch {rel}")
        missing_rows = sorted(set(actual_files) - set(manifest_rows))
        extra_rows = sorted(set(manifest_rows) - set(actual_files))
        for rel in missing_rows:
            findings.append(f"SPEC.sha256: unrecorded file {rel}")
        for rel in extra_rows:
            findings.append(f"SPEC.sha256: row has no frozen file {rel}")

    if findings:
        print(f"audit: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print(
        f"audit: PASS ({len(markdown)} markdown documents, {len(contract_ids)} contract IDs, "
        f"{len(actual_files)} frozen files)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

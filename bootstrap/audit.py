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
STALE_TERMS = ("FileBat", "filebat package", "bat-vm package", "src/_types", "meaning-free umbrella")
STALE_EXEMPT = {
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
    "docs/30_DECISION_AND_REJECTION_LEDGER.md",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md",
    "FINAL_RECONCILIATION.md",
}
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
        if not package_path or not role or package_class not in {"Production", "BinaryAdapter", "DevOnly"} or not layer.isdigit():
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
            r'DecisionSpec \{ id: "([^"]+)", disposition: Disposition::([A-Za-z]+), subject: "([^"]+)", successor: "([^"]+)" \}',
            decision_source,
        )
    }
    for value in parsed.get("decision", []):
        if value not in decision_doc_rows:
            findings.append(f"decision seed {value} absent from docs/30_DECISION_AND_REJECTION_LEDGER.md")
        elif decision_seed_rows.get(value) != decision_doc_rows[value]:
            findings.append(f"decision seed/document mismatch for {value}")
    legacy_doc = (root / "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md").read_text(encoding="utf-8")
    legacy_doc_rows: dict[str, tuple[str, str, str]] = {}
    for line in legacy_doc.splitlines():
        if not line.startswith("| LEG-"):
            continue
        cells = [cell.strip() for cell in line.strip().split("|")[1:-1]]
        if len(cells) != 7:
            findings.append(f"legacy ledger malformed row: {line}")
            continue
        ident, law, _evidence, owner, _mechanism, _witness, gate = cells
        clean = lambda value: re.sub(r"`([^`]*)`", r"\1", value)
        legacy_doc_rows[ident] = (clean(law), clean(owner), clean(gate))
    legacy_source = (root / "spec/legacy_obligations.rs").read_text(encoding="utf-8")
    legacy_seed_rows = {
        ident: (law, owner, gate)
        for ident, law, owner, gate in re.findall(
            r'LegacyObligation \{ id: "([^"]+)", law: "([^"]+)", clean_owner: "([^"]+)", gate: "([^"]+)" \}',
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
    for value in parsed.get("legacy_coverage", []):
        if value not in coverage_doc_rows:
            findings.append(f"legacy invariant coverage seed {value} absent from docs/34_LEGACY_INVARIANT_COVERAGE.md")
        elif coverage_seed_rows.get(value) != coverage_doc_rows[value]:
            findings.append(f"legacy invariant coverage seed/document mismatch for {value}")


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
            if rel not in STALE_EXEMPT:
                for token in STALE_TERMS:
                    if token.lower() in text.lower():
                        findings.append(f"{rel}: stale architecture term {token!r}")
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

#!/usr/bin/env python3
"""Independent portability self-test for the bootstrap freeze/audit tools.

Turns the first executable finding (host-dependent manifest ordering) into a
permanent guarantee. Standard library only; operates on synthetic temporary
trees and leaves no repository artifacts.

Proven properties:

1. Manifest ordering derives ONLY from the UTF-8 bytes of the POSIX relative
   path, never from filesystem enumeration or the host locale/case behavior.
2. audit rejects a case-folded path collision (e.g. ``Foo.md`` vs ``foo.md``)
   that a case-insensitive filesystem could not materialize, while leaving
   genuinely distinct paths untouched.
"""
from __future__ import annotations
import importlib.util
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_manifest_ordering(freeze) -> list[str]:
    findings: list[str] = []
    names = ["A.md", "a-child.md", "Z.md", "nested/Á.md", "nested/z.md"]
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for name in names:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode("utf-8"))
        rendered = freeze.render(root).decode("utf-8").splitlines()
    order = [line.split("  ", 1)[1] for line in rendered]
    expected = sorted(names, key=lambda value: value.encode("utf-8"))
    if order != expected:
        findings.append(f"ordering is not UTF-8-byte canonical: {order} != {expected}")
    return findings


def test_casefold_collision(audit) -> list[str]:
    findings: list[str] = []
    collisions = audit.casefold_collisions(["Foo.md", "foo.md", "bar.md"])
    if [("Foo.md", "foo.md")] != collisions:
        findings.append(f"expected Foo.md/foo.md collision, got {collisions}")
    unrelated = audit.casefold_collisions(["a.md", "b.md", "nested/a.md"])
    if unrelated:
        findings.append(f"false collision reported on distinct paths: {unrelated}")
    return findings


def main() -> int:
    freeze = load("freeze")
    audit = load("audit")
    findings: list[str] = []
    findings += test_manifest_ordering(freeze)
    findings += test_casefold_collision(audit)
    if findings:
        print(f"selftest: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    print("selftest: PASS (manifest ordering is UTF-8-canonical; casefold collision detected)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

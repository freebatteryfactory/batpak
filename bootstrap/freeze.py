#!/usr/bin/env python3
"""Write or verify the deterministic SHA-256 specification manifest.

Determinism is host-independent by construction:

* Ordering derives ONLY from the UTF-8 bytes of each POSIX relative path.
  It never relies on `Path` object comparison, which is case-sensitive on
  POSIX and case-insensitive on Windows, nor on filesystem enumeration order
  or the host locale.
* The manifest is emitted as UTF-8 with literal LF line endings and exactly
  one trailing newline on every platform (bytes, not text mode).

Standard library only.
"""
from __future__ import annotations
import argparse
import hashlib
from pathlib import Path

EXCLUDE = {"SPEC.sha256"}
EXCLUDE_DIRS = {".git", "target", "__pycache__"}
EXCLUDE_SUFFIXES = {".zip", ".pyc"}


def canonical_path(root: Path, path: Path) -> str:
    """The single canonical key for ordering and identity: POSIX relative path."""
    return path.relative_to(root).as_posix()


def files(root: Path) -> list[tuple[str, Path]]:
    collected: list[tuple[str, Path]] = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        rel = canonical_path(root, path)
        if any(part in EXCLUDE_DIRS for part in relative.parts):
            continue
        if rel in EXCLUDE or any(rel.endswith(suffix) for suffix in EXCLUDE_SUFFIXES):
            continue
        collected.append((rel, path))
    collected.sort(key=lambda item: item[0].encode("utf-8"))
    return collected


def render(root: Path) -> bytes:
    lines = [
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {rel}\n"
        for rel, path in files(root)
    ]
    return "".join(lines).encode("utf-8")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root", nargs="?", default=".")
    ap.add_argument("--check", action="store_true")
    ns = ap.parse_args()
    root = Path(ns.root).resolve()
    expected = render(root)
    target = root / "SPEC.sha256"
    if ns.check:
        actual = target.read_bytes() if target.exists() else b""
        if actual != expected:
            print("freeze: FAIL")
            return 1
        print("freeze: PASS")
        return 0
    target.write_bytes(expected)
    print(f"freeze: wrote {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Write or verify the deterministic SHA-256 specification manifest."""
from __future__ import annotations
import argparse, hashlib
from pathlib import Path

EXCLUDE = {"SPEC.sha256"}
EXCLUDE_DIRS = {".git", "target", "__pycache__"}
EXCLUDE_SUFFIXES = {".zip", ".pyc"}

def files(root: Path):
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        rel_path = path.relative_to(root)
        rel = rel_path.as_posix()
        if any(part in EXCLUDE_DIRS for part in rel_path.parts):
            continue
        if rel in EXCLUDE or any(rel.endswith(suffix) for suffix in EXCLUDE_SUFFIXES):
            continue
        yield rel, path

def render(root: Path) -> str:
    return "".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {rel}\n" for rel, path in files(root))

def main() -> int:
    ap=argparse.ArgumentParser()
    ap.add_argument("root", nargs="?", default=".")
    ap.add_argument("--check", action="store_true")
    ns=ap.parse_args()
    root=Path(ns.root).resolve()
    expected=render(root)
    target=root/"SPEC.sha256"
    if ns.check:
        actual=target.read_text() if target.exists() else ""
        if actual != expected:
            print("freeze: FAIL")
            return 1
        print("freeze: PASS")
        return 0
    target.write_text(expected)
    print(f"freeze: wrote {target}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

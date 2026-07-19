"""Shared selftest infrastructure: paths, receipts, the tool loader, the
guarded-mutation sandbox, and the canonical-validator commitments.

HERE resolves the bootstrap/ directory (the package parent), so every consumer
still sees bootstrap/ and root = HERE.parent still resolves the repo root."""
from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import re
import shutil
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent


def spec_module_source(root: Path, name: str) -> str:
    """The complete source of one spec module: every file of its domain
    directory `spec/<name>/`, sorted (SGB source grammar: a sibling door file
    `spec/<name>.rs` is an unlawful shape and is never read). Whole-module
    reads join through this so the domain-directory grammar keeps every item
    in the selftest harness's view; a missing directory yields the empty
    string, exactly as a missing module did before."""
    pieces: list[str] = []
    pkg = root / "spec" / name
    if pkg.is_dir():
        for rel in sorted(pkg.rglob("*.rs")):
            pieces.append(rel.read_text(encoding="utf-8"))
    return "\n".join(pieces)


# Qualification receipts. NOT one flat status: availability, compilation,
# execution-attempted, and outcome are orthogonal, and flattening them is how a
# gate that was only ever compiled came to sit beside one that actually ran and
# passed. A receipt records each separately, plus the target it holds for and
# the exact reason when something did not happen.
def _toolchain_edition() -> str:
    """The edition every rustc invocation uses, read from the typed owner.

    A literal here would be a hidden Python owner; the audit refuses a
    hardcoded edition argument anywhere in bootstrap (5.5E3a)."""
    src = (HERE.parent / "spec/toolchain/types.rs").read_text(encoding="utf-8")
    m = re.search(r"edition: RustEdition::Rust(\d+)", src)
    return m.group(1) if m else "0"


TOOLCHAIN_EDITION = _toolchain_edition()


QUALIFICATION_RECEIPTS: list[dict] = []


def receipt(name, *, available, compiled=None, executed=None, passed=None,
            target=None, reason=None):
    QUALIFICATION_RECEIPTS.append({
        "name": name, "available": available, "compiled": compiled,
        "executed": executed, "passed": passed, "target": target, "reason": reason,
    })


def render_receipts() -> str:
    """One line per receipt, stage by stage.

    An unavailable or unexecuted check says so here rather than being summarised
    away by an aggregate that prints PASS. A check that was compiled and never
    executed has earned no place beside one that executed and passed.
    """
    lines = []
    for r in QUALIFICATION_RECEIPTS:
        parts = ["available" if r["available"] else "UNAVAILABLE"]
        if r["compiled"] is True:
            parts.append("compiled")
        elif r["compiled"] is False:
            parts.append("COMPILE FAILED")
        if r["executed"] is True:
            parts.append("executed")
        elif r["executed"] is False:
            parts.append("NOT EXECUTED")
        if r["passed"] is True:
            parts.append("passed")
        elif r["passed"] is False:
            parts.append("FAILED")
        if r["target"]:
            parts.append("target " + str(r["target"]))
        if r["reason"]:
            parts.append("reason: " + str(r["reason"]))
        lines.append("selftest: receipt " + r["name"] + ": " + ", ".join(parts))
    return chr(10).join(lines)


def executed_and_passed() -> list[str]:
    """Only a check that actually ran AND passed may be claimed by the banner."""
    return [r["name"] for r in QUALIFICATION_RECEIPTS
            if r["available"] and r["executed"] and r["passed"]]


def load(name: str, base: Path | None = None):
    """Load a bootstrap tool by name, package-agnostic (Wave-2 SW3a).

    A tool is either a single file `<base>/<name>.py` or a package directory
    `<base>/<name>/` whose `__init__.py` re-exports the full public surface;
    the package form is loaded with its own submodule search location so its
    relative imports resolve — no sys.path mutation either way.
    """
    root = base if base is not None else HERE
    package = root / name / "__init__.py"
    if package.is_file():
        spec = importlib.util.spec_from_file_location(
            name, package, submodule_search_locations=[str(root / name)])
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        # A package's relative imports resolve through sys.modules; register
        # for the duration of exec. The WHOLE namespace (the package name and
        # every `name.sub` entry) is snapshotted, cleared, and restored so two
        # loads (e.g. canonical and a neutered copy) can never alias each
        # other's submodules through the import cache.
        prefix = name + "."
        saved = {k: v for k, v in sys.modules.items()
                 if k == name or k.startswith(prefix)}
        for k in saved:
            del sys.modules[k]
        sys.modules[name] = module
        try:
            spec.loader.exec_module(module)
        finally:
            for k in [k for k in sys.modules
                      if k == name or k.startswith(prefix)]:
                del sys.modules[k]
            sys.modules.update(saved)
        return module
    spec = importlib.util.spec_from_file_location(name, root / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def _regen_operator_blocks(project, tmp) -> None:
    """Regenerate every registered generated view in a sandbox from its typed
    sources (5.5E4a: the registry is the one projection denominator), so
    semantic hostiles fail for the intended law and never for a stale
    generated block."""
    findings: list[str] = []
    _views, plans, _bindings = project.build_plans(tmp, findings)
    if findings:
        raise ProbeError(f"sandbox regeneration refused: {findings!r}")
    for path, original, rewritten in plans:
        if rewritten != original:
            path.write_bytes(rewritten.encode("utf-8"))


# --- Guarded mutation: a claimed negative test is itself a claim -------------
# A red light caused by the wrong severed wire is not evidence that the alarm
# works. Every mutation below proves four things: the target existed, the bytes
# actually changed, the intended validator ran, and the EXPECTED finding class
# appeared -- never merely that something somewhere returned nonzero.
class ProbeError(AssertionError):
    """The probe itself is broken: absent target or inert mutation."""


def spec_carrier(root: Path, module: str, needle: str) -> str:
    """Resolve the UNIQUE file of the domain directory `spec/<module>/` that
    carries `needle`, and return its relpath (SGB mutation-probe carrier
    resolution; a sibling door file is an unlawful shape and is never a
    candidate). Fails closed on 0 or 2+ carriers, mirroring
    neutered_validator's package-agnostic search: a needle whose home is
    ambiguous is a probe that no longer knows what it bites."""
    files: list[Path] = []
    pkg = root / "spec" / module
    if pkg.is_dir():
        files.extend(sorted(pkg.rglob("*.rs")))
    carriers = [f for f in files if needle in f.read_text(encoding="utf-8")]
    if len(carriers) != 1:
        raise ProbeError(
            f"spec_carrier({module!r}, {needle[:48]!r}) found {len(carriers)} carriers")
    return carriers[0].relative_to(root).as_posix()


def must_replace(text: str, old: str, new: str, what: str) -> str:
    if old not in text:
        raise ProbeError(f"probe target absent, mutation never applied: {what}")
    out = text.replace(old, new, 1)
    if out == text:
        raise ProbeError(f"probe mutation changed no bytes: {what}")
    return out


def gate_sandbox(edits):
    """A repo copy with guarded edits applied; returns the sandbox root.

    The canonical tracked workspace is never the mutation target (5.5D4a).
    Callers discard the copy in a finally block; `isolated_tree` is the
    context-managed form that cleans up on every exit path.
    """
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
    for sub in ("spec", "docs", "companion"):
        src = root / sub
        if src.is_dir():
            shutil.copytree(src, tmp / sub)
    # The bootstrap sources joined the sandbox in 5.5E4b: the registry-driven
    # projector renders the Tier 0 denominator FROM the harness, and the
    # registry auditor proves dispatcher parity AGAINST the projector.
    shutil.copytree(HERE, tmp / "bootstrap",
                    ignore=shutil.ignore_patterns("__pycache__"))
    # The tracked toolchain projection is part of every tree (5.5E3a): it
    # selects the compiler before the spec compiles, and its absence is a
    # refusal, not a sandbox convenience. The root-level documents joined it
    # in 5.5E4b: they carry registered generated views, so a sandbox that
    # regenerates through the registry needs them present.
    for rel in ("rust-toolchain.toml", "README.md", "AGENTS.md"):
        if (root / rel).is_file():
            shutil.copy2(root / rel, tmp / rel)
    for rel, old, new in edits:
        rel = _resolve_edit_carrier(root, rel, old)
        path = tmp / rel
        text = path.read_text(encoding="utf-8")
        path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
    return tmp


def _resolve_edit_carrier(root: Path, rel: str, old: str) -> str:
    """Door-form edit convention (SGB): an edit aimed at `spec/<name>.rs` (no
    inner slash) is the module-level spelling — no such door file exists in the
    domain-directory grammar, so it resolves to the unique file of the domain
    directory `spec/<name>/**.rs` that carries the needle. Fails closed on an
    ambiguous or absent carrier via spec_carrier. Non-spec paths and
    submodule-form paths pass through unchanged."""
    if not (rel.startswith("spec/") and rel.endswith(".rs")):
        return rel
    module = rel[len("spec/"):-3]
    if "/" in module:
        return rel  # already a submodule path
    if (root / "spec" / module).is_dir():
        return spec_carrier(root, module, old)
    return rel


# --- Hostile-probe isolation (5.5D4a) ---------------------------------------
# A probe timeout once left a disabled rule inside the canonical bootstrap/audit.py.
# Manual restoration worked, but a guard that depends on a later cleanup step is a
# guard that can be left asleep with its eyes painted open. The canonical tracked
# workspace is now never a mutation target: probes mutate an isolated copy, the
# copy is discarded on every exit path including timeout, exception, and
# cancellation, and the suite proves canonical validator bytes are unchanged.
CANONICAL_VALIDATORS = ("audit.py", "project.py", "freeze.py", "selftest.py")


def _commit(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_commitments() -> dict[str, str]:
    """Content commitments for every canonical validator, before any probe runs."""
    return {name: _commit(HERE / name) for name in CANONICAL_VALIDATORS if (HERE / name).is_file()}


def canonical_drift(before: dict[str, str]) -> list[str]:
    """Any canonical validator whose bytes moved is a residue finding, not a pass."""
    out = []
    for name, want in sorted(before.items()):
        got = _commit(HERE / name)
        if got != want:
            out.append(f"canonical validator {name} was modified by the hostile-probe suite")
    return out


@contextlib.contextmanager
def isolated_tree(subdirs=("spec", "docs", "companion")):
    """An isolated copy of the tracked material. Discarded on EVERY exit path:
    success, exception, timeout, assertion failure, or cancellation."""
    root = HERE.parent
    tmp = Path(tempfile.mkdtemp(prefix="batpak-probe-"))
    try:
        for sub in subdirs:
            src = root / sub
            if src.is_dir():
                shutil.copytree(src, tmp / sub)
        if (root / "rust-toolchain.toml").is_file():
            shutil.copy2(root / "rust-toolchain.toml", tmp / "rust-toolchain.toml")
        yield tmp
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


@contextlib.contextmanager
def neutered_validator(name: str, target: str, replacement: str = "if False:"):
    """Load a COPY of a canonical validator with one rule neutered.

    This replaces the retired pattern of patching bootstrap/audit.py in place and
    restoring afterwards. The canonical file is never opened for writing, so a
    timeout mid-probe cannot leave a disabled rule behind.
    """
    # Package-agnostic (Wave-2 SW3a): the mutated literal may live in the
    # single-file tool or in any file of its package directory. The whole tool
    # (shim + package, when one exists) is copied and the one file carrying
    # the target is mutated in the copy; the canonical tree is never written.
    candidates = [HERE / f"{name}.py"]
    if (HERE / name).is_dir():
        candidates.extend(sorted((HERE / name).rglob("*.py")))
    carrier = None
    for candidate in candidates:
        if candidate.is_file() and target in candidate.read_text(encoding="utf-8"):
            carrier = candidate
            break
    if carrier is None:
        raise ProbeError(f"probe target absent in {name}.py: {target!r}")
    before = {c: _commit(c) for c in candidates if c.is_file()}
    source = carrier.read_text(encoding="utf-8")
    mutated = source.replace(target, replacement, 1)
    if mutated == source:
        raise ProbeError(f"probe mutation changed no bytes in {name}.py: {target!r}")
    tmp = Path(tempfile.mkdtemp(prefix="batpak-neuter-"))
    try:
        for candidate in candidates:
            if not candidate.is_file():
                continue
            rel = candidate.relative_to(HERE)
            dest = tmp / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(candidate, dest)
        (tmp / carrier.relative_to(HERE)).write_text(mutated, encoding="utf-8")
        module = load(name, base=tmp)
        yield module
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
        for candidate, digest in before.items():
            if _commit(candidate) != digest:
                raise ProbeError(f"canonical {name}.py was modified during a probe")

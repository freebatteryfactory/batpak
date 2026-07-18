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
import contextlib
import hashlib
import os
import importlib.util
import json
import re
import sys
import shutil
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


# Qualification receipts. NOT one flat status: availability, compilation,
# execution-attempted, and outcome are orthogonal, and flattening them is how a
# gate that was only ever compiled came to sit beside one that actually ran and
# passed. A receipt records each separately, plus the target it holds for and
# the exact reason when something did not happen.
def _toolchain_edition() -> str:
    """The edition every rustc invocation uses, read from the typed owner.

    A literal here would be a hidden Python owner; the audit refuses a
    hardcoded edition argument anywhere in bootstrap (5.5E3a)."""
    src = (HERE.parent / "spec/toolchain.rs").read_text(encoding="utf-8")
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


# The Tier 0 receipt DENOMINATOR left Python entirely in 5.5E6b. The one
# denominator is Tier0ReceiptKind::ALL in spec/bootstrap_qualification.rs.
# selftest produces concrete evidence (produce_tier0_evidence) and the
# independent verifier bootstrap/receiptcheck.rs computes membership, ordering,
# policy, and pass posture from the typed owner, refusing a dishonest receipt
# there. There is no Python admission predicate and no hand-authored
# required-receipt tuple: an all-green row of Python booleans was exactly the
# shape E6b replaced with recomputed concrete evidence.


def _tree_digest(root: Path) -> str:
    """A digest of the typed source snapshot a qualification represents."""
    h = hashlib.sha256()
    for rel in sorted(p.relative_to(root).as_posix()
                      for p in (root / "spec").glob("*.rs")):
        h.update(rel.encode())
        h.update((root / rel).read_bytes())
    for rel in ("bootstrap/seedcheck.rs", "bootstrap/materialize.rs"):
        if (root / rel).is_file():
            h.update(rel.encode())
            h.update((root / rel).read_bytes())
    return h.hexdigest()


def qualify_binary(rustc, root: Path, workdir: Path, name: str, src: str,
                   expect: str, extra: tuple = (),
                   run_args: tuple | None = None, link_spec: bool = True) -> list[str]:
    """Compile, link, and RUN one bootstrap binary, artifact-bound.

    The binding exists because honest individual facts can still assemble into a
    dishonest composite: compile A paired with a stale or substituted binary B is
    a receipt for work that never happened. So the artifact path is unique to this
    run and must not pre-exist, its digest is taken from what THIS compile
    produced, the digest is rechecked immediately before execution, and the source
    snapshot is proven unchanged across the whole qualification.

    Type-checking is not execution, and a matching PASS banner is not a result:
    the process exit and the expected disposition must agree.
    """
    findings: list[str] = []
    before = _tree_digest(root)
    exe = workdir / f"{name}-{before[:12]}" / ("run" + (".exe" if os.name == "nt" else ""))
    exe.parent.mkdir(parents=True, exist_ok=True)
    if exe.exists():
        receipt(name, available=True, compiled=False,
                reason="artifact path already existed; a prior run's binary could be reused")
        return [f"{name}: artifact path was not unique to this run"]
    built = None
    for target in (None, "x86_64-pc-windows-gnu"):
        # The spec is linked as a real rlib (5.5E2); it is built per attempted
        # target so the binary and its library always share one triple. The
        # spec's own test harness cannot extern itself: link_spec=False.
        externs: list[str] = []
        if link_spec:
            rlib = exe.parent / f"libspec-{target or 'host'}.rlib"
            lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                       "--crate-name", "spec", "-o", str(rlib), str(root / "spec/lib.rs")]
            if target:
                lib_cmd[1:1] = ["--target", target]
            if subprocess.run(lib_cmd, capture_output=True, text=True).returncode != 0:
                continue
            externs = ["--extern", f"spec={rlib}"]
        cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", name.replace("-", "_"),
               *externs, *extra, "-o", str(exe), str(root / src)]
        if target:
            cmd[1:1] = ["--target", target]
        if subprocess.run(cmd, capture_output=True, text=True).returncode == 0 and exe.is_file():
            # The receipt names the real triple, never "host default": on a
            # hosted MSVC runner the default-target build IS the authoritative
            # x86_64-pc-windows-msvc qualification, and a receipt that hides
            # the triple cannot be that evidence.
            if target is None:
                m = re.search(r"host: (\S+)",
                              subprocess.run([rustc, "-vV"], capture_output=True,
                                             text=True).stdout)
                target = m.group(1) if m else "host default"
            built = (target, hashlib.sha256(exe.read_bytes()).hexdigest())
            break
    if built is None:
        receipt(name, available=True, compiled=True, executed=False,
                reason="no working linker for any attempted target; "
                       "hosted MSVC owns the authoritative receipt")
        return findings
    target, digest = built
    if not exe.is_file() or hashlib.sha256(exe.read_bytes()).hexdigest() != digest:
        receipt(name, available=True, compiled=True, executed=False,
                reason="the artifact changed between compilation and execution")
        return [f"{name}: the executed artifact is not the one this run compiled"]
    # A libtest harness reads a positional argument as a test FILTER, so the
    # root path would silently filter out every test: run_args exists so a
    # harness binary can run with none.
    proc = subprocess.run(
        [str(exe), *(run_args if run_args is not None else (str(root),))],
        capture_output=True, text=True)
    out = proc.stdout + proc.stderr
    # The exit code and the disposition must agree. Either alone is forgeable: a
    # banner is a string, and an exit code says nothing about what was checked.
    ok = proc.returncode == 0 and expect in out
    after = _tree_digest(root)
    if after != before:
        receipt(name, available=True, compiled=True, executed=True, passed=False,
                target=target, reason="the source snapshot changed during qualification")
        return [f"{name}: the source changed mid-qualification; the receipt binds nothing"]
    receipt(name, available=True, compiled=True, executed=True, passed=ok,
            target=f"{target} artifact sha256:{digest[:12]} source sha256:{before[:12]}")
    if not ok:
        head = "\n".join(out.splitlines()[:8])
        findings.append(f"{name} executed and FAILED ({target}):\n{head}")
    return findings


# --- 5.5E5 isolated Gate-0 materializer qualification ------------------------
# The independent output oracle and the dedicated tier0-materialize route.
# The oracle reconstructs the complete expected candidate -- every path and
# every byte -- from spec/bootstrap_output.rs, spec/architecture.rs, and
# spec/toolchain.rs alone. It never imports the materializer, never invokes a
# materializer rendering function, never reads a generated candidate as its
# expectation, and carries no package, edge, plane, feature, or target-name
# table: every identity below is parsed from the typed owners at run time.
# Duplicating the serialization logic is intentional oracle duplication, not
# a second semantic owner: both implementations consume the same typed facts
# and must agree on exact bytes.

G0_EXCLUDE_DIRS = {".git", "target", "__pycache__"}


def _scan_tree(root: Path, exclude: tuple = ()) -> dict[str, tuple]:
    """Entry-type-aware `lstat` snapshot (5.5E5a): each entry is
    (kind, payload) where kind is 'file'|'dir'|'symlink'|'special'. Symlinks
    are checked BEFORE directories so a same-bytes symlink cannot masquerade
    as its target, and empty directories are visible. This is what lets the
    oracle watch the OBJECT rather than trusting the materializer's own guard."""
    scan: dict[str, tuple] = {}
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root)
        if any(part in exclude for part in rel.parts):
            continue
        relp = rel.as_posix()
        if path.is_symlink():
            try:
                target = os.readlink(str(path))
            except OSError:
                target = "<unreadable>"
            scan[relp] = ("symlink", target)
        elif path.is_dir():
            scan[relp] = ("dir", None)
        elif path.is_file():
            scan[relp] = ("file", path.read_bytes())
        else:
            scan[relp] = ("special", None)
    return scan


def _seed_snapshot(root: Path) -> dict[str, tuple]:
    """The full seed tree with entry kinds and symlink targets. Detects a file
    replaced by a same-bytes symlink and an empty-directory change -- both
    invisible to the limited compile digest (_tree_digest) that covers only
    spec and two bootstrap sources."""
    return _scan_tree(root, exclude=G0_EXCLUDE_DIRS)


def _read_tree(root: Path) -> dict[str, bytes]:
    """File paths -> bytes, for byte comparison and the file-only digest."""
    return {rel: payload for rel, (kind, payload) in _scan_tree(root).items()
            if kind == "file"}


def _inspect_candidate_tree(root: Path, planned: set) -> list[str]:
    """Independent object-type refusal over a published candidate, BEFORE any
    file-only digest (5.5E5a). The materializer has its own symlink and
    extra-directory guards; this inspection repeats them from the outside so a
    neutered materializer guard cannot make the oracle agree with a symlink."""
    out: list[str] = []
    scan = _scan_tree(root)
    implied: set = set()
    for rel in planned:
        acc = ""
        for part in rel.split("/")[:-1]:
            acc = f"{acc}/{part}" if acc else part
            implied.add(acc)
    folded: dict[str, str] = {}
    for rel, (kind, _payload) in sorted(scan.items()):
        if kind == "symlink":
            out.append(f"the candidate carries a symlink at {rel}")
        elif kind == "special":
            out.append(f"the candidate carries a special (non-regular) file at {rel}")
        elif kind == "dir":
            if rel not in implied:
                out.append(f"the candidate carries an extra directory {rel}")
            if rel in planned:
                out.append(f"a directory sits where the planned file {rel} belongs")
        else:  # file
            if rel not in planned:
                out.append(f"the candidate carries an extra file {rel}")
            if rel in implied:
                out.append(f"a file sits where directory {rel} is implied")
        low = rel.lower()
        if low in folded and folded[low] != rel:
            out.append(f"candidate paths {folded[low]} and {rel} collide under case folding")
        folded[low] = rel
    for rel in planned:
        if scan.get(rel, (None,))[0] != "file":
            out.append(f"the candidate is missing the planned file {rel}")
    for d in implied:
        if scan.get(d, (None,))[0] != "dir":
            out.append(f"the candidate is missing the implied directory {d}")
    return out


def _output_tree_digest(tree: dict[str, bytes]) -> str:
    """Canonical output-tree digest: files in UTF-8 path-byte order, each
    entry length-framed (path length, path bytes, content length, content
    bytes) so no concatenation ambiguity exists. Directories never enter."""
    h = hashlib.sha256()
    for rel in sorted(tree, key=lambda r: r.encode("utf-8")):
        p = rel.encode("utf-8")
        h.update(len(p).to_bytes(8, "big"))
        h.update(p)
        h.update(len(tree[rel]).to_bytes(8, "big"))
        h.update(tree[rel])
    return h.hexdigest()


def _g0_uncomment(src: str) -> str:
    return re.sub(r"//[^\n]*", "", src)


def _g0_method(src: str, fn: str) -> str:
    m = re.search(r"pub const fn " + re.escape(fn) + r"\(self\)[^{]*\{(.*?)\n    \}",
                  src, re.S)
    if not m:
        raise AssertionError(f"oracle: spec declares no {fn}()")
    return m.group(1)


def _g0_free_fn(src: str, fn: str) -> str:
    m = re.search(r"pub const fn " + re.escape(fn) + r"\(package: PackageId\)[^{]*\{(.*?)\n\}",
                  src, re.S)
    if not m:
        raise AssertionError(f"oracle: spec declares no {fn}()")
    return m.group(1)


def _g0_all(src: str, enum: str) -> list[str]:
    # A hand-written `pub const ALL` (PackageId, SyncBatPlane) or, for the five
    # closed Gate-0 vocabularies, the `gate0_closed_enum!` variant list that
    # macro-generates the identical inventory (5.5E5a).
    m = re.search(r"pub const ALL: &'static \[" + enum + r"\] = &\[(.*?)\];", src, re.S)
    if m:
        return re.findall(enum + r"::(\w+)", m.group(1))
    head = src.find(f"pub enum {enum} {{")
    if head < 0:
        raise AssertionError(f"oracle: spec declares no {enum}::ALL")
    i = src.find("{", head)
    depth = 0
    body = ""
    for j in range(i, len(src)):
        if src[j] == "{":
            depth += 1
        elif src[j] == "}":
            depth -= 1
            if depth == 0:
                body = src[i + 1: j]
                break
    return re.findall(r"^\s+([A-Z]\w*),", body, re.M)


def oracle_facts(root: Path) -> dict:
    """Every typed fact the oracle consumes, parsed from the owners alone."""
    bo_raw = (root / "spec/bootstrap_output.rs").read_text(encoding="utf-8")
    bo = _g0_uncomment(bo_raw)
    arch = _g0_uncomment((root / "spec/architecture.rs").read_text(encoding="utf-8"))
    tc = _g0_uncomment((root / "spec/toolchain.rs").read_text(encoding="utf-8"))

    facts: dict = {}
    facts["root_artifacts"] = _g0_all(bo, "Gate0RootArtifact")
    facts["root_paths"] = dict(re.findall(
        r'Gate0RootArtifact::(\w+) => "([^"]*)"', _g0_method(bo, "relative_path")))
    facts["kinds"] = dict(re.findall(
        r"PackageId::(\w+) => Gate0PackageTargetKind::(\w+)",
        _g0_free_fn(bo, "target_kind")))
    facts["binary_targets"] = dict(re.findall(
        r'PackageId::(\w+) => Some\("([^"]+)"\)', _g0_free_fn(bo, "binary_target")))
    doors: dict[str, str] = {}
    for pattern, suffix in re.findall(
            r'((?:Gate0PackageTargetKind::\w+\s*\|?\s*)+)=> "([^"]+)"',
            _g0_free_fn(bo, "source_suffix")):
        for kind in re.findall(r"Gate0PackageTargetKind::(\w+)", pattern):
            doors[kind] = suffix
    facts["doors"] = doors
    facts["license"] = re.search(
        r'pub const WORKSPACE_LICENSE: &str = "([^"]*)";', bo_raw).group(1)
    facts["repository"] = re.search(
        r'pub const WORKSPACE_REPOSITORY: &str = "([^"]*)";', bo_raw).group(1)

    facts["package_order"] = _g0_all(arch, "PackageId")
    facts["cargo_names"] = dict(re.findall(
        r'PackageId::(\w+) => "([^"]*)"', _g0_method(arch, "cargo_name")))
    facts["package_paths"] = dict(re.findall(
        r'PackageId::(\w+) => "([^"]*)"', _g0_method(arch, "workspace_path")))
    facts["package_rows"] = {
        pid: {"role": role, "class": cls}
        for pid, role, cls in re.findall(
            r'PackageSpec \{\s*id: PackageId::(\w+),\s*role: "([^"]*)",\s*'
            r"class: PackageClass::(\w+),", arch)
    }
    facts["edges"] = [
        {"importer": imp, "importee": ime, "class": cls, "profile": prof}
        for imp, ime, cls, prof in re.findall(
            r"EdgeSpec \{ importer: PackageId::(\w+), importee: PackageId::(\w+), "
            r'class: EdgeClass::(\w+), profile: "([^"]*)" \}', arch)
    ]
    facts["profiles"] = [
        {"package": pkg, "profile": prof, "environment": env}
        for pkg, prof, env in re.findall(
            r'QualificationProfile \{\s*package: PackageId::(\w+),\s*profile: "([^"]+)",'
            r"\s*environment: QualificationEnvironment::(\w+)", arch)
    ]
    facts["workspace_version"] = re.search(
        r'pub const WORKSPACE_VERSION: &str = "([^"]*)";', arch).group(1)
    facts["planes"] = _g0_all(arch, "SyncBatPlane")
    facts["modules"] = dict(re.findall(
        r'SyncBatPlane::(\w+) => "([^"]*)"', _g0_method(arch, "module_name")))
    facts["ownership"] = dict(re.findall(
        r'SyncBatPlane::(\w+) => "([^"]*)"', _g0_method(arch, "authored_ownership")))
    plane_pkg = re.findall(r"=> PackageId::(\w+),\n        \}",
                           arch[arch.find("pub const fn package(self)"):])
    facts["plane_owner"] = plane_pkg[0] if plane_pkg else ""

    facts["resolver"] = re.search(
        r'CargoResolver::\w+ => "([^"]+)"', tc).group(1)
    facts["edition"] = re.search(
        r'RustEdition::\w+ => "([^"]+)"', tc).group(1)
    floor = re.search(
        r"rust_version_floor: RustVersionFloor \{ major: (\d+), minor: (\d+) \}", tc)
    facts["rust_version"] = f"{floor.group(1)}.{floor.group(2)}"
    facts["no_std"] = sorted({p["package"] for p in facts["profiles"]
                              if p["environment"] == "NoStdAlloc"})
    facts["opt_in"] = {
        pid: [p["profile"] for p in facts["profiles"]
              if p["package"] == pid and p["environment"] not in ("NoStdAlloc", "NativeStd")]
        for pid in facts["package_order"]
    }
    return facts


def _oracle_justfile(f: dict) -> str:
    # Only candidate-valid cargo operations, derived from the typed owners so
    # the oracle carries no command table of its own (5.5E5a). Byte-identical
    # to bootstrap/materialize.rs::justfile().
    out = ["# Discoverability only. Typed command authority moves to TestPak.\n\n"
           "check:\n    cargo check --workspace --all-targets\n\ncheck-no-std:\n"]
    for pid in f["package_order"]:
        if pid in f["no_std"]:
            out.append(f"    cargo check -p {f['cargo_names'][pid]} --no-default-features\n")
    for pid in f["package_order"]:
        if f["kinds"][pid] == "ExampleBinary":
            out.append(f"\nsmoke:\n    cargo run -p {f['cargo_names'][pid]} "
                       f"--bin {f['binary_targets'][pid]}\n")
    return "".join(out)

_G0_LINTS = ("\n[workspace.lints.rust]\nunsafe_op_in_unsafe_fn = \"deny\"\n"
             "unused_must_use = \"deny\"\n\n"
             "[workspace.lints.clippy]\ndbg_macro = \"deny\"\ntodo = \"deny\"\n"
             "unimplemented = \"deny\"\nunwrap_used = \"deny\"\npanic = \"deny\"\n"
             "print_stdout = \"deny\"\nprint_stderr = \"deny\"\n"
             "wildcard_enum_match_arm = \"deny\"\ncast_possible_truncation = \"deny\"\n"
             "cast_sign_loss = \"deny\"\nmissing_errors_doc = \"deny\"\n"
             "large_enum_variant = \"deny\"\nclone_on_ref_ptr = \"deny\"\n"
             "needless_pass_by_value = \"deny\"\n")


def _oracle_toolchain_toml(root: Path) -> str:
    """Reconstruct rust-toolchain.toml from spec/toolchain.rs alone (5.5E5a).
    The oracle must not read the tracked file: that file is itself a generated
    answer, and reading it would let a corrupted projection agree with itself."""
    tc = (root / "spec/toolchain.rs").read_text(encoding="utf-8")
    rel = re.search(r"exact_rust_release: RustRelease \{ major: (\d+), minor: (\d+), "
                    r"patch: (\d+) \}", tc)
    channel = f"{rel.group(1)}.{rel.group(2)}.{rel.group(3)}"
    prof = re.search(r"rustup_profile: RustupProfile::(\w+)", tc).group(1)
    prof_spell = dict(re.findall(r'RustupProfile::(\w+) => "([^"]+)"', tc))[prof]
    comp_all = re.search(r"pub const ALL: &'static \[RustupComponent\] =\s*&\[(.*?)\];",
                         tc, re.S).group(1)
    comps = re.findall(r"RustupComponent::(\w+)", comp_all)
    comp_spell = dict(re.findall(r'RustupComponent::(\w+) => "([^"]+)"', tc))
    comp_list = ", ".join(f'"{comp_spell[c]}"' for c in comps)
    body = (f'[toolchain]\nchannel = "{channel}"\nprofile = "{prof_spell}"\n'
            f'components = [{comp_list}]\n')
    return ("# generated from spec/toolchain.rs by bootstrap/project.py; do not edit\n"
            + body)


def _oracle_workspace_manifest(f: dict) -> str:
    out = ["# Generated from spec/bootstrap_output.rs, spec/architecture.rs, and spec/toolchain.rs by bootstrap/materialize.rs.\n"]
    out.append(f"[workspace]\nresolver = \"{f['resolver']}\"\nmembers = [\n")
    for pid in f["package_order"]:
        out.append(f"  \"{f['package_paths'][pid]}\",\n")
    out.append("]\n\n[workspace.package]\n")
    out.append(f"version = \"{f['workspace_version']}\"\n")
    out.append(f"edition = \"{f['edition']}\"\nrust-version = \"{f['rust_version']}\"\n")
    out.append(f"license = \"{f['license']}\"\nrepository = \"{f['repository']}\"\n")
    out.append("\n[workspace.dependencies]\n")
    for pid in f["package_order"]:
        suffix = ", default-features = false" if pid in f["no_std"] else ""
        out.append(f"{f['cargo_names'][pid]} = {{ path = \"{f['package_paths'][pid]}\"{suffix} }}\n")
    out.append(_G0_LINTS)
    return "".join(out)


def _oracle_package_manifest(f: dict, pid: str) -> str:
    name = f["cargo_names"][pid]
    cls = f["package_rows"][pid]["class"]
    kind = f["kinds"][pid]
    out = ["# Gate-0 skeleton generated from the signed architecture seed.\n[package]\n"]
    out.append(f"name = \"{name}\"\n")
    out.append("version.workspace = true\nedition.workspace = true\n"
               "rust-version.workspace = true\nlicense.workspace = true\n"
               "repository.workspace = true\n")
    if cls in ("DevOnly", "Example"):
        out.append("publish = false\n")
    if kind == "ProcMacroLibrary":
        out.append("\n[lib]\nproc-macro = true\n")
    elif kind == "Binary":
        out.append(f"\n[[bin]]\nname = \"{f['binary_targets'][pid]}\"\n"
                   f"path = \"{f['doors'][kind]}\"\n")
    required = [e for e in f["edges"] if e["importer"] == pid and e["class"] == "Required"]
    optional = [e for e in f["edges"] if e["importer"] == pid and e["class"] == "OptionalProfile"]
    dev = [e for e in f["edges"] if e["importer"] == pid and e["class"] == "DevOnly"]
    if pid in f["no_std"]:
        out.append("\n[features]\ndefault = [\"std\"]\n")
        forwards = [f"\"{f['cargo_names'][e['importee']]}/std\"" for e in required
                    if e["importee"] in f["no_std"]]
        out.append(f"std = [{', '.join(forwards)}]\n")
        for prof in f["opt_in"][pid]:
            out.append(f"{prof} = []\n")
    elif optional:
        out.append("\n[features]\ndefault = []\n")
        for e in optional:
            out.append(f"{e['profile']} = [\"dep:{f['cargo_names'][e['importee']]}\"]\n")

    def dep_suffix(importee: str) -> str:
        if importee in f["no_std"] and pid not in f["no_std"]:
            return ", default-features = true"
        return ""

    if required or optional or (cls == "DevOnly" and dev):
        out.append("\n[dependencies]\n")
        for e in required:
            out.append(f"{f['cargo_names'][e['importee']]} = "
                       f"{{ workspace = true{dep_suffix(e['importee'])} }}\n")
        for e in optional:
            out.append(f"{f['cargo_names'][e['importee']]} = "
                       f"{{ workspace = true, optional = true{dep_suffix(e['importee'])} }}\n")
        if cls == "DevOnly":
            for e in dev:
                out.append(f"{f['cargo_names'][e['importee']]} = "
                           f"{{ workspace = true{dep_suffix(e['importee'])} }}\n")
    if cls != "DevOnly" and dev:
        out.append("\n[dev-dependencies]\n")
        for e in dev:
            out.append(f"{f['cargo_names'][e['importee']]} = "
                       f"{{ workspace = true{dep_suffix(e['importee'])} }}\n")
    out.append("\n[lints]\nworkspace = true\n")
    return "".join(out)


def _oracle_source_door(f: dict, pid: str) -> str:
    name = f["cargo_names"][pid]
    crate = name.replace("-", "_")
    kind = f["kinds"][pid]
    role = f["package_rows"][pid]["role"]
    if kind == "ProcMacroLibrary":
        return (f"#![deny(missing_docs)]\n//! Proc-macro front door for `{name}`.\n//!\n"
                "//! Gate-0 publishes no macros: the contract compiler drives this crate and\n"
                "//! every exported macro lands at its signed gate. A proc-macro crate cannot\n"
                "//! export ordinary items, so this file is intentionally empty.\n")
    if kind == "Binary":
        return f"//! Thin product command adapter for `{name}`.\n\nfn main() {{}}\n"
    if kind == "ExampleBinary":
        return (f"//! Gate-0 placeholder example for `{name}`.\n\n"
                f"fn main() {{\n    println!(\"{name}: gate-0 placeholder\");\n}}\n")
    if pid == f["plane_owner"]:
        modules = sorted(f["modules"][pl] for pl in f["planes"])
        body = "".join(f"pub mod {m};\n" for m in modules)
        return ("#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n"
                "//! SyncBat runtime, PakVM, Bvisor, world, and port planes.\n\n"
                "extern crate alloc;\n\n" + body)
    if pid in f["no_std"]:
        return ("#![cfg_attr(not(feature = \"std\"), no_std)]\n#![deny(missing_docs)]\n"
                "//! Semantic and durable BatPak core.\n\nextern crate alloc;\n\n"
                f"/// Gate-0 marker for the `{crate}` package skeleton.\n"
                f"pub const PACKAGE_ID: &str = \"{name}\";\n")
    _ = role
    return (f"#![deny(missing_docs)]\n//! Gate-0 package skeleton for `{name}`.\n\n"
            "/// Stable package identity used only by the bootstrap skeleton.\n"
            f"pub const PACKAGE_ID: &str = \"{name}\";\n")


def oracle_plan(root: Path) -> dict[str, bytes]:
    """The complete expected candidate: normalized relative path -> bytes."""
    f = oracle_facts(root)
    plan: dict[str, bytes] = {}

    def put(rel: str, text: str) -> None:
        assert rel not in plan, f"oracle: {rel} planned twice"
        plan[rel] = text.encode("utf-8")

    for artifact in f["root_artifacts"]:
        rel = f["root_paths"][artifact]
        if artifact == "WorkspaceManifest":
            put(rel, _oracle_workspace_manifest(f))
        elif artifact == "RustToolchain":
            put(rel, _oracle_toolchain_toml(root))
        elif artifact == "Justfile":
            put(rel, _oracle_justfile(f))
        else:
            raise AssertionError(f"oracle: no renderer for root artifact {artifact}")
    for pid in f["package_order"]:
        base = f["package_paths"][pid]
        role = f["package_rows"][pid]["role"]
        name = f["cargo_names"][pid]
        put(f"{base}/Cargo.toml", _oracle_package_manifest(f, pid))
        put(f"{base}/README.md",
            f"# {name}\n\nGate-0 package skeleton.\n\n**Authority:** {role}\n\n"
            "The normative owner and dependency facts live in `spec/architecture.rs`. "
            "This file does not widen that role.\n")
        put(f"{base}/{f['doors'][f['kinds'][pid]]}", _oracle_source_door(f, pid))
    syncbat_base = f["package_paths"][f["plane_owner"]]
    for plane in f["planes"]:
        module = f["modules"][plane]
        ownership = f["ownership"][plane]
        put(f"{syncbat_base}/src/{module}.rs",
            f"//! SyncBat `{module}` plane: {ownership}.\n\n"
            "/// Gate-0 marker. Semantic types and transitions land only at their signed gate.\n"
            f"pub const PLANE_ID: &str = \"{module}\";\n")
        put(f"{syncbat_base}/src/{module}/README.md",
            f"# `{module}` plane\n\nOwner: {ownership}.\n\n"
            f"The root `{module}.rs` file owns the public module and primary type spine. "
            "This directory holds same-concept implementation files when the owning gate lands.\n")
    return plan


def inspect_gate0_metadata(meta: dict, f: dict) -> list[str]:
    """Semantic inspection of `cargo metadata --no-deps` for the candidate.
    A zero cargo exit is not this inspection: the packages, paths, targets,
    publication posture, dependency endpoints, optional edges, and feature
    wiring are each compared to their typed derivation."""
    out: list[str] = []
    want_names = {f["cargo_names"][pid]: pid for pid in f["package_order"]}
    got = {p["name"]: p for p in meta.get("packages", [])}
    for name in sorted(set(want_names) - set(got)):
        out.append(f"materialized workspace is missing package {name}")
    for name in sorted(set(got) - set(want_names)):
        out.append(f"materialized workspace carries forbidden standalone package {name}")
    for name, pkg in sorted(got.items()):
        pid = want_names.get(name)
        if pid is None:
            continue
        want_path = f["package_paths"][pid]
        manifest = str(pkg.get("manifest_path", "")).replace("\\", "/")
        if not manifest.endswith(f"{want_path}/Cargo.toml"):
            out.append(f"package {name} path drifted from {want_path}")
        cls = f["package_rows"][pid]["class"]
        publish = pkg.get("publish")
        if cls in ("DevOnly", "Example"):
            if publish is None or publish == True:  # noqa: E712 -- metadata uses null/[]/list
                out.append(f"package {name} must carry publish = false")
        elif publish is not None and publish != True:  # noqa: E712
            out.append(f"package {name} must remain publishable")
        kind = f["kinds"][pid]
        targets = {t["name"]: t for t in pkg.get("targets", [])}
        target_kinds = {k for t in pkg.get("targets", []) for k in t.get("kind", [])}
        if kind == "ProcMacroLibrary" and "proc-macro" not in target_kinds:
            out.append(f"package {name} must compile as a proc-macro library")
        if kind in ("Binary", "ExampleBinary"):
            want_bin = f["binary_targets"][pid]
            bins = {t["name"] for t in pkg.get("targets", []) if "bin" in t.get("kind", [])}
            if bins != {want_bin}:
                out.append(f"package {name} binary target must be exactly {want_bin}")
        _ = targets
        want_deps: dict[str, dict] = {}
        for e in f["edges"]:
            if e["importer"] != pid:
                continue
            dep_kind = "dev" if e["class"] == "DevOnly" and cls != "DevOnly" else None
            want_deps[f["cargo_names"][e["importee"]]] = {
                "optional": e["class"] == "OptionalProfile", "kind": dep_kind}
        got_deps = {d["name"]: d for d in pkg.get("dependencies", [])}
        for dep in sorted(set(want_deps) - set(got_deps)):
            out.append(f"package {name} is missing declared edge to {dep}")
        for dep in sorted(set(got_deps) - set(want_deps)):
            out.append(f"package {name} carries undeclared edge to {dep}")
        for dep, want in sorted(want_deps.items()):
            gotd = got_deps.get(dep)
            if not gotd:
                continue
            if bool(gotd.get("optional")) != want["optional"]:
                out.append(f"package {name} edge to {dep} has the wrong optional posture")
            if want["kind"] == "dev" and gotd.get("kind") != "dev":
                out.append(f"package {name} dev-only edge to {dep} leaks into production")
        examples_pid = pid if cls == "Example" else None
        if examples_pid:
            dev_only_names = {f["cargo_names"][q]
                              for q in f["package_order"]
                              if f["package_rows"][q]["class"] == "DevOnly"}
            for dep in got_deps:
                if dep in dev_only_names:
                    out.append(f"package {name} depends on dev tooling {dep}")
        features = pkg.get("features", {})
        if pid in f["no_std"]:
            if features.get("default") != ["std"] or "std" not in features:
                out.append(f"package {name} std feature wiring drifted")
            for prof in f["opt_in"][pid]:
                if prof not in features:
                    out.append(f"package {name} names no {prof} opt-in feature "
                               "for its admitted qualification profile")
    return out


def _g0_cargo_command(exe_triple: str) -> list[str] | None:
    """A cargo invocation whose HOST matches the qualified binary's triple, so
    the workspace (and its proc-macro DLLs) link with the same toolchain the
    receipt binds. Returns None when no matching toolchain exists here; the
    hosted lane owns the authoritative receipt in that case."""
    cargo = shutil.which("cargo")
    rustc = shutil.which("rustc")
    if not cargo or not rustc:
        return None
    host = re.search(r"host: (\S+)",
                     subprocess.run([rustc, "-vV"], capture_output=True,
                                    text=True).stdout)
    if host and host.group(1) == exe_triple:
        return [cargo]
    listing = subprocess.run(["rustup", "toolchain", "list"],
                             capture_output=True, text=True)
    if listing.returncode == 0:
        for line in listing.stdout.split():
            if line.endswith(exe_triple):
                return [cargo, f"+{line}"]
    return None


def qualify_materializer(rustc, root: Path, workdir: Path) -> list[str]:
    """The dedicated tier0-materialize qualification (5.5E5).

    The generic qualify_binary route ran the materializer with the live seed
    root as its only argument -- which is how a workspace scaffold appeared
    inside the signed seed with every receipt green. This route materializes
    into TWO independent temporary roots outside the seed, proves the full
    seed snapshot unchanged, byte-compares both trees against the independent
    oracle, proves the second disposition Unchanged, qualifies the workspace
    with Cargo on a disposable build copy, and records the canonical output
    tree digest as receipt evidence. Printing PASS earns nothing here.
    """
    name = "tier0-materialize"
    findings: list[str] = []
    before = _tree_digest(root)
    exe = workdir / f"{name}-{before[:12]}" / ("run" + (".exe" if os.name == "nt" else ""))
    exe.parent.mkdir(parents=True, exist_ok=True)
    if exe.exists():
        receipt(name, available=True, compiled=False,
                reason="artifact path already existed; a prior run's binary could be reused")
        return [f"{name}: artifact path was not unique to this run"]
    built = None
    for target in (None, "x86_64-pc-windows-gnu"):
        rlib = exe.parent / f"libspec-{target or 'host'}.rlib"
        lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                   "--crate-name", "spec", "-o", str(rlib), str(root / "spec/lib.rs")]
        if target:
            lib_cmd[1:1] = ["--target", target]
        if subprocess.run(lib_cmd, capture_output=True, text=True).returncode != 0:
            continue
        cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", "materialize",
               "--extern", f"spec={rlib}", "-o", str(exe),
               str(root / "bootstrap/materialize.rs")]
        if target:
            cmd[1:1] = ["--target", target]
        if subprocess.run(cmd, capture_output=True, text=True).returncode == 0 and exe.is_file():
            if target is None:
                m = re.search(r"host: (\S+)",
                              subprocess.run([rustc, "-vV"], capture_output=True,
                                             text=True).stdout)
                target = m.group(1) if m else "host default"
            built = (target, hashlib.sha256(exe.read_bytes()).hexdigest())
            break
    if built is None:
        receipt(name, available=True, compiled=True, executed=False,
                reason="no working linker for any attempted target; "
                       "hosted MSVC owns the authoritative receipt")
        return findings
    target, digest = built

    snap_before = _seed_snapshot(root)
    out_a = workdir / "g0-candidate-a"
    out_b = workdir / "g0-candidate-b"

    def run_mat(output: Path) -> subprocess.CompletedProcess:
        return subprocess.run(
            [str(exe), "--seed", str(root), "--output", str(output)],
            capture_output=True, text=True)

    ok = True
    for output in (out_a, out_b):
        proc = run_mat(output)
        if proc.returncode != 0 or "materialize: PASS Created" not in proc.stdout:
            findings.append(f"{name}: isolated materialization into {output.name} "
                            f"did not report Created:\n"
                            + "\n".join((proc.stdout + proc.stderr).splitlines()[:6]))
            ok = False
    snap_after = _seed_snapshot(root)
    if snap_after != snap_before:
        changed = sorted(set(snap_before) ^ set(snap_after))[:8] or ["(byte changes)"]
        findings.append(f"{name}: the materializer qualification mutated the seed "
                        f"tree: {', '.join(changed)}")
        ok = False

    tree_digest_hex = ""
    if ok:
        expected = oracle_plan(root)
        # Independent object-type inspection BEFORE the file-only digest: a
        # symlink, special file, or stray directory is refused here even if the
        # materializer's own guard were removed.
        for label, out_root in (("a", out_a), ("b", out_b)):
            for msg in _inspect_candidate_tree(out_root, set(expected)):
                findings.append(f"{name}: output {label} {msg}")
                ok = False
        tree_a = _read_tree(out_a)
        tree_b = _read_tree(out_b)
        for label, tree in (("a", tree_a), ("b", tree_b)):
            if tree != expected:
                missing = sorted(set(expected) - set(tree))[:4]
                extra = sorted(set(tree) - set(expected))[:4]
                diff = sorted(r for r in set(tree) & set(expected)
                              if tree[r] != expected[r])[:4]
                findings.append(
                    f"{name}: output {label} does not match the independent oracle "
                    f"(missing {missing}, extra {extra}, differing {diff})")
                ok = False
        if tree_a != tree_b:
            findings.append(f"{name}: the two isolated outputs are not byte-identical")
            ok = False
        rerun = run_mat(out_a)
        if rerun.returncode != 0 or "materialize: PASS Unchanged" not in rerun.stdout:
            findings.append(f"{name}: a second materialization against the exact "
                            "output did not report Unchanged")
            ok = False
        if _read_tree(out_a) != tree_a:
            findings.append(f"{name}: the second materialization rewrote the output")
            ok = False
        tree_digest_hex = _output_tree_digest(tree_a)

    if ok:
        cargo = _g0_cargo_command(target)
        if cargo is None:
            receipt(name, available=True, compiled=True, executed=True, passed=False,
                    target=target,
                    reason=f"no cargo toolchain matches {target}; the hosted lane "
                           "owns the authoritative workspace qualification")
            return findings + [f"{name}: the workspace qualification could not run "
                               f"(no cargo for {target})"]
        build = workdir / "g0-build"
        shutil.copytree(out_a, build)
        # The receipt binds a physical target triple, so every target-sensitive
        # command carries an explicit --target and ambient target selection is
        # cleared: a green "host is MSVC" is not proof the compilation was not
        # redirected by CARGO_BUILD_TARGET (5.5E5a).
        env = {k: v for k, v in os.environ.items()
               if k not in ("CARGO_BUILD_TARGET",)}

        def cargo_run(*args: str) -> subprocess.CompletedProcess:
            return subprocess.run([*cargo, *args], cwd=build, env=env,
                                  capture_output=True, text=True)

        meta_proc = cargo_run("metadata", "--no-deps", "--format-version", "1")
        if meta_proc.returncode != 0:
            findings.append(f"{name}: cargo metadata refused the candidate:\n"
                            + "\n".join(meta_proc.stderr.splitlines()[:6]))
            ok = False
        else:
            facts = oracle_facts(root)
            findings.extend(f"{name}: {msg}" for msg in
                            inspect_gate0_metadata(json.loads(meta_proc.stdout), facts))
            examples_pid = next((pid for pid in facts["package_order"]
                                 if facts["package_rows"][pid]["class"] == "Example"), None)
            checks = [("check", "--target", target, "--workspace", "--all-targets")]
            for pid in facts["no_std"]:
                checks.append(("check", "--target", target, "-p",
                               facts["cargo_names"][pid], "--no-default-features"))
            for args in checks:
                proc = cargo_run(*args)
                if proc.returncode != 0:
                    findings.append(f"{name}: cargo {' '.join(args)} failed:\n"
                                    + "\n".join(proc.stderr.splitlines()[-6:]))
                    ok = False
            if examples_pid:
                bin_name = facts["binary_targets"][examples_pid]
                proc = cargo_run("run", "-q", "--target", target, "-p",
                                 facts["cargo_names"][examples_pid], "--bin", bin_name)
                want_line = f"{facts['cargo_names'][examples_pid]}: gate-0 placeholder"
                if proc.returncode != 0 or want_line not in proc.stdout:
                    findings.append(f"{name}: the example binary emitted no observable "
                                    f"Gate-0 line for target {target} (wanted "
                                    f"{want_line!r})")
                    ok = False
                elif not (build / "target" / target).is_dir():
                    findings.append(f"{name}: no target-specific build directory "
                                    f"target/{target} appeared; the qualification "
                                    "was not bound to its triple")
                    ok = False
        if _seed_snapshot(root) != snap_before:
            findings.append(f"{name}: the workspace qualification mutated the seed tree")
            ok = False
        if (out_a / "Cargo.lock").exists() or _read_tree(out_a) != _read_tree(out_b):
            findings.append(f"{name}: cargo qualification touched the qualified "
                            "candidate; it must run on a disposable copy")
            ok = False

    ok = ok and not findings
    receipt(name, available=True, compiled=True, executed=True, passed=ok,
            target=f"{target} artifact sha256:{digest[:12]} source sha256:{before[:12]}",
            reason=(f"output tree sha256:{tree_digest_hex[:12]}" if tree_digest_hex else None))
    return findings


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


def test_stale_vocabulary(audit) -> list[str]:
    """Hostile fixtures for the decision-derived stale-vocabulary matcher."""
    findings: list[str] = []
    amap = {"filebat": ("DEC-003", frozenset({"DecisionLedger", "LegacyEvidence"}), "FileBat")}

    def scan(text: str, context: str) -> list[str]:
        out: list[str] = []
        audit.scan_stale_occurrences("synthetic.md", text, context, amap, out)
        return out

    if not scan("FileBat is superseded", "OrdinaryAuthoritative"):
        findings.append("stale_alias_in_authoritative_doc_is_rejected FAILED")
    if not scan("let handle = FileBat::open();", "ProductionSource"):
        findings.append("stale_alias_in_production_source_is_rejected FAILED")
    if not scan("FileBat [STALE-REF: DEC-011]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_wrong_decision_reference_is_rejected FAILED")
    if scan("FileBat [STALE-REF: DEC-003]", "OrdinaryAuthoritative"):
        findings.append("stale_alias_with_killed_decision_reference_is_allowed FAILED")
    if not scan("plain text [STALE-REF: DEC-999]", "OrdinaryAuthoritative"):
        findings.append("unknown_stale_reference_is_rejected FAILED")
    if scan("FileBat appears in the ledger", "DecisionLedger"):
        findings.append("stale_alias_in_allowed_context_passes FAILED")
    return findings


def test_stale_derivation(audit, root) -> list[str]:
    """The matcher is derived from spec/dispositions.rs, not a hand list."""
    findings: list[str] = []
    problems: list[str] = []
    amap = audit.parse_stale_vocabulary(root, problems)
    if problems:
        findings.append(f"parse_stale_vocabulary reported problems on the real seed: {problems}")
    if len(amap) < 20:
        findings.append(f"generated_stale_table_must_match_decisions: derived only {len(amap)} aliases")
    owner = amap.get("filebat")
    if not owner or owner[0] != "DEC-003":
        findings.append("FileBat alias is not derived to its killing decision DEC-003")
    if "made-up-stale-term" in amap:
        findings.append("alias_removed_from_decision_disappears_from_all_views FAILED")

    # 5.5E2: the context vocabulary is closed in BOTH directions — the enum
    # declares exactly what the scanner speaks, and the two never-permissive
    # defaults cannot be allow-listed by a row.
    def context_probe(name, old, new, needle):
        tmp = gate_sandbox([("spec/dispositions.rs", old, new)])
        try:
            problems: list[str] = []
            audit.parse_stale_vocabulary(tmp, problems)
            if not any(needle in p for p in problems):
                findings.append(f"{name} FAILED (wanted {needle!r}, got {problems!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    context_probe(
        "allow_listing_production_source_is_rejected",
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        'StaleContext::SupersessionGuide, StaleContext::LegacyEvidence], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "stale_allowed_contexts: &[StaleContext::DecisionLedger, StaleContext::RejectionRecord, "
        "StaleContext::SupersessionGuide, StaleContext::LegacyEvidence, "
        'StaleContext::ProductionSource], replacement_contract: '
        'Some(".fbat remains the BatPak-owned journal format")',
        "allow-lists the default context ProductionSource")
    context_probe(
        "dropping_a_default_context_variant_is_rejected",
        "    /// DEFAULT context for every other authoritative surface. Never permissive.\n"
        "    OrdinaryAuthoritative,\n",
        "",
        "the scanner's closed vocabulary")
    context_probe(
        "reintroducing_a_dead_context_variant_is_rejected",
        "    MigrationCompatibility,\n",
        "    MigrationCompatibility,\n    TestFixture,\n",
        "the scanner's closed vocabulary")
    return findings


def test_legacy_manifest_parity(audit) -> list[str]:
    """Hostile fixtures for the D-1 declaration-parity denominator.

    The denominator is measured against declared invariant IDs, so a structural
    key that merely resembles an invariant name (the generated INV-CATALOG
    marker) must never be counted as a declaration.
    """
    findings: list[str] = []
    base = ["INV-A", "INV-B", "INV-C"]
    check = audit.legacy_manifest_findings

    if check(base, base, 3):
        findings.append("exact_declaration_parity_should_pass FAILED")
    if not check(base + ["INV-CATALOG"], base, 3):
        findings.append("structural_INV_token_counted_as_declaration_is_rejected FAILED")
    if not check(base, ["INV-A", "INV-B"], 3):
        findings.append("declared_invariant_without_coverage_is_rejected FAILED")
    if not check(base, base + ["INV-GHOST"], 3):
        findings.append("coverage_for_undeclared_invariant_is_rejected FAILED")
    if not check(base, base, 4):
        findings.append("denominator_count_mismatch_is_rejected FAILED")
    if not check(base + ["INV-A"], base, 3):
        findings.append("duplicate_declaration_id_is_rejected FAILED")
    return findings


def _sample_operators() -> list[dict[str, str]]:
    """Resolved operator rows in the typed 5.5E3j shape (canonical/alias from
    OperatorSyntax; no word_surface, symbol_surface, or formatting fields)."""
    base = {
        "semantic_op": "s", "input_sorts": "a", "result_sort": "b", "overflow": "o",
        "exception": "e", "mutation_classes": "m", "numeric_support": "ExactSupported",
    }
    return [
        {**base, "id": "OP-ADD", "class": "Arithmetic", "shape": "SymbolOnly",
         "canonical": "+", "alias": "",
         "arity": "Binary", "fixity": "Infix", "precedence": "60", "semantic_op": "add",
         "typing": "OperatorTypingRule::PercentAdjustment, OperatorTypingRule::SameUnit",
         "associativity": "Left", "exactness": "Exact", "spoken": "plus"},
        {**base, "id": "OP-EQ", "class": "Comparison", "shape": "WordWithSymbolAlias",
         "canonical": "IS", "alias": "=",
         "arity": "Binary", "fixity": "Infix", "precedence": "50",
         "typing": "OperatorTypingRule::SameSortComparison",
         "associativity": "NonAssociative", "exactness": "Exact", "spoken": "is"},
    ]


def test_batql(audit, project) -> list[str]:
    """Named hostile fixtures for the BatQL operator/grammar/proof surfaces."""
    findings: list[str] = []
    ops = _sample_operators()
    companion = (HERE.parent / "companion/BATQL_LANGUAGE.md").read_text(encoding="utf-8")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # 1 & 2: proof-disposition axis must keep the exact five states
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("missing_legacy_weak_on_authoritative_surface_is_rejected")
    if not audit.batql_proof_disposition_findings(["VERIFIED", "UNVERIFIED", "UNVERIFIED", "PROOF_UNAVAILABLE", "PROOF_INVALID"]):
        fail("legacy_weak_collapsed_by_projection_is_rejected")

    # 3 & 4: grammar reference closure and unique ownership
    undefined, _, _ = audit.batql_grammar_closure_findings(["foo\n  := bar baz"])
    if not undefined:
        fail("undefined_grammar_leaf_is_rejected")
    dup, _, _ = audit.batql_grammar_closure_findings(['foo\n  := "x"\n\nfoo\n  := "y"'])
    if not any("more than one owning" in item for item in dup):
        fail("duplicate_grammar_owner_is_rejected")

    # 5: MATCH requires an initial step plus at least one THEN step
    if "match_step (THEN match_step)+" not in companion or "match_step (THEN match_step)*" in companion:
        fail("one_step_match_is_rejected")

    # 6: kernel declaration clauses appear once, in canonical order
    order = ["FROM KERNEL kernel_ref", "purity_clause", "determinism_clause", "capability_clause", "cost_clause"]
    positions = [companion.find(token) for token in order]
    if -1 in positions or positions != sorted(positions):
        fail("reordered_or_duplicated_kernel_clauses_are_rejected")

    # 7 & 8 & 9: operator fact discipline
    if not any("duplicate OperatorId" in x for x in audit.batql_operator_fact_findings(ops + [dict(ops[0])])):
        fail("duplicate_operator_id_is_rejected")
    clash = {**ops[0], "id": "OP-ADD-CLONE"}
    if not any("claimed by" in x for x in audit.batql_operator_fact_findings([ops[0], clash])):
        fail("ambiguous_token_plus_fixity_ownership_is_rejected")
    missing_field = {**ops[0], "spoken": ""}
    if not any("missing field spoken" in x for x in audit.batql_operator_fact_findings([missing_field])):
        fail("missing_required_operatorspec_field_is_rejected")

    # 10, 11, 12: projection parity is sensitive to alteration, missing, and extra rows
    expected = audit.batql_render_catalog(ops)
    if expected == expected.replace("Exact", "EXACT", 1):
        fail("altered_generated_operator_projection_is_rejected")
    if expected == audit.batql_render_catalog([ops[0]]):
        fail("missing_generated_operator_row_is_rejected")
    if expected == audit.batql_render_catalog(ops + [{**ops[0], "id": "OP-XTRA"}]):
        fail("extra_generated_operator_row_is_rejected")

    # 13: canonical order comes from OperatorId::ALL, so generated output
    # FOLLOWS row order — a reorder is visible, never silently re-sorted by
    # a Python ranking table (5.5E3j; the CLASS_RANK maps are dead).
    if audit.batql_render_catalog(ops) == audit.batql_render_catalog(list(reversed(ops))):
        fail("generated_output_follows_operator_id_all_order")

    # Typed legality rules (5.5E1): the rules are the operand/result authority,
    # their placement is law, and the two phantom sorts stay dead.
    if not any("declares no typing rules" in x
               for x in audit.batql_operator_fact_findings([{**ops[0], "typing": ""}])):
        fail("operator_without_typing_rules_is_rejected")
    if not any("only subtraction" in x for x in audit.batql_operator_fact_findings(
            [{**ops[0], "typing": "OperatorTypingRule::WallObservationDifference"}])):
        fail("wall_difference_outside_subtraction_is_rejected")
    if not any("must carry exactly" in x for x in audit.batql_operator_fact_findings(
            [{**ops[1], "typing": "OperatorTypingRule::TruthUnary"}])):
        fail("comparison_with_foreign_typing_rule_is_rejected")
    if audit.phantom_sort_findings(HERE.parent):
        fail("phantom_sort_baseline_is_silent")
    with isolated_tree() as tmp:
        p = tmp / "companion/BATQL_LANGUAGE.md"
        p.write_text(p.read_text(encoding="utf-8")
                     + "\nDecimalMoney is back.\n", encoding="utf-8")
        if not any("phantom sort" in x for x in audit.phantom_sort_findings(tmp)):
            fail("a_phantom_sort_cannot_reenter_the_corpus")
    # A mutated rule regenerates a DIFFERENT matrix: the block is semantic, not
    # decorative. Dropping the percent-difference rule must change the render.
    stripped = {**ops[0], "id": "OP-SUB", "canonical": "-", "semantic_op": "subtract",
                "typing": "OperatorTypingRule::SameUnit"}
    ruled = {**stripped,
             "typing": "OperatorTypingRule::PercentDifference, OperatorTypingRule::SameUnit"}
    if audit.batql_render_typing([stripped]) == audit.batql_render_typing([ruled]):
        fail("typing_rule_mutation_changes_the_generated_matrix")

    # 14: the independent generator and auditor projections must agree
    if (
        project.render_catalog(ops) != audit.batql_render_catalog(ops)
        or project.render_surfaces(ops) != audit.batql_render_surfaces(ops)
        or project.render_grammar(ops) != audit.batql_render_grammar(ops)
        or project.render_projection(ops) != audit.batql_render_projection(ops)
        or project.render_numeric(ops) != audit.batql_render_numeric(ops)
    ):
        fail("generator_auditor_disagreement_turns_gate_red")

    # 15: FLOOR/CEILING without the language-change record turns the gate red
    modes = "rounding_mode\n  := HALF_EVEN | HALF_UP | DOWN | UP | FLOOR | CEILING\n"
    if audit.batql_language_change_findings(modes + "\nLanguage-change record (BatQL V1)\n"):
        fail("language_change_record_present_passes")
    if not audit.batql_language_change_findings(modes):
        fail("floor_ceiling_without_language_change_record_is_rejected")

    return findings


def _sample_operator_model() -> dict:
    """A minimal coherent typed operator model (one operator per class) in the
    exact shape audit.batql_parse_operator_model produces."""
    row = {
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix",
        "precedence": "50", "associativity": "Left",
        "typing": "OperatorTypingRule::SameSortComparison", "input_sorts": "a",
        "result_sort": "b", "exactness": "Exact", "overflow": "o",
        "exception": "e", "spoken": "sp", "mutation_classes": "m",
        "numeric_support": "ExactSupported",
    }
    ids = ("Multiply", "Equal", "Not")
    return {
        "src": "",
        "id_type": "OperatorId",
        "id_variants": list(ids), "id_all": list(ids),
        "id_token": {"Multiply": "OP-MUL", "Equal": "OP-EQ", "Not": "OP-NOT"},
        "id_owner": {v: "BP-BATQL-LANGUAGE-1" for v in ids},
        "id_basis": {v: "DEC-060" for v in ids},
        "word_type": "OperatorWordSurface",
        "word_variants": ["Is", "Not"], "word_all": ["Is", "Not"],
        "word_token": {"Is": "IS", "Not": "NOT"},
        "word_owner": {"Is": "BP-BATQL-LANGUAGE-1", "Not": "BP-BATQL-LANGUAGE-1"},
        "word_basis": {"Is": "DEC-060", "Not": "DEC-060"},
        "symbol_type": "OperatorSymbolSurface",
        "symbol_variants": ["Multiply", "Equal"], "symbol_all": ["Multiply", "Equal"],
        "symbol_token": {"Multiply": "*", "Equal": "="},
        "symbol_owner": {"Multiply": "BP-BATQL-LANGUAGE-1", "Equal": "BP-BATQL-LANGUAGE-1"},
        "symbol_basis": {"Multiply": "DEC-060", "Equal": "DEC-060"},
        "rows": [
            {**row, "variant": "Multiply", "class": "Arithmetic",
             "shape": "SymbolOnly", "inner": "OperatorSymbolSurface::Multiply",
             "semantic_op": "multiply",
             "typing": "OperatorTypingRule::DimensionalByDimensionless"},
            {**row, "variant": "Equal", "class": "Comparison",
             "shape": "WordWithSymbolAlias",
             "inner": "OperatorWordSurface::Is, OperatorSymbolSurface::Equal",
             "associativity": "NonAssociative"},
            {**row, "variant": "Not", "class": "Logical", "shape": "WordOnly",
             "inner": "OperatorWordSurface::Not", "fixity": "Prefix",
             "arity": "Unary", "typing": "OperatorTypingRule::TruthUnary",
             "numeric_support": "NotApplicable"},
        ],
    }


def test_operator_surfaces(audit, project) -> list[str]:
    """Named hostile fixtures for the typed operator identity and
    source-surface closure (5.5E3j): OperatorId, the closed word and symbol
    surface inventories, OperatorSyntax, the class/shape law, the DEC-060
    admission boundary, projection provenance, and the retired raw fields."""
    findings: list[str] = []
    root = HERE.parent
    CONTRACTS = {"BP-BATQL-LANGUAGE-1"}
    DECISIONS = {"DEC-060"}

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def mfind(model) -> list[str]:
        return audit.batql_operator_model_findings(model)

    def rfind(rows) -> list[str]:
        return audit.batql_operator_fact_findings(rows)

    def resolved(model) -> list[dict]:
        return audit.batql_resolve_operator_rows(model, [])

    def batql(tmp) -> list[str]:
        out: list[str] = []
        audit.check_batql(tmp, out)
        return out

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # The synthetic model itself is lawful; a red baseline would let every
    # hostile below pass for the wrong reason.
    base = _sample_operator_model()
    baseline = mfind(base) + rfind(resolved(base)) \
        + audit.batql_operator_authority_findings(base, CONTRACTS, DECISIONS)
    if baseline:
        fail(f"typed_operator_model_baseline_is_red ({baseline!r})")

    # Identity/row parity in OperatorId::ALL order.
    m = _sample_operator_model()
    m["id_all"] = ["Multiply", "Equal"]
    expect("operator_id_missing_from_all_is_rejected", mfind(m),
           "is declared but missing from OperatorId::ALL")
    m = _sample_operator_model()
    m["rows"] = m["rows"][:2]
    expect("operator_id_without_operator_row_is_rejected", mfind(m),
           "OperatorId::Not has no OperatorSpec row")
    m = _sample_operator_model()
    m["rows"] = m["rows"] + [dict(m["rows"][0])]
    expect("duplicate_operator_id_row_is_rejected", mfind(m),
           "duplicate OperatorSpec row for OperatorId::Multiply")
    m = _sample_operator_model()
    m["rows"] = list(reversed(m["rows"]))
    expect("operator_row_order_must_equal_operator_id_all", mfind(m),
           "OPERATORS row order does not equal OperatorId::ALL")

    # The closed surface inventories.
    m = _sample_operator_model()
    m["word_all"] = ["Is"]
    expect("operator_word_surface_missing_from_all_is_rejected", mfind(m),
           "missing from OperatorWordSurface::ALL")
    m = _sample_operator_model()
    m["symbol_all"] = ["Multiply"]
    expect("operator_symbol_surface_missing_from_all_is_rejected", mfind(m),
           "missing from OperatorSymbolSurface::ALL")
    m = _sample_operator_model()
    for key in ("word_variants", "word_all"):
        m[key] = m[key] + ["Or"]
    m["word_token"]["Or"] = "OR"
    m["word_owner"]["Or"] = "BP-BATQL-LANGUAGE-1"
    m["word_basis"]["Or"] = "DEC-060"
    expect("orphan_operator_word_surface_is_rejected", mfind(m),
           "word surface Or is adopted by no OperatorSpec row")
    m = _sample_operator_model()
    for key in ("symbol_variants", "symbol_all"):
        m[key] = m[key] + ["Add"]
    m["symbol_token"]["Add"] = "+"
    m["symbol_owner"]["Add"] = "BP-BATQL-LANGUAGE-1"
    m["symbol_basis"]["Add"] = "DEC-060"
    expect("orphan_operator_symbol_surface_is_rejected", mfind(m),
           "symbol surface Add is adopted by no OperatorSpec row")
    m = _sample_operator_model()
    m["word_token"]["Not"] = "IS"
    expect("duplicate_operator_word_token_is_rejected", mfind(m),
           "duplicate OperatorWordSurface token 'IS'")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = "*"
    expect("duplicate_operator_symbol_token_is_rejected", mfind(m),
           "duplicate OperatorSymbolSurface token '*'")
    m = _sample_operator_model()
    m["word_token"]["Not"] = ""
    expect("empty_operator_word_token_is_rejected", mfind(m),
           "OperatorWordSurface::Not declares an empty token")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = ""
    expect("empty_operator_symbol_token_is_rejected", mfind(m),
           "OperatorSymbolSurface::Equal declares an empty token")
    m = _sample_operator_model()
    m["word_token"]["Not"] = "!"
    expect("punctuation_cannot_enter_the_word_inventory", mfind(m),
           "punctuation can never enter the word inventory")
    m = _sample_operator_model()
    m["symbol_token"]["Equal"] = "EQ"
    expect("a_word_cannot_enter_the_symbol_inventory", mfind(m),
           "a word can never enter the symbol inventory")

    # Owner and basis resolution across all three families.
    m = _sample_operator_model()
    m["id_owner"]["Multiply"] = "BP-NOBODY-1"
    expect("operator_id_with_unknown_owner_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "names owner BP-NOBODY-1, which no declared contract owns")
    m = _sample_operator_model()
    m["word_owner"]["Is"] = "BP-NOBODY-1"
    expect("operator_surface_with_unknown_owner_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "OperatorWordSurface::Is names owner BP-NOBODY-1")
    m = _sample_operator_model()
    m["id_basis"]["Multiply"] = "DEC-999"
    expect("operator_id_with_dangling_basis_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "admission basis DEC-999, which no declared decision owns")
    m = _sample_operator_model()
    m["symbol_basis"]["Equal"] = "DEC-999"
    expect("operator_surface_with_dangling_basis_is_rejected",
           audit.batql_operator_authority_findings(m, CONTRACTS, DECISIONS),
           "OperatorSymbolSurface::Equal names admission basis DEC-999")

    # The class/shape law is total.
    rows = resolved(_sample_operator_model())
    arithmetic, comparison, logical = rows[0], rows[1], rows[2]
    expect("arithmetic_operator_must_be_symbol_only",
           rfind([{**arithmetic, "shape": "WordWithSymbolAlias", "alias": "*",
                   "canonical": "TIMES"}]),
           "its class law requires SymbolOnly")
    expect("comparison_operator_must_be_word_with_symbol_alias",
           rfind([{**comparison, "shape": "WordOnly", "alias": ""}]),
           "its class law requires WordWithSymbolAlias")
    expect("comparison_symbol_cannot_become_canonical",
           rfind([{**comparison, "shape": "SymbolOnly", "canonical": "=", "alias": ""}]),
           "its class law requires WordWithSymbolAlias")
    expect("logical_operator_must_be_word_only",
           rfind([{**logical, "shape": "SymbolOnly", "canonical": "!"}]),
           "its class law requires WordOnly")
    expect("logical_operator_cannot_gain_a_symbol_alias",
           rfind([{**logical, "shape": "WordWithSymbolAlias", "alias": "!"}]),
           "its class law requires WordOnly")

    # Surface/fixity ownership: one surface under one fixity names one
    # operator; the same surface under a DISTINCT fixity is lawful.
    expect("same_surface_and_fixity_cannot_name_two_operators",
           rfind([comparison, {**comparison, "id": "OP-EQ2"}]),
           "claimed by")
    lawful = rfind([{**comparison, "fixity": "Prefix"},
                    {**comparison, "id": "OP-EQ2", "fixity": "Infix"}])
    if any("claimed by" in f for f in lawful):
        fail("same_surface_under_distinct_fixity_is_lawful")

    # A comparison alias stays attached to its one canonical OperatorId.
    m = _sample_operator_model()
    m["id_variants"] = m["id_all"] = m["id_all"] + ["NotEqual"]
    m["id_token"]["NotEqual"] = "OP-NE"
    m["id_owner"]["NotEqual"] = "BP-BATQL-LANGUAGE-1"
    m["id_basis"]["NotEqual"] = "DEC-060"
    m["word_variants"] = m["word_all"] = m["word_all"] + ["IsNot"]
    m["word_token"]["IsNot"] = "IS NOT"
    m["word_owner"]["IsNot"] = "BP-BATQL-LANGUAGE-1"
    m["word_basis"]["IsNot"] = "DEC-060"
    m["rows"] = m["rows"] + [{**m["rows"][1], "variant": "NotEqual",
                              "inner": "OperatorWordSurface::IsNot, OperatorSymbolSurface::Equal"}]
    expect("comparison_alias_must_lower_to_the_same_operator_id", mfind(m),
           "an alias stays attached to one OperatorId")

    # The DEC-060 admission boundary names the typed surface law and the
    # exact V1 comparison pairs.
    probe("dec060_must_name_the_operator_surface_boundary",
          [("spec/dispositions.rs",
            "and OperatorSyntax the closed source-shape law",
            "and a closed source-shape law")],
          batql, "must name the operator surface boundary")
    probe("dec060_must_name_the_exact_comparison_pairs",
          [("spec/dispositions.rs",
            "IS LESS THAN with <,",
            "IS LESS THAN with <>,")],
          batql, "do not equal the typed word/symbol mapping")

    # A comparison-pair swap in the typed source is refused by the DEC-060
    # mapping law even AFTER every projection is regenerated: the hostile
    # must fail for the syntax law, not for a stale block.
    tmp = gate_sandbox([
        ("spec/operators.rs",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsLessThan, OperatorSymbolSurface::LessThan)",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsLessThan, OperatorSymbolSurface::AtMost)"),
        ("spec/operators.rs",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsAtMost, OperatorSymbolSurface::AtMost)",
         "OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsAtMost, OperatorSymbolSurface::LessThan)"),
    ])
    try:
        _regen_operator_blocks(project, tmp)
        expect("comparison_pair_swap_is_rejected_after_regeneration", batql(tmp),
               "do not equal the typed word/symbol mapping")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # The formatter law and the spoken fence.
    probe("formatter_must_emit_the_canonical_surface",
          [("companion/BATQL_LANGUAGE.md", "| OP-EQ | `IS` | is |", "| OP-EQ | `=` | is |")],
          batql, "not its canonical surface")
    probe("spoken_projection_cannot_enter_operator_grammar",
          [("companion/BATQL_LANGUAGE.md",
            "logical_operator\n  := AND | OR", "logical_operator\n  := AND | OR | times")],
          batql, "narration is never source syntax")
    probe("arithmetic_spoken_word_cannot_become_source_syntax",
          [("companion/BATQL_LANGUAGE.md",
            'arithmetic_operator\n  := "*" | "/" | "+" | "-"',
            'arithmetic_operator\n  := "*" | "/" | "+" | "-" | plus')],
          batql, "narration is never source syntax")

    # The retired raw fields and the empty-string sentinel may not return.
    probe("raw_word_surface_field_cannot_return",
          [("spec/operators.rs", 'semantic_op: "multiply"',
            'word_surface: "*", semantic_op: "multiply"')],
          batql, "retired raw surface field word_surface may not return")
    probe("raw_symbol_surface_field_cannot_return",
          [("spec/operators.rs", 'semantic_op: "equal"',
            'symbol_surface: "=", semantic_op: "equal"')],
          batql, "retired raw surface field symbol_surface may not return")
    probe("duplicate_formatting_field_cannot_return",
          [("spec/operators.rs", 'spoken: "times"',
            'formatting: "*", spoken: "times"')],
          batql, "retired raw surface field formatting may not return")
    probe("empty_symbol_sentinel_cannot_return",
          [("spec/operators.rs", 'spoken: "or"', 'spoken: ""')],
          batql, "empty-string surface sentinel may not return")

    # Projection drift: each fixture edits ONLY the generated document.
    probe("operator_surface_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "| OP-NE | word-with-symbol | `IS NOT` | `!=` |",
            "| OP-NE | word-with-symbol | `!=` | `IS NOT` |")],
          batql, "generated block OPERATORS-SURFACES does not match")
    probe("operator_catalog_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "| OP-MUL | Arithmetic | Binary |", "| OP-MUL | Arithmetic | Unary |")],
          batql, "generated block OPERATORS-CATALOG does not match")
    probe("operator_grammar_projection_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            'arithmetic_operator\n  := "*"', 'arithmetic_operator\n  := "**"')],
          batql, "generated block OPERATORS-GRAMMAR does not match")

    # Provenance: every operator projection's BEGIN marker names the typed
    # source and the generator byte-exactly.
    probe("operator_projection_source_marker_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "OPERATORS-PROJECTION:BEGIN generated from spec/operators.rs by bootstrap/project.py",
            "OPERATORS-PROJECTION:BEGIN generated from spec/operators.rs by bootstrap/render.py")],
          batql, "OPERATORS-PROJECTION BEGIN marker does not name")
    probe("operator_typing_source_marker_drift_is_rejected",
          [("companion/BATQL_LANGUAGE.md",
            "OPERATORS-TYPING:BEGIN generated from spec/operators.rs",
            "OPERATORS-TYPING:BEGIN generated from spec/architecture.rs")],
          batql, "OPERATORS-TYPING BEGIN marker does not name")
    probe("operator_numeric_source_marker_drift_is_rejected",
          [("docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md",
            "OPERATORS-NUMERIC:BEGIN generated from spec/operators.rs",
            "OPERATORS-NUMERIC:BEGIN generated from spec/numeric.rs")],
          batql, "OPERATORS-NUMERIC BEGIN marker does not name")

    # The identity closure: the residue row rests on the REAL typed owner,
    # and OperatorId can never re-enter the general identity catalogs.
    tmp = gate_sandbox([("spec/operators.rs", "pub enum OperatorId {",
                         "pub enum OperatorTag {")])
    try:
        expect("operator_id_residue_requires_a_real_typed_owner",
               batql(tmp), "no public typed OperatorId owner")
        expect("operator_id_residue_requires_a_real_typed_owner (wrapper-guard parity)",
               audit.identity_catalog_findings(tmp),
               "EXISTING_TYPED_OWNER_SPELLINGS lists OperatorId, which no spec")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    probe("operator_id_cannot_enter_the_general_identity_catalog",
          [("spec/identities.rs",
            'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("OperatorId", "BP-CRYPTO-SECRET-1")')],
          audit.identity_catalog_findings,
          "duplicates a spelling that already has a typed spec owner")

    # E3j1: the documentary layer may fence operator admission, never deny
    # value existence — DEC-069/docs/37 own first-class Qualified
    # Approximation TODAY. The ordinary parser strips comments, so this law
    # executes over the raw source.
    operators_src = (root / "spec/operators.rs").read_text(encoding="utf-8")
    for stale in ("No Approximate value exists yet.",
                  "No `Approximate` value exists until a profile ships."):
        mutated = must_replace(
            operators_src, "Approximate numeric values already exist as typed",
            stale, "operator_docs_cannot_deny_existing_approximate_values")
        expect("operator_docs_cannot_deny_existing_approximate_values",
               audit.batql_operator_doc_findings(mutated),
               "approximate values exist; no current ordinary operator admits raw")
    expect("operator_docs_cannot_revive_superseded_chronology",
           audit.batql_operator_doc_findings(
               operators_src + "\n// those arrive with 5.5B2\n"),
           "approximate values exist; no current ordinary operator admits raw")
    # GREEN: the corrected module, Exactness, and NumericSupport doctrines
    # agree with DEC-069/docs/37 while every real OperatorSpec row stays
    # exact-only — value vocabulary exists, operator admission does not.
    doc_green = audit.batql_operator_doc_findings(operators_src)
    admitted_rows = [op["id"] for op in resolved(audit.batql_parse_operator_model(root))
                     if op["numeric_support"] == "QualifiedProfileOnly"]
    if doc_green or admitted_rows:
        fail("operator_docs_distinguish_value_existence_from_operator_admission "
             f"({doc_green!r}, {admitted_rows!r})")

    # GREEN structural growth — count-freedom evidence ONLY. A sandbox-only
    # synthetic comparison operator with typed identity, both surfaces, an
    # explicit owner and basis, DEC-060 naming, authored companion adoption,
    # and regenerated projections passes every gate: no frozen numeric count
    # guards the inventory. This does NOT admit the operator into real BatQL.
    tmp = gate_sandbox([
        ("spec/operators.rs", "    Not,\n    And,\n    Or,\n}",
         "    Not,\n    And,\n    Or,\n    Congruent,\n}"),
        ("spec/operators.rs", "        OperatorId::Or,\n    ];",
         "        OperatorId::Or,\n        OperatorId::Congruent,\n    ];"),
        ("spec/operators.rs", '            OperatorId::Or => "OP-OR",\n        }',
         '            OperatorId::Or => "OP-OR",\n            OperatorId::Congruent => "OP-CONG",\n        }'),
        ("spec/operators.rs",
         '            OperatorId::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorId::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorId::Congruent => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorId::Or => DecisionId("DEC-060"),\n        }',
         '            OperatorId::Or => DecisionId("DEC-060"),\n            OperatorId::Congruent => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "    Not,\n    And,\n    Or,\n}",
         "    Not,\n    And,\n    Or,\n    IsRoughly,\n}"),
        ("spec/operators.rs", "        OperatorWordSurface::Or,\n    ];",
         "        OperatorWordSurface::Or,\n        OperatorWordSurface::IsRoughly,\n    ];"),
        ("spec/operators.rs", '            OperatorWordSurface::Or => "OR",\n        }',
         '            OperatorWordSurface::Or => "OR",\n            OperatorWordSurface::IsRoughly => "IS ROUGHLY",\n        }'),
        ("spec/operators.rs",
         '            OperatorWordSurface::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorWordSurface::Or => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorWordSurface::IsRoughly => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorWordSurface::Or => DecisionId("DEC-060"),\n        }',
         '            OperatorWordSurface::Or => DecisionId("DEC-060"),\n            OperatorWordSurface::IsRoughly => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "    MoreThan,\n    AtLeast,\n}",
         "    MoreThan,\n    AtLeast,\n    Congruent,\n}"),
        ("spec/operators.rs", "        OperatorSymbolSurface::AtLeast,\n    ];",
         "        OperatorSymbolSurface::AtLeast,\n        OperatorSymbolSurface::Congruent,\n    ];"),
        ("spec/operators.rs", '            OperatorSymbolSurface::AtLeast => ">=",\n        }',
         '            OperatorSymbolSurface::AtLeast => ">=",\n            OperatorSymbolSurface::Congruent => "~",\n        }'),
        ("spec/operators.rs",
         '            OperatorSymbolSurface::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),\n        }',
         '            OperatorSymbolSurface::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),\n            OperatorSymbolSurface::Congruent => ContractId("BP-BATQL-LANGUAGE-1"),\n        }'),
        ("spec/operators.rs",
         '            OperatorSymbolSurface::AtLeast => DecisionId("DEC-060"),\n        }',
         '            OperatorSymbolSurface::AtLeast => DecisionId("DEC-060"),\n            OperatorSymbolSurface::Congruent => DecisionId("DEC-060"),\n        }'),
        ("spec/operators.rs", "numeric_support: NumericSupport::NotApplicable },\n];",
         "numeric_support: NumericSupport::NotApplicable },\n"
         '    OperatorSpec { id: OperatorId::Congruent, class: OperatorClass::Comparison, '
         "syntax: OperatorSyntax::WordWithSymbolAlias(OperatorWordSurface::IsRoughly, "
         'OperatorSymbolSurface::Congruent), semantic_op: "congruent", arity: Arity::Binary, '
         "fixity: Fixity::Infix, precedence: 50, associativity: Associativity::NonAssociative, "
         'typing: &[OperatorTypingRule::SameSortComparison], input_sorts: "exact same-sort pair", '
         'result_sort: "Truth with TypedMargin", exactness: Exactness::Exact, overflow: "none", '
         'exception: "cross-sort comparison is a type error", spoken: "is roughly", '
         'mutation_classes: "precedence, comparison-direction, margin-sign, parenthesis", '
         "numeric_support: NumericSupport::ExactSupported },\n];"),
        ("spec/dispositions.rs", "IS AT LEAST with >=, and NOT AND OR are word-only",
         "IS AT LEAST with >=, IS ROUGHLY with ~, and NOT AND OR are word-only"),
        ("companion/BATQL_LANGUAGE.md",
         "Spoken narration is not parser syntax: the spoken projection narrates the typed tree and is never accepted source.",
         "Spoken narration is not parser syntax: the spoken projection narrates the typed tree and is never accepted source.\n\n"
         "A sandbox-only synthetic comparison `IS ROUGHLY` (compact alias `~`) may be "
         "added here as structural count-freedom evidence."),
    ])
    try:
        _regen_operator_blocks(project, tmp)
        grown = batql(tmp)
        if grown:
            fail(f"synthetic_operator_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    return findings


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


def test_generated_views(audit, project) -> list[str]:
    """Named hostile fixtures for the generated-view registry (5.5E4a):
    spec/generated_views.rs is the closed denominator of every repository-
    generated view, the projector dispatches only through it, and the
    universal marker census leaves no unregistered block in the wallpaper."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def sandbox(edits):
        # gate_sandbox carries the bootstrap sources since 5.5E4b, so the
        # registry auditor's dispatcher-parity read resolves in every sandbox.
        return gate_sandbox(edits)

    def probe(name, edits, needle, validator=None):
        tmp = sandbox(edits)
        try:
            got = (validator or audit.generated_view_findings)(tmp)
            expect(name, got, needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.generated_view_findings(root):
        fail(f"generated_view_registry_passes_on_the_real_seed "
             f"({audit.generated_view_findings(root)!r})")

    GV = "spec/generated_views.rs"
    PY = "bootstrap/project.py"
    GATES_ROW = "        GeneratedView::GateInventory,\n"

    # Registry structure.
    probe("generated_view_missing_from_all_is_rejected",
          [(GV, GATES_ROW, "")],
          "is declared but missing from GeneratedView::ALL")
    probe("generated_view_without_spec_arm_is_rejected",
          [(GV, "GeneratedView::GateInventory => GeneratedViewSpec {",
            "GeneratedView::GateInventoryRenamed => GeneratedViewSpec {")],
          "GeneratedView::GateInventory has no parseable spec() arm")
    probe("duplicate_generated_view_name_is_rejected",
          [(GV, GATES_ROW, GATES_ROW + GATES_ROW)],
          "GeneratedView::ALL repeats GateInventory")
    probe("generated_view_without_authority_source_is_rejected",
          [(GV, 'authority_sources: &["spec/gates.rs"],', "authority_sources: &[],")],
          "generated view GateInventory names no authority source")
    probe("generated_view_with_missing_authority_source_is_rejected",
          [(GV, 'authority_sources: &["spec/gates.rs"],',
            'authority_sources: &["spec/gates_missing.rs"],')],
          "names authority source spec/gates_missing.rs, which does not exist")
    probe("generated_view_with_missing_static_target_is_rejected",
          [(GV, 'GeneratedViewTarget::Static(&["docs/25_IMPLEMENTATION_GATES.md"])',
            'GeneratedViewTarget::Static(&["docs/25_MISSING.md"])')],
          "targets docs/25_MISSING.md, which does not exist")

    # Surface/target shape.
    probe("embedded_view_without_marker_is_rejected",
          [(GV, 'marker: Some("GATE-INVENTORY"),', "marker: None,")],
          "embedded generated view GateInventory carries no marker")
    probe("standalone_view_with_embedded_marker_is_rejected",
          [(GV,
            'target: GeneratedViewTarget::Static(&["rust-toolchain.toml"]),\n'
            "                surface: GeneratedViewSurface::StandaloneFile,\n"
            "                marker: None,",
            'target: GeneratedViewTarget::Static(&["rust-toolchain.toml"]),\n'
            "                surface: GeneratedViewSurface::StandaloneFile,\n"
            '                marker: Some("RUST-TOOLCHAIN"),')],
          "standalone generated view RustToolchain carries an embedded marker")
    probe("corpus_frontmatter_with_static_target_is_rejected",
          [(GV,
            "target: GeneratedViewTarget::EligibleMarkdownCorpus,\n"
            "                surface: GeneratedViewSurface::CorpusFrontmatter,",
            'target: GeneratedViewTarget::Static(&["docs/00_CONSTITUTION.md"]),\n'
            "                surface: GeneratedViewSurface::CorpusFrontmatter,")],
          "must target the eligible markdown corpus, never static paths")
    probe("duplicate_target_marker_claim_is_rejected",
          [(GV, 'marker: Some("MUTATION-RESULTS"),', 'marker: Some("MUTATION-LANES"),')],
          "both claim marker MUTATION-LANES in docs/12_TESTPAK.md")
    if any("STALE-VOCAB" in f or "StaleVocabulary" in f
           for f in audit.generated_view_findings(root)):
        fail("multi_target_stale_vocabulary_is_lawful")

    # The universal marker census.
    GATES_DOC = "docs/25_IMPLEMENTATION_GATES.md"
    GATES_BEGIN = ("<!-- GATE-INVENTORY:BEGIN generated from spec/gates.rs by "
                   "bootstrap/project.py; do not edit -->")
    probe("unregistered_generated_begin_marker_is_rejected",
          [(GATES_DOC, GATES_BEGIN,
            "<!-- ROGUE-VIEW:BEGIN generated from spec/gates.rs by "
            "bootstrap/project.py; do not edit -->\nrogue\n"
            "<!-- ROGUE-VIEW:END -->\n\n" + GATES_BEGIN)],
          "generated marker ROGUE-VIEW is not a registered generated view")
    probe("orphan_generated_end_marker_is_rejected",
          [(GATES_DOC, GATES_BEGIN, "<!-- ORPHANED:END -->\n\n" + GATES_BEGIN)],
          "ORPHANED has an END with no matching BEGIN")
    probe("mismatched_generated_end_marker_is_rejected",
          [(GATES_DOC, "<!-- GATE-INVENTORY:END -->", "<!-- GATE-INVENTORYX:END -->")],
          "GATE-INVENTORY has a BEGIN with no matching END")
    probe("duplicate_generated_marker_in_one_target_is_rejected",
          [(GATES_DOC, GATES_BEGIN,
            GATES_BEGIN + "\nrogue\n<!-- GATE-INVENTORY:END -->\n\n" + GATES_BEGIN)],
          "generated marker GATE-INVENTORY appears more than once")
    probe("registered_marker_missing_from_target_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN", "GATE-INVENTORYX:BEGIN"),
           (GATES_DOC, "GATE-INVENTORY:END", "GATE-INVENTORYX:END")],
          "registered generated marker GATE-INVENTORY is absent from its target")
    probe("generated_marker_source_drift_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN generated from spec/gates.rs",
            "GATE-INVENTORY:BEGIN generated from spec/architecture.rs")],
          "names source spec/architecture.rs, not its registered authority source")
    probe("generated_marker_generator_drift_is_rejected",
          [(GATES_DOC, "GATE-INVENTORY:BEGIN generated from spec/gates.rs by bootstrap/project.py",
            "GATE-INVENTORY:BEGIN generated from spec/gates.rs by bootstrap/render.py")],
          "names generator bootstrap/render.py, not bootstrap/project.py")

    # Dispatcher parity and the registry bypass fence.
    probe("projector_dispatcher_omission_is_rejected",
          [(PY, '    "GateInventory": render_gate_inventory,\n', "")],
          "registered generated view GateInventory has no projector dispatcher entry")
    probe("unregistered_projector_dispatcher_entry_is_rejected",
          [(PY, '    "GateInventory": render_gate_inventory,\n',
            '    "GateInventory": render_gate_inventory,\n'
            '    "RogueView": render_gate_inventory,\n')],
          "projector dispatcher entry RogueView serializes no registered view")
    probe("manual_projection_plan_cannot_bypass_registry",
          [(PY, '\nif __name__ == "__main__":',
            '\n_bypass = block_pattern("ROGUE-VIEW")\n\nif __name__ == "__main__":')],
          "manual projection plan for 'ROGUE-VIEW'")

    # Standalone provenance.
    probe("guarantee_graph_generated_from_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "generated_from: spec/invariants.rs;",
            "generated_from: spec/wrong.rs;")],
          "not its registry row")
    probe("guarantee_graph_generator_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "generated_by: bootstrap/project.py",
            "generated_by: bootstrap/render.py")],
          "not bootstrap/project.py")
    probe("rust_toolchain_provenance_comment_missing_is_rejected",
          [("rust-toolchain.toml",
            "# generated from spec/toolchain.rs by bootstrap/project.py; do not edit\n[toolchain]",
            "[toolchain]")],
          "missing or drifted provenance comment")

    # Registry self-inclusion and the corpus-membership floor.
    probe("corpus_epoch_membership_cannot_leave_the_registry",
          [(GV, "        GeneratedView::CorpusEpochMembership,\n", "")],
          "CorpusEpochMembership may not leave the registry")
    probe("generated_view_registry_must_include_itself",
          [(GV, "        GeneratedView::GeneratedViewRegistry,\n", "")],
          "must include itself")
    probe("generated_view_registry_projection_drift_is_rejected",
          [("docs/28_SELF_EXPLAINING_REPOSITORY.md",
            "| GateInventory | embedded-block |",
            "| GateInventory | standalone-file |")],
          "generated block GENERATED-VIEW-REGISTRY does not match")
    probe("generated_view_registry_marker_drift_is_rejected",
          [("docs/28_SELF_EXPLAINING_REPOSITORY.md",
            "GENERATED-VIEW-REGISTRY:BEGIN generated from spec/generated_views.rs",
            "GENERATED-VIEW-REGISTRY:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source spec/generated_views.rs")

    # The retired hybrid fence and the projector's documentary honesty.
    probe("contract_kind_hybrid_fence_cannot_return",
          [("docs/06_MACBAT.md",
            "What each admitting law means, in authored prose",
            "The admitted kinds, each with its admitting law:\n\n```text\n"
            "Error LEG-047 stale hybrid\n```\n\n"
            "What each admitting law means, in authored prose")],
          "hybrid admitted-kinds fence may not return",
          validator=audit.contract_kind_findings)
    probe("projector_docs_cannot_deny_guarantee_graph_generation",
          [(PY, "project.py never generates decision text itself.",
            "project.py never generates decision text itself, and does not "
            "generate the Guarantee Graph.")],
          "denies generating the Guarantee Graph")
    probe("projector_status_cannot_report_only_operator_blocks",
          [(PY, '\nif __name__ == "__main__":',
            '\n_legacy_banner = "operator blocks current"\n\nif __name__ == "__main__":')],
          "status reports only operator blocks")

    # GREEN structural registry growth — evidence ONLY. A sandbox-only future
    # view with a fresh source, a temporary target, a complete arm, and a
    # dispatcher entry stays green after regeneration through its OWN
    # projector: no frozen numeric count guards the registry. This does not
    # justify a real new generated surface.
    tmp = sandbox([
        (GV, "    GeneratedViewRegistry,\n}",
         "    GeneratedViewRegistry,\n    FutureLedger,\n}"),
        (GV, "        GeneratedView::GeneratedViewRegistry,\n    ];",
         "        GeneratedView::GeneratedViewRegistry,\n"
         "        GeneratedView::FutureLedger,\n    ];"),
        (GV, '            GeneratedView::GeneratedViewRegistry => "GeneratedViewRegistry",\n        }',
         '            GeneratedView::GeneratedViewRegistry => "GeneratedViewRegistry",\n'
         '            GeneratedView::FutureLedger => "FutureLedger",\n        }'),
        (GV,
         'marker: Some("GENERATED-VIEW-REGISTRY"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },",
         'marker: Some("GENERATED-VIEW-REGISTRY"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },\n"
         "            GeneratedView::FutureLedger => GeneratedViewSpec {\n"
         '                authority_sources: &["spec/future_ledger.rs"],\n'
         '                target: GeneratedViewTarget::Static(&["docs/FUTURE_LEDGER.md"]),\n'
         "                surface: GeneratedViewSurface::EmbeddedBlock,\n"
         '                marker: Some("FUTURE-LEDGER"),\n'
         "                generator: BootstrapToolId::ProjectPy,\n"
         "            },"),
        (PY, '    "GeneratedViewRegistry": render_generated_view_registry,\n}',
         '    "GeneratedViewRegistry": render_generated_view_registry,\n'
         '    "FutureLedger": lambda root: "sandbox-only future ledger body",\n}'),
    ])
    try:
        (tmp / "spec/future_ledger.rs").write_bytes(
            b"// sandbox-only future ledger source (registry-growth evidence)\n")
        (tmp / "docs/FUTURE_LEDGER.md").write_bytes(
            b"# Future ledger (sandbox only)\n\n"
            b"<!-- FUTURE-LEDGER:BEGIN generated from spec/future_ledger.rs by "
            b"bootstrap/project.py; do not edit -->\n(pending)\n"
            b"<!-- FUTURE-LEDGER:END -->\n")
        spec_mod = importlib.util.spec_from_file_location(
            "project_growth_tmp", tmp / "bootstrap/project.py")
        proj_tmp = importlib.util.module_from_spec(spec_mod)
        spec_mod.loader.exec_module(proj_tmp)
        regen_findings: list[str] = []
        _views, plans, _bindings = proj_tmp.build_plans(tmp, regen_findings)
        if regen_findings:
            fail(f"structural_registry_growth_is_lawful (projector refused: "
                 f"{regen_findings!r})")
        else:
            for path, original, rewritten in plans:
                if rewritten != original:
                    path.write_bytes(rewritten.encode("utf-8"))
            grown = audit.generated_view_findings(tmp)
            if grown:
                fail(f"structural_registry_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    return findings


def test_inventory_mirrors(audit, project) -> list[str]:
    """Named hostile fixtures for the repository inventory projections
    (5.5E4b): package/edge/profile/bundle/Tier0 mirrors are generated
    derivations of their typed owners, retired authored mirrors stay retired,
    and a denominator never impersonates a run result."""
    findings: list[str] = []
    root = HERE.parent
    AR = "spec/architecture.rs"
    RM, D3, DN, D29 = ("README.md", "docs/03_REPOSITORY_AND_PACKAGES.md",
                       "DELIVERY_NOTES.md", "docs/29_STATUS_AND_SUPERSESSION.md")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def sandbox(edits):
        # gate_sandbox carries the root-level documents and bootstrap sources
        # since 5.5E4b, so the inventory projections resolve in every sandbox.
        return gate_sandbox(edits)

    def probe(name, edits, needle, validator=None):
        tmp = sandbox(edits)
        try:
            expect(name, (validator or audit.inventory_mirror_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.inventory_mirror_findings(root):
        fail(f"inventory_mirrors_pass_on_the_real_seed "
             f"({audit.inventory_mirror_findings(root)!r})")

    # Documentary class inventories.
    probe("package_class_missing_from_all_is_rejected",
          [(AR, "        PackageClass::Example,\n    ];", "    ];")],
          "PackageClass::Example is declared but missing from PackageClass::ALL")
    probe("duplicate_package_class_spelling_is_rejected",
          [(AR, 'PackageClass::Example => "example",', 'PackageClass::Example => "production",')],
          "duplicate PackageClass spelling 'production'")
    probe("edge_class_missing_from_all_is_rejected",
          [(AR, "        EdgeClass::DevOnly,\n    ];", "    ];")],
          "EdgeClass::DevOnly is declared but missing from EdgeClass::ALL")
    probe("duplicate_edge_class_spelling_is_rejected",
          [(AR, 'EdgeClass::OptionalProfile => "optional-profile",',
            'EdgeClass::OptionalProfile => "required",')],
          "duplicate EdgeClass spelling 'required'")

    # Every generated inventory row is positional law: a dropped, reordered,
    # or reclassified row — and any smuggled module — is projection drift.
    PKG_NEEDLE = "generated block PACKAGE-INVENTORY does not match its typed derivation"
    EX_ROW = ("| batpak-examples | example | 5 | examples | public-surface witness; "
              "runnable demos over production APIs only; owns no semantic law and "
              "depends on no dev tooling |")
    for name, rel, old, new, needle in (
        ("package_inventory_missing_batpak_examples_is_rejected", RM,
         EX_ROW + "\n", "", PKG_NEEDLE),
        ("package_inventory_order_drift_is_rejected", D3,
         "| macbat-compiler | production | 0 | crates/macbat/compiler | pure Rust contract compiler |\n| macbat | production | 1 |",
         "| macbat | production | 1 | crates/macbat/macros | proc-macro front door |\n| macbat-compiler | production | 0 |",
         PKG_NEEDLE),
        ("package_inventory_class_drift_is_rejected", RM,
         "| batpak-cli | binary-adapter |", "| batpak-cli | production |", PKG_NEEDLE),
        ("package_inventory_layer_drift_is_rejected", RM,
         "| macbat | production | 1 |", "| macbat | production | 2 |", PKG_NEEDLE),
        ("package_inventory_path_drift_is_rejected", RM,
         "| batpak | production | 2 | crates/batpak |",
         "| batpak | production | 2 | crates/batpak2 |", PKG_NEEDLE),
        ("package_inventory_role_drift_is_rejected", RM,
         "pure Rust contract compiler |", "impure contract compiler |", PKG_NEEDLE),
        ("internal_syncbat_plane_cannot_enter_package_inventory", RM,
         EX_ROW, EX_ROW + "\n| pakvm | production | 3 | crates/syncbat | plane |",
         PKG_NEEDLE),
        ("muterprater_cannot_enter_package_inventory", D3,
         EX_ROW, EX_ROW + "\n| muterprater | dev-only | 6 | crates/testpak | module |",
         PKG_NEEDLE),
        ("package_edge_missing_from_projection_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |\n", "",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_extra_projection_row_is_rejected", D3,
         "| batpak-examples | batql | required | example |",
         "| batpak-examples | batql | required | example |\n| batql | netbat | required | rogue |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_order_drift_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |\n| batpak | macbat | required | derive |",
         "| batpak | macbat | required | derive |\n| macbat | macbat-compiler | required | compile |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_class_drift_is_rejected", D3,
         "| batpak-cli | netbat | optional-profile | serve |",
         "| batpak-cli | netbat | required | serve |",
         "generated block PACKAGE-EDGES does not match"),
        ("package_edge_profile_drift_is_rejected", D3,
         "| macbat | macbat-compiler | required | compile |",
         "| macbat | macbat-compiler | required | compile2 |",
         "generated block PACKAGE-EDGES does not match"),
        ("qualification_profile_missing_browser_storage_is_rejected", D3,
         "| batpak | browser-storage |", "| batpak | browser-storage-gone |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_profile_order_drift_is_rejected", D3,
         "| batpak | semantic | no_std + alloc |", "| batpak | zzz-semantic | no_std + alloc |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_environment_drift_is_rejected", D3,
         "| syncbat | semantic | no_std + alloc |", "| syncbat | semantic | std |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_gate_drift_is_rejected", D3,
         "| batpak | semantic | no_std + alloc | G0/G5 |",
         "| batpak | semantic | no_std + alloc | G5 |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("qualification_requirement_drift_is_rejected", D3,
         "storage-port law compile without std |", "storage-port law need std |",
         "generated block QUALIFICATION-PROFILES does not match"),
        ("bundle_inventory_package_count_tracks_package_id_all", DN,
         "| Cargo packages | 9 |", "| Cargo packages | 8 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_edge_count_tracks_edges", DN,
         "| package edges | 19 |", "| package edges | 18 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_profile_count_tracks_profiles", DN,
         "| qualification profiles | 6 |", "| qualification profiles | 5 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_seed_count_tracks_seed", DN,
         "| SEED guarantees | 32 |", "| SEED guarantees | 31 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_decision_count_tracks_decisions", DN,
         "| decision rows | 82 |", "| decision rows | 81 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_legacy_count_tracks_obligations", DN,
         "| legacy semantic obligations | 87 |", "| legacy semantic obligations | 86 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_coverage_count_tracks_source_invariant_ids", DN,
         "| legacy invariant declarations | 115 |", "| legacy invariant declarations | 107 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_operator_count_tracks_operator_id_all", DN,
         "| BatQL operators | 13 |", "| BatQL operators | 12 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_generated_view_count_tracks_registry", DN,
         f"| registered generated views | {len(audit.a_parse_generated_views(root)['order'])} |",
         "| registered generated views | 1 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("bundle_inventory_markdown_count_tracks_eligible_corpus", DN,
         "| Markdown documents | 49 |", "| Markdown documents | 48 |",
         "generated block BUNDLE-INVENTORY does not match"),
        ("tier0_receipt_missing_from_projection_is_rejected", DN,
         "| tier0-materialize | ExecutableAndOutputTree |\n", "",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
        ("tier0_receipt_artifact_policy_drift_is_rejected", DN,
         "| tier0-seedcheck | Executable |", "| tier0-seedcheck | FixtureSet |",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
        ("tier0_projection_order_drift_is_rejected", DN,
         "| tier0-law-fixtures | FixtureSet |\n| tier0-seedcheck | Executable |",
         "| tier0-seedcheck | Executable |\n| tier0-law-fixtures | FixtureSet |",
         "generated block TIER0-RECEIPT-DENOMINATOR does not match"),
    ):
        probe(name, [(rel, old, new)], needle)

    # Retired authored mirrors stay retired; converged doctrine stays authored.
    for name, rel, insertion, needle in (
        ("readme_authored_package_table_cannot_return", RM,
         "\n| `batpak` | production Cargo package | semantic core |\n",
         "retired authored mirror may not return (authored package table)"),
        ("docs03_authored_layer_inventory_cannot_return", D3,
         "\nL0  macbat-compiler\nL1  macbat\n",
         "retired authored mirror may not return (authored layer inventory)"),
        ("docs03_ascii_dependency_inventory_cannot_return", D3,
         "\nmacbat\n  ↑\nmacbat-compiler\n",
         "retired authored mirror may not return (ASCII dependency inventory)"),
        ("docs03_five_profile_mirror_cannot_return", D3,
         "\nbatpak semantic    no_std + alloc\n",
         "retired authored mirror may not return (five-profile target matrix)"),
        ("authored_bundle_counts_cannot_return", DN,
         "\n19 declared package edges\n",
         "retired authored mirror may not return (authored bundle counts)"),
        ("legacy_107_row_count_is_rejected", DN,
         "\n107-row legacy catalog coverage equivalence PASS\n",
         "retired authored mirror may not return (107-row legacy claim)"),
        ("volatile_word_count_cannot_return", DN,
         "\n31,000+ words of architecture\n",
         "retired authored mirror may not return (volatile word count)"),
        ("static_validation_pass_table_cannot_return", DN,
         "\n## Validation completed in this delivery environment\n",
         "retired authored mirror may not return (static validation PASS table)"),
        ("historical_no_rustc_claim_cannot_return", DN,
         "\nThe environment did not contain `rustc` or `cargo`.\n",
         "retired authored mirror may not return (historical no-rustc claim)"),
        ("delivery_notes_cannot_embed_current_run_ids_as_timeless_law", DN,
         "\nHosted run 29593750281 was green.\n",
         "retired authored mirror may not return (current run id embedded as timeless law)"),
    ):
        probe(name, [(rel, "\n## ", insertion + "\n## ")], needle)
    for name, rel, fragment, needle in (
        ("feature_coverage_family_cannot_masquerade_as_qualification_profile", D3,
         "They are not additional QualificationProfile identities unless they enter",
         "converged inventory doctrine absent (feature-coverage-distinction)"),
        ("authored_metadata_requires_reconciliation_epoch", D29,
         "ReconciliationEpoch corpus membership",
         "converged inventory doctrine absent (authored-metadata-epoch)"),
        ("generated_metadata_requires_exact_sources", D29,
         "generator, exact authority sources, do-not-edit posture",
         "converged inventory doctrine absent (generated-metadata-sources)"),
        ("generated_document_does_not_require_contract_id", D29,
         "does not invent a contract ID or supersession claim",
         "converged inventory doctrine absent (generated-needs-no-contract-id)"),
        ("authored_document_cannot_use_generated_metadata_shape", D29,
         "cannot claim generated status to evade the authored fields",
         "converged inventory doctrine absent (authored-cannot-claim-generated)"),
    ):
        probe(name, [(rel, fragment, "RETIRED FRAGMENT")], needle)
    # Multi-source provenance: order IS provenance.
    probe("bundle_inventory_multi_source_order_drift_is_rejected",
          [(DN, "BUNDLE-INVENTORY:BEGIN generated from spec/architecture.rs; spec/invariants.rs;",
            "BUNDLE-INVENTORY:BEGIN generated from spec/invariants.rs; spec/architecture.rs;")],
          "not its registered authority source",
          validator=audit.generated_view_findings)

    # GREEN: profile growth — a sandbox-only QualificationProfile row flows
    # mechanically into docs/03 and the bundle count with no frozen number.
    first_profile = (
        'QualificationProfile {\n        package: PackageId::BatPak,\n'
        '        profile: "semantic",\n'
        '        environment: QualificationEnvironment::NoStdAlloc,\n'
        '        gates: &[GateId::G0, GateId::G5],\n'
        '        requirement: "contracts, schemas, codecs, image values, '
        'deterministic parsing, and storage-port law compile without std",\n    },')
    tmp = sandbox([(AR, first_profile, first_profile +
                    '\n    QualificationProfile {\n        package: PackageId::BatPak,\n'
                    '        profile: "sandbox-growth",\n'
                    '        environment: QualificationEnvironment::WasmHost,\n'
                    '        gates: &[GateId::G2, GateId::G5],\n'
                    '        requirement: "sandbox-only structural growth evidence",\n    },')])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.inventory_mirror_findings(tmp) + audit.generated_view_findings(tmp)
        doc3 = (tmp / D3).read_text(encoding="utf-8")
        if grown or "| batpak | sandbox-growth | wasm32 host | G2/G5 |" not in doc3 \
                or "| qualification profiles | 7 |" not in (tmp / DN).read_text(encoding="utf-8"):
            fail(f"qualification_profile_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: one logical view, two registered target instances — the SAME
    # PACKAGE-INVENTORY bytes land in README and docs/03.
    readme_body = audit.batql_extract_block(
        (root / RM).read_text(encoding="utf-8"), "PACKAGE-INVENTORY")
    docs3_body = audit.batql_extract_block(
        (root / D3).read_text(encoding="utf-8"), "PACKAGE-INVENTORY")
    if readme_body is None or readme_body != docs3_body:
        fail("package_inventory_multi_target_bytes_are_identical")

    return findings


def test_exact_ledgers(audit, project) -> list[str]:
    """Named hostile fixtures for 5.5E4c: version-boundary convergence, the
    generated decision ledger, the generated legacy-invariant coverage matrix,
    and the exact-mirror/semantic-owner doctrine."""
    findings: list[str] = []
    root = HERE.parent
    DI, CV, ID = ("spec/dispositions.rs", "spec/legacy_invariant_coverage.rs",
                  "spec/identities.rs")
    D30, D34 = ("docs/30_DECISION_AND_REJECTION_LEDGER.md",
                "docs/34_LEGACY_INVARIANT_COVERAGE.md")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, needle, validator=None):
        tmp = gate_sandbox(edits)
        try:
            expect(name, (validator or audit.exact_ledger_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.exact_ledger_findings(root):
        fail(f"exact_ledgers_pass_on_the_real_seed "
             f"({audit.exact_ledger_findings(root)!r})")

    LEDGER = "generated block DECISION-LEDGER does not match its typed derivation"
    MATRIX = "generated block LEGACY-INVARIANT-COVERAGE does not match its typed derivation"
    DEC1 = "| DEC-001 | LOCK | HistoricalReceipt |  | Pass 1 machine-centered architecture | Base of final v1 |"
    INV1 = "| `INV-ALLOW-IS-DESIGN` | SUPERSEDE | `BP-AGENT-GUIDE and BP-GAUNTLET-1` | Keep the zero-silencer law; replace the legacy checker implementation. |"
    for name, rel, old, new, needle in (
        # Version convergence.
        ("dec064_must_name_every_version_identity", DI,
         "PakVmIsaVersion FbatFormatVersion", "FbatFormatVersion",
         "the version-separation law names its complete inventory"),
        ("dec064_cannot_name_phantom_version_identity", DI,
         "no generic Version crosses subsystem boundaries",
         "GadgetVersion is admitted; no generic Version crosses subsystem boundaries",
         "phantom version identity"),
        ("event_frame_must_use_frame_version", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "frame_version: FrameVersion", "frame_version: u16",
         "does not carry frame_version: FrameVersion"),
        ("frame_version_cannot_be_described_as_fbat_container_version",
         "docs/05_STORAGE_FBAT_AND_TILES.md",
         "FrameVersion versions the EventFrame envelope.",
         "FrameVersion versions the .fbat container.",
         "version/convergence doctrine absent (three-way-envelope-split)"),
        ("frame_version_cannot_be_described_as_payload_record_version",
         "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
         "FrameVersion is reserved for the EventFrame envelope",
         "FrameVersion is reserved for the payload record",
         "version/convergence doctrine absent (frame-version-reservation)"),
        ("fbat_format_version_cannot_be_described_as_event_frame_version",
         "docs/05_STORAGE_FBAT_AND_TILES.md",
         "FbatFormatVersion versions the surrounding .fbat container.",
         "FbatFormatVersion versions the EventFrame envelope.",
         "version/convergence doctrine absent (three-way-envelope-split)"),
        ("bat_tagged_record_version_cannot_be_described_as_container_version",
         "docs/15_SCHEMA_CODEC_AND_MIGRATION.md",
         "It does not version EventFrame or the .fbat container.",
         "It versions the .fbat container.",
         "version/convergence doctrine absent (payload-record-boundary)"),
        ("layout_version_cannot_substitute_for_schema_version",
         "docs/02_SYSTEM_MODEL.md",
         "Neither substitutes for the other.",
         "Either substitutes for the other.",
         "version/convergence doctrine absent (layout-schema-split)"),
        ("docs24_version_witness_cannot_freeze_ten", "docs/24_GAUNTLET.md",
         "Every member of spec/identities.rs VersionIdentityKind::ALL denotes a",
         "The ten declared version identities are distinct types; every one denotes a",
         "may not freeze a numeric count"),
        ("docs24_version_witness_must_consume_version_identity_all",
         "docs/24_GAUNTLET.md",
         "VersionIdentityKind::ALL denotes a", "the version catalog denotes a",
         "version/convergence doctrine absent (version-witness-consumes-all)"),
        ("docs32_cannot_use_ambiguous_frame_header_version_wording",
         "docs/32_IMPLEMENTATION_CONSTANTS.md",
         "FrameVersion values and EventFrame envelope field IDs/order",
         "frame/header magic and version numbers",
         "version/convergence doctrine absent (frame-version-constants)"),
        ("generic_version_type_is_rejected", ID,
         "pub const EXISTING_TYPED_OWNER_SPELLINGS",
         "pub struct Version(pub u32);\npub const EXISTING_TYPED_OWNER_SPELLINGS",
         "declares a generic Version type"),
        ("event_frame_version_alias_is_rejected", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "preserving three independent version identities.",
         "preserving three independent version identities. EventFrameVersion is the alias.",
         "an unadmitted version-identity alias"),
        ("fbat_frame_version_alias_is_rejected", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "preserving three independent version identities.",
         "preserving three independent version identities. FbatFrameVersion is the alias.",
         "an unadmitted version-identity alias"),
        # Decision-ledger vocabulary and drift.
        ("disposition_missing_from_all_is_rejected", DI,
         "        Disposition::RetainAsEvidence,\n    ];", "    ];",
         "Disposition::RetainAsEvidence is declared but missing from Disposition::ALL"),
        ("duplicate_disposition_spelling_is_rejected", DI,
         'Disposition::Defer => "DEFER",', 'Disposition::Defer => "KEEP",',
         "duplicate Disposition spelling 'KEEP'"),
        ("empty_disposition_meaning_is_rejected", DI,
         'Disposition::Keep => "retained as current architecture",',
         'Disposition::Keep => "",',
         "Disposition::Keep declares no or an empty meaning"),
        ("decision_class_missing_from_all_is_rejected", DI,
         "        DecisionClass::Naming,\n", "",
         "DecisionClass::Naming is declared but missing from DecisionClass::ALL"),
        ("duplicate_decision_class_spelling_is_rejected", DI,
         'DecisionClass::Naming => "Naming",', 'DecisionClass::Naming => "Capability",',
         "duplicate DecisionClass spelling 'Capability'"),
        ("decision_ledger_missing_row_is_rejected", D30, DEC1 + "\n", "", LEDGER),
        ("decision_ledger_extra_row_is_rejected", D30, DEC1,
         DEC1 + "\n| DEC-998 | KEEP | Naming | | X | Y |", LEDGER),
        ("decision_ledger_order_drift_is_rejected", D30,
         DEC1 + "\n| DEC-002 |", "| DEC-002 |\n" + DEC1 + " ", LEDGER),
        ("decision_ledger_tag_drift_is_rejected", D30,
         "| DEC-001 | LOCK |", "| DEC-001 | KEEP |", LEDGER),
        ("decision_ledger_class_drift_is_rejected", D30,
         "| DEC-001 | LOCK | HistoricalReceipt |", "| DEC-001 | LOCK | Naming |", LEDGER),
        ("decision_ledger_gate_drift_is_rejected", D30,
         "| DEC-003 | KILL | Architecture | G2 |", "| DEC-003 | KILL | Architecture | G5 |",
         LEDGER),
        ("decision_ledger_subject_drift_is_rejected", D30,
         "Pass 1 machine-centered architecture", "Pass 1 hand-centered architecture",
         LEDGER),
        ("decision_ledger_ruling_drift_is_rejected", D30,
         "| Base of final v1 |", "| Base of final v2 |", LEDGER),
        ("decision_disposition_legend_drift_is_rejected", D30,
         "| KEEP | retained as current architecture |",
         "| KEEP | retained as legacy architecture |", LEDGER),
        ("authored_decision_row_cannot_return", D30,
         "## Reopening rule",
         "| DEC-999 | KEEP | Naming | G0 | Rogue | Rogue |\n\n## Reopening rule",
         "an authored decision row may not return"),
        ("authored_disposition_inventory_cannot_return", D30,
         "## Reopening rule",
         "KEEP                 retained as current architecture\n\n## Reopening rule",
         "an authored disposition inventory may not return"),
        # Coverage vocabulary and drift.
        ("coverage_disposition_missing_from_all_is_rejected", CV,
         "        CoverageDisposition::Requalify,\n    ];", "    ];",
         "CoverageDisposition::Requalify is declared but missing"),
        ("duplicate_coverage_disposition_spelling_is_rejected", CV,
         'CoverageDisposition::Requalify => "REQUALIFY",',
         'CoverageDisposition::Requalify => "KILL",',
         "duplicate CoverageDisposition spelling 'KILL'"),
        ("empty_coverage_disposition_meaning_is_rejected", CV,
         'CoverageDisposition::Kill => "intentionally absent from the target",',
         'CoverageDisposition::Kill => "",',
         "CoverageDisposition::Kill declares no or an empty meaning"),
        ("legacy_coverage_missing_row_is_rejected", D34, INV1 + "\n", "", MATRIX),
        ("legacy_coverage_extra_row_is_rejected", D34, INV1,
         INV1 + "\n| `INV-ROGUE` | KILL | `x` | rogue |", MATRIX),
        ("legacy_coverage_order_drift_is_rejected", D34,
         INV1 + "\n| `INV-BATCH-ATOMIC-VISIBILITY` |",
         "| `INV-BATCH-ATOMIC-VISIBILITY` |\n" + INV1 + " ", MATRIX),
        ("legacy_coverage_disposition_drift_is_rejected", D34,
         "| `INV-ALLOW-IS-DESIGN` | SUPERSEDE |", "| `INV-ALLOW-IS-DESIGN` | KILL |",
         MATRIX),
        ("legacy_coverage_successor_drift_is_rejected", D34,
         "| `BP-AGENT-GUIDE and BP-GAUNTLET-1` |", "| `BP-AGENT-GUIDE` |", MATRIX),
        ("legacy_coverage_rationale_drift_is_rejected", D34,
         "Keep the zero-silencer law; replace the legacy checker implementation.",
         "Drop the zero-silencer law.", MATRIX),
        ("legacy_coverage_legend_drift_is_rejected", D34,
         "| PRESERVE | semantic law survives through named successor |",
         "| PRESERVE | semantic law may lapse |", MATRIX),
        ("legacy_coverage_source_denominator_drift_is_rejected", D34,
         "The source declaration denominator is the 115-declaration",
         "The source declaration denominator is the 107-declaration", MATRIX),
        ("authored_legacy_coverage_row_cannot_return", D34,
         "## Note: the two fusion invariants are distinct",
         "| `INV-ROGUE` | KILL | `x` | rogue |\n\n"
         "## Note: the two fusion invariants are distinct",
         "an authored coverage row may not return"),
        ("authored_coverage_disposition_inventory_cannot_return", D34,
         "## Note: the two fusion invariants are distinct",
         "PRESERVE    semantic law survives through named successor\n\n"
         "## Note: the two fusion invariants are distinct",
         "an authored coverage disposition inventory may not return"),
        # Convergence doctrine.
        ("exact_mirror_cannot_remain_hand_authored",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "An equivalence checker babysitting two editable copies is not convergence.",
         "Two editable copies are acceptable.",
         "version/convergence doctrine absent (exact-mirror-clause)"),
        ("generator_cannot_complete_missing_semantic_fields",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "from conventions, neighbouring rows, or Python defaults.",
         "from any convenient source.",
         "version/convergence doctrine absent (no-python-prosthetics-clause)"),
        ("subset_projection_requires_a_typed_relation",
         "docs/28_SELF_EXPLAINING_REPOSITORY.md",
         "Parsing prose headings is not an authority relation.",
         "Parsing prose headings is acceptable.",
         "version/convergence doctrine absent (subset-relation-clause)"),
        ("historical_count_cannot_masquerade_as_current_inventory", D34,
         "retained as secondary historical evidence, not a competing denominator",
         "the current competing denominator",
         "version/convergence doctrine absent (historical-framing)"),
        ("docs21_cannot_be_marked_generated_before_its_missing_fields_are_owned",
         "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
         "status: AUTHORITATIVE", "status: GENERATED",
         "may not be marked GENERATED before its missing fields are owned"),
        ("docs24_semantic_meaning_cannot_be_demoted_to_generated_projection",
         "docs/24_GAUNTLET.md",
         "status: AUTHORITATIVE", "status: GENERATED",
         "may not be marked GENERATED before its missing fields are owned"),
    ):
        probe(name, [(rel, old, new)], needle)

    # The omission hostiles must silence EVERY occurrence — the boundary
    # clauses name the identities too, which is exactly the redundancy the
    # admission law provides.
    probe("dec064_omitting_frame_version_is_rejected",
          [(DI, "BatTaggedRecordVersion FrameVersion NetBatProtocolVersion",
            "BatTaggedRecordVersion NetBatProtocolVersion"),
           (DI, "FrameVersion versions the EventFrame envelope and never the "
            ".fbat container, the payload record codec, or NetBat transport framing; ",
            "")],
          "DEC-064 forward-policy fields do not name FrameVersion")
    probe("dec064_omitting_layout_version_is_rejected",
          [(DI, "SchemaVersion LayoutVersion and Tier0QualificationArtifactVersion "
            "are distinct types",
            "SchemaVersion and Tier0QualificationArtifactVersion are distinct types"),
           (DI, "LayoutVersion versions physical organization under LayoutId "
            "and never schema meaning; ", "")],
          "DEC-064 forward-policy fields do not name LayoutVersion")

    # Marker drift flows through the universal census.
    probe("decision_ledger_marker_drift_is_rejected",
          [(D30, "DECISION-LEDGER:BEGIN generated from spec/dispositions.rs",
            "DECISION-LEDGER:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source",
          validator=audit.generated_view_findings)
    probe("legacy_coverage_marker_drift_is_rejected",
          [(D34, "LEGACY-INVARIANT-COVERAGE:BEGIN generated from spec/legacy_invariant_coverage.rs",
            "LEGACY-INVARIANT-COVERAGE:BEGIN generated from spec/architecture.rs")],
          "not its registered authority source",
          validator=audit.generated_view_findings)

    # GREEN growth — structural count-freedom evidence ONLY.
    def regen_and_check(name, edits, checks, validators):
        tmp = gate_sandbox(edits)
        try:
            _regen_operator_blocks(project, tmp)
            grown: list[str] = []
            for validator in validators:
                grown += validator(tmp)
            texts = {rel: (tmp / rel).read_text(encoding="utf-8")
                     for rel, _needle in checks}
            missing = [needle for rel, needle in checks if needle not in texts[rel]]
            if grown or missing:
                fail(f"{name} ({grown!r}, missing {missing!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    regen_and_check(
        "decision_structural_growth_is_lawful",
        [(DI, "pub const DECISIONS: &[DecisionSpec] = &[",
          "pub const DECISIONS: &[DecisionSpec] = &[\n"
          '    DecisionSpec { id: "DEC-990", class: DecisionClass::HistoricalReceipt, '
          "gates: &[], disposition: Disposition::RetainAsEvidence, "
          'subject: "Sandbox growth", successor: "structural count-freedom evidence only", '
          "stale_aliases: &[], stale_allowed_contexts: &[], replacement_contract: None },")],
        [(D30, "| DEC-990 | RETAIN-AS-EVIDENCE |"),
         ("DELIVERY_NOTES.md", "| decision rows | 83 |")],
        [audit.exact_ledger_findings])
    regen_and_check(
        "coverage_structural_growth_is_lawful",
        [(CV, '    "INV-ZERO-WARNINGS",\n];',
          '    "INV-ZERO-WARNINGS",\n    "INV-ZZZ-SANDBOX-GROWTH",\n];'),
         (CV, "pub const COVERAGE: &[LegacyInvariantCoverage] = &[",
          "pub const COVERAGE: &[LegacyInvariantCoverage] = &[\n"
          '    LegacyInvariantCoverage { legacy_id: "INV-ZZZ-SANDBOX-GROWTH", '
          "disposition: CoverageDisposition::Kill, "
          'successor: "none", rationale: "structural count-freedom evidence only." },')],
        [(D34, "| `INV-ZZZ-SANDBOX-GROWTH` | KILL |"),
         (D34, "the 116-declaration `SOURCE_INVARIANT_IDS` manifest")],
        [audit.exact_ledger_findings])
    regen_and_check(
        "version_structural_growth_is_lawful",
        [(ID, "        VersionIdentityKind::Tier0QualificationArtifact,\n    ];",
          "        VersionIdentityKind::Tier0QualificationArtifact,\n"
          "        VersionIdentityKind::Gadget,\n    ];"),
         (ID, "    Tier0QualificationArtifact,\n}",
          "    Tier0QualificationArtifact,\n    Gadget,\n}"),
         (ID, 'VersionIdentityKind::Layout => entry!("LayoutVersion", ',
          'VersionIdentityKind::Gadget => entry!("GadgetVersion", "BP-STORAGE-TILES-1"),\n'
          '            VersionIdentityKind::Layout => entry!("LayoutVersion", '),
         (DI, "SchemaVersion LayoutVersion and Tier0QualificationArtifactVersion "
          "are distinct types",
          "SchemaVersion LayoutVersion Tier0QualificationArtifactVersion and "
          "GadgetVersion are distinct types"),
         ("docs/05_STORAGE_FBAT_AND_TILES.md",
          "preserving three independent version identities.",
          "preserving three independent version identities. GadgetVersion is a "
          "sandbox-only structural-growth identity owned here.")],
        [("docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "GadgetVersion")],
        [audit.exact_ledger_findings, audit.identity_catalog_findings])

    return findings


def test_proof_relations(audit, project) -> list[str]:
    """Named hostile fixtures for 5.5E4d: the complete legacy-obligation owner,
    typed proof relations, docs/24 meaning laws, domain projections, and the
    final proof/document mirror sweep."""
    findings: list[str] = []
    root = HERE.parent
    LO, PR = "spec/legacy_obligations.rs", "spec/proof.rs"
    D21, D24 = ("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md", "docs/24_GAUNTLET.md")

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, needle, validator=None):
        tmp = gate_sandbox(edits)
        try:
            expect(name, (validator or audit.proof_relation_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.proof_relation_findings(root):
        fail(f"proof_relations_pass_on_the_real_seed "
             f"({audit.proof_relation_findings(root)!r})")

    LEDGER = "LEGACY-OBLIGATION-LEDGER does not match its typed derivation"
    RELS = "PROOF-RELATIONS does not match its typed derivation"
    DOMAIN = "PROOF-REQUIREMENTS does not match its typed derivation"
    ROW23 = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
             'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
             'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }')
    for name, rel, old, new, needle in (
        # The complete legacy owner.
        ("legacy_obligation_missing_evidence_is_rejected", LO,
         'legacy_evidence: "03_INVARIANTS.md, store writer runtime"',
         'legacy_evidence: "  "', "LEG-001 carries no legacy evidence"),
        ("legacy_obligation_missing_mechanism_disposition_is_rejected", LO,
         'mechanism_disposition: "Reimplement cleanly"',
         'mechanism_disposition: "  "', "LEG-001 carries no mechanism disposition"),
        ("planned_legacy_witness_cannot_be_empty", LO,
         'LegacyWitnessRequirement::Planned("concurrency model + duplicate-writer refusal")',
         'LegacyWitnessRequirement::Planned("  ")',
         "LEG-001 carries an empty planned witness"),
        ("canonical_legacy_witness_requires_active_proof_rows", LO,
         'id: "LEG-001", law: "One live writer and one physical commit order per journal", legacy_evidence: "03_INVARIANTS.md, store writer runtime", clean_owner: "batpak::store", mechanism_disposition: "Reimplement cleanly", witness_requirement: LegacyWitnessRequirement::Planned("concurrency model + duplicate-writer refusal")',
         'id: "LEG-001", law: "One live writer and one physical commit order per journal", legacy_evidence: "03_INVARIANTS.md, store writer runtime", clean_owner: "batpak::store", mechanism_disposition: "Reimplement cleanly", witness_requirement: LegacyWitnessRequirement::CanonicalProofRows',
         "LEG-001 claims CanonicalProofRows with no active typed relation"),
        ("planned_legacy_witness_cannot_also_have_active_proof_rows", LO,
         'id: "LEG-023"', 'id: "LEG-023X"',
         "a proof relation binds LEG-023, which no obligation row declares"),
        ("proof_row_cannot_bind_an_unknown_legacy_obligation", PR,
         'GuaranteeRef::leg("LEG-023")', 'GuaranteeRef::leg("LEG-993")',
         "a proof relation binds LEG-993, which no obligation row declares"),
        # Ledger drift.
        ("legacy_obligation_ledger_missing_row_is_rejected", D21,
         "| LEG-001 | One live writer", "| One live writer", LEDGER),
        ("legacy_obligation_ledger_order_drift_is_rejected", D21,
         "| LEG-001 |", "| LEG-001B |", LEDGER),
        ("legacy_obligation_law_drift_is_rejected", D21,
         "One live writer and one physical commit order per journal",
         "Two live writers per journal", LEDGER),
        ("legacy_obligation_evidence_drift_is_rejected", D21,
         "03_INVARIANTS.md, store writer runtime",
         "04_INVARIANTS.md, store writer runtime", LEDGER),
        ("legacy_obligation_owner_drift_is_rejected", D21,
         "| batpak::store | Reimplement cleanly |",
         "| batpak::journal | Reimplement cleanly |", LEDGER),
        ("legacy_obligation_mechanism_drift_is_rejected", D21,
         "| Reimplement cleanly |", "| Rewrite casually |", LEDGER),
        ("legacy_obligation_witness_drift_is_rejected", D21,
         "concurrency model + duplicate-writer refusal",
         "vibes-based concurrency", LEDGER),
        ("legacy_obligation_status_drift_is_rejected", D21,
         "| OnSuccessorGateClosure | Active |", "| OnSuccessorGateClosure | Closed |",
         LEDGER),
        # Proof relations.
        ("active_proof_row_requires_a_projection_contract", PR,
         'guarantee: GuaranteeRef::leg("LEG-023"), projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }',
         'guarantee: GuaranteeRef::leg("LEG-023"), projection_contracts: &[], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }',
         "names no projection contract"),
        ("active_proof_row_with_unknown_projection_contract_is_rejected", PR,
         'ContractId("BP-BVISOR-1")', 'ContractId("BP-NOBODY-1")',
         "which no authored document declares"),
        ("active_proof_row_with_duplicate_projection_contract_is_rejected", PR,
         'projection_contracts: &[ContractId("BP-BVISOR-1")]',
         'projection_contracts: &[ContractId("BP-BVISOR-1"), ContractId("BP-BVISOR-1")]',
         "repeats a projection contract"),
        ("generated_document_cannot_consume_active_proof_relation", PR,
         'ContractId("BP-MIGRATION-1")', 'ContractId("BP-GAUNTLET-1")',
         "lists automatic consumer BP-GAUNTLET-1"),
        ("proof_relation_summary_missing_row_is_rejected", D24,
         "| LEG-023 | BP-STORAGE-TILES-1 |", "| LEG-023X | BP-STORAGE-TILES-1 |",
         RELS),
        ("proof_relation_guarantee_drift_is_rejected", D24,
         "| DEC-075 | BP-SYSTEM-MODEL-1 |", "| DEC-074 | BP-SYSTEM-MODEL-1 |", RELS),
        ("proof_relation_target_drift_is_rejected", D24,
         "| LEG-043 | BP-BVISOR-1 |", "| LEG-043 | BP-SYNCBAT-1 |", RELS),
        # Domain projections.
        ("domain_proof_projection_missing_row_is_rejected",
         "docs/09_BVISOR.md",
         "descriptor_postcondition_failure_is_not_reported_applied; ", "", DOMAIN),
        ("domain_proof_projection_extra_row_is_rejected", "docs/22_MIGRATION_AND_CUTOVER.md",
         "| LEG-074 | close_reopen_reimport_returns_zero_new_events |",
         "| LEG-074 | close_reopen_reimport_returns_zero_new_events; rogue_row |",
         DOMAIN),
        ("domain_proof_projection_guarantee_drift_is_rejected",
         "docs/02_SYSTEM_MODEL.md",
         "| DEC-075 |", "| DEC-074 |", DOMAIN),
        ("docs03_hash_map_row_must_move_to_docs04", "docs/03_REPOSITORY_AND_PACKAGES.md",
         "browser_and_native_profiles_preserve_program_semantics",
         "browser_and_native_profiles_preserve_program_semantics; hash_map_iteration_cannot_influence_canonical_observables",
         "docs/03 carries the hash-map proof row"),
        # Meaning and mirror sweep.
        ("docs24_cannot_claim_proof_identity_ownership", D24,
         "`spec/proof.rs` owns proof-row identity, lifecycle, succession, guarantee binding, and projection membership; this document owns",
         "This document owns proof-row identity and executable meaning; it also owns",
         "may not claim proof-row identity ownership"),
        ("documentary_convergence_temporary_wording_cannot_return", D24,
         "## Typed proof ownership",
         "The receipt details stay in docs/24 until the documentary convergence pass.\n\n## Typed proof ownership",
         "temporary documentary-convergence wording may not return"),
        ("docs24_required_witness_inventory_cannot_return", D24,
         "## Typed proof ownership",
         "Required witnesses (proof owner TestPak; gates G2), also carried by `LEG-999`:\n\n## Typed proof ownership",
         "authored required-witnesses inventory may not return"),
        ("authored_domain_proof_inventory_cannot_return", "docs/05_STORAGE_FBAT_AND_TILES.md",
         "## Required proof rows" if False else "`spec/proof.rs` owns proof-row identity and membership.",
         "Required proof rows, projected from docs/24 (LEG-023):\n\n```text\nmiddle_event_deletion_is_rejected\n```\n\n`spec/proof.rs` owns proof-row identity and membership.",
         "authored required-proof-rows inventory may not return"),
        ("authored_legacy_obligation_row_cannot_return", D21,
         "## Closure rule",
         "| LEG-999 | Rogue law | e | o | m | w | G2 | None | Never | Active |\n\n## Closure rule",
         "authored legacy-obligation row may not return"),
        ("proof_spec_cannot_absorb_expectation_prose", PR,
         "pub struct ProofRowRecord {",
         "pub struct ProofRowRecord {\n    pub expectation: &'static str,",
         "expectation prose field may not enter the typed catalog"),
    ):
        probe(name, [(rel, old, new)], needle)

    # Retired lifecycle: successor must be active (seedcheck-side law also
    # covers it; the audit-side catalog law keeps the census honest).
    probe("retired_proof_successor_must_be_active",
          [(PR, 'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
            'successors: &[ProofRowId("pre_shred_keyset_restore_is_rejected")]')],
          "retirement path terminates in no Active identity",
          validator=audit.proof_row_catalog_findings)

    # GREEN: planned obligation growth.
    tmp = gate_sandbox([
        (LO, "pub const OBLIGATIONS: &[LegacyObligation] = &[",
         "pub const OBLIGATIONS: &[LegacyObligation] = &[\n"
         '    LegacyObligation { id: "LEG-990", law: "Sandbox growth law", '
         'legacy_evidence: "sandbox evidence", clean_owner: "batpak::sandbox", '
         'mechanism_disposition: "Sandbox only", witness_requirement: '
         'LegacyWitnessRequirement::Planned("structural count-freedom evidence only"), '
         "gates: &[GateId::G2], compatibility_disposition: CompatibilityDisposition::None, "
         "deletion_condition: DeletionCondition::OnSuccessorGateClosure, "
         "active_or_closed_status: ObligationStatus::Active },")])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.proof_relation_findings(tmp)
        d21 = (tmp / D21).read_text(encoding="utf-8")
        if grown or "| LEG-990 | Sandbox growth law | sandbox evidence |" not in d21:
            fail(f"planned_obligation_structural_growth_is_lawful ({grown!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: canonical proof growth — a new active relation flows into
    # docs/21, docs/24 relations, and the bound domain document.
    tmp = gate_sandbox([
        (PR, ROW23 + ",", ROW23 + ",\n    "
         'ProofRowRecord { id: ProofRowId("sandbox_growth_row_is_lawful"), '
         'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
         'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')])
    try:
        _regen_operator_blocks(project, tmp)
        grown = audit.proof_relation_findings(tmp)
        ok = ("sandbox_growth_row_is_lawful" in (tmp / D21).read_text(encoding="utf-8")
              and "sandbox_growth_row_is_lawful" in
              (tmp / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8"))
        # The new row lacks a docs/24 meaning entry — exactly the law that
        # makes Planned->Canonical atomic with meaning admission.
        meaning_gap = any("has no authoritative meaning" in f
                          for f in audit.witness_reference_findings(tmp))
        if grown or not ok or not meaning_gap:
            fail(f"canonical_proof_growth_is_lawful ({grown!r}, projected={ok}, "
                 f"meaning_law={meaning_gap})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: retired succession — a retired row leaves the projections while
    # its successor carries the relation.
    tmp = gate_sandbox([
        (PR, ROW23,
         'ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
         'state: ProofRowState::Retired { successors: '
         '&[ProofRowId("event_reorder_is_rejected")] } }')])
    try:
        _regen_operator_blocks(project, tmp)
        d05 = (tmp / "docs/05_STORAGE_FBAT_AND_TILES.md").read_text(encoding="utf-8")
        if "middle_event_deletion_is_rejected" in audit.batql_extract_block(
                d05, "PROOF-REQUIREMENTS"):
            fail("retired_succession_leaves_current_projections")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    return findings


def test_numeric(audit) -> list[str]:
    """Named hostile fixtures for the numeric authority contract (docs/37)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    markers = [marker for _name, marker in audit.NUMERIC_LAW_MARKERS]
    if audit.numeric_law_findings("\n".join(markers)):
        fail("all_numeric_laws_present_passes")
    for name, marker in audit.NUMERIC_LAW_MARKERS:
        reduced = "\n".join(m for m in markers if m != marker)
        if not any(name in item for item in audit.numeric_law_findings(reduced)):
            fail(f"numeric_law_{name}_absence_is_rejected")

    # OperatorSpec arithmetic row lacking an approximate-support posture is rejected
    arith_no_posture = [{
        "id": "OP-ADD", "class": "Arithmetic", "shape": "SymbolOnly",
        "canonical": "+", "alias": "",
        "semantic_op": "s", "arity": "Binary", "fixity": "Infix", "precedence": "60",
        "associativity": "Left", "input_sorts": "a", "result_sort": "b", "exactness": "Exact",
        "overflow": "o", "exception": "e", "spoken": "plus",
        "mutation_classes": "m", "numeric_support": "NotApplicable",
    }]
    if not any("lacks an approximate-support posture" in x for x in audit.batql_operator_fact_findings(arith_no_posture)):
        fail("operatorspec_arithmetic_row_lacking_approx_support_is_rejected")

    # An operator that admits raw approximate operands (QualifiedProfileOnly) with no
    # qualified numeric profile in V1 is rejected; ExactSupported never admits them.
    admits_raw_approx = [{**arith_no_posture[0], "numeric_support": "QualifiedProfileOnly"}]
    if not any("no qualified numeric profile exists" in x for x in audit.batql_operator_fact_findings(admits_raw_approx)):
        fail("operator_admitting_raw_approximate_without_profile_is_rejected")

    return findings


def test_guarantees(audit, project) -> list[str]:
    """Named hostile fixtures for the Guarantee Graph (DEC-070)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def seed(sid, kind="SemanticLaw", life="Permanent", owner="o", witness="w"):
        return {"id": sid, "kind": kind, "lifetime": life, "owner": owner, "gates": "",
                "witness": witness, "failure": "f",
                "rel": {"DerivesFrom": [], "Refines": [], "Discharges": [], "Supersedes": []}}

    def node(nid, family="SEED", kind="SemanticLaw", life="Permanent", owner="o", gates="",
             witness="RowDeclared"):
        # Post-5.5D4b node shape: an admitted view carries a typed gate posture and
        # a witness posture, never a bare gate string plus free witness text.
        return {"id": nid, "family": family, "kind": kind, "lifetime": life,
                "owner": owner, "gate_posture": gates, "target": "", "witness": witness}

    good = [seed(f"SEED-{i}") for i in range(32)]
    if audit.guarantee_classification_findings(good):
        fail("valid_classification_passes")
    if not audit.guarantee_classification_findings(good[:31]):
        fail("missing_seed_row_is_rejected")
    if not audit.guarantee_classification_findings(good + [seed("SEED-32")]):
        fail("extra_seed_row_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "kind": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_kind_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "lifetime": "Bogus"}] + good[1:]):
        fail("wrong_guarantee_lifetime_is_rejected")
    if not audit.guarantee_classification_findings([{**good[0], "owner": ""}] + good[1:]):
        fail("unclassified_seed_row_is_rejected")

    ids = {"A", "B", "C"}
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "Z")]):
        fail("dangling_relation_target_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "A")]):
        fail("self_edge_is_rejected")
    if not audit.guarantee_relation_findings(ids, [("A", "DerivesFrom", "B"), ("A", "DerivesFrom", "B")]):
        fail("duplicate_edge_is_rejected")
    for fam in ("DerivesFrom", "Refines", "Supersedes", "Discharges"):
        if not any("cycle" in x for x in audit.guarantee_relation_findings(ids, [("A", fam, "B"), ("B", fam, "A")])):
            fail(f"{fam.lower()}_cycle_is_rejected")

    if not any("Permanent SemanticLaw names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("SEED-X", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_semanticlaw_without_witness_is_rejected")
    if not any("Permanent ArchitectureConstraint names no witness" in x
               for x in audit.guarantee_lifetime_findings([node("A", kind="ArchitectureConstraint", witness="NoFamilyWitness")], {}, [])):
        fail("permanent_architecture_without_witness_is_rejected")
    if audit.guarantee_lifetime_findings([node("D", kind="Decision", witness="NoFamilyWitness")], {}, []):
        fail("permanent_decision_witness_deferred_to_c2")
    if audit.guarantee_lifetime_findings([node("Q", kind="QualificationRequirement", witness="NoFamilyWitness")], {}, []):
        fail("permanent_qualification_is_self_witnessing")
    if not any("UntilGate names no active gate schedule" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilGate", gates="")], {}, [])):
        fail("untilgate_without_gate_is_rejected")
    if not any("HistoricalCoverageOnly cannot be actively scheduled" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="HistoricalCoverageOnly", gates="G2")], {}, [])):
        fail("historicalcoverageonly_carrying_active_gate_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor", owner="batpak::store", gates="G2")], {}, [])):
        fail("untilsuccessor_without_typed_successor_is_rejected")
    if audit.guarantee_lifetime_findings([node("X", life="UntilSuccessor")], {}, [("Y", "Supersedes", "X")]):
        fail("untilsuccessor_with_resolvable_successor_passes")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "G2")]):
        fail("gate_name_cannot_become_successor_edge")
    if not audit.guarantee_relation_findings({"LEG-1"}, [("LEG-1", "Supersedes", "batpak::store")]):
        fail("clean_owner_text_cannot_become_successor_edge")

    def legnode(life, gates="G2"):
        return node("LEG-1", family="LEG", kind="LegacyObligation", life=life, gates=gates, witness="")

    active_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Active"}}
    if not any("must not force Permanent" in x for x in audit.guarantee_lifetime_findings([legnode("Permanent")], active_never, [])):
        fail("deletion_never_forcing_permanent_is_rejected")
    if not any("no resolvable typed successor" in x
               for x in audit.guarantee_lifetime_findings([legnode("UntilSuccessor")], active_never, [])):
        fail("gated_leg_without_typed_successor_cannot_project_untilsuccessor")
    closed_never = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "Never", "status": "Closed"}}
    if audit.guarantee_lifetime_findings([legnode("ClosedEvidence", gates="")], closed_never, []):
        fail("deletion_never_closed_leg_projects_closedevidence")
    if not any("Active LEG projects as ClosedEvidence" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], active_never, [])):
        fail("closedevidence_marked_active_is_rejected")
    closed = {"LEG-1": {"owner": "o", "gate": "G2", "compat": "None", "deletion": "OnSuccessorGateClosure", "status": "Closed"}}
    if not any("still names an implementation gate" in x
               for x in audit.guarantee_lifetime_findings([legnode("ClosedEvidence")], closed, [])):
        fail("closed_leg_still_gating_is_rejected")

    both = {"SEED-X", "LEG-1"}
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Discharges", "LEG-1")])):
        fail("bare_law_discharges_active_leg_is_rejected")
    if not any("requires qualifying evidence" in x
               for x in audit.guarantee_relation_findings(both, [("SEED-X", "Closes", "LEG-1")])):
        fail("bare_law_closes_active_leg_is_rejected")
    if audit.guarantee_relation_findings(both, [("SEED-X", "Refines", "LEG-1")]):
        fail("refines_active_leg_passes")

    root = HERE.parent
    def shape(n):
        return (n["id"], n["family"], n["kind"], n["lifetime"], n["owner"],
                n["gate_posture"], n["target"], n["witness"])

    proj_nodes = [shape(n) for n in project.guarantee_nodes(root)]
    audit_nodes = [shape(n) for n in audit.guarantee_derive(root)[0]]
    if proj_nodes != audit_nodes:
        fail("generator_auditor_guarantee_disagreement")
    nodes, edges, _adm = audit.guarantee_derive(root)
    if nodes != sorted(nodes, key=lambda n: (audit.G_FAMILY_RANK[n["family"]], n["id"])):
        fail("source_reorder_changes_guarantee_order")
    if any(not src.startswith("SEED-") for src, _k, _t in edges):
        fail("non_seed_source_guarantee_edge_present")

    return findings


# --- Guarded mutation: a claimed negative test is itself a claim -------------
# A red light caused by the wrong severed wire is not evidence that the alarm
# works. Every mutation below proves four things: the target existed, the bytes
# actually changed, the intended validator ran, and the EXPECTED finding class
# appeared -- never merely that something somewhere returned nonzero.
class ProbeError(AssertionError):
    """The probe itself is broken: absent target or inert mutation."""


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
    for rel in ("rust-toolchain.toml", "README.md", "DELIVERY_NOTES.md",
                "AGENTS.md", "FINAL_RECONCILIATION.md"):
        if (root / rel).is_file():
            shutil.copy2(root / rel, tmp / rel)
    for rel, old, new in edits:
        path = tmp / rel
        text = path.read_text(encoding="utf-8")
        path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
    return tmp


def test_gates(audit, project) -> list[str]:
    """Named hostile fixtures for the one gate identity (5.5C2a)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        # The expected finding class must appear, not merely some error.
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def guarantee_findings(tmp):
        out: list[str] = []
        audit.check_guarantees(tmp, out)
        return out

    # --- typed gate list rules ---
    expect("unknown_gateid_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G99"]), "unknown GateId")
    expect("duplicate_gateid_in_one_fact_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G2", "G2"]), "duplicate GateId")
    expect("noncanonical_gate_order_is_rejected",
           audit.gate_findings(root, "SEED", "X", ["G5", "G2"]), "not in canonical order")
    if audit.gate_findings(root, "SEED", "X", ["G2", "G5"]):
        fail("canonical_gate_list_passes")

    # --- inventory integrity ---
    probe("gateid_missing_from_inventory_is_rejected",
          [("spec/gates.rs",
            '    GateSpec { id: GateId::G9, token: "G9", title: "Self-hosting and release seal" },',
            "")],
          audit.gate_inventory_findings, "missing from the GATES inventory")
    probe("inventory_token_duplicated_across_two_gateids_is_rejected",
          [("spec/gates.rs",
            'GateSpec { id: GateId::G1, token: "G1", title: "MacBat" }',
            'GateSpec { id: GateId::G1, token: "G0", title: "MacBat" }')],
          audit.gate_inventory_findings, "one token under two GateIds")

    # --- migration conservation: a SEED/LEG gate set may not move ---
    # None means the row no longer passes typed admission (5.5D4b). Dropping a
    # required gate list is still detected -- more sharply than before, because
    # the row cannot be admitted at all rather than projecting an empty string.
    def gates_of(tmp, ident):
        return {n["id"]: n["gate_posture"] for n in audit.guarantee_derive(tmp)[0]}.get(ident)

    base_seed = gates_of(root, "SEED-PAKVM-NAME")
    tmp = gate_sandbox([("spec/invariants.rs",
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5]',
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0]')])
    if gates_of(tmp, "SEED-PAKVM-NAME") == base_seed:
        fail("seed_gate_dropped_during_migration_is_detected")
    shutil.rmtree(tmp, ignore_errors=True)

    tmp = gate_sandbox([("spec/invariants.rs",
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5]',
                         'owner: "docs/07_PAKVM_ISA.md", gates: &[GateId::G0, GateId::G5, GateId::G7]')])
    if gates_of(tmp, "SEED-PAKVM-NAME") == base_seed:
        fail("seed_gate_added_during_migration_is_detected")
    shutil.rmtree(tmp, ignore_errors=True)

    base_leg = gates_of(root, "LEG-001")
    tmp = gate_sandbox([("spec/legacy_obligations.rs",
                         'Planned("concurrency model + duplicate-writer refusal"), gates: &[GateId::G2]',
                         'Planned("concurrency model + duplicate-writer refusal"), gates: &[]')])
    if gates_of(tmp, "LEG-001") == base_leg:
        fail("leg_gate_dropped_during_migration_is_detected")
    expect("emptied_gate_list_is_rejected",
           audit.gate_reference_findings(tmp), "names no gate")
    shutil.rmtree(tmp, ignore_errors=True)

    probe("leg_gate_added_during_migration_is_rejected",
          [("spec/legacy_obligations.rs",
            'Planned("historical vectors + future-version refusal"), gates: &[GateId::G2]',
            'Planned("historical vectors + future-version refusal"), gates: &[GateId::G2, GateId::G2]')],
          audit.gate_reference_findings, "duplicate GateId")

    # --- the retired string representation may not survive anywhere ---
    probe("old_string_gate_field_in_seed_is_rejected",
          [("spec/invariants.rs", "pub gates: &'static [GateId],", 'pub gates: "G0",')],
          audit.gate_reference_findings, "still carries a string gate field")
    probe("old_string_gate_field_in_leg_is_rejected",
          [("spec/legacy_obligations.rs", "pub gates: &'static [GateId],", 'pub gate: "G2",')],
          audit.gate_reference_findings, "still carries a string gate field")

    # --- generated projections may not drift from the typed authority ---
    probe("docs25_gate_inventory_drift_is_rejected",
          [("docs/25_IMPLEMENTATION_GATES.md",
            "| G4 | G4 | BatQL compiler |", "| G4 | G4 | BatQL compilerr |")],
          audit.gate_doc_findings, "does not equal spec/gates.rs")
    probe("docs23_rendered_seed_gate_drift_is_rejected",
          [("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
            "| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 |",
            "| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0 |")],
          guarantee_findings, "SEED-CLASSIFICATION block does not equal")
    probe("guarantee_graph_gate_metadata_drift_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| LEG-001 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 |",
            "| LEG-001 | LEG | LegacyObligation | UntilGate | - | batpak::store | G7 |")],
          guarantee_findings, "node table does not equal")
    probe("docs21_rendered_leg_gate_drift_is_rejected",
          [("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "| concurrency model + duplicate-writer refusal | G2 |",
            "| concurrency model + duplicate-writer refusal | G3 |")],
          lambda tmp: [f for f in (lambda o: (audit.check_seed_ids(tmp, o), o)[1])([])],
          "legacy seed/document mismatch")

    # --- generator and auditor agree; source order does not change bytes ---
    if [(n["id"], n["gate_posture"]) for n in project.guarantee_nodes(root)] != \
       [(n["id"], n["gate_posture"]) for n in audit.guarantee_derive(root)[0]]:
        fail("generator_auditor_gate_disagreement")
    if [g[0] for g in project.gate_inventory(root)] != [g[0] for g in audit.gate_inventory(root)]:
        fail("generator_auditor_inventory_disagreement")
    inv = [g[0] for g in audit.gate_inventory(root)]
    if inv != sorted(inv, key=audit.gate_enum_variants(root).index):
        fail("gate_inventory_is_canonically_ordered")

    # --- the probe harness itself fails closed ---
    try:
        must_replace("abc", "zzz", "q", "self-check")
    except ProbeError:
        pass
    else:
        fail("must_replace_fails_closed_on_absent_target")
    try:
        must_replace("abc", "abc", "abc", "self-check")
    except ProbeError:
        pass
    else:
        fail("must_replace_fails_closed_on_inert_mutation")

    return findings


def test_decisions(audit, project) -> list[str]:
    """Named hostile fixtures for DecisionClass and gate binding (DEC-072)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, edits, validator, needle):
        tmp = gate_sandbox(edits)
        try:
            expect(name, validator(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # A row losing its class is a parse-visible defect, not a silent default.
    probe("missing_decision_class_is_rejected",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0], ',
            'DecisionSpec { id: "DEC-016", ')],
          audit.decision_class_findings, "carry no DecisionClass")
    probe("unknown_decision_class_is_rejected",
          [("spec/dispositions.rs", "class: DecisionClass::Enforcement, gates: &[GateId::G0], "
                                    'disposition: Disposition::Keep, subject: "Status/supersession metadata"',
            "class: DecisionClass::Bogus, gates: &[GateId::G0], "
            'disposition: Disposition::Keep, subject: "Status/supersession metadata"')],
          audit.decision_class_findings, "unknown DecisionClass")

    # Each implementation-bearing class must name a gate. One representative row
    # per class, so a class-specific hole cannot hide behind its neighbours.
    for cls, did, gates in (
        ("Architecture", "DEC-007", "&[GateId::G0]"),
        ("Capability", "DEC-017", "&[GateId::G6]"),
        ("Compatibility", "DEC-020", "&[GateId::G2]"),
        ("Enforcement", "DEC-016", "&[GateId::G0]"),
        ("ImplementationPosture", "DEC-033", "&[GateId::G8]"),
    ):
        probe(f"{cls.lower()}_decision_without_gate_is_rejected",
              [("spec/dispositions.rs",
                f'id: "{did}", class: DecisionClass::{cls}, gates: {gates}',
                f'id: "{did}", class: DecisionClass::{cls}, gates: &[]')],
              audit.decision_class_findings, "names no implementation or qualification gate")

    probe("unknown_gateid_on_a_decision_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0]',
            'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G12]')],
          audit.decision_class_findings, "unknown GateId")
    probe("duplicate_gateid_on_a_decision_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0]',
            'id: "DEC-016", class: DecisionClass::Enforcement, gates: &[GateId::G0, GateId::G0]')],
          audit.decision_class_findings, "duplicate GateId")
    probe("noncanonical_decision_gate_order_is_rejected",
          [("spec/dispositions.rs", 'id: "DEC-072", class: DecisionClass::Enforcement, gates: &[GateId::G0, GateId::G9]',
            'id: "DEC-072", class: DecisionClass::Enforcement, gates: &[GateId::G9, GateId::G0]')],
          audit.decision_class_findings, "not in canonical order")

    # An ungated Naming/HistoricalReceipt row is lawful and must stay green.
    if audit.decision_class_findings(root):
        fail("ungated_naming_and_historical_receipt_rows_pass")

    def seed_id_findings(tmp):
        out: list[str] = []
        audit.check_seed_ids(tmp, out)
        return out

    probe("docs30_decision_class_mismatch_is_rejected",
          [("docs/30_DECISION_AND_REJECTION_LEDGER.md",
            "| DEC-016 | KEEP | Enforcement | G0 |", "| DEC-016 | KEEP | Capability | G0 |")],
          seed_id_findings, "decision seed/document mismatch for DEC-016")
    probe("docs30_decision_gate_mismatch_is_rejected",
          [("docs/30_DECISION_AND_REJECTION_LEDGER.md",
            "| DEC-016 | KEEP | Enforcement | G0 |", "| DEC-016 | KEEP | Enforcement | G5 |")],
          seed_id_findings, "decision seed/document mismatch for DEC-016")

    def guarantee_findings(tmp):
        out: list[str] = []
        audit.check_guarantees(tmp, out)
        return out

    probe("guarantee_graph_losing_decision_class_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| DEC-016 | DEC | Decision | Permanent | Enforcement |",
            "| DEC-016 | DEC | Decision | Permanent | - |")],
          guarantee_findings, "node table does not equal")
    probe("guarantee_graph_losing_a_decision_gate_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "| DEC-072 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0/G9 |",
            "| DEC-072 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 |")],
          guarantee_findings, "node table does not equal")

    # Decision gates are metadata. They never become edges, and no gate or
    # profile node is synthesized to hold them.
    nodes, edges, _adm = audit.guarantee_derive(root)
    families = {n["family"] for n in nodes}
    if families != {"SEED", "LEG", "DEC", "ARCH", "QUAL"}:
        fail(f"unexpected guarantee family present: {sorted(families)}")
    node_ids = {n["id"] for n in nodes}
    inv_tokens = {g[1] for g in audit.gate_inventory(root)}
    if node_ids & inv_tokens:
        fail("synthetic gate node present in the Guarantee Graph")
    for s, k, t in edges:
        if s in inv_tokens or t in inv_tokens:
            fail(f"synthetic gate edge present: {s} {k} {t}")
    for profile in ("InternalConsistency", "SignedHistory", "ExternallyAnchoredHistory"):
        if profile in node_ids:
            fail(f"authenticated-history profile added as a graph node: {profile}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision node count is not 82")

    if [(n["id"], n.get("dclass"), n["gate_posture"]) for n in project.guarantee_nodes(root) if n["family"] == "DEC"] != \
       [(n["id"], n.get("dclass"), n["gate_posture"]) for n in audit.guarantee_derive(root)[0] if n["family"] == "DEC"]:
        fail("generator_auditor_decision_metadata_disagreement")
    return findings


def test_authenticated_history(audit) -> list[str]:
    """Named hostile fixtures for the authenticated-history contract (DEC-071).

    Structural only. Bootstrap performs no signature, accumulator, witness,
    freshness, or rollback verification, and these fixtures do not pretend
    otherwise: they prove the CONTRACT cannot be weakened, not that crypto works.
    """
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, old, new, needle, validator=None):
        tmp = gate_sandbox([("spec/architecture.rs", old, new)])
        try:
            expect(name, (validator or audit.authenticated_history_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.authenticated_history_findings(root):
        fail("frozen_matrix_and_bundles_pass")

    # --- the four axes are independently representable ---------------------
    # This is the defect C2c corrects: one mutually exclusive posture could not
    # state integrity AND rollback-resistance at once.
    bundles = audit.claim_bundles(root)
    want = {
        "INTERNAL_CONSISTENCY_SUCCESS":
            ("InternalConsistencyVerified", "NotClaimed", "NotClaimed", "Unavailable"),
        "SIGNED_UNANCHORED_SUCCESS":
            ("InternalConsistencyVerified", "SignedHistoryVerified", "NotClaimed", "Unavailable"),
        "WITNESSED_SUCCESS":
            ("InternalConsistencyVerified", "SignedHistoryVerified",
             "WitnessedGenerationVerified", "ScopedToVerifiedWitness"),
    }
    for name, axes in want.items():
        got = bundles.get(name)
        if got is None:
            fail(f"{name.lower()}_bundle_exists")
        elif got != axes:
            fail(f"{name.lower()}_encodes_all_four_axes (wanted {axes}, got {got})")
    # Each axis must be independently readable, not a summary of the others.
    if bundles.get("INTERNAL_CONSISTENCY_SUCCESS", ("",))[0] != "InternalConsistencyVerified" or \
       bundles.get("INTERNAL_CONSISTENCY_SUCCESS", ("", "", "", ""))[3] != "Unavailable":
        fail("internal_consistency_success_encodes_integrity_and_rollback_unavailable_together")

    probe("history_claim_posture_reintroduced_is_rejected",
          "pub enum IntegrityClaim {", "pub enum HistoryClaimPosture {",
          "retired claim vocabulary HistoryClaimPosture")
    probe("claim_posture_field_reintroduced_is_rejected",
          "    pub integrity: IntegrityClaim,", "    pub claim_posture: IntegrityClaim,",
          "retired claim vocabulary claim_posture")
    probe("security_ladder_is_rejected",
          "pub enum IntegrityClaim {", "pub enum SecurityLevel {",
          "generic security ladder SecurityLevel")

    # --- no fallback success for a required witness ------------------------
    # The profile table dissolved (5.5E2): the mutation now attacks the arm of
    # the const fn that owns the fact, exactly where a real weakening would land.
    probe("externally_anchored_given_unanchored_success_is_rejected",
          "            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),\n"
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => None,",
          "            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),\n"
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => Some(SIGNED_UNANCHORED_SUCCESS),",
          "must refuse, not fall back to a weaker success")

    # --- per-axis claim ceilings -------------------------------------------
    probe("internal_consistency_claiming_signed_authenticity_is_rejected",
          "pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::NotClaimed,",
          "pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,",
          "!= frozen bundle")
    probe("internal_consistency_claiming_freshness_is_rejected",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "drift apart")
    probe("internal_consistency_claiming_scoped_rollback_resistance_is_rejected",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "    authenticity: AuthenticityClaim::NotClaimed,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "drift apart")
    probe("unanchored_signed_history_claiming_freshness_is_rejected",
          "pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,\n"
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {\n"
          "    integrity: IntegrityClaim::InternalConsistencyVerified,\n"
          "    authenticity: AuthenticityClaim::SignedHistoryVerified,\n"
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "unanchored success claims freshness or rollback resistance")

    # --- freshness and rollback resistance may not drift apart -------------
    probe("freshness_verified_while_rollback_unavailable_is_rejected",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::Unavailable,",
          "drift apart")
    probe("rollback_scoped_while_freshness_not_claimed_is_rejected",
          "    freshness: FreshnessClaim::WitnessedGenerationVerified,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "    freshness: FreshnessClaim::NotClaimed,\n"
          "    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,",
          "drift apart")
    probe("success_bundle_without_verified_integrity_is_rejected",
          "pub enum IntegrityClaim {\n    InternalConsistencyVerified,\n}",
          "pub enum IntegrityClaim {\n    InternalConsistencyVerified,\n    NotClaimed,\n}",
          "!= frozen")

    # --- invalid profile/policy pairs are refused, never normalized --------
    probe("internal_consistency_with_optional_witness_is_rejected",
          "            AuthenticatedHistoryProfile::InternalConsistency => &[WitnessPolicy::None],",
          "            AuthenticatedHistoryProfile::InternalConsistency => "
          "&[WitnessPolicy::None, WitnessPolicy::Optional],",
          "frozen matrix says")
    probe("signed_history_with_required_witness_is_rejected",
          "            AuthenticatedHistoryProfile::SignedHistory => {\n"
          "                &[WitnessPolicy::None, WitnessPolicy::Optional]\n"
          "            }",
          "            AuthenticatedHistoryProfile::SignedHistory => &[WitnessPolicy::Required],",
          "permits WitnessPolicy::Required outside ExternallyAnchoredHistory")
    probe("externally_anchored_with_optional_witness_is_rejected",
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Required],",
          "            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Optional],",
          "permits a non-Required witness policy")

    # --- witness axes stay distinct ---------------------------------------
    for variant in ("NotProvided", "Stale", "Conflicting", "Unverifiable",
                    "CryptographicallyInvalid", "LineageMismatch", "GenerationMismatch",
                    "AccumulatorMismatch"):
        probe(f"required_witness_{variant.lower()}_dropped_from_failure_set_is_rejected",
              f"    WitnessDisposition::{variant},\n", "", "!= frozen failure set")
    probe("witness_disposition_collapsed_into_valid_invalid_is_rejected",
          "    /// Supplied and contradicts the observed history.\n    Conflicting,\n", "",
          "collapses required distinctions")
    probe("optional_witness_absent_incorrectly_failing_is_rejected",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n    WitnessDisposition::Stale,",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n"
          "    WitnessDisposition::NotProvided,\n    WitnessDisposition::Stale,",
          "refuses an absent optional witness")
    probe("present_invalid_optional_witness_treated_as_absent_is_rejected",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n    WitnessDisposition::Stale,\n",
          "pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[\n",
          "may degrade to absence or success")
    probe("refusal_partial_evidence_law_removed_is_rejected",
          "pub const REFUSAL_PARTIAL_CLAIM_LAW", "pub const REFUSAL_PARTIAL_CLAIM_LAW_RENAMED",
          "states no refusal/partial-evidence law")
    return findings


def test_claim_receipt_law(audit) -> list[str]:
    """docs/14 must carry each claim axis separately (DEC-071)."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    cases = [
        ("docs14_missing_integrity_claim_is_rejected", "IntegrityClaim",
         "receipt preserves the integrity claim"),
        ("docs14_missing_authenticity_claim_is_rejected", "AuthenticityClaim",
         "receipt preserves the authenticity claim"),
        ("docs14_missing_freshness_claim_is_rejected", "FreshnessClaim",
         "receipt preserves the freshness claim"),
        ("docs14_missing_rollback_resistance_claim_is_rejected", "RollbackResistanceClaim",
         "receipt preserves the rollback-resistance claim"),
        ("docs14_omitting_success_versus_refusal_is_rejected", "success or refusal outcome",
         "receipt states success versus refusal"),
        ("docs14_allowing_axis_inference_from_profile_name_is_rejected",
         "may infer an omitted claim axis", "no axis inferred from a profile name"),
        ("docs14_refusal_forced_into_success_bundle_is_rejected",
         "A refusal is never forced into one", "a refusal is not a weaker success"),
        ("docs14_partial_evidence_claiming_freshness_is_rejected",
         "never carry a freshness or scoped rollback-resistance claim after witness verification failed",
         "partial evidence never claims freshness after witness failure"),
        ("docs14_required_witness_fallback_success_is_rejected",
         "no successful unanchored result at all",
         "no successful unanchored result for a required witness"),
        ("docs14_optional_absent_and_invalid_rendering_identically_is_rejected",
         "An absent optional witness is a successful unanchored result",
         "absent and invalid optional witnesses never render identically"),
    ]
    for name, old, needle in cases:
        tmp = gate_sandbox([("docs/14_RECEIPTS_AND_EXPLANATION.md", old, "")])
        try:
            produced = doc_findings(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_document_law(audit) -> list[str]:
    """The rollback threat and the anti-flattening axes must stay in the docs."""
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    for name, rel, old, needle in (
        ("whole_store_rollback_threat_removed_is_rejected",
         "docs/19_SECURITY_MODEL.md",
         "may restore an older complete generation whose internal commitments and signatures remain valid",
         "whole-store rollback threat"),
        ("integrity_authenticity_freshness_collapsed_is_rejected",
         "docs/19_SECURITY_MODEL.md",
         "rollback resistance  restoring an older valid generation is detectable",
         "four axes"),
        ("receipt_losing_exact_profile_is_rejected",
         "docs/14_RECEIPTS_AND_EXPLANATION.md", "selected AuthenticatedHistoryProfile",
         "receipt preserves the exact profile"),
        ("receipt_losing_exact_witness_disposition_is_rejected",
         "docs/14_RECEIPTS_AND_EXPLANATION.md",
         "selected WitnessPolicy\nexact WitnessDisposition",
         "receipt preserves the exact witness disposition"),
    ):
        tmp = gate_sandbox([(rel, old, "")])
        try:
            produced = doc_findings(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_control_characters(audit) -> list[str]:
    """An invisible byte impersonating a regular expression must be rejected."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    # The exact 5.5C2a defect: a backspace where a word boundary belongs.
    tmp = Path(tempfile.mkdtemp())
    (tmp / "spec").mkdir()
    (tmp / "spec" / "poisoned.rs").write_bytes(
        b'if re.search(r\'\x08gates?:\\s*"\', src):\n'
    )
    produced = audit.control_character_findings(tmp)
    if not any("forbidden control character U+0008" in f for f in produced):
        fail(f"backspace_in_a_validator_regex_is_rejected (got {produced!r})")
    shutil.rmtree(tmp, ignore_errors=True)

    for name, payload, code in (
        ("nul_byte_is_rejected", b"a\x00b\n", "U+0000"),
        ("vertical_tab_is_rejected", b"a\x0bb\n", "U+000B"),
        ("form_feed_is_rejected", b"a\x0cb\n", "U+000C"),
        ("escape_byte_is_rejected", b"a\x1bb\n", "U+001B"),
        ("carriage_return_is_rejected", b"a\r\nb\n", "U+000D"),
        ("delete_byte_is_rejected", b"a\x7fb\n", "U+007F"),
    ):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "spec").mkdir()
        (tmp / "spec" / "x.rs").write_bytes(payload)
        produced = audit.control_character_findings(tmp)
        if not any(code in f for f in produced):
            fail(f"{name} (wanted {code}, got {produced!r})")
        shutil.rmtree(tmp, ignore_errors=True)

    # LF and TAB are lawful and must not be reported.
    tmp = Path(tempfile.mkdtemp())
    (tmp / "spec").mkdir()
    (tmp / "spec" / "x.rs").write_bytes(b"fn main() {\n\tlet x = 1;\n}\n")
    if audit.control_character_findings(tmp):
        fail("lf_and_tab_are_permitted")
    shutil.rmtree(tmp, ignore_errors=True)

    # The real tree stays clean.
    if audit.control_character_findings(root):
        fail(f"repository_text_is_control_character_clean ({audit.control_character_findings(root)[:3]})")
    return findings


def test_substrate(audit) -> list[str]:
    """Named hostile fixtures for the 5.5D1 substrate primitives.

    Structural only. Bootstrap performs no storage commit, traversal, network
    call, generated dispatch, cancellation, or reconciliation, and these fixtures
    do not claim executable semantics were tested.
    """
    findings: list[str] = []

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    def probe(name, rel, old, needle, new="", validator=None):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, (validator or doc_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    LEG = "spec/legacy_obligations.rs"

    # --- publication algebra: the three axes stay orthogonal ---------------
    probe("post_commit_result_changed_to_generic_failure_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "committed                        is NOT failed",
          "post-commit publication is not an ordinary failure")
    probe("committed_receipt_incomplete_collapsed_into_failure_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "committed, receipt incomplete    is NOT failed, and is NOT outcome unknown",
          "committed with an incomplete receipt is not failure or unknown")
    probe("outcome_unknown_collapsed_into_not_committed_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "outcome unknown                  is NOT proof of non-commit",
          "outcome unknown is not proof of non-commit")
    probe("cancelled_after_admission_treated_as_non_commit_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "cancelled after admission        is NOT proof of non-commit",
          "cancelled after admission is not proof of non-commit")
    probe("reconciliation_rewriting_durable_evidence_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "It never rewrites the original durable event",
          "reconciliation appends evidence and never rewrites durable history")
    probe("commit_knowledge_axis_removed_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "commit knowledge        KnownAbsent | KnownCommitted | Unknown",
          "commit knowledge is distinct from receipt completeness and reconciliation")
    probe("generic_kind_plus_bytes_publication_made_canonical_is_rejected",
          "docs/05_STORAGE_FBAT_AND_TILES.md",
          "There is no `GenericPublicationCommand { kind: String, payload: Vec<u8> }`",
          "no generic untyped publication envelope")

    # --- typed batch atomicity --------------------------------------------
    probe("atomic_batch_allowing_partial_durable_publication_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md", "partial durable subset   forbidden",
          "an atomic group never publishes a partial durable subset")
    probe("arbitrary_cross_port_atomic_transaction_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "not an arbitrary distributed transaction across unrelated ports or services",
          "a typed batch is not a distributed transaction")
    probe("batch_receipt_order_decoupled_from_input_order_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md", "Input order and receipt order correspond exactly",
          "input order and receipt order correspond")
    probe("leg037_capability_bound_atomicity_weakened_is_rejected",
          LEG, "only where the admitting capability owns their common authority boundary",
          "LEG-037 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg037_partial_durable_subset_permitted_is_rejected",
          LEG, "never publishes a partial durable subset",
          "LEG-037 law no longer states", validator=audit.d1_substrate_findings)

    # --- logical operation versus physical attempt ------------------------
    probe("logical_operation_identity_and_attemptid_collapsed_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "LogicalOperationId   stable across retries of the same requested operation",
          "logical operation identity is stable across retries")
    probe("attempt_state_and_commit_knowledge_merged_into_one_outcome_is_rejected",
          "docs/08_SYNCBAT_RUNTIME.md",
          "never collapse into one `Outcome`, boolean, or success/failure field",
          "attempt state, logical outcome, commit knowledge, and receipt completeness stay separate")
    probe("cancelled_before_admission_distinction_removed_is_rejected",
          "docs/09_BVISOR.md", "CancelledBeforeAdmission   no physical execution was admitted",
          "cancellation before admission is distinct")
    probe("leg042_cancellation_split_weakened_is_rejected",
          LEG, "distinguishes cancellation before admission from cancellation after admission",
          "LEG-042 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg042_retry_erasing_prior_attempt_evidence_is_rejected",
          LEG, "receives a new AttemptId without erasing prior attempt evidence",
          "LEG-042 law no longer states", validator=audit.d1_substrate_findings)

    # --- bounded traversal --------------------------------------------------
    probe("cursor_losing_generation_binding_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
          "A cursor binds to its traversal identity, including store lineage, generation",
          "the cursor binds generation")
    probe("cursor_losing_source_cut_binding_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "source cut, direction, selector, and filters",
          "the cursor binds the source cut")
    probe("cursor_transplant_across_selector_accepted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
          "fails closed rather than resuming against a traversal it never described",
          "a transplanted cursor fails closed")
    probe("page_completeness_omitted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "page completeness or partiality posture",
          "the page carries a completeness posture")
    probe("work_observation_omitted_is_rejected",
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md", "WorkObservation",
          "the page carries a source stamp and work observation")
    probe("limit_applied_after_decode_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "constrain discovery **before** unbounded decode",
          "the limit applies before decode")
    probe("scan_then_truncate_called_bounded_is_rejected",
          "docs/13_BATQL_CONTRACT.md",
          "scan everything -> decode everything -> truncate the returned vector",
          "scan-then-truncate is not bounded traversal")
    probe("leg028_limit_before_decode_weakened_is_rejected",
          LEG, "constrain discovery before decode",
          "LEG-028 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg029_one_shot_equivalence_weakened_is_rejected",
          LEG, "concatenated valid pages equal a one-shot reference traversal",
          "LEG-029 law no longer states", validator=audit.d1_substrate_findings)

    # --- lowerings stay distinct -------------------------------------------
    probe("get_lowered_to_scan_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "GET           -> bounded seek",
          "GET lowers to a bounded seek")
    probe("children_of_lowered_to_subtree_scan_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "CHILDREN OF   -> bounded one-level traversal",
          "CHILDREN OF lowers to one level")
    probe("scan_under_lowered_to_one_level_is_rejected",
          "docs/13_BATQL_CONTRACT.md", "SCAN UNDER    -> bounded subtree traversal",
          "SCAN UNDER lowers to a bounded subtree")

    # --- generated typed effect routing ------------------------------------
    probe("application_raw_effect_dispatcher_made_canonical_is_rejected",
          "docs/06_MACBAT.md",
          "Applications do not hand-author raw `kind integer + byte payload + switch` dispatch",
          "hand-authored raw dispatch is not the canonical path")
    probe("missing_or_duplicate_generated_route_accepted_is_rejected",
          "docs/06_MACBAT.md", "Generated routing fails closed when a declared variant has no route",
          "generated routing fails closed")
    probe("ambient_route_discovery_accepted_is_rejected",
          "docs/06_MACBAT.md", "never ambient linker registration, startup constructors",
          "route discovery is never ambient")
    probe("leg046_closed_exhaustive_routing_weakened_is_rejected",
          LEG, "closed and exhaustive", "LEG-046 law no longer states",
          validator=audit.d1_substrate_findings)
    probe("leg046_router_absorbing_domain_logic_is_rejected",
          LEG, "owns structure and glue rather than application domain transition logic",
          "LEG-046 law no longer states", validator=audit.d1_substrate_findings)

    # --- absolute monotonic deadline ---------------------------------------
    probe("relative_inactivity_timeout_called_absolute_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md",
          "A relative inactivity timeout is not an overall operation deadline",
          "a relative inactivity timeout is not a deadline")
    probe("retry_resets_overall_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md", "retry does not reset the overall deadline",
          "retry does not reset the overall deadline")
    probe("per_attempt_deadline_exceeding_overall_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md", "remains bounded by the overall operation deadline",
          "a per-attempt deadline stays bounded by the overall deadline")
    probe("lower_repeating_mechanism_outliving_deadline_is_rejected",
          "docs/10_WORLD_IMAGES_AND_PORTS.md",
          "The lowest mechanism capable of internally extending work enforces the remaining absolute deadline",
          "the lowest extending mechanism enforces the remaining deadline")
    probe("leg066_deadline_meaning_removed_is_rejected",
          LEG, "a relative inactivity timeout never substitutes for an absolute monotonic deadline",
          "LEG-066 law no longer states", validator=audit.d1_substrate_findings)
    probe("leg066_retry_resetting_deadline_permitted_is_rejected",
          LEG, "a retry never resets the overall deadline",
          "LEG-066 law no longer states", validator=audit.d1_substrate_findings)

    return findings


def test_specialization(audit, project) -> list[str]:
    """Named hostile fixtures for adaptive residual specialization (DEC-073).

    Structural only. Bootstrap does not execute PakVM, specialize a program, run
    a scan, measure a benchmark, test SIMD, compare residual results, or validate
    machine code, and these fixtures claim none of that.
    """
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    def probe(name, rel, old, needle, new="", validator=None):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, (validator or doc_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    ISA = "docs/07_PAKVM_ISA.md"
    ECS = "docs/18_DATA_ORIENTED_ECS.md"
    DISP = "spec/dispositions.rs"

    # --- DEC-073 identity ---------------------------------------------------
    if audit.specialization_findings(root) or audit.specialization_key_findings(root):
        fail("dec073_contract_passes")
    probe("dec073_wrong_class_is_rejected", DISP,
          'id: "DEC-073", class: DecisionClass::Capability',
          "DEC-073 class Enforcement is not Capability",
          'id: "DEC-073", class: DecisionClass::Enforcement',
          validator=audit.specialization_findings)
    probe("dec073_wrong_gates_are_rejected", DISP,
          "gates: &[GateId::G4, GateId::G5, GateId::G8, GateId::G9], disposition: Disposition::Lock, "
          'subject: "Adaptive residual specialization"',
          "!= [G4, G5, G8, G9]",
          "gates: &[GateId::G4], disposition: Disposition::Lock, "
          'subject: "Adaptive residual specialization"',
          validator=audit.specialization_findings)

    # --- one probe per key component: a broad marker would prove several at once
    for component in audit.D2_KEY_COMPONENTS:
        probe(f"specialization_key_missing_{component.split()[0].lower()}_is_rejected",
              ISA, component, f"specialization key omits {component}",
              validator=audit.specialization_key_findings)

    # --- reference authority ------------------------------------------------
    probe("reference_interpreter_demoted_from_semantic_authority_is_rejected",
          ISA, "The reference interpreter is the executable semantic authority for admitted programs",
          "the reference interpreter is the executable semantic authority")
    probe("specialized_plan_made_program_authority_is_rejected",
          ISA, "It is never program authority, canonical source, a new `ProgramImage`",
          "SpecializedPlan is derived, never authority")
    probe("residual_made_sole_oracle_is_rejected",
          ISA, "the residual path is never the sole oracle", "the residual is never the sole oracle")
    probe("plan_reuse_under_a_different_key_permitted_is_rejected",
          ISA, "SpecializedPlan reused under a different key", "reuse under a different key is illegal")

    # --- cache and fallback -------------------------------------------------
    probe("cache_deletion_permitted_to_change_meaning_is_rejected",
          ISA, "Deleting every specialization cache changes performance and `WorkObservation` only",
          "cache deletion changes performance only")
    probe("cache_key_mismatch_accepted_is_rejected",
          ISA, "it discards, refuses reuse, and falls back to reference execution",
          "cache mismatch discards and falls back")
    probe("corrupt_residual_reused_is_rejected",
          ISA, "An old residual is never reinterpreted under a new key",
          "an old residual is never reinterpreted under a new key")
    probe("specialization_failure_without_reference_fallback_is_rejected",
          ISA, "because reference execution remains complete",
          "reference execution remains complete without a cache")

    # --- bounded first use --------------------------------------------------
    probe("first_use_specialization_without_budget_or_deadline_is_rejected",
          ISA, "an explicit work budget, an absolute monotonic deadline, a bounded memory posture",
          "first use is budgeted and deadline-bound")
    probe("specialization_attempt_resets_logical_deadline_is_rejected",
          ISA, "The specialization attempt never resets the logical operation deadline",
          "specialization never resets the logical deadline")

    # --- safety boundary ----------------------------------------------------
    probe("specializer_executes_an_effect_is_rejected",
          ISA, "It may not pre-execute durable effects, port effects, wall-clock reads",
          "specialization pre-executes no effects")
    probe("specializer_bypasses_bvisor_admission_is_rejected",
          ISA, "It may not eliminate or bypass Bvisor admission",
          "specialization cannot bypass Bvisor admission")
    probe("specializer_mints_a_capability_is_rejected",
          ISA, "PakVM specialization cannot mint capabilities.", "specialization cannot mint capabilities")
    probe("specializer_advances_a_durable_checkpoint_is_rejected",
          ISA, "PakVM specialization cannot advance a durable checkpoint.",
          "specialization cannot advance a durable checkpoint")
    probe("specializer_authorizes_a_physical_effect_is_rejected",
          ISA, "PakVM specialization cannot authorize a physical effect.",
          "specialization cannot authorize a physical effect")
    probe("check_discharged_because_it_usually_passes_is_rejected",
          ISA, "A check is not eliminated merely because the current implementation usually passes it",
          "a check is not discharged because it usually passes")
    probe("dynamic_fact_bound_as_static_is_rejected",
          ISA, "wall-clock time, file descriptor numbers, socket state",
          "dynamic facts are not bound as static")
    probe("key_component_silently_omitted_is_rejected",
          ISA, "It may never be silently omitted", "a key component is never silently omitted")

    # --- V1 requirement boundary -------------------------------------------
    probe("native_jit_or_executable_memory_made_a_v1_requirement_is_rejected",
          ISA, "machine-code JIT, executable-memory allocation, self-modifying code",
          "no machine-code JIT or executable memory in V1")
    probe("nightly_unsafe_or_simd_made_mandatory_is_rejected",
          ECS, "V1 does not require nightly Rust, unsafe target intrinsics, a portable-SIMD dependency",
          "nightly, unsafe intrinsics, and SIMD are not required")

    # --- result parity ------------------------------------------------------
    probe("specialized_execution_omits_proof_parity_is_rejected",
          ISA, "preserve semantic value, `Availability`, `Truth`, `Decision`, `Completeness`, `ProofDisposition`",
          "result axes are preserved across paths")
    probe("faster_path_emitting_less_proof_is_rejected",
          ISA, "A faster path never emits less proof, explanation, or receipt material",
          "a faster path emits no less proof")
    probe("work_observation_execution_path_omitted_is_rejected",
          ISA, "FallbackAfterSpecializationRefusal", "WorkObservation names the execution path")

    # --- scalar seam --------------------------------------------------------
    probe("scalar_scan_kernel_removed_is_rejected",
          ECS, "The scalar `ScanKernel` is mandatory and is the reference behavior",
          "the scalar scan kernel is the reference behavior")
    probe("optimized_kernel_made_sole_oracle_is_rejected",
          ECS, "No optimized kernel is ever the sole oracle", "no optimized kernel is the sole oracle")
    probe("optimized_kernel_accepted_without_qualification_is_rejected",
          ECS, "An unavailable optimized kernel falls back to scalar reference behavior",
          "an optimized kernel requires qualification and fallback")
    probe("selection_mask_frozen_to_64_rows_is_rejected",
          ECS, "The physical representation is not frozen", "the mask width is not frozen")
    probe("aosoa64_made_canonical_is_rejected",
          ECS, "Nothing here constitutionalizes `u64`, 64 rows, `AoSoA64`, or one tile width forever",
          "AoSoA64 and fixed 64-row masks are not constitutional")
    probe("mask_ops_across_row_domains_is_rejected",
          ECS, "legal only for masks over the same row domain, logical length, source cut",
          "mask algebra is domain bound")
    probe("mask_not_selecting_unused_high_bits_is_rejected",
          ECS, "unused high bits cannot select nonexistent rows", "NOT is bounded by the logical row length")
    probe("column_tile_made_authority_is_rejected",
          ECS, "`ColumnTile` remains an earned physical derived layout", "ColumnTile remains derived")
    probe("late_materialization_reorders_rows_is_rejected",
          ECS, "preserving source row order, row identity", "late materialization preserves row order")
    probe("unbounded_materialization_before_selection_is_rejected",
          ECS, "It may not materialize an unbounded result and then claim the mask saved work",
          "no unbounded materialization before selection")
    probe("work_observation_omitted_from_scan_seam_is_rejected",
          ECS, "the frozen obligation is that work remains observable", "WorkObservation stays observable")

    # --- graph conservation -------------------------------------------------
    nodes, edges, _adm = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-073" not in ids:
        fail("guarantee_graph_missing_dec073")
    for synthetic in ("SpecializedPlan", "SelectionMask", "ScanKernel", "Column", "ColumnTile",
                      "SpecializationKey", "WorkObservation"):
        if synthetic in ids:
            fail(f"synthetic specialization node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision_node_count_is_not_82")
    if len(nodes) != 216 or len(edges) != 24:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    return findings


def test_delivery_notes_d2(audit) -> list[str]:
    """The delivery inventory tracks the live decision count — through the
    generated BUNDLE-INVENTORY derivation (5.5E4b), never an authored number."""
    findings: list[str] = []
    root = HERE.parent
    text = (root / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    live = len(audit.G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8")))
    if f"| decision rows | {live} |" not in text:
        findings.append("delivery_notes_tracks_live_decision_count FAILED")
    return findings


def test_proof_policy(audit) -> list[str]:
    """Named hostile fixtures for mutation lanes and proof-policy anti-weakening.

    Structural only. Bootstrap compiles no mutant, activates no slot, runs no
    nextest, invokes no rustc, compares no cargo-mutants run, kills nothing,
    proves no equivalence, promotes nothing, and classifies no real diff
    semantically. These fixtures claim none of that.
    """
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def doc_findings(tmp):
        out: list[str] = []
        audit.check_document_law(tmp, out)
        return out

    def probe(name, rel, old, needle, new="", validator=None):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, (validator or doc_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    ARCH = "spec/architecture.rs"
    DISP = "spec/dispositions.rs"
    TP = "docs/12_TESTPAK.md"
    GA = "docs/24_GAUNTLET.md"
    pp = audit.proof_policy_findings

    if pp(root):
        fail("proof_policy_contract_passes")

    # --- DEC-074 identity ---------------------------------------------------
    probe("dec074_missing_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement',
          "DEC-074 is absent", 'id: "DEC-0740", class: DecisionClass::Enforcement', validator=pp)
    probe("dec074_wrong_class_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement',
          "is not Enforcement", 'id: "DEC-074", class: DecisionClass::Naming', validator=pp)
    probe("dec074_missing_g3_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3, GateId::G9]',
          "DEC-074 does not name gate G3",
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G9]', validator=pp)
    probe("dec074_missing_g9_is_rejected", DISP,
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3, GateId::G9]',
          "DEC-074 does not name gate G9",
          'id: "DEC-074", class: DecisionClass::Enforcement, gates: &[GateId::G3]', validator=pp)
    probe("dec015_losing_testpak_confinement_is_rejected", DISP,
          "Mutation testing only inside TestPak", "no longer confines mutation testing to TestPak",
          "Mutation testing anywhere", validator=pp)

    # --- lane and result vocabularies (typed owner: spec/mutation.rs) --------
    MUT = "spec/mutation.rs"
    probe("fourth_mutation_lane_is_rejected", MUT,
          "    CompilerBacked,\n}", "MutationLane variants", "    CompilerBacked,\n    NativeLane,\n}",
          validator=pp)
    probe("removed_mutation_lane_is_rejected", MUT,
          "    /// Implementation-sensitive material whose truth depends on real\n"
          "    /// compiler and platform behavior. Runs under real rustc semantics.\n"
          "    CompilerBacked,\n}",
          "MutationLane variants", "}", validator=pp)
    for variant in ("NotActivated", "Refused", "TimedOut", "InfrastructureFailure", "EquivalentCandidate"):
        probe(f"mutation_result_{variant.lower()}_removed_is_rejected", MUT,
              f"    {variant},\n", "MutationResult variants", "", validator=pp)
    probe("change_class_collapsed_is_rejected", ARCH,
          "    Neutral,\n", "ProofPolicyChangeClass variants", "", validator=pp)
    probe("proof_policy_surface_removed_is_rejected", ARCH,
          "    WaiverLogic,\n", "ProofPolicySurface variants", "", validator=pp)

    # --- candidate and promotion boundary -----------------------------------
    for rootdir in ("src/", "tests/", "spec/", "docs/", "companion/"):
        probe(f"candidate_write_into_{rootdir.strip('/')}_permitted_is_rejected", ARCH,
              f'"{rootdir}"', f"not forbidden from writing {rootdir}", '"__none__"', validator=pp)
    probe("candidate_root_moved_into_tracked_source_is_rejected", ARCH,
          "target/muterprater/candidates/", "no candidate output root is declared", "tests/generated/",
          validator=pp)
    # --- documentary law ----------------------------------------------------
    for name, rel, old, needle in (
        ("survived_without_activation_is_rejected", TP,
         "A `Survived` verdict without activation evidence is invalid", "Survived requires activation"),
        ("killed_without_baseline_is_rejected", TP,
         "A `Killed` verdict without a qualified baseline and activation evidence is invalid",
         "Killed requires baseline and activation"),
        ("timeout_counted_as_kill_is_rejected", TP, "A timeout is not a kill", "a timeout is not a kill"),
        ("unbuildable_counted_as_kill_is_rejected", TP,
         "a compiler error produced by an invalid mutation is not evidence",
         "an unbuildable candidate is not a kill"),
        ("equivalent_candidate_without_witness_is_rejected", TP,
         "a classification pending its witness, never equivalence proof",
         "EquivalentCandidate is not equivalence proof"),
        ("denominator_silently_shrunk_is_rejected", TP,
         "Nothing silently leaves the denominator", "nothing leaves the denominator silently"),
        ("score_without_distribution_is_rejected", TP,
         "A mutation score exposes the full result distribution",
         "the full result distribution is published"),
        ("lane_a_requiring_rustc_is_rejected", TP, "No per-candidate Rust compile. The reference interpreter",
         "Lane A needs no per-candidate Rust compile"),
        ("lane_b_slots_in_production_is_rejected", TP, "ordinary production artifact. A release artifact",
         "Lane B slots are test-profile only"),
        ("lane_b_without_activation_is_rejected", TP, "Activation is evidence, never an assumption",
         "Lane B records activation"),
        ("lane_c_replacing_rustc_is_rejected", TP, "never claims a homegrown Rust evaluator equals rustc",
         "no homegrown evaluator equals rustc"),
        ("nextest_as_semantic_authority_is_rejected", TP, "Nextest executes; it is never semantic authority",
         "nextest executes and is not semantic authority"),
        ("differential_tool_retired_by_prose_is_rejected", TP, "No coronation by architecture prose",
         "compiler-backed tooling is not retired by prose"),
        ("muterprater_as_standalone_product_is_rejected", TP,
         "It is not a standalone product package, a second semantic authority",
         "Muterprater is not a standalone product"),
        ("production_code_as_own_oracle_is_rejected", TP,
         "passing production evaluator against itself -> not an independent oracle",
         "self-calling production code is not an oracle"),
        ("second_pakvm_mandated_is_rejected", TP,
         "Building a second full PakVM merely to call it independent is not independence",
         "no second full PakVM is required for independence"),
        ("direct_tracked_write_path_is_rejected", TP,
         "It may not write candidates directly into `src/`, `tests/`, `spec/`, `docs/`, or `companion/`",
         "candidates land outside tracked source"),
        ("generation_and_promotion_merged_is_rejected", TP,
         "Generation and promotion are two different operations with two different receipts",
         "generation and promotion are separate"),
        ("unclassified_policy_change_accepted_is_rejected", TP,
         "An unclassified proof-policy change is refused",
         "an unclassified proof-policy change is refused"),
        ("neutral_without_parity_is_rejected", TP,
         "and documentary and receipt meaning. Requires parity evidence:",
         "Neutral requires parity evidence"),
        ("narrowed_domain_escaping_weakening_is_rejected", TP,
         "narrows the tested domain or silently removes denominator units is Weakening",
         "a narrowed domain is Weakening"),
        ("weakening_without_hostile_qualification_is_rejected", TP,
         "hostile qualification proving the new boundary is enforced",
         "a Weakening carries the full authority package"),
        ("issue_link_as_authority_is_rejected", TP, "A generic issue link is not decision authority",
         "an issue link is not authority"),
        ("commit_message_as_receipt_is_rejected", TP, "A commit message is not a weakening receipt",
         "a commit message is not a receipt"),
        ("gate_disabling_itself_as_cleanup_is_rejected", TP,
         "A proof-policy gate that can be disabled as unclassified cleanup is not an anti-weakening gate",
         "a disableable gate is not a gate"),
        ("bootstrap_claiming_diff_inference_is_rejected", TP,
         "Bootstrap does not understand arbitrary semantic diffs and never claims to",
         "bootstrap does not infer semantic diffs"),
        ("d1_publication_proof_ownership_removed_is_rejected", GA,
         "committed is never reported as an ordinary operation failure",
         "D1 publication proof family is owned"),
        ("d1_attempt_proof_ownership_removed_is_rejected", GA,
         "CancelledAfterAdmission does not prove non-commit", "D1 attempt proof family is owned"),
        ("d1_traversal_proof_ownership_removed_is_rejected", GA,
         "valid pages concatenate to the one-shot reference traversal",
         "D1 traversal proof family is owned"),
        ("d1_routing_proof_ownership_removed_is_rejected", GA,
         "every declared route is present exactly once", "D1 routing proof family is owned"),
        ("d1_deadline_proof_ownership_removed_is_rejected", GA,
         "retry does not reset the overall deadline", "D1 deadline proof family is owned"),
        ("d2_specialization_square_removed_is_rejected", GA,
         "Specialization proof square (DEC-073)", "the D2 specialization square is assigned"),
        ("one_lane_made_sole_proof_route_is_rejected", GA, "No lane is the sole proof route",
         "no lane is the sole proof route"),
    ):
        probe(name, rel, old, needle)

    # --- graph conservation -------------------------------------------------
    nodes, edges, _adm = audit.guarantee_derive(root)
    ids = {n["id"] for n in nodes}
    if "DEC-074" not in ids:
        fail("guarantee_graph_missing_dec074")
    for synthetic in ("SemanticIr", "SelectableCompiled", "CompilerBacked", "MutationLane",
                      "MutationResult", "ProofPolicySurface", "Killed", "Survived"):
        if synthetic in ids:
            fail(f"synthetic mutation node introduced: {synthetic}")
    if len([n for n in nodes if n["family"] == "DEC"]) != 82:
        fail("decision_node_count_is_not_82")
    if len(nodes) != 216 or len(edges) != 24:
        fail(f"graph topology moved: {len(nodes)} nodes, {len(edges)} edges")
    text = (root / "DELIVERY_NOTES.md").read_text(encoding="utf-8")
    live_decisions = len(audit.G_DEC_ROW.findall(
        (root / "spec/dispositions.rs").read_text(encoding="utf-8")))
    if f"| decision rows | {live_decisions} |" not in text:
        fail("delivery_notes_tracks_live_decision_count")
    return findings


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


def test_bootstrap_output(audit, project) -> list[str]:
    """The 5.5E5 isolated Gate-0 materializer-output qualification fixtures.

    Typed-plan closure, command-boundary isolation, non-overwriting
    publication, manifest and compilation qualification, independent-oracle
    integrity, sealed substitution probes, and structural growth. The
    behavioral probes run the REAL compiled materializer against scratch
    outputs; nothing here writes the seed.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, (validator or audit.bootstrap_output_findings)(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    bo = audit.bootstrap_output_findings
    BOS = "spec/bootstrap_output.rs"
    ARCH = "spec/architecture.rs"

    if bo(root):
        fail(f"bootstrap_output_contract_passes (got {bo(root)!r})")

    # --- typed plan closure --------------------------------------------------
    # Enum-to-ALL divergence is now unrepresentable: `gate0_closed_enum!`
    # generates the inventory from the one variant list (5.5E5a). The former
    # "authored but ALL omits it" hostiles are retired as impossible; what the
    # auditor enforces instead is the CONSTRUCTION -- no hand-written ALL and no
    # enum declared outside the macro.
    probe("gate0_closed_enum_hand_written_all_is_rejected", BOS,
          "pub const WORKSPACE_LICENSE",
          "carries a hand-written ALL",
          "impl Gate0RootArtifact { pub const ALL: &'static [Gate0RootArtifact] = &[]; }\n\n"
          "pub const WORKSPACE_LICENSE")
    probe("gate0_closed_enum_bypass_is_rejected", BOS,
          "gate0_closed_enum! {\n    /// A workspace-root file",
          "is not declared through gate0_closed_enum!",
          "gate0_bypassed_macro! {\n    /// A workspace-root file")
    probe("duplicate_gate0_root_path_is_rejected", BOS,
          'Gate0RootArtifact::Justfile => "justfile",',
          "two root artifacts project one path",
          'Gate0RootArtifact::Justfile => "Cargo.toml",')
    probe("future_package_without_explicit_target_kind_is_rejected", BOS,
          "        PackageId::BatPakExamples => Gate0PackageTargetKind::ExampleBinary,\n",
          "package BatPakExamples has no explicit target kind")
    probe("binary_package_without_target_name_is_rejected", BOS,
          '        PackageId::BatPakCli => Some("batpak"),\n',
          "binary-kind package BatPakCli names no binary target")
    probe("library_package_with_binary_target_is_rejected", BOS,
          '        PackageId::BatPakCli => Some("batpak"),',
          "library-kind package BatPak names a binary target",
          '        PackageId::BatPakCli => Some("batpak"),\n'
          '        PackageId::BatPak => Some("bp"),')

    def plan_refusal(tmp):
        try:
            project.gate0_plan_facts(tmp)
            return []
        except project.Unadmitted as exc:
            return [str(exc)]

    probe("package_source_door_collision_is_rejected", ARCH,
          'PackageId::BatQl => "crates/batql",',
          "lists crates/netbat/Cargo.toml twice",
          'PackageId::BatQl => "crates/netbat",', validator=plan_refusal)
    probe("duplicate_expanded_gate0_path_is_rejected", ARCH,
          'PackageId::NetBat => "crates/netbat",',
          "lists crates/batpak/Cargo.toml twice",
          'PackageId::NetBat => "crates/batpak",', validator=plan_refusal)
    probe("syncbat_plane_missing_from_all_is_rejected", ARCH,
          "        SyncBatPlane::Port,\n",
          "plane Port is authored but SyncBatPlane::ALL omits it",
          validator=audit.syncbat_firewall_findings)
    probe("duplicate_syncbat_plane_module_name_is_rejected", ARCH,
          'SyncBatPlane::Port => "port",',
          "two SyncBat planes project one module name",
          'SyncBatPlane::Port => "world",')
    probe("raw_syncbat_required_planes_table_cannot_return", ARCH,
          "pub const FORBIDDEN_TARGET_PATHS",
          "raw plane inventory SYNCBAT_REQUIRED_PLANES",
          "pub const SYNCBAT_REQUIRED_PLANES: &[&str] = &[];\n\n"
          "pub const FORBIDDEN_TARGET_PATHS")
    probe("parallel_syncbat_plane_inventory_cannot_return", ARCH,
          "pub const FORBIDDEN_TARGET_PATHS",
          "raw plane inventory SYNCBAT_PLANES",
          "pub const SYNCBAT_PLANES: &[SyncBatPlane] = &[];\n\n"
          "pub const FORBIDDEN_TARGET_PATHS")
    probe("absolute_gate0_output_path_is_rejected", BOS,
          'Gate0RootArtifact::Justfile => "justfile",',
          "not output-root-relative and traversal-free",
          'Gate0RootArtifact::Justfile => "/justfile",')
    probe("parent_traversal_in_gate0_output_path_is_rejected", BOS,
          'Gate0RootArtifact::Justfile => "justfile",',
          "not output-root-relative and traversal-free",
          'Gate0RootArtifact::Justfile => "../justfile",')

    # The one portable-path law, exercised through the auditor's independent
    # reconstruction: a package path that is clean under forward-slash rules can
    # still carry a backslash, a dot component, a Windows device name, or a
    # case-fold twin -- all safe today, none portable.
    probe("backslash_parent_traversal_is_rejected", ARCH,
          'PackageId::BatQl => "crates/batql",',
          "is not a portable relative path",
          'PackageId::BatQl => "crates\\\\..\\\\batql",')
    probe("dot_component_is_rejected", ARCH,
          'PackageId::BatQl => "crates/batql",',
          "is not a portable relative path",
          'PackageId::BatQl => "crates/./batql",')
    probe("windows_device_component_is_rejected", ARCH,
          'PackageId::BatQl => "crates/batql",',
          "is not a portable relative path",
          'PackageId::BatQl => "crates/nul",')
    probe("casefold_colliding_output_paths_are_rejected", ARCH,
          'PackageId::BatQl => "crates/batql",',
          "collide under case folding",
          'PackageId::BatQl => "crates/BATPAK",')

    # the expanded plan carries both artifacts of every plane
    facts = oracle_facts(root)
    plan = oracle_plan(root)
    plane_base = facts["package_paths"][facts["plane_owner"]]
    for plane in facts["planes"]:
        module = facts["modules"][plane]
        if f"{plane_base}/src/{module}.rs" not in plan:
            fail("syncbat_plane_module_missing_from_plan_is_rejected")
        if f"{plane_base}/src/{module}/README.md" not in plan:
            fail("syncbat_plane_readme_missing_from_plan_is_rejected")

    # --- isolation laws over the harness and the corpus ----------------------
    # The forged payload is concatenated at run time so THIS source never
    # contains the contiguous one-root tuple the audit law scans for.
    forged_route = ('_FORGED_ROUTE = ("tier0-mat' + 'erialize", '
                    '"bootstrap/materialize.rs", "materialize: PASS")'
                    "\n\n\ndef qualify_materializer")
    probe("materializer_qualification_cannot_use_live_repository_as_output",
          "bootstrap/selftest.py",
          "def qualify_materializer",
          "rides the generic one-root binary qualification",
          forged_route)
    with isolated_tree(subdirs=("spec", "bootstrap")) as tmp:
        d0 = _tree_digest(tmp)
        s0 = _seed_snapshot(tmp)
        (tmp / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
        if _tree_digest(tmp) != d0:
            fail("limited_source_digest_cannot_substitute_for_seed_snapshot "
                 "(the limited digest saw the scaffold; the fixture premise moved)")
        if _seed_snapshot(tmp) == s0:
            fail("limited_source_digest_cannot_substitute_for_seed_snapshot "
                 "(the full snapshot is blind to a root scaffold file)")
    for readme_rel in [f"{facts['package_paths'][pid]}/README.md"
                       for pid in facts["package_order"]][:1]:
        text = plan[readme_rel].decode("utf-8")
        if text.startswith("---") or "reconciliation_epoch" in text:
            fail("package_readme_cannot_force_architecture_frontmatter")
    if {"crates", "apps", "examples"} & set(audit.EXCLUDE_DIRS):
        fail("materializer_output_cannot_create_a_corpus_eligibility_carveout")

    # --- oracle integrity ----------------------------------------------------
    st_src = (HERE / "selftest.py").read_text(encoding="utf-8")
    oracle_block = re.search(r"\ndef oracle_plan\(.*?(?=\ndef )", st_src, re.S).group(0)
    if "materialize" in oracle_block:
        fail("output_oracle_cannot_import_materializer_renderer")
    if "g0-candidate" in oracle_block or "read_tree" in oracle_block:
        fail("output_oracle_cannot_use_generated_output_as_expected_bytes")
    probe("output_oracle_cannot_carry_package_name_map", "bootstrap/selftest.py",
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}",
          "the output oracle carries the package name",
          '    _forged = "macbat-compiler"\n'
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}")
    probe("output_oracle_cannot_carry_plane_name_map", "bootstrap/selftest.py",
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}",
          "the output oracle carries the plane module name",
          '    _forged = "pakvm"\n'
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}")
    probe("output_oracle_cannot_carry_feature_map", "bootstrap/selftest.py",
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}",
          "the output oracle carries the feature name",
          '    _forged = "browser-storage"\n'
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}")
    probe("gate0_materialization_plan_projection_drift_is_rejected",
          "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
          "| justfile | Root / Justfile |",
          "does not match its typed derivation",
          "| justfile-drifted | Root / Justfile |")
    probe("gate0_materialization_plan_marker_drift_is_rejected",
          "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
          "GATE0-MATERIALIZATION-PLAN:BEGIN generated from spec/bootstrap_output.rs; "
          "spec/architecture.rs; spec/toolchain.rs",
          "GATE0-MATERIALIZATION-PLAN",
          "GATE0-MATERIALIZATION-PLAN:BEGIN generated from spec/architecture.rs; "
          "spec/bootstrap_output.rs; spec/toolchain.rs",
          validator=audit.generated_view_findings)

    # --- behavioral probes against the real binary ---------------------------
    rustc = shutil.which("rustc")
    if not rustc:
        fail("bootstrap_output_behavioral_probes (no rustc on this machine)")
        findings.extend(canonical_drift(before))
        return findings
    workdir = Path(tempfile.mkdtemp(prefix="batpak-g0-"))
    try:
        exe = None
        exe_triple = None
        for target in (None, "x86_64-pc-windows-gnu"):
            rlib = workdir / f"libspec-{target or 'host'}.rlib"
            lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                       "--crate-name", "spec", "-o", str(rlib), str(root / "spec/lib.rs")]
            candidate = workdir / f"materialize-{target or 'host'}.exe"
            cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", "materialize",
                   "--extern", f"spec={rlib}", "-o", str(candidate),
                   str(root / "bootstrap/materialize.rs")]
            if target:
                lib_cmd[1:1] = ["--target", target]
                cmd[1:1] = ["--target", target]
            if subprocess.run(lib_cmd, capture_output=True, text=True).returncode == 0 \
                    and subprocess.run(cmd, capture_output=True, text=True).returncode == 0:
                exe = candidate
                exe_triple = target
                break
        if exe is None:
            fail("bootstrap_output_behavioral_probes (no working linker; the "
                 "hosted lane owns these probes)")
            findings.extend(canonical_drift(before))
            return findings
        if exe_triple is None:
            m = re.search(r"host: (\S+)",
                          subprocess.run([rustc, "-vV"], capture_output=True,
                                         text=True).stdout)
            exe_triple = m.group(1) if m else "host default"

        def mat(*args: str) -> subprocess.CompletedProcess:
            return subprocess.run([str(exe), *args], capture_output=True, text=True)

        def mat_out(output: Path) -> subprocess.CompletedProcess:
            return mat("--seed", str(root), "--output", str(output))

        def refusal(name: str, proc: subprocess.CompletedProcess, needle: str) -> None:
            if proc.returncode == 0 or needle not in (proc.stdout + proc.stderr):
                fail(f"{name} (wanted {needle!r}, got "
                     f"{(proc.stdout + proc.stderr).strip()[:160]!r})")

        refusal("materializer_requires_explicit_seed_root",
                mat("--output", str(workdir / "x1")), "the --seed root is required")
        refusal("materializer_requires_explicit_output_root",
                mat("--seed", str(root)), "the --output root is required")
        refusal("materializer_refuses_same_seed_and_output_root",
                mat_out(root), "the seed and output roots are the same path")
        refusal("materializer_refuses_output_inside_seed_root",
                mat_out(root / "g0-probe-out"), "the output root is inside the seed root")
        refusal("materializer_refuses_seed_inside_output_root",
                mat_out(root.parent), "the seed root is inside the output root")

        snap = _seed_snapshot(root)
        base = workdir / "base"
        proc = mat_out(base)
        if proc.returncode != 0 or "materialize: PASS Created" not in proc.stdout:
            fail(f"materializer_publishes_a_created_candidate (got "
                 f"{(proc.stdout + proc.stderr).strip()[:160]!r})")
            findings.extend(canonical_drift(before))
            return findings
        if _seed_snapshot(root) != snap:
            fail("tier0_materializer_does_not_mutate_seed_tree")
        base_tree = _read_tree(base)
        if base_tree != plan:
            fail("materialized_tree_matches_the_independent_oracle")

        def scratch(name: str) -> Path:
            out = workdir / name
            shutil.copytree(base, out)
            return out

        o = scratch("p-missing")
        (o / "justfile").unlink()
        refusal("materializer_missing_planned_file_is_rejected", mat_out(o),
                "missing the planned file justfile")
        o = scratch("p-mismatch")
        with open(o / "justfile", "ab") as fh:
            fh.write(b"# drift\n")
        refusal("materializer_mismatching_planned_file_is_rejected", mat_out(o),
                "does not match its planned bytes")
        if (o / "justfile").read_bytes() == plan["justfile"]:
            fail("divergent_existing_output_is_not_repaired")
        o = scratch("p-extra")
        (o / "stray.txt").write_text("x", encoding="utf-8")
        refusal("materializer_extra_file_is_rejected", mat_out(o),
                "extra file stray.txt")
        o = scratch("p-extradir")
        (o / "straydir").mkdir()
        refusal("materializer_extra_directory_is_rejected", mat_out(o),
                "extra directory straydir")
        o = scratch("p-symlink")
        try:
            os.symlink(o / "Cargo.toml", o / "link.toml")
            refusal("materializer_symlink_is_rejected", mat_out(o),
                    "carries a symlink")
        except OSError:
            # Symlink creation needs privilege this environment lacks; the
            # refusal still has to EXIST in the publication path, and the
            # hosted lane exercises it behaviorally.
            if "is_symlink" not in (root / "bootstrap/materialize.rs").read_text(encoding="utf-8"):
                fail("materializer_symlink_is_rejected (no symlink refusal in the "
                     "publication path)")
        o = scratch("p-collision")
        (o / "justfile").unlink()
        (o / "justfile").mkdir()
        refusal("materializer_file_directory_collision_is_rejected", mat_out(o),
                "a directory sits where the planned file justfile belongs")
        o = workdir / "p-staging-dir"
        (workdir / ".p-staging-dir.g0-staging").mkdir()
        refusal("late_mismatch_refusal_creates_no_earlier_files", mat_out(o),
                "staging path")
        if o.exists():
            fail("late_mismatch_refusal_creates_no_earlier_files (final output appeared)")
        o = workdir / "p-staging-file"
        (workdir / ".p-staging-file.g0-staging").write_text("x", encoding="utf-8")
        refusal("failed_staging_leaves_final_output_absent", mat_out(o), "staging path")
        if o.exists():
            fail("failed_staging_leaves_final_output_absent (final output appeared)")

        # The binary is bound to its compiled seed manifest: a seed carrying a
        # different SPEC.sha256 is refused before any plan is built.
        alt_seed = workdir / "alt-seed"
        alt_seed.mkdir()
        (alt_seed / "SPEC.sha256").write_text(
            (root / "SPEC.sha256").read_text(encoding="utf-8") + "# tampered\n",
            encoding="utf-8")
        refusal("materializer_refuses_seed_manifest_different_from_compiled_manifest",
                mat("--seed", str(alt_seed), "--output", str(workdir / "alt-out")),
                "the seed manifest differs")

        # The candidate justfile advertises no seed-only tool it does not carry.
        jf = (base / "justfile").read_text(encoding="utf-8")
        if any(t in jf for t in ("bootstrap/audit.py", "bootstrap/freeze.py",
                                 "bootstrap/seedcheck.rs")):
            fail("materialized_justfile_cannot_reference_unplanned_seed_tools")

        # The independent object inspection stays red on a symlink even if the
        # materializer's own guard were removed, and the file-only digest never
        # reads a symlink's bytes.
        src_text = (HERE / "selftest.py").read_text(encoding="utf-8")
        o = scratch("p-oracle-symlink")
        try:
            os.symlink(o / "Cargo.toml", o / "shadow.toml")
            ins = _inspect_candidate_tree(o, set(plan))
            if not any("symlink" in m for m in ins):
                fail("oracle_stays_red_when_materializer_symlink_guard_is_neutered")
            if "shadow.toml" in _read_tree(o):
                fail("output_symlink_cannot_enter_tree_digest "
                     "(a symlink was read as a regular file)")
        except OSError:
            if "carries a symlink at" not in src_text:
                fail("oracle_stays_red_when_materializer_symlink_guard_is_neutered "
                     "(no independent symlink refusal)")
                fail("output_symlink_cannot_enter_tree_digest (no symlink refusal)")
        if "special (non-regular) file" not in src_text:
            fail("special_output_file_is_rejected (no independent special-file refusal)")

        # The seed snapshot is entry-type aware: an empty directory and a
        # same-bytes symlink both change it, though the compile digest is blind.
        with isolated_tree(subdirs=("spec", "bootstrap")) as t2:
            snap_a = _seed_snapshot(t2)
            (t2 / "g0-empty").mkdir()
            if _seed_snapshot(t2) == snap_a:
                fail("empty_seed_directory_change_is_detected")
            link = t2 / "spec" / "g0_shadow.rs"
            target = t2 / "spec" / "lib.rs"
            link.write_bytes(target.read_bytes())
            snap_c = _seed_snapshot(t2)
            try:
                link.unlink()
                os.symlink(target, link)
                if _seed_snapshot(t2) == snap_c:
                    fail("seed_file_replaced_by_same_bytes_symlink_is_detected")
            except OSError:
                if "symlink" not in src_text:
                    fail("seed_file_replaced_by_same_bytes_symlink_is_detected "
                         "(snapshot is not entry-type aware)")

        # The toolchain oracle renders from spec/toolchain.rs: corrupting the
        # tracked file cannot move the oracle's expected bytes.
        with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as t3:
            (t3 / "rust-toolchain.toml").write_text("# CORRUPTED\n", encoding="utf-8")
            rendered = _oracle_toolchain_toml(t3)
            if rendered == "# CORRUPTED\n" or "channel" not in rendered:
                fail("tracked_toolchain_projection_cannot_be_used_as_the_oracle")

        proc = mat_out(base)
        if proc.returncode != 0 or "materialize: PASS Unchanged" not in proc.stdout:
            fail("exact_existing_output_returns_unchanged")
        if _read_tree(base) != base_tree:
            fail("second_materialization_is_byte_idempotent")
        o = workdir / "twin"
        mat_out(o)
        if _read_tree(o) != base_tree:
            fail("two_isolated_outputs_are_byte_identical")
        fwd = dict(sorted(base_tree.items()))
        rev = dict(sorted(base_tree.items(), reverse=True))
        if _output_tree_digest(fwd) != _output_tree_digest(rev):
            fail("materialized_tree_digest_is_order_independent")
        if "Cargo.lock" in plan or (base / "Cargo.lock").exists():
            fail("cargo_generated_lockfile_cannot_enter_materializer_plan")

        # --- manifest and compilation qualification --------------------------
        members = re.findall(r'^  "([^"]+)",$',
                             base_tree["Cargo.toml"].decode("utf-8"), re.M)
        if members != [facts["package_paths"][pid] for pid in facts["package_order"]]:
            fail("materialized_workspace_member_order_is_canonical")
        cargo = _g0_cargo_command(exe_triple)
        if cargo is None:
            fail(f"materialized_workspace_qualification (no cargo toolchain "
                 f"matches {exe_triple})")
        else:
            build = workdir / "build"
            shutil.copytree(base, build)
            clean_env = {k: v for k, v in os.environ.items()
                         if k != "CARGO_BUILD_TARGET"}

            def cargo_run(*args: str, env=None) -> subprocess.CompletedProcess:
                return subprocess.run([*cargo, *args], cwd=build,
                                      env=env if env is not None else clean_env,
                                      capture_output=True, text=True)

            meta_proc = cargo_run("metadata", "--no-deps", "--format-version", "1")
            if meta_proc.returncode != 0:
                fail("materialized_workspace_members_equal_package_id_all "
                     "(cargo metadata refused the candidate)")
            else:
                meta = json.loads(meta_proc.stdout)
                inspect = inspect_gate0_metadata
                real = inspect(meta, facts)
                if real:
                    fail(f"materialized_workspace_members_equal_package_id_all "
                         f"(got {real!r})")
                import copy

                def mutated(mutator) -> list[str]:
                    m2 = copy.deepcopy(meta)
                    mutator(m2)
                    return inspect(m2, facts)

                def by_name(m2, name):
                    return next(p for p in m2["packages"] if p["name"] == name)

                names = facts["cargo_names"]
                dev_pid = next(pid for pid in facts["package_order"]
                               if facts["package_rows"][pid]["class"] == "DevOnly")
                ex_pid = next(pid for pid in facts["package_order"]
                              if facts["package_rows"][pid]["class"] == "Example")
                any_pid = facts["package_order"][0]
                expect("materialized_package_name_drift_is_rejected",
                       mutated(lambda m2: by_name(m2, names[any_pid]).update(
                           name=names[any_pid] + "-drift")),
                       "is missing package")
                expect("materialized_package_path_drift_is_rejected",
                       mutated(lambda m2: by_name(m2, names[any_pid]).update(
                           manifest_path="elsewhere/Cargo.toml")),
                       "path drifted")
                edge = next(e for e in facts["edges"] if e["class"] == "Required")
                expect("materialized_edge_missing_is_rejected",
                       mutated(lambda m2: by_name(m2, names[edge["importer"]])
                               ["dependencies"].remove(next(
                                   d for d in by_name(m2, names[edge["importer"]])["dependencies"]
                                   if d["name"] == names[edge["importee"]]))),
                       "is missing declared edge")
                expect("materialized_edge_extra_is_rejected",
                       mutated(lambda m2: by_name(m2, names[any_pid])
                               ["dependencies"].append(
                                   {"name": names[dev_pid], "optional": False,
                                    "kind": None})),
                       "carries undeclared edge")
                opt = next((e for e in facts["edges"] if e["class"] == "OptionalProfile"),
                           None)
                if opt is None:
                    fail("materialized_optional_edge_is_not_required "
                         "(the seed declares no optional edge to defend)")
                else:
                    expect("materialized_optional_edge_is_not_required",
                           mutated(lambda m2: next(
                               d for d in by_name(m2, names[opt["importer"]])["dependencies"]
                               if d["name"] == names[opt["importee"]]).update(optional=False)),
                           "wrong optional posture")
                dev_edge = next((e for e in facts["edges"] if e["class"] == "DevOnly"
                                 and facts["package_rows"][e["importer"]]["class"] != "DevOnly"),
                                None)
                if dev_edge is not None:
                    expect("materialized_dev_edge_cannot_leak_into_production",
                           mutated(lambda m2: next(
                               d for d in by_name(m2, names[dev_edge["importer"]])["dependencies"]
                               if d["name"] == names[dev_edge["importee"]]
                               and d.get("kind") == "dev").update(kind=None)),
                           "leaks into production")
                else:
                    # The seed declares no production dev edge at all, so the
                    # leak the law refuses can only arrive as an UNDECLARED
                    # normal dependency on dev tooling -- plant exactly that.
                    prod_pid = next(pid for pid in facts["package_order"]
                                    if facts["package_rows"][pid]["class"] == "Production")
                    expect("materialized_dev_edge_cannot_leak_into_production",
                           mutated(lambda m2: by_name(m2, names[prod_pid])
                                   ["dependencies"].append(
                                       {"name": names[dev_pid], "optional": False,
                                        "kind": None})),
                           "carries undeclared edge")
                expect("materialized_examples_cannot_depend_on_testpak_is_rejected",
                       mutated(lambda m2: by_name(m2, names[ex_pid])
                               ["dependencies"].append(
                                   {"name": names[dev_pid], "optional": False,
                                    "kind": None})),
                       "depends on dev tooling")
                expect("materialized_publish_false_posture_is_enforced",
                       mutated(lambda m2: by_name(m2, names[ex_pid]).update(publish=None)),
                       "must carry publish = false")
                wasm = [p for p in facts["profiles"]
                        if p["environment"] not in ("NoStdAlloc", "NativeStd")]
                for fixture, row in zip(
                        ("browser_storage_profile_requires_named_feature",
                         "syncbat_browser_profile_requires_named_feature"),
                        sorted(wasm, key=lambda r: r["profile"])):
                    expect(fixture,
                           mutated(lambda m2, row=row: by_name(
                               m2, names[row["package"]])["features"].pop(
                                   row["profile"], None)),
                           f"names no {row['profile']} opt-in feature")

                proc = cargo_run("check", "--target", exe_triple, "--workspace",
                                 "--all-targets")
                if proc.returncode != 0:
                    fail("macbat_proc_macro_skeleton_must_compile (workspace check "
                         "failed):\n" + "\n".join(proc.stderr.splitlines()[-4:]))
                macbat_pid = next(pid for pid, k in facts["kinds"].items()
                                  if k == "ProcMacroLibrary")
                if "proc-macro" not in {k for p in meta["packages"]
                                        if p["name"] == names[macbat_pid]
                                        for t in p["targets"] for k in t["kind"]}:
                    fail("macbat_proc_macro_skeleton_must_compile (no proc-macro target)")
                for fixture, pid in zip(("batpak_no_std_skeleton_must_compile",
                                         "syncbat_no_std_skeleton_must_compile"),
                                        facts["no_std"]):
                    proc = cargo_run("check", "--target", exe_triple, "-p", names[pid],
                                     "--no-default-features")
                    if proc.returncode != 0:
                        fail(fixture + ":\n" + "\n".join(proc.stderr.splitlines()[-4:]))
                ws_manifest = base_tree["Cargo.toml"].decode("utf-8")
                for pid in facts["no_std"]:
                    if f"{names[pid]} = {{ path = \"{facts['package_paths'][pid]}\", " \
                       "default-features = false }" not in ws_manifest:
                        fail("syncbat_no_std_cannot_reenable_batpak_std (a no_std-"
                             "capable dependency ships with default features on)")
                # An incompatible ambient CARGO_BUILD_TARGET cannot redirect a
                # command that carries an explicit --target: the qualified
                # compile must still bind the receipt's exact triple.
                decoy = ("x86_64-pc-windows-gnu"
                         if exe_triple != "x86_64-pc-windows-gnu"
                         else "x86_64-pc-windows-msvc")
                redirect_env = dict(clean_env, CARGO_BUILD_TARGET=decoy)
                pid0 = facts["no_std"][0]
                proc = cargo_run("check", "--target", exe_triple, "-p", names[pid0],
                                 "--no-default-features", env=redirect_env)
                if proc.returncode != 0:
                    fail("ambient_cargo_build_target_cannot_redirect_qualification "
                         "(explicit --target did not survive an incompatible "
                         f"CARGO_BUILD_TARGET):\n"
                         + "\n".join(proc.stderr.splitlines()[-4:]))
                proc = cargo_run("run", "-q", "--target", exe_triple, "-p", names[ex_pid],
                                 "--bin", facts["binary_targets"][ex_pid])
                if proc.returncode != 0 or \
                        f"{names[ex_pid]}: gate-0 placeholder" not in proc.stdout:
                    fail("family_smoke_target_must_be_observable")
                elif not (build / "target" / exe_triple).is_dir():
                    fail("family_smoke_target_must_be_observable (no target-specific "
                         f"build dir target/{exe_triple})")
                if names[ex_pid] != facts["binary_targets"][ex_pid]:
                    cli_pid = next(pid for pid, k in facts["kinds"].items()
                                   if k == "Binary")
                    bins = {t["name"] for p in meta["packages"]
                            if p["name"] == names[cli_pid]
                            for t in p["targets"] if "bin" in t["kind"]}
                    if bins != {facts["binary_targets"][cli_pid]}:
                        fail("batpak_cli_target_must_be_batpak")

        # --- sealed substitution probes (compile against the real boundary) --
        spec_meta = workdir / "libspec.rmeta"
        proc = subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib",
             "--emit=metadata", "--crate-name", "spec", "-o", str(spec_meta),
             str(root / "spec/lib.rs")],
            capture_output=True, text=True)
        if proc.returncode != 0:
            fail("bootstrap_output_sealed_probes (spec does not compile)")
        else:
            for name, body, want in (
                ("raw_string_cannot_substitute_for_gate0_root_artifact",
                 'let _: spec::bootstrap_output::Gate0RootArtifact = "Cargo.toml";',
                 "mismatched types"),
                ("package_class_cannot_substitute_for_gate0_package_target_kind",
                 "let _: spec::bootstrap_output::Gate0PackageTargetKind = "
                 "spec::architecture::PackageClass::Production;",
                 "mismatched types"),
                ("materialization_id_cannot_substitute_for_gate0_output_artifact",
                 "use spec::bootstrap_output::MaterializationId;",
                 "no `MaterializationId`"),
                ("architecture_module_does_not_reexport_gate0_output_types",
                 "use spec::architecture::Gate0RootArtifact;",
                 "no `Gate0RootArtifact`"),
            ):
                src = workdir / f"{name}.rs"
                if body.startswith("use "):
                    src.write_text(f"{body}\nfn main() {{}}\n", encoding="utf-8")
                else:
                    src.write_text(f"fn main() {{ {body} }}\n", encoding="utf-8")
                proc = subprocess.run(
                    [rustc, "--edition", TOOLCHAIN_EDITION, "--emit=metadata",
                     "--crate-name", name, "--extern", f"spec={spec_meta}",
                     "-o", str(workdir / f"{name}.rmeta"), str(src)],
                    capture_output=True, text=True)
                if proc.returncode == 0:
                    fail(f"{name} (the hostile consumer compiled)")
                elif want not in proc.stderr:
                    fail(f"{name} (refused for the wrong reason: "
                         f"{proc.stderr.splitlines()[:3]!r})")

        # --- structural growth: the counts are evidence, never law -----------
        def growth(name: str, edits, expect_new: list[str], delta: int,
                   metadata_gains: str = "", oracle_check: bool = True) -> None:
            tmp = gate_sandbox(edits)
            try:
                # the sandbox must be a COMPLETE seed for validate_seed
                for rel in ("SPEC.sha256", "history/README.md"):
                    if (root / rel).is_file():
                        (tmp / rel).parent.mkdir(parents=True, exist_ok=True)
                        shutil.copy2(root / rel, tmp / rel)
                grlib = workdir / f"libspec-{name}.rlib"
                gexe = workdir / f"{name}.exe"
                for cmdset in (
                    [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                     "--crate-name", "spec", "-o", str(grlib), str(tmp / "spec/lib.rs")],
                    [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name",
                     "materialize", "--extern", f"spec={grlib}", "-o", str(gexe),
                     str(tmp / "bootstrap/materialize.rs")],
                ):
                    if exe_triple != "host default":
                        cmdset[1:1] = ["--target", exe_triple]
                    proc = subprocess.run(cmdset, capture_output=True, text=True)
                    if proc.returncode != 0:
                        fail(f"{name} (sandbox spec/materializer does not compile):\n"
                             + "\n".join(proc.stderr.splitlines()[:6]))
                        return
                ga = workdir / f"{name}-a"
                gb = workdir / f"{name}-b"
                for out in (ga, gb):
                    proc = subprocess.run(
                        [str(gexe), "--seed", str(tmp), "--output", str(out)],
                        capture_output=True, text=True)
                    if proc.returncode != 0:
                        fail(f"{name} (sandbox materialization refused):\n"
                             + "\n".join((proc.stdout + proc.stderr).splitlines()[:6]))
                        return
                ta, tb = _read_tree(ga), _read_tree(gb)
                if ta != tb:
                    fail(f"{name} (the two grown outputs diverge)")
                if len(ta) != len(plan) + delta:
                    fail(f"{name} (expected {len(plan) + delta} files, got {len(ta)})")
                for rel in expect_new:
                    if rel not in ta:
                        fail(f"{name} (the grown output lacks {rel})")
                # The independent oracle grows with the plan, from typed facts
                # alone: a new package or plane is renderable, so a mismatch
                # here means the oracle carried a count or table it should not.
                if oracle_check:
                    grown = oracle_plan(tmp)
                    if ta != grown:
                        miss = sorted(set(grown) - set(ta))[:3]
                        extra = sorted(set(ta) - set(grown))[:3]
                        fail(f"{name} (the grown oracle disagrees with the grown "
                             f"materialization; missing {miss}, extra {extra})")
                if metadata_gains and cargo is not None:
                    gbuild = workdir / f"{name}-build"
                    shutil.copytree(ga, gbuild)
                    proc = subprocess.run(
                        [*cargo, "metadata", "--no-deps", "--format-version", "1"],
                        cwd=gbuild, capture_output=True, text=True)
                    if proc.returncode != 0 or metadata_gains not in {
                            p["name"] for p in json.loads(proc.stdout or "{}")
                            .get("packages", [])}:
                        fail(f"{name} (cargo metadata does not gain {metadata_gains})")
            finally:
                shutil.rmtree(tmp, ignore_errors=True)

        growth(
            "a_new_package_grows_the_plan_structurally",
            [(ARCH, "    BatPakExamples,\n}", "    BatPakExamples,\n    FuturePak,\n}"),
             (ARCH, "        PackageId::BatPakExamples,\n    ];",
              "        PackageId::BatPakExamples,\n        PackageId::FuturePak,\n    ];"),
             (ARCH, '            PackageId::BatPakExamples => "batpak-examples",\n        }\n    }\n\n    /// The workspace-relative path',
              '            PackageId::BatPakExamples => "batpak-examples",\n            PackageId::FuturePak => "futurepak",\n        }\n    }\n\n    /// The workspace-relative path'),
             (ARCH, '            PackageId::BatPakExamples => "examples",\n        }\n    }',
              '            PackageId::BatPakExamples => "examples",\n            PackageId::FuturePak => "crates/futurepak",\n        }\n    }'),
             (ARCH, '            PackageId::BatPakExamples => "BatPak examples",\n        }\n    }',
              '            PackageId::BatPakExamples => "BatPak examples",\n            PackageId::FuturePak => "FuturePak",\n        }\n    }'),
             (ARCH, "pub const QUALIFICATION_PROFILES",
              'pub const FUTUREPAK_SPEC: PackageSpec = PackageSpec { id: PackageId::FuturePak, role: "sandbox growth witness", class: PackageClass::Production, layer: 0 };\n\npub const QUALIFICATION_PROFILES'),
             (ARCH, "        layer: 5,\n    },\n];",
              "        layer: 5,\n    },\n    FUTUREPAK_SPEC,\n];"),
             (BOS, "        PackageId::BatPakExamples => Gate0PackageTargetKind::ExampleBinary,\n    }",
              "        PackageId::BatPakExamples => Gate0PackageTargetKind::ExampleBinary,\n        PackageId::FuturePak => Gate0PackageTargetKind::Library,\n    }"),
             (BOS, "        | PackageId::TestPak => None,\n    }",
              "        | PackageId::TestPak\n        | PackageId::FuturePak => None,\n    }")],
            ["crates/futurepak/Cargo.toml", "crates/futurepak/README.md",
             "crates/futurepak/src/lib.rs"],
            3, metadata_gains="futurepak")
        growth(
            "a_new_plane_grows_the_plan_structurally",
            [(ARCH, "    /// Explicit host requests and responses.\n    Port,\n}",
              "    /// Explicit host requests and responses.\n    Port,\n    /// Sandbox growth witness.\n    Futura,\n}"),
             (ARCH, "        SyncBatPlane::Port,\n    ];",
              "        SyncBatPlane::Port,\n        SyncBatPlane::Futura,\n    ];"),
             (ARCH, '            SyncBatPlane::Port => "port",\n        }',
              '            SyncBatPlane::Port => "port",\n            SyncBatPlane::Futura => "futura",\n        }'),
             (ARCH, "            | SyncBatPlane::Port => PackageId::SyncBat,",
              "            | SyncBatPlane::Port\n            | SyncBatPlane::Futura => PackageId::SyncBat,"),
             (ARCH, '            SyncBatPlane::Port => "port owns explicit host requests and responses",\n        }',
              '            SyncBatPlane::Port => "port owns explicit host requests and responses",\n            SyncBatPlane::Futura => "futura owns the sandbox growth witness",\n        }')],
            ["crates/syncbat/src/futura.rs", "crates/syncbat/src/futura/README.md"],
            2)
        # A lawful new root artifact grows the plan with NO cardinality edit:
        # the macro regenerates ALL from the variant list, so only the variant,
        # its path arm, and its render arm are touched -- not a single number.
        growth(
            "future_root_artifact_grows_without_count_edit",
            [(BOS, "        /// The discoverability-only command file.\n        Justfile,\n    }",
              "        /// The discoverability-only command file.\n        Justfile,\n        /// Sandbox growth witness.\n        SeedNote,\n    }"),
             (BOS, '            Gate0RootArtifact::Justfile => "justfile",\n        }',
              '            Gate0RootArtifact::Justfile => "justfile",\n            Gate0RootArtifact::SeedNote => "SEED-NOTE.md",\n        }'),
             ("bootstrap/materialize.rs",
              "            bootstrap_output::Gate0RootArtifact::Justfile => justfile(),",
              "            bootstrap_output::Gate0RootArtifact::Justfile => justfile(),\n"
              "            bootstrap_output::Gate0RootArtifact::SeedNote => \"sandbox growth witness\\n\".to_owned(),")],
            ["SEED-NOTE.md"],
            1, oracle_check=False)
    finally:
        shutil.rmtree(workdir, ignore_errors=True)

    findings.extend(canonical_drift(before))
    return findings


def test_bootstrap_qualification(audit) -> list[str]:
    """The typed Tier 0 receipt policy (5.5E6a). The admission ALGEBRA is
    executed by seedcheck (the tier0-seedcheck receipt); these fixtures pin the
    typed-owner laws the auditor reconstructs: closed inventories, the
    denominator's move out of Python, the product-identity firewall, the
    authoritative target, and the Delivery Notes projection."""
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new=""):
        tmp = gate_sandbox([(rel, old, new)])
        try:
            expect(name, audit.bootstrap_qualification_findings(tmp), needle)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    BQ = "spec/bootstrap_qualification.rs"
    TC = "spec/toolchain.rs"

    if audit.bootstrap_qualification_findings(root):
        fail(f"bootstrap_qualification_contract_passes "
             f"(got {audit.bootstrap_qualification_findings(root)!r})")

    probe("tier0_receipt_kind_missing_from_all_is_rejected", BQ,
          "        Tier0ReceiptKind::SpecTests,\n    ];",
          "SpecTests is authored but ALL omits it",
          "    ];")
    probe("duplicate_tier0_slug_is_rejected", BQ,
          'Tier0ReceiptKind::SpecTests => "tier0-spec-tests",',
          "is claimed twice",
          'Tier0ReceiptKind::SpecTests => "tier0-seedcheck",')
    probe("wrong_authoritative_target_is_rejected", TC,
          "pub const AUTHORITATIVE_TARGET: RustTargetTriple = RustTargetTriple::X86_64PcWindowsMsvc;",
          "not the MSVC triple",
          "pub const AUTHORITATIVE_TARGET: RustTargetTriple = RustTargetTriple::X86_64PcWindowsGnu;")
    probe("product_receipt_id_cannot_substitute_for_tier0_kind", BQ,
          "pub enum Tier0ReceiptKind {",
          "reuses the product ReceiptId",
          "pub type ReceiptId = u32;\n\npub enum Tier0ReceiptKind {")
    # Built at run time so THIS source never contains the contiguous
    # hand-authored tuple the audit law scans for. The Python denominator left
    # in E6b; a reintroduced slug tuple must redden the audit.
    forged_denominator = ('REQUIRED_TIER0_RECEIPTS = ((' + '"tier0-'
                          + 'law-fixtures", False),)')
    probe("python_denominator_cannot_return", "bootstrap/selftest.py",
          "def _tree_digest(root: Path) -> str:",
          "hand-authored slug tuple",
          forged_denominator + "\n\n\ndef _tree_digest(root: Path) -> str:")
    probe("tier0_denominator_block_drift_is_rejected", "DELIVERY_NOTES.md",
          "| tier0-materialize | ExecutableAndOutputTree |",
          "does not match its typed derivation",
          "| tier0-materialize | Executable |")

    findings.extend(canonical_drift(before))
    return findings


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
    canonical = HERE / f"{name}.py"
    before = _commit(canonical)
    source = canonical.read_text(encoding="utf-8")
    if target not in source:
        raise ProbeError(f"probe target absent in {name}.py: {target!r}")
    mutated = source.replace(target, replacement, 1)
    if mutated == source:
        raise ProbeError(f"probe mutation changed no bytes in {name}.py: {target!r}")
    tmp = Path(tempfile.mkdtemp(prefix="batpak-neuter-"))
    try:
        path = tmp / f"{name}.py"
        path.write_text(mutated, encoding="utf-8")
        spec = importlib.util.spec_from_file_location(f"{name}_neutered", path)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        yield module
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
        if _commit(canonical) != before:
            raise ProbeError(f"canonical {name}.py was modified during a probe")


def test_probe_harness(audit) -> list[str]:
    """The harness cannot leave residue in the canonical workspace (5.5D4a)."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    before = canonical_commitments()

    # 1 & 2: an exception or an abort mid-probe still cleans up, and the
    # canonical validator is untouched because it was never the target.
    leaked: list[Path] = []
    try:
        with isolated_tree() as tmp:
            leaked.append(tmp)
            raise RuntimeError("simulated probe explosion")
    except RuntimeError:
        pass
    if leaked and leaked[0].exists():
        fail("probe_exception_cannot_leave_validator_modified")
    if canonical_drift(before):
        fail("probe_exception_cannot_leave_validator_modified")

    leaked = []
    try:
        with isolated_tree() as tmp:
            leaked.append(tmp)
            # a timeout or cancellation surfaces as a BaseException, not Exception
            raise KeyboardInterrupt("simulated probe timeout/abort")
    except KeyboardInterrupt:
        pass
    if leaked and leaked[0].exists():
        fail("probe_abort_cannot_leave_validator_modified")
    if canonical_drift(before):
        fail("probe_abort_cannot_leave_validator_modified")

    # A neutered validator runs from a copy; the canonical file never moves.
    with neutered_validator("audit", 'if dec["class"] != "Capability":') as neutered:
        if neutered.specialization_findings(root):
            fail("neutered_validator_runs_from_a_copy")
    if canonical_drift(before):
        fail("neutered_validator_cannot_modify_canonical_bytes")

    # An abort inside a neutered probe still leaves canonical bytes intact.
    try:
        with neutered_validator("audit", 'if dec["class"] != "Capability":'):
            raise KeyboardInterrupt("simulated timeout while a rule is disabled")
    except KeyboardInterrupt:
        pass
    if canonical_drift(before):
        fail("probe_abort_cannot_leave_validator_modified")

    # 3 & 4: the harness fails closed on a broken probe rather than passing.
    try:
        with neutered_validator("audit", "zzz-absent-target-zzz"):
            pass
    except ProbeError:
        pass
    else:
        fail("absent_probe_target_is_rejected")
    try:
        must_replace("abc", "abc", "abc", "self-check")
    except ProbeError:
        pass
    else:
        fail("inert_probe_mutation_is_rejected")

    # 5: a probe that fires the WRONG diagnostic is not a pass. A red light from
    # the wrong severed wire is not evidence that the alarm works.
    with neutered_validator("audit", 'if dec["class"] != "Capability":') as neutered:
        produced = neutered.decision_class_findings(root)
        if any("unknown DecisionClass" in f for f in produced):
            fail("wrong_diagnostic_class_is_rejected")

    # 6: the canonical workspace is never a probe target.
    with isolated_tree() as tmp:
        if tmp == root or str(tmp).startswith(str(root)):
            fail("canonical_workspace_is_not_a_probe_target")
        (tmp / "spec" / "gates.rs").write_text("poisoned", encoding="utf-8")
        if (root / "spec" / "gates.rs").read_text(encoding="utf-8") == "poisoned":
            fail("canonical_workspace_is_not_a_probe_target")
    if canonical_drift(before):
        fail("canonical_workspace_is_not_a_probe_target")

    findings.extend(canonical_drift(before))
    return findings


def test_integrity_witnesses(audit) -> list[str]:
    """LEG-023 hostile integrity witnesses and the witness-reference resolver.

    Structural only. Bootstrap executes no journal verification, reads no real
    chain, detects no real event loss, and runs no SIDX recovery. These fixtures
    prove the witness identities, owner bindings, meanings, and cross-surface
    agreement survive -- not that a chain was verified.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    iw = audit.integrity_witness_findings

    if audit.witness_reference_findings(root) or iw(root):
        fail("leg023_witness_contract_passes")

    # Each of the seven witness identities must survive in the typed catalog
    # (5.5E4d: spec/proof.rs owns membership; docs/24 owns meaning only).
    for wid in audit.D4B_LEG023_WITNESSES:
        probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
              f'id: ProofRowId("{wid}")', f"{wid} is absent from docs/24",
              f'id: ProofRowId("{wid}_gone")', validator=iw)

    # The resolver: unknown, duplicated, misowned, missing, extra.
    probe("unknown_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "references unknown proof-row id ghost_witness",
          "ghost_witness; middle_event_deletion_is_rejected;")
    probe("duplicate_leg_witness_reference_is_rejected", D21,
          "middle_event_deletion_is_rejected;", "projects a duplicate witness reference",
          "middle_event_deletion_is_rejected; middle_event_deletion_is_rejected;")
    probe("witness_owned_by_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("cross_lane_predecessor_is_rejected"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")',
          "LEG-023 witness cross_lane_predecessor_is_rejected is bound to LEG-081",
          'ProofRowId("cross_lane_predecessor_is_rejected"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081")',
          validator=iw)
    probe("docs21_missing_owned_witness_is_rejected", D21,
          "; midstream_genesis_is_rejected", "omits owned proof row midstream_genesis_is_rejected")
    probe("docs21_extra_witness_is_rejected", D21,
          "middle_event_deletion_is_rejected;",
          "references unknown proof-row id unknown_leg_proof_row_for_fixture",
          "unknown_leg_proof_row_for_fixture; middle_event_deletion_is_rejected;")
    ROW23F = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
              'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("duplicate_docs24_proof_row_id_is_rejected", "spec/proof.rs", ROW23F,
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          ROW23F + "\n    " + ROW23F)
    probe("witness_meaning_removal_is_rejected", GA,
          "midstream_genesis_is_rejected\n    A genesis marker cannot reset an already-started stream.",
          "has no authoritative meaning", "")

    # Weakening the law itself: each impersonation the law forbids.
    for clause, label in (
        ("never proves EventCommitment equality", "content_digest_substituting_for_event_commitment_is_rejected"),
        ("an event appearing somewhere in a verified set is not thereby the immediate predecessor",
         "set_membership_accepted_as_predecessor_is_rejected"),
        ("legal genesis occurs only at the lawful stream head", "midstream_genesis_allowed_is_rejected"),
        ("may not select the expected authority bytes and then use those selected bytes to authenticate its own selection",
         "forged_index_selecting_and_authenticating_is_rejected"),
        ("requires the exact immediate predecessor", "inexact_predecessor_allowed_is_rejected"),
        ("lane isolation is owned by LEG-050", "leg023_absorbing_leg050_is_rejected"),
        ("visible linearization by LEG-067", "leg023_absorbing_leg067_is_rejected"),
    ):
        probe(label, LEG, clause, f"LEG-023 law no longer states: {clause}", "", validator=iw)

    findings.extend(canonical_drift(before))
    return findings


def test_derived_material_witnesses(audit) -> list[str]:
    """LEG-019 duals plus the LEG-020/LEG-028 bounded-work rows (5.5D4b-2a).

    Structural only. Bootstrap runs no SIDX recovery, detects no real loss,
    performs no traversal, and measures no allocation.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    LEG = "spec/legacy_obligations.rs"
    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    dm = audit.derived_material_findings

    if dm(root) or audit.witness_reference_findings(root):
        fail("d4b2a_witness_contract_passes")

    # every new proof-row identity survives in the typed catalog (5.5E4d)
    for leg, ids in audit.D4B2A_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
                  f'id: ProofRowId("{wid}")',
                  f"{leg} witness {wid} is absent from docs/24",
                  f'id: ProofRowId("{wid}_gone")', validator=dm)

    # the law's load-bearing clauses -- both duals must stay
    for clause, label in (
        ("cannot prove safety and cannot prove loss", "sidx_proving_loss_or_safety_is_rejected"),
        ("proves only that exact row-to-authoritative-frame relationship",
         "corroborated_row_authenticating_siblings_count_order_or_tail_is_rejected"),
        ("never an authoritative proven-loss or proven-safety conclusion",
         "derived_table_reaching_an_authoritative_conclusion_is_rejected"),
        ("no trusted-local SIDX authority", "trusted_local_sidx_authority_is_rejected"),
        ("cannot authenticate themselves", "derived_material_self_authentication_is_rejected"),
    ):
        probe(label, LEG, clause, f"LEG-019 law no longer states: {clause}", "", validator=dm)

    # bounded discovery and bounded allocation keep distinct owners
    probe("bounded_output_accepted_as_bounded_discovery_work_is_rejected", "spec/proof.rs",
          'id: ProofRowId("page_limit_bounds_discovery_work_not_only_output")',
          "LEG-028 witness page_limit_bounds_discovery_work_not_only_output is absent",
          'id: ProofRowId("page_limit_bounds_discovery_work_not_only_output_gone")',
          validator=dm)
    probe("full_matched_set_allocation_accepted_is_rejected", "spec/proof.rs",
          'id: ProofRowId("allocation_does_not_scale_with_full_matched_set")',
          "LEG-020 witness allocation_does_not_scale_with_full_matched_set is absent",
          'id: ProofRowId("allocation_does_not_scale_with_full_matched_set_gone")',
          validator=dm)
    probe("discovery_and_allocation_sharing_one_owner_is_rejected", "spec/proof.rs",
          'ProofRowId("allocation_does_not_scale_with_full_matched_set"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-020")',
          "share one qualification target",
          'ProofRowId("allocation_does_not_scale_with_full_matched_set"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028")',
          validator=dm)

    # resolver coverage for the new rows
    probe("d4b2a_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("forged_sibling_cannot_cause_false_loss"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-019")',
          "which docs/24 binds to LEG-023",
          'ProofRowId("forged_sibling_cannot_cause_false_loss"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")')
    probe("docs21_missing_a_d4b2a_row_is_rejected", D21,
          "; derived_row_cannot_prove_absence_or_loss",
          "omits owned proof row derived_row_cannot_prove_absence_or_loss")
    probe("docs21_extra_d4b2a_row_is_rejected", D21,
          "allocation_does_not_scale_with_full_matched_set",
          "references derived_row_cannot_authenticate_order, which docs/24 binds to LEG-019",
          "derived_row_cannot_authenticate_order")
    probe("duplicate_row_across_d4b1_and_d4b2a_is_rejected", "spec/proof.rs",
          'id: ProofRowId("forged_sibling_cannot_cause_false_loss")',
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          'id: ProofRowId("middle_event_deletion_is_rejected")')
    probe("d4b2a_meaning_removed_is_rejected", GA,
          "derived_row_cannot_authenticate_order\n    Corroborating row contents does not establish table ordering.",
          "has no authoritative meaning", "")

    findings.extend(canonical_drift(before))
    return findings


def test_deferred_witnesses(audit) -> list[str]:
    """LEG-043 deferred native rows and the LEG-074 reimport row (5.5D4b-2b,
    rebased in 5.5E4d): the typed catalog owns membership; the fence-header
    posture prose retired with the membership mirrors, and execution evidence
    lives in run receipts. What survives here is membership and meaning."""
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    dp = audit.deferred_posture_findings

    if dp(root) or audit.witness_reference_findings(root):
        fail("d4b2b_witness_contract_passes")

    # every new proof-row identity survives in the typed catalog
    for leg, ids in audit.D4B2B_ROWS.items():
        for wid in ids:
            probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
                  f'id: ProofRowId("{wid}")',
                  f"{leg} witness {wid} is absent from docs/24",
                  f'id: ProofRowId("{wid}_gone")', validator=dp)
    probe("leg043_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-043")',
          "LEG-043 witness fcntl_getfd_failure_fails_closed is bound to LEG-074",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-074")',
          validator=dp)

    # meanings carry the law they exist to defend
    # A proof row's MEANING is what a future implementation qualifies against.
    # Removing a clause from it is a weakening even though the ID survives.
    for frag, label, needle in (
        ("not reported as established",
         "descriptor_postcondition_reported_applied_after_failure_is_rejected",
         "meaning no longer states: not reported as established"),
        ("Failure to read descriptor flags cannot produce a verified close-on-exec",
         "fcntl_read_failure_reported_verified_is_rejected",
         "meaning no longer states: Failure to read descriptor flags"),
        ("Failure to apply descriptor flags cannot produce an applied or verified",
         "fcntl_write_failure_reported_applied_is_rejected",
         "meaning no longer states: Failure to apply descriptor flags"),
        ("imports zero new events", "reimport_creating_new_events_is_rejected",
         "meaning no longer states: imports zero new events"),
        ("already-established authority outcome", "reimport_inventing_an_outcome_is_rejected",
         "meaning no longer states: already-established authority outcome"),
        ("witness binds stable import identity", "reimport_losing_bound_identities_is_rejected",
         "meaning no longer states: witness binds stable import identity"),
    ):
        probe(label, GA, frag, needle, "", validator=audit.proof_meaning_findings)

    # resolver coverage for the new rows, on the typed catalog (5.5E4d)
    probe("d4b2b_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("close_reopen_reimport_returns_zero_new_events"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-074")',
          "which docs/24 binds to LEG-023",
          'ProofRowId("close_reopen_reimport_returns_zero_new_events"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023")')
    probe("duplicate_row_across_prior_d4b_passes_is_rejected", "spec/proof.rs",
          'ProofRowId("fcntl_getfd_failure_fails_closed"), state',
          "binds proof-row id middle_event_deletion_is_rejected more than once",
          'ProofRowId("middle_event_deletion_is_rejected"), state')

    findings.extend(canonical_drift(before))
    return findings


def test_leg081_authority(audit) -> list[str]:
    """LEG-081 proof-row authority migration (5.5D4b-2c).

    Structural only. Bootstrap never shredded a key, never called a key backend,
    never crashed a store, and never inspected key material. These fixtures prove
    that the nine proof contracts are owned in one place, projected exactly
    elsewhere, and that every clause of the typed law carries a witness.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.witness_reference_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    D21 = "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md"
    D35 = "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md"
    la = audit.leg081_authority_findings
    pm = audit.proof_meaning_findings
    H81 = ("Required witnesses (proof owner TestPak; gates G2/G3; future executable: yes; "
           "bootstrap executed: no), also carried by `LEG-081`:")
    PROJ = ("Required proof rows, projected from docs/24 (qualification target: LEG-081; "
            "canonical proof-row owner: docs/24 Gauntlet):")
    LEAK = "travel only through an explicit, independent ceremony."

    if la(root) or audit.witness_reference_findings(root) or pm(root):
        fail("d4b2c_leg081_contract_passes")

    # The retarget premise: the old fixture id is now owned, and its replacement
    # is genuinely unknown. Without both, the retargeted probe would go red for
    # the wrong severed wire.
    rows = audit.witness_rows(root)
    if rows.get("shred_ack_waits_for_backend_durability", {}).get("leg") != "LEG-081":
        fail("retired_fixture_id_is_now_owned_by_leg081")
    if "unknown_leg_proof_row_for_fixture" in rows:
        fail("replacement_fixture_id_is_genuinely_unknown")

    # Every canonical identity survives in the typed catalog (5.5E4d).
    for wid in audit.D4B2C_ROWS:
        probe(f"{wid}_identity_removed_is_rejected", "spec/proof.rs",
              f'id: ProofRowId("{wid}")',
              f"LEG-081 canonical proof row {wid} is absent from docs/24",
              f'id: ProofRowId("{wid}_gone")', validator=la)

    # Retired vocabulary cannot return outside the historical migration note, in
    # ANY document: one registry, one rule. `stale_or_pre_shred_keyset_restore_is_rejected`
    # contains a retired id as a substring; the clean tree passing above proves the
    # boundary rule does not fire on it, and these prove it does fire on the bare
    # retired id. docs/35 is the plant site precisely because it owns none of these
    # rows now: a rule scoped to a phase's own documents would miss the leak.
    rp = audit.retired_proof_row_findings
    for old in audit.retired_proof_rows(HERE.parent):
        probe(f"retired_{old}_reintroduced_is_rejected", D35, LEAK,
              f"reintroduces retired proof-row id {old} outside the historical migration note",
              LEAK + f" See {old}.", validator=rp)

    # The catalog is TYPED and must agree with the migration notes in both
    # directions (5.5E2). The Python-dict era shipped exactly this defect: a
    # docs/24 note retired an id the dict never learned, so its scanner
    # guarded five of six retirements while claiming to guard them all.
    PR = "spec/proof.rs"
    probe("registry_missing_a_noted_retirement_is_rejected", PR,
          '    ProofRowRecord { id: ProofRowId("test_local_fixture_type_is_classified_correctly"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")] } },\n',
          "spec/proof.rs PROOF_ROWS never learned",
          "", validator=rp)
    probe("registry_retirement_without_a_note_is_rejected", PR,
          'ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"),',
          "no docs/24 migration note recording the retirement",
          'ProofRowRecord { id: ProofRowId("phantom_row_never_noted"),', validator=rp)

    # The catalog is the LIVING CENSUS (5.5E2j): active identities equal the
    # structurally parsed canonical docs/24 rows in both directions, one
    # identity carries one lifecycle, and both lifecycle states are
    # constructed. A name surviving in a migration note or prose is not a row.
    cat = audit.proof_row_catalog_findings
    if cat(HERE.parent):
        fail("proof_row_catalog_passes_on_the_real_seed")
    # The typed-vs-docs/24 membership parity retired in 5.5E4d: the typed
    # catalog is the one membership authority, and the meaning-coverage laws
    # (witness_reference_findings) own the docs/24 side.
    ROW23C = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
              'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("same_identity_cannot_be_active_and_retired", PR, ROW23C,
          "is declared twice in the proof-identity catalog",
          ROW23C + '\n    ProofRowRecord { id: '
          'ProofRowId("middle_event_deletion_is_rejected"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("event_reorder_is_rejected")] } },', validator=cat)
    # E3 preflight: succession terminates. A two-node cycle satisfies
    # existence and non-self-succession while owning no living obligation;
    # a multi-hop chain through a retired row that reaches an Active identity
    # is lawful.
    RETIRED_PAIR = (
        '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },\n'
        '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct")] } },\n')
    RETIRED_CYCLE = (
        '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_and_keyset_missing_remain_distinct")] } },\n'
        '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("pre_shred_keyset_restore_is_rejected")] } },\n')
    probe("two_retired_rows_cannot_succeed_each_other_cyclically", PR,
          RETIRED_PAIR, "retirement succession is cyclic",
          RETIRED_CYCLE, validator=cat)
    probe("retirement_component_with_no_active_leaf_is_rejected", PR,
          RETIRED_PAIR, "retirement path terminates in no Active identity",
          RETIRED_CYCLE, validator=cat)
    # A GREEN fixture: the lawful multi-hop chain must produce no finding.
    tmp = gate_sandbox([(PR,
        'ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },',
        'ProofRowId("pre_shred_keyset_restore_is_rejected"), '
        'state: ProofRowState::Retired { successors: '
        '&[ProofRowId("shredded_and_keyset_missing_remain_distinct")] } },')])
    try:
        got = cat(tmp)
        if got:
            fail(f"multi_hop_retirement_chain_terminates_in_active_identity (got {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    probe("proof_row_state_variants_are_constructed", PR,
          '    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },\n'
          '    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct")] } },\n'
          '    ProofRowRecord { id: ProofRowId("snapshot_and_fork_exclude_keys_by_default"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys")] } },\n'
          '    ProofRowRecord { id: ProofRowId("hash_map_iteration_cannot_change_canonical_bytes"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("hash_map_iteration_cannot_influence_canonical_observables")] } },\n'
          '    ProofRowRecord { id: ProofRowId("attempt_receipt_cannot_cross_invocation_classes"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("entrypoint_receipt_cannot_satisfy_query_program_execution"), '
          'ProofRowId("query_program_receipt_cannot_satisfy_entrypoint_invocation")] } },\n'
          '    ProofRowRecord { id: ProofRowId("test_local_fixture_type_is_classified_correctly"), '
          'state: ProofRowState::Retired { successors: '
          '&[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")] } },\n',
          "constructs no Retired row",
          "", validator=cat)

    # docs/21 projects exactly the nine.
    probe("docs21_leg081_missing_projected_id_is_rejected", D21,
          "; foreign_keyset_generation_is_rejected",
          "omits owned proof row foreign_keyset_generation_is_rejected")
    probe("docs21_leg081_extra_projected_id_is_rejected", D21,
          "shred_ack_waits_for_backend_durability;",
          "references middle_event_deletion_is_rejected, which docs/24 binds to LEG-023",
          "middle_event_deletion_is_rejected; shred_ack_waits_for_backend_durability;")

    # docs/35 projects exactly the nine and reclaims nothing.
    # docs/35's hand list is a REGISTERED generated view since 5.5E4d
    # (SecretAuthorityProofRequirements): the universal census and domain
    # projection parity own its membership now. What survives is the reclaim
    # fence: docs/35 may never again author ownership surfaces.
    probe("docs35_reclaims_authoritative_ownership_is_rejected", D35, LEAK,
          "docs/35 reclaims authoritative proof-row ownership",
          LEAK + "\n\nRequired witnesses (proof owner TestPak; gates G2/G3), also carried by "
          "`LEG-081`:\n\n```text\nshred_ack_waits_for_backend_durability\n```\n",
          validator=la)
    probe("docs35_reclaims_authoritative_witness_meaning_is_rejected", D35, LEAK,
          "docs/35 reclaims per-ID authoritative witness meaning",
          LEAK + "\n\nAuthoritative meanings:\n\n```text\n"
          "shred_ack_waits_for_backend_durability\n    A local restatement.\n```\n",
          validator=la)

    # docs/24 row posture, owner, gates, target, meaning, uniqueness.
    probe("docs24_leg081_row_bound_to_wrong_leg_is_rejected", "spec/proof.rs",
          'ProofRowId("shred_ack_waits_for_backend_durability"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081")',
          "LEG-081 canonical proof row shred_ack_waits_for_backend_durability is bound to LEG-020",
          'ProofRowId("shred_ack_waits_for_backend_durability"), state: '
          'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-020")',
          validator=la)
    probe("docs24_leg081_meaning_removal_is_rejected", GA,
          "shred_transition_binding_mismatch_is_rejected\n    The shred transition binds StoreId,"
          " AuthorityGeneration, KeyGeneration, key\n",
          "LEG-081 proof row shred_transition_binding_mismatch_is_rejected has no authoritative meaning",
          "", validator=la)
    ROW81F = ('ProofRowRecord { id: ProofRowId("foreign_keyset_generation_is_rejected"), '
              'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-081"), '
              'projection_contracts: &[ContractId("BP-CRYPTO-SECRET-1")], claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } },')
    probe("duplicate_leg081_proof_row_id_is_rejected", "spec/proof.rs", ROW81F,
          "docs/24 binds LEG-081 proof row foreign_keyset_generation_is_rejected more than once",
          ROW81F + "\n    " + ROW81F, validator=la)
    probe("leg081_proof_row_id_rename_is_rejected", "spec/proof.rs",
          'ProofRowId("foreign_keyset_generation_is_rejected")',
          "docs/24 binds unexpected proof row foreign_keyset_generation_renamed to LEG-081",
          'ProofRowId("foreign_keyset_generation_renamed")', validator=la)

    # Meanings still carry the law they exist to defend.
    for frag, label in (
        ("established destruction of the relevant key authority.",
         "shred_ack_durability_clause_removal_is_rejected"),
        ("acknowledgement or a successful final shred result.",
         "crash_recovery_posture_clause_removal_is_rejected"),
        ("scope, and shred-transition identity.",
         "binding_components_clause_removal_is_rejected"),
        ("projection or formatter may collapse one into another.",
         "state_distinction_clause_removal_is_rejected"),
        ("not export usable raw secret-key bytes",
         "raw_key_export_clause_removal_is_rejected"),
        ("never semantics.",
         "external_backend_parity_clause_removal_is_rejected"),
    ):
        probe(label, GA, frag, "meaning no longer states", "", validator=pm)

    # Every law clause keeps an owned executable witness.
    probe("coverage_clause_missing_is_rejected", GA,
          "external backend semantic parity\n    external_key_backend_preserves_shred_semantics\n",
          "LEG-081 coverage matrix omits law clause: external backend semantic parity",
          "", validator=la)
    probe("coverage_clause_with_no_proof_row_is_rejected", GA,
          "external backend semantic parity\n    external_key_backend_preserves_shred_semantics",
          "LEG-081 coverage clause names no proof row: external backend semantic parity",
          "external backend semantic parity", validator=la)
    probe("canonical_row_omitted_from_coverage_matrix_is_rejected", GA,
          "    reopen_after_ack_cannot_recover_shredded_plaintext\n",
          "LEG-081 canonical proof row reopen_after_ack_cannot_recover_shredded_plaintext"
          " appears in no coverage clause", "", validator=la)
    probe("unknown_proof_row_in_coverage_matrix_is_rejected", GA,
          "    external_key_backend_preserves_shred_semantics\n",
          "LEG-081 coverage matrix names unknown proof row ghost_coverage_row",
          "    external_key_backend_preserves_shred_semantics\n    ghost_coverage_row\n",
          validator=la)
    probe("coverage_matrix_removed_is_rejected", GA, "LEG-081 proof-coverage matrix:",
          "docs/24 states no LEG-081 proof-coverage matrix",
          "LEG-081 proof-coverage notes:", validator=la)

    findings.extend(canonical_drift(before))
    return findings


def test_proof_target_resolver(audit) -> list[str]:
    """Generic source-qualified GuaranteeRef resolution (5.5D4b-3b0).

    Structural only. Bootstrap executes no proof row of any family. These
    fixtures prove the resolver accepts every guarantee family through one row
    format, reads gates from the typed target rather than the source block, and
    fails closed on an unknown or non-guarantee target.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    def silent(name: str, produced) -> None:
        if produced:
            fail(f"{name} (expected no finding, got {produced!r})")

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, f"{rel}: {old[:48]!r}"), encoding="utf-8")
            expect(name, (validator or audit.proof_target_findings)(tmp), needle)

    GA = "docs/24_GAUNTLET.md"
    pt = audit.proof_target_findings
    pg = audit.proof_target_grammar_findings
    # LEG-028 owns exactly one row, so retargeting its block cannot collide.
    H28 = "Required witnesses (proof owner TestPak; gates G2), also carried by `LEG-028`:"
    W28 = "page_limit_bounds_discovery_work_not_only_output"

    if pt(root) or pg(root) or audit.witness_reference_findings(root):
        fail("d4b3b0_resolver_contract_passes")

    # The index resolves every declared guarantee, and only those.
    idx = audit.guarantee_index(root)
    if len(idx) != 216:
        fail(f"guarantee_index_covers_every_guarantee (got {len(idx)})")
    for ref, want in (("LEG-081", "G2/G3"), ("DEC-065", "G0/G5"), ("SEED-FBAT-CORE", "G2")):
        if audit.guarantee_gates(root, ref) != want:
            fail(f"{ref}_gates_resolve_from_typed_target")
    # ARCH and QUAL now resolve typed gate postures (5.5D4b): ARCH from its
    # declared family policy, QUAL from its own row. Before the authority closure
    # both returned None and a qualification target sat in the gate field.
    for ref, want in (("ARCH-batpak", "G0"), ("QUAL-batpak-native", "G0/G5"),
                      ("QUAL-syncbat-browser", "G0/G5")):
        if audit.guarantee_gates(root, ref) != want:
            fail(f"{ref}_resolves_its_typed_gate_posture "
                 f"(wanted {want}, got {audit.guarantee_gates(root, ref)!r})")
    # A qualification target must never be readable as a gate.
    if audit.guarantee_gates(root, "QUAL-batpak-semantic") in ("no_std + alloc", "std", "wasm32 host"):
        fail("qualification_target_cannot_be_read_as_a_gate_posture")
    # LEG resolution is unchanged by the widening.
    if audit.leg_gates(root, "LEG-043") != "G5" or audit.leg_gates(root, "LEG-081") != "G2/G3":
        fail("leg_gates_behaviour_unchanged_by_widening")
    if audit.leg_gates(root, "DEC-065") != "":
        fail("leg_gates_refuses_a_non_leg_ref")

    # Target selection is TYPED since 5.5E4d: the fence-header retarget,
    # multi-target, family-grammar, and block-gate-override laws became
    # structural (an Active row carries exactly one GuaranteeRef and no gate
    # text). What survives executable here: a dangling member fails closed.
    probe("unknown_guarantee_member_fails_closed", "spec/proof.rs",
          'guarantee: GuaranteeRef::dec("DEC-050")',
          "targets DEC-999, which resolves to no existing DEC guarantee",
          'guarantee: GuaranteeRef::dec("DEC-999")')
    probe("dangling_legacy_guarantee_fails_closed", "spec/proof.rs",
          'guarantee: GuaranteeRef::leg("LEG-028")',
          "targets LEG-998, which resolves to no existing LEG guarantee",
          'guarantee: GuaranteeRef::leg("LEG-998")')

    # Structural discovery: heading text is not the parser API.
    #
    # These plant synthetic blocks rather than asserting against whichever
    # unnormalized block still happens to exist. The earlier form read the label
    # of a real docs/12 candidate, so normalizing that block would have broken the
    # probe -- the test measured the debt instead of the parser, and would have
    # had nothing to stand on once D4d drives the candidate count to zero.
    PLANT = "docs/12_TESTPAK.md"
    IDS = "planted_fixture_alpha_is_rejected\nplanted_fixture_beta_is_rejected"

    def plant(label: str) -> list[dict]:
        with isolated_tree() as tmp:
            p = tmp / PLANT
            p.write_text(p.read_text(encoding="utf-8")
                         + f"\n\n{label}\n\n```text\n{IDS}\n```\n", encoding="utf-8")
            return [c for c in audit.candidate_fences(tmp) if c["doc"] == PLANT]

    base = len([c for c in audit.candidate_fences(root) if c["doc"] == PLANT])
    # A block naming a proof owner is a candidate whatever noun its label uses.
    for label in ("Named hostile fixtures (proof owner TestPak; gates G3):",
                  "Hostile fixture obligations (proof owner: TestPak; gates G3)",
                  "Completely different authored label (proof owner TestPak; gates G3):"):
        got = plant(label)
        if len(got) != base + 1:
            fail(f"planted_block_is_discovered_structurally ({label!r})")
        if got[-1]["kind"] != "UnboundCandidate":
            fail(f"planted_block_with_a_proof_owner_is_an_unbound_candidate ({label!r})")
    # A gate token alone is executable metadata: no proof-owner noun required.
    if plant("Implemented at G3:")[-1]["kind"] != "UnboundCandidate":
        fail("planted_block_naming_only_a_gate_is_an_unbound_candidate")
    # Prose with no executable metadata is evidence, never a promotable obligation.
    if plant("Illustrative scenarios:")[-1]["kind"] != "DescriptiveEvidence":
        fail("fence_without_executable_metadata_is_descriptive_not_promoted")

    # Universal expectation-clause enforcement. The transitional ceiling is
    # spent (D4d): every canonical row carries the clause, the baseline count is
    # zero, and ANY row without one -- newly promoted or newly stripped -- is a
    # finding. The probes therefore corrupt the authored clause on the LEG-028
    # row instead of granting it one.
    ids, blocks, pending = audit.candidate_summary(root)
    if (ids, blocks, pending) != (0, 0, 0):
        fail(f"candidate_summary_reports_current_state (got {(ids, blocks, pending)})")
    W28E = ("    expects: discovery under a page limit and work budget observes decode,\n"
            "      allocation, and materialization work bounded by the declared budget\n"
            "      while the matched set grows far beyond the page")
    W28D = ("    disposition: a page of results with within-budget work evidence, or a\n"
            "      typed budget refusal; a full-set scan truncated at output fails the\n"
            "      witness")
    W28C = W28E + "\n" + W28D
    probe("expectation_clause_without_disposition_is_rejected", GA, W28C,
          f"proof row {W28} expectation clause states no disposition", W28E)
    probe("expectation_clause_without_predicate_is_rejected", GA, W28C,
          f"proof row {W28} expectation clause states no expects", W28D)
    probe("unfalsifiable_expectation_clause_is_rejected", GA, W28E,
          "expectation clause expects is not falsifiable",
          "    expects: TBD")
    ROW28 = ('ProofRowRecord { id: ProofRowId("page_limit_bounds_discovery_work_not_only_output"), '
             'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028"), '
             'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::ResourceEnvelope, verification: PLAN_COMPLEXITY } },')
    probe("newly_promoted_row_without_an_expectation_clause_is_rejected", "spec/proof.rs",
          ROW28,
          "proof rows carry no expectation clause, above the transitional ceiling of 0",
          ROW28 + '\n    ProofRowRecord { id: '
          'ProofRowId("newly_promoted_row_without_a_clause"), '
          'state: ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-028"), '
          'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], claim: VerificationClaimKind::ResourceEnvelope, verification: PLAN_COMPLEXITY } },')
    # Enforcement is universal, not grandfathered: stripping the clause from a
    # long-migrated row is refused exactly like omitting it on a new one.
    probe("stripping_a_migrated_clause_is_rejected", GA, W28C,
          "proof rows carry no expectation clause, above the transitional ceiling of 0")
    # A clause may wrap, so the scan must read the folded value. Before the fold
    # existed, this mutation passed: the vague qualifier sits on the continuation
    # line, and a first-line-only parser never saw it.
    probe("unfalsifiable_expectation_clause_continuation_is_rejected", GA, W28E,
          "expectation clause expects is not falsifiable",
          W28E + ",\n      as appropriate")
    # The authored clause is itself wrapped: the scan must return the whole
    # folded value, not the first line it happens to see.
    got = audit.witness_expectations(root).get(W28, {}).get("expects", "")
    if got != ("discovery under a page limit and work budget observes decode, "
               "allocation, and materialization work bounded by the declared budget "
               "while the matched set grows far beyond the page"):
        fail(f"expectation_clause_continuation_folds_into_its_value (got {got!r})")

    findings.extend(canonical_drift(before))
    return findings


def test_guarantee_authority(audit, project) -> list[str]:
    """Guarantee admission and projection authority (5.5D4b closure).

    These are the adversaries that would have caught the C1 defect. The old suite
    attacked mutation and parity, and both are blind to common-mode invention: two
    scripts carrying the same unauthorized constant agree perfectly.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def expect(name: str, produced, needle: str) -> None:
        if not any(needle in f for f in produced):
            fail(f"{name} (wanted {needle!r}, got {produced!r})")

    GU = "spec/guarantees.rs"
    DI = "spec/dispositions.rs"
    AR = "spec/architecture.rs"

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new,
                                         f"{rel}: {old[:44]!r}"), encoding="utf-8")
            expect(name, (validator or audit.guarantee_admission_findings)(tmp), needle)

    if audit.guarantee_admission_findings(root):
        fail("every_node_passes_typed_admission")

    # 1. No semantic literal may live in both bootstrap paths. A constant in two
    #    "independent" scripts is common-mode invention, not evidence.
    import ast as _ast

    def lits(rel):
        tree = _ast.parse((HERE / rel).read_text(encoding="utf-8"))
        return {n.value for n in _ast.walk(tree)
                if isinstance(n, _ast.Constant) and isinstance(n.value, str)}

    shared = lits("project.py") & lits("audit.py")
    banned = {"Permanent", "docs/30_DECISION_AND_REJECTION_LEDGER.md", "spec/architecture.rs",
              "LegacyObligation", "Decision", "ArchitectureConstraint", "QualificationRequirement",
              "spec/architecture.rs; audit.py architecture checks"}
    pj = (HERE / "project.py").read_text(encoding="utf-8")
    au = (HERE / "audit.py").read_text(encoding="utf-8")
    for lit in sorted(shared & banned):
        for dim in ("lifetime", "owner", "kind", "gates", "witness"):
            asn = f'"{dim}": "{lit}"'
            if asn in pj and asn in au:
                fail(f"common_mode_semantic_constant_{dim}_{lit[:24]}_present_in_both_paths")

    # 2. Every family policy is declared; nothing is left to a projector default.
    pol = audit.guarantee_family_policies(root)
    if set(pol) != {"SEED", "LEG", "DEC", "ARCH", "QUAL"}:
        fail(f"all_five_families_declare_typed_policy (got {sorted(pol)})")
    if project.family_policies(root) != pol:
        fail("generator_and_auditor_read_the_same_typed_family_policy")
    if audit.decision_lifetime_map(root) != project.disposition_lifetimes(root):
        fail("generator_and_auditor_read_the_same_typed_disposition_map")

    # 3. The DEC lifetime map is owned by the typed source, not by either script.
    dm = audit.decision_lifetime_map(root)
    for disp, life in (("Lock", "Permanent"), ("Keep", "Permanent"), ("Kill", "Permanent"),
                       ("Defer", "Permanent"), ("OpenImplementation", "UntilGate"),
                       ("Demote", "UntilCompatibilityExpiry"),
                       ("Supersede", "HistoricalCoverageOnly"),
                       ("RetainAsEvidence", "HistoricalCoverageOnly")):
        if dm.get(disp) != life:
            fail(f"disposition_{disp}_maps_to_{life} (got {dm.get(disp)!r})")
    probe("illegal_disposition_to_lifetime_mapping_is_rejected", DI,
          "Disposition::OpenImplementation => GuaranteeLifetime::UntilGate,",
          "has no typed lifetime", "", validator=audit.guarantee_admission_findings)

    # 4. A projector may not supply a semantic default when policy is absent.
    probe("family_policy_removal_fails_closed", GU,
          '        family: "ARCH",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),',
          "no declared family policy for ARCH",
          '        family: "ARCH-RENAMED",\n        kind: KindRule::FamilyConstant(GuaranteeKind::ArchitectureConstraint),')
    probe("family_constant_without_a_value_fails_closed", GU,
          'owner: OwnerRule::FamilyConstant("docs/30_DECISION_AND_REJECTION_LEDGER.md"),',
          "owner is FamilyConstant but names no value",
          'owner: OwnerRule::FamilyConstant(""),')

    # 5. A qualification target may never enter a gate field.
    probe("qualification_target_in_a_gate_field_is_rejected", AR,
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::G0, GateId::G5],",
          "names a value that is not a declared GateId",
          "environment: QualificationEnvironment::NativeStd,\n"
          "        gates: &[GateId::std],")
    if any(audit.guarantee_gates(root, f"QUAL-{p}") in ("std", "no_std + alloc", "wasm32 host")
           for p in ("batpak-native", "batpak-semantic", "syncbat-browser")):
        fail("qualification_target_cannot_be_read_as_a_gate_posture")

    # 6. Missing metadata is not gate-independence, and neither is an empty list.
    probe("qual_row_without_a_typed_schedule_fails_closed", AR,
          "        gates: &[GateId::G0, GateId::G5],\n        requirement: \"contracts, schemas",
          "gate posture is RowDeclared but the row declares none",
          "        gates: &[],\n        requirement: \"contracts, schemas")
    probe("dec_requiring_a_gate_without_one_fails_closed", DI,
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[GateId::G2]',
          "requires a gate but the row declares none",
          'id: "DEC-003", class: DecisionClass::Architecture, gates: &[]')
    # An ungated HistoricalReceipt is GateIndependent by an AUTHORED class claim.
    idx = {n["id"]: n for n in audit.guarantee_derive(root)[0]}
    if idx["DEC-001"]["gate_posture"] != "GateIndependent":
        fail("ungated_historical_receipt_is_gate_independent_by_class")
    if idx["DEC-002"]["lifetime"] != "HistoricalCoverageOnly":
        fail("retain_as_evidence_is_not_projected_as_permanent")
    if idx["DEC-005"]["gate_posture"] != "historical:G0":
        fail("requires_gate_is_a_floor_not_a_ceiling")
    # A retired decision keeps its authored GateIds as provenance, and provenance
    # is never an active schedule. No family is exempt from the invariant.
    for did, want in (("DEC-009", "historical:G0"), ("DEC-010", "historical:G5")):
        if idx[did]["gate_posture"] != want:
            fail(f"{did}_retains_its_gate_reference_as_historical_association")
    def hist_node(posture):
        return {"id": "D", "family": "DEC", "kind": "Decision",
                "lifetime": "HistoricalCoverageOnly", "owner": "o",
                "gate_posture": posture, "target": "", "witness": "NoFamilyWitness"}

    if audit.guarantee_lifetime_findings([hist_node("historical:G0")], {}, []):
        fail("historical_association_is_not_an_active_schedule")
    if not any("cannot be actively scheduled" in x
               for x in audit.guarantee_lifetime_findings([hist_node("G0")], {}, [])):
        fail("historical_lifetime_with_an_active_schedule_is_rejected_for_every_family")

    # 7. ARCH loses its declared G0 schedule.
    probe("arch_row_losing_g0_scheduling_is_rejected", GU,
          "gate_posture: GatePostureRule::FamilyConstant(GatePosture::Scheduled(&[GateId::G0])),",
          "gate posture is RowDeclared but the row declares none",
          "gate_posture: GatePostureRule::RowDeclared,")

    # 7b. Relations are typed constructors (5.5E2): the auditor refuses a bare
    # string, a mistagged family, and an undeclared constructor. Existence of
    # the referenced row is seedcheck's executed law, proven in the Tier 0
    # fixtures with a dangling reference.
    IN = "spec/invariants.rs"

    def rel_validator(tmp):
        return audit.guarantee_typed_relation_findings(audit.guarantee_seed_rows(tmp))

    probe("bare_string_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          "not a typed GuaranteeRef constructor",
          'derives_from: &["DEC-068"]', validator=rel_validator)
    probe("mistagged_relation_family_is_rejected", IN,
          'refines: &[GuaranteeRef::leg("LEG-080")]',
          "whose family owns the LEG- prefix",
          'refines: &[GuaranteeRef::leg("DEC-080")]', validator=rel_validator)
    probe("undeclared_relation_constructor_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-065")]',
          "undeclared relation constructor GuaranteeRef::arch",
          'derives_from: &[GuaranteeRef::arch("DEC-065")]', validator=rel_validator)

    # 7c. Witness citations are typed and resolve (5.5E2): a bare string, an
    # unknown contract id, and a mistagged family are each refused by the
    # auditor; the dangling-guarantee case is seedcheck's executed law and is
    # proven in the Tier 0 fixtures.
    def wit_validator(tmp):
        return audit.guarantee_typed_witness_findings(tmp, audit.guarantee_seed_rows(tmp))

    probe("bare_string_witness_is_rejected", IN,
          'witnesses: &[WitnessRef::dec("DEC-015")]',
          "not a canonical typed WitnessRef",
          'witnesses: &["DEC-015"]', validator=wit_validator)
    probe("unknown_contract_witness_is_rejected", IN,
          'WitnessRef::contract("BP-GAUNTLET-1")',
          "which no document declares",
          'WitnessRef::contract("BP-NOPE-1")', validator=wit_validator)
    probe("mistagged_witness_family_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]',
          "whose family owns the LEG- prefix",
          'witnesses: &[WitnessRef::leg("DEC-080"), '
          'WitnessRef::BootstrapTool(BootstrapToolId::Seedcheck)]', validator=wit_validator)

    # 8. Parity passes while provenance fails: mutate the SAME constant in both
    #    paths. This is the exact hole -- before the closure the graph regenerated,
    #    parity passed, and every gate stayed green.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        gu = tmp / GU
        gu.write_text(must_replace(gu.read_text(encoding="utf-8"),
                      "lifetime: LifetimeRule::FromDecisionDisposition,",
                      "lifetime: LifetimeRule::FamilyConstant(GuaranteeLifetime::Permanent),",
                      "common-mode lifetime"), encoding="utf-8")
        lives = {n["id"]: n["lifetime"] for n in audit.guarantee_derive(tmp)[0]}
        if lives.get("DEC-002") != "Permanent":
            fail("common_mode_mutation_probe_did_not_take_effect")
        # The typed source now says Permanent, so BOTH paths agree and parity is
        # silent. Only the typed owner changed -- which is the point: policy lives
        # in one place and a reviewer sees it in the diff, not in two scripts.
        proj = {n["id"]: n["lifetime"] for n in project.guarantee_nodes(tmp)}
        if proj.get("DEC-002") != lives.get("DEC-002"):
            fail("policy_change_is_visible_to_both_paths_from_one_owner")

    findings.extend(canonical_drift(before))
    return findings


# Multi-line Rust fragments the ISA probes mutate. Named once: repeating them
# inline invites a transcription drift that would make a probe miss its target
# and pass for the wrong reason.
def _isa_class_row(cls: str, algebra: str, family: str) -> str:
    return (
        f"        class: PakVmNodeClass::{cls},\n"
        f"        algebra: PakVmAlgebra::{algebra},\n"
        f"        work_formula: WorkFormulaFamily::{family},"
    )


WINDOWING_ROWS = _isa_class_row("Windowing", "QueryDataflow", "Rows")
WINDOWING_OUTPUTS = _isa_class_row("Windowing", "QueryDataflow", "EmittedOutputs")
QD_EVIDENCE = ("        evidence: PakVmRule::AlgebraConstant"
               "(EvidenceClass::ReferenceInterpreterModel),\n")
PUBLIC_LOWERING = ("        lowering: PakVmRule::AlgebraConstant"
                   "(CandidateLoweringPosture::PublicSemanticIdentity),")
INTERNAL_LOWERING = ("        lowering: PakVmRule::AlgebraConstant"
                     "(CandidateLoweringPosture::InternalLoweringIdentity),")


def test_pakvm_semantic_isa(audit, project) -> list[str]:
    """PakVM semantic ISA lineage, projection, and provenance (5.5D4c1).

    Every probe must reach and trip the refusal it names. A probe that goes red
    on an earlier unrelated rule proves the wrong wire was cut, so each mutation
    below is chosen to clear the checks that precede its target.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    ISA = "spec/pakvm_isa.rs"
    D07 = "docs/07_PAKVM_ISA.md"
    pf = audit.pakvm_isa_findings

    def probe(name, rel, old, needle, new="", validator=None):
        with isolated_tree() as tmp:
            path = tmp / rel
            text = path.read_text(encoding="utf-8")
            path.write_text(must_replace(text, old, new, name), encoding="utf-8")
            produced = (validator or pf)(tmp)
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    # The premise: the ISA admits cleanly today, through BOTH independent
    # derivations. Without this every probe below could be red for free.
    if pf(root):
        fail(f"pakvm_isa_contract_passes (got {pf(root)!r})")
    try:
        auditor = audit.pakvm_isa_views(root)
        generator = project.pakvm_specs(root)
    except Exception as exc:  # noqa: BLE001 - a crash here is itself the finding
        auditor, generator = [], []
        fail(f"both_derivations_admit_the_isa ({exc!r})")
    if len(auditor) != 36 or len(generator) != 36:
        fail(f"both_derivations_admit_36_nodes (auditor {len(auditor)}, generator {len(generator)})")
    # Independence is only worth something if the two agree on the ANSWER while
    # sharing no code to compute it.
    if [v["node"] for v in auditor] != [s["node"] for s in generator]:
        fail("independent_derivations_agree_on_the_node_inventory")
    for a, g in zip(auditor, generator):
        if (a["algebra"], a["class"], a["effect"], a["capability"], a["work_formula"],
                a["units"], a["evidence"], a["boundedness"]) != (
                g["algebra"], g["class"], g["effect"], g["capability"], g["work_formula"],
                g["units"], g["evidence"], g["boundedness"]):
            fail(f"independent_derivations_agree_on_{a['node']}")
            break

    # --- admission is complete or the node is not in the ISA -------------------
    probe("node_with_no_authored_class_does_not_admit", ISA,
          "            PakVmNodeId::Require => PakVmNodeClass::DecisionGuard,",
          "Require: class() states no authored family",
          "")
    probe("field_with_two_owners_does_not_admit", ISA,
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,\n        boundedness: BoundednessPosture::BoundedIteration,\n        effect: None,",
          "effect is declared by both its algebra and its class",
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,\n        boundedness: BoundednessPosture::BoundedIteration,\n        effect: Some(EffectPosture::ObservationalOnly),")
    probe("field_with_no_owner_does_not_admit", ISA,
          "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
          "effect is declared by neither its algebra nor its class",
          "        effect: PakVmRule::ClassDeclared,")

    # --- authoring surface: every node names a lawful producer (5.5E1) --------
    # Join is the one V1 node the frozen grammar cannot express; deleting its
    # authoring arm must refuse admission (rustc separately refuses the
    # non-exhaustive match under tier0-seedcheck), and smuggling 'join' back
    # into docs/13's member list must be caught as language re-admission.
    probe("a_node_with_no_lawful_producer_is_refused", ISA,
          "            PakVmNodeId::Join => NodeAuthoringSurface::CanonicalProgramImageOnly,\n",
          "Join: authoring_surface() names no lawful producer",
          "")
    probe("join_reentering_the_batql_member_list_is_rejected",
          "docs/13_BATQL_CONTRACT.md",
          "Source, subject, snapshot, time range, filter, fold, match, partition, "
          "group, aggregate, order, page, project, proof request.",
          "as a BatQL semantic-algebra member",
          "Source, subject, snapshot, time range, filter, fold, match, partition, "
          "group, aggregate, join, order, page, project, proof request.")

    # --- work-formula lineage, not coverage -----------------------------------
    # Coverage cannot catch these: the wrong family still claims its unit.
    probe("windowing_accounted_by_returned_output_does_not_admit", ISA,
          "        class: PakVmNodeClass::Windowing,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,",
          "accounts in Outputs, off the QueryDataflow plane",
          "        class: PakVmNodeClass::Windowing,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::EmittedOutputs,")
    probe("effect_node_with_a_query_cost_law_does_not_admit", ISA,
          "        class: PakVmNodeClass::DurableAppend,\n        algebra: PakVmAlgebra::Effect,\n        work_formula: WorkFormulaFamily::DeclaredEffects,",
          "accounts in Rows, off the Effect plane",
          "        class: PakVmNodeClass::DurableAppend,\n        algebra: PakVmAlgebra::Effect,\n        work_formula: WorkFormulaFamily::Rows,")
    probe("query_node_with_an_unrelated_real_unit_does_not_admit", ISA,
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::Rows,",
          "accounts in Artifacts, off the QueryDataflow plane",
          "        class: PakVmNodeClass::RowTransform,\n        algebra: PakVmAlgebra::QueryDataflow,\n        work_formula: WorkFormulaFamily::StagedArtifacts,")
    # A unit on two planes would let a family launder cost through whichever
    # plane it likes; a unit on none could never be accounted at all.
    probe("a_work_unit_on_two_planes_is_rejected", ISA,
          "            PakVmAlgebra::Effect => &[WorkUnit::Effects, WorkUnit::Artifacts, WorkUnit::Outputs],",
          "sits on more than one algebra plane",
          "            PakVmAlgebra::Effect => &[WorkUnit::Effects, WorkUnit::Artifacts, WorkUnit::Outputs, WorkUnit::Rows],")

    # --- the effect-posture biconditional --------------------------------------
    # These mutate the ISA and then REGENERATE the projection before auditing.
    # Auditing the stale projection would only prove drift detection works, which
    # is what this rule's absence already looked like: before it existed, an
    # effectful query algebra was reported as a drifted row and vanished entirely
    # the moment someone reran the generator.
    def law_probe(name, old, new, needle):
        with isolated_tree() as tmp:
            path = tmp / ISA
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            d07 = tmp / D07
            text = d07.read_text(encoding="utf-8")
            for marker, render in (("PAKVM-SEMANTIC-ISA", project.render_pakvm_isa),
                                   ("PAKVM-SIGNATURES", project.render_pakvm_signatures)):
                pat = project.block_pattern(marker)
                text = pat.sub(lambda m: m.group(1) + render(tmp) + m.group(3), text)
            d07.write_text(text, encoding="utf-8")
            produced = pf(tmp)
            if any("drifted" in f for f in produced):
                fail(f"{name} (regeneration left drift; the probe is testing the wrong rule)")
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    law_probe("an_effectful_query_algebra_is_rejected_after_regeneration",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),",
              "admits as Effectful outside the Effect algebra")
    law_probe("a_pure_effect_algebra_is_rejected_after_regeneration",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::Effectful),\n        capability",
              "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),\n        capability",
              "does not admit as Effectful, so it would pass the pure-query validator")

    # --- the semantic layer never acquires an encoding -------------------------
    probe("an_explicit_discriminant_is_rejected", ISA,
          "pub enum PakVmNodeId {\n    // Formula and decision (docs/07: 16)\n    Literal,",
          "assigns an explicit discriminant",
          "pub enum PakVmNodeId {\n    // Formula and decision (docs/07: 16)\n    Literal = 0,")
    probe("a_repr_on_the_semantic_node_is_rejected", ISA,
          "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum PakVmNodeId {",
          "fixes a semantic node's representation",
          "#[repr(u8)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum PakVmNodeId {")

    # --- an internal lowering identity is not a semantic node ------------------
    # The lowering line is byte-identical in all three algebra policies, so the
    # anchor carries the unique line above it. must_replace rewrites the first
    # match: an ambiguous anchor would mutate whichever policy happens to come
    # first and the probe would drift silently the day the order changes.
    probe("an_internal_lowering_identity_does_not_admit", ISA,
          QD_EVIDENCE + PUBLIC_LOWERING,
          "proposes the non-semantic lowering InternalLoweringIdentity",
          QD_EVIDENCE + INTERNAL_LOWERING)

    # --- provenance ------------------------------------------------------------
    probe("a_node_with_an_empty_authored_name_does_not_admit", ISA,
          '            PakVmNodeId::Literal => "literal",',
          "states an empty name",
          '            PakVmNodeId::Literal => "",')
    probe("a_node_with_no_source_origin_does_not_admit", ISA,
          '            PakVmAlgebra::Effect => "docs/07 Three instruction algebras: Effects",',
          "names no source origin",
          '            PakVmAlgebra::Effect => "",')

    # --- projection drift ------------------------------------------------------
    probe("a_projected_row_edited_by_hand_is_rejected", D07,
          "| literal | FormulaDecision | ScalarComputation | Pure |",
          "drifted from its admitted spec",
          "| literal | FormulaDecision | ScalarComputation | Effectful |")
    probe("a_projected_row_deleted_by_hand_is_rejected", D07,
          "| binding | FormulaDecision | ScalarComputation | Pure | None | ConstantWork | ConstantInstruction | Instructions | ReferenceInterpreterModel |\n",
          "projects 35 PakVM nodes, 36 admit",
          "")
    probe("a_signature_deleted_by_hand_is_rejected", D07,
          "    operands  none; a constant-pool reference",
          "signature projection omits the operand sorts",
          "    operands  ")

    # --- a test injection cannot enter the projection or gain provenance -------
    with isolated_tree() as tmp:
        d07 = tmp / D07
        d07.write_text(d07.read_text(encoding="utf-8").replace(
            "| literal | FormulaDecision |",
            "| injected_micro_op | FormulaDecision | ScalarComputation | Pure | None | "
            "ConstantWork | ConstantInstruction | Instructions | ReferenceInterpreterModel |\n"
            "| literal | FormulaDecision |", 1), encoding="utf-8")
        if not any("PakVM nodes" in f or "drifted" in f for f in pf(tmp)):
            fail("an_injected_projection_row_is_rejected")

    # --- the generator refuses to project an unadmitted node -------------------
    # A projector that completed a missing field would reintroduce the exact
    # defect this pass exists to remove.
    with isolated_tree() as tmp:
        path = tmp / ISA
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            "        effect: PakVmRule::AlgebraConstant(EffectPosture::ObservationalOnly),",
            "        effect: PakVmRule::ClassDeclared,",
            "generator refusal"), encoding="utf-8")
        try:
            project.pakvm_specs(tmp)
            fail("the_generator_refuses_to_project_an_unadmitted_node")
        except project.Unadmitted:
            pass

    # --- detector neutering ----------------------------------------------------
    # Silencing the plane rule must not leave a green suite. Uses the canonical
    # neutered_validator helper: the real audit.py is never opened for writing, so
    # a probe that dies mid-mutation cannot leave a disabled rule behind.
    with isolated_tree() as tmp:
        p2 = tmp / ISA
        p2.write_text(must_replace(
            p2.read_text(encoding="utf-8"),
            WINDOWING_ROWS, WINDOWING_OUTPUTS,
            "cross-plane mutation"), encoding="utf-8")
        # the live rule sees it...
        if not any("off the QueryDataflow plane" in f for f in pf(tmp)):
            fail("the_plane_rule_bites_before_it_is_neutered")
        # ...and the neutered rule does not, which is what makes it a real rule.
        with neutered_validator("audit", "        stray = [u for u in units if u not in plane]",
                                "        stray = []") as neutered:
            if any("off the QueryDataflow plane" in f
                   for f in neutered.pakvm_isa_findings(tmp)):
                fail("neutering_the_plane_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings


def test_syncbat_firewall(audit, project) -> list[str]:
    """SyncBat authority firewall: ownership, crossing legality, projection (5.5D4c2).

    Every probe runs the full sequence: mutate the authoritative source, prove
    the bytes moved, REGENERATE the projection, refuse a drift alibi, then
    require the exact finding the rule names. Auditing a stale projection would
    only prove drift detection works -- and drift detection working is precisely
    what the PakVM effect biconditional looked like right before it turned out to
    be enforcing nothing at all.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    FW = "spec/syncbat_firewall.rs"
    D08 = "docs/08_SYNCBAT_RUNTIME.md"
    ff = audit.syncbat_firewall_findings

    OWNS_COMPOSITION = (
        "            SyncBatAuthority::CompositionAndInstanceIdentity => SyncBatPlane::World,")
    STEALS_COMPOSITION = (
        "            SyncBatAuthority::CompositionAndInstanceIdentity => SyncBatPlane::Port,")
    RETRY_ARM = (
        "            | SyncBatAuthority::RetryRestartAuthority => SyncBatPlane::Runtime,")
    OWNERLESS_ARM = "            => SyncBatPlane::Runtime,"
    EFFECT_ROUTE = (
        "    SyncBatCrossing {\n"
        "        from: SyncBatPlane::PakVm,\n"
        "        to: SyncBatPlane::Bvisor,\n"
        "        carries: SyncBatAuthority::TypedEffectRequest,\n"
        "        posture: CrossingPosture::Required,\n"
        '        law: "an effectful node emits a typed request for physical admission",\n'
        "    },\n")

    def regenerate(tmp) -> bool:
        """Make docs/08 current with the mutated spec. False when the generator
        refuses, which is itself lawful: a projector that completed a broken fact
        would be the defect. The caller then knows drift is expected."""
        d = tmp / D08
        text = d.read_text(encoding="utf-8")
        try:
            for marker, render in (
                    ("SYNCBAT-PLANE-OWNERSHIP", project.render_syncbat_plane_ownership),
                    ("SYNCBAT-AUTHORITIES", project.render_syncbat_authorities),
                    ("SYNCBAT-CROSSINGS", project.render_syncbat_crossings)):
                body = render(tmp)
                text = project.block_pattern(marker).sub(
                    lambda m: m.group(1) + body + m.group(3), text)
        except project.Unadmitted:
            return False
        d.write_text(text, encoding="utf-8")
        return True

    def probe(name, rel, old, needle, new="", regen=True):
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            regenerated = regenerate(tmp) if regen else False
            produced = ff(tmp)
            if regenerated and any("drifted" in f for f in produced):
                fail(f"{name} (regeneration left drift; the probe is testing the wrong rule)")
            if not any(needle in f for f in produced):
                fail(f"{name} (wanted {needle!r}, got {produced!r})")

    # The premise. Without it every probe below could be red for free.
    if ff(root):
        fail(f"syncbat_firewall_contract_passes (got {ff(root)!r})")
    try:
        auditor = audit.syncbat_firewall_views(root)
        generator = project.syncbat_firewall_facts(root)
    except Exception as exc:  # noqa: BLE001 - a crash here is itself the finding
        auditor, generator = {}, {}
        fail(f"both_derivations_admit_the_firewall ({exc!r})")
    if auditor and generator:
        # Independence is only worth something if the two agree on the ANSWER
        # while sharing no code to compute it.
        if auditor["listed_planes"] != generator["order"]:
            fail("independent_derivations_agree_on_the_plane_inventory")
        if auditor["listed"] != [a["authority"] for a in generator["authorities"]]:
            fail("independent_derivations_agree_on_the_authority_inventory")
        for a in generator["authorities"]:
            if auditor["owners"].get(a["authority"]) != a["owner"]:
                fail(f"independent_derivations_agree_on_the_owner_of_{a['authority']}")
                break
        if ([(c["from"], c["to"], c["carries"]) for c in auditor["crossings"]]
                != [(c["from"], c["to"], c["carries"]) for c in generator["crossings"]]):
            fail("independent_derivations_agree_on_the_crossing_whitelist")
        # A rule quantified over an empty set is vacuously true forever. No count
        # is asserted here; the sets must merely be inhabited.
        if not auditor["crossings"] or not auditor["semantic"] or not auditor["planes"]:
            fail("the_firewall_is_inhabited")

    # --- one owner per authority, never two and never none --------------------
    probe("an_authority_with_two_owners_does_not_admit", FW,
          "            SyncBatAuthority::CapabilityAdmission\n"
          "            | SyncBatAuthority::PhysicalAdmission",
          "owner() answers twice for LogicalLegality",
          "            SyncBatAuthority::LogicalLegality\n"
          "            | SyncBatAuthority::CapabilityAdmission\n"
          "            | SyncBatAuthority::PhysicalAdmission")
    probe("an_ownerless_authority_does_not_admit", FW,
          RETRY_ARM,
          "authority RetryRestartAuthority names no owning plane",
          OWNERLESS_ARM)
    # The reading rustc cannot perform. An authority left out of the listing
    # compiles perfectly and vanishes from every law that walks the listing:
    # seedcheck goes on proving things about the authorities it was handed and
    # never mentions the one it was not.
    probe("an_authority_missing_from_the_listing_is_found", FW,
          "    SyncBatAuthority::RetryRestartAuthority,\n"
          "    SyncBatAuthority::SemanticNodeInterpretation,",
          "is authored but SYNCBAT_AUTHORITIES omits it",
          "    SyncBatAuthority::SemanticNodeInterpretation,")

    # --- crossing legality, surviving regeneration ----------------------------
    # docs/08's own forbidden example: "Bvisor minting semantic restart
    # authorization". The projection is made perfectly current first; the finding
    # must still be there, because the rule reads the spec and not the document.
    probe("a_forbidden_crossing_is_rejected_after_regeneration", FW,
          "pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[\n",
          "lets Bvisor exercise RetryRestartAuthority, which Runtime owns",
          "pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[\n"
          "    SyncBatCrossing {\n"
          "        from: SyncBatPlane::Bvisor,\n"
          "        to: SyncBatPlane::Runtime,\n"
          "        carries: SyncBatAuthority::RetryRestartAuthority,\n"
          "        posture: CrossingPosture::Required,\n"
          '        law: "bvisor mints semantic restart authorization",\n'
          "    },\n")
    # Deleting a lawful crossing is the quiet one. Every other rule iterates the
    # whitelist, so a removal does not fail them -- it gives them less to say.
    # Only the carrier rule notices that an authority which must name a PakVM
    # origin now has nowhere to carry it.
    probe("a_deleted_semantic_route_is_rejected_after_regeneration", FW,
          EFFECT_ROUTE,
          "TypedEffectRequest must name a PakVM origin but no required crossing carries it off PakVm",
          "")
    probe("a_noncanonical_plane_identity_is_rejected", FW,
          "        from: SyncBatPlane::World,\n        to: SyncBatPlane::Runtime,",
          "names 'Kernel', which is no SyncBat plane",
          "        from: SyncBatPlane::Kernel,\n        to: SyncBatPlane::Runtime,")

    # --- a stale projection is not an alibi, and a fresh one is not absolution -
    # The same mutation, audited both ways. Before regeneration the semantic
    # finding must already be present rather than hiding behind a drift report;
    # after regeneration it must still be present rather than vanishing with it.
    for name, regen in (("a_stale_projection_does_not_mask_an_ownership_transfer", False),
                        ("a_regenerated_projection_does_not_launder_an_ownership_transfer", True)):
        probe(name, FW, OWNS_COMPOSITION,
              "lets World exercise CompositionAndInstanceIdentity, which Port owns",
              STEALS_COMPOSITION, regen=regen)

    # --- a hand-authored copy is a second answer ------------------------------
    probe("a_hand_authored_inventory_competing_with_the_projection_is_found", D08,
          "Forbidden examples:",
          "is also hand-authored outside the generated block",
          "```text\nruntime owns logical legality\n```\n\nForbidden examples:")

    # --- the generator refuses; it never completes -----------------------------
    # A projector that could supply a missing owner would be a second authority
    # wearing a serializer's name.
    with isolated_tree() as tmp:
        path = tmp / FW
        path.write_text(must_replace(path.read_text(encoding="utf-8"), RETRY_ARM,
                                     OWNERLESS_ARM, "generator refusal"), encoding="utf-8")
        try:
            project.syncbat_firewall_facts(tmp)
            fail("the_generator_refuses_to_project_an_ownerless_authority")
        except project.Unadmitted:
            pass

    # --- detector neutering ----------------------------------------------------
    # Silencing the ownership rule must not leave a green suite. Without this the
    # probes above prove only that SOMETHING returned a string.
    with isolated_tree() as tmp:
        p2 = tmp / FW
        p2.write_text(must_replace(p2.read_text(encoding="utf-8"), OWNS_COMPOSITION,
                                   STEALS_COMPOSITION, "ownership mutation"), encoding="utf-8")
        if not any("which Port owns" in f for f in ff(tmp)):
            fail("the_ownership_rule_bites_before_it_is_neutered")
        with neutered_validator("audit", "        if owner and owner != frm:",
                                "        if False:") as neutered:
            if any("which Port owns" in f for f in neutered.syncbat_firewall_findings(tmp)):
                fail("neutering_the_ownership_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings


def test_reconciliation(audit, project) -> list[str]:
    """DEC-075 single-writer dual-axis reconciliation: the auditor's half
    (5.5E1d). The four-signal retry fence itself is executed Rust inside
    seedcheck; here the composition's structure, carriers, and projections are
    attacked through the auditor.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    if audit.recon_findings(root):
        fail(f"reconciliation_contract_passes (got {audit.recon_findings(root)!r})")
    RS = "spec/reconciliation.rs"
    # A role bound twice is one name owning two authorities -- the HlcPoint
    # disease this decision exists to prevent.
    with isolated_tree() as tmp:
        p = tmp / RS
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "        role: ReconciliationRole::PhysicalIdentity,",
            "        role: ReconciliationRole::LogicalIdentity,",
            "rebind a role"), encoding="utf-8")
        got = audit.recon_findings(tmp)
        if not any("five distinct roles" in f for f in got):
            fail(f"a_rebound_reconciliation_role_is_refused (got {got!r})")
    # A hand-edited projection is drift, not authority.
    with isolated_tree() as tmp:
        p = tmp / "docs/02_SYSTEM_MODEL.md"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "| PhysicalIdentity | AttemptId |",
            "| PhysicalIdentity | AttemptId, Hlc |",
            "drift the coordinates block"), encoding="utf-8")
        if not any("drifted" in f for f in audit.recon_findings(tmp)):
            fail("a_drifted_reconciliation_block_is_caught")
    # A chronology carrier the time contract does not own would be a second
    # time authority minted by the composition.
    with isolated_tree() as tmp:
        p = tmp / RS
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            '"Hlc", "ObservedWallTime"', '"Hlc", "WallStamp"',
            "invent a carrier"), encoding="utf-8")
        got = audit.recon_findings(tmp)
        if not any("not owned by docs/16" in f for f in got):
            fail(f"an_invented_chronology_carrier_is_refused (got {got!r})")
    # DEC-058's seal inventory (5.5E1e): one owner, projected, never restated.
    if audit.release_seal_findings(root):
        fail(f"release_seal_contract_passes (got {audit.release_seal_findings(root)!r})")
    with isolated_tree() as tmp:
        p = tmp / "spec/architecture.rs"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "    ReleaseSealField::KernelQualificationSet,\n", "",
            "drop the kernel set"), encoding="utf-8")
        got = audit.release_seal_findings(tmp)
        if not any("never disappears" in f for f in got):
            fail(f"dropping_the_kernel_set_from_the_seal_is_refused (got {got!r})")
    with isolated_tree() as tmp:
        p = tmp / "docs/36_PUBLIC_API_CI_AND_RELEASE.md"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"), "\nSbom\n", "\n",
            "hand-trim the projection"), encoding="utf-8")
        if not any("drifted" in f for f in audit.release_seal_findings(tmp)):
            fail("a_trimmed_seal_projection_is_caught")
    with isolated_tree() as tmp:
        p = tmp / "spec/dispositions.rs"
        p.write_text(must_replace(
            p.read_text(encoding="utf-8"),
            "binds every field of the typed ReleaseSealField inventory",
            "binds source tree toolchain and SBOM",
            "restate the list"), encoding="utf-8")
        got = audit.release_seal_findings(tmp)
        if not any("instead of naming the typed" in f for f in got):
            fail(f"a_decision_restating_the_seal_list_is_refused (got {got!r})")
    findings.extend(canonical_drift(before))
    return findings


def test_syncbat_requiredness(audit, project) -> list[str]:
    """SyncBat crossing requiredness: liveness, not just safety (5.5D4c2f).

    Safety proves no unauthorized door exists. Requiredness proves the authorized
    doors have not been bricked over. A stricter firewall that deletes a required
    execution path preserves safety by destroying liveness, and that is still a
    specification failure. Every probe mutates the authoritative source, proves
    the bytes moved, REGENERATES the projection, refuses a drift alibi, and
    requires the exact semantic finding -- deletion of a required route must
    survive a perfectly current document.
    """
    findings: list[str] = []
    root = HERE.parent
    before = canonical_commitments()

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    FW = "spec/syncbat_firewall.rs"
    D08 = "docs/08_SYNCBAT_RUNTIME.md"
    ff = audit.syncbat_firewall_findings

    import re as _re
    fwsrc = (root / FW).read_text(encoding="utf-8")
    const = fwsrc[fwsrc.index("pub const SYNCBAT_LEGAL_CROSSINGS"):]
    const = const[: const.index("\n];") + 3]
    rows = _re.findall(r"    SyncBatCrossing \{.*?\n    \},\n", const, _re.S)
    if len(rows) != 7:
        fail(f"the_firewall_declares_the_expected_crossing_rows (found {len(rows)})")
    crossings = []
    for b in rows:
        crossings.append({
            "block": b,
            "from": _re.search(r"from: SyncBatPlane::(\w+)", b).group(1),
            "to": _re.search(r"to: SyncBatPlane::(\w+)", b).group(1),
            "carries": _re.search(r"carries: SyncBatAuthority::(\w+)", b).group(1),
        })

    def regenerate(tmp) -> bool:
        d = tmp / D08
        text = d.read_text(encoding="utf-8")
        try:
            for marker, render in (
                    ("SYNCBAT-PLANE-OWNERSHIP", project.render_syncbat_plane_ownership),
                    ("SYNCBAT-AUTHORITIES", project.render_syncbat_authorities),
                    ("SYNCBAT-CROSSINGS", project.render_syncbat_crossings)):
                body = render(tmp)
                text = project.block_pattern(marker).sub(
                    lambda m: m.group(1) + body + m.group(3), text)
        except project.Unadmitted:
            return False
        d.write_text(text, encoding="utf-8")
        return True

    def semantic_probe(name, new_fwsrc, needle):
        """Apply a whole-file firewall mutation, regenerate, require a NON-drift
        finding matching needle."""
        with isolated_tree() as tmp:
            path = tmp / FW
            cur = path.read_text(encoding="utf-8")
            if new_fwsrc == cur:
                fail(f"{name} (mutation changed no bytes)")
                return
            path.write_text(new_fwsrc, encoding="utf-8")
            regenerate(tmp)
            produced = ff(tmp)
            semantic = [f for f in produced if "drifted" not in f]
            if not any(needle in f for f in semantic):
                fail(f"{name} (wanted non-drift {needle!r}, got {produced!r})")

    # The premise: today the firewall admits and every route is Required.
    if ff(root):
        fail(f"syncbat_requiredness_contract_passes (got {ff(root)!r})")

    # --- 1 & 2: deleting each Required crossing is a semantic finding, not drift
    for c in crossings:
        semantic_probe(
            f"deleting_the_{c['carries']}_route_is_a_semantic_finding",
            fwsrc.replace(c["block"], "", 1),
            "required")  # every deletion message names a missing/downgraded required route

    # --- 3: Required -> Optional is caught when no alternate route exists -------
    for c in crossings:
        downgraded = c["block"].replace(
            "posture: CrossingPosture::Required,", "posture: CrossingPosture::Optional,")
        semantic_probe(
            f"downgrading_the_{c['carries']}_route_to_optional_is_caught",
            fwsrc.replace(c["block"], downgraded, 1),
            "required")

    # --- 5: swapping a crossing's direction is refused -------------------------
    # World -> Runtime becomes Runtime -> World; World no longer owns what it now
    # purports to send, and the ownership rule refuses it.
    swap = crossings[0]
    swapped = (swap["block"]
               .replace(f"from: SyncBatPlane::{swap['from']}", "from: SyncBatPlane::__TMP__")
               .replace(f"to: SyncBatPlane::{swap['to']}", f"to: SyncBatPlane::{swap['from']}")
               .replace("from: SyncBatPlane::__TMP__", f"from: SyncBatPlane::{swap['to']}"))
    semantic_probe("swapping_a_crossing_direction_is_refused",
                   fwsrc.replace(swap["block"], swapped, 1),
                   f"lets {swap['to']} exercise {swap['carries']}")

    # --- 6: keeping the endpoints but changing the carried authority is refused -
    c1 = crossings[1]  # Runtime -> PakVm, SemanticAuthorization
    reauth = c1["block"].replace(
        "carries: SyncBatAuthority::SemanticAuthorization,",
        "carries: SyncBatAuthority::AttemptEvidence,")
    semantic_probe("changing_the_carried_authority_is_refused",
                   fwsrc.replace(c1["block"], reauth, 1),
                   "lets Runtime exercise AttemptEvidence")

    # --- 7: a required route cannot be laundered by a similar-looking receipt ---
    # Delete the AttemptEvidence return, then add a Bvisor -> Runtime crossing
    # carrying SemanticReceipt -- a superficially similar "receipt". Bvisor does
    # not own SemanticReceipt, so the substitute is refused AND the connectivity
    # graph counts only legal edges, so it cannot reconnect Bvisor to the root.
    evidence = next(c for c in crossings if c["carries"] == "AttemptEvidence")
    substitute = (
        "    SyncBatCrossing {\n"
        "        from: SyncBatPlane::Bvisor,\n"
        "        to: SyncBatPlane::Runtime,\n"
        "        carries: SyncBatAuthority::SemanticReceipt,\n"
        "        posture: CrossingPosture::Required,\n"
        '        law: "a receipt that looks like evidence",\n'
        "    },\n")
    laundered = fwsrc.replace(evidence["block"], substitute, 1)
    with isolated_tree() as tmp:
        (tmp / FW).write_text(laundered, encoding="utf-8")
        regenerate(tmp)
        produced = ff(tmp)
        if not any("lets Bvisor exercise SemanticReceipt" in f for f in produced):
            fail(f"a_similar_receipt_cannot_launder_a_required_route:substitute_refused "
                 f"(got {produced!r})")
        if not any("Bvisor cannot reach the logical root" in f for f in produced):
            fail(f"a_similar_receipt_cannot_launder_a_required_route:route_still_missing "
                 f"(got {produced!r})")

    # --- 8: neutering the requiredness detector must not leave a green suite ----
    deleted = fwsrc.replace(
        next(c for c in crossings if c["carries"] == "PhysicalAttempt")["block"], "", 1)
    with isolated_tree() as tmp:
        (tmp / FW).write_text(deleted, encoding="utf-8")
        regenerate(tmp)
        if not any("cannot reach plane Port" in f for f in ff(tmp)):
            fail("the_requiredness_rule_bites_before_it_is_neutered")
        with neutered_validator(
                "audit",
                "            if plane != source and not reaches(logical_root, plane):",
                "            if False:") as neutered:
            if any("cannot reach plane Port" in f
                   for f in neutered.syncbat_firewall_findings(tmp)):
                fail("neutering_the_requiredness_rule_silences_it")

    findings.extend(canonical_drift(before))
    return findings


def test_identity_catalogs(audit) -> list[str]:
    """Named hostile fixtures for the identity catalogs (5.5E3d). Four axes,
    one term one axis, owners resolve, chronology stays out, existing typed
    owners are never duplicated, docs/16 projects ordered entries AND owners,
    and the permanent corpus denominator refuses any identity-shaped term
    that resolves through none of the five classification paths."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.identity_catalog_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.identity_catalog_findings(root):
        fail("identity_catalogs_pass_on_the_real_seed")

    ID = "spec/identities.rs"
    DOC16 = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
    # EventId was the sweep's headline restoration; deleting it again is
    # refused by the corpus denominator itself: the term is still authored
    # in docs/04, docs/05, and the companion, so it must classify.
    probe("event_id_cannot_disappear_from_identity_catalog",
          [(ID, "        IdentityKind::Event,\n", ""),
           (ID, '            IdentityKind::Event => entry!("EventId", "BP-STORAGE-TILES-1"),\n',
            "")],
          "identity-shaped term EventId appears in the authoritative corpus "
          "and resolves through none of the five classification paths")
    # A binding is a commitment to content, never the object's identity:
    # cross-axis uniqueness means the misclassification has no spelling.
    probe("content_digest_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("CorrelationId", "BP-STORAGE-TILES-1")',
            'entry!("ContentDigest", "BP-STORAGE-TILES-1")')],
          "ContentDigest answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    probe("commitment_cannot_be_classified_as_object_identity",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("Commitment", "BP-IDENTITY-TIME-NAV-1")')],
          "Commitment answers two questions: it appears in both "
          "IdentityKind and BindingKind")
    # Chronology and navigation vocabulary is excluded by executed law.
    probe("hlc_cannot_enter_identity_catalog",
          [(ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "Hlc is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("commit_point_cannot_enter_identity_catalog",
          [(ID, 'entry!("UnitId", "BP-NUMERIC-1")',
            'entry!("CommitPoint", "BP-IDENTITY-TIME-NAV-1")')],
          "CommitPoint is chronology/navigation vocabulary and may not enter "
          "the IdentityKind catalog")
    probe("identity_kind_missing_from_all_is_rejected",
          [(ID, "        IdentityKind::Tile,\n", "")],
          "IdentityKind::Tile declares an entry but is omitted from IdentityKind::ALL")
    probe("catalog_entry_with_unknown_owner_is_rejected",
          [(ID, 'entry!("TileId", "BP-STORAGE-TILES-1")',
            'entry!("TileId", "BP-NONEXISTENT-1")')],
          "IdentityKind entry TileId names owner BP-NONEXISTENT-1, "
          "which no declared contract owns")
    probe("docs_identity_block_drift_is_rejected",
          [(DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "docs/16 IDENTITY-CATALOG row 7 states EventId BP-SYNCBAT-1; the "
          "typed catalog states EventId BP-STORAGE-TILES-1 at that position")
    # Existing typed owners stay owners: a duplicate variant is a wrapper
    # passport, refused by spelling.
    probe("existing_package_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry PackageId duplicates a spelling that already "
          "has a typed spec owner")
    probe("existing_proof_row_id_owner_cannot_be_duplicated",
          [(ID, 'entry!("KeyBackendId", "BP-CRYPTO-SECRET-1")',
            'entry!("ProofRowId", "BP-CRYPTO-SECRET-1")')],
          "IdentityKind entry ProofRowId duplicates a spelling that already "
          "has a typed spec owner")
    # The standing denominator law: a new identity-shaped term authored
    # anywhere in the corpus turns red until classified through exactly one
    # of the five paths.
    probe("new_identity_shaped_term_requires_classification",
          [(DOC16, "BatPak never invents its own entropy primitive.",
            "BatPak never invents its own entropy primitive. A future "
            "WidgetTrackerId is under discussion.")],
          "identity-shaped term WidgetTrackerId appears in the authoritative "
          "corpus and resolves through none of the five classification paths")
    probe("noncataloged_term_cannot_also_be_admitted",
          [(ID, 'entry!("KeyId", "BP-CRYPTO-SECRET-1")',
            'entry!("NavigationId", "BP-IDENTITY-TIME-NAV-1")')],
          "NavigationId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("owned_elsewhere_term_requires_a_live_contract_owner",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-GHOST-1"))')],
          "residue term GateId is owned elsewhere by BP-GHOST-1, "
          "which no declared contract owns")
    # CompressionId is a passport application stapled to DEC-063's checklist.
    # Sneaking it into a catalog while the residue row stands is the two-path
    # refusal; entry happens only by amending DEC-063 AND moving the term.
    probe("compression_id_cannot_enter_without_dec063_amendment",
          [(ID, 'entry!("FloatFormatId", "BP-NUMERIC-1")',
            'entry!("CompressionId", "BP-NUMERIC-1")')],
          "CompressionId resolves through two paths: the residue table and "
          "the IdentityKind catalog")
    probe("not_yet_admitted_term_requires_a_live_decision",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-963"))')],
          "residue term CompressionId is not yet admitted by DEC-963, "
          "which no declared decision owns")
    # A decision retained only as historical coverage has no forward policy
    # authority: the archive does not issue future visas. DEC-005 is the
    # standing Supersede row.
    probe("historical_decision_cannot_own_not_yet_admitted_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-005"))')],
          "residue term CompressionId is not yet admitted by DEC-005, which "
          "is retained only as historical coverage and cannot own a future "
          "admission barrier")
    # 5.5E3d1: TRUE set equality — a catalog entry with only its own
    # generated projection is an answer sheet, not an adopter. The typed
    # entry and the docs/16 row alone must refuse; add a real authored
    # sentence under the live owner contract and the same growth is lawful.
    _SESSION_GROWTH = [
        (ID, "        IdentityKind::Tile,\n",
         "        IdentityKind::Tile,\n        IdentityKind::Session,\n"),
        (ID, '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),',
         '            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),\n'
         '            IdentityKind::Session => entry!("SessionId", "BP-STORAGE-TILES-1"),'),
        (DOC16, f"{'TileId':<30} BP-STORAGE-TILES-1\n",
         f"{'TileId':<30} BP-STORAGE-TILES-1\n"
         f"{'SessionId':<30} BP-STORAGE-TILES-1\n"),
    ]
    probe("catalog_entry_without_authored_adopter_is_rejected",
          list(_SESSION_GROWTH),
          "catalog entry SessionId appears in no authored corpus document; "
          "an entry without an authored adopter is a brochure passport")
    # Self-neutering guards: deleting the exclusion or wrapper-guard row
    # before smuggling the term in is refused by independent parity with
    # the authored docs/16 fences and the real spec declarations.
    probe("chronology_exclusion_cannot_be_removed_then_cataloged",
          [(ID, '    "Hlc",\n', ''),
           (ID, 'entry!("RewrapId", "BP-CRYPTO-SECRET-1")',
            'entry!("Hlc", "BP-IDENTITY-TIME-NAV-1")')],
          "EXCLUDED_CHRONOLOGY_AND_NAVIGATION omits Hlc, which docs/16's "
          "navigation and time-and-order fences declare")
    probe("existing_owner_guard_cannot_be_removed_then_readmitted",
          [(ID, '"PackageId", ', ''),
           (ID, 'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
            'entry!("PackageId", "BP-CRYPTO-SECRET-1")')],
          "EXISTING_TYPED_OWNER_SPELLINGS omits PackageId, which the spec "
          "declares as an existing typed owner")
    # One term, one path — and one ROW on that path.
    probe("duplicate_residue_term_is_rejected",
          [(ID, '        term: "GateId",\n',
            '        term: "GateId",\n        disposition: '
            'IdentityTermDisposition::OwnedElsewhere(ContractId("BP-GATES-1")),\n'
            '    },\n    NonCatalogedIdentityTerm {\n        term: "GateId",\n')],
          "residue term GateId is declared twice; one term, one path also "
          "means one row on that path")
    # Permanent is not enough: Kill is a dead passport and Keep is admitted
    # retained policy — neither is a pending application.
    probe("killed_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Kill disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    probe("kept_decision_cannot_own_not_yet_admitted_term",
          [("spec/dispositions.rs",
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
            'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
            'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Keep,')],
          "residue term CompressionId is not yet admitted by DEC-063, whose "
          "Keep disposition is not a standing future-entry policy; only Lock "
          "and Defer own pending applications")
    # Provenance is verified independently of the renderer: a BEGIN marker
    # naming any other source is not a generated identity block at all.
    probe("identity_projection_source_marker_drift_is_rejected",
          [(DOC16,
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identities.rs "
            "by bootstrap/project.py; do not edit -->",
            "<!-- IDENTITY-CATALOG:BEGIN generated from spec/identity_soup.rs "
            "by bootstrap/project.py; do not edit -->")],
          "docs/16 carries no generated IDENTITY-CATALOG block naming "
          "spec/identities.rs as source")
    # Owner coherence (5.5E3d2): the owner must AUTHOR the term. Each owner
    # mutation moves the generated docs/16 row in lockstep, so the hostile
    # fails for owner incoherence, never for a stale projection.
    probe("catalog_owner_must_author_the_term",
          [(ID, 'entry!("EventId", "BP-STORAGE-TILES-1")',
            'entry!("EventId", "BP-SYNCBAT-1")'),
           (DOC16, f"{'EventId':<30} BP-STORAGE-TILES-1",
            f"{'EventId':<30} BP-SYNCBAT-1")],
          "IdentityKind entry EventId names owner BP-SYNCBAT-1, whose "
          "authoritative document does not author the term")
    probe("owned_elsewhere_owner_must_author_the_term",
          [(ID, 'OwnedElsewhere(ContractId("BP-GATES-1"))',
            'OwnedElsewhere(ContractId("BP-NETBAT-1"))'),
           (DOC16, f"{'GateId':<30} {'owned elsewhere':<22} BP-GATES-1",
            f"{'GateId':<30} {'owned elsewhere':<22} BP-NETBAT-1")],
          "residue term GateId is owned elsewhere by BP-NETBAT-1, "
          "which does not author the term")
    # DEC-058 is live and Lock, so it passes existence and the disposition
    # predicate — but it is about the release seal and says nothing about
    # CompressionId. A live but unrelated authority is refused by name.
    probe("not_yet_admitted_decision_must_name_the_term",
          [(ID, 'NotYetAdmittedBy(DecisionId("DEC-063"))',
            'NotYetAdmittedBy(DecisionId("DEC-058"))'),
           (DOC16, f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-063",
            f"{'CompressionId':<30} {'not yet admitted by':<22} DEC-058")],
          "residue term CompressionId is not yet admitted by DEC-058, whose "
          "forward-policy fields do not name the term")
    # GREEN fixture: a term may lawfully appear in many documents while its
    # canonical owner document also authors it — breadth of adoption is not
    # an ownership dispute.
    tmp = gate_sandbox([
        ("docs/14_RECEIPTS_AND_EXPLANATION.md",
         "It is structured evidence, not a debug log.",
         "It is structured evidence, not a debug log. A replay receipt names "
         "the EventId range it covered.")])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"term_in_many_documents_with_authoring_owner_is_lawful "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    # GREEN fixture: the counts are observed output, never acceptance
    # thresholds. The SAME growth as the brochure hostile above, plus a real
    # authored adopter sentence under the live owner contract (docs/05,
    # BP-STORAGE-TILES-1), is lawful with no frozen number touched.
    tmp = gate_sandbox(_SESSION_GROWTH + [
        ("docs/05_STORAGE_FBAT_AND_TILES.md",
         "EventFrame identity\n",
         "EventFrame identity\nSessionId scoping\n"),
    ])
    try:
        got = audit.identity_catalog_findings(tmp)
        if got:
            fail(f"catalog_counts_are_derived_not_frozen (lawful growth "
                 f"was refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_commands(audit) -> list[str]:
    """Named hostile fixtures for the three command namespaces (5.5E3e). The
    variant is the identity; namespace identity and invoked semantic
    authority are separate axes; composition never transfers ownership."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.command_catalog_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    # GREEN: the real seed passes whole — which simultaneously proves the two
    # ruled lawful shapes: the same "inspect" token lives in ProductCommand
    # AND TestPakCommand (uniqueness is per namespace), and the composite
    # commands delegate to several contracts without transferring ownership.
    if audit.command_catalog_findings(root):
        fail("commands_pass_on_the_real_seed")

    CM = "spec/commands.rs"
    D26 = "docs/26_COMMAND_PLANE.md"
    CP = "companion/BATQL_LANGUAGE.md"
    probe("product_command_missing_from_inventory_is_rejected",
          [(CM, "        ProductCommand::Repl,\n    ];", "    ];")],
          "ProductCommand::Repl is omitted from ProductCommand::ALL")
    probe("testpak_command_missing_from_inventory_is_rejected",
          [(CM, "        TestPakCommand::Seal,\n    ];", "    ];")],
          "TestPakCommand::Seal is omitted from TestPakCommand::ALL")
    probe("batql_mode_missing_from_inventory_is_rejected",
          [(CM, "&[BatQlSourceMode::Ask, BatQlSourceMode::Do]",
            "&[BatQlSourceMode::Ask]")],
          "BatQlSourceMode::Do is omitted from BatQlSourceMode::ALL")
    probe("duplicate_token_within_product_namespace_is_rejected",
          [(CM, 'ProductCommand::Repl => "repl",', 'ProductCommand::Repl => "run",')],
          "ProductCommand token run is claimed twice; tokens are unique "
          "within their namespace")
    probe("do_cannot_enter_product_commands",
          [(CM, 'ProductCommand::Repl => "repl",', 'ProductCommand::Repl => "do",')],
          "ProductCommand admits do: ASK and DO are language modes, never "
          "CLI verbs, and DO is never a top-level command")
    probe("ask_cannot_enter_testpak_commands",
          [(CM, 'TestPakCommand::Forge => "forge",', 'TestPakCommand::Forge => "ask",')],
          "TestPakCommand admits ask: ASK and DO are language modes")
    # Owner coherence, with the generated row moved in lockstep so the
    # refusal is authorship, not projection staleness.
    probe("direct_command_owner_must_author_the_token",
          [(CM, 'ProductCommand::Serve => CommandAuthority::Direct(ContractId("BP-NETBAT-1")),',
            'ProductCommand::Serve => CommandAuthority::Direct(ContractId("BP-CRYPTO-SECRET-1")),'),
           (D26, "serve      direct     BP-NETBAT-1",
            "serve      direct     BP-CRYPTO-SECRET-1")],
          "ProductCommand serve cites owner BP-CRYPTO-SECRET-1, whose "
          "authoritative document does not author the token")
    probe("composite_command_owner_must_author_the_token",
          [(CM, 'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),',
            'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-NUMERIC-1"),'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-NUMERIC-1                 "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1")],
          "ProductCommand verify cites composition owner BP-NUMERIC-1, whose "
          "authoritative document does not author the token")
    probe("composite_command_requires_at_least_one_delegate",
          [(CM, 'delegates: &[\n                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            "delegates: &[],"),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1")],
          "ProductCommand verify is a composite with no delegates")
    probe("composite_command_cannot_repeat_a_delegate",
          [(CM, '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n                ],'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1")],
          "ProductCommand verify repeats a delegate")
    probe("composition_owner_cannot_be_its_own_delegate",
          [(CM, '                    ContractId("BP-GAUNTLET-1"),\n                ],',
            '                    ContractId("BP-GAUNTLET-1"),\n'
                '                    ContractId("BP-COMMAND-PLANE-1"),\n                ],'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1 BP-COMMAND-PLANE-1")],
          "ProductCommand verify lists its composition owner as its own "
          "delegate; composition never transfers ownership")
    # The composition boundary is declared in authored docs/26 prose, and the
    # typed authority must agree in BOTH directions.
    probe("compile_cannot_collapse_to_macbat_only",
          [(CM, 'ProductCommand::Compile => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-MACBAT-1"),\n'
                '                    ContractId("BP-BATQL-ARCH-1"),\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                ],\n            },',
            'ProductCommand::Compile => CommandAuthority::Direct(ContractId("BP-MACBAT-1")),'),
           (D26, "compile    composite  BP-COMMAND-PLANE-1           "
                 "BP-MACBAT-1 BP-BATQL-ARCH-1 BP-WORLD-PORTS-1",
            "compile    direct     BP-MACBAT-1")],
          "docs/26 declares compile a composition boundary; a Direct owner "
          "would collapse it to a single sovereign")
    probe("query_cannot_silently_gain_raw_source_compilation",
          [(CM, 'ProductCommand::Query => {\n'
                '                CommandAuthority::Direct(ContractId("BP-WORLD-PORTS-1"))\n'
                '            }',
            'ProductCommand::Query => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-BATQL-ARCH-1"),\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                ],\n            },'),
           (D26, "query      direct     BP-WORLD-PORTS-1",
            "query      composite  BP-COMMAND-PLANE-1           "
                 "BP-BATQL-ARCH-1 BP-WORLD-PORTS-1")],
          "docs/26 does not declare query a composition boundary; the typed "
          "authority may not silently gain a composite")
    probe("product_inspect_cannot_claim_receipts_as_sole_owner",
          [(CM, 'ProductCommand::Inspect => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-SCHEMA-CODEC-1"),\n'
                '                    ContractId("BP-PAKVM-ISA-1"),\n'
                '                    ContractId("BP-BVISOR-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                ],\n            },',
            'ProductCommand::Inspect => CommandAuthority::Direct(ContractId("BP-RECEIPTS-1")),'),
           (D26, "inspect    composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-SCHEMA-CODEC-1 BP-PAKVM-ISA-1 BP-BVISOR-1 BP-RECEIPTS-1",
            "inspect    direct     BP-RECEIPTS-1")],
          "docs/26 declares inspect a composition boundary")
    probe("product_verify_cannot_claim_receipts_as_sole_owner",
          [(CM, 'ProductCommand::Verify => CommandAuthority::Composite {\n'
                '                composition_owner: ContractId("BP-COMMAND-PLANE-1"),\n'
                '                delegates: &[\n'
                '                    ContractId("BP-WORLD-PORTS-1"),\n'
                '                    ContractId("BP-RECEIPTS-1"),\n'
                '                    ContractId("BP-GAUNTLET-1"),\n'
                '                ],\n            },',
            'ProductCommand::Verify => CommandAuthority::Direct(ContractId("BP-RECEIPTS-1")),'),
           (D26, "verify     composite  BP-COMMAND-PLANE-1           "
                 "BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1",
            "verify     direct     BP-RECEIPTS-1")],
          "docs/26 declares verify a composition boundary")
    # The ruled TestPak owners are pinned by the generated projection: moving
    # an owner in the spec without regenerating docs/26 is positional drift,
    # and regenerating docs/26 is a visible diff the promotion review reads.
    probe("testpak_inspect_is_owned_by_self_explaining_contract",
          [(CM, "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-SELF-EXPLAINING-1"))',
            "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 1 states inspect "
          "direct BP-SELF-EXPLAINING-1; the typed catalog states inspect "
          "direct BP-TESTPAK-1 at that position")
    probe("testpak_context_is_owned_by_self_explaining_contract",
          [(CM, "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-SELF-EXPLAINING-1"))',
            "TestPakCommand::Inspect | TestPakCommand::Context => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 8 states context "
          "direct BP-SELF-EXPLAINING-1; the typed catalog states context "
          "direct BP-TESTPAK-1 at that position")
    probe("fuzz_bench_and_prove_are_gauntlet_commands",
          [(CM, "TestPakCommand::Fuzz | TestPakCommand::Bench | TestPakCommand::Prove => {\n"
                '                CommandAuthority::Direct(ContractId("BP-GAUNTLET-1"))',
            "TestPakCommand::Fuzz | TestPakCommand::Bench | TestPakCommand::Prove => {\n"
                '                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))')],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 5 states fuzz "
          "direct BP-GAUNTLET-1; the typed catalog states fuzz direct "
          "BP-TESTPAK-1 at that position")
    probe("docs_product_command_order_drift_is_rejected",
          [(D26, "run        direct     BP-WORLD-PORTS-1\n"
                 "query      direct     BP-WORLD-PORTS-1",
            "query      direct     BP-WORLD-PORTS-1\n"
                 "run        direct     BP-WORLD-PORTS-1")],
          "docs/26_COMMAND_PLANE.md PRODUCT-COMMANDS row 2 states query "
          "direct BP-WORLD-PORTS-1; the typed catalog states run direct "
          "BP-WORLD-PORTS-1 at that position")
    probe("docs_testpak_command_order_drift_is_rejected",
          [(D26, "forge      direct     BP-TESTPAK-1\n"
                 "test       direct     BP-TESTPAK-1",
            "test       direct     BP-TESTPAK-1\n"
                 "forge      direct     BP-TESTPAK-1")],
          "docs/26_COMMAND_PLANE.md TESTPAK-COMMANDS row 2 states test "
          "direct BP-TESTPAK-1; the typed catalog states forge direct "
          "BP-TESTPAK-1 at that position")
    probe("batql_mode_projection_drift_is_rejected",
          [(CP, "ASK        direct     BP-BATQL-LANGUAGE-1",
            "ASK        direct     BP-BATQL-ARCH-1")],
          "companion/BATQL_LANGUAGE.md BATQL-SOURCE-MODES row 1 states ASK "
          "direct BP-BATQL-ARCH-1; the typed catalog states ASK direct "
          "BP-BATQL-LANGUAGE-1 at that position")
    probe("command_projection_source_marker_drift_is_rejected",
          [(D26, "<!-- PRODUCT-COMMANDS:BEGIN generated from spec/commands.rs "
                 "by bootstrap/project.py; do not edit -->",
            "<!-- PRODUCT-COMMANDS:BEGIN generated from spec/command_soup.rs "
                 "by bootstrap/project.py; do not edit -->")],
          "docs/26_COMMAND_PLANE.md carries no generated PRODUCT-COMMANDS "
          "block naming spec/commands.rs as source")
    probe("universal_command_id_is_absent_from_the_public_spec",
          [(CM, "use crate::guarantees::ContractId;",
            "use crate::guarantees::ContractId;\npub struct CommandId;")],
          "declares a universal CommandId; command identity lives in three "
          "separate namespaces or nowhere")
    return findings


def test_mutation(audit) -> list[str]:
    """Named hostile fixtures for the canonical mutation vocabulary (5.5E3f):
    one typed owner, total classification functions, no shadow tables, no
    lettered lane nicknames."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.mutation_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.mutation_findings(root):
        fail("mutation_vocabulary_passes_on_the_real_seed")

    MU = "spec/mutation.rs"
    D12 = "docs/12_TESTPAK.md"
    probe("mutation_lane_missing_from_all_is_rejected",
          [(MU, "        MutationLane::SelectableCompiled,\n", "")],
          "MutationLane::SelectableCompiled is omitted from MutationLane::ALL")
    probe("mutation_result_missing_from_all_is_rejected",
          [(MU, "        MutationResult::TimedOut,\n", "")],
          "MutationResult::TimedOut is omitted from MutationResult::ALL")
    probe("mutation_lane_with_unknown_owner_is_rejected",
          [(MU, '| MutationLane::CompilerBacked => ContractId("BP-TESTPAK-1"),',
            '| MutationLane::CompilerBacked => ContractId("BP-GHOST-1"),')],
          "MutationLane SemanticIr cites owner BP-GHOST-1, which no declared "
          "contract owns")
    probe("mutation_lane_with_dangling_admission_basis_is_rejected",
          [(MU, '| MutationLane::CompilerBacked => DecisionId("DEC-015"),',
            '| MutationLane::CompilerBacked => DecisionId("DEC-915"),')],
          "lane SemanticIr cites admission basis DEC-915, which no declared "
          "decision owns")
    # The admission boundary must NAME what it admits: a DEC-015 amendment
    # that stops spelling a lane un-admits it.
    probe("mutation_lane_basis_must_name_the_lane",
          [("spec/dispositions.rs",
            "SemanticIr mutates BatPak-owned semantic structures",
            "The semantic lane mutates BatPak-owned semantic structures")],
          "DEC-015 forward-policy fields do not name lane SemanticIr")
    # The semantic fences.
    probe("semantic_ir_cannot_require_real_rustc_semantics",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,",
            "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => true,")],
          "SemanticIr claims real rustc semantics; the reference interpreter "
          "lane must not")
    probe("selectable_compiled_requires_real_rustc_semantics",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,")],
          "SelectableCompiled must run under real rustc semantics")
    probe("selectable_compiled_does_not_require_per_candidate_compile",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,")],
          "SelectableCompiled must not require a per-candidate Rust compile")
    probe("compiler_backed_requires_per_candidate_compile",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,")],
          "CompilerBacked must require a per-candidate Rust compile")
    probe("lane_without_activation_evidence_is_rejected",
          [(MU, "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,",
            "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => true,\n"
                "            MutationLane::CompilerBacked => true,")],
          "lane SemanticIr does not require activation evidence")
    probe("lane_permitting_production_profile_slots_is_rejected",
          [(MU, "            MutationLane::SemanticIr => false,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,",
            "            MutationLane::SemanticIr => true,\n"
                "            MutationLane::SelectableCompiled => false,\n"
                "            MutationLane::CompilerBacked => false,")],
          "lane SemanticIr permits production-profile mutation slots")
    probe("lane_without_independent_evidence_route_is_rejected",
          [(MU, "pub const fn requires_independent_evidence_route(self) -> bool {\n"
                "        match self {\n"
                "            MutationLane::SemanticIr => true,",
            "pub const fn requires_independent_evidence_route(self) -> bool {\n"
                "        match self {\n"
                "            MutationLane::SemanticIr => false,")],
          "lane SemanticIr does not require an independent evidence route")
    probe("mutation_lane_wrong_gate_is_rejected",
          [(MU, "            MutationLane::SemanticIr => &[GateId::G3],",
            "            MutationLane::SemanticIr => &[GateId::G2],")],
          "lane SemanticIr gates ['G2'] are not exactly G3")
    # The classification teeth.
    probe("only_killed_counts_as_kill",
          [(MU, "            MutationResult::Survived => false,",
            "            MutationResult::Survived => true,")],
          "only Killed counts as a kill; Survived claims one")
    probe("only_survived_counts_as_survival",
          [(MU, "            MutationResult::Survived => true,\n"
                "            MutationResult::NotActivated => false,",
            "            MutationResult::Survived => true,\n"
                "            MutationResult::NotActivated => true,")],
          "only Survived counts as survival; NotActivated claims one")
    probe("equivalent_candidate_counts_as_neither_kill_nor_survival",
          [(MU, "            MutationResult::EquivalentCandidate => false,",
            "            MutationResult::EquivalentCandidate => true,")],
          "only Killed counts as a kill; EquivalentCandidate claims one")
    probe("unbuildable_cannot_count_as_kill",
          [(MU, "            MutationResult::Unbuildable => false,",
            "            MutationResult::Unbuildable => true,")],
          "only Killed counts as a kill; Unbuildable claims one")
    probe("every_mutation_result_stays_in_the_denominator",
          [(MU, "            MutationResult::TimedOut => true,",
            "            MutationResult::TimedOut => false,")],
          "TimedOut leaves the denominator")
    # The lettered nicknames are dead.
    probe("lane_letter_alias_is_rejected_in_authoritative_docs",
          [(D12, "### Three frozen lanes",
            "### Three frozen lanes (Lane A, Lane B, Lane C)")],
          "cites the letter alias 'Lane A' in authoritative prose")
    probe("docs_mutation_lane_projection_drift_is_rejected",
          [(D12, "SemanticIr           false   false   true    false   true    G3",
            "SemanticIr           false   true    true    false   true    G3")],
          "docs/12 MUTATION-LANES row 2 states SemanticIr false true true "
          "false true G3; the typed catalog states SemanticIr false false "
          "true false true G3 at that position")
    probe("docs_mutation_result_projection_drift_is_rejected",
          [(D12, "Killed                 true    false   true",
            "Killed                 true    true    true")],
          "docs/12 MUTATION-RESULTS row 2 states Killed true true true; the "
          "typed catalog states Killed true false true at that position")
    probe("mutation_projection_source_marker_drift_is_rejected",
          [(D12, "<!-- MUTATION-LANES:BEGIN generated from spec/mutation.rs "
                 "by bootstrap/project.py; do not edit -->",
            "<!-- MUTATION-LANES:BEGIN generated from spec/mutation_soup.rs "
                 "by bootstrap/project.py; do not edit -->")],
          "docs/12 carries no generated MUTATION-LANES block naming "
          "spec/mutation.rs as source")
    return findings


def test_corpus_epoch(audit) -> list[str]:
    """Named hostile fixtures for the corpus reconciliation epoch (5.5E3g):
    one typed owner, one current selection, corpus-wide membership, and no
    date wearing an epoch costume."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.corpus_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.corpus_findings(root):
        fail("corpus_epoch_passes_on_the_real_seed")

    CO = "spec/corpus.rs"
    D16 = "docs/16_IDENTITY_TIME_AND_NAVIGATION.md"
    _V2_ARMS = [
        (CO, "    CleanroomV1,\n}", "    CleanroomV1,\n    CleanroomV2,\n}"),
        (CO, "&[ReconciliationEpoch::CleanroomV1]",
         "&[ReconciliationEpoch::CleanroomV1, ReconciliationEpoch::CleanroomV2]"),
        (CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
         'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",\n'
         '            ReconciliationEpoch::CleanroomV2 => "cleanroom-v2",'),
        (CO, 'ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),',
         'ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),\n'
         '            ReconciliationEpoch::CleanroomV2 => ContractId("BP-CONSTITUTION-1"),'),
        (CO, 'ReconciliationEpoch::CleanroomV1 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }',
         'ReconciliationEpoch::CleanroomV1 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }\n'
             '            ReconciliationEpoch::CleanroomV2 => {\n'
             '                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))\n'
             '            }'),
    ]
    probe("reconciliation_epoch_missing_from_all_is_rejected",
          [(CO, "&[ReconciliationEpoch::CleanroomV1]", "&[]")],
          "ReconciliationEpoch::CleanroomV1 is omitted from ReconciliationEpoch::ALL")
    probe("current_reconciliation_epoch_must_be_declared",
          [(CO, "ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;",
            "ReconciliationEpoch = ReconciliationEpoch::CleanroomV9;")],
          "CURRENT_RECONCILIATION_EPOCH names CleanroomV9, which "
          "ReconciliationEpoch::ALL does not declare")
    probe("empty_reconciliation_epoch_spelling_is_rejected",
          [(CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
            'ReconciliationEpoch::CleanroomV1 => "",')],
          "ReconciliationEpoch::CleanroomV1 projects an empty spelling")
    probe("duplicate_reconciliation_epoch_spelling_is_rejected",
          [_V2_ARMS[0], _V2_ARMS[1],
           (CO, 'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",',
            'ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",\n'
            '            ReconciliationEpoch::CleanroomV2 => "cleanroom-v1",'),
           _V2_ARMS[3], _V2_ARMS[4]],
          "epoch spelling cleanroom-v1 is claimed twice")
    probe("reconciliation_epoch_with_unknown_owner_is_rejected",
          [(CO, 'ContractId("BP-CONSTITUTION-1")', 'ContractId("BP-GHOST-1")')],
          "epoch cleanroom-v1 cites owner BP-GHOST-1, which no declared "
          "contract owns")
    probe("reconciliation_epoch_owner_must_author_the_spelling",
          [(CO, 'ContractId("BP-CONSTITUTION-1")', 'ContractId("BP-NETBAT-1")')],
          "epoch cleanroom-v1 cites owner BP-NETBAT-1, whose authoritative "
          "document does not author ReconciliationEpoch")
    probe("reconciliation_epoch_with_dangling_basis_is_rejected",
          [(CO, 'SeedId("SEED-DOC-STATUS")', 'SeedId("SEED-GHOST")')],
          "epoch cleanroom-v1 cites admission basis SEED-GHOST, which no "
          "declared seed row owns")
    probe("reconciliation_epoch_basis_must_name_the_type",
          [("spec/invariants.rs",
            "reconciliation date, and its ReconciliationEpoch corpus membership.",
            "reconciliation date, and its corpus membership.")],
          "SEED-DOC-STATUS does not name ReconciliationEpoch in its statement")
    probe("authoritative_document_missing_epoch_is_rejected",
          [(D16, "reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md: declares no "
          "reconciliation_epoch")
    probe("authoritative_document_with_wrong_epoch_is_rejected",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: cleanroom-v0")],
          "claims corpus epoch cleanroom-v0; the current corpus epoch is "
          "cleanroom-v1")
    # Evidence stays in the corpus as evidence; it still claims membership.
    probe("evidence_document_missing_epoch_is_rejected",
          [("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "status: AUTHORITATIVE", "status: EVIDENCE-ONLY"),
           ("docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
            "reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md: declares no "
          "reconciliation_epoch")
    probe("two_documents_cannot_claim_different_current_epochs",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: cleanroom-v2")],
          "docs/16_IDENTITY_TIME_AND_NAVIGATION.md: claims corpus epoch "
          "cleanroom-v2; the current corpus epoch is cleanroom-v1")
    probe("date_cannot_substitute_for_reconciliation_epoch",
          [(D16, "reconciliation_epoch: cleanroom-v1",
            "reconciliation_epoch: 2026-07-13")],
          "a date (2026-07-13) cannot substitute for a corpus epoch")
    probe("last_reconciled_cannot_be_deleted_when_epoch_exists",
          [(D16, "last_reconciled: 2026-07-13\n", "")],
          "last_reconciled is deleted; the epoch answers WHICH corpus")
    probe("generated_document_missing_source_epoch_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "source_reconciliation_epoch: cleanroom-v1\n", "")],
          "docs/GUARANTEE_GRAPH.generated.md: declares no "
          "source_reconciliation_epoch")
    probe("generated_document_with_wrong_source_epoch_is_rejected",
          [("docs/GUARANTEE_GRAPH.generated.md",
            "source_reconciliation_epoch: cleanroom-v1",
            "source_reconciliation_epoch: cleanroom-v0")],
          "claims corpus epoch cleanroom-v0; the current corpus epoch is "
          "cleanroom-v1")
    probe("corpus_epoch_projection_drift_is_rejected",
          [("docs/00_CONSTITUTION.md",
            f"{'current epoch':<18} cleanroom-v1",
            f"{'current epoch':<18} cleanroom-v9")],
          "the typed owner states")
    probe("corpus_epoch_projection_source_marker_drift_is_rejected",
          [("docs/00_CONSTITUTION.md",
            "<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from "
            "spec/corpus.rs by bootstrap/project.py; do not edit -->",
            "<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from "
            "spec/corpse.rs by bootstrap/project.py; do not edit -->")],
          "docs/00 carries no generated CORPUS-RECONCILIATION-EPOCH block "
          "naming spec/corpus.rs as source")
    # Moving CURRENT without re-projecting the corpus strands every document
    # on the old epoch — the whole corpus reds, not just the constant.
    probe("changing_current_epoch_requires_whole_corpus_projection",
          _V2_ARMS + [
              (CO, "ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;",
               "ReconciliationEpoch = ReconciliationEpoch::CleanroomV2;")],
          "claims corpus epoch cleanroom-v1; the current corpus epoch is "
          "cleanroom-v2")
    probe("runtime_reconciliation_module_does_not_own_corpus_epoch",
          [("spec/reconciliation.rs", "pub enum ReconciliationRole {",
            "pub struct ReconciliationEpoch;\npub enum ReconciliationRole {")],
          "spec/reconciliation.rs speaks ReconciliationEpoch; the runtime "
          "reconciliation module does not own the corpus epoch")
    # GREEN growth: a declared future epoch with a unique spelling, the same
    # live constitutional owner, an authored constitutional description, and
    # ALL membership — CURRENT unchanged, no frozen count anywhere.
    tmp = gate_sandbox(_V2_ARMS + [
        ("docs/00_CONSTITUTION.md",
         "reconciliation that moves the current selection.",
         "reconciliation that moves the current selection. A future "
         "`cleanroom-v2` epoch is declared only by such a reconciliation.")])
    try:
        got = audit.corpus_findings(tmp)
        if got:
            fail(f"future_epoch_may_be_declared_without_moving_current "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_compiler_assumptions(audit) -> list[str]:
    """Named hostile fixtures for the admitted compiler-assumption kinds
    (5.5E3h): two sharp instruments, borders proven by hostiles — no hard
    violation, test allowance, dependency mechanism, or not-yet-earned idea
    can enter the ledger wearing an assumption costume."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.compiler_assumption_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.compiler_assumption_findings(root):
        fail("compiler_assumptions_pass_on_the_real_seed")

    CA = "spec/compiler_assumptions.rs"
    D19 = "docs/19_SECURITY_MODEL.md"

    def synthetic_kind(name, basis):
        """Full structural arms for a smuggled kind: everything present
        except an authoring owner and a basis that names it."""
        return [
            (CA, "    PointerProvenance,\n}",
             f"    PointerProvenance,\n    {name},\n}}"),
            (CA, "        CompilerAssumptionKind::PointerProvenance,\n    ];",
             "        CompilerAssumptionKind::PointerProvenance,\n"
             f"        CompilerAssumptionKind::{name},\n    ];"),
            (CA, 'CompilerAssumptionKind::PointerProvenance => "PointerProvenance",',
             'CompilerAssumptionKind::PointerProvenance => "PointerProvenance",\n'
             f'            CompilerAssumptionKind::{name} => "{name}",'),
            (CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
             'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),\n'
             f'            CompilerAssumptionKind::{name} => ContractId("BP-SECURITY-1"),'),
            (CA, 'CompilerAssumptionKind::PointerProvenance => GuaranteeRef::dec("DEC-067"),',
             'CompilerAssumptionKind::PointerProvenance => GuaranteeRef::dec("DEC-067"),\n'
             f'            CompilerAssumptionKind::{name} => GuaranteeRef::dec("{basis}"),'),
            (CA, "CompilerAssumptionKind::PointerProvenance => false,",
             "CompilerAssumptionKind::PointerProvenance => false,\n"
             f"            CompilerAssumptionKind::{name} => false,"),
            (CA, "CompilerAssumptionKind::PointerProvenance => GateId::G3,",
             "CompilerAssumptionKind::PointerProvenance => GateId::G3,\n"
             f"            CompilerAssumptionKind::{name} => GateId::G3,"),
            (CA, "CompilerAssumptionKind::PointerProvenance => GateId::G9,",
             "CompilerAssumptionKind::PointerProvenance => GateId::G9,\n"
             f"            CompilerAssumptionKind::{name} => GateId::G9,"),
        ]

    probe("compiler_assumption_kind_missing_from_all_is_rejected",
          [(CA, "        CompilerAssumptionKind::PointerProvenance,\n    ];", "    ];")],
          "CompilerAssumptionKind::PointerProvenance is omitted from "
          "CompilerAssumptionKind::ALL")
    probe("assumption_kind_with_unknown_owner_is_rejected",
          [(CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
            'CompilerAssumptionKind::PointerProvenance => ContractId("BP-GHOST-1"),')],
          "assumption kind PointerProvenance cites owner BP-GHOST-1, which no "
          "declared contract owns")
    probe("assumption_kind_owner_must_author_the_spelling",
          [(CA, 'CompilerAssumptionKind::PointerProvenance => ContractId("BP-SECURITY-1"),',
            'CompilerAssumptionKind::PointerProvenance => ContractId("BP-NETBAT-1"),')],
          "assumption kind PointerProvenance cites owner BP-NETBAT-1, whose "
          "authoritative document does not author the spelling")
    probe("assumption_kind_with_dangling_basis_is_rejected",
          [(CA, 'GuaranteeRef::dec("DEC-067"),', 'GuaranteeRef::dec("DEC-967"),')],
          "assumption kind PointerProvenance cites admission basis DEC-967, "
          "which no declared decision owns")
    probe("assumption_kind_basis_must_name_the_kind",
          [("spec/dispositions.rs",
            "pointer and integer-pointer crossings is PointerProvenance",
            "pointer and integer-pointer crossings is the pointer kind")],
          "assumption kind PointerProvenance cites admission basis DEC-067, "
          "whose forward-policy fields do not name the kind")
    probe("only_unsafe_memory_contract_requires_safety_marker",
          [(CA, "CompilerAssumptionKind::PointerProvenance => false,",
            "CompilerAssumptionKind::PointerProvenance => true,")],
          "only UnsafeMemoryContract intrinsically requires the SAFETY-CONTRACT "
          "marker; PointerProvenance claims it")
    probe("unsafe_memory_contract_must_require_safety_marker",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => true,",
            "CompilerAssumptionKind::UnsafeMemoryContract => false,")],
          "UnsafeMemoryContract must intrinsically require the SAFETY-CONTRACT "
          "marker")
    probe("assumption_kind_wrong_classification_gate_is_rejected",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G3,",
            "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G2,")],
          "assumption kind UnsafeMemoryContract classifies at G2; the ledger "
          "boundary is classified at G3")
    probe("assumption_kind_wrong_release_gate_is_rejected",
          [(CA, "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G9,",
            "CompilerAssumptionKind::UnsafeMemoryContract => GateId::G8,")],
          "assumption kind UnsafeMemoryContract qualifies for release at G8; "
          "the release seal consumes the ledger at G9")
    # The border: nothing enters without a basis that names it and an owner
    # that authors it — a decision sentence about a mechanism is an admission
    # direction, not a passport.
    for name, basis, fixture in (
            ("NarrowingNumericCast", "DEC-067",
             "narrowing_numeric_cast_cannot_become_a_ledger_escape"),
            ("LintSuppression", "DEC-068",
             "production_lint_suppression_cannot_become_an_assumption_kind"),
            ("ProductionExpect", "DEC-067",
             "production_expect_cannot_become_an_assumption_kind"),
            ("TestContextualExpect", "DEC-068",
             "test_contextual_expect_is_an_allowance_not_an_assumption"),
            ("SerdeInterop", "DEC-020",
             "compatibility_dependency_is_not_a_compiler_assumption"),
            ("RepresentationLayout", "DEC-067",
             "representation_layout_is_not_currently_admitted"),
            ("AtomicOrdering", "DEC-068",
             "atomic_ordering_is_not_currently_admitted"),
            ("FfiAbi", "DEC-068", "ffi_abi_is_not_currently_admitted"),
            ("TargetFeature", "DEC-068",
             "target_feature_is_not_currently_admitted"),
    ):
        probe(fixture, synthetic_kind(name, basis),
              f"assumption kind {name} cites admission basis {basis}, whose "
              "forward-policy fields do not name the kind")
    probe("docs_compiler_assumption_projection_drift_is_rejected",
          [(D19, "UnsafeMemoryContract   DEC-068   true    G3        G9",
            "UnsafeMemoryContract   DEC-068   false   G3        G9")],
          "docs/19 COMPILER-ASSUMPTION-KINDS row 2 states UnsafeMemoryContract "
          "DEC-068 false G3 G9; the typed catalog states UnsafeMemoryContract "
          "DEC-068 true G3 G9 at that position")
    probe("compiler_assumption_projection_source_marker_drift_is_rejected",
          [(D19, "<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
                 "spec/compiler_assumptions.rs by bootstrap/project.py; do not edit -->",
            "<!-- COMPILER-ASSUMPTION-KINDS:BEGIN generated from "
                 "spec/compiler_presumptions.rs by bootstrap/project.py; do not edit -->")],
          "docs/19 carries no generated COMPILER-ASSUMPTION-KINDS block naming "
          "spec/compiler_assumptions.rs as source")
    probe("universal_assumption_identity_is_absent_from_the_public_spec",
          [(CA, "use crate::gates::GateId;",
            "use crate::gates::GateId;\npub struct CompilerAssumptionId;")],
          "declares a universal assumption identity")
    probe("reconciliation_module_does_not_speak_assumption_kinds",
          [("spec/reconciliation.rs", "pub enum ReconciliationRole {",
            "pub struct CompilerAssumptionKindMirror(pub CompilerAssumptionKind);\n"
            "pub enum ReconciliationRole {")],
          "spec/reconciliation.rs speaks CompilerAssumptionKind")
    probe("architecture_module_does_not_reexport_assumption_kinds",
          [("spec/architecture.rs", "pub const REPOSITORY_NAME:",
            "pub use crate::compiler_assumptions::CompilerAssumptionKind;\n"
            "pub const REPOSITORY_NAME:")],
          "spec/architecture.rs speaks CompilerAssumptionKind")
    # GREEN structural growth — evidence that the parser and projection are
    # count-free ONLY: every structural surface present (variant, ALL, unique
    # spelling, live authoring owner, live decision naming it, full arms,
    # matching docs/19 row). It does not prove a real future adopter exists.
    tmp = gate_sandbox(synthetic_kind("FutureKindProbe", "DEC-068") + [
        ("spec/dispositions.rs",
         "unsafe blocks functions and impls is UnsafeMemoryContract;",
         "unsafe blocks functions and impls is UnsafeMemoryContract and a "
         "sandbox structural probe kind named FutureKindProbe;"),
        (D19, "A future kind enters",
         "A sandbox structural probe (`FutureKindProbe`) exercises this "
         "boundary as growth evidence only. A future kind enters"),
        (D19, "PointerProvenance      DEC-067   false   G3        G9",
         "PointerProvenance      DEC-067   false   G3        G9\n"
         "FutureKindProbe        DEC-068   false   G3        G9"),
    ])
    try:
        got = audit.compiler_assumption_findings(tmp)
        if got:
            fail(f"structural_future_kind_growth_is_count_free (refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_promotion(audit) -> list[str]:
    """Named hostile fixtures for the typed promotion denominator (5.5E3i):
    four bouncers with badges, all conjunctive, and the receipt cannot claim
    the other three showed up."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.promotion_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.promotion_findings(root):
        fail("promotion_requirements_pass_on_the_real_seed")

    PR = "spec/promotion.rs"
    D12 = "docs/12_TESTPAK.md"
    probe("promotion_requirement_missing_from_all_is_rejected",
          [(PR, "        PromotionRequirement::AuditablePromotionReceipt,\n    ];",
            "    ];")],
          "PromotionRequirement::AuditablePromotionReceipt is omitted from "
          "PromotionRequirement::ALL; the denominator is conjunctive")
    probe("duplicate_promotion_requirement_spelling_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => "NamedProofTarget",',
            'PromotionRequirement::NamedProofTarget => "IndependentEvidenceRoute",')],
          "promotion-requirement spelling IndependentEvidenceRoute is claimed twice")
    probe("empty_promotion_requirement_spelling_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => "NamedProofTarget",',
            'PromotionRequirement::NamedProofTarget => "",')],
          "PromotionRequirement::NamedProofTarget projects an empty spelling")
    probe("promotion_requirement_with_unknown_owner_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => ContractId("BP-TESTPAK-1"),',
            'PromotionRequirement::NamedProofTarget => ContractId("BP-GHOST-1"),')],
          "promotion requirement NamedProofTarget cites owner BP-GHOST-1, "
          "which no declared contract owns")
    probe("promotion_requirement_owner_must_author_the_spelling",
          [(PR, 'PromotionRequirement::NamedProofTarget => ContractId("BP-TESTPAK-1"),',
            'PromotionRequirement::NamedProofTarget => ContractId("BP-NETBAT-1"),')],
          "promotion requirement NamedProofTarget cites owner BP-NETBAT-1, "
          "whose authoritative document does not author the spelling")
    probe("promotion_requirement_with_dangling_basis_is_rejected",
          [(PR, 'PromotionRequirement::NamedProofTarget => GuaranteeRef::dec("DEC-015"),',
            'PromotionRequirement::NamedProofTarget => GuaranteeRef::dec("DEC-915"),')],
          "promotion requirement NamedProofTarget cites admission basis DEC-915, "
          "which no declared decision owns")
    probe("promotion_requirement_basis_must_name_the_requirement",
          [("spec/dispositions.rs",
            "is IndependentEvidenceRoute NamedProofTarget QualifiedHostileEvidence",
            "is IndependentEvidenceRoute QualifiedHostileEvidence")],
          "promotion requirement NamedProofTarget cites admission basis DEC-015, "
          "whose forward-policy fields do not name the requirement")
    # The doctrine clauses are law; deleting one reds.
    probe("promotion_requirements_are_conjunctive",
          [(D12, "a missing member\nrefuses promotion,", "each matters,")],
          "docs/12 promotion doctrine no longer states: the requirements are "
          "conjunctive")
    probe("independent_evidence_route_cannot_be_self_grading",
          [(D12, "are not independence.", "are usually enough.")],
          "docs/12 promotion doctrine no longer states: the evidence route "
          "cannot be self-grading")
    probe("named_proof_target_cannot_be_a_generic_issue",
          [(D12, "A generic issue link, chat\ntranscript,", "A chat\ntranscript,")],
          "docs/12 promotion doctrine no longer states: a generic issue is not "
          "a named proof target")
    probe("named_proof_target_cannot_be_a_commit_message",
          [(D12, "chat\ntranscript, commit message, or vague",
            "chat\ntranscript or vague")],
          "docs/12 promotion doctrine no longer states: a commit message is not "
          "a named proof target")
    probe("documented_proof_gap_requires_a_stable_owner",
          [(D12, "a stable name, a ContractId owner, the affected",
            "a name and the affected")],
          "docs/12 promotion doctrine no longer states: a documented proof gap "
          "requires a stable owner")
    probe("qualified_hostile_evidence_requires_rule_qualification",
          [(D12, "satisfying the docs/24 Rule qualification law",
            "satisfying a reviewer")],
          "docs/12 promotion doctrine no longer states: qualified hostile "
          "evidence consumes the docs/24 Rule qualification law")
    for token, fixture in (
            ("NotActivated", "not_activated_cannot_satisfy_qualified_hostile_evidence"),
            ("Unbuildable", "unbuildable_cannot_satisfy_qualified_hostile_evidence"),
            ("InfrastructureFailure",
             "infrastructure_failure_cannot_satisfy_qualified_hostile_evidence")):
        probe(fixture,
              [(D12, f"{token},", "")] if token != "InfrastructureFailure" else
              [(D12, "TimedOut, InfrastructureFailure,", "TimedOut,")],
              f"docs/12 no longer states that {token} cannot satisfy "
              "QualifiedHostileEvidence")
    probe("equivalent_candidate_is_not_a_killed_mutant",
          [(D12, "EquivalentCandidate without its independent equivalence witness,",
            "EquivalentCandidate,")],
          "docs/12 promotion doctrine no longer states: an equivalent candidate "
          "is not a killed mutant")
    probe("baseline_failure_cannot_kill_a_candidate",
          [(D12, "or a failure produced by the unchanged baseline.",
            "or a flaky run.")],
          "docs/12 promotion doctrine no longer states: a baseline failure "
          "cannot kill a candidate")
    probe("commit_message_cannot_substitute_for_promotion_receipt",
          [(D12, "A commit message\nis not this receipt.", "The receipt is above.")],
          "docs/12 promotion doctrine no longer states: a commit message is not "
          "the promotion receipt")
    probe("promotion_receipt_must_bind_resulting_tracked_content",
          [(D12, "promotion decision, and resulting tracked-content commitment.",
            "and promotion decision.")],
          "docs/12 promotion doctrine no longer states: the receipt binds the "
          "resulting tracked-content commitment")
    # The family facts answer four different questions.
    probe("promotion_policy_surface_must_be_candidate_promotion",
          [(PR, "ProofPolicySurface::CandidatePromotion;",
            "ProofPolicySurface::WaiverLogic;")],
          "the promotion policy surface is not ProofPolicySurface::CandidatePromotion")
    probe("promotion_change_basis_must_be_dec074",
          [(PR, 'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec("DEC-074");',
            'PROMOTION_CHANGE_BASIS: GuaranteeRef = GuaranteeRef::dec("DEC-015");')],
          "the promotion policy-change basis is not DEC-074")
    probe("promotion_enforcement_gate_must_be_g3",
          [(PR, "PROMOTION_ENFORCEMENT_GATE: GateId = GateId::G3;",
            "PROMOTION_ENFORCEMENT_GATE: GateId = GateId::G9;")],
          "candidate-promotion policy is not enforced at G3")
    probe("promotion_release_visibility_gate_must_be_g9",
          [(PR, "PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::G9;",
            "PROMOTION_RELEASE_VISIBILITY_GATE: GateId = GateId::G3;")],
          "promotion policy changes are not release-visibly qualified at G9")
    probe("docs_promotion_projection_drift_is_rejected",
          [(D12, "NamedProofTarget           BP-TESTPAK-1   DEC-015",
            "NamedProofTarget           BP-TESTPAK-1   DEC-074")],
          "docs/12 PROMOTION-REQUIREMENTS row 3 states NamedProofTarget "
          "BP-TESTPAK-1 DEC-074; the typed catalog states NamedProofTarget "
          "BP-TESTPAK-1 DEC-015 at that position")
    probe("promotion_projection_source_marker_drift_is_rejected",
          [(D12, "<!-- PROMOTION-REQUIREMENTS:BEGIN generated from "
                 "spec/promotion.rs by bootstrap/project.py; do not edit -->",
            "<!-- PROMOTION-REQUIREMENTS:BEGIN generated from "
                 "spec/demotion.rs by bootstrap/project.py; do not edit -->")],
          "docs/12 carries no generated PROMOTION-REQUIREMENTS block naming "
          "spec/promotion.rs as source")
    probe("docs24_must_consume_rule_qualification_without_reowning_requirements",
          [("docs/24_GAUNTLET.md",
            "is not restated here.",
            "is not restated here, except IndependentEvidenceRoute.")],
          "docs/24 restates the promotion-requirement inventory")
    probe("mutation_lane_consumer_cannot_point_to_architecture_module",
          [("docs/24_GAUNTLET.md",
            "The typed names are `MutationLane` in `spec/mutation.rs`",
            "The typed names are `MutationLane` in `spec/architecture.rs`")],
          "docs/24 points MutationLane at spec/architecture.rs; the typed owner "
          "moved to spec/mutation.rs")
    probe("docs31_cannot_restate_a_second_promotion_inventory",
          [("docs/31_FINAL_CONTRADICTION_AUDIT.md",
            "`PromotionRequirement::ALL` is unsatisfied",
            "`PromotionRequirement::ALL` (IndependentEvidenceRoute, "
            "NamedProofTarget, QualifiedHostileEvidence, "
            "AuditablePromotionReceipt) is unsatisfied")],
          "docs/31 restates IndependentEvidenceRoute; the audit consumer owns "
          "no second promotion inventory")
    # GREEN structural growth — count-freedom evidence only.
    tmp = gate_sandbox([
        (PR, "    AuditablePromotionReceipt,\n}",
         "    AuditablePromotionReceipt,\n    ReproducibleCandidateOrigin,\n}"),
        (PR, "        PromotionRequirement::AuditablePromotionReceipt,\n    ];",
         "        PromotionRequirement::AuditablePromotionReceipt,\n"
         "        PromotionRequirement::ReproducibleCandidateOrigin,\n    ];"),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => "AuditablePromotionReceipt",',
         'PromotionRequirement::AuditablePromotionReceipt => "AuditablePromotionReceipt",\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => "ReproducibleCandidateOrigin",'),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => ContractId("BP-TESTPAK-1"),',
         'PromotionRequirement::AuditablePromotionReceipt => ContractId("BP-TESTPAK-1"),\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => ContractId("BP-TESTPAK-1"),'),
        (PR, 'PromotionRequirement::AuditablePromotionReceipt => GuaranteeRef::dec("DEC-015"),',
         'PromotionRequirement::AuditablePromotionReceipt => GuaranteeRef::dec("DEC-015"),\n'
         '            PromotionRequirement::ReproducibleCandidateOrigin => GuaranteeRef::dec("DEC-015"),'),
        ("spec/dispositions.rs",
         "QualifiedHostileEvidence and AuditablePromotionReceipt",
         "QualifiedHostileEvidence AuditablePromotionReceipt and "
         "ReproducibleCandidateOrigin"),
        (D12, "`AuditablePromotionReceipt`: the complete receipt below.",
         "`ReproducibleCandidateOrigin`: a sandbox structural probe as growth "
         "evidence only. `AuditablePromotionReceipt`: the complete receipt below."),
        (D12, "AuditablePromotionReceipt  BP-TESTPAK-1   DEC-015",
         "AuditablePromotionReceipt  BP-TESTPAK-1   DEC-015\n"
         "ReproducibleCandidateOrigin BP-TESTPAK-1   DEC-015"),
    ])
    try:
        got = audit.promotion_findings(tmp)
        if got:
            fail(f"structural_future_requirement_growth_is_count_free "
                 f"(refused: {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_seedcheck_executes_its_law(_audit) -> list[str]:
    """Tier 0 rules, proven by RUNNING seedcheck against mutated typed sources.

    Two rules were repaired in 5.5D4c2 after seedcheck was executed for the first
    time. A repair with no adversary is indistinguishable from a nerf, so each one
    is planted against here. These build and run the real binary: asserting on the
    source text would prove the file contains a string, not that the gate bites.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        receipt("tier0-law-fixtures", available=False, reason="rustc absent")
        return findings

    used_triples: set[str] = set()

    def run_seedcheck(tree: Path) -> tuple[int, str]:
        # seedcheck links the typed tables at COMPILE time through the tree's
        # spec rlib (5.5E2), so a mutated spec/ is only tested by building the
        # library from the mutated tree. The real seedcheck.rs is copied in
        # rather than reproduced: a hand-written stand-in would drift from the
        # gate it claims to test. A mutation the COMPILER refuses is a refusal,
        # not linker unavailability: its rustc output is returned so a probe
        # can assert the exact reason instead of silently skipping.
        (tree / "bootstrap").mkdir(exist_ok=True)
        if not (tree / "bootstrap/seedcheck.rs").is_file():
            shutil.copy2(root / "bootstrap/seedcheck.rs", tree / "bootstrap/seedcheck.rs")
        tmp = Path(tempfile.mkdtemp(prefix="batpak-tier0-"))
        try:
            exe = tmp / ("sc" + (".exe" if os.name == "nt" else ""))
            built = None
            for target in (None, "x86_64-pc-windows-gnu"):
                rlib = tmp / f"libspec-{target or 'host'}.rlib"
                lib_cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "rlib",
                           "--crate-name", "spec", "-o", str(rlib),
                           str(tree / "spec/lib.rs")]
                if target:
                    lib_cmd[1:1] = ["--target", target]
                lib_built = subprocess.run(lib_cmd, capture_output=True, text=True)
                if lib_built.returncode != 0:
                    if "linking with" not in lib_built.stderr:
                        return (lib_built.returncode, lib_built.stderr)
                    continue
                cmd = [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-name", "sc",
                       "--extern", f"spec={rlib}",
                       "-o", str(exe), str(tree / "bootstrap/seedcheck.rs")]
                if target:
                    cmd[1:1] = ["--target", target]
                built = subprocess.run(cmd, capture_output=True, text=True)
                if built.returncode == 0:
                    # The receipt names the real triple, never "host default"
                    # (5.5E2c): on the hosted MSVC runner the default-target
                    # build IS the authoritative qualification, and a
                    # law-fixture receipt that hides its triple can never
                    # enter the required-receipt denominator.
                    if target is None:
                        m = re.search(r"host: (\S+)",
                                      subprocess.run([rustc, "-vV"], capture_output=True,
                                                     text=True).stdout)
                        target = m.group(1) if m else "host default"
                    used_triples.add(target)
                    break
                if "linking with" not in built.stderr:
                    return (built.returncode, built.stderr)
            else:
                return (-1, "unavailable")
            if not exe.is_file():
                return (-1, "unavailable")
            proc = subprocess.run([str(exe), str(tree)], capture_output=True, text=True)
            return (proc.returncode, proc.stdout + proc.stderr)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def probe(name: str, rel: str, old: str, new: str, needle: str) -> None:
        with isolated_tree() as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            code, out = run_seedcheck(tmp)
            if code == -1:
                print(f"selftest: {name} unavailable (no working linker)")
                return
            if code == 0:
                findings.append(f"{name} FAILED (seedcheck admitted the mutation)")
            elif needle not in out:
                findings.append(f"{name} FAILED (wanted {needle!r}, got {out.strip()[:200]!r})")

    # The premise: Tier 0 passes before any mutation. Without this every probe
    # below could be red for free.
    code, out = run_seedcheck(root)
    if code == -1:
        receipt("tier0-law-fixtures", available=False, compiled=True, executed=False,
                reason="no working linker for any attempted target")
        return findings
    if code != 0:
        receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
                passed=False, reason="seedcheck already fails before any mutation")
        findings.append(f"seedcheck_passes_before_mutation FAILED ({out.strip()[:200]!r})")
        return findings

    DI = "spec/dispositions.rs"
    IN = "spec/invariants.rs"

    # DEC-072: the CLASS decides. A class that requires a gate cannot be empty.
    probe("gate_requiring_decision_class_with_no_gate_is_rejected", DI,
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8],',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, gates: &[],',
          "DEC-063 is implementation-bearing and names no gate")

    # ...and the exception is DEC-specific. SEED declares no gate-independent
    # class, so it must not inherit DEC's lawful emptiness. This is the probe that
    # separates the repair from a nerf.
    probe("seed_does_not_inherit_the_decision_gate_exception", IN,
          "gates: &[GateId::G0, GateId::G5]",
          "gates: &[]",
          "names no gate")

    # A typed relation carries its family in the TYPE; existence is this
    # executed law. A reference to an undeclared decision must redden the
    # running seedcheck, not merely fail to render an edge.
    probe("dangling_typed_relation_is_rejected", IN,
          'derives_from: &[GuaranteeRef::dec("DEC-068")]',
          'derives_from: &[GuaranteeRef::dec("DEC-999")]',
          "which no declared row owns")

    # The same existence law for witness citations (5.5E2): a witness naming
    # an undeclared obligation reddens the running binary.
    probe("dangling_witness_citation_is_rejected", IN,
          'witnesses: &[WitnessRef::leg("LEG-066"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          'witnesses: &[WitnessRef::leg("LEG-999"), '
          'WitnessRef::contract("BP-GAUNTLET-1")]',
          "which no declared row owns")

    # A contract kind's admitting law resolves or the running binary reddens:
    # a kind whose law was deleted survives as nothing (5.5E3c).
    probe("contract_kind_with_dangling_admitting_law_is_rejected",
          "spec/contracts.rs",
          'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
          'ContractKind::Error => GuaranteeRef::leg("LEG-999"),',
          "resolves to no declared row")

    # Retirement is supersession with a forwarding address that must point at
    # the LIVING CENSUS (5.5E2j): a successor resolves inside the typed
    # catalog or the running binary reddens.
    probe("dangling_proof_row_successor_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("row_that_was_never_authored_anywhere")]',
          "resolves to no typed catalog identity")

    # ...and a name that occurs in docs/24 PROSE is not a catalog identity:
    # under the deleted substring search, "committed" (which opens a substrate
    # prose line) satisfied existence.
    probe("retired_successor_mentioned_only_in_prose_is_rejected", "spec/proof.rs",
          'successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")]',
          'successors: &[ProofRowId("committed")]',
          "resolves to no typed catalog identity")

    # The census defect (an active row deleted while a migration note kept
    # its name) retired with the docs/24 membership parity in 5.5E4d: the
    # typed catalog in spec/proof.rs is the one membership authority, and
    # deleting a typed row takes its guarantee binding and its docs/21
    # projection with it -- witness_reference_findings and the ledger-parity
    # laws own that refusal now.

    # The RUNNING binary refuses a hand-edited toolchain projection and a
    # physical triple sitting in a semantic qualification target (5.5E3a).
    probe("hand_edited_toolchain_projection_reddens_seedcheck",
          "rust-toolchain.toml",
          'channel = "1.97.0"',
          'channel = "1.96.0"',
          "toolchain tracked rust-toolchain.toml violated")
    # 5.5E3a1: the environment is a closed enum, so a triple in the field is
    # a COMPILE refusal — the strongest form. run_seedcheck returns the rustc
    # output for a compile-refused mutation, and the probe asserts the exact
    # type error rather than a runtime finding.
    probe("physical_triple_cannot_substitute_for_semantic_profile",
          "spec/architecture.rs",
          'environment: QualificationEnvironment::WasmHost,',
          'environment: "wasm32-unknown-unknown",',
          "expected `QualificationEnvironment`")
    # 5.5E3d residue lifecycle, executed by the RUNNING gate: a decision
    # retained only as historical coverage cannot own a future admission
    # barrier, and an existing typed owner's spelling is never re-admitted.
    probe("seedcheck_historical_decision_cannot_own_not_yet_admitted_term",
          "spec/identities.rs",
          'NotYetAdmittedBy(DecisionId("DEC-063"))',
          'NotYetAdmittedBy(DecisionId("DEC-005"))',
          "CompressionId is not yet admitted by DEC-005, which is retained "
          "only as historical coverage and cannot own a future admission barrier")
    probe("seedcheck_wrapper_passport_is_rejected",
          "spec/identities.rs",
          'entry!("RotationId", "BP-CRYPTO-SECRET-1")',
          'entry!("PackageId", "BP-CRYPTO-SECRET-1")',
          "duplicates PackageId, which already has a typed spec owner")
    # 5.5E3d1: the typed predicate, not the Permanent lifetime, decides who
    # owns a pending application — Kill is a dead passport.
    probe("seedcheck_killed_decision_cannot_own_not_yet_admitted_term",
          "spec/dispositions.rs",
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Lock,',
          'DecisionSpec { id: "DEC-063", class: DecisionClass::ImplementationPosture, '
          'gates: &[GateId::G2, GateId::G8], disposition: Disposition::Kill,',
          "CompressionId is not yet admitted by DEC-063, whose Kill disposition is "
          "not a standing future-entry policy; only Lock and Defer own pending "
          "applications")

    # SEED-AUDITED-DENOMINATOR's fence is executed law: reclassifying Expired
    # as green must redden the running seedcheck through
    # is_positive_semantic_terminal().
    probe("expired_proof_counting_green_is_refused",
          "spec/proof.rs",
          "            ProofUnitTerminal::Passed => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Expired\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "            ProofUnitTerminal::Passed | ProofUnitTerminal::Expired => true,\n"
          "            ProofUnitTerminal::Failed\n"
          "            | ProofUnitTerminal::Refused\n"
          "            | ProofUnitTerminal::Unsupported\n"
          "            | ProofUnitTerminal::SkippedWithAuthority\n"
          "            | ProofUnitTerminal::Superseded => false,",
          "is positive; only Passed may be")

    # DEC-075's retry fence is executed law, not a comment: reclassifying
    # elapsed wall time as an admissible retry signal must redden the running
    # seedcheck through the real admissible() function.
    probe("weakened_retry_classification_is_refused",
          "spec/reconciliation.rs",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::ProcessDeath",
          "            | RetrySignal::OverallMonotonicDeadline\n"
          "            | RetrySignal::ElapsedWallTime\n"
          "            | RetrySignal::RuntimeRestartAuthorization => true,\n"
          "            RetrySignal::ProcessDeath",
          "elapsed wall time may not authorize retry")

    # Candidate containment is one law across two constants: an output root
    # relocated under a forbidden write surface must redden seedcheck, not wait
    # for Phase 6 to write a candidate into tracked source.
    probe("candidate_output_root_under_a_forbidden_root_is_rejected",
          "spec/architecture.rs",
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";',
          'pub const CANDIDATE_OUTPUT_ROOT: &str = "spec/candidates/";',
          "sits under forbidden write root")

    # A generated projection is classified by its MARKER, not its filename. An
    # authored document cannot shed its frontmatter duties by looking generated,
    # and a projection that drops its marker takes the authored duties back.
    probe("authored_document_cannot_evade_frontmatter_by_looking_generated",
          "docs/GUARANTEE_GRAPH.generated.md",
          "status: GENERATED", "status: AUTHORITATIVE",
          "missing contract_id: in docs/GUARANTEE_GRAPH.generated.md")

    # A generated projection must still name what produced it. Provenance is the
    # price of not claiming authored authority.
    probe("generated_projection_without_source_provenance_is_rejected",
          "docs/GUARANTEE_GRAPH.generated.md",
          "generated_from:", "produced_from:",
          "missing generated_from: in docs/GUARANTEE_GRAPH.generated.md")

    receipt("tier0-law-fixtures", available=True, compiled=True, executed=True,
            passed=not findings,
            target=", ".join(sorted(used_triples)) or "no triple resolved")
    return findings


def test_rust_specification_compiles(_audit) -> list[str]:
    """The typed specification surface must actually compile (5.5D4b).

    Until this session no rustc was available and every Rust contract shipped
    compiler-unverified. When a toolchain IS present this runs; when it is absent
    it reports that plainly rather than passing silently. A skipped check that
    prints PASS is exactly the fake proof this project refuses.
    """
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: rustc unavailable; Rust specification surface remains "
              "compiler-unverified until Gate 0")
        return findings
    tmp = Path(tempfile.mkdtemp(prefix="batpak-rustc-"))
    try:
        # The spec is a real LIBRARY (spec/lib.rs, 5.5E2): one rlib boundary,
        # edition 2024, zero suppressions. The binaries LINK it; nothing mounts
        # spec modules textually anymore, so the visibility the probes below
        # attack is the exact boundary production uses.
        spec_meta = tmp / "libspec.rmeta"
        proc = subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
             "--crate-name", "spec", "-o", str(spec_meta), str(root / "spec/lib.rs")],
            capture_output=True, text=True)
        if proc.returncode != 0:
            head = "\n".join(proc.stderr.splitlines()[:6])
            findings.append(f"rust specification surface does not compile (spec):\n{head}")
            return findings
        for name, src in (
            ("seedcheck", root / "bootstrap/seedcheck.rs"),
            ("materialize", root / "bootstrap/materialize.rs"),
        ):
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--emit=metadata", "--crate-name", name,
                 "--extern", f"spec={spec_meta}",
                 "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode != 0:
                head = "\n".join(proc.stderr.splitlines()[:6])
                findings.append(
                    f"rust specification surface does not compile ({name}):\n{head}")
        # seedcheck is Tier 0. Compiling it is not running it: until this check
        # existed it was only ever type-checked, and it had been FAILING since
        # 5.5C1 -- three DEC rows red against a gate rule its own file already
        # said the class decides, and the generated Guarantee Graph red against a
        # frontmatter rule that predated generated documents. Nobody saw it
        # because nobody ran it. A gate that is compiled and never executed is a
        # gate in the same sense that a locked door in a field is a gate.
        # Local execution is SUPPLEMENTAL evidence. The authoritative Windows
        # receipt is a hosted MSVC runner; this machine's linker is whatever
        # happens to be installed, and no toolchain here is a declared BatPak
        # prerequisite. Report exactly one of executed-and-passed,
        # executed-and-failed, or unavailable. Never let unavailable wear the
        # green of a check that passed.
        # BOTH bootstrap binaries execute. materialize.rs was the same species as
        # seedcheck: it existed, it type-checked, and it had never once run.
        findings.extend(qualify_binary(rustc, root, tmp, "tier0-seedcheck",
                                       "bootstrap/seedcheck.rs", "seedcheck: PASS"))
        # tier0-materialize rides its own isolated qualification (5.5E5): two
        # temporary output roots outside the seed, a full seed snapshot, an
        # independent byte oracle, an Unchanged rerun, and Cargo on a
        # disposable copy of the candidate. The generic one-root route is the
        # exact defect that scaffolded the inspector's kitchen.
        findings.extend(qualify_materializer(rustc, root, tmp))
        # The #[cfg(test)] refusal tests inside seedcheck are claims too, and a
        # plain compile STRIPS them: until D4d they had never executed or even
        # type-checked -- the same species as materialize, wearing test syntax.
        # The expected count derives from the authored source at run time, so a
        # deleted test shrinks the expectation and an emptied harness is a
        # finding, never a quiet "ok. 0 passed".
        n_tests = (root / "bootstrap/seedcheck.rs").read_text(
            encoding="utf-8").count("#[test]")
        if n_tests == 0:
            findings.append(
                "seedcheck declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-seedcheck-tests", "bootstrap/seedcheck.rs",
            f"test result: ok. {n_tests} passed", extra=("--test",), run_args=()))
        # The spec's OWN tests (5.5E2): the ISA admission refusals moved into
        # spec/pakvm_isa.rs where the seam they exercise lives — a downstream
        # crate's test cfg no longer reaches across the boundary, which is the
        # boundary working. Qualified with the same artifact binding.
        n_spec_tests = sum(p.read_text(encoding="utf-8").count("#[test]")
                           for p in sorted((root / "spec").glob("*.rs")))
        if n_spec_tests == 0:
            findings.append(
                "the spec declares no #[test] refusal tests; the harness was emptied")
        findings.extend(qualify_binary(
            rustc, root, tmp, "tier0-spec-tests", "spec/lib.rs",
            f"test result: ok. {n_spec_tests} passed", extra=("--test",),
            run_args=(), link_spec=False))
        # The admission desks have no public bypass and the sealed witnesses
        # cannot be forged or fabricated — proven by COMPILING a hostile
        # consumer against the REAL crate boundary (--extern spec), the exact
        # visibility configuration production uses since the 5.5E2 bake. Each
        # probe must fail for ITS reason: a probe red for an unrelated compile
        # error would certify a boundary nobody tested.
        for name, uses, body, want in (
            ("isa_seam_is_absent_from_production",
             "use spec::pakvm_isa::*;",
             "let ap = algebra_policy(PakVmAlgebra::QueryDataflow).unwrap();\n"
             "    let cp = class_policy(PakVmNodeClass::RowTransform).unwrap();\n"
             "    let _ = admit_candidate_policy(PakVmNodeId::Filter, ap, cp);",
             "cannot find function `admit_candidate_policy`"),
            ("isa_admission_witness_cannot_be_forged",
             "use spec::pakvm_isa::*;",
             "let _ = AdmittedAsPublicSemanticNode(());",
             "cannot initialize a tuple struct which contains private fields"),
            ("firewall_admitted_crossing_cannot_be_forged",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = AdmittedCrossing {\n"
             "        from: SyncBatPlane::Bvisor, to: SyncBatPlane::Runtime,\n"
             "        carries: SyncBatAuthority::RetryRestartAuthority,\n"
             "        posture: CrossingPosture::Required,\n"
             '        origin: None, law: "minted by hand", seal: (),\n'
             "    };",
             "field `seal` of struct"),
            ("firewall_origin_cannot_be_fabricated",
             "use spec::architecture::SyncBatPlane;\n"
             "use spec::pakvm_isa::PakVmNodeId;\nuse spec::syncbat_firewall::*;",
             "let _ = admit_crossing(SyncBatPlane::PakVm, SyncBatPlane::Bvisor,\n"
             "        SyncBatAuthority::TypedEffectRequest, Some(PakVmNodeId::ReachTheHost));",
             "named `ReachTheHost` found for enum `PakVmNodeId`"),
            # 5.5E2: per-family guarantee identity is sealed at the crate
            # boundary. A consumer can RECEIVE a GuaranteeRef and read it, but
            # cannot mint one claiming a family it did not come from.
            ("guarantee_identity_cannot_be_minted",
             "use spec::guarantees::{GuaranteeRef, SeedId};",
             'let _ = GuaranteeRef::Seed(SeedId("SEED-FORGED"));',
             "cannot initialize a tuple struct which contains private fields"),
            # 5.5E3b: the package identity is the variant; strings cannot
            # mint the guarantee identities and typed relationships cannot
            # be read as raw names.
            ("architecture_guarantee_id_cannot_be_minted_from_a_string",
             "use spec::guarantees::ArchitectureGuaranteeId;",
             'let _ = ArchitectureGuaranteeId("batpak");',
             "cannot initialize a tuple struct which contains private fields"),
            ("qualification_id_cannot_carry_a_string_package",
             "use spec::guarantees::QualificationId;",
             'let _ = QualificationId { package: "batpak", profile: "semantic" };',
             "expected `PackageId`, found `&str`"),
            ("dependency_endpoint_is_package_typed",
             "use spec::architecture::EDGES;",
             "let _: &str = EDGES[0].importer;",
             "expected `&str`, found `PackageId`"),
            ("qualification_package_is_package_typed",
             "use spec::architecture::QUALIFICATION_PROFILES;",
             "let _: &str = QUALIFICATION_PROFILES[0].package;",
             "expected `&str`, found `PackageId`"),
            ("syncbat_plane_package_is_package_typed",
             "use spec::architecture::SyncBatPlane;",
             "let _: &str = SyncBatPlane::Runtime.package();",
             "expected `&str`, found `PackageId`"),
            # 5.5E3d: "not currently admitted" and "forbidden by decision"
            # are DIFFERENT laws, and no term is currently decision-forbidden,
            # so a forbidden disposition has no spelling at all. Mislabeling
            # the pending passport application as a dead passport is a
            # compile error, not a reclassification.
            ("future_required_identity_cannot_be_mislabeled_rejected",
             "use spec::identities::IdentityTermDisposition;\n"
             "use spec::guarantees::DecisionId;",
             'let _ = IdentityTermDisposition::RejectedBy(DecisionId("DEC-063"));',
             "no variant, associated function, or constant named `RejectedBy`"),
            # 5.5E3e: the three command namespaces do not substitute for one
            # another — the same surface token, different passports.
            ("product_inspect_cannot_substitute_for_testpak_inspect",
             "use spec::commands::{ProductCommand, TestPakCommand};",
             "let _: TestPakCommand = ProductCommand::Inspect;",
             "expected `TestPakCommand`, found `ProductCommand`"),
            ("batql_mode_cannot_substitute_for_product_command",
             "use spec::commands::{BatQlSourceMode, ProductCommand};",
             "let _: ProductCommand = BatQlSourceMode::Ask;",
             "expected `ProductCommand`, found `BatQlSourceMode`"),
            # 5.5E3f: the mutation vocabulary is typed — no raw string mints
            # a lane or result, and architecture.rs no longer re-exports the
            # identity it surrendered.
            ("raw_string_cannot_substitute_for_mutation_lane",
             "use spec::mutation::MutationLane;",
             'let _: MutationLane = "SemanticIr";',
             "expected `MutationLane`, found `&str`"),
            ("raw_string_cannot_substitute_for_mutation_result",
             "use spec::mutation::MutationResult;",
             'let _: MutationResult = "Killed";',
             "expected `MutationResult`, found `&str`"),
            ("architecture_module_does_not_reexport_mutation_identity",
             "use spec::architecture::MutationLane;",
             "let _ = ();",
             "no `MutationLane` in `architecture`"),
            # 5.5E3g: the corpus epoch is typed — no raw string and no
            # runtime reconciliation role substitutes for it.
            ("raw_string_cannot_substitute_for_reconciliation_epoch",
             "use spec::corpus::ReconciliationEpoch;",
             'let _: ReconciliationEpoch = "cleanroom-v1";',
             "expected `ReconciliationEpoch`, found `&str`"),
            ("reconciliation_role_cannot_substitute_for_reconciliation_epoch",
             "use spec::corpus::ReconciliationEpoch;\n"
             "use spec::reconciliation::ReconciliationRole;",
             "let _: ReconciliationEpoch = ReconciliationRole::DurableOrderWitness;",
             "expected `ReconciliationEpoch`, found `ReconciliationRole`"),
            # 5.5E3i: the promotion denominator is typed — no raw string, no
            # policy-surface substitution, and no alternate owner module.
            ("raw_string_cannot_substitute_for_promotion_requirement",
             "use spec::promotion::PromotionRequirement;",
             'let _: PromotionRequirement = "NamedProofTarget";',
             "expected `PromotionRequirement`, found `&str`"),
            ("architecture_module_does_not_reexport_promotion_requirement",
             "use spec::architecture::PromotionRequirement;",
             "let _ = ();",
             "no `PromotionRequirement` in `architecture`"),
            ("mutation_module_does_not_reexport_promotion_requirement",
             "use spec::mutation::PromotionRequirement;",
             "let _ = ();",
             "no `PromotionRequirement` in `mutation`"),
            ("proof_policy_surface_cannot_substitute_for_promotion_requirement",
             "use spec::architecture::ProofPolicySurface;\n"
             "use spec::promotion::PromotionRequirement;",
             "let _: PromotionRequirement = ProofPolicySurface::CandidatePromotion;",
             "expected `PromotionRequirement`, found `ProofPolicySurface`"),
            # 5.5E3j: the operator identity and both source surfaces are
            # typed — no raw string mints them, the word and symbol
            # inventories never substitute for one another, and a spoken
            # narration string is not a source-syntax shape.
            ("raw_string_cannot_substitute_for_operator_id",
             "use spec::operators::OperatorId;",
             'let _: OperatorId = "OP-MUL";',
             "expected `OperatorId`, found `&str`"),
            ("raw_string_cannot_substitute_for_operator_word_surface",
             "use spec::operators::OperatorWordSurface;",
             'let _: OperatorWordSurface = "IS";',
             "expected `OperatorWordSurface`, found `&str`"),
            ("raw_string_cannot_substitute_for_operator_symbol_surface",
             "use spec::operators::OperatorSymbolSurface;",
             'let _: OperatorSymbolSurface = "=";',
             "expected `OperatorSymbolSurface`, found `&str`"),
            ("operator_word_surface_cannot_substitute_for_operator_symbol_surface",
             "use spec::operators::{OperatorSymbolSurface, OperatorWordSurface};",
             "let _: OperatorSymbolSurface = OperatorWordSurface::Is;",
             "expected `OperatorSymbolSurface`, found `OperatorWordSurface`"),
            ("spoken_string_cannot_substitute_for_operator_syntax",
             "use spec::operators::OperatorSyntax;",
             'let _: OperatorSyntax = "times";',
             "expected `OperatorSyntax`, found `&str`"),
            # 5.5E4a: the generated-view registry is typed — no raw string
            # or surface value mints a view, no other module re-exports the
            # identity, and no universal GeneratedViewId exists.
            ("raw_string_cannot_substitute_for_generated_view",
             "use spec::generated_views::GeneratedView;",
             'let _: GeneratedView = "OperatorsCatalog";',
             "expected `GeneratedView`, found `&str`"),
            ("generated_view_surface_cannot_substitute_for_generated_view",
             "use spec::generated_views::{GeneratedView, GeneratedViewSurface};",
             "let _: GeneratedView = GeneratedViewSurface::EmbeddedBlock;",
             "expected `GeneratedView`, found `GeneratedViewSurface`"),
            ("architecture_module_does_not_reexport_generated_view",
             "use spec::architecture::GeneratedView;",
             "let _ = ();",
             "no `GeneratedView` in `architecture`"),
            ("universal_generated_view_id_is_absent",
             "use spec::generated_views::GeneratedViewId;",
             "let _ = ();",
             "no `GeneratedViewId` in `generated_views`"),
        ):
            src = tmp / f"{name}.rs"
            src.write_text(
                uses + f"\npub fn probe() {{\n    {body}\n}}\n", encoding="utf-8")
            proc = subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--crate-type", "lib", "--emit=metadata",
                 "--extern", f"spec={spec_meta}",
                 "--crate-name", name, "-o", str(tmp / (name + ".rmeta")), str(src)],
                capture_output=True, text=True)
            if proc.returncode == 0:
                findings.append(
                    f"{name}: production code reached a sealed, test-only, or "
                    f"fabricated spec item across the crate boundary")
            elif want not in proc.stderr:
                head = "\n".join(proc.stderr.splitlines()[:4])
                findings.append(
                    f"{name}: compile failed, but not with {want!r}:\n{head}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_toolchain(audit) -> list[str]:
    """Named hostile fixtures for the typed toolchain owner (5.5E3a).

    spec/toolchain.rs owns every value; the tracked root projection, the
    materializer, and the harness derive. Each fixture plants a hidden owner
    or a substituted dimension and must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, rel, old, new, needle, subdirs=("spec", "docs", "companion", "bootstrap")):
        with isolated_tree(subdirs=subdirs) as tmp:
            path = tmp / rel
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got = audit.toolchain_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    if audit.toolchain_findings(root):
        fail("toolchain_authority_passes_on_the_real_seed")

    # Hidden owners: a literal in the materializer or the harness is a second
    # authority the moment it exists.
    probe("materialized_resolver_mismatch_is_rejected", "bootstrap/materialize.rs",
          'resolver = \\"{}\\"',
          'resolver = \\"2\\"',
          "hardcodes a resolver literal")
    probe("workspace_msrv_mismatch_is_rejected", "bootstrap/materialize.rs",
          'rust-version = \\"{}\\"',
          'rust-version = \\"1.96\\"',
          "hardcodes a rust-version literal")
    # The planted literal is assembled at run time: written contiguously it
    # would sit in THIS file's source and the hidden-owner scan would fire on
    # its own fixture — the phantom ban catching its author again.
    probe("rustc_edition_mismatch_is_rejected", "bootstrap/selftest.py",
          '"--edition", TOOLCHAIN_EDITION,',
          '"--edition", ' + '"2021",',
          "hardcodes a rustc edition")
    # exact >= floor, never "same minor" (5.5E3a1). A NEWER qualifying
    # compiler that preserves the MSRV floor is lawful — the tracked
    # projection moves with it and NO finding fires; one BELOW the floor is
    # the contradiction. The tracked file is mutated in lockstep so only the
    # law under test can fire.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 98, patch: 0 }",
            "newer_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.98.0"',
                      "newer channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if got:
            fail(f"newer_qualifying_compiler_may_preserve_msrv (got {got!r})")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "RustRelease { major: 1, minor: 97, patch: 0 }",
            "RustRelease { major: 1, minor: 96, patch: 0 }",
            "older_qualifying_compiler"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'channel = "1.97.0"', 'channel = "1.96.0"',
                      "older channel"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("below the declared MSRV floor" in f for f in got):
            fail(f"qualifying_compiler_below_msrv_is_rejected (got {got!r})")
    # Dimension substitution, semantic side: renaming an environment spelling
    # into a triple is refused at the enum's own spelling arm.
    probe("semantic_target_shaped_like_a_triple_is_rejected", "spec/architecture.rs",
          'QualificationEnvironment::WasmHost => "wasm32 host",',
          'QualificationEnvironment::WasmHost => "wasm32-unknown-unknown",',
          "is shaped like a physical target triple")
    # RustupComponent::ALL is the one component denominator: dropping a
    # declared component from it (with the projection moving in lockstep)
    # must be refused, or Rustfmt becomes a declared-but-unconsumed variant.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        tc = tmp / "spec/toolchain.rs"
        tc.write_text(must_replace(
            tc.read_text(encoding="utf-8"),
            "&[RustupComponent::Clippy, RustupComponent::Rustfmt]",
            "&[RustupComponent::Clippy]",
            "component inventory"), encoding="utf-8")
        rt = tmp / "rust-toolchain.toml"
        rt.write_text(must_replace(rt.read_text(encoding="utf-8"),
                      'components = ["clippy", "rustfmt"]',
                      'components = ["clippy"]',
                      "component projection"), encoding="utf-8")
        got = audit.toolchain_findings(tmp)
        if not any("omitted from RustupComponent::ALL" in f for f in got):
            fail("declared_toolchain_component_cannot_be_omitted_from_"
                 f"required_inventory (got {got!r})")

    # Hand-editing or losing the tracked projection is refused.
    probe("root_toolchain_channel_mismatch_is_rejected", "rust-toolchain.toml",
          'channel = "1.97.0"', 'channel = "1.96.0"',
          "does not equal the deterministic projection")
    probe("root_toolchain_component_omission_is_rejected", "rust-toolchain.toml",
          'components = ["clippy", "rustfmt"]', 'components = ["clippy"]',
          "does not equal the deterministic projection")
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        (tmp / "rust-toolchain.toml").unlink()
        got = audit.toolchain_findings(tmp)
        if not any("absent" in f for f in got):
            fail(f"tracked_toolchain_projection_cannot_be_missing (got {got!r})")
    return findings


def test_package_identity(audit) -> list[str]:
    """Named hostile fixtures for the typed package identity (5.5E3b). The
    variant is the identity; cargo names and workspace paths are projections;
    every catalog defect must be refused for ITS law."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, old, new, needle):
        with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
            path = tmp / "spec/architecture.rs"
            path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, name),
                            encoding="utf-8")
            got: list[str] = []
            audit.check_architecture(tmp, got)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")

    clean: list[str] = []
    audit.check_architecture(root, clean)
    if clean:
        fail(f"package_catalog_passes_on_the_real_seed (got {clean!r})")

    probe("package_variant_missing_from_all_is_rejected",
          "        PackageId::NetBat,\n        PackageId::TestPak,",
          "        PackageId::TestPak,",
          "PackageId::NetBat is omitted from PackageId::ALL")
    probe("package_id_missing_from_package_specs_is_rejected",
          "    PackageSpec {\n        id: PackageId::NetBat,",
          "    PackageSpec {\n        id: PackageId::TestPak,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("package_spec_duplicate_id_is_rejected",
          "    PackageSpec {\n        id: PackageId::MacBat,",
          "    PackageSpec {\n        id: PackageId::MacBatCompiler,",
          "PACKAGES does not declare exactly PackageId::ALL in canonical order")
    probe("two_package_ids_cannot_project_the_same_cargo_name",
          'PackageId::BatQl => "batql",',
          'PackageId::BatQl => "batpak",',
          "two package identities project the same cargo name")
    probe("two_package_ids_cannot_project_the_same_workspace_path",
          'PackageId::BatQl => "crates/batql",',
          'PackageId::BatQl => "crates/batpak",',
          "two package identities project the same workspace path")

    # The materializer may not select behavior by raw package name.
    with isolated_tree(subdirs=("spec", "docs", "companion", "bootstrap")) as tmp:
        path = tmp / "bootstrap/materialize.rs"
        path.write_text(must_replace(
            path.read_text(encoding="utf-8"),
            "package.id == architecture::PackageId::BatPak",
            'package.id.cargo_name() == "batpak"',
            "raw-name selection"), encoding="utf-8")
        got: list[str] = []
        audit.check_architecture(tmp, got)
        if not any("selects behavior by raw package name" in f for f in got):
            fail(f"materializer_cannot_select_package_behavior_by_raw_name (got {got!r})")
    return findings


def test_contract_kinds(audit) -> list[str]:
    """Named hostile fixtures for the admitted contract kinds (5.5E3c). The
    enum is the admitted border crossing: a kind without an admitting law is
    a brochure entry, and the doc fence and the enum may not disagree."""
    findings: list[str] = []
    root = HERE.parent

    def fail(name: str) -> None:
        findings.append(f"{name} FAILED")

    def probe(name, edits, needle):
        tmp = gate_sandbox(edits)
        try:
            got = audit.contract_kind_findings(tmp)
            if not any(needle in f for f in got):
                fail(f"{name} (wanted {needle!r}, got {got!r})")
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    if audit.contract_kind_findings(root):
        fail("contract_kinds_pass_on_the_real_seed")

    CK = "spec/contracts.rs"
    # A kind with no admitting law is speculation wearing a variant: adding
    # StateMachine to the enum, the inventory, and the spelling map without a
    # citation is refused for exactly that absence.
    probe("speculative_contract_kind_is_rejected",
          [(CK, "    Composition,\n}", "    Composition,\n    StateMachine,\n}"),
           (CK, "        ContractKind::Composition,\n    ];",
                "        ContractKind::Composition,\n        ContractKind::StateMachine,\n    ];"),
           (CK, 'ContractKind::Composition => "Composition",',
                'ContractKind::Composition => "Composition",\n            '
                'ContractKind::StateMachine => "StateMachine",')],
          "ContractKind::StateMachine cites no admission basis")
    probe("contract_kind_missing_from_all_is_rejected",
          [(CK, "        ContractKind::Composition,\n    ];", "    ];")],
          "ContractKind::Composition is omitted from ContractKind::ALL")
    probe("docs_contract_kind_missing_row_is_rejected",
          [("docs/06_MACBAT.md", "| Composition | LEG-041 |\n", "")],
          "docs/06 omits admitted contract kind row Composition LEG-041")
    probe("docs_contract_kind_extra_row_is_rejected",
          [("docs/06_MACBAT.md", "| Composition | LEG-041 |",
            "| Composition | LEG-041 |\n| StateMachine | LEG-999 |")],
          "docs/06 admits StateMachine, which ContractKind does not declare")
    # The BASIS is semantic identity, not decoration: a wrong law, swapped
    # laws, and reordered rows are each refused positionally.
    probe("docs_contract_kind_wrong_basis_is_rejected",
          [("docs/06_MACBAT.md", "| Error | LEG-047 |", "| Error | LEG-999 |")],
          "docs/06 CONTRACT-KINDS row 1 states Error LEG-999; the typed "
          "catalog states Error LEG-047 at that position")
    probe("docs_contract_kind_swapped_bases_are_rejected",
          [("docs/06_MACBAT.md", "| Error | LEG-047 |", "| Error | LEG-002 |"),
           ("docs/06_MACBAT.md", "| Event | LEG-002 |", "| Event | LEG-047 |")],
          "docs/06 CONTRACT-KINDS row 1 states Error LEG-002; the typed "
          "catalog states Error LEG-047 at that position")
    probe("docs_contract_kind_order_drift_is_rejected",
          [("docs/06_MACBAT.md",
            "| Error | LEG-047 |\n| Event | LEG-002 |",
            "| Event | LEG-002 |\n| Error | LEG-047 |")],
          "docs/06 CONTRACT-KINDS row 1 states Event LEG-002; the typed "
          "catalog states Error LEG-047 at that position")
    probe("mistagged_admitting_guarantee_is_rejected",
          [(CK, 'ContractKind::Error => GuaranteeRef::leg("LEG-047"),',
                'ContractKind::Error => GuaranteeRef::leg("DEC-047"),')],
          "whose family owns the LEG- prefix")

    # Lifecycle law (5.5E3c1): a DELETED basis refuses the kind; a lawfully
    # CLOSED basis is retained evidence and the kind remains admitted —
    # closing the founding obligation must not erase the vocabulary it
    # earned. Process citing superseded DEC-009 on the real seed is the
    # standing green witness for the historical tier.
    probe("deleted_admission_basis_is_rejected",
          [("spec/legacy_obligations.rs",
            'LegacyObligation { id: "LEG-047",',
            'LegacyObligation { id: "LEG-047-GONE",')],
          "ContractKind::Error cites admission basis LEG-047, which no "
          "declared row owns")
    tmp = gate_sandbox([("spec/legacy_obligations.rs",
        'Planned("variant addition compile failure/mutant"), '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Active }',
        'Planned("variant addition compile failure/mutant"), '
        'gates: &[GateId::G1, GateId::G2], compatibility_disposition: '
        'CompatibilityDisposition::None, deletion_condition: '
        'DeletionCondition::OnSuccessorGateClosure, active_or_closed_status: '
        'ObligationStatus::Closed }')])
    try:
        got = audit.contract_kind_findings(tmp)
        if got:
            fail("closed_admission_basis_remains_valid_retained_evidence "
                 f"(got {got!r})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def _t0_canonical_artifact() -> str:
    """A well-formed frozen-export qualification.t0 whose digests need not match
    any bundle: every hostile below is refused at PARSE, before receiptcheck
    recomputes a single digest. The verify()-level rules (denominator, policy,
    target, source, hosted-run) are exercised directly against the sealed spec
    by seedcheck; these fixtures pin the STRICT GRAMMAR the artifact rides on."""
    d64 = {c: c * 64 for c in "abcdef012345"}
    h40 = {c: c * 40 for c in "abcd"}
    lines = [
        "BATPAK-TIER0-QUALIFICATION/2",
        "source-kind: frozen-export",
        f"spec-manifest-digest: {d64['a']}",
        f"export-tree-digest: {d64['b']}",
        "rustc-release: 1.97.0",
        f"rustc-commit: {h40['a']}",
        "cargo-release: 1.97.0",
        f"cargo-commit: {h40['b']}",
        f"toolchain-file-digest: {d64['c']}",
        "python-release: 3.12.10",
        "target: x86_64-pc-windows-gnu",
        f"receipt: tier0-law-fixtures compiled=succeeded execution=attempted "
        f"outcome=passed evidence=fixture-set digest={d64['d']}",
        f"receipt: tier0-seedcheck compiled=succeeded execution=attempted "
        f"outcome=passed evidence=executable digest={d64['e']}",
        f"receipt: tier0-materialize compiled=succeeded execution=attempted "
        f"outcome=passed evidence=executable+output-tree "
        f"executable-digest={d64['f']} output-tree-digest={d64['0']}",
        f"receipt: tier0-seedcheck-tests compiled=succeeded execution=attempted "
        f"outcome=passed evidence=executable digest={d64['1']}",
        f"receipt: tier0-spec-tests compiled=succeeded execution=attempted "
        f"outcome=passed evidence=executable digest={d64['2']}",
    ]
    return "\n".join(lines) + "\n"


def test_receiptcheck_refuses_dishonest_artifacts() -> list[str]:
    """The independent verifier refuses every artifact-grammar perturbation.

    Replaces the retired Python admission predicate (5.5E6b): there is no
    Python judgment to fool anymore. These build the REAL receiptcheck and feed
    it perturbations of a canonical qualification.t0, asserting each is refused
    for its exact reason. Grammar refusals fire before any digest recompute, so
    no evidence bundle is needed; the digest-mismatch and typed verify() rules
    are proven by the producer round-trip and by seedcheck respectively."""
    findings: list[str] = []
    root = HERE.parent
    # Evidence independence (5.5E6b): the verifier computes its own digests and
    # defers to no producer. It carries a self-checked internal SHA-256 and
    # shells out to no python — its lone "selftest.py" mention is a comment
    # asserting exactly this, not a call.
    rc_src = (root / "bootstrap/receiptcheck.rs").read_text(encoding="utf-8")
    if "fn sha256(" not in rc_src or "self_check_sha256" not in rc_src:
        findings.append("receiptcheck_computes_its_own_sha256 FAILED "
                        "(no self-checked internal hasher)")
    if re.search(r'Command::new\(\s*"python', rc_src) or "import selftest" in rc_src:
        findings.append("receiptcheck_does_not_defer_to_the_producer FAILED "
                        "(shells out to or imports selftest)")
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: receiptcheck_refuses_dishonest_artifacts unavailable (no rustc)")
        return findings
    tmp = Path(tempfile.mkdtemp(prefix="batpak-hostile-t0-"))
    try:
        target = _t0_working_exe_target(rustc, TOOLCHAIN_EDITION, tmp)
        rlib = tmp / "libspec.rlib"
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
                 "--crate-type", "rlib", "--crate-name", "spec", "-o", str(rlib),
                 str(root / "spec/lib.rs")], capture_output=True, text=True).returncode != 0:
            print("selftest: receiptcheck_refuses_dishonest_artifacts unavailable "
                  "(spec rlib did not build)")
            return findings
        rc = tmp / ("receiptcheck" + (".exe" if os.name == "nt" else ""))
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
                 "--crate-name", "receiptcheck", "--extern", f"spec={rlib}",
                 "-o", str(rc), str(root / "bootstrap/receiptcheck.rs")],
                capture_output=True, text=True).returncode != 0 or not rc.is_file():
            print("selftest: receiptcheck_refuses_dishonest_artifacts unavailable "
                  "(receiptcheck did not build)")
            return findings

        good = _t0_canonical_artifact()

        def refuses(name: str, text, needle: str) -> None:
            # The artifact must be the bundle's own qualification.t0 (5.5E6c2), so
            # each perturbation is written there; the grammar refusal fires before
            # any digest recompute or bundle-shape check.
            art = tmp / "qualification.t0"
            art.write_bytes(text.encode("utf-8") if isinstance(text, str) else text)
            proc = subprocess.run([str(rc), "verify", str(art), "--root", str(tmp),
                                   "--evidence", str(tmp),
                                   "--python-executable", sys.executable],
                                  capture_output=True, text=True)
            out = (proc.stdout + proc.stderr)
            if proc.returncode == 0:
                findings.append(f"receiptcheck_{name} FAILED (artifact was admitted)")
            elif needle not in out:
                findings.append(f"receiptcheck_{name} FAILED "
                                f"(wanted {needle!r}, got {out.strip()[:160]!r})")

        # Strict byte grammar.
        refuses("crlf_is_rejected", good.replace("\n", "\r\n", 1),
                "artifact contains a CR byte")
        refuses("missing_final_newline_is_rejected", good[:-1],
                "artifact has no final newline")
        refuses("non_ascii_byte_is_rejected", good.replace("frozen-export", "frozen-expørt"),
                "artifact contains a non-ASCII byte")
        refuses("trailing_whitespace_is_rejected",
                good.replace("target: x86_64-pc-windows-gnu\n",
                             "target: x86_64-pc-windows-gnu \n"),
                "artifact line has trailing whitespace")
        # Fixed header and section order. The retired v1 magic cannot satisfy the
        # v2 builder-binding format (5.5E6c2).
        refuses("qualification_v1_cannot_satisfy_v2_builder_binding",
                good.replace("BATPAK-TIER0-QUALIFICATION/2",
                             "BATPAK-TIER0-QUALIFICATION/1", 1),
                "expected line")
        # A noncanonical release spelling that merely parses numerically is refused.
        refuses("noncanonical_release_spelling_is_rejected",
                good.replace("rustc-release: 1.97.0", "rustc-release: 1.097.0", 1),
                "is not canonical")
        # The bootstrap Python runtime must be bound.
        refuses("python_runtime_must_be_bound",
                good.replace("python-release: 3.12.10\n", "", 1),
                "expected key")
        refuses("reordered_source_keys_are_rejected",
                good.replace(
                    f"spec-manifest-digest: {'a' * 64}\nexport-tree-digest: {'b' * 64}",
                    f"export-tree-digest: {'b' * 64}\nspec-manifest-digest: {'a' * 64}"),
                "expected key")
        refuses("unknown_key_is_rejected",
                good.replace("target: x86_64-pc-windows-gnu\n",
                             "target: x86_64-pc-windows-gnu\nbogus-key: 1\n"),
                "unexpected line in receipts block")
        # Strict lowercase, full-length hex.
        refuses("uppercase_hex_is_rejected",
                good.replace(f"spec-manifest-digest: {'a' * 64}",
                             f"spec-manifest-digest: {'A' + 'a' * 63}"),
                "bad sha256 digest")
        refuses("abbreviated_hex_is_rejected",
                good.replace(f"toolchain-file-digest: {'c' * 64}",
                             "toolchain-file-digest: cccc"),
                "bad sha256 digest")
        # Receipt tokens: only known slugs, no duplicate keys. A product
        # ReceiptId is not a Tier0ReceiptKind slug (5.5E6b).
        refuses("unknown_receipt_slug_is_rejected",
                good.replace("receipt: tier0-seedcheck ", "receipt: tier0-bogus "),
                "unknown receipt slug")
        refuses("duplicate_receipt_token_is_rejected",
                good.replace("receipt: tier0-seedcheck compiled=succeeded",
                             "receipt: tier0-seedcheck compiled=succeeded compiled=succeeded"),
                "duplicate receipt token key")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return findings


def test_receiptcheck_bundle_perimeter() -> list[str]:
    """The verifier enforces the EXACT evidence-bundle shape (5.5E6c2): a real GNU
    bundle verifies, but a stray PDB/scratch file in executables/, an unmanifested
    top-level file, a supplemental bundle carrying run-metadata, and an external
    artifact substituted for the bundle's own qualification.t0 are all refused."""
    findings: list[str] = []
    root = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: receiptcheck_bundle_perimeter unavailable (no rustc)")
        return findings
    work = Path(tempfile.mkdtemp(prefix="batpak-bundle-perimeter-"))
    try:
        bundle = work / "tier0-evidence"
        _artifact, source_root, problems = produce_tier0_evidence(bundle, _T0_GNU)
        if problems:
            print("selftest: receiptcheck_bundle_perimeter unavailable "
                  f"(GNU evidence incomplete: {problems[0]})")
            return findings
        wrlib = work / "libspec.rlib"
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_GNU,
                 "--crate-type", "rlib", "--crate-name", "spec", "-o", str(wrlib),
                 str(root / "spec/lib.rs")], capture_output=True, text=True).returncode != 0:
            print("selftest: receiptcheck_bundle_perimeter unavailable (spec rlib)")
            return findings
        rc = work / ("receiptcheck" + (".exe" if os.name == "nt" else ""))
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_GNU,
                 "--crate-name", "receiptcheck", "--extern", f"spec={wrlib}",
                 "-o", str(rc), str(root / "bootstrap/receiptcheck.rs")],
                capture_output=True, text=True).returncode != 0 or not rc.is_file():
            print("selftest: receiptcheck_bundle_perimeter unavailable (receiptcheck)")
            return findings

        def verify_bundle():
            proc = subprocess.run(
                [str(rc), "verify", str(bundle / "qualification.t0"),
                 "--root", str(source_root), "--evidence", str(bundle),
                 "--python-executable", sys.executable], capture_output=True, text=True)
            return proc.returncode, (proc.stdout + proc.stderr)

        rcode, out = verify_bundle()
        if rcode != 0:
            findings.append(f"receiptcheck_honest_gnu_bundle_verifies FAILED ({out.strip()[:200]})")

        def refuses_bundle(name, make, unmake, needle):
            make()
            try:
                rcode, out = verify_bundle()
                if rcode == 0:
                    findings.append(f"receiptcheck_{name} FAILED (bundle was admitted)")
                elif needle not in out:
                    findings.append(f"receiptcheck_{name} FAILED "
                                    f"(wanted {needle!r}, got {out.strip()[:200]!r})")
            finally:
                unmake()

        exe_dir = bundle / "executables"
        pdb = exe_dir / "tier0-seedcheck.pdb"
        refuses_bundle("pdb_cannot_enter_tier0_evidence_bundle",
                       lambda: pdb.write_bytes(b"stray"),
                       lambda: pdb.unlink(missing_ok=True),
                       "unmanifested file in executables/")
        lib = exe_dir / "tier0-seedcheck.lib"
        refuses_bundle("compiler_scratch_output_cannot_enter_evidence_bundle",
                       lambda: lib.write_bytes(b"scratch"),
                       lambda: lib.unlink(missing_ok=True),
                       "unmanifested file in executables/")
        top = bundle / "extra.txt"
        refuses_bundle("unmanifested_evidence_file_is_rejected",
                       lambda: top.write_bytes(b"extra"),
                       lambda: top.unlink(missing_ok=True),
                       "unmanifested file in evidence bundle")
        meta = bundle / "run-metadata.t0"
        refuses_bundle("local_bundle_carrying_run_metadata",
                       lambda: meta.write_bytes(b"BATPAK-TIER0-RUN-METADATA/1\n"),
                       lambda: meta.unlink(missing_ok=True),
                       "must not carry run-metadata.t0")

        other = work / "elsewhere"
        other.mkdir()
        (other / "qualification.t0").write_bytes((bundle / "qualification.t0").read_bytes())
        proc = subprocess.run(
            [str(rc), "verify", str(other / "qualification.t0"),
             "--root", str(source_root), "--evidence", str(bundle),
             "--python-executable", sys.executable], capture_output=True, text=True)
        eout = proc.stdout + proc.stderr
        if proc.returncode == 0:
            findings.append("receiptcheck_external_qualification_artifact_cannot_"
                            "substitute_for_bundle_artifact FAILED (admitted)")
        elif "is not the bundle's own qualification.t0" not in eout:
            findings.append("receiptcheck_external_qualification_artifact_cannot_"
                            "substitute_for_bundle_artifact FAILED "
                            f"(got {eout.strip()[:200]!r})")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return findings


def test_verification_plane() -> list[str]:
    """audit.verification_findings: the frozen axes, the shape-based
    anti-ladder guard, tool-neutral methods, plan admissibility, route kinds
    closed by adoption, the runtime matrix, and the retired counts_green
    spelling (5.5F2). Each hostile mutates a sandbox copy of the real spec and
    expects its detector to fire; the unmutated copy must yield zero findings
    so no detector over-fires."""
    findings: list[str] = []
    audit = load("audit")
    root = HERE.parent
    base = Path(tempfile.mkdtemp(prefix="batpak-verif-"))
    try:
        (base / "spec").mkdir()
        (base / "bootstrap").mkdir()
        for rel in sorted((root / "spec").glob("*.rs")):
            shutil.copyfile(rel, base / "spec" / rel.name)
        for rel in ("bootstrap/seedcheck.rs", "bootstrap/audit.py"):
            shutil.copyfile(root / rel, base / rel)
        clean = audit.verification_findings(base)
        if clean:
            findings.append(f"verification_plane baseline is not clean: {clean[:3]}")

        vpath = base / "spec" / "verification.rs"
        ppath = base / "spec" / "proof.rs"
        vsrc = vpath.read_text(encoding="utf-8")
        psrc = ppath.read_text(encoding="utf-8")

        def expect(name: str, needle: str) -> None:
            got = audit.verification_findings(base)
            if not any(needle in f for f in got):
                findings.append(f"{name} FAILED (no finding containing {needle!r})")
            vpath.write_text(vsrc, encoding="utf-8", newline="\n")
            ppath.write_text(psrc, encoding="utf-8", newline="\n")

        def mutate(path: Path, source: str, old: str, new: str, what: str) -> bool:
            if old not in source:
                findings.append(f"{what}: probe target absent, mutation never applied")
                return False
            path.write_text(source.replace(old, new, 1), encoding="utf-8", newline="\n")
            return True

        if mutate(vpath, vsrc, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum VerificationCoverage",
                  "#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]\npub enum VerificationCoverage",
                  "ordered_coverage"):
            expect("verification_axis_ordering_is_rejected", "derives an ordering")
        vpath.write_text(vsrc + "\npub fn score(_x: bool) {}\n", encoding="utf-8", newline="\n")
        expect("verification_score_fn_is_rejected", "rank/level/score")
        vpath.write_text(
            vsrc + "\npub fn strength(c: VerificationCoverage) -> u8 { c as u8 }\n",
            encoding="utf-8", newline="\n")
        expect("verification_numeric_conversion_is_rejected", "numerically converts")
        vpath.write_text(vsrc + "\npub enum VerificationToolId { Placeholder }\n",
                         encoding="utf-8", newline="\n")
        expect("verification_tool_vocabulary_is_rejected", "tool vocabulary VerificationToolId")
        vpath.write_text(vsrc + "\nuse crate::tier0_cross_run as _t0;\n",
                         encoding="utf-8", newline="\n")
        expect("tier0_import_into_verification_is_rejected", "never product verification authority")
        if mutate(vpath, vsrc, "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n}",
                  "    ExhaustiveWithinDeclaredModel,\n    ObservedHistory,\n    Extra,\n}",
                  "extra_coverage_variant"):
            expect("verification_axis_variant_drift_is_rejected", "!= frozen")
        if mutate(vpath, vsrc,
                  "D::NoDivergenceObserved | D::Divergent | D::Incomplete | D::Stale | D::Unsupported",
                  "D::NoDivergenceObserved | D::ConformantForObservedHistory | D::Divergent | D::Incomplete | D::Stale | D::Unsupported",
                  "inband_conformant"):
            expect("in_band_conformance_is_rejected", "only independent offline replay")
        ppath.write_text(
            psrc + "\n// fn _retired() { let _ = ProofUnitTerminal::Passed.counts_green(); }\n",
            encoding="utf-8", newline="\n")
        expect("counts_green_reintroduction_is_rejected", "still calls counts_green")
        hostile_plan = ("pub const PLAN_HOSTILE_BOUNDARY: &[VerificationRequirement] = "
                        "&[VerificationRequirement { method: VerificationMethod::PropertySequence, "
                        "basis: VerificationBasis::DirectBoundary")
        if mutate(ppath, psrc, hostile_plan,
                  hostile_plan.replace("VerificationBasis::DirectBoundary",
                                       "VerificationBasis::ContractProjection"),
                  "projection_route_plan"):
            expect("projection_supplied_route_in_plan_is_rejected",
                   "ContractProjection supply an ")
        if mutate(ppath, psrc,
                  "independent_route: Some(IndependentEvidenceRouteKind::DifferentialImplementation) }];",
                  "independent_route: None }];", "orphan_differential_route"):
            expect("route_kind_without_adopter_is_rejected",
                   "IndependentEvidenceRouteKind::DifferentialImplementation has no active")
        row = ('ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), state: '
               'ProofRowState::Active { guarantee: GuaranteeRef::leg("LEG-023"), '
               'projection_contracts: &[ContractId("BP-STORAGE-TILES-1")], '
               'claim: VerificationClaimKind::Safety, verification: PLAN_HOSTILE_BOUNDARY } }')
        if mutate(ppath, psrc, row,
                  row.replace("VerificationClaimKind::Safety", "VerificationClaimKind::Bogus"),
                  "bogus_claim"):
            expect("unknown_claim_kind_is_rejected", "claims unknown kind Bogus")
        if mutate(ppath, psrc, row,
                  row.replace("verification: PLAN_HOSTILE_BOUNDARY",
                              "verification: PLAN_NONEXISTENT"),
                  "undeclared_plan"):
            expect("undeclared_plan_reference_is_rejected", "undeclared plan PLAN_NONEXISTENT")
    finally:
        shutil.rmtree(base, ignore_errors=True)
    return findings


def test_workflow_pinning() -> list[str]:
    """audit.workflow_pinning_findings refuses movable/abbreviated/docker-untagged
    action references and accepts local `./` actions and full-SHA pins (5.5E6c2)."""
    findings: list[str] = []
    audit = load("audit")
    work = Path(tempfile.mkdtemp(prefix="batpak-wf-pin-"))
    try:
        wf = work / ".github" / "workflows"
        wf.mkdir(parents=True)
        sha = "3" * 40

        def one(uses: str) -> bool:
            (wf / "x.yml").write_text(
                "on: push\njobs:\n  j:\n    runs-on: ubuntu-latest\n    steps:\n"
                f"      - uses: {uses}\n", encoding="utf-8", newline="\n")
            return any("x.yml" in f for f in audit.workflow_pinning_findings(work))

        cases = [
            (f"actions/checkout@{sha}", False),
            ("actions/checkout@v4", True),
            ("actions/checkout@main", True),
            (f"actions/checkout@{'3' * 10}", True),
            ("./.github/actions/local", False),
            ("docker://alpine:3.14", True),
            (f"docker://alpine@sha256:{'a' * 64}", False),
        ]
        for uses, want in cases:
            got = one(uses)
            if got != want:
                findings.append(f"workflow_pinning {uses!r}: expected finding={want}, got {got}")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return findings


# === 5.5E6b: Tier 0 evidence producer ======================================
# selftest is the OBSERVATION PRODUCER, not the admission authority. It runs
# the real Tier 0 gates against a clean export, retains the concrete evidence
# (executables, the materialized candidate tree, a fixture ledger), writes the
# canonical qualification.t0 artifact, and hands the whole thing to the
# independent verifier bootstrap/receiptcheck.rs. receiptcheck recomputes every
# digest from the bytes on disk and calls the sealed spec::verify. A dishonest
# receipt is caught THERE, not by a Python predicate here. The tree-digest
# algorithm below mirrors receiptcheck.rs collect_tree/tree_digest exactly;
# the artifact grammar mirrors its strict parser. Both are implemented
# independently and must agree byte for byte.

_T0_GNU = "x86_64-pc-windows-gnu"
_T0_MSVC = "x86_64-pc-windows-msvc"
_T0_SLUGS = ("tier0-law-fixtures", "tier0-seedcheck", "tier0-materialize",
             "tier0-seedcheck-tests", "tier0-spec-tests")
_T0_WORKFLOW_PATH = ".github/workflows/msvc-qualification.yml"


def _t0_tree_digest(root: Path) -> str:
    """Deterministic tree digest matching receiptcheck.rs exactly: one row per
    entry ('<rel>\\tdir\\t-' or '<rel>\\tfile\\t<sha256hex>'), rel with '/'
    separators relative to root, the root itself excluded; sort the full rows,
    join each followed by '\\n', sha256 the join."""
    rows: list[str] = []
    for path in root.rglob("*"):
        rel = path.relative_to(root).as_posix()
        if path.is_symlink() or (not path.is_dir() and not path.is_file()):
            raise RuntimeError(f"tree entry {rel} is neither file nor dir")
        if path.is_dir():
            rows.append(f"{rel}\tdir\t-")
        else:
            rows.append(f"{rel}\tfile\t{hashlib.sha256(path.read_bytes()).hexdigest()}")
    rows.sort()
    return hashlib.sha256("".join(r + "\n" for r in rows).encode("utf-8")).hexdigest()


def _t0_probe(tool: str) -> tuple[str, str]:
    """The default toolchain's release and commit hash, parsed the way
    receiptcheck's probe_tool does — so the artifact this producer writes and
    the verifier that re-probes agree. Raises when the tool reports no usable
    commit (a channel with no commit-hash cannot produce authoritative
    evidence locally; hosted CI owns that lane)."""
    args = ["-vV"] if tool == "rustc" else ["-Vv"]
    out = subprocess.run([tool, *args], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(f"{tool} {args} exited nonzero")
    release = commit = None
    for line in out.stdout.splitlines():
        if line.startswith("release: "):
            release = line[len("release: "):].strip()
        if line.startswith("commit-hash: "):
            commit = line[len("commit-hash: "):].strip()
    if not release or not commit or not re.fullmatch(r"[0-9a-f]{40}", commit or ""):
        raise RuntimeError(f"{tool} reported no usable release/commit "
                           f"(release={release!r} commit={commit!r})")
    return release, commit


def _t0_working_exe_target(rustc, edition, work: Path) -> str:
    """A target that builds AND links a runnable exe on THIS machine: the host
    default when its linker works (the hosted MSVC runner), else the gnu target
    (local dev whose MSVC linker is absent)."""
    probe = work / "t0probe.rs"
    probe.write_text("fn main() {}\n", encoding="utf-8")
    for target in (None, _T0_GNU):
        exe = work / ("t0probe" + (".exe" if os.name == "nt" else ""))
        if exe.exists():
            exe.unlink()
        flags = ["--target", target] if target else []
        if subprocess.run([rustc, "--edition", edition, *flags, "-o", str(exe),
                           str(probe)], capture_output=True, text=True).returncode == 0 \
                and exe.is_file():
            if target:
                return target
            m = re.search(r"host: (\S+)", subprocess.run(
                [rustc, "-vV"], capture_output=True, text=True).stdout)
            return m.group(1) if m else _T0_GNU
    return _T0_GNU


def _t0_build(rustc, edition, target, rlib, src: Path, out_exe: Path,
              extra: tuple = (), link_spec: bool = True) -> bool:
    """Build one binary for `target`. The rlib name MUST be lib*.rlib for rustc
    to accept it as an extern crate."""
    cmd = [rustc, "--edition", edition, "--target", target,
           "--crate-name", out_exe.stem.replace("-", "_")]
    if link_spec:
        cmd += ["--extern", f"spec={rlib}"]
    cmd += [*extra, "-o", str(out_exe), str(src)]
    return subprocess.run(cmd, capture_output=True, text=True).returncode == 0 \
        and out_exe.is_file()


def _t0_law_fixtures(rustc, edition, target, rlib, work: Path) -> tuple[str, list[str]]:
    """Run compile-fail law fixtures through the real toolchain and record a
    ledger. Each fixture is a hostile consumer that MUST fail to compile with
    its own reason; a fixture that compiles is a hole in the boundary. Returns
    (manifest_text, problems)."""
    fixtures = (
        ("guarantee_identity_cannot_be_minted",
         "use spec::guarantees::{GuaranteeRef, SeedId};\n"
         "fn main() { let _ = GuaranteeRef::Seed(SeedId(\"SEED-FORGED\")); }",
         "cannot initialize a tuple struct which contains private fields"),
        ("qualification_id_cannot_carry_a_string_package",
         "use spec::guarantees::QualificationId;\n"
         "fn main() { let _ = QualificationId { package: \"batpak\", profile: \"semantic\" }; }",
         "expected `PackageId`, found `&str`"),
    )
    rows: list[str] = []
    problems: list[str] = []
    for identity, body, want in fixtures:
        srcf = work / f"{identity}.rs"
        srcf.write_text(body + "\n", encoding="utf-8")
        proc = subprocess.run(
            [rustc, "--edition", edition, "--target", target,
             "--extern", f"spec={rlib}", "--emit=metadata",
             "-o", str(work / f"{identity}.rmeta"), str(srcf)],
            capture_output=True, text=True)
        diag = proc.stdout + proc.stderr
        observed = "refused" if proc.returncode != 0 else "ADMITTED"
        if proc.returncode == 0:
            problems.append(f"law-fixture {identity} compiled; the boundary is open")
        elif want not in diag:
            problems.append(f"law-fixture {identity} failed for the wrong reason")
        diag_digest = hashlib.sha256(diag.encode("utf-8")).hexdigest()
        rows.append(f"{identity} | compile-fail | refused | {observed} | {diag_digest} | -")
    return "\n".join(rows) + "\n", problems


def _t0_gate_exe(rustc, edition, target, rlib, source_root: Path, bundle: Path,
                 slug: str, src_rel: str, expect: str, extra: tuple = (),
                 run_args=None, link_spec: bool = True) -> tuple[str | None, list[str]]:
    """Build one executable gate in the WORK dir, run it, then COPY only the
    admitted executable into the evidence bundle (5.5E6c2) — the evidence dir is
    never a compiler scratch dir, so link residue (`.pdb`/`.lib`) never enters it.
    The exit code and the expected banner must agree."""
    suffix = ".exe" if os.name == "nt" else ""
    built = bundle.parent / "t0-work" / (slug + suffix)
    if not _t0_build(rustc, edition, target, rlib, source_root / src_rel, built,
                     extra, link_spec):
        return None, [f"{slug}: did not build for {target}"]
    proc = subprocess.run(
        [str(built), *(run_args if run_args is not None else (str(source_root),))],
        capture_output=True, text=True)
    out = proc.stdout + proc.stderr
    if proc.returncode != 0 or expect not in out:
        head = " / ".join(out.splitlines()[:6])
        return None, [f"{slug}: executed and did not pass ({head})"]
    dest = bundle / "executables" / (slug + suffix)
    shutil.copyfile(built, dest)
    digest = hashlib.sha256(dest.read_bytes()).hexdigest()
    return f"evidence=executable digest={digest}", []


def _t0_git(root: Path, rev: str) -> str | None:
    """A 40-hex object id from `git rev-parse`, or None."""
    p = subprocess.run(["git", "-C", str(root), "rev-parse", rev],
                       capture_output=True, text=True)
    out = p.stdout.strip()
    return out if p.returncode == 0 and re.fullmatch(r"[0-9a-f]{40}", out) else None


def _t0_hosted_run_lines() -> list[str] | None:
    """The hosted GitHub Actions run block from the environment, or None when
    not under Actions: a local machine cannot mint an authoritative hosted
    receipt. Values are single ASCII tokens with no trailing whitespace."""
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT", "")
    os_name = (os.environ.get("ImageOS") or os.environ.get("RUNNER_OS", "")).strip()
    os_version = (os.environ.get("ImageVersion")
                  or os.environ.get("RUNNER_ARCH", "")).strip()
    if not (repo and run_id.isdigit() and run_attempt.isdigit()
            and os_name and os_version):
        return None
    return [
        f"github-repository: {repo}",
        f"github-workflow-path: {_T0_WORKFLOW_PATH}",
        f"github-run-id: {run_id}",
        f"github-run-attempt: {run_attempt}",
        f"runner-image-os: {os_name}",
        f"runner-image-version: {os_version}",
    ]


def produce_tier0_evidence(bundle: Path,
                           target: str) -> tuple[Path | None, Path | None, list[str]]:
    """Run every Tier 0 gate for `target`, materialize the Gate-0 candidate, and
    write the canonical qualification.t0 + evidence bundle. The authoritative
    MSVC target rides a git-checkout source with a hosted-run binding; the
    supplemental GNU target rides a clean frozen `git archive` export. Returns
    (artifact, source_root, problems); a nonempty problems list means the
    evidence is incomplete and no artifact should be trusted."""
    repo = HERE.parent
    rustc = shutil.which("rustc")
    if not rustc:
        return None, None, ["rustc absent; no Tier 0 evidence can be produced"]
    edition = TOOLCHAIN_EDITION
    problems: list[str] = []
    authoritative = (target == _T0_MSVC)

    if bundle.exists():
        shutil.rmtree(bundle, ignore_errors=True)
    (bundle / "executables").mkdir(parents=True)
    work = bundle.parent / "t0-work"
    if work.exists():
        shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)

    # Source root and posture. The authoritative lane qualifies the real git
    # checkout (binding its commit, tree, and hosted-run identity); the
    # supplemental lane qualifies a clean export that carries no .git.
    hosted: list[str] | None = None
    if authoritative:
        source_root = repo
        commit = _t0_git(repo, "HEAD")
        tree = _t0_git(repo, "HEAD^{tree}")
        hosted = _t0_hosted_run_lines()
        wf = repo / _T0_WORKFLOW_PATH
        if not (commit and tree):
            return None, None, ["authoritative target requires a git checkout; "
                                "no HEAD commit/tree found"]
        if hosted is None:
            return None, None, ["authoritative target requires a hosted GitHub "
                                "Actions run; no runner environment found"]
        if not wf.is_file():
            return None, None, [f"authoritative target requires {_T0_WORKFLOW_PATH}"]
        source_lines = [
            "source-kind: git-checkout",
            f"git-commit: {commit}",
            f"git-tree: {tree}",
            "spec-manifest-digest: "
            + hashlib.sha256((repo / "SPEC.sha256").read_bytes()).hexdigest(),
            f"workflow-path: {_T0_WORKFLOW_PATH}",
            f"workflow-digest: {hashlib.sha256(wf.read_bytes()).hexdigest()}",
        ]
    else:
        source_root = bundle.parent / "t0-export"
        if source_root.exists():
            shutil.rmtree(source_root, ignore_errors=True)
        source_root.mkdir(parents=True)
        archive = subprocess.run(["git", "archive", "HEAD"], cwd=repo, capture_output=True)
        if archive.returncode != 0:
            return None, None, ["git archive HEAD failed; not a clean checkout"]
        if subprocess.run(["tar", "-x", "-C", str(source_root)], input=archive.stdout,
                          capture_output=True).returncode != 0:
            return None, None, ["tar extraction of the export failed"]
        source_lines = [
            "source-kind: frozen-export",
            "spec-manifest-digest: "
            + hashlib.sha256((source_root / "SPEC.sha256").read_bytes()).hexdigest(),
            f"export-tree-digest: {_t0_tree_digest(source_root)}",
        ]

    # The spec rlib for the target, built once from the source root.
    rlib = work / "libspec.rlib"
    if subprocess.run(
            [rustc, "--edition", edition, "--target", target, "--crate-type",
             "rlib", "--crate-name", "spec", "-o", str(rlib),
             str(source_root / "spec/lib.rs")],
            capture_output=True, text=True).returncode != 0:
        return None, source_root, [f"the spec rlib did not build for {target}"]

    # Executable gates.
    tails: dict[str, str] = {}
    tail, probs = _t0_gate_exe(rustc, edition, target, rlib, source_root, bundle,
                               "tier0-seedcheck", "bootstrap/seedcheck.rs",
                               "seedcheck: PASS")
    problems += probs
    if tail:
        tails["tier0-seedcheck"] = tail
    n_sc = (source_root / "bootstrap/seedcheck.rs").read_text(
        encoding="utf-8").count("#[test]")
    tail, probs = _t0_gate_exe(rustc, edition, target, rlib, source_root, bundle,
                               "tier0-seedcheck-tests", "bootstrap/seedcheck.rs",
                               f"test result: ok. {n_sc} passed",
                               extra=("--test",), run_args=())
    problems += probs
    if tail:
        tails["tier0-seedcheck-tests"] = tail
    n_spec = sum(p.read_text(encoding="utf-8").count("#[test]")
                 for p in sorted((source_root / "spec").glob("*.rs")))
    tail, probs = _t0_gate_exe(rustc, edition, target, rlib, source_root, bundle,
                               "tier0-spec-tests", "spec/lib.rs",
                               f"test result: ok. {n_spec} passed",
                               extra=("--test",), run_args=(), link_spec=False)
    problems += probs
    if tail:
        tails["tier0-spec-tests"] = tail

    # Materializer: build in WORK, run into the candidate tree, copy only the
    # admitted executable into the bundle, digest the copied exe and the tree.
    mat_suffix = ".exe" if os.name == "nt" else ""
    mat_built = work / ("tier0-materialize" + mat_suffix)
    mat_exe = bundle / "executables" / ("tier0-materialize" + mat_suffix)
    candidate = bundle / "gate0-candidate"
    if not _t0_build(rustc, edition, target, rlib,
                     source_root / "bootstrap/materialize.rs", mat_built):
        problems.append(f"tier0-materialize: did not build for {target}")
    else:
        mp = subprocess.run([str(mat_built), "--seed", str(source_root),
                             "--output", str(candidate)], capture_output=True, text=True)
        if mp.returncode != 0 or not candidate.is_dir():
            problems.append("tier0-materialize: did not materialize the candidate "
                            f"({mp.stderr.strip()[:160]})")
        else:
            shutil.copyfile(mat_built, mat_exe)
            ed = hashlib.sha256(mat_exe.read_bytes()).hexdigest()
            otd = _t0_tree_digest(candidate)
            tails["tier0-materialize"] = (f"evidence=executable+output-tree "
                                          f"executable-digest={ed} output-tree-digest={otd}")

    # Law fixtures: compile-fail ledger.
    manifest_text, fx_probs = _t0_law_fixtures(rustc, edition, target, rlib, work)
    problems += fx_probs
    (bundle / "law-fixtures.manifest").write_text(manifest_text, encoding="utf-8",
                                                  newline="\n")
    tails["tier0-law-fixtures"] = ("evidence=fixture-set digest="
                                   + hashlib.sha256(
                                       (bundle / "law-fixtures.manifest").read_bytes()).hexdigest())

    if problems:
        return None, source_root, problems

    # Toolchain bindings, then the artifact.
    try:
        rustc_rel, rustc_commit = _t0_probe("rustc")
        cargo_rel, cargo_commit = _t0_probe("cargo")
    except RuntimeError as exc:
        return None, source_root, [str(exc)]
    toolchain_file_digest = hashlib.sha256(
        (source_root / "rust-toolchain.toml").read_bytes()).hexdigest()
    python_rel = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    lines = ["BATPAK-TIER0-QUALIFICATION/2", *source_lines,
             f"rustc-release: {rustc_rel}", f"rustc-commit: {rustc_commit}",
             f"cargo-release: {cargo_rel}", f"cargo-commit: {cargo_commit}",
             f"toolchain-file-digest: {toolchain_file_digest}",
             f"python-release: {python_rel}",
             f"target: {target}"]
    if hosted:
        lines += hosted
    for slug in _T0_SLUGS:
        lines.append(f"receipt: {slug} compiled=succeeded execution=attempted "
                     f"outcome=passed {tails[slug]}")
    artifact = bundle / "qualification.t0"
    artifact.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return artifact, source_root, []


def verify_tier0_evidence(bundle: Path, artifact: Path, source_root: Path,
                          target: str) -> tuple[bool, str]:
    """Compile bootstrap/receiptcheck.rs and run it as the AUTHORITATIVE
    verifier over the produced evidence. selftest never re-implements the
    judgment; it delegates to the independent computer."""
    rustc = shutil.which("rustc")
    if not rustc:
        return False, "rustc absent; receiptcheck cannot run"
    work = bundle.parent / "t0-work"
    rlib = work / "libspec.rlib"
    rc = work / ("receiptcheck" + (".exe" if os.name == "nt" else ""))
    if subprocess.run(
            [rustc, "--edition", TOOLCHAIN_EDITION, "--target", target,
             "--crate-name", "receiptcheck", "--extern", f"spec={rlib}",
             "-o", str(rc), str(source_root / "bootstrap/receiptcheck.rs")],
            capture_output=True, text=True).returncode != 0 or not rc.is_file():
        return False, "receiptcheck did not build"
    proc = subprocess.run([str(rc), "verify", str(artifact), "--root", str(source_root),
                           "--evidence", str(bundle),
                           "--python-executable", sys.executable],
                          capture_output=True, text=True)
    return proc.returncode == 0, (proc.stdout + proc.stderr).strip()


def _emit_run_metadata(path: str, pairs: list[str]) -> int:
    """Write a strict BATPAK-TIER0-RUN-METADATA/1 file from key=value pairs
    (5.5E6c1). The confirming workflow supplies the fields — from the runner
    environment for its own run, from the GitHub Actions API for the candidate
    run — and Python controls the exact LF/ASCII bytes receiptcheck's strict
    reader demands. No field becomes source identity: it is the run's OWN record
    that `receiptcheck compare` ties each uploaded bundle to."""
    order = ["conclusion", "head-sha", "repository", "workflow-path", "run-id", "run-attempt"]
    got: dict[str, str] = {}
    for pair in pairs:
        if "=" not in pair:
            print(f"selftest: run-metadata field {pair!r} is not key=value", file=sys.stderr)
            return 2
        key, value = pair.split("=", 1)
        if key not in order:
            print(f"selftest: unknown run-metadata field {key!r}", file=sys.stderr)
            return 2
        if key in got:
            print(f"selftest: duplicate run-metadata field {key!r}", file=sys.stderr)
            return 2
        if not value:
            print(f"selftest: run-metadata field {key!r} is empty", file=sys.stderr)
            return 2
        got[key] = value
    missing = [key for key in order if key not in got]
    if missing:
        print(f"selftest: run-metadata missing fields {missing}", file=sys.stderr)
        return 2
    lines = ["BATPAK-TIER0-RUN-METADATA/1"] + [f"{key}: {got[key]}" for key in order]
    Path(path).write_text("\n".join(lines) + "\n", encoding="ascii", newline="\n")
    print(f"selftest: wrote run metadata ({path})")
    return 0


def _confirm_promotion(candidate_bundle: str, candidate_meta: str,
                       cleanroom_bundle: str, cleanroom_meta: str,
                       root: str) -> int:
    """Build receiptcheck and run `compare --require-promotion-confirmation` over
    two verified bundles and their run-record metadata (5.5E6c1). selftest builds
    and invokes; receiptcheck independently recomputes both bundles and COMPUTES
    the comparison — Python never parses or duplicates the comparator law."""
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: rustc absent; cannot confirm promotion", file=sys.stderr)
        return 1
    # Each bundle argument points DIRECTLY at a tier0-evidence directory (the
    # uploaded/downloaded artifact contents), holding qualification.t0 beside its
    # executables, gate0-candidate tree, and fixture manifest.
    root_p = Path(root).resolve()
    cand = Path(candidate_bundle).resolve()
    clean = Path(cleanroom_bundle).resolve()
    work = Path(tempfile.mkdtemp(prefix="batpak-tier0-compare-"))
    try:
        rlib = work / "libspec.rlib"
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_MSVC,
                 "--crate-type", "rlib", "--crate-name", "spec", "-o", str(rlib),
                 str(root_p / "spec/lib.rs")],
                capture_output=True, text=True).returncode != 0:
            print("selftest: spec rlib did not build for compare", file=sys.stderr)
            return 1
        rc = work / ("receiptcheck" + (".exe" if os.name == "nt" else ""))
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_MSVC,
                 "--crate-name", "receiptcheck", "--extern", f"spec={rlib}",
                 "-o", str(rc), str(root_p / "bootstrap/receiptcheck.rs")],
                capture_output=True, text=True).returncode != 0 or not rc.is_file():
            print("selftest: receiptcheck did not build for compare", file=sys.stderr)
            return 1
        proc = subprocess.run(
            [str(rc), "compare",
             "--candidate-artifact", str(cand / "qualification.t0"),
             "--candidate-evidence", str(cand),
             "--candidate-run-metadata", str(Path(candidate_meta).resolve()),
             "--cleanroom-artifact", str(clean / "qualification.t0"),
             "--cleanroom-evidence", str(clean),
             "--cleanroom-run-metadata", str(Path(cleanroom_meta).resolve()),
             "--root", str(root_p),
             "--python-executable", sys.executable,
             "--require-promotion-confirmation"],
            capture_output=True, text=True)
        print((proc.stdout + proc.stderr).strip())
        if proc.returncode != 0:
            print("selftest: promotion confirmation FAILED", file=sys.stderr)
            return 1
        print("selftest: promotion confirmed (verified by bootstrap/receiptcheck.rs)")
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)


def _verify_bundle(bundle: str, root: str) -> int:
    """Build receiptcheck and verify the UPLOAD-READY hosted evidence bundle shape
    (5.5E6c2): run-metadata required, exact admitted files, artifact bound to the
    bundle. Run after the workflow writes run-metadata and before upload."""
    rustc = shutil.which("rustc")
    if not rustc:
        print("selftest: rustc absent; cannot verify bundle", file=sys.stderr)
        return 1
    root_p = Path(root).resolve()
    bundle_p = Path(bundle).resolve()
    work = Path(tempfile.mkdtemp(prefix="batpak-tier0-verify-"))
    try:
        rlib = work / "libspec.rlib"
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_MSVC,
                 "--crate-type", "rlib", "--crate-name", "spec", "-o", str(rlib),
                 str(root_p / "spec/lib.rs")],
                capture_output=True, text=True).returncode != 0:
            print("selftest: spec rlib did not build for verify-bundle", file=sys.stderr)
            return 1
        rc = work / ("receiptcheck" + (".exe" if os.name == "nt" else ""))
        if subprocess.run(
                [rustc, "--edition", TOOLCHAIN_EDITION, "--target", _T0_MSVC,
                 "--crate-name", "receiptcheck", "--extern", f"spec={rlib}",
                 "-o", str(rc), str(root_p / "bootstrap/receiptcheck.rs")],
                capture_output=True, text=True).returncode != 0 or not rc.is_file():
            print("selftest: receiptcheck did not build for verify-bundle", file=sys.stderr)
            return 1
        proc = subprocess.run(
            [str(rc), "verify", str(bundle_p / "qualification.t0"),
             "--root", str(root_p), "--evidence", str(bundle_p),
             "--upload-ready", "--python-executable", sys.executable],
            capture_output=True, text=True)
        print((proc.stdout + proc.stderr).strip())
        if proc.returncode != 0:
            print("selftest: upload-ready bundle verification FAILED", file=sys.stderr)
            return 1
        print("selftest: upload-ready hosted bundle verified (bootstrap/receiptcheck.rs)")
        return 0
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main() -> int:
    # --emit-evidence <dir>: produce the Tier 0 evidence bundle and hand it to
    # the independent verifier (5.5E6b). A standalone mode: it builds the real
    # gates against a clean export, writes qualification.t0, and reports
    # receiptcheck's verdict — no admission predicate of its own.
    argv0 = sys.argv[1:]
    if argv0[:1] == ["--emit-run-metadata"] and len(argv0) >= 2:
        return _emit_run_metadata(argv0[1], argv0[2:])
    if argv0[:1] == ["--confirm-promotion"] and len(argv0) == 6:
        return _confirm_promotion(*argv0[1:])
    if argv0[:1] == ["--verify-bundle"] and len(argv0) == 3:
        return _verify_bundle(argv0[1], argv0[2])
    if len(argv0) == 2 and argv0[0] == "--emit-evidence":
        bundle = Path(argv0[1]).resolve() / "tier0-evidence"
        artifact, source_root, problems = produce_tier0_evidence(bundle, _T0_GNU)
        if problems:
            print("selftest: TIER0 EVIDENCE INCOMPLETE:", file=sys.stderr)
            for p in problems:
                print(f"- {p}", file=sys.stderr)
            return 1
        ok, out = verify_tier0_evidence(bundle, artifact, source_root, _T0_GNU)
        print(out)
        if not ok:
            print("selftest: receiptcheck REFUSED the produced evidence", file=sys.stderr)
            return 1
        print(f"selftest: tier0 evidence verified by receiptcheck ({artifact})")
        return 0

    # --require-receipts <triple>: the authoritative-lane posture (5.5E2c).
    # Without it, receipts render honestly but an unearned one is a printed
    # "NOT QUALIFIED LOCALLY" line and exit zero — correct for a machine with
    # no linker, never sufficient for the authoritative profile.
    require_target = None
    require_emit = None
    argv = sys.argv[1:]
    if len(argv) >= 2 and argv[0] == "--require-receipts":
        require_target = argv[1]
        rest = argv[2:]
        if rest[:1] == ["--emit"] and len(rest) == 2:
            # Persist the produced bundle at <dir>/tier0-evidence for upload
            # (5.5E6c1), instead of a temp dir the run discards.
            require_emit = rest[1]
        elif rest:
            print(f"selftest: unknown --require-receipts arguments {rest!r} "
                  "(usage: --require-receipts <triple> [--emit <dir>])", file=sys.stderr)
            return 2
    elif argv:
        print(f"selftest: unknown arguments {argv!r} "
              "(usage: selftest.py [--require-receipts <triple> [--emit <dir>]])",
              file=sys.stderr)
        return 2
    freeze = load("freeze")
    audit = load("audit")
    project = load("project")
    # Canonical validator bytes are committed before any hostile probe runs and
    # proven unchanged after the suite. A probe that dies mid-mutation cannot
    # leave a disabled rule behind and still report PASS.
    canonical_before = canonical_commitments()
    findings: list[str] = []
    findings += test_manifest_ordering(freeze)
    findings += test_casefold_collision(audit)
    findings += test_stale_vocabulary(audit)
    findings += test_stale_derivation(audit, HERE.parent)
    findings += test_legacy_manifest_parity(audit)
    findings += test_batql(audit, project)
    findings += test_operator_surfaces(audit, project)
    findings += test_generated_views(audit, project)
    findings += test_inventory_mirrors(audit, project)
    findings += test_exact_ledgers(audit, project)
    findings += test_proof_relations(audit, project)
    findings += test_bootstrap_output(audit, project)
    findings += test_bootstrap_qualification(audit)
    findings += test_numeric(audit)
    findings += test_guarantees(audit, project)
    findings += test_gates(audit, project)
    findings += test_decisions(audit, project)
    findings += test_authenticated_history(audit)
    findings += test_document_law(audit)
    findings += test_claim_receipt_law(audit)
    findings += test_substrate(audit)
    findings += test_specialization(audit, project)
    findings += test_delivery_notes_d2(audit)
    findings += test_proof_policy(audit)
    findings += test_integrity_witnesses(audit)
    findings += test_derived_material_witnesses(audit)
    findings += test_deferred_witnesses(audit)
    findings += test_leg081_authority(audit)
    findings += test_proof_target_resolver(audit)
    findings += test_guarantee_authority(audit, project)
    findings += test_pakvm_semantic_isa(audit, project)
    findings += test_syncbat_firewall(audit, project)
    findings += test_syncbat_requiredness(audit, project)
    findings += test_reconciliation(audit, project)
    findings += test_toolchain(audit)
    findings += test_package_identity(audit)
    findings += test_contract_kinds(audit)
    findings += test_identity_catalogs(audit)
    findings += test_commands(audit)
    findings += test_mutation(audit)
    findings += test_corpus_epoch(audit)
    findings += test_compiler_assumptions(audit)
    findings += test_promotion(audit)
    findings += test_receiptcheck_refuses_dishonest_artifacts()
    findings += test_receiptcheck_bundle_perimeter()
    findings += test_workflow_pinning()
    findings += test_verification_plane()
    findings += test_seedcheck_executes_its_law(audit)
    findings += test_rust_specification_compiles(audit)
    findings += test_probe_harness(audit)
    findings += canonical_drift(canonical_before)
    findings += test_control_characters(audit)
    if findings:
        print(f"selftest: FAIL ({len(findings)} finding(s))", file=sys.stderr)
        for finding in findings:
            print(f"- {finding}", file=sys.stderr)
        return 1
    # Every receipt, stage by stage, BEFORE the banner. The banner then claims
    # only the qualifications that actually executed and passed: listing "Tier 0
    # execution" unconditionally is precisely how a check that never ran came to
    # read as green.
    rendered = render_receipts()
    if rendered:
        print(rendered)
    claimed = ["portability", "stale-vocabulary", "BatQL", "numeric", "guarantee",
               "gate", "decision", "authenticated-history", "control-character",
               "substrate", "specialization", "proof-policy", "probe-isolation",
               "integrity-witness", "derived-material", "deferred-witness",
               "LEG-081 authority", "proof-target resolver",
               "guarantee-authority hostile fixtures", "PakVM semantic ISA",
               "SyncBat authority firewall",
               "SyncBat crossing requiredness",
               "DEC-075 reconciliation",
               "toolchain authority",
               "package identity",
               "contract kinds",
               "isolated materializer output",
               "typed Tier 0 receipt policy",
               "rust specification compile"] + executed_and_passed()
    unearned = [r["name"] for r in QUALIFICATION_RECEIPTS
                if not (r["available"] and r["executed"] and r["passed"])]
    print("selftest: PASS (" + " + ".join(claimed) + ")")
    if unearned:
        # A passing aggregate must never absorb a check that did not run.
        print("selftest: NOT QUALIFIED LOCALLY: " + ", ".join(unearned))
    if require_target:
        # Compat surface (5.5E6b): the flag no longer runs a Python admission
        # predicate. It produces concrete evidence for the requested target and
        # delegates the verdict to the independent verifier. The authoritative
        # MSVC lane rides a git-checkout with a hosted-run binding (the hosted
        # runner); the supplemental GNU lane rides a clean frozen export.
        if require_target not in (_T0_MSVC, _T0_GNU):
            print(f"selftest: --require-receipts {require_target} is not a Tier 0 "
                  f"qualification target ({_T0_MSVC} or {_T0_GNU})", file=sys.stderr)
            return 1
        if require_emit:
            bundle = Path(require_emit).resolve() / "tier0-evidence"
        else:
            bundle = Path(tempfile.mkdtemp(prefix="batpak-tier0-evidence-")) / "tier0-evidence"
        try:
            artifact, source_root, problems = produce_tier0_evidence(bundle, require_target)
            if problems:
                print("selftest: REQUIRED RECEIPTS NOT EARNED "
                      f"(--require-receipts {require_target}):", file=sys.stderr)
                for p in problems:
                    print(f"- {p}", file=sys.stderr)
                return 1
            ok, out = verify_tier0_evidence(bundle, artifact, source_root, require_target)
            if not ok:
                print("selftest: receiptcheck REFUSED the produced evidence "
                      f"(--require-receipts {require_target}):", file=sys.stderr)
                print(out, file=sys.stderr)
                return 1
            print(out)
            print(f"selftest: receipts-qualified target={require_target} "
                  "(verified by bootstrap/receiptcheck.rs)")
            if require_emit:
                print(f"selftest: tier0 evidence bundle persisted ({bundle})")
        finally:
            # Persisted bundles (for upload) survive; temp bundles are discarded.
            if not require_emit:
                shutil.rmtree(bundle.parent, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

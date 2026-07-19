"""Tier 0 producer and its hostile fixtures: the isolated materializer-output
qualification, the independent output oracle, the evidence-bundle producer, and
the require-receipts / emit / verify-bundle / confirm-promotion modes."""
from __future__ import annotations
import hashlib
import os
import json
import re
import sys
import shutil
import subprocess
import tempfile
from pathlib import Path
from .core import (
    HERE,
    TOOLCHAIN_EDITION,
    canonical_commitments,
    canonical_drift,
    gate_sandbox,
    isolated_tree,
    receipt,
    spec_module_source,
)


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
                      for p in (root / "spec").rglob("*.rs")):
        h.update(rel.encode())
        h.update((root / rel).read_bytes())
    for tool in ("seedcheck", "materialize"):
        rel = f"bootstrap/{tool}.rs"
        if (root / rel).is_file():
            h.update(rel.encode())
            h.update((root / rel).read_bytes())
        pkg = root / "bootstrap" / tool
        if pkg.is_dir():
            for p in sorted(pkg.glob("*.rs")):
                r = p.relative_to(root).as_posix()
                h.update(r.encode())
                h.update(p.read_bytes())
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
    arch = _g0_uncomment(spec_module_source(root, "architecture"))
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
          "bootstrap/selftest/tier0.py",
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
    st_src = (HERE / "selftest" / "tier0.py").read_text(encoding="utf-8")
    oracle_block = re.search(r"\ndef oracle_plan\(.*?(?=\ndef )", st_src, re.S).group(0)
    if "materialize" in oracle_block:
        fail("output_oracle_cannot_import_materializer_renderer")
    if "g0-candidate" in oracle_block or "read_tree" in oracle_block:
        fail("output_oracle_cannot_use_generated_output_as_expected_bytes")
    probe("output_oracle_cannot_carry_package_name_map", "bootstrap/selftest/tier0.py",
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}",
          "the output oracle carries the package name",
          '    _forged = "macbat-compiler"\n'
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}")
    probe("output_oracle_cannot_carry_plane_name_map", "bootstrap/selftest/tier0.py",
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}",
          "the output oracle carries the plane module name",
          '    _forged = "pakvm"\n'
          "    f = oracle_facts(root)\n    plan: dict[str, bytes] = {}")
    probe("output_oracle_cannot_carry_feature_map", "bootstrap/selftest/tier0.py",
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
            if "is_symlink" not in (root / "bootstrap/materialize/publish.rs").read_text(encoding="utf-8"):
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
        src_text = (HERE / "selftest" / "tier0.py").read_text(encoding="utf-8")
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
                for rel in ("SPEC.sha256",):
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
             ("bootstrap/materialize/plan.rs",
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
    authoritative target, and the docs/23 Tier 0 denominator projection."""
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
    probe("python_denominator_cannot_return", "bootstrap/selftest/tier0.py",
          "def _tree_digest(root: Path) -> str:",
          "hand-authored slug tuple",
          forged_denominator + "\n\n\ndef _tree_digest(root: Path) -> str:")
    probe("tier0_denominator_block_drift_is_rejected", "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
          "| tier0-materialize | ExecutableAndOutputTree |",
          "does not match its typed derivation",
          "| tier0-materialize | Executable |")

    findings.extend(canonical_drift(before))
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
    pkg_rc = root / "bootstrap" / "receiptcheck"
    if pkg_rc.is_dir():
        rc_src += "\n" + "\n".join(p.read_text(encoding="utf-8")
                                   for p in sorted(pkg_rc.glob("*.rs")))
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
    pkg_sc = source_root / "bootstrap" / "seedcheck"
    if pkg_sc.is_dir():
        n_sc += sum(p.read_text(encoding="utf-8").count("#[test]")
                    for p in sorted(pkg_sc.glob("*.rs")))
    tail, probs = _t0_gate_exe(rustc, edition, target, rlib, source_root, bundle,
                               "tier0-seedcheck-tests", "bootstrap/seedcheck.rs",
                               f"test result: ok. {n_sc} passed",
                               extra=("--test",), run_args=())
    problems += probs
    if tail:
        tails["tier0-seedcheck-tests"] = tail
    n_spec = sum(p.read_text(encoding="utf-8").count("#[test]")
                 for p in sorted((source_root / "spec").rglob("*.rs")))
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

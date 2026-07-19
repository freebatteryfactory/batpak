"""Gate-0 output shape, Tier-0 qualification, and toolchain authority.

Reparses spec/bootstrap_output/types.rs, the spec/bootstrap_qualification and
spec/tier0_cross_run domains, the workflow-pinning and python-tooling planes,
and spec/toolchain/types.rs, holding the static posture laws while the dynamic
isolated qualification lives in selftest. Shares no parser with
bootstrap/project.py.
"""
from __future__ import annotations

import ast
import hashlib
import re
import sys
from pathlib import Path

from .corpus import (
    G_QUAL_ROW,
    _bootstrap_rust_source,
    _bootstrap_source,
    _spec_module_source,
    _uncomment,
)

# --- 5.5E5 Gate-0 materializer output shape ----------------------------------
# spec/bootstrap_output/types.rs owns the candidate's file shape; the
# materializer and the selftest oracle both expand it. This auditor re-parses
# the typed owner independently and holds the STATIC posture laws: the dynamic
# isolated qualification (two output roots, seed snapshot, byte oracle, Cargo)
# lives in selftest, and audit never executes Cargo or materializes a tree.

A_BO_SPEC = "spec/bootstrap_output/types.rs"
A_BO_ENUMS = ("Gate0RootArtifact", "Gate0PackageArtifact", "Gate0PackageTargetKind",
              "Gate0PlaneArtifact", "Gate0OutputDisposition")


def bootstrap_output_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_BO_SPEC
    if not path.is_file():
        return [f"missing {A_BO_SPEC}"]
    raw = path.read_text(encoding="utf-8")
    src = _uncomment(raw)
    arch = _uncomment(_spec_module_source(root, "architecture"))

    def enum_variants(text: str, enum: str) -> list[str]:
        # Indentation-robust brace match: the five closed enums live inside a
        # `gate0_closed_enum!` invocation (5.5E5a), so their variants sit two
        # indents deep and their `ALL` is macro-generated, not authored.
        head = text.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_BO_SPEC}: declares no {enum}")
            return []
        i = text.find("{", head)
        depth = 0
        body = ""
        for j in range(i, len(text)):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    body = text[i + 1: j]
                    break
        return re.findall(r"^\s+([A-Z]\w*),", body, re.M)

    def arm_pairs(text: str, fn: str, head_re: str, key: str) -> dict[str, str]:
        m = re.search(head_re, text, re.S)
        if not m:
            out.append(f"{A_BO_SPEC}: declares no {fn}()")
            return {}
        return dict(re.findall(key + r"::(\w+) => \"([^\"]*)\"", m.group(1)))

    # The five closed vocabularies are each declared through the one
    # `gate0_closed_enum!` primitive, which emits the enum AND its `ALL`
    # inventory from a single variant list (5.5E5a). Enum-to-`ALL` divergence
    # is therefore unrepresentable, so there is no authored-vs-listed
    # comparison and no cardinality here: the auditor verifies the CONSTRUCTION
    # instead. A hand-written `pub const ALL` for one of these enums is the
    # divergence surface the macro exists to close, so it is a finding.
    macro_enums = set(re.findall(
        r"gate0_closed_enum!\s*\{[^{]*?pub enum (\w+)", src))
    inventories: dict[str, list[str]] = {}
    for enum in A_BO_ENUMS:
        authored = enum_variants(src, enum)
        inventories[enum] = authored
        if enum not in macro_enums:
            out.append(f"{A_BO_SPEC}: {enum} is not declared through gate0_closed_enum!; "
                       "its inventory could diverge from the enum")
        if re.search(r"pub const ALL: &'static \[" + enum + r"\]", src):
            out.append(f"{A_BO_SPEC}: {enum} carries a hand-written ALL; the inventory "
                       "must come from gate0_closed_enum! so it cannot diverge")
        if len(set(authored)) != len(authored):
            out.append(f"{A_BO_SPEC}: {enum} lists a variant twice")

    # Root artifact paths: total, nonempty, unique; the RustToolchain artifact
    # must project the tracked root toolchain path.
    root_paths = arm_pairs(
        src, "relative_path",
        r"pub const fn relative_path\(self\)[^{]*\{(.*?)\n    \}", "Gate0RootArtifact")
    for name in inventories.get("Gate0RootArtifact", []):
        path_str = root_paths.get(name, "")
        if not path_str.strip():
            out.append(f"{A_BO_SPEC}: root artifact {name} projects no path")
        elif path_str.startswith("/") or ".." in path_str.split("/"):
            out.append(f"{A_BO_SPEC}: root artifact {name} path {path_str!r} is not "
                       "output-root-relative and traversal-free")
    if len(set(root_paths.values())) != len(root_paths):
        out.append(f"{A_BO_SPEC}: two root artifacts project one path")
    if root_paths.get("RustToolchain", "rust-toolchain.toml") != "rust-toolchain.toml":
        out.append(f"{A_BO_SPEC}: RustToolchain does not project the tracked root toolchain path")

    # Target kinds: every PackageId arm explicit, no wildcard, no class fallback.
    packages = enum_variants(arch, "PackageId")
    tk = re.search(r"pub const fn target_kind\(package: PackageId\)[^{]*\{(.*?)\n\}",
                   src, re.S)
    kinds: dict[str, str] = {}
    if not tk:
        out.append(f"{A_BO_SPEC}: declares no target_kind()")
    else:
        body = tk.group(1)
        kinds = dict(re.findall(r"PackageId::(\w+) => Gate0PackageTargetKind::(\w+)", body))
        if re.search(r"\n\s*_\s*=>", body) or "PackageClass" in body:
            out.append(f"{A_BO_SPEC}: target_kind() carries a wildcard or class fallback; "
                       "a future package must be classified explicitly")
        for package in packages:
            if package not in kinds:
                out.append(f"{A_BO_SPEC}: package {package} has no explicit target kind")
        for package, kind in kinds.items():
            if package not in packages:
                out.append(f"{A_BO_SPEC}: target_kind() names unknown package {package}")
            if kind not in inventories.get("Gate0PackageTargetKind", []):
                out.append(f"{A_BO_SPEC}: target_kind() names unknown kind {kind}")

    # Binary targets: named exactly where the kind requires one, nonempty, unique.
    bt = re.search(r"pub const fn binary_target\(package: PackageId\)[^{]*\{(.*?)\n\}",
                   src, re.S)
    named: dict[str, str] = {}
    if not bt:
        out.append(f"{A_BO_SPEC}: declares no binary_target()")
    else:
        named = dict(re.findall(r"PackageId::(\w+) => Some\(\"([^\"]+)\"\)", bt.group(1)))
        want = {p for p, k in kinds.items() if k in ("Binary", "ExampleBinary")}
        for package in sorted(want - set(named)):
            out.append(f"{A_BO_SPEC}: binary-kind package {package} names no binary target")
        for package in sorted(set(named) - want):
            out.append(f"{A_BO_SPEC}: library-kind package {package} names a binary target")
        if len(set(named.values())) != len(named):
            out.append(f"{A_BO_SPEC}: two packages claim one binary target name")

    # Source doors: every authored kind projects a nonempty door.
    door = re.search(r"pub const fn source_suffix\(package: PackageId\)[^{]*\{(.*?)\n\}",
                     src, re.S)
    doors: dict[str, str] = {}
    if not door:
        out.append(f"{A_BO_SPEC}: declares no source_suffix()")
    else:
        for pattern, suffix in re.findall(
                r"((?:Gate0PackageTargetKind::\w+\s*\|?\s*)+)=> \"([^\"]+)\"",
                door.group(1)):
            for kind in re.findall(r"Gate0PackageTargetKind::(\w+)", pattern):
                doors[kind] = suffix
        for kind in inventories.get("Gate0PackageTargetKind", []):
            if not doors.get(kind, "").strip():
                out.append(f"{A_BO_SPEC}: target kind {kind} projects no source door")

    # Plane modules: total over SyncBatPlane, nonempty, unique.
    modules = arm_pairs(
        arch, "module_name",
        r"pub const fn module_name\(self\)[^{]*\{(.*?)\n    \}", "SyncBatPlane")
    planes = enum_variants(arch, "SyncBatPlane")
    for plane in planes:
        if not modules.get(plane, "").strip():
            out.append(f"{A_BO_SPEC}: SyncBat plane {plane} projects no module name")
    if len(set(modules.values())) != len(modules):
        out.append(f"{A_BO_SPEC}: two SyncBat planes project one module name")

    # Workspace metadata is owned here and nonempty.
    # Parsed from the RAW text: _uncomment would truncate the // in a URL.
    for const in ("WORKSPACE_LICENSE", "WORKSPACE_REPOSITORY"):
        m = re.search(r"pub const " + const + r": &str = \"([^\"]*)\";", raw)
        if not m or not m.group(1).strip():
            out.append(f"{A_BO_SPEC}: {const} is missing or empty")

    # The docs/23 plan block equals this auditor's own expansion, path for
    # path, in canonical order: roots, packages, planes.
    expected_paths: list[str] = []
    for name in inventories.get("Gate0RootArtifact", []):
        expected_paths.append(root_paths.get(name, ""))
    package_paths = arm_pairs(
        arch, "workspace_path",
        r"pub const fn workspace_path\(self\)[^{]*\{(.*?)\n    \}", "PackageId")
    package_all = re.search(r"pub const ALL: &'static \[PackageId\] = &\[(.*?)\];",
                            arch, re.S)
    package_order = re.findall(r"PackageId::(\w+)", package_all.group(1)) if package_all else []
    for package in package_order:
        base = package_paths.get(package, "")
        door = doors.get(kinds.get(package, ""), "")
        for suffix in ("Cargo.toml", "README.md", door):
            expected_paths.append(f"{base}/{suffix}" if base and suffix else "")
    plane_all = re.search(r"pub const ALL: &'static \[SyncBatPlane\] = &\[(.*?)\];",
                          arch, re.S)
    plane_order = re.findall(r"SyncBatPlane::(\w+)", plane_all.group(1)) if plane_all else []
    syncbat_base = package_paths.get("SyncBat", "")
    for plane in plane_order:
        module = modules.get(plane, "")
        expected_paths.append(f"{syncbat_base}/src/{module}/mod.rs")
        expected_paths.append(f"{syncbat_base}/src/{module}/types.rs")
    doc23 = root / "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md"
    if doc23.is_file():
        text = doc23.read_text(encoding="utf-8")
        m = re.search(r"<!-- GATE0-MATERIALIZATION-PLAN:BEGIN[^>]*-->\n(.*?)\n"
                      r"<!-- GATE0-MATERIALIZATION-PLAN:END -->", text, re.S)
        if not m:
            out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: carries no "
                       "GATE0-MATERIALIZATION-PLAN block")
        else:
            got_paths = [line.split("|")[1].strip()
                         for line in m.group(1).splitlines()
                         if line.startswith("|") and not line.startswith("| ---")
                         and not line.startswith("| Path")]
            if got_paths != expected_paths:
                out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: the "
                           "GATE0-MATERIALIZATION-PLAN block does not match its "
                           "typed derivation")
    else:
        out.append("missing docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")

    # Every expanded plan path obeys one portable-path law, independently
    # reconstructed here (5.5E5a; rebuilt as a POSITIVE grammar in 5.5E6b). The
    # old negative filter admitted everything nobody forbade — a non-ASCII
    # component and the Windows-invalid set `< > " | ? *` slipped through. This
    # reconstruction spells out what a component MAY contain (ASCII
    # `[A-Za-z0-9._-]` only), so a byte that cannot be spelled cannot smuggle a
    # backslash separator, a drive colon, a wildcard, or a homoglyph.
    def portable_component(component: str) -> bool:
        if component in ("", ".", ".."):
            return False
        if not all(
            ("a" <= c <= "z") or ("A" <= c <= "Z") or ("0" <= c <= "9")
            or c in "._-"
            for c in component
        ):
            return False
        if component.endswith("."):
            return False
        stem = component.split(".")[0].upper()
        if stem in ("CON", "PRN", "AUX", "NUL"):
            return False
        return not (len(stem) == 4 and stem[:3] in ("COM", "LPT")
                    and stem[3].isdigit() and stem[3] != "0")

    def portable(path: str) -> bool:
        if not path or path.startswith("/"):
            return False
        return all(portable_component(c) for c in path.split("/"))

    folded: dict[str, str] = {}
    for path in expected_paths:
        if not portable(path):
            out.append(f"{A_BO_SPEC}: expanded plan path {path!r} is not a portable "
                       "relative path")
        low = path.lower()
        if low in folded and folded[low] != path:
            out.append(f"{A_BO_SPEC}: plan paths {folded[low]!r} and {path!r} collide "
                       "under case folding")
        folded[low] = path

    # The typed path law is the one law: the materializer and seedcheck must
    # USE it, not carry their own slash-only reimplementation, and no
    # cardinality assertion may return in seedcheck.
    if "pub fn is_portable_gate0_relative_path" not in raw:
        out.append(f"{A_BO_SPEC}: the portable-path law is missing")
    sc = _bootstrap_rust_source(root, "seedcheck")
    if sc:
        if "is_portable_gate0_relative_path" not in sc:
            out.append("bootstrap/seedcheck.rs: does not use the typed portable-path law")
        if re.search(r"Gate0\w+::ALL\.len\(\)\s*!=\s*\d", sc):
            out.append("bootstrap/seedcheck.rs: a Gate0 ALL cardinality assertion "
                       "returned; the inventory is count-free by construction")

    # The parallel plane inventories cannot return.
    for token in ("SYNCBAT_REQUIRED_PLANES", "pub const SYNCBAT_PLANES"):
        if token in arch:
            out.append(f"spec/architecture/types.rs: the raw plane inventory {token.split()[-1]} "
                       "returned; SyncBatPlane::ALL is the one inventory")

    # Static materializer posture laws.
    mat = _bootstrap_rust_source(root, "materialize")
    if not mat:
        out.append("missing bootstrap/materialize.rs")
    else:
        if 'PathBuf::from(".")' in mat or "args().nth(1)" in mat:
            out.append("bootstrap/materialize.rs: an implicit current-directory or "
                       "one-root invocation route exists; both roots must be explicit")
        for flag in ("--seed", "--output"):
            if f'"{flag}"' not in mat:
                out.append(f"bootstrap/materialize.rs: the explicit {flag} boundary is gone")
        if "write_checked" in mat:
            out.append("bootstrap/materialize.rs: the per-file overwrite-or-repair "
                       "route returned; publication is a staged whole tree")
        if "fs::rename(" not in mat:
            out.append("bootstrap/materialize.rs: publication does not rename a "
                       "complete staged tree into place")
        if "plane_roles" in mat:
            out.append("bootstrap/materialize.rs: a materializer-local plane role "
                       "map returned; authored_ownership() is the one description")
        if re.search(r'"browser(?:-storage)?"', mat):
            out.append("bootstrap/materialize.rs: a handwritten qualification-feature "
                       "name returned; features derive from the profile ledger")
        if "is_portable_gate0_relative_path" not in mat:
            out.append("bootstrap/materialize.rs: does not use the typed portable-path "
                       "law; a hand-rolled path check can drift from the owner")
        # The candidate justfile is internally executable: it may not advertise
        # seed-only tools it does not contain.
        jf = re.search(r"fn justfile\(\)[^{]*\{(.*?)\n\}", mat, re.S)
        if jf and re.search(r"audit\.py|freeze\.py|seedcheck\.rs", jf.group(1)):
            out.append("bootstrap/materialize.rs: the candidate justfile references a "
                       "seed-only tool the candidate does not contain")
        # The binary is bound to the seed manifest it was compiled from.
        if "COMPILED_SEED_MANIFEST" not in mat or "include_str!" not in mat:
            out.append("bootstrap/materialize.rs: the binary is not bound to its "
                       "compiled SPEC.sha256 seed manifest")

    # The dedicated isolated qualification exists and the generic binary route
    # no longer carries tier0-materialize. Read the whole tool (shim + package)
    # so a decomposition cannot move the symbol out of the detector's view.
    st = _bootstrap_source(root, "selftest")
    if st:
        if "def qualify_materializer" not in st:
            out.append("bootstrap/selftest.py: no dedicated isolated materializer "
                       "qualification exists")
        if re.search(r'\("tier0-materialize",\s*"bootstrap/materialize\.rs"', st):
            out.append("bootstrap/selftest.py: tier0-materialize still rides the "
                       "generic one-root binary qualification")
        oracle = re.search(r"\ndef oracle_plan\(.*?(?=\ndef )", st, re.S)
        if oracle:
            block = oracle.group(0)
            cargo_names = re.findall(r'PackageId::\w+ => "([^"]+)"',
                                     re.search(r"pub const fn cargo_name\(self\)[^{]*\{(.*?)\n    \}",
                                               arch, re.S).group(1))
            for name in cargo_names:
                if f'"{name}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"package name {name!r}; names derive from spec/architecture/types.rs")
            for module in modules.values():
                if f'"{module}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"plane module name {module!r}; modules derive from the typed inventory")
            wasm_profiles = [prof for _, prof, env in re.findall(
                r'package: PackageId::(\w+),\s*profile: "([^"]+)",\s*'
                r"environment: QualificationEnvironment::(\w+)", arch) if env == "WasmHost"]
            for prof in wasm_profiles:
                if f'"{prof}"' in block:
                    out.append(f"bootstrap/selftest.py: the output oracle carries the "
                               f"feature name {prof!r}; features derive from the profile ledger")
    return out


# --- 5.5E6a typed Tier 0 receipt policy -------------------------------------
A_BQ_SPEC = "spec/bootstrap_qualification"


def bootstrap_qualification_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_BQ_SPEC / "mod.rs"
    if not path.is_file():
        return [f"missing {A_BQ_SPEC}/mod.rs"]
    src = _uncomment(_spec_module_source(root, "bootstrap_qualification"))
    tc = _uncomment((root / "spec/toolchain/types.rs").read_text(encoding="utf-8"))

    def variants(text: str, enum: str) -> list[str]:
        head = text.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_BQ_SPEC}: declares no {enum}")
            return []
        return re.findall(r"^\s{4}(\w+),$", text[head: text.find("\n}", head)], re.M)

    def all_of(text: str, enum: str) -> list[str]:
        m = re.search(r"pub const ALL: &'static \[" + enum + r"\] = &\[(.*?)\];", text, re.S)
        return re.findall(enum + r"::(\w+)", m.group(1)) if m else []

    # Closed vocabularies equal their ALL inventory, both directions.
    for text, enum in ((src, "Tier0ReceiptKind"), (src, "Tier0ArtifactPolicy"),
                       (tc, "RustTargetTriple")):
        authored = variants(text, enum)
        listed = all_of(text, enum)
        for name in authored:
            if name not in listed:
                out.append(f"{enum}::{name} is authored but ALL omits it")
        for name in listed:
            if name not in authored:
                out.append(f"{enum}::ALL lists {name}, which the enum does not author")
        if not listed:
            out.append(f"{enum}::ALL is empty")

    # Every Tier0ReceiptKind has a nonempty unique slug and a total policy arm.
    kinds = all_of(src, "Tier0ReceiptKind")
    slugs = dict(re.findall(r'Tier0ReceiptKind::(\w+) => "([^"]+)"', src))
    policy = dict(re.findall(r"Tier0ReceiptKind::(\w+) => Tier0ArtifactPolicy::(\w+)", src))
    seen: set[str] = set()
    for kind in kinds:
        s = slugs.get(kind, "")
        if not s:
            out.append(f"{A_BQ_SPEC}: {kind} projects no slug")
        elif s in seen:
            out.append(f"{A_BQ_SPEC}: slug {s} is claimed twice")
        seen.add(s)
        if kind not in policy:
            out.append(f"{A_BQ_SPEC}: {kind} names no artifact policy")

    # The authoritative target is the MSVC triple, distinct from the semantic
    # QualificationEnvironment.
    auth = re.search(r"pub const AUTHORITATIVE_TARGET: RustTargetTriple = "
                     r"RustTargetTriple::(\w+);", tc)
    if not auth or auth.group(1) != "X86_64PcWindowsMsvc":
        out.append("spec/toolchain/types.rs: AUTHORITATIVE_TARGET is not the MSVC triple")
    if "QualificationEnvironment" in src:
        out.append(f"{A_BQ_SPEC}: speaks QualificationEnvironment; a semantic environment "
                   "is not a physical target")

    # The denominator moved out of Python: selftest DERIVES it, never authors a
    # literal slug tuple, and the product ReceiptId cannot substitute.
    st = _bootstrap_source(root, "selftest")
    # 5.5E6b: the Tier 0 admission authority left Python. selftest holds no
    # hand-authored denominator and no Python pass-predicate; it produces
    # concrete evidence and delegates the verdict to the independent verifier
    # bootstrap/receiptcheck.rs. A reintroduced slug tuple, or a selftest that
    # no longer delegates, is refused here.
    if re.search(r'REQUIRED_TIER0_RECEIPTS = \(\s*\("tier0-', st):
        out.append("bootstrap/selftest.py: REQUIRED_TIER0_RECEIPTS is a hand-authored "
                   "slug tuple; Tier0ReceiptKind::ALL is the one denominator")
    if "verify_tier0_evidence(" not in st:
        out.append("bootstrap/selftest.py: does not delegate Tier 0 qualification to "
                   "the independent verifier bootstrap/receiptcheck.rs")
    if "ReceiptId" in src:
        out.append(f"{A_BQ_SPEC}: reuses the product ReceiptId; Tier0ReceiptKind is the "
                   "bootstrap receipt identity")

    # The Tier 0 denominator block equals the typed projection.
    dn = (root / "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")
    if dn.is_file():
        m = re.search(r"<!-- TIER0-RECEIPT-DENOMINATOR:BEGIN[^>]*-->\n(.*?)\n"
                      r"<!-- TIER0-RECEIPT-DENOMINATOR:END -->",
                      dn.read_text(encoding="utf-8"), re.S)
        if not m:
            out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: carries no "
                       "TIER0-RECEIPT-DENOMINATOR block")
        else:
            rows = [tuple(c.strip() for c in line.strip("|").split("|"))
                    for line in m.group(1).splitlines()
                    if line.startswith("|") and not line.startswith("| ---")
                    and not line.startswith("| Receipt")]
            want = [(slugs[k], policy[k]) for k in kinds]
            if rows != want:
                out.append("docs/23_BOOTSTRAP_AND_SELF_HOSTING.md: the Tier 0 "
                           "denominator block does not match its typed derivation")
    else:
        out.append("missing docs/23_BOOTSTRAP_AND_SELF_HOSTING.md")

    # === E6b: the evidence-verification algebra is internally coherent ======
    # Independent reconstruction (regex, no shared parse with the spec crate):
    # the rule inventory equals its ALL both directions; every rule carries an
    # owner, a law, and a repair; ownership resolves only to the three declared
    # owners; the artifact-evidence sum matches the policy vocabulary exactly;
    # and the concrete-digest and refusal primitives are present. This proves
    # the SHAPES are admissible; bootstrap/receiptcheck.rs proves the concrete
    # evidence matches them, and seedcheck executes verify() against each rule.
    rule_enum = "Tier0VerificationRule"
    rule_authored = variants(src, rule_enum)
    rule_listed = all_of(src, rule_enum)
    for name in rule_authored:
        if name not in rule_listed:
            out.append(f"{rule_enum}::{name} is authored but ALL omits it")
    for name in rule_listed:
        if name not in rule_authored:
            out.append(f"{rule_enum}::ALL lists {name}, which the enum does not author")
    if not rule_listed:
        out.append(f"{rule_enum}::ALL is empty")

    def arm_body(fn_sig: str) -> str:
        m = re.search(re.escape(fn_sig) + r"\s*\{(.*?)\n    \}", src, re.S)
        return m.group(1) if m else ""

    owner_body = arm_body("pub const fn owner(self) -> ContractId")
    law_body = arm_body("pub const fn law(self) -> &'static str")
    repair_body = arm_body("pub const fn repair(self) -> &'static str")
    for label, body in (("owner", owner_body), ("law", law_body),
                        ("repair", repair_body)):
        if not body:
            out.append(f"{A_BQ_SPEC}: {rule_enum} declares no {label}() arm")
    for name in rule_authored:
        token = f"{rule_enum}::{name}"
        if token not in owner_body:
            out.append(f"{A_BQ_SPEC}: rule {name} names no owning contract")
        if f"{token} =>" not in law_body:
            out.append(f"{A_BQ_SPEC}: rule {name} states no law")
        if f"{token} =>" not in repair_body:
            out.append(f"{A_BQ_SPEC}: rule {name} states no repair")
    # Every law and repair arm is a nonempty string literal.
    for label, body in (("law", law_body), ("repair", repair_body)):
        for name, text in re.findall(
                rule_enum + r"::(\w+) =>\s*\"((?:[^\"\\]|\\.)*)\"", body):
            if not text.strip():
                out.append(f"{A_BQ_SPEC}: rule {name} {label} is empty")

    # The three owners, and only those three, resolve rule ownership; each is
    # declared with its authoritative contract id.
    owner_decl = dict(re.findall(
        r'pub const (TIER0_\w+_OWNER): ContractId =\s*ContractId\("([^"]+)"\);', src))
    expected_owners = {
        "TIER0_RECEIPT_ALGEBRA_OWNER": "BP-RECEIPTS-1",
        "TIER0_DENOMINATOR_OWNER": "BP-BOOTSTRAP-1",
        "TIER0_HOSTED_QUALIFICATION_OWNER": "BP-PUBLIC-API-CI-RELEASE-1",
    }
    if owner_decl != expected_owners:
        out.append(f"{A_BQ_SPEC}: the three Tier 0 ownership constants are not "
                   "declared with their authoritative contract ids")
    owners_used = set(re.findall(r"TIER0_\w+_OWNER", owner_body))
    for used in owners_used:
        if used not in expected_owners:
            out.append(f"{A_BQ_SPEC}: owner() resolves to {used}, not a declared "
                       "Tier 0 ownership constant")
    for declared in expected_owners:
        if declared not in owners_used:
            out.append(f"{A_BQ_SPEC}: ownership constant {declared} owns no rule")

    # The artifact-evidence sum matches the policy vocabulary one-to-one, and
    # each evidence variant reports the same-named policy.
    ev_head = src.find("pub enum Tier0ArtifactEvidence {")
    ev_body = src[ev_head: src.find("\n}", ev_head)] if ev_head >= 0 else ""
    ev_variants = set(re.findall(r"^\s{4}(\w+)\s*(?:\{|,)", ev_body, re.M))
    policy_variants = set(variants(src, "Tier0ArtifactPolicy"))
    if not ev_variants or ev_variants != policy_variants:
        out.append(f"{A_BQ_SPEC}: Tier0ArtifactEvidence variants do not match the "
                   "Tier0ArtifactPolicy vocabulary")
    ev_policy = re.findall(
        r"Tier0ArtifactEvidence::(\w+) \{ \.\. \} => \{?\s*Tier0ArtifactPolicy::(\w+)",
        src)
    for variant, mapped in ev_policy:
        if variant != mapped:
            out.append(f"{A_BQ_SPEC}: evidence {variant} reports policy {mapped}")
    if {v for v, _ in ev_policy} != ev_variants:
        out.append(f"{A_BQ_SPEC}: Tier0ArtifactEvidence::policy() is not total")

    # A slug resolves back to its kind, and the refusal error names its rule.
    if "pub fn from_slug(" not in src:
        out.append(f"{A_BQ_SPEC}: Tier0ReceiptKind has no from_slug resolver")
    err_head = src.find("pub enum Tier0VerificationError {")
    err_body = src[err_head: src.find("\n}", err_head)] if err_head >= 0 else ""
    for arm in ("Receipt", "Denominator", "Qualification"):
        if f"{arm} {{" not in err_body:
            out.append(f"{A_BQ_SPEC}: Tier0VerificationError omits its {arm} arm")
    if "pub const fn rule(self) -> Tier0VerificationRule" not in src:
        out.append(f"{A_BQ_SPEC}: Tier0VerificationError projects no rule()")

    # The concrete-digest and hex-parse primitives are present and
    # lowercase-strict: a Tier 0 evidence digest is its own bootstrap type.
    for prim in ("Sha256Digest", "GitCommitSha", "GitTreeSha", "ToolchainCommit"):
        impl = re.search(r"impl " + prim + r" \{(.*?)\n\}", src, re.S)
        body = impl.group(1) if impl else ""
        if "fn from_hex(" not in body or "fn render(" not in body:
            out.append(f"{A_BQ_SPEC}: {prim} is not a from_hex/render digest primitive")
    hex_err = set(variants(src, "HexParseError"))
    if hex_err != {"WrongLength", "NonLowerHexDigit"}:
        out.append(f"{A_BQ_SPEC}: HexParseError does not distinguish a wrong length "
                   "from a non-lowercase-hex digit")

    # receiptcheck.rs is the authoritative independent computer, and it is wired
    # into the required-file denominator so it can never silently vanish.
    if "bootstrap/receiptcheck.rs" not in \
            _spec_module_source(root, "architecture"):
        out.append("spec/architecture/inventory.rs: REQUIRED_DOCS omits bootstrap/receiptcheck.rs")
    return out


A_XR_SPEC = "spec/tier0_cross_run"


def tier0_cross_run_findings(root: Path) -> list[str]:
    out: list[str] = []
    path = root / A_XR_SPEC / "mod.rs"
    if not path.is_file():
        return [f"missing {A_XR_SPEC}/mod.rs"]
    src = _uncomment(_spec_module_source(root, "tier0_cross_run"))

    # Independent reconstruction (regex, no shared parse with the spec crate) of
    # the cross-run same-source comparator (5.5E6c). The verdict is EXECUTED by
    # seedcheck (compare_runs over verified qualifications); here we prove the
    # SHAPE is admissible: the outcomes, the preconditions, and — the load-
    # bearing property — that the source-identity coordinates are exactly the
    # deterministic ones and NO compiled-artifact digest is treated as a source
    # coordinate.
    def variants(enum: str) -> set[str]:
        head = src.find(f"pub enum {enum} {{")
        if head < 0:
            out.append(f"{A_XR_SPEC}: declares no {enum}")
            return set()
        body = src[head: src.find("\n}", head)]
        return set(re.findall(r"^\s{4}(\w+)\s*(?:\{|,|\()", body, re.M))

    # The comparison result names exactly its three outcomes.
    if variants("CrossRunComparison") != {"NotComparable", "DifferentSource", "SameSource"}:
        out.append(f"{A_XR_SPEC}: CrossRunComparison is not the three-outcome result "
                   "(NotComparable | DifferentSource | SameSource)")

    # Non-comparability names exactly its three precondition failures.
    if variants("NotComparable") != {"FrozenExportSource", "MissingHostedRun", "SameHostedRun"}:
        out.append(f"{A_XR_SPEC}: NotComparable does not name exactly the frozen-export, "
                   "missing-hosted-run, and same-hosted-run preconditions")

    # The source-identity coordinates are EXACTLY the five deterministic ones;
    # an executable/artifact coordinate here would be the dishonest-red mistake.
    expected_div = {"SourceCommit", "SourceTree", "SpecManifestDigest",
                    "WorkflowDigest", "MaterializerOutputTree"}
    div = variants("SourceDivergence")
    if div != expected_div:
        out.append(f"{A_XR_SPEC}: SourceDivergence is not exactly the five source-identity "
                   "coordinates (commit, tree, spec-manifest, workflow, materializer-output-tree)")
    if any(("xecutable" in v) or ("Artifact" in v) for v in div):
        out.append(f"{A_XR_SPEC}: SourceDivergence treats a compiled-artifact digest as a "
                   "source coordinate; executable digests legitimately differ across runs")

    # Side and the two agreement classifications.
    if variants("Side") != {"Left", "Right", "Both"}:
        out.append(f"{A_XR_SPEC}: Side is not Left/Right/Both")
    if variants("ToolchainAgreement") != {"Identical", "Divergent"}:
        out.append(f"{A_XR_SPEC}: ToolchainAgreement is not Identical/Divergent")
    if variants("TargetAgreement") != {"SameTarget", "CrossTarget"}:
        out.append(f"{A_XR_SPEC}: TargetAgreement is not SameTarget/CrossTarget")
    if variants("BootstrapRuntimeAgreement") != {"Identical", "Divergent"}:
        out.append(f"{A_XR_SPEC}: BootstrapRuntimeAgreement is not Identical/Divergent")

    # compare_runs actually EMITS every declared outcome — no coordinate is
    # declared but never checked, and no precondition is unreachable — consumes
    # the load-bearing materializer output tree, and inspects no executable digest.
    fn_head = src.find("pub fn compare_runs(")
    # Bound compare_runs at the next top-level item (confirm_promotion) so the
    # executable-digest scan below does not reach into confirm_promotion or the
    # in-crate test module.
    fn_end = src.find("\npub fn confirm_promotion(", fn_head) if fn_head >= 0 else -1
    if fn_end < 0:
        fn_end = src.find("\n#[cfg(test)]", fn_head) if fn_head >= 0 else -1
    fn_body = src[fn_head: fn_end if fn_end >= 0 else len(src)] if fn_head >= 0 else ""
    if not fn_body:
        out.append(f"{A_XR_SPEC}: declares no compare_runs")
    else:
        for variant in sorted(expected_div):
            if f"SourceDivergence::{variant}" not in fn_body:
                out.append(f"{A_XR_SPEC}: compare_runs never emits the {variant} divergence")
        for variant in ("FrozenExportSource", "MissingHostedRun", "SameHostedRun"):
            if f"NotComparable::{variant}" not in fn_body:
                out.append(f"{A_XR_SPEC}: compare_runs never emits NotComparable::{variant}")
        if "materializer_output_tree" not in fn_body:
            out.append(f"{A_XR_SPEC}: compare_runs ignores the materializer output tree")
        if "executable_digest" in fn_body:
            out.append(f"{A_XR_SPEC}: compare_runs inspects an executable digest; compiled "
                       "artifacts legitimately differ across independent runs")

    # The proof is sealed (compare_runs is its sole constructor) and retains the
    # proven-common coordinates plus the two attesting run identities.
    proof_head = src.find("pub struct SameSourceProof {")
    proof_body = src[proof_head: src.find("\n}", proof_head)] if proof_head >= 0 else ""
    if "_seal: ()" not in proof_body:
        out.append(f"{A_XR_SPEC}: SameSourceProof is not sealed; compare_runs is not its "
                   "sole constructor")
    for field in ("commit", "tree", "spec_manifest_digest", "workflow_digest",
                  "materializer_output_tree", "left_run", "right_run",
                  "bootstrap_runtime_agreement"):
        if f"{field}:" not in proof_body:
            out.append(f"{A_XR_SPEC}: SameSourceProof retains no {field}")

    # Promotion confirmation (5.5E6c1) is STRICTLY STRONGER than same-source. Its
    # error algebra names exactly the posture requirements it adds over
    # same-source; its sealed proof retains the confirmed authority.
    expected_pc_err = {"NotSameSource", "NonAuthoritativeTarget", "CrossTarget",
                       "ToolchainDivergent", "BootstrapRuntimeDivergent",
                       "RepositoryMismatch", "NonCanonicalWorkflowPath"}
    if variants("PromotionConfirmationError") != expected_pc_err:
        out.append(f"{A_XR_SPEC}: PromotionConfirmationError is not exactly the seven promotion "
                   "posture refusals (not-same-source, non-authoritative-target, cross-target, "
                   "toolchain-divergent, bootstrap-runtime-divergent, repository-mismatch, "
                   "non-canonical-workflow-path)")

    pc_head = src.find("pub struct PromotionConfirmationProof {")
    pc_body = src[pc_head: src.find("\n}", pc_head)] if pc_head >= 0 else ""
    if "_seal: ()" not in pc_body:
        out.append(f"{A_XR_SPEC}: PromotionConfirmationProof is not sealed; confirm_promotion is "
                   "not its sole constructor")
    for field in ("same_source", "target", "toolchain", "candidate_run", "cleanroom_run"):
        if f"{field}:" not in pc_body:
            out.append(f"{A_XR_SPEC}: PromotionConfirmationProof retains no {field}")

    # confirm_promotion builds on the same-source proof, refuses on every added
    # posture requirement, requires the authoritative target, and pins the
    # canonical authoritative workflow — a promotion cannot be confirmed from a
    # branch name or a supplemental target.
    cp_head = src.find("pub fn confirm_promotion(")
    # confirm_promotion is the last non-test item. The concept-door decomposition
    # (SW5) relocated `#[cfg(test)] mod tests` to the door, so the executed-token
    # scan bounds at the function's own column-0 closing brace instead of the
    # (now-relocated) test-module marker — otherwise the scan would reach into the
    # test module and a removed token could still be seen there. Works unchanged
    # on an undecomposed tree, where the same brace precedes `#[cfg(test)]`.
    cp_close = src.find("\n}\n", cp_head) if cp_head >= 0 else -1
    cp_end = cp_close if cp_close >= 0 else len(src)
    cp_body = src[cp_head: cp_end] if cp_head >= 0 else ""
    if not cp_body:
        out.append(f"{A_XR_SPEC}: declares no confirm_promotion")
    else:
        for variant in sorted(expected_pc_err):
            if f"PromotionConfirmationError::{variant}" not in cp_body:
                out.append(f"{A_XR_SPEC}: confirm_promotion never emits the {variant} refusal")
        if "compare_runs(" not in cp_body or "SameSource" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not build on the same-source proof")
        if "is_authoritative()" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not require the authoritative target")
        if "AUTHORITATIVE_WORKFLOW_PATH" not in cp_body:
            out.append(f"{A_XR_SPEC}: confirm_promotion does not pin the canonical authoritative "
                       "workflow path")

    # The retained binding the comparator consumes is exposed by the sealed E6b
    # qualification, the canonical workflow path is owned there, and seedcheck
    # executes both the comparator and the promotion confirmation end to end.
    bq = _uncomment(_spec_module_source(root, "bootstrap_qualification"))
    if "fn materializer_output_tree(&self) -> Sha256Digest" not in bq:
        out.append("spec/bootstrap_qualification/verify.rs: VerifiedTier0Qualification exposes no "
                   "materializer_output_tree() accessor for the cross-run comparator")
    if "AUTHORITATIVE_WORKFLOW_PATH" not in bq:
        out.append("spec/bootstrap_qualification/evidence.rs: declares no AUTHORITATIVE_WORKFLOW_PATH "
                   "constant for promotion confirmation")

    # Builder-identity perimeter (5.5E6c2): typed Python authority, the runtime
    # binding, the authoritative-runtime rule, and the v2 format bump.
    for needed in ("AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE", "struct PythonRelease",
                   "struct BootstrapRuntimeBinding", "AuthoritativeBootstrapRuntimeMismatch"):
        if needed not in bq:
            out.append(f"spec/bootstrap_qualification: missing builder-identity item {needed!r}")
    if "fn bootstrap_runtime(&self) -> &BootstrapRuntimeBinding" not in bq:
        out.append("spec/bootstrap_qualification/verify.rs: VerifiedTier0Qualification exposes no "
                   "bootstrap_runtime() accessor")
    rc_raw = _bootstrap_rust_source(root, "receiptcheck")
    if 'expect_exact("BATPAK-TIER0-QUALIFICATION/2")' not in rc_raw:
        out.append("bootstrap/receiptcheck.rs: does not parse the v2 qualification magic")
    if 'expect_exact("BATPAK-TIER0-QUALIFICATION/1")' in rc_raw:
        out.append("bootstrap/receiptcheck.rs: still parses the retired v1 qualification magic")
    for needed in ("check_bundle_shape", "probe_python", "python-release",
                   "--python-executable"):
        if needed not in rc_raw:
            out.append(f"bootstrap/receiptcheck.rs: missing E6c2 perimeter piece {needed!r}")
    st_raw = _bootstrap_source(root, "selftest")
    if '"BATPAK-TIER0-QUALIFICATION/2"' not in st_raw:
        out.append("bootstrap/selftest.py: producer does not emit the v2 qualification magic")

    sc = _bootstrap_rust_source(root, "seedcheck")
    if "check_tier0_cross_run" not in sc or "compare_runs(" not in sc:
        out.append("bootstrap/seedcheck.rs: does not execute the cross-run comparator")
    if "confirm_promotion(" not in sc:
        out.append("bootstrap/seedcheck.rs: does not execute the promotion confirmation")
    return out


def workflow_pinning_findings(root: Path) -> list[str]:
    """Every `uses:` in a qualification workflow or local composite action must
    reference immutable-content code (5.5E6c2): a local `./` action (bytes bound
    by the git tree), an external action/reusable-workflow pinned by a full 40-hex
    commit, or a docker action pinned by a sha256 digest. Movable tags, branches,
    expressions, and abbreviated SHAs are refused. The qualification workflow's
    setup-python version projects the typed AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE."""
    out: list[str] = []
    wf_dir = root / ".github" / "workflows"
    act_dir = root / ".github" / "actions"
    files: list[Path] = []
    if wf_dir.is_dir():
        for pat in ("*.yml", "*.yaml"):
            files.extend(sorted(wf_dir.glob(pat)))
    if act_dir.is_dir():
        for pat in ("action.yml", "action.yaml"):
            files.extend(sorted(act_dir.rglob(pat)))
    uses_re = re.compile(r'^\s*(?:-\s*)?uses:\s*(\S+)')
    full_sha = re.compile(r'^[0-9a-fA-F]{40}$')
    docker_digest = re.compile(r'^docker://[^@]+@sha256:[0-9a-f]{64}$')
    for f in files:
        rel = f.relative_to(root).as_posix()
        for i, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            m = uses_re.match(line)
            if not m:
                continue
            ref = m.group(1).strip().strip('"').strip("'")
            if ref.startswith(("./", "../")):
                continue
            if ref.startswith("docker://"):
                if not docker_digest.match(ref):
                    out.append(f"{rel}:{i}: docker action {ref!r} is not sha256-digest pinned")
                continue
            if "@" not in ref:
                out.append(f"{rel}:{i}: uses {ref!r} has no pinned ref")
                continue
            if not full_sha.match(ref.rsplit("@", 1)[1]):
                out.append(f"{rel}:{i}: uses {ref!r} is not pinned by a full 40-hex commit "
                           "(movable tag, branch, expression, or abbreviated SHA)")
    wf = wf_dir / "msvc-qualification.yml"
    if wf.is_file():
        bq = _spec_module_source(root, "bootstrap_qualification")
        m = re.search(r"AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE:\s*PythonRelease\s*=\s*"
                      r"PythonRelease\s*\{\s*major:\s*(\d+),\s*minor:\s*(\d+),\s*patch:\s*(\d+)",
                      bq)
        if not m:
            out.append("spec/bootstrap_qualification/evidence.rs: cannot read "
                       "AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE for the workflow projection check")
        else:
            want = f"{m.group(1)}.{m.group(2)}.{m.group(3)}"
            pv = re.search(r'python-version:\s*"([^"]+)"', wf.read_text(encoding="utf-8"))
            if not pv:
                out.append("msvc-qualification.yml: no pinned setup-python python-version")
            elif pv.group(1) != want:
                out.append(f"msvc-qualification.yml: setup-python python-version {pv.group(1)!r} "
                           f"does not equal the typed authority {want!r}")
        # Isolated-mode law (Wave-2 seam closure): every bootstrap gate
        # invocation in the qualification workflow must run `python -I`
        # (isolated: no site packages, no PYTHON* env, no implicit sys.path
        # entry) so accidental site-package access can neither rescue nor
        # sabotage a gate. The tools load each other by absolute path, so
        # isolation costs them nothing; a plain `python` here silently
        # re-admits the ambient interpreter environment into the perimeter.
        for i, line in enumerate(wf.read_text(encoding="utf-8").splitlines(), 1):
            if not re.search(r'bootstrap/(?:project|audit|freeze|selftest)\.py', line):
                continue
            if not re.search(r'\bpython(?:3)?\b', line):
                continue
            if not re.search(r'\bpython(?:3)?\s+-I\b', line):
                out.append(f"msvc-qualification.yml:{i}: bootstrap gate invocation without "
                           "isolated mode (python -I is law for bootstrap gates)")
    else:
        out.append(".github/workflows/msvc-qualification.yml: missing (the hosted "
                   "qualification workflow is a required, audited input)")
    # The isolated-mode law also binds the authoritative runbooks: a plain
    # `python bootstrap/...` instruction molts back into practice even when
    # the workflow stays lawful.
    for runbook in ("README.md", "bootstrap/README.md"):
        rb = root / runbook
        if not rb.is_file():
            continue
        for i, line in enumerate(rb.read_text(encoding="utf-8").splitlines(), 1):
            if not re.search(r'\bpython(?:3)?\s+bootstrap/', line):
                continue
            if not re.search(r'\bpython(?:3)?\s+-I\s+bootstrap/', line):
                out.append(f"{runbook}:{i}: runbook instructs a plain-python bootstrap "
                           "invocation (python -I is law for bootstrap gates)")
    return out


def python_tooling_findings(root: Path) -> list[str]:
    """The Python tooling plane must stay honest about the bootstrap runtime.
    `.python-version` PROJECTS the typed authority
    AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE (spec/bootstrap_qualification/evidence.rs) —
    it is never a second authority, so its MAJOR.MINOR.PATCH is parsed FROM the
    constant, never hardcoded. `pyproject.toml` owns only the development tooling
    plane: its `requires-python` must agree with that major.minor, and it must
    NOT declare a populated `[project].dependencies` list — the bootstrap runtime
    stays standard-library-only (a lawful dev dependency group is fine)."""
    out: list[str] = []
    bq_path = root / "spec/bootstrap_qualification/evidence.rs"
    if not bq_path.is_file():
        out.append("spec/bootstrap_qualification/evidence.rs: missing; cannot resolve the typed "
                   "AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE authority")
        return out
    bq = _spec_module_source(root, "bootstrap_qualification")
    m = re.search(r"AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE[^;]*major:\s*(\d+),"
                  r"[^;]*minor:\s*(\d+),[^;]*patch:\s*(\d+)", bq, re.S)
    if not m:
        out.append("spec/bootstrap_qualification/evidence.rs: cannot read "
                   "AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE for the python-tooling projection check")
        return out
    want_full = f"{m.group(1)}.{m.group(2)}.{m.group(3)}"
    want_mm = f"{m.group(1)}.{m.group(2)}"

    # (a) .python-version projects the exact MAJOR.MINOR.PATCH of the authority.
    pv_path = root / ".python-version"
    if not pv_path.is_file():
        out.append(".python-version: missing (must project "
                   "AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE)")
    else:
        got = pv_path.read_text(encoding="utf-8").strip()
        if got != want_full:
            out.append(f".python-version: {got!r} does not equal the typed authority "
                       f"{want_full!r}")

    # (b) pyproject.toml owns only the dev tooling plane: requires-python agrees
    # with the authority's major.minor, and no populated runtime dependency list.
    pp_path = root / "pyproject.toml"
    if not pp_path.is_file():
        out.append("pyproject.toml: missing (development tooling manifest)")
    else:
        pp = pp_path.read_text(encoding="utf-8")
        rp = re.search(r'requires-python\s*=\s*"([^"]+)"', pp)
        if not rp:
            out.append("pyproject.toml: no requires-python declared")
        elif want_mm not in rp.group(1):
            out.append(f"pyproject.toml: requires-python {rp.group(1)!r} is not consistent "
                       f"with the typed authority major.minor {want_mm!r}")
        dep = re.search(r'^\s*dependencies\s*=\s*\[(.*?)\]', pp, re.S | re.M)
        if dep and re.search(r'["\']', dep.group(1)):
            out.append("pyproject.toml: [project].dependencies declares runtime packages; "
                       "the bootstrap runtime must stay standard-library-only")

    # (c) uv.lock is a required, independently checked input (F4 prelude): the
    # dev-tooling gate runs from it with hash verification, so a deleted or
    # manifest-divergent lock silently un-pins the tool supply chain.
    lock_path = root / "uv.lock"
    if not lock_path.is_file():
        out.append("uv.lock: missing (the locked dev-tooling environment is a "
                   "required, audited input)")
    if lock_path.is_file() and pp_path.is_file():
        import tomllib
        try:
            lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as exc:
            lock = None
            out.append(f"uv.lock: unparseable ({exc})")
        try:
            manifest = tomllib.loads(pp_path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as exc:
            manifest = None
            out.append(f"pyproject.toml: unparseable ({exc})")
        if lock is not None and manifest is not None:
            declared = set()
            for req in manifest.get("dependency-groups", {}).get("dev", []):
                m2 = re.match(r"[A-Za-z0-9_.-]+", str(req))
                if m2:
                    declared.add(m2.group(0).lower().replace("_", "-"))
            locked = {str(pkg.get("name", "")).lower().replace("_", "-")
                      for pkg in lock.get("package", [])}
            for name in sorted(declared - locked):
                out.append(f"uv.lock: declared dev tool {name!r} is not locked "
                           "(lock-to-manifest parity)")
            if "requires-python" not in lock:
                out.append("uv.lock: carries no requires-python (cannot bind the "
                           "lock to the pinned interpreter line)")
    return out


# --- Toolchain authority (5.5E3a) --------------------------------------------
# spec/toolchain/types.rs owns every toolchain value. This auditor independently
# reconstructs the tracked projection's bytes and refuses any hidden owner: a
# hardcoded resolver/edition/MSRV/channel literal in the materializer or a
# hardcoded rustc edition in the harness is a second authority.
A_TRIPLE_SHAPE = re.compile(r"[a-z0-9_]+(-[a-z0-9_]+){2,}")


A_TC_PROFILE_SPELLING = {"Minimal": "minimal"}
A_TC_COMPONENT_SPELLING = {"Clippy": "clippy", "Rustfmt": "rustfmt"}


def toolchain_profile(root: Path) -> dict:
    src = _uncomment((root / "spec/toolchain/types.rs").read_text(encoding="utf-8"))
    release = re.search(
        r"exact_rust_release: RustRelease \{ major: (\d+), minor: (\d+), patch: (\d+) \}", src)
    floor = re.search(
        r"rust_version_floor: RustVersionFloor \{ major: (\d+), minor: (\d+) \}", src)
    edition = re.search(r"edition: RustEdition::Rust(\d+)", src)
    resolver = re.search(r"cargo_resolver: CargoResolver::V(\d+)", src)
    profile = re.search(r"rustup_profile: RustupProfile::(\w+)", src)
    comps = re.search(
        r"pub const ALL: &'static \[RustupComponent\] =\s*&\[([^\]]*)\]", src)
    enum_body = re.search(r"pub enum RustupComponent \{(.*?)\n\}", src, re.S)
    # The semantic environments live on the typed enum in architecture.rs;
    # the auditor reads the authored spelling arms, never a copied list.
    arch = _uncomment(_spec_module_source(root, "architecture"))
    spellings = re.findall(
        r'QualificationEnvironment::(\w+) => "([^"]+)",', arch)
    return {
        "exact_parts": tuple(int(x) for x in release.groups()) if release else None,
        "floor_parts": tuple(int(x) for x in floor.groups()) if floor else None,
        "exact": ".".join(release.groups()) if release else "",
        "floor": ".".join(floor.groups()) if floor else "",
        "edition": edition.group(1) if edition else "",
        "resolver": resolver.group(1) if resolver else "",
        "profile": A_TC_PROFILE_SPELLING.get(profile.group(1), "") if profile else "",
        "components": [A_TC_COMPONENT_SPELLING.get(v, "")
                       for v in re.findall(r"RustupComponent::(\w+)", comps.group(1))]
                      if comps else [],
        "component_all": re.findall(r"RustupComponent::(\w+)", comps.group(1))
                         if comps else [],
        "component_variants": re.findall(r"^\s{4}(\w+),", enum_body.group(1), re.M)
                              if enum_body else [],
        "environment_spellings": dict(spellings),
    }


def toolchain_findings(root: Path) -> list[str]:
    t = toolchain_profile(root)
    out: list[str] = []
    for key in ("exact", "floor", "edition", "resolver", "profile"):
        if not t[key]:
            out.append(f"spec/toolchain/types.rs authors no toolchain {key}")
    if not t["components"]:
        out.append("spec/toolchain/types.rs authors no required components")
    # RustupComponent::ALL is the ONE component denominator: every declared
    # variant appears in it exactly once. Dropping a component from ALL while
    # the projection moves in lockstep would leave a declared-but-unconsumed
    # variant — the TestFixture defect wearing a toolbelt.
    for missing in sorted(set(t["component_variants"]) - set(t["component_all"])):
        out.append(f"declared toolchain component {missing} is omitted from "
                   "RustupComponent::ALL, the one component inventory")
    for phantom in sorted(set(t["component_all"]) - set(t["component_variants"])):
        out.append(f"RustupComponent::ALL names {phantom}, which the enum "
                   "does not declare")
    if len(t["component_all"]) != len(set(t["component_all"])):
        out.append("RustupComponent::ALL repeats a component")
    if not t["environment_spellings"]:
        out.append("spec/architecture/types.rs declares no qualification environments")
    # exact >= floor, never "same minor": a newer qualifying compiler that
    # preserves the MSRV floor is ordinary; one below the floor would qualify
    # a foundation its own consumers may lawfully refuse.
    if t["exact_parts"] and t["floor_parts"] and t["exact_parts"][:2] < t["floor_parts"]:
        out.append(
            f"exact_rust_release {t['exact']} is below the declared MSRV "
            f"floor {t['floor']}; the qualifying compiler must satisfy the "
            "floor the generated workspace claims")
    # The tracked root projection opens with its machine-readable provenance
    # line (5.5E4a): the comment is part of the exact projected bytes.
    want = ('# generated from spec/toolchain/types.rs by bootstrap/project.py; do not edit\n'
            '[toolchain]\nchannel = "{}"\nprofile = "{}"\ncomponents = [{}]\n'
            .format(t["exact"], t["profile"],
                    ", ".join(f'"{c}"' for c in t["components"])))
    tracked = root / "rust-toolchain.toml"
    if not tracked.is_file():
        out.append("tracked rust-toolchain.toml is absent; the toolchain "
                   "projection selects the compiler before the spec compiles")
    elif tracked.read_text(encoding="utf-8") != want:
        out.append("tracked rust-toolchain.toml does not equal the "
                   "deterministic projection of ToolchainProfile "
                   "(owner: spec/toolchain/types.rs; repair: regenerate, never hand-edit)")
    mat = _bootstrap_rust_source(root, "materialize")
    for pattern, what in ((r'resolver = \\"[0-9]', "resolver"),
                          (r'edition = \\"[0-9]', "edition"),
                          (r'rust-version = \\"[0-9]', "rust-version"),
                          (r'channel = \\"[0-9]', "channel")):
        if re.search(pattern, mat):
            out.append(f"bootstrap/materialize.rs hardcodes a {what} literal; "
                       "spec/toolchain/types.rs ToolchainProfile is the owner")
    harness = _bootstrap_source(root, "selftest")
    if re.search(r'"--edition", "[0-9]', harness):
        out.append("bootstrap/selftest.py hardcodes a rustc edition; "
                   "spec/toolchain/types.rs ToolchainProfile is the owner")
    # Environment membership dissolved into construction (5.5E3a1): a QUAL
    # row carries a closed QualificationEnvironment variant, so the auditor
    # checks the authored SPELLINGS and that rows name declared variants.
    for variant, spelling in t["environment_spellings"].items():
        if A_TRIPLE_SHAPE.fullmatch(spelling):
            out.append(f"QualificationEnvironment::{variant} spells {spelling!r}, "
                       "which is shaped like a physical target triple; "
                       "environments answer WHAT holds, never WHERE a binary ran")
    arch = _spec_module_source(root, "architecture")
    for pkg, profile, variant, _gates in G_QUAL_ROW.findall(arch):
        if t["environment_spellings"] and variant not in t["environment_spellings"]:
            out.append(f"QualificationProfile {pkg}:{profile} names environment "
                       f"{variant}, which the QualificationEnvironment enum "
                       "does not declare")
    return out


# --- Wave-2 AST topology law over the bootstrap Python plane -----------------
# A structural law over the bootstrap Python plane (three packages + four entry
# shims), enforced by re-parsing every module with ast. It authors the first
# import-provenance enforcement and the first size ratchet: entry shims stay
# thin, package modules have no import-time effects, no file mutates sys.path,
# no package statically imports a sibling bootstrap tool (only the dynamic
# load() is lawful), the intra-package import graph stays acyclic, every import
# resolves to the standard library, and no file drifts past its ratchet ceiling.
_TOPOLOGY_TOOLS = ("audit", "project", "selftest", "freeze")
_TOPOLOGY_PACKAGES = ("audit", "project", "selftest")

# R8 size ratchet: measured line counts of every file over 1000 lines, rounded
# UP to the next multiple of 100. Every other file in scope has ceiling 1000.
# tier0.py grows in this change; its ceiling is set from the post-edit count.
_TOPOLOGY_SIZE_CEILINGS: dict[str, int] = {
    "bootstrap/audit/__init__.py": 1200,
    "bootstrap/audit/architecture.py": 1400,
    "bootstrap/audit/catalogs.py": 1300,
    # 1300 -> 1400 (Wave-2 seam closure ruling): the isolated-mode workflow
    # law landed here, its ruled home beside the other workflow projections.
    # 1400 -> 1500 (F4-prelude ruling): the hardened topology matcher, the
    # runbook isolation rule, and the uv.lock parity law all landed here.
    "bootstrap/audit/tier0.py": 1500,
    # 2100 -> 2300 (SGB tranche 4b ruling): the source-grammar hostile/neuter
    # battery landed beside the seedcheck-mutation probes, its ruled home.
    "bootstrap/selftest/catalogs.py": 2300,
    "bootstrap/selftest/corpus.py": 1100,
    "bootstrap/selftest/inventory.py": 1600,
    "bootstrap/selftest/proof.py": 1100,
    "bootstrap/selftest/tier0.py": 2500,
}


def _topology_files(root: Path) -> list[tuple[str, Path]]:
    """Scope set S: every *.py under bootstrap/ except __pycache__ -- the four
    entry shims plus the flat package modules, as (posix-relpath, path)."""
    boot = root / "bootstrap"
    out: list[tuple[str, Path]] = []
    for name in _TOPOLOGY_TOOLS:
        shim = boot / f"{name}.py"
        if shim.is_file():
            out.append((shim.relative_to(root).as_posix(), shim))
    for name in _TOPOLOGY_PACKAGES:
        pkg = boot / name
        if pkg.is_dir():
            for p in pkg.rglob("*.py"):
                if "__pycache__" not in p.parts:
                    out.append((p.relative_to(root).as_posix(), p))
    out.sort(key=lambda item: item[0])
    return out


# Mutating methods on a sys.path alias -- `p.append(...)` is a mutation exactly
# as `sys.path.append(...)` is (H4).
_TOPOLOGY_PATH_METHODS = frozenset({
    "append", "insert", "extend", "remove", "clear", "pop"})


def _topology_syspath_violations(tree: ast.AST, rel: str) -> list[str]:
    """R3 (H4): sys.path mutation via `<sys-alias>.path` (an assignment target,
    method receiver, or call arg), OR via a name bound by `from sys import path
    [as X]` used as an assign/augmented target, a mutating-method receiver, or a
    call argument. Two alias families are collected first -- names bound to
    sys.path, and module aliases for `sys` (seeded with `sys` so bare sys.path
    still bites) -- so a rename cannot smuggle a mutation past the matcher."""
    out: list[str] = []
    seen: set[int] = set()
    path_names: set[str] = set()
    sys_aliases: set[str] = {"sys"}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == "sys":
                    sys_aliases.add(alias.asname or "sys")
        elif isinstance(node, ast.ImportFrom) and not (node.level or 0) and node.module == "sys":
            for alias in node.names:
                if alias.name == "path":
                    path_names.add(alias.asname or "path")

    def is_sys_path(node: ast.AST) -> bool:
        return (isinstance(node, ast.Attribute) and node.attr == "path"
                and isinstance(node.value, ast.Name) and node.value.id in sys_aliases)

    def is_path_alias(node: ast.AST) -> bool:
        return isinstance(node, ast.Name) and node.id in path_names

    def flag(node: ast.AST) -> None:
        if node.lineno not in seen:
            seen.add(node.lineno)
            out.append(f"{rel}:{node.lineno}: sys.path mutation")

    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            if isinstance(node.func, ast.Attribute):
                recv = node.func.value
                if is_sys_path(recv):
                    flag(recv)
                if is_path_alias(recv) and node.func.attr in _TOPOLOGY_PATH_METHODS:
                    flag(recv)
            for arg in list(node.args) + [k.value for k in node.keywords]:
                if is_sys_path(arg) or is_path_alias(arg):
                    flag(arg)
        elif isinstance(node, ast.Assign):
            for tgt in node.targets:
                base = tgt.value if isinstance(tgt, ast.Subscript) else tgt
                if is_sys_path(base) or is_path_alias(base):
                    flag(base)
        elif isinstance(node, (ast.AugAssign, ast.AnnAssign)):
            tgt = node.target
            base = tgt.value if isinstance(tgt, ast.Subscript) else tgt
            if is_sys_path(base) or is_path_alias(base):
                flag(base)
    return out


def _topology_import_violations(tree: ast.AST, rel: str, pkg: str | None) -> list[str]:
    """R4/R5 (cross-package, absolute-sibling) and R7 (stdlib-only). Import and
    ImportFrom fold into one (first-segment, spelling) stream so each rule has a
    single guard. Relative imports (level >= 1) stay inside the package and pass
    both."""
    out: list[str] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            refs = [(alias.name.split(".")[0], alias.name) for alias in node.names]
        elif isinstance(node, ast.ImportFrom):
            if node.level and node.level >= 1:
                continue
            mod = node.module or ""
            refs = [(mod.split(".")[0], mod)]
        else:
            continue
        for first, spelling in refs:
            if pkg is not None and first in _TOPOLOGY_TOOLS:
                out.append(f"{rel}:{node.lineno}: static import of sibling "
                           f"bootstrap tool '{first}' (only dynamic load() is lawful)")
            elif first not in sys.stdlib_module_names:
                out.append(f"{rel}:{node.lineno}: non-stdlib import "
                           f"'{spelling}' (bootstrap runtime is stdlib-only)")
    return out


# R2 treats a module-level Assign as an import-time EFFECT only when its value
# performs an impure call: a Call whose callee is not a pure constructor. Pure
# constructors (re.compile plus the stdlib container/scalar builtins) and lambda
# bodies (deferred, not run at import) build module-level constants with no side
# effect -- the pervasive precompiled-regex / frozenset / dict-of-lambda idiom
# the census under-counted (see report). A call that reaches the file system or
# a local builder still trips, and is grandfathered by name below.
_TOPOLOGY_PURE_BUILTINS = frozenset({
    "frozenset", "set", "tuple", "list", "dict", "bytes", "bytearray",
    "str", "int", "float", "bool", "complex", "range"})

# Grandfathered import-time bindings (H3). The exemption binds to an EXACT value
# shape: each entry maps (relpath, name) to the sha256 hexdigest of
# ast.dump(value), computed from the canonical a8bc571e sources. Rebinding a
# whitelisted name to a different value no longer inherits the exemption.
_TOPOLOGY_EFFECT_WHITELIST: dict[tuple[str, str], str] = {
    # TOOLCHAIN_EDITION = _toolchain_edition()  (reads the typed toolchain edition)
    ("bootstrap/selftest/core.py", "TOOLCHAIN_EDITION"):
        "617504c69a1910ee59aadb7ade254594dad79efd0dbd0e208b0e0540db709263",
    # HERE = Path(__file__).resolve().parent.parent
    ("bootstrap/selftest/core.py", "HERE"):
        "c2ce60a0a3687f5b2ed5498c1cafb122c267ebcd1ceba22d77997e6b0dbe0053",
    # WINDOWING_ROWS = _isa_class_row("Windowing", "QueryDataflow", "Rows")
    ("bootstrap/selftest/domains.py", "WINDOWING_ROWS"):
        "3ed3d0de723cebd36e617afc4bf865eb4044175ab4f993617613254e047042b8",
    # WINDOWING_OUTPUTS = _isa_class_row("Windowing", "QueryDataflow", "EmittedOutputs")
    ("bootstrap/selftest/domains.py", "WINDOWING_OUTPUTS"):
        "0f630ede7b4c2999c1cdc6a632308032dc2c6f1963eb2115bb48a9eb40b092ef",
    # VIEW_RENDERERS = { ... the registry-driven renderer table (incl. the two
    # F4-prelude catalog renderers) ... }
    ("bootstrap/project/__init__.py", "VIEW_RENDERERS"):
        "562e513a54c1554184200282fe87a84f3bc23430176bdb96739cea33d0752b75",
}


def _topology_pure_ctor(func: ast.AST) -> bool:
    if isinstance(func, ast.Name):
        return func.id in _TOPOLOGY_PURE_BUILTINS
    return (isinstance(func, ast.Attribute) and func.attr == "compile"
            and isinstance(func.value, ast.Name) and func.value.id == "re")


def _topology_impure(node: ast.AST) -> bool:
    """True if evaluating `node` at import time makes an impure call. A lambda
    body is deferred, so a lambda is pure to construct; a Call is pure only when
    its callee is a pure constructor and its evaluated arguments are pure too."""
    if isinstance(node, ast.Lambda):
        return False
    if isinstance(node, ast.Call):
        if not _topology_pure_ctor(node.func):
            return True
        parts = list(node.args) + [k.value for k in node.keywords]
        return any(_topology_impure(p) for p in parts)
    return any(_topology_impure(c) for c in ast.iter_child_nodes(node))


# H1: the only top-level If packages authored to run at import time.
# _TOPOLOGY_SELFREG_FILES are the two package doors that self-register into
# sys.modules for loader paths that exec them without a prior sys.modules entry.
_TOPOLOGY_SELFREG_FILES = frozenset({
    "bootstrap/project/__init__.py",
    "bootstrap/selftest/__init__.py",
})


def _topology_is_main_guard(test: ast.AST) -> bool:
    """H1(a): the exact `if __name__ == "__main__":` guard shape --
    Compare(Name('__name__'), [Eq], [Constant('__main__')]) -- and nothing
    looser (a bare `if __name__:` or `if __name__ == x:` is rejected)."""
    return (isinstance(test, ast.Compare)
            and isinstance(test.left, ast.Name) and test.left.id == "__name__"
            and len(test.ops) == 1 and isinstance(test.ops[0], ast.Eq)
            and len(test.comparators) == 1
            and isinstance(test.comparators[0], ast.Constant)
            and test.comparators[0].value == "__main__")


def _topology_is_self_register(test: ast.AST) -> bool:
    """H1(b): the loader self-registration block's FULL test shape --
    Compare(Name('__name__'), [NotIn], [Attribute(value=Name('sys'),
    attr='modules')]) -- verified via ast.dump on the canonical door files."""
    return (isinstance(test, ast.Compare)
            and isinstance(test.left, ast.Name) and test.left.id == "__name__"
            and len(test.ops) == 1 and isinstance(test.ops[0], ast.NotIn)
            and len(test.comparators) == 1
            and isinstance(test.comparators[0], ast.Attribute)
            and test.comparators[0].attr == "modules"
            and isinstance(test.comparators[0].value, ast.Name)
            and test.comparators[0].value.id == "sys")


def _topology_stmt_ok(stmt: ast.stmt, rel: str) -> bool:
    if isinstance(stmt, (ast.Import, ast.ImportFrom, ast.FunctionDef,
                         ast.AsyncFunctionDef, ast.ClassDef)):
        return True
    if isinstance(stmt, ast.Expr):
        return isinstance(stmt.value, ast.Constant)
    if isinstance(stmt, ast.Assert):
        # H2: an assert is lawful only if neither test nor message calls impurely.
        if _topology_impure(stmt.test):
            return False
        return not (stmt.msg is not None and _topology_impure(stmt.msg))
    if isinstance(stmt, ast.If):
        # H1: a top-level If is lawful ONLY as the exact main guard (any file),
        # or as the self-registration block in the two package doors.
        if _topology_is_main_guard(stmt.test):
            return True
        return rel in _TOPOLOGY_SELFREG_FILES and _topology_is_self_register(stmt.test)
    if isinstance(stmt, (ast.Assign, ast.AnnAssign)):
        targets = stmt.targets if isinstance(stmt, ast.Assign) else [stmt.target]
        names = [t.id for t in targets if isinstance(t, ast.Name)]
        if "__all__" in names:
            return True
        if stmt.value is None:
            return True
        # H3: a whitelisted (relpath, name) is exempt ONLY when ast.dump(value)
        # hashes to the recorded digest; on drift the value is judged as impure.
        for n in names:
            exempt_digest = _TOPOLOGY_EFFECT_WHITELIST.get((rel, n))
            if exempt_digest is not None:
                value_digest = hashlib.sha256(ast.dump(stmt.value).encode("utf-8")).hexdigest()
                if value_digest != exempt_digest:
                    return not _topology_impure(stmt.value)
                return True
        return not _topology_impure(stmt.value)
    return False


def _topology_toplevel_violations(tree: ast.Module, rel: str) -> list[str]:
    """R2: package modules carry no import-time effects outside the whitelist."""
    out: list[str] = []
    for stmt in tree.body:
        if not _topology_stmt_ok(stmt, rel):
            out.append(f"{rel}:{stmt.lineno}: import-time effect "
                       f"({type(stmt).__name__}) outside the whitelist")
    return out


def _topology_find_cycle(nodes: set, edges: dict) -> list | None:
    color: dict = {}
    stack: list = []
    box: dict = {"cycle": None}

    def visit(u) -> bool:
        color[u] = 1
        stack.append(u)
        for v in sorted(edges.get(u, ())):
            state = color.get(v, 0)
            if state == 1:
                box["cycle"] = stack[stack.index(v):]
                return True
            if state == 0 and visit(v):
                return True
        stack.pop()
        color[u] = 2
        return False

    for n in sorted(nodes):
        if color.get(n, 0) == 0 and visit(n):
            break
    return box["cycle"]


def _topology_cycle_violations(root: Path) -> list[str]:
    """R6: the intra-package relative-import digraph must be acyclic."""
    out: list[str] = []
    boot = root / "bootstrap"
    for pkg in _TOPOLOGY_PACKAGES:
        pdir = boot / pkg
        if not pdir.is_dir():
            continue
        modules = {p.stem: p for p in sorted(pdir.glob("*.py"))}
        edges: dict = {}
        for stem, path in modules.items():
            try:
                tree = ast.parse(path.read_text(encoding="utf-8"))
            except SyntaxError:
                continue
            targets: set = set()
            for node in ast.walk(tree):
                if isinstance(node, ast.ImportFrom) and node.level == 1:
                    if node.module:
                        head = node.module.split(".")[0]
                        if head in modules:
                            targets.add(head)
                    else:
                        for alias in node.names:
                            if alias.name in modules:
                                targets.add(alias.name)
            targets.discard(stem)
            edges[stem] = targets
        cycle = _topology_find_cycle(set(modules), edges)
        if cycle:
            i = cycle.index(min(cycle))
            rot = cycle[i:] + cycle[:i]
            chain = " -> ".join(f"{m}.py" for m in rot + [rot[0]])
            out.append(f"bootstrap/{pkg}: import cycle: {chain}")
    return out


def _topology_sort_key(finding: str) -> tuple:
    head, _, rest = finding.partition(":")
    m = re.match(r"\s*(\d+):", rest)
    return (head, int(m.group(1)) if m else 0, finding)


def bootstrap_topology_findings(root: Path) -> list[str]:
    """The bootstrap Python plane must hold its structural shape. Eight rules
    over scope set S (every bootstrap *.py except __pycache__): R1 thin entry
    shims (<= 90 lines), R2 no import-time effects in package modules, R3 no
    sys.path mutation, R4/R5 no static sibling-tool imports, R6 acyclic
    intra-package imports, R7 stdlib-only imports, R8 the size ratchet. Empty
    list == PASS; a SyntaxError is itself a finding."""
    out: list[str] = []
    for rel, path in _topology_files(root):
        text = path.read_text(encoding="utf-8")
        count = text.count("\n")
        is_shim = rel.count("/") == 1
        # R1 thin-CLI.
        if is_shim and count > 90:
            out.append(f"{rel}: entry shim has {count} lines (> 90); "
                       "entry shims must stay thin")
        # R8 size-ratchet.
        ceiling = _TOPOLOGY_SIZE_CEILINGS.get(rel, 1000)
        if count > ceiling:
            out.append(f"{rel}: {count} lines exceeds its ratchet ceiling "
                       f"{ceiling} (raise the ceiling only by ruling)")
        try:
            tree = ast.parse(text)
        except SyntaxError as exc:
            out.append(f"{rel}:{exc.lineno or 0}: syntax error: {exc.msg}")
            continue
        # R3 (all files), R4/R5 + R7 (all files; sibling rule package-only).
        out.extend(_topology_syspath_violations(tree, rel))
        out.extend(_topology_import_violations(
            tree, rel, None if is_shim else rel.split("/")[1]))
        # R2 (package modules only; shims are loaders, exempt).
        if not is_shim:
            out.extend(_topology_toplevel_violations(tree, rel))
    # R6 (per package).
    out.extend(_topology_cycle_violations(root))
    out.sort(key=_topology_sort_key)
    return out


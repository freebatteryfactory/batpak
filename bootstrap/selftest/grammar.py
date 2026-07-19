"""The domain-directory source-grammar battery (SGB tranche 4b), split out of
catalogs.py on the F4 grammar proof per the ratified split-on-next-edit rule:
hostile trees for every filesystem grammar rule, plus the neuter probes that
prove each detector goes silent when blinded."""
from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
from pathlib import Path

from .catalogs import _seedcheck_exe
from .core import (
    HERE,
    must_replace,
    receipt,
)


def test_source_grammar(_audit) -> list[str]:
    """The domain-directory source grammar (SGB tranche 4b), proven by RUNNING
    seedcheck against grammar-hostile trees.

    Grammar rules are FILESYSTEM law: the gate scans whatever tree it is aimed
    at, so each probe aims one canonical binary (built once from pristine
    sources) at a mutated COPY. Building from the mutated tree would prove only
    that rustc refuses a broken module graph (E0761 ambiguous door, E0583
    unresolved module, an unresolvable deleted types.rs) -- the compiler's
    refusal, not the grammar gate's; the build/scan split is what lets every
    rule fire from the running gate. Each neuter probe blinds exactly ONE rule
    in a copy of grammar.rs, rebuilds, and reruns the SAME hostile mutation
    expecting a full PASS: a detector that cannot go silent when disabled was
    never the thing detecting."""
    findings: list[str] = []
    root = HERE.parent
    if not shutil.which("rustc"):
        receipt("source-grammar-fixtures", available=False, reason="rustc absent")
        return findings
    used_triples: set[str] = set()

    def grammar_tree() -> Path:
        # A COMPLETE tree copy: seedcheck's non-grammar checks scan required
        # docs, bootstrap tools, and the frozen manifest at runtime, and the
        # neuter probes assert a full PASS -- an incomplete sandbox would
        # redden for reasons no probe claims.
        tmp = Path(tempfile.mkdtemp(prefix="batpak-grammar-"))
        for sub in ("spec", "docs", "companion"):
            shutil.copytree(root / sub, tmp / sub)
        shutil.copytree(root / "bootstrap", tmp / "bootstrap",
                        ignore=shutil.ignore_patterns("__pycache__"))
        for rel in ("rust-toolchain.toml", "README.md", "AGENTS.md", "SPEC.sha256"):
            shutil.copy2(root / rel, tmp / rel)
        return tmp

    def scan(exe: Path, tree: Path) -> tuple[int, str]:
        proc = subprocess.run([str(exe), str(tree)], capture_output=True, text=True)
        return proc.returncode, proc.stdout + proc.stderr

    def planted(tree: Path, rel: str, data: bytes) -> None:
        # A planted file joins the sandbox manifest so R10 (manifest
        # completeness) stays satisfied and exactly the claimed rule sees it.
        (tree / rel).write_bytes(data)
        manifest = tree / "SPEC.sha256"
        digest = hashlib.sha256(data).hexdigest()
        manifest.write_text(manifest.read_text(encoding="utf-8") + f"{digest}  {rel}\n",
                            encoding="utf-8")

    def edit(tree: Path, rel: str, old: str, new: str, what: str) -> None:
        path = tree / rel
        path.write_text(must_replace(path.read_text(encoding="utf-8"), old, new, what),
                        encoding="utf-8")

    # Mutations shared verbatim by a hostile and its neuter probe, so the
    # two-sided law bites and silences the SAME tree shape.
    def mutate_sibling_door(tree: Path) -> None:
        planted(tree, "spec/gates.rs", b"// hostile: restored sibling door\n")

    def mutate_glob_reexport(tree: Path) -> None:
        edit(tree, "spec/corpus/mod.rs",
             "pub use types::{CURRENT_RECONCILIATION_EPOCH, ReconciliationEpoch};",
             "pub use types::*;", "glob_reexport")

    def mutate_dangling_authority(tree: Path) -> None:
        edit(tree, "spec/generated_views/registry.rs",
             '"spec/corpus/types.rs"', '"spec/nonexistent/types.rs"', "dangling_authority")

    def mutate_orphan_domain(tree: Path) -> None:
        # A complete-looking domain directory lib.rs never mounts. Both files
        # are manifested so R10 stays satisfied and only the census rule (R11)
        # sees the orphan.
        (tree / "spec/phantom").mkdir()
        planted(tree, "spec/phantom/mod.rs", b"mod types;\n")
        planted(tree, "spec/phantom/types.rs", b"// hostile: orphan domain noun carrier\n")

    def mutate_orphan_carrier(tree: Path) -> None:
        # A manifested child carrier no mod statement declares: R10 is
        # satisfied by the manifest row, so exactly R12 fires.
        planted(tree, "spec/corpus/stray.rs", b"// hostile: orphan carrier\n")

    exe_root = Path(tempfile.mkdtemp(prefix="batpak-grammar-exe-"))
    try:
        build_dir = exe_root / "canonical"
        build_dir.mkdir()
        exe, err = _seedcheck_exe(root, used_triples, build_dir)
        if exe is None:
            if err == "unavailable":
                print("selftest: source_grammar unavailable (no working linker)")
                receipt("source-grammar-fixtures", available=False, compiled=True,
                        executed=False, reason="no working linker for any attempted target")
                return findings
            receipt("source-grammar-fixtures", available=True, compiled=False,
                    reason="canonical seedcheck does not compile")
            findings.append(f"grammar_gate_compiles FAILED ({err.strip()[:300]!r})")
            return findings

        # The premise: a pristine, complete tree copy passes the WHOLE gate.
        # Without this every probe below could be red for free.
        base = grammar_tree()
        try:
            code, out = scan(exe, base)
            if code != 0:
                receipt("source-grammar-fixtures", available=True, compiled=True,
                        executed=True, passed=False,
                        reason="seedcheck already fails on a pristine tree copy")
                findings.append(f"grammar_baseline_passes FAILED ({out.strip()[:300]!r})")
                return findings
        finally:
            shutil.rmtree(base, ignore_errors=True)

        def hostile(name: str, mutate, needle: str) -> None:
            tmp = grammar_tree()
            try:
                mutate(tmp)
                code, out = scan(exe, tmp)
                if code == 0:
                    findings.append(f"{name} FAILED (seedcheck admitted the grammar mutation)")
                elif needle not in out:
                    findings.append(
                        f"{name} FAILED (wanted {needle!r}, got {out.strip()[:300]!r})")
            finally:
                shutil.rmtree(tmp, ignore_errors=True)

        # R1: a declared domain with no directory facade behind it.
        hostile("declared_domain_without_directory_is_rejected",
                lambda t: edit(t, "spec/lib.rs", "pub mod verification;",
                               "pub mod verification;\npub mod phantom;", "phantom_domain"),
                "declared domain phantom does not resolve to spec/phantom/mod.rs")
        # R2: the killed concept-door shape restored beside the directory.
        hostile("restored_sibling_door_is_rejected", mutate_sibling_door,
                "sibling door file spec/gates.rs exists")
        # R3: a domain stripped of its canonical noun carrier.
        hostile("missing_types_carrier_is_rejected",
                lambda t: (t / "spec/gates/types.rs").unlink(),
                "domain directory spec/gates/ carries no types.rs")
        # R4: the universal type drawer at the crate root.
        hostile("crate_root_type_drawer_is_rejected",
                lambda t: planted(t, "spec/types.rs", b"// hostile: universal type drawer\n"),
                "crate-root type drawer spec/types.rs exists")

        # R5: a generic helper drawer, mounted so R6 stays satisfied and the
        # forbidden NAME is what fires.
        def mutate_helper_drawer(t: Path) -> None:
            planted(t, "spec/corpus/helpers.rs", b"// hostile: generic helper drawer\n")
            edit(t, "spec/corpus/mod.rs", "mod types;", "mod types;\nmod helpers;",
                 "helper_drawer")
        hostile("forbidden_helper_drawer_is_rejected", mutate_helper_drawer,
                "forbidden module name spec/corpus/helpers.rs")
        # R6: a facade declaring a child that does not exist.
        hostile("facade_declaring_absent_child_is_rejected",
                lambda t: edit(t, "spec/corpus/mod.rs", "mod types;",
                               "mod types;\nmod phantom_piece;", "absent_child"),
                "declares mod phantom_piece; but no sibling")
        # R7: the noun carrier declared as a public module path.
        hostile("public_types_module_is_rejected",
                lambda t: edit(t, "spec/corpus/mod.rs", "mod types;", "pub mod types;",
                               "public_types"),
                "spec/corpus/mod.rs declares its types module public")
        # R8: the wildcard facade.
        hostile("glob_reexport_is_rejected", mutate_glob_reexport,
                "glob re-export in spec/corpus/mod.rs")
        # R9: a generated-view authority source pointing at a phantom carrier.
        hostile("dangling_view_authority_source_is_rejected", mutate_dangling_authority,
                "generated-view authority source spec/nonexistent/types.rs")
        # R10: a module file the frozen manifest never admitted.
        hostile("unmanifested_spec_source_is_rejected",
                lambda t: (t / "spec/corpus/stray.rs").write_bytes(b"// hostile: stray\n"),
                "spec source spec/corpus/stray.rs is missing from SPEC.sha256")
        # R11 (reverse of R1): a mod.rs-bearing domain directory lib.rs never
        # mounts -- invisible to the compiler and every census.
        hostile("orphan_domain_directory_is_rejected", mutate_orphan_domain,
                "orphan domain directory spec/phantom/ carries a mod.rs facade "
                "but spec/lib.rs declares no pub mod phantom")
        # R12 (reverse of R6): a domain child no facade mod statement declares.
        hostile("orphan_carrier_is_rejected", mutate_orphan_carrier,
                "orphan carrier spec/corpus/stray.rs is declared by no mod statement "
                "in spec/corpus/mod.rs; every domain child passes through the facade")

        def neuter(name: str, blind: str, mutate) -> None:
            # Blind ONE rule in a copied grammar.rs, rebuild, rerun the same
            # hostile mutation: the run must now PASS end to end.
            build_tree = grammar_tree()
            scan_tree = grammar_tree()
            ndir = exe_root / name
            ndir.mkdir()
            try:
                gpath = build_tree / "bootstrap/seedcheck/grammar.rs"
                gpath.write_text(
                    must_replace(gpath.read_text(encoding="utf-8"), blind, "if false {", name),
                    encoding="utf-8")
                blinded, nerr = _seedcheck_exe(build_tree, used_triples, ndir)
                if blinded is None:
                    if nerr == "unavailable":
                        print(f"selftest: {name} unavailable (no working linker)")
                        return
                    findings.append(f"{name} FAILED (blinded grammar gate does not "
                                    f"compile: {nerr.strip()[:300]!r})")
                    return
                mutate(scan_tree)
                code, out = scan(blinded, scan_tree)
                if code != 0:
                    findings.append(f"{name} FAILED (mutation still reddens the blinded "
                                    f"detector: {out.strip()[:300]!r})")
            finally:
                shutil.rmtree(build_tree, ignore_errors=True)
                shutil.rmtree(scan_tree, ignore_errors=True)

        neuter("neutered_sibling_door_rule_goes_silent",
               'if spec.join(format!("{name}.rs")).is_file() {', mutate_sibling_door)
        neuter("neutered_glob_reexport_rule_goes_silent",
               "if has_glob_reexport(&sanitized) {", mutate_glob_reexport)
        neuter("neutered_view_authority_rule_goes_silent",
               "if !root.join(&path).is_file() {", mutate_dangling_authority)
        neuter("neutered_orphan_domain_rule_goes_silent",
               "if !domains.iter().any(|domain| domain == name) {", mutate_orphan_domain)
        neuter("neutered_orphan_carrier_rule_goes_silent",
               "if !declared.iter().any(|(child, _)| child == stem) {", mutate_orphan_carrier)

        receipt("source-grammar-fixtures", available=True, compiled=True, executed=True,
                passed=not findings,
                target=", ".join(sorted(used_triples)) or "no triple resolved")
        return findings
    finally:
        shutil.rmtree(exe_root, ignore_errors=True)

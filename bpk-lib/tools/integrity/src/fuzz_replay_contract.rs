//! `fuzz-replay-contract` gate (GAUNT-PROOF-OF-PROOF #197, gates (b)+(c) and the
//! meta-gate hardening): the feature-INDEPENDENT structural mirror of the
//! fuzz-replay lockstep plus the anti-ceremony law.
//!
//! The in-binary meta-test (`crates/core/tests/fuzz_replay.rs::
//! fuzz_replay_covers_every_target`) lives behind `dangerous-test-hooks` and so
//! can be silently compiled out. This gate moves the four-leg lockstep OUT of the
//! feature-gated binary into `structural-check`, where no feature flag can hollow
//! it. The four legs, held in EXACT set-equality:
//!   * leg 1 — `fuzz/Cargo.toml` `[[bin]]` set == `fuzz_replay.rs` `dispatch_table`;
//!   * leg 2 — that set == `fuzz/regressions/manifest.yaml` `target` set;
//!   * leg 3 — every manifest fixture file exists on disk, and every on-disk
//!     regression file is listed (an unlisted file is ceremonial paperwork);
//!   * leg 4 — `class: semantic` targets keep >= 1 NON-EMPTY fixture, each with a
//!     `red_proof` naming a `#[test]` fn in a file carrying a `gauntlet_red_fixture`
//!     branch (ProductionFlip) that is registered in `gate_registry.rs`.
//! A 0-byte "regression" under a semantic target is the ceremony the gate exists
//! to kill (INV-FUZZ-REGRESSION-SEMANTIC): file presence alone never counts as a
//! historical RED receipt.

use crate::repo_surface::{ensure, resolve_repo_or_core_path};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

/// One manifest entry: a fuzz target, its class, and its committed fixtures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegressionManifestEntry {
    target: String,
    class: Class,
    fixtures: Vec<Fixture>,
}

/// A decoder target reds only on panic (empty boundary inputs are legal); a
/// semantic target must carry a non-empty, red-proven regression.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Class {
    Decoder,
    Semantic,
}

/// One committed fixture: the on-disk file name, the forbidden behavior it
/// reproduces, and (semantic only) the ProductionFlip test that proves it reds
/// the pre-fix decision core.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    file: String,
    forbidden: String,
    #[serde(default)]
    red_proof: Option<String>,
}

/// The repo-relative path to the fuzz-replay meta-test whose `dispatch_table` is
/// the single source of truth for the wired target set.
const FUZZ_REPLAY_REL: &str = "crates/core/tests/fuzz_replay.rs";
/// The repo-relative path to the gate registry every `red_proof` must appear in.
const GATE_REGISTRY_REL: &str = "tools/integrity/src/gate_registry.rs";

/// Production entry: validate the fuzz-replay lockstep against the live tree.
/// Returns the set of files consulted (manifest, `fuzz/Cargo.toml`,
/// `fuzz_replay.rs`, every regression fixture file) for the gate receipt.
pub(crate) fn check(repo_root: &Path) -> Result<BTreeSet<PathBuf>> {
    let fuzz_dir = repo_root.join("fuzz");
    let fuzz_replay_path = resolve_repo_or_core_path(repo_root, FUZZ_REPLAY_REL);
    let fuzz_replay_src = fs::read_to_string(&fuzz_replay_path)
        .with_context(|| format!("read {}", fuzz_replay_path.display()))?;
    let registry_path = repo_root.join(GATE_REGISTRY_REL);
    let registry_src = fs::read_to_string(&registry_path)
        .with_context(|| format!("read {}", registry_path.display()))?;

    check_over(&fuzz_dir, &fuzz_replay_src, &registry_src)?;

    let mut inputs = BTreeSet::new();
    inputs.insert(fuzz_dir.join("Cargo.toml"));
    inputs.insert(fuzz_dir.join("regressions").join("manifest.yaml"));
    inputs.insert(fuzz_replay_path);
    for fixture in regression_fixture_files(&fuzz_dir)? {
        inputs.insert(fixture);
    }
    Ok(inputs)
}

/// Testable core over a planted fuzz tree plus the two source texts that live
/// OUTSIDE `fuzz/` (so `check()` reads them and passes them in). `red_proof`
/// files are resolved relative to `fuzz_dir`'s parent (the repo root).
fn check_over(fuzz_dir: &Path, fuzz_replay_src: &str, registry_src: &str) -> Result<()> {
    let repo_root = fuzz_dir.parent().unwrap_or(fuzz_dir);

    // Leg 1 — [[bin]] set == dispatch_table set.
    let bins = declared_bin_targets(&fuzz_dir.join("Cargo.toml"));
    ensure(
        !bins.is_empty(),
        "fuzz-replay-contract (leg 1): no `[[bin]]` target parsed from fuzz/Cargo.toml \
         — the bin parser broke or the manifest moved; a zero-bin set would pass every \
         later leg vacuously.",
    )?;
    let bins: BTreeSet<String> = bins.into_iter().collect();

    let dispatched = dispatched_targets(fuzz_replay_src)?;
    ensure(
        !dispatched.is_empty(),
        format!(
            "fuzz-replay-contract (leg 1): no `dispatch_table` string literals extracted \
             from {FUZZ_REPLAY_REL} — the syn extractor rotted against the file's shape; \
             a zero-dispatch set would pass leg 1 vacuously."
        ),
    )?;
    assert_set_equality(
        &bins,
        &dispatched,
        "fuzz-replay-contract (leg 1)",
        |target| {
            format!(
                "`[[bin]]` target `{target}` in fuzz/Cargo.toml has no `dispatch_table` \
                 entry in {FUZZ_REPLAY_REL} — add a replay closure for it."
            )
        },
        |target| {
            format!(
                "`dispatch_table` in {FUZZ_REPLAY_REL} dispatches `{target}` with no \
                 `[[bin]]` in fuzz/Cargo.toml — remove the stale dispatcher or restore the bin."
            )
        },
    )?;

    // Leg 2 — manifest target set == bin set.
    let manifest_path = fuzz_dir.join("regressions").join("manifest.yaml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let entries: Vec<RegressionManifestEntry> =
        yaml_serde::from_str(&manifest_text).context("parse fuzz/regressions/manifest.yaml")?;
    let mut manifest_targets = BTreeSet::new();
    for entry in &entries {
        ensure(
            manifest_targets.insert(entry.target.clone()),
            format!(
                "fuzz-replay-contract (leg 2): duplicate target `{}` in \
                 fuzz/regressions/manifest.yaml — each fuzz target is declared exactly once.",
                entry.target
            ),
        )?;
    }
    assert_set_equality(
        &bins,
        &manifest_targets,
        "fuzz-replay-contract (leg 2)",
        |target| {
            format!(
                "`[[bin]]` target `{target}` is not declared in \
                 fuzz/regressions/manifest.yaml — every fuzz target needs a manifest entry \
                 (>= 1 committed regression fixture)."
            )
        },
        |target| {
            format!(
                "fuzz/regressions/manifest.yaml declares target `{target}` with no matching \
                 `[[bin]]` in fuzz/Cargo.toml — remove it or restore the bin."
            )
        },
    )?;

    // Legs 3 + 4 — per target: on-disk == listed, forbidden non-empty, semantic
    // fixtures non-empty + red-proven.
    for entry in &entries {
        check_target(fuzz_dir, repo_root, registry_src, entry)?;
    }

    // No stray subdir under regressions/ or corpus/ names a non-bin target.
    check_no_stray_subdirs(&fuzz_dir.join("regressions"), &bins)?;
    check_no_stray_subdirs(&fuzz_dir.join("corpus"), &bins)?;
    Ok(())
}

/// Legs 3+4 for one target: regressions dir present, on-disk file set == manifest
/// fixture file set (both directions), every `forbidden` non-empty, and semantic
/// fixtures non-empty with a registered ProductionFlip `red_proof`.
fn check_target(
    fuzz_dir: &Path,
    repo_root: &Path,
    registry_src: &str,
    entry: &RegressionManifestEntry,
) -> Result<()> {
    let reg_dir = fuzz_dir.join("regressions").join(&entry.target);
    ensure(
        reg_dir.is_dir(),
        format!(
            "fuzz-replay-contract (leg 3): fuzz/regressions/{}/ is missing — the manifest \
             claims a regression fixture for this target but the directory does not exist.",
            entry.target
        ),
    )?;

    let on_disk = dir_file_names(&reg_dir)?;
    let mut listed = BTreeSet::new();
    for fixture in &entry.fixtures {
        ensure(
            listed.insert(fixture.file.clone()),
            format!(
                "fuzz-replay-contract (leg 3): duplicate fixture file `{}` under target `{}` \
                 in the manifest.",
                fixture.file, entry.target
            ),
        )?;
    }
    for file in &listed {
        ensure(
            on_disk.contains(file),
            format!(
                "fuzz-replay-contract (leg 3): manifest fixture fuzz/regressions/{}/{} does \
                 not exist on disk — commit the fixture or drop the manifest line.",
                entry.target, file
            ),
        )?;
    }
    for file in &on_disk {
        ensure(
            listed.contains(file),
            format!(
                "fuzz-replay-contract (leg 3): fuzz/regressions/{}/{} is on disk but not \
                 listed in fuzz/regressions/manifest.yaml — an unlisted fixture is \
                 ceremonial paperwork; list it (with a `forbidden`) or delete it.",
                entry.target, file
            ),
        )?;
    }

    for fixture in &entry.fixtures {
        ensure(
            !fixture.forbidden.trim().is_empty(),
            format!(
                "fuzz-replay-contract (leg 3): fixture fuzz/regressions/{}/{} has an empty \
                 `forbidden` — every fixture must name the behavior it reproduces.",
                entry.target, fixture.file
            ),
        )?;
    }

    match entry.class {
        // Decoder fixtures red only on panic; an empty boundary input is legal and
        // `red_proof` is optional.
        Class::Decoder => Ok(()),
        Class::Semantic => check_semantic_target(&reg_dir, repo_root, registry_src, entry),
    }
}

/// Leg 4 for a `class: semantic` target: >= 1 fixture, each NON-EMPTY with a
/// registered ProductionFlip `red_proof`.
fn check_semantic_target(
    reg_dir: &Path,
    repo_root: &Path,
    registry_src: &str,
    entry: &RegressionManifestEntry,
) -> Result<()> {
    ensure(
        !entry.fixtures.is_empty(),
        format!(
            "fuzz-replay-contract (leg 4): semantic-class target `{}` declares no fixture \
             — a critical fuzz target must keep >= 1 committed regression.",
            entry.target
        ),
    )?;
    for fixture in &entry.fixtures {
        let fixture_path = reg_dir.join(&fixture.file);
        let len = fs::metadata(&fixture_path)
            .with_context(|| format!("stat {}", fixture_path.display()))?
            .len();
        ensure(
            len > 0,
            format!(
                "fuzz-replay-contract (leg 4): semantic-class target `{}` fixture `{}` is \
                 0 bytes — a ceremonial empty file is not a load-bearing regression; commit \
                 the minimized RED bytes that discriminate the cured decision core.",
                entry.target, fixture.file
            ),
        )?;
        let red_proof = match &fixture.red_proof {
            Some(reference) => reference,
            None => bail!(
                "fuzz-replay-contract (leg 4): semantic-class target `{}` fixture `{}` has no \
                 `red_proof` — a semantic regression must name the ProductionFlip test that \
                 proves it reds the pre-fix decision core.",
                entry.target,
                fixture.file
            ),
        };
        validate_red_proof(repo_root, red_proof, registry_src)?;
    }
    Ok(())
}

/// A `red_proof = <repo-rel .rs>::<fn>` must resolve to a real `#[test]` fn in a
/// file carrying a `gauntlet_red_fixture` branch (ProductionFlip qualification),
/// and the reference must appear verbatim in `gate_registry.rs` (so the fixture
/// is actually bitten by `prove-gates-bite`).
fn validate_red_proof(repo_root: &Path, red_proof: &str, registry_src: &str) -> Result<()> {
    let (rel, fn_name) = split_ref(red_proof, "red_proof")?;
    let path = resolve_repo_or_core_path(repo_root, rel);
    ensure(
        path.is_file(),
        format!(
            "fuzz-replay-contract (leg 4): red_proof `{red_proof}` points at a missing file \
             {rel}."
        ),
    )?;
    let src = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    ensure(
        source_declares_test_fn(&src, fn_name)?,
        format!(
            "fuzz-replay-contract (leg 4): red_proof `{red_proof}` names no `#[test] fn \
             {fn_name}` in {rel}."
        ),
    )?;
    ensure(
        src.contains("gauntlet_red_fixture"),
        format!(
            "fuzz-replay-contract (leg 4): red_proof file {rel} carries no \
             `gauntlet_red_fixture` branch — a ProductionFlip red_proof must flip its \
             expectation to the forbidden outcome under `--cfg gauntlet_red_fixture` so \
             `prove-gates-bite` can prove it reds the cured core."
        ),
    )?;
    ensure(
        registry_src.contains(red_proof),
        format!(
            "fuzz-replay-contract (leg 4): red_proof `{red_proof}` is not registered in \
             {GATE_REGISTRY_REL} — the fixture must be a registered ProductionFlip gate so \
             `prove-gates-bite` actually bites it."
        ),
    )?;
    Ok(())
}

/// EXACT set equality in both directions, with a per-side diagnosis closure.
fn assert_set_equality(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    context: &str,
    missing_from_right: impl Fn(&str) -> String,
    missing_from_left: impl Fn(&str) -> String,
) -> Result<()> {
    for item in left {
        ensure(right.contains(item), format!("{context}: {}", missing_from_right(item)))?;
    }
    for item in right {
        ensure(left.contains(item), format!("{context}: {}", missing_from_left(item)))?;
    }
    Ok(())
}

/// Every immediate subdir of `dir` must name a declared bin. A missing `dir` is
/// tolerated (corpus dirs are not required for every target). A stray dir is a
/// fixture left behind by a renamed/retired target.
fn check_no_stray_subdirs(dir: &Path, bins: &BTreeSet<String>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for name in subdir_names(dir)? {
        ensure(
            bins.contains(&name),
            format!(
                "fuzz-replay-contract: {}/{name}/ names no `[[bin]]` fuzz target — a stray \
                 regression/corpus dir left by a renamed or retired target; remove it or \
                 restore the bin.",
                dir.display()
            ),
        )?;
    }
    Ok(())
}

/// Every regular file under `fuzz/regressions/<target>/` (for the gate receipt).
/// The top-level `manifest.yaml` is not a fixture and is added by `check()`.
fn regression_fixture_files(fuzz_dir: &Path) -> Result<Vec<PathBuf>> {
    let regressions = fuzz_dir.join("regressions");
    let mut files = Vec::new();
    if !regressions.is_dir() {
        return Ok(files);
    }
    for name in subdir_names(&regressions)? {
        let target_dir = regressions.join(&name);
        for file in dir_file_names(&target_dir)? {
            files.push(target_dir.join(file));
        }
    }
    Ok(files)
}

/// The set of regular-file names directly under `dir`.
fn dir_file_names(dir: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let read = fs::read_dir(dir).with_context(|| format!("read dir {}", dir.display()))?;
    for entry in read {
        let entry = entry.with_context(|| format!("read entry under {}", dir.display()))?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                names.insert(name.to_string());
            }
        }
    }
    Ok(names)
}

/// The set of immediate subdirectory names under `dir`.
fn subdir_names(dir: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let read = fs::read_dir(dir).with_context(|| format!("read dir {}", dir.display()))?;
    for entry in read {
        let entry = entry.with_context(|| format!("read entry under {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                names.insert(name.to_string());
            }
        }
    }
    Ok(names)
}

/// Parse the `name = "..."` of every `[[bin]]` section from the fuzz crate's
/// `Cargo.toml` (mirror of `fuzz_replay.rs::declared_bin_targets`: section
/// tracking + first `name = "..."` per `[[bin]]`). Returns empty on a read error;
/// the caller rejects an empty set.
fn declared_bin_targets(manifest: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(manifest) else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    let mut in_bin = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
            continue;
        }
        if in_bin {
            if let Some(name) = parse_name_value(line) {
                targets.push(name);
                in_bin = false;
            }
        }
    }
    targets
}

/// If `line` is `name = "value"` (ignoring inline whitespace), return `value`.
fn parse_name_value(line: &str) -> Option<String> {
    let rest = line.strip_prefix("name")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Syn-parse `fuzz_replay.rs` and collect every string literal that appears as the
/// FIRST element of a tuple inside `fn dispatch_table`'s `vec![...]`.
fn dispatched_targets(fuzz_replay_src: &str) -> Result<BTreeSet<String>> {
    let file = syn::parse_file(fuzz_replay_src).context("parse fuzz_replay.rs for dispatch_table")?;
    let mut visitor = DispatchVisitor {
        in_dispatch: false,
        targets: BTreeSet::new(),
    };
    visitor.visit_file(&file);
    Ok(visitor.targets)
}

/// Collects the first-tuple-element string literals of `vec![...]` occurrences,
/// but only while inside `fn dispatch_table`.
struct DispatchVisitor {
    in_dispatch: bool,
    targets: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for DispatchVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if node.sig.ident == "dispatch_table" {
            let outer = self.in_dispatch;
            self.in_dispatch = true;
            syn::visit::visit_block(self, &node.block);
            self.in_dispatch = outer;
        } else {
            syn::visit::visit_item_fn(self, node);
        }
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if self.in_dispatch && node.mac.path.is_ident("vec") {
            if let Ok(items) = node.mac.parse_body_with(
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
            ) {
                for expr in items {
                    if let syn::Expr::Tuple(tuple) = expr {
                        if let Some(syn::Expr::Lit(first)) = tuple.elems.first() {
                            if let syn::Lit::Str(literal) = &first.lit {
                                self.targets.insert(literal.value());
                            }
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr_macro(self, node);
    }
}

/// Split a `<repo-rel path>::<Item>` reference. `label` names the field for the error.
fn split_ref<'a>(reference: &'a str, label: &str) -> Result<(&'a str, &'a str)> {
    reference.rsplit_once("::").ok_or_else(|| {
        anyhow::anyhow!("fuzz-replay-contract: {label} `{reference}` must be `path::Item`")
    })
}

/// Whether `source` declares `fn <fn_name>` carrying `#[test]` (recursing into
/// inline modules). A plain non-`#[test]` fn is intentionally rejected.
fn source_declares_test_fn(source: &str, fn_name: &str) -> Result<bool> {
    let file = syn::parse_file(source).context("parse red_proof source")?;
    Ok(items_declare_test_fn(&file.items, fn_name))
}

fn items_declare_test_fn(items: &[syn::Item], fn_name: &str) -> bool {
    for item in items {
        if let syn::Item::Fn(item_fn) = item {
            if item_fn.sig.ident == fn_name && fn_has_test_attr(&item_fn.attrs) {
                return true;
            }
        } else if let syn::Item::Mod(item_mod) = item {
            if let Some((_, nested)) = &item_mod.content {
                if items_declare_test_fn(nested, fn_name) {
                    return true;
                }
            }
        }
    }
    false
}

fn fn_has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        path.is_ident("test")
            || path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "test")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const RED_PROOF: &str = "crates/core/tests/sem_flip.rs::sem_is_load_bearing";
    const SEM_FLIP_REL: &str = "crates/core/tests/sem_flip.rs";

    /// A fresh, isolated temp repo root for one planted-tree test.
    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "batpak-fuzz-replay-contract-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(path, bytes).expect("write fixture");
    }

    /// A `fuzz_replay.rs`-shaped source whose `dispatch_table` wires `targets`.
    fn fuzz_replay_src(targets: &[&str]) -> String {
        let rows = targets
            .iter()
            .map(|name| format!("        (\"{name}\", replay as Replay),"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "type Replay = fn(&[u8]);\n\
             fn dispatch_table() -> Vec<(&'static str, Replay)> {{\n    vec![\n{rows}\n    ]\n}}\n\
             fn replay(_: &[u8]) {{}}\n"
        )
    }

    /// A registry source text carrying the green tree's red_proof reference.
    fn registry_src() -> String {
        format!("// gate table\n// red_fixture: {RED_PROOF}\n")
    }

    /// A ProductionFlip red_proof file: a `#[test]` fn with a `gauntlet_red_fixture`
    /// branch. `with_flip=false` drops the branch (for the leg-4 negative case).
    fn sem_flip_src(fn_name: &str, with_flip: bool) -> String {
        let flip = if with_flip {
            "    #[cfg(gauntlet_red_fixture)]\n    {\n        assert!(std::hint::black_box(false), \"RED\");\n    }\n"
        } else {
            ""
        };
        format!("#[test]\nfn {fn_name}() {{\n{flip}}}\n")
    }

    const MANIFEST_GREEN: &str = "\
- target: dec
  class: decoder
  fixtures:
    - { file: RED-dec, forbidden: \"panic on empty input\" }
- target: sem
  class: semantic
  fixtures:
    - file: RED-sem
      forbidden: \"pre-fix blind admission\"
      red_proof: crates/core/tests/sem_flip.rs::sem_is_load_bearing
";

    /// Write the green tree; return `(fuzz_replay_src, registry_src)`.
    fn write_green(root: &Path) -> (String, String) {
        write(
            root,
            "fuzz/Cargo.toml",
            b"[[bin]]\nname = \"dec\"\n[[bin]]\nname = \"sem\"\n",
        );
        write(root, "fuzz/regressions/manifest.yaml", MANIFEST_GREEN.as_bytes());
        write(root, "fuzz/regressions/dec/RED-dec", b"");
        write(root, "fuzz/regressions/sem/RED-sem", b"\xC0");
        write(root, SEM_FLIP_REL, sem_flip_src("sem_is_load_bearing", true).as_bytes());
        (fuzz_replay_src(&["dec", "sem"]), registry_src())
    }

    fn run(root: &Path, replay_src: &str, reg_src: &str) -> Result<()> {
        check_over(&root.join("fuzz"), replay_src, reg_src)
    }

    #[test]
    fn dispatched_targets_extracts_first_tuple_literals() {
        let set = dispatched_targets(&fuzz_replay_src(&["alpha", "beta"]))
            .expect("dispatch extraction");
        let expected: BTreeSet<String> =
            ["alpha".to_string(), "beta".to_string()].into_iter().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn green_synthetic_tree_passes() {
        let root = temp_root("green");
        let (replay, registry) = write_green(&root);
        run(&root, &replay, &registry).expect("green tree must pass");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn bin_without_dispatch_entry_is_rejected() {
        let root = temp_root("bin-no-dispatch");
        let (_replay, registry) = write_green(&root);
        // A third bin that no dispatcher wires.
        write(
            &root,
            "fuzz/Cargo.toml",
            b"[[bin]]\nname = \"dec\"\n[[bin]]\nname = \"sem\"\n[[bin]]\nname = \"gamma\"\n",
        );
        let err = run(&root, &fuzz_replay_src(&["dec", "sem"]), &registry)
            .expect_err("bin without dispatch must fail");
        assert!(
            err.to_string().contains("no `dispatch_table` entry"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn dispatch_entry_without_bin_is_rejected() {
        let root = temp_root("dispatch-no-bin");
        let (_replay, registry) = write_green(&root);
        let err = run(&root, &fuzz_replay_src(&["dec", "sem", "gamma"]), &registry)
            .expect_err("dispatch without bin must fail");
        assert!(
            err.to_string().contains("no `[[bin]]`"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn missing_regressions_dir_is_rejected() {
        let root = temp_root("no-regressions");
        let (replay, registry) = write_green(&root);
        fs::remove_dir_all(root.join("fuzz/regressions/sem")).expect("drop sem dir");
        let err = run(&root, &replay, &registry).expect_err("missing regressions dir must fail");
        assert!(err.to_string().contains("is missing"), "wrong error: {err:#}");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn on_disk_fixture_absent_from_manifest_is_rejected() {
        let root = temp_root("ceremonial-file");
        let (replay, registry) = write_green(&root);
        // An on-disk fixture the manifest never lists.
        write(&root, "fuzz/regressions/dec/STRAY-unlisted", b"junk");
        let err = run(&root, &replay, &registry).expect_err("unlisted fixture must fail");
        assert!(
            err.to_string().contains("ceremonial paperwork"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn semantic_target_with_only_empty_ceremonial_fixture_is_rejected() {
        let root = temp_root("empty-semantic");
        let (replay, registry) = write_green(&root);
        // The semantic fixture exists and is listed but is 0 bytes.
        write(&root, "fuzz/regressions/sem/RED-sem", b"");
        let err = run(&root, &replay, &registry).expect_err("empty semantic fixture must fail");
        assert!(err.to_string().contains("0 bytes"), "wrong error: {err:#}");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn semantic_fixture_without_red_proof_is_rejected() {
        let root = temp_root("no-red-proof");
        let (replay, registry) = write_green(&root);
        let manifest = MANIFEST_GREEN.replace(
            "      red_proof: crates/core/tests/sem_flip.rs::sem_is_load_bearing\n",
            "",
        );
        write(&root, "fuzz/regressions/manifest.yaml", manifest.as_bytes());
        let err = run(&root, &replay, &registry).expect_err("missing red_proof must fail");
        assert!(err.to_string().contains("no `red_proof`"), "wrong error: {err:#}");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn red_proof_naming_missing_test_fn_is_rejected() {
        let root = temp_root("red-proof-missing-fn");
        let (replay, registry) = write_green(&root);
        // The red_proof file exists and carries the flip branch, but the named fn
        // is not a `#[test]` fn.
        write(&root, SEM_FLIP_REL, sem_flip_src("some_other_fn", true).as_bytes());
        let err = run(&root, &replay, &registry).expect_err("missing test fn must fail");
        assert!(
            err.to_string().contains("names no `#[test] fn"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn red_proof_file_without_production_flip_branch_is_rejected() {
        let root = temp_root("red-proof-no-flip");
        let (replay, registry) = write_green(&root);
        write(&root, SEM_FLIP_REL, sem_flip_src("sem_is_load_bearing", false).as_bytes());
        let err = run(&root, &replay, &registry).expect_err("missing flip branch must fail");
        assert!(
            err.to_string().contains("gauntlet_red_fixture"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn red_proof_not_in_registry_is_rejected() {
        let root = temp_root("red-proof-unregistered");
        let (replay, _registry) = write_green(&root);
        let err = run(&root, &replay, "// registry with no red_proof reference\n")
            .expect_err("unregistered red_proof must fail");
        assert!(
            err.to_string().contains("not registered"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn stray_regression_dir_is_rejected() {
        let root = temp_root("stray-dir");
        let (replay, registry) = write_green(&root);
        // A regression dir for a retired target that no bin declares.
        write(&root, "fuzz/regressions/retired/RED-x", b"x");
        let err = run(&root, &replay, &registry).expect_err("stray dir must fail");
        assert!(
            err.to_string().contains("names no `[[bin]]`"),
            "wrong error: {err:#}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}

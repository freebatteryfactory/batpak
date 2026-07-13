//! `crash-matrix` gate (GAUNT-PROOF-OF-PROOF #197, gate (d)): an L4
//! crash-coverage invariant that claims exhaustive namespace-transition coverage
//! must back every declared transition with a real fixture, and the declared set
//! must equal the harness enum EXACTLY.
//!
//! INV-COMPACTION-NAMESPACE-COMMIT is the L4 law: the compaction crash harness
//! enumerates its crash boundaries as `enum CompactionCrashBoundary`. This gate
//! reads `traceability/crash_matrix.yaml`, syn-parses the named harness enum, and
//! binds the two together with EXACT set equality (kebab-case of each variant ==
//! one `boundaries[].id`). A variant with no declared fixture fails "crash
//! coverage claimed without a fixture for transition X"; a yaml boundary that
//! names no variant fails "phantom transition". Every `witness_test` must resolve
//! to a real `#[test]` fn. Exhaustiveness of any `match` over the enum in the
//! harness is carried by the repo-wide `wildcard_enum_match_arm` denial; this gate
//! carries the yaml↔enum agreement so a new boundary cannot ship uncovered.

use crate::docs_catalog::load_catalog;
use crate::repo_surface::{ensure, resolve_repo_or_core_path};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One crash-coverage claim: an L4 invariant, the harness enum that enumerates
/// its crash boundaries, and the per-transition fixtures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CrashMatrixEntry {
    /// L4 invariant id — must exist as an `id:` in `traceability/invariants.yaml`.
    invariant: String,
    /// `<repo-rel .rs path>::<EnumName>` naming the harness crash-boundary enum.
    harness_enum: String,
    boundaries: Vec<Boundary>,
}

/// One declared crash transition: a kebab id (== a kebab-cased enum variant) and
/// the `<repo-rel .rs path>::<fn>` `#[test]` that witnesses its reopen.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Boundary {
    id: String,
    witness_test: String,
}

/// Production entry: validate `traceability/crash_matrix.yaml` against the live
/// invariant catalog, harness enum, and witness tests. Returns the set of files
/// consulted (for the gate receipt).
pub(crate) fn check(repo_root: &Path) -> Result<BTreeSet<PathBuf>> {
    let matrix_path = repo_root.join("traceability").join("crash_matrix.yaml");
    let yaml_text = std::fs::read_to_string(&matrix_path)
        .with_context(|| format!("read {}", matrix_path.display()))?;
    let invariant_ids: BTreeSet<String> =
        load_catalog(repo_root)?.into_iter().map(|c| c.id).collect();

    let referenced = evaluate_matrix(&yaml_text, &invariant_ids, |rel| {
        let full = resolve_repo_or_core_path(repo_root, rel);
        if full.is_file() {
            let src = std::fs::read_to_string(&full)
                .with_context(|| format!("read {}", full.display()))?;
            Ok(Some(src))
        } else {
            Ok(None)
        }
    })?;

    let mut inputs = BTreeSet::new();
    inputs.insert(matrix_path);
    inputs.insert(repo_root.join("traceability").join("invariants.yaml"));
    for rel in &referenced {
        inputs.insert(resolve_repo_or_core_path(repo_root, rel));
    }
    Ok(inputs)
}

/// Testable core over synthetic yaml text + a source resolver + a synthetic
/// invariant-id set. `resolve_source(rel)` returns the source text of a `.rs`
/// file (`None` when it does not exist). Returns the set of `.rs` rel-paths the
/// matrix referenced (harness enums + witness tests) for the receipt.
fn evaluate_matrix<R>(
    yaml_text: &str,
    invariant_ids: &BTreeSet<String>,
    resolve_source: R,
) -> Result<BTreeSet<String>>
where
    R: Fn(&str) -> Result<Option<String>>,
{
    let entries: Vec<CrashMatrixEntry> =
        yaml_serde::from_str(yaml_text).context("parse crash_matrix.yaml")?;
    ensure(
        !entries.is_empty(),
        "crash_matrix.yaml is empty — a crash-coverage claim with no entries is \
         vacuous; declare the L4 invariant and its crash-boundary matrix or delete \
         the file.",
    )?;

    let mut referenced = BTreeSet::new();
    for entry in &entries {
        ensure(
            invariant_ids.contains(&entry.invariant),
            format!(
                "crash_matrix.yaml: invariant `{}` is not an `id:` in \
                 traceability/invariants.yaml — a crash matrix cannot claim an \
                 undeclared invariant.",
                entry.invariant
            ),
        )?;

        let (enum_rel, enum_name) = split_ref(&entry.harness_enum, "harness_enum")?;
        referenced.insert(enum_rel.to_string());
        let enum_src = resolve_source(enum_rel)?.ok_or_else(|| {
            anyhow::anyhow!(
                "crash_matrix.yaml: harness_enum `{}` points at a missing file {enum_rel}",
                entry.harness_enum
            )
        })?;
        let variants = enum_variant_kebabs(&enum_src, enum_name)?.ok_or_else(|| {
            anyhow::anyhow!(
                "crash_matrix.yaml: harness_enum `{}` names no `enum {enum_name}` in \
                 {enum_rel}",
                entry.harness_enum
            )
        })?;
        ensure(
            !variants.is_empty(),
            format!(
                "crash_matrix.yaml: harness enum `{enum_name}` in {enum_rel} has zero \
                 variants — an empty crash-boundary enum proves nothing."
            ),
        )?;

        let declared = boundary_id_set(&entry.boundaries)?;
        assert_set_equality(&variants, &declared, enum_name)?;
        validate_witnesses(&entry.boundaries, &resolve_source, &mut referenced)?;
    }
    Ok(referenced)
}

/// Resolve every boundary's `witness_test` to a real `#[test]` fn, recording each
/// referenced `.rs` rel-path in `referenced`.
fn validate_witnesses<R>(
    boundaries: &[Boundary],
    resolve_source: &R,
    referenced: &mut BTreeSet<String>,
) -> Result<()>
where
    R: Fn(&str) -> Result<Option<String>>,
{
    for boundary in boundaries {
        let (test_rel, test_fn) = split_ref(&boundary.witness_test, "witness_test")?;
        referenced.insert(test_rel.to_string());
        let test_src = resolve_source(test_rel)?.ok_or_else(|| {
            anyhow::anyhow!(
                "crash_matrix.yaml: boundary `{}` witness_test `{}` points at a \
                 missing file {test_rel}",
                boundary.id,
                boundary.witness_test
            )
        })?;
        ensure(
            source_declares_test_fn(&test_src, test_fn)?,
            format!(
                "crash_matrix.yaml: boundary `{}` witness_test `{}` names no \
                 `#[test] fn {test_fn}` in {test_rel}.",
                boundary.id, boundary.witness_test
            ),
        )?;
    }
    Ok(())
}

/// Reject duplicate boundary ids while building the declared-id set.
fn boundary_id_set(boundaries: &[Boundary]) -> Result<BTreeSet<String>> {
    let mut declared = BTreeSet::new();
    for boundary in boundaries {
        ensure(
            declared.insert(boundary.id.clone()),
            format!(
                "crash_matrix.yaml: duplicate boundary id `{}` — each crash \
                 transition is declared exactly once.",
                boundary.id
            ),
        )?;
    }
    Ok(declared)
}

/// EXACT set equality between the kebab-cased enum variants and the declared
/// boundary ids, in both directions with a diagnosis per side.
fn assert_set_equality(
    variants: &BTreeSet<String>,
    declared: &BTreeSet<String>,
    enum_name: &str,
) -> Result<()> {
    for variant in variants {
        ensure(
            declared.contains(variant),
            format!(
                "crash_matrix.yaml: crash coverage claimed without a fixture for \
                 transition `{variant}` — `enum {enum_name}` declares this variant \
                 but no `boundaries[].id` covers it. Add a boundary with a \
                 witness_test or remove the variant."
            ),
        )?;
    }
    for id in declared {
        ensure(
            variants.contains(id),
            format!(
                "crash_matrix.yaml: phantom transition `{id}` — it names no `enum \
                 {enum_name}` variant. Rename it to a real (kebab-cased) variant or \
                 remove it."
            ),
        )?;
    }
    Ok(())
}

/// Split a `<repo-rel path>::<Item>` reference. `label` names the yaml field for
/// the error message.
fn split_ref<'a>(reference: &'a str, label: &str) -> Result<(&'a str, &'a str)> {
    reference.rsplit_once("::").ok_or_else(|| {
        anyhow::anyhow!("crash_matrix.yaml: {label} `{reference}` must be `path::Item`")
    })
}

/// Camel/Pascal ident -> kebab-case (`MarkerDurableNoSourceRename` ->
/// `marker-durable-no-source-rename`). A `-` is inserted before every uppercase
/// letter except the first; digits pass through. The canonical crash-boundary
/// variants carry no consecutive capitals, so this is a faithful inverse of the
/// PascalCase-of-kebab-id promotion the harness enum performs.
fn kebab_case(ident: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in ident.char_indices() {
        if ch.is_ascii_uppercase() {
            if idx != 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Parse `source`, find `enum <enum_name>` (recursing into inline modules), and
/// return its kebab-cased variant idents. `None` when the enum is absent.
fn enum_variant_kebabs(source: &str, enum_name: &str) -> Result<Option<BTreeSet<String>>> {
    let file = syn::parse_file(source).context("parse harness enum source")?;
    Ok(find_enum_variants(&file.items, enum_name))
}

fn find_enum_variants(items: &[syn::Item], enum_name: &str) -> Option<BTreeSet<String>> {
    for item in items {
        if let syn::Item::Enum(item_enum) = item {
            if item_enum.ident == enum_name {
                return Some(
                    item_enum
                        .variants
                        .iter()
                        .map(|variant| kebab_case(&variant.ident.to_string()))
                        .collect(),
                );
            }
        } else if let syn::Item::Mod(item_mod) = item {
            if let Some((_, nested)) = &item_mod.content {
                if let Some(found) = find_enum_variants(nested, enum_name) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Whether `source` declares `fn <fn_name>` carrying `#[test]` (recursing into
/// inline modules). A plain non-`#[test]` fn is intentionally rejected.
fn source_declares_test_fn(source: &str, fn_name: &str) -> Result<bool> {
    let file = syn::parse_file(source).context("parse witness_test source")?;
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
    use std::collections::BTreeMap;

    /// The 9 canonical crash boundaries (contract R3), PascalCase paired with the
    /// kebab id the harness enum promotes from.
    const CANONICAL: &[(&str, &str)] = &[
        ("MarkerDurableNoSourceRename", "marker-durable-no-source-rename"),
        ("SourceRenamesPerformed", "source-renames-performed"),
        (
            "PartialReplacementMaterialization",
            "partial-replacement-materialization",
        ),
        (
            "StagedReplacementCompleteAuthorityUnpublished",
            "staged-replacement-complete-authority-unpublished",
        ),
        (
            "AuthorityPublishedFinalRenameNotDurable",
            "authority-published-final-rename-not-durable",
        ),
        (
            "FinalReplacementDurableSourcesNotRetired",
            "final-replacement-durable-sources-not-retired",
        ),
        ("SourcesPartiallyRetired", "sources-partially-retired"),
        ("MarkerNotCleared", "marker-not-cleared"),
        ("MarkerCleared", "marker-cleared"),
    ];

    const HARNESS_REL: &str = "crates/core/tests/compaction_crash/mod.rs";
    const WINDOWS_REL: &str = "crates/core/tests/compaction_crash_windows.rs";
    const INVARIANT: &str = "INV-COMPACTION-NAMESPACE-COMMIT";

    fn invariant_ids() -> BTreeSet<String> {
        [INVARIANT.to_string()].into_iter().collect()
    }

    fn harness_source(variants: &[&str]) -> String {
        let body = variants
            .iter()
            .map(|name| format!("    {name},"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("pub enum CompactionCrashBoundary {{\n{body}\n}}\n")
    }

    /// A witness source declaring one `#[test]` per named fn.
    fn windows_source(fns: &[&str]) -> String {
        fns.iter()
            .map(|name| format!("#[test]\nfn {name}() {{}}\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build the green matrix yaml plus a resolver over its two synthetic files.
    fn green_fixture() -> (String, BTreeMap<String, String>) {
        let pascal: Vec<&str> = CANONICAL.iter().map(|(p, _)| *p).collect();
        let mut boundaries = String::new();
        for (_, kebab) in CANONICAL {
            let fn_name = kebab.replace('-', "_");
            boundaries.push_str(&format!(
                "    - {{ id: {kebab}, witness_test: \"{WINDOWS_REL}::{fn_name}\" }}\n"
            ));
        }
        let yaml = format!(
            "- invariant: {INVARIANT}\n  harness_enum: {HARNESS_REL}::CompactionCrashBoundary\n  \
             boundaries:\n{boundaries}"
        );
        let fns: Vec<String> = CANONICAL.iter().map(|(_, k)| k.replace('-', "_")).collect();
        let fn_refs: Vec<&str> = fns.iter().map(String::as_str).collect();
        let mut sources = BTreeMap::new();
        sources.insert(HARNESS_REL.to_string(), harness_source(&pascal));
        sources.insert(WINDOWS_REL.to_string(), windows_source(&fn_refs));
        (yaml, sources)
    }

    fn run(yaml: &str, sources: &BTreeMap<String, String>) -> Result<BTreeSet<String>> {
        evaluate_matrix(yaml, &invariant_ids(), |rel| {
            Ok(sources.get(rel).cloned())
        })
    }

    #[test]
    fn kebab_case_inverts_pascal_promotion() {
        for (pascal, kebab) in CANONICAL {
            assert_eq!(&kebab_case(pascal), kebab, "kebab of {pascal}");
        }
    }

    #[test]
    fn green_matrix_passes() {
        let (yaml, sources) = green_fixture();
        let referenced = run(&yaml, &sources).expect("green matrix must pass");
        assert!(referenced.contains(HARNESS_REL), "enum file referenced");
        assert!(referenced.contains(WINDOWS_REL), "witness file referenced");
    }

    #[test]
    fn declared_boundary_without_witness_fixture_is_rejected() {
        // Enum has 9 variants; the yaml lists only 8 (drop the last boundary) —
        // the ninth transition is claimed by the enum with no fixture.
        let (mut yaml, sources) = green_fixture();
        let (_, last_kebab) = CANONICAL[CANONICAL.len() - 1];
        let fn_name = last_kebab.replace('-', "_");
        let drop_line =
            format!("    - {{ id: {last_kebab}, witness_test: \"{WINDOWS_REL}::{fn_name}\" }}\n");
        yaml = yaml.replace(&drop_line, "");
        let err = run(&yaml, &sources).expect_err("missing fixture must fail");
        assert!(
            err.to_string().contains("without a fixture for transition"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn phantom_boundary_not_in_enum_is_rejected() {
        let (mut yaml, mut sources) = green_fixture();
        // Add a phantom boundary that maps to no enum variant, and a witness for
        // it, so the ONLY failure is the phantom-transition set mismatch.
        yaml.push_str(&format!(
            "    - {{ id: not-a-real-transition, witness_test: \"{WINDOWS_REL}::phantom\" }}\n"
        ));
        let mut fns: Vec<String> = CANONICAL.iter().map(|(_, k)| k.replace('-', "_")).collect();
        fns.push("phantom".to_string());
        let fn_refs: Vec<&str> = fns.iter().map(String::as_str).collect();
        sources.insert(WINDOWS_REL.to_string(), windows_source(&fn_refs));
        let err = run(&yaml, &sources).expect_err("phantom boundary must fail");
        assert!(
            err.to_string().contains("phantom transition"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn witness_naming_missing_test_fn_is_rejected() {
        let (yaml, mut sources) = green_fixture();
        // Drop every #[test] fn from the witness file, keeping the enum intact.
        sources.insert(WINDOWS_REL.to_string(), "// no tests here\n".to_string());
        let err = run(&yaml, &sources).expect_err("missing witness fn must fail");
        assert!(
            err.to_string().contains("names no `#[test] fn"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn non_test_witness_fn_is_rejected() {
        let (yaml, mut sources) = green_fixture();
        // A plain (non-`#[test]`) fn of the right name must not satisfy the witness.
        let plain = CANONICAL
            .iter()
            .map(|(_, k)| format!("fn {}() {{}}\n", k.replace('-', "_")))
            .collect::<String>();
        sources.insert(WINDOWS_REL.to_string(), plain);
        let err = run(&yaml, &sources).expect_err("non-test fn must fail");
        assert!(
            err.to_string().contains("names no `#[test] fn"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn unknown_invariant_id_is_rejected() {
        let (yaml, sources) = green_fixture();
        let bogus: BTreeSet<String> = ["INV-SOMETHING-ELSE".to_string()].into_iter().collect();
        let err = evaluate_matrix(&yaml, &bogus, |rel| Ok(sources.get(rel).cloned()))
            .expect_err("unknown invariant must fail");
        assert!(
            err.to_string().contains("not an `id:` in"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn empty_matrix_is_rejected() {
        let sources = BTreeMap::new();
        let err = run("[]\n", &sources).expect_err("empty matrix must fail");
        assert!(err.to_string().contains("empty"), "wrong error: {err:#}");
    }

    #[test]
    fn duplicate_boundary_id_is_rejected() {
        let (mut yaml, sources) = green_fixture();
        let (_, first_kebab) = CANONICAL[0];
        let fn_name = first_kebab.replace('-', "_");
        // Re-declare the first boundary a second time.
        yaml.push_str(&format!(
            "    - {{ id: {first_kebab}, witness_test: \"{WINDOWS_REL}::{fn_name}\" }}\n"
        ));
        let err = run(&yaml, &sources).expect_err("duplicate boundary must fail");
        assert!(
            err.to_string().contains("duplicate boundary id"),
            "wrong error: {err:#}"
        );
    }

    #[test]
    fn missing_harness_enum_file_is_rejected() {
        let (yaml, mut sources) = green_fixture();
        sources.remove(HARNESS_REL);
        let err = run(&yaml, &sources).expect_err("missing enum file must fail");
        assert!(
            err.to_string().contains("missing file"),
            "wrong error: {err:#}"
        );
    }
}

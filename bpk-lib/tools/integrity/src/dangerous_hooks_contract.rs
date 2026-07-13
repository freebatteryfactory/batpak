//! INV-DANGEROUS-TEST-HOOKS-NONDEFAULT: dangerous test hooks stay out of
//! default production builds and their public surfaces stay feature-gated.
//!
//! Two non-default gates carry test-support surface (contract A12): the internal
//! `dangerous-test-hooks` levers (fault injection, SimFs promotion, `__sim`
//! namespace matrix) and the clean external `conformance-harness` surface
//! (`store::conformance`, `ShadowFs`). `dangerous-test-hooks` implies
//! `conformance-harness`; NEITHER may enter default features, and every guarded
//! exposure must sit under the gate that owns it — a fault lever gated only by
//! the weaker `conformance-harness`, or the conformance corpus gated only by the
//! stronger `dangerous-test-hooks`, both trip this detector.

use crate::repo_surface::ensure;
use crate::source_cache::SourceCache;
use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const FEATURE: &str = "dangerous-test-hooks";
const CONFORMANCE_FEATURE: &str = "conformance-harness";

struct GuardedNeedle {
    rel: &'static str,
    needle: &'static str,
    label: &'static str,
    /// The exact non-default feature this exposure must be gated under.
    feature: &'static str,
}

const GUARDED_NEEDLES: &[GuardedNeedle] = &[
    GuardedNeedle {
        rel: "crates/core/src/lib.rs",
        needle: "pub mod __fuzz;",
        label: "__fuzz public module",
        feature: FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/lib.rs",
        needle: "pub mod __sim {",
        label: "__sim public module",
        feature: FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/store/mod.rs",
        needle: "pub mod fault;",
        label: "store::fault module",
        feature: FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/store/mod.rs",
        needle: "pub use fault::{",
        label: "store::fault re-export",
        feature: FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/store/config.rs",
        needle: "pub fn with_fault_injector",
        label: "StoreConfig::with_fault_injector",
        feature: FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/store/error.rs",
        needle: "FaultInjected(String),",
        label: "StoreError::FaultInjected",
        feature: FEATURE,
    },
    // Clean external conformance surface (contract A12): gated by
    // `conformance-harness`, NOT `dangerous-test-hooks`, so a downstream backend
    // author proves their `StoreFs` without pulling the dangerous levers.
    GuardedNeedle {
        rel: "crates/core/src/store/mod.rs",
        needle: "pub mod conformance;",
        label: "store::conformance module",
        feature: CONFORMANCE_FEATURE,
    },
    GuardedNeedle {
        rel: "crates/core/src/store/mod.rs",
        needle: "pub use sim::ShadowFs;",
        label: "store::ShadowFs re-export",
        feature: CONFORMANCE_FEATURE,
    },
    // The byte-fault layer + its fault vocabulary stay internal: a dangerous
    // lever, not part of the clean conformance surface (contract A12).
    GuardedNeedle {
        rel: "crates/core/src/store/mod.rs",
        needle: "pub use sim::fs::{CrashOp, ReadFaultKind, SimFs};",
        label: "store::SimFs fault-layer re-export",
        feature: FEATURE,
    },
];

pub(crate) fn check(repo_root: &Path, source_cache: &mut SourceCache) -> Result<BTreeSet<PathBuf>> {
    let mut inputs = BTreeSet::new();
    let manifest = repo_root.join("Cargo.toml");
    inputs.insert(manifest.clone());
    check_feature_metadata(repo_root)?;

    for guarded in GUARDED_NEEDLES {
        let path = repo_root.join(guarded.rel);
        inputs.insert(path.clone());
        let content = source_cache
            .read_to_string(&path)
            .with_context(|| format!("read {}", guarded.rel))?;
        check_guarded_needles(guarded.rel, &content, std::slice::from_ref(guarded))?;
    }
    Ok(inputs)
}

fn check_feature_metadata(repo_root: &Path) -> Result<()> {
    let metadata = MetadataCommand::new()
        .manifest_path(repo_root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("read Cargo metadata for dangerous hooks feature contract")?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name == "batpak")
        .context("Cargo metadata must contain root batpak package")?;
    let declared_features = package.features.keys().map(String::as_str);
    let default_features = package
        .features
        .get("default")
        .context("batpak package must declare default features")?
        .iter()
        .map(String::as_str);
    check_feature_sets(declared_features, default_features)
}

fn check_feature_sets<'a>(
    declared_features: impl IntoIterator<Item = &'a str>,
    default_features: impl IntoIterator<Item = &'a str>,
) -> Result<()> {
    let declared = declared_features.into_iter().collect::<BTreeSet<_>>();
    let default = default_features.into_iter().collect::<BTreeSet<_>>();
    ensure(
        declared.contains(FEATURE),
        "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
         batpak must declare the dangerous-test-hooks feature explicitly",
    )?;
    ensure(
        !default.contains(FEATURE),
        "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
         default features must not include dangerous-test-hooks",
    )?;
    ensure(
        declared.contains(CONFORMANCE_FEATURE),
        "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
         batpak must declare the conformance-harness feature explicitly (contract A12)",
    )?;
    ensure(
        !default.contains(CONFORMANCE_FEATURE),
        "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
         default features must not include conformance-harness (contract A12)",
    )
}

fn check_guarded_needles(rel: &str, content: &str, needles: &[GuardedNeedle]) -> Result<()> {
    let lines = content.lines().collect::<Vec<_>>();
    for needle in needles {
        let Some(line_index) = lines.iter().position(|line| line.contains(needle.needle)) else {
            anyhow::bail!(
                "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
                 {} missing `{}` in {rel}",
                needle.label,
                needle.needle
            );
        };
        ensure(
            has_feature_cfg_before(&lines, line_index, needle.feature),
            format!(
                "dangerous-hooks-contract (INV-DANGEROUS-TEST-HOOKS-NONDEFAULT): \
                 {} in {rel}:{} is not guarded by #[cfg(feature = \"{}\")]",
                needle.label,
                line_index + 1,
                needle.feature
            ),
        )?;
    }
    Ok(())
}

fn has_feature_cfg_before(lines: &[&str], line_index: usize, feature: &str) -> bool {
    let plain = format!("#[cfg(feature = \"{feature}\")]");
    let any_test = format!("#[cfg(any(test, feature = \"{feature}\"))]");
    let start = line_index.saturating_sub(6);
    lines[start..line_index]
        .iter()
        .any(|line| line.contains(&plain) || line.contains(&any_test))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_hooks_default_feature_and_cfg_contract_rejects_planted_exposure() {
        assert!(
            check_feature_sets(
                ["blake3", "conformance-harness", "dangerous-test-hooks"],
                ["blake3", "dangerous-test-hooks"],
            )
            .is_err(),
            "dangerous-test-hooks in default features must be rejected"
        );

        assert!(
            check_feature_sets(
                ["blake3", "conformance-harness", "dangerous-test-hooks"],
                ["blake3", "conformance-harness"],
            )
            .is_err(),
            "conformance-harness in default features must be rejected (contract A12)"
        );

        let red = "pub mod fault;\n";
        assert!(
            check_guarded_needles(
                "crates/core/src/store/mod.rs",
                red,
                &[GuardedNeedle {
                    rel: "crates/core/src/store/mod.rs",
                    needle: "pub mod fault;",
                    label: "store::fault module",
                    feature: FEATURE,
                }],
            )
            .is_err(),
            "an ungated dangerous hook surface must be rejected"
        );

        // A conformance surface gated only by the stronger dangerous-test-hooks
        // (so external authors could not reach it) must be rejected: the needle
        // requires the conformance-harness gate specifically.
        let wrong_gate = "#[cfg(feature = \"dangerous-test-hooks\")]\npub mod conformance;\n";
        assert!(
            check_guarded_needles(
                "crates/core/src/store/mod.rs",
                wrong_gate,
                &[GuardedNeedle {
                    rel: "crates/core/src/store/mod.rs",
                    needle: "pub mod conformance;",
                    label: "store::conformance module",
                    feature: CONFORMANCE_FEATURE,
                }],
            )
            .is_err(),
            "the conformance surface must be gated by conformance-harness, not dangerous-test-hooks"
        );
    }

    #[test]
    fn dangerous_hooks_cfg_contract_accepts_feature_gated_surface() {
        check_feature_sets(
            ["blake3", "conformance-harness", "dangerous-test-hooks"],
            ["blake3"],
        )
        .expect("both features declared and absent from defaults");
        check_guarded_needles(
            "crates/core/src/store/mod.rs",
            "#[cfg(feature = \"dangerous-test-hooks\")]\npub mod fault;\n",
            &[GuardedNeedle {
                rel: "crates/core/src/store/mod.rs",
                needle: "pub mod fault;",
                label: "store::fault module",
                feature: FEATURE,
            }],
        )
        .expect("feature-gated dangerous hook surface");
        check_guarded_needles(
            "crates/core/src/store/mod.rs",
            "#[cfg(feature = \"conformance-harness\")]\npub use sim::ShadowFs;\n",
            &[GuardedNeedle {
                rel: "crates/core/src/store/mod.rs",
                needle: "pub use sim::ShadowFs;",
                label: "store::ShadowFs re-export",
                feature: CONFORMANCE_FEATURE,
            }],
        )
        .expect("conformance-harness-gated conformance surface");
    }
}

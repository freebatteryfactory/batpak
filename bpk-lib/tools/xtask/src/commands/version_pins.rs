use crate::util::repo_root;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn check_version_pins() -> Result<()> {
    let root = repo_root()?;
    let members = workspace_members(&root)?;
    // The family versions in lockstep off `[workspace.package] version`; each
    // `crates/*` member inherits it via `version.workspace = true` rather than a
    // literal `package.version`. Resolve the workspace value once and use it
    // whenever a member does not pin a literal version of its own.
    let workspace_version = workspace_package_version(&root)?;
    let mut versions_by_manifest = BTreeMap::new();

    for member in &members {
        let manifest = root.join(member).join("Cargo.toml");
        let parsed = read_manifest(&manifest)?;
        let package = parsed
            .get("package")
            .and_then(toml::Value::as_table)
            .with_context(|| format!("{} missing [package]", manifest.display()))?;
        let name = string_value(package, "name", &manifest)?;
        let version = package
            .get("version")
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| workspace_version.clone());
        versions_by_manifest.insert(canonical_manifest(&manifest)?, (name, version));
    }

    let mut mismatches = Vec::new();
    for member in &members {
        let manifest = root.join(member).join("Cargo.toml");
        let parsed = read_manifest(&manifest)?;
        for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(table) = parsed.get(table_name).and_then(toml::Value::as_table) else {
                continue;
            };
            for (dep_name, dep_value) in table {
                let Some(dep_table) = dep_value.as_table() else {
                    continue;
                };
                let (Some(path), Some(actual)) = (
                    dep_table.get("path").and_then(toml::Value::as_str),
                    dep_table.get("version").and_then(toml::Value::as_str),
                ) else {
                    continue;
                };
                let dep_manifest = manifest
                    .parent()
                    .expect("manifest has parent")
                    .join(path)
                    .join("Cargo.toml");
                let dep_manifest = canonical_manifest(&dep_manifest)?;
                let Some((declared_name, expected)) = versions_by_manifest.get(&dep_manifest)
                else {
                    continue;
                };
                if declared_name != dep_name {
                    mismatches.push(format!(
                        "{}: dependency `{dep_name}` points to package `{declared_name}`",
                        manifest.display()
                    ));
                }
                if actual != expected {
                    mismatches.push(format!(
                        "{}:{}: dependency `{dep_name}` pins `{actual}`, expected `{expected}`",
                        manifest.display(),
                        dependency_line(&manifest, dep_name).unwrap_or(0)
                    ));
                }
            }
        }
    }

    if !mismatches.is_empty() {
        bail!("version pin drift:\n{}", mismatches.join("\n"));
    }

    outln!(
        "check-version-pins: ok; checked {} workspace member(s)",
        members.len()
    );
    Ok(())
}

/// Read `[workspace.package] version` — the single family-version source every
/// `crates/*` member inherits via `version.workspace = true`.
fn workspace_package_version(root: &Path) -> Result<String> {
    let manifest = root.join("Cargo.toml");
    let parsed = read_manifest(&manifest)?;
    parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{} missing [workspace.package] version", manifest.display()))
}

fn workspace_members(root: &Path) -> Result<Vec<PathBuf>> {
    let manifest = root.join("Cargo.toml");
    let parsed = read_manifest(&manifest)?;
    let members = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .with_context(|| format!("{} missing workspace.members", manifest.display()))?;
    Ok(members
        .iter()
        .filter_map(toml::Value::as_str)
        .map(PathBuf::from)
        .collect())
}

fn read_manifest(path: &Path) -> Result<toml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn canonical_manifest(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).with_context(|| format!("canonicalize {}", path.display()))
}

fn string_value(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    manifest: &Path,
) -> Result<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{} missing package.{key}", manifest.display()))
}

fn dependency_line(manifest: &Path, dependency: &str) -> Result<usize> {
    let text =
        fs::read_to_string(manifest).with_context(|| format!("read {}", manifest.display()))?;
    let needle = format!("{dependency} =");
    Ok(text
        .lines()
        .position(|line| line.trim_start().starts_with(&needle))
        .map(|index| index + 1)
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract the `=X.Y.Z` seccompiler requirement from bvisor's `Cargo.toml`.
    /// The dependency is an inline table (`seccompiler = { version = "=0.5.0",
    /// optional = true }`); pull the quoted `version` value out of that line.
    fn cargo_seccompiler_pin(manifest_text: &str) -> Option<String> {
        let line = manifest_text
            .lines()
            .find(|line| line.trim_start().starts_with("seccompiler"))?;
        let after = line.split("version").nth(1)?;
        let start = after.find('"')? + 1;
        let rest = &after[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    }

    /// Extract the mirrored `SECCOMPILER_VERSION` const value from the seccomp
    /// backend source.
    fn source_seccompiler_pin(source_text: &str) -> Option<String> {
        let line = source_text
            .lines()
            .find(|line| line.contains("SECCOMPILER_VERSION") && line.contains('='))?;
        let after = line.split_once('=')?.1;
        let start = after.find('"')? + 1;
        let rest = &after[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    }

    /// GAUNT-SECCOMPILER-PIN: the seccomp BPF assembler version is pinned in two
    /// places — bvisor's `Cargo.toml` (`seccompiler = "=0.5.0"`) and the
    /// `SECCOMPILER_VERSION` const recorded in every `SeccompEvidence`. A bump of
    /// one without the other means the evidence would misreport which assembler
    /// produced the committed BPF bytes. This gate ties the two together so they
    /// can never drift. (bvisor's own evidence test only proves the evidence
    /// mirrors the const, not that the const mirrors the Cargo pin.)
    #[test]
    fn seccompiler_cargo_pin_matches_source_const() {
        let root = repo_root().expect("locate workspace root");
        let manifest = root.join("crates/bvisor/Cargo.toml");
        let source = root.join("crates/bvisor/src/backend/linux/seccomp.rs");
        let manifest_text = fs::read_to_string(&manifest).expect("read bvisor Cargo.toml");
        let source_text = fs::read_to_string(&source).expect("read bvisor seccomp.rs");

        let cargo_pin =
            cargo_seccompiler_pin(&manifest_text).expect("seccompiler pin in bvisor Cargo.toml");
        let source_pin =
            source_seccompiler_pin(&source_text).expect("SECCOMPILER_VERSION const in seccomp.rs");
        assert_eq!(
            cargo_pin, source_pin,
            "seccompiler Cargo.toml pin `{cargo_pin}` != SECCOMPILER_VERSION `{source_pin}`; \
             a divergent pin makes SeccompEvidence misreport the BPF assembler version"
        );
        assert!(
            cargo_pin.starts_with('='),
            "seccompiler must stay exactly pinned (`=X.Y.Z`), got `{cargo_pin}`"
        );
    }
}

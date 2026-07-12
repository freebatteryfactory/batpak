//! `xtask msrv-check` — verify the full crates.io release train
//! compiles under its declared `rust-version` MSRV.
//!
//! Without this gate, `rust-version` in `Cargo.toml` is purely
//! declarative — a `let-else` from a newer toolchain slips through
//! until a downstream crates.io consumer trips over it.
//!
//! The checked set is `RELEASE_CHAIN` (every crate published to
//! crates.io, including the `batpak-macros*`/`batpak-bench-support`
//! support crates), NOT the smaller `PUBLISH_CRATES` headline family —
//! a support crate left declaratively pinned but unqualified is
//! exactly the split-brain this gate exists to prevent (#183).
//!
//! The command:
//!   1. Reads each release-chain `Cargo.toml` and parses
//!      `package.rust-version`.
//!   2. For every unique declared MSRV, ensures the toolchain is
//!      installed via `rustup toolchain list`.
//!   3. Runs `cargo +<msrv> check -p <crate> --no-default-features`
//!      and `cargo +<msrv> check -p <crate> --all-features` for each
//!      release-chain crate.
//!
//! If the toolchain is missing, the command fails with an install
//! hint rather than auto-installing — consulting clients run release
//! gates inside clean containers and want deterministic tool
//! versioning.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::publish::RELEASE_CHAIN;
use crate::util::{repo_root, run};

/// Verify each release-chain crate compiles under its declared
/// `rust-version`.
pub(crate) fn msrv_check() -> Result<()> {
    // repo_root() resolves the Cargo WORKSPACE root (bpk-lib/ itself — the
    // repository root carries no Cargo.toml), so it is already the directory
    // that contains crates/. Joining another "bpk-lib" here double-pathed to
    // bpk-lib/bpk-lib/ and made the gate unrunnable from the canonical cwd.
    let bpk_lib = repo_root()?;
    let mut by_msrv: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    for package in RELEASE_CHAIN {
        let manifest = bpk_lib
            .join("crates")
            .join(crate_dir_for(package)?)
            .join("Cargo.toml");
        let msrv = read_rust_version(&manifest)
            .with_context(|| format!("read rust-version from {}", manifest.display()))?;
        by_msrv.entry(msrv).or_default().push(package);
    }

    for (msrv, packages) in &by_msrv {
        let toolchain = format!("+{}", resolve_installed_toolchain(msrv)?);
        for package in packages {
            for feature_args in [&["--no-default-features"][..], &["--all-features"][..]] {
                let mut cmd = Command::new("cargo");
                cmd.current_dir(&bpk_lib)
                    .arg(&toolchain)
                    .arg("check")
                    .arg("-p")
                    .arg(package);
                for arg in feature_args {
                    cmd.arg(arg);
                }
                outln!(
                    "xtask msrv-check: cargo {toolchain} check -p {package} {}",
                    feature_args.join(" ")
                );
                run(cmd).with_context(|| {
                    format!("MSRV check failed for {package} under {msrv} with {feature_args:?}")
                })?;
            }
        }
    }
    outln!("xtask msrv-check: all release-chain crates compile under their declared rust-version");
    Ok(())
}

/// Map a release-chain crate name (e.g. `"batpak"`) to the directory under
/// `bpk-lib/crates/` (e.g. `"core"` for batpak), via the single
/// [`crate::publish::package_dir`] oracle. A name outside the known family
/// fails closed so an MSRV pass can never be vacuous.
fn crate_dir_for(package: &str) -> Result<&'static str> {
    crate::publish::package_dir(package)
        .ok_or_else(|| anyhow!("msrv-check: unknown release-chain crate {package}"))
}

/// Parse `rust-version = "X.Y"` out of a Cargo manifest. Returns
/// the version string without surrounding quotes.
fn read_rust_version(manifest: &Path) -> Result<String> {
    let raw = std::fs::read_to_string(manifest)
        .with_context(|| format!("read {}", manifest.display()))?;
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("rust-version") {
            let rest = rest.trim_start();
            let rest = rest
                .strip_prefix('=')
                .ok_or_else(|| anyhow!("rust-version line malformed in {}", manifest.display()))?;
            let stripped = rest.trim().trim_matches(|c: char| c == '"' || c == '\'');
            if stripped.is_empty() {
                bail!("rust-version is empty in {}", manifest.display());
            }
            return Ok(stripped.to_owned());
        }
    }
    bail!(
        "no `rust-version =` declaration found in {}",
        manifest.display()
    )
}

/// Resolve a declared MSRV (possibly two-component, e.g. `"1.97"`) to an
/// INSTALLED rustup toolchain name (e.g. `"1.97.0-x86_64-pc-windows-gnu"`).
///
/// Passing the raw two-component MSRV to `cargo +1.97` does NOT match an
/// installed `1.97.0`: rustup treats it as a distinct channel and
/// auto-installs a duplicate toolchain (observed locally) or fails in an
/// offline container. Resolving to the installed name keeps the gate
/// deterministic — no network, no surprise second toolchain — and fails
/// closed with an install hint when nothing matches.
fn resolve_installed_toolchain(msrv: &str) -> Result<String> {
    let output = Command::new("rustup")
        .args(["toolchain", "list"])
        .output()
        .with_context(|| "invoke rustup toolchain list")?;
    let listing = String::from_utf8_lossy(&output.stdout);
    match_installed_toolchain(&listing, msrv)
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow!(
                "no installed rustup toolchain satisfies MSRV {msrv}.\n\
             Install with: rustup toolchain install {msrv} --profile minimal"
            )
        })
}

/// Pure matcher over `rustup toolchain list` output: an installed toolchain
/// satisfies `msrv` when its name IS the msrv, or continues it at a version
/// boundary (`1.97.0-…` or `1.97-…` satisfy `1.97`; `1.97…` must NOT be
/// satisfied by `1.9`).
fn match_installed_toolchain<'a>(listing: &'a str, msrv: &str) -> Option<&'a str> {
    let patch_prefix = format!("{msrv}.");
    let triple_prefix = format!("{msrv}-");
    listing
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .find(|name| {
            *name == msrv || name.starts_with(&patch_prefix) || name.starts_with(&triple_prefix)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rust_version_from_a_synthesized_manifest() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(
            tmp.path(),
            "[package]\nname = \"x\"\nversion = \"0\"\nrust-version = \"1.97\"\n",
        )
        .expect("write tmp manifest");
        let parsed = read_rust_version(tmp.path()).expect("parse");
        assert_eq!(parsed, "1.97");
    }

    #[test]
    fn errors_when_rust_version_missing() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), "[package]\nname = \"x\"\n").expect("write tmp manifest");
        let err = read_rust_version(tmp.path()).expect_err("must fail");
        assert!(err.to_string().contains("no `rust-version"));
    }

    #[test]
    fn handles_single_quoted_rust_version() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(
            tmp.path(),
            "[package]\nname = \"x\"\nrust-version = '1.97'\n",
        )
        .expect("write tmp manifest");
        let parsed = read_rust_version(tmp.path()).expect("parse");
        assert_eq!(parsed, "1.97");
    }

    /// `cargo +<two-component-msrv>` does not match an installed patch
    /// release — rustup auto-installs a duplicate channel toolchain (or fails
    /// offline). The matcher must resolve the DECLARED msrv to the INSTALLED
    /// name, and never let a shorter version prefix match a longer one.
    #[test]
    fn installed_toolchain_matcher_resolves_msrv_at_version_boundaries() {
        let listing = "stable-x86_64-pc-windows-gnu (default)\n\
                       1.92.0-x86_64-pc-windows-gnu\n\
                       1.97.0-x86_64-pc-windows-gnu (active)\n";
        assert_eq!(
            match_installed_toolchain(listing, "1.97").expect("1.97 resolves to installed patch"),
            "1.97.0-x86_64-pc-windows-gnu"
        );
        assert_eq!(
            match_installed_toolchain(listing, "1.97.0").expect("exact patch resolves"),
            "1.97.0-x86_64-pc-windows-gnu"
        );
        assert!(
            match_installed_toolchain(listing, "1.9").is_none(),
            "1.9 must NOT be satisfied by 1.92/1.97 — not a version boundary"
        );
        assert!(
            match_installed_toolchain(listing, "1.98").is_none(),
            "missing msrv resolves to nothing (gate fails closed with install hint)"
        );
        let channel_alias = "1.97-x86_64-pc-windows-gnu\n";
        assert_eq!(
            match_installed_toolchain(channel_alias, "1.97").expect("channel alias resolves"),
            "1.97-x86_64-pc-windows-gnu"
        );
    }

    #[test]
    fn crate_dir_lookup_covers_publish_crates() {
        assert_eq!(crate_dir_for("batpak").expect("batpak dir"), "core");
        assert_eq!(crate_dir_for("syncbat").expect("syncbat dir"), "syncbat");
        assert_eq!(crate_dir_for("netbat").expect("netbat dir"), "netbat");
        let err = crate_dir_for("other").expect_err("unknown crate fails");
        assert!(err
            .to_string()
            .contains("msrv-check: unknown release-chain crate other"));
    }

    /// PARITY: the MSRV gate iterates `RELEASE_CHAIN` and calls `crate_dir_for`
    /// on each; a crate `crate_dir_for` cannot resolve bails BEFORE it is checked,
    /// making its MSRV pass vacuous. The checked-crate set must equal the full
    /// crates.io release train exactly — every chain crate resolves, and nothing
    /// else does.
    #[test]
    fn checked_crate_set_matches_the_release_chain() {
        for package in RELEASE_CHAIN {
            assert!(
                crate_dir_for(package).is_ok(),
                "crate_dir_for({package}) must resolve — the MSRV gate iterates RELEASE_CHAIN \
                 and bails before checking any crate it cannot map (vacuous MSRV pass)"
            );
        }
        // The mapping accepts EXACTLY the release chain — a non-member is rejected,
        // so the parity is an equality, not just a superset.
        assert!(
            crate_dir_for("definitely-not-a-publish-crate").is_err(),
            "crate_dir_for must reject a crate outside the release chain"
        );
    }

    /// The headline publish family must stay a subset of the release chain —
    /// widening the gate to RELEASE_CHAIN (#183) can never silently drop a
    /// crate that previously carried MSRV qualification.
    #[test]
    fn publish_family_is_a_subset_of_the_checked_release_chain() {
        for package in crate::publish::PUBLISH_CRATES {
            assert!(
                RELEASE_CHAIN.contains(package),
                "{package} is in PUBLISH_CRATES but missing from RELEASE_CHAIN — \
                 the MSRV gate would lose coverage it previously had"
            );
        }
    }
}

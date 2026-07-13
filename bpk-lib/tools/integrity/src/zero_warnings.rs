//! `zero-warnings-posture` gate (INV-ZERO-WARNINGS, #225).
//!
//! The repo advertises a ZERO-WARNINGS posture: a clean `-D warnings` build with
//! no silenced lints. That claim is worthless as prose — a stale sentence in a
//! doc cannot detect that someone downgraded `panic = "deny"` to `"warn"` or
//! dropped `-D warnings` from the clippy lane. This gate mechanically DERIVES the
//! posture from four load-bearing facts and fails the instant any of them is
//! weakened, so the posture is a machine-checked property, never a promise:
//!
//! 1. **Workspace lint table** — `[workspace.lints.clippy]` keeps the doctrine
//!    deny lints (`unwrap_used`, `panic`, `wildcard_enum_match_arm`, the cast and
//!    print lints, …) at `deny`, and `[workspace.lints.rust]` keeps `deprecated`
//!    at `deny`. A downgrade to `warn`/`allow` is a posture regression.
//! 2. **Clippy `-D warnings` lane** — the `cargo xtask ci-fast --lane lint` body
//!    (`tools/xtask/src/commands/ci.rs`) invokes clippy across the whole
//!    workspace/family with `-D warnings`. Dropping `--workspace`/`--all-targets`
//!    or the deny flag would let a warning ship.
//! 3. **Rustdoc `-D warnings` lane** — the same lane file builds workspace
//!    rustdoc with `RUSTDOCFLAGS=-D warnings` across `--all-features`.
//! 4. **Armed zero-allow tripwire** — `gate_registry.rs` still arms the
//!    `dead-code-silencers` and `allow-justifications` gates with blocking
//!    authority, so the AST detector that bans every `#[allow]`/`#[expect]` keeps
//!    its teeth.
//!
//! The gate reads the live files, but its decision core ([`check_over`]) is pure
//! over source strings so the red-fixture unit tests drive planted violations
//! without touching the tree.

use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::repo_surface::relative;

/// Clippy lints whose `deny` tier is load-bearing for the zero-warnings posture
/// (the doctrine set named in AGENTS.md). A downgrade of any one to `warn`/`allow`
/// is a posture regression the gate refuses.
const REQUIRED_CLIPPY_DENY: &[&str] = &[
    "unwrap_used",
    "panic",
    "todo",
    "unimplemented",
    "dbg_macro",
    "wildcard_enum_match_arm",
    "assertions_on_constants",
    "cast_possible_truncation",
    "cast_sign_loss",
    "print_stdout",
    "print_stderr",
];

/// Rust (rustc) lints that must stay `deny` in `[workspace.lints.rust]`.
const REQUIRED_RUST_DENY: &[&str] = &["deprecated"];

/// The zero-allow tripwire gates that must stay armed (blocking) in the registry.
const TRIPWIRE_SLUGS: &[&str] = &["dead-code-silencers", "allow-justifications"];

/// Production entry: derive the zero-warnings posture from the live tree and fail
/// on any weakened fact. Returns the set of files the derivation read (for the
/// gate receipt).
pub(crate) fn check(repo_root: &Path) -> Result<BTreeSet<PathBuf>> {
    let manifest_path = repo_root.join("Cargo.toml");
    let lane_path = repo_root.join("tools/xtask/src/commands/ci.rs");
    let registry_path = repo_root.join("tools/integrity/src/gate_registry.rs");

    let manifest_src = read(repo_root, &manifest_path)?;
    let lane_src = read(repo_root, &lane_path)?;
    let registry_src = read(repo_root, &registry_path)?;

    check_over(&manifest_src, &lane_src, &registry_src)?;

    let mut inputs = BTreeSet::new();
    inputs.insert(manifest_path);
    inputs.insert(lane_path);
    inputs.insert(registry_path);
    Ok(inputs)
}

fn read(repo_root: &Path, path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("zero-warnings-posture: read {}", relative(repo_root, path)))
}

/// Pure decision core over source strings. Each leg `bail!`s with a message
/// naming the fact and the cure so a posture regression is self-diagnosing.
pub(crate) fn check_over(manifest_src: &str, lane_src: &str, registry_src: &str) -> Result<()> {
    check_lint_table(manifest_src)?;
    check_clippy_lane(lane_src)?;
    check_rustdoc_lane(lane_src)?;
    check_tripwire_armed(registry_src)?;
    Ok(())
}

/// Leg 1: the workspace lint table keeps the doctrine lints at `deny`.
fn check_lint_table(manifest_src: &str) -> Result<()> {
    let clippy = section_map(manifest_src, "workspace.lints.clippy");
    if clippy.is_empty() {
        bail!(
            "zero-warnings-posture (INV-ZERO-WARNINGS): no `[workspace.lints.clippy]` table in \
             Cargo.toml — the workspace deny-lint posture is unset. Restore the doctrine deny lints."
        );
    }
    for lint in REQUIRED_CLIPPY_DENY {
        require_deny(&clippy, lint, "workspace.lints.clippy")?;
    }
    let rust = section_map(manifest_src, "workspace.lints.rust");
    for lint in REQUIRED_RUST_DENY {
        require_deny(&rust, lint, "workspace.lints.rust")?;
    }
    Ok(())
}

fn require_deny(table: &BTreeMap<String, String>, lint: &str, section: &str) -> Result<()> {
    match table.get(lint) {
        Some(level) if level == "deny" => Ok(()),
        Some(level) => bail!(
            "zero-warnings-posture (INV-ZERO-WARNINGS): lint `{lint}` in `[{section}]` is `{level}`, \
             expected `deny`. Downgrading a doctrine lint reopens the zero-warnings posture; keep it \
             at `deny`."
        ),
        None => bail!(
            "zero-warnings-posture (INV-ZERO-WARNINGS): lint `{lint}` is missing from `[{section}]`. \
             The zero-warnings posture requires it at `deny`."
        ),
    }
}

/// Leg 2: the clippy lane denies warnings across the whole workspace + family.
fn check_clippy_lane(lane_src: &str) -> Result<()> {
    for needle in ["clippy", "--workspace", "--all-targets", "--all-features", "-D", "warnings"] {
        if !lane_src.contains(needle) {
            bail!(
                "zero-warnings-posture (INV-ZERO-WARNINGS): the clippy lint lane \
                 (tools/xtask/src/commands/ci.rs) is missing `{needle}` — the `-D warnings` clippy \
                 invocation over `--workspace --all-targets --all-features` is what makes a warning \
                 fail CI. Restore the full deny-warnings clippy lane."
            );
        }
    }
    Ok(())
}

/// Leg 3: the rustdoc lane denies warnings across all features.
fn check_rustdoc_lane(lane_src: &str) -> Result<()> {
    for needle in ["RUSTDOCFLAGS", "-D warnings", "doc", "--all-features"] {
        if !lane_src.contains(needle) {
            bail!(
                "zero-warnings-posture (INV-ZERO-WARNINGS): the rustdoc lane \
                 (tools/xtask/src/commands/ci.rs) is missing `{needle}` — rustdoc must build with \
                 `RUSTDOCFLAGS=-D warnings` over `--all-features` so a doc warning fails CI. Restore \
                 the deny-warnings rustdoc lane."
            );
        }
    }
    Ok(())
}

/// Leg 4: the zero-allow tripwire gates stay armed with blocking authority.
fn check_tripwire_armed(registry_src: &str) -> Result<()> {
    for slug in TRIPWIRE_SLUGS {
        if !tripwire_armed(registry_src, slug) {
            bail!(
                "zero-warnings-posture (INV-ZERO-WARNINGS): the zero-allow tripwire gate `{slug}` is \
                 not armed with `has_blocking_authority: true` in gate_registry.rs. A disarmed \
                 tripwire lets a silencing `#[allow]`/`#[expect]` through and voids the zero-warnings \
                 posture. Re-arm the gate."
            );
        }
    }
    Ok(())
}

/// True when the `Gate { slug: "<slug>", … }` literal in `registry_src` carries
/// `has_blocking_authority: true` (the field that follows the slug in the struct).
fn tripwire_armed(registry_src: &str, slug: &str) -> bool {
    let needle = format!("slug: \"{slug}\"");
    let Some(at) = registry_src.find(&needle) else {
        return false;
    };
    let rest = &registry_src[at..];
    let Some(auth_at) = rest.find("has_blocking_authority:") else {
        return false;
    };
    rest[auth_at + "has_blocking_authority:".len()..]
        .trim_start()
        .starts_with("true")
}

/// Collect `key = level` pairs from a single TOML `[section]`, where `level` is
/// the lint level (`"deny"`) either as a bare string value or the `level = "…"`
/// field of a table value. Comment and blank lines are skipped; nested table
/// keys (containing spaces after trimming) are ignored.
fn section_map(src: &str, section: &str) -> BTreeMap<String, String> {
    let header = format!("[{section}]");
    let mut map = BTreeMap::new();
    let mut in_section = false;
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == header;
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.contains(' ') {
            continue;
        }
        if let Some(level) = parse_level(rest) {
            map.insert(key.to_string(), level);
        }
    }
    map
}

/// Extract the lint level string from the right-hand side of a lint entry:
/// `"deny"` → `deny`; `{ level = "deny", … }` → `deny`.
fn parse_level(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if rest.starts_with('{') {
        let idx = rest.find("level")?;
        first_quoted(&rest[idx..]).map(str::to_string)
    } else {
        first_quoted(rest).map(str::to_string)
    }
}

/// Content between the first pair of double quotes in `s`.
fn first_quoted(s: &str) -> Option<&str> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some(&s[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn green_manifest() -> String {
        let mut src = String::from("[workspace.lints.clippy]\n");
        for lint in REQUIRED_CLIPPY_DENY {
            src.push_str(&format!("{lint} = \"deny\"\n"));
        }
        // A deliberate `allow` and a table-valued entry must not disturb parsing.
        src.push_str("module_name_repetitions = \"allow\"\n");
        src.push_str("\n[workspace.lints.rust]\n");
        for lint in REQUIRED_RUST_DENY {
            src.push_str(&format!("{lint} = \"deny\"\n"));
        }
        src.push_str("unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(x)'] }\n");
        src
    }

    fn green_lane() -> String {
        "fn run_workspace_clippy() -> Result<()> {\n\
             cargo([\"clippy\", \"--workspace\", \"--all-features\", \"--all-targets\",\n\
                    \"--\", \"-D\", \"warnings\"])\n\
         }\n\
         fn doc_deny_warnings() -> Result<()> {\n\
             cmd.args([\"doc\", \"--workspace\", \"--all-features\", \"--no-deps\"])\n\
                .env(\"RUSTDOCFLAGS\", \"-D warnings\");\n\
         }\n"
            .to_string()
    }

    fn gate_literal(slug: &str, blocking: bool) -> String {
        format!(
            "    Gate {{\n        slug: \"{slug}\",\n        red_fixture_test: Some(\"x::y\"),\n        \
             red_fixture_kind: Some(RedFixtureKind::GateNegativePath),\n        \
             has_blocking_authority: {blocking},\n    }},\n"
        )
    }

    fn registry_with(dead_armed: bool, allow_armed: bool) -> String {
        format!(
            "pub(crate) const GATES: &[Gate] = &[\n{}{}];\n",
            gate_literal("dead-code-silencers", dead_armed),
            gate_literal("allow-justifications", allow_armed),
        )
    }

    fn green_registry() -> String {
        registry_with(true, true)
    }

    #[test]
    fn green_posture_passes() {
        check_over(&green_manifest(), &green_lane(), &green_registry())
            .expect("the derived green posture must pass");
    }

    /// REGISTERED red fixture (gate_registry `zero-warnings-posture`): a doctrine
    /// deny lint downgraded to `warn` is a posture regression and must be rejected.
    #[test]
    fn planted_lint_downgrade_fails() {
        let manifest = green_manifest().replace("panic = \"deny\"", "panic = \"warn\"");
        let err = check_over(&manifest, &green_lane(), &green_registry())
            .expect_err("a downgraded doctrine deny lint must be rejected");
        assert!(
            err.to_string().contains("panic") && err.to_string().contains("deny"),
            "wrong rejection: {err}"
        );
    }

    #[test]
    fn missing_clippy_lint_table_is_rejected() {
        let err = check_over("[workspace]\nmembers = []\n", &green_lane(), &green_registry())
            .expect_err("a missing clippy lint table must be rejected");
        assert!(err.to_string().contains("lints.clippy"), "wrong rejection: {err}");
    }

    #[test]
    fn clippy_lane_without_full_deny_surface_is_rejected() {
        // `--all-targets` is clippy-only (rustdoc uses `--no-deps`), so dropping
        // it reds the clippy leg specifically.
        let lane = green_lane().replace("--all-targets", "");
        let err = check_over(&green_manifest(), &lane, &green_registry())
            .expect_err("a clippy lane missing `--all-targets` must be rejected");
        assert!(err.to_string().contains("clippy"), "wrong rejection: {err}");
    }

    #[test]
    fn rustdoc_lane_without_deny_warnings_is_rejected() {
        let lane = green_lane().replace("-D warnings", "");
        let err = check_over(&green_manifest(), &lane, &green_registry())
            .expect_err("a rustdoc lane without `-D warnings` must be rejected");
        assert!(err.to_string().contains("rustdoc"), "wrong rejection: {err}");
    }

    #[test]
    fn disarmed_zero_allow_tripwire_is_rejected() {
        let registry = registry_with(true, false);
        let err = check_over(&green_manifest(), &green_lane(), &registry)
            .expect_err("a disarmed zero-allow tripwire must be rejected");
        assert!(
            err.to_string().contains("allow-justifications"),
            "wrong rejection: {err}"
        );
    }

    #[test]
    fn section_map_reads_bare_and_table_levels() {
        let map = section_map(&green_manifest(), "workspace.lints.rust");
        assert_eq!(map.get("deprecated").map(String::as_str), Some("deny"));
        assert_eq!(map.get("unexpected_cfgs").map(String::as_str), Some("warn"));
    }
}

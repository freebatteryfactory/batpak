use crate::repo_surface::{ensure, files_with_extension, relative};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

/// Human-facing `just` recipes map to one or more xtask policy commands.
const JUST_TO_XTASK_COMMANDS: &[(&str, &[&str])] = &[
    ("ci-fast", &["ci-fast"]),
    ("ci-windows", &["ci-windows-surface"]),
    ("verify", &["preflight"]),
    ("mutants-smoke", &["mutants"]),
    ("mutants-full", &["mutants"]),
    (
        "inspect",
        &["structural", "boundary", "architecture-ir", "ast-grep"],
    ),
    (
        "seal",
        &["check-version-pins", "evidence-audit", "release-manifest"],
    ),
    ("ship-dry", &["release"]),
    ("ship-real", &["release"]),
    ("ci", &["ci"]),
];

/// Assert that the live GitHub Actions workflows do not drift from the local
/// `cargo xtask` command surface and the canonical devcontainer Dockerfile.
///
/// This is the safety harness that catches the kind of drift we hit
/// repeatedly during the v0.3 prep work: someone updates a workflow without
/// updating the xtask source tree (or vice versa), and CI passes locally but
/// fails on GitHub because the two pipelines run different commands. The check
/// is purely mechanical:
///
/// 1. Every `cargo xtask <subcommand>` referenced in `ci.yml`, `perf.yml`,
///    or `release.yml` must exist
///    as an `XtaskCommand` variant in `tools/xtask/src/main.rs`.
/// 2. Every tool installed via `taiki-e/install-action` in the workflow
///    must also be installed by `cargo xtask setup --install-tools` in the xtask source tree.
/// 3. The Dockerfile and the workflow must agree on tool versions for
///    `cargo-deny`, `cargo-llvm-cov`, `cargo-mutants`, `cargo-nextest`, and
///    `cargo-audit` (the tools we care about pinning).
/// 4. Workflow-owned matrix values for perf surfaces and manual full-mutation
///    shards must stay inside the exact xtask-owned truth set.
///
/// The implementation uses string-grep instead of YAML parsing to keep the
/// dependency surface minimal and the failure messages legible. If the
/// workflow YAML reorganizes substantially, this check will need updates,
/// which is the right behavior — drift detection requires the check to
/// itself be regularly maintained.
/// Construct a regex that matches `<tool>@<semver>` for a caller-supplied
/// `tool` name. The `tool` string is validated to contain only
/// `[A-Za-z0-9-]+` so no regex metacharacter can slip through. A malformed
/// tool name returns an error rather than relying on `.unwrap()` at regex
/// build time.
fn build_tool_pin_regex(tool: &str) -> Result<Regex> {
    if tool.is_empty()
        || !tool
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!(
            "internal error: tool name `{tool}` contains characters outside [A-Za-z0-9_-]; refusing to build a regex that would be shaped by user input"
        );
    }
    Regex::new(&format!(r"{tool}@(\d+(?:\.\d+)+)"))
        .with_context(|| format!("compile tool-pin regex for `{tool}`"))
}

pub(crate) fn check(repo_root: &Path) -> Result<()> {
    let project_root = repo_root.parent().unwrap_or(repo_root);
    let ci_yml = fs::read_to_string(project_root.join(".github/workflows/ci.yml"))
        .context("read .github/workflows/ci.yml")?;
    let perf_yml = fs::read_to_string(project_root.join(".github/workflows/perf.yml"))
        .context("read .github/workflows/perf.yml")?;
    let release_yml = fs::read_to_string(project_root.join(".github/workflows/release.yml"))
        .context("read .github/workflows/release.yml")?;
    let xtask_main = fs::read_to_string(repo_root.join("tools/xtask/src/main.rs"))
        .context("read tools/xtask/src/main.rs")?;
    let xtask_sources = xtask_source_text(repo_root)?;
    // Anti-rebury (P1-1): the L2+ contract gates must stay on the default PR
    // path inside the `ci_fast` lane family. This blocks silently moving them
    // back into the label-gated `ci()`/`preflight()` lanes.
    assert_ci_fast_keeps_default_path_gates(&xtask_sources)?;
    // Fan-out parity: every ci-fast lane must exist as an always-on ci.yml job
    // and fan back into the `ci-fast-linux` summary (the required check).
    assert_ci_fast_fanout_jobs(&ci_yml)?;
    let dockerfile = fs::read_to_string(project_root.join(".devcontainer/Dockerfile"))
        .context("read .devcontainer/Dockerfile")?;
    let workflows = [
        (".github/workflows/ci.yml", ci_yml.as_str()),
        (".github/workflows/perf.yml", perf_yml.as_str()),
        (".github/workflows/release.yml", release_yml.as_str()),
    ];

    // 1. Every `cargo xtask <subcommand>` in the live workflows must exist in
    //    xtask main.rs.
    //    Match `cargo xtask <word>` patterns and look the word up in the
    //    XtaskCommand enum.
    // justifies: INV-LITERAL-REGEX-UNWRAP-SAFE; pattern is a string literal known-safe at compile time in tools/integrity/src/structural.rs; this expect cannot fire in any reachable code path
    let xtask_cmd_re = Regex::new(r"cargo\s+xtask\s+([a-z][a-z0-9-]*)")
        .expect("internal regex is a compile-time constant and will compile");
    let mut found_subcommands: BTreeSet<String> = BTreeSet::new();
    for (_, workflow) in workflows {
        for cap in xtask_cmd_re.captures_iter(workflow) {
            if let Some(sub) = cap.get(1) {
                found_subcommands.insert(sub.as_str().to_string());
            }
        }
    }
    assert_workflow_just_recipes_map_to_xtask(&workflows, &xtask_main)?;

    for sub in &found_subcommands {
        // Map kebab-case to PascalCase: "perf-gates" → "PerfGates".
        let pascal: String = sub
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect();
        // The variant must appear in the XtaskCommand enum (with optional args).
        let needle_a = format!("    {pascal},");
        let needle_b = format!("    {pascal}(");
        if !xtask_main.contains(&needle_a) && !xtask_main.contains(&needle_b) {
            bail!(
                "ci-parity: workflow references `cargo xtask {sub}` but no \
                 matching `XtaskCommand::{pascal}` variant in \
                 tools/xtask/src/main.rs. Either add the command to xtask \
                 or fix the workflow."
            );
        }
    }

    assert_workflow_list_values(
        ".github/workflows/perf.yml",
        &perf_yml,
        "surface",
        &["neutral", "native"],
    )?;
    assert_workflow_list_values(
        ".github/workflows/ci.yml",
        &ci_yml,
        "surface",
        &["all-features", "no-default-features"],
    )?;
    assert_workflow_list_values(
        ".github/workflows/ci.yml",
        &ci_yml,
        "shard",
        &[
            "0/12", "1/12", "2/12", "3/12", "4/12", "5/12", "6/12", "7/12", "8/12", "9/12",
            "10/12", "11/12",
        ],
    )?;
    assert_workflow_list_values(
        ".github/workflows/ci.yml",
        &ci_yml,
        "seam",
        &[
            "writer-commit",
            "cursor-delivery",
            "projection-flow",
            "projection-fusion",
            "segment-scan",
            "hash-chain-replay",
            "frontier-wait-durable",
            "frontier-append-gate",
            "event-payload-registry-validator",
            "platform-backend",
            "testing-ledger-structural-lint",
            "integrity-graders",
            "syncbat-runtime-dispatch",
            "syncbat-register-catalog",
            "syncbat-subscription-runtime",
            "netbat-boundary-protocol",
            "fork-isolation",
            "import-reapply",
            "lane-branch",
            "lane-frontier",
            "bvisor-admission",
            "bvisor-report-seal",
        ],
    )?;
    assert_workflow_list_values(
        ".github/workflows/ci.yml",
        &ci_yml,
        "features",
        &[
            "",
            "--features dangerous-test-hooks",
            "--no-default-features",
            "--no-default-features --features dangerous-test-hooks",
            "--all-features",
        ],
    )?;
    ensure(
        release_yml.contains("bash ./scripts/run-in-devcontainer.sh 'cargo xtask release --dry-run'"),
        "ci-parity: `.github/workflows/release.yml` must run `cargo xtask release --dry-run` through `scripts/run-in-devcontainer.sh` from bpk-lib.",
    )?;

    // 2. Every tool installed via taiki-e/install-action in ci.yml must
    //    be pinned to the same version that `.devcontainer/Dockerfile`
    //    and `cargo xtask setup --install-tools` use.
    //
    //    This guards three drift vectors at once:
    //    (a) workflow installs an unpinned tool (Windows CI silently picks
    //        up a new release that breaks against pinned Linux);
    //    (b) workflow pins to a different version than the container;
    //    (c) tool added to workflow but missing from xtask setup.
    //
    //    The regex requires the canonical `name@x.y[.z]` form. A bare
    //    `tool: nextest` is intentionally rejected — see ci.yml for the
    //    drift comment that explains the lock-step requirement.
    // justifies: INV-LITERAL-REGEX-UNWRAP-SAFE; pattern is a string literal known-safe at compile time in tools/integrity/src/structural.rs; this expect cannot fire in any reachable code path
    let tool_install_re = Regex::new(r#"tool:\s*([a-z][a-z0-9-]*)@(\d+(?:\.\d+)+)"#)
        .expect("internal regex is a compile-time constant and will compile");
    // justifies: INV-LITERAL-REGEX-UNWRAP-SAFE; pattern is a string literal known-safe at compile time in tools/integrity/src/structural.rs; this expect cannot fire in any reachable code path
    let bare_tool_re = Regex::new(r#"tool:\s*([a-z][a-z0-9-]*)\s*$"#)
        .expect("internal regex is a compile-time constant and will compile");
    // Reject any unpinned `tool:` entry up front so we never have to wonder
    // why a Windows install drifted from the canonical Linux pin.
    for line in ci_yml.lines() {
        if let Some(cap) = bare_tool_re.captures(line.trim_end()) {
            let tool = cap.get(1).map(|m| m.as_str()).unwrap_or("?");
            bail!(
                "ci-parity: `.github/workflows/ci.yml` installs `{tool}` via \
                 taiki-e/install-action without a version pin. Use \
                 `tool: {tool}@<version>` so Linux and Windows CI install the \
                 same version. Match the pin in `.devcontainer/Dockerfile` \
                 and `cargo xtask setup --install-tools`."
            );
        }
    }
    let mut workflow_tools: BTreeSet<(String, String)> = BTreeSet::new();
    for cap in tool_install_re.captures_iter(&ci_yml) {
        if let (Some(tool), Some(ver)) = (cap.get(1), cap.get(2)) {
            workflow_tools.insert((tool.as_str().to_string(), ver.as_str().to_string()));
        }
    }
    for (tool, wf_ver) in &workflow_tools {
        // The xtask setup list must contain `"{tool}"` as a tuple key
        // followed (within ~80 bytes on the next install line) by
        // `{tool}@{version}`. Both checks together kill the
        // accidental-substring-match failure mode.
        let setup_key = format!("\"{tool}\"");
        if !xtask_sources.contains(&setup_key) {
            bail!(
                "ci-parity: workflow installs `{tool}@{wf_ver}` via \
                 taiki-e/install-action but `cargo xtask setup --install-tools` in \
                 tools/xtask/src/ does not list a `{setup_key}` entry. \
                 Either add it to setup or remove from the workflow."
            );
        }
        let xtask_pin_re = build_tool_pin_regex(tool)?;
        let xtask_ver = xtask_pin_re
            .captures(&xtask_sources)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        match xtask_ver {
            Some(x) if &x != wf_ver => {
                bail!(
                    "ci-parity: tool `{tool}` is pinned to `{wf_ver}` in \
                     `.github/workflows/ci.yml` but `{x}` in `cargo xtask setup --install-tools` \
                     (tools/xtask/src/). Pick one version and update both."
                );
            }
            None => {
                bail!(
                    "ci-parity: tool `{tool}` is pinned in \
                     `.github/workflows/ci.yml` but unpinned (or missing) in \
                     `cargo xtask setup --install-tools`. Add the same pin so local installs \
                     match the workflow."
                );
            }
            _ => {}
        }
    }

    let docker_pins = dockerfile_tool_pins(&dockerfile)?;

    // 3. Every workflow-owned tool pin must also be present in the
    //    Dockerfile with the same version. This is derived from the workflow
    //    install-action entries instead of a fixed list so newly added tools
    //    are covered automatically.
    for (tool, wf_ver) in &workflow_tools {
        match docker_pins.get(tool) {
            Some(dock_ver) if dock_ver == wf_ver => {}
            Some(dock_ver) => {
                bail!(
                    "ci-parity: tool `{tool}` is pinned to `{wf_ver}` in \
                     `.github/workflows/ci.yml` but `{dock_ver}` in \
                     `.devcontainer/Dockerfile`. Pick one version and update both."
                );
            }
            None => {
                bail!(
                    "ci-parity: workflow installs `{tool}@{wf_ver}` via \
                     taiki-e/install-action but `.devcontainer/Dockerfile` does not pin `{tool}`. \
                     Add the same tool pin to the Dockerfile or remove it from the workflow."
                );
            }
        }
    }

    // 4. Tool version pin parity between Dockerfile and xtask setup. This is
    //    also dynamic over the Dockerfile so a new canonical container tool
    //    must be represented in xtask setup.
    for (tool, dock_v) in &docker_pins {
        let xtask_pin_re = build_tool_pin_regex(tool)?;
        let xtask_v = xtask_pin_re
            .captures(&xtask_sources)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        match xtask_v {
            Some(x) if &x != dock_v => {
                bail!(
                    "ci-parity: tool `{tool}` is pinned to `{d}` in \
                     `.devcontainer/Dockerfile` but `{x}` in `cargo xtask setup --install-tools` \
                     (tools/xtask/src/). Pick one version and update both.",
                    d = dock_v
                );
            }
            None => {
                bail!(
                    "ci-parity: tool `{tool}` is pinned in \
                     `.devcontainer/Dockerfile` but unpinned (or missing) in \
                     `cargo xtask setup --install-tools`. Add the same pin to xtask setup so \
                     local installs match the container."
                );
            }
            _ => {}
        }
    }

    Ok(())
}

pub(crate) fn dockerfile_tool_pins(dockerfile: &str) -> Result<HashMap<String, String>> {
    // justifies: INV-LITERAL-REGEX-UNWRAP-SAFE; pattern is a string literal known-safe at compile time in tools/integrity/src/structural.rs; this expect cannot fire in any reachable code path
    let pin_re = Regex::new(r"\b(cargo-[a-z0-9-]+)@(\d+(?:\.\d+)+)\b")
        .expect("internal regex is a compile-time constant and will compile");
    let mut pins = HashMap::new();
    for cap in pin_re.captures_iter(dockerfile) {
        let tool = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let version = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
        match pins.insert(tool.to_owned(), version.to_owned()) {
            Some(existing) if existing != version => {
                bail!(
                    "ci-parity: `.devcontainer/Dockerfile` pins `{tool}` to both `{existing}` and `{version}`"
                );
            }
            _ => {}
        }
    }
    Ok(pins)
}

fn assert_workflow_list_values(
    workflow_name: &str,
    workflow: &str,
    key: &str,
    expected: &[&str],
) -> Result<()> {
    let expected_set: BTreeSet<String> = expected.iter().map(|value| (*value).to_owned()).collect();
    let actual_set: BTreeSet<String> = workflow_list_values(workflow, key)?.into_iter().collect();
    ensure(
        actual_set == expected_set,
        format!(
            "ci-parity: `{workflow_name}` must declare `{key}` values {:?}, found {:?}",
            expected, actual_set
        ),
    )?;
    Ok(())
}

pub(crate) fn workflow_list_values(workflow: &str, key: &str) -> Result<Vec<String>> {
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        bail!(
            "internal error: workflow list key `{key}` must be `[A-Za-z0-9_]+` so regex construction is safe"
        );
    }
    let inline_re = Regex::new(&format!(r"^\s*{}\s*:\s*\[(?P<values>[^\]]+)\]\s*$", key))
        .with_context(|| format!("compile workflow list regex for key `{key}`"))?;
    let mut lines = workflow.lines().enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        if let Some(caps) = inline_re.captures(line) {
            let values = caps["values"]
                .split(',')
                .map(|value| value.trim().trim_matches('"').trim_matches('\'').to_owned())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            return Ok(values);
        }

        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(key) else {
            continue;
        };
        if !rest.trim_start().starts_with(':') {
            continue;
        }
        let base_indent = indentation(line);
        let mut values = Vec::new();
        while let Some((_, next_line)) = lines.peek() {
            let next_trimmed = next_line.trim();
            if next_trimmed.is_empty() {
                lines.next();
                continue;
            }
            let next_indent = indentation(next_line);
            if next_indent <= base_indent {
                break;
            }
            if let Some(value) = next_trimmed.strip_prefix("- ") {
                values.push(value.trim().trim_matches('"').trim_matches('\'').to_owned());
                lines.next();
                continue;
            }
            break;
        }
        if !values.is_empty() {
            return Ok(values);
        }
        bail!(
            "ci-parity: found `{key}:` in workflow but could not read any values near line {}",
            index + 1
        );
    }

    bail!("ci-parity: could not find `{key}` list in workflow")
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn assert_workflow_just_recipes_map_to_xtask(
    workflows: &[(&str, &str)],
    xtask_main: &str,
) -> Result<()> {
    // justifies: INV-LITERAL-REGEX-UNWRAP-SAFE; pattern is a string literal known-safe at compile time in tools/integrity/src/ci_parity.rs; this expect cannot fire in any reachable code path
    let just_re = Regex::new(r"\bjust\s+([a-z][a-z0-9-]*)\b")
        .expect("internal regex is a compile-time constant and will compile");
    let mut found_recipes: BTreeSet<(String, String)> = BTreeSet::new();
    for (workflow_name, workflow) in workflows {
        for cap in just_re.captures_iter(workflow) {
            if let Some(recipe) = cap.get(1) {
                found_recipes.insert((workflow_name.to_string(), recipe.as_str().to_string()));
            }
        }
    }
    for (workflow_name, recipe) in &found_recipes {
        let xtask_cmds = match JUST_TO_XTASK_COMMANDS
            .iter()
            .find(|(just_recipe, _)| just_recipe == recipe)
        {
            Some((_, cmds)) => *cmds,
            None => {
                bail!(
                    "ci-parity: `{workflow_name}` references `just {recipe}` but it is not \
                     registered in JUST_TO_XTASK_COMMANDS (tools/integrity/src/ci_parity.rs). \
                     Add the mapping or stop calling the recipe from CI."
                );
            }
        };
        for xtask_cmd in xtask_cmds {
            let pascal: String = xtask_cmd
                .split('-')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                        None => String::new(),
                    }
                })
                .collect();
            let needle_a = format!("    {pascal},");
            let needle_b = format!("    {pascal}(");
            if !xtask_main.contains(&needle_a) && !xtask_main.contains(&needle_b) {
                bail!(
                    "ci-parity: `just {recipe}` maps to `cargo xtask {xtask_cmd}` but no \
                     matching `XtaskCommand::{pascal}` variant exists in tools/xtask/src/main.rs"
                );
            }
        }
    }
    Ok(())
}

/// The exact xtask call substrings that MUST appear inside the `ci_fast`
/// function family (the serial dispatch plus its `ci_fast_<lane>` lane
/// functions) so the L2+ contract gates stay on the default PR path. Each is a
/// distinctive marker of the corresponding gate invocation (P1-1). If a future
/// edit re-buries a gate (moves it back into the label-gated `ci()`/`preflight`
/// lanes, or drops it entirely), the marker disappears from every `ci_fast*`
/// body and this gate fails — preventing silent re-burial.
const CI_FAST_REQUIRED_GATE_MARKERS: &[(&str, &str)] = &[
    ("template compile", "templates()?"),
    ("coverage floor", "coverage::cover(CoverArgs"),
    (
        "public-api baseline",
        "crate::public_api::public_api(PublicApiArgs",
    ),
    (
        "package-leak-scan",
        "super::package_leak_scan(PackageLeakScanArgs",
    ),
    ("doctor --strict", "integrity(\"doctor\", [\"--strict\"])"),
];

/// The named fan-out lanes of `ci-fast`. Kept in lockstep with
/// `CiFastLane` in tools/xtask/src/main.rs and with the `ci-fast-<lane>` jobs
/// in `.github/workflows/ci.yml`: every lane must exist as an always-on
/// (rust-changed-gated, never label-gated) ci.yml job AND stay wired into the
/// serial `ci_fast()` dispatch, so the local one-command bundle and the CI
/// fan-out enforce the same gate set.
const CI_FAST_LANES: &[&str] = &["check", "lint", "test", "contracts", "coverage"];

/// Extract the body of EVERY function in the `ci_fast` family — any
/// `fn ci_fast…(` definition — in the concatenated xtask source surface. That
/// covers the serial dispatch (`fn ci_fast(args: CiFastArgs)`), the one-line
/// delegator in `commands.rs`, and the `fn ci_fast_<lane>()` lane functions the
/// gates actually live in. Each body is the text between the function's
/// signature line (ending in `{`) and the next top-level `}` (a line that is
/// exactly `}`), which is how rustfmt closes a free function in this tree.
/// Scoping to the family — and nothing else — is what lets the anti-rebury
/// assertion catch true re-burial: a gate moved out into `ci()`/`preflight`
/// appears in NO `ci_fast*` body.
fn extract_ci_fast_family_bodies(xtask_sources: &str) -> Result<Vec<String>> {
    let mut bodies = Vec::new();
    let mut lines = xtask_sources.lines().peekable();
    while let Some(line) = lines.next() {
        if !line.contains("fn ci_fast") || !line.trim_end().ends_with('{') {
            continue;
        }
        let mut body = String::new();
        let mut closed = false;
        for body_line in lines.by_ref() {
            if body_line == "}" {
                closed = true;
                break;
            }
            body.push_str(body_line);
            body.push('\n');
        }
        if !closed {
            bail!(
                "ci-parity: found a `fn ci_fast…` signature but could not locate its closing \
                 `}}` at column 0 in the xtask source surface; the anti-rebury check cannot \
                 scope to its body."
            );
        }
        bodies.push(body);
    }
    if bodies.is_empty() {
        bail!(
            "ci-parity: could not find any `fn ci_fast…` definition in the xtask source \
             surface (tools/xtask/src/). The default-path fast lane must exist so its L2+ \
             gates can be verified."
        );
    }
    Ok(bodies)
}

/// Assert that the `ci_fast` family still invokes every L2+ contract gate on
/// the default PR path, and that the serial dispatch still runs every fan-out
/// lane. Each gate marker must appear in at least one `ci_fast*` body; a gate
/// re-buried into `ci()`/`preflight` appears in none and fails. Additionally,
/// the no-lane serial path (the `args.lane else` block) must call every
/// `ci_fast_<lane>()` so a bare `cargo xtask ci-fast` keeps covering the full
/// lane set locally. See [`CI_FAST_REQUIRED_GATE_MARKERS`] and
/// [`CI_FAST_LANES`].
fn assert_ci_fast_keeps_default_path_gates(xtask_sources: &str) -> Result<()> {
    let bodies = extract_ci_fast_family_bodies(xtask_sources)?;
    for (gate, marker) in CI_FAST_REQUIRED_GATE_MARKERS {
        if !bodies.iter().any(|body| body.contains(marker)) {
            bail!(
                "ci-parity: the `ci_fast` lane family no longer invokes the {gate} gate \
                 (expected to find `{marker}` in a `ci_fast*` body). The L2+ contract gates \
                 must run on the DEFAULT PR path (P1-1); do not re-bury them in the \
                 label-gated `ci()`/`preflight` lanes. If you are intentionally relocating \
                 the gate, update CI_FAST_REQUIRED_GATE_MARKERS in \
                 tools/integrity/src/ci_parity.rs and the meta-gate/gate_registry accordingly."
            );
        }
    }
    // Scope the serial-completeness proof to the NO-LANE path (the
    // `let Some(lane) = args.lane else { … }` block), not the whole dispatch
    // body: the single-lane `match` arms also spell every `ci_fast_<lane>()`
    // call, so a whole-body scan would stay green even after the bare
    // `cargo xtask ci-fast` path silently dropped a lane.
    let serial_dispatch_complete = bodies.iter().any(|body| {
        let Some(else_start) = body.find("args.lane else {") else {
            return false;
        };
        let after_else = &body[else_start..];
        let Some(else_end) = after_else.find("};") else {
            return false;
        };
        let serial_block = &after_else[..else_end];
        CI_FAST_LANES
            .iter()
            .all(|lane| serial_block.contains(&format!("ci_fast_{lane}()")))
    });
    ensure(
        serial_dispatch_complete,
        "ci-parity: the no-lane serial path of `ci_fast()` (the `args.lane else` block) \
         does not call every `ci_fast_<lane>()` lane function. The serial dispatch must \
         keep running ALL lanes (check, lint, test, contracts, coverage) so a bare \
         `cargo xtask ci-fast` covers the same gate set as the CI fan-out. Restore the \
         missing lane call in tools/xtask/src/commands/ci.rs.",
    )?;
    Ok(())
}

/// Assert that `.github/workflows/ci.yml` fans the fast bundle out into one
/// always-on job per [`CI_FAST_LANES`] lane and fans them back into the
/// `ci-fast-linux` summary job. Each lane job must run
/// `cargo xtask ci-fast --lane <lane>` through the devcontainer wrapper, be
/// gated on the rust-changed detector ONLY (never a label — the meta-gate
/// watches for label-gated reburial), and appear in the summary's `needs` so a
/// failed lane fails the single required check.
fn assert_ci_fast_fanout_jobs(ci_yml: &str) -> Result<()> {
    for lane in CI_FAST_LANES {
        let job_key = format!("ci-fast-{lane}");
        let block = workflow_job_block(ci_yml, &job_key).with_context(|| {
            format!(
                "ci-parity: `.github/workflows/ci.yml` must declare an always-on `{job_key}:` \
                 fan-out job (one per ci-fast lane); see CI_FAST_LANES in \
                 tools/integrity/src/ci_parity.rs"
            )
        })?;
        let run_line = format!(
            "run: bash ./scripts/run-in-devcontainer.sh 'cargo xtask ci-fast --lane {lane}'"
        );
        ensure(
            block.contains(&run_line),
            format!(
                "ci-parity: ci.yml job `{job_key}` must run `{run_line}` (the lane must go \
                 through the checked-in devcontainer wrapper)"
            ),
        )?;
        ensure(
            block.contains("if: needs.rust-changed.outputs.rust == 'true'"),
            format!(
                "ci-parity: ci.yml job `{job_key}` must be gated on \
                 `needs.rust-changed.outputs.rust == 'true'` so it runs on every Rust-touching PR"
            ),
        )?;
        // Reject the label-gating IDIOM (`github.event.pull_request.labels`),
        // not the bare word "labels" — a comment or action input mentioning
        // labels must not red a lane that is still rust-changed-gated only.
        ensure(
            !block.contains("github.event.pull_request.labels"),
            format!(
                "ci-parity: ci.yml job `{job_key}` must never be label-gated (no \
                 `github.event.pull_request.labels` condition) — the ci-fast lanes are the \
                 DEFAULT PR path (P1-1)"
            ),
        )?;
    }
    let summary = workflow_job_block(ci_yml, "ci-fast-linux").context(
        "ci-parity: `.github/workflows/ci.yml` must keep the `ci-fast-linux:` fan-in summary \
         job (the single required check downstream `needs:` edges point at)",
    )?;
    for lane in CI_FAST_LANES {
        ensure(
            summary.contains(&format!("ci-fast-{lane}")),
            format!(
                "ci-parity: the `ci-fast-linux` summary job must `needs:` the `ci-fast-{lane}` \
                 lane job so a failed lane fails the required check"
            ),
        )?;
    }
    ensure(
        summary.contains("if: always()"),
        "ci-parity: the `ci-fast-linux` summary job must run with `if: always()` so it \
         reports a verdict even when a lane fails or is skipped",
    )?;
    Ok(())
}

/// Slice the block of one top-level ci.yml job: the lines after `  <job_key>:`
/// up to (excluding) the next two-space-indented `key:` line. String-level like
/// the rest of this detector — no YAML parser — so failure messages stay
/// legible and the dependency surface stays minimal.
pub(crate) fn workflow_job_block(workflow: &str, job_key: &str) -> Result<String> {
    let header = format!("  {job_key}:");
    let next_job_re =
        Regex::new(r"^  [a-z][a-z0-9-]*:\s*$").context("compile workflow job-boundary regex")?;
    let mut block = String::new();
    let mut collecting = false;
    for line in workflow.lines() {
        if collecting {
            if next_job_re.is_match(line) {
                break;
            }
            block.push_str(line);
            block.push('\n');
        } else if line.trim_end() == header {
            collecting = true;
        }
    }
    if !collecting {
        bail!("ci-parity: could not find job `{job_key}:` in the workflow");
    }
    Ok(block)
}

fn xtask_source_text(repo_root: &Path) -> Result<String> {
    let mut combined = String::new();
    for path in files_with_extension(&repo_root.join("tools/xtask/src"), "rs") {
        combined.push_str(
            &fs::read_to_string(&path)
                .with_context(|| format!("read {}", relative(repo_root, &path)))?,
        );
        combined.push('\n');
    }
    Ok(combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // justifies: INV-TEST-PANIC-AS-ASSERTION; setup panics signal fixture breakage, see tools/integrity/src/main.rs
    fn temp_project(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "batpak-ci-parity-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    /// Minimal `cargo xtask` source surface that satisfies every parity rule
    /// reachable from a synthetic green fixture: it declares the handful of
    /// `XtaskCommand` variants the green `ci.yml` references and pins each tool
    /// to the same version used by the green workflow + Dockerfile.
    const GREEN_XTASK_MAIN: &str = r#"
enum XtaskCommand {
    CiFast(CiFastArgs),
    Preflight,
    Mutants,
    Setup,
    Release(ReleaseArgs),
}

pub(crate) fn ci_fast(args: CiFastArgs) -> Result<()> {
    let Some(lane) = args.lane else {
        ci_fast_check()?;
        ci_fast_lint()?;
        ci_fast_test()?;
        ci_fast_contracts()?;
        return ci_fast_coverage();
    };
    match lane {
        CiFastLane::Check => ci_fast_check(),
        CiFastLane::Lint => ci_fast_lint(),
        CiFastLane::Test => ci_fast_test(),
        CiFastLane::Contracts => ci_fast_contracts(),
        CiFastLane::Coverage => ci_fast_coverage(),
    }
}

fn ci_fast_check() -> Result<()> {
    cargo(["fmt", "--check"])
}

fn ci_fast_lint() -> Result<()> {
    run_workspace_clippy()
}

fn ci_fast_test() -> Result<()> {
    run_nextest_ci(["--workspace", "--all-features"])
}

fn ci_fast_contracts() -> Result<()> {
    templates()?;
    crate::public_api::public_api(PublicApiArgs { strict: true, check_baseline: true, bless_baseline: false })?;
    super::package_leak_scan(PackageLeakScanArgs { allow_dirty: false, strict_language: true })?;
    integrity("doctor", ["--strict"])?;
    integrity("gauntlet-receipts-present", [])
}

fn ci_fast_coverage() -> Result<()> {
    coverage::cover(CoverArgs { ci: true, json: false, threshold: Some(80) })
}

fn install_tools() {
    let tools = [
        ("cargo-nextest", "cargo-nextest@0.9.132"),
        ("cargo-mutants", "cargo-mutants@27.0.0"),
    ];
}
"#;

    /// Green workflow body: every `cargo xtask <cmd>` maps to a variant in
    /// [`GREEN_XTASK_MAIN`]; the only pinned tool (`cargo-nextest@0.9.132`)
    /// matches both the xtask source and the Dockerfile; every workflow-owned
    /// list value matches the hardcoded expected set in [`check`].
    fn green_ci_yml() -> String {
        let mut yml = String::new();
        yml.push_str("name: ci\n");
        yml.push_str("jobs:\n");
        for lane in super::CI_FAST_LANES {
            yml.push_str(&format!("  ci-fast-{lane}:\n"));
            yml.push_str("    if: needs.rust-changed.outputs.rust == 'true'\n");
            yml.push_str("    steps:\n");
            yml.push_str(&format!(
                "      - run: bash ./scripts/run-in-devcontainer.sh 'cargo xtask ci-fast --lane {lane}'\n"
            ));
        }
        yml.push_str("  ci-fast-linux:\n");
        yml.push_str("    needs:\n");
        yml.push_str("      [\n");
        yml.push_str("        rust-changed,\n");
        for lane in super::CI_FAST_LANES {
            yml.push_str(&format!("        ci-fast-{lane},\n"));
        }
        yml.push_str("      ]\n");
        yml.push_str("    if: always()\n");
        yml.push_str("    steps:\n");
        yml.push_str("      - run: echo summary\n");
        yml.push_str("  fast:\n");
        yml.push_str("    steps:\n");
        yml.push_str("      - run: cargo xtask preflight\n");
        yml.push_str("      - uses: taiki-e/install-action@v2\n");
        yml.push_str("        with:\n");
        yml.push_str("          tool: cargo-nextest@0.9.132\n");
        yml.push_str("  matrix-surface:\n");
        yml.push_str("    strategy:\n");
        yml.push_str("      matrix:\n");
        yml.push_str("        surface: [all-features, no-default-features]\n");
        yml.push_str(
            "        shard: [\"0/12\", \"1/12\", \"2/12\", \"3/12\", \"4/12\", \"5/12\", \"6/12\", \"7/12\", \"8/12\", \"9/12\", \"10/12\", \"11/12\"]\n",
        );
        yml.push_str("        seam:\n");
        for seam in GREEN_SEAMS {
            yml.push_str(&format!("          - {seam}\n"));
        }
        yml.push_str("        features:\n");
        yml.push_str("          - \"\"\n");
        yml.push_str("          - \"--features dangerous-test-hooks\"\n");
        yml.push_str("          - \"--no-default-features\"\n");
        yml.push_str("          - \"--no-default-features --features dangerous-test-hooks\"\n");
        yml.push_str("          - \"--all-features\"\n");
        yml
    }

    // Must stay in EXACT lockstep with the canonical seam list `check` asserts
    // above (the `seam` entry in the ci.yml `assert_workflow_list_values` call):
    // the green fixture runs the full `check`, so any seam present there but
    // absent here makes the green sanity-floor fixture fail.
    const GREEN_SEAMS: &[&str] = &[
        "writer-commit",
        "cursor-delivery",
        "projection-flow",
        "projection-fusion",
        "segment-scan",
        "hash-chain-replay",
        "frontier-wait-durable",
        "frontier-append-gate",
        "event-payload-registry-validator",
        "platform-backend",
        "testing-ledger-structural-lint",
        "integrity-graders",
        "syncbat-runtime-dispatch",
        "syncbat-register-catalog",
        "syncbat-subscription-runtime",
        "netbat-boundary-protocol",
        "fork-isolation",
        "import-reapply",
        "lane-branch",
        "lane-frontier",
        "bvisor-admission",
        "bvisor-report-seal",
    ];

    const GREEN_PERF_YML: &str = "name: perf\njobs:\n  bench:\n    strategy:\n      matrix:\n        surface: [neutral, native]\n";

    const GREEN_RELEASE_YML: &str = "name: release\njobs:\n  ship:\n    steps:\n      - run: bash ./scripts/run-in-devcontainer.sh 'cargo xtask release --dry-run'\n";

    const GREEN_DOCKERFILE: &str = "FROM rust\nRUN cargo install --locked cargo-nextest@0.9.132\n";

    /// Write a complete green project tree under `<tmp>/bpk-lib` (the
    /// `repo_root` passed to [`check`]) and return that `repo_root`.
    fn write_green_project(name: &str, ci_yml: &str) -> PathBuf {
        let project = temp_project(name);
        let repo_root = project.join("bpk-lib");
        fs::create_dir_all(project.join(".github/workflows")).expect("workflows dir");
        fs::create_dir_all(project.join(".devcontainer")).expect("devcontainer dir");
        fs::create_dir_all(repo_root.join("tools/xtask/src")).expect("xtask src dir");
        fs::write(project.join(".github/workflows/ci.yml"), ci_yml).expect("ci.yml");
        fs::write(project.join(".github/workflows/perf.yml"), GREEN_PERF_YML).expect("perf.yml");
        fs::write(
            project.join(".github/workflows/release.yml"),
            GREEN_RELEASE_YML,
        )
        .expect("release.yml");
        fs::write(project.join(".devcontainer/Dockerfile"), GREEN_DOCKERFILE).expect("Dockerfile");
        fs::write(repo_root.join("tools/xtask/src/main.rs"), GREEN_XTASK_MAIN).expect("xtask main");
        repo_root
    }

    fn cleanup(repo_root: &Path) {
        if let Some(project) = repo_root.parent() {
            let _ = fs::remove_dir_all(project);
        }
    }

    #[test]
    fn ci_parity_green_fixture_passes() {
        // Sanity floor for every planted-violation test below: the unmodified
        // synthetic tree must pass, otherwise a later Err could be spurious.
        let repo_root = write_green_project("green", &green_ci_yml());
        check(&repo_root).expect("synthetic green fixture must pass ci-parity");
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_rejects_unknown_xtask_command() {
        // Green passes; planting `cargo xtask doesnotexist` (no matching
        // XtaskCommand variant) must make rule 1 bail.
        let mut yml = green_ci_yml();
        yml.push_str("      - run: cargo xtask doesnotexist\n");
        let repo_root = write_green_project("unknown-xtask", &yml);
        let err = check(&repo_root).expect_err("unknown xtask subcommand must fail");
        assert!(
            err.to_string().contains("Doesnotexist") || err.to_string().contains("doesnotexist"),
            "unexpected error: {err:#}"
        );
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_rejects_unpinned_tool() {
        // Green passes; an unpinned `tool: cargo-nextest` line (no @version)
        // must trip the bare-tool rejection up front.
        let yml = green_ci_yml().replace("cargo-nextest@0.9.132", "cargo-nextest");
        let repo_root = write_green_project("unpinned", &yml);
        let err = check(&repo_root).expect_err("unpinned tool must fail");
        assert!(
            err.to_string().contains("without a version pin"),
            "unexpected error: {err:#}"
        );
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_rejects_dockerfile_version_mismatch() {
        // Green passes; bumping ONLY the Dockerfile pin (so workflow says
        // 0.9.132 but Dockerfile says 0.9.999) must trip the Dockerfile parity
        // rule.
        let repo_root = write_green_project("docker-mismatch", &green_ci_yml());
        fs::write(
            repo_root
                .parent()
                .expect("project root")
                .join(".devcontainer/Dockerfile"),
            "FROM rust\nRUN cargo install --locked cargo-nextest@0.9.999\n",
        )
        .expect("rewrite Dockerfile");
        let err = check(&repo_root).expect_err("dockerfile version mismatch must fail");
        assert!(
            err.to_string().contains("Dockerfile") && err.to_string().contains("cargo-nextest"),
            "unexpected error: {err:#}"
        );
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_rejects_xtask_version_mismatch() {
        // Green passes; pinning the xtask source to a different version than
        // the workflow must trip the xtask-vs-workflow parity rule.
        let xtask = GREEN_XTASK_MAIN.replace("cargo-nextest@0.9.132", "cargo-nextest@0.8.0");
        let repo_root = write_green_project("xtask-mismatch", &green_ci_yml());
        fs::write(repo_root.join("tools/xtask/src/main.rs"), xtask).expect("rewrite xtask");
        let err = check(&repo_root).expect_err("xtask version mismatch must fail");
        assert!(
            err.to_string().contains("cargo xtask setup")
                && err.to_string().contains("cargo-nextest"),
            "unexpected error: {err:#}"
        );
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_rejects_seam_matrix_missing_registry_seam() {
        // Green passes; dropping `platform-backend` from the ci.yml `seam:`
        // matrix must make the seam lockstep rule bail because the registry
        // (the hardcoded expected set) still declares it.
        let yml = green_ci_yml().replace("          - platform-backend\n", "");
        let repo_root = write_green_project("seam-missing", &yml);
        let err = check(&repo_root).expect_err("missing seam must fail");
        assert!(
            err.to_string().contains("seam"),
            "unexpected error: {err:#}"
        );
        cleanup(&repo_root);
    }

    #[test]
    fn ci_parity_assert_workflow_list_values_detects_missing_value() {
        // Drive the factored list-lockstep helper directly: green set matches,
        // a workflow missing one expected value Errs.
        let green = "matrix:\n  surface: [neutral, native]\n";
        assert_workflow_list_values("perf.yml", green, "surface", &["neutral", "native"])
            .expect("matching list passes");
        let planted = "matrix:\n  surface: [neutral]\n";
        let err =
            assert_workflow_list_values("perf.yml", planted, "surface", &["neutral", "native"])
                .expect_err("missing value must fail");
        assert!(err.to_string().contains("surface"), "unexpected: {err:#}");
    }

    /// A synthetic `ci_fast` lane family that invokes every required L2+ gate
    /// and dispatches every lane, used as the green floor for the anti-rebury
    /// self-tests below.
    const GREEN_CI_FAST_SOURCE: &str = r#"
pub(crate) fn ci_fast(args: CiFastArgs) -> Result<()> {
    let Some(lane) = args.lane else {
        ci_fast_check()?;
        ci_fast_lint()?;
        ci_fast_test()?;
        ci_fast_contracts()?;
        return ci_fast_coverage();
    };
    match lane {
        CiFastLane::Check => ci_fast_check(),
        CiFastLane::Lint => ci_fast_lint(),
        CiFastLane::Test => ci_fast_test(),
        CiFastLane::Contracts => ci_fast_contracts(),
        CiFastLane::Coverage => ci_fast_coverage(),
    }
}

fn ci_fast_check() -> Result<()> {
    cargo(["fmt", "--check"])
}

fn ci_fast_lint() -> Result<()> {
    run_workspace_clippy()
}

fn ci_fast_test() -> Result<()> {
    run_nextest_ci(["--workspace", "--all-features"])
}

fn ci_fast_contracts() -> Result<()> {
    templates()?;
    crate::public_api::public_api(PublicApiArgs { strict: true, check_baseline: true, bless_baseline: false })?;
    super::package_leak_scan(PackageLeakScanArgs { allow_dirty: false, strict_language: true })?;
    integrity("doctor", ["--strict"])?;
    integrity("gauntlet-receipts-present", [])
}

fn ci_fast_coverage() -> Result<()> {
    coverage::cover(CoverArgs { ci: true, json: false, threshold: Some(80) })
}
"#;

    #[test]
    fn ci_parity_rejects_ci_fast_missing_coverage_gate() {
        // Anti-rebury (P1-1): the green ci_fast family containing every gate
        // passes; removing the coverage call (re-burying the coverage floor)
        // must make the anti-rebury assertion bail. Anti-vacuous: both the
        // green AND the planted-violation case are asserted.
        assert_ci_fast_keeps_default_path_gates(GREEN_CI_FAST_SOURCE)
            .expect("green ci_fast family with all gates must pass anti-rebury");

        let reburied = GREEN_CI_FAST_SOURCE.replace(
            "    coverage::cover(CoverArgs { ci: true, json: false, threshold: Some(80) })\n",
            "    Ok(())\n",
        );
        let err = assert_ci_fast_keeps_default_path_gates(&reburied)
            .expect_err("ci_fast family missing the coverage gate must fail anti-rebury");
        assert!(
            err.to_string().contains("coverage floor"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_rejects_ci_fast_missing_template_gate() {
        // Template compilation must stay in the every-PR fast lane, not only in
        // the heavier consumer-smoke lane.
        let reburied = GREEN_CI_FAST_SOURCE.replace("    templates()?;\n", "");
        let err = assert_ci_fast_keeps_default_path_gates(&reburied)
            .expect_err("ci_fast family missing the template gate must fail anti-rebury");
        assert!(
            err.to_string().contains("template compile"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_rejects_ci_fast_missing_public_api_gate() {
        // Companion to the coverage case: dropping the public-api baseline call
        // from the contracts lane must also trip the anti-rebury assertion.
        let reburied = GREEN_CI_FAST_SOURCE.replace(
            "    crate::public_api::public_api(PublicApiArgs { strict: true, check_baseline: true, bless_baseline: false })?;\n",
            "",
        );
        let err = assert_ci_fast_keeps_default_path_gates(&reburied)
            .expect_err("ci_fast family missing the public-api gate must fail anti-rebury");
        assert!(
            err.to_string().contains("public-api baseline"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_anti_rebury_reads_only_the_ci_fast_family() {
        // A gate present elsewhere (e.g. still called inside `ci()`) must NOT
        // satisfy the assertion: the scope is the ci_fast lane family only.
        // Strip the coverage call from its lane but leave a decoy call in a
        // non-family function; the assertion must still bail.
        let reburied = GREEN_CI_FAST_SOURCE.replace(
            "    coverage::cover(CoverArgs { ci: true, json: false, threshold: Some(80) })\n",
            "    Ok(())\n",
        );
        let with_decoy = format!(
            "{reburied}\npub(crate) fn ci() -> Result<()> {{\n    coverage::cover(CoverArgs {{ ci: true, json: false, threshold: Some(80) }})?;\n    Ok(())\n}}\n"
        );
        let err = assert_ci_fast_keeps_default_path_gates(&with_decoy)
            .expect_err("coverage call outside the ci_fast family must not satisfy anti-rebury");
        assert!(
            err.to_string().contains("coverage floor"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_rejects_serial_dispatch_dropping_a_lane() {
        // The no-lane serial path of `ci_fast()` must keep calling every lane
        // so the local one-command bundle covers the same gates as the CI
        // fan-out. Dropping the contracts call from the `args.lane else` block
        // ONLY — the single-lane `match` arm still spells the call — must bail:
        // the check scopes to the serial block, so the match arms cannot
        // satisfy it (the hole a whole-body scan would miss).
        let dropped = GREEN_CI_FAST_SOURCE.replace("        ci_fast_contracts()?;\n", "");
        assert!(
            dropped.contains("CiFastLane::Contracts => ci_fast_contracts(),"),
            "fixture must keep the match arm so the test proves serial-block scoping"
        );
        let err = assert_ci_fast_keeps_default_path_gates(&dropped)
            .expect_err("serial dispatch missing a lane call must fail");
        assert!(
            err.to_string().contains("ci_fast_<lane>()"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_fanout_rejects_missing_lane_job() {
        // Green fan-out passes; deleting one lane job from ci.yml must bail.
        let green = green_ci_yml();
        assert_ci_fast_fanout_jobs(&green).expect("green fan-out must pass");
        let planted = green.replace("  ci-fast-coverage:\n", "  ci-fast-coverage-renamed:\n");
        let err = assert_ci_fast_fanout_jobs(&planted)
            .expect_err("missing lane job must fail fan-out parity");
        assert!(
            err.to_string().contains("ci-fast-coverage"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_fanout_rejects_label_gated_lane_job() {
        // A lane job gaining a label-gate condition is a reburial of the
        // default PR path and must bail.
        let planted = green_ci_yml().replace(
            "  ci-fast-test:\n    if: needs.rust-changed.outputs.rust == 'true'\n",
            "  ci-fast-test:\n    if: contains(github.event.pull_request.labels.*.name, 'run-heavy-ci')\n",
        );
        let err = assert_ci_fast_fanout_jobs(&planted)
            .expect_err("label-gated lane job must fail fan-out parity");
        assert!(
            err.to_string().contains("ci-fast-test"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_fanout_accepts_harmless_labels_mention_in_lane_comment() {
        // The label-gate rejection matches the gating idiom, not the bare word:
        // a lane comment that merely says "labels" must not red the fan-out
        // parity check while the job stays rust-changed-gated only.
        let commented = green_ci_yml().replace(
            "  ci-fast-test:\n    if: needs.rust-changed.outputs.rust == 'true'\n",
            "  ci-fast-test:\n    # never gate this lane on PR labels\n    if: needs.rust-changed.outputs.rust == 'true'\n",
        );
        assert_ci_fast_fanout_jobs(&commented)
            .expect("a comment mentioning labels must not trip the label-gate rejection");
    }

    #[test]
    fn ci_parity_fanout_rejects_summary_missing_a_lane_need() {
        // The summary must fan in every lane; dropping one from its needs list
        // must bail.
        let planted = green_ci_yml().replace("        ci-fast-lint,\n", "");
        let err = assert_ci_fast_fanout_jobs(&planted)
            .expect_err("summary missing a lane need must fail fan-out parity");
        assert!(
            err.to_string().contains("ci-fast-lint"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn ci_parity_rejects_unregistered_just_recipe() {
        // Drive the just-recipe lockstep helper directly: a registered recipe
        // whose mapped xtask command exists passes; an unregistered recipe
        // Errs.
        let xtask_main = "enum XtaskCommand {\n    CiFast,\n}\n";
        let green = [("ci.yml", "run: just ci-fast\n")];
        assert_workflow_just_recipes_map_to_xtask(&green, xtask_main)
            .expect("registered recipe passes");
        let planted = [("ci.yml", "run: just totally-made-up\n")];
        let err = assert_workflow_just_recipes_map_to_xtask(&planted, xtask_main)
            .expect_err("unregistered recipe must fail");
        assert!(
            err.to_string().contains("totally-made-up"),
            "unexpected: {err:#}"
        );
    }
}

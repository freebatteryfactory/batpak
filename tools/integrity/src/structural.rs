use crate::common::{
    ensure, files_with_extension, load_yaml, relative, repo_root, rust_files, tracked_repo_files,
};
use crate::shared_checks::{
    ast_references_name, collect_dead_code_silencer_sites, line_carries_justification,
    load_dead_code_silencer_allowlist, load_known_invariants, public_item_names,
};
use crate::{architecture_lints, harness_lints};
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use syn::Item;

#[derive(Debug, Deserialize)]
struct AllowlistEntry {
    name: String,
    justification: String,
    witness: Vec<AllowlistWitness>,
}

#[derive(Debug, Deserialize)]
struct AllowlistWitness {
    path: String,
    // justifies: INV-TRACEABILITY-COMPLETE; lines is supplementary line-number metadata for human review; the AST walker in tools/integrity/src/structural.rs verifies the path contains the item regardless of specific lines
    #[serde(default)]
    lines: Vec<u32>,
}

impl AllowlistWitness {
    fn line_hints(&self) -> &[u32] {
        &self.lines
    }
}

pub(crate) fn run() -> Result<()> {
    let repo_root = repo_root()?;
    let tracked_files = tracked_repo_files(&repo_root)?;
    architecture_lints::check(&repo_root, &tracked_files)?;
    harness_lints::check(&repo_root, &tracked_files)?;
    check_no_dead_code_silencers(&repo_root)?;
    check_allow_justifications(&repo_root)?;
    check_pub_items_have_references(&repo_root)?;
    check_ci_parity(&repo_root)?;
    check_store_pub_fn_coverage(&repo_root)?;
    println!("structural-check: ok");
    Ok(())
}

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
/// 4. Workflow-owned matrix values for perf surfaces and scheduled mutation
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

fn check_ci_parity(repo_root: &Path) -> Result<()> {
    let ci_yml = fs::read_to_string(repo_root.join(".github/workflows/ci.yml"))
        .context("read .github/workflows/ci.yml")?;
    let perf_yml = fs::read_to_string(repo_root.join(".github/workflows/perf.yml"))
        .context("read .github/workflows/perf.yml")?;
    let release_yml = fs::read_to_string(repo_root.join(".github/workflows/release.yml"))
        .context("read .github/workflows/release.yml")?;
    let xtask_main = fs::read_to_string(repo_root.join("tools/xtask/src/main.rs"))
        .context("read tools/xtask/src/main.rs")?;
    let xtask_sources = xtask_source_text(repo_root)?;
    let dockerfile = fs::read_to_string(repo_root.join(".devcontainer/Dockerfile"))
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
            "1/12", "2/12", "3/12", "4/12", "5/12", "6/12", "7/12", "8/12", "9/12", "10/12",
            "11/12", "12/12",
        ],
    )?;
    assert_workflow_list_values(
        ".github/workflows/ci.yml",
        &ci_yml,
        "features",
        &[
            "",
            "--features blake3",
            "--features dangerous-test-hooks",
            "--no-default-features",
            "--no-default-features --features blake3",
            "--no-default-features --features dangerous-test-hooks",
            "--all-features",
        ],
    )?;
    ensure(
        release_yml.contains("bash ./scripts/run-in-devcontainer.sh 'cargo xtask release --dry-run'"),
        "ci-parity: `.github/workflows/release.yml` must run `cargo xtask release --dry-run` through `scripts/run-in-devcontainer.sh`.",
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

/// Assert that every `pub fn` declared in inherent `impl Store { ... }` blocks
/// under `src/store/` has at least one reference in the test or source tree.
///
/// This is a structural guard: if a method is added to `Store` and no test
/// exercises it, a developer (or agent) has likely forgotten to write the test.
/// The check uses `syn` to parse the AST — no regex heuristics for pub fn
/// detection — so methods inside `#[cfg(...)]` blocks or across multiple `impl`
/// blocks are handled correctly.
///
/// Reference detection uses regex against the combined text of `tests/` and
/// `src/` (which covers both standalone test files and `#[cfg(test)] mod tests`
/// inline in source files).
fn check_store_pub_fn_coverage(repo_root: &Path) -> Result<()> {
    // Methods that are deliberately exercised only indirectly or are
    // intentionally infrastructure-only. Start empty and add only proven
    // false positives with a justification comment.
    let allowlist: &[&str] = &[
        // `subscription` is doc(hidden) glue for async integration, exercised
        // indirectly via `subscribe` in every subscription test.
        "subscription",
    ];

    // 1. Parse every store source file with syn and walk all inherent
    // `impl Store` blocks. Store's public surface is intentionally split by
    // owner modules; the detector follows that architecture instead of
    // hardcoding `src/store/mod.rs`.
    let mut pub_fns: BTreeSet<String> = BTreeSet::new();
    for store_source_path in rust_files(&repo_root.join("src/store")) {
        let source = fs::read_to_string(&store_source_path)
            .with_context(|| format!("read {}", store_source_path.display()))?;
        let ast = syn::parse_file(&source)
            .with_context(|| format!("syn parse {}", store_source_path.display()))?;

        for item in &ast.items {
            if let Item::Impl(impl_block) = item {
                // Match `impl Store`, `impl Store<Open>`, and `impl<T> Store<T>`.
                // We only care about blocks whose self type path segment is
                // exactly `Store`.
                let is_store_impl = match impl_block.self_ty.as_ref() {
                    syn::Type::Path(tp) => tp
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident == "Store")
                        .unwrap_or(false),
                    _ => false,
                };
                // Trait impls (e.g., `impl Drop for Store`) are excluded — we
                // only want inherent impls.
                if !is_store_impl || impl_block.trait_.is_some() {
                    continue;
                }
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if matches!(method.vis, syn::Visibility::Public(_)) {
                            let name = method.sig.ident.to_string();
                            // Skip names starting with `_` (private convention).
                            if !name.starts_with('_') {
                                pub_fns.insert(name);
                            }
                        }
                    }
                }
            }
        }
    }

    if pub_fns.is_empty() {
        bail!(
            "structural-check: Store pub fn coverage — could not find any `impl Store` \
             block under src/store/. The Store surface may have moved outside the declared owner."
        );
    }

    // 2. Build the reference corpus: all .rs files under tests/ and src/.
    let mut search_files: Vec<PathBuf> = rust_files(&repo_root.join("tests"));
    search_files.extend(rust_files(&repo_root.join("src")));

    // 3. For each pub fn, check that at least one file references it as a call.
    //    Patterns matched: `.name(`, `Store::name(`, `store.name(`
    let mut unreferenced: Vec<String> = Vec::new();
    for name in &pub_fns {
        if allowlist.contains(&name.as_str()) {
            continue;
        }
        // Build patterns that strongly indicate a method call or direct use.
        // We accept any of:
        //   `.name(`        — method call syntax
        //   `.name::<`      — method call with turbofish (e.g., `.watch_projection::<T>(...)`)
        //   `Store::name(`  — fully-qualified call
        //   `Store::name::<` — fully-qualified call with turbofish
        //   `store.name(`   — conventional variable name
        //   `store.name::<` — conventional variable name with turbofish
        // The turbofish variants are critical: we miss generic method calls
        // without them. Caught by the watch_projection false-positive when
        // this check first ran against the real codebase.
        let patterns = [
            format!(".{}(", name),
            format!(".{}::<", name),
            format!("Store::{}(", name),
            format!("Store::{}::<", name),
            format!("store.{}(", name),
            format!("store.{}::<", name),
        ];
        let mut found = false;
        'files: for path in &search_files {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for pat in &patterns {
                if content.contains(pat.as_str()) {
                    found = true;
                    break 'files;
                }
            }
        }
        if !found {
            unreferenced.push(name.clone());
        }
    }

    if !unreferenced.is_empty() {
        let list = unreferenced
            .iter()
            .map(|n| format!("  - {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "structural-check: Store pub fn coverage failure — the following methods on\n\
             `impl Store` have ZERO test or source references and are likely orphaned:\n\
             {list}\n\
             Investigate: src/store/ and add a test exercising each, or remove the\n\
             method if it's truly unused."
        );
    }

    Ok(())
}

fn check_no_dead_code_silencers(repo_root: &Path) -> Result<()> {
    let allowlisted = load_dead_code_silencer_allowlist(repo_root).map_err(|err| anyhow!(err))?;
    let mut paths = rust_files(&repo_root.join("src"));
    paths.extend(rust_files(&repo_root.join("tools/xtask/src")));
    paths.extend(rust_files(&repo_root.join("tools/integrity/src")));
    paths.extend(rust_files(&repo_root.join("crates/macros/src")));
    paths.extend(rust_files(&repo_root.join("crates/macros-support/src")));
    paths.extend(rust_files(&repo_root.join("tests")));
    paths.extend(rust_files(&repo_root.join("examples")));
    paths.extend(rust_files(&repo_root.join("benches")));
    paths.push(repo_root.join("build.rs"));
    for path in paths {
        let content = fs::read_to_string(&path)?;
        let sites = collect_dead_code_silencer_sites(&content)
            .map_err(|err| anyhow!("parse {}: {}", relative(repo_root, &path), err))?;
        for site in sites {
            let allowlist_site = format!("{}:{}", relative(repo_root, &path), site.line);
            if allowlisted.contains(&allowlist_site) {
                continue;
            }
            bail!(
                "dead_code silencers are not tolerated in {}:{}:{}.\n\
                 Found `{}`.\n\
                 If code is test-only, use #[cfg(test)]. If it is unused, delete it.\n\
                 If it is shared infrastructure, restructure it so the compiler sees the real ownership surface.\n\
                 If this is the rare legitimate exception, add `{}` to traceability/dead_code_silencer_allowlist.yaml with `reason` and `adr`.",
                relative(repo_root, &path),
                site.line,
                site.column,
                site.rendered,
                allowlist_site,
            );
        }
    }
    Ok(())
}

fn check_allow_justifications(repo_root: &Path) -> Result<()> {
    let known_invariants = load_known_invariants(repo_root).map_err(|err| anyhow!(err))?;
    let mut paths = rust_files(&repo_root.join("src"));
    paths.extend(rust_files(&repo_root.join("tools/xtask/src")));
    paths.extend(rust_files(&repo_root.join("tools/integrity/src")));
    paths.extend(rust_files(&repo_root.join("crates/macros/src")));
    paths.extend(rust_files(&repo_root.join("crates/macros-support/src")));
    paths.extend(rust_files(&repo_root.join("tests")));
    paths.extend(rust_files(&repo_root.join("examples")));
    paths.extend(rust_files(&repo_root.join("benches")));
    paths.push(repo_root.join("build.rs"));
    for path in paths {
        let content = fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("#![allow(") || trimmed.starts_with("#[allow(") {
                let justified = line_carries_justification(line, repo_root, &known_invariants)
                    || index
                        .checked_sub(1)
                        .and_then(|prev| lines.get(prev))
                        .map(|prev| line_carries_justification(prev, repo_root, &known_invariants))
                        .unwrap_or(false);
                ensure(
                    justified,
                    format!(
                        "unjustified allow in {}:{} — every #[allow(...)] must carry a `// justifies: <>=5 words + >=1 resolvable anchor>` comment. \
                         An anchor is an INV-id from traceability/invariants.yaml, an ADR-NNNN whose file exists under docs/adr/, \
                         or a concrete repo path (src/..., tests/..., examples/..., crates/macros/..., crates/macros-support/..., benches/..., tools/..., build.rs). \
                         See INV-ALLOW-IS-DESIGN.",
                        relative(repo_root, &path),
                        index + 1
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn check_pub_items_have_references(repo_root: &Path) -> Result<()> {
    let allowlist: Vec<AllowlistEntry> =
        load_yaml(&repo_root.join("traceability/pub_item_allowlist.yaml"))?;
    let allowed: HashMap<&str, &AllowlistEntry> = allowlist
        .iter()
        .map(|entry| (entry.name.as_str(), entry))
        .collect();

    // For every allowlist entry, validate every witness path:
    //   - file must exist
    //   - file must parse as Rust
    //   - file must contain a real AST reference to the item name (not just a
    //     substring in a string literal or comment)
    for entry in &allowlist {
        ensure(
            !entry.justification.trim().is_empty(),
            format!(
                "pub_item_allowlist entry `{}` must include a non-empty supplementary `justification:`",
                entry.name
            ),
        )?;
        ensure(
            !entry.witness.is_empty(),
            format!(
                "pub_item_allowlist entry `{}` must declare at least one `witness:` path pointing at a test that uses the item; narrative `justification:` is supplementary, not load-bearing",
                entry.name
            ),
        )?;
        for witness in &entry.witness {
            ensure(
                witness.path.starts_with("tests/"),
                format!(
                    "pub_item_allowlist entry `{}` witness `{}` must point at a file under tests/, not production code",
                    entry.name, witness.path
                ),
            )?;
            ensure(
                !witness.line_hints().is_empty(),
                format!(
                    "pub_item_allowlist entry `{}` witness `{}` must include at least one concrete line hint",
                    entry.name, witness.path
                ),
            )?;
            let abs = repo_root.join(&witness.path);
            ensure(
                abs.exists(),
                format!(
                    "pub_item_allowlist entry `{}` declares witness path `{}` but that file does not exist",
                    entry.name, witness.path
                ),
            )?;
            let content = fs::read_to_string(&abs)
                .with_context(|| format!("read witness {}", witness.path))?;
            let file = syn::parse_file(&content)
                .with_context(|| format!("parse witness {}", witness.path))?;
            ensure(
                ast_references_name(&file, &entry.name),
                format!(
                    "pub_item_allowlist entry `{}` witness `{}` (line hints {:?}) does not contain a real path-position reference to `{}`; either update the witness path or hide the item via `#[doc(hidden)]`",
                    entry.name,
                    witness.path,
                    witness.line_hints(),
                    entry.name,
                ),
            )?;
        }
    }

    let test_files: Vec<PathBuf> = rust_files(&repo_root.join("tests"));
    let mut parsed_tests: Vec<(PathBuf, syn::File)> = Vec::with_capacity(test_files.len());
    for path in test_files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("read {}", relative(repo_root, &path)))?;
        let file = syn::parse_file(&content)
            .with_context(|| format!("parse {}", relative(repo_root, &path)))?;
        parsed_tests.push((path, file));
    }

    for path in rust_files(&repo_root.join("src")) {
        if path.ends_with("prelude.rs") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        let file = syn::parse_file(&content)
            .with_context(|| format!("parse {}", relative(repo_root, &path)))?;
        for name in public_item_names(&file) {
            if allowed.contains_key(name.as_str()) {
                continue;
            }
            let found = parsed_tests
                .iter()
                .any(|(_, ast)| ast_references_name(ast, &name));
            ensure(
                found,
                format!(
                    "pub item `{}` declared at {} has no test reference (checked {} test files via AST); either add a real test use, add an allowlist entry with a `witness:` path that points to an actual use, or hide the item via `#[doc(hidden)]`.",
                    name,
                    relative(repo_root, &path),
                    parsed_tests.len(),
                ),
            )?;
        }
    }
    Ok(())
}

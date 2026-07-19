use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use crate::tokens::sanitize_rust;

/// Module names that are generic drawers, not semantic owners. Admitting one
/// requires an explicit ruling; none exists today, so the list is total.
const FORBIDDEN_MODULE_NAMES: [&str; 8] =
    ["_types", "common", "shared", "helpers", "utils", "functions", "logic", "misc"];

/// The recursive domain-directory source grammar, EXECUTED (SGB tranche 4b,
/// DEC-007 successor / SEED-CONCEPT-SPINE). The constitutional grammar is
/// filesystem-level law: every lib.rs-declared top-level domain is a
/// directory whose mod.rs is the facade and whose private types.rs carries
/// the domain-boundary canonical nouns; no sibling door file, crate-root
/// type drawer, generic helper drawer, wildcard facade, dangling facade
/// declaration, phantom generated-view authority source, unmanifested
/// module file, orphan domain directory, orphan carrier file, or non-test
/// `extern crate std` relink exists.
/// Thirteen rules, each producing a distinct finding that names the offending
/// path; the future AST gate owns finer placement law, so nothing here
/// parses Rust beyond comment/string-blind line scanning.
pub(crate) fn check_source_grammar(root: &Path, findings: &mut Vec<String>) {
    let spec = root.join("spec");
    let Ok(lib_text) = fs::read_to_string(spec.join("lib.rs")) else {
        findings.push("cannot read spec/lib.rs; the crate facade is the grammar denominator".to_string());
        return;
    };
    let lib_sanitized = sanitize_rust(&lib_text);
    let domains: Vec<String> = module_declarations(&lib_sanitized)
        .into_iter()
        .filter(|(_, public)| *public)
        .map(|(name, _)| name)
        .collect();
    for name in &domains {
        let dir = spec.join(name);
        // R1: the declared domain resolves to a directory facade.
        if !dir.join("mod.rs").is_file() {
            findings.push(format!(
                "declared domain {name} does not resolve to spec/{name}/mod.rs; every top-level domain is a directory with a mod.rs facade"
            ));
        }
        // R2: the killed concept-door shape stays dead.
        if spec.join(format!("{name}.rs")).is_file() {
            findings.push(format!(
                "sibling door file spec/{name}.rs exists beside the domain directory; the mod.rs facade is the only domain door"
            ));
        }
        // R3: the domain carries its canonical noun file.
        if dir.is_dir() && !dir.join("types.rs").is_file() {
            findings.push(format!(
                "domain directory spec/{name}/ carries no types.rs canonical noun carrier"
            ));
        }
        let Ok(mod_text) = fs::read_to_string(dir.join("mod.rs")) else {
            continue; // R1 already named the absent facade.
        };
        let sanitized = sanitize_rust(&mod_text);
        let declared = module_declarations(&sanitized);
        let mut mounts_types = false;
        for (child, public) in &declared {
            let (child, public) = (child.as_str(), *public);
            if child == "types" {
                mounts_types = true;
                // R7: the noun carrier is private vocabulary, re-exported
                // explicitly, never a public module path.
                if public {
                    findings.push(format!(
                        "spec/{name}/mod.rs declares its types module public; the canonical noun carrier is private and re-exported explicitly"
                    ));
                }
            }
            // R6: a facade declares only children that exist.
            if !dir.join(format!("{child}.rs")).is_file()
                && !dir.join(&child).join("mod.rs").is_file()
            {
                findings.push(format!(
                    "spec/{name}/mod.rs declares mod {child}; but no sibling {child}.rs or {child}/mod.rs exists"
                ));
            }
        }
        // R7 (mounting direction): an existing types.rs is mounted privately,
        // never left as an unreachable orphan.
        if dir.join("types.rs").is_file() && !mounts_types {
            findings.push(format!(
                "spec/{name}/mod.rs does not mount its types.rs; a private `mod types;` declaration is required"
            ));
        }
        // R8: public items pass through an explicit re-export list.
        if has_glob_reexport(&sanitized) {
            findings.push(format!(
                "glob re-export in spec/{name}/mod.rs; public items pass through an explicit re-export list, never a wildcard facade"
            ));
        }
        // R12 (reverse of R6): every immediate source child of the domain is
        // a declared carrier. `#[cfg(test)] mod tests;` counts (attributes
        // are stripped by module_declarations) and a declaration may resolve
        // to a child/mod.rs subdirectory instead of a sibling file.
        let Ok(entries) = fs::read_dir(&dir) else {
            continue; // R1 already named the absent directory.
        };
        let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        children.sort();
        for child_path in children {
            if !child_path.is_file()
                || !child_path.extension().is_some_and(|extension| extension == "rs")
            {
                continue;
            }
            let Some(stem) = child_path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if stem == "mod" {
                continue;
            }
            if !declared.iter().any(|(child, _)| child == stem) {
                findings.push(format!(
                    "orphan carrier spec/{name}/{stem}.rs is declared by no mod statement in spec/{name}/mod.rs; every domain child passes through the facade"
                ));
            }
        }
    }
    // R11 (reverse of R1): every domain directory on disk is a declared
    // facade — a mod.rs-bearing spec/ subdirectory lib.rs never mounts is an
    // orphan domain, invisible to the compiler and every census.
    if let Ok(entries) = fs::read_dir(&spec) {
        let mut dirs: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        dirs.sort();
        for dir in dirs {
            if !dir.is_dir() || !dir.join("mod.rs").is_file() {
                continue;
            }
            let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !domains.iter().any(|domain| domain == name) {
                findings.push(format!(
                    "orphan domain directory spec/{name}/ carries a mod.rs facade but spec/lib.rs declares no pub mod {name}"
                ));
            }
        }
    } else {
        findings.push("cannot read the spec/ directory; the domain census is the grammar denominator".to_string());
    }
    // R4: no universal type drawer at the crate root.
    if spec.join("types.rs").exists() {
        findings.push(
            "crate-root type drawer spec/types.rs exists; no universal type drawer is admitted"
                .to_string(),
        );
    }
    // R8 at the crate facade itself.
    if has_glob_reexport(&lib_sanitized) {
        findings.push(
            "glob re-export in spec/lib.rs; the crate facade re-exports nothing through wildcards"
                .to_string(),
        );
    }
    // R5: the whole tree, recursively; drawers hide in subdirectories too.
    let mut sources: Vec<PathBuf> = Vec::new();
    walk_spec(&spec, root, &mut sources, findings);
    // R13: no non-test spec source relinks std. `#![no_std]` on the facade
    // does not prevent a deliberate `extern crate std` inside a carrier, so
    // the refusal is SOURCE law scanned here, never compile luck. `extern
    // crate alloc` stays lawful, and tests.rs carriers compile under the
    // host test harness and are exempt per the existing test exclusion.
    for path in &sources {
        if path.file_stem().and_then(|stem| stem.to_str()) == Some("tests") {
            continue;
        }
        let rel = posix(path.strip_prefix(root).unwrap_or(path));
        match fs::read_to_string(path) {
            Ok(text) => {
                let relink = extern_std_line(&sanitize_rust(&text));
                if relink.is_some() {
                    findings.push(format!(
                        "spec source {rel} declares extern crate std (line {}); the spec is #![no_std] source law and a deliberate std relink is refused (extern crate alloc stays lawful)",
                        relink.unwrap_or(0)
                    ));
                }
            }
            Err(_) => findings.push(format!("cannot read {rel}")),
        }
    }
    // R9: every generated-view authority source is a real byte carrier.
    match fs::read_to_string(spec.join("generated_views/registry.rs")) {
        Ok(registry) => {
            for path in view_authority_paths(&registry) {
                if !root.join(&path).is_file() {
                    findings.push(format!(
                        "generated-view authority source {path} resolves to no file; the registry cites byte carriers, not phantoms"
                    ));
                }
            }
        }
        Err(_) => findings.push("cannot read spec/generated_views/registry.rs".to_string()),
    }
    // R10: the frozen manifest carries the complete module tree.
    match fs::read_to_string(root.join("SPEC.sha256")) {
        Ok(manifest) => {
            let members = manifest_members(&manifest);
            for path in &sources {
                let rel = posix(path.strip_prefix(root).unwrap_or(path));
                if !members.contains(&rel) {
                    findings.push(format!(
                        "spec source {rel} is missing from SPEC.sha256; the frozen manifest carries the complete module tree"
                    ));
                }
            }
        }
        Err(_) => findings.push(
            "cannot read SPEC.sha256; the frozen manifest is the module-tree denominator"
                .to_string(),
        ),
    }
}

/// The `mod` declarations of a facade file as (name, declared_public), parsed
/// line-by-line from comment/string-blind text. Same-line attributes
/// (`#[cfg(test)] mod tests;`) are stripped; any `pub` spelling (bare or
/// restricted) counts as public. Inline module bodies do not occur in facade
/// files and fail the trailing-`;` requirement.
pub(crate) fn module_declarations(sanitized: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for line in sanitized.lines() {
        let mut rest = line.trim();
        while rest.starts_with("#[") {
            match rest.find(']') {
                Some(end) => rest = rest[end + 1..].trim_start(),
                None => break,
            }
        }
        let (public, rest) = if let Some(after) = rest.strip_prefix("pub ") {
            (true, after.trim_start())
        } else if let Some(after) = rest.strip_prefix("pub(") {
            match after.find(')') {
                Some(close) => (true, after[close + 1..].trim_start()),
                None => continue,
            }
        } else {
            (false, rest)
        };
        let Some(decl) = rest.strip_prefix("mod ") else { continue };
        let Some(name) = decl.strip_suffix(';') else { continue };
        let name = name.trim();
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            out.push((name.to_string(), public));
        }
    }
    out
}

/// The 1-based line of the first `extern crate std` declaration (bare,
/// public, or renamed) in comment/string-blind source, if any. Line-scanned
/// like module_declarations: leading attributes and any `pub` spelling are
/// stripped, so a cfg-gated or re-exported relink cannot hide behind its own
/// decoration. Only `std` is refused; `extern crate alloc` stays lawful.
pub(crate) fn extern_std_line(sanitized: &str) -> Option<usize> {
    for (index, line) in sanitized.lines().enumerate() {
        let mut rest = line.trim();
        while rest.starts_with("#[") {
            match rest.find(']') {
                Some(end) => rest = rest[end + 1..].trim_start(),
                None => break,
            }
        }
        let rest = if let Some(after) = rest.strip_prefix("pub ") {
            after.trim_start()
        } else if let Some(after) = rest.strip_prefix("pub(") {
            match after.find(')') {
                Some(close) => after[close + 1..].trim_start(),
                None => rest,
            }
        } else {
            rest
        };
        if let Some(after) = rest.strip_prefix("extern crate std") {
            let after = after.trim_start();
            if after.starts_with(';') || after.starts_with("as ") {
                return Some(index + 1);
            }
        }
    }
    None
}

/// Whether comment/string-blind source contains a wildcard re-export. The
/// statement split keeps a plain `use ...::*;` import (lawful inside a
/// domain) from being confused with a `pub use ...::*;` facade, and catches
/// a glob nested inside a brace list.
pub(crate) fn has_glob_reexport(sanitized: &str) -> bool {
    sanitized
        .split(';')
        .any(|stmt| stmt.contains("pub use") && stmt.contains("::*"))
}

fn forbidden_module_name(name: &str) -> bool {
    FORBIDDEN_MODULE_NAMES.contains(&name)
}

// One sorted traversal serves R5 (forbidden drawer names) and collects the
// *.rs census R10 compares against the frozen manifest.
fn walk_spec(dir: &Path, root: &Path, sources: &mut Vec<PathBuf>, findings: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        findings.push(format!("cannot read {}", dir.display()));
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        let rel = posix(path.strip_prefix(root).unwrap_or(&path));
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()).is_some_and(forbidden_module_name) {
                findings.push(format!(
                    "forbidden module name {rel}: no _types/common/shared/helpers/utils/functions/logic/misc drawer exists without an explicit ruling"
                ));
            }
            walk_spec(&path, root, sources, findings);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if path.file_stem().and_then(|n| n.to_str()).is_some_and(forbidden_module_name) {
                findings.push(format!(
                    "forbidden module name {rel}: no _types/common/shared/helpers/utils/functions/logic/misc drawer exists without an explicit ruling"
                ));
            }
            sources.push(path);
        }
    }
}

/// Every `"spec/..."` string literal of the registry source, deduplicated.
/// The registry's compiled table is executed by check_generated_views; this
/// TEXTUAL scan holds the tracked bytes to the same law, so a repointed
/// literal reddens the gate even before the mutated table is ever linked.
fn view_authority_paths(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (index, _) in text.match_indices("\"spec/") {
        let rest = &text[index + 1..];
        if let Some(end) = rest.find('"') {
            out.insert(rest[..end].to_string());
        }
    }
    out
}

/// The relpath members of the frozen manifest (`<hex>  <relpath>` rows;
/// malformed rows are the freeze tool's refusal, not this rule's).
fn manifest_members(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(_digest), Some(path), None) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        out.insert(path.to_string());
    }
    out
}

fn posix(rel: &Path) -> String {
    rel.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

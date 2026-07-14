#![deny(warnings)]

#[path = "../spec/architecture.rs"]
mod architecture;
#[path = "../spec/invariants.rs"]
mod invariants;
#[path = "../spec/dispositions.rs"]
mod dispositions;
#[path = "../spec/legacy_obligations.rs"]
mod legacy_obligations;
#[path = "../spec/legacy_invariant_coverage.rs"]
mod legacy_invariant_coverage;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let root = env::args().nth(1).map_or_else(|| PathBuf::from("."), PathBuf::from);
    let findings = inspect(&root);
    if findings.is_empty() {
        let production = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::Production).count();
        let adapters = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::BinaryAdapter).count();
        let dev_only = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::DevOnly).count();
        let examples = architecture::PACKAGES.iter().filter(|p| p.class == architecture::PackageClass::Example).count();
        println!(
            "seedcheck: PASS {} v{} train {} ({} packages: {} production, {} adapter, {} dev-only, {} example; {} edges; {} qualification profiles; {} invariants; {} decisions; {} legacy obligations; {} legacy invariant dispositions)",
            architecture::REPOSITORY_NAME,
            architecture::SPEC_VERSION,
            architecture::WORKSPACE_VERSION,
            architecture::PACKAGES.len(),
            production,
            adapters,
            dev_only,
            examples,
            architecture::EDGES.len(),
            architecture::QUALIFICATION_PROFILES.len(),
            invariants::INVARIANTS.len(),
            dispositions::DECISIONS.len(),
            legacy_obligations::OBLIGATIONS.len(),
            legacy_invariant_coverage::COVERAGE.len(),
        );
        return;
    }
    eprintln!("seedcheck: FAIL ({} finding(s))", findings.len());
    for finding in findings { eprintln!("- {finding}"); }
    std::process::exit(1);
}

fn inspect(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for relative in architecture::REQUIRED_DOCS {
        if !root.join(relative).is_file() { findings.push(format!("missing required file {relative}")); }
    }
    for relative in architecture::FORBIDDEN_TARGET_PATHS {
        if root.join(relative).exists() { findings.push(format!("forbidden target path exists: {relative}")); }
    }
    check_graph(&mut findings);
    check_profiles(&mut findings);
    check_version(&mut findings);
    check_unique_ids(&mut findings);
    check_frontmatter(root, &mut findings);
    check_syncbat_shape(root, &mut findings);
    check_source_debt(root, &mut findings);
    findings
}

fn check_version(findings: &mut Vec<String>) {
    // Reject any workspace version below the signed 1.0 implementation train so
    // a template cannot regress the family to a pre-1.0 line (e.g. 0.1.0).
    let version = architecture::WORKSPACE_VERSION;
    let major = version.split('.').next().and_then(|part| part.parse::<u64>().ok());
    match major {
        Some(major) if major >= 1 => {}
        _ => findings.push(format!("workspace version {version} is below the signed 1.0 train")),
    }
}

fn check_graph(findings: &mut Vec<String>) {
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.package).collect();
    let layers: BTreeMap<&str, u8> = architecture::PACKAGES.iter().map(|p| (p.package, p.layer)).collect();
    if packages.len() != architecture::PACKAGES.len() { findings.push("duplicate package name".into()); }
    let paths: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.path).collect();
    if paths.len() != architecture::PACKAGES.len() { findings.push("duplicate package path".into()); }
    for package in architecture::PACKAGES {
        if package.role.trim().is_empty() { findings.push(format!("empty role for {}", package.package)); }
        if package.path.trim().is_empty() { findings.push(format!("empty path for {}", package.package)); }
    }
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in architecture::EDGES {
        if !packages.contains(edge.importer) { findings.push(format!("unknown importer {}", edge.importer)); }
        if !packages.contains(edge.importee) { findings.push(format!("unknown importee {}", edge.importee)); }
        if edge.importer == edge.importee { findings.push(format!("self dependency {}", edge.importer)); }
        if edge.profile.is_empty() { findings.push(format!("edge has empty profile {} -> {}", edge.importer, edge.importee)); }
        if let (Some(importer_layer), Some(importee_layer)) = (layers.get(edge.importer), layers.get(edge.importee)) {
            if importer_layer <= importee_layer {
                findings.push(format!("dependency direction violation {}(L{}) -> {}(L{})", edge.importer, importer_layer, edge.importee, importee_layer));
            }
        }
        if edge.importer == "testpak" && edge.class != architecture::EdgeClass::DevOnly {
            findings.push(format!("testpak edge must be dev-only: {}", edge.importee));
        }
        if edge.importee == "batpak-examples" {
            findings.push(format!("nothing may depend on the examples package: {} -> batpak-examples", edge.importer));
        }
        if edge.importer == "batpak-examples" && edge.importee == "testpak" {
            findings.push("batpak-examples must not depend on dev tooling (testpak)".to_string());
        }
        if edge.importer == "batpak-cli" && edge.class == architecture::EdgeClass::DevOnly {
            findings.push(format!("CLI edge cannot be dev-only: {}", edge.importee));
        }
        graph.entry(edge.importer).or_default().push(edge.importee);
    }
    for package in architecture::PACKAGES {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if cycle(package.package, &graph, &mut visiting, &mut visited) {
            findings.push(format!("dependency cycle reaches {}", package.package));
        }
    }
}

fn check_profiles(findings: &mut Vec<String>) {
    let packages: BTreeSet<&str> = architecture::PACKAGES.iter().map(|p| p.package).collect();
    let mut identities = BTreeSet::new();
    for profile in architecture::QUALIFICATION_PROFILES {
        if !packages.contains(profile.package) {
            findings.push(format!("unknown qualification package {}", profile.package));
        }
        if profile.profile.trim().is_empty() || profile.target.trim().is_empty() || profile.requirement.trim().is_empty() {
            findings.push(format!("incomplete qualification profile {}:{}", profile.package, profile.profile));
        }
        if !identities.insert((profile.package, profile.profile)) {
            findings.push(format!("duplicate qualification profile {}:{}", profile.package, profile.profile));
        }
    }
    for package in ["batpak", "syncbat"] {
        if !architecture::QUALIFICATION_PROFILES.iter().any(|p| p.package == package && p.profile == "semantic" && p.target == "no_std + alloc") {
            findings.push(format!("missing no_std + alloc semantic profile for {package}"));
        }
    }
}

fn cycle<'a>(node: &'a str, graph: &BTreeMap<&'a str, Vec<&'a str>>, visiting: &mut BTreeSet<&'a str>, visited: &mut BTreeSet<&'a str>) -> bool {
    if visited.contains(node) { return false; }
    if !visiting.insert(node) { return true; }
    if let Some(next) = graph.get(node) {
        for child in next { if cycle(child, graph, visiting, visited) { return true; } }
    }
    visiting.remove(node);
    visited.insert(node);
    false
}

fn check_unique_ids(findings: &mut Vec<String>) {
    let invariant_ids: BTreeSet<&str> = invariants::INVARIANTS.iter().map(|v| v.id).collect();
    if invariant_ids.len() != invariants::INVARIANTS.len() { findings.push("duplicate invariant ID".into()); }
    for value in invariants::INVARIANTS {
        if value.statement.trim().is_empty() { findings.push(format!("empty invariant statement {}", value.id)); }
    }
    let decision_ids: BTreeSet<&str> = dispositions::DECISIONS.iter().map(|v| v.id).collect();
    if decision_ids.len() != dispositions::DECISIONS.len() { findings.push("duplicate decision ID".into()); }
    for value in dispositions::DECISIONS {
        if value.subject.trim().is_empty() || value.successor.trim().is_empty() {
            findings.push(format!("incomplete decision {}", value.id));
        }
        match value.disposition {
            dispositions::Disposition::Keep
            | dispositions::Disposition::Lock
            | dispositions::Disposition::Kill
            | dispositions::Disposition::Supersede
            | dispositions::Disposition::Demote
            | dispositions::Disposition::Defer
            | dispositions::Disposition::OpenImplementation
            | dispositions::Disposition::RetainAsEvidence => {}
        }
    }
    let legacy_ids: BTreeSet<&str> = legacy_obligations::OBLIGATIONS.iter().map(|v| v.id).collect();
    if legacy_ids.len() != legacy_obligations::OBLIGATIONS.len() { findings.push("duplicate legacy obligation ID".into()); }
    for value in legacy_obligations::OBLIGATIONS {
        if value.law.trim().is_empty() || value.clean_owner.trim().is_empty() || value.gate.trim().is_empty() {
            findings.push(format!("incomplete legacy obligation {}", value.id));
        }
    }
    let coverage_ids: BTreeSet<&str> = legacy_invariant_coverage::COVERAGE.iter().map(|v| v.legacy_id).collect();
    if coverage_ids.len() != legacy_invariant_coverage::COVERAGE.len() { findings.push("duplicate legacy invariant coverage ID".into()); }
    if legacy_invariant_coverage::COVERAGE.len() != legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS {
        findings.push(format!("legacy invariant coverage row count is {}, expected {}", legacy_invariant_coverage::COVERAGE.len(), legacy_invariant_coverage::EXPECTED_COVERAGE_ROWS));
    }
    if legacy_invariant_coverage::LEGACY_SOURCE_COMMIT.len() != 40 {
        findings.push("legacy invariant source commit is not a full SHA".into());
    }
    for value in legacy_invariant_coverage::COVERAGE {
        if value.legacy_id.trim().is_empty() || value.successor.trim().is_empty() || value.rationale.trim().is_empty() {
            findings.push(format!("incomplete legacy invariant coverage {}", value.legacy_id));
        }
        match value.disposition {
            legacy_invariant_coverage::CoverageDisposition::Preserve
            | legacy_invariant_coverage::CoverageDisposition::Supersede
            | legacy_invariant_coverage::CoverageDisposition::Demote
            | legacy_invariant_coverage::CoverageDisposition::Kill
            | legacy_invariant_coverage::CoverageDisposition::Requalify => {}
        }
    }
}

fn check_frontmatter(root: &Path, findings: &mut Vec<String>) {
    for relative in architecture::REQUIRED_DOCS {
        if !relative.ends_with(".md") { continue; }
        let path=root.join(relative);
        if !path.is_file() { continue; }
        match fs::read_to_string(&path) {
            Ok(text) => {
                if !text.starts_with("---\n") { findings.push(format!("missing frontmatter: {relative}")); }
                for key in ["status:", "contract_id:", "authority_scope:", "supersedes:", "last_reconciled:"] {
                    if !text.contains(key) { findings.push(format!("missing {key} in {relative}")); }
                }
            }
            Err(error) => findings.push(format!("cannot read {relative}: {error}")),
        }
    }
}

fn check_syncbat_shape(root: &Path, findings: &mut Vec<String>) {
    let base=root.join("crates/syncbat");
    if !base.exists() { return; }
    for relative in architecture::SYNCBAT_REQUIRED_PLANES {
        if !base.join(relative).exists() { findings.push(format!("syncbat missing required plane {relative}")); }
    }
}

fn check_source_debt(root: &Path, findings: &mut Vec<String>) {
    for base in [root.join("crates"), root.join("apps")] {
        if !base.is_dir() { continue; }
        walk(&base, root, findings);
    }
}

fn walk(path: &Path, root: &Path, findings: &mut Vec<String>) {
    let Ok(entries)=fs::read_dir(path) else { findings.push(format!("cannot read {}", path.display())); return; };
    for entry in entries.flatten() {
        let p=entry.path();
        if p.is_dir() { walk(&p, root, findings); continue; }
        if p.extension().is_some_and(|e| e == "rs") {
            let Ok(text)=fs::read_to_string(&p) else { findings.push(format!("cannot read {}", p.display())); continue; };
            let rel=p.strip_prefix(root).unwrap_or(&p).display();
            let code = sanitize_rust(&text);
            for banned in ["#[allow", "#![allow", "#[expect", "#![expect", "todo!", "unimplemented!"] {
                if code.contains(banned) { findings.push(format!("banned source token {banned:?} in {rel}")); }
            }
            let tokens = rust_tokens(&code);
            let has_unsafe = tokens.iter().any(|token| token.text == "unsafe");
            if has_unsafe {
                let location = rel.to_string();
                let admitted_path = location.contains("kernel") || location.contains("adapter") || location.contains("ffi");
                if !admitted_path { findings.push(format!("unsafe code outside kernel/adapter/ffi path in {rel}")); }
                if !text.contains("SAFETY-CONTRACT:") { findings.push(format!("unsafe code lacks SAFETY-CONTRACT record in {rel}")); }
            }
            for line in function_local_type_lines(&tokens) {
                findings.push(format!("function-local named type at {rel}:{line}"));
            }
        }
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
struct RustToken {
    text: String,
    line: usize,
}

fn sanitize_rust(text: &str) -> String {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State { Code, LineComment, BlockComment, String, Char, RawString }

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut state = State::Code;
    let mut block_depth = 0usize;
    let mut raw_hashes = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();
        match state {
            State::Code => {
                if byte == b'/' && next == Some(b'/') {
                    out.push_str("  ");
                    index += 2;
                    state = State::LineComment;
                    continue;
                }
                if byte == b'/' && next == Some(b'*') {
                    out.push_str("  ");
                    index += 2;
                    block_depth = 1;
                    state = State::BlockComment;
                    continue;
                }
                if byte == b'"' {
                    out.push(' ');
                    index += 1;
                    state = State::String;
                    continue;
                }
                if byte == b'\'' {
                    let has_close = bytes[index + 1..bytes.len().min(index + 8)]
                        .iter()
                        .position(|candidate| *candidate == b'\'');
                    if has_close.is_some() {
                        out.push(' ');
                        index += 1;
                        state = State::Char;
                        continue;
                    }
                }
                if byte == b'r' {
                    let mut cursor = index + 1;
                    while bytes.get(cursor) == Some(&b'#') { cursor += 1; }
                    if bytes.get(cursor) == Some(&b'"') {
                        raw_hashes = cursor - index - 1;
                        for _ in index..=cursor { out.push(' '); }
                        index = cursor + 1;
                        state = State::RawString;
                        continue;
                    }
                }
                out.push(char::from(byte));
                index += 1;
            }
            State::LineComment => {
                if byte == b'\n' { out.push('\n'); state = State::Code; } else { out.push(' '); }
                index += 1;
            }
            State::BlockComment => {
                if byte == b'/' && next == Some(b'*') {
                    out.push_str("  "); index += 2; block_depth += 1; continue;
                }
                if byte == b'*' && next == Some(b'/') {
                    out.push_str("  "); index += 2; block_depth -= 1;
                    if block_depth == 0 { state = State::Code; }
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
            State::String => {
                if byte == b'\\' {
                    out.push(' ');
                    if index + 1 < bytes.len() { out.push(if bytes[index + 1] == b'\n' { '\n' } else { ' ' }); }
                    index += 2;
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
                if byte == b'"' { state = State::Code; }
            }
            State::Char => {
                if byte == b'\\' {
                    out.push(' ');
                    if index + 1 < bytes.len() { out.push(if bytes[index + 1] == b'\n' { '\n' } else { ' ' }); }
                    index += 2;
                    continue;
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
                if byte == b'\'' { state = State::Code; }
            }
            State::RawString => {
                if byte == b'"' {
                    let end = index + 1 + raw_hashes;
                    if end <= bytes.len() && bytes[index + 1..end].iter().all(|candidate| *candidate == b'#') {
                        for _ in index..end { out.push(' '); }
                        index = end;
                        state = State::Code;
                        continue;
                    }
                }
                out.push(if byte == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
        }
    }
    out
}

fn rust_tokens(code: &str) -> Vec<RustToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_line = 1usize;
    let mut line = 1usize;
    let flush = |tokens: &mut Vec<RustToken>, current: &mut String, current_line: usize| {
        if !current.is_empty() {
            tokens.push(RustToken { text: std::mem::take(current), line: current_line });
        }
    };
    for character in code.chars() {
        if character == '\n' {
            flush(&mut tokens, &mut current, current_line);
            line += 1;
            continue;
        }
        if character.is_ascii_alphanumeric() || character == '_' {
            if current.is_empty() { current_line = line; }
            current.push(character);
        } else {
            flush(&mut tokens, &mut current, current_line);
            if matches!(character, '{' | '}' | ';') {
                tokens.push(RustToken { text: character.to_string(), line });
            }
        }
    }
    flush(&mut tokens, &mut current, current_line);
    tokens
}

fn function_local_type_lines(tokens: &[RustToken]) -> BTreeSet<usize> {
    let mut contexts: Vec<bool> = Vec::new();
    let mut pending_function = false;
    let mut findings = BTreeSet::new();
    for token in tokens {
        match token.text.as_str() {
            "fn" => pending_function = true,
            "{" => {
                contexts.push(pending_function);
                pending_function = false;
            }
            ";" => pending_function = false,
            "}" => {
                let _ = contexts.pop();
                pending_function = false;
            }
            "struct" | "enum" | "trait" | "union" | "type"
                if contexts.iter().any(|is_function| *is_function) => {
                    findings.insert(token.line);
                }
            _ => {}
        }
    }
    findings
}

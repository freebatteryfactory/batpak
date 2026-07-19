use spec::architecture;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn check_syncbat_shape(root: &Path, findings: &mut Vec<String>) {
    // The signed seed carries no generated workspace (5.5E5): plane files are
    // checked only where a materialized candidate is present, and their paths
    // derive from SyncBatPlane::ALL, never from a raw path table.
    let base = root.join("crates/syncbat");
    if !base.exists() { return; }
    for &plane in architecture::SyncBatPlane::ALL {
        let module = plane.module_name();
        if !base.join(format!("src/{module}/mod.rs")).exists() {
            findings.push(format!("syncbat missing required plane src/{module}/mod.rs"));
        }
        if !base.join(format!("src/{module}/types.rs")).exists() {
            findings.push(format!("syncbat missing required plane src/{module}/types.rs"));
        }
    }
}

pub(crate) fn check_source_debt(root: &Path, findings: &mut Vec<String>) {
    for base in [root.join("crates"), root.join("apps"), root.join("examples")] {
        if !base.is_dir() { continue; }
        walk(&base, root, findings);
    }
}

// Conservative, path-based test ownership. The bootstrap oracle never parses
// arbitrary Rust test bodies; TestPak's AST gate owns the precise distinction.
pub(crate) fn is_test_owned(rel: &Path) -> bool {
    for component in rel.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(text) = name.to_str() {
                if matches!(text, "tests" | "benches" | "fixtures" | "corpus" | "fuzz") {
                    return true;
                }
            }
        }
    }
    false
}

fn walk(path: &Path, root: &Path, findings: &mut Vec<String>) {
    let Ok(entries)=fs::read_dir(path) else { findings.push(format!("cannot read {}", path.display())); return; };
    for entry in entries.flatten() {
        let p=entry.path();
        if p.is_dir() { walk(&p, root, findings); continue; }
        if p.extension().is_some_and(|e| e == "rs") {
            let Ok(text)=fs::read_to_string(&p) else { findings.push(format!("cannot read {}", p.display())); continue; };
            let rel_path = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
            let rel=rel_path.display();
            let code = sanitize_rust(&text);
            for banned in ["#[allow", "#![allow", "#[expect", "#![expect", "todo!", "unimplemented!"] {
                if code.contains(banned) { findings.push(format!("banned source token {banned:?} in {rel}")); }
            }
            // Production-source lexical debt (defense in depth). Test-owned paths
            // are exempt so contextual .expect() in tests remains legal.
            if !is_test_owned(&rel_path) {
                for banned in [".unwrap(", ".expect(", "panic!", "dbg!"] {
                    if code.contains(banned) { findings.push(format!("banned production token {banned:?} in {rel}")); }
                }
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

pub(crate) fn sanitize_rust(text: &str) -> String {
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


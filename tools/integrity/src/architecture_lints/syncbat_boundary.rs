use super::{ensure, relative};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

struct BoundaryTerm {
    token: &'static str,
    reason: &'static str,
}

struct InternalPathTerm {
    module: &'static str,
    reason: &'static str,
}

const CORE_LAYER_LEAKS: &[BoundaryTerm] = &[
    BoundaryTerm {
        token: "syncbat",
        reason: "runtime layer name belongs outside batpak core",
    },
    BoundaryTerm {
        token: "Syncbat",
        reason: "runtime layer type names belong outside batpak core",
    },
    BoundaryTerm {
        token: "clawbat",
        reason: "contract layer names belong outside batpak core",
    },
    BoundaryTerm {
        token: "Clawbat",
        reason: "contract layer type names belong outside batpak core",
    },
    BoundaryTerm {
        token: "netbat",
        reason: "network layer names belong outside batpak core",
    },
    BoundaryTerm {
        token: "Netbat",
        reason: "network layer type names belong outside batpak core",
    },
    BoundaryTerm {
        token: "contract.context_v1",
        reason: "PCP profile wire validation belongs outside batpak core",
    },
    BoundaryTerm {
        token: "authority_required",
        reason: "authority claims are caller policy input, not substrate law",
    },
    BoundaryTerm {
        token: "PCP-Core",
        reason: "PCP semantics stay outside batpak core",
    },
    BoundaryTerm {
        token: "PcpProfile",
        reason: "PCP profile types stay outside batpak core",
    },
];

const SYNCBAT_LAYER_LEAKS: &[BoundaryTerm] = &[
    BoundaryTerm {
        token: "clawbat",
        reason: "contract layer names belong outside syncbat",
    },
    BoundaryTerm {
        token: "Clawbat",
        reason: "contract layer type names belong outside syncbat",
    },
    BoundaryTerm {
        token: "netbat",
        reason: "network layer names belong outside syncbat",
    },
    BoundaryTerm {
        token: "Netbat",
        reason: "network layer type names belong outside syncbat",
    },
    BoundaryTerm {
        token: "contract.context_v1",
        reason: "PCP profile wire validation belongs outside syncbat",
    },
    BoundaryTerm {
        token: "authority_required",
        reason: "authority claims are caller policy input, not syncbat law",
    },
    BoundaryTerm {
        token: "PCP-Core",
        reason: "PCP semantics stay outside syncbat",
    },
    BoundaryTerm {
        token: "PcpProfile",
        reason: "PCP profile types stay outside syncbat",
    },
];

const CLAWBAT_LAYER_LEAKS: &[BoundaryTerm] = &[
    BoundaryTerm {
        token: "netbat",
        reason: "network layer names belong outside clawbat",
    },
    BoundaryTerm {
        token: "Netbat",
        reason: "network layer type names belong outside clawbat",
    },
    BoundaryTerm {
        token: "contract.context_v1",
        reason: "PCP profile wire validation belongs outside clawbat",
    },
    BoundaryTerm {
        token: "authority_required",
        reason: "authority claims are caller policy input, not clawbat law",
    },
    BoundaryTerm {
        token: "PCP-Core",
        reason: "PCP semantics stay outside clawbat",
    },
    BoundaryTerm {
        token: "PcpProfile",
        reason: "PCP profile types stay outside clawbat",
    },
];

const NETBAT_LAYER_LEAKS: &[BoundaryTerm] = &[
    BoundaryTerm {
        token: "contract.context_v1",
        reason: "PCP profile wire validation belongs outside netbat",
    },
    BoundaryTerm {
        token: "authority_required",
        reason: "authority claims are caller policy input, not netbat law",
    },
    BoundaryTerm {
        token: "PCP-Core",
        reason: "PCP semantics stay outside netbat",
    },
    BoundaryTerm {
        token: "PcpProfile",
        reason: "PCP profile types stay outside netbat",
    },
    BoundaryTerm {
        token: "batpak::",
        reason: "netbat should expose syncbat, not bypass the runtime into batpak",
    },
];

const FAMILY_INTERNAL_BATPAK_PATHS: &[InternalPathTerm] = &[
    InternalPathTerm {
        module: "write",
        reason: "family layers must use batpak's public substrate API, not store write internals",
    },
    InternalPathTerm {
        module: "segment",
        reason: "family layers must use batpak's public substrate API, not segment internals",
    },
    InternalPathTerm {
        module: "index",
        reason: "family layers must use batpak's public substrate API, not index internals",
    },
    InternalPathTerm {
        module: "cold_start",
        reason: "family layers must use batpak's public substrate API, not cold-start internals",
    },
    InternalPathTerm {
        module: "platform",
        reason: "family layers must use batpak's public substrate API, not platform internals",
    },
    InternalPathTerm {
        module: "projection",
        reason: "family layers must use batpak's public substrate API, not projection internals",
    },
    InternalPathTerm {
        module: "delivery",
        reason: "family layers must use batpak's public substrate API, not delivery internals",
    },
    InternalPathTerm {
        module: "ancestry",
        reason: "family layers must use batpak's public substrate API, not ancestry internals",
    },
];

pub(super) fn check(repo_root: &Path, tracked_files: &[PathBuf]) -> Result<()> {
    for path in tracked_files {
        let layer = match source_layer(repo_root, path) {
            Some(layer) => layer,
            None => continue,
        };
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

        let semantic_content = semantic_content(&content);

        for term in forbidden_layer_terms(layer, &semantic_content) {
            ensure(
                false,
                format!(
                    "{} layer leak in {}: `{}` ({})",
                    layer.label(),
                    relative(repo_root, path),
                    term.token,
                    term.reason
                ),
            )?;
        }

        if layer.checks_internal_batpak_paths() {
            for term in family_internal_batpak_paths(&semantic_content) {
                ensure(
                    false,
                    format!(
                        "{} batpak internal dependency in {}: `batpak::store::{}` ({})",
                        layer.label(),
                        relative(repo_root, path),
                        term.module,
                        term.reason
                    ),
                )?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceLayer {
    Core,
    Syncbat,
    Clawbat,
    Netbat,
}

impl SourceLayer {
    fn label(self) -> &'static str {
        match self {
            SourceLayer::Core => "batpak core",
            SourceLayer::Syncbat => "syncbat",
            SourceLayer::Clawbat => "clawbat",
            SourceLayer::Netbat => "netbat",
        }
    }

    fn checks_internal_batpak_paths(self) -> bool {
        matches!(
            self,
            SourceLayer::Syncbat | SourceLayer::Clawbat | SourceLayer::Netbat
        )
    }
}

fn source_layer(repo_root: &Path, path: &Path) -> Option<SourceLayer> {
    let rel = relative(repo_root, path);
    if !rel.ends_with(".rs") {
        return None;
    }
    if rel.starts_with("crates/core/src/") {
        return Some(SourceLayer::Core);
    }
    if rel.starts_with("crates/syncbat/src/") || rel.starts_with("crates/syncbat-macros/src/") {
        return Some(SourceLayer::Syncbat);
    }
    if rel.starts_with("crates/clawbat/src/") {
        return Some(SourceLayer::Clawbat);
    }
    if rel.starts_with("crates/netbat/src/") {
        return Some(SourceLayer::Netbat);
    }
    None
}

fn forbidden_layer_terms(layer: SourceLayer, content: &str) -> Vec<&'static BoundaryTerm> {
    let terms = match layer {
        SourceLayer::Core => CORE_LAYER_LEAKS,
        SourceLayer::Syncbat => SYNCBAT_LAYER_LEAKS,
        SourceLayer::Clawbat => CLAWBAT_LAYER_LEAKS,
        SourceLayer::Netbat => NETBAT_LAYER_LEAKS,
    };
    matching_terms(terms, content)
}

fn family_internal_batpak_paths(content: &str) -> Vec<&'static InternalPathTerm> {
    let compact = compact(content);
    FAMILY_INTERNAL_BATPAK_PATHS
        .iter()
        .filter(|term| {
            let direct = format!("batpak::store::{}", term.module);
            let grouped_crate = format!("batpak::{{store::{}", term.module);
            let nested_grouped_crate = format!("batpak::{{store::{{{}", term.module);
            compact.contains(&direct)
                || compact.contains(&grouped_crate)
                || compact.contains(&nested_grouped_crate)
                || grouped_path_contains(&compact, "batpak::store::{", term.module)
                || grouped_path_contains(&compact, "batpak::{store::{", term.module)
        })
        .collect()
}

fn matching_terms(terms: &'static [BoundaryTerm], content: &str) -> Vec<&'static BoundaryTerm> {
    terms
        .iter()
        .filter(|term| content.contains(term.token))
        .collect()
}

fn semantic_content(content: &str) -> String {
    strip_block_comments(content)
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn grouped_path_contains(content: &str, prefix: &str, module: &str) -> bool {
    let mut rest = content;
    while let Some(start) = rest.find(prefix) {
        let group = &rest[start + prefix.len()..];
        let end = group.find('}').unwrap_or(group.len());
        let group = &group[..end];
        if group_entry_matches(group, module) {
            return true;
        }
        rest = &rest[start + prefix.len()..];
    }
    false
}

fn group_entry_matches(group: &str, module: &str) -> bool {
    let colon = format!("{module}::");
    let comma = format!("{module},");
    let brace = format!("{module}}}");
    group == module
        || group.starts_with(&colon)
        || group.starts_with(&comma)
        || group.ends_with(&brace)
        || group.contains(&format!(",{colon}"))
        || group.contains(&format!("{{{colon}"))
        || group.contains(&format!(",{comma}"))
        || group.contains(&format!("{{{comma}"))
}

fn strip_block_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_block = false;

    while let Some(ch) = chars.next() {
        if in_block {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block = true;
            continue;
        }

        out.push(ch);
    }

    out
}

fn compact(content: &str) -> String {
    content.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        family_internal_batpak_paths, forbidden_layer_terms, semantic_content, source_layer,
        SourceLayer,
    };
    use std::path::Path;

    fn tokens(leaks: Vec<&'static super::BoundaryTerm>) -> Vec<&'static str> {
        leaks.iter().map(|leak| leak.token).collect()
    }

    fn path_modules(leaks: Vec<&'static super::InternalPathTerm>) -> Vec<&'static str> {
        leaks.iter().map(|leak| leak.module).collect()
    }

    #[test]
    fn detects_core_layer_leaks() {
        let content = "pub struct SyncbatCore;\nconst PROFILE: &str = \"contract.context_v1\";\n";
        let tokens = tokens(forbidden_layer_terms(SourceLayer::Core, content));
        assert!(tokens.contains(&"Syncbat"));
        assert!(tokens.contains(&"contract.context_v1"));
    }

    #[test]
    fn detects_syncbat_layer_leaks() {
        let content = "pub struct ClawbatRuntime;\nconst CLAIM: &str = \"authority_required\";\n";
        let tokens = tokens(forbidden_layer_terms(SourceLayer::Syncbat, content));
        assert!(tokens.contains(&"Clawbat"));
        assert!(tokens.contains(&"authority_required"));
    }

    #[test]
    fn detects_clawbat_layer_leaks() {
        let content = "pub struct NetbatGateway;\nconst PROFILE: &str = \"contract.context_v1\";\n";
        let tokens = tokens(forbidden_layer_terms(SourceLayer::Clawbat, content));
        assert!(tokens.contains(&"Netbat"));
        assert!(tokens.contains(&"contract.context_v1"));
    }

    #[test]
    fn detects_netbat_layer_leaks() {
        let content = "let _ = batpak::Store::open;\nconst CLAIM: &str = \"authority_required\";\n";
        let tokens = tokens(forbidden_layer_terms(SourceLayer::Netbat, content));
        assert!(tokens.contains(&"batpak::"));
        assert!(tokens.contains(&"authority_required"));
    }

    #[test]
    fn allows_public_substrate_terms() {
        let content = "Store AppendReceipt GateSet Pipeline opaque extension cargo";
        assert!(forbidden_layer_terms(SourceLayer::Core, content).is_empty());
        assert!(forbidden_layer_terms(SourceLayer::Syncbat, content).is_empty());
        assert!(forbidden_layer_terms(SourceLayer::Clawbat, content).is_empty());
        assert!(forbidden_layer_terms(SourceLayer::Netbat, content).is_empty());
    }

    #[test]
    fn allows_syncbat_public_batpak_paths() {
        let content = "use batpak::{AppendOptions, Store};\nuse batpak::prelude::*;\n";
        assert!(forbidden_layer_terms(SourceLayer::Syncbat, content).is_empty());
        assert!(family_internal_batpak_paths(content).is_empty());
    }

    #[test]
    fn rejects_syncbat_internal_batpak_paths() {
        let content = "use batpak::store::segment::FrameHeader;\n";
        let tokens = path_modules(family_internal_batpak_paths(content));
        assert_eq!(tokens, vec!["segment"]);
    }

    #[test]
    fn rejects_syncbat_grouped_internal_batpak_paths() {
        let direct_group = "use batpak::store::{Store, segment::FrameHeader};\n";
        let crate_group = "use batpak::{store::index::IndexEntry};\n";
        let nested_group = "use batpak::{store::{Store, platform::Probe}};\n";

        assert_eq!(
            path_modules(family_internal_batpak_paths(direct_group)),
            vec!["segment"]
        );
        assert_eq!(
            path_modules(family_internal_batpak_paths(crate_group)),
            vec!["index"]
        );
        assert_eq!(
            path_modules(family_internal_batpak_paths(nested_group)),
            vec!["platform"]
        );
    }

    #[test]
    fn ignores_comment_only_boundary_terms() {
        let content = "//! This layer does not implement PCP-Core.\n/** Nor PcpProfile. */\n/*! Nor contract.context_v1. */\npub struct Plain;\n";
        let semantic = semantic_content(content);
        assert!(forbidden_layer_terms(SourceLayer::Syncbat, &semantic).is_empty());
    }

    #[test]
    fn selects_only_production_rust_sources() {
        let root = Path::new("/repo");

        assert_eq!(
            source_layer(root, Path::new("/repo/crates/core/src/store/mod.rs")),
            Some(SourceLayer::Core)
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/syncbat/src/lib.rs")),
            Some(SourceLayer::Syncbat)
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/syncbat-macros/src/lib.rs")),
            Some(SourceLayer::Syncbat)
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/clawbat/src/lib.rs")),
            Some(SourceLayer::Clawbat)
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/netbat/src/lib.rs")),
            Some(SourceLayer::Netbat)
        );
        assert_eq!(
            source_layer(
                root,
                Path::new("/repo/crates/core/tests/substrate_additions.rs")
            ),
            None
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/syncbat/examples/basic.rs")),
            None
        );
        assert_eq!(
            source_layer(root, Path::new("/repo/crates/syncbat/src/readme.md")),
            None
        );
    }
}

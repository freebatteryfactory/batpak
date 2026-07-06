use crate::repo_surface::{core_src_root, core_tests_root, rust_files};
use crate::rust_ast::{callee_path_segments, tail_owner_method};
use crate::source_cache::SourceCache;
use anyhow::{bail, Context, Result};
use quote::ToTokens;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;
use syn::visit::{self, Visit};
use syn::Item;

const ALLOWLIST: &[&str] = &[
    // `subscription` is doc(hidden) glue for async integration, exercised
    // indirectly via `subscribe` in every subscription test.
    "subscription",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StorePubFnCoverage {
    pub(crate) name: String,
    pub(crate) covered: bool,
    pub(crate) allowlisted: bool,
}

/// Assert that every `pub fn` declared in inherent `impl Store { ... }` blocks
/// under `crates/core/src/store/` has at least one reference in the test or source tree.
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
pub(crate) fn check(repo_root: &Path, source_cache: &mut SourceCache) -> Result<()> {
    let inventory = inventory(repo_root, source_cache)?;
    let unreferenced = inventory
        .iter()
        .filter(|entry| !entry.covered && !entry.allowlisted)
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();

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
             Investigate: crates/core/src/store/ and add a test exercising each, or remove the\n\
             method if it's truly unused."
        );
    }

    Ok(())
}

pub(crate) fn inventory(
    repo_root: &Path,
    source_cache: &mut SourceCache,
) -> Result<Vec<StorePubFnCoverage>> {
    // 1. Parse every store source file with syn and walk all inherent
    // `impl Store` blocks. Store's public surface is intentionally split by
    // owner modules; the detector follows that architecture instead of
    // hardcoding `src/store/mod.rs`.
    let mut pub_fns: BTreeSet<String> = BTreeSet::new();
    for store_source_path in rust_files(&core_src_root(repo_root).join("store")) {
        let ast = source_cache
            .parse_rust(&store_source_path)
            .with_context(|| format!("syn parse {}", store_source_path.display()))?;

        for item in &ast.items {
            if let Item::Impl(impl_block) = item {
                // Match `impl Store`, `impl Store<Open>`, and `impl<T> Store<T>`.
                // We only care about blocks whose self type path segment is
                // exactly `Store`.
                let is_store_impl = if let syn::Type::Path(tp) = impl_block.self_ty.as_ref() {
                    tp.path
                        .segments
                        .last()
                        .map(|s| s.ident == "Store")
                        .unwrap_or(false)
                } else {
                    false
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
             block under crates/core/src/store/. The Store surface may have moved outside the declared owner."
        );
    }

    // 2. Build the reference corpus: parseable .rs files under tests/ and src/.
    // Compile-fail UI fixtures are intentionally invalid Rust and are skipped;
    // comments and string literals never count because reference detection is AST-based.
    let mut search_asts: Vec<Rc<syn::File>> = Vec::new();
    for path in rust_files(&core_tests_root(repo_root))
        .into_iter()
        .chain(rust_files(&core_src_root(repo_root)))
    {
        if let Some(ast) = source_cache.parse_rust_if_valid(&path)? {
            search_asts.push(ast);
        }
    }

    // Corpus-wide set of identifiers evidenced as `Store`-typed (params or `let`
    // bindings whose type or initializer mentions the `Store` type). A method call
    // counts as Store coverage only when its receiver is one of these idents,
    // `self` inside an `impl Store`, or a `Store::` associated-call chain — so an
    // unrelated same-named method on a DIFFERENT type no longer satisfies coverage.
    let mut store_idents: BTreeSet<String> = BTreeSet::new();
    for ast in &search_asts {
        collect_store_idents(ast, &mut store_idents);
    }

    Ok(pub_fns
        .into_iter()
        .map(|name| StorePubFnCoverage {
            allowlisted: ALLOWLIST.contains(&name.as_str()),
            covered: search_asts
                .iter()
                .any(|ast| ast_references_store_method(ast, &name, &store_idents)),
            name,
        })
        .collect())
}

/// True when an `impl` block is an inherent `impl Store` (self type's last path
/// segment is exactly `Store`, e.g. `impl Store`, `impl Store<Open>`,
/// `impl<T> Store<T>`) — matching the pub-fn inventory's own `impl Store` filter.
fn impl_self_is_store(node: &syn::ItemImpl) -> bool {
    if let syn::Type::Path(tp) = node.self_ty.as_ref() {
        tp.path
            .segments
            .last()
            .map(|s| s.ident == "Store")
            .unwrap_or(false)
    } else {
        false
    }
}

/// True when a token stream carries an `Ident` token exactly equal to `Store`
/// (recursing into groups). Distinguishes the `Store` type from same-prefixed
/// idents like `StoreConfig`/`StoreIndex`/`StoreFile`, which never match.
fn tokens_mention_store(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(id) => id == "Store",
        proc_macro2::TokenTree::Group(group) => tokens_mention_store(group.stream()),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// The bound identifier of a simple binding pattern (`x`, `mut x`, `x: T`).
/// Returns `None` for destructuring patterns (tuples, structs) — those are not
/// tracked as Store receivers.
fn pat_bound_ident(pat: &syn::Pat) -> Option<String> {
    // `syn::Pat` is a large `#[non_exhaustive]` foreign enum; use `if let` dispatch
    // over the two relevant variants rather than a `match` with a wildcard arm
    // (which the workspace `wildcard_enum_match_arm` lint forbids).
    if let syn::Pat::Ident(ident) = pat {
        Some(ident.ident.to_string())
    } else if let syn::Pat::Type(typed) = pat {
        pat_bound_ident(&typed.pat)
    } else {
        None
    }
}

/// Collect, into `idents`, every identifier evidenced as `Store`-typed in `ast`:
/// a function/closure parameter whose type mentions `Store`, or a `let` binding
/// whose type annotation OR initializer mentions `Store` (so `let s = Store::open(..)`,
/// `let s = Arc::new(Store::open(..))`, and `fn f(s: &Store)` all register `s`).
fn collect_store_idents(ast: &syn::File, idents: &mut BTreeSet<String>) {
    struct Collector<'a> {
        idents: &'a mut BTreeSet<String>,
    }

    impl<'ast> Visit<'ast> for Collector<'_> {
        fn visit_local(&mut self, node: &'ast syn::Local) {
            if let Some(name) = pat_bound_ident(&node.pat) {
                let typed_store = matches!(&node.pat, syn::Pat::Type(typed)
                    if tokens_mention_store(typed.ty.to_token_stream()));
                let init_store = node
                    .init
                    .as_ref()
                    .is_some_and(|init| tokens_mention_store(init.expr.to_token_stream()));
                if typed_store || init_store {
                    self.idents.insert(name);
                }
            }
            visit::visit_local(self, node);
        }

        fn visit_signature(&mut self, node: &'ast syn::Signature) {
            for input in &node.inputs {
                if let syn::FnArg::Typed(typed) = input {
                    if tokens_mention_store(typed.ty.to_token_stream()) {
                        if let Some(name) = pat_bound_ident(&typed.pat) {
                            self.idents.insert(name);
                        }
                    }
                }
            }
            visit::visit_signature(self, node);
        }
    }

    let mut collector = Collector { idents };
    collector.visit_file(ast);
}

fn ast_references_store_method(
    ast: &syn::File,
    name: &str,
    store_idents: &BTreeSet<String>,
) -> bool {
    struct MethodCallFinder<'a> {
        name: &'a str,
        store_idents: &'a BTreeSet<String>,
        self_is_store: bool,
        found: bool,
    }

    impl MethodCallFinder<'_> {
        /// True when `recv` is a `Store` value: `self` inside an `impl Store`, an
        /// identifier evidenced as `Store`-typed, or a `Store::` associated-call
        /// chain base (peeling `?`/`.await`/parens).
        fn receiver_is_store(&self, recv: &syn::Expr) -> bool {
            // `syn::Expr` is a large `#[non_exhaustive]` foreign enum; use `if let`
            // dispatch rather than a `match` with a wildcard arm (which the
            // workspace `wildcard_enum_match_arm` lint forbids). Peel type-preserving
            // wrappers, then classify the base receiver.
            if let syn::Expr::Paren(inner) = recv {
                return self.receiver_is_store(&inner.expr);
            }
            if let syn::Expr::Reference(inner) = recv {
                return self.receiver_is_store(&inner.expr);
            }
            if let syn::Expr::Try(inner) = recv {
                return self.receiver_is_store(&inner.expr);
            }
            if let syn::Expr::Await(inner) = recv {
                return self.receiver_is_store(&inner.base);
            }
            if let syn::Expr::MethodCall(inner) = recv {
                return self.receiver_is_store(&inner.receiver);
            }
            if let syn::Expr::Path(path) = recv {
                if path.path.is_ident("self") {
                    return self.self_is_store;
                }
                return path
                    .path
                    .get_ident()
                    .is_some_and(|ident| self.store_idents.contains(&ident.to_string()));
            }
            if let syn::Expr::Call(call) = recv {
                return callee_path_segments(&call.func)
                    .and_then(|segments| {
                        tail_owner_method(&segments).map(|(owner, _)| owner == "Store")
                    })
                    .unwrap_or(false);
            }
            false
        }
    }

    impl<'ast> Visit<'ast> for MethodCallFinder<'_> {
        fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
            let previous = self.self_is_store;
            self.self_is_store = impl_self_is_store(node);
            visit::visit_item_impl(self, node);
            self.self_is_store = previous;
        }

        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            if node.method == self.name && self.receiver_is_store(&node.receiver) {
                self.found = true;
                return;
            }
            visit::visit_expr_method_call(self, node);
        }

        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            if let Some(segments) = callee_path_segments(&node.func) {
                if let Some((owner, method)) = tail_owner_method(&segments) {
                    if owner == "Store" && method == self.name {
                        self.found = true;
                        return;
                    }
                }
            }
            visit::visit_expr_call(self, node);
        }
    }

    let mut finder = MethodCallFinder {
        name,
        store_idents,
        self_is_store: false,
        found: false,
    };
    finder.visit_file(ast);
    finder.found
}

#[cfg(test)]
mod tests {
    use super::{ast_references_store_method, check};
    use crate::repo_surface::repo_root;
    use crate::source_cache::SourceCache;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "batpak-store-pub-fn-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp repo");
        path
    }

    fn write_file(repo: &Path, rel: &str, body: &str) {
        let path = repo.join(rel);
        fs::create_dir_all(path.parent().expect("parent dir")).expect("create dirs");
        fs::write(&path, body).expect("write fixture file");
    }

    /// END-TO-END RED FIXTURE: a `pub fn` on `impl Store` with ZERO test/source
    /// references must make the gate's `check(..)` fail; a single witnessing
    /// reference must make it pass. Both halves run so the test cannot pass
    /// vacuously — neutralizing the violation (adding the witness) turns it red.
    #[test]
    fn store_pub_fn_coverage_rejects_uncovered_store_method() {
        let repo = temp_repo("uncovered");
        write_file(
            &repo,
            "crates/core/src/store/mod.rs",
            "pub struct Store;\nimpl Store {\n    pub fn orphaned_probe(&self) -> u8 { 0 }\n}\n",
        );

        // GREEN: a test file that actually calls the method satisfies coverage.
        write_file(
            &repo,
            "crates/core/tests/probe.rs",
            "#[test]\nfn exercises() {\n    let store = batpak::store::Store;\n    let _ = store.orphaned_probe();\n}\n",
        );
        let mut cache = SourceCache::new(&repo);
        check(&repo, &mut cache).expect("a witnessed Store pub fn is accepted");

        // RED: remove the witness — the orphaned pub fn must be flagged.
        write_file(
            &repo,
            "crates/core/tests/probe.rs",
            "#[test]\nfn unrelated() {\n    assert_eq!(1, 1);\n}\n",
        );
        let mut cache = SourceCache::new(&repo);
        let err = check(&repo, &mut cache).expect_err("orphaned Store pub fn is rejected");
        assert!(
            err.to_string().contains("Store pub fn coverage failure")
                && err.to_string().contains("orphaned_probe"),
            "{err:?}"
        );

        fs::remove_dir_all(repo).expect("remove temp repo");
    }

    /// RED FIXTURE: a `pub fn` on `impl Store` whose ONLY corpus reference is a
    /// method call on a NON-Store receiver must NOT count as coverage. Name-only
    /// matching wrongly credits the unrelated same-named method; receiver-typed
    /// matching does not.
    #[test]
    fn store_pub_fn_coverage_ignores_same_named_method_on_other_type() {
        let repo = temp_repo("wrong-receiver");
        write_file(
            &repo,
            "crates/core/src/store/mod.rs",
            "pub struct Store;\nimpl Store {\n    pub fn distinctive_probe(&self) -> u8 { 0 }\n}\n",
        );
        write_file(
            &repo,
            "crates/core/tests/probe.rs",
            "struct Other;\nimpl Other {\n    fn distinctive_probe(&self) -> u8 { 0 }\n}\n\
             #[test]\nfn t() {\n    let other = Other;\n    let _ = other.distinctive_probe();\n}\n",
        );
        let mut cache = SourceCache::new(&repo);
        let err = check(&repo, &mut cache)
            .expect_err("a same-named method on a non-Store receiver must NOT satisfy coverage");
        assert!(err.to_string().contains("distinctive_probe"), "{err:?}");

        fs::remove_dir_all(repo).expect("remove temp repo");
    }

    /// REGRESSION GUARD: tightening receiver matching must not drop any real
    /// Store method's coverage — the committed tree must stay green.
    #[test]
    fn store_pub_fn_coverage_is_green_on_committed_tree() {
        let repo = repo_root().expect("repo root resolves from tools/integrity");
        let mut cache = SourceCache::new(&repo);
        check(&repo, &mut cache)
            .expect("committed Store pub fn coverage must stay green after receiver tightening");
    }

    #[test]
    fn ast_reference_detection_ignores_comments_and_strings() {
        let ast = syn::parse_file(
            r#"
// store.forgotten_method()
const TEXT: &str = "Store::forgotten_method()";
fn unrelated() {}
"#,
        )
        .expect("parse fixture");
        let mut idents = std::collections::BTreeSet::new();
        super::collect_store_idents(&ast, &mut idents);
        assert!(
            !ast_references_store_method(&ast, "forgotten_method", &idents),
            "comments and strings must not satisfy Store pub fn coverage"
        );
    }

    #[test]
    fn ast_reference_detection_accepts_method_and_associated_calls() {
        let method = syn::parse_file(
            r#"
fn exercise(store: &batpak::store::Store) {
    let _ = store.watch_projection::<Projection>("entity");
}
"#,
        )
        .expect("parse method fixture");
        let mut method_idents = std::collections::BTreeSet::new();
        super::collect_store_idents(&method, &mut method_idents);
        assert!(ast_references_store_method(
            &method,
            "watch_projection",
            &method_idents
        ));

        let associated = syn::parse_file(
            r#"
fn exercise(config: batpak::store::StoreConfig) {
    let _ = batpak::store::Store::open(config);
}
"#,
        )
        .expect("parse associated fixture");
        let mut assoc_idents = std::collections::BTreeSet::new();
        super::collect_store_idents(&associated, &mut assoc_idents);
        assert!(ast_references_store_method(
            &associated,
            "open",
            &assoc_idents
        ));
    }
}

//! Three-artifact byte-equality proof harness (Cluster 14).
//!
//! For every positive specimen under `specimens/<kind>/<name>/` this harness
//! drives the pure compiler (`compile_derive` / `compile_attribute` — NOT rustc)
//! and materializes three checked-in artifacts:
//!
//! * `contract.ir`     — the span-free canonical text of the `ContractSnapshot`
//!                       projection of the `ContractIr` (never `Debug` of any syn
//!                       value: spans + syn-version drift must not leak into the
//!                       proof corpus).
//! * `expansion.norm`  — the deterministic `NormalizedExpansion` canonical string.
//! * `expanded.rs`     — the emitted `TokenStream`, stringified.
//!
//! Run mode is env-driven (tests own disk; the compiler `src/` tree does not):
//! `BPK_SNAPSHOT=update` regenerates the artifacts; otherwise each is asserted
//! byte-equal. The harness owns ALL disk I/O for the specimen corpus.

use std::fs;
use std::path::Path;

use macbat_compiler::{
    ContractKind, ContractSnapshot, ExpandOptions, ExpansionArtifact, ExpansionOutcome,
    compile_attribute, compile_derive,
};

/// The positive corpus (Section G). Each row is `(kind, "<kind_dir>/<name>")`.
/// Every surface carries at least one specimen lifted from a real consumer.
const SPECIMENS: &[(ContractKind, &str)] = &[
    // Error — real in-lane (::core/::std only).
    (ContractKind::Error, "error/coordinate_error"),
    (ContractKind::Error, "error/boxed_source"),
    (ContractKind::Error, "error/from_variant"),
    (ContractKind::Error, "error/positional_args"),
    (ContractKind::Error, "error/named_and_specs"),
    (ContractKind::Error, "error/unit_variant"),
    // Event — structural in-lane (::batpak paths).
    (ContractKind::Event, "event/minimal_store"),
    (ContractKind::Event, "event/versioned"),
    (ContractKind::Event, "event/default_version"),
    (ContractKind::Event, "event/max_type_id"),
    // VariantInventory — real in-lane.
    (ContractKind::VariantInventory, "variant_inventory/requirement_kind"),
    (ContractKind::VariantInventory, "variant_inventory/two_variant"),
    (ContractKind::VariantInventory, "variant_inventory/single_variant"),
    // Projection — structural in-lane.
    (ContractKind::Projection, "projection/event_sourced_counter"),
    (ContractKind::Projection, "projection/bounded_state"),
    (ContractKind::Projection, "projection/cached"),
    (ContractKind::Projection, "projection/multi_binding"),
    // Subscription — structural in-lane.
    (ContractKind::Subscription, "subscription/typed_reactor_multi"),
    (ContractKind::Subscription, "subscription/raw_lane"),
    (ContractKind::Subscription, "subscription/two_handlers"),
    // Operation — structural in-lane (attribute macro).
    (ContractKind::Operation, "operation/status_stream"),
    (ContractKind::Operation, "operation/with_effect_row"),
    (ContractKind::Operation, "operation/with_title"),
    (ContractKind::Operation, "operation/with_register"),
];

/// Read a specimen's `decl.rs` and run the matching compiler entry point.
///
/// Derive specimens carry only their helper attributes + item (the `#[derive(..)]`
/// dispatch marker is redundant: the `ContractKind` is passed explicitly). Operation
/// specimens carry `#[operation(..)] fn ..`; the harness splits the attribute meta-list
/// (the `attr` token stream) from the function item exactly as the attribute front door
/// would receive them.
fn compile_specimen(kind: ContractKind, decl_src: &str) -> ExpansionArtifact {
    if kind.is_attribute() {
        let (attr, item) = split_operation_attribute(decl_src);
        compile_attribute(kind, attr, item, ExpandOptions::DEFAULT)
    } else {
        let item: proc_macro2::TokenStream = decl_src
            .parse()
            .expect("specimen decl.rs must lex as a token stream");
        compile_derive(kind, item, ExpandOptions::DEFAULT)
    }
}

/// Split `#[operation(..)] fn f(..)` into the attribute's inner tokens and the
/// function item with that attribute removed.
fn split_operation_attribute(
    decl_src: &str,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let mut item_fn: syn::ItemFn = syn::parse_str(decl_src)
        .expect("operation specimen decl.rs must parse as a free function");
    let attrs = std::mem::take(&mut item_fn.attrs);
    let mut attr_tokens = proc_macro2::TokenStream::new();
    let mut kept: Vec<syn::Attribute> = Vec::new();
    for attribute in attrs {
        if attribute.path().is_ident("operation") {
            if let syn::Meta::List(list) = attribute.meta {
                attr_tokens = list.tokens;
            }
        } else {
            kept.push(attribute);
        }
    }
    item_fn.attrs = kept;
    (attr_tokens, quote::quote!(#item_fn))
}

/// A byte string ready to write/compare: trailing whitespace trimmed, single `\n`.
fn artifact_body(content: &str) -> String {
    format!("{}\n", content.trim_end())
}

/// Update or assert one artifact file for a specimen.
fn sync_artifact(dir: &Path, name: &str, generated: &str, update: bool) {
    let path = dir.join(name);
    if update {
        fs::write(&path, generated).expect("write snapshot artifact");
    } else {
        let expected_raw = fs::read_to_string(&path).expect(
            "read snapshot artifact — run `just macro-lane-snapshot -- --update` (BPK_SNAPSHOT=update) to create it",
        );
        // Tolerate git autocrlf normalization on checked-in goldens.
        let expected = expected_raw.replace("\r\n", "\n");
        assert_eq!(
            generated, expected,
            "specimen artifact `{}` drifted; re-run with BPK_SNAPSHOT=update to regenerate",
            path.display()
        );
    }
}

#[test]
fn specimen_artifacts_are_byte_stable() {
    let update = std::env::var("BPK_SNAPSHOT").ok().as_deref() == Some("update");
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("specimens");

    for &(kind, rel) in SPECIMENS {
        let dir = base.join(rel);
        // Precompute the bail message (a plain `&String`, not an inline call) so the
        // sanctioned `.expect(..)` does not trip `clippy::expect_fun_call`.
        let missing = format!("specimen `{rel}` is missing its decl.rs input");
        let decl_src = fs::read_to_string(dir.join("decl.rs")).expect(&missing);
        let artifact = compile_specimen(kind, &decl_src);

        match artifact.outcome {
            ExpansionOutcome::Emitted {
                contract,
                normalized,
                tokens,
            } => {
                // contract.ir is the ContractSnapshot canonical text — span-free by
                // construction (see the cluster-6 projection), NOT a Debug dump.
                let ir_text = artifact_body(&ContractSnapshot::project(&contract).canonical_text());
                let norm_text = artifact_body(&normalized.canonical);
                let expanded_text = artifact_body(&tokens.to_string());

                sync_artifact(&dir, "contract.ir", &ir_text, update);
                sync_artifact(&dir, "expansion.norm", &norm_text, update);
                sync_artifact(&dir, "expanded.rs", &expanded_text, update);
            }
            ExpansionOutcome::Rejected { diagnostics, .. } => {
                let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.as_str()).collect();
                assert!(
                    diagnostics.is_empty(),
                    "positive specimen `{rel}` was unexpectedly REJECTED: {codes:?}"
                );
            }
        }
    }
}

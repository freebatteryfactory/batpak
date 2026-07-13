//! `expand <kind_dir>/<name>` — a specimen stepper CLI (Cluster 14).
//!
//! Drives the pure compiler over a checked-in specimen with tracing enabled and
//! prints the per-pass trace, metrics, and final outcome. Being an example (a
//! `not(test)` target), it may NOT use `.expect`/`.unwrap`/`panic!`; every
//! fallible step propagates with `?` and all output goes through a locked
//! `writeln!` (the `print_stdout` lint forbids `println!`).
//!
//! Example: `cargo run -p macbat-compiler --example expand -- error/coordinate_error`.

use std::io::Write;
use std::path::Path;

use macbat_compiler::{
    ContractKind, ContractSnapshot, ExpandOptions, ExpansionArtifact, ExpansionOutcome,
    compile_attribute, compile_derive,
};

type StepError = Box<dyn std::error::Error>;

fn main() -> Result<(), StepError> {
    let specimen = std::env::args()
        .nth(1)
        .ok_or("usage: expand <kind_dir>/<name>  (e.g. error/coordinate_error)")?;
    let kind_dir = specimen
        .split_once('/')
        .ok_or("specimen must be `<kind_dir>/<name>`")?
        .0;
    let kind = kind_from_dir(kind_dir)
        .ok_or_else(|| format!("unknown specimen kind directory: `{kind_dir}`"))?;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("specimens")
        .join(&specimen);
    let decl_src = std::fs::read_to_string(dir.join("decl.rs"))?;

    let artifact = compile_specimen(kind, &decl_src)?;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    writeln!(out, "== expand {specimen} ({kind_dir}) ==")?;

    writeln!(out, "-- trace --")?;
    for record in artifact.trace.records() {
        writeln!(out, "  [{}] {}", record.pass, record.summary)?;
    }

    let m = &artifact.metrics;
    writeln!(
        out,
        "-- metrics -- keys={} variants={} items={} tokens={} diagnostics={}",
        m.keys_parsed, m.variants, m.emitted_items, m.tokens, m.diagnostics
    )?;

    match artifact.outcome {
        ExpansionOutcome::Emitted {
            contract,
            normalized,
            tokens,
        } => {
            writeln!(out, "-- outcome: EMITTED --")?;
            writeln!(out, "contract.ir:\n{}", ContractSnapshot::project(&contract).canonical_text())?;
            writeln!(out, "expansion.norm:\n{}", normalized.canonical)?;
            writeln!(out, "expanded.rs:\n{tokens}")?;
        }
        ExpansionOutcome::Rejected {
            partial_contract: _,
            diagnostics,
        } => {
            writeln!(out, "-- outcome: REJECTED --")?;
            for diagnostic in &diagnostics {
                writeln!(out, "  {} {}", diagnostic.code.as_str(), diagnostic.message)?;
            }
        }
    }

    Ok(())
}

/// Map a specimen `<kind_dir>` to its `ContractKind`. The `_` arm is over `&str`
/// (a primitive, not an enum), so it does not trip `wildcard_enum_match_arm`.
fn kind_from_dir(dir: &str) -> Option<ContractKind> {
    match dir {
        "error" => Some(ContractKind::Error),
        "event" => Some(ContractKind::Event),
        "variant_inventory" => Some(ContractKind::VariantInventory),
        "projection" => Some(ContractKind::Projection),
        "subscription" => Some(ContractKind::Subscription),
        "operation" => Some(ContractKind::Operation),
        _ => None,
    }
}

/// Run the matching compiler entry point with tracing on.
fn compile_specimen(kind: ContractKind, decl_src: &str) -> Result<ExpansionArtifact, StepError> {
    let options = ExpandOptions { trace: true };
    if kind.is_attribute() {
        let (attr, item) = split_operation_attribute(decl_src)?;
        Ok(compile_attribute(kind, attr, item, options))
    } else {
        let item: proc_macro2::TokenStream = decl_src.parse()?;
        Ok(compile_derive(kind, item, options))
    }
}

/// Split `#[operation(..)] fn ..` into the attribute meta-list tokens and the
/// function item with that attribute stripped.
fn split_operation_attribute(
    decl_src: &str,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), StepError> {
    let mut item_fn: syn::ItemFn = syn::parse_str(decl_src)?;
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
    Ok((attr_tokens, quote::quote!(#item_fn)))
}

//! The one bridge between the proc-macro entry points and the pure compiler.
//!
//! Each front door forwards its raw tokens here; this module runs the compiler's
//! `compile_derive` / `compile_attribute` driver and folds the resulting
//! [`ExpansionArtifact`] into the `TokenStream` the compiler hands back to
//! rustc by matching its [`ExpansionOutcome`] exhaustively (two arms, no
//! wildcard — the type makes "tokens AND diagnostics" unrepresentable):
//!
//! * `Emitted`  → the emitted tokens verbatim;
//! * `Rejected` → each diagnostic lowered to `compile_error!` tokens carrying
//!   its span; any partial contract is dropped, no tokens reach rustc.

use proc_macro::TokenStream;

use macbat_compiler::{
    compile_attribute, compile_derive, ContractKind, ExpandOptions, ExpansionArtifact,
    ExpansionOutcome,
};

/// Front door for a `#[proc_macro_derive]`: run the derive pipeline and lower.
pub fn run_derive(kind: ContractKind, input: TokenStream) -> TokenStream {
    let artifact = compile_derive(kind, input.into(), ExpandOptions::DEFAULT);
    lower(artifact).into()
}

/// Front door for a `#[proc_macro_attribute]`: run the attribute pipeline and lower.
pub fn run_attribute(kind: ContractKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let artifact = compile_attribute(kind, attr.into(), item.into(), ExpandOptions::DEFAULT);
    lower(artifact).into()
}

/// Fold a finished artifact into rustc-bound tokens: the clean expansion on
/// `Emitted`, the lowered `compile_error!` diagnostics on `Rejected`.
fn lower(artifact: ExpansionArtifact) -> proc_macro2::TokenStream {
    match artifact.outcome {
        ExpansionOutcome::Emitted {
            contract: _,
            normalized: _,
            tokens,
        } => tokens,
        ExpansionOutcome::Rejected {
            partial_contract: _,
            diagnostics,
        } => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.into_compile_error())
            .collect(),
    }
}

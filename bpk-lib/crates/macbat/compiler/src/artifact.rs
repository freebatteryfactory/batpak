//! The pure-expansion result types: [`ExpandOptions`] in, [`ExpansionArtifact`]
//! out. The driver entry points (`compile_derive`/`compile_attribute`) live in
//! `lib.rs` (Cluster 1); this module owns only the value types they exchange.
//!
//! [`NormalizedExpansion`] is defined here and consumed by `normalize` (Pass 7).

use crate::diagnostic::MacroDiagnostic;
use crate::identity::ContractId;
use crate::ir::ContractIr;
use crate::metrics::ExpansionMetrics;
use crate::origin::OriginMap;
use crate::trace::ExpansionTrace;
use proc_macro2::TokenStream;

/// Options controlling one expansion. Trace records are populated only when
/// `trace` is set (the `#[batpak(debug_expand)]` sugar requests it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpandOptions {
    pub trace: bool,
}

impl ExpandOptions {
    pub const DEFAULT: Self = ExpandOptions { trace: false };
}

/// The deterministic, byte-identical canonical rendering of one contract's
/// emitted items (exit gate 20). `canonical` is what the snapshot harness
/// asserts as `expansion.norm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedExpansion {
    pub contract: ContractId,
    pub canonical: String,
}

/// The outcome of an expansion, modelled so contradictory states are
/// unrepresentable: either the passes succeeded and produced tokens, OR they
/// were rejected and carry diagnostics. There is no way to hold both tokens and
/// error diagnostics, so "diagnostics mean the tokens are dropped" is structural
/// rather than a promise. Cluster 13's adapter matches on this enum: `Emitted`
/// forwards `tokens`; `Rejected` lowers `diagnostics` to
/// `compile_error!`/`syn::Error`.
pub enum ExpansionOutcome {
    /// All passes succeeded. Carries the semantic authority (`contract`), the
    /// canonical rendering (`normalized`), and the tokens to emit.
    Emitted {
        contract: ContractIr,
        normalized: NormalizedExpansion,
        tokens: TokenStream,
    },
    /// Some pass failed. `partial_contract` is `Some` iff the IR pass had already
    /// completed before the failure (`None` on a Pass 1/2 failure); no tokens are
    /// carried, by construction.
    Rejected {
        partial_contract: Option<ContractIr>,
        diagnostics: Vec<MacroDiagnostic>,
    },
}

/// The complete result of a pure expansion: an outcome plus the cross-cutting
/// evidence that is meaningful regardless of success (origin/blame map, trace,
/// per-pass metrics).
pub struct ExpansionArtifact {
    pub outcome: ExpansionOutcome,
    pub origin_map: OriginMap,
    pub trace: ExpansionTrace,
    pub metrics: ExpansionMetrics,
}

//! Origin tracking: the blame map from each emitted item back to the user's
//! declaration. Every emitted item carries one `OriginEdge` so a downstream tool
//! (or a diagnostic) can point at the exact declaration field a generated item
//! came from. Kept in a `Vec` for deterministic, source-order iteration — no
//! `HashMap` in this path (purity / determinism law).

use proc_macro2::Span;

use crate::identity::{ContractId, EmittedItemId, FieldPath, LoweringRole};

/// One emitted-item → declaration edge.
#[derive(Clone)]
pub struct OriginEdge {
    pub emitted: EmittedItemId,
    pub contract: ContractId,
    pub lowering: LoweringRole,
    pub declaration_field: Option<FieldPath>,
    pub user_span: Span,
}

/// The ordered set of origin edges for one expansion, assembled in `emit`.
///
/// NOTE (flagged to the coordinator): the master contract §B.12 pins only the
/// private `edges` field and no methods. `emit` (a different module) must populate
/// it and consumers must read it, which a private field with no API forbids, so
/// this adds the minimal functional surface (`push` / `edges` / `is_empty` +
/// `Default`) — no behavior beyond append-and-read.
#[derive(Clone, Default)]
pub struct OriginMap {
    edges: Vec<OriginEdge>,
}

impl OriginMap {
    /// Record one emitted-item → declaration edge.
    pub fn push(&mut self, edge: OriginEdge) {
        self.edges.push(edge);
    }

    /// The ordered origin edges.
    #[must_use]
    pub fn edges(&self) -> &[OriginEdge] {
        &self.edges
    }

    /// Whether no origins have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

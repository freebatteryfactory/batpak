//! The corpus reconciliation epoch (5.5E3g).
//!
//! Answers exactly one question: which whole-corpus reconciliation does a
//! document claim membership in? It is NOT wall time, document modification
//! time, Git commit identity, a source-tree digest, a product/spec/release
//! version, proof freshness, runtime reconciliation posture, or a
//! `ReconciliationRole` — `spec/reconciliation.rs` owns logical/physical
//! execution reconciliation under DEC-075, and corpus reconciliation is
//! documentary authority, never runtime execution state.
//!
//! `last_reconciled` stays: it answers WHEN a document was reviewed and is
//! human-facing diagnostic metadata. The epoch answers WHICH coherent
//! semantic corpus the document belongs to. Neither derives from the other,
//! and a date never substitutes for an epoch.
//!
//! Changing one document does not mint a new epoch. A new epoch is admitted
//! only when a deliberate whole-corpus reconciliation establishes a new
//! coherent baseline and moves `CURRENT_RECONCILIATION_EPOCH`; Git history
//! preserves earlier bytes, so the tree keeps no museum of past epochs.

use crate::guarantees::{ContractId, GuaranteeRef, SeedId};

/// The declared corpus epochs. The inventory may grow; the CURRENT
/// selection moves only by deliberate whole-corpus reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationEpoch {
    /// The currently reconciled clean-room V1 corpus.
    CleanroomV1,
}

/// The one current corpus epoch every eligible document claims.
pub const CURRENT_RECONCILIATION_EPOCH: ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;

impl ReconciliationEpoch {
    pub const ALL: &'static [ReconciliationEpoch] = &[ReconciliationEpoch::CleanroomV1];

    pub const fn spelling(self) -> &'static str {
        match self {
            ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",
        }
    }

    /// The Constitution owns document-status doctrine, and it must author
    /// both the type name and the canonical spelling.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),
        }
    }

    /// The standing seed row whose statement names ReconciliationEpoch.
    pub const fn admission_basis(self) -> GuaranteeRef {
        match self {
            ReconciliationEpoch::CleanroomV1 => {
                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))
            }
        }
    }
}

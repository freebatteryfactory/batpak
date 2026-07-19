//! The corpus reconciliation epoch (5.5E3g).
//!
//! Answers exactly one question: which whole-corpus reconciliation does a
//! document claim membership in? It is NOT wall time, document modification
//! time, Git commit identity, a source-tree digest, a product/spec/release
//! version, proof freshness, runtime reconciliation posture, or a
//! `ReconciliationRole` — `spec/reconciliation/` owns logical/physical
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

mod types;

pub use types::{CURRENT_RECONCILIATION_EPOCH, ReconciliationEpoch};

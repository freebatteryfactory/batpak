use super::types::{ReconciliationCoordinate, ReconciliationRole};

/// The five coordinates of the dual-axis model, one per role.
pub const RECONCILIATION_COORDINATES: &[ReconciliationCoordinate] = &[
    ReconciliationCoordinate {
        role: ReconciliationRole::DurableOrderWitness,
        carriers: &["GlobalSequence", "CommitPoint"],
        law: "one writer establishes exact durable order; nothing else does",
    },
    ReconciliationCoordinate {
        role: ReconciliationRole::ChronologyWitness,
        carriers: &["Hlc", "ObservedWallTime"],
        law: "chronology, causal evidence, lag, and temporal pruning only; \
              never promoted into order, commit, retry, or deadline authority",
    },
    ReconciliationCoordinate {
        role: ReconciliationRole::LogicalIdentity,
        carriers: &["TurnId", "LogicalOperationId"],
        law: "binds the frozen input cuts and remains stable across replay",
    },
    ReconciliationCoordinate {
        role: ReconciliationRole::PhysicalIdentity,
        carriers: &["AttemptId"],
        law: "fresh for every physical execution; a response binds to exactly \
              one attempt and never crosses attempts",
    },
    ReconciliationCoordinate {
        role: ReconciliationRole::BalancingEvidence,
        carriers: &["Receipt", "ReconciliationPosture"],
        law: "joins the logical and physical entries without pretending they \
              are identical; a discrepancy becomes a named posture, never an \
              edit to history",
    },
];

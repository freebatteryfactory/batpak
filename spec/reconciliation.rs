//! The DEC-075 composition: single-writer dual-axis reconciliation.
//!
//! Every coordinate this file binds is owned elsewhere — durable order by the
//! journal law (LEG-001/067, docs/05), chronology by the time contract
//! (docs/16, DEC-061), turn and attempt identity by the runtime and Bvisor
//! contracts (docs/08/09), receipts and reconciliation posture by docs/14 —
//! and this file redeclares none of them. What it owns is the COMPOSITION:
//! which coordinates exist, what role each plays in reconciling the logical
//! and physical books, which axes a receipt preserves without inference, and
//! which signals may decide a retry. The lesson this composition encodes is
//! the old `HlcPoint` autopsy (issue #208): one type may carry fields
//! efficiently, but one NAME may never own multiple ordering authorities.
//!
//! `docs/02_SYSTEM_MODEL.md` owns the end-to-end prose law; its coordinate
//! and retry tables are generated projections of the constants below.

/// The role one reconciliation coordinate plays. Closed: the dual-axis model
/// has exactly these planes, and a coordinate claiming two roles is the
/// overloaded-noun defect this decision exists to prevent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationRole {
    /// Exact single-writer durable order: the logical ledger's spine.
    DurableOrderWitness,
    /// Chronology, causal evidence, lag, temporal pruning. Never commit,
    /// retry, deadline, checkpoint, or cursor authority.
    ChronologyWitness,
    /// Stable logical identity over a frozen source cut; identical on replay.
    LogicalIdentity,
    /// Fresh identity for each physical execution; never reused on retry.
    PhysicalIdentity,
    /// The balancing record joining the books without collapsing them.
    BalancingEvidence,
}

/// One reconciliation coordinate: a role and the identities that carry it.
/// Carriers reference the vocabulary owned by docs/16, docs/08/09, and
/// docs/14 by name; they are retyped against the identity inventory when it
/// lands (5.5E3) and are never a second declaration of those identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconciliationCoordinate {
    pub role: ReconciliationRole,
    pub carriers: &'static [&'static str],
    pub law: &'static str,
}

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

/// The double-entry axes an effectful or durable operation preserves where
/// applicable. No axis is inferred from another, and a later reconciliation
/// appends or references new evidence — it never rewrites the original event,
/// attempt observation, or receipt into a cleaner story.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoubleEntryAxis {
    LogicalOperationIdentity,
    TurnAndFrozenCuts,
    ChronologyEvidence,
    PhysicalAttemptIdentity,
    EffectIntent,
    AttemptObservation,
    CommitKnowledge,
    ReceiptCompleteness,
    ReconciliationPosture,
    CheckpointDisposition,
}

/// Every axis, in declaration order. Completeness is enforced by seedcheck's
/// exhaustive classification: a new axis cannot be added without appearing
/// here, and none may appear twice.
pub const DOUBLE_ENTRY_AXES: &[DoubleEntryAxis] = &[
    DoubleEntryAxis::LogicalOperationIdentity,
    DoubleEntryAxis::TurnAndFrozenCuts,
    DoubleEntryAxis::ChronologyEvidence,
    DoubleEntryAxis::PhysicalAttemptIdentity,
    DoubleEntryAxis::EffectIntent,
    DoubleEntryAxis::AttemptObservation,
    DoubleEntryAxis::CommitKnowledge,
    DoubleEntryAxis::ReceiptCompleteness,
    DoubleEntryAxis::ReconciliationPosture,
    DoubleEntryAxis::CheckpointDisposition,
];

/// Every signal a retry decision might plausibly consult, in one vocabulary,
/// with a typed classification — the `GateId` pattern. Retry, resume,
/// compensation, or terminal refusal is selected only from admissible
/// signals; an inadmissible signal is deliberately expressible so the refusal
/// has something to refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetrySignal {
    DeclaredRecoveryClass,
    DurableIntent,
    AttemptEvidence,
    CommitKnowledge,
    ReceiptCompleteness,
    OverallMonotonicDeadline,
    RuntimeRestartAuthorization,
    /// Wall time passing proves nothing about commit or completion.
    ElapsedWallTime,
    /// A dead process is not evidence its effect failed to commit.
    ProcessDeath,
    /// A lost acknowledgement requires reconciliation, not optimism.
    MissingAcknowledgement,
    /// Losing an in-memory waiter loses no semantic identity: durable intent,
    /// attempt evidence, commit knowledge, receipts, and checkpoints are the
    /// recovery surface.
    MissingInMemoryWaiter,
}

impl RetrySignal {
    /// Whether this signal may participate in selecting retry, resume,
    /// compensation, or refusal. Exhaustive: a new signal must be classified
    /// here, not defaulted.
    pub const fn admissible(self) -> bool {
        match self {
            RetrySignal::DeclaredRecoveryClass
            | RetrySignal::DurableIntent
            | RetrySignal::AttemptEvidence
            | RetrySignal::CommitKnowledge
            | RetrySignal::ReceiptCompleteness
            | RetrySignal::OverallMonotonicDeadline
            | RetrySignal::RuntimeRestartAuthorization => true,
            RetrySignal::ElapsedWallTime
            | RetrySignal::ProcessDeath
            | RetrySignal::MissingAcknowledgement
            | RetrySignal::MissingInMemoryWaiter => false,
        }
    }
}

/// Every retry signal, in declaration order, for projection and completeness
/// checking.
pub const RETRY_SIGNALS: &[RetrySignal] = &[
    RetrySignal::DeclaredRecoveryClass,
    RetrySignal::DurableIntent,
    RetrySignal::AttemptEvidence,
    RetrySignal::CommitKnowledge,
    RetrySignal::ReceiptCompleteness,
    RetrySignal::OverallMonotonicDeadline,
    RetrySignal::RuntimeRestartAuthorization,
    RetrySignal::ElapsedWallTime,
    RetrySignal::ProcessDeath,
    RetrySignal::MissingAcknowledgement,
    RetrySignal::MissingInMemoryWaiter,
];

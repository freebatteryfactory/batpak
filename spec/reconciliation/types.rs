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

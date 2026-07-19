use crate::gates::GateId;

/// Whether a retained obligation still gates clean-room implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationStatus {
    /// Still gating: a clean successor must discharge it at its gate.
    Active,
    /// Discharged; retained as historical evidence and no longer gating.
    Closed,
}

/// The cross-version / cross-format compatibility burden an obligation carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompatibilityDisposition {
    /// Pure semantic law with no durable-byte or wire compatibility burden.
    None,
    /// Must keep decoding older accepted bytes through explicit migration.
    ReadOlderAccepted,
    /// Every persisted format/version pair resolves to one canonical open or typed refusal.
    CanonicalOrTypedRefusal,
    /// Canonical identity, bytes, or goldens are frozen and versioned.
    FrozenCanonicalIdentity,
}

/// The condition under which the obligation may retire from active gating.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeletionCondition {
    /// Permanent constitutional law; keeps gating regardless of implementation progress.
    Never,
    /// Dischargeable once the named clean successor closes at its gate.
    OnSuccessorGateClosure,
    /// Additionally bound: cannot retire until its declared compatibility window closes.
    OnCompatibilityWindowExpiry,
}

/// The witness ROUTE for one obligation (5.5E4d): a posture, not proof
/// evidence. `CanonicalProofRows` means the required witness identities are
/// obtained from active `spec/proof.rs` rows whose guarantee is this exact
/// LEG identity - no free-text plan substitutes for those rows. `Planned`
/// means no canonical active proof row has yet been admitted, and the
/// nonempty text states the required future evidence route; it is never
/// counted as an active proof row and is not release evidence. Moving from
/// Planned to CanonicalProofRows is atomic with real row admission, docs/24
/// meaning, typed bindings, projection targets, and hostile fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyWitnessRequirement {
    CanonicalProofRows,
    Planned(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyObligation {
    pub id: &'static str,
    pub law: &'static str,
    /// The legacy evidence pointer docs/21 previously authored (5.5E4d).
    pub legacy_evidence: &'static str,
    pub clean_owner: &'static str,
    /// The mechanism disposition docs/21 previously authored (5.5E4d).
    pub mechanism_disposition: &'static str,
    /// The one witness route for this obligation.
    pub witness_requirement: LegacyWitnessRequirement,
    pub gates: &'static [GateId],
    pub compatibility_disposition: CompatibilityDisposition,
    pub deletion_condition: DeletionCondition,
    pub active_or_closed_status: ObligationStatus,
}

/// The one lawful LEG lifetime derivation (5.5C1a), executed in Rust since
/// the 5.5E2 bake. A LEG row names a clean owner and gates but no typed
/// successor GuaranteeRef, so an active gate-closed obligation is `UntilGate`
/// (`UntilSuccessor` requires a resolved typed successor, which the LEG
/// schema does not provide); a closed row is retained evidence; a
/// compatibility-bound row cannot retire before its window.
pub const fn legacy_lifetime(
    status: ObligationStatus,
    deletion: DeletionCondition,
) -> crate::guarantees::GuaranteeLifetime {
    use crate::guarantees::GuaranteeLifetime;
    match (status, deletion) {
        (ObligationStatus::Closed, _) => GuaranteeLifetime::ClosedEvidence,
        (ObligationStatus::Active, DeletionCondition::OnCompatibilityWindowExpiry) => {
            GuaranteeLifetime::UntilCompatibilityExpiry
        }
        (ObligationStatus::Active, DeletionCondition::Never)
        | (ObligationStatus::Active, DeletionCondition::OnSuccessorGateClosure) => {
            GuaranteeLifetime::UntilGate
        }
    }
}

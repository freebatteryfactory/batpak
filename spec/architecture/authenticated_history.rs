use crate::gates::GateId;

// --- Authenticated history (DEC-071) ----------------------------------------
// A DIFFERENT concept from the build-target `QualificationProfile` above, which
// says which package compiles for which target. These facts say what a store's
// history verification actually proves. The two families deliberately share no
// type, field name, parser, projection, or audit rule; nothing here is named a
// bare `Profile`, `Policy`, or `Disposition`.
//
// This is a contract, not an implementation: bootstrap performs no signature,
// accumulator, witness, freshness, or cryptographic verification of any kind.

/// What a caller explicitly selects. A stronger claim requires a stronger
/// profile; an invalid pairing is refused, never normalized into a neighbour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticatedHistoryProfile {
    /// Local coherence only. No authorship and no freshness claim.
    InternalConsistency,
    /// Authenticated authorship and integrity within the signed history.
    SignedHistory,
    /// Signed history plus an independent monotonic witness.
    ExternallyAnchoredHistory,
}

/// Whether an independent monotonic witness participates. Typed, and
/// constrained by profile: `Required` exists only with
/// `ExternallyAnchoredHistory`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessPolicy {
    None,
    Optional,
    Required,
}

/// The outcome of witness evaluation. These stay distinct on purpose: collapsing
/// them into `Valid`/`Invalid` is what lets a restored older history read as
/// healthy. An absent optional witness and a supplied broken one are different
/// facts and must never render identically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessDisposition {
    /// The policy does not use witnesses (`WitnessPolicy::None`).
    NotApplicable,
    /// The policy permits or requires a witness and none was supplied.
    NotProvided,
    /// Supplied and verified for this lineage, generation, and accumulator.
    Verified,
    /// Supplied and authentic, but older than the observed history.
    Stale,
    /// Supplied and contradicts the observed history.
    Conflicting,
    /// Supplied but could not be evaluated (unreachable or unparseable).
    Unverifiable,
    /// Supplied and failed cryptographic verification.
    CryptographicallyInvalid,
    /// Supplied but bound to a different store lineage.
    LineageMismatch,
    /// Supplied but bound to a different generation.
    GenerationMismatch,
    /// Supplied but bound to a different history accumulator.
    AccumulatorMismatch,
}

/// Whether the selected generation is internally coherent: local authoritative
/// commitments and authority-generation relationships verify, and derived
/// material is not treated as authority. Every admitted success bundle carries
/// this; there is no success without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityClaim {
    InternalConsistencyVerified,
}

/// Whether an authenticated signer produced this history. Independent of
/// whether the history is the newest one: a restored older generation can be
/// perfectly authentic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticityClaim {
    NotClaimed,
    SignedHistoryVerified,
}

/// Whether this generation is the newest history ever acknowledged. Only an
/// independent monotonic witness carries this claim.
///
/// `WitnessedGenerationVerified` is scoped to the exact store lineage,
/// generation, history or accumulator commitment, witness identity, witness
/// monotonicity guarantee, and trust assumptions that verified. It is never a
/// universal newest-history proof, and must not be renamed to imply one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshnessClaim {
    NotClaimed,
    WitnessedGenerationVerified,
}

/// Whether restoring an older valid generation is detectable, and within what
/// scope. Never universal rollback prevention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackResistanceClaim {
    /// No freshness evidence exists. Stated explicitly rather than omitted.
    Unavailable,
    /// Detectable only within the verified witness's own scope and assumptions.
    ScopedToVerifiedWitness,
}

/// Everything a SUCCESSFUL authenticated-history verification claims, stated on
/// four independent axes.
///
/// This replaces a single mutually exclusive posture enum, which could not say
/// two true things at once: an InternalConsistency success must assert both
/// `InternalConsistencyVerified` and `RollbackResistanceUnavailable`, and one
/// variant cannot. A consumer reads every axis directly and never reconstructs
/// an omitted claim from the profile name, the witness policy, the witness
/// disposition, or another axis.
///
/// This is a SUCCESS bundle only. It is not an arbitrary verification state, an
/// error category, a refusal outcome, or an incomplete attempt. A refusal is
/// never forced into one; see `REFUSAL_PARTIAL_CLAIM_LAW`.
///
/// The axes are independent, not an ordered ladder. There is no
/// `SecurityPosture`, `VerificationLevel`, `AssuranceLevel`, or `SecurityLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthenticatedHistoryClaims {
    pub integrity: IntegrityClaim,
    pub authenticity: AuthenticityClaim,
    pub freshness: FreshnessClaim,
    pub rollback_resistance: RollbackResistanceClaim,
}

/// Coherent, local, unsigned. Proves the generation hangs together and nothing
/// more: not who wrote it, and not that it is current.
pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::NotClaimed,
    freshness: FreshnessClaim::NotClaimed,
    rollback_resistance: RollbackResistanceClaim::Unavailable,
};

/// Authentic authorship, no external anchor. A restored older validly signed
/// history satisfies exactly this bundle, which is why freshness stays
/// `NotClaimed`.
pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::SignedHistoryVerified,
    freshness: FreshnessClaim::NotClaimed,
    rollback_resistance: RollbackResistanceClaim::Unavailable,
};

/// Verified against an independent monotonic witness, scoped to exactly that
/// witness's lineage, generation, accumulator, guarantee, and trust
/// assumptions.
pub const WITNESSED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::SignedHistoryVerified,
    freshness: FreshnessClaim::WitnessedGenerationVerified,
    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,
};

// AuthenticatedHistoryProfileSpec { profile, permitted_witness_policies,
// requires_local_commitment_verification, requires_signed_history_verification,
// requires_independent_witness_verification, implementation_gates,
// release_qualification_gates, unanchored_success_claims,
// verified_witness_success_claims } and its three-row
// AUTHENTICATED_HISTORY_PROFILES table stood here from 5.5C2c until the 5.5E2
// bake. Every field was a pure function of the profile variant — the table
// restated per row what the enum determines, and three checkers (seedcheck,
// audit, selftest) then policed the copies back into agreement: duplicate-row
// checks, row-count checks, an always-true bool, Required-outside-EAH scans.
// A table whose every cell is derivable is not a fact family; it is a ladder
// of invitations to drift. Deleted rather than kept: the authorship moved
// into the const fns below, where an invalid pairing has no row to live in.

impl AuthenticatedHistoryProfile {
    /// The three profiles, in ascending claim order (for iteration, not as a
    /// ladder type: the CLAIMS stay four independent axes).
    pub const ALL: &'static [AuthenticatedHistoryProfile] = &[
        AuthenticatedHistoryProfile::InternalConsistency,
        AuthenticatedHistoryProfile::SignedHistory,
        AuthenticatedHistoryProfile::ExternallyAnchoredHistory,
    ];

    /// The frozen valid pairings. Anything absent here is refused, never
    /// normalized into a neighbour: `SignedHistory + Required` is not
    /// silently upgraded to `ExternallyAnchoredHistory` — the caller selects
    /// the stronger profile. `Required` exists only here, in the
    /// `ExternallyAnchoredHistory` arm.
    pub const fn permitted_witness_policies(self) -> &'static [WitnessPolicy] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => &[WitnessPolicy::None],
            AuthenticatedHistoryProfile::SignedHistory => {
                &[WitnessPolicy::None, WitnessPolicy::Optional]
            }
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Required],
        }
    }

    /// Local authoritative commitments and authority generations verify under
    /// EVERY profile. This is a law of the family, not a per-profile choice:
    /// there is no profile that skips local coherence.
    pub const fn requires_local_commitment_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// Signed seals and a signed whole-history commitment verify.
    pub const fn requires_signed_history_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => false,
            AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// An independent monotonic witness verifies.
    pub const fn requires_independent_witness_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory => false,
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// Where the profile's mechanism is implemented. One gate for the family:
    /// authenticated history is storage-core work.
    pub const fn implementation_gates(self) -> &'static [GateId] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[GateId::G2],
        }
    }

    /// Where the profile's release qualification is scheduled.
    pub const fn release_qualification_gates(self) -> &'static [GateId] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[GateId::G9],
        }
    }

    /// The success bundle when no verified external anchor is present.
    ///
    /// `None` means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED — it does not
    /// mean unknown, not configured, posture unavailable, or fallback
    /// success. An absent or invalid required witness refuses; it never falls
    /// back to a weaker success.
    pub const fn unanchored_success_claims(self) -> Option<AuthenticatedHistoryClaims> {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => {
                Some(INTERNAL_CONSISTENCY_SUCCESS)
            }
            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => None,
        }
    }

    /// The success bundle when an independent witness verifies. `None` means
    /// the profile admits no witness at all.
    pub const fn verified_witness_success_claims(self) -> Option<AuthenticatedHistoryClaims> {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => None,
            AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => Some(WITNESSED_SUCCESS),
        }
    }
}

/// A refusal is not a weaker success.
///
/// The normative result model distinguishes:
///
/// ```text
/// Success {
///     claims: AuthenticatedHistoryClaims,   // complete, all four axes
///     witness_disposition, profile, policy, identity, proof fields
/// }
///
/// Refusal {
///     final_claims: None,                   // no successful claim bundle
///     partial_verified_claims: Option<AuthenticatedHistoryClaims>,
///     witness_disposition or failure reason, profile, policy,
///     identity, proof fields
/// }
/// ```
///
/// `partial_verified_claims` preserves ONLY sub-results that independently
/// succeeded, for diagnosis. It may carry local integrity or signed-history
/// evidence. It may never carry `FreshnessClaim::WitnessedGenerationVerified`
/// or `RollbackResistanceClaim::ScopedToVerifiedWitness` when witness
/// verification failed, and its presence never converts a refusal into success.
pub const REFUSAL_PARTIAL_CLAIM_LAW: &str =
    "a refusal carries no final claim bundle; partial_verified_claims holds only \
     independently verified local evidence and never freshness or scoped rollback \
     resistance after witness failure";

/// Every witness disposition that must fail closed under
/// `WitnessPolicy::Required`. `NotProvided` is included: a required witness that
/// was never supplied is a refusal, not a silent success.
pub const REQUIRED_WITNESS_FAILURE_SET: &[WitnessDisposition] = &[
    WitnessDisposition::NotProvided,
    WitnessDisposition::Stale,
    WitnessDisposition::Conflicting,
    WitnessDisposition::Unverifiable,
    WitnessDisposition::CryptographicallyInvalid,
    WitnessDisposition::LineageMismatch,
    WitnessDisposition::GenerationMismatch,
    WitnessDisposition::AccumulatorMismatch,
];

/// A supplied optional witness that failed. Optional to supply; once supplied,
/// mandatory to validate. None of these may degrade into `NotProvided` or emit
/// a success receipt that discards the failure.
pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[
    WitnessDisposition::Stale,
    WitnessDisposition::Conflicting,
    WitnessDisposition::Unverifiable,
    WitnessDisposition::CryptographicallyInvalid,
    WitnessDisposition::LineageMismatch,
    WitnessDisposition::GenerationMismatch,
    WitnessDisposition::AccumulatorMismatch,
];

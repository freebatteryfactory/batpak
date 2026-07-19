// --- TestPak proof policy and mutation (DEC-015, DEC-074) -------------------
// Typed facts only. Bootstrap compiles no mutant, activates no slot, runs no
// nextest, invokes no rustc, kills nothing, proves no equivalence, promotes
// nothing, and classifies no real diff semantically. It proves the contract.

/// How a proof-policy change moves the boundary. An unclassified change is
/// refused (DEC-074).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofPolicyChangeClass {
    /// Adds or raises proof obligations, increases hostile coverage, reduces
    /// unjustified waiver scope, or improves freshness or denominator
    /// visibility, without silently removing a proof unit or terminal
    /// disposition.
    Strengthening,
    /// Changes representation while preserving denominator, selection meaning,
    /// terminal dispositions, activation, freshness, blocking posture,
    /// promotion requirements, and documentary meaning. Requires parity
    /// evidence: "refactor" is not automatically Neutral.
    Neutral,
    /// Removes a proof unit, lowers a threshold, widens a waiver, makes a
    /// required proof optional, reduces hostile coverage or activation
    /// requirements, weakens freshness, turns a blocking failure into a
    /// warning, shrinks selected tests without equivalent qualification, lets a
    /// candidate promote on less evidence, drops units from the denominator, or
    /// retires an independent oracle. An apparently stronger policy that
    /// narrows the tested domain is also Weakening.
    Weakening,
}

/// The surfaces the anti-weakening gate owns. A change declaration names the
/// affected surfaces explicitly; the gate never infers one from a filename or
/// keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofPolicySurface {
    MutationThreshold,
    WaiverLogic,
    EquivalentMutantRule,
    HostileFixture,
    CandidatePromotion,
    ProofReceiptSchema,
    ProofFreshness,
    ReleaseBlocking,
    TestSelection,
    AssuranceRequirement,
    GateQualification,
}

//! Proof-unit vocabulary (5.5E1 ruling; extended by the 5.5E2 bake).
//!
//! docs/12 owns the audited-denominator doctrine in prose; this file is the
//! typed authority for the closed vocabularies that doctrine depends on. The
//! sibling eight-state `MutationResult` earned a typed owner long before this
//! file existed; the general proof denominator now gets the same treatment.

/// What a proof unit concluded ABOUT THE LAW. Semantic terminals only: what
/// happened to an execution attempt (not executed, timed out, infrastructure
/// failed) is the qualification-receipt algebra's vocabulary, and the two
/// axes never collapse into one enum — a row whose attempt died has NO
/// semantic terminal and stays honestly unterminated in the denominator. A
/// subject violating its own deadline law is `Failed` with that law cited; a
/// harness timeout is a receipt-stage fact carrying no semantic verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofUnitTerminal {
    /// The obligation held under the executed proof.
    Passed,
    /// The obligation was violated; the evidence names the boundary.
    Failed,
    /// The proof refused to run its subject (illegal input, unmet
    /// precondition it exists to enforce). A refusal is a verdict.
    Refused,
    /// The platform or profile cannot express the obligation; stated, never
    /// silently skipped.
    Unsupported,
    /// An authorized policy skipped the proof; the receipt proving that
    /// authority accompanies the terminal.
    SkippedWithAuthority,
    /// The proof's freshness bound lapsed; its verdict no longer counts.
    Expired,
    /// A successor proof row owns the obligation now.
    Superseded,
}

/// Every terminal, in declaration order. Completeness is enforced by
/// seedcheck's exhaustive classification.
pub const PROOF_UNIT_TERMINALS: &[ProofUnitTerminal] = &[
    ProofUnitTerminal::Passed,
    ProofUnitTerminal::Failed,
    ProofUnitTerminal::Refused,
    ProofUnitTerminal::Unsupported,
    ProofUnitTerminal::SkippedWithAuthority,
    ProofUnitTerminal::Expired,
    ProofUnitTerminal::Superseded,
];

impl ProofUnitTerminal {
    /// Whether this terminal may count toward the green side of the audited
    /// denominator. Exhaustive: a new terminal must be classified here, not
    /// defaulted — an unclassified proof unit cannot silently count green
    /// (SEED-AUDITED-DENOMINATOR).
    pub const fn counts_green(self) -> bool {
        match self {
            ProofUnitTerminal::Passed => true,
            ProofUnitTerminal::Failed
            | ProofUnitTerminal::Refused
            | ProofUnitTerminal::Unsupported
            | ProofUnitTerminal::SkippedWithAuthority
            | ProofUnitTerminal::Expired
            | ProofUnitTerminal::Superseded => false,
        }
    }
}

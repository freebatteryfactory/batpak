use crate::guarantees::{ContractId, GuaranteeRef};
use crate::verification::{VerificationClaimKind, VerificationRequirement};

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
    /// Whether this terminal is the positive SEMANTIC terminal. Renamed from
    /// `counts_green` (5.5F2, DEC-077/docs/38): a positive semantic terminal
    /// is one input to requirement qualification among eleven, never the
    /// verdict — no gate or denominator may use it alone, and a sampled or
    /// runtime-observed row can never launder into denominator strength
    /// through it. Exhaustive: a new terminal must be classified here, not
    /// defaulted — an unclassified proof unit cannot silently count positive
    /// (SEED-AUDITED-DENOMINATOR).
    pub const fn is_positive_semantic_terminal(self) -> bool {
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

/// A proof unit's stable identity (5.5E2). Sealed like the per-family
/// guarantee ids: the spec's own registry authors identities, a consumer
/// reads one through `raw()`.
///
/// Execution completeness and inventory retention are DIFFERENT AXES. A
/// libtest count proves every currently declared test executed and no filter
/// hid one; it cannot prove a required identity was not deleted, because
/// deleting a test shrinks the expected count and the executed count
/// together. Typed identity plus the migration registry below is the
/// retention half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowId(pub(crate) &'static str);

impl ProofRowId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}

/// The lifecycle of a proof identity. Retirement PRESERVES successor
/// identity: a rename names its one successor, a split names every
/// successor, and the original claim must be carried by the conjunction of
/// the successors — retirement is supersession with a forwarding address,
/// never deletion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofRowState {
    /// The identity is authoritative in the proof inventory.
    /// An active row carries the exact LEG or DEC guarantee it pressures
    /// and the domain contracts whose documents consume its identity. It
    /// carries no meaning: docs/24 permanently owns semantic summary,
    /// expected observation, and terminal disposition. Automatic consumers
    /// (BP-GAUNTLET-1, BP-LEGACY-OBLIGATIONS-1) are never listed here.
    Active {
        guarantee: GuaranteeRef,
        projection_contracts: &'static [ContractId],
        /// The one primary property this row challenges (docs/38 section 5).
        claim: VerificationClaimKind,
        /// The authored verification plan: the exact axes evidence must
        /// match to qualify this row. Nonempty, duplicate-free, admissible —
        /// executed seedcheck law, and never a default rubber stamp.
        verification: &'static [VerificationRequirement],
    },
    /// The identity is retired; the successors own its obligation now. An
    /// active witness may reach a retired row only by explicitly following
    /// this mapping.
    Retired { successors: &'static [ProofRowId] },
}

/// One entry in the proof-identity catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowRecord {
    pub id: ProofRowId,
    pub state: ProofRowState,
}

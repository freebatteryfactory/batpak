use crate::gates::GateId;
use crate::guarantees::{ContractId, DecisionId};

/// The three frozen mutation lanes (DEC-015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationLane {
    /// Mutates BatPak-owned semantic structures. No per-candidate Rust
    /// compile; the reference interpreter executes the mutant and an
    /// independent evidence route judges it.
    SemanticIr,
    /// One test-profile shard holds many candidate slots; one stable
    /// mutation identity selects one slot per isolated process. Slots never
    /// enter an ordinary production artifact.
    SelectableCompiled,
    /// Implementation-sensitive material whose truth depends on real
    /// compiler and platform behavior. Runs under real rustc semantics.
    CompilerBacked,
}

/// Every planned mutation reaches exactly one of these. They stay distinct
/// because collapsing them into pass/fail is how a denominator quietly
/// shrinks: an unbuildable candidate is not a detected fault, and a timeout
/// is not a kill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationResult {
    /// Activated, baseline and selected tests qualified, and one or more
    /// qualified witnesses reject the mutant.
    Killed,
    /// Activated, selected tests qualified, and no selected witness
    /// rejected it.
    Survived,
    /// Not observed as reached or selected. This is never Survived.
    NotActivated,
    /// Admission or policy refused the candidate or run. This is never
    /// Killed.
    Refused,
    /// Could not produce the required executable or admitted semantic
    /// artifact. A compiler error from an invalid mutation is not
    /// detection. Never Killed.
    Unbuildable,
    /// Exceeded its declared deadline. Neither Killed nor Survived.
    TimedOut,
    /// The runner, compiler, environment, or proof mechanism failed.
    /// Neither Killed nor Survived.
    InfrastructureFailure,
    /// May be equivalent under the named semantic scope. A terminal
    /// denominator classification pending an independent equivalence
    /// witness — neither killed nor survived, and never silently dropped.
    EquivalentCandidate,
}

impl MutationLane {
    pub const ALL: &'static [MutationLane] = &[
        MutationLane::SemanticIr,
        MutationLane::SelectableCompiled,
        MutationLane::CompilerBacked,
    ];

    pub const fn spelling(self) -> &'static str {
        match self {
            MutationLane::SemanticIr => "SemanticIr",
            MutationLane::SelectableCompiled => "SelectableCompiled",
            MutationLane::CompilerBacked => "CompilerBacked",
        }
    }

    /// docs/12 owns mutation-lane semantics.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            MutationLane::SemanticIr
            | MutationLane::SelectableCompiled
            | MutationLane::CompilerBacked => ContractId("BP-TESTPAK-1"),
        }
    }

    /// The standing decision whose forward-policy fields name every
    /// admitted lane. DEC-015's exact-three ruling is the admission
    /// boundary; no loose length check substitutes for it.
    pub const fn admission_basis(self) -> DecisionId {
        match self {
            MutationLane::SemanticIr
            | MutationLane::SelectableCompiled
            | MutationLane::CompilerBacked => DecisionId("DEC-015"),
        }
    }

    /// Whether a candidate requires its own Rust compilation.
    pub const fn requires_per_candidate_rust_compile(self) -> bool {
        match self {
            MutationLane::SemanticIr => false,
            MutationLane::SelectableCompiled => false,
            MutationLane::CompilerBacked => true,
        }
    }

    /// Whether execution requires real rustc semantics. The reference
    /// interpreter lane must NOT claim this; a homegrown evaluator never
    /// equals rustc.
    pub const fn requires_real_rustc_semantics(self) -> bool {
        match self {
            MutationLane::SemanticIr => false,
            MutationLane::SelectableCompiled => true,
            MutationLane::CompilerBacked => true,
        }
    }

    /// Whether a candidate must carry activation evidence before a
    /// Survived or Killed verdict is admissible. A green test with a
    /// dormant mutant is not evidence.
    pub const fn requires_activation_evidence(self) -> bool {
        match self {
            MutationLane::SemanticIr => true,
            MutationLane::SelectableCompiled => true,
            MutationLane::CompilerBacked => true,
        }
    }

    /// Whether mutation slots may exist in an ordinary production artifact.
    pub const fn permits_production_profile_slots(self) -> bool {
        match self {
            MutationLane::SemanticIr => false,
            MutationLane::SelectableCompiled => false,
            MutationLane::CompilerBacked => false,
        }
    }

    /// Whether an independent (non-self-calling) evidence route is required.
    pub const fn requires_independent_evidence_route(self) -> bool {
        match self {
            MutationLane::SemanticIr => true,
            MutationLane::SelectableCompiled => true,
            MutationLane::CompilerBacked => true,
        }
    }

    pub const fn gates(self) -> &'static [GateId] {
        match self {
            MutationLane::SemanticIr => &[GateId::G3],
            MutationLane::SelectableCompiled => &[GateId::G3],
            MutationLane::CompilerBacked => &[GateId::G3],
        }
    }
}

impl MutationResult {
    pub const ALL: &'static [MutationResult] = &[
        MutationResult::Killed,
        MutationResult::Survived,
        MutationResult::NotActivated,
        MutationResult::Refused,
        MutationResult::Unbuildable,
        MutationResult::TimedOut,
        MutationResult::InfrastructureFailure,
        MutationResult::EquivalentCandidate,
    ];

    pub const fn spelling(self) -> &'static str {
        match self {
            MutationResult::Killed => "Killed",
            MutationResult::Survived => "Survived",
            MutationResult::NotActivated => "NotActivated",
            MutationResult::Refused => "Refused",
            MutationResult::Unbuildable => "Unbuildable",
            MutationResult::TimedOut => "TimedOut",
            MutationResult::InfrastructureFailure => "InfrastructureFailure",
            MutationResult::EquivalentCandidate => "EquivalentCandidate",
        }
    }

    /// docs/12 owns result-classification semantics.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            MutationResult::Killed
            | MutationResult::Survived
            | MutationResult::NotActivated
            | MutationResult::Refused
            | MutationResult::Unbuildable
            | MutationResult::TimedOut
            | MutationResult::InfrastructureFailure
            | MutationResult::EquivalentCandidate => ContractId("BP-TESTPAK-1"),
        }
    }

    /// Only Killed. An invalid mutation that fails to compile proves
    /// nothing about the test suite.
    pub const fn counts_as_kill(self) -> bool {
        match self {
            MutationResult::Killed => true,
            MutationResult::Survived => false,
            MutationResult::NotActivated => false,
            MutationResult::Refused => false,
            MutationResult::Unbuildable => false,
            MutationResult::TimedOut => false,
            MutationResult::InfrastructureFailure => false,
            MutationResult::EquivalentCandidate => false,
        }
    }

    /// Only Survived. A mutant that was never reached did not survive the
    /// test suite; it was never tested.
    pub const fn counts_as_survival(self) -> bool {
        match self {
            MutationResult::Killed => false,
            MutationResult::Survived => true,
            MutationResult::NotActivated => false,
            MutationResult::Refused => false,
            MutationResult::Unbuildable => false,
            MutationResult::TimedOut => false,
            MutationResult::InfrastructureFailure => false,
            MutationResult::EquivalentCandidate => false,
        }
    }

    /// Every planned mutation appears in the reported denominator. Nothing
    /// leaves silently to improve a score.
    pub const fn appears_in_denominator(self) -> bool {
        match self {
            MutationResult::Killed => true,
            MutationResult::Survived => true,
            MutationResult::NotActivated => true,
            MutationResult::Refused => true,
            MutationResult::Unbuildable => true,
            MutationResult::TimedOut => true,
            MutationResult::InfrastructureFailure => true,
            MutationResult::EquivalentCandidate => true,
        }
    }
}

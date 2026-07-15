//! Frozen clean-room repository architecture facts.

use crate::gates::GateId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageClass {
    Production,
    BinaryAdapter,
    DevOnly,
    Example,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageSpec {
    pub package: &'static str,
    pub path: &'static str,
    pub role: &'static str,
    pub class: PackageClass,
    pub layer: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeClass {
    Required,
    OptionalProfile,
    DevOnly,
}

/// One target-specific qualification requirement.
///
/// `target` names the ENVIRONMENT qualified against; `gates` names WHEN that
/// qualification is scheduled. They are different dimensions and neither may
/// stand in for the other: "std" is not a GateId. The schedule is stated here
/// because it is row-specific and cannot be derived from a compilation target or
/// from neighbouring SEED vocabulary.
///
/// The requirement stays Permanent after a gate passes; the receipt is
/// gate/run-specific evidence, not the lifetime of the law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationProfile {
    pub package: &'static str,
    pub profile: &'static str,
    pub target: &'static str,
    pub gates: &'static [GateId],
    pub requirement: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSpec {
    pub importer: &'static str,
    pub importee: &'static str,
    pub class: EdgeClass,
    pub profile: &'static str,
}

pub const REPOSITORY_NAME: &str = "BatPak";
pub const SPEC_VERSION: &str = "1.0.0";

/// Workspace implementation train. Publishable production packages ship
/// lockstep to `1.0.0`; nonpublishable packages inherit this version without
/// becoming release artifacts. Bootstrap rejects any version below this train
/// so a template cannot regress the family to a pre-1.0 line.
pub const WORKSPACE_VERSION: &str = "1.0.0-alpha.1";

pub const PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        package: "macbat-compiler",
        path: "crates/macbat/compiler",
        role: "pure Rust contract compiler",
        class: PackageClass::Production,
        layer: 0,
    },
    PackageSpec {
        package: "macbat",
        path: "crates/macbat/macros",
        role: "proc-macro front door",
        class: PackageClass::Production,
        layer: 1,
    },
    PackageSpec {
        package: "batpak",
        path: "crates/batpak",
        role: "semantic and durable core, including .fbat",
        class: PackageClass::Production,
        layer: 2,
    },
    PackageSpec {
        package: "syncbat",
        path: "crates/syncbat",
        role: "runtime crate containing runtime, PakVM, Bvisor, world, and port planes",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        package: "batql",
        path: "crates/batql",
        role: "BatQL parser, type checker, planner, partial evaluator, and ProgramImage compiler",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        package: "netbat",
        path: "crates/netbat",
        role: "bounded typed transport over declared SyncBat world entrypoints",
        class: PackageClass::Production,
        layer: 4,
    },
    PackageSpec {
        package: "testpak",
        path: "crates/testpak",
        role: "repository proof, forge, gauntlet, benchmark, and mutation battery",
        class: PackageClass::DevOnly,
        layer: 6,
    },
    PackageSpec {
        package: "batpak-cli",
        path: "apps/batpak-cli",
        role: "thin product command adapter; owns no semantic law",
        class: PackageClass::BinaryAdapter,
        layer: 5,
    },
    PackageSpec {
        package: "batpak-examples",
        path: "examples",
        role: "public-surface witness; runnable demos over production APIs only; owns no semantic law and depends on no dev tooling",
        class: PackageClass::Example,
        layer: 5,
    },
];

pub const QUALIFICATION_PROFILES: &[QualificationProfile] = &[
    QualificationProfile {
        package: "batpak",
        profile: "semantic",
        target: "no_std + alloc",
        gates: &[GateId::G0, GateId::G5],
        requirement: "contracts, schemas, codecs, image values, deterministic parsing, and storage-port law compile without std",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "semantic",
        target: "no_std + alloc",
        gates: &[GateId::G0, GateId::G5],
        requirement: "runtime transition core, PakVM validation/interpreter, Bvisor admission state, world values, and port protocols compile without std host adapters",
    },
    QualificationProfile {
        package: "batpak",
        profile: "native",
        target: "std",
        gates: &[GateId::G0, GateId::G5],
        requirement: "native filesystem, mapping, clock, entropy, and threaded storage adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "native",
        target: "std",
        gates: &[GateId::G0, GateId::G5],
        requirement: "threaded driver and native host-port adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "browser",
        target: "wasm32 host",
        gates: &[GateId::G0, GateId::G5],
        requirement: "browser adapters preserve semantic result, receipt, bounds, and recovery contracts without OS-backend normalization",
    },
];

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

/// One authenticated-history profile: what it admits, where it is implemented,
/// where it is qualified, and exactly which success bundles it can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthenticatedHistoryProfileSpec {
    pub profile: AuthenticatedHistoryProfile,
    /// The frozen valid pairings. Anything absent here is refused.
    pub permitted_witness_policies: &'static [WitnessPolicy],
    /// Local authoritative commitments and authority generations verify.
    pub requires_local_commitment_verification: bool,
    /// Signed seals and a signed whole-history commitment verify.
    pub requires_signed_history_verification: bool,
    /// An independent monotonic witness verifies.
    pub requires_independent_witness_verification: bool,
    pub implementation_gates: &'static [GateId],
    pub release_qualification_gates: &'static [GateId],
    /// The success bundle when no verified external anchor is present.
    ///
    /// `None` means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED. It does not
    /// mean unknown, not configured, posture unavailable, or fallback success.
    pub unanchored_success_claims: Option<AuthenticatedHistoryClaims>,
    /// The success bundle when an independent witness verifies. `None` means the
    /// profile admits no witness at all.
    pub verified_witness_success_claims: Option<AuthenticatedHistoryClaims>,
}

/// The frozen matrix. `SignedHistory + Required` is not silently upgraded to
/// `ExternallyAnchoredHistory`: the caller selects the stronger profile.
pub const AUTHENTICATED_HISTORY_PROFILES: &[AuthenticatedHistoryProfileSpec] = &[
    AuthenticatedHistoryProfileSpec {
        profile: AuthenticatedHistoryProfile::InternalConsistency,
        permitted_witness_policies: &[WitnessPolicy::None],
        requires_local_commitment_verification: true,
        requires_signed_history_verification: false,
        requires_independent_witness_verification: false,
        implementation_gates: &[GateId::G2],
        release_qualification_gates: &[GateId::G9],
        unanchored_success_claims: Some(INTERNAL_CONSISTENCY_SUCCESS),
        // Admits no witness, so no witnessed success exists.
        verified_witness_success_claims: None,
    },
    AuthenticatedHistoryProfileSpec {
        profile: AuthenticatedHistoryProfile::SignedHistory,
        permitted_witness_policies: &[WitnessPolicy::None, WitnessPolicy::Optional],
        requires_local_commitment_verification: true,
        requires_signed_history_verification: true,
        requires_independent_witness_verification: false,
        implementation_gates: &[GateId::G2],
        release_qualification_gates: &[GateId::G9],
        unanchored_success_claims: Some(SIGNED_UNANCHORED_SUCCESS),
        verified_witness_success_claims: Some(WITNESSED_SUCCESS),
    },
    AuthenticatedHistoryProfileSpec {
        profile: AuthenticatedHistoryProfile::ExternallyAnchoredHistory,
        permitted_witness_policies: &[WitnessPolicy::Required],
        requires_local_commitment_verification: true,
        requires_signed_history_verification: true,
        requires_independent_witness_verification: true,
        implementation_gates: &[GateId::G2],
        release_qualification_gates: &[GateId::G9],
        // No successful unanchored result is admitted. An absent or invalid
        // required witness refuses; it never falls back to a weaker success.
        unanchored_success_claims: None,
        verified_witness_success_claims: Some(WITNESSED_SUCCESS),
    },
];

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

// --- TestPak proof policy and mutation (DEC-015, DEC-074) -------------------
// Typed facts only. Bootstrap compiles no mutant, activates no slot, runs no
// nextest, invokes no rustc, kills nothing, proves no equivalence, promotes
// nothing, and classifies no real diff semantically. It proves the contract.

/// The three frozen mutation lanes. Muterprater is a TestPak plane, never a
/// standalone product, a second semantic authority, or a replacement compiler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationLane {
    /// Mutates BatPak-owned semantic structures. No per-candidate Rust compile;
    /// the reference interpreter executes the mutant and an independent evidence
    /// route judges it.
    SemanticIr,
    /// One test-profile shard holds many candidate slots; one stable mutation
    /// identity selects one slot per isolated process. Slots never enter an
    /// ordinary production artifact.
    SelectableCompiled,
    /// Implementation-sensitive material whose truth depends on real compiler
    /// and platform behavior. Runs under real rustc semantics.
    CompilerBacked,
}

/// Every planned mutation reaches exactly one of these. They stay distinct
/// because collapsing them into pass/fail is how a denominator quietly shrinks:
/// an unbuildable candidate is not a detected fault, and a timeout is not a kill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationResult {
    /// Activated, baseline and selected tests qualified, and one or more
    /// qualified witnesses reject the mutant.
    Killed,
    /// Activated, selected tests qualified, and no selected witness rejected it.
    Survived,
    /// Not observed as reached or selected. This is never Survived.
    NotActivated,
    /// Admission or policy refused the candidate or run. This is never Killed.
    Refused,
    /// Could not produce the required executable or admitted semantic artifact.
    /// A compiler error from an invalid mutation is not detection. Never Killed.
    Unbuildable,
    /// Exceeded its declared deadline. Neither Killed nor Survived.
    TimedOut,
    /// The runner, compiler, environment, or proof mechanism failed. Neither
    /// Killed nor Survived.
    InfrastructureFailure,
    /// May be equivalent under the named semantic scope. This is a candidate
    /// classification pending an independent equivalence witness, not proof.
    EquivalentCandidate,
}

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

/// One lane and what it may assume. Not a TestPak product catalog: three rows,
/// frozen, enforcing the lane contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MutationLaneSpec {
    pub lane: MutationLane,
    /// Whether a candidate requires its own Rust compilation.
    pub requires_per_candidate_rust_compile: bool,
    /// Whether execution requires real rustc semantics.
    pub requires_real_rustc_semantics: bool,
    /// Whether a candidate must carry activation evidence before a Survived or
    /// Killed verdict is admissible.
    pub requires_activation_evidence: bool,
    /// Whether mutation slots may exist in an ordinary production artifact.
    pub permits_production_profile_slots: bool,
    /// Whether an independent (non-self-calling) evidence route is required.
    pub requires_independent_evidence_route: bool,
    pub gates: &'static [GateId],
}

pub const MUTATION_LANES: &[MutationLaneSpec] = &[
    MutationLaneSpec {
        lane: MutationLane::SemanticIr,
        requires_per_candidate_rust_compile: false,
        requires_real_rustc_semantics: false,
        requires_activation_evidence: true,
        permits_production_profile_slots: false,
        requires_independent_evidence_route: true,
        gates: &[GateId::G3],
    },
    MutationLaneSpec {
        lane: MutationLane::SelectableCompiled,
        requires_per_candidate_rust_compile: false,
        requires_real_rustc_semantics: true,
        requires_activation_evidence: true,
        permits_production_profile_slots: false,
        requires_independent_evidence_route: true,
        gates: &[GateId::G3],
    },
    MutationLaneSpec {
        lane: MutationLane::CompilerBacked,
        requires_per_candidate_rust_compile: true,
        requires_real_rustc_semantics: true,
        requires_activation_evidence: true,
        permits_production_profile_slots: false,
        requires_independent_evidence_route: true,
        gates: &[GateId::G3],
    },
];

/// Results that may never be counted as a kill. An invalid mutation that fails
/// to compile proves nothing about the test suite.
pub const NEVER_KILLED: &[MutationResult] = &[
    MutationResult::NotActivated,
    MutationResult::Refused,
    MutationResult::Unbuildable,
    MutationResult::TimedOut,
    MutationResult::InfrastructureFailure,
];

/// Results that may never be counted as survival. A mutant that was never
/// reached did not survive the test suite; it was never tested.
pub const NEVER_SURVIVED: &[MutationResult] = &[
    MutationResult::NotActivated,
    MutationResult::Refused,
    MutationResult::Unbuildable,
    MutationResult::TimedOut,
    MutationResult::InfrastructureFailure,
];

/// Every planned mutation terminates in one of these, and every one appears in
/// the reported denominator. Nothing leaves silently to improve a score.
pub const TERMINAL_MUTATION_RESULTS: &[MutationResult] = &[
    MutationResult::Killed,
    MutationResult::Survived,
    MutationResult::NotActivated,
    MutationResult::Refused,
    MutationResult::Unbuildable,
    MutationResult::TimedOut,
    MutationResult::InfrastructureFailure,
    MutationResult::EquivalentCandidate,
];

/// Candidate material is disposable and derived. It is never durable proof
/// truth, and there is no canonical direct-write path from generation into
/// tracked source.
pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";

/// Tracked surfaces a generated candidate may never write directly. Promotion
/// is a separate admitted action with its own receipt.
pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS: &[&str] =
    &["src/", "tests/", "spec/", "docs/", "companion/"];

/// No oracle, no promotion. No named obligation, no promotion. No killed mutant
/// or equivalent hostile evidence, no promotion. No proof receipt, no trust.
pub const PROMOTION_REQUIREMENTS: &[&str] = &[
    "independent evidence or oracle identity",
    "named invariant, guarantee, obligation, or documented proof gap",
    "killed real semantic mutant or equivalent hostile evidence",
    "auditable proof and promotion receipt",
];

pub const EDGES: &[EdgeSpec] = &[
    EdgeSpec { importer: "macbat", importee: "macbat-compiler", class: EdgeClass::Required, profile: "compile" },
    EdgeSpec { importer: "batpak", importee: "macbat", class: EdgeClass::Required, profile: "derive" },
    EdgeSpec { importer: "syncbat", importee: "batpak", class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: "batql", importee: "batpak", class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: "netbat", importee: "batpak", class: EdgeClass::Required, profile: "transport-contract" },
    EdgeSpec { importer: "netbat", importee: "syncbat", class: EdgeClass::Required, profile: "runtime-entrypoints" },
    EdgeSpec { importer: "batpak-cli", importee: "batpak", class: EdgeClass::Required, profile: "core" },
    EdgeSpec { importer: "batpak-cli", importee: "syncbat", class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: "batpak-cli", importee: "batql", class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: "batpak-cli", importee: "netbat", class: EdgeClass::OptionalProfile, profile: "serve" },
    EdgeSpec { importer: "testpak", importee: "macbat-compiler", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "macbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "batpak", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "syncbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "batql", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "netbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "batpak-examples", importee: "batpak", class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: "batpak-examples", importee: "syncbat", class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: "batpak-examples", importee: "batql", class: EdgeClass::Required, profile: "example" },
];

pub const REQUIRED_DOCS: &[&str] = &[
    "README.md",
    "SPEC.sha256",
    "AGENTS.md",
    "FINAL_RECONCILIATION.md",
    "DELIVERY_NOTES.md",
    "docs/00_CONSTITUTION.md",
    "docs/01_FACTORY.md",
    "docs/02_SYSTEM_MODEL.md",
    "docs/03_REPOSITORY_AND_PACKAGES.md",
    "docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md",
    "docs/05_STORAGE_FBAT_AND_TILES.md",
    "docs/06_MACBAT.md",
    "docs/07_PAKVM_ISA.md",
    "docs/08_SYNCBAT_RUNTIME.md",
    "docs/09_BVISOR.md",
    "docs/10_WORLD_IMAGES_AND_PORTS.md",
    "docs/11_NETBAT.md",
    "docs/12_TESTPAK.md",
    "docs/13_BATQL_CONTRACT.md",
    "docs/14_RECEIPTS_AND_EXPLANATION.md",
    "docs/15_SCHEMA_CODEC_AND_MIGRATION.md",
    "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
    "docs/17_DELIVERY_AND_CONCURRENCY.md",
    "docs/18_DATA_ORIENTED_ECS.md",
    "docs/19_SECURITY_MODEL.md",
    "docs/20_DEPENDENCY_SOVEREIGNTY.md",
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
    "docs/22_MIGRATION_AND_CUTOVER.md",
    "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
    "docs/24_GAUNTLET.md",
    "docs/25_IMPLEMENTATION_GATES.md",
    "docs/26_COMMAND_PLANE.md",
    "docs/27_WORKFLOW.md",
    "docs/28_SELF_EXPLAINING_REPOSITORY.md",
    "docs/29_STATUS_AND_SUPERSESSION.md",
    "docs/30_DECISION_AND_REJECTION_LEDGER.md",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md",
    "docs/32_IMPLEMENTATION_CONSTANTS.md",
    "docs/33_AGENT_FINISH_LINE_CHECKLIST.md",
    "docs/34_LEGACY_INVARIANT_COVERAGE.md",
    "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md",
    "docs/36_PUBLIC_API_CI_AND_RELEASE.md",
    "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md",
    "docs/GUARANTEE_GRAPH.generated.md",
    "companion/BATQL_LANGUAGE.md",
    "spec/architecture.rs",
    "spec/invariants.rs",
    "spec/dispositions.rs",
    "spec/legacy_obligations.rs",
    "spec/legacy_invariant_coverage.rs",
    "spec/operators.rs",
    "spec/guarantees.rs",
    "spec/gates.rs",
    "bootstrap/README.md",
    "bootstrap/seedcheck.rs",
    "bootstrap/materialize.rs",
    "bootstrap/audit.py",
    "bootstrap/freeze.py",
    "bootstrap/project.py",
    "spec/README.md",
    "history/README.md",
];

pub const FORBIDDEN_TARGET_PATHS: &[&str] = &[
    "crates/filebat",
    "crates/bat-vm",
    "crates/batpak-core",
    "crates/core",
    "crates/hostbat",
    "crates/bvisor",
    "crates/pakvm",
    "crates/vpak",
    "crates/testkit",
    "tools/xtask",
    "tools/integrity",
    "corpus",
    "fixtures",
];

pub const SYNCBAT_REQUIRED_PLANES: &[&str] = &[
    "src/runtime.rs",
    "src/runtime",
    "src/pakvm.rs",
    "src/pakvm",
    "src/bvisor.rs",
    "src/bvisor",
    "src/world.rs",
    "src/world",
    "src/port.rs",
    "src/port",
];

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

/// Candidate material is disposable and derived. It is never durable proof
/// truth, and there is no canonical direct-write path from generation into
/// tracked source.
pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";

/// What a release receipt binds (DEC-058, 5.5E1 ruling). One typed inventory:
/// docs/36 projects the full list, docs/24 names the gauntlet inputs, and
/// DEC-058 references this owner instead of restating a third copy — three
/// hand-authored seal lists disagreed until this enum existed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseSealField {
    SourceTree,
    Toolchain,
    DependencyGraph,
    GeneratedFacts,
    CompatibilityCorpus,
    TestDispositions,
    MutationDispositions,
    FuzzDispositions,
    BenchmarkDispositions,
    CompilerAssumptionLedger,
    DependencyLedger,
    /// Mandatory even when empty: an empty set states "no kernels admitted",
    /// it never disappears from the schema.
    KernelQualificationSet,
    PackageContents,
    PublicApi,
    Sbom,
    LicenseEvidence,
    ProofFreshness,
}

/// Every seal field, in declaration order. Completeness is enforced by
/// seedcheck's exhaustive classification: a new field cannot be added without
/// appearing here, and none may appear twice.
pub const RELEASE_SEAL_FIELDS: &[ReleaseSealField] = &[
    ReleaseSealField::SourceTree,
    ReleaseSealField::Toolchain,
    ReleaseSealField::DependencyGraph,
    ReleaseSealField::GeneratedFacts,
    ReleaseSealField::CompatibilityCorpus,
    ReleaseSealField::TestDispositions,
    ReleaseSealField::MutationDispositions,
    ReleaseSealField::FuzzDispositions,
    ReleaseSealField::BenchmarkDispositions,
    ReleaseSealField::CompilerAssumptionLedger,
    ReleaseSealField::DependencyLedger,
    ReleaseSealField::KernelQualificationSet,
    ReleaseSealField::PackageContents,
    ReleaseSealField::PublicApi,
    ReleaseSealField::Sbom,
    ReleaseSealField::LicenseEvidence,
    ReleaseSealField::ProofFreshness,
];

/// Tracked surfaces a generated candidate may never write directly. Promotion
/// is a separate admitted action with its own receipt.
pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS: &[&str] =
    &["src/", "tests/", "spec/", "docs/", "companion/"];

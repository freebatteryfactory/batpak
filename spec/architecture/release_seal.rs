// This vocabulary's destination is the release/ domain created in F4; it lives
// here as a focused concept file until that domain exists.

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

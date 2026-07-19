use super::types::ReleaseSealField;

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
    ReleaseSealField::ModelDispositions,
    ReleaseSealField::RuntimeConformanceDispositions,
    ReleaseSealField::CompilerAssumptionLedger,
    ReleaseSealField::DependencyLedger,
    ReleaseSealField::KernelQualificationSet,
    ReleaseSealField::CandidatePromotionSet,
    ReleaseSealField::PackageContents,
    ReleaseSealField::PublicApi,
    ReleaseSealField::Sbom,
    ReleaseSealField::LicenseEvidence,
    ReleaseSealField::ProofFreshness,
];

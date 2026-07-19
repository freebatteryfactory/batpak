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
    /// The model-based verification dispositions (docs/38): outcomes of the
    /// independently owned model evaluations the release consumed. Mandatory
    /// even when empty: an empty set states "no model evaluations admitted";
    /// a missing field is an incomplete envelope, so the field never
    /// disappears from the schema.
    ModelDispositions,
    /// The runtime-conformance dispositions (DEC-078, docs/38): the
    /// append-only replay-witness conformance outcomes the release consumed.
    /// Mandatory even when empty: an empty set states "no runtime
    /// conformance observed"; a missing field is an incomplete envelope, so
    /// the field never disappears from the schema.
    RuntimeConformanceDispositions,
    CompilerAssumptionLedger,
    DependencyLedger,
    /// Mandatory even when empty: an empty set states "no kernels admitted",
    /// it never disappears from the schema.
    KernelQualificationSet,
    /// The candidate promotion set (DEC-080, docs/39): every promoted
    /// candidate this release binds. Mandatory even when empty: an empty set
    /// states "no candidates promoted"; a missing field is an incomplete
    /// envelope, so the field never disappears from the schema.
    CandidatePromotionSet,
    PackageContents,
    PublicApi,
    Sbom,
    LicenseEvidence,
    ProofFreshness,
}

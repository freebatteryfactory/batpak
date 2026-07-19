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

/// How a seal field relates to emptiness (pre-F5 item A ruling). A set-valued
/// field distinguishes explicit emptiness from omission: an empty set is
/// authored evidence ("nothing of this family was admitted"), while a missing
/// field is an incomplete envelope. A non-set field has no empty form at all
/// — it binds exactly one commitment, so only presence or absence exists for
/// it and "empty" is not a state it can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptySetPosture {
    /// The field binds one singular commitment (a source tree, a toolchain,
    /// a graph, a freshness verdict). It has no empty form: it is present or
    /// the envelope is incomplete.
    NotSetValued,
    /// The field binds a set of evidence rows whose explicit empty form is
    /// itself evidence, distinct from omission; the field never disappears
    /// from the schema.
    ExplicitEvenWhenEmpty,
}

impl ReleaseSealField {
    /// The typed empty-set classification of every seal field. Exhaustive and
    /// wildcard-free BY LAW: a new field does not compile until an arm
    /// classifies it here, and the docs/36 matrix is a projection of these
    /// arms — never of doc-comment prose, which carries rationale only.
    pub const fn empty_set_posture(self) -> EmptySetPosture {
        match self {
            ReleaseSealField::SourceTree => EmptySetPosture::NotSetValued,
            ReleaseSealField::Toolchain => EmptySetPosture::NotSetValued,
            ReleaseSealField::DependencyGraph => EmptySetPosture::NotSetValued,
            ReleaseSealField::GeneratedFacts => EmptySetPosture::NotSetValued,
            ReleaseSealField::CompatibilityCorpus => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::TestDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::MutationDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::FuzzDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::BenchmarkDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::ModelDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::RuntimeConformanceDispositions => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::CompilerAssumptionLedger => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::DependencyLedger => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::KernelQualificationSet => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::CandidatePromotionSet => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::PackageContents => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::PublicApi => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::Sbom => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::LicenseEvidence => EmptySetPosture::ExplicitEvenWhenEmpty,
            ReleaseSealField::ProofFreshness => EmptySetPosture::NotSetValued,
        }
    }
}

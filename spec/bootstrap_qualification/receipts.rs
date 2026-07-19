use crate::guarantees::ContractId;

/// Owner of the receipt ALGEBRA — what a bootstrap qualification receipt means.
pub const TIER0_RECEIPT_ALGEBRA_OWNER: ContractId = ContractId("BP-RECEIPTS-1");
/// Owner of the DENOMINATOR — which Tier 0 gates the qualification requires.
pub const TIER0_DENOMINATOR_OWNER: ContractId = ContractId("BP-BOOTSTRAP-1");
/// Owner of the HOSTED qualification posture — which target and run may close a gate.
pub const TIER0_HOSTED_QUALIFICATION_OWNER: ContractId =
    ContractId("BP-PUBLIC-API-CI-RELEASE-1");

// ===========================================================================
// Denominator vocabulary (unchanged from E6a: the one closed set of gates)
// ===========================================================================

/// One Tier 0 gate whose passing receipt the qualification requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0ReceiptKind {
    /// The compile-fail law fixtures: sealed refusals exercised through the
    /// real toolchain. A fixture SET, not a single executable.
    LawFixtures,
    /// The independent Rust seed checker binary.
    Seedcheck,
    /// The isolated Gate-0 materializer, which also binds an output-tree digest.
    Materialize,
    /// The seedcheck in-crate refusal-test harness.
    SeedcheckTests,
    /// The specification crate's own in-crate refusal-test harness.
    SpecTests,
}

impl Tier0ReceiptKind {
    /// The ONE Tier 0 denominator, in canonical order. Every consumer — the
    /// Delivery Notes projection, the hosted workflow, the receipt verifier —
    /// derives its required set from this inventory, never from a copied tuple.
    pub const ALL: &'static [Tier0ReceiptKind] = &[
        Tier0ReceiptKind::LawFixtures,
        Tier0ReceiptKind::Seedcheck,
        Tier0ReceiptKind::Materialize,
        Tier0ReceiptKind::SeedcheckTests,
        Tier0ReceiptKind::SpecTests,
    ];

    /// The stable receipt slug the harness and artifact use.
    pub const fn slug(self) -> &'static str {
        match self {
            Tier0ReceiptKind::LawFixtures => "tier0-law-fixtures",
            Tier0ReceiptKind::Seedcheck => "tier0-seedcheck",
            Tier0ReceiptKind::Materialize => "tier0-materialize",
            Tier0ReceiptKind::SeedcheckTests => "tier0-seedcheck-tests",
            Tier0ReceiptKind::SpecTests => "tier0-spec-tests",
        }
    }

    /// Resolve a slug back to its kind, or `None` for an unknown slug. The
    /// artifact parser uses this: an unknown receipt slug is unrepresentable as
    /// a `Tier0ReceiptKind` and is refused before it can reach verification.
    pub fn from_slug(slug: &str) -> Option<Tier0ReceiptKind> {
        Tier0ReceiptKind::ALL
            .iter()
            .copied()
            .find(|k| k.slug() == slug)
    }

    /// What artifact evidence this receipt must carry.
    pub const fn artifact_policy(self) -> Tier0ArtifactPolicy {
        match self {
            Tier0ReceiptKind::LawFixtures => Tier0ArtifactPolicy::FixtureSet,
            Tier0ReceiptKind::Seedcheck => Tier0ArtifactPolicy::Executable,
            Tier0ReceiptKind::Materialize => Tier0ArtifactPolicy::ExecutableAndOutputTree,
            Tier0ReceiptKind::SeedcheckTests => Tier0ArtifactPolicy::Executable,
            Tier0ReceiptKind::SpecTests => Tier0ArtifactPolicy::Executable,
        }
    }
}

/// Which artifact evidence SHAPE a receipt must carry. This classifies the
/// evidence a kind demands; the concrete evidence itself is `Tier0ArtifactEvidence`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0ArtifactPolicy {
    /// A set of short-lived fixture binaries; the receipt binds the set digest.
    FixtureSet,
    /// One executable, bound by its artifact digest.
    Executable,
    /// One executable plus the materializer's canonical output-tree digest.
    ExecutableAndOutputTree,
}

impl Tier0ArtifactPolicy {
    /// Every artifact policy, in canonical order.
    pub const ALL: &'static [Tier0ArtifactPolicy] = &[
        Tier0ArtifactPolicy::FixtureSet,
        Tier0ArtifactPolicy::Executable,
        Tier0ArtifactPolicy::ExecutableAndOutputTree,
    ];

    /// The stable policy slug the artifact projection uses.
    pub const fn slug(self) -> &'static str {
        match self {
            Tier0ArtifactPolicy::FixtureSet => "fixture-set",
            Tier0ArtifactPolicy::Executable => "executable",
            Tier0ArtifactPolicy::ExecutableAndOutputTree => "executable+output-tree",
        }
    }
}

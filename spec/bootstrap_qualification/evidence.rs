use crate::toolchain::RustRelease;
use super::*;

// ===========================================================================
// Typed stage outcomes (5.5E6b): "not attempted" is not "failed"
// ===========================================================================

/// Whether a gate binary compiled. A stage that was never attempted is a
/// DIFFERENT fact from one that was attempted and failed; collapsing them into a
/// single boolean is the mistake the E6a shape made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilationOutcome {
    NotAttempted,
    Failed,
    Succeeded,
}

/// Whether execution was attempted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionAttempt {
    NotAttempted,
    Attempted,
}

/// The result of an attempted execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Failed,
    Passed,
}

// ===========================================================================
// Artifact evidence sum (5.5E6b): shape and evidence cannot diverge
// ===========================================================================

/// The concrete artifact evidence a receipt carries. A sum type, not a bag of
/// optional digests: a fixture set WITHOUT a digest, an ordinary executable
/// carrying an output-tree digest, and a materializer MISSING its output tree
/// are all unrepresentable after parsing. The variant must equal the receipt
/// kind's `artifact_policy()` exactly — no missing evidence, no surplus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0ArtifactEvidence {
    /// A set of fixture binaries, bound by one set digest.
    FixtureSet { digest: Sha256Digest },
    /// One executable, bound by its artifact digest.
    Executable { digest: Sha256Digest },
    /// One executable plus the materializer's canonical output-tree digest.
    ExecutableAndOutputTree {
        executable_digest: Sha256Digest,
        output_tree_digest: Sha256Digest,
    },
}

impl Tier0ArtifactEvidence {
    /// Which policy this concrete evidence satisfies. Verification compares this
    /// against the kind's demanded policy; a mismatch is refused.
    pub const fn policy(&self) -> Tier0ArtifactPolicy {
        match self {
            Tier0ArtifactEvidence::FixtureSet { .. } => Tier0ArtifactPolicy::FixtureSet,
            Tier0ArtifactEvidence::Executable { .. } => Tier0ArtifactPolicy::Executable,
            Tier0ArtifactEvidence::ExecutableAndOutputTree { .. } => {
                Tier0ArtifactPolicy::ExecutableAndOutputTree
            }
        }
    }
}

// ===========================================================================
// Source, toolchain, and hosted-run bindings (5.5E6b)
// ===========================================================================

/// WHERE and against WHAT source a qualification ran. Two real postures, never
/// forged into one another:
///
/// * `GitCheckout` — the authoritative hosted run has a real `.git` checkout and
///   MUST bind the commit and tree it qualified.
/// * `FrozenExport` — a local run against a clean `git archive` export with no
///   `.git`. It binds the manifest and export-tree digests it CAN compute and
///   never invents git coordinates it cannot verify. Supplemental only: it can
///   never qualify the authoritative target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceBinding {
    GitCheckout {
        commit: GitCommitSha,
        tree: GitTreeSha,
        spec_manifest_digest: Sha256Digest,
        workflow_digest: Sha256Digest,
    },
    FrozenExport {
        spec_manifest_digest: Sha256Digest,
        export_tree_digest: Sha256Digest,
    },
}

impl SourceBinding {
    /// Whether this source posture is permitted to qualify the authoritative
    /// target. Only a real git checkout may: a frozen export has no commit or
    /// tree to bind and is corroborating evidence only.
    pub const fn may_qualify_authoritative(&self) -> bool {
        matches!(self, SourceBinding::GitCheckout { .. })
    }
}

/// The exact toolchain a qualification ran on. Rustc and cargo are bound
/// separately by release and commit; the tracked `rust-toolchain.toml` is bound
/// by its file digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolchainBinding {
    pub rustc_release: RustRelease,
    pub rustc_commit: ToolchainCommit,
    pub cargo_release: RustRelease,
    pub cargo_commit: ToolchainCommit,
    pub toolchain_file_digest: Sha256Digest,
}

/// The hosted GitHub Actions run that produced authoritative evidence. There is
/// no provider-neutral CI abstraction: GitHub Actions is the one real adopter,
/// and inventing an enum for hypothetical providers would be vocabulary nobody
/// consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubActionsRunBinding {
    pub repository: String,
    pub workflow_path: String,
    pub run_id: u64,
    pub run_attempt: u32,
    pub runner_image_os: String,
    pub runner_image_version: String,
}

/// The ONE canonical workflow path that produces authoritative Tier 0 evidence.
/// A hosted run whose `workflow_path` is not this exact string did not run the
/// authoritative qualification, so it cannot stand as either side of a promotion
/// confirmation (5.5E6c1). Owned by `BP-PUBLIC-API-CI-RELEASE-1`, the hosted
/// qualification posture; the tracked workflow file lives at this path and the
/// comparator compares against this constant, never a copied literal.
pub const AUTHORITATIVE_WORKFLOW_PATH: &str = ".github/workflows/msvc-qualification.yml";

/// The exact CPython release the bootstrap gates run under (5.5E6c2). Most Tier 0
/// gates are executed by Python (project/audit/freeze/selftest), so the
/// interpreter is part of the BUILDER IDENTITY, alongside rustc and cargo.
/// Rendered canonically as `MAJOR.MINOR.PATCH`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythonRelease {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl PythonRelease {
    /// Canonical `MAJOR.MINOR.PATCH` spelling.
    pub fn render(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The bootstrap runtime a qualification ran under (5.5E6c2). One admitted
/// implementation: the `python-release` binding MEANS a CPython release, and the
/// verifier refuses another implementation. A one-member `PythonImplementation`
/// enum would be vocabulary nobody consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapRuntimeBinding {
    pub python_release: PythonRelease,
}

/// The one authoritative bootstrap Python release (5.5E6c2). A qualification that
/// closes a gate on the authoritative target MUST run under this exact CPython
/// release. The tracked workflow's `setup-python` version is a PROJECTION of this
/// typed selection, never its owner (`BP-PUBLIC-API-CI-RELEASE-1`). Pinning the
/// exact patch makes cross-run promotion confirmation's runtime equality
/// satisfiable: the candidate and confirming runs bind the identical release, and
/// two runs on the same WRONG runtime are still refused against this anchor.
pub const AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE: PythonRelease =
    PythonRelease { major: 3, minor: 12, patch: 10 };

// ===========================================================================
// Observations (5.5E6b): the raw, possibly-incoherent report
// ===========================================================================

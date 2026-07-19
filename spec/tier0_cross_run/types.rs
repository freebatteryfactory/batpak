//! Canonical cross-run boundary nouns: the side vocabulary, the recorded
//! agreement strengths, and the two sealed proofs the comparison and promotion
//! algorithms produce and consume across the sibling boundary.

use crate::bootstrap_qualification::{
    BootstrapRuntimeBinding, GitCommitSha, GitHubActionsRunBinding, GitTreeSha, Sha256Digest,
    ToolchainBinding,
};
use crate::toolchain::RustTargetTriple;

/// Which side of a two-run comparison a condition applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Both,
}

impl Side {
    /// Resolve which side(s) a boolean pair implicates. `(false, false)` cannot
    /// occur at a call site guarded by `left || right`; it maps to `Both` so the
    /// function is total.
    pub(crate) const fn of(left: bool, right: bool) -> Side {
        match (left, right) {
            (true, false) => Side::Left,
            (false, true) => Side::Right,
            _ => Side::Both,
        }
    }
}

/// Whether two same-source runs ran on the same toolchain. Divergence does NOT
/// weaken the same-source verdict — it records that the source was reproduced
/// under a different compiler, which is a legitimate re-run posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolchainAgreement {
    Identical(ToolchainBinding),
    Divergent {
        left: ToolchainBinding,
        right: ToolchainBinding,
    },
}

/// Whether two same-source runs ran under the same bootstrap runtime (CPython
/// release). Like the toolchain, a divergent runtime does NOT weaken the
/// same-source verdict — it records that the source was reproduced under a
/// different builder interpreter. Promotion confirmation requires `Identical`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapRuntimeAgreement {
    Identical(BootstrapRuntimeBinding),
    Divergent {
        left: BootstrapRuntimeBinding,
        right: BootstrapRuntimeBinding,
    },
}

/// Whether two same-source runs qualified the same physical target. A cross-
/// target pair still describes one source — it is ORTHOGONAL corroboration (the
/// same source qualified on a second platform: broader coverage, not automatically
/// stronger along every assurance axis), and their executable digests differ by
/// construction. A cross-target pair does NOT confirm a promotion, which requires
/// the authoritative target on both sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetAgreement {
    SameTarget(RustTargetTriple),
    CrossTarget {
        left: RustTargetTriple,
        right: RustTargetTriple,
    },
}

/// A sealed proof that two INDEPENDENT hosted runs describe the same committed
/// source snapshot. Obtainable ONLY through `compare_runs`. Retains the
/// proven-common source coordinates, the two DISTINCT run identities that
/// independently attested them, and the toolchain/target agreement strength. It
/// is NECESSARY but not SUFFICIENT for a promotion confirmation (see
/// `confirm_promotion`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameSourceProof {
    pub(super) commit: GitCommitSha,
    pub(super) tree: GitTreeSha,
    pub(super) spec_manifest_digest: Sha256Digest,
    pub(super) workflow_digest: Sha256Digest,
    pub(super) materializer_output_tree: Sha256Digest,
    pub(super) left_run: GitHubActionsRunBinding,
    pub(super) right_run: GitHubActionsRunBinding,
    pub(super) toolchain_agreement: ToolchainAgreement,
    pub(super) target_agreement: TargetAgreement,
    pub(super) bootstrap_runtime_agreement: BootstrapRuntimeAgreement,
    pub(super) _seal: (),
}

impl SameSourceProof {
    pub const fn commit(&self) -> GitCommitSha {
        self.commit
    }
    pub const fn tree(&self) -> GitTreeSha {
        self.tree
    }
    pub const fn spec_manifest_digest(&self) -> Sha256Digest {
        self.spec_manifest_digest
    }
    pub const fn workflow_digest(&self) -> Sha256Digest {
        self.workflow_digest
    }
    /// The one deterministic, source-derived artifact digest both runs
    /// recomputed — the heart of the corroboration.
    pub const fn materializer_output_tree(&self) -> Sha256Digest {
        self.materializer_output_tree
    }
    pub const fn left_run(&self) -> &GitHubActionsRunBinding {
        &self.left_run
    }
    pub const fn right_run(&self) -> &GitHubActionsRunBinding {
        &self.right_run
    }
    pub const fn toolchain_agreement(&self) -> &ToolchainAgreement {
        &self.toolchain_agreement
    }
    pub const fn target_agreement(&self) -> &TargetAgreement {
        &self.target_agreement
    }
    /// Whether the two runs ran under the same bootstrap runtime — recorded as
    /// corroboration strength, required `Identical` for promotion confirmation.
    pub const fn bootstrap_runtime_agreement(&self) -> &BootstrapRuntimeAgreement {
        &self.bootstrap_runtime_agreement
    }
}

/// A sealed proof that a candidate qualification run and a cleanroom
/// qualification run CONFIRM the promotion of one committed source snapshot to
/// the authoritative target. Obtainable ONLY through `confirm_promotion`.
///
/// This is strictly stronger than `SameSourceProof`. Same-source asks "do these
/// two runs describe one committed source snapshot?" and tolerates a cross-target
/// or cross-toolchain pair as orthogonal corroboration. Promotion confirmation
/// additionally requires that BOTH runs qualified the AUTHORITATIVE target under
/// the IDENTICAL pinned toolchain, in the SAME repository and canonical workflow —
/// the exact posture the candidate-exact-SHA and cleanroom-exact-SHA runs share
/// across a lawful fast-forward. It retains the same-source proof, the confirmed
/// authoritative target and toolchain, and the two DISTINCT run identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromotionConfirmationProof {
    pub(super) same_source: SameSourceProof,
    pub(super) target: RustTargetTriple,
    pub(super) toolchain: ToolchainBinding,
    pub(super) bootstrap_runtime: BootstrapRuntimeBinding,
    pub(super) candidate_run: GitHubActionsRunBinding,
    pub(super) cleanroom_run: GitHubActionsRunBinding,
    pub(super) _seal: (),
}

impl PromotionConfirmationProof {
    /// The same-source proof this confirmation is built on. Promotion
    /// confirmation inherits every same-source coordinate and adds authority.
    pub const fn same_source(&self) -> &SameSourceProof {
        &self.same_source
    }
    /// The authoritative target both runs qualified.
    pub const fn target(&self) -> RustTargetTriple {
        self.target
    }
    /// The identical pinned toolchain both runs ran on.
    pub const fn toolchain(&self) -> &ToolchainBinding {
        &self.toolchain
    }
    /// The identical pinned bootstrap runtime (CPython release) both runs ran on.
    pub const fn bootstrap_runtime(&self) -> &BootstrapRuntimeBinding {
        &self.bootstrap_runtime
    }
    /// The candidate-branch hosted run.
    pub const fn candidate_run(&self) -> &GitHubActionsRunBinding {
        &self.candidate_run
    }
    /// The cleanroom hosted run.
    pub const fn cleanroom_run(&self) -> &GitHubActionsRunBinding {
        &self.cleanroom_run
    }
}

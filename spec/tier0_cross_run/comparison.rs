use crate::bootstrap_qualification::{
    BootstrapRuntimeBinding, GitCommitSha, GitHubActionsRunBinding, GitTreeSha, Sha256Digest,
    SourceBinding, ToolchainBinding, VerifiedTier0Qualification,
};
use crate::toolchain::RustTargetTriple;
use super::*;

/// Why two verified qualifications cannot be compared for same-source AT ALL — a
/// precondition failure, distinct from a source DIVERGENCE (a real disagreement
/// between two otherwise-comparable runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotComparable {
    /// A side rode a frozen export: supplemental evidence with no committed
    /// commit or tree to compare. `which` names the offending side(s).
    FrozenExportSource { which: Side },
    /// A side bound no hosted-run identity: it is not a hosted run, so it cannot
    /// stand as one of "two hosted runs".
    MissingHostedRun { which: Side },
    /// Both sides are the SAME hosted run (equal repository, run id, and
    /// attempt). One run compared against itself is not two independent
    /// witnesses, so it proves nothing about reproducibility.
    SameHostedRun,
}

/// The single source-identity coordinate on which two comparable runs disagree.
/// Each arm names the coordinate and carries both sides, so a divergence is
/// diagnosable — never a bare "not equal".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceDivergence {
    SourceCommit {
        left: GitCommitSha,
        right: GitCommitSha,
    },
    SourceTree {
        left: GitTreeSha,
        right: GitTreeSha,
    },
    SpecManifestDigest {
        left: Sha256Digest,
        right: Sha256Digest,
    },
    WorkflowDigest {
        left: Sha256Digest,
        right: Sha256Digest,
    },
    MaterializerOutputTree {
        left: Sha256Digest,
        right: Sha256Digest,
    },
}

/// The result of comparing two verified qualifications for same-source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrossRunComparison {
    /// A precondition failed; the runs cannot be compared for same-source.
    NotComparable(NotComparable),
    /// Comparable, but the runs describe DIFFERENT sources; EVERY differing
    /// coordinate is named (collected, never short-circuited).
    DifferentSource(Vec<SourceDivergence>),
    /// Two independent runs describe the SAME committed source snapshot.
    SameSource(SameSourceProof),
}

impl CrossRunComparison {
    /// The sealed same-source proof, if this comparison found one.
    pub const fn same_source(&self) -> Option<&SameSourceProof> {
        match self {
            CrossRunComparison::SameSource(p) => Some(p),
            _ => None,
        }
    }
    /// Whether the two runs describe the same committed source snapshot.
    pub const fn is_same_source(&self) -> bool {
        matches!(self, CrossRunComparison::SameSource(_))
    }
}

/// The source description a run must expose to enter a cross-run comparison: the
/// four committed git-checkout coordinates plus the independently-recomputed
/// materializer output tree, and the run/toolchain/target it ran under.
struct ComparableRun<'a> {
    commit: GitCommitSha,
    tree: GitTreeSha,
    spec_manifest_digest: Sha256Digest,
    workflow_digest: Sha256Digest,
    materializer_output_tree: Sha256Digest,
    hosted_run: &'a GitHubActionsRunBinding,
    toolchain: &'a ToolchainBinding,
    bootstrap_runtime: &'a BootstrapRuntimeBinding,
    target: RustTargetTriple,
}

/// Why a single qualification cannot enter a comparison.
enum SideReason {
    FrozenExport,
    MissingHostedRun,
}

/// Read one qualification's comparable source description, or the reason it has
/// none. A frozen export lacks committed coordinates; a git checkout without a
/// hosted run is not a hosted run.
fn as_comparable(q: &VerifiedTier0Qualification) -> Result<ComparableRun<'_>, SideReason> {
    let SourceBinding::GitCheckout {
        commit,
        tree,
        spec_manifest_digest,
        workflow_digest,
    } = q.source()
    else {
        return Err(SideReason::FrozenExport);
    };
    let Some(hosted_run) = q.hosted_run() else {
        return Err(SideReason::MissingHostedRun);
    };
    Ok(ComparableRun {
        commit: *commit,
        tree: *tree,
        spec_manifest_digest: *spec_manifest_digest,
        workflow_digest: *workflow_digest,
        materializer_output_tree: q.materializer_output_tree(),
        hosted_run,
        toolchain: q.toolchain(),
        bootstrap_runtime: q.bootstrap_runtime(),
        target: q.target(),
    })
}

/// Two hosted runs are the SAME run iff they share repository, run id, and
/// attempt. A distinct run id or attempt is a distinct, independent witness.
fn same_hosted_run(a: &GitHubActionsRunBinding, b: &GitHubActionsRunBinding) -> bool {
    a.repository == b.repository && a.run_id == b.run_id && a.run_attempt == b.run_attempt
}

/// Compare two verified Tier 0 qualifications for same-source.
///
/// Preconditions first (a frozen export or a missing hosted run cannot be
/// compared; the same run compared to itself is not two witnesses), then the
/// source-identity coordinates — collecting EVERY divergence so a different
/// source explains all of them at once. When every source coordinate agrees, the
/// result is a sealed `SameSourceProof` recording the two attesting runs and the
/// toolchain/target agreement strength.
pub fn compare_runs(
    left: &VerifiedTier0Qualification,
    right: &VerifiedTier0Qualification,
) -> CrossRunComparison {
    let l = as_comparable(left);
    let r = as_comparable(right);

    // Frozen exports cannot be compared, and take precedence: a frozen side has
    // no committed coordinates regardless of any hosted run.
    let left_frozen = matches!(l, Err(SideReason::FrozenExport));
    let right_frozen = matches!(r, Err(SideReason::FrozenExport));
    if left_frozen || right_frozen {
        return CrossRunComparison::NotComparable(NotComparable::FrozenExportSource {
            which: Side::of(left_frozen, right_frozen),
        });
    }
    // A git checkout with no hosted run is not a hosted run.
    let left_norun = matches!(l, Err(SideReason::MissingHostedRun));
    let right_norun = matches!(r, Err(SideReason::MissingHostedRun));
    if left_norun || right_norun {
        return CrossRunComparison::NotComparable(NotComparable::MissingHostedRun {
            which: Side::of(left_norun, right_norun),
        });
    }

    let (Ok(l), Ok(r)) = (l, r) else {
        // Unreachable: every `Err` reason is handled above.
        return CrossRunComparison::NotComparable(NotComparable::FrozenExportSource {
            which: Side::Both,
        });
    };

    // Independence: two DISTINCT hosted runs.
    if same_hosted_run(l.hosted_run, r.hosted_run) {
        return CrossRunComparison::NotComparable(NotComparable::SameHostedRun);
    }

    // Source-identity coordinates: name every one that differs.
    let mut divergences: Vec<SourceDivergence> = Vec::new();
    if l.commit != r.commit {
        divergences.push(SourceDivergence::SourceCommit {
            left: l.commit,
            right: r.commit,
        });
    }
    if l.tree != r.tree {
        divergences.push(SourceDivergence::SourceTree {
            left: l.tree,
            right: r.tree,
        });
    }
    if l.spec_manifest_digest != r.spec_manifest_digest {
        divergences.push(SourceDivergence::SpecManifestDigest {
            left: l.spec_manifest_digest,
            right: r.spec_manifest_digest,
        });
    }
    if l.workflow_digest != r.workflow_digest {
        divergences.push(SourceDivergence::WorkflowDigest {
            left: l.workflow_digest,
            right: r.workflow_digest,
        });
    }
    if l.materializer_output_tree != r.materializer_output_tree {
        divergences.push(SourceDivergence::MaterializerOutputTree {
            left: l.materializer_output_tree,
            right: r.materializer_output_tree,
        });
    }
    if !divergences.is_empty() {
        return CrossRunComparison::DifferentSource(divergences);
    }

    // Same source. Record toolchain/target agreement strength.
    let toolchain_agreement = if l.toolchain == r.toolchain {
        ToolchainAgreement::Identical(*l.toolchain)
    } else {
        ToolchainAgreement::Divergent {
            left: *l.toolchain,
            right: *r.toolchain,
        }
    };
    let target_agreement = if l.target == r.target {
        TargetAgreement::SameTarget(l.target)
    } else {
        TargetAgreement::CrossTarget {
            left: l.target,
            right: r.target,
        }
    };
    let bootstrap_runtime_agreement = if l.bootstrap_runtime == r.bootstrap_runtime {
        BootstrapRuntimeAgreement::Identical(*l.bootstrap_runtime)
    } else {
        BootstrapRuntimeAgreement::Divergent {
            left: *l.bootstrap_runtime,
            right: *r.bootstrap_runtime,
        }
    };

    CrossRunComparison::SameSource(SameSourceProof {
        commit: l.commit,
        tree: l.tree,
        spec_manifest_digest: l.spec_manifest_digest,
        workflow_digest: l.workflow_digest,
        materializer_output_tree: l.materializer_output_tree,
        left_run: l.hosted_run.clone(),
        right_run: r.hosted_run.clone(),
        toolchain_agreement,
        target_agreement,
        bootstrap_runtime_agreement,
        _seal: (),
    })
}

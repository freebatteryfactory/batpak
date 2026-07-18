//! Cross-run same-source comparison over verified Tier 0 qualifications (5.5E6c).
//!
//! E6b sealed a `VerifiedTier0Qualification` that RETAINS every binding. E6c
//! asks the question those retained bindings exist to answer: do two INDEPENDENT
//! hosted runs describe the SAME promoted source?
//!
//! ## The honest boundary, grounded in real evidence
//!
//! Two real hosted MSVC runs of one commit (`8d811dab`, runs `29629647060` and
//! `29629798576`) printed byte-identical source-commit, git-tree, spec-manifest,
//! workflow, and materializer-output-tree digests — but DIFFERENT seedcheck,
//! materialize, and spec-tests EXECUTABLE digests. An MSVC build is not
//! byte-reproducible. A comparator that demanded executable-digest equality would
//! FALSELY reject two legitimately-same-source runs: a dishonest red. So the
//! same-source proof rests only on the deterministic, source-derived
//! coordinates:
//!
//! ```text
//! source commit          committed, git-checkout only
//! source tree            committed, git-checkout only
//! spec-manifest digest   digest of the committed SPEC.sha256
//! workflow digest        digest of the committed hosted workflow
//! materializer output    independently recomputed Gate-0 workspace tree
//! ```
//!
//! The materializer output tree is the load-bearing corroboration: two separate
//! machines each rebuilt the Gate-0 workspace and reduced it to the same bytes.
//! Compiled-artifact (executable) digests legitimately differ across runs and are
//! NEVER a divergence — they are recorded build-nondeterminism, not disagreement.
//!
//! Toolchain and target may also legitimately differ (a re-run after a runner
//! image bump; the same source qualified on a second platform) while the source
//! is unchanged. They are therefore RECORDED as agreement strength, not required
//! for the same-source verdict.
//!
//! ## What this does and does not prove
//!
//! `compare_runs` proves two runs describe the same SOURCE, not that they are
//! byte-identical builds. The `SameSourceProof` seal prevents a consumer from
//! FABRICATING the conclusion; it does not re-prove that either run's inputs were
//! computed honestly — `bootstrap/receiptcheck.rs` already recomputed those, per
//! run, before `verify` sealed each qualification this comparator consumes.

use crate::bootstrap_qualification::{
    GitCommitSha, GitHubActionsRunBinding, GitTreeSha, Sha256Digest, SourceBinding,
    ToolchainBinding, VerifiedTier0Qualification,
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
    const fn of(left: bool, right: bool) -> Side {
        match (left, right) {
            (true, false) => Side::Left,
            (false, true) => Side::Right,
            _ => Side::Both,
        }
    }
}

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

/// Whether two same-source runs qualified the same physical target. A cross-
/// target pair still describes one source — it is a STRONGER statement (the same
/// source qualified on two platforms), and their executable digests differ by
/// construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetAgreement {
    SameTarget(RustTargetTriple),
    CrossTarget {
        left: RustTargetTriple,
        right: RustTargetTriple,
    },
}

/// A sealed proof that two INDEPENDENT hosted runs describe the same promoted
/// source. Obtainable ONLY through `compare_runs`. Retains the proven-common
/// source coordinates, the two DISTINCT run identities that independently
/// attested them, and the toolchain/target agreement strength.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameSourceProof {
    commit: GitCommitSha,
    tree: GitTreeSha,
    spec_manifest_digest: Sha256Digest,
    workflow_digest: Sha256Digest,
    materializer_output_tree: Sha256Digest,
    left_run: GitHubActionsRunBinding,
    right_run: GitHubActionsRunBinding,
    toolchain_agreement: ToolchainAgreement,
    target_agreement: TargetAgreement,
    _seal: (),
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
}

/// The result of comparing two verified qualifications for same-source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrossRunComparison {
    /// A precondition failed; the runs cannot be compared for same-source.
    NotComparable(NotComparable),
    /// Comparable, but the runs describe DIFFERENT sources; EVERY differing
    /// coordinate is named (collected, never short-circuited).
    DifferentSource(Vec<SourceDivergence>),
    /// Two independent runs describe the SAME promoted source.
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
    /// Whether the two runs describe the same promoted source.
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
        _seal: (),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_qualification::{
        verify, CompilationOutcome, ExecutionAttempt, ExecutionOutcome, Tier0ArtifactEvidence,
        Tier0QualificationObservation, Tier0ReceiptKind, Tier0ReceiptObservation,
    };
    use crate::toolchain::{RustRelease, AUTHORITATIVE_TARGET};

    /// The exact source-identity coordinates two real hosted MSVC runs of commit
    /// `8d811dab` printed (runs `29629647060` and `29629798576`). They are the
    /// regression anchor: the comparator must call these two runs same-source
    /// even though their compiled-executable digests differ.
    const REAL_COMMIT: &str = "8d811dabd522492bfac6858b1d698d2014765692";
    const REAL_TREE: &str = "f9ab1895748d901b9de75ac446f0c9d349c21c04";
    const REAL_OUTPUT_TREE: &str =
        "fddae89ac400f549f481bb3c4063002190450df9d67edfd546c6717a3cbf2bb6";
    const REAL_LEFT_RUN: u64 = 29629647060;
    const REAL_RIGHT_RUN: u64 = 29629798576;

    /// Build a verified authoritative (MSVC git-checkout + hosted-run)
    /// qualification with controllable commit/tree/output-tree, run id, and a
    /// per-run executable salt (real MSVC builds are not byte-reproducible).
    fn verified_run(commit: &str, tree: &str, output_tree: &str, run_id: u64, exe: u8) -> VerifiedTier0Qualification {
        let source = SourceBinding::GitCheckout {
            commit: GitCommitSha::from_hex(commit).unwrap(),
            tree: GitTreeSha::from_hex(tree).unwrap(),
            spec_manifest_digest: Sha256Digest::from_bytes([3u8; 32]),
            workflow_digest: Sha256Digest::from_bytes([4u8; 32]),
        };
        let output = Sha256Digest::from_hex(output_tree).unwrap();
        let receipts = Tier0ReceiptKind::ALL
            .iter()
            .map(|&kind| {
                let artifact = match kind.artifact_policy() {
                    crate::bootstrap_qualification::Tier0ArtifactPolicy::FixtureSet => {
                        Tier0ArtifactEvidence::FixtureSet {
                            digest: Sha256Digest::from_bytes([exe; 32]),
                        }
                    }
                    crate::bootstrap_qualification::Tier0ArtifactPolicy::Executable => {
                        Tier0ArtifactEvidence::Executable {
                            digest: Sha256Digest::from_bytes([exe; 32]),
                        }
                    }
                    crate::bootstrap_qualification::Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                        Tier0ArtifactEvidence::ExecutableAndOutputTree {
                            executable_digest: Sha256Digest::from_bytes([exe; 32]),
                            output_tree_digest: output,
                        }
                    }
                };
                Tier0ReceiptObservation {
                    kind,
                    target: AUTHORITATIVE_TARGET,
                    available: true,
                    compilation: CompilationOutcome::Succeeded,
                    execution: ExecutionAttempt::Attempted,
                    outcome: Some(ExecutionOutcome::Passed),
                    artifact: Some(artifact),
                }
            })
            .collect();
        let obs = Tier0QualificationObservation {
            source,
            toolchain: ToolchainBinding {
                rustc_release: RustRelease { major: 1, minor: 97, patch: 0 },
                rustc_commit: crate::bootstrap_qualification::ToolchainCommit::from_bytes([6u8; 20]),
                cargo_release: RustRelease { major: 1, minor: 97, patch: 0 },
                cargo_commit: crate::bootstrap_qualification::ToolchainCommit::from_bytes([6u8; 20]),
                toolchain_file_digest: Sha256Digest::from_bytes([6u8; 32]),
            },
            target: AUTHORITATIVE_TARGET,
            hosted_run: Some(GitHubActionsRunBinding {
                repository: "freebatteryfactory/batpak".to_owned(),
                workflow_path: ".github/workflows/msvc-qualification.yml".to_owned(),
                run_id,
                run_attempt: 1,
                runner_image_os: "Windows".to_owned(),
                runner_image_version: "win25-vs2026/20260714.173".to_owned(),
            }),
            receipts,
        };
        verify(obs).expect("honest authoritative qualification verifies")
    }

    #[test]
    fn two_real_hosted_runs_prove_same_source() {
        // The two runs share every source-identity coordinate but carry
        // DIFFERENT executable digests (exe 1 vs exe 2) — exactly as the real
        // MSVC runs did. The comparator must still call them same-source.
        let left = verified_run(REAL_COMMIT, REAL_TREE, REAL_OUTPUT_TREE, REAL_LEFT_RUN, 1);
        let right = verified_run(REAL_COMMIT, REAL_TREE, REAL_OUTPUT_TREE, REAL_RIGHT_RUN, 2);
        let proof = match compare_runs(&left, &right) {
            CrossRunComparison::SameSource(p) => p,
            other => panic!("expected SameSource for two real runs, got {other:?}"),
        };
        assert_eq!(proof.commit(), GitCommitSha::from_hex(REAL_COMMIT).unwrap());
        assert_eq!(proof.tree(), GitTreeSha::from_hex(REAL_TREE).unwrap());
        assert_eq!(
            proof.materializer_output_tree(),
            Sha256Digest::from_hex(REAL_OUTPUT_TREE).unwrap()
        );
        assert_eq!(proof.left_run().run_id, REAL_LEFT_RUN);
        assert_eq!(proof.right_run().run_id, REAL_RIGHT_RUN);
        assert!(matches!(proof.toolchain_agreement(), ToolchainAgreement::Identical(_)));
        assert!(matches!(proof.target_agreement(), TargetAgreement::SameTarget(_)));
    }

    #[test]
    fn a_different_output_tree_is_named_not_same_source() {
        // Same commit and tree, but the independently-recomputed materializer
        // output tree differs — reproducibility is broken, and the comparator
        // must NAME that coordinate rather than call the runs same-source.
        let real = verified_run(REAL_COMMIT, REAL_TREE, REAL_OUTPUT_TREE, REAL_LEFT_RUN, 1);
        let forged = verified_run(
            REAL_COMMIT,
            REAL_TREE,
            "0000000000000000000000000000000000000000000000000000000000000000",
            REAL_RIGHT_RUN,
            2,
        );
        match compare_runs(&real, &forged) {
            CrossRunComparison::DifferentSource(divs) => assert!(divs
                .iter()
                .any(|d| matches!(d, SourceDivergence::MaterializerOutputTree { .. }))),
            other => panic!("expected DifferentSource naming the output tree, got {other:?}"),
        }
    }

    #[test]
    fn one_run_compared_to_itself_is_not_two_witnesses() {
        let run = verified_run(REAL_COMMIT, REAL_TREE, REAL_OUTPUT_TREE, REAL_LEFT_RUN, 1);
        assert_eq!(
            compare_runs(&run, &run),
            CrossRunComparison::NotComparable(NotComparable::SameHostedRun)
        );
    }
}

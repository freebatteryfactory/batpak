//! Cross-run same-source comparison over verified Tier 0 qualifications (5.5E6c).
//!
//! E6b sealed a `VerifiedTier0Qualification` that RETAINS every binding. E6c
//! asks the question those retained bindings exist to answer: do two INDEPENDENT
//! hosted runs describe the SAME committed source snapshot? A candidate-branch
//! run happens BEFORE promotion, so the corroborated thing is a committed source
//! snapshot, not yet a "promoted" one — E6c1 adds a strictly stronger
//! `confirm_promotion` for the candidate-to-cleanroom promotion question.
//!
//! ## The honest boundary, grounded in real evidence
//!
//! Two real hosted MSVC runs of one commit (`8d811dab`) printed byte-identical
//! source-commit, git-tree, spec-manifest, workflow, and materializer-output-tree
//! digests — but DIFFERENT seedcheck, materialize, and spec-tests EXECUTABLE
//! digests. This is NOT a universal law that "MSVC builds are never
//! byte-reproducible": the current Tier 0 contract simply does not REQUIRE
//! cross-run executable-byte identity, and each run independently binds its own
//! executables. A comparator that demanded executable-digest equality would
//! therefore FALSELY reject two legitimately-same-source runs: a dishonest red. A
//! later reproducible-build law may ADD executable equality as stronger
//! corroboration without redefining today's source identity. So the same-source
//! proof rests only on the deterministic, source-derived coordinates:
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
//!
//! ## Same-source is not promotion confirmation (5.5E6c1)
//!
//! Same-source corroboration and promotion confirmation are DIFFERENT questions,
//! and conflating them would smuggle the conclusion into the premise. Two runs
//! can describe one committed source snapshot across different targets or
//! toolchains (orthogonal corroboration) yet not confirm a candidate-to-cleanroom
//! promotion. `confirm_promotion` is the strictly stronger check: it requires a
//! `SameSourceProof` AND that both runs qualified the AUTHORITATIVE target under
//! the IDENTICAL pinned toolchain in the SAME repository and canonical workflow.
//! Provenance equivalence is not promotion equivalence.

use crate::bootstrap_qualification::{
    BootstrapRuntimeBinding, GitCommitSha, GitHubActionsRunBinding, GitTreeSha, Sha256Digest,
    SourceBinding, ToolchainBinding, VerifiedTier0Qualification, AUTHORITATIVE_WORKFLOW_PATH,
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
    commit: GitCommitSha,
    tree: GitTreeSha,
    spec_manifest_digest: Sha256Digest,
    workflow_digest: Sha256Digest,
    materializer_output_tree: Sha256Digest,
    left_run: GitHubActionsRunBinding,
    right_run: GitHubActionsRunBinding,
    toolchain_agreement: ToolchainAgreement,
    target_agreement: TargetAgreement,
    bootstrap_runtime_agreement: BootstrapRuntimeAgreement,
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
    /// Whether the two runs ran under the same bootstrap runtime — recorded as
    /// corroboration strength, required `Identical` for promotion confirmation.
    pub const fn bootstrap_runtime_agreement(&self) -> &BootstrapRuntimeAgreement {
        &self.bootstrap_runtime_agreement
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

// ===========================================================================
// Promotion confirmation (5.5E6c1): strictly stronger than same-source
// ===========================================================================

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
    same_source: SameSourceProof,
    target: RustTargetTriple,
    toolchain: ToolchainBinding,
    bootstrap_runtime: BootstrapRuntimeBinding,
    candidate_run: GitHubActionsRunBinding,
    cleanroom_run: GitHubActionsRunBinding,
    _seal: (),
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

/// Why two verified qualifications do NOT confirm a promotion — even when they
/// may still corroborate the same committed source snapshot. Each arm names the
/// exact posture requirement promotion confirmation adds over same-source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromotionConfirmationError {
    /// The two runs are not even same-source. Promotion confirmation is strictly
    /// stronger, so it inherits every same-source refusal; the underlying
    /// comparison (a divergence or a not-comparable precondition) is carried.
    NotSameSource(CrossRunComparison),
    /// A side qualified a NON-authoritative target. Promotion confirms only the
    /// authoritative target; a supplemental target never promotes.
    NonAuthoritativeTarget {
        which: Side,
        target: RustTargetTriple,
    },
    /// The two runs qualified DIFFERENT targets. A cross-target pair is orthogonal
    /// corroboration of one source, not a confirmation of one promotion.
    CrossTarget {
        candidate: RustTargetTriple,
        cleanroom: RustTargetTriple,
    },
    /// The two runs ran on DIFFERENT toolchains. Same-source tolerates this;
    /// promotion confirmation requires the identical pinned toolchain on both.
    ToolchainDivergent {
        candidate: ToolchainBinding,
        cleanroom: ToolchainBinding,
    },
    /// The two runs ran under DIFFERENT bootstrap runtimes. Same-source tolerates
    /// this; promotion confirmation requires the identical pinned CPython release.
    BootstrapRuntimeDivergent {
        candidate: BootstrapRuntimeBinding,
        cleanroom: BootstrapRuntimeBinding,
    },
    /// The two runs ran in DIFFERENT repositories.
    RepositoryMismatch {
        candidate: String,
        cleanroom: String,
    },
    /// A run's workflow path is not the canonical authoritative workflow
    /// (`AUTHORITATIVE_WORKFLOW_PATH`). `which` names the offending side(s).
    NonCanonicalWorkflowPath { which: Side, path: String },
}

/// Confirm that a candidate run and a cleanroom run promote one committed source
/// snapshot to the authoritative target.
///
/// Same-source is necessary but not sufficient: this additionally requires the
/// authoritative target on BOTH sides, the identical pinned toolchain, the same
/// repository, and the canonical authoritative workflow path — the posture the
/// candidate-exact-SHA and cleanroom-exact-SHA runs share across a lawful
/// fast-forward. The two runs are already DISTINCT (same-source refuses one run
/// compared against itself), and each qualification was already independently
/// verified before `verify` sealed it (and, on the wire, recomputed by
/// `bootstrap/receiptcheck.rs`). `candidate` becomes the left run and `cleanroom`
/// the right run of the retained same-source proof.
pub fn confirm_promotion(
    candidate: &VerifiedTier0Qualification,
    cleanroom: &VerifiedTier0Qualification,
) -> Result<PromotionConfirmationProof, PromotionConfirmationError> {
    // Same-source first — promotion confirmation inherits every same-source arm.
    let comparison = compare_runs(candidate, cleanroom);
    let CrossRunComparison::SameSource(same_source) = comparison else {
        return Err(PromotionConfirmationError::NotSameSource(comparison));
    };

    // Both sides must qualify the authoritative target.
    let cand_target = candidate.target();
    let clean_target = cleanroom.target();
    let cand_auth = cand_target.is_authoritative();
    let clean_auth = clean_target.is_authoritative();
    if !cand_auth || !clean_auth {
        return Err(PromotionConfirmationError::NonAuthoritativeTarget {
            which: Side::of(!cand_auth, !clean_auth),
            target: if !cand_auth { cand_target } else { clean_target },
        });
    }
    // Both authoritative implies the same target today (one authoritative target
    // exists), but name a cross-target pair explicitly so this stays honest if a
    // second authoritative target is ever admitted.
    if cand_target != clean_target {
        return Err(PromotionConfirmationError::CrossTarget {
            candidate: cand_target,
            cleanroom: clean_target,
        });
    }

    // Identical pinned toolchain — recorded, not merely observed, by same-source.
    let ToolchainAgreement::Identical(toolchain) = *same_source.toolchain_agreement() else {
        return Err(PromotionConfirmationError::ToolchainDivergent {
            candidate: *candidate.toolchain(),
            cleanroom: *cleanroom.toolchain(),
        });
    };

    // Identical pinned bootstrap runtime (CPython release) — the interpreter that
    // executed the Python gates is builder identity beside rustc/cargo.
    let BootstrapRuntimeAgreement::Identical(bootstrap_runtime) =
        *same_source.bootstrap_runtime_agreement()
    else {
        return Err(PromotionConfirmationError::BootstrapRuntimeDivergent {
            candidate: *candidate.bootstrap_runtime(),
            cleanroom: *cleanroom.bootstrap_runtime(),
        });
    };

    // Same repository, canonical authoritative workflow, on both distinct runs.
    let cand_run = same_source.left_run();
    let clean_run = same_source.right_run();
    if cand_run.repository != clean_run.repository {
        return Err(PromotionConfirmationError::RepositoryMismatch {
            candidate: cand_run.repository.clone(),
            cleanroom: clean_run.repository.clone(),
        });
    }
    let cand_canon = cand_run.workflow_path == AUTHORITATIVE_WORKFLOW_PATH;
    let clean_canon = clean_run.workflow_path == AUTHORITATIVE_WORKFLOW_PATH;
    if !cand_canon || !clean_canon {
        return Err(PromotionConfirmationError::NonCanonicalWorkflowPath {
            which: Side::of(!cand_canon, !clean_canon),
            path: if !cand_canon {
                cand_run.workflow_path.clone()
            } else {
                clean_run.workflow_path.clone()
            },
        });
    }

    // Clone the run identities into owned values so the same-source proof can be
    // moved into the confirmation without an outstanding borrow.
    let candidate_run = cand_run.clone();
    let cleanroom_run = clean_run.clone();
    Ok(PromotionConfirmationProof {
        same_source,
        target: cand_target,
        toolchain,
        bootstrap_runtime,
        candidate_run,
        cleanroom_run,
        _seal: (),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap_qualification::{
        verify, BootstrapRuntimeBinding, CompilationOutcome, ExecutionAttempt, ExecutionOutcome,
        PythonRelease, Tier0ArtifactEvidence, Tier0ArtifactPolicy, Tier0QualificationObservation,
        Tier0ReceiptKind, Tier0ReceiptObservation, Tier0VerificationRule, ToolchainCommit,
    };
    use crate::toolchain::{RustRelease, AUTHORITATIVE_TARGET};

    const REPO: &str = "owner/repo";

    /// A fully SYNTHETIC run description: every coordinate is an invented dial.
    /// No historical commit, run id, or runner image is asserted to be a real
    /// artifact here — these tests prove the comparator/confirmation LOGIC.
    /// Actual hosted run-pair evidence is proven by the workflow integration path
    /// (`receiptcheck compare` over two uploaded bundles), not by a unit test; a
    /// unit fixture cannot masquerade as a hosted comparison.
    #[derive(Clone, Copy)]
    struct RunSpec {
        commit: u8,
        tree: u8,
        manifest: u8,
        workflow: u8,
        otree: u8,
        tc: u8,
        run_id: u64,
        target: RustTargetTriple,
        repository: &'static str,
        workflow_path: &'static str,
        exe: u8,
        python: (u16, u16, u16),
    }

    impl RunSpec {
        /// The canonical authoritative run: git checkout, MSVC, hosted, canonical
        /// workflow, the authoritative CPython release. Tests perturb one dial
        /// with struct-update syntax.
        const fn base() -> RunSpec {
            RunSpec {
                commit: 1,
                tree: 2,
                manifest: 3,
                workflow: 4,
                otree: 5,
                tc: 6,
                run_id: 100,
                target: AUTHORITATIVE_TARGET,
                repository: REPO,
                workflow_path: AUTHORITATIVE_WORKFLOW_PATH,
                exe: 7,
                python: (3, 12, 10),
            }
        }
    }

    /// The raw observation for a synthetic run description.
    fn observe(s: RunSpec) -> Tier0QualificationObservation {
        let source = SourceBinding::GitCheckout {
            commit: GitCommitSha::from_bytes([s.commit; 20]),
            tree: GitTreeSha::from_bytes([s.tree; 20]),
            spec_manifest_digest: Sha256Digest::from_bytes([s.manifest; 32]),
            workflow_digest: Sha256Digest::from_bytes([s.workflow; 32]),
        };
        let receipts = Tier0ReceiptKind::ALL
            .iter()
            .map(|&kind| {
                let artifact = match kind.artifact_policy() {
                    Tier0ArtifactPolicy::FixtureSet => Tier0ArtifactEvidence::FixtureSet {
                        digest: Sha256Digest::from_bytes([s.exe; 32]),
                    },
                    Tier0ArtifactPolicy::Executable => Tier0ArtifactEvidence::Executable {
                        digest: Sha256Digest::from_bytes([s.exe; 32]),
                    },
                    Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                        Tier0ArtifactEvidence::ExecutableAndOutputTree {
                            executable_digest: Sha256Digest::from_bytes([s.exe; 32]),
                            output_tree_digest: Sha256Digest::from_bytes([s.otree; 32]),
                        }
                    }
                };
                Tier0ReceiptObservation {
                    kind,
                    target: s.target,
                    available: true,
                    compilation: CompilationOutcome::Succeeded,
                    execution: ExecutionAttempt::Attempted,
                    outcome: Some(ExecutionOutcome::Passed),
                    artifact: Some(artifact),
                }
            })
            .collect();
        Tier0QualificationObservation {
            source,
            toolchain: ToolchainBinding {
                rustc_release: RustRelease { major: 1, minor: 97, patch: 0 },
                rustc_commit: ToolchainCommit::from_bytes([s.tc; 20]),
                cargo_release: RustRelease { major: 1, minor: 97, patch: 0 },
                cargo_commit: ToolchainCommit::from_bytes([s.tc; 20]),
                toolchain_file_digest: Sha256Digest::from_bytes([s.tc; 32]),
            },
            bootstrap_runtime: BootstrapRuntimeBinding {
                python_release: PythonRelease {
                    major: s.python.0,
                    minor: s.python.1,
                    patch: s.python.2,
                },
            },
            target: s.target,
            hosted_run: Some(GitHubActionsRunBinding {
                repository: s.repository.to_owned(),
                workflow_path: s.workflow_path.to_owned(),
                run_id: s.run_id,
                run_attempt: 1,
                runner_image_os: "synthetic".to_owned(),
                runner_image_version: "synthetic".to_owned(),
            }),
            receipts,
        }
    }

    /// Build a verified qualification from a synthetic run description, through
    /// the sealed `verify`.
    fn verified_run(s: RunSpec) -> VerifiedTier0Qualification {
        verify(observe(s)).expect("synthetic authoritative qualification verifies")
    }

    #[test]
    fn synthetic_same_source_runs_allow_different_executable_digests() {
        // Two runs share every source-identity coordinate but carry DIFFERENT
        // executable digests (exe 7 vs 8). The comparator must still call them
        // same-source: the current contract requires no executable-byte identity.
        let left = verified_run(RunSpec::base());
        let right = verified_run(RunSpec { run_id: 200, exe: 8, ..RunSpec::base() });
        let proof = match compare_runs(&left, &right) {
            CrossRunComparison::SameSource(p) => p,
            other => panic!("expected SameSource, got {other:?}"),
        };
        assert_eq!(proof.left_run().run_id, 100);
        assert_eq!(proof.right_run().run_id, 200);
        assert!(matches!(proof.toolchain_agreement(), ToolchainAgreement::Identical(_)));
        assert!(matches!(proof.target_agreement(), TargetAgreement::SameTarget(_)));
    }

    #[test]
    fn a_different_output_tree_is_named_not_same_source() {
        // Same commit and tree, but the independently-recomputed materializer
        // output tree differs — the comparator must NAME that coordinate.
        let left = verified_run(RunSpec::base());
        let right = verified_run(RunSpec { run_id: 200, otree: 9, exe: 8, ..RunSpec::base() });
        match compare_runs(&left, &right) {
            CrossRunComparison::DifferentSource(divs) => assert!(divs
                .iter()
                .any(|d| matches!(d, SourceDivergence::MaterializerOutputTree { .. }))),
            other => panic!("expected DifferentSource naming the output tree, got {other:?}"),
        }
    }

    #[test]
    fn one_run_compared_to_itself_is_not_two_witnesses() {
        let run = verified_run(RunSpec::base());
        assert_eq!(
            compare_runs(&run, &run),
            CrossRunComparison::NotComparable(NotComparable::SameHostedRun)
        );
    }

    #[test]
    fn same_source_records_bootstrap_runtime_agreement() {
        // A supplemental GNU run under a different CPython release still describes
        // the same committed source snapshot; the comparator RECORDS the runtime
        // divergence as corroboration strength without defeating same-source
        // (builder identity, not a source coordinate).
        let authoritative = verified_run(RunSpec::base());
        let supplemental = verified_run(RunSpec {
            run_id: 200,
            target: RustTargetTriple::X86_64PcWindowsGnu,
            python: (3, 11, 9),
            exe: 8,
            ..RunSpec::base()
        });
        match compare_runs(&authoritative, &supplemental) {
            CrossRunComparison::SameSource(p) => assert!(matches!(
                p.bootstrap_runtime_agreement(),
                BootstrapRuntimeAgreement::Divergent { .. }
            )),
            other => panic!("expected SameSource across lanes, got {other:?}"),
        }
    }

    #[test]
    fn authoritative_target_wrong_python_runtime_is_rejected() {
        // The authoritative target must run under AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE.
        // A different CPython release is refused at verification, so two runs on the
        // same WRONG runtime can never both verify and slip past an equality check.
        let errors = verify(observe(RunSpec { python: (3, 11, 9), ..RunSpec::base() }))
            .expect_err("a wrong authoritative Python runtime must be refused");
        assert!(errors
            .iter()
            .any(|e| e.rule() == Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch));
    }

    // ---- promotion confirmation (5.5E6c1) ----

    #[test]
    fn two_authoritative_runs_confirm_promotion() {
        // The candidate-exact-SHA and cleanroom-exact-SHA posture: same source,
        // authoritative target, identical toolchain, same repository, canonical
        // workflow, two distinct runs. Executable digests still differ.
        let candidate = verified_run(RunSpec::base());
        let cleanroom = verified_run(RunSpec { run_id: 200, exe: 8, ..RunSpec::base() });
        let proof = confirm_promotion(&candidate, &cleanroom)
            .expect("two authoritative same-source runs confirm promotion");
        assert_eq!(proof.target(), AUTHORITATIVE_TARGET);
        assert_eq!(proof.candidate_run().run_id, 100);
        assert_eq!(proof.cleanroom_run().run_id, 200);
        assert_eq!(
            proof.bootstrap_runtime().python_release,
            PythonRelease { major: 3, minor: 12, patch: 10 }
        );
    }

    #[test]
    fn different_run_ids_are_required() {
        // One run confirmed against itself is not two witnesses; promotion
        // confirmation inherits the same-source refusal.
        let run = verified_run(RunSpec::base());
        assert!(matches!(
            confirm_promotion(&run, &run),
            Err(PromotionConfirmationError::NotSameSource(
                CrossRunComparison::NotComparable(NotComparable::SameHostedRun)
            ))
        ));
    }

    #[test]
    fn different_executable_digests_are_lawful() {
        // The dedicated statement of the honest boundary: an executable-digest
        // difference alone neither defeats same-source nor promotion confirmation.
        let candidate = verified_run(RunSpec::base());
        let cleanroom = verified_run(RunSpec { run_id: 200, exe: 200, ..RunSpec::base() });
        assert!(confirm_promotion(&candidate, &cleanroom).is_ok());
    }

    #[test]
    fn same_source_cross_target_does_not_confirm_promotion() {
        // The same source on a second (supplemental GNU) target still corroborates
        // same-source, but the non-authoritative side cannot confirm a promotion.
        let candidate = verified_run(RunSpec::base());
        let cleanroom = verified_run(RunSpec {
            run_id: 200,
            target: RustTargetTriple::X86_64PcWindowsGnu,
            exe: 8,
            ..RunSpec::base()
        });
        assert!(compare_runs(&candidate, &cleanroom).is_same_source());
        assert!(matches!(
            confirm_promotion(&candidate, &cleanroom),
            Err(PromotionConfirmationError::NonAuthoritativeTarget { which: Side::Right, .. })
        ));
    }

    #[test]
    fn same_source_different_toolchain_does_not_confirm_promotion() {
        let candidate = verified_run(RunSpec::base());
        let cleanroom = verified_run(RunSpec { run_id: 200, tc: 99, exe: 8, ..RunSpec::base() });
        assert!(compare_runs(&candidate, &cleanroom).is_same_source());
        assert!(matches!(
            confirm_promotion(&candidate, &cleanroom),
            Err(PromotionConfirmationError::ToolchainDivergent { .. })
        ));
    }

    #[test]
    fn same_source_different_repository_does_not_confirm_promotion() {
        let candidate = verified_run(RunSpec::base());
        let cleanroom =
            verified_run(RunSpec { run_id: 200, repository: "other/repo", exe: 8, ..RunSpec::base() });
        assert!(compare_runs(&candidate, &cleanroom).is_same_source());
        assert!(matches!(
            confirm_promotion(&candidate, &cleanroom),
            Err(PromotionConfirmationError::RepositoryMismatch { .. })
        ));
    }

    #[test]
    fn same_source_different_workflow_path_does_not_confirm_promotion() {
        let candidate = verified_run(RunSpec::base());
        let cleanroom = verified_run(RunSpec {
            run_id: 200,
            workflow_path: ".github/workflows/not-canonical.yml",
            exe: 8,
            ..RunSpec::base()
        });
        assert!(compare_runs(&candidate, &cleanroom).is_same_source());
        assert!(matches!(
            confirm_promotion(&candidate, &cleanroom),
            Err(PromotionConfirmationError::NonCanonicalWorkflowPath { which: Side::Right, .. })
        ));
    }
}

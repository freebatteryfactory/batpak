use crate::bootstrap_qualification::{
    AUTHORITATIVE_WORKFLOW_PATH, GitCommitSha, GitHubActionsRunBinding, GitTreeSha, Sha256Digest,
    SourceBinding, ToolchainBinding, VerifiedTier0Qualification,
};
use crate::toolchain::RustTargetTriple;

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

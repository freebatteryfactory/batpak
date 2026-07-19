use spec::{bootstrap_qualification, tier0_cross_run, toolchain};

/// The cross-run same-source comparator, EXECUTED (5.5E6c). Seedcheck builds
/// verified qualifications through the sealed `verify` and runs `compare_runs`
/// across every outcome: two independent runs of one source prove SameSource;
/// each source-identity coordinate, perturbed alone, is named as a divergence;
/// a frozen export, a missing hosted run, and a run compared to itself are each
/// refused as NotComparable; and a differing toolchain or target is RECORDED as
/// agreement strength while the same-source verdict still holds. The design is
/// grounded in real evidence: two hosted MSVC runs of one commit differ in their
/// executable digests but agree on every source coordinate, so executable
/// digests are deliberately varied here and never treated as a divergence.
pub(crate) fn check_tier0_cross_run(findings: &mut Vec<String>) {
    use bootstrap_qualification as bq;
    use tier0_cross_run as xr;
    use toolchain::{RustRelease, RustTargetTriple, AUTHORITATIVE_TARGET};

    // A controllable qualification: each source-identity coordinate, the
    // toolchain, the run identity, the target, the source posture, and the
    // (same-source-irrelevant) executable digest is a dial.
    #[derive(Clone, Copy)]
    struct Spec {
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
        git: bool,
        hosted: bool,
        exe: u8,
        python: (u16, u16, u16),
    }

    let evidence = |kind: bq::Tier0ReceiptKind, exe: u8, otree: u8| -> bq::Tier0ArtifactEvidence {
        match kind.artifact_policy() {
            bq::Tier0ArtifactPolicy::FixtureSet => bq::Tier0ArtifactEvidence::FixtureSet {
                digest: bq::Sha256Digest::from_bytes([exe; 32]),
            },
            bq::Tier0ArtifactPolicy::Executable => bq::Tier0ArtifactEvidence::Executable {
                digest: bq::Sha256Digest::from_bytes([exe; 32]),
            },
            bq::Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                bq::Tier0ArtifactEvidence::ExecutableAndOutputTree {
                    executable_digest: bq::Sha256Digest::from_bytes([exe; 32]),
                    output_tree_digest: bq::Sha256Digest::from_bytes([otree; 32]),
                }
            }
        }
    };

    let verified = |s: &Spec,
                    label: &str,
                    findings: &mut Vec<String>|
     -> Option<bq::VerifiedTier0Qualification> {
        let source = if s.git {
            bq::SourceBinding::GitCheckout {
                commit: bq::GitCommitSha::from_bytes([s.commit; 20]),
                tree: bq::GitTreeSha::from_bytes([s.tree; 20]),
                spec_manifest_digest: bq::Sha256Digest::from_bytes([s.manifest; 32]),
                workflow_digest: bq::Sha256Digest::from_bytes([s.workflow; 32]),
            }
        } else {
            bq::SourceBinding::FrozenExport {
                spec_manifest_digest: bq::Sha256Digest::from_bytes([s.manifest; 32]),
                export_tree_digest: bq::Sha256Digest::from_bytes([s.tree; 32]),
            }
        };
        let hosted_run = if s.hosted {
            Some(bq::GitHubActionsRunBinding {
                repository: s.repository.to_owned(),
                workflow_path: s.workflow_path.to_owned(),
                run_id: s.run_id,
                run_attempt: 1,
                runner_image_os: "Windows".to_owned(),
                runner_image_version: "0".to_owned(),
            })
        } else {
            None
        };
        let toolchain = bq::ToolchainBinding {
            rustc_release: RustRelease { major: 1, minor: 97, patch: 1 },
            rustc_commit: bq::ToolchainCommit::from_bytes([s.tc; 20]),
            cargo_release: RustRelease { major: 1, minor: 97, patch: 1 },
            cargo_commit: bq::ToolchainCommit::from_bytes([s.tc; 20]),
            toolchain_file_digest: bq::Sha256Digest::from_bytes([s.tc; 32]),
        };
        let obs = bq::Tier0QualificationObservation {
            source,
            toolchain,
            bootstrap_runtime: bq::BootstrapRuntimeBinding {
                python_release: bq::PythonRelease {
                    major: s.python.0,
                    minor: s.python.1,
                    patch: s.python.2,
                },
            },
            target: s.target,
            hosted_run,
            receipts: bq::Tier0ReceiptKind::ALL
                .iter()
                .map(|&k| bq::Tier0ReceiptObservation {
                    kind: k,
                    target: s.target,
                    available: true,
                    compilation: bq::CompilationOutcome::Succeeded,
                    execution: bq::ExecutionAttempt::Attempted,
                    outcome: Some(bq::ExecutionOutcome::Passed),
                    artifact: Some(evidence(k, s.exe, s.otree)),
                })
                .collect(),
        };
        match bq::verify(obs) {
            Ok(v) => Some(v),
            Err(errs) => {
                let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
                findings.push(format!("cross-run fixture {label} did not verify: {rules:?}"));
                None
            }
        }
    };

    let base = Spec {
        commit: 1,
        tree: 2,
        manifest: 3,
        workflow: 4,
        otree: 5,
        tc: 6,
        run_id: 100,
        target: AUTHORITATIVE_TARGET,
        repository: "owner/repo",
        workflow_path: bq::AUTHORITATIVE_WORKFLOW_PATH,
        git: true,
        hosted: true,
        exe: 7,
        python: (3, 12, 10),
    };

    let compare = |right: &Spec,
                   label: &str,
                   findings: &mut Vec<String>|
     -> Option<xr::CrossRunComparison> {
        let l = verified(&base, label, findings)?;
        let r = verified(right, label, findings)?;
        Some(xr::compare_runs(&l, &r))
    };

    // Two independent runs of one source: SameSource, retaining the common
    // coordinates and recording identical toolchain and target. The executable
    // digest differs (exe 77 vs base 7) and MUST NOT block the verdict.
    let right = Spec { run_id: 200, exe: 77, ..base };
    if let Some(cmp) = compare(&right, "same-source", findings) {
        match cmp {
            xr::CrossRunComparison::SameSource(p) => {
                if p.materializer_output_tree() != bq::Sha256Digest::from_bytes([5u8; 32]) {
                    findings.push("cross-run same-source retained the wrong output tree".to_owned());
                }
                if !matches!(p.toolchain_agreement(), xr::ToolchainAgreement::Identical(_)) {
                    findings.push("cross-run same-source did not record identical toolchain".to_owned());
                }
                if !matches!(p.target_agreement(), xr::TargetAgreement::SameTarget(_)) {
                    findings.push("cross-run same-source did not record the same target".to_owned());
                }
                if p.left_run().run_id == p.right_run().run_id {
                    findings.push("cross-run same-source retained one run identity twice".to_owned());
                }
            }
            other => findings.push(format!("cross-run same-source expected SameSource, got {other:?}")),
        }
    }

    // Each source-identity coordinate, perturbed ALONE (with a distinct run so
    // the independence precondition passes), is named as its own divergence.
    let divergence_cases: &[(Spec, &str)] = &[
        (Spec { run_id: 200, commit: 9, ..base }, "commit"),
        (Spec { run_id: 200, tree: 9, ..base }, "tree"),
        (Spec { run_id: 200, manifest: 9, ..base }, "spec-manifest"),
        (Spec { run_id: 200, workflow: 9, ..base }, "workflow"),
        (Spec { run_id: 200, otree: 9, ..base }, "output-tree"),
    ];
    for (right, coord) in divergence_cases {
        if let Some(cmp) = compare(right, coord, findings) {
            match cmp {
                xr::CrossRunComparison::DifferentSource(divs) => {
                    let named = divs.iter().any(|d| match (coord, d) {
                        (&"commit", xr::SourceDivergence::SourceCommit { .. }) => true,
                        (&"tree", xr::SourceDivergence::SourceTree { .. }) => true,
                        (&"spec-manifest", xr::SourceDivergence::SpecManifestDigest { .. }) => true,
                        (&"workflow", xr::SourceDivergence::WorkflowDigest { .. }) => true,
                        (&"output-tree", xr::SourceDivergence::MaterializerOutputTree { .. }) => true,
                        _ => false,
                    });
                    if !named {
                        findings.push(format!(
                            "cross-run {coord} divergence was not named: {divs:?}"
                        ));
                    }
                }
                other => findings.push(format!(
                    "cross-run {coord} divergence expected DifferentSource, got {other:?}"
                )),
            }
        }
    }

    // A frozen export cannot be compared: no committed source coordinates.
    let frozen = Spec {
        run_id: 200,
        git: false,
        hosted: false,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        ..base
    };
    match compare(&frozen, "frozen-export", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::FrozenExportSource {
            which: xr::Side::Right,
        })) => {}
        Some(other) => findings.push(format!(
            "cross-run frozen export expected NotComparable::FrozenExportSource, got {other:?}"
        )),
        None => {}
    }

    // A git checkout with no hosted run is not one of two hosted runs.
    let no_run = Spec {
        run_id: 200,
        hosted: false,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        ..base
    };
    match compare(&no_run, "missing-hosted-run", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::MissingHostedRun {
            which: xr::Side::Right,
        })) => {}
        Some(other) => findings.push(format!(
            "cross-run missing hosted run expected NotComparable::MissingHostedRun, got {other:?}"
        )),
        None => {}
    }

    // One run compared to itself (same repository, run id, and attempt) is not
    // two independent witnesses.
    let same_run = base;
    match compare(&same_run, "same-hosted-run", findings) {
        Some(xr::CrossRunComparison::NotComparable(xr::NotComparable::SameHostedRun)) => {}
        Some(other) => findings.push(format!(
            "cross-run same hosted run expected NotComparable::SameHostedRun, got {other:?}"
        )),
        None => {}
    }

    // A differing toolchain does not weaken the same-source verdict; it is
    // recorded as divergent agreement strength.
    let other_toolchain = Spec { run_id: 200, tc: 99, ..base };
    match compare(&other_toolchain, "toolchain-divergent", findings) {
        Some(xr::CrossRunComparison::SameSource(p)) => {
            if !matches!(p.toolchain_agreement(), xr::ToolchainAgreement::Divergent { .. }) {
                findings.push("cross-run differing toolchain was not recorded as divergent".to_owned());
            }
        }
        Some(other) => findings.push(format!(
            "cross-run differing toolchain expected SameSource, got {other:?}"
        )),
        None => {}
    }

    // The same source qualified on a second target is still SameSource, recorded
    // as a cross-target pair. A GNU git checkout WITH a hosted run is comparable,
    // and a supplemental lane may run under a different (non-authoritative)
    // CPython release — recorded as a divergent bootstrap-runtime agreement.
    let cross_target = Spec {
        run_id: 200,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        exe: 55,
        python: (3, 11, 9),
        ..base
    };
    match compare(&cross_target, "cross-target", findings) {
        Some(xr::CrossRunComparison::SameSource(p)) => {
            if !matches!(p.target_agreement(), xr::TargetAgreement::CrossTarget { .. }) {
                findings.push("cross-run cross-target pair was not recorded as cross-target".to_owned());
            }
            if !matches!(
                p.bootstrap_runtime_agreement(),
                xr::BootstrapRuntimeAgreement::Divergent { .. }
            ) {
                findings.push("cross-run differing bootstrap runtime was not recorded as divergent".to_owned());
            }
        }
        Some(other) => findings.push(format!(
            "cross-run cross-target expected SameSource, got {other:?}"
        )),
        None => {}
    }

    // Promotion confirmation (5.5E6c1), EXECUTED: strictly stronger than
    // same-source. The canonical candidate/cleanroom posture confirms; each
    // missing posture requirement refuses even where same-source still holds.
    let confirm = |right: &Spec,
                   label: &str,
                   findings: &mut Vec<String>|
     -> Option<Result<xr::PromotionConfirmationProof, xr::PromotionConfirmationError>> {
        let c = verified(&base, label, findings)?;
        let r = verified(right, label, findings)?;
        Some(xr::confirm_promotion(&c, &r))
    };

    // Two authoritative, same-source, identical-toolchain runs in the same
    // repository and canonical workflow confirm the promotion. The executable
    // digest still differs (exe 77 vs base 7).
    let promote_ok = Spec { run_id: 200, exe: 77, ..base };
    match confirm(&promote_ok, "confirm-ok", findings) {
        Some(Ok(p)) => {
            if !p.target().is_authoritative() {
                findings.push("promotion confirmation did not bind the authoritative target".to_owned());
            }
            if p.candidate_run().run_id == p.cleanroom_run().run_id {
                findings.push("promotion confirmation retained one run identity twice".to_owned());
            }
        }
        Some(Err(e)) => findings.push(format!("promotion confirmation refused an honest pair: {e:?}")),
        None => {}
    }

    // One run confirmed against itself is not two witnesses; promotion
    // confirmation inherits the same-source refusal.
    match confirm(&base, "confirm-same-run", findings) {
        Some(Err(xr::PromotionConfirmationError::NotSameSource(
            xr::CrossRunComparison::NotComparable(xr::NotComparable::SameHostedRun),
        ))) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation of one run against itself expected NotSameSource/SameHostedRun, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair on a non-authoritative (supplemental GNU) target does
    // not confirm a promotion.
    let promote_gnu = Spec {
        run_id: 200,
        target: RustTargetTriple::X86_64PcWindowsGnu,
        exe: 55,
        ..base
    };
    match confirm(&promote_gnu, "confirm-non-authoritative", findings) {
        Some(Err(xr::PromotionConfirmationError::NonAuthoritativeTarget {
            which: xr::Side::Right,
            ..
        })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation on a non-authoritative target expected NonAuthoritativeTarget, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair on a divergent toolchain does not confirm.
    let promote_tc = Spec { run_id: 200, tc: 99, ..base };
    match confirm(&promote_tc, "confirm-toolchain", findings) {
        Some(Err(xr::PromotionConfirmationError::ToolchainDivergent { .. })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation on a divergent toolchain expected ToolchainDivergent, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair in a different repository does not confirm.
    let promote_repo = Spec { run_id: 200, repository: "other/repo", ..base };
    match confirm(&promote_repo, "confirm-repository", findings) {
        Some(Err(xr::PromotionConfirmationError::RepositoryMismatch { .. })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation across repositories expected RepositoryMismatch, got {other:?}"
        )),
        None => {}
    }

    // A same-source pair whose workflow path is not the canonical authoritative
    // workflow does not confirm.
    let promote_wf = Spec {
        run_id: 200,
        workflow_path: ".github/workflows/not-canonical.yml",
        ..base
    };
    match confirm(&promote_wf, "confirm-workflow-path", findings) {
        Some(Err(xr::PromotionConfirmationError::NonCanonicalWorkflowPath {
            which: xr::Side::Right,
            ..
        })) => {}
        Some(other) => findings.push(format!(
            "promotion confirmation with a non-canonical workflow path expected NonCanonicalWorkflowPath, got {other:?}"
        )),
        None => {}
    }
}


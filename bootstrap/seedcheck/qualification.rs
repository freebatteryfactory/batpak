use spec::{architecture, bootstrap_output, bootstrap_qualification, toolchain};
use std::collections::BTreeSet;

/// The Gate-0 materializer output shape, EXECUTED from the typed plan
/// (5.5E5). Seedcheck expands the same closed inventories the materializer
/// and the independent selftest oracle consume; it renders no TOML and
/// inspects no filesystem output -- the plan's internal lawfulness is the
/// subject here, and the isolated qualification owns the published tree.
pub(crate) fn check_bootstrap_output(findings: &mut Vec<String>) {
    use bootstrap_output as bo;

    // Each closed vocabulary is generated with its ALL inventory from one
    // variant list (5.5E5a), so enum-to-ALL divergence is unrepresentable and
    // no cardinality is asserted here. What remains executable: the inventory
    // is nonempty and carries no duplicate. A duplicate would already be a
    // compile error in the enum, so this is belt-and-braces evidence, count
    // free.
    for (name, len, dup) in [
        ("Gate0RootArtifact", bo::Gate0RootArtifact::ALL.len(), {
            let a = bo::Gate0RootArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PackageArtifact", bo::Gate0PackageArtifact::ALL.len(), {
            let a = bo::Gate0PackageArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PackageTargetKind", bo::Gate0PackageTargetKind::ALL.len(), {
            let a = bo::Gate0PackageTargetKind::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0PlaneArtifact", bo::Gate0PlaneArtifact::ALL.len(), {
            let a = bo::Gate0PlaneArtifact::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
        ("Gate0OutputDisposition", bo::Gate0OutputDisposition::ALL.len(), {
            let a = bo::Gate0OutputDisposition::ALL;
            (0..a.len()).any(|i| ((i + 1)..a.len()).any(|j| a[i] == a[j]))
        }),
    ] {
        if len == 0 {
            findings.push(format!("{name}::ALL is empty"));
        }
        if dup {
            findings.push(format!("{name}::ALL lists a variant twice"));
        }
    }

    // Workspace metadata is nonempty.
    if bo::WORKSPACE_LICENSE.trim().is_empty() {
        findings.push("bootstrap-output workspace license is empty".into());
    }
    if bo::WORKSPACE_REPOSITORY.trim().is_empty() {
        findings.push("bootstrap-output workspace repository is empty".into());
    }

    // Expand the complete plan's PATHS exactly as the materializer does:
    // roots, then three files per package, then two files per plane. Every
    // path must be nonempty, relative, traversal-free, and unique.
    let mut planned: Vec<String> = Vec::new();
    for &artifact in bo::Gate0RootArtifact::ALL {
        planned.push(artifact.relative_path().to_owned());
    }
    let mut roots: BTreeSet<&str> = BTreeSet::new();
    for &artifact in bo::Gate0RootArtifact::ALL {
        if !roots.insert(artifact.relative_path()) {
            findings.push(format!("duplicate Gate0 root path {}", artifact.relative_path()));
        }
    }
    for &package in architecture::PackageId::ALL {
        let base = package.workspace_path();
        // one target kind per package (total function), and a binary target
        // name exactly where the kind requires one
        let kind = bo::target_kind(package);
        let named = bo::binary_target(package).is_some();
        let needs_name = matches!(
            kind,
            bo::Gate0PackageTargetKind::Binary | bo::Gate0PackageTargetKind::ExampleBinary
        );
        if needs_name && !named {
            findings.push(format!("{} is a binary-kind package with no binary target name", package.cargo_name()));
        }
        if !needs_name && named {
            findings.push(format!("{} is a library-kind package carrying a binary target name", package.cargo_name()));
        }
        if bo::source_suffix(package).trim().is_empty() {
            findings.push(format!("{} has an empty source door", package.cargo_name()));
        }
        // exactly one path per artifact family
        for &family in bo::Gate0PackageArtifact::ALL {
            let suffix = match family {
                bo::Gate0PackageArtifact::Manifest => "Cargo.toml",
                bo::Gate0PackageArtifact::Readme => "README.md",
                bo::Gate0PackageArtifact::SourceDoor => bo::source_suffix(package),
            };
            planned.push(format!("{base}/{suffix}"));
        }
    }
    for &plane in architecture::SyncBatPlane::ALL {
        let base = architecture::PackageId::SyncBat.workspace_path();
        let module = plane.module_name();
        if module.trim().is_empty() {
            findings.push(format!("SyncBat plane {plane:?} has an empty module name"));
        }
        for &family in bo::Gate0PlaneArtifact::ALL {
            planned.push(match family {
                bo::Gate0PlaneArtifact::ModuleDoor => format!("{base}/src/{module}/mod.rs"),
                bo::Gate0PlaneArtifact::TypesCarrier => format!("{base}/src/{module}/types.rs"),
            });
        }
    }
    // Every expanded path obeys the one portable-path law, and no two collide
    // under case folding (the authoritative host is case-insensitive).
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut folded: BTreeSet<String> = BTreeSet::new();
    for path in &planned {
        if !bo::is_portable_gate0_relative_path(path) {
            findings.push(format!("expanded Gate0 path {path} is not a portable relative path"));
        }
        if !folded.insert(path.to_ascii_lowercase()) {
            findings.push(format!("expanded Gate0 path {path} case-fold collides in the plan"));
        }
        if !seen.insert(path) {
            findings.push(format!("expanded Gate0 path {path} is planned twice"));
        }
    }
}

/// The typed Tier 0 qualification algebra, EXECUTED (5.5E6a; rebuilt for the
/// evidence-verification algebra in 5.5E6b). Seedcheck runs the real `verify`
/// over an honest whole qualification and over a perturbation for EVERY rule in
/// `Tier0VerificationRule::ALL` — no rule is a fire alarm whose battery was
/// never installed. The concrete digest recomputation lives in
/// bootstrap/receiptcheck.rs; here we exercise the typed shape/denominator/
/// target/source/hosted-run laws.
pub(crate) fn check_bootstrap_qualification(findings: &mut Vec<String>) {
    use bootstrap_qualification as bq;
    use toolchain::{RustRelease, RustTargetTriple, AUTHORITATIVE_TARGET};

    // Every kind has a nonempty unique slug and a total artifact policy.
    let mut slugs: Vec<&str> = Vec::new();
    for &kind in bq::Tier0ReceiptKind::ALL {
        if kind.slug().is_empty() {
            findings.push(format!("Tier0ReceiptKind {kind:?} has an empty slug"));
        }
        if slugs.contains(&kind.slug()) {
            findings.push(format!("Tier0 slug {} is claimed twice", kind.slug()));
        }
        slugs.push(kind.slug());
        let _ = kind.artifact_policy();
    }

    // Honest evidence exactly matching a kind's policy.
    fn evidence(kind: bq::Tier0ReceiptKind) -> bq::Tier0ArtifactEvidence {
        match kind.artifact_policy() {
            bq::Tier0ArtifactPolicy::FixtureSet => bq::Tier0ArtifactEvidence::FixtureSet {
                digest: bq::Sha256Digest::from_bytes([0u8; 32]),
            },
            bq::Tier0ArtifactPolicy::Executable => bq::Tier0ArtifactEvidence::Executable {
                digest: bq::Sha256Digest::from_bytes([1u8; 32]),
            },
            bq::Tier0ArtifactPolicy::ExecutableAndOutputTree => {
                bq::Tier0ArtifactEvidence::ExecutableAndOutputTree {
                    executable_digest: bq::Sha256Digest::from_bytes([2u8; 32]),
                    output_tree_digest: bq::Sha256Digest::from_bytes([3u8; 32]),
                }
            }
        }
    }
    fn receipt(
        kind: bq::Tier0ReceiptKind,
        target: RustTargetTriple,
    ) -> bq::Tier0ReceiptObservation {
        bq::Tier0ReceiptObservation {
            kind,
            target,
            available: true,
            compilation: bq::CompilationOutcome::Succeeded,
            execution: bq::ExecutionAttempt::Attempted,
            outcome: Some(bq::ExecutionOutcome::Passed),
            artifact: Some(evidence(kind)),
        }
    }

    let git_source = || bq::SourceBinding::GitCheckout {
        commit: bq::GitCommitSha::from_bytes([0u8; 20]),
        tree: bq::GitTreeSha::from_bytes([0u8; 20]),
        spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
        workflow_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let export_source = || bq::SourceBinding::FrozenExport {
        spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
        export_tree_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let toolchain = bq::ToolchainBinding {
        rustc_release: RustRelease { major: 1, minor: 97, patch: 1 },
        rustc_commit: bq::ToolchainCommit::from_bytes([0u8; 20]),
        cargo_release: RustRelease { major: 1, minor: 97, patch: 1 },
        cargo_commit: bq::ToolchainCommit::from_bytes([0u8; 20]),
        toolchain_file_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
    };
    let runtime = bq::BootstrapRuntimeBinding {
        python_release: bq::AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE,
    };
    let hosted = || {
        Some(bq::GitHubActionsRunBinding {
            repository: "freebatteryfactory/batpak".to_owned(),
            workflow_path: ".github/workflows/msvc-qualification.yml".to_owned(),
            run_id: 1,
            run_attempt: 1,
            runner_image_os: "Windows".to_owned(),
            runner_image_version: "0".to_owned(),
        })
    };
    let build = |target: RustTargetTriple,
                 hosted_run: Option<bq::GitHubActionsRunBinding>,
                 source: bq::SourceBinding|
     -> bq::Tier0QualificationObservation {
        bq::Tier0QualificationObservation {
            source,
            toolchain,
            bootstrap_runtime: runtime,
            target,
            hosted_run,
            receipts: bq::Tier0ReceiptKind::ALL
                .iter()
                .map(|&k| receipt(k, target))
                .collect(),
        }
    };

    // The honest MSVC qualification verifies; the honest supplemental GNU
    // qualification (frozen export, no hosted run) also verifies.
    if let Err(errs) = bq::verify(build(AUTHORITATIVE_TARGET, hosted(), git_source())) {
        let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
        findings.push(format!("an honest authoritative qualification was refused: {rules:?}"));
    }
    if let Err(errs) = bq::verify(build(
        RustTargetTriple::X86_64PcWindowsGnu,
        None,
        export_source(),
    )) {
        let rules: Vec<_> = errs.iter().map(|e| e.rule()).collect();
        findings.push(format!("an honest supplemental GNU qualification was refused: {rules:?}"));
    }

    // Perturb the honest MSVC qualification once per rule; verify must refuse
    // AND report that specific rule.
    let want = |mut obs: bq::Tier0QualificationObservation,
                rule: bq::Tier0VerificationRule,
                label: &str,
                mutate: &dyn Fn(&mut bq::Tier0QualificationObservation),
                findings: &mut Vec<String>| {
        mutate(&mut obs);
        match bq::verify(obs) {
            Ok(_) => findings.push(format!("{label}: an incoherent qualification verified")),
            Err(errs) => {
                if !errs.iter().any(|e| e.rule() == rule) {
                    let got: Vec<_> = errs.iter().map(|e| e.rule()).collect();
                    findings.push(format!("{label}: refused, but not for {rule:?}: {got:?}"));
                }
            }
        }
    };
    let base = || build(AUTHORITATIVE_TARGET, hosted(), git_source());
    use bq::Tier0VerificationRule as R;
    want(base(), R::ReceiptNotAvailable, "not_available",
         &|o| o.receipts[0].available = false, findings);
    want(base(), R::ExecutionOutcomeWithoutAttempt, "outcome_without_attempt",
         &|o| o.receipts[0].execution = bq::ExecutionAttempt::NotAttempted, findings);
    want(base(), R::ExecutionAttemptWithoutOutcome, "attempt_without_outcome",
         &|o| o.receipts[0].outcome = None, findings);
    want(base(), R::ExecutionAfterFailedCompilation, "execution_after_failed_compilation",
         &|o| o.receipts[0].compilation = bq::CompilationOutcome::Failed, findings);
    want(base(), R::ReceiptNotPassed, "not_passed",
         &|o| o.receipts[0].outcome = Some(bq::ExecutionOutcome::Failed), findings);
    want(base(), R::ArtifactMissing, "artifact_missing",
         &|o| o.receipts[0].artifact = None, findings);
    want(base(), R::ArtifactPolicyMismatch, "artifact_policy_mismatch",
         &|o| o.receipts[1].artifact = Some(bq::Tier0ArtifactEvidence::FixtureSet {
             digest: bq::Sha256Digest::from_bytes([9u8; 32]),
         }), findings);
    want(base(), R::ReceiptTargetMismatch, "receipt_target_mismatch",
         &|o| o.receipts[0].target = RustTargetTriple::X86_64PcWindowsGnu, findings);
    want(base(), R::MissingRequiredReceipt, "missing_required_receipt",
         &|o| { o.receipts.pop(); }, findings);
    want(base(), R::DuplicateReceipt, "duplicate_receipt",
         &|o| { let last = o.receipts[o.receipts.len() - 1]; o.receipts.push(last); }, findings);
    want(base(), R::ReceiptsOutOfCanonicalOrder, "out_of_canonical_order",
         &|o| o.receipts.reverse(), findings);
    want(base(), R::SourceCannotQualifyAuthoritativeTarget, "export_cannot_qualify_authoritative",
         &|o| o.source = bq::SourceBinding::FrozenExport {
             spec_manifest_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
             export_tree_digest: bq::Sha256Digest::from_bytes([0u8; 32]),
         }, findings);
    want(base(), R::AuthoritativeTargetWithoutHostedRun, "authoritative_without_hosted_run",
         &|o| o.hosted_run = None, findings);
    want(base(), R::AuthoritativeBootstrapRuntimeMismatch, "authoritative_wrong_python_runtime",
         &|o| o.bootstrap_runtime = bq::BootstrapRuntimeBinding {
             python_release: bq::PythonRelease { major: 3, minor: 11, patch: 9 },
         }, findings);

    // Every rule names a nonempty law, a repair, and one of the three declared
    // owners — no rule is unowned or unexplained.
    for &rule in bq::Tier0VerificationRule::ALL {
        if rule.law().is_empty() {
            findings.push(format!("verification rule {rule:?} has an empty law"));
        }
        if rule.repair().is_empty() {
            findings.push(format!("verification rule {rule:?} has an empty repair"));
        }
        let owner = rule.owner();
        let known = owner == bq::TIER0_RECEIPT_ALGEBRA_OWNER
            || owner == bq::TIER0_DENOMINATOR_OWNER
            || owner == bq::TIER0_HOSTED_QUALIFICATION_OWNER;
        if !known {
            findings.push(format!("verification rule {rule:?} names an unknown owner {owner:?}"));
        }
    }
}


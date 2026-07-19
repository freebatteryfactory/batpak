use crate::guarantees::ContractId;
use crate::toolchain::RustTargetTriple;
use super::*;

/// A raw observation of one Tier 0 receipt attempt. It MAY express a
/// contradiction — an outcome with no attempt, an attempt with no outcome, an
/// execution after a failed compilation — so verification can DIAGNOSE it rather
/// than the shape making the incoherent state unreportable. It carries no
/// authority until `verify` seals the whole qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tier0ReceiptObservation {
    pub kind: Tier0ReceiptKind,
    pub target: RustTargetTriple,
    /// The gate binary (or fixture set) was present and runnable.
    pub available: bool,
    pub compilation: CompilationOutcome,
    pub execution: ExecutionAttempt,
    /// The execution result, if any. `Some` with no attempt, or `None` with an
    /// attempt, are contradictions verification refuses.
    pub outcome: Option<ExecutionOutcome>,
    /// The concrete artifact evidence, if the run bound any.
    pub artifact: Option<Tier0ArtifactEvidence>,
}

/// The whole qualification a run reports: common source, toolchain, target, and
/// hosted-run coordinates, plus the receipts. Verified as ONE unit — five
/// disconnected receipts are not a qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tier0QualificationObservation {
    pub source: SourceBinding,
    pub toolchain: ToolchainBinding,
    pub bootstrap_runtime: BootstrapRuntimeBinding,
    pub target: RustTargetTriple,
    pub hosted_run: Option<GitHubActionsRunBinding>,
    pub receipts: Vec<Tier0ReceiptObservation>,
}

// ===========================================================================
// Verified results (5.5E6b): retain every binding, seal the shape
// ===========================================================================

/// A verified receipt. The concrete artifact evidence SURVIVES verification —
/// the E6c comparator asks a verified receipt for its executable and output-tree
/// digests. The private seal means the only way to obtain one is through
/// `verify`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedTier0Receipt {
    kind: Tier0ReceiptKind,
    target: RustTargetTriple,
    artifact: Tier0ArtifactEvidence,
    _seal: (),
}

impl VerifiedTier0Receipt {
    pub const fn kind(&self) -> Tier0ReceiptKind {
        self.kind
    }
    pub const fn target(&self) -> RustTargetTriple {
        self.target
    }
    pub const fn artifact(&self) -> Tier0ArtifactEvidence {
        self.artifact
    }
}

/// A verified whole qualification. Retains the source, toolchain, target,
/// hosted-run, and per-receipt evidence so the E6c comparator can prove two
/// hosted runs describe the same promoted source. The private seal prevents
/// bypass of the verified shape — it does NOT prove the inputs were computed
/// honestly; `bootstrap/receiptcheck.rs` is the independent computer of those.
///
/// The `materializer_output_tree` is a projection of the Materialize receipt's
/// output-tree digest, lifted by `verify` to a first-class binding because it is
/// THE cross-run same-source coordinate (5.5E6c): two independent hosted runs of
/// one source recompute it to identical bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedTier0Qualification {
    source: SourceBinding,
    toolchain: ToolchainBinding,
    bootstrap_runtime: BootstrapRuntimeBinding,
    target: RustTargetTriple,
    hosted_run: Option<GitHubActionsRunBinding>,
    receipts: Vec<VerifiedTier0Receipt>,
    materializer_output_tree: Sha256Digest,
    _seal: (),
}

impl VerifiedTier0Qualification {
    pub fn source(&self) -> &SourceBinding {
        &self.source
    }
    pub const fn toolchain(&self) -> &ToolchainBinding {
        &self.toolchain
    }
    /// The bootstrap runtime (CPython release) this qualification ran under —
    /// builder identity beside rustc/cargo (5.5E6c2). Promotion confirmation
    /// requires two runs to share it; the authoritative target must bind exactly
    /// `AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE`.
    pub const fn bootstrap_runtime(&self) -> &BootstrapRuntimeBinding {
        &self.bootstrap_runtime
    }
    pub const fn target(&self) -> RustTargetTriple {
        self.target
    }
    pub fn hosted_run(&self) -> Option<&GitHubActionsRunBinding> {
        self.hosted_run.as_ref()
    }
    pub fn receipts(&self) -> &[VerifiedTier0Receipt] {
        &self.receipts
    }
    /// The materializer's canonical output-tree digest — the load-bearing
    /// cross-run same-source coordinate (5.5E6c). `verify` lifts it from the
    /// Materialize receipt so the comparator reads one deterministic,
    /// source-derived digest without re-scanning the receipt vector. Total:
    /// every verified qualification binds exactly one Materialize receipt
    /// carrying an output tree.
    pub const fn materializer_output_tree(&self) -> Sha256Digest {
        self.materializer_output_tree
    }
}

// ===========================================================================
// Verification rules (5.5E6b): one inventory, each with law/owner/repair
// ===========================================================================

/// Every rule `verify` can refuse on. Fieldless so it forms one closed `ALL`
/// inventory — the refusal-test harness exercises EVERY rule, so no rule becomes
/// a fire alarm whose battery was never installed. The corresponding
/// `Tier0VerificationError` carries the context (which kind, which target).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0VerificationRule {
    ReceiptNotAvailable,
    ExecutionOutcomeWithoutAttempt,
    ExecutionAttemptWithoutOutcome,
    ExecutionAfterFailedCompilation,
    ReceiptNotPassed,
    ArtifactMissing,
    ArtifactPolicyMismatch,
    ReceiptTargetMismatch,
    MissingRequiredReceipt,
    DuplicateReceipt,
    ReceiptsOutOfCanonicalOrder,
    SourceCannotQualifyAuthoritativeTarget,
    AuthoritativeTargetWithoutHostedRun,
    AuthoritativeBootstrapRuntimeMismatch,
}

impl Tier0VerificationRule {
    /// The closed inventory of verification rules, in declaration order.
    pub const ALL: &'static [Tier0VerificationRule] = &[
        Tier0VerificationRule::ReceiptNotAvailable,
        Tier0VerificationRule::ExecutionOutcomeWithoutAttempt,
        Tier0VerificationRule::ExecutionAttemptWithoutOutcome,
        Tier0VerificationRule::ExecutionAfterFailedCompilation,
        Tier0VerificationRule::ReceiptNotPassed,
        Tier0VerificationRule::ArtifactMissing,
        Tier0VerificationRule::ArtifactPolicyMismatch,
        Tier0VerificationRule::ReceiptTargetMismatch,
        Tier0VerificationRule::MissingRequiredReceipt,
        Tier0VerificationRule::DuplicateReceipt,
        Tier0VerificationRule::ReceiptsOutOfCanonicalOrder,
        Tier0VerificationRule::SourceCannotQualifyAuthoritativeTarget,
        Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun,
        Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch,
    ];

    /// The contract that owns this rule's semantics.
    pub const fn owner(self) -> ContractId {
        match self {
            Tier0VerificationRule::ReceiptNotAvailable
            | Tier0VerificationRule::ExecutionOutcomeWithoutAttempt
            | Tier0VerificationRule::ExecutionAttemptWithoutOutcome
            | Tier0VerificationRule::ExecutionAfterFailedCompilation
            | Tier0VerificationRule::ReceiptNotPassed
            | Tier0VerificationRule::ArtifactMissing
            | Tier0VerificationRule::ArtifactPolicyMismatch
            | Tier0VerificationRule::ReceiptTargetMismatch => TIER0_RECEIPT_ALGEBRA_OWNER,
            Tier0VerificationRule::MissingRequiredReceipt
            | Tier0VerificationRule::DuplicateReceipt
            | Tier0VerificationRule::ReceiptsOutOfCanonicalOrder => TIER0_DENOMINATOR_OWNER,
            Tier0VerificationRule::SourceCannotQualifyAuthoritativeTarget
            | Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun
            | Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch => {
                TIER0_HOSTED_QUALIFICATION_OWNER
            }
        }
    }

    /// The law this rule enforces.
    pub const fn law(self) -> &'static str {
        match self {
            Tier0VerificationRule::ReceiptNotAvailable =>
                "an unavailable Tier 0 binary cannot qualify; provide the binary or omit the receipt",
            Tier0VerificationRule::ExecutionOutcomeWithoutAttempt =>
                "an execution outcome without an attempt is incoherent",
            Tier0VerificationRule::ExecutionAttemptWithoutOutcome =>
                "an attempted execution must record an outcome",
            Tier0VerificationRule::ExecutionAfterFailedCompilation =>
                "execution cannot follow a compilation that did not succeed",
            Tier0VerificationRule::ReceiptNotPassed =>
                "a receipt qualifies only when it compiled, executed, and passed",
            Tier0VerificationRule::ArtifactMissing =>
                "every receipt binds concrete artifact evidence",
            Tier0VerificationRule::ArtifactPolicyMismatch =>
                "the artifact evidence variant must equal the receipt kind's policy exactly",
            Tier0VerificationRule::ReceiptTargetMismatch =>
                "every receipt in a qualification binds the qualification's own target",
            Tier0VerificationRule::MissingRequiredReceipt =>
                "the qualification binds every Tier 0 receipt kind",
            Tier0VerificationRule::DuplicateReceipt =>
                "each Tier 0 receipt kind appears exactly once",
            Tier0VerificationRule::ReceiptsOutOfCanonicalOrder =>
                "receipts appear in the canonical Tier0ReceiptKind::ALL order",
            Tier0VerificationRule::SourceCannotQualifyAuthoritativeTarget =>
                "a frozen export is supplemental and cannot qualify the authoritative target",
            Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun =>
                "an authoritative-target qualification binds a hosted-run identity",
            Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch =>
                "an authoritative-target qualification runs under the selected bootstrap Python release",
        }
    }

    /// How to repair a violation.
    pub const fn repair(self) -> &'static str {
        match self {
            Tier0VerificationRule::ReceiptNotAvailable =>
                "compile the gate binary and record its real availability",
            Tier0VerificationRule::ExecutionOutcomeWithoutAttempt =>
                "record the execution attempt, or drop the outcome",
            Tier0VerificationRule::ExecutionAttemptWithoutOutcome =>
                "record the execution's pass or fail",
            Tier0VerificationRule::ExecutionAfterFailedCompilation =>
                "fix compilation before recording an execution",
            Tier0VerificationRule::ReceiptNotPassed =>
                "fix the gate; never bind a failed run",
            Tier0VerificationRule::ArtifactMissing =>
                "bind the executable, fixture-set, or output-tree digest this run produced",
            Tier0VerificationRule::ArtifactPolicyMismatch =>
                "bind evidence matching the kind: fixture set, executable, or executable+output-tree",
            Tier0VerificationRule::ReceiptTargetMismatch =>
                "split receipts at other targets into their own qualification",
            Tier0VerificationRule::MissingRequiredReceipt =>
                "run and bind the missing Tier 0 gate",
            Tier0VerificationRule::DuplicateReceipt =>
                "bind each Tier 0 kind exactly once",
            Tier0VerificationRule::ReceiptsOutOfCanonicalOrder =>
                "emit receipts in Tier0ReceiptKind::ALL order",
            Tier0VerificationRule::SourceCannotQualifyAuthoritativeTarget =>
                "qualify the authoritative target from a real git checkout",
            Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun =>
                "bind the hosted GitHub Actions run identity",
            Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch =>
                "run the authoritative qualification under AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE",
        }
    }
}

/// A verification failure: the rule it violated plus the context needed to
/// locate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0VerificationError {
    Receipt {
        kind: Tier0ReceiptKind,
        rule: Tier0VerificationRule,
    },
    Denominator {
        kind: Tier0ReceiptKind,
        rule: Tier0VerificationRule,
    },
    Qualification {
        rule: Tier0VerificationRule,
    },
}

impl Tier0VerificationError {
    /// The rule this error reports.
    pub const fn rule(self) -> Tier0VerificationRule {
        match self {
            Tier0VerificationError::Receipt { rule, .. }
            | Tier0VerificationError::Denominator { rule, .. }
            | Tier0VerificationError::Qualification { rule } => rule,
        }
    }
}

/// Check one receipt's shape, pushing every rule it violates. Returns the
/// verified artifact evidence when the receipt is internally coherent AND passed
/// AND its evidence matches its policy; otherwise `None`.
fn verify_receipt_shape(
    obs: &Tier0ReceiptObservation,
    errors: &mut Vec<Tier0VerificationError>,
) -> Option<Tier0ArtifactEvidence> {
    let kind = obs.kind;
    let mut coherent = true;
    let push = |errors: &mut Vec<Tier0VerificationError>, rule| {
        errors.push(Tier0VerificationError::Receipt { kind, rule });
    };

    if !obs.available {
        push(errors, Tier0VerificationRule::ReceiptNotAvailable);
        coherent = false;
    }
    // Contradiction: an outcome recorded with no attempt.
    if obs.execution == ExecutionAttempt::NotAttempted && obs.outcome.is_some() {
        push(errors, Tier0VerificationRule::ExecutionOutcomeWithoutAttempt);
        coherent = false;
    }
    // Contradiction: an attempt recorded with no outcome.
    if obs.execution == ExecutionAttempt::Attempted && obs.outcome.is_none() {
        push(errors, Tier0VerificationRule::ExecutionAttemptWithoutOutcome);
        coherent = false;
    }
    // Contradiction: execution attempted on a compilation that did not succeed.
    if obs.execution == ExecutionAttempt::Attempted
        && obs.compilation != CompilationOutcome::Succeeded
    {
        push(errors, Tier0VerificationRule::ExecutionAfterFailedCompilation);
        coherent = false;
    }
    // The passing shape: compiled, attempted, and passed.
    let passed = obs.compilation == CompilationOutcome::Succeeded
        && obs.execution == ExecutionAttempt::Attempted
        && obs.outcome == Some(ExecutionOutcome::Passed);
    if !passed {
        push(errors, Tier0VerificationRule::ReceiptNotPassed);
        coherent = false;
    }
    // Artifact evidence must be present and match the kind's policy exactly.
    let evidence = match obs.artifact {
        None => {
            push(errors, Tier0VerificationRule::ArtifactMissing);
            coherent = false;
            None
        }
        Some(ev) => {
            if ev.policy() != kind.artifact_policy() {
                push(errors, Tier0VerificationRule::ArtifactPolicyMismatch);
                coherent = false;
                None
            } else {
                Some(ev)
            }
        }
    };
    if coherent { evidence } else { None }
}

/// Verify a whole Tier 0 qualification. Collects EVERY violation so a red
/// artifact explains all of them at once, then — only if there are none —
/// returns the sealed, binding-preserving verified qualification.
///
/// The rules enforced, all from `Tier0VerificationRule::ALL`:
/// per-receipt shape and policy; every receipt bound to the qualification's own
/// target; the exact `Tier0ReceiptKind::ALL` denominator, once each, in
/// canonical order; a source posture permitted to qualify the target; and a
/// hosted-run binding for the authoritative target.
pub fn verify(
    obs: Tier0QualificationObservation,
) -> Result<VerifiedTier0Qualification, Vec<Tier0VerificationError>> {
    let mut errors: Vec<Tier0VerificationError> = Vec::new();
    let target = obs.target;

    // Per-receipt shape, and target coherence with the qualification.
    let mut verified: Vec<VerifiedTier0Receipt> = Vec::new();
    for r in &obs.receipts {
        let evidence = verify_receipt_shape(r, &mut errors);
        if r.target != target {
            errors.push(Tier0VerificationError::Receipt {
                kind: r.kind,
                rule: Tier0VerificationRule::ReceiptTargetMismatch,
            });
        }
        if let Some(ev) = evidence {
            if r.target == target {
                verified.push(VerifiedTier0Receipt {
                    kind: r.kind,
                    target: r.target,
                    artifact: ev,
                    _seal: (),
                });
            }
        }
    }

    // Denominator: every required kind, exactly once, at the qualification target.
    for &kind in Tier0ReceiptKind::ALL {
        let count = obs
            .receipts
            .iter()
            .filter(|r| r.kind == kind && r.target == target)
            .count();
        if count == 0 {
            errors.push(Tier0VerificationError::Denominator {
                kind,
                rule: Tier0VerificationRule::MissingRequiredReceipt,
            });
        } else if count > 1 {
            errors.push(Tier0VerificationError::Denominator {
                kind,
                rule: Tier0VerificationRule::DuplicateReceipt,
            });
        }
    }

    // Canonical order: the receipts at the qualification target, in the order
    // observed, must equal Tier0ReceiptKind::ALL.
    let observed_order: Vec<Tier0ReceiptKind> = obs
        .receipts
        .iter()
        .filter(|r| r.target == target)
        .map(|r| r.kind)
        .collect();
    if observed_order != Tier0ReceiptKind::ALL {
        // Only report order drift when the SET is otherwise complete and unique;
        // a missing or duplicated kind is already reported above and would make
        // an order complaint noise.
        let complete_and_unique = Tier0ReceiptKind::ALL.iter().all(|&k| {
            obs.receipts
                .iter()
                .filter(|r| r.kind == k && r.target == target)
                .count()
                == 1
        });
        if complete_and_unique {
            errors.push(Tier0VerificationError::Qualification {
                rule: Tier0VerificationRule::ReceiptsOutOfCanonicalOrder,
            });
        }
    }

    // Source posture and hosted-run posture for the target.
    if target.is_authoritative() {
        if !obs.source.may_qualify_authoritative() {
            errors.push(Tier0VerificationError::Qualification {
                rule: Tier0VerificationRule::SourceCannotQualifyAuthoritativeTarget,
            });
        }
        if obs.hosted_run.is_none() {
            errors.push(Tier0VerificationError::Qualification {
                rule: Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun,
            });
        }
        // The authoritative target runs under the ONE selected bootstrap runtime.
        // A supplemental lane may record a different release, but two runs on the
        // same WRONG runtime cannot pass an equality check against this anchor.
        if obs.bootstrap_runtime.python_release != AUTHORITATIVE_BOOTSTRAP_PYTHON_RELEASE {
            errors.push(Tier0VerificationError::Qualification {
                rule: Tier0VerificationRule::AuthoritativeBootstrapRuntimeMismatch,
            });
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    // Lift the materializer output tree to a first-class binding (5.5E6c). An
    // empty error set guarantees exactly one Materialize receipt carrying
    // `ExecutableAndOutputTree`, so the `None` arm is unreachable; it is folded
    // into a verification error rather than a panic to keep `verify` total and
    // this crate panic-free.
    let materializer_output_tree = verified.iter().find_map(|r| match (r.kind, r.artifact) {
        (
            Tier0ReceiptKind::Materialize,
            Tier0ArtifactEvidence::ExecutableAndOutputTree {
                output_tree_digest, ..
            },
        ) => Some(output_tree_digest),
        _ => None,
    });
    match materializer_output_tree {
        Some(tree) => Ok(VerifiedTier0Qualification {
            source: obs.source,
            toolchain: obs.toolchain,
            bootstrap_runtime: obs.bootstrap_runtime,
            target: obs.target,
            hosted_run: obs.hosted_run,
            receipts: verified,
            materializer_output_tree: tree,
            _seal: (),
        }),
        None => Err(vec![Tier0VerificationError::Denominator {
            kind: Tier0ReceiptKind::Materialize,
            rule: Tier0VerificationRule::MissingRequiredReceipt,
        }]),
    }
}

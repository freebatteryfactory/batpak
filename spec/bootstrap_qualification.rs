//! The typed Tier 0 qualification receipt policy (5.5E6a). Semantic owner:
//! BP-RECEIPTS-1 for the receipt algebra; the Tier 0 denominator itself is a
//! bootstrap fact owned here.
//!
//! This is NOT the product receipt model (`ReceiptId`, docs/14). A product
//! receipt records what a running store proved about a data operation. A Tier 0
//! receipt records that a BOOTSTRAP gate binary was available, compiled,
//! executed, and passed on a physical target, bound to the source it ran
//! against. The two families deliberately share no type, field, spelling, or
//! authority: `ReceiptId` can never substitute for a `Tier0ReceiptKind`.
//!
//! The denominator moved OUT of Python here (5.5E6a): the former
//! `REQUIRED_TIER0_RECEIPTS` tuple in `bootstrap/selftest.py` is no longer an
//! authority. `Tier0ReceiptKind::ALL` is the ONE denominator; the Delivery
//! Notes projection and the hosted workflow both derive from it. There is no
//! count assertion anywhere: completeness is the closed `ALL`, not a number.

use crate::toolchain::RustTargetTriple;

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

/// What a receipt must bind. A fixture set binds a set digest but no single
/// executable; an executable binds its own artifact digest; the materializer
/// additionally binds the canonical output-tree digest.
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
    /// Every artifact policy.
    pub const ALL: &'static [Tier0ArtifactPolicy] = &[
        Tier0ArtifactPolicy::FixtureSet,
        Tier0ArtifactPolicy::Executable,
        Tier0ArtifactPolicy::ExecutableAndOutputTree,
    ];

    /// Whether this policy requires a bound EXECUTABLE artifact digest. The
    /// fixture set runs many short-lived binaries and binds the physical
    /// triple only (a fixture-set digest is E6b work); a single executable and
    /// the materializer bind their own artifact digest.
    pub const fn requires_artifact(self) -> bool {
        match self {
            Tier0ArtifactPolicy::FixtureSet => false,
            Tier0ArtifactPolicy::Executable
            | Tier0ArtifactPolicy::ExecutableAndOutputTree => true,
        }
    }

    /// Whether this policy additionally requires the materializer output-tree
    /// digest.
    pub const fn requires_output_tree(self) -> bool {
        match self {
            Tier0ArtifactPolicy::ExecutableAndOutputTree => true,
            Tier0ArtifactPolicy::FixtureSet | Tier0ArtifactPolicy::Executable => false,
        }
    }
}

/// A raw, UNVALIDATED observation of one Tier 0 receipt attempt. Every axis is
/// a separate field so an incoherent shape — executed yet not compiled, passed
/// yet not executed, an artifact-bound kind with no artifact — is EXPRESSIBLE
/// here and refused only at admission. This is the seam a run reports through;
/// it carries no authority until `admit` seals it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptObservation {
    pub kind: Tier0ReceiptKind,
    /// The physical target the binary ran on.
    pub target: RustTargetTriple,
    /// The gate binary (or fixture set) was present and runnable.
    pub available: bool,
    /// It compiled and linked.
    pub compiled: bool,
    /// It executed.
    pub executed: bool,
    /// Its execution agreed with the expected disposition.
    pub passed: bool,
    /// A source-snapshot digest is bound (WHAT source this ran against).
    pub source_bound: bool,
    /// The rustc/cargo toolchain identity is bound.
    pub toolchain_bound: bool,
    /// The artifact (executable or fixture-set) digest is bound.
    pub artifact_bound: bool,
    /// The materializer output-tree digest is bound.
    pub output_tree_bound: bool,
    /// A hosted-run identity (run id + attempt) is bound.
    pub hosted_run_bound: bool,
}

/// Why an observation was refused admission. Each names the law, its owner, and
/// the repair, so a red receipt explains itself instead of forcing a reader to
/// reconstruct the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier0AdmissionError {
    /// The gate binary was not available; an unavailable binary never wears the
    /// green of a passing one.
    NotAvailable,
    /// Execution was claimed without compilation — a contradictory shape.
    ExecutedWithoutCompilation,
    /// A pass was claimed without execution — a banner is not a result.
    PassedWithoutExecution,
    /// The kind binds an artifact digest, and none was bound.
    ArtifactRequiredButUnbound,
    /// The materializer receipt carries no output-tree digest.
    MaterializerWithoutOutputTree,
    /// The source snapshot the receipt represents is unbound.
    SourceUnbound,
    /// The toolchain identity is unbound.
    ToolchainUnbound,
    /// An authoritative-target receipt carries no hosted-run identity; only a
    /// hosted runner produces authoritative evidence.
    AuthoritativeWithoutHostedRun,
    /// Execution did not pass.
    NotPassed,
}

impl Tier0AdmissionError {
    /// The law, owner, and repair for this refusal.
    pub const fn law(self) -> &'static str {
        match self {
            Tier0AdmissionError::NotAvailable =>
                "an unavailable Tier 0 binary cannot be admitted (owner: BP-RECEIPTS-1); \
                 provide the compiled binary or omit the receipt, never mark it passed",
            Tier0AdmissionError::ExecutedWithoutCompilation =>
                "a receipt cannot be executed without being compiled (owner: BP-RECEIPTS-1); \
                 record the real compilation outcome",
            Tier0AdmissionError::PassedWithoutExecution =>
                "a receipt cannot pass without executing (owner: BP-RECEIPTS-1); \
                 run the binary, do not read its banner",
            Tier0AdmissionError::ArtifactRequiredButUnbound =>
                "this receipt kind binds an artifact digest (owner: BP-RECEIPTS-1); \
                 bind the executable or fixture-set digest this run produced",
            Tier0AdmissionError::MaterializerWithoutOutputTree =>
                "the materializer receipt binds a canonical output-tree digest \
                 (owner: BP-BOOTSTRAP-1); compute and bind it",
            Tier0AdmissionError::SourceUnbound =>
                "a receipt must bind the source snapshot it ran against \
                 (owner: BP-RECEIPTS-1); bind the git commit and tree identity",
            Tier0AdmissionError::ToolchainUnbound =>
                "a receipt must bind its toolchain identity (owner: BP-TOOLCHAIN-1); \
                 bind the rustc and cargo release and commit",
            Tier0AdmissionError::AuthoritativeWithoutHostedRun =>
                "an authoritative-target receipt must bind a hosted-run identity \
                 (owner: BP-PUBLIC-API-CI-RELEASE-1); only the hosted runner \
                 produces authoritative evidence",
            Tier0AdmissionError::NotPassed =>
                "the receipt did not pass (owner: BP-RECEIPTS-1); fix the gate, \
                 never admit a failed run",
        }
    }
}

/// A sealed, admitted Tier 0 receipt. The private field means the ONLY way to
/// obtain one is through `ReceiptObservation::admit`: a downstream consumer
/// cannot fabricate a passing receipt, it can only observe and admit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedTier0Receipt {
    kind: Tier0ReceiptKind,
    target: RustTargetTriple,
    _seal: (),
}

impl AdmittedTier0Receipt {
    /// The kind this admitted receipt covers.
    pub const fn kind(self) -> Tier0ReceiptKind {
        self.kind
    }

    /// The physical target this admitted receipt is bound to.
    pub const fn target(self) -> RustTargetTriple {
        self.target
    }
}

impl ReceiptObservation {
    /// Admit this observation as a sealed Tier 0 receipt, or refuse it with the
    /// exact law it violated. The checks are ordered from the most structural
    /// (availability, compile/execute/pass coherence) to the binding
    /// requirements. An authoritative-target receipt additionally requires a
    /// hosted-run binding.
    pub fn admit(self) -> Result<AdmittedTier0Receipt, Tier0AdmissionError> {
        if !self.available {
            return Err(Tier0AdmissionError::NotAvailable);
        }
        if self.executed && !self.compiled {
            return Err(Tier0AdmissionError::ExecutedWithoutCompilation);
        }
        if self.passed && !self.executed {
            return Err(Tier0AdmissionError::PassedWithoutExecution);
        }
        let policy = self.kind.artifact_policy();
        if policy.requires_artifact() && !self.artifact_bound {
            return Err(Tier0AdmissionError::ArtifactRequiredButUnbound);
        }
        if policy.requires_output_tree() && !self.output_tree_bound {
            return Err(Tier0AdmissionError::MaterializerWithoutOutputTree);
        }
        if !self.source_bound {
            return Err(Tier0AdmissionError::SourceUnbound);
        }
        if !self.toolchain_bound {
            return Err(Tier0AdmissionError::ToolchainUnbound);
        }
        if self.target.is_authoritative() && !self.hosted_run_bound {
            return Err(Tier0AdmissionError::AuthoritativeWithoutHostedRun);
        }
        if !self.passed {
            return Err(Tier0AdmissionError::NotPassed);
        }
        Ok(AdmittedTier0Receipt {
            kind: self.kind,
            target: self.target,
            _seal: (),
        })
    }
}

/// Why a set of admitted receipts fails to qualify for a required target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tier0DenominatorError {
    /// A required kind has no admitted receipt at the required target.
    MissingKind(Tier0ReceiptKind),
    /// A required kind was admitted more than once at the required target.
    DuplicateKind(Tier0ReceiptKind),
}

/// Prove that a set of admitted receipts covers the ENTIRE Tier 0 denominator
/// at the required target, exactly once each. A receipt bound to a different
/// target does not count toward the required target (that is how "wrong target"
/// surfaces: the required kind is simply missing at the required triple).
pub fn qualifies(
    admitted: &[AdmittedTier0Receipt],
    required: RustTargetTriple,
) -> Result<(), Vec<Tier0DenominatorError>> {
    let mut errors: Vec<Tier0DenominatorError> = Vec::new();
    for &kind in Tier0ReceiptKind::ALL {
        let count = admitted
            .iter()
            .filter(|r| r.kind() == kind && r.target() == required)
            .count();
        if count == 0 {
            errors.push(Tier0DenominatorError::MissingKind(kind));
        } else if count > 1 {
            errors.push(Tier0DenominatorError::DuplicateKind(kind));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

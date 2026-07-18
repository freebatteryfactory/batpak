//! The typed Tier 0 qualification receipt algebra (5.5E6a; rebuilt from
//! shape-admission to evidence-verification in 5.5E6b).
//!
//! Three questions, three owners — declared once here, never repeated on the
//! individual receipt rows (5.5E6b):
//!
//! ```text
//! BP-RECEIPTS-1                    what a bootstrap qualification receipt MEANS
//! BP-BOOTSTRAP-1                   which Tier 0 gates form the required denominator
//! BP-PUBLIC-API-CI-RELEASE-1       which hosted target and run posture may close a gate
//! ```
//!
//! This is NOT the product receipt model (`ReceiptId`, docs/14). A product
//! receipt records what a running store proved about a data operation. A Tier 0
//! receipt records that a BOOTSTRAP gate binary was available, compiled,
//! executed, and passed on a physical target, bound to the source it ran
//! against and the concrete evidence it produced. The two families deliberately
//! share no type, field, spelling, or authority: `ReceiptId` can never
//! substitute for a `Tier0ReceiptKind`.
//!
//! ## What E6a did and what E6b adds
//!
//! E6a moved DENOMINATOR MEMBERSHIP out of Python: `Tier0ReceiptKind::ALL` is
//! the one denominator. But E6a admitted a SHAPE — a row of booleans — and its
//! sealed type discarded the concrete bindings. All-true booleans admitted; the
//! seal proved only that the seal existed.
//!
//! E6b admits CONCRETE EVIDENCE. A receipt carries typed stage outcomes (a
//! stage that was never attempted is distinct from one that failed), a typed
//! artifact-evidence sum whose variant must equal the kind's policy exactly, and
//! the whole qualification carries one source binding, one toolchain binding,
//! one target, and — for the authoritative target — one hosted-run binding. The
//! verified type RETAINS every concrete binding so the E6c comparator can ask it
//! which commit, which tree, which output tree, which run.
//!
//! ## The honest limit of the seal
//!
//! The private seal on `VerifiedTier0Qualification` prevents a consumer from
//! BYPASSING the verified shape — the only way to obtain one is through
//! `verify`. It does NOT cryptographically prove that an arbitrary caller
//! computed its verification inputs (the actual file digests, the real git
//! commit, the true toolchain identity) honestly. `bootstrap/receiptcheck.rs` is
//! the authoritative independent computer of those inputs: it recomputes every
//! digest from the bytes on disk, compares them against the artifact's claims,
//! and only then builds the observation this module verifies.

use crate::guarantees::ContractId;
use crate::toolchain::{RustRelease, RustTargetTriple};

/// Owner of the receipt ALGEBRA — what a bootstrap qualification receipt means.
pub const TIER0_RECEIPT_ALGEBRA_OWNER: ContractId = ContractId("BP-RECEIPTS-1");
/// Owner of the DENOMINATOR — which Tier 0 gates the qualification requires.
pub const TIER0_DENOMINATOR_OWNER: ContractId = ContractId("BP-BOOTSTRAP-1");
/// Owner of the HOSTED qualification posture — which target and run may close a gate.
pub const TIER0_HOSTED_QUALIFICATION_OWNER: ContractId =
    ContractId("BP-PUBLIC-API-CI-RELEASE-1");

// ===========================================================================
// Denominator vocabulary (unchanged from E6a: the one closed set of gates)
// ===========================================================================

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

    /// Resolve a slug back to its kind, or `None` for an unknown slug. The
    /// artifact parser uses this: an unknown receipt slug is unrepresentable as
    /// a `Tier0ReceiptKind` and is refused before it can reach verification.
    pub fn from_slug(slug: &str) -> Option<Tier0ReceiptKind> {
        Tier0ReceiptKind::ALL
            .iter()
            .copied()
            .find(|k| k.slug() == slug)
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

/// Which artifact evidence SHAPE a receipt must carry. This classifies the
/// evidence a kind demands; the concrete evidence itself is `Tier0ArtifactEvidence`.
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
    /// Every artifact policy, in canonical order.
    pub const ALL: &'static [Tier0ArtifactPolicy] = &[
        Tier0ArtifactPolicy::FixtureSet,
        Tier0ArtifactPolicy::Executable,
        Tier0ArtifactPolicy::ExecutableAndOutputTree,
    ];

    /// The stable policy slug the artifact projection uses.
    pub const fn slug(self) -> &'static str {
        match self {
            Tier0ArtifactPolicy::FixtureSet => "fixture-set",
            Tier0ArtifactPolicy::Executable => "executable",
            Tier0ArtifactPolicy::ExecutableAndOutputTree => "executable+output-tree",
        }
    }
}

// ===========================================================================
// Digest and revision primitives (5.5E6b)
// ===========================================================================

/// Why a hexadecimal identity failed to parse. Both arms are load-bearing
/// hostiles: `WrongLength` refuses an abbreviated digest, `NonLowerHexDigit`
/// refuses an uppercase or non-hex one — the artifact is strict lowercase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexParseError {
    /// The hex string was not exactly the expected byte-length times two.
    WrongLength,
    /// A character was not a lowercase hex digit `[0-9a-f]`.
    NonLowerHexDigit,
}

fn lower_hex_value(b: u8) -> Result<u8, HexParseError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(HexParseError::NonLowerHexDigit),
    }
}

fn parse_lower_hex<const N: usize>(s: &str) -> Result<[u8; N], HexParseError> {
    let bytes = s.as_bytes();
    if bytes.len() != N * 2 {
        return Err(HexParseError::WrongLength);
    }
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        let hi = lower_hex_value(bytes[2 * i])?;
        let lo = lower_hex_value(bytes[2 * i + 1])?;
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    Ok(out)
}

fn render_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// A SHA-256 content digest, held as raw bytes and rendered as exactly 64
/// lowercase hex characters. This is a BOOTSTRAP representation primitive: it is
/// deliberately NOT `ContentDigest`, `CommitmentDigest`, or any product identity
/// — a Tier 0 evidence digest and a stored-tile content digest share no type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Parse exactly 64 lowercase hex characters.
    pub fn from_hex(s: &str) -> Result<Self, HexParseError> {
        Ok(Sha256Digest(parse_lower_hex::<32>(s)?))
    }
    /// Wrap raw bytes computed by an independent hasher.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Sha256Digest(bytes)
    }
    /// The canonical 64-character lowercase spelling.
    pub fn render(&self) -> String {
        render_lower_hex(&self.0)
    }
    /// The raw bytes.
    pub const fn bytes(&self) -> [u8; 32] {
        self.0
    }
}

/// A source-repository git COMMIT object id (40 lowercase hex), distinct by type
/// from a tree id so the two can never be swapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GitCommitSha([u8; 20]);

impl GitCommitSha {
    pub fn from_hex(s: &str) -> Result<Self, HexParseError> {
        Ok(GitCommitSha(parse_lower_hex::<20>(s)?))
    }
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        GitCommitSha(bytes)
    }
    pub fn render(&self) -> String {
        render_lower_hex(&self.0)
    }
}

/// A source-repository git TREE object id (40 lowercase hex).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GitTreeSha([u8; 20]);

impl GitTreeSha {
    pub fn from_hex(s: &str) -> Result<Self, HexParseError> {
        Ok(GitTreeSha(parse_lower_hex::<20>(s)?))
    }
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        GitTreeSha(bytes)
    }
    pub fn render(&self) -> String {
        render_lower_hex(&self.0)
    }
}

/// A toolchain component (rustc or cargo) release COMMIT hash (40 lowercase
/// hex), as reported by `rustc -vV` / `cargo -Vv`. Distinct from the source
/// repository's commit: it identifies the compiler, not the code it compiled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolchainCommit([u8; 20]);

impl ToolchainCommit {
    pub fn from_hex(s: &str) -> Result<Self, HexParseError> {
        Ok(ToolchainCommit(parse_lower_hex::<20>(s)?))
    }
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        ToolchainCommit(bytes)
    }
    pub fn render(&self) -> String {
        render_lower_hex(&self.0)
    }
}

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

// ===========================================================================
// Observations (5.5E6b): the raw, possibly-incoherent report
// ===========================================================================

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
            | Tier0VerificationRule::AuthoritativeTargetWithoutHostedRun => {
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

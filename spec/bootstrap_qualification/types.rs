use alloc::string::String;

use crate::guarantees::ContractId;

// ===========================================================================
// Receipt algebra: owners, denominator, artifact policy (5.5E6a/E6b)
// ===========================================================================

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

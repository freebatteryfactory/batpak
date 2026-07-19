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

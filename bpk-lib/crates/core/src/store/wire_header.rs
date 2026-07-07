//! Shared on-disk/wire header shape for batpak's fixed-prefix formats.
//!
//! Several store/wire formats (checkpoint, idempotency index, keyset, hidden
//! ranges, fork-evidence, mmap index) open with the identical 12-byte prefix:
//!
//! ```text
//! magic (6 bytes) | version (u16 LE) | crc32 (u32 LE)
//! ```
//!
//! followed by a format-specific body over which the CRC is computed. Only the
//! byte LAYOUT is centralized here. Each format keeps its own magic literal,
//! current version, version-acceptance policy (strict / range / future-version),
//! and error types — those genuinely differ between formats and MUST stay with
//! the format, so the parse helper reports a minimal [`PrefixError`] the caller
//! maps into its own error surface.

use std::ops::Range;

/// Length of the magic prefix every format leads with.
pub(crate) const MAGIC_LEN: usize = 6;

/// Byte range of the little-endian `u16` format version.
pub(crate) const VERSION_RANGE: Range<usize> = 6..8;

/// Byte range of the little-endian `u32` CRC-32 of the body.
pub(crate) const CRC_RANGE: Range<usize> = 8..12;

/// Total prefix length (`magic(6) + version(2) + crc(4)`).
pub(crate) const HEADER_LEN: usize = 12;

/// Why a prefix failed to parse. Each caller maps these to its own error so the
/// format-specific error surface is preserved.
#[derive(Debug)]
pub(crate) enum PrefixError {
    /// The input is shorter than the 12-byte prefix; carries the actual length.
    TooShort { len: usize },
    /// The leading [`MAGIC_LEN`] bytes did not match the expected magic.
    BadMagic,
}

/// A parsed 12-byte prefix plus a borrow of the body that follows it.
pub(crate) struct Prefix<'a> {
    /// The little-endian `u16` version read from [`VERSION_RANGE`].
    pub version: u16,
    /// The little-endian `u32` CRC read from [`CRC_RANGE`].
    pub stored_crc: u32,
    /// The bytes after the prefix (`raw[HEADER_LEN..]`) — the CRC-covered body.
    pub body: &'a [u8],
}

/// Parse the shared 12-byte prefix, checking length then magic.
///
/// The version-acceptance policy and the CRC verification stay with the caller —
/// only the fixed byte layout is centralized here.
pub(crate) fn parse<'a>(raw: &'a [u8], magic: &[u8; MAGIC_LEN]) -> Result<Prefix<'a>, PrefixError> {
    if raw.len() < HEADER_LEN {
        return Err(PrefixError::TooShort { len: raw.len() });
    }
    if raw[..MAGIC_LEN] != magic[..] {
        return Err(PrefixError::BadMagic);
    }
    // Direct fixed-width copies (not `try_into().expect(...)`): core denies
    // `clippy::expect_used`, and the length is already guaranteed by the
    // `HEADER_LEN` check above. `VERSION_RANGE`/`CRC_RANGE` span exactly 2 and 4
    // bytes.
    let mut version_bytes = [0u8; 2];
    version_bytes.copy_from_slice(&raw[VERSION_RANGE]);
    let version = u16::from_le_bytes(version_bytes);
    let mut crc_bytes = [0u8; 4];
    crc_bytes.copy_from_slice(&raw[CRC_RANGE]);
    let stored_crc = u32::from_le_bytes(crc_bytes);
    Ok(Prefix {
        version,
        stored_crc,
        body: &raw[HEADER_LEN..],
    })
}

/// Build the 12-byte prefix `magic | version | crc`, ready to write or extend
/// into the output ahead of the body.
pub(crate) fn encode(magic: &[u8; MAGIC_LEN], version: u16, crc: u32) -> [u8; HEADER_LEN] {
    let mut prefix = [0u8; HEADER_LEN];
    prefix[..MAGIC_LEN].copy_from_slice(magic);
    prefix[VERSION_RANGE].copy_from_slice(&version.to_le_bytes());
    prefix[CRC_RANGE].copy_from_slice(&crc.to_le_bytes());
    prefix
}

#[cfg(test)]
mod tests {
    use super::{encode, parse, PrefixError, CRC_RANGE, HEADER_LEN, MAGIC_LEN, VERSION_RANGE};

    const MAGIC: &[u8; MAGIC_LEN] = b"FBATXX";

    #[test]
    fn ranges_are_contiguous_and_total_header_len() {
        assert_eq!(MAGIC_LEN, 6);
        assert_eq!(VERSION_RANGE, 6..8);
        assert_eq!(CRC_RANGE, 8..12);
        assert_eq!(HEADER_LEN, 12);
    }

    #[test]
    fn encode_then_parse_round_trips() {
        let mut image = encode(MAGIC, 7, 0xdead_beef).to_vec();
        image.extend_from_slice(b"body-bytes");
        let parsed = parse(&image, MAGIC).expect("well-formed prefix must parse");
        assert_eq!(parsed.version, 7);
        assert_eq!(parsed.stored_crc, 0xdead_beef);
        assert_eq!(parsed.body, b"body-bytes");
    }

    #[test]
    fn rejects_short_and_bad_magic() {
        assert!(matches!(
            parse(&[0u8; 4], MAGIC),
            Err(PrefixError::TooShort { len: 4 })
        ));
        let mut image = encode(b"OTHER!", 1, 0).to_vec();
        image.extend_from_slice(b"body");
        assert!(matches!(parse(&image, MAGIC), Err(PrefixError::BadMagic)));
    }
}

/// Typed reason the store lineage-metadata sidecar (`store.meta`) could not be
/// admitted. Mirrors [`super::HiddenRangesCorruption`]: `store.meta` is
/// authority-expectation state (the store's lineage identity plus, once keyed
/// idempotency traffic exists, the expected durable-authority anchor), so a
/// present-but-unreadable file must fail closed — silently reminting would
/// reset the store's identity out from under every externally anchored
/// consumer.
#[derive(Debug)]
#[non_exhaustive]
pub enum StoreMetaCorruption {
    /// Reading the store-metadata file failed.
    ReadFailed(std::io::Error),
    /// The file was shorter than the fixed header.
    TooShort {
        /// Bytes available in the file.
        actual: usize,
        /// Bytes required for the fixed header.
        required: usize,
    },
    /// The file did not start with the store-metadata magic.
    BadMagic,
    /// The store-metadata version is unsupported (zero/garbled; a FUTURE
    /// version is the distinct canonical
    /// [`super::StoreError::StoreMetadataFutureVersion`] refusal).
    UnsupportedVersion {
        /// Version observed on disk.
        observed: u16,
        /// Version this crate accepts.
        expected: u16,
    },
    /// The stored CRC did not match the decoded body.
    CrcMismatch {
        /// CRC stored in the metadata header.
        stored: u32,
        /// CRC computed from the metadata body.
        computed: u32,
    },
    /// MessagePack decoding of the metadata body failed.
    DecodeFailed(rmp_serde::decode::Error),
}

impl std::fmt::Display for StoreMetaCorruption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed(error) => {
                write!(f, "failed to read store metadata: {error}")
            }
            Self::TooShort { .. } => write!(f, "store metadata file too short"),
            Self::BadMagic => write!(f, "store metadata file has wrong magic"),
            Self::UnsupportedVersion { observed, .. } => {
                write!(f, "unsupported store metadata version: {observed}")
            }
            Self::CrcMismatch { .. } => write!(f, "store metadata CRC mismatch"),
            Self::DecodeFailed(error) => {
                write!(f, "store metadata deserialisation failed: {error}")
            }
        }
    }
}

impl StoreMetaCorruption {
    pub(super) fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadFailed(error) => Some(error),
            Self::DecodeFailed(error) => Some(error),
            Self::TooShort { .. }
            | Self::BadMagic
            | Self::UnsupportedVersion { .. }
            | Self::CrcMismatch { .. } => None,
        }
    }
}

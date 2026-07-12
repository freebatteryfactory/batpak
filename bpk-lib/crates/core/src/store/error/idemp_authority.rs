/// Typed reason the durable idempotency authority (`index.idemp`) could not be
/// admitted at cold start. Mirrors [`super::StoreMetaCorruption`]: once
/// retention compaction can evict an event's frames, the sidecar is the ONLY
/// remaining proof that a keyed retry is not a new command — so damaged
/// authority bytes must never be reinterpreted as an empty set
/// (GAUNT-IDEMPOTENCY-AUTHORITY, #189).
#[derive(Debug)]
#[non_exhaustive]
pub enum IdempAuthorityCorruption {
    /// Reading the sidecar failed.
    ReadFailed(std::io::Error),
    /// The file was shorter than the fixed header.
    TooShort {
        /// Bytes available in the file.
        actual: usize,
        /// Bytes required for the fixed header.
        required: usize,
    },
    /// The file did not start with the idempotency-store magic.
    BadMagic,
    /// The version field is garbled (zero); supported older versions take the
    /// legacy path, future versions the canonical
    /// [`super::StoreError::IdempotencyFutureVersion`] refusal.
    UnsupportedVersion {
        /// Version observed on disk.
        observed: u16,
        /// Version this crate accepts.
        expected: u16,
    },
    /// The stored CRC did not match the decoded body.
    CrcMismatch {
        /// CRC stored in the header.
        stored: u32,
        /// CRC computed from the body.
        computed: u32,
    },
    /// MessagePack decoding of the body failed.
    DecodeFailed(rmp_serde::decode::Error),
}

impl std::fmt::Display for IdempAuthorityCorruption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed(error) => {
                write!(f, "failed to read idempotency authority: {error}")
            }
            Self::TooShort { .. } => write!(f, "idempotency authority file too short"),
            Self::BadMagic => write!(f, "idempotency authority file has wrong magic"),
            Self::UnsupportedVersion { observed, .. } => {
                write!(f, "unsupported idempotency authority version: {observed}")
            }
            Self::CrcMismatch { .. } => write!(f, "idempotency authority CRC mismatch"),
            Self::DecodeFailed(error) => {
                write!(f, "idempotency authority deserialisation failed: {error}")
            }
        }
    }
}

impl IdempAuthorityCorruption {
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

/// Why a structurally valid idempotency-authority image belongs to a DIFFERENT
/// history than this store's (never remediable by accepting it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdempAuthorityForeignKind {
    /// The image was written for a different store lineage (its stamped
    /// lineage id does not match this store's `store.meta`).
    Lineage,
    /// Same lineage, same covered sequence — but the image's history anchor
    /// (event id / chain commitment at that sequence) disagrees with the
    /// expectation: a diverged sibling fork's image transplanted between
    /// directories. Lineage + numeric frontier alone cannot catch this; the
    /// compound anchor exists exactly for it (owner ruling, #189/#205).
    HistoryAnchor,
}

impl std::fmt::Display for IdempAuthorityForeignKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lineage => write!(f, "different store lineage"),
            Self::HistoryAnchor => {
                write!(f, "diverged history anchor at the covered sequence")
            }
        }
    }
}

#![warn(missing_docs)]
//! Claw kit facade for batpak-family sync operations.
//!
//! cb declares; sb runs; bp banks.
//!
//! Use this crate as `use downstream-kit as cb;` when declaring operation-kit
//! vocabulary. Runtime composition and invocation remain owned by
//! [`syncbat`].

use std::error::Error;
use std::fmt;

pub use syncbat::operation;
pub use syncbat::{EffectClass, OperationDescriptor, ReceiptEnvelope, ReceiptOutcome};

/// Lightweight validated reference to a pass declared by an operation kit.
pub type PassRef = Ref<Pass>;

/// Lightweight validated reference to a capability declared by an operation kit.
pub type CapabilityRef = Ref<Capability>;

/// Validation error for operation-kit references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefError {
    /// The reference string was empty.
    Empty,
    /// The reference exceeded the maximum supported length.
    TooLong {
        /// Maximum accepted byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// The reference contained a byte outside the allowed vocabulary.
    InvalidByte {
        /// Byte offset of the invalid byte.
        index: usize,
        /// Invalid byte.
        byte: u8,
    },
    /// The reference started or ended with punctuation instead of an
    /// alphanumeric token byte.
    InvalidBoundary {
        /// Byte offset of the invalid boundary byte.
        index: usize,
        /// Invalid boundary byte.
        byte: u8,
    },
    /// The reference contained two adjacent separator bytes.
    RepeatedSeparator {
        /// Byte offset of the repeated separator byte.
        index: usize,
        /// Repeated separator byte.
        byte: u8,
    },
}

impl fmt::Display for RefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("reference must not be empty"),
            Self::TooLong { max, actual } => {
                write!(f, "reference length {actual} exceeds maximum {max}")
            }
            Self::InvalidByte { index, byte } => {
                write!(
                    f,
                    "reference contains invalid byte 0x{byte:02x} at offset {index}"
                )
            }
            Self::InvalidBoundary { index, byte } => {
                write!(
                    f,
                    "reference contains boundary separator byte 0x{byte:02x} at offset {index}"
                )
            }
            Self::RepeatedSeparator { index, byte } => {
                write!(
                    f,
                    "reference contains repeated separator byte 0x{byte:02x} at offset {index}"
                )
            }
        }
    }
}

impl Error for RefError {}

/// Validated operation-kit reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Ref<K> {
    value: &'static str,
    _kind: std::marker::PhantomData<K>,
}

impl<K> Ref<K> {
    /// Maximum accepted reference length in bytes.
    pub const MAX_LEN: usize = 128;

    /// Construct a validated reference.
    ///
    /// # Errors
    /// Returns [`RefError`] when the value is empty, too long, or contains a
    /// byte outside `[A-Za-z0-9._:-]`, starts or ends with punctuation, or
    /// contains adjacent separator bytes.
    pub const fn new(value: &'static str) -> Result<Self, RefError> {
        let bytes = value.as_bytes();
        if bytes.is_empty() {
            return Err(RefError::Empty);
        }
        if bytes.len() > Self::MAX_LEN {
            return Err(RefError::TooLong {
                max: Self::MAX_LEN,
                actual: bytes.len(),
            });
        }

        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            let valid = matches!(
                byte,
                b'a'..=b'z'
                    | b'A'..=b'Z'
                    | b'0'..=b'9'
                    | b'.'
                    | b'_'
                    | b':'
                    | b'-'
            );
            if !valid {
                return Err(RefError::InvalidByte { index, byte });
            }
            if !is_ref_alnum(byte) {
                if index == 0 || index + 1 == bytes.len() {
                    return Err(RefError::InvalidBoundary { index, byte });
                }
                if !is_ref_alnum(bytes[index - 1]) {
                    return Err(RefError::RepeatedSeparator { index, byte });
                }
            }
            index += 1;
        }

        Ok(Self {
            value,
            _kind: std::marker::PhantomData,
        })
    }

    /// Return the reference as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.value
    }
}

const fn is_ref_alnum(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9')
}

impl<K> AsRef<str> for Ref<K> {
    fn as_ref(&self) -> &str {
        self.value
    }
}

impl<K> fmt::Display for Ref<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value)
    }
}

/// Marker for pass references.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Pass {}

/// Marker for capability references.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Capability {}

/// Common imports for declaring claw kit operations.
pub mod prelude {
    pub use crate::{
        operation, CapabilityRef, EffectClass, OperationDescriptor, PassRef, ReceiptEnvelope,
        ReceiptOutcome, Ref, RefError,
    };
}

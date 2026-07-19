//! The typed toolchain owner (5.5E3a, type-baked in 5.5E3a1).
//!
//! Until this file existed the toolchain facts lived as scattered string
//! literals: the materializer hardcoded the resolver, edition, MSRV, and the
//! emitted workspace toolchain bytes; the qualification harness hardcoded
//! seven copies of the edition; and no tracked root toolchain selection
//! existed at all. Every consumer now derives from `TOOLCHAIN`, and the two
//! independent Python paths reconstruct and attack the rendered boundary
//! rather than owning copies.
//!
//! The closed fields are TYPES, not validated strings: a malformed edition,
//! a non-numeric resolver, or an unsorted component list has no spelling
//! here, and the string-shape validators that compensated for invalid
//! construction died with the strings.

mod types;

pub use types::{
    AUTHORITATIVE_TARGET, CargoResolver, RustEdition, RustRelease, RustTargetTriple,
    RustVersionFloor, RustupComponent, RustupProfile, TOOLCHAIN, ToolchainProfile,
};

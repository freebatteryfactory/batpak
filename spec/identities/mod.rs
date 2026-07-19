//! The identity, generation, binding, and version catalogs (5.5E3d).
//!
//! docs/16 owns the identity doctrine in prose; this file is the typed
//! authority for WHICH stable vocabularies exist on each of four axes. The
//! axes are SEPARATE closed vocabularies, never one mega enum:
//!
//! ```text
//! IdentityKind          which semantic, instance, event, operation, or
//!                       evidence OBJECT
//! GenerationKind        which evolution or authority generation
//! BindingKind           which bytes, interface, or history COMMITMENT
//! VersionIdentityKind   which independently versioned format, protocol,
//!                       language, or schema
//! ```
//!
//! Chronology, order, topology, coordinates, cursors, and navigation are NOT
//! identity vocabulary and have no spelling here: ObservedWallTime,
//! TimeDelta, MonotonicDeadline, Hlc, GlobalSequence, CommitPoint,
//! StreamPosition, DagPosition, Coordinate, PageCursor, WorldPath, and
//! ProjectionPath stay owned by docs/16's time/order/navigation sections and
//! `spec/reconciliation.rs`. An identity may bind or reference a CommitPoint;
//! this catalog defines no CommitPoint comparison, no HLC ordering or
//! tie-breaking, no frontier progress, and no part of the DEC-075
//! dual-coordinate theorem.
//!
//! Existing typed owners stay owners: PackageId, ProofRowId, GateId, and
//! ContractKind are referenced where needed and never redefined or wrapped.
//! No byte layout, UUID layout, wire encoding, ordering implementation, or
//! ID generator is selected here — those belong to the owning implementation
//! gates.
//!
//! Every entry names its canonical semantic owner by declared contract id,
//! and the owner reference resolves or admission refuses. The inventory was
//! derived by corpus sweep (every stable `...Id`, `...Hash`, `...Digest`,
//! `...Commitment`, `...Generation`, `...Version` in the authoritative
//! docs), never by copying the old docs/16 list — which omitted EventId,
//! LogicalOperationId, and LayoutVersion, and misfiled AuthorityGeneration,
//! ContentDigest, and Commitment as object identities. CompressionId is
//! deliberately ABSENT: DEC-063 names it as a future compressed profile's
//! admission requirement, and a requirement for an unadmitted profile is a
//! brochure entry, not an identity.

mod inventory;
mod types;

pub use types::{
    BindingKind, CatalogEntry, GenerationKind, IdentityKind, IdentityTermDisposition,
    NonCatalogedIdentityTerm, VersionIdentityKind,
};
pub use inventory::{
    EXCLUDED_CHRONOLOGY_AND_NAVIGATION, EXISTING_TYPED_OWNER_SPELLINGS,
    NON_CATALOGED_IDENTITY_TERMS,
};

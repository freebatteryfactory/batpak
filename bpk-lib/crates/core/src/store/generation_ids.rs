//! Branded identity types for the compaction namespace-commit protocol.
//!
//! A compaction transaction is decided by THREE durable artifacts agreeing on
//! the same identities (the marker, the authority image, and `store.meta`; see
//! ADR-0037 / `INV-COMPACTION-NAMESPACE-COMMIT`). Confusing a compaction id for
//! an authority-image id — or letting a raw `u128` off a disk artifact stand in
//! for either — is exactly the class of bug that lets "file presence" vote on
//! recovery direction. These newtypes make that unrepresentable:
//!
//! - There is **no** `From`/`Into` between the brands or with raw `u128`. The
//!   only way in from a raw value is the internal [`from_u128`] seam (used when
//!   re-reading a CRC-checked binary artifact) or minting; the only way out is
//!   [`as_u128`] for hex rendering. Cross-brand comparison cannot type-check.
//! - Serde rides the canonical [`crate::wire::u128_bytes`] 16-byte big-endian
//!   helpers, identical on the wire to every other store id.
//! - Minting follows the `generate_v7_id_with_clock` precedent so a fresh id is
//!   deterministic under a supplied [`Clock`](crate::store::Clock).
//!
//! [`from_u128`]: CompactionId::from_u128
//! [`as_u128`]: CompactionId::as_u128

/// Declares a `u128`-backed identity brand with the store's fixed discipline:
/// wire-serde via `u128_bytes`, deterministic minting, an internal raw-value
/// reconstruction seam, and a single `as_u128` exit — but deliberately NO
/// `From`/`Into` traits (which would erase the brand) and no cross-brand path.
macro_rules! branded_u128_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct $name(u128);

        impl $name {
            /// Reconstruct from a raw `u128` read out of a durable, already
            /// CRC-validated binary artifact (compaction marker payload,
            /// authority-image body). This is an internal seam, not a `From`
            /// impl: raw bytes off disk are only ever promoted into a branded
            /// id at the honest parse boundary, never implicitly.
            #[must_use]
            pub(crate) const fn from_u128(id: u128) -> Self {
                Self(id)
            }

            /// Mint a fresh id from the store clock (UUIDv7 bits), matching the
            /// `generate_v7_id_with_clock` precedent shared by every other
            /// minted identity in the store.
            #[must_use]
            pub(crate) fn mint(clock: &dyn crate::store::Clock) -> Self {
                Self(crate::id::generate_v7_id_with_clock(clock))
            }

            /// The underlying `u128`, for hex rendering / display only. This is
            /// the single escape hatch out of the brand; there is intentionally
            /// no cross-brand conversion or comparison.
            #[must_use]
            pub fn as_u128(self) -> u128 {
                self.0
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
                crate::wire::u128_bytes::serialize(&self.0, ser)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
                crate::wire::u128_bytes::deserialize(de).map(Self)
            }
        }
    };
}

branded_u128_id! {
    /// Identity of one compaction transaction. Minted when the transaction is
    /// planned, stamped into the pending marker, the authority image, and
    /// `store.meta`'s commit record; recovery rolls forward only when all three
    /// carry the same `CompactionId`.
    CompactionId
}

branded_u128_id! {
    /// Identity of one published idempotency-authority image. Each image also
    /// records its predecessor's `AuthorityImageId`, and `store.meta` records
    /// the authorized image id, so a stale or foreign image is detectable
    /// without trusting file presence.
    AuthorityImageId
}

branded_u128_id! {
    /// The store's lineage identity as it lives on durable artifacts (the raw
    /// `u128` in `store.meta` and stamped into every authority image). Kept
    /// distinct from the public `batpak::id::StoreIdentity` surface precisely
    /// because this brand admits no `From`/`Into` — a lineage value cannot be
    /// silently swapped for any other id on the durable path.
    StoreLineage
}

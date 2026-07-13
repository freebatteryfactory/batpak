//! Runtime support types for the batpak `EventPayload` / projection macro
//! surface.
//!
//! This is a **temporary phase-bridge crate** (`macbat-registration`): a
//! verbatim-in-behavior move of the runtime scan types that used to live in
//! `batpak-macros-support`, plus the one additive hard-signal helper
//! [`projection_decode_abort`] that the interim projection lowering calls. Its
//! public items are re-exported from `batpak::__private`; do not depend on it
//! directly. Its crate-3 deletion is warranted in `DEATH_WARRANT.md`.

#![cfg_attr(not(test), deny(clippy::expect_used))]

/// Re-exported so generated code can reference `batpak::__private::inventory`
/// without requiring users to add `inventory` to their own Cargo.toml.
pub extern crate inventory;

/// Registration item emitted once per `#[derive(EventPayload)]` type.
///
/// `inventory::submit!` calls that produce these items are emitted
/// unconditionally by generated code. That keeps dependency-crate payloads
/// visible to a downstream binary's explicit registry validator instead of
/// losing them behind each dependency crate's own `cfg(test)` boundary.
pub struct EventPayloadRegistration {
    /// Packed `(category << 12) | type_id`; equals `EventKind::as_raw_u16`
    /// for the resulting kind.
    pub kind_bits: u16,
    /// The derived type's declared `EventPayload::PAYLOAD_VERSION` (>= 1).
    ///
    /// Carried here so a binary-wide scan can enumerate `(kind, version)` pairs
    /// at `Store::open` and confirm a `version > 1` kind has a complete
    /// registered upcast chain (see [`find_incomplete_upcast_chains`]) instead
    /// of letting its older events become undecodable at read time.
    pub payload_version: u16,
    /// `std::any::type_name::<T>()` for the derived type.
    pub type_name: &'static str,
}

inventory::collect!(EventPayloadRegistration);

/// Registration item emitted once per `(KIND, from_version)` upcast step.
///
/// Mirrors [`EventPayloadRegistration`]: collected link-time via `inventory`
/// so the decode seam can look up the registered vN→vN+1 migration for a given
/// stored kind/version without a runtime registry. The `step` operates on a
/// type-erased `rmpv::Value` so a single chain runner is lane-neutral (it works
/// for both the JSON and raw-msgpack replay lanes). The concrete error type is
/// owned by `batpak` core, so `step` returns a boxed `dyn Error` here to keep
/// this support crate dependency-light.
pub struct UpcastRegistration {
    /// Packed `(category << 12) | type_id`; equals `EventKind::as_raw_u16`.
    pub kind_bits: u16,
    /// The stored version this step upgrades *from* (it produces `from + 1`).
    pub from_version: u16,
    /// The migration applied to a decoded `rmpv::Value` of version
    /// `from_version`, producing a value of version `from_version + 1`.
    pub step: fn(rmpv::Value) -> Result<rmpv::Value, Box<dyn std::error::Error + Send + Sync>>,
}

inventory::collect!(UpcastRegistration);

/// Return all registered upcast steps for `kind_bits`, indexed by `from_version`.
///
/// A duplicate `(kind_bits, from_version)` registration is a programming error
/// (two migrations claiming the same hop); the caller in `batpak` core surfaces
/// it as a decode-time failure rather than silently picking one.
pub fn upcast_steps_for(kind_bits: u16) -> Vec<&'static UpcastRegistration> {
    inventory::iter::<UpcastRegistration>()
        .filter(|reg| reg.kind_bits == kind_bits)
        .collect()
}

/// A registered payload kind whose declared `PAYLOAD_VERSION > 1` is missing one
/// or more `from_version` hops in its registered upcast chain.
///
/// Such a kind compiles fine, but an event stored at any uncovered version would
/// be undecodable at read time (the decode seam hits `UpcastError::MissingStep`
/// with no step to run). Surfacing it lets `Store::open` fail closed up front.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncompleteUpcastChain {
    /// Packed `(category << 12) | type_id`; equals `EventKind::as_raw_u16`.
    pub kind_bits: u16,
    /// The declared current payload version (always `> 1` for a reported gap).
    pub current_version: u16,
    /// The `from_version` hops in `1..current_version` that have no registered
    /// step, ascending. Always non-empty for a reported gap.
    pub missing_from_versions: Vec<u16>,
    /// A registered type name for `kind_bits`, for diagnostics.
    pub type_name: &'static str,
}

/// Return every registered payload kind whose declared version `> 1` lacks a
/// complete `1 -> ... -> N` upcast chain in the linked registry.
///
/// Mirrors [`find_kind_collisions`]: a binary-wide scan over the link-time
/// `inventory` registries so a downstream binary can refuse to open before any
/// historical event silently becomes undecodable. The result is sorted by
/// `kind_bits` for deterministic diagnostics. A kind registered more than once
/// (a separate collision error) is scored against the highest declared version
/// so the completeness bar is never understated.
pub fn find_incomplete_upcast_chains() -> Vec<IncompleteUpcastChain> {
    let mut declared: std::collections::HashMap<u16, (u16, &'static str)> =
        std::collections::HashMap::new();
    for reg in inventory::iter::<EventPayloadRegistration>() {
        declared
            .entry(reg.kind_bits)
            .and_modify(|current| {
                if reg.payload_version > current.0 {
                    *current = (reg.payload_version, reg.type_name);
                }
            })
            .or_insert((reg.payload_version, reg.type_name));
    }

    let mut out = Vec::new();
    for (kind_bits, (current_version, type_name)) in declared {
        if current_version <= 1 {
            continue;
        }
        let registered: std::collections::HashSet<u16> = inventory::iter::<UpcastRegistration>()
            .filter(|step| step.kind_bits == kind_bits)
            .map(|step| step.from_version)
            .collect();
        let missing_from_versions: Vec<u16> = (1..current_version)
            .filter(|hop| !registered.contains(hop))
            .collect();
        if !missing_from_versions.is_empty() {
            out.push(IncompleteUpcastChain {
                kind_bits,
                current_version,
                missing_from_versions,
                type_name,
            });
        }
    }
    out.sort_by_key(|chain| chain.kind_bits);
    out
}

/// One duplicate `EventKind` registration discovered in the current binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventKindCollision {
    /// Packed `(category << 12) | type_id` representation.
    pub kind_bits: u16,
    /// First registered type name for `kind_bits`.
    pub first_type_name: &'static str,
    /// Second registered type name for `kind_bits`.
    pub second_type_name: &'static str,
}

/// Return all `EventPayload` kind collisions visible in the current binary.
pub fn find_kind_collisions() -> Vec<EventKindCollision> {
    let mut seen: std::collections::HashMap<u16, &'static str> = std::collections::HashMap::new();
    let mut collisions = Vec::new();
    for item in inventory::iter::<EventPayloadRegistration>() {
        if let Some(prior) = seen.insert(item.kind_bits, item.type_name) {
            collisions.push(EventKindCollision {
                kind_bits: item.kind_bits,
                first_type_name: prior,
                second_type_name: item.type_name,
            });
        }
    }
    collisions
}

/// Panic if the current binary has duplicate `EventPayload` kind registrations.
///
/// # Panics
///
/// Panics when more than one registered payload type uses the same
/// `(category, type_id)` pair in the current binary.
pub fn assert_no_kind_collisions() {
    let first = find_kind_collisions().into_iter().next();
    assert!(
        first.is_none(),
        "batpak EventKind collision detected.\n\
         Types `{prior}` and `{ty}` share the same (category, type_id) \
         pair (kind_bits = 0x{bits:04X}).\n\
         Each EventPayload type must have a unique kind within a binary.",
        prior = first.as_ref().map_or("", |c| c.first_type_name),
        ty = first.as_ref().map_or("", |c| c.second_type_name),
        bits = first.as_ref().map_or(0, |c| c.kind_bits),
    );
}

/// Scan all `EventPayload` registrations in the current binary for kind collisions.
///
/// Called by the `#[test]` function generated by `#[derive(EventPayload)]`.
/// Detection scope is per-binary: two separate binaries can reuse the same
/// `(category, type_id)` without triggering a panic here.
///
/// Safe to call in non-test binaries. Prefer [`find_kind_collisions`] for a
/// structured result.
pub fn scan_for_kind_collisions() {
    assert_no_kind_collisions();
}

/// Hard-signal an interim projection decode failure and diverge.
///
/// Emitted by the generated `EventSourced::apply_event` dispatch (the crate-1
/// projection lowering) via `::batpak::__private::projection_decode_abort(...)`
/// when an event whose kind matches a bound payload fails to decode into that
/// payload type — a hard correctness signal (schema drift, `type_id` reuse,
/// corruption), never a normal wrong-kind filter.
///
/// This is a **temporary panic-equivalent hard signal**: panic machinery behind
/// a named seam. It keeps the literal `panic!` token out of the generated code
/// and out of this crate's source, but what it does IS panic — the same
/// unwind/abort behavior and message shape as the pre-bridge generated
/// `panic!`. The seam exists so crate 2's fallible `EventSourced::apply_event`
/// can replace the call with a structured decode-failure error (event kind,
/// payload version, coordinate, cause); this helper is deleted immediately when
/// that lands.
///
/// # Panics
///
/// Always. This function never returns; it fails the current thread with a
/// message naming `kind` and `cause`.
pub fn projection_decode_abort(kind: &'static str, cause: &dyn ::core::fmt::Display) -> ! {
    assert!(
        std::hint::black_box(false),
        "EventSourced: decode failed for matched kind {kind}: {cause}"
    );
    unreachable!()
}

// ─── Validation ───────────────────────────────────────────────────────────────
//
// NOTE (ownership split, contract §H): the reserved-range oracle
// (`validate_category`, `validate_type_id`, `TYPE_ID_MAX`) moved to
// `macbat-compiler`'s `grammar/kind_rule.rs` (checked-in generated projection +
// PARITY_HASH). Registration does NOT carry `validate_*` — it never performed the
// validation; the derive did. This crate keeps only the runtime scans above.

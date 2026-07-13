//! Checked-in generated projection of core's event-category rule (§16 phase
//! bridge).
//!
//! The authoritative rule lives in core (`EventCategory` / `CustomEventCategory`
//! and, in the current tree, `macros-support`'s `validate_category` /
//! `validate_type_id` / `TYPE_ID_MAX`). Because `macbat-compiler` may never
//! import core, this module carries a *projection* of that rule: the exact
//! category/type-id predicate, verbatim-in-behaviour, plus a [`PARITY_HASH`]
//! that pins the projected facts. The crate-3 parity gate recomputes the digest
//! from core's authoritative rule and fails the build on drift.
//!
//! Ownership note: this projection carries only the reserved-range oracle. It is
//! consumed by Pass 3 (validation) and by `ir::event` when computing `kind_bits`.
//! Version-domain rules (`version == 0` reserved, `version` overflow) are the
//! validation pass's concern, not this module's.

/// Maximum valid `type_id` value (inclusive), 12 bits.
pub const TYPE_ID_MAX: u16 = 0x0FFF;

/// Validate an event category value.
///
/// Valid range: `0x1..=0xF`, excluding `0xD` (reserved for effect events) and
/// `0x0` (reserved for system events).
///
/// # Errors
///
/// Returns an error when `cat` is zero, reserved for effects, or outside the
/// four-bit custom category range.
pub fn validate_category(cat: u8) -> Result<(), &'static str> {
    match cat {
        0x0 => Err("category 0x0 is reserved for system events"),
        0xD => Err("category 0xD is reserved for effect events"),
        1..=15 => Ok(()),
        _ => Err("category must fit in 4 bits (0x1-0xF, excluding 0x0 and 0xD)"),
    }
}

/// Validate an event `type_id` value.
///
/// Valid range: `0x000..=0xFFF` (12 bits).
///
/// # Errors
///
/// Returns an error when `tid` does not fit in the 12-bit type-id range.
pub fn validate_type_id(tid: u16) -> Result<(), &'static str> {
    if tid > TYPE_ID_MAX {
        Err("type_id must fit in 12 bits (0x000-0xFFF)")
    } else {
        Ok(())
    }
}

/// Digest pinning the projected event-category rule to core's authoritative
/// rule (§16 phase bridge).
///
/// Computed at compile time from [`RULE_DESCRIPTOR`] — a canonical, order-stable
/// encoding of the rule facts (reserved categories, the valid category range,
/// and the 12-bit type-id ceiling). The crate-3 parity gate reconstructs the
/// same descriptor from core's rule and compares; any drift in the projected
/// facts changes this constant and trips the gate.
pub const PARITY_HASH: [u8; 32] = parity_hash();

/// Canonical, order-stable encoding of the projected rule facts. The parity
/// gate rebuilds this exact byte sequence from core's authoritative rule.
const RULE_DESCRIPTOR: &[u8] =
    b"macbat.kind_rule.v1|category.reserved=0x00,0x0D|category.valid=0x01..=0x0F|type_id.max=0x0FFF|type_id.bits=12";

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over `data`, seeded with a leading `salt` byte so four independent
/// lanes cover the 32-byte digest. Pure and const-evaluable (integer ops only).
const fn fnv1a_salted(salt: u8, data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    hash ^= salt as u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// Assemble the 32-byte parity digest from four salted FNV-1a lanes over the
/// canonical rule descriptor, each rendered big-endian.
const fn parity_hash() -> [u8; 32] {
    let lane0 = fnv1a_salted(0x00, RULE_DESCRIPTOR).to_be_bytes();
    let lane1 = fnv1a_salted(0x01, RULE_DESCRIPTOR).to_be_bytes();
    let lane2 = fnv1a_salted(0x02, RULE_DESCRIPTOR).to_be_bytes();
    let lane3 = fnv1a_salted(0x03, RULE_DESCRIPTOR).to_be_bytes();
    [
        lane0[0], lane0[1], lane0[2], lane0[3], lane0[4], lane0[5], lane0[6], lane0[7],
        lane1[0], lane1[1], lane1[2], lane1[3], lane1[4], lane1[5], lane1[6], lane1[7],
        lane2[0], lane2[1], lane2[2], lane2[3], lane2[4], lane2[5], lane2[6], lane2[7],
        lane3[0], lane3[1], lane3[2], lane3[3], lane3[4], lane3[5], lane3[6], lane3[7],
    ]
}

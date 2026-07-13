//! Expansion hygiene: crate-root resolution and deterministic private idents.
//!
//! Two concerns, both pure and deterministic:
//!
//! * [`CratePath`] resolves the cross-family crate root a lowering emits paths
//!   through — `::batpak` for the derive surfaces, `::syncbat` for the operation
//!   runtime surface. All tokens are spanned at [`Span::mixed_site`] so generated
//!   references cannot capture user identifiers.
//! * [`ReservedIdent`] mints the compiler-owned private idents used across emitted
//!   items. Names are a deterministic function of the [`ContractId`] and
//!   [`LoweringRole`] — never derived from user text, so a user cannot name them.
//!
//! [`HygieneContext`] and [`LoweringContext`] bundle these for the driver and the
//! per-kind lowerings.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote_spanned};

use crate::identity::{ContractId, LoweringRole};

/// A cross-family crate root, carried as a `mixed_site`-spanned token prefix.
///
/// Built via [`CratePath::batpak`] or [`CratePath::runtime`]; extended with
/// [`CratePath::join`] to form full paths such as `::batpak::event::EventPayload`.
#[derive(Clone)]
pub struct CratePath {
    root: TokenStream,
}

impl CratePath {
    /// The `::batpak` root — the default for the Error, Event, Projection,
    /// Subscription, and VariantInventory surfaces.
    #[must_use]
    pub fn batpak() -> Self {
        let span = Span::mixed_site();
        CratePath {
            root: quote_spanned! { span => ::batpak },
        }
    }

    /// The runtime root — `::syncbat` in v1.
    ///
    /// This is the single literal that crate 4 flips when the operation runtime
    /// re-homes; nothing at any call site changes.
    #[must_use]
    pub fn runtime() -> Self {
        let span = Span::mixed_site();
        CratePath {
            // CRATE-4 SEAM: the one literal to rename when the runtime re-homes.
            root: quote_spanned! { span => ::syncbat },
        }
    }

    /// Append a `::`-separated path tail to this root, e.g. `join("event::EventKind")`
    /// on `::batpak` yields `::batpak::event::EventKind`.
    ///
    /// The tail is a compiler-internal constant of one or more identifier segments;
    /// each segment is emitted as a `mixed_site`-spanned ident so no parse and no
    /// fallible path is involved.
    #[must_use]
    pub fn join(&self, tail: &str) -> TokenStream {
        let span = Span::mixed_site();
        let root = &self.root;
        let segments: Vec<syn::Ident> = tail
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(|segment| syn::Ident::new(segment, span))
            .collect();
        if segments.is_empty() {
            return quote_spanned! { span => #root };
        }
        quote_spanned! { span => #root :: #(#segments)::* }
    }
}

/// Mints the compiler-owned private idents shared across a contract's emitted items.
pub struct ReservedIdent;

impl ReservedIdent {
    /// A deterministic private ident `__bpk_<hex16>_<role_snake>`, where `hex16` is
    /// the lower-hex FNV-1a hash of the contract identity and `role_snake` is the
    /// snake-case spelling of the lowering role.
    ///
    /// Pure and free of iteration-order dependence: the same contract and role
    /// always yield the same ident within an expansion, so cross-item references
    /// resolve. It is not constructible from user text.
    #[must_use]
    pub fn for_role(contract: &ContractId, role: LoweringRole) -> syn::Ident {
        let hex16 = format!("{:016x}", fnv1a_64(contract.as_str().as_bytes()));
        format_ident!(
            "__bpk_{}_{}",
            hex16,
            role_snake(role),
            span = Span::mixed_site()
        )
    }

    /// The Event collision-check test-fn ident — the LEGACY observable name scheme
    /// `__batpak_kind_collision_check_<snake_ident>`, preserved verbatim for parity
    /// (consumer tests reference it). Emitted by the Event lowering directly, not
    /// via [`ReservedIdent::for_role`].
    #[must_use]
    pub fn collision_check(ident: &syn::Ident) -> syn::Ident {
        let snake = to_snake_case(&ident.to_string());
        format_ident!("__batpak_kind_collision_check_{}", snake)
    }
}

/// Per-contract hygiene bundle built once by the driver.
#[derive(Clone)]
pub struct HygieneContext {
    /// The crate root this contract's lowerings emit paths through.
    pub crate_path: CratePath,
}

/// The hygiene + identity context threaded into each per-kind lowering.
#[derive(Clone, Copy)]
pub struct LoweringContext<'a> {
    /// The shared hygiene bundle.
    pub hygiene: &'a HygieneContext,
    /// The contract identity, for deriving reserved idents.
    pub contract: &'a ContractId,
}

/// The 64-bit FNV-1a hash of `bytes`. Deterministic; order-free by construction.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// The snake-case spelling of a lowering role, used as the trailing segment of a
/// reserved ident. Exhaustive over every v1 role.
fn role_snake(role: LoweringRole) -> &'static str {
    match role {
        LoweringRole::DisplayImpl => "display_impl",
        LoweringRole::ErrorSourceImpl => "error_source_impl",
        LoweringRole::FromImpl => "from_impl",
        LoweringRole::EventPayloadImpl => "event_payload_impl",
        LoweringRole::EventInventorySubmit => "event_inventory_submit",
        LoweringRole::EventCollisionCheck => "event_collision_check",
        LoweringRole::VariantInventoryConst => "variant_inventory_const",
        LoweringRole::ProjectionInputAssertion => "projection_input_assertion",
        LoweringRole::ProjectionImpl => "projection_impl",
        LoweringRole::SubscriptionAttrAssertion => "subscription_attr_assertion",
        LoweringRole::SubscriptionImpl => "subscription_impl",
        LoweringRole::OperationFunction => "operation_function",
        LoweringRole::OperationDescriptor => "operation_descriptor",
        LoweringRole::OperationSignaturePin => "operation_signature_pin",
        LoweringRole::OperationRegisterItem => "operation_register_item",
        LoweringRole::OperationRegister => "operation_register",
    }
}

/// Convert a (typically CamelCase) Rust identifier into `snake_case`.
///
/// Preserved byte-for-byte from the legacy `event_payload.rs` derive so the Event
/// collision-check test-fn name stays observably identical.
fn to_snake_case(ident: &str) -> String {
    let mut out = String::with_capacity(ident.len() + 4);
    let mut prev_lower_or_digit = false;
    for ch in ident.chars() {
        if ch.is_ascii_uppercase() {
            if prev_lower_or_digit {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = false;
        } else {
            out.push(ch);
            prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    out
}

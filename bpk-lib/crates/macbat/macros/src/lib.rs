//! MacBat proc-macro front doors.
//!
//! This crate (`macbat`, `proc-macro = true`) is the thin entry surface for the
//! batpak contract macros. It contains **no** parsing, `quote!`, or validation
//! logic of its own: every entry point is a small adapter that hands its raw
//! `TokenStream` to `macbat_compiler` and lowers the returned
//! [`macbat_compiler::ExpansionArtifact`] into either the emitted tokens or
//! `compile_error!` diagnostics. All semantics live in the pure compiler crate.
//!
//! Rustc requires `#[proc_macro_derive]` / `#[proc_macro_attribute]` functions
//! at the proc-macro crate root, so every entry point lives in this file; the
//! `adapter` module holds the one artifact-to-tokens bridge.
//!
//! Two names ship for each renamed derive — a new contract name plus its legacy
//! alias — and both resolve to the same `ContractKind`, so they are
//! behaviour-identical. The legacy aliases keep existing consumer compiles (and
//! core's current `pub use`) working until crate 2 re-points them:
//!
//! | new name           | legacy alias        | kind               |
//! |--------------------|---------------------|--------------------|
//! | `Event`            | `EventPayload`      | `Event`            |
//! | `Projection`       | `EventSourced`      | `Projection`       |
//! | `Subscription`     | `MultiEventReactor` | `Subscription`     |
//! | `VariantInventory` | `AllVariants`       | `VariantInventory` |
//! | `Error`            | (unchanged)         | `Error`            |
//! | `operation`        | (unchanged)         | `Operation`        |
//!
//! The `macbat` / `macbat-compiler` crates never import batpak core.

#![cfg_attr(not(test), deny(clippy::expect_used))]

mod adapter;

use proc_macro::TokenStream;

use macbat_compiler::ContractKind;

/// Derives `batpak::event::EventPayload` for a named-field struct.
///
/// Requires `#[batpak(category = N, type_id = N)]` on the struct; `version = N`
/// is optional (defaults to 1).
#[proc_macro_derive(Event, attributes(batpak))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Event, input)
}

/// Legacy alias for the [`Event`](macro@Event) derive. Retained until crate 2
/// re-points core's `pub use` to the new name; behaviour-identical.
#[proc_macro_derive(EventPayload, attributes(batpak))]
pub fn derive_event_payload(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Event, input)
}

/// Derives `core::fmt::Display` + `std::error::Error` for an error enum from a
/// per-variant `#[error("...")]` format literal. `#[source]` wires
/// `Error::source()`; `#[from]` additionally generates `impl From`. Enums only.
#[proc_macro_derive(Error, attributes(error, source, from))]
pub fn derive_error(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Error, input)
}

/// Derives a `pub const ALL: [Self; N]` listing every variant of a fieldless
/// enum, in declaration order.
#[proc_macro_derive(VariantInventory)]
pub fn derive_variant_inventory(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::VariantInventory, input)
}

/// Legacy alias for the [`VariantInventory`](macro@VariantInventory) derive;
/// behaviour-identical.
#[proc_macro_derive(AllVariants)]
pub fn derive_all_variants(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::VariantInventory, input)
}

/// Derives `batpak::event::EventSourced` for a named-field struct.
///
/// Requires `#[batpak(input = <Lane>)]` (plus optional `cache_version = N` /
/// `state_max_cardinality = 1`) and at least one
/// `#[batpak(event = <Payload>, handler = <fn>)]` binding.
#[proc_macro_derive(Projection, attributes(batpak))]
pub fn derive_projection(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Projection, input)
}

/// Legacy alias for the [`Projection`](macro@Projection) derive;
/// behaviour-identical.
#[proc_macro_derive(EventSourced, attributes(batpak))]
pub fn derive_event_sourced(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Projection, input)
}

/// Derives `batpak::event::MultiReactive<Input>` for a named-field struct,
/// for use with `Store::react_loop_multi` (JSON) or
/// `Store::react_loop_multi_raw` (msgpack).
///
/// Requires `#[batpak(input = <Lane>, error = <ErrorType>)]` and at least one
/// `#[batpak(event = <Payload>, handler = <fn>)]` binding.
#[proc_macro_derive(Subscription, attributes(batpak))]
pub fn derive_subscription(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Subscription, input)
}

/// Legacy alias for the [`Subscription`](macro@Subscription) derive;
/// behaviour-identical.
#[proc_macro_derive(MultiEventReactor, attributes(batpak))]
pub fn derive_multi_event_reactor(input: TokenStream) -> TokenStream {
    adapter::run_derive(ContractKind::Subscription, input)
}

/// `#[operation(...)]` — generate a syncbat operation descriptor plus optional
/// registration functions. Re-exported as `syncbat::operation`; users never
/// name this crate directly.
#[proc_macro_attribute]
pub fn operation(attr: TokenStream, item: TokenStream) -> TokenStream {
    adapter::run_attribute(ContractKind::Operation, attr, item)
}

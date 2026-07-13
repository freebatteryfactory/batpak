//! Positive `syn::parse2` round-trip per specimen (Cluster 14).
//!
//! For every checked-in specimen, the emitted expansion (`expanded.rs`, produced
//! and byte-frozen by the compiler crate's `snapshot` harness) must parse as a
//! well-formed Rust file. This is the structural in-lane proof for the four
//! surfaces whose expansions reference `::batpak`/`::syncbat` and therefore cannot
//! be resolved (only PARSED) without core: `syn::parse_str::<syn::File>` checks
//! SYNTAX, not name resolution, so a parseable expansion is the honest structural
//! witness. (The Error and VariantInventory surfaces get true behavioral oracles
//! in `error_derive.rs` / `variant_inventory.rs`.)
//!
//! Consumes the snapshot corpus at `../compiler/specimens/<dir>/expanded.rs`, so
//! it runs AFTER the snapshot harness has materialized the corpus (first lane run
//! is `BPK_SNAPSHOT=update`).

use std::path::Path;

/// The 24 specimen directories (relative to `../compiler/specimens`).
const SPECIMEN_DIRS: &[&str] = &[
    "error/coordinate_error",
    "error/boxed_source",
    "error/from_variant",
    "error/positional_args",
    "error/named_and_specs",
    "error/unit_variant",
    "event/minimal_store",
    "event/versioned",
    "event/default_version",
    "event/max_type_id",
    "variant_inventory/requirement_kind",
    "variant_inventory/two_variant",
    "variant_inventory/single_variant",
    "projection/event_sourced_counter",
    "projection/bounded_state",
    "projection/cached",
    "projection/multi_binding",
    "subscription/typed_reactor_multi",
    "subscription/raw_lane",
    "subscription/two_handlers",
    "operation/status_stream",
    "operation/with_effect_row",
    "operation/with_title",
    "operation/with_register",
];

#[test]
fn every_specimen_expansion_parses_as_rust() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../compiler/specimens");

    for &rel in SPECIMEN_DIRS {
        let path = base.join(rel).join("expanded.rs");
        let missing =
            format!("specimen `{rel}` has no expanded.rs — run the snapshot harness with BPK_SNAPSHOT=update first");
        let expanded = std::fs::read_to_string(&path).expect(&missing);

        let parsed: Result<syn::File, syn::Error> = syn::parse_str(&expanded);
        assert!(
            parsed.is_ok(),
            "emitted expansion for `{rel}` did not parse as a Rust file: {:?}",
            parsed.err()
        );
    }
}

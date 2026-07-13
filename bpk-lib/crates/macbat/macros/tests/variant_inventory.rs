//! Behaviour tests for `#[derive(VariantInventory)]` (legacy alias `AllVariants`).
//!
//! Moved VERBATIM from `batpak-macros`' `tests/all_variants.rs`; only the import
//! path changed (`batpak_macros` -> `macbat`). The alias `AllVariants` is
//! exercised on purpose — it must keep compiling consumer code until crate 2
//! re-points to the new name. The surface emits ONLY `::core` paths, so this runs
//! for real in the core-free macro lane: order + exhaustive-match coupling.

use macbat::AllVariants;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AllVariants)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn all_lists_every_variant_in_order() {
    assert_eq!(Color::ALL, [Color::Red, Color::Green, Color::Blue]);
    assert_eq!(Color::ALL.len(), 3);
}

#[test]
fn all_is_a_usable_const() {
    // Exhaustive match: adding a variant without updating this fails to compile,
    // proving ALL and the match stay coupled.
    for color in Color::ALL {
        match color {
            Color::Red | Color::Green | Color::Blue => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AllVariants)]
enum Single {
    Only,
}

#[test]
fn single_variant_enum() {
    assert_eq!(Single::ALL, [Single::Only]);
}

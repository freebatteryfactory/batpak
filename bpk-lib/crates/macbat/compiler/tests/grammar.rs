//! Grammar closed-vocabulary + ranked-alternative unit tests (Cluster 14).
//!
//! These exercise the pure grammar surface directly (no compiler run, no I/O):
//! the per-kind `AttributeGrammar` projection, the closed key vocabulary, the
//! declared item-shape law, and the edit-distance `rank_alternatives` suggester
//! that turns an unknown key into ranked replacements.

use macbat_compiler::grammar::{grammar_for, rank_alternatives};
use macbat_compiler::{ContractKind, ShapeRule};

/// Does this grammar declare a key with the given name?
fn has_key(kind: ContractKind, name: &str) -> bool {
    grammar_for(kind)
        .keys
        .iter()
        .any(|spec| spec.name.as_str() == name)
}

#[test]
fn event_vocabulary_is_closed_to_the_three_known_keys() {
    assert!(has_key(ContractKind::Event, "category"), "category is a known Event key");
    assert!(has_key(ContractKind::Event, "type_id"), "type_id is a known Event key");
    assert!(has_key(ContractKind::Event, "version"), "version is a known Event key");
    // A near-miss is NOT in the closed vocabulary.
    assert!(
        !has_key(ContractKind::Event, "catagory"),
        "a typo'd key must not be part of the closed vocabulary"
    );
}

#[test]
fn unknown_key_ranks_the_nearest_known_key_first() {
    let grammar = grammar_for(ContractKind::Event);
    let suggestions = rank_alternatives("catagory", grammar.keys, 3);
    assert!(
        !suggestions.is_empty(),
        "a one-edit typo of a known key must yield at least one ranked alternative"
    );
    assert_eq!(
        suggestions.first().map(|s| s.replacement.as_str()),
        Some("category"),
        "nearest edit-distance replacement must rank first; got {suggestions:?}"
    );
    assert!(
        suggestions.len() <= 3,
        "the suggestion count must honour the cap; got {}",
        suggestions.len()
    );
}

#[test]
fn ranked_alternatives_honour_the_type_id_neighbourhood() {
    let grammar = grammar_for(ContractKind::Event);
    let suggestions = rank_alternatives("type_i", grammar.keys, 3);
    assert_eq!(
        suggestions.first().map(|s| s.replacement.as_str()),
        Some("type_id"),
        "nearest replacement for `type_i` must be `type_id`; got {suggestions:?}"
    );
}

#[test]
fn each_kind_declares_its_item_shape_law() {
    assert!(matches!(grammar_for(ContractKind::Event).shape_rule, ShapeRule::NamedFieldStruct));
    assert!(matches!(grammar_for(ContractKind::Error).shape_rule, ShapeRule::Enum));
    assert!(matches!(
        grammar_for(ContractKind::VariantInventory).shape_rule,
        ShapeRule::UnitOnlyEnum
    ));
    assert!(matches!(
        grammar_for(ContractKind::Projection).shape_rule,
        ShapeRule::NamedFieldStruct
    ));
    assert!(matches!(
        grammar_for(ContractKind::Subscription).shape_rule,
        ShapeRule::NamedFieldStruct
    ));
    assert!(matches!(grammar_for(ContractKind::Operation).shape_rule, ShapeRule::FreeFunction));
}

#[test]
fn only_projection_and_subscription_carry_binding_keys() {
    assert!(grammar_for(ContractKind::Projection).binding_keys.is_some());
    assert!(grammar_for(ContractKind::Subscription).binding_keys.is_some());
    assert!(grammar_for(ContractKind::Event).binding_keys.is_none());
    assert!(grammar_for(ContractKind::Error).binding_keys.is_none());
    assert!(grammar_for(ContractKind::VariantInventory).binding_keys.is_none());
    assert!(grammar_for(ContractKind::Operation).binding_keys.is_none());
}

//! The typed clean-room specification, compiled once as a real library
//! (5.5E2 ruling).
//!
//! Before this file existed, every bootstrap binary mounted these modules
//! textually through `#[path]` and wore tracked `#[allow(dead_code)]` — the
//! exact suppression the repository's own doctrine forbids, and the reason
//! no compiler could say which public fact was genuine library vocabulary
//! and which was a dead mannequin. One rlib, one boundary: binaries link
//! this crate, every `pub` item below is API by construction, and
//! `dead_code_pub_in_binary` can police the binaries without misclassifying
//! library facts.

pub mod architecture;
pub mod commands;
pub mod compiler_assumptions;
pub mod contracts;
pub mod corpus;
pub mod dispositions;
pub mod gates;
pub mod generated_views;
pub mod guarantees;
pub mod identities;
pub mod invariants;
pub mod legacy_invariant_coverage;
pub mod legacy_obligations;
pub mod mutation;
pub mod operators;
pub mod promotion;
pub mod pakvm_isa;
pub mod proof;
pub mod reconciliation;
pub mod syncbat_firewall;
pub mod toolchain;

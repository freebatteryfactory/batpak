//! Per-pass expansion counters. `ExpansionMetrics` is a plain tally the passes
//! increment as they run; it carries no timing (the compiler tree is pure — no
//! `std::time`) and is always populated, independent of the `trace` option. It is
//! surfaced on the `ExpansionArtifact` for the harness and the `expand` stepper.

/// Counters accumulated across the eight typed passes. All public so each pass can
/// bump the fields it owns; `Default` yields the zeroed starting tally.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ExpansionMetrics {
    pub keys_parsed: u32,
    pub variants: u32,
    pub emitted_items: u32,
    pub tokens: u32,
    pub diagnostics: u32,
}

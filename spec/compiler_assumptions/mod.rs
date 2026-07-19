//! The admitted compiler-assumption kinds (5.5E3h).
//!
//! A ledgerable compiler assumption is a dangerous mechanism the corpus has
//! EARNED the right to record: it has a real detector, an adopter, a
//! semantic owner, a decision naming it, required ledger fields, a hostile
//! witness, and a release-visible proof surface. Everything else keeps its
//! existing owner — hard production violations and test allowances stay
//! with DEC-067/DEC-068 and docs/12, dependency and compatibility
//! mechanisms stay with docs/20 and their decisions, and future assumption
//! ideas (representation layout, atomic ordering, FFI ABI, target
//! features) are ABSENT until they earn admission. This file is two sharp
//! instruments, not a junk drawer: no universal AssumptionKind, no
//! CompilerAssumptionId, no census residue table.
//!
//! Detection CONSUMES these kinds; it does not own them. docs/19 owns what
//! an admitted dangerous mechanism means and the complete SAFETY-CONTRACT
//! record (preconditions, invariants, postconditions, failure policy,
//! owner, independent witness, mutation/fault seam — authored law, not a
//! field-name enum); docs/12 owns detection and hard-finding versus
//! ledgerable routing; docs/24 owns the hostile proof-row identities.
//! Narrowing numeric casts are NOT layout assumptions: a lossy numeric
//! conversion remains a hard production finding and may never be legalized
//! through a ledger entry.
//!
//! One physical occurrence may carry more than one obligation: a pointer
//! crossing that also uses unsafe Rust independently satisfies
//! UnsafeMemoryContract and therefore requires the SAFETY-CONTRACT marker.
//! The kinds do not collapse. An actual ledger row will additionally bind
//! the kind, an exact source anchor, the target/profile, and an explicit
//! assumption statement as required non-optional fields on the ledger
//! record type when real rows land — E3h adds no fake rows merely to adopt
//! a schema.

mod types;

pub use types::CompilerAssumptionKind;

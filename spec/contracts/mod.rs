//! The admitted contract kinds (5.5E3c).
//!
//! docs/06 owns the MacBat contract doctrine in prose; this file is the typed
//! authority for WHICH kinds are admitted at Gate 1. The doc's own entry law
//! decides membership: a kind enters only when one declaration becomes
//! canonical, a real adopter exists, and every lowering/proof surface is
//! named. The old "Expected families include" list carried ten names; two of
//! them — StateMachine and EvidenceBody — appeared nowhere else in the
//! corpus: no owning law, no adopter, no proof surface. They are NOT
//! variants. The enum is the admitted border crossing, not a brochure for
//! countries that might someday exist; either enters later by the same entry
//! law that admitted the eight.

mod types;

pub use types::ContractKind;

//! Typed SyncBat authority firewall (D4c2).
//!
//! `docs/08_SYNCBAT_RUNTIME.md` states the accepted law:
//!
//! ```text
//! runtime owns logical legality
//! pakvm owns program semantics
//! bvisor owns attempt admission and physical evidence
//! world owns composition and instance identity
//! port owns explicit host requests and responses
//! ```
//!
//! This file RATIFIES that law into typed authority. It is not a second model and
//! not a second topology: `spec/architecture.rs` owns the plane identities and
//! their package membership, and this file owns exactly one thing — which plane
//! owns which authority, and which authorities may lawfully cross which boundary.
//! docs/08's firewall section becomes a projection of this file.
//!
//! Why the firewall must be typed at all: all five planes live in one crate, so
//! no module boundary separates them. docs/08 says it plainly — "Sharing a crate
//! does not permit one plane to perform another's transition." Nothing structural
//! stops runtime from reaching a socket; only a law does.
//!
//! SCOPE. This models the SyncBat organism and nothing else. It is not a
//! universal authority framework, defines no scheduling, channels, queues,
//! executor internals, or containment backend, and assigns no opcode or byte.

mod admission;
mod inventory;
mod types;

pub use types::{
    AdmittedCrossing, CrossingAdmission, CrossingPosture, SYNCBAT_AUTHORITIES, SyncBatAuthority,
    SyncBatCrossing,
};
pub use inventory::SYNCBAT_LEGAL_CROSSINGS;
pub use admission::admit_crossing;

//! The typed Tier 0 qualification receipt algebra (5.5E6a; rebuilt from
//! shape-admission to evidence-verification in 5.5E6b).
//!
//! Three questions, three owners — declared once here, never repeated on the
//! individual receipt rows (5.5E6b):
//!
//! ```text
//! BP-RECEIPTS-1                    what a bootstrap qualification receipt MEANS
//! BP-BOOTSTRAP-1                   which Tier 0 gates form the required denominator
//! BP-PUBLIC-API-CI-RELEASE-1       which hosted target and run posture may close a gate
//! ```
//!
//! This is NOT the product receipt model (`ReceiptId`, docs/14). A product
//! receipt records what a running store proved about a data operation. A Tier 0
//! receipt records that a BOOTSTRAP gate binary was available, compiled,
//! executed, and passed on a physical target, bound to the source it ran
//! against and the concrete evidence it produced. The two families deliberately
//! share no type, field, spelling, or authority: `ReceiptId` can never
//! substitute for a `Tier0ReceiptKind`.
//!
//! ## What E6a did and what E6b adds
//!
//! E6a moved DENOMINATOR MEMBERSHIP out of Python: `Tier0ReceiptKind::ALL` is
//! the one denominator. But E6a admitted a SHAPE — a row of booleans — and its
//! sealed type discarded the concrete bindings. All-true booleans admitted; the
//! seal proved only that the seal existed.
//!
//! E6b admits CONCRETE EVIDENCE. A receipt carries typed stage outcomes (a
//! stage that was never attempted is distinct from one that failed), a typed
//! artifact-evidence sum whose variant must equal the kind's policy exactly, and
//! the whole qualification carries one source binding, one toolchain binding,
//! one target, and — for the authoritative target — one hosted-run binding. The
//! verified type RETAINS every concrete binding so the E6c comparator can ask it
//! which commit, which tree, which output tree, which run.
//!
//! ## The honest limit of the seal
//!
//! The private seal on `VerifiedTier0Qualification` prevents a consumer from
//! BYPASSING the verified shape — the only way to obtain one is through
//! `verify`. It does NOT cryptographically prove that an arbitrary caller
//! computed its verification inputs (the actual file digests, the real git
//! commit, the true toolchain identity) honestly. `bootstrap/receiptcheck.rs` is
//! the authoritative independent computer of those inputs: it recomputes every
//! digest from the bytes on disk, compares them against the artifact's claims,
//! and only then builds the observation this module verifies.

mod receipts;
mod digests;
mod evidence;
mod verify;

pub use receipts::*;
pub use digests::*;
pub use evidence::*;
pub use verify::*;

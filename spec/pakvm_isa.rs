//! Typed PakVM semantic ISA (D4c1).
//!
//! This file is the ONE authority for what a PakVM semantic node MEANS. It is
//! the first of the three separated ISA layers and never crosses into the other
//! two:
//!
//! ```text
//! semantic ISA    what an operation means             THIS FILE
//! VPak wire ISA   how an operation is encoded         DEC-036, frozen at G5
//! internal micro-ops / SpecializedPlan  how an operation is executed fast   DEC-073
//! ```
//!
//! No numeric opcode is assigned here and none may be inferred. `PakVmNodeId`
//! carries no `#[repr]` and no explicit discriminant: the declaration order below
//! is the authored docs/07 reading order, NOT an encoding. Exact opcode numbers
//! and the ISA version value are G5 implementation constants (docs/07), exact
//! VPak bytes are DEC-036 at G5, and EventFrameV2 bytes are G2. A node's identity
//! here is symbolic and stays symbolic.
//!
//! The node inventory is not invented: `07_PAKVM_ISA.md` authors exactly these
//! 36 nodes under exactly these three algebras, and `bootstrap/audit.py`
//! independently verifies that the typed inventory and the document agree.
//!
//! ADMISSION LAW (5.5D4b, applied here). Every field of a `PakVmNodeSpec` comes
//! from exactly one of: a value authored per node, a value declared once as
//! algebra policy, a value declared once as node-class policy, or one named
//! lawful derivation. A projector serializes an admitted spec; it may not
//! complete one. A node whose spec does not admit is not in the semantic ISA.
//! This is the same law that repaired the Guarantee Graph, applied before the
//! table exists rather than after it shipped.

mod vocab;
mod policies;
mod nodes;
mod admission;
#[cfg(test)]
mod tests;

pub use vocab::*;
pub use policies::*;
pub use nodes::*;
pub use admission::*;

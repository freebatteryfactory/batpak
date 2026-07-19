//! The one gate identity.
//!
//! Gate identity is an architectural concept referenced by every typed fact
//! family that names where a law is implemented or qualified. Before this
//! module it existed only as slash-delimited prose (`"G2/G5/G7"`), which no
//! validator could resolve: a typo'd `G12`, a repeated `G2/G2`, or drift from
//! the docs/25 inventory were all expressible and silently accepted.
//!
//! `GateId` is that identity. There is no `DecisionGateId`, `SeedGateId`, or
//! `GateName`; a second gate identity type is the defect this module exists to
//! remove. Gate-bearing facts carry `&'static [GateId]`, never a string.
//!
//! `docs/25_IMPLEMENTATION_GATES.md` owns the gate doctrine in prose; its gate
//! inventory table is a generated projection of `GATES` below. This file is the
//! authority for gate identity, order, and token spelling.

mod inventory;
mod types;

pub use types::{GateId, GateSpec};
pub use inventory::GATES;

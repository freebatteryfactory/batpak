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

/// The eleven gates of the destination order (docs/25). Declaration order is
/// canonical order: a gate list is canonical when its elements are ascending in
/// this order. `GJ` is the integrated final tree; it is a real review unit that
/// no individual fact currently names, and it is not deleted for being unused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GateId {
    G0,
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    G7,
    G8,
    G9,
    GJ,
}

/// One gate of the destination order. `token` is the rendered spelling; a
/// rendered gate set joins canonical tokens with `/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateSpec {
    pub id: GateId,
    pub token: &'static str,
    pub title: &'static str,
}

/// The gate inventory, in canonical order. Titles are the exact current
/// meanings from the docs/25 destination order; no gate is renamed, aliased,
/// added, or removed here.
pub const GATES: &[GateSpec] = &[
    GateSpec { id: GateId::G0, token: "G0", title: "Constitution and skeleton" },
    GateSpec { id: GateId::G1, token: "G1", title: "MacBat" },
    GateSpec { id: GateId::G2, token: "G2", title: "BatPak semantic and durable core" },
    GateSpec { id: GateId::G3, token: "G3", title: "TestPak seed" },
    GateSpec { id: GateId::G4, token: "G4", title: "BatQL compiler" },
    GateSpec { id: GateId::G5, token: "G5", title: "SyncBat world, ports, PakVM reference, Bvisor admission" },
    GateSpec { id: GateId::G6, token: "G6", title: "SyncBat logical runtime and recovery" },
    GateSpec { id: GateId::G7, token: "G7", title: "NetBat and product CLI" },
    GateSpec { id: GateId::G8, token: "G8", title: "Optimized tiles, codecs, and delivery succession" },
    GateSpec { id: GateId::G9, token: "G9", title: "Self-hosting and release seal" },
    GateSpec { id: GateId::GJ, token: "GJ", title: "Integrated final tree" },
];

impl GateId {
    /// The canonical rendered token for this gate.
    pub const fn token(self) -> &'static str {
        match self {
            GateId::G0 => "G0",
            GateId::G1 => "G1",
            GateId::G2 => "G2",
            GateId::G3 => "G3",
            GateId::G4 => "G4",
            GateId::G5 => "G5",
            GateId::G6 => "G6",
            GateId::G7 => "G7",
            GateId::G8 => "G8",
            GateId::G9 => "G9",
            GateId::GJ => "GJ",
        }
    }
}

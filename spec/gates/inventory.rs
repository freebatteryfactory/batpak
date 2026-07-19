use super::types::{GateId, GateSpec};

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
];

use crate::architecture::SyncBatPlane;

use super::types::{CrossingPosture, SyncBatAuthority, SyncBatCrossing};

/// Every lawful crossing. Anything absent is forbidden — the table is a
/// whitelist, because a blacklist of bad crossings is a list someone has to keep
/// imagining new entries for.
///
/// Every V1 crossing is `Required`: the accepted end-to-end turn cannot complete
/// if any one is deleted, because one of the five plane owners would then have no
/// lawful route by which to discharge or return a responsibility already assigned
/// to it. That is not asserted as a count of seven -- the required set is
/// whatever the posture column declares, and the connectivity proof in seedcheck
/// and audit.py recomputes it. Seven is merely today's derived inventory.
pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[
    SyncBatCrossing {
        from: SyncBatPlane::World,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::CompositionAndInstanceIdentity,
        posture: CrossingPosture::Required,
        law: "world supplies admitted composition and instance identity to the turn",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Runtime,
        to: SyncBatPlane::PakVm,
        carries: SyncBatAuthority::SemanticAuthorization,
        posture: CrossingPosture::Required,
        law: "runtime authorizes a semantic operation it has found legal",
    },
    SyncBatCrossing {
        from: SyncBatPlane::PakVm,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::SemanticNodeInterpretation,
        posture: CrossingPosture::Required,
        law: "pakvm returns what an admitted node means",
    },
    SyncBatCrossing {
        from: SyncBatPlane::PakVm,
        to: SyncBatPlane::Bvisor,
        carries: SyncBatAuthority::TypedEffectRequest,
        posture: CrossingPosture::Required,
        law: "an effectful node emits a typed request for physical admission",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Bvisor,
        to: SyncBatPlane::Port,
        carries: SyncBatAuthority::PhysicalAttempt,
        posture: CrossingPosture::Required,
        law: "bvisor executes an admitted attempt through the typed host boundary",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Port,
        to: SyncBatPlane::Bvisor,
        carries: SyncBatAuthority::TypedHostResponse,
        posture: CrossingPosture::Required,
        law: "the port returns a typed host response to the attempt that made it",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Bvisor,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::AttemptEvidence,
        posture: CrossingPosture::Required,
        law: "bvisor reports what the attempt did; runtime decides what it means",
    },
];

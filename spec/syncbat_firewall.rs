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

use crate::architecture::SyncBatPlane;
use crate::pakvm_isa::{admit, EffectPosture, PakVmAdmission, PakVmNodeId};

/// One distinguishable authority inside SyncBat.
///
/// These stay separate on purpose. Collapsing them into one status, one receipt,
/// or one message envelope is how a physical attempt receipt comes to satisfy a
/// logical semantic receipt: not through a decision anyone made, but because one
/// type was convenient enough to carry both meanings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncBatAuthority {
    /// Whether a logical turn is legal at all.
    LogicalLegality,
    /// Whether a specific semantic operation may proceed.
    SemanticAuthorization,
    /// The logical result of a legal turn.
    LogicalResult,
    /// The receipt attesting a logical execution.
    SemanticReceipt,
    /// Whether a semantic operation may be retried, restarted, or replayed.
    RetryRestartAuthority,
    /// What an admitted PakVM node means.
    SemanticNodeInterpretation,
    /// A typed effect request emitted by an effectful node. An emission, never an
    /// effect: the node asks, it does not act.
    TypedEffectRequest,
    /// Whether a capability requirement is satisfied.
    CapabilityAdmission,
    /// Whether a physical attempt is admitted.
    PhysicalAdmission,
    /// Execution of one physical attempt.
    PhysicalAttempt,
    /// Evidence of what a physical attempt actually did.
    AttemptEvidence,
    /// Admitted composition, wiring, and instance identity.
    CompositionAndInstanceIdentity,
    /// A typed request at the host boundary.
    TypedHostRequest,
    /// A typed response from the host boundary.
    TypedHostResponse,
}

impl SyncBatAuthority {
    /// The one plane that owns this authority.
    ///
    /// Total by construction, and a partition: every authority has exactly one
    /// owner, so "who decides this" never has two answers. Each assignment is
    /// derived from docs/08's ownership sentences, not chosen for symmetry.
    pub const fn owner(self) -> SyncBatPlane {
        match self {
            // "runtime owns logical legality". Retry legality is a legality
            // question, so it lands here rather than with the plane that happens
            // to execute the attempt. Bvisor observing a failed attempt is not
            // Bvisor deciding the operation may be tried again.
            SyncBatAuthority::LogicalLegality
            | SyncBatAuthority::SemanticAuthorization
            | SyncBatAuthority::LogicalResult
            | SyncBatAuthority::SemanticReceipt
            | SyncBatAuthority::RetryRestartAuthority => SyncBatPlane::Runtime,
            // "pakvm owns program semantics". An effectful node emits a request;
            // the request is PakVM's because its meaning is PakVM's.
            SyncBatAuthority::SemanticNodeInterpretation
            | SyncBatAuthority::TypedEffectRequest => SyncBatPlane::PakVm,
            // "bvisor owns attempt admission and physical evidence". Capability
            // admission is containment, and containment is admission.
            SyncBatAuthority::CapabilityAdmission
            | SyncBatAuthority::PhysicalAdmission
            | SyncBatAuthority::PhysicalAttempt
            | SyncBatAuthority::AttemptEvidence => SyncBatPlane::Bvisor,
            // "world owns composition and instance identity".
            SyncBatAuthority::CompositionAndInstanceIdentity => SyncBatPlane::World,
            // "port owns explicit host requests and responses".
            SyncBatAuthority::TypedHostRequest
            | SyncBatAuthority::TypedHostResponse => SyncBatPlane::Port,
        }
    }

    /// Whether exercising this authority must name the admitted PakVM node it
    /// came from. Semantic authorities carry their origin; physical and
    /// compositional ones do not have one to carry.
    pub const fn requires_semantic_origin(self) -> bool {
        matches!(
            self,
            SyncBatAuthority::SemanticNodeInterpretation | SyncBatAuthority::TypedEffectRequest
        )
    }
}

/// Every authority, in owner order.
pub const SYNCBAT_AUTHORITIES: &[SyncBatAuthority] = &[
    SyncBatAuthority::LogicalLegality,
    SyncBatAuthority::SemanticAuthorization,
    SyncBatAuthority::LogicalResult,
    SyncBatAuthority::SemanticReceipt,
    SyncBatAuthority::RetryRestartAuthority,
    SyncBatAuthority::SemanticNodeInterpretation,
    SyncBatAuthority::TypedEffectRequest,
    SyncBatAuthority::CapabilityAdmission,
    SyncBatAuthority::PhysicalAdmission,
    SyncBatAuthority::PhysicalAttempt,
    SyncBatAuthority::AttemptEvidence,
    SyncBatAuthority::CompositionAndInstanceIdentity,
    SyncBatAuthority::TypedHostRequest,
    SyncBatAuthority::TypedHostResponse,
];

/// One lawful boundary crossing.
///
/// A crossing TRANSPORTS an already-owned value. It never transfers ownership:
/// after `Bvisor -> Runtime` carries `AttemptEvidence`, Bvisor still owns attempt
/// evidence and Runtime still cannot mint any. This is why the table is keyed by
/// the authority rather than by a generic message: the value moves, the authority
/// does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncBatCrossing {
    pub from: SyncBatPlane,
    pub to: SyncBatPlane,
    pub carries: SyncBatAuthority,
    /// The authored law this crossing realizes.
    pub law: &'static str,
}

/// Every lawful crossing. Anything absent is forbidden — the table is a
/// whitelist, because a blacklist of bad crossings is a list someone has to keep
/// imagining new entries for.
pub const SYNCBAT_LEGAL_CROSSINGS: &[SyncBatCrossing] = &[
    SyncBatCrossing {
        from: SyncBatPlane::World,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::CompositionAndInstanceIdentity,
        law: "world supplies admitted composition and instance identity to the turn",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Runtime,
        to: SyncBatPlane::PakVm,
        carries: SyncBatAuthority::SemanticAuthorization,
        law: "runtime authorizes a semantic operation it has found legal",
    },
    SyncBatCrossing {
        from: SyncBatPlane::PakVm,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::SemanticNodeInterpretation,
        law: "pakvm returns what an admitted node means",
    },
    SyncBatCrossing {
        from: SyncBatPlane::PakVm,
        to: SyncBatPlane::Bvisor,
        carries: SyncBatAuthority::TypedEffectRequest,
        law: "an effectful node emits a typed request for physical admission",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Bvisor,
        to: SyncBatPlane::Port,
        carries: SyncBatAuthority::PhysicalAttempt,
        law: "bvisor executes an admitted attempt through the typed host boundary",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Port,
        to: SyncBatPlane::Bvisor,
        carries: SyncBatAuthority::TypedHostResponse,
        law: "the port returns a typed host response to the attempt that made it",
    },
    SyncBatCrossing {
        from: SyncBatPlane::Bvisor,
        to: SyncBatPlane::Runtime,
        carries: SyncBatAuthority::AttemptEvidence,
        law: "bvisor reports what the attempt did; runtime decides what it means",
    },
];

/// An admitted crossing.
///
/// Constructible only by `admit_crossing`: the private unit field means no plane,
/// generator, auditor, or fixture can mint one. A value of this type IS the proof
/// that the firewall allowed it, rather than a promise that someone checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedCrossing {
    pub from: SyncBatPlane,
    pub to: SyncBatPlane,
    pub carries: SyncBatAuthority,
    pub origin: Option<PakVmNodeId>,
    pub law: &'static str,
    seal: (),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingAdmission {
    Admitted(AdmittedCrossing),
    Refused(&'static str),
}

fn declared_crossing(
    from: SyncBatPlane,
    to: SyncBatPlane,
    carries: SyncBatAuthority,
) -> Option<&'static SyncBatCrossing> {
    let mut i = 0;
    while i < SYNCBAT_LEGAL_CROSSINGS.len() {
        let c = &SYNCBAT_LEGAL_CROSSINGS[i];
        if c.from == from && c.to == to && c.carries == carries {
            return Some(c);
        }
        i += 1;
    }
    None
}

/// Admit one authority crossing, or refuse it with the reason.
///
/// This is the whole production API. There is no injection seam and none is
/// needed: the caller already proposes the crossing, so a hostile fixture can
/// propose an unlawful one through the same door production uses. Nothing here
/// can be exercised in a test that cannot be exercised in production, which is
/// the point — a firewall with a test-only entrance is a firewall with an
/// entrance.
pub fn admit_crossing(
    from: SyncBatPlane,
    to: SyncBatPlane,
    carries: SyncBatAuthority,
    origin: Option<PakVmNodeId>,
) -> CrossingAdmission {
    if from == to {
        return CrossingAdmission::Refused(
            "a plane does not cross to itself; that is an internal transition",
        );
    }
    // Ownership first. This is the rule that does the real work, and it is
    // derived rather than listed: docs/08's forbidden examples all reduce to a
    // plane exercising an authority it does not own. Bvisor minting semantic
    // restart, runtime fabricating attempt evidence, and world redefining
    // BatPak-owned identity are the same defect wearing three coats.
    if carries.owner() != from {
        return CrossingAdmission::Refused(
            "the sending plane does not own the authority it is exercising",
        );
    }
    let declared = match declared_crossing(from, to, carries) {
        Some(c) => c,
        None => {
            return CrossingAdmission::Refused(
                "no lawful crossing carries this authority between these planes",
            )
        }
    };
    // A semantic authority names the admitted node it came from. Without this,
    // "the ISA is the only execution route" would be a sentence in a document
    // rather than a condition on a value.
    let origin = match (carries.requires_semantic_origin(), origin) {
        (true, None) => {
            return CrossingAdmission::Refused("a semantic authority names no PakVM origin")
        }
        (false, Some(_)) => {
            return CrossingAdmission::Refused(
                "a non-semantic authority names a PakVM origin it cannot have",
            )
        }
        (false, None) => None,
        (true, Some(node)) => {
            // ISA-ONLY EXECUTION. The origin must be a node the D4c1 authority
            // admits. A fabricated identity cannot reach this point: PakVmNodeId
            // has no constructor beyond its authored variants, so the type
            // forecloses the value. What admission adds is that the node's spec
            // must still resolve.
            match admit(node) {
                PakVmAdmission::Refused(_) => {
                    return CrossingAdmission::Refused(
                        "the named PakVM origin does not admit into the semantic ISA",
                    )
                }
                PakVmAdmission::Admitted(spec) => {
                    // Only an effectful node may emit an effect request. A pure
                    // or observational node doing so would be an effect appearing
                    // in a program the validator cleared as pure.
                    if matches!(carries, SyncBatAuthority::TypedEffectRequest)
                        && !matches!(spec.effect, EffectPosture::Effectful)
                    {
                        return CrossingAdmission::Refused(
                            "a node that does not admit as Effectful emits an effect request",
                        );
                    }
                    Some(node)
                }
            }
        }
    };
    CrossingAdmission::Admitted(AdmittedCrossing {
        from,
        to,
        carries,
        origin,
        law: declared.law,
        seal: (),
    })
}

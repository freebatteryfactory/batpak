use crate::architecture::SyncBatPlane;
use crate::pakvm_isa::PakVmNodeId;

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

/// Whether a lawful crossing is part of the accepted machine or merely allowed.
///
/// Permission and requiredness are orthogonal. Permission answers "may this
/// crossing legally exist"; requiredness answers "must it exist for the accepted
/// V1 machine to be realizable at all". A firewall that proved only the first
/// would be safe the way a vault welded shut is safe: no unauthorized door
/// opens, and no authorized one does either. Deleting a required route preserves
/// safety by destroying liveness, and that is still a specification failure --
/// it just lands on the liveness side.
///
/// This is a narrow posture, not a loose flag. There is no default: every row
/// states its posture, because an absent posture silently reading as `Optional`
/// is exactly how a required door gets quietly bricked over. Today every V1 row
/// is `Required`; the variant `Optional` earns its place because the two
/// dimensions are genuinely independent and a future legal-but-optional crossing
/// may exist, not as aesthetic furniture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingPosture {
    /// The accepted machine cannot complete its turn without this route: some
    /// plane owner could not discharge or return a responsibility already
    /// assigned to it. It must be present, legal, and direction-preserving.
    Required,
    /// The machine functions without it. If present it must still be legal, but
    /// its absence is not a contradiction.
    Optional,
}

/// One lawful boundary crossing.
///
/// A crossing TRANSPORTS an already-owned value. It never transfers ownership:
/// after `Bvisor -> Runtime` carries `AttemptEvidence`, Bvisor still owns attempt
/// evidence and Runtime still cannot mint any. This is why the table is keyed by
/// the authority rather than by a generic message: the value moves, the authority
/// does not.
///
/// The row is the single owner of all of its facts: source plane, destination
/// plane, carried authority, direction, and requiredness posture. There is no
/// separate required-crossing registry to drift against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncBatCrossing {
    pub from: SyncBatPlane,
    pub to: SyncBatPlane,
    pub carries: SyncBatAuthority,
    /// Whether the accepted machine requires this route or merely permits it.
    pub posture: CrossingPosture,
    /// The authored law this crossing realizes. For a `Required` crossing this
    /// prose states the producer/consumer obligation the route discharges; the
    /// executable proof that the obligation stays connected lives in seedcheck
    /// and audit.py, not here.
    pub law: &'static str,
}

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
    pub posture: CrossingPosture,
    pub origin: Option<PakVmNodeId>,
    pub law: &'static str,
    pub(super) seal: (),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingAdmission {
    Admitted(AdmittedCrossing),
    Refused(&'static str),
}

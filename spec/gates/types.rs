/// The ten gates of the destination order (docs/25). Declaration order is
/// canonical order: a gate list is canonical when its elements are ascending in
/// this order.
//
// GateId::GJ ("Integrated final tree") stood here until the 5.5E1 ruling. It
// was defended as "a real review unit no fact currently names" — but a gate
// with no independent transition, no owner, and no consumer is an acceptance
// CONDITION, not a phase. Its refusal list is folded into G9's release seal
// (docs/25), which already owned self-hosting and sealing; the integrated
// final tree is what G9 refuses to seal without. Same law as GateResolution
// and GuaranteeEdgeKind: unconsumed vocabulary claiming to be law is deleted,
// and any fact still naming GJ now refuses at compile time.
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
}

/// One gate of the destination order. `token` is the rendered spelling; a
/// rendered gate set joins canonical tokens with `/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateSpec {
    pub id: GateId,
    pub token: &'static str,
    pub title: &'static str,
}

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
        }
    }
}

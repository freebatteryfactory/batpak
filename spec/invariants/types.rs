use crate::gates::GateId;
use crate::guarantees::{GuaranteeKind, GuaranteeLifetime, GuaranteeRef, WitnessRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvariantSpec {
    pub id: &'static str,
    pub statement: &'static str,
    pub kind: GuaranteeKind,
    pub lifetime: GuaranteeLifetime,
    pub owner: &'static str,
    pub gates: &'static [GateId],
    /// Typed witness citations (5.5E2): WHICH owned evidence obligations this
    /// law depends on. Every reference resolves or the row is refused.
    pub witnesses: &'static [WitnessRef],
    /// How a human should understand the cited evidence. May be empty.
    pub witness_note: &'static str,
    pub failure_disposition: &'static str,
    /// Typed relations (5.5E2): a reference names its family in the type, so
    /// a decision cited as a legacy obligation has no spelling. Whether each
    /// reference resolves to a declared row is an executed seedcheck law.
    pub derives_from: &'static [GuaranteeRef],
    pub refines: &'static [GuaranteeRef],
    pub discharges: &'static [GuaranteeRef],
    pub supersedes: &'static [GuaranteeRef],
}

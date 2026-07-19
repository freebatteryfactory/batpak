use crate::guarantees::{ContractId, GuaranteeRef, SeedId};

/// The declared corpus epochs. The inventory may grow; the CURRENT
/// selection moves only by deliberate whole-corpus reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationEpoch {
    /// The currently reconciled clean-room V1 corpus.
    CleanroomV1,
}

/// The one current corpus epoch every eligible document claims.
pub const CURRENT_RECONCILIATION_EPOCH: ReconciliationEpoch = ReconciliationEpoch::CleanroomV1;

impl ReconciliationEpoch {
    pub const ALL: &'static [ReconciliationEpoch] = &[ReconciliationEpoch::CleanroomV1];

    pub const fn spelling(self) -> &'static str {
        match self {
            ReconciliationEpoch::CleanroomV1 => "cleanroom-v1",
        }
    }

    /// The Constitution owns document-status doctrine, and it must author
    /// both the type name and the canonical spelling.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            ReconciliationEpoch::CleanroomV1 => ContractId("BP-CONSTITUTION-1"),
        }
    }

    /// The standing seed row whose statement names ReconciliationEpoch.
    pub const fn admission_basis(self) -> GuaranteeRef {
        match self {
            ReconciliationEpoch::CleanroomV1 => {
                GuaranteeRef::Seed(SeedId("SEED-DOC-STATUS"))
            }
        }
    }
}

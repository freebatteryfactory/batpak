use crate::guarantees::{ContractId, DecisionId};

use super::types::{IdentityTermDisposition, NonCatalogedIdentityTerm};

/// Every suffix-bearing corpus term the catalogs deliberately do not admit.
/// Discovery covers the AUTHORED corpus only — generated projection blocks
/// and this file are excluded, so the denominator never proves itself by
/// rereading its own answer sheet. GuaranteeId is deliberately absent: its
/// only documentary appearances are generated projections of the typed
/// guarantee vocabulary, so it is not an authored corpus term. GateId and
/// OperatorId have typed spec owners whose
/// documentary contracts are cited; NavigationId is docs/16 navigation
/// vocabulary; EntityId and the application-shaped ids are the BatQL
/// companion's example material; SystemId and TypeId are docs/18 ECS
/// implementation algebra; CompressionId is a passport application stapled
/// to DEC-063's very demanding checklist — the locked decision names the
/// prerequisites a future compressed profile must meet, so the term is NOT
/// YET admitted rather than rejected, and it enters only by amending
/// DEC-063 and joining exactly one catalog.
pub const NON_CATALOGED_IDENTITY_TERMS: &[NonCatalogedIdentityTerm] = &[
    NonCatalogedIdentityTerm {
        term: "GateId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-GATES-1")),
    },
    NonCatalogedIdentityTerm {
        term: "OperatorId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "ProofRowId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-GAUNTLET-1")),
    },
    NonCatalogedIdentityTerm {
        term: "NavigationId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-IDENTITY-TIME-NAV-1")),
    },
    NonCatalogedIdentityTerm {
        term: "EntityId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "CustomerId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "InvoiceId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "OrderId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "BorrowerId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "TenantId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-BATQL-LANGUAGE-1")),
    },
    NonCatalogedIdentityTerm {
        term: "SystemId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-ECS-1")),
    },
    NonCatalogedIdentityTerm {
        term: "TypeId",
        disposition: IdentityTermDisposition::OwnedElsewhere(ContractId("BP-ECS-1")),
    },
    NonCatalogedIdentityTerm {
        term: "CompressionId",
        disposition: IdentityTermDisposition::NotYetAdmittedBy(DecisionId("DEC-063")),
    },
];

/// Spellings whose types already have owners in other spec modules
/// (architecture.rs, proof.rs, gates.rs, operators.rs). The catalogs may
/// REFERENCE those types where necessary; they may never re-admit the
/// spellings as catalog entries — a duplicate variant is a wrapper passport.
/// GateId and OperatorId additionally appear in the residue table as
/// owned-elsewhere REFERENCES, which is lawful; a catalog entry would not be.
pub const EXISTING_TYPED_OWNER_SPELLINGS: &[&str] = &["PackageId", "ProofRowId", "GateId", "OperatorId"];

/// The vocabulary this catalog must NEVER admit: time, order, topology,
/// coordinate, cursor, and navigation terms owned by docs/16's other
/// sections and spec/reconciliation.rs. Executed exclusion law: no catalog
/// spelling may equal one of these.
pub const EXCLUDED_CHRONOLOGY_AND_NAVIGATION: &[&str] = &[
    "ObservedWallTime",
    "TimeDelta",
    "MonotonicDeadline",
    "Hlc",
    "GlobalSequence",
    "CommitPoint",
    "StreamPosition",
    "DagPosition",
    "Coordinate",
    "PageCursor",
    "WorldPath",
    "ProjectionPath",
];

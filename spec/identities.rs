//! The identity, generation, binding, and version catalogs (5.5E3d).
//!
//! docs/16 owns the identity doctrine in prose; this file is the typed
//! authority for WHICH stable vocabularies exist on each of four axes. The
//! axes are SEPARATE closed vocabularies, never one mega enum:
//!
//! ```text
//! IdentityKind          which semantic, instance, event, operation, or
//!                       evidence OBJECT
//! GenerationKind        which evolution or authority generation
//! BindingKind           which bytes, interface, or history COMMITMENT
//! VersionIdentityKind   which independently versioned format, protocol,
//!                       language, or schema
//! ```
//!
//! Chronology, order, topology, coordinates, cursors, and navigation are NOT
//! identity vocabulary and have no spelling here: ObservedWallTime,
//! TimeDelta, MonotonicDeadline, Hlc, GlobalSequence, CommitPoint,
//! StreamPosition, DagPosition, Coordinate, PageCursor, WorldPath, and
//! ProjectionPath stay owned by docs/16's time/order/navigation sections and
//! `spec/reconciliation.rs`. An identity may bind or reference a CommitPoint;
//! this catalog defines no CommitPoint comparison, no HLC ordering or
//! tie-breaking, no frontier progress, and no part of the DEC-075
//! dual-coordinate theorem.
//!
//! Existing typed owners stay owners: PackageId, ProofRowId, GateId, and
//! ContractKind are referenced where needed and never redefined or wrapped.
//! No byte layout, UUID layout, wire encoding, ordering implementation, or
//! ID generator is selected here — those belong to the owning implementation
//! gates.
//!
//! Every entry names its canonical semantic owner by declared contract id,
//! and the owner reference resolves or admission refuses. The inventory was
//! derived by corpus sweep (every stable `...Id`, `...Hash`, `...Digest`,
//! `...Commitment`, `...Generation`, `...Version` in the authoritative
//! docs), never by copying the old docs/16 list — which omitted EventId,
//! LogicalOperationId, and LayoutVersion, and misfiled AuthorityGeneration,
//! ContentDigest, and Commitment as object identities. CompressionId is
//! deliberately ABSENT: DEC-063 names it as a future compressed profile's
//! admission requirement, and a requirement for an unadmitted profile is a
//! brochure entry, not an identity.

use crate::guarantees::{ContractId, DecisionId};

/// Which semantic, instance, event, operation, or evidence OBJECT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityKind {
    Contract,
    Schema,
    Codec,
    Layout,
    Materialization,
    Store,
    /// Restored by the 5.5E3d sweep: the event is the core durable object
    /// and its identity was absent from the documented stack.
    Event,
    /// The logical operation an attempt executes; reconciliation READS this
    /// identity as a coordinate carrier, identity OWNS it.
    LogicalOperation,
    ProgramImage,
    ContractImage,
    AuthorityImage,
    WorldImage,
    WorldInstance,
    ProcessInstance,
    Turn,
    Attempt,
    Receipt,
    Entrypoint,
    Invocation,
    Correlation,
    Tile,
    KernelContract,
    KernelImplementation,
    QualifiedKernel,
    KernelQualificationReceipt,
    Key,
    KeyBackend,
    Rotation,
    Rewrap,
    Unit,
    NumericProfile,
    FloatFormat,
    ApproximationProfile,
    WideExactProfile,
    QuantizationPolicy,
    RoundingMode,
}

/// Which evolution or authority generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationKind {
    Authority,
    Key,
    Process,
    World,
    Materialization,
}

/// Which bytes, interface, or history COMMITMENT. A binding proves a
/// relationship to content; it is never the object's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    ContentDigest,
    Commitment,
    EventCommitment,
    CommitmentDigest,
    WorldInterfaceHash,
    KernelInterfaceHash,
    CapabilityGrantHash,
    InputDigest,
    OutputDigest,
    EffectBatchDigest,
}

/// Which independently versioned format, protocol, language, or schema
/// (DEC-064): distinct version types do not typecheck when substituted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionIdentityKind {
    BatQlLanguage,
    ProgramImage,
    WorldImage,
    PakVmIsa,
    FbatFormat,
    BatTaggedRecord,
    /// Versions the EventFrame envelope (authored adopter: EventFrameV2 in
    /// docs/05). Distinct from FbatFormatVersion, which versions the .fbat
    /// container format, and from BatTaggedRecordVersion, which versions the
    /// payload record codec.
    Frame,
    NetBatProtocol,
    KernelManifest,
    ReceiptSchema,
    Schema,
    /// Sweep-discovered beside LayoutId in docs/02; the old docs/16 version
    /// fence omitted it.
    Layout,
    /// The Tier 0 qualification evidence artifact format (5.5E6b). An
    /// independently versioned, line-oriented bootstrap format — distinct from
    /// `ReceiptSchema` (the product receipt codec), the `ReleaseSeal` schema,
    /// and the product `ReceiptId`. Its first line names its own version.
    Tier0QualificationArtifact,
}

/// One catalog entry: the canonical spelling and the declared contract that
/// owns the term's semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub spelling: &'static str,
    pub owner: ContractId,
}

macro_rules! entry {
    ($spelling:literal, $owner:literal) => {
        CatalogEntry { spelling: $spelling, owner: ContractId($owner) }
    };
}

impl IdentityKind {
    pub const ALL: &'static [IdentityKind] = &[
        IdentityKind::Contract,
        IdentityKind::Schema,
        IdentityKind::Codec,
        IdentityKind::Layout,
        IdentityKind::Materialization,
        IdentityKind::Store,
        IdentityKind::Event,
        IdentityKind::LogicalOperation,
        IdentityKind::ProgramImage,
        IdentityKind::ContractImage,
        IdentityKind::AuthorityImage,
        IdentityKind::WorldImage,
        IdentityKind::WorldInstance,
        IdentityKind::ProcessInstance,
        IdentityKind::Turn,
        IdentityKind::Attempt,
        IdentityKind::Receipt,
        IdentityKind::Entrypoint,
        IdentityKind::Invocation,
        IdentityKind::Correlation,
        IdentityKind::Tile,
        IdentityKind::KernelContract,
        IdentityKind::KernelImplementation,
        IdentityKind::QualifiedKernel,
        IdentityKind::KernelQualificationReceipt,
        IdentityKind::Key,
        IdentityKind::KeyBackend,
        IdentityKind::Rotation,
        IdentityKind::Rewrap,
        IdentityKind::Unit,
        IdentityKind::NumericProfile,
        IdentityKind::FloatFormat,
        IdentityKind::ApproximationProfile,
        IdentityKind::WideExactProfile,
        IdentityKind::QuantizationPolicy,
        IdentityKind::RoundingMode,
    ];

    pub const fn entry(self) -> CatalogEntry {
        match self {
            IdentityKind::Contract => entry!("ContractId", "BP-IDENTITY-TIME-NAV-1"),
            IdentityKind::Schema => entry!("SchemaId", "BP-SCHEMA-CODEC-1"),
            IdentityKind::Codec => entry!("CodecId", "BP-SCHEMA-CODEC-1"),
            IdentityKind::Layout => entry!("LayoutId", "BP-SYSTEM-MODEL-1"),
            IdentityKind::Materialization => entry!("MaterializationId", "BP-STORAGE-TILES-1"),
            IdentityKind::Store => entry!("StoreId", "BP-STORAGE-TILES-1"),
            IdentityKind::Event => entry!("EventId", "BP-STORAGE-TILES-1"),
            IdentityKind::LogicalOperation => entry!("LogicalOperationId", "BP-IDENTITY-TIME-NAV-1"),
            IdentityKind::ProgramImage => entry!("ProgramImageId", "BP-IDENTITY-TIME-NAV-1"),
            IdentityKind::ContractImage => entry!("ContractImageId", "BP-WORLD-PORTS-1"),
            IdentityKind::AuthorityImage => entry!("AuthorityImageId", "BP-STORAGE-TILES-1"),
            IdentityKind::WorldImage => entry!("WorldImageId", "BP-IDENTITY-TIME-NAV-1"),
            IdentityKind::WorldInstance => entry!("WorldInstanceId", "BP-WORLD-PORTS-1"),
            IdentityKind::ProcessInstance => entry!("ProcessInstanceId", "BP-SYNCBAT-1"),
            IdentityKind::Turn => entry!("TurnId", "BP-IDENTITY-TIME-NAV-1"),
            IdentityKind::Attempt => entry!("AttemptId", "BP-BVISOR-1"),
            IdentityKind::Receipt => entry!("ReceiptId", "BP-RECEIPTS-1"),
            IdentityKind::Entrypoint => entry!("EntrypointId", "BP-WORLD-PORTS-1"),
            IdentityKind::Invocation => entry!("InvocationId", "BP-WORLD-PORTS-1"),
            IdentityKind::Correlation => entry!("CorrelationId", "BP-STORAGE-TILES-1"),
            IdentityKind::Tile => entry!("TileId", "BP-STORAGE-TILES-1"),
            IdentityKind::KernelContract => entry!("KernelContractId", "BP-PUBLIC-API-CI-RELEASE-1"),
            IdentityKind::KernelImplementation => {
                entry!("KernelImplementationId", "BP-PUBLIC-API-CI-RELEASE-1")
            }
            IdentityKind::QualifiedKernel => entry!("QualifiedKernelId", "BP-PUBLIC-API-CI-RELEASE-1"),
            IdentityKind::KernelQualificationReceipt => {
                entry!("KernelQualificationReceiptId", "BP-PUBLIC-API-CI-RELEASE-1")
            }
            IdentityKind::Key => entry!("KeyId", "BP-CRYPTO-SECRET-1"),
            IdentityKind::KeyBackend => entry!("KeyBackendId", "BP-CRYPTO-SECRET-1"),
            IdentityKind::Rotation => entry!("RotationId", "BP-CRYPTO-SECRET-1"),
            IdentityKind::Rewrap => entry!("RewrapId", "BP-CRYPTO-SECRET-1"),
            IdentityKind::Unit => entry!("UnitId", "BP-NUMERIC-1"),
            IdentityKind::NumericProfile => entry!("NumericProfileId", "BP-NUMERIC-1"),
            IdentityKind::FloatFormat => entry!("FloatFormatId", "BP-NUMERIC-1"),
            IdentityKind::ApproximationProfile => entry!("ApproximationProfileId", "BP-NUMERIC-1"),
            IdentityKind::WideExactProfile => entry!("WideExactProfileId", "BP-NUMERIC-1"),
            IdentityKind::QuantizationPolicy => entry!("QuantizationPolicyId", "BP-NUMERIC-1"),
            IdentityKind::RoundingMode => entry!("RoundingModeId", "BP-NUMERIC-1"),
        }
    }
}

impl GenerationKind {
    pub const ALL: &'static [GenerationKind] = &[
        GenerationKind::Authority,
        GenerationKind::Key,
        GenerationKind::Process,
        GenerationKind::World,
        GenerationKind::Materialization,
    ];

    pub const fn entry(self) -> CatalogEntry {
        match self {
            GenerationKind::Authority => entry!("AuthorityGeneration", "BP-STORAGE-TILES-1"),
            GenerationKind::Key => entry!("KeyGeneration", "BP-CRYPTO-SECRET-1"),
            GenerationKind::Process => entry!("ProcessGeneration", "BP-SYNCBAT-1"),
            GenerationKind::World => entry!("WorldGeneration", "BP-WORLD-PORTS-1"),
            GenerationKind::Materialization => {
                entry!("MaterializationGeneration", "BP-STORAGE-TILES-1")
            }
        }
    }
}

impl BindingKind {
    pub const ALL: &'static [BindingKind] = &[
        BindingKind::ContentDigest,
        BindingKind::Commitment,
        BindingKind::EventCommitment,
        BindingKind::CommitmentDigest,
        BindingKind::WorldInterfaceHash,
        BindingKind::KernelInterfaceHash,
        BindingKind::CapabilityGrantHash,
        BindingKind::InputDigest,
        BindingKind::OutputDigest,
        BindingKind::EffectBatchDigest,
    ];

    pub const fn entry(self) -> CatalogEntry {
        match self {
            BindingKind::ContentDigest => entry!("ContentDigest", "BP-STORAGE-TILES-1"),
            BindingKind::Commitment => entry!("Commitment", "BP-IDENTITY-TIME-NAV-1"),
            BindingKind::EventCommitment => entry!("EventCommitment", "BP-STORAGE-TILES-1"),
            BindingKind::CommitmentDigest => entry!("CommitmentDigest", "BP-CRYPTO-SECRET-1"),
            BindingKind::WorldInterfaceHash => entry!("WorldInterfaceHash", "BP-WORLD-PORTS-1"),
            BindingKind::KernelInterfaceHash => {
                entry!("KernelInterfaceHash", "BP-PUBLIC-API-CI-RELEASE-1")
            }
            BindingKind::CapabilityGrantHash => entry!("CapabilityGrantHash", "BP-BVISOR-1"),
            BindingKind::InputDigest => entry!("InputDigest", "BP-RECEIPTS-1"),
            BindingKind::OutputDigest => entry!("OutputDigest", "BP-RECEIPTS-1"),
            BindingKind::EffectBatchDigest => entry!("EffectBatchDigest", "BP-SYNCBAT-1"),
        }
    }
}

impl VersionIdentityKind {
    pub const ALL: &'static [VersionIdentityKind] = &[
        VersionIdentityKind::BatQlLanguage,
        VersionIdentityKind::ProgramImage,
        VersionIdentityKind::WorldImage,
        VersionIdentityKind::PakVmIsa,
        VersionIdentityKind::FbatFormat,
        VersionIdentityKind::BatTaggedRecord,
        VersionIdentityKind::Frame,
        VersionIdentityKind::NetBatProtocol,
        VersionIdentityKind::KernelManifest,
        VersionIdentityKind::ReceiptSchema,
        VersionIdentityKind::Schema,
        VersionIdentityKind::Layout,
        VersionIdentityKind::Tier0QualificationArtifact,
    ];

    pub const fn entry(self) -> CatalogEntry {
        match self {
            VersionIdentityKind::BatQlLanguage => entry!("BatQlLanguageVersion", "BP-BATQL-LANGUAGE-1"),
            VersionIdentityKind::ProgramImage => entry!("ProgramImageVersion", "BP-IDENTITY-TIME-NAV-1"),
            VersionIdentityKind::WorldImage => entry!("WorldImageVersion", "BP-IDENTITY-TIME-NAV-1"),
            VersionIdentityKind::PakVmIsa => entry!("PakVmIsaVersion", "BP-PAKVM-ISA-1"),
            VersionIdentityKind::FbatFormat => entry!("FbatFormatVersion", "BP-STORAGE-TILES-1"),
            VersionIdentityKind::BatTaggedRecord => {
                entry!("BatTaggedRecordVersion", "BP-SCHEMA-CODEC-1")
            }
            VersionIdentityKind::Frame => entry!("FrameVersion", "BP-STORAGE-TILES-1"),
            VersionIdentityKind::NetBatProtocol => entry!("NetBatProtocolVersion", "BP-NETBAT-1"),
            VersionIdentityKind::KernelManifest => {
                entry!("KernelManifestVersion", "BP-PUBLIC-API-CI-RELEASE-1")
            }
            VersionIdentityKind::ReceiptSchema => entry!("ReceiptSchemaVersion", "BP-RECEIPTS-1"),
            VersionIdentityKind::Schema => entry!("SchemaVersion", "BP-SCHEMA-CODEC-1"),
            VersionIdentityKind::Layout => entry!("LayoutVersion", "BP-SYSTEM-MODEL-1"),
            VersionIdentityKind::Tier0QualificationArtifact => {
                entry!("Tier0QualificationArtifactVersion", "BP-RECEIPTS-1")
            }
        }
    }
}

/// The disposition of a corpus-discovered identity-shaped term that the
/// catalogs do NOT admit. This is the PERMANENT corpus denominator's residue
/// half: every stable `...Id`/`...Hash`/`...Digest`/`...Commitment`/
/// `...Generation`/`...Version` term in the authoritative documents resolves
/// through exactly one of five paths — the four catalogs or this table —
/// and a new unclassified term reds the audit immediately. Without this,
/// the sweep would be an archaeological expedition whose map quietly
/// expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityTermDisposition {
    /// The term has a canonical typed or documentary owner outside these
    /// catalogs; the reference must stay live.
    OwnedElsewhere(ContractId),
    /// The term is NOT YET admitted: the cited standing decision names the
    /// prerequisites a future profile must meet, and admission happens only
    /// by amending that decision AND moving the term into exactly one
    /// catalog — changing the decision alone never silently admits it.
    /// "Not currently admitted" and "forbidden by decision" are DIFFERENT
    /// laws, and a forbidden disposition has no spelling here because no
    /// term is currently decision-forbidden: mislabeling a pending passport
    /// application as a dead passport is a compile error.
    NotYetAdmittedBy(DecisionId),
}

/// One non-admitted term and its disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonCatalogedIdentityTerm {
    pub term: &'static str,
    pub disposition: IdentityTermDisposition,
}

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

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
    /// The sprouted realization-candidate lineage object (5.5F3). Owned by
    /// BP-SPROUTING-1; its content is bound independently through a
    /// `BindingKind` commitment and its parents are explicit parent
    /// `CandidateId`s on the manifest — this axis names the OBJECT only, not a
    /// representation or a manifest version.
    Candidate,
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
    /// The candidate lineage MANIFEST format (F5): the persisted,
    /// line-oriented serialization of a `spec/campaign/` `CandidateRecord`,
    /// whose first line names its own version
    /// (`BATPAK-CANDIDATE-MANIFEST/1`). Distinct from `IdentityKind::Candidate`
    /// (the lineage OBJECT this format serializes), from `ContentDigest` (the
    /// content commitment a manifest carries), and from
    /// `Tier0QualificationArtifact` (the Tier 0 evidence format). Minted now
    /// because the serialized schema lands with the F5 campaign: docs/39 §3's
    /// "independently versioned only when the actual serialized schema lands"
    /// condition is met, and no unversioned "temporary" format ever exists.
    CandidateManifest,
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
        IdentityKind::Candidate,
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
            IdentityKind::Candidate => entry!("CandidateId", "BP-SPROUTING-1"),
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
        VersionIdentityKind::CandidateManifest,
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
            VersionIdentityKind::CandidateManifest => {
                entry!("CandidateManifestVersion", "BP-SPROUTING-1")
            }
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

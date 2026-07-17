//! Frozen clean-room repository architecture facts.

use crate::gates::GateId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageClass {
    Production,
    BinaryAdapter,
    DevOnly,
    Example,
}

/// The five SyncBat planes (docs/08 ownership firewall).
///
/// `spec/architecture.rs` owns package topology, so it owns the plane IDENTITIES
/// and their package membership. It does NOT own what each plane may do: the
/// legal-crossing and forbidden-transfer law lives in `spec/syncbat_firewall.rs`,
/// which is the one authority for it. Splitting them this way keeps the plane
/// inventory in one place while leaving the firewall a narrow, single-purpose
/// authority rather than another topology.
///
/// These planes are specific to the SyncBat organism. They are not a universal
/// plane framework and no other package has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncBatPlane {
    /// Logical legality.
    Runtime,
    /// Program semantics.
    PakVm,
    /// Attempt admission and physical evidence.
    Bvisor,
    /// Composition and instance identity.
    World,
    /// Explicit host requests and responses.
    Port,
}

impl SyncBatPlane {
    /// The package this plane lives in. Every plane is inside `syncbat`: sharing
    /// a crate is exactly why the firewall has to be typed rather than
    /// structural, because no module boundary separates them (docs/08: "Sharing a
    /// crate does not permit one plane to perform another's transition").
    /// Typed since 5.5E3b: the plane cites the identity, not a raw name.
    pub const fn package(self) -> PackageId {
        match self {
            SyncBatPlane::Runtime
            | SyncBatPlane::PakVm
            | SyncBatPlane::Bvisor
            | SyncBatPlane::World
            | SyncBatPlane::Port => PackageId::SyncBat,
        }
    }

    /// The authored docs/08 ownership sentence this plane's identity comes from.
    pub const fn authored_ownership(self) -> &'static str {
        match self {
            SyncBatPlane::Runtime => "runtime owns logical legality",
            SyncBatPlane::PakVm => "pakvm owns program semantics",
            SyncBatPlane::Bvisor => "bvisor owns attempt admission and physical evidence",
            SyncBatPlane::World => "world owns composition and instance identity",
            SyncBatPlane::Port => "port owns explicit host requests and responses",
        }
    }
}

/// The five planes, in docs/08 order.
pub const SYNCBAT_PLANES: &[SyncBatPlane] = &[
    SyncBatPlane::Runtime,
    SyncBatPlane::PakVm,
    SyncBatPlane::Bvisor,
    SyncBatPlane::World,
    SyncBatPlane::Port,
];

/// The closed identity of an admitted workspace package (5.5E3b). The
/// VARIANT is the identity; the Cargo package name, the workspace path, and
/// the display label are deterministic projections of it — never additional
/// identities, and never flattened into one another: BatPakCli's Cargo
/// package is `batpak-cli`, its binary target is `batpak`, and its path is
/// `apps/batpak-cli`, three related facts about one identity. A package that
/// is not a variant here has no spelling in any typed relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageId {
    MacBatCompiler,
    MacBat,
    BatPak,
    SyncBat,
    BatQl,
    NetBat,
    TestPak,
    BatPakCli,
    BatPakExamples,
}

impl PackageId {
    /// Every admitted package, in canonical workspace order. The catalog law
    /// requires PACKAGES to declare exactly these, in this order; counts are
    /// DERIVED from this inventory, never hardcoded.
    pub const ALL: &'static [PackageId] = &[
        PackageId::MacBatCompiler,
        PackageId::MacBat,
        PackageId::BatPak,
        PackageId::SyncBat,
        PackageId::BatQl,
        PackageId::NetBat,
        PackageId::TestPak,
        PackageId::BatPakCli,
        PackageId::BatPakExamples,
    ];

    /// The Cargo package name this identity projects.
    pub const fn cargo_name(self) -> &'static str {
        match self {
            PackageId::MacBatCompiler => "macbat-compiler",
            PackageId::MacBat => "macbat",
            PackageId::BatPak => "batpak",
            PackageId::SyncBat => "syncbat",
            PackageId::BatQl => "batql",
            PackageId::NetBat => "netbat",
            PackageId::TestPak => "testpak",
            PackageId::BatPakCli => "batpak-cli",
            PackageId::BatPakExamples => "batpak-examples",
        }
    }

    /// The workspace-relative path this identity projects.
    pub const fn workspace_path(self) -> &'static str {
        match self {
            PackageId::MacBatCompiler => "crates/macbat/compiler",
            PackageId::MacBat => "crates/macbat/macros",
            PackageId::BatPak => "crates/batpak",
            PackageId::SyncBat => "crates/syncbat",
            PackageId::BatQl => "crates/batql",
            PackageId::NetBat => "crates/netbat",
            PackageId::TestPak => "crates/testpak",
            PackageId::BatPakCli => "apps/batpak-cli",
            PackageId::BatPakExamples => "examples",
        }
    }

    /// The human display label this identity projects.
    pub const fn display_name(self) -> &'static str {
        match self {
            PackageId::MacBatCompiler => "MacBat compiler",
            PackageId::MacBat => "MacBat",
            PackageId::BatPak => "BatPak",
            PackageId::SyncBat => "SyncBat",
            PackageId::BatQl => "BatQL",
            PackageId::NetBat => "NetBat",
            PackageId::TestPak => "TestPak",
            PackageId::BatPakCli => "BatPak CLI",
            PackageId::BatPakExamples => "BatPak examples",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageSpec {
    /// The typed identity. Cargo name and workspace path are projections of
    /// it and are not repeated here.
    pub id: PackageId,
    pub role: &'static str,
    pub class: PackageClass,
    pub layer: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeClass {
    Required,
    OptionalProfile,
    DevOnly,
}

/// The semantic assumptions a qualification holds under (5.5E3a1). These
/// are capability profiles, NOT Rust target triples: an environment answers
/// WHAT a build may assume, a triple answers WHERE an artifact compiled and
/// ran. A triple has no spelling here — the collapse the old string field
/// merely policed is now unrepresentable. The physical axis arrives as its
/// own type with the receipt-binding work; the two must never merge into a
/// generalized "build coordinate".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualificationEnvironment {
    /// Core semantic profiles: no_std with the alloc crate.
    NoStdAlloc,
    /// Full standard library on a native host.
    NativeStd,
    /// Browser/wasm host mechanisms behind explicit adapters.
    WasmHost,
}

impl QualificationEnvironment {
    /// Every environment, in declaration order.
    pub const ALL: &'static [QualificationEnvironment] = &[
        QualificationEnvironment::NoStdAlloc,
        QualificationEnvironment::NativeStd,
        QualificationEnvironment::WasmHost,
    ];

    /// The canonical documentary spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            QualificationEnvironment::NoStdAlloc => "no_std + alloc",
            QualificationEnvironment::NativeStd => "std",
            QualificationEnvironment::WasmHost => "wasm32 host",
        }
    }
}

/// One environment-specific qualification requirement.
///
/// `environment` names the semantic assumptions qualified under; `gates`
/// names WHEN that qualification is scheduled. They are different dimensions
/// and neither may stand in for the other: an environment is not a GateId.
/// The schedule is stated here because it is row-specific and cannot be
/// derived from an environment or from neighbouring SEED vocabulary.
///
/// The requirement stays Permanent after a gate passes; the receipt is
/// gate/run-specific evidence, not the lifetime of the law.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationProfile {
    pub package: PackageId,
    pub profile: &'static str,
    pub environment: QualificationEnvironment,
    pub gates: &'static [GateId],
    pub requirement: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSpec {
    pub importer: PackageId,
    pub importee: PackageId,
    pub class: EdgeClass,
    pub profile: &'static str,
}

pub const REPOSITORY_NAME: &str = "BatPak";
pub const SPEC_VERSION: &str = "1.0.0";

/// Workspace implementation train. Publishable production packages ship
/// lockstep to `1.0.0`; nonpublishable packages inherit this version without
/// becoming release artifacts. Bootstrap rejects any version below this train
/// so a template cannot regress the family to a pre-1.0 line.
pub const WORKSPACE_VERSION: &str = "1.0.0-alpha.1";

pub const PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        id: PackageId::MacBatCompiler,
        role: "pure Rust contract compiler",
        class: PackageClass::Production,
        layer: 0,
    },
    PackageSpec {
        id: PackageId::MacBat,
        role: "proc-macro front door",
        class: PackageClass::Production,
        layer: 1,
    },
    PackageSpec {
        id: PackageId::BatPak,
        role: "semantic and durable core, including .fbat",
        class: PackageClass::Production,
        layer: 2,
    },
    PackageSpec {
        id: PackageId::SyncBat,
        role: "runtime crate containing runtime, PakVM, Bvisor, world, and port planes",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        id: PackageId::BatQl,
        role: "BatQL parser, type checker, planner, partial evaluator, and ProgramImage compiler",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        id: PackageId::NetBat,
        role: "bounded typed transport over declared SyncBat world entrypoints",
        class: PackageClass::Production,
        layer: 4,
    },
    PackageSpec {
        id: PackageId::TestPak,
        role: "repository proof, forge, gauntlet, benchmark, and mutation battery",
        class: PackageClass::DevOnly,
        layer: 6,
    },
    PackageSpec {
        id: PackageId::BatPakCli,
        role: "thin product command adapter; owns no semantic law",
        class: PackageClass::BinaryAdapter,
        layer: 5,
    },
    PackageSpec {
        id: PackageId::BatPakExamples,
        role: "public-surface witness; runnable demos over production APIs only; owns no semantic law and depends on no dev tooling",
        class: PackageClass::Example,
        layer: 5,
    },
];

pub const QUALIFICATION_PROFILES: &[QualificationProfile] = &[
    QualificationProfile {
        package: PackageId::BatPak,
        profile: "semantic",
        environment: QualificationEnvironment::NoStdAlloc,
        gates: &[GateId::G0, GateId::G5],
        requirement: "contracts, schemas, codecs, image values, deterministic parsing, and storage-port law compile without std",
    },
    QualificationProfile {
        package: PackageId::SyncBat,
        profile: "semantic",
        environment: QualificationEnvironment::NoStdAlloc,
        gates: &[GateId::G0, GateId::G5],
        requirement: "runtime transition core, PakVM validation/interpreter, Bvisor admission state, world values, and port protocols compile without std host adapters",
    },
    QualificationProfile {
        package: PackageId::BatPak,
        profile: "native",
        environment: QualificationEnvironment::NativeStd,
        gates: &[GateId::G0, GateId::G5],
        requirement: "native filesystem, mapping, clock, entropy, and threaded storage adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: PackageId::SyncBat,
        profile: "native",
        environment: QualificationEnvironment::NativeStd,
        gates: &[GateId::G0, GateId::G5],
        requirement: "threaded driver and native host-port adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: PackageId::SyncBat,
        profile: "browser",
        environment: QualificationEnvironment::WasmHost,
        gates: &[GateId::G0, GateId::G5],
        requirement: "browser adapters preserve semantic result, receipt, bounds, and recovery contracts without OS-backend normalization",
    },
    QualificationProfile {
        package: PackageId::BatPak,
        profile: "browser-storage",
        environment: QualificationEnvironment::WasmHost,
        gates: &[GateId::G2, GateId::G5, GateId::G7],
        requirement: "the browser persistence adapter proves its own atomicity, ordering, durability, quota, crash/reload, authority-generation, and bounded-size behavior without borrowing native filesystem claims",
    },
];

// --- Authenticated history (DEC-071) ----------------------------------------
// A DIFFERENT concept from the build-target `QualificationProfile` above, which
// says which package compiles for which target. These facts say what a store's
// history verification actually proves. The two families deliberately share no
// type, field name, parser, projection, or audit rule; nothing here is named a
// bare `Profile`, `Policy`, or `Disposition`.
//
// This is a contract, not an implementation: bootstrap performs no signature,
// accumulator, witness, freshness, or cryptographic verification of any kind.

/// What a caller explicitly selects. A stronger claim requires a stronger
/// profile; an invalid pairing is refused, never normalized into a neighbour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticatedHistoryProfile {
    /// Local coherence only. No authorship and no freshness claim.
    InternalConsistency,
    /// Authenticated authorship and integrity within the signed history.
    SignedHistory,
    /// Signed history plus an independent monotonic witness.
    ExternallyAnchoredHistory,
}

/// Whether an independent monotonic witness participates. Typed, and
/// constrained by profile: `Required` exists only with
/// `ExternallyAnchoredHistory`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessPolicy {
    None,
    Optional,
    Required,
}

/// The outcome of witness evaluation. These stay distinct on purpose: collapsing
/// them into `Valid`/`Invalid` is what lets a restored older history read as
/// healthy. An absent optional witness and a supplied broken one are different
/// facts and must never render identically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessDisposition {
    /// The policy does not use witnesses (`WitnessPolicy::None`).
    NotApplicable,
    /// The policy permits or requires a witness and none was supplied.
    NotProvided,
    /// Supplied and verified for this lineage, generation, and accumulator.
    Verified,
    /// Supplied and authentic, but older than the observed history.
    Stale,
    /// Supplied and contradicts the observed history.
    Conflicting,
    /// Supplied but could not be evaluated (unreachable or unparseable).
    Unverifiable,
    /// Supplied and failed cryptographic verification.
    CryptographicallyInvalid,
    /// Supplied but bound to a different store lineage.
    LineageMismatch,
    /// Supplied but bound to a different generation.
    GenerationMismatch,
    /// Supplied but bound to a different history accumulator.
    AccumulatorMismatch,
}

/// Whether the selected generation is internally coherent: local authoritative
/// commitments and authority-generation relationships verify, and derived
/// material is not treated as authority. Every admitted success bundle carries
/// this; there is no success without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityClaim {
    InternalConsistencyVerified,
}

/// Whether an authenticated signer produced this history. Independent of
/// whether the history is the newest one: a restored older generation can be
/// perfectly authentic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticityClaim {
    NotClaimed,
    SignedHistoryVerified,
}

/// Whether this generation is the newest history ever acknowledged. Only an
/// independent monotonic witness carries this claim.
///
/// `WitnessedGenerationVerified` is scoped to the exact store lineage,
/// generation, history or accumulator commitment, witness identity, witness
/// monotonicity guarantee, and trust assumptions that verified. It is never a
/// universal newest-history proof, and must not be renamed to imply one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshnessClaim {
    NotClaimed,
    WitnessedGenerationVerified,
}

/// Whether restoring an older valid generation is detectable, and within what
/// scope. Never universal rollback prevention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackResistanceClaim {
    /// No freshness evidence exists. Stated explicitly rather than omitted.
    Unavailable,
    /// Detectable only within the verified witness's own scope and assumptions.
    ScopedToVerifiedWitness,
}

/// Everything a SUCCESSFUL authenticated-history verification claims, stated on
/// four independent axes.
///
/// This replaces a single mutually exclusive posture enum, which could not say
/// two true things at once: an InternalConsistency success must assert both
/// `InternalConsistencyVerified` and `RollbackResistanceUnavailable`, and one
/// variant cannot. A consumer reads every axis directly and never reconstructs
/// an omitted claim from the profile name, the witness policy, the witness
/// disposition, or another axis.
///
/// This is a SUCCESS bundle only. It is not an arbitrary verification state, an
/// error category, a refusal outcome, or an incomplete attempt. A refusal is
/// never forced into one; see `REFUSAL_PARTIAL_CLAIM_LAW`.
///
/// The axes are independent, not an ordered ladder. There is no
/// `SecurityPosture`, `VerificationLevel`, `AssuranceLevel`, or `SecurityLevel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthenticatedHistoryClaims {
    pub integrity: IntegrityClaim,
    pub authenticity: AuthenticityClaim,
    pub freshness: FreshnessClaim,
    pub rollback_resistance: RollbackResistanceClaim,
}

/// Coherent, local, unsigned. Proves the generation hangs together and nothing
/// more: not who wrote it, and not that it is current.
pub const INTERNAL_CONSISTENCY_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::NotClaimed,
    freshness: FreshnessClaim::NotClaimed,
    rollback_resistance: RollbackResistanceClaim::Unavailable,
};

/// Authentic authorship, no external anchor. A restored older validly signed
/// history satisfies exactly this bundle, which is why freshness stays
/// `NotClaimed`.
pub const SIGNED_UNANCHORED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::SignedHistoryVerified,
    freshness: FreshnessClaim::NotClaimed,
    rollback_resistance: RollbackResistanceClaim::Unavailable,
};

/// Verified against an independent monotonic witness, scoped to exactly that
/// witness's lineage, generation, accumulator, guarantee, and trust
/// assumptions.
pub const WITNESSED_SUCCESS: AuthenticatedHistoryClaims = AuthenticatedHistoryClaims {
    integrity: IntegrityClaim::InternalConsistencyVerified,
    authenticity: AuthenticityClaim::SignedHistoryVerified,
    freshness: FreshnessClaim::WitnessedGenerationVerified,
    rollback_resistance: RollbackResistanceClaim::ScopedToVerifiedWitness,
};

// AuthenticatedHistoryProfileSpec { profile, permitted_witness_policies,
// requires_local_commitment_verification, requires_signed_history_verification,
// requires_independent_witness_verification, implementation_gates,
// release_qualification_gates, unanchored_success_claims,
// verified_witness_success_claims } and its three-row
// AUTHENTICATED_HISTORY_PROFILES table stood here from 5.5C2c until the 5.5E2
// bake. Every field was a pure function of the profile variant — the table
// restated per row what the enum determines, and three checkers (seedcheck,
// audit, selftest) then policed the copies back into agreement: duplicate-row
// checks, row-count checks, an always-true bool, Required-outside-EAH scans.
// A table whose every cell is derivable is not a fact family; it is a ladder
// of invitations to drift. Deleted rather than kept: the authorship moved
// into the const fns below, where an invalid pairing has no row to live in.

impl AuthenticatedHistoryProfile {
    /// The three profiles, in ascending claim order (for iteration, not as a
    /// ladder type: the CLAIMS stay four independent axes).
    pub const ALL: &'static [AuthenticatedHistoryProfile] = &[
        AuthenticatedHistoryProfile::InternalConsistency,
        AuthenticatedHistoryProfile::SignedHistory,
        AuthenticatedHistoryProfile::ExternallyAnchoredHistory,
    ];

    /// The frozen valid pairings. Anything absent here is refused, never
    /// normalized into a neighbour: `SignedHistory + Required` is not
    /// silently upgraded to `ExternallyAnchoredHistory` — the caller selects
    /// the stronger profile. `Required` exists only here, in the
    /// `ExternallyAnchoredHistory` arm.
    pub const fn permitted_witness_policies(self) -> &'static [WitnessPolicy] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => &[WitnessPolicy::None],
            AuthenticatedHistoryProfile::SignedHistory => {
                &[WitnessPolicy::None, WitnessPolicy::Optional]
            }
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[WitnessPolicy::Required],
        }
    }

    /// Local authoritative commitments and authority generations verify under
    /// EVERY profile. This is a law of the family, not a per-profile choice:
    /// there is no profile that skips local coherence.
    pub const fn requires_local_commitment_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// Signed seals and a signed whole-history commitment verify.
    pub const fn requires_signed_history_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => false,
            AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// An independent monotonic witness verifies.
    pub const fn requires_independent_witness_verification(self) -> bool {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory => false,
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => true,
        }
    }

    /// Where the profile's mechanism is implemented. One gate for the family:
    /// authenticated history is storage-core work.
    pub const fn implementation_gates(self) -> &'static [GateId] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[GateId::G2],
        }
    }

    /// Where the profile's release qualification is scheduled.
    pub const fn release_qualification_gates(self) -> &'static [GateId] {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency
            | AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => &[GateId::G9],
        }
    }

    /// The success bundle when no verified external anchor is present.
    ///
    /// `None` means NO SUCCESSFUL UNANCHORED RESULT IS ADMITTED — it does not
    /// mean unknown, not configured, posture unavailable, or fallback
    /// success. An absent or invalid required witness refuses; it never falls
    /// back to a weaker success.
    pub const fn unanchored_success_claims(self) -> Option<AuthenticatedHistoryClaims> {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => {
                Some(INTERNAL_CONSISTENCY_SUCCESS)
            }
            AuthenticatedHistoryProfile::SignedHistory => Some(SIGNED_UNANCHORED_SUCCESS),
            AuthenticatedHistoryProfile::ExternallyAnchoredHistory => None,
        }
    }

    /// The success bundle when an independent witness verifies. `None` means
    /// the profile admits no witness at all.
    pub const fn verified_witness_success_claims(self) -> Option<AuthenticatedHistoryClaims> {
        match self {
            AuthenticatedHistoryProfile::InternalConsistency => None,
            AuthenticatedHistoryProfile::SignedHistory
            | AuthenticatedHistoryProfile::ExternallyAnchoredHistory => Some(WITNESSED_SUCCESS),
        }
    }
}

/// A refusal is not a weaker success.
///
/// The normative result model distinguishes:
///
/// ```text
/// Success {
///     claims: AuthenticatedHistoryClaims,   // complete, all four axes
///     witness_disposition, profile, policy, identity, proof fields
/// }
///
/// Refusal {
///     final_claims: None,                   // no successful claim bundle
///     partial_verified_claims: Option<AuthenticatedHistoryClaims>,
///     witness_disposition or failure reason, profile, policy,
///     identity, proof fields
/// }
/// ```
///
/// `partial_verified_claims` preserves ONLY sub-results that independently
/// succeeded, for diagnosis. It may carry local integrity or signed-history
/// evidence. It may never carry `FreshnessClaim::WitnessedGenerationVerified`
/// or `RollbackResistanceClaim::ScopedToVerifiedWitness` when witness
/// verification failed, and its presence never converts a refusal into success.
pub const REFUSAL_PARTIAL_CLAIM_LAW: &str =
    "a refusal carries no final claim bundle; partial_verified_claims holds only \
     independently verified local evidence and never freshness or scoped rollback \
     resistance after witness failure";

/// Every witness disposition that must fail closed under
/// `WitnessPolicy::Required`. `NotProvided` is included: a required witness that
/// was never supplied is a refusal, not a silent success.
pub const REQUIRED_WITNESS_FAILURE_SET: &[WitnessDisposition] = &[
    WitnessDisposition::NotProvided,
    WitnessDisposition::Stale,
    WitnessDisposition::Conflicting,
    WitnessDisposition::Unverifiable,
    WitnessDisposition::CryptographicallyInvalid,
    WitnessDisposition::LineageMismatch,
    WitnessDisposition::GenerationMismatch,
    WitnessDisposition::AccumulatorMismatch,
];

/// A supplied optional witness that failed. Optional to supply; once supplied,
/// mandatory to validate. None of these may degrade into `NotProvided` or emit
/// a success receipt that discards the failure.
pub const OPTIONAL_WITNESS_REFUSAL_SET: &[WitnessDisposition] = &[
    WitnessDisposition::Stale,
    WitnessDisposition::Conflicting,
    WitnessDisposition::Unverifiable,
    WitnessDisposition::CryptographicallyInvalid,
    WitnessDisposition::LineageMismatch,
    WitnessDisposition::GenerationMismatch,
    WitnessDisposition::AccumulatorMismatch,
];

// --- TestPak proof policy and mutation (DEC-015, DEC-074) -------------------
// Typed facts only. Bootstrap compiles no mutant, activates no slot, runs no
// nextest, invokes no rustc, kills nothing, proves no equivalence, promotes
// nothing, and classifies no real diff semantically. It proves the contract.

/// How a proof-policy change moves the boundary. An unclassified change is
/// refused (DEC-074).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofPolicyChangeClass {
    /// Adds or raises proof obligations, increases hostile coverage, reduces
    /// unjustified waiver scope, or improves freshness or denominator
    /// visibility, without silently removing a proof unit or terminal
    /// disposition.
    Strengthening,
    /// Changes representation while preserving denominator, selection meaning,
    /// terminal dispositions, activation, freshness, blocking posture,
    /// promotion requirements, and documentary meaning. Requires parity
    /// evidence: "refactor" is not automatically Neutral.
    Neutral,
    /// Removes a proof unit, lowers a threshold, widens a waiver, makes a
    /// required proof optional, reduces hostile coverage or activation
    /// requirements, weakens freshness, turns a blocking failure into a
    /// warning, shrinks selected tests without equivalent qualification, lets a
    /// candidate promote on less evidence, drops units from the denominator, or
    /// retires an independent oracle. An apparently stronger policy that
    /// narrows the tested domain is also Weakening.
    Weakening,
}

/// The surfaces the anti-weakening gate owns. A change declaration names the
/// affected surfaces explicitly; the gate never infers one from a filename or
/// keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofPolicySurface {
    MutationThreshold,
    WaiverLogic,
    EquivalentMutantRule,
    HostileFixture,
    CandidatePromotion,
    ProofReceiptSchema,
    ProofFreshness,
    ReleaseBlocking,
    TestSelection,
    AssuranceRequirement,
    GateQualification,
}

/// Candidate material is disposable and derived. It is never durable proof
/// truth, and there is no canonical direct-write path from generation into
/// tracked source.
pub const CANDIDATE_OUTPUT_ROOT: &str = "target/muterprater/candidates/";

/// What a release receipt binds (DEC-058, 5.5E1 ruling). One typed inventory:
/// docs/36 projects the full list, docs/24 names the gauntlet inputs, and
/// DEC-058 references this owner instead of restating a third copy — three
/// hand-authored seal lists disagreed until this enum existed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseSealField {
    SourceTree,
    Toolchain,
    DependencyGraph,
    GeneratedFacts,
    CompatibilityCorpus,
    TestDispositions,
    MutationDispositions,
    FuzzDispositions,
    BenchmarkDispositions,
    CompilerAssumptionLedger,
    DependencyLedger,
    /// Mandatory even when empty: an empty set states "no kernels admitted",
    /// it never disappears from the schema.
    KernelQualificationSet,
    PackageContents,
    PublicApi,
    Sbom,
    LicenseEvidence,
    ProofFreshness,
}

/// Every seal field, in declaration order. Completeness is enforced by
/// seedcheck's exhaustive classification: a new field cannot be added without
/// appearing here, and none may appear twice.
pub const RELEASE_SEAL_FIELDS: &[ReleaseSealField] = &[
    ReleaseSealField::SourceTree,
    ReleaseSealField::Toolchain,
    ReleaseSealField::DependencyGraph,
    ReleaseSealField::GeneratedFacts,
    ReleaseSealField::CompatibilityCorpus,
    ReleaseSealField::TestDispositions,
    ReleaseSealField::MutationDispositions,
    ReleaseSealField::FuzzDispositions,
    ReleaseSealField::BenchmarkDispositions,
    ReleaseSealField::CompilerAssumptionLedger,
    ReleaseSealField::DependencyLedger,
    ReleaseSealField::KernelQualificationSet,
    ReleaseSealField::PackageContents,
    ReleaseSealField::PublicApi,
    ReleaseSealField::Sbom,
    ReleaseSealField::LicenseEvidence,
    ReleaseSealField::ProofFreshness,
];

/// Tracked surfaces a generated candidate may never write directly. Promotion
/// is a separate admitted action with its own receipt.
pub const CANDIDATE_FORBIDDEN_WRITE_ROOTS: &[&str] =
    &["src/", "tests/", "spec/", "docs/", "companion/"];

/// No oracle, no promotion. No named obligation, no promotion. No killed mutant
/// or equivalent hostile evidence, no promotion. No proof receipt, no trust.
pub const EDGES: &[EdgeSpec] = &[
    EdgeSpec { importer: PackageId::MacBat, importee: PackageId::MacBatCompiler, class: EdgeClass::Required, profile: "compile" },
    EdgeSpec { importer: PackageId::BatPak, importee: PackageId::MacBat, class: EdgeClass::Required, profile: "derive" },
    EdgeSpec { importer: PackageId::SyncBat, importee: PackageId::BatPak, class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: PackageId::BatQl, importee: PackageId::BatPak, class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: PackageId::NetBat, importee: PackageId::BatPak, class: EdgeClass::Required, profile: "transport-contract" },
    EdgeSpec { importer: PackageId::NetBat, importee: PackageId::SyncBat, class: EdgeClass::Required, profile: "runtime-entrypoints" },
    EdgeSpec { importer: PackageId::BatPakCli, importee: PackageId::BatPak, class: EdgeClass::Required, profile: "core" },
    EdgeSpec { importer: PackageId::BatPakCli, importee: PackageId::SyncBat, class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: PackageId::BatPakCli, importee: PackageId::BatQl, class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: PackageId::BatPakCli, importee: PackageId::NetBat, class: EdgeClass::OptionalProfile, profile: "serve" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::MacBatCompiler, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::MacBat, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::BatPak, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::SyncBat, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::BatQl, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::TestPak, importee: PackageId::NetBat, class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: PackageId::BatPakExamples, importee: PackageId::BatPak, class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: PackageId::BatPakExamples, importee: PackageId::SyncBat, class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: PackageId::BatPakExamples, importee: PackageId::BatQl, class: EdgeClass::Required, profile: "example" },
];

pub const REQUIRED_DOCS: &[&str] = &[
    "README.md",
    "SPEC.sha256",
    "AGENTS.md",
    "FINAL_RECONCILIATION.md",
    "DELIVERY_NOTES.md",
    "docs/00_CONSTITUTION.md",
    "docs/01_FACTORY.md",
    "docs/02_SYSTEM_MODEL.md",
    "docs/03_REPOSITORY_AND_PACKAGES.md",
    "docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md",
    "docs/05_STORAGE_FBAT_AND_TILES.md",
    "docs/06_MACBAT.md",
    "docs/07_PAKVM_ISA.md",
    "docs/08_SYNCBAT_RUNTIME.md",
    "docs/09_BVISOR.md",
    "docs/10_WORLD_IMAGES_AND_PORTS.md",
    "docs/11_NETBAT.md",
    "docs/12_TESTPAK.md",
    "docs/13_BATQL_CONTRACT.md",
    "docs/14_RECEIPTS_AND_EXPLANATION.md",
    "docs/15_SCHEMA_CODEC_AND_MIGRATION.md",
    "docs/16_IDENTITY_TIME_AND_NAVIGATION.md",
    "docs/17_DELIVERY_AND_CONCURRENCY.md",
    "docs/18_DATA_ORIENTED_ECS.md",
    "docs/19_SECURITY_MODEL.md",
    "docs/20_DEPENDENCY_SOVEREIGNTY.md",
    "docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md",
    "docs/22_MIGRATION_AND_CUTOVER.md",
    "docs/23_BOOTSTRAP_AND_SELF_HOSTING.md",
    "docs/24_GAUNTLET.md",
    "docs/25_IMPLEMENTATION_GATES.md",
    "docs/26_COMMAND_PLANE.md",
    "docs/27_WORKFLOW.md",
    "docs/28_SELF_EXPLAINING_REPOSITORY.md",
    "docs/29_STATUS_AND_SUPERSESSION.md",
    "docs/30_DECISION_AND_REJECTION_LEDGER.md",
    "docs/31_FINAL_CONTRADICTION_AUDIT.md",
    "docs/32_IMPLEMENTATION_CONSTANTS.md",
    "docs/33_AGENT_FINISH_LINE_CHECKLIST.md",
    "docs/34_LEGACY_INVARIANT_COVERAGE.md",
    "docs/35_CRYPTO_AND_SECRET_AUTHORITY.md",
    "docs/36_PUBLIC_API_CI_AND_RELEASE.md",
    "docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md",
    "docs/GUARANTEE_GRAPH.generated.md",
    "companion/BATQL_LANGUAGE.md",
    "spec/architecture.rs",
    "spec/invariants.rs",
    "spec/dispositions.rs",
    "spec/legacy_obligations.rs",
    "spec/legacy_invariant_coverage.rs",
    "spec/operators.rs",
    "spec/guarantees.rs",
    "spec/gates.rs",
    "spec/pakvm_isa.rs",
    "spec/syncbat_firewall.rs",
    "spec/lib.rs",
    "spec/reconciliation.rs",
    "spec/proof.rs",
    "spec/toolchain.rs",
    "spec/commands.rs",
    "spec/compiler_assumptions.rs",
    "spec/contracts.rs",
    "spec/corpus.rs",
    "spec/identities.rs",
    "spec/mutation.rs",
    "spec/promotion.rs",
    "rust-toolchain.toml",
    "bootstrap/README.md",
    "bootstrap/seedcheck.rs",
    "bootstrap/materialize.rs",
    "bootstrap/audit.py",
    "bootstrap/freeze.py",
    "bootstrap/project.py",
    "spec/README.md",
    "history/README.md",
];

pub const FORBIDDEN_TARGET_PATHS: &[&str] = &[
    "crates/filebat",
    "crates/bat-vm",
    "crates/batpak-core",
    "crates/core",
    "crates/hostbat",
    "crates/bvisor",
    "crates/pakvm",
    "crates/vpak",
    "crates/testkit",
    "tools/xtask",
    "tools/integrity",
    "corpus",
    "fixtures",
];

pub const SYNCBAT_REQUIRED_PLANES: &[&str] = &[
    "src/runtime.rs",
    "src/runtime",
    "src/pakvm.rs",
    "src/pakvm",
    "src/bvisor.rs",
    "src/bvisor",
    "src/world.rs",
    "src/world",
    "src/port.rs",
    "src/port",
];

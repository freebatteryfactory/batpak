use crate::gates::GateId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageClass {
    Production,
    BinaryAdapter,
    DevOnly,
    Example,
}

impl PackageClass {
    pub const ALL: &'static [PackageClass] = &[
        PackageClass::Production,
        PackageClass::BinaryAdapter,
        PackageClass::DevOnly,
        PackageClass::Example,
    ];

    /// The documentary spelling (5.5E4b): the one spelling every generated
    /// inventory projection renders — no Python class-spelling map exists.
    pub const fn spelling(self) -> &'static str {
        match self {
            PackageClass::Production => "production",
            PackageClass::BinaryAdapter => "binary-adapter",
            PackageClass::DevOnly => "dev-only",
            PackageClass::Example => "example",
        }
    }
}

/// The five SyncBat planes (docs/08 ownership firewall).
///
/// `spec/architecture/types.rs` owns package topology, so it owns the plane IDENTITIES
/// and their package membership. It does NOT own what each plane may do: the
/// legal-crossing and forbidden-transfer law lives in `spec/syncbat_firewall/`,
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
    /// The five planes, in docs/08 order. This is the ONE plane inventory
    /// (5.5E5): the raw `SYNCBAT_PLANES` alias and the string-path
    /// `SYNCBAT_REQUIRED_PLANES` table retired with the isolated-materializer
    /// closure, and every consumer — projector, auditor, seedcheck, and the
    /// materializer's plan — derives plane files from this inventory and
    /// `module_name()`.
    pub const ALL: &'static [SyncBatPlane] = &[
        SyncBatPlane::Runtime,
        SyncBatPlane::PakVm,
        SyncBatPlane::Bvisor,
        SyncBatPlane::World,
        SyncBatPlane::Port,
    ];

    /// The Rust module name this plane's identity projects.
    pub const fn module_name(self) -> &'static str {
        match self {
            SyncBatPlane::Runtime => "runtime",
            SyncBatPlane::PakVm => "pakvm",
            SyncBatPlane::Bvisor => "bvisor",
            SyncBatPlane::World => "world",
            SyncBatPlane::Port => "port",
        }
    }

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

impl EdgeClass {
    pub const ALL: &'static [EdgeClass] = &[
        EdgeClass::Required,
        EdgeClass::OptionalProfile,
        EdgeClass::DevOnly,
    ];

    /// The documentary spelling (5.5E4b): the one spelling every generated
    /// edge projection renders — no Python class-spelling map exists.
    pub const fn spelling(self) -> &'static str {
        match self {
            EdgeClass::Required => "required",
            EdgeClass::OptionalProfile => "optional-profile",
            EdgeClass::DevOnly => "dev-only",
        }
    }
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

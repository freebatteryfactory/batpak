//! Frozen clean-room repository architecture facts.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageClass {
    Production,
    BinaryAdapter,
    DevOnly,
    Example,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageSpec {
    pub package: &'static str,
    pub path: &'static str,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualificationProfile {
    pub package: &'static str,
    pub profile: &'static str,
    pub target: &'static str,
    pub requirement: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSpec {
    pub importer: &'static str,
    pub importee: &'static str,
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
        package: "macbat-compiler",
        path: "crates/macbat/compiler",
        role: "pure Rust contract compiler",
        class: PackageClass::Production,
        layer: 0,
    },
    PackageSpec {
        package: "macbat",
        path: "crates/macbat/macros",
        role: "proc-macro front door",
        class: PackageClass::Production,
        layer: 1,
    },
    PackageSpec {
        package: "batpak",
        path: "crates/batpak",
        role: "semantic and durable core, including .fbat",
        class: PackageClass::Production,
        layer: 2,
    },
    PackageSpec {
        package: "syncbat",
        path: "crates/syncbat",
        role: "runtime crate containing runtime, PakVM, Bvisor, world, and port planes",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        package: "batql",
        path: "crates/batql",
        role: "BatQL parser, type checker, planner, partial evaluator, and ProgramImage compiler",
        class: PackageClass::Production,
        layer: 3,
    },
    PackageSpec {
        package: "netbat",
        path: "crates/netbat",
        role: "bounded typed transport over declared SyncBat world entrypoints",
        class: PackageClass::Production,
        layer: 4,
    },
    PackageSpec {
        package: "testpak",
        path: "crates/testpak",
        role: "repository proof, forge, gauntlet, benchmark, and mutation battery",
        class: PackageClass::DevOnly,
        layer: 6,
    },
    PackageSpec {
        package: "batpak-cli",
        path: "apps/batpak-cli",
        role: "thin product command adapter; owns no semantic law",
        class: PackageClass::BinaryAdapter,
        layer: 5,
    },
    PackageSpec {
        package: "batpak-examples",
        path: "examples",
        role: "public-surface witness; runnable demos over production APIs only; owns no semantic law and depends on no dev tooling",
        class: PackageClass::Example,
        layer: 5,
    },
];

pub const QUALIFICATION_PROFILES: &[QualificationProfile] = &[
    QualificationProfile {
        package: "batpak",
        profile: "semantic",
        target: "no_std + alloc",
        requirement: "contracts, schemas, codecs, image values, deterministic parsing, and storage-port law compile without std",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "semantic",
        target: "no_std + alloc",
        requirement: "runtime transition core, PakVM validation/interpreter, Bvisor admission state, world values, and port protocols compile without std host adapters",
    },
    QualificationProfile {
        package: "batpak",
        profile: "native",
        target: "std",
        requirement: "native filesystem, mapping, clock, entropy, and threaded storage adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "native",
        target: "std",
        requirement: "threaded driver and native host-port adapters are explicit std mechanisms",
    },
    QualificationProfile {
        package: "syncbat",
        profile: "browser",
        target: "wasm32 host",
        requirement: "browser adapters preserve semantic result, receipt, bounds, and recovery contracts without OS-backend normalization",
    },
];

pub const EDGES: &[EdgeSpec] = &[
    EdgeSpec { importer: "macbat", importee: "macbat-compiler", class: EdgeClass::Required, profile: "compile" },
    EdgeSpec { importer: "batpak", importee: "macbat", class: EdgeClass::Required, profile: "derive" },
    EdgeSpec { importer: "syncbat", importee: "batpak", class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: "batql", importee: "batpak", class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: "netbat", importee: "batpak", class: EdgeClass::Required, profile: "transport-contract" },
    EdgeSpec { importer: "netbat", importee: "syncbat", class: EdgeClass::Required, profile: "runtime-entrypoints" },
    EdgeSpec { importer: "batpak-cli", importee: "batpak", class: EdgeClass::Required, profile: "core" },
    EdgeSpec { importer: "batpak-cli", importee: "syncbat", class: EdgeClass::Required, profile: "runtime" },
    EdgeSpec { importer: "batpak-cli", importee: "batql", class: EdgeClass::Required, profile: "compiler" },
    EdgeSpec { importer: "batpak-cli", importee: "netbat", class: EdgeClass::OptionalProfile, profile: "serve" },
    EdgeSpec { importer: "testpak", importee: "macbat-compiler", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "macbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "batpak", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "syncbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "batql", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "testpak", importee: "netbat", class: EdgeClass::DevOnly, profile: "proof" },
    EdgeSpec { importer: "batpak-examples", importee: "batpak", class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: "batpak-examples", importee: "syncbat", class: EdgeClass::Required, profile: "example" },
    EdgeSpec { importer: "batpak-examples", importee: "batql", class: EdgeClass::Required, profile: "example" },
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
    "companion/BATQL_LANGUAGE.md",
    "spec/architecture.rs",
    "spec/invariants.rs",
    "spec/dispositions.rs",
    "spec/legacy_obligations.rs",
    "spec/legacy_invariant_coverage.rs",
    "spec/operators.rs",
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

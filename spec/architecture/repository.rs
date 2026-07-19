use crate::gates::GateId;
use super::*;

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

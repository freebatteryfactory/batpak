//! Gate-0 materializer output shape (5.5E5). Semantic owner: BP-BOOTSTRAP-1.
//!
//! This module owns exactly one thing: the SHAPE of the isolated Gate-0
//! workspace candidate the bootstrap materializer publishes. It does not own
//! package semantics, package topology, toolchain semantics, or the behavior
//! of any generated source file. Package membership, canonical order, Cargo
//! names, and workspace paths stay in `spec/architecture.rs`; toolchain
//! channel, version floor, edition, resolver, and the tracked
//! `rust-toolchain.toml` bytes stay in `spec/toolchain.rs`. Completeness of
//! the expanded plan is DERIVED from these closed inventories and the
//! Cartesian expansion over `PackageId::ALL` and `SyncBatPlane::ALL` — never
//! from a hardcoded file table or an expected count.
//!
//! There is deliberately no `Gate0ArtifactId`, no `GeneratedFileId`, and no
//! `WorkspaceFileId`: the artifact FAMILIES below plus the typed identities
//! they expand over are the whole vocabulary. `MaterializationId` remains the
//! BatPak domain identity for a physical data materialization and cannot
//! substitute for a bootstrap-output artifact.

use crate::architecture::PackageId;

/// A workspace-root file the Gate-0 candidate carries exactly once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate0RootArtifact {
    /// The Cargo workspace manifest.
    WorkspaceManifest,
    /// The pinned toolchain selection, byte-identical to the tracked root file.
    RustToolchain,
    /// The discoverability-only command file.
    Justfile,
}

impl Gate0RootArtifact {
    /// Every root artifact, in canonical output order.
    pub const ALL: &'static [Gate0RootArtifact] = &[
        Gate0RootArtifact::WorkspaceManifest,
        Gate0RootArtifact::RustToolchain,
        Gate0RootArtifact::Justfile,
    ];

    /// The output-root-relative path this artifact is published at.
    pub const fn relative_path(self) -> &'static str {
        match self {
            Gate0RootArtifact::WorkspaceManifest => "Cargo.toml",
            Gate0RootArtifact::RustToolchain => "rust-toolchain.toml",
            Gate0RootArtifact::Justfile => "justfile",
        }
    }
}

/// A file family every admitted package expands to exactly once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate0PackageArtifact {
    /// `<workspace_path>/Cargo.toml`.
    Manifest,
    /// `<workspace_path>/README.md` — implementation output, not an
    /// architecture-corpus document; its eventual document-surface ruling
    /// belongs to Phase 6.
    Readme,
    /// The package's single compilation entry file, selected by target kind.
    SourceDoor,
}

impl Gate0PackageArtifact {
    /// Every package artifact family, in canonical per-package output order.
    pub const ALL: &'static [Gate0PackageArtifact] = &[
        Gate0PackageArtifact::Manifest,
        Gate0PackageArtifact::Readme,
        Gate0PackageArtifact::SourceDoor,
    ];
}

/// How a package's Gate-0 skeleton compiles. The kind selects the source
/// door and the Cargo target stanza; it never selects semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate0PackageTargetKind {
    /// An ordinary library crate.
    Library,
    /// A proc-macro library crate; its front door exports no ordinary
    /// public constant.
    ProcMacroLibrary,
    /// A binary adapter crate with one named binary target.
    Binary,
    /// An example crate whose single runnable lives under `src/bin/`.
    ExampleBinary,
}

impl Gate0PackageTargetKind {
    /// Every target kind.
    pub const ALL: &'static [Gate0PackageTargetKind] = &[
        Gate0PackageTargetKind::Library,
        Gate0PackageTargetKind::ProcMacroLibrary,
        Gate0PackageTargetKind::Binary,
        Gate0PackageTargetKind::ExampleBinary,
    ];
}

/// A file family every SyncBat plane expands to exactly once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate0PlaneArtifact {
    /// `crates/syncbat/src/<module>.rs` — the plane's public module file.
    Module,
    /// `crates/syncbat/src/<module>/README.md` — the plane directory's
    /// ownership note.
    DirectoryReadme,
}

impl Gate0PlaneArtifact {
    /// Every plane artifact family, in canonical per-plane output order.
    pub const ALL: &'static [Gate0PlaneArtifact] = &[
        Gate0PlaneArtifact::Module,
        Gate0PlaneArtifact::DirectoryReadme,
    ];
}

/// The only two successful publication outcomes. Anything else — a partial
/// tree, a mismatching file, an extra file, a symlink — is a refusal, never
/// an in-place repair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate0OutputDisposition {
    /// The output path did not exist and an exact new candidate tree was
    /// published atomically.
    Created,
    /// The output path already contained exactly the planned tree; nothing
    /// was written.
    Unchanged,
}

impl Gate0OutputDisposition {
    /// Every lawful publication outcome.
    pub const ALL: &'static [Gate0OutputDisposition] = &[
        Gate0OutputDisposition::Created,
        Gate0OutputDisposition::Unchanged,
    ];
}

/// The Gate-0 target kind of every admitted package. Every arm is explicit:
/// a future package that reaches `PackageId` without an arm here fails to
/// compile instead of being silently classified by a wildcard or a
/// `PackageClass` fallback.
pub const fn target_kind(package: PackageId) -> Gate0PackageTargetKind {
    match package {
        PackageId::MacBatCompiler => Gate0PackageTargetKind::Library,
        PackageId::MacBat => Gate0PackageTargetKind::ProcMacroLibrary,
        PackageId::BatPak => Gate0PackageTargetKind::Library,
        PackageId::SyncBat => Gate0PackageTargetKind::Library,
        PackageId::BatQl => Gate0PackageTargetKind::Library,
        PackageId::NetBat => Gate0PackageTargetKind::Library,
        PackageId::TestPak => Gate0PackageTargetKind::Library,
        PackageId::BatPakCli => Gate0PackageTargetKind::Binary,
        PackageId::BatPakExamples => Gate0PackageTargetKind::ExampleBinary,
    }
}

/// The package-relative path of the package's single source door.
pub const fn source_suffix(package: PackageId) -> &'static str {
    match target_kind(package) {
        Gate0PackageTargetKind::Library | Gate0PackageTargetKind::ProcMacroLibrary => "src/lib.rs",
        Gate0PackageTargetKind::Binary => "src/main.rs",
        Gate0PackageTargetKind::ExampleBinary => "src/bin/family_smoke.rs",
    }
}

/// The named Cargo binary target a package carries, if any. Explicit per
/// package: a binary-kind package without a name here is a seedcheck finding,
/// and a library package can never grow one silently.
pub const fn binary_target(package: PackageId) -> Option<&'static str> {
    match package {
        PackageId::BatPakCli => Some("batpak"),
        PackageId::BatPakExamples => Some("family_smoke"),
        PackageId::MacBatCompiler
        | PackageId::MacBat
        | PackageId::BatPak
        | PackageId::SyncBat
        | PackageId::BatQl
        | PackageId::NetBat
        | PackageId::TestPak => None,
    }
}

/// The common workspace license expression every generated manifest inherits.
pub const WORKSPACE_LICENSE: &str = "MIT OR Apache-2.0";

/// The canonical repository URL every generated manifest inherits.
pub const WORKSPACE_REPOSITORY: &str = "https://github.com/freebatteryfactory/batpak";

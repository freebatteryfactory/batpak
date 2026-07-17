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

/// One closed enum and its `ALL` inventory from a single variant list, so a
/// variant can never be authored without joining the inventory the laws walk
/// (5.5E5a). No `ALL.len() == N` cardinality survives anywhere: completeness
/// is the construction, not a counted assertion. The auditor re-parses this
/// invocation independently; seedcheck executes uniqueness and expansion over
/// the constructed inventory, never a frozen number.
macro_rules! gate0_closed_enum {
    (
        $(#[$emeta:meta])*
        pub enum $name:ident { $( $(#[$vmeta:meta])* $variant:ident ),+ $(,)? }
    ) => {
        $(#[$emeta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $( $(#[$vmeta])* $variant ),+ }

        impl $name {
            /// Every variant, in declaration order, generated from the one
            /// variant list above. The enum and its inventory cannot diverge.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];
        }
    };
}

gate0_closed_enum! {
    /// A workspace-root file the Gate-0 candidate carries exactly once.
    pub enum Gate0RootArtifact {
        /// The Cargo workspace manifest.
        WorkspaceManifest,
        /// The pinned toolchain selection, byte-identical to the tracked root file.
        RustToolchain,
        /// The discoverability-only command file.
        Justfile,
    }
}

impl Gate0RootArtifact {
    /// The output-root-relative path this artifact is published at.
    pub const fn relative_path(self) -> &'static str {
        match self {
            Gate0RootArtifact::WorkspaceManifest => "Cargo.toml",
            Gate0RootArtifact::RustToolchain => "rust-toolchain.toml",
            Gate0RootArtifact::Justfile => "justfile",
        }
    }
}

gate0_closed_enum! {
    /// A file family every admitted package expands to exactly once.
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
}

gate0_closed_enum! {
    /// How a package's Gate-0 skeleton compiles. The kind selects the source
    /// door and the Cargo target stanza; it never selects semantics.
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
}

gate0_closed_enum! {
    /// A file family every SyncBat plane expands to exactly once.
    pub enum Gate0PlaneArtifact {
        /// `crates/syncbat/src/<module>.rs` — the plane's public module file.
        Module,
        /// `crates/syncbat/src/<module>/README.md` — the plane directory's
        /// ownership note.
        DirectoryReadme,
    }
}

gate0_closed_enum! {
    /// The only two successful publication outcomes. Anything else — a partial
    /// tree, a mismatching file, an extra file, a symlink — is a refusal, never
    /// an in-place repair.
    pub enum Gate0OutputDisposition {
        /// The output path did not exist and an exact new candidate tree was
        /// published atomically.
        Created,
        /// The output path already contained exactly the planned tree; nothing
        /// was written.
        Unchanged,
    }
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

/// One portable relative-path grammar for every planned output path (5.5E5a).
/// The plan is language-neutral POSIX identity, but the authoritative host is
/// Windows: a path that is clean under forward-slash rules can still acquire
/// filesystem meaning through a backslash, a drive colon, a trailing dot, or a
/// reserved device name. This law admits only paths that are safe on every
/// platform the candidate is published to. Case-fold collision is a PLAN-level
/// property (two paths colliding), checked where the plan is expanded, not
/// here.
pub fn is_portable_gate0_relative_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') {
        return false;
    }
    if path.contains('\\') || path.contains(':') {
        return false;
    }
    if path.chars().any(|c| (c as u32) < 0x20 || c == '\u{7f}') {
        return false;
    }
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return false;
        }
        if component.ends_with('.') || component.ends_with(' ') {
            return false;
        }
        // Windows reserved device names, with or without an extension, are
        // refused regardless of case: `NUL`, `com1.rs`, `AUX` all bind to a
        // device, not a file.
        let stem = component.split('.').next().unwrap_or(component);
        let upper = stem.to_ascii_uppercase();
        let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (upper.len() == 4
                && (upper.starts_with("COM") || upper.starts_with("LPT"))
                && upper.as_bytes()[3].is_ascii_digit()
                && upper.as_bytes()[3] != b'0');
        if reserved {
            return false;
        }
    }
    true
}

/// The common workspace license expression every generated manifest inherits.
pub const WORKSPACE_LICENSE: &str = "MIT OR Apache-2.0";

/// The canonical repository URL every generated manifest inherits.
pub const WORKSPACE_REPOSITORY: &str = "https://github.com/freebatteryfactory/batpak";

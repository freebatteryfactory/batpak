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
        /// The workspace Cargo configuration carrying the native
        /// `build.warnings = "deny"` posture (Cargo 1.97). Generated
        /// workspace only: the bootstrap tools keep `#![deny(warnings)]`.
        CargoConfig,
    }
}

impl Gate0RootArtifact {
    /// The output-root-relative path this artifact is published at.
    pub const fn relative_path(self) -> &'static str {
        match self {
            Gate0RootArtifact::WorkspaceManifest => "Cargo.toml",
            Gate0RootArtifact::RustToolchain => "rust-toolchain.toml",
            Gate0RootArtifact::Justfile => "justfile",
            Gate0RootArtifact::CargoConfig => ".cargo/config.toml",
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
    /// A file family every SyncBat plane expands to exactly once. A plane is a
    /// directory under the recursive domain-directory source grammar: its
    /// `mod.rs` is the plane facade and its private `types.rs` carries the
    /// plane's canonical nouns. No README artifact exists in the output
    /// grammar: the ownership prose lives in the module door's `//!` docs.
    pub enum Gate0PlaneArtifact {
        /// `crates/syncbat/src/<module>/mod.rs` — the plane's module door:
        /// facade docs (absorbing the plane's authored ownership), the
        /// `mod types;` declaration, and the Gate-0 plane marker.
        ModuleDoor,
        /// `crates/syncbat/src/<module>/types.rs` — the plane's private
        /// canonical-noun carrier; its vocabulary arrives only at the
        /// plane's signed gate, so Gate-0 publishes it intentionally empty.
        TypesCarrier,
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

/// One portable relative-path grammar for every planned output path (5.5E5a,
/// rebuilt as a POSITIVE grammar in 5.5E6b). The plan is language-neutral POSIX
/// identity, but the authoritative host is Windows: a path that is clean under
/// forward-slash rules can still acquire filesystem meaning through a
/// backslash, a drive colon, a trailing dot, a Windows-invalid metacharacter,
/// or a reserved device name.
///
/// A negative filter (reject these bad characters) is the wrong shape: it
/// admits everything nobody thought to forbid, which is how the E5a form let a
/// non-ASCII component and the Windows-invalid set `< > " | ? *` through. This
/// law instead SPELLS OUT what a component may contain — ASCII
/// `[A-Za-z0-9._-]` only — so a byte that cannot be spelled cannot smuggle a
/// device colon, a backslash separator, a wildcard, or a non-ASCII homoglyph.
/// The allow-list subsumes the old backslash/colon/control/space rejections by
/// construction. Case-fold collision is a PLAN-level property (two paths
/// colliding), checked where the plan is expanded, not here.
pub fn is_portable_gate0_relative_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') {
        return false;
    }
    path.split('/').all(is_portable_gate0_path_component)
}

/// The positive component grammar. A single path segment is portable iff it is
/// nonempty, is not a traversal token, is drawn only from the ASCII portable
/// set, does not end in a dot, and is not a Windows reserved device stem.
pub fn is_portable_gate0_path_component(component: &str) -> bool {
    if component.is_empty() || component == "." || component == ".." {
        return false;
    }
    // Positive character grammar: ASCII `[A-Za-z0-9._-]` only. This rejects, by
    // construction, every non-ASCII byte, every Windows-invalid character
    // (`< > : " | ? * \`), whitespace, and every control character — the
    // component simply cannot be spelled with them.
    if !component
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    {
        return false;
    }
    // A component may not end in a dot: Windows silently strips trailing dots,
    // so `foo.` and `foo` would collide on the authoritative host. (A trailing
    // space is already impossible under the character grammar above.)
    if component.ends_with('.') {
        return false;
    }
    // Windows reserved device names, with or without an extension, bind to a
    // device rather than a file, regardless of case: `NUL`, `com1.rs`, `AUX`.
    let stem = component.split('.').next().unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0');
    !reserved
}

/// The common workspace license expression every generated manifest inherits.
pub const WORKSPACE_LICENSE: &str = "MIT OR Apache-2.0";

/// The canonical repository URL every generated manifest inherits.
pub const WORKSPACE_REPOSITORY: &str = "https://github.com/freebatteryfactory/batpak";

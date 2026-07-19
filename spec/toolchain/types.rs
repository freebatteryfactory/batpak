use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// An exact Rust release — the compiler that qualifies this foundation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RustRelease {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl RustRelease {
    /// Canonical `MAJOR.MINOR.PATCH` spelling.
    pub fn render(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Whether this release satisfies the declared MSRV floor. The exact
    /// qualifying compiler and the floor answer DIFFERENT questions: a newer
    /// qualifying compiler that preserves the floor is ordinary; a
    /// qualifying compiler below the floor is a contradiction — the
    /// foundation would be qualified by a compiler its own consumers may
    /// lawfully refuse.
    pub const fn satisfies_floor(self, floor: RustVersionFloor) -> bool {
        self.major > floor.major || (self.major == floor.major && self.minor >= floor.minor)
    }
}

/// The MSRV floor the generated workspace claims (`rust-version`): the
/// oldest compiler consumers may use. Deliberately carries no patch — an
/// MSRV claim is a minor-line promise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RustVersionFloor {
    pub major: u16,
    pub minor: u16,
}

impl RustVersionFloor {
    /// Canonical `MAJOR.MINOR` spelling.
    pub fn render(self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

/// The Rust edition every compilation unit uses. One variant: the cleanroom
/// admits exactly one edition, and a second one arrives by ruling, not by
/// string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustEdition {
    Rust2024,
}

impl RustEdition {
    pub const fn spelling(self) -> &'static str {
        match self {
            RustEdition::Rust2024 => "2024",
        }
    }
}

/// The Cargo dependency resolver the generated workspace declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoResolver {
    V3,
}

impl CargoResolver {
    pub const fn spelling(self) -> &'static str {
        match self {
            CargoResolver::V3 => "3",
        }
    }
}

/// The rustup profile the toolchain selection installs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustupProfile {
    Minimal,
}

impl RustupProfile {
    pub const fn spelling(self) -> &'static str {
        match self {
            RustupProfile::Minimal => "minimal",
        }
    }
}

/// The components qualification depends on. Declaration order IS the
/// canonical projection order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustupComponent {
    Clippy,
    Rustfmt,
}

impl RustupComponent {
    /// The ONE component denominator (5.5E3 preflight): every declared
    /// variant, in canonical projection order. The profile renders from this
    /// inventory directly — a `required_components` field beside it would be
    /// a second inventory the checkers must police back into agreement, and
    /// dropping a component from such a field while the projection moved in
    /// lockstep would leave a declared-but-unconsumed variant, the
    /// TestFixture defect wearing a toolbelt. The auditor independently
    /// compares the declared variants against this inventory.
    pub const ALL: &'static [RustupComponent] =
        &[RustupComponent::Clippy, RustupComponent::Rustfmt];

    pub const fn spelling(self) -> &'static str {
        match self {
            RustupComponent::Clippy => "clippy",
            RustupComponent::Rustfmt => "rustfmt",
        }
    }
}

/// A physical compiler/build target triple (5.5E6a): WHERE a binary was
/// compiled, linked, and run. This is deliberately distinct from
/// `QualificationEnvironment`, which says WHAT semantic profile must hold
/// (no_std + alloc, native std, wasm host). A receipt binds a triple; a
/// semantic environment can never substitute for one, and the two share no
/// type, spelling, or authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustTargetTriple {
    /// The authoritative Windows target: link.exe and the Windows SDK.
    X86_64PcWindowsMsvc,
    /// The supplemental Windows target: developer machines that carry a GNU
    /// toolchain and never install MSVC Build Tools.
    X86_64PcWindowsGnu,
}

impl RustTargetTriple {
    /// Every admitted physical target, in canonical order (authoritative first).
    pub const ALL: &'static [RustTargetTriple] = &[
        RustTargetTriple::X86_64PcWindowsMsvc,
        RustTargetTriple::X86_64PcWindowsGnu,
    ];

    /// The exact rustc/cargo triple spelling.
    pub const fn triple(self) -> &'static str {
        match self {
            RustTargetTriple::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
            RustTargetTriple::X86_64PcWindowsGnu => "x86_64-pc-windows-gnu",
        }
    }

    /// Whether a receipt bound to this triple is the AUTHORITATIVE Windows
    /// qualification. The supplemental target is local, corroborating evidence
    /// and never closes a gate on its own.
    ///
    /// Authority has ONE owner: `AUTHORITATIVE_TARGET`. A per-variant
    /// `true`/`false` match here would be a second declaration of the same
    /// fact — move authority to a new triple and the two would silently
    /// disagree. This compares against the single constant instead.
    pub const fn is_authoritative(self) -> bool {
        matches!(self, AUTHORITATIVE_TARGET)
    }
}

/// The one authoritative Windows target. A qualification that closes a gate
/// must carry receipts bound to this triple. This constant is the SOLE owner
/// of the authority fact; `is_authoritative()` compares against it.
pub const AUTHORITATIVE_TARGET: RustTargetTriple = RustTargetTriple::X86_64PcWindowsMsvc;

/// The one toolchain authority. Two version fields because they answer
/// DIFFERENT questions and may never be flattened into one string:
///
/// ```text
/// exact_rust_release   which compiler qualifies this foundation
/// rust_version_floor   oldest compiler the generated workspace claims
/// ```
///
/// The executed law between them is `exact >= floor`, never "same minor":
/// requiring the qualifying compiler to share the MSRV minor would quietly
/// re-couple the fields this type exists to separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolchainProfile {
    /// The exact compiler release that qualifies the cleanroom foundation.
    pub exact_rust_release: RustRelease,
    /// The MSRV floor the generated workspace claims (`rust-version`).
    pub rust_version_floor: RustVersionFloor,
    /// The Rust edition every compilation unit uses.
    pub edition: RustEdition,
    /// The Cargo dependency resolver the generated workspace declares.
    pub cargo_resolver: CargoResolver,
    /// The rustup profile the toolchain selection installs. The required
    /// components are NOT a field: `RustupComponent::ALL` is the one
    /// component denominator and the projection renders from it directly.
    pub rustup_profile: RustupProfile,
}

pub const TOOLCHAIN: ToolchainProfile = ToolchainProfile {
    exact_rust_release: RustRelease { major: 1, minor: 97, patch: 1 },
    rust_version_floor: RustVersionFloor { major: 1, minor: 97 },
    edition: RustEdition::Rust2024,
    cargo_resolver: CargoResolver::V3,
    rustup_profile: RustupProfile::Minimal,
};

impl ToolchainProfile {
    /// The deterministic projection the TRACKED root `rust-toolchain.toml`
    /// must equal byte-for-byte. The tracked file has a special role: it
    /// selects the compiler BEFORE this typed owner can compile, so it is
    /// necessarily a bootstrap projection — but a projection is not a second
    /// authority, and seedcheck compares the tracked bytes against this
    /// rendering on every run.
    pub fn root_toolchain_toml(&self) -> String {
        let components = RustupComponent::ALL
            .iter()
            .map(|c| format!("\"{}\"", c.spelling()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[toolchain]\nchannel = \"{}\"\nprofile = \"{}\"\ncomponents = [{}]\n",
            self.exact_rust_release.render(),
            self.rustup_profile.spelling(),
            components
        )
    }

    /// The TRACKED root projection (5.5E4a): the bare selection above plus
    /// the machine-readable provenance line, because the tracked file is a
    /// registered generated view (spec/generated_views/registry.rs RustToolchain)
    /// whose generator is bootstrap/project.py. The comment is part of the
    /// exact projected bytes; the materializer consumes this SAME form so
    /// its idempotent write matches the tracked file — one typed owner, two
    /// mechanical consumers, one byte sequence.
    pub fn tracked_root_toolchain_toml(&self) -> String {
        format!(
            "# generated from spec/toolchain/types.rs by bootstrap/project.py; do not edit\n{}",
            self.root_toolchain_toml()
        )
    }
}

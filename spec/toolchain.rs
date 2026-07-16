//! The typed toolchain owner (5.5E3a).
//!
//! Until this file existed the toolchain facts lived as scattered string
//! literals: the materializer hardcoded the resolver, edition, MSRV, and the
//! emitted workspace toolchain bytes; the qualification harness hardcoded
//! seven copies of the edition; and no tracked root toolchain selection
//! existed at all. Every consumer now derives from `TOOLCHAIN`, and the two
//! independent Python paths reconstruct and attack these values rather than
//! owning copies.

/// The one toolchain authority. Two version fields because they answer
/// DIFFERENT questions and may never be flattened into one string:
///
/// ```text
/// exact_rust_release   which compiler qualifies this foundation
/// rust_version_floor   oldest compiler the generated workspace claims
/// ```
///
/// Semantic qualification targets (`no_std + alloc`, `std`, `wasm32 host`)
/// and physical target triples (`x86_64-pc-windows-msvc`) are likewise
/// different dimensions: a capability environment names WHAT must hold, a
/// triple names WHERE a binary ran. Neither substitutes for the other, and
/// the laws in seedcheck and the receipt denominator refuse both directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolchainProfile {
    /// The exact compiler release that qualifies the cleanroom foundation.
    pub exact_rust_release: &'static str,
    /// The MSRV floor the generated workspace claims (`rust-version`).
    pub rust_version_floor: &'static str,
    /// The Rust edition every compilation unit uses.
    pub edition: &'static str,
    /// The Cargo dependency resolver the generated workspace declares.
    pub cargo_resolver: &'static str,
    /// The rustup profile the toolchain selection installs.
    pub rustup_profile: &'static str,
    /// Required components, in canonical (sorted) order.
    pub required_components: &'static [&'static str],
}

pub const TOOLCHAIN: ToolchainProfile = ToolchainProfile {
    exact_rust_release: "1.97.0",
    rust_version_floor: "1.97",
    edition: "2024",
    cargo_resolver: "3",
    rustup_profile: "minimal",
    required_components: &["clippy", "rustfmt"],
};

/// The semantic qualification environments (capability profiles). These are
/// NOT Rust target triples: `std` names what a build may assume, never where
/// it ran. `QualificationProfile::target` must name one of these, and the
/// receipt denominator refuses one of these where a triple is required.
pub const SEMANTIC_QUALIFICATION_TARGETS: &[&str] = &["no_std + alloc", "std", "wasm32 host"];

impl ToolchainProfile {
    /// The deterministic projection the TRACKED root `rust-toolchain.toml`
    /// must equal byte-for-byte. The tracked file has a special role: it
    /// selects the compiler BEFORE this typed owner can compile, so it is
    /// necessarily a bootstrap projection — but a projection is not a second
    /// authority, and seedcheck compares the tracked bytes against this
    /// rendering on every run.
    pub fn root_toolchain_toml(&self) -> String {
        let components = self
            .required_components
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[toolchain]\nchannel = \"{}\"\nprofile = \"{}\"\ncomponents = [{}]\n",
            self.exact_rust_release, self.rustup_profile, components
        )
    }
}

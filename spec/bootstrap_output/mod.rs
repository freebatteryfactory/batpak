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

mod types;

pub use types::{
    Gate0OutputDisposition, Gate0PackageArtifact, Gate0PackageTargetKind, Gate0PlaneArtifact,
    Gate0RootArtifact, WORKSPACE_LICENSE, WORKSPACE_REPOSITORY, binary_target,
    is_portable_gate0_path_component, is_portable_gate0_relative_path, source_suffix, target_kind,
};

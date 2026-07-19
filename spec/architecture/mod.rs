//! Frozen clean-room repository architecture facts.

mod types;
mod inventory;

pub use types::{
    EdgeClass, EdgeSpec, PackageClass, PackageId, PackageSpec, QualificationEnvironment,
    QualificationProfile, SyncBatPlane,
};
pub use inventory::{
    EDGES, FORBIDDEN_TARGET_PATHS, PACKAGES, QUALIFICATION_PROFILES, REPOSITORY_NAME,
    REQUIRED_DOCS, SPEC_VERSION, WORKSPACE_VERSION,
};

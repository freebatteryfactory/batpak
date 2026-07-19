//! Frozen clean-room repository architecture facts.

mod types;
mod inventory;
mod authenticated_history;
mod release_seal;

pub use types::{
    EdgeClass, EdgeSpec, PackageClass, PackageId, PackageSpec, QualificationEnvironment,
    QualificationProfile, SyncBatPlane,
};
pub use inventory::{
    EDGES, FORBIDDEN_TARGET_PATHS, PACKAGES, QUALIFICATION_PROFILES, REPOSITORY_NAME,
    REQUIRED_DOCS, SPEC_VERSION, WORKSPACE_VERSION,
};
pub use authenticated_history::{
    AuthenticatedHistoryClaims, AuthenticatedHistoryProfile, AuthenticityClaim, FreshnessClaim,
    IntegrityClaim, RollbackResistanceClaim, WitnessDisposition, WitnessPolicy,
    INTERNAL_CONSISTENCY_SUCCESS, OPTIONAL_WITNESS_REFUSAL_SET, REFUSAL_PARTIAL_CLAIM_LAW,
    REQUIRED_WITNESS_FAILURE_SET, SIGNED_UNANCHORED_SUCCESS, WITNESSED_SUCCESS,
};
pub use release_seal::{ReleaseSealField, RELEASE_SEAL_FIELDS};

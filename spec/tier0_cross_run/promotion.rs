use alloc::string::String;

use crate::bootstrap_qualification::{
    AUTHORITATIVE_WORKFLOW_PATH, BootstrapRuntimeBinding, ToolchainBinding,
    VerifiedTier0Qualification,
};
use crate::toolchain::RustTargetTriple;
use super::*;

// ===========================================================================
// Promotion confirmation (5.5E6c1): strictly stronger than same-source
// ===========================================================================

/// Why two verified qualifications do NOT confirm a promotion — even when they
/// may still corroborate the same committed source snapshot. Each arm names the
/// exact posture requirement promotion confirmation adds over same-source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromotionConfirmationError {
    /// The two runs are not even same-source. Promotion confirmation is strictly
    /// stronger, so it inherits every same-source refusal; the underlying
    /// comparison (a divergence or a not-comparable precondition) is carried.
    NotSameSource(CrossRunComparison),
    /// A side qualified a NON-authoritative target. Promotion confirms only the
    /// authoritative target; a supplemental target never promotes.
    NonAuthoritativeTarget {
        which: Side,
        target: RustTargetTriple,
    },
    /// The two runs qualified DIFFERENT targets. A cross-target pair is orthogonal
    /// corroboration of one source, not a confirmation of one promotion.
    CrossTarget {
        candidate: RustTargetTriple,
        cleanroom: RustTargetTriple,
    },
    /// The two runs ran on DIFFERENT toolchains. Same-source tolerates this;
    /// promotion confirmation requires the identical pinned toolchain on both.
    ToolchainDivergent {
        candidate: ToolchainBinding,
        cleanroom: ToolchainBinding,
    },
    /// The two runs ran under DIFFERENT bootstrap runtimes. Same-source tolerates
    /// this; promotion confirmation requires the identical pinned CPython release.
    BootstrapRuntimeDivergent {
        candidate: BootstrapRuntimeBinding,
        cleanroom: BootstrapRuntimeBinding,
    },
    /// The two runs ran in DIFFERENT repositories.
    RepositoryMismatch {
        candidate: String,
        cleanroom: String,
    },
    /// A run's workflow path is not the canonical authoritative workflow
    /// (`AUTHORITATIVE_WORKFLOW_PATH`). `which` names the offending side(s).
    NonCanonicalWorkflowPath { which: Side, path: String },
}

/// Confirm that a candidate run and a cleanroom run promote one committed source
/// snapshot to the authoritative target.
///
/// Same-source is necessary but not sufficient: this additionally requires the
/// authoritative target on BOTH sides, the identical pinned toolchain, the same
/// repository, and the canonical authoritative workflow path — the posture the
/// candidate-exact-SHA and cleanroom-exact-SHA runs share across a lawful
/// fast-forward. The two runs are already DISTINCT (same-source refuses one run
/// compared against itself), and each qualification was already independently
/// verified before `verify` sealed it (and, on the wire, recomputed by
/// `bootstrap/receiptcheck.rs`). `candidate` becomes the left run and `cleanroom`
/// the right run of the retained same-source proof.
pub fn confirm_promotion(
    candidate: &VerifiedTier0Qualification,
    cleanroom: &VerifiedTier0Qualification,
) -> Result<PromotionConfirmationProof, PromotionConfirmationError> {
    // Same-source first — promotion confirmation inherits every same-source arm.
    let comparison = compare_runs(candidate, cleanroom);
    let CrossRunComparison::SameSource(same_source) = comparison else {
        return Err(PromotionConfirmationError::NotSameSource(comparison));
    };

    // Both sides must qualify the authoritative target.
    let cand_target = candidate.target();
    let clean_target = cleanroom.target();
    let cand_auth = cand_target.is_authoritative();
    let clean_auth = clean_target.is_authoritative();
    if !cand_auth || !clean_auth {
        return Err(PromotionConfirmationError::NonAuthoritativeTarget {
            which: Side::of(!cand_auth, !clean_auth),
            target: if !cand_auth { cand_target } else { clean_target },
        });
    }
    // Both authoritative implies the same target today (one authoritative target
    // exists), but name a cross-target pair explicitly so this stays honest if a
    // second authoritative target is ever admitted.
    if cand_target != clean_target {
        return Err(PromotionConfirmationError::CrossTarget {
            candidate: cand_target,
            cleanroom: clean_target,
        });
    }

    // Identical pinned toolchain — recorded, not merely observed, by same-source.
    let ToolchainAgreement::Identical(toolchain) = *same_source.toolchain_agreement() else {
        return Err(PromotionConfirmationError::ToolchainDivergent {
            candidate: *candidate.toolchain(),
            cleanroom: *cleanroom.toolchain(),
        });
    };

    // Identical pinned bootstrap runtime (CPython release) — the interpreter that
    // executed the Python gates is builder identity beside rustc/cargo.
    let BootstrapRuntimeAgreement::Identical(bootstrap_runtime) =
        *same_source.bootstrap_runtime_agreement()
    else {
        return Err(PromotionConfirmationError::BootstrapRuntimeDivergent {
            candidate: *candidate.bootstrap_runtime(),
            cleanroom: *cleanroom.bootstrap_runtime(),
        });
    };

    // Same repository, canonical authoritative workflow, on both distinct runs.
    let cand_run = same_source.left_run();
    let clean_run = same_source.right_run();
    if cand_run.repository != clean_run.repository {
        return Err(PromotionConfirmationError::RepositoryMismatch {
            candidate: cand_run.repository.clone(),
            cleanroom: clean_run.repository.clone(),
        });
    }
    let cand_canon = cand_run.workflow_path == AUTHORITATIVE_WORKFLOW_PATH;
    let clean_canon = clean_run.workflow_path == AUTHORITATIVE_WORKFLOW_PATH;
    if !cand_canon || !clean_canon {
        return Err(PromotionConfirmationError::NonCanonicalWorkflowPath {
            which: Side::of(!cand_canon, !clean_canon),
            path: if !cand_canon {
                cand_run.workflow_path.clone()
            } else {
                clean_run.workflow_path.clone()
            },
        });
    }

    // Clone the run identities into owned values so the same-source proof can be
    // moved into the confirmation without an outstanding borrow.
    let candidate_run = cand_run.clone();
    let cleanroom_run = clean_run.clone();
    Ok(PromotionConfirmationProof {
        same_source,
        target: cand_target,
        toolchain,
        bootstrap_runtime,
        candidate_run,
        cleanroom_run,
        _seal: (),
    })
}

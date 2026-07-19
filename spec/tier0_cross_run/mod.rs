//! Cross-run same-source comparison over verified Tier 0 qualifications (5.5E6c).
//!
//! E6b sealed a `VerifiedTier0Qualification` that RETAINS every binding. E6c
//! asks the question those retained bindings exist to answer: do two INDEPENDENT
//! hosted runs describe the SAME committed source snapshot? A candidate-branch
//! run happens BEFORE promotion, so the corroborated thing is a committed source
//! snapshot, not yet a "promoted" one — E6c1 adds a strictly stronger
//! `confirm_promotion` for the candidate-to-cleanroom promotion question.
//!
//! ## The honest boundary, grounded in real evidence
//!
//! Two real hosted MSVC runs of one commit (`8d811dab`) printed byte-identical
//! source-commit, git-tree, spec-manifest, workflow, and materializer-output-tree
//! digests — but DIFFERENT seedcheck, materialize, and spec-tests EXECUTABLE
//! digests. This is NOT a universal law that "MSVC builds are never
//! byte-reproducible": the current Tier 0 contract simply does not REQUIRE
//! cross-run executable-byte identity, and each run independently binds its own
//! executables. A comparator that demanded executable-digest equality would
//! therefore FALSELY reject two legitimately-same-source runs: a dishonest red. A
//! later reproducible-build law may ADD executable equality as stronger
//! corroboration without redefining today's source identity. So the same-source
//! proof rests only on the deterministic, source-derived coordinates:
//!
//! ```text
//! source commit          committed, git-checkout only
//! source tree            committed, git-checkout only
//! spec-manifest digest   digest of the committed SPEC.sha256
//! workflow digest        digest of the committed hosted workflow
//! materializer output    independently recomputed Gate-0 workspace tree
//! ```
//!
//! The materializer output tree is the load-bearing corroboration: two separate
//! machines each rebuilt the Gate-0 workspace and reduced it to the same bytes.
//! Compiled-artifact (executable) digests legitimately differ across runs and are
//! NEVER a divergence — they are recorded build-nondeterminism, not disagreement.
//!
//! Toolchain and target may also legitimately differ (a re-run after a runner
//! image bump; the same source qualified on a second platform) while the source
//! is unchanged. They are therefore RECORDED as agreement strength, not required
//! for the same-source verdict.
//!
//! ## What this does and does not prove
//!
//! `compare_runs` proves two runs describe the same SOURCE, not that they are
//! byte-identical builds. The `SameSourceProof` seal prevents a consumer from
//! FABRICATING the conclusion; it does not re-prove that either run's inputs were
//! computed honestly — `bootstrap/receiptcheck.rs` already recomputed those, per
//! run, before `verify` sealed each qualification this comparator consumes.
//!
//! ## Same-source is not promotion confirmation (5.5E6c1)
//!
//! Same-source corroboration and promotion confirmation are DIFFERENT questions,
//! and conflating them would smuggle the conclusion into the premise. Two runs
//! can describe one committed source snapshot across different targets or
//! toolchains (orthogonal corroboration) yet not confirm a candidate-to-cleanroom
//! promotion. `confirm_promotion` is the strictly stronger check: it requires a
//! `SameSourceProof` AND that both runs qualified the AUTHORITATIVE target under
//! the IDENTICAL pinned toolchain in the SAME repository and canonical workflow.
//! Provenance equivalence is not promotion equivalence.

mod types;
mod comparison;
mod promotion;
#[cfg(test)]
mod tests;

pub use types::{
    BootstrapRuntimeAgreement, PromotionConfirmationProof, SameSourceProof, Side,
    TargetAgreement, ToolchainAgreement,
};
pub use comparison::{CrossRunComparison, NotComparable, SourceDivergence, compare_runs};
pub use promotion::{PromotionConfirmationError, confirm_promotion};

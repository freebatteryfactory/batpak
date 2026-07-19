//! The typed campaign lineage records, manifest grammar, terminals, frontier
//! vocabulary, and closure-graph projection nouns (F5).
//!
//! Owned by DEC-079 (immutable candidate nursery and complete lineage),
//! DEC-080 (realization-preserving versus law-changing promotion and the
//! ArchitectRequired terminal home), DEC-081 (bounded search, evaluation-set
//! separation, and transfer reuse), and DEC-082 (whole-tree speculative
//! materialization, frozen judge binding, and frontier qualification);
//! docs/39 owns the campaign doctrine in prose.
//!
//! This domain owns the RECORD side of the campaign: what a persisted,
//! content-addressed nursery lineage record carries, how its versioned
//! manifest serializes (`BATPAK-CANDIDATE-MANIFEST/1` from day one — no
//! unversioned "temporary" format ever exists), which terminal dispositions a
//! campaign can reach, how the dependency-first frontier and evidence
//! freshness are spoken about, what a frozen judge snapshot binds, and the
//! node/edge nouns the generated campaign closure view projects.
//!
//! What it deliberately does NOT own:
//!
//! - `spec/sprouting/` owns the candidate AXES (origin, change class,
//!   evaluation-set role, realization posture, repair authority) and the
//!   promotion-plan admission law; this domain consumes those axes and never
//!   duplicates them.
//! - `spec/promotion/` owns the conjunctive promotion denominator.
//! - `spec/identities/` owns the `Candidate` identity kind and the
//!   `CandidateManifest` version identity as catalog facts.
//! - `spec/bootstrap_qualification/` owns the `Sha256Digest` bootstrap
//!   primitive, which this domain reuses rather than minting a second digest
//!   type.
//! - The campaign HARNESS — search loop, materializer, judge runtime,
//!   evidence bundle — is bootstrap/selftest territory, not spec vocabulary.
//!
//! Lineage relations are DERIVED from the required record fields, never
//! authored beside them: a parent edge is the parent-`CandidateId` field, a
//! dependency edge is a dependency commitment, a reuse edge is the reuse key,
//! an invalidation edge is the invalidation receipt. The closure edge kind
//! here is the projection's read-back vocabulary for the generated view, not
//! a parallel relation authority. Every axis is a closed enum with a frozen
//! `ALL` inventory and derives `Clone`/`Copy`/`Debug`/`PartialEq`/`Eq` ONLY:
//! no ordering, rank, level, or score — a campaign terminal is a name, not a
//! magnitude. This domain authors no rows, so it carries no inventory.rs:
//! records are runtime-minted in the external campaign root, never authored
//! in the tracked tree.

mod types;

pub use types::{
    CampaignClosureEdge, CampaignClosureEdgeKind, CampaignClosureNode, CampaignTerminal,
    CampaignTerminalReceipt, CandidateId, CandidateManifest, CandidateRecord,
    DependencyCommitment, EvidenceFreshness, EvidenceRef, FrontierState, FrozenJudgeSnapshot,
    ReceiptRef, ReuseKey,
};

#[cfg(test)]
mod tests;

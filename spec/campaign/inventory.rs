use crate::proof::ProofRowId;

/// A typed campaign evidence profile (E7, TL-10): the spec-owned declaration
/// of WHICH proof rows a named campaign rehearsal must realize with fresh
/// executed evidence. Before this type existed the row set lived as a Python
/// list (`REALIZED_PROOF_ROWS` in the supernova harness) — a dynamic-semantics
/// map the E7 ruling removes: Rust owns the profile, Python PARSES this const
/// (the `project/campaign.py` regex idiom), and receiptcheck consumes the
/// compiled profile — producer and verifier stay independent, and no shared
/// parser exists.
///
/// A profile NAMES rows and authors none: `spec/proof/` owns the proof-row
/// identity catalog, and seedcheck proves every named row is Active there —
/// a profile cannot claim a row the catalog does not carry alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignEvidenceProfile {
    /// The profile's own versioned identity, named by the reuse-key
    /// preimage's `profile` line.
    pub id: &'static str,
    /// The proof rows the campaign must realize with fresh executed
    /// evidence, in `spec/proof/inventory.rs` catalog order. Receiptcheck
    /// verifies fresh evidence per row; a variant is not adopted merely
    /// because a row mentions it.
    pub realized_rows: &'static [ProofRowId],
}

/// The mini-supernova rehearsal profile: the NINE F5 rehearsal rows the
/// campaign rehearsal harness realizes literally (docs/24 owns each row's
/// semantic meaning; `spec/proof/inventory.rs` carries the rows Active).
pub const MINI_SUPERNOVA_PROFILE: CampaignEvidenceProfile = CampaignEvidenceProfile {
    id: "mini-supernova-rehearsal/1",
    realized_rows: &[
        ProofRowId("planted_semantic_mutant_is_activated_and_killed"),
        ProofRowId("bounded_generated_trace_attack_holds_boundary"),
        ProofRowId("deterministic_replay_equality_of_simulated_trace"),
        ProofRowId("runtime_capture_appends_observed_history_without_rewrite"),
        ProofRowId("offline_replay_concludes_conformance_for_captured_history"),
        ProofRowId("every_admitted_campaign_obligation_reaches_a_terminal"),
        ProofRowId("candidate_search_terminates_within_declared_budget"),
        ProofRowId("bounded_repair_loop_reaches_stable_qualified_frontier"),
        ProofRowId("confirming_rerun_changes_no_authoritative_result"),
    ],
};

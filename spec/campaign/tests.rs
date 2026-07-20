use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::bootstrap_qualification::{HexParseError, Sha256Digest};
use crate::proof::{ProofRowId, ProofRowState, PROOF_ROWS};
use crate::sprouting::{CandidateChangeClass, CandidateOriginKind, RealizationPosture};

use super::*;

fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}

fn hex64(byte: u8) -> String {
    digest(byte).render()
}

/// A fully populated V2 record: a repair child naming two failed-lineage
/// parents, two named proof targets, and one dependency. The proof targets
/// and parents are deliberately OUT of lexicographic order so the byte-exact
/// rendering test proves the renderer canonicalizes.
fn full_record() -> CandidateRecord {
    CandidateRecord {
        id: CandidateId::from_digest(digest(0x11)),
        proof_targets: vec![
            ProofRowId("planted_semantic_mutant_is_activated_and_killed"),
            ProofRowId("bounded_repair_loop_reaches_stable_qualified_frontier"),
        ],
        parents: vec![
            CandidateId::from_digest(digest(0x22)),
            CandidateId::from_digest(digest(0x21)),
        ],
        source_frontier_commitment: digest(0x33),
        dependency_commitments: vec![DependencyCommitment {
            dependency: CandidateId::from_digest(digest(0x44)),
            commitment: digest(0x55),
        }],
        content_commitment: digest(0x66),
        origin: CandidateOriginKind::RepairOfCandidate,
        change_class: CandidateChangeClass::RealizationPreserving,
        realization_posture: RealizationPosture::Candidate,
        reuse_key: ReuseKey::from_digest(digest(0xaa)),
    }
}

#[test]
fn candidate_id_round_trips_canonical_hex() {
    let id = CandidateId::from_hex(&hex64(0x5a)).expect("64 lowercase hex parses");
    assert_eq!(id.render(), hex64(0x5a));
    assert_eq!(id, CandidateId::from_digest(digest(0x5a)));
}

#[test]
fn candidate_id_refuses_abbreviated_hex() {
    assert_eq!(
        CandidateId::from_hex("5a5a5a"),
        Err(HexParseError::WrongLength)
    );
}

#[test]
fn candidate_id_refuses_uppercase_hex() {
    let uppercase = hex64(0x5a).to_uppercase();
    assert_eq!(
        CandidateId::from_hex(&uppercase),
        Err(HexParseError::NonLowerHexDigit)
    );
}

#[test]
fn manifest_first_line_is_the_versioned_magic() {
    let rendered = CandidateManifest { record: full_record() }.render();
    assert_eq!(
        rendered.lines().next(),
        Some("BATPAK-CANDIDATE-MANIFEST/2")
    );
    assert_eq!(CandidateManifest::MAGIC, "BATPAK-CANDIDATE-MANIFEST/2");
}

#[test]
fn manifest_renders_the_exact_documented_field_order() {
    // Byte-exact V2: proof targets before parents, counts before every
    // repeated section, NO evidence/receipt/terminal lines, and every
    // repeated section canonicalized to lexicographic order (the fixture
    // authors both its targets and its parents out of order).
    let rendered = CandidateManifest { record: full_record() }.render();
    let expected = alloc::format!(
        "BATPAK-CANDIDATE-MANIFEST/2\n\
         candidate-id {id}\n\
         proof-target-count 2\n\
         proof-target bounded_repair_loop_reaches_stable_qualified_frontier\n\
         proof-target planted_semantic_mutant_is_activated_and_killed\n\
         parent-count 2\n\
         parent {parent_low}\n\
         parent {parent_high}\n\
         source-frontier-commitment {frontier}\n\
         dependency-count 1\n\
         dependency {dep} {depc}\n\
         content-commitment {content}\n\
         origin RepairOfCandidate\n\
         change-class RealizationPreserving\n\
         realization-posture Candidate\n\
         reuse-key {reuse}\n",
        id = hex64(0x11),
        parent_low = hex64(0x21),
        parent_high = hex64(0x22),
        frontier = hex64(0x33),
        dep = hex64(0x44),
        depc = hex64(0x55),
        content = hex64(0x66),
        reuse = hex64(0xaa),
    );
    assert_eq!(rendered, expected);
}

#[test]
fn candidate_id_preimage_follows_the_documented_grammar() {
    // NORMATIVE candidate-id-preimage/2: the domain line, then every
    // manifest line AFTER the candidate-id line, in manifest order, each
    // LF-terminated. This test executes the documented derivation over the
    // rendered manifest and pins the exact preimage BYTES; hashing those
    // bytes to the CandidateId is receiptcheck's law — this no_std
    // vocabulary crate carries no hasher, and the Python producer and Rust
    // verifier implement the hash independently from this one grammar.
    let rendered = CandidateManifest { record: full_record() }.render();
    let mut preimage = String::from("candidate-id-preimage/2\n");
    let candidate_id_line = alloc::format!("candidate-id {}\n", hex64(0x11));
    let after_id = rendered
        .split_once(&candidate_id_line)
        .expect("the manifest carries the candidate-id line")
        .1;
    preimage.push_str(after_id);
    let expected = alloc::format!(
        "candidate-id-preimage/2\n\
         proof-target-count 2\n\
         proof-target bounded_repair_loop_reaches_stable_qualified_frontier\n\
         proof-target planted_semantic_mutant_is_activated_and_killed\n\
         parent-count 2\n\
         parent {parent_low}\n\
         parent {parent_high}\n\
         source-frontier-commitment {frontier}\n\
         dependency-count 1\n\
         dependency {dep} {depc}\n\
         content-commitment {content}\n\
         origin RepairOfCandidate\n\
         change-class RealizationPreserving\n\
         realization-posture Candidate\n\
         reuse-key {reuse}\n",
        parent_low = hex64(0x21),
        parent_high = hex64(0x22),
        frontier = hex64(0x33),
        dep = hex64(0x44),
        depc = hex64(0x55),
        content = hex64(0x66),
        reuse = hex64(0xaa),
    );
    assert_eq!(preimage, expected);
}

#[test]
fn manifest_states_empty_sets_by_count_and_carries_no_receipt_lines() {
    let mut record = full_record();
    record.parents = Vec::new();
    record.dependency_commitments = Vec::new();
    record.origin = CandidateOriginKind::DeterministicGeneration;
    record.realization_posture = RealizationPosture::Scaffold;
    let rendered = CandidateManifest { record }.render();
    // An empty set is stated by its 0 count, never omitted.
    assert!(rendered.contains("parent-count 0\n"));
    assert!(rendered.contains("dependency-count 0\n"));
    assert!(rendered.contains("realization-posture Scaffold\n"));
    // V2 owns mint-time facts ONLY: no evidence, receipt, or terminal line
    // exists in the grammar — those are append-only campaign receipts beside
    // the record, never manifest fields.
    assert!(!rendered.contains("evidence"));
    assert!(!rendered.contains("receipt"));
    assert!(!rendered.contains("terminal"));
    assert!(rendered.ends_with(&alloc::format!("reuse-key {}\n", hex64(0xaa))));
}

#[test]
fn repair_lineage_is_carried_by_required_fields() {
    // DEC-079: a repair is a NEW record whose origin is RepairOfCandidate and
    // whose parent field names the failed candidate; the relation is DERIVED
    // from those fields, never authored beside them.
    let record = full_record();
    assert_eq!(record.origin, CandidateOriginKind::RepairOfCandidate);
    let parent = record.parents[0];
    let edge = CampaignClosureEdge {
        from: record.id,
        to: parent,
        kind: CampaignClosureEdgeKind::Parent,
    };
    assert_eq!(edge.to, CandidateId::from_digest(digest(0x22)));
    assert_eq!(edge.kind.spelling(), "Parent");
}

#[test]
fn every_campaign_terminal_remains_in_denominator() {
    // DEC-080: no terminal ever leaves the denominator — an escalated
    // campaign is accounted for as escalated, not quietly absent.
    for terminal in CampaignTerminal::ALL {
        assert!(terminal.remains_in_denominator(), "{} left the denominator", terminal.spelling());
    }
    assert!(CampaignTerminal::ArchitectRequired.remains_in_denominator());
}

#[test]
fn campaign_terminal_inventory_is_frozen() {
    assert_eq!(CampaignTerminal::ALL.len(), 4);
    let spellings: Vec<&str> = CampaignTerminal::ALL.iter().map(|t| t.spelling()).collect();
    assert_eq!(
        spellings,
        ["Promoted", "Refused", "Invalidated", "ArchitectRequired"]
    );
}

#[test]
fn campaign_receipt_kind_inventory_is_frozen() {
    // Exactly the TEN kinds the BATPAK-CAMPAIGN-RECEIPT/3 wire serializes,
    // spelled as their lowercase wire tokens — generation FIRST because the
    // mint receipt is causally first.
    assert_eq!(CampaignReceiptKind::ALL.len(), 10);
    let spellings: Vec<&str> = CampaignReceiptKind::ALL.iter().map(|k| k.spelling()).collect();
    assert_eq!(
        spellings,
        [
            "generation",
            "qualification",
            "holdout",
            "promotion",
            "refusal",
            "invalidation",
            "escalation",
            "reuse",
            "fuzz",
            "convergence",
        ]
    );
}

#[test]
fn campaign_invalidation_cause_inventory_is_frozen() {
    // The typed RECEIPT/3 invalidation cause names WHICH bound coordinate
    // changed; EvidenceFreshness names the RESULTING evidence state.
    assert_eq!(CampaignInvalidationCause::ALL.len(), 2);
    let spellings: Vec<&str> = CampaignInvalidationCause::ALL
        .iter()
        .map(|c| c.spelling())
        .collect();
    assert_eq!(spellings, ["dependency-commitment-changed", "judge-changed"]);
}

#[test]
fn campaign_refusal_cause_inventory_is_frozen() {
    // The typed RECEIPT/3 refusal cause: every cause is an own-content
    // judgment, never upstream churn.
    assert_eq!(CampaignRefusalCause::ALL.len(), 3);
    let spellings: Vec<&str> = CampaignRefusalCause::ALL
        .iter()
        .map(|c| c.spelling())
        .collect();
    assert_eq!(
        spellings,
        [
            "compile-refusal",
            "witness-differential-kill",
            "unrealized-obligation-open",
        ]
    );
}

#[test]
fn campaign_escalation_reason_inventory_is_frozen() {
    // The typed RECEIPT/3 escalation reason; the escalation disposition is
    // always ArchitectRequired.
    assert_eq!(CampaignEscalationReason::ALL.len(), 3);
    let spellings: Vec<&str> = CampaignEscalationReason::ALL
        .iter()
        .map(|r| r.spelling())
        .collect();
    assert_eq!(
        spellings,
        [
            "law-changing-candidate",
            "judge-defect",
            "proof-policy-conflict",
        ]
    );
    assert!(CampaignTerminal::ArchitectRequired.remains_in_denominator());
}

#[test]
fn frontier_and_freshness_inventories_are_frozen() {
    // E7 four-state frontier law: one narrow question (admission into the
    // current trusted frontier), four answers.
    assert_eq!(FrontierState::ALL.len(), 4);
    let spellings: Vec<&str> = FrontierState::ALL.iter().map(|s| s.spelling()).collect();
    assert_eq!(
        spellings,
        ["Qualified", "BlockedByDependency", "Unqualified", "Invalidated"]
    );
    // Blocked, unqualified, and invalidated are different laws: a blocked
    // candidate's fault lies upstream, an unqualified one is not admitted on
    // its own posture, and an invalidated one once WAS admissible.
    assert_ne!(FrontierState::BlockedByDependency, FrontierState::Unqualified);
    assert_ne!(FrontierState::Unqualified, FrontierState::Invalidated);
    assert_eq!(EvidenceFreshness::ALL.len(), 3);
    assert_eq!(
        EvidenceFreshness::StaleByJudgeChange.spelling(),
        "StaleByJudgeChange"
    );
    assert_eq!(CampaignClosureEdgeKind::ALL.len(), 4);
}

#[test]
fn mini_supernova_profile_rows_are_active_in_the_proof_catalog() {
    // TL-10: the campaign evidence profile names ACTIVE proof rows only, each
    // exactly once — a rehearsal cannot claim to realize a row the catalog
    // does not carry alive.
    assert_eq!(MINI_SUPERNOVA_PROFILE.id, "mini-supernova-rehearsal/1");
    assert_eq!(MINI_SUPERNOVA_PROFILE.realized_rows.len(), 9);
    for (index, row) in MINI_SUPERNOVA_PROFILE.realized_rows.iter().enumerate() {
        assert!(
            !MINI_SUPERNOVA_PROFILE.realized_rows[..index].contains(row),
            "{} is named twice by the profile",
            row.raw()
        );
        let active = PROOF_ROWS.iter().any(|record| {
            record.id.raw() == row.raw()
                && matches!(record.state, ProofRowState::Active { .. })
        });
        assert!(active, "{} is not an Active proof identity", row.raw());
    }
}

#[test]
fn closure_node_projects_the_blocked_downstream_fact() {
    // The rehearsal's downstream candidate: locally green, unable to promote
    // above the red dependency — visible in the closure view as blocked with
    // no terminal, its evidence stale by the upstream change.
    let node = CampaignClosureNode {
        candidate: CandidateId::from_digest(digest(0xcc)),
        realization_posture: RealizationPosture::Candidate,
        frontier: FrontierState::BlockedByDependency,
        terminal: None,
        freshness: EvidenceFreshness::StaleByDependencyChange,
    };
    assert_eq!(node.frontier, FrontierState::BlockedByDependency);
    assert_eq!(node.terminal, None);
}

#[test]
fn frozen_judge_snapshot_binds_the_exact_judge_digest() {
    let snapshot = FrozenJudgeSnapshot { judge_root_digest: digest(0xdd) };
    let same = FrozenJudgeSnapshot { judge_root_digest: digest(0xdd) };
    let mutated = FrozenJudgeSnapshot { judge_root_digest: digest(0xde) };
    assert_eq!(snapshot, same);
    // A mutated judge is a DIFFERENT snapshot: evidence bound to the old
    // digest cannot verify against it.
    assert_ne!(snapshot, mutated);
}

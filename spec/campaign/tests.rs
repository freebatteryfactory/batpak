use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::bootstrap_qualification::{HexParseError, Sha256Digest};
use crate::sprouting::{CandidateChangeClass, CandidateOriginKind, RealizationPosture};

use super::*;

fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}

fn hex64(byte: u8) -> String {
    digest(byte).render()
}

/// A fully populated record: a repair child naming its failed parent, one
/// dependency, one evidence artifact, one receipt per role, and a promoted
/// terminal.
fn full_record() -> CandidateRecord {
    CandidateRecord {
        id: CandidateId::from_digest(digest(0x11)),
        parents: vec![CandidateId::from_digest(digest(0x22))],
        source_frontier_commitment: digest(0x33),
        dependency_commitments: vec![DependencyCommitment {
            dependency: CandidateId::from_digest(digest(0x44)),
            commitment: digest(0x55),
        }],
        content_commitment: digest(0x66),
        origin: CandidateOriginKind::RepairOfCandidate,
        change_class: CandidateChangeClass::RealizationPreserving,
        realization_posture: RealizationPosture::Candidate,
        generator_or_pilot_evidence: vec![EvidenceRef::from_digest(digest(0x77))],
        qualification_receipts: vec![ReceiptRef::from_digest(digest(0x88))],
        holdout_receipts: vec![ReceiptRef::from_digest(digest(0x99))],
        reuse_key: ReuseKey::from_digest(digest(0xaa)),
        terminal: Some(CampaignTerminalReceipt {
            terminal: CampaignTerminal::Promoted,
            receipt: ReceiptRef::from_digest(digest(0xbb)),
        }),
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
        Some("BATPAK-CANDIDATE-MANIFEST/1")
    );
    assert_eq!(CandidateManifest::MAGIC, "BATPAK-CANDIDATE-MANIFEST/1");
}

#[test]
fn manifest_renders_the_exact_documented_field_order() {
    let rendered = CandidateManifest { record: full_record() }.render();
    let expected = alloc::format!(
        "BATPAK-CANDIDATE-MANIFEST/1\n\
         candidate-id {id}\n\
         parent-count 1\n\
         parent {parent}\n\
         source-frontier-commitment {frontier}\n\
         dependency-count 1\n\
         dependency {dep} {depc}\n\
         content-commitment {content}\n\
         origin RepairOfCandidate\n\
         change-class RealizationPreserving\n\
         realization-posture Candidate\n\
         evidence-count 1\n\
         evidence {evidence}\n\
         qualification-receipt-count 1\n\
         qualification-receipt {qual}\n\
         holdout-receipt-count 1\n\
         holdout-receipt {holdout}\n\
         reuse-key {reuse}\n\
         terminal Promoted {term}\n",
        id = hex64(0x11),
        parent = hex64(0x22),
        frontier = hex64(0x33),
        dep = hex64(0x44),
        depc = hex64(0x55),
        content = hex64(0x66),
        evidence = hex64(0x77),
        qual = hex64(0x88),
        holdout = hex64(0x99),
        reuse = hex64(0xaa),
        term = hex64(0xbb),
    );
    assert_eq!(rendered, expected);
}

#[test]
fn manifest_states_empty_sets_and_absent_terminal_explicitly() {
    let mut record = full_record();
    record.parents = Vec::new();
    record.dependency_commitments = Vec::new();
    record.generator_or_pilot_evidence = Vec::new();
    record.qualification_receipts = Vec::new();
    record.holdout_receipts = Vec::new();
    record.origin = CandidateOriginKind::DeterministicGeneration;
    record.realization_posture = RealizationPosture::Scaffold;
    record.terminal = None;
    let rendered = CandidateManifest { record }.render();
    // An empty set is stated, never omitted; an absent terminal is `none`,
    // never a missing line.
    assert!(rendered.contains("parent-count 0\n"));
    assert!(rendered.contains("dependency-count 0\n"));
    assert!(rendered.contains("evidence-count 0\n"));
    assert!(rendered.contains("qualification-receipt-count 0\n"));
    assert!(rendered.contains("holdout-receipt-count 0\n"));
    assert!(rendered.contains("realization-posture Scaffold\n"));
    assert!(rendered.ends_with("terminal none\n"));
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
fn frontier_and_freshness_inventories_are_frozen() {
    assert_eq!(FrontierState::ALL.len(), 3);
    assert_eq!(FrontierState::BlockedByDependency.spelling(), "BlockedByDependency");
    // Blocked and invalidated are different laws: a blocked candidate awaits
    // its dependency; an invalidated one must requalify.
    assert_ne!(FrontierState::BlockedByDependency, FrontierState::Invalidated);
    assert_eq!(EvidenceFreshness::ALL.len(), 3);
    assert_eq!(
        EvidenceFreshness::StaleByJudgeChange.spelling(),
        "StaleByJudgeChange"
    );
    assert_eq!(CampaignClosureEdgeKind::ALL.len(), 4);
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

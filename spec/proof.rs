//! Proof-unit vocabulary (5.5E1 ruling; extended by the 5.5E2 bake).
//!
//! docs/12 owns the audited-denominator doctrine in prose; this file is the
//! typed authority for the closed vocabularies that doctrine depends on. The
//! sibling eight-state `MutationResult` earned a typed owner long before this
//! file existed; the general proof denominator now gets the same treatment.

/// What a proof unit concluded ABOUT THE LAW. Semantic terminals only: what
/// happened to an execution attempt (not executed, timed out, infrastructure
/// failed) is the qualification-receipt algebra's vocabulary, and the two
/// axes never collapse into one enum — a row whose attempt died has NO
/// semantic terminal and stays honestly unterminated in the denominator. A
/// subject violating its own deadline law is `Failed` with that law cited; a
/// harness timeout is a receipt-stage fact carrying no semantic verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofUnitTerminal {
    /// The obligation held under the executed proof.
    Passed,
    /// The obligation was violated; the evidence names the boundary.
    Failed,
    /// The proof refused to run its subject (illegal input, unmet
    /// precondition it exists to enforce). A refusal is a verdict.
    Refused,
    /// The platform or profile cannot express the obligation; stated, never
    /// silently skipped.
    Unsupported,
    /// An authorized policy skipped the proof; the receipt proving that
    /// authority accompanies the terminal.
    SkippedWithAuthority,
    /// The proof's freshness bound lapsed; its verdict no longer counts.
    Expired,
    /// A successor proof row owns the obligation now.
    Superseded,
}

/// Every terminal, in declaration order. Completeness is enforced by
/// seedcheck's exhaustive classification.
pub const PROOF_UNIT_TERMINALS: &[ProofUnitTerminal] = &[
    ProofUnitTerminal::Passed,
    ProofUnitTerminal::Failed,
    ProofUnitTerminal::Refused,
    ProofUnitTerminal::Unsupported,
    ProofUnitTerminal::SkippedWithAuthority,
    ProofUnitTerminal::Expired,
    ProofUnitTerminal::Superseded,
];

impl ProofUnitTerminal {
    /// Whether this terminal may count toward the green side of the audited
    /// denominator. Exhaustive: a new terminal must be classified here, not
    /// defaulted — an unclassified proof unit cannot silently count green
    /// (SEED-AUDITED-DENOMINATOR).
    pub const fn counts_green(self) -> bool {
        match self {
            ProofUnitTerminal::Passed => true,
            ProofUnitTerminal::Failed
            | ProofUnitTerminal::Refused
            | ProofUnitTerminal::Unsupported
            | ProofUnitTerminal::SkippedWithAuthority
            | ProofUnitTerminal::Expired
            | ProofUnitTerminal::Superseded => false,
        }
    }
}

/// A proof unit's stable identity (5.5E2). Sealed like the per-family
/// guarantee ids: the spec's own registry authors identities, a consumer
/// reads one through `raw()`.
///
/// Execution completeness and inventory retention are DIFFERENT AXES. A
/// libtest count proves every currently declared test executed and no filter
/// hid one; it cannot prove a required identity was not deleted, because
/// deleting a test shrinks the expected count and the executed count
/// together. Typed identity plus the migration registry below is the
/// retention half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowId(pub(crate) &'static str);

impl ProofRowId {
    pub const fn raw(self) -> &'static str {
        self.0
    }
}

/// The lifecycle of a proof identity. Retirement PRESERVES successor
/// identity: a rename names its one successor, a split names every
/// successor, and the original claim must be carried by the conjunction of
/// the successors — retirement is supersession with a forwarding address,
/// never deletion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofRowState {
    /// The identity is authoritative in the proof inventory.
    Active,
    /// The identity is retired; the successors own its obligation now. An
    /// active witness may reach a retired row only by explicitly following
    /// this mapping.
    Retired { successors: &'static [ProofRowId] },
}

/// One entry in the proof-identity migration registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowMigration {
    pub id: ProofRowId,
    pub state: ProofRowState,
}

/// Every proof-row identity that ever stopped being authoritative, with its
/// successors. This registry is the TYPED OWNER the retired-name scanner
/// derives from: until the 5.5E2 bake it existed only as a Python dict
/// inside the auditor, and the dict was already missing an entry — docs/24
/// retired `test_local_fixture_type_is_classified_correctly` in a migration
/// note the registry never learned, so the scanner guarded five of the six
/// retirements while claiming to guard them all. A registry the auditor
/// owns is a registry the auditor cannot be caught neglecting.
///
/// Identities here are RETIRED entries only: the active inventory lives in
/// docs/24 until the documentary convergence pass lifts it, and every
/// successor named below must either appear there or carry its own
/// retirement entry.
pub const PROOF_ROW_MIGRATIONS: &[ProofRowMigration] = &[
    ProofRowMigration {
        id: ProofRowId("pre_shred_keyset_restore_is_rejected"),
        state: ProofRowState::Retired {
            successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")],
        },
    },
    ProofRowMigration {
        id: ProofRowId("shredded_and_keyset_missing_remain_distinct"),
        state: ProofRowState::Retired {
            successors: &[ProofRowId(
                "shredded_unavailable_and_keyset_missing_remain_distinct",
            )],
        },
    },
    ProofRowMigration {
        id: ProofRowId("snapshot_and_fork_exclude_keys_by_default"),
        state: ProofRowState::Retired {
            successors: &[ProofRowId(
                "snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys",
            )],
        },
    },
    ProofRowMigration {
        id: ProofRowId("hash_map_iteration_cannot_change_canonical_bytes"),
        state: ProofRowState::Retired {
            successors: &[ProofRowId(
                "hash_map_iteration_cannot_influence_canonical_observables",
            )],
        },
    },
    ProofRowMigration {
        id: ProofRowId("attempt_receipt_cannot_cross_invocation_classes"),
        state: ProofRowState::Retired {
            successors: &[
                ProofRowId("entrypoint_receipt_cannot_satisfy_query_program_execution"),
                ProofRowId("query_program_receipt_cannot_satisfy_entrypoint_invocation"),
            ],
        },
    },
    ProofRowMigration {
        id: ProofRowId("test_local_fixture_type_is_classified_correctly"),
        state: ProofRowState::Retired {
            successors: &[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")],
        },
    },
];

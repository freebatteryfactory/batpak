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

/// One entry in the proof-identity catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRowRecord {
    pub id: ProofRowId,
    pub state: ProofRowState,
}

/// The COMPLETE proof-identity catalog (5.5E2j): every canonical active
/// proof-row identity docs/24 declares, in document order, followed by every
/// identity that ever stopped being authoritative. The catalog owns identity,
/// lifecycle, and succession ONLY — expectation prose, proof meaning, and
/// receipt details stay in docs/24 until the documentary convergence pass.
///
/// This replaced PROOF_ROW_MIGRATIONS, which held retirements alone and
/// validated successors by substring search over the whole of docs/24 — so a
/// successor whose canonical active row was deleted still "existed" as long
/// as its name survived in a migration note or a prose paragraph. A
/// forwarding address must point at the living census, not at a name painted
/// on an old wall: successor resolution and the active/docs equality laws
/// now run against this catalog and the structurally parsed canonical rows.
pub const PROOF_ROWS: &[ProofRowRecord] = &[
    ProofRowRecord { id: ProofRowId("middle_event_deletion_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("event_reorder_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("duplicate_payload_splice_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("cross_lane_predecessor_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("cross_entity_predecessor_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("midstream_genesis_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("forged_index_row_cannot_choose_and_authenticate_bytes"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("forged_sibling_cannot_cause_false_loss"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("agreeing_truncated_table_cannot_cause_false_safety"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("derived_row_cannot_authenticate_siblings"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("derived_row_cannot_authenticate_table_count"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("derived_row_cannot_authenticate_order"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("derived_row_cannot_authenticate_tail_boundary"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("derived_row_cannot_prove_absence_or_loss"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("page_limit_bounds_discovery_work_not_only_output"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("allocation_does_not_scale_with_full_matched_set"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("descriptor_postcondition_failure_is_not_reported_applied"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("fcntl_getfd_failure_fails_closed"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("fcntl_setfd_failure_fails_closed"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("close_reopen_reimport_returns_zero_new_events"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("signed_compaction_preserves_or_commits_the_removed_set"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compaction_inputs_survive_until_replacement_evidence_is_durable"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("matched_kind_decode_failure_is_a_typed_terminal"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("replay_routes_agree_on_first_failure_and_class"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("trapping_host_filesystem_remains_unreached_under_injected_storage"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("paired_result_and_receipt_share_one_turn_evaluation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("hlc_cannot_substitute_for_commit_sequence"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("receipt_binds_chronology_and_commit_without_collapsing_them"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("replay_reconstructs_same_turn_and_returns_original_receipts"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("lost_acknowledgement_requires_reconciliation_before_retry"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("port_response_cannot_cross_attempts"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("driver_await_and_cooperative_drive_produce_equivalent_logical_trace"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("checkpoint_gap_does_not_duplicate_committed_effect"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("reconciliation_appends_evidence_without_rewriting_original_observation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("shred_ack_waits_for_backend_durability"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("crash_before_durable_key_delete_does_not_report_shred_success"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("reopen_after_ack_cannot_recover_shredded_plaintext"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("shred_transition_binding_mismatch_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("foreign_keyset_generation_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("external_key_backend_preserves_shred_semantics"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compressed_fbat_authority_frame_is_rejected_in_v1"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compressed_vpak_semantic_section_is_rejected_in_v1"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compression_profile_requires_expansion_bound"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("future_format_version_fails_with_its_own_typed_disposition"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("program_image_id_is_independent_of_compiler_provenance"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("distinct_version_types_do_not_typecheck_when_substituted"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("netbat_version_does_not_upgrade_pakvm_isa"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("order_by_hlc_uses_commit_tiebreak"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("cross_store_hlc_order_is_total"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("hlc_range_is_half_open"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("frontier_progress_never_uses_hlc_order"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("observed_wall_time_is_not_promoted_to_hlc"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("no_std_batpak_has_no_std_dependency_route"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("no_std_syncbat_has_no_std_dependency_route"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("default_std_does_not_enable_threaded_or_browser_adapters"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("browser_and_native_profiles_preserve_program_semantics"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("hash_map_iteration_cannot_influence_canonical_observables"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("attacker_length_is_checked_before_reserve"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("allocation_failure_returns_resource_exhausted"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("declared_work_overrun_returns_budget_exceeded"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("resource_exhaustion_never_publishes_partial_event"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("resource_exhaustion_never_advances_checkpoint"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("artifact_staging_failure_publishes_no_artifact_ref"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("pakvm_arena_exhaustion_returns_typed_terminal_disposition"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("kernel_cannot_resolve_by_display_name"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("kernel_contract_and_implementation_ids_are_not_interchangeable"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("qualified_interface_requires_matching_qualification_receipt"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("exact_kernel_binding_rejects_another_implementation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("attempt_receipt_records_exact_kernel_implementation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("local_domain_type_inside_function_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("local_domain_type_inside_closure_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("test_local_nonsemantic_fixture_type_is_allowed"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("production_expect_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("test_expect_with_context_is_allowed"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("unledgered_unsafe_fn_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("unledgered_pointer_cast_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("public_flume_receiver_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("hash_map_iteration_in_canonical_encoder_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("drawer_module_name_requires_explicit_disposition"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("raw_batql_is_not_a_netbat_invocation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compiler_port_is_not_available_to_guest_programs"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compiler_port_is_absent_from_default_server_profile"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("compiler_output_still_requires_program_validation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_with_effect_instruction_is_rejected"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_cannot_install_process"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_cannot_request_write_capability"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_over_work_bound_is_denied_before_execution"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_result_binds_program_and_grant_identity"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("entrypoint_receipt_cannot_satisfy_query_program_execution"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("entrypoint_cannot_substitute_foreign_program_image"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("entrypoint_effect_must_be_declared_by_world_interface"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("query_program_receipt_cannot_satisfy_entrypoint_invocation"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("tls_does_not_upgrade_program_or_proof_authority"), state: ProofRowState::Active },
    ProofRowRecord { id: ProofRowId("pre_shred_keyset_restore_is_rejected"), state: ProofRowState::Retired { successors: &[ProofRowId("stale_or_pre_shred_keyset_restore_is_rejected")] } },
    ProofRowRecord { id: ProofRowId("shredded_and_keyset_missing_remain_distinct"), state: ProofRowState::Retired { successors: &[ProofRowId("shredded_unavailable_and_keyset_missing_remain_distinct")] } },
    ProofRowRecord { id: ProofRowId("snapshot_and_fork_exclude_keys_by_default"), state: ProofRowState::Retired { successors: &[ProofRowId("snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys")] } },
    ProofRowRecord { id: ProofRowId("hash_map_iteration_cannot_change_canonical_bytes"), state: ProofRowState::Retired { successors: &[ProofRowId("hash_map_iteration_cannot_influence_canonical_observables")] } },
    ProofRowRecord { id: ProofRowId("attempt_receipt_cannot_cross_invocation_classes"), state: ProofRowState::Retired { successors: &[ProofRowId("entrypoint_receipt_cannot_satisfy_query_program_execution"), ProofRowId("query_program_receipt_cannot_satisfy_entrypoint_invocation")] } },
    ProofRowRecord { id: ProofRowId("test_local_fixture_type_is_classified_correctly"), state: ProofRowState::Retired { successors: &[ProofRowId("test_local_nonsemantic_fixture_type_is_allowed")] } },
];

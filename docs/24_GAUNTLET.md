---
status: AUTHORITATIVE
contract_id: BP-GAUNTLET-1
authority_scope: proof economy, hostile closure, mutation, fuzzing, benchmarking, and release proof
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Gauntlet

## Typed proof ownership

`spec/proof/` owns ProofRowId identity, active or retired lifecycle, succession, guarantee binding, and projection membership. docs/24 owns each active row's authoritative semantic summary, expected observation, and terminal disposition. A generated relation cannot change proof meaning. A prose meaning cannot silently move a proof row to another guarantee.

<!-- PROOF-RELATIONS:BEGIN generated from spec/proof/inventory.rs by bootstrap/project.py; do not edit -->
| Guarantee | Projection contract(s) | Active proof rows |
| --- | --- | --- |
| LEG-023 | BP-STORAGE-TILES-1 | middle_event_deletion_is_rejected; event_reorder_is_rejected; duplicate_payload_splice_is_rejected; cross_lane_predecessor_is_rejected; cross_entity_predecessor_is_rejected; midstream_genesis_is_rejected; forged_index_row_cannot_choose_and_authenticate_bytes |
| LEG-019 | BP-STORAGE-TILES-1 | forged_sibling_cannot_cause_false_loss; agreeing_truncated_table_cannot_cause_false_safety; derived_row_cannot_authenticate_siblings; derived_row_cannot_authenticate_table_count; derived_row_cannot_authenticate_order; derived_row_cannot_authenticate_tail_boundary; derived_row_cannot_prove_absence_or_loss |
| LEG-028 | BP-STORAGE-TILES-1 | page_limit_bounds_discovery_work_not_only_output |
| LEG-020 | BP-TESTPAK-1 | allocation_does_not_scale_with_full_matched_set |
| LEG-043 | BP-BVISOR-1 | descriptor_postcondition_failure_is_not_reported_applied; fcntl_getfd_failure_fails_closed; fcntl_setfd_failure_fails_closed |
| LEG-074 | BP-MIGRATION-1 | close_reopen_reimport_returns_zero_new_events |
| LEG-085 | BP-STORAGE-TILES-1 | signed_compaction_preserves_or_commits_the_removed_set; compaction_inputs_survive_until_replacement_evidence_is_durable |
| LEG-086 | BP-STORAGE-TILES-1 | matched_kind_decode_failure_is_a_typed_terminal; replay_routes_agree_on_first_failure_and_class |
| LEG-087 | BP-STORAGE-TILES-1 | trapping_host_filesystem_remains_unreached_under_injected_storage |
| DEC-075 | BP-SYSTEM-MODEL-1 | paired_result_and_receipt_share_one_turn_evaluation; hlc_cannot_substitute_for_commit_sequence; receipt_binds_chronology_and_commit_without_collapsing_them; replay_reconstructs_same_turn_and_returns_original_receipts; lost_acknowledgement_requires_reconciliation_before_retry; port_response_cannot_cross_attempts; driver_await_and_cooperative_drive_produce_equivalent_logical_trace; checkpoint_gap_does_not_duplicate_committed_effect; reconciliation_appends_evidence_without_rewriting_original_observation |
| LEG-081 | BP-CRYPTO-SECRET-1 | shred_ack_waits_for_backend_durability; crash_before_durable_key_delete_does_not_report_shred_success; reopen_after_ack_cannot_recover_shredded_plaintext; shred_transition_binding_mismatch_is_rejected; stale_or_pre_shred_keyset_restore_is_rejected; foreign_keyset_generation_is_rejected; shredded_unavailable_and_keyset_missing_remain_distinct; snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys; external_key_backend_preserves_shred_semantics |
| LEG-053 | BP-STORAGE-TILES-1 | compressed_fbat_authority_frame_is_rejected_in_v1; compressed_vpak_semantic_section_is_rejected_in_v1; compression_profile_requires_expansion_bound |
| DEC-063 | BP-STORAGE-TILES-1 | future_format_version_fails_with_its_own_typed_disposition |
| DEC-064 | BP-IDENTITY-TIME-NAV-1 | program_image_id_is_independent_of_compiler_provenance; distinct_version_types_do_not_typecheck_when_substituted; netbat_version_does_not_upgrade_pakvm_isa |
| DEC-061 | BP-IDENTITY-TIME-NAV-1 | order_by_hlc_uses_commit_tiebreak; cross_store_hlc_order_is_total; hlc_range_is_half_open; frontier_progress_never_uses_hlc_order; observed_wall_time_is_not_promoted_to_hlc |
| DEC-065 | BP-REPOSITORY-PACKAGES-1 | no_std_batpak_has_no_std_dependency_route; no_std_syncbat_has_no_std_dependency_route; default_std_does_not_enable_threaded_or_browser_adapters; browser_and_native_profiles_preserve_program_semantics |
| DEC-065 | BP-TYPE-SYSTEM-1 | hash_map_iteration_cannot_influence_canonical_observables |
| DEC-066 | BP-SYNCBAT-1 | attacker_length_is_checked_before_reserve; allocation_failure_returns_resource_exhausted; declared_work_overrun_returns_budget_exceeded; resource_exhaustion_never_publishes_partial_event; resource_exhaustion_never_advances_checkpoint; artifact_staging_failure_publishes_no_artifact_ref; pakvm_arena_exhaustion_returns_typed_terminal_disposition |
| DEC-062 | BP-WORLD-PORTS-1 | kernel_cannot_resolve_by_display_name; kernel_contract_and_implementation_ids_are_not_interchangeable; qualified_interface_requires_matching_qualification_receipt; exact_kernel_binding_rejects_another_implementation; attempt_receipt_records_exact_kernel_implementation |
| DEC-068 | BP-TESTPAK-1 | local_domain_type_inside_function_is_rejected; local_domain_type_inside_closure_is_rejected; test_local_nonsemantic_fixture_type_is_allowed; production_expect_is_rejected; test_expect_with_context_is_allowed; unledgered_unsafe_fn_is_rejected; unledgered_pointer_cast_is_rejected; public_flume_receiver_is_rejected; hash_map_iteration_in_canonical_encoder_is_rejected; drawer_module_name_requires_explicit_disposition |
| DEC-051 | BP-WORLD-PORTS-1 | raw_batql_is_not_a_netbat_invocation; compiler_port_is_not_available_to_guest_programs; compiler_port_is_absent_from_default_server_profile; compiler_output_still_requires_program_validation |
| DEC-050 | BP-WORLD-PORTS-1 | query_program_with_effect_instruction_is_rejected; query_program_cannot_install_process; query_program_cannot_request_write_capability; query_program_over_work_bound_is_denied_before_execution; query_program_result_binds_program_and_grant_identity; entrypoint_receipt_cannot_satisfy_query_program_execution |
| DEC-049 | BP-WORLD-PORTS-1 | entrypoint_cannot_substitute_foreign_program_image; entrypoint_effect_must_be_declared_by_world_interface; query_program_receipt_cannot_satisfy_entrypoint_invocation |
| LEG-045 | BP-WORLD-PORTS-1 | tls_does_not_upgrade_program_or_proof_authority |
| DEC-079 | BP-SPROUTING-1 | candidate_origin_confers_no_authority; scaffold_cannot_close_realization; qualified_nursery_record_is_not_promoted_source |
| DEC-080 | BP-SPROUTING-1 | law_changing_candidate_cannot_enter_realization_preserving_lane; bounded_generated_trace_attack_holds_boundary; every_admitted_campaign_obligation_reaches_a_terminal |
| DEC-081 | BP-SPROUTING-1 | search_budget_is_declared_and_receipted; search_and_holdout_sets_are_disjoint; failed_holdout_candidate_is_quarantined; planted_semantic_mutant_is_activated_and_killed; deterministic_replay_equality_of_simulated_trace; candidate_search_terminates_within_declared_budget |
| DEC-082 | BP-SPROUTING-1 | candidate_cannot_modify_frozen_judge; later_green_cannot_bless_unresolved_dependency; bounded_repair_loop_reaches_stable_qualified_frontier |
| DEC-073 | BP-SPROUTING-1 | specialized_plan_benchmark_is_advisory_only |
| DEC-078 | BP-SPROUTING-1 | runtime_capture_appends_observed_history_without_rewrite; offline_replay_concludes_conformance_for_captured_history |
| DEC-076 | BP-SPROUTING-1 | confirming_rerun_changes_no_authoritative_result |
<!-- PROOF-RELATIONS:END -->

## Purpose

The gauntlet proves that BatPak's claims survive hostile inputs, faults, schedules, stale artifacts, omissions, and self-certification traps. It does not merely count green tests.

## Command tiers

```text
Tier 0  seedcheck, format, structural facts, fast unit/model checks
Tier 1  package conformance and differential tests
Tier 2  deterministic schedules, crash matrices, semantic fuzz, SemanticIr/SelectableCompiled mutation
Tier 3  kernel mutation, cross-target adapters, sustained workload envelopes
Tier 4  release seal, compatibility corpus, proof freshness, package audit
```

## Proof density

Every critical claim names owner, clause, witness, negative fixture, mutation seam, corpus, freshness rule, and receipt. Large test counts do not compensate for a missing trust-boundary witness.

## Trust closure

The gauntlet asks whether the proof system itself can be fooled:

```text
fixture never activated
mutant never reached
expected result generated by implementation under test
stale corpus accepted
zero tests counted green
manifest omits the actual boundary
benchmark verdict hardcoded
cache verifies itself
```

Planted red cases must demonstrate that the gate bites.

## Rule qualification

A structural rule is accepted only after a conforming fixture passes, a violating fixture fails for the intended reason, the scanner/parser limitation is documented, and future syntax cannot silently evade it.

## Mutation lanes

Exactly three, frozen (DEC-015). The typed names are `MutationLane` in `spec/mutation/types.rs`; the contract, result algebra, activation evidence, and promotion law live in `12_TESTPAK.md`. The promotion denominator's `QualifiedHostileEvidence` requirement consumes this document's Rule qualification law; the requirement inventory itself is owned by `spec/promotion/types.rs` and is not restated here.

### Semantic IR (`MutationLane::SemanticIr`)

Mutate Contract IR, schemas, availability rules, BatQL AST, ProgramImage, decision nodes, query plans, capabilities, and proof facts without compiling Rust. The reference interpreter executes the mutant; an independent evidence route judges it.

### Generated selectable (`MutationLane::SelectableCompiled`)

Compile generated alternatives once per shard and select one mutation site at runtime. Activation is observed, never assumed. Slots live only in a test or mutation profile.

### Kernel source (`MutationLane::CompilerBacked`)

Use compiler-backed mutation only for narrow handwritten mechanisms such as codec, journal recovery, concurrency, crypto-domain plumbing, or optimized tile kernels. Candidates run under real rustc semantics.

## Semantic fuzzing

Generate valid semantic objects, encode them, make one coherent mutation while preserving framing, and compare production disposition with an independent model. Persist minimized semantic counterexamples with contract/version identity.

## Schedule and crash proof

Explore bounded interleavings for admission, writer drain, completion, checkpoint, idempotency, visibility, compaction, port suspension, attempt response, and shutdown. Crash points cover publication boundaries and ambiguous external outcomes.

## Complexity and performance

Work counts, allocation counts, bytes touched, scaling exponent, fallback, and logical high-water are primary. Wall-time distributions supplement physical evidence. Plant slower-complexity cases to prove the gate is anti-vacuous. The repository `bench` command runs under this contract: physical measurement supplements the logical work counts, never replaces them.

## Cross-target qualification

Native, browser, embedded, and in-memory adapters qualify only the guarantees they implement. One platform's proof does not silently bless another.

## Release seal

The release seal binds the typed `ReleaseSealField` inventory
(`spec/release/`; full projection in `36_PUBLIC_API_CI_AND_RELEASE.md`,
DEC-058). The gauntlet supplies its `TestDispositions`, `MutationDispositions`,
`FuzzDispositions`, `BenchmarkDispositions`, and `ProofFreshness` inputs; this
document restates no second copy of the list.

A release with an unclassified denominator row is refused.

## Numeric proof obligations (DEC-069 / docs/37)

The numeric proof families declared in `37_NUMERIC_SEMANTICS_AND_AUTHORITY.md` section 13 are gauntlet obligations: decimal-to-coefficient exactness, exact-ratio canonicalization, dimensional legality, raw-bit preservation, finite dyadic decomposition, NaN and infinity refusals, interval well-formedness and the six IntervalDecision truth tables, quantization containment and receipt completeness, rounding-boundary and mode-specific laws, and wide-exact-to-Fixed128 conversion. Their executable TestPak and Muterprater ownership lands in 5.5D (`12_TESTPAK.md`). Bootstrap enforces the numeric contract only; it never evaluates or quantizes actual numbers.

## Specialization proof square (DEC-073)

Future TestPak qualification owns this square; 5.5D2 records the obligation and its ownership only. Bootstrap does not execute it.

```text
1. interpret the original admitted program
2. specialize the original admitted program and execute the residual
3. mutate the semantic program and interpret the semantic mutant
4. mutate the semantic program, specialize it, and execute the residual mutant
```

Required relationships:

```text
reference(original)        == residual(original)
reference(semantic mutant) == residual(semantic mutant)
```

Where a residual-level mutation corresponds to a semantic mutation, the residual mutation route agrees with the specialize-the-semantic-mutant route. This proves the specialization transformation is meaning preserving. It does not replace the independent semantic oracle (SEED-INDEPENDENT-ORACLE).

Execution routes bind to the mutation lanes (DEC-015):

```text
semantic mutation                                    Lane SemanticIr
selectable residual or generated route mutation      Lane SelectableCompiled
handwritten specializer, cache, kernel, mechanism    Lane CompilerBacked
```

No lane is the sole proof route. A semantic mutant that changes meaning must be observable through the reference path and the specialized path. A residual-only mutation either maps to a semantic counterpart or is classified as an implementation/mechanism mutation; it never pretends to be a semantic mutant.

A specialized-plan candidate is admitted under DEC-073's realization-preserving policy; its benchmark evidence is advisory, never a gate. `docs/07_PAKVM_ISA.md` owns the specialization law; this row owns the executable meaning that a specialized plan's performance envelope can neither block promotion nor stand in for the reference interpreter's semantic verdict.


Authoritative meanings:

```text
specialized_plan_benchmark_is_advisory_only
    A SpecializedPlan's benchmark envelope is advisory. Deleting every
    specialization cache changes performance and WorkObservation only, never
    meaning, authority, proof posture, effect legality, or receipt semantics,
    so a residual that runs slower than its declared envelope raises a
    visible advisory and blocks nothing, and the reference interpreter — not
    the fastest kernel — remains the semantic oracle.
    expects: a specialized plan whose measured work exceeds its declared
      benchmark envelope surfaces an advisory finding that gates no merge,
      promotion, or release, while the reference interpreter and the
      specialized residual return identical observables
    disposition: an advisory benchmark record that stays visible and never
      blocks; a benchmark result presented as a correctness or promotion
      verdict fails the witness
```

## Integrity and predecessor witnesses (LEG-023)

`spec/proof/` owns proof-row identity, lifecycle, succession, guarantee binding, and projection membership; this document owns each active row's semantic summary, expected observation, and terminal disposition. `21_LEGACY_SEMANTIC_OBLIGATIONS.md` projects the required witness IDs for each legacy obligation from the typed relations; that projection is audited but is not a second authority. A witness meaning is changed here, never there.


Authoritative meanings:

```text
middle_event_deletion_is_rejected
    Removing a middle event cannot make the surviving neighbours verify as
    one intact chain.
    expects: verifying a chain with one middle event removed terminates in a
      typed refusal at the broken predecessor linkage; the surviving
      neighbours never verify as one intact chain
    disposition: a typed chain-verification failure; no acceptance of the
      spliced remainder

event_reorder_is_rejected
    Individually valid events in the wrong immediate-predecessor order fail
    verification.
    expects: individually valid events presented out of immediate-predecessor
      order fail verification at the first out-of-order commitment
    disposition: a typed predecessor-mismatch refusal; no reordered stream is
      reported verified

duplicate_payload_splice_is_rejected
    Matching payload bytes or ContentDigest cannot substitute a different
    EventCommitment or predecessor position.
    expects: an event whose payload bytes or ContentDigest match but whose
      EventCommitment or predecessor position differs fails verification at
      that event
    disposition: a typed commitment-mismatch refusal; payload equality never
      substitutes for commitment identity

cross_lane_predecessor_is_rejected
    A predecessor from another lane cannot satisfy the expected stream
    predecessor. Lane isolation itself is owned by LEG-050.
    expects: an event naming a predecessor from another lane fails
      verification at the crossing event
    disposition: a typed predecessor-mismatch refusal; lane-isolation law
      itself stays owned by LEG-050

cross_entity_predecessor_is_rejected
    A predecessor from another entity cannot satisfy the expected stream
    predecessor. Lane isolation itself is owned by LEG-050.
    expects: an event naming a predecessor from another entity fails
      verification at the crossing event
    disposition: a typed predecessor-mismatch refusal; entity-isolation law
      itself stays owned by LEG-050

midstream_genesis_is_rejected
    A genesis marker cannot reset an already-started stream. Dense visible
    linearization itself is owned by LEG-067.
    expects: a genesis marker appearing after a stream has started fails
      verification at that marker and resets nothing
    disposition: a typed refusal naming the illegal genesis position; the
      established prefix keeps its own verification state

forged_index_row_cannot_choose_and_authenticate_bytes
    Derived metadata cannot select authority bytes and bootstrap verification
    from the selection it supplied.
    expects: verification steered by a forged index row terminates in a typed
      refusal or verifies only against authoritative bytes the row did not
      select; the row's own selection never becomes the verified authority
    disposition: a typed refusal or an authoritative-storage verification
      result; never an acceptance grounded solely in derived metadata
```

These are future executable TestPak witnesses. Bootstrap proves their names, owner binding, documentary meaning, and cross-surface agreement only. It does not execute journal verification, read a real chain, or detect real event loss.

## Derived-material false-loss and false-safety witnesses (LEG-019)

Unauthenticated derived material may accelerate, locate, or describe suspicion. It cannot authenticate itself, and the two failure directions stay equally visible: it can neither prove safety nor prove loss. A corroborated row proves one row-to-frame relationship and nothing about its neighbours, the table, or what is missing.


Authoritative meanings:

```text
forged_sibling_cannot_cause_false_loss
    One forged or contradictory derived sibling cannot make the system declare an
    authoritative event lost without checking authoritative storage.
    expects: one forged or contradictory derived sibling yields at most a
      suspicion outcome that names authoritative storage as the unfinished
      check; it never yields a loss verdict by itself
    disposition: a typed suspicion or corroboration-failure outcome; a loss
      verdict issues only after authoritative storage is consulted

agreeing_truncated_table_cannot_cause_false_safety
    A derived table whose present rows agree with authority cannot prove that
    omitted middle or trailing authoritative events do not exist.
    expects: a derived table whose present rows all corroborate still yields
      no completeness verdict; omitted middle or trailing authoritative
      events remain undetermined until authority is consulted
    disposition: a typed per-row corroboration result carrying no
      table-completeness claim

derived_row_cannot_authenticate_siblings
    Corroborating one row does not authenticate another row.
    expects: corroborating one row leaves every other row's authentication
      state unchanged
    disposition: a typed single-row corroboration result scoped to that row
      alone

derived_row_cannot_authenticate_table_count
    Corroborating present rows does not establish the authoritative number of rows.
    expects: corroborating every present row yields no authoritative row
      count; the cardinality claim remains unproven
    disposition: a typed corroboration result that carries no cardinality
      authority

derived_row_cannot_authenticate_order
    Corroborating row contents does not establish table ordering.
    expects: corroborating row contents yields no ordering verdict; a
      permuted table with identical row contents corroborates identically
    disposition: a typed content-corroboration result that carries no
      ordering authority

derived_row_cannot_authenticate_tail_boundary
    A derived claimed tail is not authoritative merely because its final present
    row verifies.
    expects: a verifying final present row yields no tail-boundary verdict;
      a claimed tail authenticates only against authoritative storage
    disposition: a typed row corroboration carrying no tail claim; tail
      authority resolves against authoritative storage or not at all

derived_row_cannot_prove_absence_or_loss
    An unauthenticated omission is not authoritative evidence of absence or loss.
    expects: an omission in derived material yields no absence or loss
      verdict while authoritative storage stands unconsulted
    disposition: a typed suspicion outcome at most; absence and loss verdicts
      issue only from authoritative evidence
```

## Bounded-discovery witness (LEG-028)

Bounded traversal is a claim about discovery work, not about the size of the returned vector.


Authoritative meanings:

```text
page_limit_bounds_discovery_work_not_only_output
    The page limit and work budget constrain discovery before unbounded decode,
    allocation, or materialization. Scanning or decoding the complete matched set
    and truncating only the returned output does not satisfy bounded traversal.
    expects: discovery under a page limit and work budget observes decode,
      allocation, and materialization work bounded by the declared budget
      while the matched set grows far beyond the page
    disposition: a page of results with within-budget work evidence, or a
      typed budget refusal; a full-set scan truncated at output fails the
      witness
```

## Bounded-allocation witness (LEG-020)

Complexity evidence is a claim about retained work. A small answer computed by materializing everything is not bounded. Cross-references `page_limit_bounds_discovery_work_not_only_output` (LEG-028), which owns the discovery-side claim.


Authoritative meanings:

```text
allocation_does_not_scale_with_full_matched_set
    Memory allocation and retained materialization do not scale with the complete
    matched set when the contract promises bounded paging or streaming. Small
    returned output is not proof of bounded discovery, allocation, or retained work.
    expects: peak allocation and retained materialization observed under a
      bounded paging or streaming contract stay within the declared bound
      while the complete matched set grows; matched-set growth does not grow
      retained memory
    disposition: within-bound allocation evidence accompanying the result,
      or a typed resource refusal; small returned output alone is not
      accepted as the evidence
```

## Deferred native-postcondition witnesses (LEG-043)

A requested or attempted mechanism is not an established postcondition. These rows are named now so the proof boundary exists; their execution is intentionally blocked until the relevant adapter is admitted. Their presence claims no native launcher, no descriptor table, and no syscall.


Authoritative meanings:

```text
descriptor_postcondition_failure_is_not_reported_applied
    A requested descriptor postcondition is not reported as established,
    applied, or verified when the required postcondition verification fails.
    expects: an injected postcondition-verification failure yields a typed
      not-established result; no surface reports the postcondition as
      established, applied, or verified
    disposition: a typed postcondition failure; the descriptor state claim
      stays at requested or attempted

fcntl_getfd_failure_fails_closed
    Failure to read descriptor flags cannot produce a verified close-on-exec
    or equivalent descriptor postcondition.
    expects: an injected flag-read failure yields a typed failure; no
      verified close-on-exec or equivalent postcondition is reported from an
      unread flag state
    disposition: a typed descriptor-flag read failure that leaves the
      postcondition unestablished

fcntl_setfd_failure_fails_closed
    Failure to apply descriptor flags cannot produce an applied or verified
    descriptor postcondition.
    expects: an injected flag-apply failure yields a typed failure; no
      applied or verified postcondition is reported from a failed
      application
    disposition: a typed descriptor-flag apply failure that leaves the
      postcondition unestablished
```

## Reopen and reimport idempotency witness (LEG-074)

One sharp reproducer inside LEG-074's broader crash-boundary matrix. It does not replace that matrix, and it does not replace the general idempotency-authority export/restore law owned by `LEG-083`.


Authoritative meanings:

```text
close_reopen_reimport_returns_zero_new_events
    After a completed import is durably closed and reopened, retrying the same
    stable import identity imports zero new events and returns the
    already-established authority outcome rather than duplicating source events.
    The witness binds stable import identity, source lineage, destination
    lineage, completed close state, reopened destination state, idempotency
    authority binding, the original established authority outcome, and a
    resulting new-event count of zero.
    expects: retrying the same stable import identity after a durable close
      and reopen admits no additional source event and returns the outcome
      the first import established
    disposition: the original outcome with a new-event count of exactly zero;
      a duplicated source event fails the witness
```

## Proof-carrying compaction witnesses (LEG-085)


Authoritative meanings:

```text
signed_compaction_preserves_or_commits_the_removed_set
    Under a signed-history profile, a compaction from one physical segment set
    to another is either a proof-preserving rewrite of the same ordered
    logical commitments or an authorized retention transition that binds the
    prior and resulting roots, the retained and removed commitment sets, the
    retention policy, and the replacement seals. No third shape exists.
    expects: a compaction that alters the committed logical set without an
      authorized retention transition is refused; an authorized transition
      binds every named component and verifies against both roots
    disposition: a proof-preserving rewrite receipt or a retention-transition
      record; a segment-set change carrying neither fails the witness

compaction_inputs_survive_until_replacement_evidence_is_durable
    Compaction inputs are not retired before the transition record and the
    replacement evidence are durably recoverable. Recovery at any crash point
    finds the old set, the new set, or both -- never neither.
    expects: a crash injected at every compaction boundary recovers a readable
      history whose commitments verify under the old or the new seal set
    disposition: an old-or-new recovery outcome; an unreadable window between
      retirement and durable replacement fails the witness
```

## Typed projection-replay failure witnesses (LEG-086)


Authoritative meanings:

```text
matched_kind_decode_failure_is_a_typed_terminal
    A wrong event kind is a normal filter. A matched kind whose bytes fail to
    decode, or whose application fails, is a typed replay terminal -- never a
    panic, never a silent skip, and never laundered into a cache miss that
    continues producing apparently-valid state.
    expects: replay over a matched event with malformed bytes terminates in a
      typed decode failure naming the event; replay over a failing apply
      terminates in a typed apply failure; no route skips the event and
      continues
    disposition: a typed replay terminal carrying the failing event identity
      and failure class; silent continuation fails the witness

replay_routes_agree_on_first_failure_and_class
    Full replay, incremental replay, cache rebuild, and checkpoint restore
    reach the same first failing event and the same failure class over the
    same history.
    expects: every replay route over one failing history reports the identical
      first failing event identity and failure class
    disposition: agreeing typed terminals across all routes; a route that
      diverges or downgrades the failure to a miss fails the witness
```

## Injected-storage isolation witness (LEG-087)

Mirrors the injected-clock doctrine (LEG-055): an injected backend excludes
the ambient host resource it replaces, including diagnostics.


Authoritative meanings:

```text
trapping_host_filesystem_remains_unreached_under_injected_storage
    Under an injected non-host storage backend, no code path -- including
    diagnostics, evidence probes, and fallbacks -- touches the ambient host
    filesystem.
    expects: a store driven end to end over an injected backend with a
      trapping host filesystem completes every exercised operation with the
      trap recording zero host-filesystem calls
    disposition: a zero-contact trap report accompanying the conformance run;
      any host-filesystem touch, including from a diagnostic, fails the
      witness
```

## Single-writer dual-axis reconciliation witnesses (DEC-075)

The composition law's executable denominator. `docs/02_SYSTEM_MODEL.md` owns
the end-to-end law and `spec/reconciliation/` owns the closed bindings;
these rows own the executable meanings. Bootstrap proves their names, owner
binding, documentary meaning, and cross-surface agreement only: it executes
no turn, commits no effect, and drives no port.


Authoritative meanings:

```text
paired_result_and_receipt_share_one_turn_evaluation
    The semantic projection and the evidence projection of one turn derive
    from one admitted evaluation over one frozen input cut. Production never
    re-evaluates the expression to construct its receipt; TestPak's
    independent references are computed outside the paired implementation.
    expects: instrumented execution of one turn records exactly one admitted
      evaluation producing both the semantic result and its receipt; a second
      production evaluation performed to construct the receipt fails the
      witness
    disposition: one paired-evaluation trace with both projections bound to
      one TurnId, judged against independent semantic and evidence references

hlc_cannot_substitute_for_commit_sequence
    No commit, durability, checkpoint, cursor, or retry decision accepts an
    Hlc value or wall observation where GlobalSequence or CommitPoint is
    required. Chronology is evidence, never the ledger.
    expects: an operation offered chronology evidence in place of a required
      commit coordinate is refused with a typed error naming the required
      coordinate
    disposition: a typed coordinate-substitution refusal; chronology remains
      admissible for pruning, lag, and temporal filtering only

receipt_binds_chronology_and_commit_without_collapsing_them
    A receipt for one durable operation carries the chronology evidence and
    the commit coordinate as distinct bound fields. The two books never merge
    into one timestamp.
    expects: altering either the chronology field or the commit field of a
      receipt changes receipt identity independently of the other
    disposition: one receipt with both books present and separately bound; a
      projection collapsing them into one value fails the witness

replay_reconstructs_same_turn_and_returns_original_receipts
    Replay after output commit but before checkpoint advance reconstructs the
    identical TurnId over the same frozen cuts and returns the original
    receipts rather than duplicating work.
    expects: replaying such a turn yields the identical TurnId and the
      original receipts, and the count of newly executed work for that turn
      is exactly zero
    disposition: the original receipts, unchanged; a duplicated effect or a
      fresh receipt for already-committed work fails the witness

lost_acknowledgement_requires_reconciliation_before_retry
    An effect whose acknowledgement is lost surfaces OutcomeUnknown and
    admits no retry until reconciliation consults durable intent and attempt
    evidence. A missing acknowledgement is ambiguity, not permission.
    expects: after an injected acknowledgement loss the operation reports
      OutcomeUnknown and a retry attempted before reconciliation is refused
    disposition: a typed reconciliation outcome first; retry proceeds only
      under the declared recovery class with commit knowledge resolved

port_response_cannot_cross_attempts
    A port response is bound to exactly one AttemptId. Offering it to any
    other attempt, including a retry of the same logical operation, is
    refused.
    expects: a response constructed for one attempt is refused when delivered
      to a different attempt, and the suspended attempt resumes only with its
      own response
    disposition: a typed attempt-binding refusal; no cross-attempt resume

driver_await_and_cooperative_drive_produce_equivalent_logical_trace
    Driver strategy is physical, never semantic. One program under a blocking
    driver, a cooperative driver, and an awaiting host produces the same
    logical behavior.
    expects: one program over one input yields identical logical traces,
      identities, receipts, and recovery dispositions under each admitted
      driver strategy
    disposition: per-driver trace and receipt sets that compare equal; a
      divergence is a driver defect reported against the adapter, never a
      semantic difference

checkpoint_gap_does_not_duplicate_committed_effect
    A crash between effect commit and checkpoint advance recovers without a
    second physical execution of the committed intent.
    expects: recovery after an injected crash in the gap emits no second
      physical effect for the committed intent and advances the checkpoint
    disposition: the committed effect stands exactly once; recovery returns
      the original outcome rather than re-executing

reconciliation_appends_evidence_without_rewriting_original_observation
    Reconciliation settles ambiguity by appending or referencing new
    evidence. The original event, attempt observation, and receipt remain
    exactly as first recorded.
    expects: reconciling an ambiguous attempt leaves the original event,
      attempt observation, and receipt byte-identical while an appended
      reconciliation record links them to the new evidence
    disposition: an appended reconciliation record referencing the originals;
      any mutation of the original evidence fails the witness
```

## Secret-authority shred witnesses (LEG-081)

Canonical proof-row identity and meaning for `LEG-081`. `docs/35_CRYPTO_AND_SECRET_AUTHORITY.md` owns the secret-authority law, shred semantics, the cryptographic-erasure threat model, and key-authority transitions; it projects these IDs and states no per-row executable meaning. The typed law in `spec/legacy_obligations/inventory.rs` is unchanged by this pass: its owner, gates, status, compatibility disposition, deletion condition, and lifetime all stand.


Authoritative meanings:

```text
shred_ack_waits_for_backend_durability
    A shred acknowledgement is emitted only after the owning backend has durably
    established destruction of the relevant key authority. An accepted request or
    an attempted delete is not sufficient.
    expects: no shred acknowledgement is observable before the owning
      backend reports durable destruction of the relevant key authority; an
      accepted request or attempted delete surfaces as unfinished or
      refused, never acknowledged
    disposition: an acknowledgement emitted only after durable destruction,
      or a typed unfinished or refused posture

crash_before_durable_key_delete_does_not_report_shred_success
    A crash before durable key destruction cannot leave a successful shred
    acknowledgement or a successful final shred result. Recovery preserves the
    incomplete or refused posture.
    expects: a crash injected before durable key destruction recovers into
      an incomplete or refused shred posture; no surface reports a
      successful acknowledgement or final shred result
    disposition: the recovered incomplete or refused posture; success
      appears only once durable destruction completes

reopen_after_ack_cannot_recover_shredded_plaintext
    After a valid durable shred acknowledgement, closing and reopening the store
    cannot recover plaintext through the shredded key scope.
    expects: after a valid durable shred acknowledgement, close and reopen
      followed by reads through the shredded key scope return no plaintext
    disposition: a typed Shredded outcome for the shredded scope; never
      recovered plaintext

shred_transition_binding_mismatch_is_rejected
    The shred transition binds StoreId, AuthorityGeneration, KeyGeneration, key
    scope, and shred-transition identity. A mismatch in any bound component fails
    closed and cannot produce an acknowledged, applied, or verified shred
    transition.
    expects: a shred transition presenting a mismatched StoreId,
      AuthorityGeneration, KeyGeneration, key scope, or shred-transition
      identity fails closed at the binding check
    disposition: a typed binding refusal; no acknowledged, applied, or
      verified transition from mismatched components

stale_or_pre_shred_keyset_restore_is_rejected
    A stale keyset, or a keyset from before the acknowledged shred transition,
    cannot restore readability for the shredded scope.
    expects: restoring a stale keyset, or one predating the acknowledged
      shred transition, leaves the shredded scope unreadable
    disposition: a typed restore refusal or an unreadable scope; readability
      never returns through an out-of-date keyset

foreign_keyset_generation_is_rejected
    A keyset from another store lineage, authority generation, key generation, or
    otherwise foreign authority context cannot restore readability for the
    shredded scope.
    expects: a keyset originating outside the store's own lineage, authority
      generation, key generation, and authority context leaves the shredded
      scope unreadable
    disposition: a typed foreign-keyset refusal; readability never returns
      through a foreign authority context

shredded_unavailable_and_keyset_missing_remain_distinct
    Shredded, Unavailable, and KeysetMissing remain distinct availability or
    authority outcomes in typed results, receipts, recovery, and explanations. No
    projection or formatter may collapse one into another.
    expects: scenarios producing Shredded, Unavailable, and KeysetMissing
      yield three distinct typed outcomes across results, receipts,
      recovery, and explanations
    disposition: the distinct typed outcome for each scenario; a projection
      or formatter collapsing any two of them fails the witness

snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys
    Snapshot, fork, WorldImage, artifact-bundle, and ordinary receipt surfaces
    exclude raw key material by default. They may carry approved identities,
    commitments, encrypted or opaque references, and proof posture, but they may
    not export usable raw secret-key bytes through an ordinary projection.
    expects: snapshot, fork, WorldImage, artifact-bundle, and ordinary
      receipt exports produced over a keyed store contain no usable raw
      secret-key bytes
    disposition: exports carrying only approved identities, commitments,
      encrypted or opaque references, and proof posture; raw key bytes on
      any ordinary projection fail the witness

external_key_backend_preserves_shred_semantics
    An external key backend must preserve the same acknowledgement durability,
    transition binding, stale and foreign restore rejection, result-state
    distinctions, raw-key export exclusions, and fail-closed behaviour as the
    in-process authority contract. Backend indirection may change mechanism,
    never semantics.
    expects: every shred witness in this table observes the same outcome
      when the key authority is served by an external backend:
      acknowledgement durability, transition binding, restore rejection,
      result-state distinctions, export exclusions, and fail-closed
      behaviour are unchanged
    disposition: the same typed outcomes as the in-process authority
      contract; a backend-specific divergence is a witness failure, not an
      alternative semantics
```

Coverage is a property of the law, not of the prose around it. Every clause of the
typed `LEG-081` law carries at least one owned executable witness, and every
canonical row names the clause it pressures. This table is local to `LEG-081`; it
is not a universal proof-clause framework.

LEG-081 proof-coverage matrix:

```text
durable destruction before acknowledgement
    shred_ack_waits_for_backend_durability
    crash_before_durable_key_delete_does_not_report_shred_success
    reopen_after_ack_cannot_recover_shredded_plaintext

shred-transition identity and authority binding
    shred_transition_binding_mismatch_is_rejected

stale, pre-shred, and foreign restore rejection
    stale_or_pre_shred_keyset_restore_is_rejected
    foreign_keyset_generation_is_rejected

Shredded / Unavailable / KeysetMissing distinction
    shredded_unavailable_and_keyset_missing_remain_distinct

raw-key exclusion from snapshot, fork, WorldImage, artifact, and ordinary receipt surfaces
    snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys

external backend semantic parity
    external_key_backend_preserves_shred_semantics
```

<!-- HISTORICAL-MIGRATION:BEGIN -->
### Historical migration note (5.5D4b-2c)

Before this pass, `docs/35_CRYPTO_AND_SECRET_AUTHORITY.md` listed eight witness names and no per-row meaning. Those names were a preimplementation sketch, never a shipped compatibility surface, and they did not cover the typed law: the shred-transition binding clause carried no witness at all, and three names claimed narrower scope than the clause they defended. The law was not amended to fit the names; the names were corrected to fit the law. This note is the only surface on which the retired IDs may appear.

```text
preserved                                        5
renamed for scope accuracy                       3
added to cover a previously unwitnessed clause   1
silently deleted                                 0
aliased                                          0
```

Renamed, with no alias retained:

```text
pre_shred_keyset_restore_is_rejected
    now stale_or_pre_shred_keyset_restore_is_rejected

shredded_and_keyset_missing_remain_distinct
    now shredded_unavailable_and_keyset_missing_remain_distinct

snapshot_and_fork_exclude_keys_by_default
    now snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys
```

Added to witness the previously unwitnessed binding clause:

```text
shred_transition_binding_mismatch_is_rejected
```
<!-- HISTORICAL-MIGRATION:END -->

## Compression and format-version witnesses (DEC-063, LEG-053)

Canonical proof-row identity and meaning for the V1 compression posture and the format/version open law. `docs/05_STORAGE_FBAT_AND_TILES.md` owns the storage law, the compression posture itself, and the future-profile admission requirements; it projects these IDs and states no per-row executable meaning. The two obligations qualify at their own gates: a shared source paragraph is not a shared gate schedule.



Authoritative meanings:

```text
compressed_fbat_authority_frame_is_rejected_in_v1
    A `.fbat` authority frame that arrives compressed, or that declares any
    compression profile, is refused in V1. Authority-frame identity and recovery
    depend on exact bytes, so V1 offers no route by which an authority frame is
    stored, opened, or accepted in compressed form.
    expects: the authority-frame open path refuses the compressed frame and
      admits no authority frame
    disposition: a typed V1 compression refusal on the format compatibility path,
      never a transparent decompress-and-accept

compressed_vpak_semantic_section_is_rejected_in_v1
    A `.vpak` canonical semantic section that arrives compressed, or that declares
    any compression profile, is refused in V1. Canonical semantic bytes carry
    program identity; a decompressed reconstruction is not the canonical byte
    sequence and cannot stand in for it.
    expects: the semantic-section reader refuses the compressed section and admits
      no ProgramImage
    disposition: a typed V1 compression refusal, never a decompressed section
      presented as canonical bytes

compression_profile_requires_expansion_bound
    A compression profile offered for derived tiles or artifacts is admitted only
    when it declares a CompressionId, canonical parameters, a bounded
    decompression rule, and a maximum expansion ratio. A profile that omits the
    expansion bound is refused: an unbounded decode is a resource-exhaustion
    route, not a compression posture.
    expects: profile admission refuses a compression profile that declares no
      maximum expansion ratio
    disposition: a typed profile-admission refusal naming the absent expansion
      bound, never an admitted profile carrying an unbounded decode

future_format_version_fails_with_its_own_typed_disposition
    A persisted format/version pair that a reader cannot open fails with the typed
    refusal belonging to that pair, never a generic parse error, a corruption
    verdict, or a silent misread. `FbatFormatVersion` and
    `BatTaggedRecordVersion` are distinct typed identities (DEC-064), and the
    refusal names which identity it refused.
    expects: opening a future FbatFormatVersion returns the version-specific typed
      refusal and yields no store handle
    disposition: a typed unsupported-version refusal naming the encountered
      version, distinct from a corruption verdict and from a canonical open
```

## Version-identity witnesses (DEC-064)

Canonical proof-row identity and meaning for the distinct-version-identity law. `docs/16_IDENTITY_TIME_AND_NAVIGATION.md` owns the version vocabulary, the identity commitments, and the negotiation posture; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
program_image_id_is_independent_of_compiler_provenance
    `ProgramImageId` commits to the canonical ProgramImage bytes, including the
    image version, and to nothing else. Compiler implementation and compiler
    version are provenance, not identity: two qualified compilers emitting
    identical canonical bytes produce the same id, and one compiler emitting
    different canonical bytes produces a different id.
    expects: two qualified compilers emitting byte-identical canonical images
      yield equal ProgramImageId values while their recorded compiler provenance
      differs
    disposition: identity is decided by the canonical bytes; provenance is
      carried as receipt evidence and never enters the id computation

distinct_version_types_do_not_typecheck_when_substituted
    Every member of spec/identities/types.rs VersionIdentityKind::ALL denotes a
    distinct version type. No generic Version crosses a subsystem boundary, and
    substituting one identity where another is required is rejected by the type
    system rather than by a runtime range check. The boundary includes the
    three-way EventFrame/container/payload split (FrameVersion versus
    FbatFormatVersion versus BatTaggedRecordVersion) and the
    SchemaVersion/LayoutVersion split.
    expects: substituting any member of VersionIdentityKind::ALL for another
      fails to typecheck at the specification surface
    disposition: a compile-time type error, never a runtime comparison of two
      version numbers that happen to share a representation

netbat_version_does_not_upgrade_pakvm_isa
    NetBat protocol negotiation is independent of `PakVmIsaVersion` and
    `WorldImageVersion`. A negotiated transport version never raises, lowers, or
    otherwise selects the ISA or image version a program executes against.
    expects: negotiating any NetBatProtocolVersion leaves the resolved
      PakVmIsaVersion and WorldImageVersion unchanged
    disposition: transport negotiation yields a transport version only; an
      attempt to carry an ISA or image version through negotiation is refused
```

## HLC ordering and range witnesses (DEC-061)

Canonical proof-row identity and meaning for the HLC query-ordering and range law. `docs/16_IDENTITY_TIME_AND_NAVIGATION.md` owns the time vocabulary, the lowering rule, and the forbidden-use list; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
order_by_hlc_uses_commit_tiebreak
    Within one journal, `ORDER BY HLC` canonicalizes and lowers to the complete
    key HLC then GlobalSequence. Two events that share or do not compare on HLC
    alone are separated by the physical journal commit sequence, so the emitted
    order is total and deterministic rather than dependent on scan order.
    expects: two events sharing an HLC value emit in GlobalSequence order, and
      the same query over the same journal emits the same order on every run
    disposition: the compiled plan carries the two-component key; a plan ordering
      on HLC alone is not a lawful lowering of ORDER BY HLC

cross_store_hlc_order_is_total
    Across journals, `ORDER BY HLC` lowers to HLC then StoreId then
    GlobalSequence. Events from distinct stores that share an HLC value are
    separated by store lineage before commit sequence, so a cross-store result is
    totally ordered without consulting wall time or arrival order.
    expects: events from two stores sharing an HLC value emit in StoreId order,
      and the emitted order is independent of which store was read first
    disposition: the compiled plan carries the three-component key; a
      cross-journal plan that omits StoreId is not a lawful lowering

hlc_range_is_half_open
    `DURING HLC start THROUGH HLC end` denotes the half-open interval
    [start, end). An event at exactly `start` is included and an event at exactly
    `end` is excluded, so adjacent ranges tile without overlap or gap.
    expects: an event at start is returned, an event at end is not, and two
      adjacent DURING ranges return each event exactly once
    disposition: the range predicate is compiled half-open; a closed or
      implementation-defined boundary is a lowering defect, not a tuning choice

frontier_progress_never_uses_hlc_order
    HLC is causal and chronological evidence, never journal-progress authority.
    Frontier advancement, durability coverage, cursor resume, and commit identity
    are decided by commit sequence and durable cuts; HLC cannot advance a frontier
    even when it is available and appears consistent.
    expects: no frontier advancement consumes an HLC value, and an HLC that runs
      ahead of the durable cut advances no frontier
    disposition: frontier progress resolves from GlobalSequence and CommitPoint
      only; an HLC-derived progress claim is refused rather than approximated

observed_wall_time_is_not_promoted_to_hlc
    `ObservedWallTime` and `Hlc` remain distinct identities. When causal HLC
    evidence is absent the result is `Unavailable`; an observed wall clock is
    never silently promoted into an HLC to fill the gap, however close the two
    readings appear.
    expects: a query needing absent HLC evidence returns Unavailable while an
      ObservedWallTime reading is present and unused
    disposition: a typed Unavailable, distinct from a returned HLC and from an
      ordinary operation failure
```

## Realization-matrix witnesses (DEC-065)

Canonical proof-row identity and meaning for the no_std/alloc realization matrix, the derived-cache collection posture, and cross-profile semantic equivalence. `docs/03_REPOSITORY_AND_PACKAGES.md` owns the package inventory, the feature posture, and the qualification matrix; `docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md` owns the collection and canonical-order law. Both project these IDs and state no per-row executable meaning.


Authoritative meanings:

```text
no_std_batpak_has_no_std_dependency_route
    Built with `default-features = false`, `batpak` realizes its no_std + alloc
    surface: no dependency route reaches `std`, whether direct, transitive,
    feature-unified, or reintroduced by a dev or build dependency. The property is
    compile-proven against the no_std target; a green default build proves nothing
    about it.
    expects: the no_std + alloc qualification target compiles and links with no
      std route present in the resolved dependency graph
    disposition: a compile-time or resolution failure naming the offending route,
      never a passing default build offered as evidence of the no_std surface

no_std_syncbat_has_no_std_dependency_route
    Built with `default-features = false`, `syncbat` realizes its no_std + alloc
    surface under the same law as `batpak`. Its runtime plane structure grants no
    exemption: a std route arriving through an adapter, a feature union, or a
    plane-local dependency is the same defect.
    expects: the no_std + alloc qualification target compiles and links with no
      std route present in the resolved dependency graph
    disposition: a compile-time or resolution failure naming the offending route,
      never a runtime capability check standing in for the compile proof

default_std_does_not_enable_threaded_or_browser_adapters
    The default std profile enables neither the threaded adapter nor the browser
    adapter. Threading and browser hosting are opt-in features, so a default build
    cannot silently acquire threads, web APIs, or their ambient authority through
    feature unification.
    expects: the default std build resolves with the threaded and browser adapter
      features off, including under feature unification across the workspace
    disposition: an adapter absent from the build entirely; a compiled-in adapter
      left dormant at runtime does not satisfy this row

browser_and_native_profiles_preserve_program_semantics
    Every qualified profile preserves program semantics. The same program over the
    same inputs yields the same canonical observables on native and browser alike:
    an adapter supplies mechanism and evidence, never semantic meaning (LEG-066).
    Profile-owned differences are confined to mechanism, performance, and available
    capability. A capability a profile does not offer is refused with its own typed
    disposition, never emulated into a different program-observable result.
    expects: one program over one input yields byte-identical canonical results,
      receipts, and typed dispositions on the native std and browser profiles
    disposition: a divergence is a profile defect reported against the adapter,
      never reconciled by declaring one profile's result authoritative

hash_map_iteration_cannot_influence_canonical_observables
    A hash map is a derived cache. Its iteration order never influences canonical
    identity, canonical bytes, formatting, diagnostics, execution order, or
    receipts. Canonical order comes from ordered structures — `Vec`, bounded
    arenas, `BTreeMap`, and sorted catalogs — so a perturbed hash seed changes
    nothing a program or an auditor can observe.
    expects: replaying the same inputs under a perturbed hash-map iteration order
      yields identical canonical bytes, formatting, diagnostics, execution order,
      and receipts
    disposition: canonical order resolves from the ordered structure that owns it;
      an observable that varies with iteration order is a defect, not a tolerance
```

<!-- HISTORICAL-MIGRATION:BEGIN -->
### Historical migration note (5.5D4b-3b, DEC-065)

`docs/08_SYNCBAT_RUNTIME.md` carried `hash_map_iteration_cannot_change_canonical_bytes` inside its resource-exhaustion fixture block. Both the placement and the name were wrong, and neither was a shipped compatibility surface.

The placement: hash-map iteration order is not a resource-exhaustion property. The row defends the derived-cache collection posture, which DEC-065 owns, not the try_reserve and budget law docs/08 owns.

The name: DEC-065 states that hash-map iteration never influences canonical identity, formatting, diagnostics, execution order, or receipts. "Canonical bytes" is one of five observables. A row named for the narrowest one would pass while formatting, diagnostics, execution order, or a receipt varied with the hash seed. The law was not narrowed to fit the name; the name was corrected to fit the law.

```text
retired    hash_map_iteration_cannot_change_canonical_bytes
successor  hash_map_iteration_cannot_influence_canonical_observables
aliased    0
```
<!-- HISTORICAL-MIGRATION:END -->

## Resource-exhaustion witnesses (DEC-066)

Canonical proof-row identity and meaning for typed resource exhaustion and the no-partial-publication law. `docs/08_SYNCBAT_RUNTIME.md` owns the allocation discipline, the two canonical failure classes, and the plane coverage list; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
attacker_length_is_checked_before_reserve
    Every attacker-influenced allocation decodes the declared length, checks the
    arithmetic, and enforces the semantic bound BEFORE reserving. A declared
    length is a claim, not a size: a hostile length must be refused without the
    reservation ever being attempted, so the bound cannot be enforced by
    observing the allocator fail.
    expects: a hostile declared length is refused before any reservation is
      attempted, and the process allocates no memory proportional to it
    disposition: a typed bound violation raised at the decode boundary, never an
      allocation failure standing in for a missing length check

allocation_failure_returns_resource_exhausted
    A genuine allocation failure returns `ResourceExhausted`. It is an execution
    failure, never `INVALID`: no value violated its contract. The two never
    collapse into each other, into a string, or into a shared error code.
    expects: a failing try_reserve yields ResourceExhausted, distinct from INVALID
      and from BudgetExceeded
    disposition: a typed ResourceExhausted; a plane-specific error may wrap it but
      may not flatten it into a contract violation

declared_work_overrun_returns_budget_exceeded
    Exceeding the declared work budget returns `BudgetExceeded`, which remains
    distinct from `ResourceExhausted`. Running out of a budget the caller declared
    is not the same event as the machine running out of a resource, and the two
    disposals cannot be merged even when both terminate the same operation.
    expects: an operation exceeding its declared work budget yields
      BudgetExceeded, distinct from ResourceExhausted and from INVALID
    disposition: a typed BudgetExceeded naming the exceeded budget; a plane error
      may wrap it but may not collapse it into resource exhaustion

resource_exhaustion_never_publishes_partial_event
    Resource exhaustion at any point never publishes a partial event. Publication
    happens only after completion, so an exhausted operation leaves no
    half-written event visible to any reader, replay, or projection.
    expects: exhaustion injected at every publication boundary leaves zero
      partially published events observable
    disposition: the operation fails with its typed exhaustion class and publishes
      nothing; a published event is always complete

resource_exhaustion_never_advances_checkpoint
    Resource exhaustion never advances a checkpoint. A checkpoint asserts durable
    progress; an operation that failed made none, so recovery from the checkpoint
    cannot skip the work the failure prevented.
    expects: exhaustion injected around checkpoint advancement leaves the
      checkpoint at its prior durable position
    disposition: the checkpoint remains unadvanced and the typed exhaustion class
      is returned; a partially advanced checkpoint is not a lawful state

artifact_staging_failure_publishes_no_artifact_ref
    A staging failure publishes no `ArtifactRef`. A reference names a complete
    content-addressed artifact, so a failed staging yields no reference at all
    rather than one pointing at incomplete content.
    expects: a failure injected at every staging boundary yields no ArtifactRef on
      any surface, including receipts and diagnostics
    disposition: the typed staging failure is returned and no reference is minted;
      an ArtifactRef always names complete content

pakvm_arena_exhaustion_returns_typed_terminal_disposition
    PakVM arena exhaustion returns a typed terminal disposition. It never panics,
    aborts, presents an abort as an ordinary refusal, wraps an integer, or leaves
    a partially initialized canonical value in the arena.
    expects: exhausting a PakVM arena mid-execution yields a typed terminal
      disposition, with no panic, no abort, and no partially initialized value
      readable afterwards
    disposition: a typed terminal execution disposition distinct from INVALID; an
      abort presented as an ordinary refusal is a defect, not a disposition
```

## Kernel identity and binding witnesses (DEC-062)

Canonical proof-row identity and meaning for the kernel identity and binding policy. `docs/10_WORLD_IMAGES_AND_PORTS.md` owns the kernel vocabulary, the two binding modes, and the receipt rule; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
kernel_cannot_resolve_by_display_name
    `FROM KERNEL` refers to a canonical `KernelManifest` entry. PakVM never
    resolves a kernel by display name, Rust path, or function pointer, so a
    renamed or relocated implementation cannot be reached by spelling its label,
    and two kernels sharing a label remain distinct.
    expects: resolution by display name, Rust path, or function pointer is
      refused, and resolution succeeds only against the canonical manifest entry
    disposition: a typed resolution refusal; a name-based lookup path does not
      exist to fall back to

kernel_contract_and_implementation_ids_are_not_interchangeable
    `KernelContractId`, `KernelInterfaceHash`, `KernelImplementationId`, and
    `KernelQualificationReceiptId` are distinct identities. Substituting one where
    another is required is rejected: a contract is not an implementation, and a
    hash of an interface is neither.
    expects: substituting any one kernel identity for another is rejected rather
      than silently resolving to a plausible kernel
    disposition: a typed identity mismatch naming both the expected and supplied
      identity kinds

qualified_interface_requires_matching_qualification_receipt
    A `QualifiedInterface` binding pins `KernelContractId` plus
    `KernelInterfaceHash` and requires an ACCEPTED matching
    `KernelQualificationReceiptId`. A missing, foreign, or non-matching receipt
    fails closed: portability is granted by qualification evidence, never by
    interface shape alone.
    expects: a QualifiedInterface binding with an absent, foreign, or
      non-matching qualification receipt is refused before execution
    disposition: a typed binding refusal naming the unsatisfied qualification;
      a matching interface hash alone does not authorize execution

exact_kernel_binding_rejects_another_implementation
    An `ExactImplementation` binding pins one `KernelImplementationId`. Another
    implementation is rejected even when it satisfies the same contract and
    interface hash: an exact pin is a statement about identity, not about
    capability.
    expects: an ExactImplementation binding refuses a different
      KernelImplementationId that satisfies the same contract and interface hash
    disposition: a typed binding refusal naming the pinned and encountered
      implementation ids

attempt_receipt_records_exact_kernel_implementation
    The `AttemptReceipt` records the exact `KernelImplementationId` actually used,
    including when the WorldImage allowed a `QualifiedInterface`. A portable
    qualified binding still yields an exact execution record, so what ran is
    always recoverable from the receipt.
    expects: an attempt executed under a QualifiedInterface binding records the
      exact KernelImplementationId that ran, not the contract id or interface hash
    disposition: the receipt names one implementation; a receipt recording only
      the qualified interface is incomplete evidence, not an alternative form
```

## AST enforcement-gate witnesses (DEC-068)

Canonical proof-row identity and meaning for the TestPak AST enforcement gate. `docs/12_TESTPAK.md` owns the detector inventory, the required-behavior table, and the defense-in-depth posture; it projects these IDs and states no per-row executable meaning. These rows prove the DETECTOR: that the gate classifies a source pattern as it must. The semantic behaviors the patterns endanger are owned elsewhere and witnessed there — `hash_map_iteration_in_canonical_encoder_is_rejected` proves the gate rejects the source construct, while `hash_map_iteration_cannot_influence_canonical_observables` (DEC-065) proves the runtime observables do not move. Neither substitutes for the other.


Authoritative meanings:

```text
local_domain_type_inside_function_is_rejected
    A domain type declared inside a function body is a hard finding in production
    code. Domain vocabulary belongs to a module an owner can name; a type hidden
    in a function body escapes the ownership, review, and reuse rules that apply
    to every other domain type.
    expects: the AST gate reports a hard finding for a domain type declared in a
      production function body
    disposition: a hard finding naming the enclosing function; a lint suppression
      at the site does not clear it

local_domain_type_inside_closure_is_rejected
    The same law inside a closure body. A closure is not a loophole: nesting the
    declaration one scope deeper does not change the ownership question, and an
    AST gate that inspects only function bodies would miss it.
    expects: the AST gate reports a hard finding for a domain type declared in a
      production closure body, including a closure nested inside another closure
    disposition: a hard finding naming the enclosing closure; depth of nesting
      does not attenuate it

test_local_nonsemantic_fixture_type_is_allowed
    A test-local type carrying no domain semantics is permitted. The gate
    distinguishes a fixture that exists to stage a test from a domain type that
    happens to be declared in a test: the first is allowed, the second is a
    finding wherever it sits.
    expects: a non-semantic fixture type in a test yields no finding, while a
      domain type declared in a test still yields one
    disposition: no finding for the fixture; the permission is decided by the
      absence of domain semantics, never by the file being a test

production_expect_is_rejected
    `expect` in production code is a hard finding, as are panic, unwrap, and dbg.
    A production path states its failure in a typed result rather than terminating
    the process with a message.
    expects: the AST gate reports a hard finding for expect, unwrap, panic, and
      dbg on a production path
    disposition: a hard finding at the call site; a message argument, however
      descriptive, does not make the call permitted

test_expect_with_context_is_allowed
    A contextual `expect` in a test is permitted. A test asserts; terminating the
    test process on a violated assumption is the intended behavior, so the
    production rule does not extend to it.
    expects: a contextual expect in a test yields no finding while the same call
      on a production path yields a hard finding
    disposition: no finding in the test; the permission is scoped to test code and
      is not inherited by production code a test happens to call

unledgered_unsafe_fn_is_rejected
    An `unsafe fn` without an exact compiler-assumption ledger entry is a hard
    finding. The mechanism is not forbidden; the unrecorded assumption is. What
    the unsafe block assumes about the compiler must be written where a reviewer
    can find it.
    expects: an unsafe fn with no matching compiler-assumption ledger entry yields
      a hard finding, and one with an exact matching entry does not
    disposition: a hard finding naming the unledgered site; a nearby comment is
      not a ledger entry

unledgered_pointer_cast_is_rejected
    A pointer cast or a narrowing cast without an exact compiler-assumption ledger
    entry is a hard finding, under the same law as `unsafe fn`. A cast that
    silently discards range or provenance is a dangerous mechanism whether or not
    it appears inside an unsafe block.
    expects: a pointer or narrowing cast with no matching compiler-assumption
      ledger entry yields a hard finding
    disposition: a hard finding naming the unledgered cast; being outside an
      unsafe block does not exempt it

public_flume_receiver_is_rejected
    A dependency-owned type in an ordinary public API is a hard finding; this row
    names the instance the gate proves. A public signature naming a foreign type
    exports that dependency's contract and version as though it were ours.
    expects: a public API exposing a dependency-owned type yields a hard finding,
      while the same type used internally does not
    disposition: a hard finding naming the leaked type and its owning dependency;
      a re-export does not launder it

hash_map_iteration_in_canonical_encoder_is_rejected
    Canonical-order dependence on hash-map iteration is a hard finding at the
    source level. This row proves the DETECTOR fires on the construct; DEC-065
    separately owns the runtime law that iteration never influences canonical
    observables. A passing detector is not evidence about the runtime, and passing
    runtime observables are not evidence the detector works.
    expects: an encoder deriving canonical order from hash-map iteration yields a
      hard finding, while one deriving it from an ordered structure does not
    disposition: a hard finding naming the iteration site; a stable-in-practice
      iteration order is not a defence

drawer_module_name_requires_explicit_disposition
    A drawer module name — utils, common, helpers, manager, processor, misc —
    requires an explicit disposition. The name is not banned outright: it must be
    justified where the module is declared, so that a drawer is a decision someone
    made rather than a place things accumulated.
    expects: a drawer-named module with no explicit disposition yields a finding,
      and one carrying an explicit disposition does not
    disposition: a finding naming the module and the absent disposition; renaming
      the drawer to an unlisted synonym does not resolve the ownership question
```

<!-- HISTORICAL-MIGRATION:BEGIN -->
### Historical migration note (5.5D4b-3b, DEC-068)

`docs/12_TESTPAK.md` carried `test_local_fixture_type_is_classified_correctly`. The name stated a verdict without stating the claim: "correctly" names no observable, so the row would have passed against any classification the implementation happened to produce, including the opposite one. That is the shape of a fixture that cannot fail. The word is one the expectation-clause falsifiability scan already rejects in a clause; it cannot be lawful in the row identity itself.

DEC-068 states that test-only material is permitted while production violations are hard findings. The successor names that claim and the condition it depends on: the type is allowed because it carries no domain semantics, not because it sits in a test file.

```text
retired    test_local_fixture_type_is_classified_correctly
successor  test_local_nonsemantic_fixture_type_is_allowed
aliased    0
```
<!-- HISTORICAL-MIGRATION:END -->

## Compilation-boundary witnesses (DEC-051)

Canonical proof-row identity and meaning for the source-compilation and raw-BatQL authority boundary. `docs/10_WORLD_IMAGES_AND_PORTS.md` owns the boundary law, the evidence chain, and the CompilerPort posture; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
raw_batql_is_not_a_netbat_invocation
    Ordinary transport permits `InvokeEntrypoint` and `ExecuteQueryProgram` and
    nothing else. `CompileAndExecuteRawBatQL`, `EvalText`, and `ExecuteSource` are
    not NetBat invocations: BatQL source text is authoring input, never executable
    authority, so there is no transport route that accepts source and runs it.
    expects: a NetBat request carrying raw BatQL source is refused as an unknown
      invocation class, and no compilation or execution is attempted
    disposition: a typed transport refusal at admission; the request never reaches
      a compiler, so the refusal cannot depend on what the source said

compiler_port_is_not_available_to_guest_programs
    `CompilerPort` is a separately admitted host capability. A guest program
    cannot hold it, request it, or reach it through another capability it does
    hold: compilation authority is not implied by execution authority.
    expects: a guest program requesting or invoking CompilerPort is refused, and
      holding any ordinary grant does not confer it
    disposition: a typed capability refusal; the port is absent from the guest's
      capability envelope rather than present and guarded

compiler_port_is_absent_from_default_server_profile
    `CompilerPort` is absent from the default server profile. It belongs to a
    development or admin profile that must be admitted deliberately, so a
    default-configured server offers no compilation surface to remove later.
    expects: the default server profile resolves with no CompilerPort admitted,
      and admitting one is an explicit profile change
    disposition: the port is not in the profile; a port present but disabled by
      configuration does not satisfy this row

compiler_output_still_requires_program_validation
    A ProgramImage emitted by `CompilerPort` passes ordinary validation and
    admission like any other. The compiler is never a tunnel around PakVM
    validation: source identity is provenance, not runtime authority, and a
    CompilationReceipt attests where an image came from, never that it is
    admissible.
    expects: a compiler-emitted ProgramImage that fails ordinary validation is
      refused, and its CompilationReceipt does not shorten or skip the admission
      path
    disposition: the ordinary typed validation refusal; the image is judged by its
      canonical bytes, exactly as a client-supplied image would be

```
## Query-program invocation witnesses (DEC-050)

Canonical proof-row identity and meaning for ephemeral query-program invocation. `docs/10_WORLD_IMAGES_AND_PORTS.md` owns the invocation-class law, the capability envelope, and the response binding; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
query_program_with_effect_instruction_is_rejected
    A query-only ProgramImage containing any node whose admitted `PakVmNodeSpec`
    carries the `Effectful` effect posture is rejected at validation, before
    execution. docs/07: "A pure query image cannot contain Effect instructions.
    The validator rejects the image before execution."

    The effect instructions are not enumerated here, or anywhere in prose. They
    are exactly the nodes `spec/pakvm_isa/nodes.rs` admits with `EffectPosture::Effectful`,
    which admission binds to the Effect algebra in both directions: an Effect-algebra
    node cannot claim purity, and a node outside it cannot claim effectfulness. A
    node admitted into that algebra is therefore covered on the day it is admitted,
    with no list to update and no list to forget.
    expects: validating a query ProgramImage that contains any node admitted with
      the Effectful posture yields a typed rejection, with no execution performed
      and no capability consulted
    disposition: a typed pre-execution validation refusal naming the offending
      node identity; a capability denial raised when the effect is attempted comes
      too late and does not satisfy this row

query_program_cannot_install_process
    An `ExecuteQueryProgram` invocation is ephemeral. Process installation and
    lifecycle hooks are forbidden to it: a query cannot leave anything behind that
    outlives the invocation, whatever its result says.
    expects: a query program attempting process installation or a lifecycle hook
      is refused, and nothing persists after the invocation completes
    disposition: a typed refusal of the forbidden operation; ALLOW, DENY, or DEFER
      appearing in the query's data never drives an effect

query_program_cannot_request_write_capability
    A query program runs under a server-issued read-only query capability
    envelope. It cannot request, escalate to, or otherwise acquire a write
    capability: the envelope is issued by the server, so the program has no route
    to widen it.
    expects: a query program requesting a write capability is refused, and its
      envelope remains read-only for the whole invocation
    disposition: a typed capability refusal; the write capability is absent from
      the envelope rather than present and denied at use

query_program_over_work_bound_is_denied_before_execution
    A query program exceeding its declared work bound is denied BEFORE execution.
    The bound is checked against the declared plan, not discovered by running the
    program until it costs too much: a hostile query must not get to execute at
    all.
    expects: a query program whose declared work exceeds the bound is denied with
      no execution performed and no partial work observable
    disposition: a typed pre-execution denial; a mid-execution budget termination
      is a different event and does not satisfy this row

query_program_result_binds_program_and_grant_identity
    A query response binds ProgramImageId, query authority digest, historical cut,
    source generation, completeness, proof disposition, actual work, result
    digest, and receipt. The result names exactly what ran and under whose
    authority, so a response cannot be reattributed to another program or grant.
    expects: a response carries every bound component, and altering the program or
      the grant produces a different binding
    disposition: the receipt binds all named components; a response missing one is
      incomplete evidence, not an abbreviated form

entrypoint_receipt_cannot_satisfy_query_program_execution
    An `InvokeEntrypoint` receipt cannot stand as evidence that an
    `ExecuteQueryProgram` invocation occurred. The two classes differ in
    deployment authority, lifetime, effect permission, and capability origin, so
    entrypoint authority cannot be replayed to claim a query execution that never
    ran under a query envelope.
    expects: an entrypoint AttemptReceipt offered as evidence of query-program
      execution is refused as a class mismatch
    disposition: a typed invocation-class mismatch naming both classes; matching
      ProgramImageId does not make the receipts interchangeable

```
## Entrypoint invocation witnesses (DEC-049)

Canonical proof-row identity and meaning for declared WorldImage entrypoint invocation. `docs/10_WORLD_IMAGES_AND_PORTS.md` owns the entrypoint contract, the identity binding, and the effect-declaration rule; it projects these IDs and states no per-row executable meaning.


Authoritative meanings:

```text
entrypoint_cannot_substitute_foreign_program_image
    A request cannot substitute a foreign ProgramImage while keeping entrypoint
    authority. Identity binds WorldImageId, WorldInterfaceHash, EntrypointId,
    ProgramImageId, InvocationId, and AttemptId together, so swapping the image
    breaks the binding rather than inheriting the entrypoint's grant.
    expects: a request naming an admitted entrypoint but a ProgramImageId the
      entrypoint does not bind is refused before execution
    disposition: a typed binding refusal naming the bound and supplied image ids;
      a valid image that the entrypoint does not bind is still foreign

entrypoint_effect_must_be_declared_by_world_interface
    An entrypoint performs effects only when its contract declares them. The
    declaration lives in the world interface, so an undeclared effect is refused
    however the program is written and whatever capability the grant carries.
    expects: an entrypoint attempting an effect its contract does not declare is
      refused, while a declared effect proceeds
    disposition: a typed effect refusal naming the undeclared effect; holding a
      capability is not a declaration

query_program_receipt_cannot_satisfy_entrypoint_invocation
    An `ExecuteQueryProgram` receipt cannot stand as evidence that an
    `InvokeEntrypoint` invocation occurred. An ephemeral query executed under a
    read-only envelope never carried entrypoint deployment authority, so its
    receipt cannot be replayed to claim a persistent invocation.
    expects: a query-program AttemptReceipt offered as evidence of entrypoint
      invocation is refused as a class mismatch
    disposition: a typed invocation-class mismatch naming both classes; matching
      ProgramImageId does not make the receipts interchangeable

```
## Transport-authority witnesses (LEG-045)

Canonical proof-row identity and meaning for the transport authority boundary. `docs/10_WORLD_IMAGES_AND_PORTS.md` owns the transport posture; it projects this ID and states no per-row executable meaning.


Authoritative meanings:

```text
tls_does_not_upgrade_program_or_proof_authority
    Transport is bounded, domain-thin, and cannot launder proof. A TLS session,
    however authenticated, grants no program authority and upgrades no proof
    posture: securing the channel says something about the channel and nothing
    about the image that travelled through it or the evidence it carries.
    expects: an image or proof arriving over an authenticated TLS session receives
      the same validation, admission, and proof posture as one arriving otherwise
    disposition: transport authenticity is recorded as transport evidence; a
      request to treat it as program or proof authority is refused
```

<!-- HISTORICAL-MIGRATION:BEGIN -->
### Historical migration note (5.5D4b-3b, DEC-049 and DEC-050)

`docs/10_WORLD_IMAGES_AND_PORTS.md` carried `attempt_receipt_cannot_cross_invocation_classes` as one row. The claim is directional and the name is not: a receipt crossing FROM entrypoint TO query program and a receipt crossing FROM query program TO entrypoint are two different forgeries, defended by two different laws. One row would pass while proving one direction and leaving the other route open, and no owner could be named for it — DEC-049 owns entrypoint authority and DEC-050 owns query-program authority, so a bidirectional row belongs to neither.

Split into two directional rows, each owned by the class whose authority is being claimed. Nothing was weakened: the original claim is the conjunction of the two successors.

```text
retired     attempt_receipt_cannot_cross_invocation_classes
successors  entrypoint_receipt_cannot_satisfy_query_program_execution   (DEC-050)
            query_program_receipt_cannot_satisfy_entrypoint_invocation  (DEC-049)
aliased     0
```
<!-- HISTORICAL-MIGRATION:END -->

## Candidate-origin and realization-posture witnesses (DEC-079)

Canonical proof-row identity and meaning for the immutable candidate nursery and its complete-lineage law. `spec/sprouting/types.rs` authors the candidate lifecycle vocabulary and `spec/identities/types.rs` authors the candidate lineage identity; `docs/39_SPROUTING_NURSERY_AND_PROMOTION.md` owns the nursery law and projects these IDs while stating no per-row executable meaning. Bootstrap proves their names, owner binding, documentary meaning, and cross-surface agreement only: it generates no candidate, qualifies none, and promotes none.


Authoritative meanings:

```text
candidate_origin_confers_no_authority
    A candidate's origin records how it was produced -- deterministic
    generation, bounded search, transfer reuse, human authorship,
    machine-assisted synthesis, or repair of a prior candidate -- and confers
    no admission authority. Recording a synthesizer or a repair provenance is
    a truthful lineage fact, never evidence that the candidate qualified.
    expects: a candidate offered for admission on the strength of its recorded
      origin alone is refused, and two candidates differing only in origin
      receive the same admission decision from the same qualification evidence
    disposition: a typed refusal grounding authority in qualification evidence;
      origin is carried as lineage provenance and never enters the admission
      decision

scaffold_cannot_close_realization
    A compiling stub that merely returns success carries the Scaffold
    realization posture: structurally present, unqualified, and counted in the
    unrealized denominator. A binary realized graph can never report green over
    a tree of scaffolds, because a scaffold satisfies no obligation it stands
    in for.
    expects: a candidate whose realization posture is Scaffold is counted
      unrealized, and a graph containing one scaffold does not report a closed
      realization
    disposition: an unrealized count naming each scaffold; a green realization
      verdict standing over a scaffold fails the witness

qualified_nursery_record_is_not_promoted_source
    An immutable nursery record -- even one carrying qualification and holdout
    receipts -- is speculative lineage evidence, never the promoted realization
    source. Promotion is a separate authorized act that consumes the record and
    never rewrites it, so a qualified record does not itself become
    authoritative program or proof material.
    expects: a fully qualified nursery record offered as the promoted source is
      refused until an explicit promotion act admits it, and the promotion
      leaves the immutable record byte-identical
    disposition: a typed separation between qualified lineage evidence and
      promoted authority; a nursery record treated as the realized source fails
      the witness
```

## Realization-preserving versus law-changing promotion witness (DEC-080)

Canonical proof-row identity and meaning for the promotion-class law. `docs/39_SPROUTING_NURSERY_AND_PROMOTION.md` owns the promotion law and projects this ID while stating no per-row executable meaning. The class adds an authority path and never replaces a common promotion requirement.


Authoritative meanings:

```text
law_changing_candidate_cannot_enter_realization_preserving_lane
    Promotion classifies a candidate change as realization-preserving or
    law-changing. A law-changing candidate cannot promote through the mechanical
    or bounded-search lane reserved for realization-preserving changes; it
    routes to the architect and the DEC-074 proof-policy anti-weakening process.
    A candidate never repairs its own courtroom.
    expects: a candidate whose change class is LawChanging is refused entry to
      the realization-preserving promotion lane and is routed to the
      architect-required disposition
    disposition: a typed lane refusal naming the law-changing class; a
      law-changing candidate admitted through the mechanical or bounded-search
      lane fails the witness
```

## Bounded-search and evaluation-set witnesses (DEC-081)

Canonical proof-row identity and meaning for the bounded-search, evaluation-set-separation, and transfer-reuse law. `docs/39_SPROUTING_NURSERY_AND_PROMOTION.md` owns the search and evaluation law and projects these IDs while stating no per-row executable meaning.


Authoritative meanings:

```text
search_budget_is_declared_and_receipted
    Every candidate search declares finite candidate-count, logical-work,
    memory, and monotonic-deadline bounds before it runs, and receipts the
    actual work it performed against them. An unbounded search, or one whose
    receipt omits the work it spent, is not a lawful search.
    expects: a candidate search missing any of the four declared bounds, or
      whose receipt does not record the actual work spent against them, is
      refused
    disposition: a typed declaration-or-receipt refusal naming the absent bound
      or the missing work receipt; an unbounded search admitted as lawful fails
      the witness

search_and_holdout_sets_are_disjoint
    Search, qualification, holdout, and regression inputs carry explicit
    evaluation-set roles, and holdout evidence for a campaign cannot reuse that
    campaign's search inputs. A candidate tuned against a set cannot be judged
    final against the same set, so a search/holdout overlap voids the holdout
    evidence.
    expects: a holdout evaluation drawing any input that also appears in the
      campaign's search set is refused, and the two role-tagged sets share no
      member
    disposition: a typed set-overlap refusal naming the shared input; holdout
      evidence computed over a search input fails the witness

failed_holdout_candidate_is_quarantined
    A candidate that passed qualification but fails its holdout evaluation is
    quarantined rather than promoted: the affected candidate is held out of the
    promoted set while its lineage and failing evidence stay visible. Holdout
    failure quarantines the candidate; it neither silently drops it from the
    denominator nor blocks the surrounding campaign.
    expects: a candidate failing holdout is placed in a typed quarantine
      disposition, remains counted with its failing evidence, and does not enter
      the promoted set
    disposition: a quarantine record naming the failed holdout; a
      holdout-failing candidate that promotes, or that vanishes from the
      denominator, fails the witness
```

## Frozen-judge and dependency-frontier witnesses (DEC-082)

Canonical proof-row identity and meaning for the whole-tree speculative materialization and frontier-qualification law. `docs/39_SPROUTING_NURSERY_AND_PROMOTION.md` owns the whole-tree topology and projects these IDs while stating no per-row executable meaning.


Authoritative meanings:

```text
candidate_cannot_modify_frozen_judge
    The judge root is a content-addressed frozen snapshot, and every evidence
    artifact binds the exact judge digest it was produced under. A candidate
    never modifies its frozen judge or prior evidence; digest binding detects a
    judge mutation and invalidates the tainted evidence, so evidence produced
    against a mutated judge cannot verify.
    expects: evidence produced by a candidate that mutated its judge root fails
      digest verification against the frozen judge snapshot, while the original
      prior evidence remains byte-identical
    disposition: a typed judge-digest invalidation; evidence that verifies
      against a mutated judge, or a mutated prior artifact accepted as original,
      fails the witness

later_green_cannot_bless_unresolved_dependency
    Authority in a speculative tree advances dependency-first: a later frontier
    that qualifies green can never bless an unresolved earlier frontier it
    depends on. A whole tree may be materialized before its gates close, but the
    trusted frontier advances only through dependency-ordered qualification.
    expects: a downstream candidate reporting green while an upstream dependency
      remains unresolved advances no trusted frontier past that unresolved
      dependency
    disposition: a typed dependency-order refusal; a green descendant that
      promotes over an unqualified ancestor fails the witness
```

## Mini-supernova rehearsal witnesses (DEC-076, DEC-078, DEC-080, DEC-081, DEC-082)

The F5 rehearsal's proof rows bind campaign behavior the harness actually
executes: an activated and killed semantic mutant, a seeded bounded trace
attack, deterministic replay-equality of a fully simulated trace, capture-then-
offline-replay runtime conformance, and the four campaign-liveness claims.

Authoritative meanings:

```text
planted_semantic_mutant_is_activated_and_killed
    A planted semantic mutant that compiles and passes the deliberately
    insufficient happy-path test is recorded as activated and is killed by
    the independent differential route or the holdout set.
    expects: the mutant's activation is recorded in campaign evidence; the
      independently implemented witness or holdout evaluation terminates it
      in a recorded kill, never a silent skip
    disposition: a recorded kill; a surviving semantic mutant fails the
      witness and quarantines the candidate

bounded_generated_trace_attack_holds_boundary
    A seeded deterministic generator attacks the real boundary with a
    bounded trace set; the receipt always binds the exact seed, trace
    count, operation bound, and value bound so the candidate and
    confirming runs reproduce the identical attack.
    expects: every generated illegal trace terminates in a typed refusal at
      the boundary; the receipt records seed, trace count, operation
      bound, value bound, and traces executed
    disposition: the boundary holds with typed refusals; an unrefused
      illegal trace or an unreceipted attack fails the witness

deterministic_replay_equality_of_simulated_trace
    A fully simulated trace re-executes to identical states and observables
    — replay-equality of the complete simulation, distinct from interleaving
    exploration and from reachable-set exploration.
    expects: re-execution of the declared bounded trace set yields byte- and
      state-identical observables on every trace
    disposition: equality holds; any divergence is a typed failure naming
      the first divergent step

runtime_capture_appends_observed_history_without_rewrite
    In-band runtime capture appends observed history and never rewrites,
    and in-band observation states at most that no divergence was observed.
    expects: the captured history is append-only across the rehearsal; the
      in-band side never emits a conformance conclusion
    disposition: an append-only observation record; a rewrite or an in-band
      conformance claim fails the witness

offline_replay_concludes_conformance_for_captured_history
    Only independent offline replay of the complete captured history may
    conclude conformance for that history.
    expects: offline replay of the full captured history terminates in
      conformant-for-observed-history or a typed divergence
    disposition: conformance concluded only offline; divergence quarantines
      the candidate rather than blocking the running boundary

every_admitted_campaign_obligation_reaches_a_terminal
    Every admitted candidate obligation reaches Promoted, Refused,
    Invalidated, or ArchitectRequired; no obligation silently drops from
    the denominator.
    expects: the campaign closure evidence shows one terminal per admitted
      obligation, with ArchitectRequired visible and campaign-stopping
    disposition: terminals visible in the denominator; a missing terminal
      or a vanished ArchitectRequired outcome fails the witness

candidate_search_terminates_within_declared_budget
    Candidate search stops within its declared CandidateSearchBudget, and
    the receipt binds declared bounds to actual use.
    expects: the search receipt records declared versus actual candidates,
      logical work, memory, and monotonic time, all within bounds
    disposition: a within-budget termination receipt; an overrun or a stop
      without a bound-binding receipt fails the witness

bounded_repair_loop_reaches_stable_qualified_frontier
    The RepairOfCandidate lineage converges: the bounded repair loop
    reaches a trusted frontier that is qualified and stops moving.
    expects: after the repair child qualifies, the frontier advances
      dependency-first and then remains stable under re-evaluation
    disposition: a stable qualified frontier; oscillation or an unbounded
      repair sequence fails the witness

confirming_rerun_changes_no_authoritative_result
    The confirming run re-executes the rehearsal and the campaign
    authoritative results are identical across runs: the confirming rerun
    changes no authoritative result.
    expects: terminals, frontier state, and dispositions from the confirming
      rehearsal equal the candidate rehearsal's bound results
    disposition: identical authority across the two runs; any drift refuses
      promotion confirmation
```

## Substrate proof families (5.5D1)

Future TestPak owns these executable properties. Bootstrap verifies only that they are named, owned, and assigned; it does not execute them.

### Publication and reconciliation (LEG-037, docs/05)

```text
committed is never reported as an ordinary operation failure
committed with an incomplete receipt remains committed
outcome unknown is not treated as known non-commit
reconciliation appends evidence and never rewrites the original durable observation
admitted atomic publication produces all or none of its durable group
```

### Logical operation and attempt (DEC-028, LEG-042)

```text
retry retains LogicalOperationId
retry receives a new AttemptId
CancelledBeforeAdmission proves no admission
CancelledAfterAdmission does not prove non-commit
```

### Bounded traversal (LEG-028, LEG-029)

```text
valid pages concatenate to the one-shot reference traversal
no gaps, no duplicates, no reordering
stale, cross-store, cross-generation, cross-source-cut, cross-direction,
  cross-selector, and cross-filter cursors fail closed
pre-decode limit and work budget bound discovery
```

### Generated typed routing (LEG-046)

```text
every declared route is present exactly once
unknown route refuses
duplicate route refuses
decoder, capability, handler, and receipt bindings remain aligned
```

### Absolute deadline (LEG-066, docs/10)

```text
retry does not reset the overall deadline
per-attempt deadline cannot exceed the overall deadline
lower repeating mechanisms cannot outlive the remaining deadline
expiry before and after admission preserve attempt and commit distinctions
```

## Proof-policy anti-weakening (DEC-074)

TestPak owns the gate. Every change to an owned proof-policy surface declares one `ProofPolicyChangeClass`; an unclassified change is refused; a Weakening carries the complete authority package, including a Weakening of the gate itself. The contract is in `12_TESTPAK.md`. Bootstrap proves the contract and the detector rule, never arbitrary semantic diff inference.

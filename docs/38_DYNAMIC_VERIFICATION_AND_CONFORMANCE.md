---
status: AUTHORITATIVE
contract_id: BP-DYNAMIC-VERIFICATION-1
authority_scope: layered dynamic verification, model/trace separation, runtime conformance, coverage strength, claim kinds, and the anti-self-grading discipline
supersedes: verification wording previously implied only through DEC-073, DEC-075, and SEED-INDEPENDENT-ORACLE
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Dynamic Verification and Conformance

This document is clean-room operating law: how BatPak verifies dynamic behavior,
what independence a verdict requires, and what runtime observation may and may
not do to durable law. It is the Phase 5.5F1 charter — F1 authors the law, later
phases land the typed bodies. Where a type is named as forthcoming, this section
is the law that type will enforce, not a placeholder.

This document authors SEED-LAYERED-DYNAMIC-VERIFICATION,
SEED-MODEL-TRACE-SEPARATION, and SEED-RUNTIME-CONFORMANCE-NO-REWRITE, and is
backed by decisions `DEC-077` (verification axes, coverage strength, and claim
kinds) and `DEC-078` (enforcement posture and runtime conformance lock). It
states law, not a new durable behavior obligation, so it introduces no `LEG-*`
row.

## 1. Layered axes, no assurance ladder

Verification keeps its concerns as SEPARATE typed axes. A verdict is not a rank;
it is a tuple whose fields never collapse into one another:

```text
basis                the relationship of the evidence source to the subject and
                     its semantic owner (section 2)
method               the evidence-producing technique: structural rule, compile
                     refusal, property sequence, bounded state exploration,
                     schedule exploration, deterministic simulation, differential
                     execution, translation validation, fault injection, crash
                     recovery, fuzzing, mutation, complexity contract, benchmark
                     envelope, history replay
claim                the property being challenged (section 5)
coverage             Sampled, Bounded, ExhaustiveWithinDeclaredModel,
                     ObservedHistory (section 4)
enforcement posture  Blocking, Quarantine, Advisory
lane                 PullRequestFast, Merge, Scheduled, Release, Runtime
freshness            whether the evidence still binds the current source, policy,
                     model, toolchain, profile, and affected boundary
terminal             the native semantic or execution result carried by its
                     owning algebra; never one universal cross-domain result enum
```

Method is the technique, never the strength: "exhaustive" and "sampled" are
coverage facts, "replayed" and "observed" are execution-posture facts, and none
of them names how evidence is produced. Enforcement posture is the gate posture
of the requirement — Blocking, Quarantine, Advisory — distinct from the runtime
refuse-or-append behavior law of section 6. The lane names where the requirement
executes; a requirement is satisfied in its declared lane, not a faster one.

No aggregate "assurance level", "confidence tier", or "maturity" ladder may
substitute for these axes. A single ordered enum spanning method + coverage +
freshness + result is forbidden by shape: ordering it would let a sampled-but-
fresh row outrank an exhaustive-but-stale row, or let a strong method launder a
weak claim. Each axis is compared only against its own kind.

This is SEED-LAYERED-DYNAMIC-VERIFICATION (derives from `DEC-077`, refines
SEED-AVAILABILITY-AXES). The executable shape-based guard — the audit that
refuses any type presenting these concerns as one ordered scalar — is authored
in F2 against the concrete verification types. This section is the law that
guard enforces.

<!-- DYNAMIC-VERIFICATION-VOCABULARY:BEGIN generated from spec/verification/types.rs by bootstrap/project.py; do not edit -->
```text
VerificationMethod (VERIFICATION_METHODS)
  StructuralRule
  CompileRefusal
  PropertySequence
  BoundedStateExploration
  ScheduleExploration
  DeterministicSimulation
  DifferentialExecution
  TranslationValidation
  FaultInjection
  CrashRecovery
  Fuzzing
  Mutation
  ComplexityContract
  BenchmarkEnvelope
  HistoryReplay

VerificationClaimKind (VERIFICATION_CLAIM_KINDS)
  Safety
  Liveness
  BoundedResponse
  Convergence
  Stability
  NonOscillation
  Determinism
  Refinement
  Conformance
  ResourceEnvelope

VerificationCoverage (VERIFICATION_COVERAGES)
  Sampled
  Bounded
  ExhaustiveWithinDeclaredModel
  ObservedHistory

VerificationEnforcementPosture (VERIFICATION_ENFORCEMENT_POSTURES)
  Blocking
  Quarantine
  Advisory

VerificationLane (VERIFICATION_LANES)
  PullRequestFast
  Merge
  Scheduled
  Release
  Runtime

IndependentEvidenceRouteKind (INDEPENDENT_EVIDENCE_ROUTE_KINDS)
  DifferentialImplementation
  HostileBoundary
  IndependentHistoryReplay

RuntimeVerificationMode (RUNTIME_VERIFICATION_MODES)
  PreventiveGuard
  InBandObservation
  OfflineReplay

RuntimeVerificationDisposition (RUNTIME_VERIFICATION_DISPOSITIONS)
  GuardAdmitted
  GuardRefused
  NoDivergenceObserved
  ConformantForObservedHistory
  Divergent
  Incomplete
  Stale
  Unsupported
```
<!-- DYNAMIC-VERIFICATION-VOCABULARY:END -->

## 2. Model/trace separation

Four distinct verification bases exist and never collapse:

```text
ContractProjection                a projection generated from typed authority; drives inputs, never concludes
IndependentReference { route }    a separately owned model, evaluator, or relation authored against the law
DirectBoundary { route }          a hostile probe of the real boundary under test
RuntimeObservation                evidence recorded from an executing system
```

A generated projection can never be the SOLE independent oracle: it is derived
from the same authority it would grade. A self-observation can never be the SOLE
independent oracle either: a system observing itself corroborates but does not
independently conclude. An independent verdict requires evidence whose basis is
distinct from the subject it grades.

This is SEED-MODEL-TRACE-SEPARATION (derives from `DEC-077`, refines
SEED-INDEPENDENT-ORACLE). The typed basis is a field ON the proof requirement,
not a comment beside it: a `ContractProjection` offered on a route that demands
independent evidence is a compile error, not a runtime warning. That basis field
is authored in F2 `spec/verification/types.rs`.

The independent-evidence route lives INSIDE the basis, not beside it.
`IndependentReference { route }` carries a `DifferentialImplementation` or
`IndependentHistoryReplay` route and never `HostileBoundary`; `DirectBoundary
{ route }` carries `Some(HostileBoundary)` or `None`; `ContractProjection` and
`RuntimeObservation` structurally carry no route. Admission refuses any other
pairing, so a route can never attach to a basis that cannot host it.

Enforcement posture is authored gate policy, not an observed fact. It stays on
the requirement and is ABSENT from every observation. `Blocking` fails the row
when its requirement fails; `Quarantine` quarantines the affected candidate or
artifact without blocking the run; `Advisory` stays visible and never blocks and
never confers. A caller cannot observe its way past a blocking posture, because
the observation never carries enforcement at all.

## 3. Projection is oracle-ineligible, but its completeness still matters

A `ContractProjection` generates test material and never serves as its own
oracle or a voting witness. This does NOT mean a projection's completeness is
unmeasured or irrelevant — that inference is wrong and the law refuses it.

Projection completeness still matters because the projection drives the space in
which every other basis operates:

```text
valid-action generation      what the tester is allowed to attempt
transition enumeration       which state moves are reachable in test
test-space coverage          the denominator other bases sample or exhaust
runtime monitor schemas      the shape a RuntimeObservation monitor recognizes
generated proof inputs       the material fed to an IndependentReference
conformance replay           the trace a replay witness re-executes
```

A nonvoting projection that omits one transition still prevents that transition
from ever being tested — no independent oracle can grade a case the projection
never generated. Completeness of the projection and independence of the oracle
are two different obligations, and neither excuses the other.

The clean split:

```text
projection completeness   exact typed-authority <-> generated-projection parity
independent evidence      a separately owned model, evaluator, hostile boundary, or relation
```

The projection cannot VOTE as an independent oracle, but it must still faithfully
PROJECT the law. A projection that projects the law completely and votes on
nothing is exactly correct; a projection that votes is a category error; a
projection that is incomplete silently shrinks every downstream verdict.

## 4. Coverage strength is not a ladder

Verification coverage strengths are DISTINCT axes, not an ordered ladder. The
four strengths — Sampled, Bounded, ExhaustiveWithinDeclaredModel, and
ObservedHistory — cover different kinds of space; none is a higher rung of the
others, and none converts into another by counting. A requirement declaring one
strength is satisfied only by evidence of exactly that strength: exhaustive
evidence does not satisfy a sampled requirement, and sampled evidence does not
satisfy an exhaustive one.

Coverage-of-a-declared-model is NOT coverage-of-its-law. An "exhaustive within
the declared model" pass is exhaustive only over the declared subset:

```text
exhaustive within the declared model   complete over the declared states and transitions
                                        and NOTHING outside them
law-completeness                        a claim over the actual law, which the declared
                                        model may under-approximate
```

An exhaustive-within-model pass carries no law-completeness claim. If the
declared model omits a transition (section 3), an "exhaustive" pass over that
model is silent about it. The model's declared scope is part of the verdict, not
erased by the word "exhaustive".

This law lives inside `DEC-077`. The green-counting method is renamed
`ProofUnitTerminal::is_positive_semantic_terminal` (`spec/proof/types.rs`) so a
sampled or runtime-observed row can never launder into denominator strength: a
positive semantic terminal is one input to observation admission among nine —
method, basis (which now carries the independent-evidence route), exact
coverage, lane, freshness, actual execution, artifact bindings, counterexample
posture, and the terminal itself — and no gate, release gate, or denominator
calculation may use it alone. Enforcement posture is authored gate policy applied
when a plan is assessed, never a fact the observation carries. The declared
requirement is admitted exactly (`spec/verification/observation.rs` `admit_observation`);
there is no coverage ranking in either direction.

F2 proves plan and observation-shape coherence. It does not prove that the
caller honestly computed execution, freshness, artifact, or counterexample
facts. Phase 6 TestPak receipts and independent verifiers convert concrete
evidence into release-eligible proof. At the release boundary that
runtime-conformance evidence binds through the release seal's
`RuntimeConformanceDispositions` field (`spec/release/`,
`36_PUBLIC_API_CI_AND_RELEASE.md`), which is mandatory even when empty: an
empty set is evidence, a missing field is an incomplete envelope. The typed admission (`admit_observation`,
`assess_plan`) refuses an incoherent SHAPE; it does not vouch for the TRUTH of
the boolean facts a `RequirementEvidenceObservation` carries.

## 5. Claim kinds, including NonOscillation

A verdict names WHICH property it claims. The typed claim kinds are distinct;
one is not a weaker or stronger form of another:

```text
Safety            a named bad thing cannot happen at the challenged boundary
Liveness          a named good thing eventually happens
BoundedResponse   each obligation meets its declared deadline
Convergence       the system reaches a bounded stable region
Stability         the system remains within a bounded region once reached
NonOscillation    each control or retry cycle admits new evidence (progress)
Determinism       the same admitted inputs yield the same canonical observables
Refinement        an implementation agrees with its reference semantics
Conformance       observed behavior agrees with the admitted model
ResourceEnvelope  work, allocation, and retention stay within declared bounds
```

Four of these are near neighbors and their distinctions are the point:

```text
NonOscillation    progress-per-cycle: each control or retry cycle must admit new evidence
Liveness          something good eventually happens; may terminate yet thrash first
BoundedResponse   each obligation meets its deadline; may keep cycling forever within budget
Convergence       the system reaches a bounded stable region; a limit-cycle satisfies this
```

The distinctions are load-bearing:

```text
a cycle that meets every deadline           satisfies BoundedResponse, may violate NonOscillation
a run that eventually terminates            satisfies Liveness, may have thrashed (violated NonOscillation)
a limit-cycle in a bounded stable region    satisfies Convergence while making no progress
a control/retry cycle admitting no new evidence   violates NonOscillation and is refused
```

NonOscillation / progress-per-cycle means: a control or retry cycle that admits
no new evidence makes no progress and is refused. A system may meet deadlines,
eventually terminate, and sit in a bounded region while still oscillating without
progress; NonOscillation is the claim that separates real progress from motion.

NonOscillation is a CLAIM KIND inside `DEC-077`, not a standalone decision. It
gives the docs/17 "no retry oscillation without new admitted evidence" prose a
typed proof-row carrier — the row that carries a NonOscillation claim and its
admitted-evidence witness is authored in F2.

## 6. Runtime conformance: refuse-or-append, never rewrite; independent replay

Runtime verification has exactly two lawful effects on the running system, and a
strict prohibition:

```text
REFUSE   halt before an admitted irreversible boundary is crossed
APPEND   record conformance evidence alongside durable history
NEVER    rewrite semantic law, durable history, or proof disposition
```

A runtime monitor may stop an effect before it commits, and it may append what
it observed. It may not edit the law it checks against, mutate durable history,
or flip a proof disposition after the fact. Observation is not authorship.

This is SEED-RUNTIME-CONFORMANCE-NO-REWRITE (derives from `DEC-078`, refines
`DEC-075`). `DEC-078` (Enforcement, Lock) owns this law.

Runtime verification runs in exactly three modes, and each mode owns its lawful
results (`RuntimeVerificationMode` / `RuntimeVerificationDisposition`, authored
in F2 `spec/verification/types.rs`):

```text
PreventiveGuard      GuardAdmitted, GuardRefused, Unsupported
InBandObservation    NoDivergenceObserved, Divergent, Incomplete, Stale, Unsupported
OfflineReplay        ConformantForObservedHistory, Divergent, Incomplete, Stale, Unsupported
```

The refused pairings are the law:

```text
InBandObservation + ConformantForObservedHistory    refused
PreventiveGuard   + ConformantForObservedHistory    refused
OfflineReplay     + NoDivergenceObserved            refused
```

<!-- VERIFICATION-RUNTIME-MATRIX:BEGIN generated from spec/verification/types.rs by bootstrap/project.py; do not edit -->
| Disposition | PreventiveGuard | InBandObservation | OfflineReplay |
| --- | --- | --- | --- |
| GuardAdmitted | yes | - | - |
| GuardRefused | yes | - | - |
| NoDivergenceObserved | - | yes | - |
| ConformantForObservedHistory | - | - | yes |
| Divergent | - | yes | yes |
| Incomplete | - | yes | yes |
| Stale | - | yes | yes |
| Unsupported | yes | yes | yes |
<!-- VERIFICATION-RUNTIME-MATRIX:END -->

A preventive guard needs no replay before refusing: it admits or refuses at the
boundary, and that is its whole verdict. An in-band monitor may state "no
divergence was observed in this trace" — never conformance. Only INDEPENDENT
offline replay of a complete, current observed history may state "this observed
history conforms to the admitted model" — and even that says nothing about
executions outside the observed history. A monitor generated from a
`ContractProjection` corroborates but never alone concludes conformance — by
section 2 it shares its basis with the authority it grades.

The Tier 0 two-distinct-runs law (`spec/tier0_cross_run/`) is a useful ANALOGY
for why self-corroboration is refused; it is not a type dependency.
`spec/verification/` does not import or reuse the Tier 0 cross-run types as
product-level verification authority.

## 7. Independent-evidence route kind

Independence has kinds, and the kinds are named vocabulary, not free text. F2
MINTS `IndependentEvidenceRouteKind` — the single vocabulary describing WHICH
kind of independent route supplies independence: a differential implementation,
a hostile boundary probe, an independent history replay, and the other admitted
routes F2 enumerates. It is minted, not forthcoming: the enum exists, and its
route/basis pairings are enforced at admission (section 2). The route lives
INSIDE `VerificationBasis` — `IndependentReference { route }` and `DirectBoundary
{ route }` carry it — so a requirement names its independent route as typed
structure, never as adjacent prose.

Consumers name the NEED; a concrete carrier supplies the KIND:

```text
PromotionRequirement::IndependentEvidenceRoute   names the need: at least one admitted independent route must exist
the anti-self-grading SEED law                   forbids a verdict resting only on the subject's own basis
```

F3's `CandidatePromotionPlan` (`spec/sprouting/types.rs`) is the FIRST concrete
consumer of `IndependentEvidenceRouteKind`.
`PromotionRequirement::IndependentEvidenceRoute` names the need; the promotion
plan carries the selected `IndependentEvidenceRouteKind` for the candidate it
admits. The vocabulary is minted once in F2 and consumed downstream, never
redefined there.

Tier 0 cross-run independence remains a separate bootstrap concern
(`BP-PUBLIC-API-CI-RELEASE-1`, `spec/tier0_cross_run/`): it proves two hosted
runs describe one source snapshot, a distinct bootstrap independence from the
verification-route independence this vocabulary names, and the two are not
merged.

## 8. Verification plans (generated)

Every active proof row carries exactly one primary claim and a nonempty,
duplicate-free, admissible verification plan (`spec/proof/`, qualified only
through `spec/verification/`). This inventory is a generated projection of
those typed facts.

<!-- VERIFICATION-PLANS:BEGIN generated from spec/proof/inventory.rs; spec/verification/types.rs by bootstrap/project.py; do not edit -->
| Plan | Method | Basis | Coverage | Lane | Enforcement |
| --- | --- | --- | --- | --- | --- |
| PLAN_HOSTILE_BOUNDARY | PropertySequence | DirectBoundary(HostileBoundary) | Sampled | Merge | Blocking |
| PLAN_FAULT_INJECTION | FaultInjection | DirectBoundary(HostileBoundary) | Sampled | Merge | Blocking |
| PLAN_CRASH_RECOVERY | CrashRecovery | DirectBoundary(HostileBoundary) | Sampled | Merge | Blocking |
| PLAN_DIFFERENTIAL | DifferentialExecution | IndependentReference(DifferentialImplementation) | Sampled | Merge | Blocking |
| PLAN_HISTORY_REPLAY | HistoryReplay | IndependentReference(IndependentHistoryReplay) | Sampled | Merge | Blocking |
| PLAN_STRUCTURAL | StructuralRule | DirectBoundary | ExhaustiveWithinDeclaredModel | PullRequestFast | Blocking |
| PLAN_COMPILE_REFUSAL | CompileRefusal | DirectBoundary | Sampled | PullRequestFast | Blocking |
| PLAN_PROPERTY | PropertySequence | DirectBoundary | Sampled | Merge | Blocking |
| PLAN_COMPLEXITY | ComplexityContract | DirectBoundary | Sampled | Merge | Blocking |
| PLAN_RECONCILIATION_REFINEMENT | StructuralRule | ContractProjection | ExhaustiveWithinDeclaredModel | Merge | Blocking |
| PLAN_RECONCILIATION_REFINEMENT | DifferentialExecution | IndependentReference(DifferentialImplementation) | Bounded | Merge | Blocking |
| PLAN_RECONCILIATION_REFINEMENT | ScheduleExploration | DirectBoundary(HostileBoundary) | Bounded | Scheduled | Blocking |
| PLAN_LAW_CHANGE_REFUSAL | TranslationValidation | IndependentReference(DifferentialImplementation) | Bounded | Merge | Blocking |
| PLAN_LAW_CHANGE_REFUSAL | PropertySequence | DirectBoundary(HostileBoundary) | Sampled | Merge | Blocking |
| PLAN_FRONTIER_EXPLORATION | BoundedStateExploration | DirectBoundary(HostileBoundary) | Bounded | Scheduled | Blocking |
| PLAN_BENCHMARK_ADVISORY | BenchmarkEnvelope | DirectBoundary | Sampled | Release | Advisory |
| PLAN_HOLDOUT_QUARANTINE | DifferentialExecution | IndependentReference(DifferentialImplementation) | Bounded | Scheduled | Quarantine |

| Active proof row | Claim | Plan |
| --- | --- | --- |
| middle_event_deletion_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| event_reorder_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| duplicate_payload_splice_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| cross_lane_predecessor_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| cross_entity_predecessor_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| midstream_genesis_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| forged_index_row_cannot_choose_and_authenticate_bytes | Safety | PLAN_HOSTILE_BOUNDARY |
| forged_sibling_cannot_cause_false_loss | Safety | PLAN_HOSTILE_BOUNDARY |
| agreeing_truncated_table_cannot_cause_false_safety | Safety | PLAN_HOSTILE_BOUNDARY |
| derived_row_cannot_authenticate_siblings | Safety | PLAN_HOSTILE_BOUNDARY |
| derived_row_cannot_authenticate_table_count | Safety | PLAN_HOSTILE_BOUNDARY |
| derived_row_cannot_authenticate_order | Safety | PLAN_HOSTILE_BOUNDARY |
| derived_row_cannot_authenticate_tail_boundary | Safety | PLAN_HOSTILE_BOUNDARY |
| derived_row_cannot_prove_absence_or_loss | Safety | PLAN_HOSTILE_BOUNDARY |
| page_limit_bounds_discovery_work_not_only_output | ResourceEnvelope | PLAN_COMPLEXITY |
| allocation_does_not_scale_with_full_matched_set | ResourceEnvelope | PLAN_COMPLEXITY |
| descriptor_postcondition_failure_is_not_reported_applied | Safety | PLAN_FAULT_INJECTION |
| fcntl_getfd_failure_fails_closed | Safety | PLAN_FAULT_INJECTION |
| fcntl_setfd_failure_fails_closed | Safety | PLAN_FAULT_INJECTION |
| close_reopen_reimport_returns_zero_new_events | Safety | PLAN_CRASH_RECOVERY |
| signed_compaction_preserves_or_commits_the_removed_set | Safety | PLAN_PROPERTY |
| compaction_inputs_survive_until_replacement_evidence_is_durable | Safety | PLAN_CRASH_RECOVERY |
| matched_kind_decode_failure_is_a_typed_terminal | Safety | PLAN_HOSTILE_BOUNDARY |
| replay_routes_agree_on_first_failure_and_class | Refinement | PLAN_DIFFERENTIAL |
| trapping_host_filesystem_remains_unreached_under_injected_storage | Safety | PLAN_FAULT_INJECTION |
| paired_result_and_receipt_share_one_turn_evaluation | Safety | PLAN_PROPERTY |
| hlc_cannot_substitute_for_commit_sequence | Safety | PLAN_PROPERTY |
| receipt_binds_chronology_and_commit_without_collapsing_them | Safety | PLAN_PROPERTY |
| replay_reconstructs_same_turn_and_returns_original_receipts | Determinism | PLAN_HISTORY_REPLAY |
| lost_acknowledgement_requires_reconciliation_before_retry | NonOscillation | PLAN_PROPERTY |
| port_response_cannot_cross_attempts | Safety | PLAN_PROPERTY |
| driver_await_and_cooperative_drive_produce_equivalent_logical_trace | Refinement | PLAN_RECONCILIATION_REFINEMENT |
| checkpoint_gap_does_not_duplicate_committed_effect | Safety | PLAN_CRASH_RECOVERY |
| reconciliation_appends_evidence_without_rewriting_original_observation | Safety | PLAN_PROPERTY |
| shred_ack_waits_for_backend_durability | Safety | PLAN_PROPERTY |
| crash_before_durable_key_delete_does_not_report_shred_success | Safety | PLAN_CRASH_RECOVERY |
| reopen_after_ack_cannot_recover_shredded_plaintext | Safety | PLAN_CRASH_RECOVERY |
| shred_transition_binding_mismatch_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| stale_or_pre_shred_keyset_restore_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| foreign_keyset_generation_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| shredded_unavailable_and_keyset_missing_remain_distinct | Safety | PLAN_PROPERTY |
| snapshot_fork_worldimage_artifact_and_receipt_exports_exclude_raw_keys | Safety | PLAN_PROPERTY |
| external_key_backend_preserves_shred_semantics | Refinement | PLAN_DIFFERENTIAL |
| compressed_fbat_authority_frame_is_rejected_in_v1 | Safety | PLAN_HOSTILE_BOUNDARY |
| compressed_vpak_semantic_section_is_rejected_in_v1 | Safety | PLAN_HOSTILE_BOUNDARY |
| compression_profile_requires_expansion_bound | Safety | PLAN_HOSTILE_BOUNDARY |
| future_format_version_fails_with_its_own_typed_disposition | Safety | PLAN_HOSTILE_BOUNDARY |
| program_image_id_is_independent_of_compiler_provenance | Determinism | PLAN_PROPERTY |
| distinct_version_types_do_not_typecheck_when_substituted | Safety | PLAN_COMPILE_REFUSAL |
| netbat_version_does_not_upgrade_pakvm_isa | Safety | PLAN_PROPERTY |
| order_by_hlc_uses_commit_tiebreak | Determinism | PLAN_PROPERTY |
| cross_store_hlc_order_is_total | Determinism | PLAN_PROPERTY |
| hlc_range_is_half_open | Conformance | PLAN_PROPERTY |
| frontier_progress_never_uses_hlc_order | Safety | PLAN_PROPERTY |
| observed_wall_time_is_not_promoted_to_hlc | Safety | PLAN_COMPILE_REFUSAL |
| no_std_batpak_has_no_std_dependency_route | Safety | PLAN_STRUCTURAL |
| no_std_syncbat_has_no_std_dependency_route | Safety | PLAN_STRUCTURAL |
| default_std_does_not_enable_threaded_or_browser_adapters | Safety | PLAN_STRUCTURAL |
| browser_and_native_profiles_preserve_program_semantics | Refinement | PLAN_DIFFERENTIAL |
| hash_map_iteration_cannot_influence_canonical_observables | Determinism | PLAN_PROPERTY |
| attacker_length_is_checked_before_reserve | Safety | PLAN_HOSTILE_BOUNDARY |
| allocation_failure_returns_resource_exhausted | Safety | PLAN_FAULT_INJECTION |
| declared_work_overrun_returns_budget_exceeded | ResourceEnvelope | PLAN_PROPERTY |
| resource_exhaustion_never_publishes_partial_event | Safety | PLAN_FAULT_INJECTION |
| resource_exhaustion_never_advances_checkpoint | Safety | PLAN_FAULT_INJECTION |
| artifact_staging_failure_publishes_no_artifact_ref | Safety | PLAN_FAULT_INJECTION |
| pakvm_arena_exhaustion_returns_typed_terminal_disposition | ResourceEnvelope | PLAN_FAULT_INJECTION |
| kernel_cannot_resolve_by_display_name | Safety | PLAN_HOSTILE_BOUNDARY |
| kernel_contract_and_implementation_ids_are_not_interchangeable | Safety | PLAN_COMPILE_REFUSAL |
| qualified_interface_requires_matching_qualification_receipt | Safety | PLAN_HOSTILE_BOUNDARY |
| exact_kernel_binding_rejects_another_implementation | Safety | PLAN_HOSTILE_BOUNDARY |
| attempt_receipt_records_exact_kernel_implementation | Conformance | PLAN_PROPERTY |
| local_domain_type_inside_function_is_rejected | Safety | PLAN_STRUCTURAL |
| local_domain_type_inside_closure_is_rejected | Safety | PLAN_STRUCTURAL |
| test_local_nonsemantic_fixture_type_is_allowed | Conformance | PLAN_STRUCTURAL |
| production_expect_is_rejected | Safety | PLAN_STRUCTURAL |
| test_expect_with_context_is_allowed | Conformance | PLAN_STRUCTURAL |
| unledgered_unsafe_fn_is_rejected | Safety | PLAN_STRUCTURAL |
| unledgered_pointer_cast_is_rejected | Safety | PLAN_STRUCTURAL |
| public_flume_receiver_is_rejected | Safety | PLAN_STRUCTURAL |
| hash_map_iteration_in_canonical_encoder_is_rejected | Safety | PLAN_STRUCTURAL |
| drawer_module_name_requires_explicit_disposition | Safety | PLAN_STRUCTURAL |
| raw_batql_is_not_a_netbat_invocation | Safety | PLAN_HOSTILE_BOUNDARY |
| compiler_port_is_not_available_to_guest_programs | Safety | PLAN_HOSTILE_BOUNDARY |
| compiler_port_is_absent_from_default_server_profile | Safety | PLAN_STRUCTURAL |
| compiler_output_still_requires_program_validation | Safety | PLAN_PROPERTY |
| query_program_with_effect_instruction_is_rejected | Safety | PLAN_HOSTILE_BOUNDARY |
| query_program_cannot_install_process | Safety | PLAN_HOSTILE_BOUNDARY |
| query_program_cannot_request_write_capability | Safety | PLAN_HOSTILE_BOUNDARY |
| query_program_over_work_bound_is_denied_before_execution | ResourceEnvelope | PLAN_HOSTILE_BOUNDARY |
| query_program_result_binds_program_and_grant_identity | Conformance | PLAN_PROPERTY |
| entrypoint_receipt_cannot_satisfy_query_program_execution | Safety | PLAN_HOSTILE_BOUNDARY |
| entrypoint_cannot_substitute_foreign_program_image | Safety | PLAN_HOSTILE_BOUNDARY |
| entrypoint_effect_must_be_declared_by_world_interface | Safety | PLAN_HOSTILE_BOUNDARY |
| query_program_receipt_cannot_satisfy_entrypoint_invocation | Safety | PLAN_HOSTILE_BOUNDARY |
| tls_does_not_upgrade_program_or_proof_authority | Safety | PLAN_PROPERTY |
| candidate_origin_confers_no_authority | Safety | PLAN_HOSTILE_BOUNDARY |
| scaffold_cannot_close_realization | Safety | PLAN_STRUCTURAL |
| qualified_nursery_record_is_not_promoted_source | Safety | PLAN_HOSTILE_BOUNDARY |
| law_changing_candidate_cannot_enter_realization_preserving_lane | Safety | PLAN_LAW_CHANGE_REFUSAL |
| search_budget_is_declared_and_receipted | Conformance | PLAN_STRUCTURAL |
| search_and_holdout_sets_are_disjoint | Conformance | PLAN_STRUCTURAL |
| failed_holdout_candidate_is_quarantined | Refinement | PLAN_HOLDOUT_QUARANTINE |
| candidate_cannot_modify_frozen_judge | Safety | PLAN_HOSTILE_BOUNDARY |
| later_green_cannot_bless_unresolved_dependency | Safety | PLAN_FRONTIER_EXPLORATION |
| specialized_plan_benchmark_is_advisory_only | ResourceEnvelope | PLAN_BENCHMARK_ADVISORY |
<!-- VERIFICATION-PLANS:END -->

## 9. Ownership

```text
DEC-077   verification axes, coverage strength, and claim kinds
DEC-078   enforcement posture and runtime conformance lock
DEC-073   verification independence wording (superseded here by explicit basis law)
DEC-075   durable-history and proof-disposition immutability (refined by section 6)
docs/17   retry/oscillation prose, now carried by the NonOscillation claim kind
docs/36   Tier 0 cross-run independence (bootstrap concern, analogy only here)
spec/verification/     F2 home of the typed basis, layered axes, and runtime matrix
spec/proof/types.rs    counts_green seam renamed in F2 for method/coverage strength
spec/tier0_cross_run/     analogous self-corroboration refusal; never imported here
```

This document is clean-room operating law. It authors
SEED-LAYERED-DYNAMIC-VERIFICATION, SEED-MODEL-TRACE-SEPARATION, and
SEED-RUNTIME-CONFORMANCE-NO-REWRITE, and is backed by `DEC-077` and `DEC-078`.
It states law, not a new durable behavior obligation, and introduces no `LEG-*`
row.

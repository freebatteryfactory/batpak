---
status: AUTHORITATIVE
contract_id: BP-SPROUTING-1
authority_scope: candidate sprouting, immutable nursery lineage, bounded search, evaluation-set roles, realization-preserving versus law-changing promotion, and generate-wide/qualify-deep frontier qualification
supersedes: candidate and campaign wording previously implied only through DEC-048, DEC-062, DEC-073, and DEC-075
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Sprouting, Nursery, and Promotion

This document is clean-room operating law: how a candidate is sprouted, where
its lineage is persisted, how a search is bounded, which evaluation roles keep
oracles independent, how a promotion is classified, and how authority advances
through a speculative tree without granting stale bytes squatters' rights. It
freezes the campaign vocabulary before the typed lifecycle hardens. It does not
implement the search loop, the materializer, or the judge runtime; those are
later gates (F2/F3/F4) named concretely below.

## 1. Candidate is not authority

A generated, machine-assisted, searched, transferred, repaired, or
human-authored candidate remains NONCANONICAL until exact qualification and
promotion. Candidate origin confers NO authority. This is
SEED-CANDIDATE-NOT-AUTHORITY (derives from DEC-079 and DEC-080, refines
SEED-ONE-OWNER).

Candidate origin is recorded TRUTHFULLY — including a machine-assisted-synthesis
origin and a repair-of-prior-candidate origin — precisely so the architecture
never lies about provenance. Recording a synthesizer confers no authority; that
is exactly what makes the trust sentence structurally true. A truthful synthesis
record and a zero-authority candidate are the same fact stated once.

The typed candidate lifecycle is authored in F3 `spec/sprouting/types.rs`. A
`RealizationPosture` with THREE states exists so a stub cannot pass for a
realization:

```text
Missing     no implementation material present
Scaffold    structurally present but unqualified (a compiling stub)
Candidate   qualification-eligible material under review
```

A compiling `fn f() -> Result<()> { Ok(()) }` stub is a Scaffold —
structurally present, unqualified — and counts in the UNREALIZED denominator. A
binary realized|unrealized graph would go green over a tree of stubs; the third
state is why it cannot. Candidate lineage is carried by required manifest and
receipt fields — parent candidate identities, the source-frontier commitment, the
dependency commitments, the content commitment, the generator or pilot evidence,
the qualification receipts, the reuse key, and the promotion, refusal, and
invalidation receipts — and relations are DERIVED from those required fields, so
no generic `CandidateEvidenceRelation` vocabulary exists.

## 2. Immutable nursery: persistent records, disposable workspaces

The nursery is BOTH immutable and disposable, split by object. It is not a
purely disposable downstream tree, and it is not a persistent nursery of
whole-tree bytes. The ruled resolution divides by what is being stored.

The nursery persists IMMUTABLE, content-addressed lineage RECORDS. Lineage is
carried by REQUIRED manifest and receipt fields, not by a free-form edge graph:

```text
candidate lineage identity                        the CandidateId object
parent candidate identities                       explicit parent CandidateIds on the manifest
source-frontier commitment
dependency commitments
content commitment
generator / pilot evidence
qualification receipts
reuse key
promotion, refusal, and invalidation receipts
```

Every lineage RELATION is DERIVED from those required fields — a parent edge is
the parent-CandidateId field, a reuse edge is the reuse key, an invalidation edge
is the invalidation receipt. There is NO generic `CandidateEvidenceRelation` enum
and no parallel edge vocabulary: the required fields ARE the lineage, and
relations are read back out of them, never authored beside them.

Full speculative WORKSPACES — the materialized `crates/` tree — are DISPOSABLE
rematerializations, never persisted as authority. After a trusted-frontier
promotion the loop:

```text
recomputes transitive staleness
preserves unaffected candidate units
invalidates affected descendants
rematerializes the full workspace
```

This is how the campaign earns compound interest — persisted records plus
transfer reuse plus invalidation — WITHOUT granting stale whole-tree bytes
squatters' rights. Records accrue; workspaces are rebuilt.

## 3. The candidate lineage identity is orthogonal, never a generalization

The candidate lineage identity is orthogonal. It does NOT generalize or
replace the kernel identity quartet or the content and version identities. The
narrow, correct assignment against the live identity catalog is:

```text
candidate lineage identity   one IdentityKind object: CandidateId, owner BP-SPROUTING-1
candidate content            bound independently via ContentDigest or another admitted BindingKind commitment
candidate parents            explicit parent CandidateIds recorded on the candidate manifest
kernel specialization        may additionally carry KernelContractId, KernelImplementationId,
                             QualifiedKernelId, and KernelQualificationReceiptId per their existing meanings
candidate manifest format    independently versioned as CandidateManifestVersion; current schema BATPAK-CANDIDATE-MANIFEST/2 in spec/campaign/
```

`ContentDigest` is a `BindingKind` content commitment, not a version snapshot,
and the candidate manifest's serialized schema is versioned from day one, so its
version identity `CandidateManifestVersion` is minted for the whole versioned
manifest family — candidate identity, candidate content, and candidate
parents are three distinct facts carried by three distinct mechanisms, never one
collapsed axis. A specialized kernel candidate may carry BOTH a candidate lineage
identity (its `CandidateId` object) AND a `KernelImplementationId` (its
implementation under the kernel contract); neither absorbs the other, and the
kernel quartet keeps its existing meanings unchanged.

The current manifest grammar is `BATPAK-CANDIDATE-MANIFEST/2` (E7, DEC-064),
and it splits identity from evidence: the manifest owns ONLY mint-time
identity facts —

```text
candidate id · named proof targets · parent candidate ids · source-frontier
commitment · dependency commitments · content commitment · origin · change
class · realization posture · reuse key
```

Generation, qualification, holdout, promotion, refusal, invalidation, and
escalation are never manifest fields. They are APPEND-ONLY campaign receipts
(`BATPAK-CAMPAIGN-RECEIPT/3`, exactly the ten typed kinds the wire grammar
serializes — generation, qualification, holdout, promotion, refusal,
invalidation, escalation, reuse, fuzz, convergence) stored beside the record
at `nursery/<candidate>/receipts/` and referencing the immutable manifest by
candidate id; no mutating receipt vector and no terminal state lives inside
the immutable content-addressed candidate definition. The causally-first
`generation` receipt (DEC-079) records complete creation provenance at the
mint site, and every promotion receipt is the CONSEQUENCE of completed proof —
assembled from the store only after each named proof target already carries a
naming receipt, never an optimistic bookmark.

`CandidateId` is recomputable from the parsed manifest alone: it is the
SHA-256 of the domain-separated `candidate-id-preimage/2` — the domain line
followed by every manifest line after the `candidate-id` line, in manifest
order, each LF-terminated — so every identity-bearing field sits inside the
preimage. `spec/campaign/types.rs` is the normative owner of the manifest
grammar and both preimage laws; this document does not duplicate them.

A campaign evidence bundle additionally DECLARES its trusted frontier roots
(CL-1). An already-trusted input — for the rehearsal, the unchanged
mini-ledger — is modeled as a ROOT, not a newly promoted candidate: a root IS
its content commitment (its id is that digest), so it carries no nursery
record, no generation or promotion receipt, and no decorative proof rows, and
its fresh standing is earned through the reuse receipt's requalification
evidence. The closure resolves candidate dependencies and reuse edges against
candidates UNION roots, and a root's dependencies are Qualified by definition.

`BATPAK-CANDIDATE-MANIFEST/1` and `BATPAK-CAMPAIGN-EVIDENCE/1` are explicitly
historical F5 evidence: they remain parseable through the historical verifier
arm, and they are NOT admissible for Phase-6 opening. The E7-mechanical
`BATPAK-CAMPAIGN-RECEIPT/2` and `BATPAK-CAMPAIGN-EVIDENCE/2` records are
likewise reclassified historical: their full verifier is retired, a `/2`
bundle is refused, and only the `/1` arm is retained — the live campaign
grammar is `/3`.

The candidate lineage identity is authored in F3 `spec/identities/types.rs` alongside
those kinds. A lineage object is never a shortcut around the identity it must
still earn.

## 4. Bounded search and evaluation-set roles

Every candidate search declares finite bounds and receipts the actual work
performed — SEED-BOUNDED-SEARCH (derives from DEC-081):

```text
candidate bound        finite maximum candidates considered
logical-work bound     finite maximum logical work units
memory bound           finite maximum working memory
monotonic deadline     a monotonic-clock deadline, never wall-clock
```

The search receipts the actual candidates, work, memory, and elapsed monotonic
time against those declared bounds; an unbounded or unreceipted search is
refused.

Search, qualification, holdout, and regression inputs carry explicit
evaluation-set roles. F2/F3 mints:

```text
EvaluationSetRole::Search
EvaluationSetRole::Qualification
EvaluationSetRole::Holdout
EvaluationSetRole::Regression
```

Holdout evidence for a campaign cannot reuse its search inputs —
SEED-HOLDOUT-INDEPENDENCE (derives from DEC-081, refines
SEED-INDEPENDENT-ORACLE). A holdout that overlaps the search set proves nothing
it was tuned against and is refused. Transfer reuse is keyed and requalified
against upstream churn. The reuse key binds the candidate's named proof
targets, its content commitment, its dependency commitments, and the campaign
evidence profile; the key alone NEVER licenses reuse — a reuse receipt
additionally binds the current judge digest, the current source frontier, the
current dependency commitments, and fresh requalification evidence. A reuse
key that no longer matches the current upstream frontier licenses nothing.

## 5. Realization-preserving versus law-changing promotion; repair authority

Promotion distinguishes a realization-preserving change from a law-changing one.
F3 mints:

```text
CandidateChangeClass::RealizationPreserving
CandidateChangeClass::LawChanging
```

Both classes satisfy the SAME conjunctive promotion denominator —
`PromotionRequirement::ALL`, all four requirements on duty at once, never a
subset:

```text
IndependentEvidenceRoute     at least one admitted independent-evidence route
NamedProofTarget             a named proof target the candidate must meet
QualifiedHostileEvidence     qualified hostile evidence against the boundary
AuditablePromotionReceipt    an auditable receipt of the promotion decision
```

Those auditable receipts also reach the release envelope: the release seal's
`CandidatePromotionSet` field (`spec/release/`, `36_PUBLIC_API_CI_AND_RELEASE.md`)
binds the promotion receipts a release carries and is mandatory even when
empty — an empty set states "no candidates promoted", it never disappears
from the schema.

A change class ADDS an authority path; it never REPLACES a common requirement. A
realization-preserving candidate satisfies all four through a mechanical or
bounded-search lane. A law-changing candidate satisfies all four AND additionally
requires explicit amendment of the actual semantic owner — the owning DEC, the
owning contract, the affected identity or version, compatibility, migration, and
whole-corpus reconciliation as applicable — and routes to the architect; it
enters the DEC-074 proof-policy process ONLY when proof policy or
candidate-promotion policy itself changes or weakens, not for every law-changing
edit.

Candidate origin (HOW a candidate was produced) and authority posture (WHETHER it
is admitted) are SEPARATE axes: a truthfully recorded origin never implies
admission, and admission is never inferred from origin. Candidate `RepairAuthority`
is:

```text
Mechanical         a deterministic mechanical fix
BoundedSearch      a bounded, receipted search repair
ArchitectRequired  escalated to the architect; no self-service lane
```

There is NO ordinary "judge-repair lane" a candidate may take on its own
courtroom. When the JUDGE itself is wrong — a drifted independent model, a
mis-specified fixture, a stale judge-root view — the campaign STOPS as
ArchitectRequired:

```text
campaign stops as ArchitectRequired
judge / proof-policy amendment proceeds SEPARATELY under DEC-074
affected evidence becomes stale
campaign resumes from the NEW frozen judge snapshot
```

A candidate never edits the court that judges it. ArchitectRequired has a
terminal disposition home so it can never silently drop from the denominator: an
escalated campaign is accounted for as escalated, not quietly absent. DEC-080
(Enforcement, Lock) owns the promotion distinction.

<!-- SPROUTING-PROMOTION-MATRIX:BEGIN generated from spec/promotion/types.rs; spec/sprouting/inventory.rs by bootstrap/project.py; do not edit -->
| Requirement | Owner | Admission basis |
| --- | --- | --- |
| IndependentEvidenceRoute | BP-TESTPAK-1 | DEC-015 |
| NamedProofTarget | BP-TESTPAK-1 | DEC-015 |
| QualifiedHostileEvidence | BP-TESTPAK-1 | DEC-015 |
| AuditablePromotionReceipt | BP-TESTPAK-1 | DEC-015 |

```text
policy surface                 CandidatePromotion
policy-change basis            DEC-074
enforcement gate               G3
release-visibility gate        G9
specialized-plan basis         DEC-073
specialized-plan route         DifferentialImplementation
specialized-plan requirements  IndependentEvidenceRoute; NamedProofTarget; QualifiedHostileEvidence; AuditablePromotionReceipt
```
<!-- SPROUTING-PROMOTION-MATRIX:END -->

## 6. Generate-wide, qualify-deep

A complete speculative implementation tree may be generated BEFORE
implementation gates close, but AUTHORITY advances only through
dependency-ordered qualification — SEED-GENERATE-WIDE-QUALIFY-DEEP (derives from
DEC-082, refines DEC-048 and DEC-075):

```text
later green cannot bless an unresolved earlier frontier
the trusted frontier advances dependency-first
candidate work cannot modify its frozen judge or prior evidence
```

Generation is wide and speculative; qualification is deep and ordered. A green
leaf above an unresolved dependency is not authority — the frontier has not
reached it.

The frontier answers ONE narrow question — admission into the current trusted
frontier — and every candidate holds exactly one of the four `FrontierState`
answers (four-state law ruled in E7):

```text
Qualified            admitted into the current trusted frontier
BlockedByDependency  own qualification otherwise promotable; at least one dependency not qualified
Unqualified          own posture, evidence, semantic result, or authority path does not admit it
Invalidated          evidence or standing that once applied no longer binds current judge or frontier commitments
```

A candidate still in flight, refused on its own content, sitting at a
`Scaffold` or `Missing` realization posture, or stopped as `ArchitectRequired`
classifies as `Unqualified`. Refused, Scaffold, and ArchitectRequired are NOT
frontier variants: those meanings are owned by `CampaignTerminal` and
`RealizationPosture`, and the frontier only records that such a candidate is
`Unqualified`.

## 7. Campaign-root topology and judge integrity

The speculative `crates/` tree cannot live in the tracked checkout.
`FORBIDDEN_TARGET_PATHS` (`spec/architecture/inventory.rs`) refuses `crates/`, `corpus`,
and `fixtures` from ever existing in the tracked tree. The campaign lives in an
EXTERNAL campaign root with physically disjoint roots:

```text
judge/      physically disjoint, read-only where practical, content-addressed
candidate/  writable
evidence/   append-oriented
nursery/    immutable / content-addressed
```

Digest binding DETECTS and INVALIDATES judge mutation: a candidate that mutated
a judge produces evidence that cannot verify. It does NOT make filesystem writes
"unrepresentable." Types do not make arbitrary filesystem writes impossible.
Isolation comes from the COMBINATION:

```text
physically disjoint roots
write containment
entry-type-aware before/after snapshots
exact digest binding
root-containment checks
evidence invalidation on judge change
```

The judge root is a content-addressed `FrozenJudgeSnapshot`, and every evidence
artifact binds the exact judge digest it was produced under. DEC-082
(Architecture, Lock) owns the whole-tree topology. The campaign closure graph
enters `GeneratedView::ALL` in F5 when its typed closure owner and real
rehearsal adopter land.

<!-- CAMPAIGN-CLOSURE-GRAPH:BEGIN generated from spec/campaign/types.rs by bootstrap/project.py; do not edit -->
```text
Closure node fields (one node per lineage record)
  candidate             CandidateId
  realization_posture   RealizationPosture
  frontier              FrontierState
  terminal              Option<CampaignTerminal>
  freshness             EvidenceFreshness

Closure edge kinds (derived from required record fields)
  Parent         Read from the `parents` field
  Dependency     Read from a `dependency_commitments` entry
  Reuse          Read from a reuse-key match
  Invalidation   Read from an invalidation receipt

Campaign terminals (denominator home)
  Promoted            remains_in_denominator true
  Refused             remains_in_denominator true
  Invalidated         remains_in_denominator true
  ArchitectRequired   remains_in_denominator true

Frontier states
  Qualified
  BlockedByDependency
  Unqualified
  Invalidated

Evidence freshness
  Fresh
  StaleByJudgeChange
  StaleByDependencyChange
```
<!-- CAMPAIGN-CLOSURE-GRAPH:END -->

## 8. Sprouting vocabulary (generated)

`spec/sprouting/types.rs` owns the frozen campaign vocabulary — candidate origins,
change classes, evaluation-set roles, realization postures, and repair
authorities. This inventory is a generated projection of those typed enums.

<!-- SPROUTING-VOCABULARY:BEGIN generated from spec/sprouting/types.rs by bootstrap/project.py; do not edit -->
```text
Candidate origins
  DeterministicGeneration
  BoundedSearch
  TransferReuse
  HumanAuthored
  MachineAssistedSynthesis
  RepairOfCandidate

Candidate change classes
  RealizationPreserving
  LawChanging

Evaluation set roles
  Search
  Qualification
  Holdout
  Regression

Realization postures
  Missing
  Scaffold
  Candidate

Repair authorities
  Mechanical
  BoundedSearch
  ArchitectRequired
```
<!-- SPROUTING-VOCABULARY:END -->

## 9. Required proof rows

`spec/proof/` owns proof-row identity and membership. docs/24 owns proof-row meaning. This document owns the domain law being pressured:

<!-- PROOF-REQUIREMENTS:BEGIN generated from spec/proof/inventory.rs by bootstrap/project.py; do not edit -->
| Guarantee | Required proof rows |
| --- | --- |
| DEC-079 | candidate_origin_confers_no_authority; scaffold_cannot_close_realization; qualified_nursery_record_is_not_promoted_source |
| DEC-080 | law_changing_candidate_cannot_enter_realization_preserving_lane; bounded_generated_trace_attack_holds_boundary; every_admitted_campaign_obligation_reaches_a_terminal |
| DEC-081 | search_budget_is_declared_and_receipted; search_and_holdout_sets_are_disjoint; failed_holdout_candidate_is_quarantined; planted_semantic_mutant_is_activated_and_killed; deterministic_replay_equality_of_simulated_trace; candidate_search_terminates_within_declared_budget |
| DEC-082 | candidate_cannot_modify_frozen_judge; later_green_cannot_bless_unresolved_dependency; bounded_repair_loop_reaches_stable_qualified_frontier |
| DEC-073 | specialized_plan_benchmark_is_advisory_only |
| DEC-078 | runtime_capture_appends_observed_history_without_rewrite; offline_replay_concludes_conformance_for_captured_history |
| DEC-076 | confirming_rerun_changes_no_authoritative_result |
<!-- PROOF-REQUIREMENTS:END -->

## Ownership

This document is clean-room operating law authoring SEED-CANDIDATE-NOT-AUTHORITY,
SEED-BOUNDED-SEARCH, SEED-HOLDOUT-INDEPENDENCE, and
SEED-GENERATE-WIDE-QUALIFY-DEEP, backed by decisions DEC-079, DEC-080, DEC-081,
and DEC-082. It introduces no new `LEG-*` row.

```text
DEC-079           candidate is noncanonical; origin confers no authority
DEC-080           realization-preserving versus law-changing promotion (Enforcement, Lock)
DEC-081           bounded search and holdout independence
DEC-082           generate-wide/qualify-deep whole-tree topology (Architecture, Lock)
DEC-074           proof-policy amendment process for a wrong judge
DEC-048 / DEC-075 dependency-ordered advancement refined here
spec/sprouting/     F3 candidate lifecycle and RealizationPosture
spec/identities/    F3 candidate lineage identity alongside kernel and content
spec/architecture/   FORBIDDEN_TARGET_PATHS and campaign-root topology
spec/campaign/inventory.rs  E7 campaign evidence profile naming the proof rows a rehearsal must realize
GeneratedView::ALL  F5 CampaignClosureGraph view; typed owner spec/campaign/
```

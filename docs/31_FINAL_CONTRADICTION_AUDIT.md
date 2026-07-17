---
status: AUTHORITATIVE
contract_id: BP-FINAL-AUDIT-1
authority_scope: final cross-document contradiction and debt audit
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Final Contradiction Audit

## Result

The final v1 architecture has one owner and one dependency direction for every package-scale concept. No known contradiction remains between narrative documents and `spec/architecture.rs`.

## Pass 2 defects removed

```text
separate storage package
empty product umbrella
standalone runtime VM package
universal type drawer
universal layer-folder grammar
all runtime libraries flattened to sibling-only core dependencies
package boundaries chosen mainly for purity
review record disagreeing with architecture facts
```

## Retained Pass 2 repairs

```text
document status/supersession
unified availability vocabulary
bidirectional protocol versus dependency direction
bounded long-lived push plus durable pull
honest K3 and NC1 scope
web/server/embedded ports
no_std + alloc semantic qualification for BatPak and SyncBat
BatQL compiler versus runtime execution
role-based TestPak
artifact plane
schema/codec/layout separation
delivery topology split
identity/time/navigation taxonomy
WorldImage composition
```

Each now lives under the recovered SyncBat/BatPak architecture.

## Cross-owner checks

- BatPak owns ProgramImage/WorldImage contracts and `.vpak` framing; PakVM owns executable validation/interpretation.
- BatQL compiles images without depending on SyncBat.
- SyncBat runtime owns restart legality; Bvisor owns attempt mechanics.
- PakVM cannot access host mechanisms except through Bvisor-mediated ports.
- `.fbat` remains BatPak source truth; `.vpak` is immutable executable packaging.
- NetBat carries SyncBat entrypoints but cannot interpret proof or domain meaning.
- TestPak owns Repo IR and commands; Muterprater owns mutation only.
- MacBat owns compile-time generation; it does not own runtime semantic types.

## Debt traps explicitly checked

```text
no dual old/new product model
no compatibility writer for legacy format
no “temporary” crate with no deletion condition
no duplicate ID or error taxonomy
no raw dependency type in public law
no hidden async runtime
no ambient std dependency in semantic profiles
no unledgered or misplaced unsafe kernel
no cache as authority
no self-generated expected result as sole oracle
no unbounded push memory
no generic timestamp/cursor
no cross-attempt result attachment
no outcome-unknown laundering
no unresolved architecture marker in authoritative docs
no operator relationship defined in two places (spec/operators.rs is the one operator authority; grammar/precedence/formatting/spoken tables are generated and independently re-audited)
no undefined grammar nonterminal referenced by a normative fence (grammar 15/15.2/15.3 is closed)
no proof disposition collapsed on projection (the five states VERIFIED, LEGACY_WEAK, UNVERIFIED, PROOF_UNAVAILABLE, PROOF_INVALID pass through unchanged)
no approximate value granted silent numeric authority (crossings require a receipted Quantize or IntervalDecision; DEC-069, docs/37)
no numeric classification collapsed into availability (a known NaN is not Missing or Pending; the axes stay orthogonal)
no exact/approximate authority boundary flattened into one universal numeric type
no universal hand-authored guarantee ledger (SEED/LEG/DEC/architecture/qualification keep native authority; the Guarantee Graph is derived, DEC-070)
no guarantee lifetime conflated with deletion condition or active/closed status (DeletionCondition::Never never forces Permanent or Active)
no guarantee relation inferred from prose similarity, shared owner, or document section
no clean_owner text or gate name treated as a successor guarantee edge (UntilSuccessor requires a resolvable typed successor; a gate-closed active obligation is UntilGate)
no bare SEED, DEC, or architecture law Discharges or Closes an active obligation before its qualifying evidence or closure receipt exists at Gate 0
no second gate identity type (spec/gates.rs owns GateId, GateSpec, and the gate inventory; every gate-bearing fact family references &[GateId], never a gate name in prose or a string)
no gate reference unresolvable against the inventory, duplicated within one fact, or out of canonical order
no gate identity authored twice (the docs/25 gate inventory is a generated projection of spec/gates.rs and is independently re-audited; the surrounding gate doctrine stays authored)
no decision without a typed DecisionClass, and no implementation-bearing decision without a typed GateId (Naming and HistoricalReceipt may be ungated; the class decides, never the title or ID range; DEC-072)
no integrity/authenticity/freshness/rollback-resistance collapsed into one verified boolean or one mutually exclusive posture value (they are four independently representable typed claims: IntegrityClaim, AuthenticityClaim, FreshnessClaim, RollbackResistanceClaim; DEC-071, docs/19)
no claim axis inferred from profile identity, witness policy, witness disposition, or another axis (profile identity does not substitute for an explicit claim bundle, and neither does witness disposition)
no refusal carrying a successful claim bundle, and no refusal with partial local evidence rendered as a successful weaker profile (partial evidence never includes freshness or scoped rollback resistance after witness failure)
no successful unanchored result for ExternallyAnchoredHistory + Required (an absent or invalid required witness refuses; there is no fallback success)
no ordered security ladder replacing the four axes (no SecurityPosture, VerificationLevel, AssuranceLevel, or SecurityLevel)
no authenticated-history claim exceeding the selected profile and verified witness posture (InternalConsistency never claims authorship or freshness; SignedHistory without a verified anchor never claims freshness)
no invalid profile/witness-policy pair normalized into a neighbouring profile (SignedHistory + Required is refused, never upgraded to ExternallyAnchoredHistory)
no supplied invalid optional witness rendered as an absent one, and no absent optional witness claiming rollback resistance
no authenticated-history profile, witness, claim-posture, or gate node in the Guarantee Graph (decision class and gates are node metadata, never synthetic nodes or edges)
no second witness authority (spec/proof.rs owns proof-row identity and membership, docs/24 owns proof-row meaning; the docs/21 Required witness column projects the typed relations and is audited, never authoritative; LEG-023)
no ContentDigest equality accepted as EventCommitment equality, and no verified-set membership accepted as immediate-predecessor proof (LEG-023, DEC-052)
no genesis marker resetting an already-started stream, and no derived index selecting the expected authority bytes and then authenticating its own selection
no LEG-023 absorption of the lane-isolation law owned by LEG-050 or the visible-linearization law owned by LEG-067 (it cites them)
no unclassified proof-policy change, and no Weakening without DEC or supersession authority, affected guarantees, old and new boundary, replacement or accepted-risk disposition, hostile qualification, release visibility, and a receipt (DEC-074); weakening the anti-weakening gate itself needs the same package
no proof-policy Neutral claimed without parity evidence (a refactor is not automatically neutral), and no apparently stronger policy that narrows the tested domain or drops denominator units escaping the Weakening class
no mutation result collapsed into pass/fail or killed/not-killed, and no NotActivated, Refused, Unbuildable, TimedOut, or InfrastructureFailure counted as Killed or Survived (DEC-015)
no planned mutation leaving the denominator silently, and no mutation score published without its full result distribution
no Survived without activation evidence, no Killed without a qualified baseline, and no EquivalentCandidate excluded without an independent equivalence witness
no Muterprater as a standalone product, second semantic authority, replacement compiler, or replacement test runner; nextest executes and never owns semantic authority, and compiler-backed tooling is not retired without a qualification receipt
no candidate written directly into src/, tests/, spec/, docs/, or companion/, and no generated candidate grading the evidence that judges it; generation and promotion stay separate admitted actions with separate receipts
no candidate promotes while any member of `spec/promotion.rs` `PromotionRequirement::ALL` is unsatisfied; no production implementation serving as its own sole oracle, and no second full PakVM required to supply independence
no residual SpecializedPlan treated as program authority, canonical source, a ProgramImage, a replacement ISA, a second VM, or the sole oracle (the reference interpreter is the executable semantic authority; DEC-073, docs/07)
no specialization cache whose deletion changes value, availability, truth, decision, completeness, ProofDisposition, effects, effect ordering, authority meaning, or required receipts (it may change performance and WorkObservation only)
no residual reused under a different specialization key, and no key silently omitting a component that can influence the plan or binding a dynamic fact as static
no specialization that pre-executes an effect, bypasses Bvisor admission, mints a capability, advances a durable checkpoint, or resets the logical operation deadline
no faster execution path emitting less proof, explanation, or receipt material than reference execution
no optimized ScanKernel as sole oracle or without qualification and scalar fallback, and no SelectionMask width, AoSoA64 layout, nightly Rust, unsafe intrinsic, or SIMD dependency made a V1 requirement (DEC-073 refining DEC-032/DEC-033)
no mask operation across differing row domains, and no NOT selecting unused high bits beyond the logical row length
no public BatQL specialization, SIMD, mask, AoSoA, or CPU-feature syntax (execution path never changes program meaning)
no durable publication reported as an ordinary operation failure, and no commit knowledge, receipt completeness, or reconciliation posture collapsed into one status (LEG-037, docs/05)
no outcome-unknown or cancelled-after-admission treated as proof of non-commit, and no reconciliation rewriting the original durable event or attempt observation
no atomic typed batch publishing a partial durable subset, and no generic kind-plus-bytes publication envelope as the canonical route (LEG-037)
no logical operation identity collapsed into AttemptId, and no physical attempt state, logical outcome, commit knowledge, and receipt completeness merged into one Outcome (DEC-028, LEG-042)
no cursor unbound from store lineage, generation, source cut, direction, selector, or filters, and no item limit applied after decode or materialization (LEG-028, LEG-029)
no GET lowered to a scan, CHILDREN OF lowered to a subtree walk, or SCAN UNDER lowered to one level
no Contract IR-owned effect family routed by hand-authored kind-plus-bytes dispatch, and no ambient linker, startup-constructor, or process-global route discovery (LEG-046)
no relative inactivity timeout presented as an absolute monotonic deadline, no retry resetting the overall deadline, and no lower repeating mechanism outliving it (LEG-066)
no invisible C0 control character in tracked specification, bootstrap, or contract text (LF and TAB only; a backspace impersonating a word boundary once made a rule unmatchable)
no bootstrap qualification that mutates the signed seed tree it claims to prove, and no materializer output published inside the seed root (5.5E5, docs/23)
```

## Remaining implementation work is not architecture ambiguity

Exact byte/opcode numbers, adapter physics, and measured thresholds are listed in `32_IMPLEMENTATION_CONSTANTS.md`. Their owners, gates, constraints, and proof requirements are fixed.

## Validation rule

If a future audit finds a contradiction, it opens a finding against this contract and the affected domain contract. It does not silently revise generated facts or choose the newer-looking file.

---
status: AUTHORITATIVE
contract_id: BP-RECEIPTS-1
authority_scope: receipt families, capsules, explanations, margins, proof, and offline verification
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Receipts and Explanation

## Receipt law

A receipt records what was admitted, attempted, completed, denied, deferred, persisted, replayed, verified, projected, imported, exported, or inspected. It is structured evidence, not a debug log. Every receipt states the `ReceiptSchemaVersion` it was written under; a reader never guesses a receipt's shape.

## Receipt families

```text
BatPak receipt       journal/store transition and durability
SyncBat receipt      logical turn, effect, checkpoint, or invocation
Bvisor report        physical AttemptId, grants, budgets, observations, terminal
NetBat receipt       transport/session carriage facts where required
TestPak receipt      build, test, mutation, benchmark, proof, release
```

They share small primitives but do not collapse into one universal generic receipt.

## Common primitives

```text
ReceiptId
ContentDigest
Commitment
StoreId / AuthorityGeneration
WorldImageId / WorldInterfaceHash
ProgramImageId
ProcessInstanceId / TurnId / AttemptId
CommitPoint
Completeness
Freshness
ProofDisposition
TerminalDisposition
WorkObservation
```

## World invocation capsule

Every invocation binds, when applicable:

```text
WorldImageId
WorldInterfaceHash
ProgramImageId
EntrypointId
CapabilityGrantHash
InputDigest
SourceCommitPoint
TurnId
AttemptId
OutputDigest
EffectBatchDigest
TerminalDisposition
```

Ambient composition state cannot change execution without changing the capsule or causing refusal.

## Structured explanation

Canonical explanation is a tree of stable reason codes, polarity, typed arguments, margins, evidence requirements, source references, and child explanations. English, JSON, terminal, audio, and HTML are renderings.

Prose wording is not policy or query identity.

## Typed margin

Margins preserve units and direction:

```text
Money<USD>
PercentagePoints
Duration
Bytes
Count
```

A passing and failing comparison may both report exact distance from the boundary. Incompatible margin units cannot be combined.

## Proof disposition

```text
Verified
Unverified
ProofUnavailable
ProofInvalid
LegacyWeak
```

A valid result can be unverified. TLS, decoding success, or a cache hit cannot strengthen proof posture.

## Completeness and freshness

Completeness describes required source coverage. Freshness describes the reflected cut relative to the request. They remain orthogonal.

Crypto-shredded or unavailable required inputs produce explicit incompleteness. A fresh result over replayable remnants is not relabeled complete relative to original accepted history.

## Publication and attempt projection (LEG-037, LEG-042)

A receipt joins the logical and physical views without merging them. It must be able to state all of these at once:

```text
LogicalOperationId       stable across retries
AttemptId                unique per physical admission attempt
attempt state            including CancelledBeforeAdmission or CancelledAfterAdmission
logical operation outcome
commit knowledge         KnownAbsent | KnownCommitted | Unknown
receipt completeness     Complete | Incomplete
reconciliation posture   NotRequired | Pending | ReconciledCommitted |
                         ReconciledNotCommitted
ordered command receipts when the operation published a typed batch
```

None of these is derivable from another. A committed publication with an incomplete receipt is not a failure. `Unknown` is not `KnownAbsent`. `CancelledAfterAdmission` is not proof of non-commit.

A reconciliation record appends or references new evidence. It never rewrites the original durable event or falsifies the original attempt observation. The receipt this document owns is the BalancingEvidence coordinate of DEC-075's composition (`02_SYSTEM_MODEL.md`): it joins the logical and physical books without collapsing them.

## Specialization manifest (DEC-073)

A specialization result exposes a manifest or receipt describing:

```text
originating AdmittedProgram identity
complete SpecializationKey
facts bound as static
dynamic inputs still required
branches eliminated
constant expressions folded
schema and path resolutions prebound
kernel bindings selected
capability structure precomputed
checks retained for execution
checks discharged from proven static facts
specialization work consumed
specialization deadline and budget posture
residual content commitment
specialization success, refusal, or fallback posture
```

A check is recorded as discharged only when the static facts and typed law prove it cannot vary for that plan, never because the current implementation usually passes it.

This receipt is evidence about derivation. It does not upgrade the residual into authority. Specialized execution emits the same proof, explanation, and receipt material as reference execution; only `WorkObservation` differs, and it names which path ran.

## Outcome uncertainty

External effects may end with `OutcomeUnknown` after a crash or lost observation. That is not failure and not success. Reconciliation follows the effect's recovery class and durable evidence.

## Authenticated-history posture (DEC-071)

Every authenticated-history verification result preserves each of these, unmerged:

```text
selected AuthenticatedHistoryProfile
selected WitnessPolicy
exact WitnessDisposition

IntegrityClaim                        the four claim axes are independent and
AuthenticityClaim                     each is stated explicitly; none is a
FreshnessClaim                        summary of the others
RollbackResistanceClaim

store lineage identity
generation identity
history or accumulator commitment
signer identity and verification posture   when the profile signs
witness identity and exact scope           when a witness participates

ProofDisposition                      passed through, never upgraded or collapsed
success or refusal outcome
verification or refusal reason
```

The four claims are separate because a real result must assert several of them at once. An `InternalConsistency` success says integrity is verified **and** rollback resistance is unavailable, simultaneously. A single posture value cannot say both, so no single value is used.

No projection may infer an omitted claim axis from the profile name, the witness policy, the witness disposition, or another claim axis. A consumer reads the axis it needs; it never reverse-engineers security meaning.

## Success versus refusal (DEC-071)

`AuthenticatedHistoryClaims` is a **successful** claim bundle. A refusal is never forced into one:

```text
success   complete claims bundle, all four axes stated
          exact WitnessDisposition, profile, policy, identity, proof fields

refusal   no final claim bundle
          optional partial evidence: only sub-results that independently
          succeeded, kept for diagnosis
          exact WitnessDisposition or failure reason, profile, policy,
          identity, proof fields
```

Partial refusal evidence may carry local integrity or signed-history results that genuinely verified. It may never carry a freshness or scoped rollback-resistance claim after witness verification failed, and its presence never converts a refusal into a success or into a weaker profile's success.

Three pairs that must never render identically:

```text
optional witness absent          WitnessDisposition::NotProvided
                                 success; freshness NotClaimed;
                                 rollback resistance Unavailable

optional witness supplied        WitnessDisposition::Stale | Conflicting |
and invalid                      Unverifiable | CryptographicallyInvalid |
                                 LineageMismatch | GenerationMismatch |
                                 AccumulatorMismatch
                                 refusal naming the exact disposition
--------------------------------------------------------------------------
signed history, no anchor        authenticity SignedHistoryVerified
                                 freshness NotClaimed
                                 rollback resistance Unavailable

verified against a witness       authenticity SignedHistoryVerified
                                 freshness WitnessedGenerationVerified
                                 rollback resistance ScopedToVerifiedWitness
--------------------------------------------------------------------------
successful verification          a complete claims bundle
refusal with partial evidence    no claims bundle; partial evidence only
```

An absent optional witness is a successful unanchored result. A supplied invalid one is a refusal that names why, never degrades into `NotProvided`, and never emits a fallback success receipt that discards the failure.

`ExternallyAnchoredHistory` with a required witness has **no successful unanchored result at all**. An absent or invalid required witness refuses; it does not fall back to a weaker success.

No formatter, transport, receipt, explanation, or release note may upgrade or collapse a claim. Rendering is projection, not promotion.

## Sealing

Receipts are canonicalized before hashing. The hash commits to semantic structured fields, not unstable diagnostic prose. Signatures, inclusion proofs, and external anchors are separate strengthening layers.

## Offline verification

Verification may occur without opening the live store when the receipt bundle carries the required contracts, digests, keys, proof references, and source commitments. The verifier reports exactly which claims it established.

## Numeric receipts (DEC-069 / docs/37)

`QuantizationReceipt` records every approximation-to-authority crossing: source interval, target profile, rounding mode, exact/inexact disposition, and discarded-remainder loss evidence. Approximation evidence — format identity, raw bits, classification, provenance — and typed margins (unit, direction, distance interval, and whether zero lies inside the margin) project through the receipt and explanation surfaces and never upgrade or collapse the recorded proof disposition.

## Refusals are API (5.5E1)

Every admission refusal and audit finding names, where structurally
available: the violated law, its typed owner, the offending value or
boundary, and the repair direction. This is not a universal error object —
diagnostics keep their native types — but enough structure that the CLI,
documentation, agents, and receipts can each project an actionable
explanation. An error message a reader must reverse-engineer is a defect of
the same species as an unowned semantic fact.

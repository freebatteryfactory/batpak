---
status: AUTHORITATIVE
contract_id: BP-RECEIPTS-1
authority_scope: receipt families, capsules, explanations, margins, proof, and offline verification
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Receipts and Explanation

## Receipt law

A receipt records what was admitted, attempted, completed, denied, deferred, persisted, replayed, verified, projected, imported, exported, or inspected. It is structured evidence, not a debug log.

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

## Outcome uncertainty

External effects may end with `OutcomeUnknown` after a crash or lost observation. That is not failure and not success. Reconciliation follows the effect's recovery class and durable evidence.

## Authenticated-history posture (DEC-071)

Every authenticated-history verification result preserves each of these, unmerged:

```text
selected AuthenticatedHistoryProfile
selected WitnessPolicy
exact WitnessDisposition
exact HistoryClaimPosture
store lineage identity
generation identity
history or accumulator commitment
signer posture                        when the profile signs
external witness identity and scope   when a witness participates
verification or refusal reason
ProofDisposition                      passed through, never upgraded or collapsed
```

Two renderings that must never be identical:

```text
optional witness absent      WitnessDisposition::NotProvided
                             success, authenticated history verified,
                             rollback resistance unavailable

optional witness supplied    WitnessDisposition::Stale | Conflicting |
and invalid                  Unverifiable | CryptographicallyInvalid |
                             LineageMismatch | GenerationMismatch |
                             AccumulatorMismatch
                             refusal, naming the exact disposition
```

An absent optional witness is a successful unanchored result. A supplied invalid one is a refusal that names why, preserves independently established local signed-history sub-results for diagnosis, and never degrades into `NotProvided` or a success receipt that discards the failure.

Equally, a locally valid signed history and an externally anchored history never render identically: the first is `AuthenticatedHistoryVerifiedNoFreshnessClaim`, the second `ExternallyAnchoredForThisWitnessedGeneration`, scoped to the exact witness that verified.

No formatter, transport, receipt, explanation, or release note may upgrade a claim posture. Rendering is projection, not promotion.

## Sealing

Receipts are canonicalized before hashing. The hash commits to semantic structured fields, not unstable diagnostic prose. Signatures, inclusion proofs, and external anchors are separate strengthening layers.

## Offline verification

Verification may occur without opening the live store when the receipt bundle carries the required contracts, digests, keys, proof references, and source commitments. The verifier reports exactly which claims it established.

## Numeric receipts (DEC-069 / docs/37)

`QuantizationReceipt` records every approximation-to-authority crossing: source interval, target profile, rounding mode, exact/inexact disposition, and discarded-remainder loss evidence. Approximation evidence — format identity, raw bits, classification, provenance — and typed margins (unit, direction, distance interval, and whether zero lies inside the margin) project through the receipt and explanation surfaces and never upgrade or collapse the recorded proof disposition.

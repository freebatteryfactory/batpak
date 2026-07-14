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

## Sealing

Receipts are canonicalized before hashing. The hash commits to semantic structured fields, not unstable diagnostic prose. Signatures, inclusion proofs, and external anchors are separate strengthening layers.

## Offline verification

Verification may occur without opening the live store when the receipt bundle carries the required contracts, digests, keys, proof references, and source commitments. The verifier reports exactly which claims it established.

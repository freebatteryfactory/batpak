---
status: AUTHORITATIVE
contract_id: BP-IDENTITY-TIME-NAV-1
authority_scope: IDs, entropy, UUID succession, Coordinate, path/navigation, time/order, TurnId and AttemptId
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Identity, Time, and Navigation

## Root law

Same representation does not imply same identity. A crate boundary does not mint a duplicate passport.

## ID and entropy contracts

BatPak owns branded IDs, domain separation, text/wire/display law, layout version, clock-regression policy, and `IdSource`/`EntropySource` request contracts. Host profiles supply operating-system, browser, hardware, or deterministic TestPak entropy.

BatPak never invents its own entropy primitive.

## UUID succession

Externally interoperable IDs may preserve UUIDv7-compatible bits under a named `IdLayout`. The UUID crate remains a differential oracle until version/variant bits, ordering, same-time sequencing, regression, collision budget, native/browser parity, and language-neutral goldens close.

## Identity stack

```text
ContractId
SchemaId
CodecId
LayoutId
MaterializationId
StoreId
AuthorityGeneration
ProgramImageId
WorldImageId
WorldInstanceId
ProcessInstanceId
TurnId
AttemptId
ReceiptId
ContentDigest
Commitment
```

No identity answers two questions.

## Coordinate

`Coordinate` remains public BatPak vocabulary for a logical event/process location. It is not a filesystem path, network address, physical shard, wall time, or generic cursor.

Its components, normalization, and bounds are contract-owned.

## Navigation types

```text
WorldPath        capability namespace navigation
ProjectionPath   derived ordered-tree navigation
Coordinate       event/process logical location
StreamPosition   position in one stream/lane
CommitPoint      frozen durable journal cut
PageCursor       one query/source/generation continuation
```

They may share a segment algebra while remaining distinct contracts.

## Time and order

```text
ObservedWallTime     external observation
MonotonicDeadline    local elapsed-time deadline
Hlc                  causally monotone approximate chronology
GlobalSequence       physical journal commit order
CommitPoint          durable frozen cut
StreamPosition       logical stream/lane position
DagPosition          explicit predecessor topology
```

HLC enables temporal range pruning, approximate chronology, lag observation, and tile min/max bounds. It does not prove durability, replace causation edges, measure elapsed time, or supersede commit sequence.

## TurnId

TurnId commits to ProcessInstanceId, ProcessContract version, and exact input start/end frontiers. It is stable across replay of the same logical turn and seeds output/effect idempotency.

## AttemptId

Each physical execution receives a fresh AttemptId. Stale responses and reports cannot cross attempts even when world, program, turn, or plan identities match.

## Deterministic simulation

TestPak injects clock, entropy, ID, and schedule responses. Same seed, inputs, contracts, and declared deterministic profile produce the same identity and transition trace.

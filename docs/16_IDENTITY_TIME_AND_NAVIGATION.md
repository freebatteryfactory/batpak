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

## Version identities (DEC-064)

Version is not one generic type crossing subsystem boundaries. Each format and protocol carries its own distinct version identity:

```text
BatQlLanguageVersion
ProgramImageVersion
WorldImageVersion
PakVmIsaVersion
FbatFormatVersion
BatTaggedRecordVersion
NetBatProtocolVersion
KernelManifestVersion
ReceiptSchemaVersion
SchemaVersion
```

Distinct version types do not typecheck when substituted for one another. Canonical bytes carry their own owning format/version identity.

`ProgramImageId` commits to the canonical ProgramImage bytes, including its version. Compiler implementation/version is provenance, not identity: two qualified compilers emitting identical canonical bytes produce the same `ProgramImageId`. `WorldImageId` commits to its linked ProgramImages, contracts, interfaces, and WorldImage version. NetBat protocol negotiation is independent of `PakVmIsaVersion` and `WorldImageVersion` — a transport version never upgrades the ISA or the image.

Named hostile fixtures (proof owner TestPak; gates G2/G5/G7):

```text
program_image_id_is_independent_of_compiler_provenance
distinct_version_types_do_not_typecheck_when_substituted
netbat_version_does_not_upgrade_pakvm_isa
```

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

## HLC query ordering and range (DEC-061)

HLC is causal/chronological evidence. It is not a total order and never becomes journal-progress authority. Because two events may share or not compare on HLC alone, `ORDER BY HLC` canonicalizes and lowers to a complete key:

```text
within one journal:   HLC, then GlobalSequence
across journals:      HLC, then StoreId, then GlobalSequence
```

`GlobalSequence` is the physical journal commit sequence, so the resulting order is total and deterministic. HLC range predicates are half-open:

```text
DURING HLC start THROUGH HLC end   →   [start, end)
```

HLC remains valid for chronology, temporal filtering, tile pruning, and lag observation. It remains forbidden for durability coverage, cursor resume, frontier advancement, commit identity, deadline measurement, and implicit causation. When causal HLC evidence is absent, the result is `Unavailable`; `ObservedWallTime` is never silently promoted into `Hlc`.

Named hostile fixtures (proof owner TestPak; gates G2/G3):

```text
order_by_hlc_uses_commit_tiebreak
cross_store_hlc_order_is_total
hlc_range_is_half_open
frontier_progress_never_uses_hlc_order
observed_wall_time_is_not_promoted_to_hlc
```

## TurnId

TurnId commits to ProcessInstanceId, ProcessContract version, and exact input start/end frontiers. It is stable across replay of the same logical turn and seeds output/effect idempotency.

## AttemptId

Each physical execution receives a fresh AttemptId. Stale responses and reports cannot cross attempts even when world, program, turn, or plan identities match.

## Deterministic simulation

TestPak injects clock, entropy, ID, and schedule responses. Same seed, inputs, contracts, and declared deterministic profile produce the same identity and transition trace.

---
status: AUTHORITATIVE
contract_id: BP-IDENTITY-TIME-NAV-1
authority_scope: IDs, entropy, UUID succession, Coordinate, path/navigation, time/order, TurnId and AttemptId
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Identity, Time, and Navigation

## Root law

Same representation does not imply same identity. A crate boundary does not mint a duplicate passport.

## ID and entropy contracts

BatPak owns branded IDs, domain separation, text/wire/display law, layout version, clock-regression policy, and `IdSource`/`EntropySource` request contracts. Host profiles supply operating-system, browser, hardware, or deterministic TestPak entropy.

BatPak never invents its own entropy primitive.

Branded means the owning contract rides the type: a `ContractId` names one declared contract, and no other identity substitutes for it.

## UUID succession

Externally interoperable IDs may preserve UUIDv7-compatible bits under a named `IdLayout`. The UUID crate remains a differential oracle until version/variant bits, ordering, same-time sequencing, regression, collision budget, native/browser parity, and language-neutral goldens close.

## Identity stack

The identity catalog answers exactly one question: which semantic, instance,
event, operation, or evidence object. Each entry names its canonical semantic
owner. Spellings, order, and owners are projected from the typed catalogs.

<!-- IDENTITY-CATALOG:BEGIN generated from spec/identities.rs by bootstrap/project.py; do not edit -->
```text
ContractId                     BP-IDENTITY-TIME-NAV-1
SchemaId                       BP-SCHEMA-CODEC-1
CodecId                        BP-SCHEMA-CODEC-1
LayoutId                       BP-SYSTEM-MODEL-1
MaterializationId              BP-STORAGE-TILES-1
StoreId                        BP-STORAGE-TILES-1
EventId                        BP-STORAGE-TILES-1
LogicalOperationId             BP-IDENTITY-TIME-NAV-1
ProgramImageId                 BP-IDENTITY-TIME-NAV-1
ContractImageId                BP-WORLD-PORTS-1
AuthorityImageId               BP-STORAGE-TILES-1
WorldImageId                   BP-IDENTITY-TIME-NAV-1
WorldInstanceId                BP-WORLD-PORTS-1
ProcessInstanceId              BP-SYNCBAT-1
TurnId                         BP-IDENTITY-TIME-NAV-1
AttemptId                      BP-BVISOR-1
ReceiptId                      BP-RECEIPTS-1
EntrypointId                   BP-WORLD-PORTS-1
InvocationId                   BP-WORLD-PORTS-1
CorrelationId                  BP-STORAGE-TILES-1
TileId                         BP-STORAGE-TILES-1
KernelContractId               BP-PUBLIC-API-CI-RELEASE-1
KernelImplementationId         BP-PUBLIC-API-CI-RELEASE-1
QualifiedKernelId              BP-PUBLIC-API-CI-RELEASE-1
KernelQualificationReceiptId   BP-PUBLIC-API-CI-RELEASE-1
KeyId                          BP-CRYPTO-SECRET-1
KeyBackendId                   BP-CRYPTO-SECRET-1
RotationId                     BP-CRYPTO-SECRET-1
RewrapId                       BP-CRYPTO-SECRET-1
UnitId                         BP-NUMERIC-1
NumericProfileId               BP-NUMERIC-1
FloatFormatId                  BP-NUMERIC-1
ApproximationProfileId         BP-NUMERIC-1
WideExactProfileId             BP-NUMERIC-1
QuantizationPolicyId           BP-NUMERIC-1
RoundingModeId                 BP-NUMERIC-1
```
<!-- IDENTITY-CATALOG:END -->

No identity answers two questions.

## Generation identities

A generation answers "which evolution or authority generation", never which
object. `AuthorityGeneration` lives here, not in the identity stack: a
generation of authority is not an object passport.

<!-- GENERATION-CATALOG:BEGIN generated from spec/identities.rs by bootstrap/project.py; do not edit -->
```text
AuthorityGeneration            BP-STORAGE-TILES-1
KeyGeneration                  BP-CRYPTO-SECRET-1
ProcessGeneration              BP-SYNCBAT-1
WorldGeneration                BP-WORLD-PORTS-1
MaterializationGeneration      BP-STORAGE-TILES-1
```
<!-- GENERATION-CATALOG:END -->

## Binding commitments

A binding proves a relationship to bytes, an interface, or history; it is
never the object's identity. `ContentDigest` and `Commitment` live here, not
in the identity stack. A binding may reference a `CommitPoint`; it defines no
CommitPoint comparison, chronology, or frontier law.

<!-- BINDING-CATALOG:BEGIN generated from spec/identities.rs by bootstrap/project.py; do not edit -->
```text
ContentDigest                  BP-STORAGE-TILES-1
Commitment                     BP-IDENTITY-TIME-NAV-1
EventCommitment                BP-STORAGE-TILES-1
CommitmentDigest               BP-CRYPTO-SECRET-1
WorldInterfaceHash             BP-WORLD-PORTS-1
KernelInterfaceHash            BP-PUBLIC-API-CI-RELEASE-1
CapabilityGrantHash            BP-BVISOR-1
InputDigest                    BP-RECEIPTS-1
OutputDigest                   BP-RECEIPTS-1
EffectBatchDigest              BP-SYNCBAT-1
```
<!-- BINDING-CATALOG:END -->

## Version identities (DEC-064)

Version is not one generic type crossing subsystem boundaries. Each format and protocol carries its own distinct version identity, with its canonical semantic owner:

<!-- VERSION-CATALOG:BEGIN generated from spec/identities.rs by bootstrap/project.py; do not edit -->
```text
BatQlLanguageVersion           BP-BATQL-LANGUAGE-1
ProgramImageVersion            BP-IDENTITY-TIME-NAV-1
WorldImageVersion              BP-IDENTITY-TIME-NAV-1
PakVmIsaVersion                BP-PAKVM-ISA-1
FbatFormatVersion              BP-STORAGE-TILES-1
BatTaggedRecordVersion         BP-SCHEMA-CODEC-1
FrameVersion                   BP-STORAGE-TILES-1
NetBatProtocolVersion          BP-NETBAT-1
KernelManifestVersion          BP-PUBLIC-API-CI-RELEASE-1
ReceiptSchemaVersion           BP-RECEIPTS-1
SchemaVersion                  BP-SCHEMA-CODEC-1
LayoutVersion                  BP-SYSTEM-MODEL-1
```
<!-- VERSION-CATALOG:END -->

The catalog answers which independently versioned identities exist and who owns them. The owning contracts answer what each versions. FrameVersion is reserved for the EventFrame envelope. FbatFormatVersion is the .fbat container version. BatTaggedRecordVersion is the tagged payload-record version. LayoutVersion is the physical-layout version. Sharing an integer representation or a migration does not permit one of these types to cross another boundary.

Distinct version types do not typecheck when substituted for one another. Canonical bytes carry their own owning format/version identity.

`ProgramImageId` commits to the canonical ProgramImage bytes, including its `ProgramImageVersion`. Compiler implementation/version is provenance, not identity: two qualified compilers emitting identical canonical bytes produce the same `ProgramImageId`. `WorldImageId` commits to its linked ProgramImages, contracts, interfaces, and WorldImage version. NetBat protocol negotiation is independent of `PakVmIsaVersion` and `WorldImageVersion` — a transport version never upgrades the ISA or the image.

`spec/identities.rs` owns WHICH version identities exist and their owner mapping; this document owns their semantic distinctions and the identity commitment laws above. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere.

`spec/proof.rs` owns proof-row identity and membership. docs/24 owns proof-row meaning. This document owns the domain law being pressured:

<!-- PROOF-REQUIREMENTS:BEGIN generated from spec/proof.rs by bootstrap/project.py; do not edit -->
| Guarantee | Required proof rows |
| --- | --- |
| DEC-064 | program_image_id_is_independent_of_compiler_provenance; distinct_version_types_do_not_typecheck_when_substituted; netbat_version_does_not_upgrade_pakvm_isa |
| DEC-061 | order_by_hlc_uses_commit_tiebreak; cross_store_hlc_order_is_total; hlc_range_is_half_open; frontier_progress_never_uses_hlc_order; observed_wall_time_is_not_promoted_to_hlc |
<!-- PROOF-REQUIREMENTS:END -->

## Non-cataloged identity-shaped terms

Every stable identity-shaped term in the authoritative corpus resolves through
exactly one of five paths: the four catalogs above, or this residue table. A
new unclassified term reds the audit immediately. "Owned elsewhere" cites the
live contract that owns the term outside these catalogs; "not yet admitted by"
cites the standing decision naming the prerequisites — entry happens only by
amending that decision AND joining exactly one catalog, never by the amendment
alone.

<!-- IDENTITY-RESIDUE:BEGIN generated from spec/identities.rs by bootstrap/project.py; do not edit -->
```text
GateId                         owned elsewhere        BP-GATES-1
OperatorId                     owned elsewhere        BP-BATQL-LANGUAGE-1
ProofRowId                     owned elsewhere        BP-GAUNTLET-1
NavigationId                   owned elsewhere        BP-IDENTITY-TIME-NAV-1
EntityId                       owned elsewhere        BP-BATQL-LANGUAGE-1
CustomerId                     owned elsewhere        BP-BATQL-LANGUAGE-1
InvoiceId                      owned elsewhere        BP-BATQL-LANGUAGE-1
OrderId                        owned elsewhere        BP-BATQL-LANGUAGE-1
BorrowerId                     owned elsewhere        BP-BATQL-LANGUAGE-1
TenantId                       owned elsewhere        BP-BATQL-LANGUAGE-1
SystemId                       owned elsewhere        BP-ECS-1
TypeId                         owned elsewhere        BP-ECS-1
CompressionId                  not yet admitted by    DEC-063
```
<!-- IDENTITY-RESIDUE:END -->

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

## Bounded traversal identity (LEG-028, LEG-029)

This document owns traversal request identity, selector, direction, source cut, and cursor binding. `05_STORAGE_FBAT_AND_TILES.md` owns execution, durable visibility, work bounds, and storage complexity. `13_BATQL_CONTRACT.md` owns lowering.

One general bounded directional traversal primitive. The request carries at minimum:

```text
NavigationId                traversal identity
store lineage or entity/region scope
lane
typed selector
optional event kind or category filter
direction
pre-decode item limit
generation-bound cursor      when continuing
frozen source cut
work budget
proof or evidence request
```

### Cursor binding

A cursor binds to its traversal identity, including store lineage, generation, source cut, direction, selector, and filters. A cursor cannot be transplanted across a different store, generation, source cut, selector, filter set, direction, or contract: each of those fails closed rather than resuming against a traversal it never described.

A cursor is not a general-purpose offset. It is evidence of where one specific bounded traversal stopped.

### Page result

```text
ordered entries
page completeness or partiality posture
next cursor              when continuation is lawful
source stamp
WorkObservation
proof or evidence result
```

`MonotonicDeadline` remains the absolute deadline type for port operations (`10_WORLD_IMAGES_AND_PORTS.md`); HLC remains forbidden for cursor resume and deadline measurement (DEC-061).

## Time and order

```text
ObservedWallTime     external observation
TimeDelta            signed difference between two observations (diagnostic)
MonotonicDeadline    local elapsed-time deadline
Hlc                  causally monotone approximate chronology
GlobalSequence       physical journal commit order
CommitPoint          durable frozen cut
StreamPosition       logical stream/lane position
DagPosition          explicit predecessor topology
```

HLC enables temporal range pruning, approximate chronology, lag observation, and tile min/max bounds. It does not prove durability, replace causation edges, measure elapsed time, or supersede commit sequence.

## TimeDelta (5.5E1)

`ObservedWallTime - ObservedWallTime` yields `TimeDelta`: a signed diagnostic
difference between two physical observations. A negative value is lawful
evidence of wall-clock regression or differing observers, never an error to
normalize away. The typed operator rule is owned by `spec/operators.rs`
(`OperatorTypingRule::WallObservationDifference`).

`TimeDelta` is observation evidence, not authority. It is never `Duration`,
`MonotonicDeadline`, journal progress, stream position, causal authority,
`Hlc`, commit identity, cursor authority, or durability evidence. There is no
implicit conversion between `TimeDelta` and `Duration` or into any of the
types above; `TimeDelta` satisfies no budget or timeout law; no generic
operation silently chooses wall order where journal or causal order is
required. Any future conversion is explicitly named and justified by its own
semantic contract. This is the same fence that keeps `ObservedWallTime` from
being promoted into `Hlc`: the physical book never forges the logical book.

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

This document owns the HLC ordering rule, the range law, and the forbidden-use list. In DEC-075's composition (`02_SYSTEM_MODEL.md`) these identities are the ChronologyWitness and DurableOrderWitness coordinates; the composition binds them and owns none of their law. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere.


## TurnId

TurnId commits to ProcessInstanceId, ProcessContract version, and exact input start/end frontiers. It is stable across replay of the same logical turn and seeds output/effect idempotency. The `LogicalOperationId` names the logical operation a turn executes: retries share it while every physical attempt stays distinct.

## AttemptId

Each physical execution receives a fresh AttemptId. Stale responses and reports cannot cross attempts even when world, program, turn, or plan identities match.

## Deterministic simulation

TestPak injects clock, entropy, ID, and schedule responses. Same seed, inputs, contracts, and declared deterministic profile produce the same identity and transition trace.

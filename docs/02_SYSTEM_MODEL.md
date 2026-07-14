---
status: AUTHORITATIVE
contract_id: BP-SYSTEM-MODEL-1
authority_scope: end-to-end semantic, execution, storage, and proof model
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# System Model

## Complete path

```text
Rust declaration ──MacBat──────────────┐
                                      ├→ ContractImage / Schema / facts
BatQL source ──BatQL compiler──────────┘                 │
                                                        ▼
                                                   WorldImage
                                                     (.vpak)
                                                        │
                                      SyncBat world instantiate
                                                        │
                                      runtime selects ProcessTurn
                                                        │
                                      Bvisor admits AttemptId
                                                        │
                                      PakVM interprets program
                                             │                    │
                                      ProgramOutcome         PortRequest
                                             │                    │
                                             └──── Bvisor mediates ┘
                                                        │
                                      runtime applies logical result
                                                        │
                                      EffectBatch commits to .fbat
                                                        │
                                      receipt and checkpoint advance
```

## Semantic graph

```text
Contract IR
├── Schema / Compatibility
├── Codec / Representation
├── Query / Projection
├── Process / Effect
├── Program / World
├── Wire / Interface
└── Proof / Mutation obligations
```

Proof obligations originate with the contract. They do not appear only after implementation.

## Identity stack

```text
ContractId
SchemaId + SchemaVersion
CodecId
LayoutId + LayoutVersion
MaterializationId + Generation
StoreId + AuthorityGeneration
ProgramImageId
WorldImageId + WorldInterfaceHash
ProcessInstanceId + ProcessGeneration
TurnId
AttemptId
ReceiptId
ContentDigest
Commitment
```

Each identity answers one question.

## Contract, tile, turn, attempt, receipt

### Contract

What a concept means, including schema, bounds, lifecycle, effects, compatibility, and proof obligations.

### Tile

A bounded occurrence of one physical layout. A tile may be source truth, mutable authority, derived cache, diagnostic output, or secret authority. Its role is explicit.

### Turn

A logical process step over a frozen input frontier. Turn identity remains stable across replay.

### Attempt

One physical execution of a turn or terminal invocation. Each retry receives a new AttemptId.

### Receipt

Structured evidence describing admission, execution, durability, result, uncertainty, or proof. It is not a log line.

## Runtime worlds

The implementation has several typed worlds with distinct authority:

```text
Contract world   declarations, schemas, lowerings, origins
Store world      events, frontiers, commitments, tiles, authorities
Process world    runnable processes, turns, cursors, effects, checkpoints
Attempt world    capabilities, budgets, attempts, observations, terminal outcomes
Proof world      owners, obligations, fixtures, mutants, receipts
```

They exchange typed artifacts. They are not one shared dynamic object bag.

## Bidirectional information, acyclic dependencies

Runtime information may travel both directions:

```text
turn plan → PakVM
program outcome → runtime
port request → Bvisor/host
port response → suspended PakVM attempt
push notice → observer
pull cursor → journal truth
```

Cargo dependencies remain acyclic. Bidirectional protocol is not circular ownership.

## Process model

A process is:

```text
ProcessContract
+ Coordinate
+ ProcessGeneration
```

Its durable mailbox is a selector plus cursor. A turn consumes bounded input and produces an EffectBatch. Stable idempotency derives from TurnId so replay after output commit but before checkpoint advance returns original receipts.

## External effects

BatPak guarantees exactly-once intent admission within one atomic local append. Physical external execution is at least once unless the capability's protocol proves more. Outcomes remain `Completed`, `Failed`, or `OutcomeUnknown`; uncertainty is never relabeled success.

## Durable truth and views

`.fbat` stores immutable event truth and mutable authority companions. `.vpak` packages immutable executable worlds. Large artifacts live in a content-addressed artifact plane referenced by events and receipts.

## Execution claims

The guest language has strong semantic and capability isolation because it cannot express ambient host operations. An in-process interpreter is not hardware isolation against an interpreter or host-port bug. The security model states that boundary plainly.

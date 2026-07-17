---
status: AUTHORITATIVE
contract_id: BP-SYSTEM-MODEL-1
authority_scope: end-to-end semantic, execution, storage, and proof model
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
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

## Single-writer dual-axis reconciliation (DEC-075)

This is BatPak's execution model, stated once. Async is a host adaptation of
this model, never a second semantic runtime. The pieces are owned by their own
contracts — durable order by the journal law (docs/05, LEG-001/067),
chronology by the time contract (docs/16, DEC-061), turns and attempts by the
runtime and Bvisor (docs/08/09), receipts and reconciliation posture by
docs/14 — and this section owns their composition. The typed owner of the
closed bindings is `spec/reconciliation.rs`; the tables below are generated
from it.

<!-- RECONCILIATION-COORDINATES:BEGIN generated from spec/reconciliation.rs by bootstrap/project.py; do not edit -->
| Role | Carriers | Law |
| --- | --- | --- |
| DurableOrderWitness | GlobalSequence, CommitPoint | one writer establishes exact durable order; nothing else does |
| ChronologyWitness | Hlc, ObservedWallTime | chronology, causal evidence, lag, and temporal pruning only; never promoted into order, commit, retry, or deadline authority |
| LogicalIdentity | TurnId, LogicalOperationId | binds the frozen input cuts and remains stable across replay |
| PhysicalIdentity | AttemptId | fresh for every physical execution; a response binds to exactly one attempt and never crosses attempts |
| BalancingEvidence | Receipt, ReconciliationPosture | joins the logical and physical entries without pretending they are identical; a discrepancy becomes a named posture, never an edit to history |
<!-- RECONCILIATION-COORDINATES:END -->

Every event carries two books — the physical observation and the logical
position — and the receipt is the balancing record that joins them without
pretending they are identical. A discrepancy is never erased; it becomes a
named reconciliation posture, and later reconciliation appends or references
new evidence without rewriting the original event, attempt observation, or
receipt. One type may carry fields efficiently; one name may never own
multiple ordering authorities.

One admitted evaluation over one frozen turn produces both the semantic
projection and the evidence projection. Production never re-evaluates the
expression to construct its receipt; TestPak proves both projections against
independent references that do not grade themselves through the paired
implementation.

The async escape hatch is a reconciliation protocol, not a hidden future:

```text
PakVM step
-> NeedPort(request)
-> host mechanism
-> PortResponse or durable attempt evidence
-> resume the same attempt, or reconcile after interruption
```

A host driver may block, poll, suspend, or use language-level await. Those
are physical driver strategies only: the same request identity, TurnId,
AttemptId, capability, deadline, result type, receipt fields, recovery class,
and reconciliation law apply under every driver, and losing an in-memory
waiter loses no semantic identity.

Retry, resume, compensation, or terminal refusal is selected only from the
typed signal classification:

<!-- RECONCILIATION-RETRY:BEGIN generated from spec/reconciliation.rs by bootstrap/project.py; do not edit -->
```text
admissible for retry, resume, compensation, or refusal:
    DeclaredRecoveryClass
    DurableIntent
    AttemptEvidence
    CommitKnowledge
    ReceiptCompleteness
    OverallMonotonicDeadline
    RuntimeRestartAuthorization

never sufficient on their own:
    ElapsedWallTime
    ProcessDeath
    MissingAcknowledgement
    MissingInMemoryWaiter
```
<!-- RECONCILIATION-RETRY:END -->

The executable witnesses for this law are owned by `docs/24_GAUNTLET.md`
(carried by `DEC-075`).

## Durable truth and views

`.fbat` stores immutable event truth and mutable authority companions. `.vpak` packages immutable executable worlds. Large artifacts live in a content-addressed artifact plane referenced by events and receipts.

## Execution claims

The guest language has strong semantic and capability isolation because it cannot express ambient host operations. An in-process interpreter is not hardware isolation against an interpreter or host-port bug. The security model states that boundary plainly.

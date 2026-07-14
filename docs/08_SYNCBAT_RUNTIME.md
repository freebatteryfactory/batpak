---
status: AUTHORITATIVE
contract_id: BP-SYNCBAT-1
authority_scope: SyncBat crate architecture, logical runtime, PakVM/Bvisor integration, process and effect law
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# SyncBat Runtime

## Identity

SyncBat is the complete runtime crate. It is one heartbeat with five explicit internal planes:

```text
runtime
pakvm
bvisor
world
port
```

These are not separate packages and do not create duplicate public types.

## Source tree

```text
src/
├── runtime.rs
├── runtime/
│   ├── process.rs
│   ├── turn.rs
│   ├── schedule.rs
│   ├── delivery.rs
│   ├── checkpoint.rs
│   ├── effect.rs
│   ├── supervision.rs
│   └── recover.rs
├── pakvm.rs
├── pakvm/
│   ├── validate.rs
│   ├── value.rs
│   ├── frame.rs
│   ├── interpret.rs
│   ├── kernel.rs
│   ├── suspend.rs
│   └── work.rs
├── bvisor.rs
├── bvisor/
│   ├── admission.rs
│   ├── capability.rs
│   ├── budget.rs
│   ├── attempt.rs
│   ├── supervise.rs
│   ├── reconcile.rs
│   └── report.rs
├── world.rs
├── world/
│   ├── link.rs
│   ├── validate.rs
│   ├── instance.rs
│   ├── entrypoint.rs
│   └── generation.rs
├── port.rs
└── port/
    ├── journal.rs
    ├── artifact.rs
    ├── clock.rs
    ├── entropy.rs
    └── transport.rs
```

## Build profiles

The default semantic profile is `no_std + alloc` and contains the logical transition core, PakVM reference semantics, Bvisor admission state, WorldImage/instance values, and typed port protocols. It contains no filesystem, socket, thread, process, ambient clock, entropy provider, or browser API.

The `std` profile adds native threaded drivers and host adapters. The browser profile adds host bindings and suspension/resume adapters. Both must conform to the same logical trace and receipt model as the semantic profile.

## Ownership firewall

```text
runtime owns logical legality
pakvm owns program semantics
bvisor owns attempt admission and physical evidence
world owns composition and instance identity
port owns explicit host requests/responses
```

Sharing a crate does not permit one plane to perform another's transition.

Forbidden examples:

```text
PakVM advancing a durable checkpoint
Bvisor minting semantic restart authorization
runtime bypassing Bvisor to execute a program
PakVM calling filesystem/network/clock directly
world redefining BatPak-owned IDs or schema law
```

## Logical process

A process is `ProcessContract + Coordinate + ProcessGeneration`.

Its durable mailbox is a declared selector plus cursor over `.fbat`. Ephemeral notifications only reduce polling latency.

## Turn identity

`TurnId` commits to process identity, process contract version, input start frontier, and input end frontier. Outputs and effect intents derive stable idempotency from TurnId.

If outputs commit but the checkpoint does not advance, replay reconstructs the same TurnId and receives the original receipts instead of duplicating work.

## Turn flow

```text
select runnable process
→ freeze input frontier
→ construct ProcessTurn
→ Bvisor admits attempt and reserves budget
→ PakVM execute/resume
→ Bvisor seals AttemptReport
→ runtime validates logical disposition
→ atomically append EffectBatch/results
→ advance checkpoint
```

## Effect algebra

Effects are typed intents. Runtime owns sequencing, declaration subset checks, idempotency, and checkpoint coupling. Bvisor mediates host-port attempts. BatPak stores intent, attempt, completion, failure, or outcome-unknown evidence.

Every external effect contract declares a recovery class:

```text
IdempotentByKey
QueryableOutcome
Compensatable
AtLeastOnceOnly
NonReplayable
```

Runtime uses that contract and durable evidence to decide whether another attempt is legal.

## Resource exhaustion (DEC-066)

Two distinct semantic outcomes, never collapsed into each other, into a string, or into `INVALID`:

```text
BudgetExceeded      a declared request/program/work bound was exceeded
ResourceExhausted   the host could not satisfy an otherwise legal bounded allocation
```

`INVALID` means a value violated its semantic contract. Resource exhaustion is an execution failure, not a value-availability state. Do not add an overlapping `AllocationRefused` unless a later implementation proves it a distinct public meaning.

Every attacker-influenced allocation follows one order, publishing nothing until it completes:

```text
decode declared length
→ checked arithmetic
→ enforce semantic bound
→ try_reserve
→ initialize
→ publish only after completion
```

Failure must never produce a panic, an abort presented as ordinary refusal, an integer wrap, a partially initialized canonical value, a partially published event or artifact, or a partially advanced checkpoint. This applies to `.fbat` decode/append, `.vpak` validation, BatQL parsing/normalization, PakVM arenas, group/match state, NetBat framing, proof bundles, artifact staging, and tile materialization. Plane-specific errors may wrap the two canonical classes but may not collapse them.

Named hostile fixtures (proof owner TestPak; gates G2/G5/G6):

```text
attacker_length_is_checked_before_reserve
allocation_failure_returns_resource_exhausted
declared_work_overrun_returns_budget_exceeded
resource_exhaustion_never_publishes_partial_event
resource_exhaustion_never_advances_checkpoint
artifact_staging_failure_publishes_no_artifact_ref
pakvm_arena_exhaustion_returns_typed_terminal_disposition
hash_map_iteration_cannot_change_canonical_bytes
```

## Supervision

Logical strategies may include one-for-one, one-for-all, and rest-for-one where the process graph earns them. Restart intensity and windows are typed. A crash is not automatically retryable.

## Delivery

Runtime consumes BatPak-owned delivery contracts for Mailbox, Completion, Broadcast, and Permit topologies. Mechanisms remain private. Guaranteed work uses durable pull. Push remains bounded awareness.

## Drivers

The same logical transition core supports:

```text
cooperative single-threaded drive
threaded native drive
browser suspension/resume drive
deterministic TestPak drive
```

Drivers may differ physically. Their accepted logical traces and receipts must satisfy one conformance model.

## Public surface

The ordinary public API is concept-oriented:

```text
Runtime
RuntimeBuilder
WorldImage / WorldInstance
ProcessContract
ProgramImage
CapabilityGrant
Invocation / InvocationResult
RuntimeReceipt
```

Private protocol details such as VM yields and scheduler directives are not frozen as public API until an independent consumer earns them.

---
status: AUTHORITATIVE
contract_id: BP-BVISOR-1
authority_scope: Bvisor admission, capability, budget, attempts, supervision, and recovery
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Bvisor

## Identity

Bvisor is the capability and attempt membrane inside SyncBat. It is not a portable wrapper over operating-system sandbox APIs and does not execute arbitrary native or Wasm guests in V1.

## Responsibilities

Bvisor owns:

```text
WorldPlan admission
capability grant table
budget reservation and accounting
WorldInstance and generation binding
AttemptId
PakVM invocation and suspension mediation
terminal outcome classification
attempt supervision
startup reconciliation
AttemptReport
```

Bvisor does not own logical process state, checkpoint authority, TurnId semantics, idempotency meaning, or restart legality.

## Admission membrane

Admission validates in an explicit, cheapest-safe order:

```text
image identity and structural validity
principal/delegation policy where required
capability requirements against grants
schema/interface compatibility
proof/freshness requirements
program decision requirements
resource reservation
attempt creation
```

The exact ordered stages are part of the admitted plan identity. Validation never mutates an execution budget; reservation occurs only after all prior checks pass.

## Capability table

Capabilities are typed, rights-bounded, resource-bounded, WorldInstance-scoped, and generation-scoped. A path or slot number alone grants nothing.

PakVM receives opaque capability slots. Only Bvisor resolves them into typed PortRequests.

## Budget model

Bvisor reserves and observes logical units:

```text
instructions
input rows and bytes
decoded bytes
tile bytes
groups and matches
outputs and artifacts
effect count
call depth
suspended-state bytes
```

Host adapters independently enforce physical constraints and report weaker guarantees honestly.

## Attempt lifecycle

```text
Planned
→ Admitted
→ Running
↔ SuspendedOnPort
→ Terminal
→ Reported
→ Reconciled
```

Each physical execution receives a fresh AttemptId. A report or response from one attempt cannot satisfy another, even when world, program, turn, or request identities match.

## Postcondition honesty

A witness type appears only after its postcondition is established:

```text
RequestedCapability ≠ GrantedCapability
ReservedBudget ≠ ConsumedBudget
EffectRequested ≠ EffectAttempted
EffectAttempted ≠ EffectCompleted
BytesWritten ≠ BytesDurable
DigestMatched ≠ AuthorityVerified
```

Bvisor reports observations, not intentions.

## Recovery and reconciliation

At startup Bvisor reconciles nonterminal attempts against durable event/effect evidence, port receipts, and runtime-provided restart authorization.

Bvisor may execute an authorized retry, resume, compensation, or terminal refusal. It may not infer semantic safety from process death or elapsed time.

## Simulation

TestPak drives the same Bvisor transition core with scripted ports, crash points, budget faults, stale responses, and independent ground truth. The report path may not grade itself.

## Deferred foreign execution

A future real adopter may earn a separately qualified external-effect adapter for a native tool. Such an adapter receives immutable content-addressed inputs and returns artifacts plus physical evidence. It does not restore an OS-backend matrix to Bvisor's product identity.

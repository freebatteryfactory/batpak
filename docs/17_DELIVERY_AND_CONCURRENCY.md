---
status: AUTHORITATIVE
contract_id: BP-DELIVERY-CONCURRENCY-1
authority_scope: delivery topologies, push/pull, mechanisms, Flume succession, and runtime work
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Delivery and Concurrency

## Root law

A channel mechanism does not define delivery semantics. BatPak owns four topologies:

```text
Mailbox
Completion
Broadcast
Permit
```

Each contract declares capacity, ordering, overflow, cancellation, wake, durability, and shutdown.

## Completion

One command and one eventual result use a Completion contract, not a public general channel. The mechanism may be a small cell with wait, poll, typed close, and optional host adapter.

## Permit

Capacity admission uses a Permit contract with acquisition, release-on-drop, shutdown, fairness statement, and observed high-water. It does not circulate unit messages through a general queue.

## Broadcast

Lossy awareness declares retained window and lag policy. A future native ring may publish once with per-subscriber read positions. A slow observer receives overrun/drop disposition and never paces the writer unless the contract explicitly says so.

Guaranteed delivery uses journal truth and durable cursors.

## Mailbox

The runtime command lane is bounded, many-producer, one logical interpreter, batch-drain aware, and carries a Completion identity. It distinguishes attempted, reserved, admitted, and completed.

A native candidate may use an admission ring, payload arena, completion table, wake source, and one drain cursor. Lock-free design is not a goal.

## Flume disposition

Flume may remain a private reference mechanism during succession. Completion, Permit, Broadcast, and Mailbox migrate independently. There is no requirement to replace all uses with one homemade channel library.

A native mechanism must pass reference equivalence, schedule/model checking, cancellation/disconnect races, shutdown/panic containment, backpressure semantics, mutation pressure, and real workload evidence.

Raw Flume types do not remain in ordinary public APIs.

## Pull lane

Pull is explicitly bounded by rows, bytes, groups, matches, turn input, work units, and result cardinality. Continuation binds query, source, generation, direction, and last position.

## Push lane

Push may live indefinitely. Memory may not. Every contract names message/batch/buffer bounds, overflow policy, retained window, lag disposition, and durable recovery pull path.

Lossy push is awareness. Durable pull is truth.

## Bidirectional protocol

```text
turn request → PakVM
program result → runtime
port request → Bvisor/host
port response → suspended attempt
push notice → observer
pull cursor → source truth
```

Information can flow both directions without reverse Cargo ownership.

## No hidden async runtime

Production semantic libraries require no Tokio or ambient executor. Browser and async hosts adapt typed suspension/resume at the port boundary. Native hosts may block or drive cooperatively.

## Work observations

Mechanisms report enqueue attempts, admitted items, drain batches, wakeups, spurious wakes, lag/overruns, bytes copied, allocations, contention, and fallback. Wall time supplements, not replaces, logical work.

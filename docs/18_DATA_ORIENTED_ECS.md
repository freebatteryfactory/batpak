---
status: AUTHORITATIVE
contract_id: BP-ECS-1
authority_scope: typed table/system architecture, tiles, cache physics, fusion, and non-goals
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Data-Oriented ECS

## Identity

BatPak uses ECS as a relational/data-oriented implementation algebra, not as a game framework or universal object model.

```text
row identity       branded stable ID
component          typed fact column
layout family      declared physical organization
system             pure or explicit transition
world              bounded fact/table set under one owner
command buffer     EffectBatch or transition intent
```

There is no dynamic `TypeId → Box<dyn Any>` world.

## Semantic versus physical

Contract is the semantic primitive. Representation and layout give it physical form. Tile is a bounded materialization. Tables organize repeated facts. Systems perform explicit transitions.

A contract, proof obligation, or error class does not become a tile merely because ECS is useful elsewhere.

## Package applications

### BatPak

Event locations, frontiers, commitments, projections, and derived indexes may use typed record/column tables. The SIDX successor is the first production column tile.

### SyncBat runtime

Process, cursor, checkpoint, effect, completion, and runnable-state tables may support batched scheduling. One logical actor per heap object or thread is not required.

### PakVM

Value frames, selections, groups, capability slots, and work counters may use bounded typed arenas/tables.

### Bvisor

Attempt, capability, budget, suspension, observation, and terminal columns form an attempt supervision forest across containment, authority, resource, causality, and evidence graphs.

### MacBat

Typed Contract IR remains authoritative. MacBat exports relational facts; it is not rewritten into a generic ECS internally.

### TestPak

Repo IR is a typed fact plane for ownership, dependency, proof, mutation, compatibility, and docs.

## System contract

A system declares:

```text
SystemId
read set
write set
input/output bounds
required witnesses
capability requirements
fallback policy
work counters
proof owner
```

Direct pure functions remain preferred when a table/system abstraction buys nothing.

## Cache and byte physics

Hot paths touch only needed columns. Filters need kind/sequence/visibility/location before payload. Payload decode touches selected byte ranges. Proof verification touches commitment/proof columns.

Schema identity and layout identity remain separate.

## Fold fusion

One frozen traversal may feed multiple projections, index facts, and work counters. The law is equivalence to running every fold independently over the same input cut.

Runtime fusion is encouraged. TestPak retains separate simple-fold oracles.

## Typestate and witnesses

Typestate governs dangerous affine transitions. Orthogonal trust, durability, authority, completeness, and proof facts use witness values/columns. Do not produce hostage types with every cache width and generation as a generic parameter.

## Non-goals

```text
no universal ECS crate
no tile for every temporary
no ambient system registration
no game-loop ownership
no cache-friendly claim without access/work evidence
no custom column engine before a real adopter
```

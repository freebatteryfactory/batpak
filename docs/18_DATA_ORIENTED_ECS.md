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

## Scalar data-flow seam (DEC-073, refining DEC-032/DEC-033)

The V1 optimization primitives are `Column`, `SelectionMask`, `ScanKernel`, `LateMaterialization`, and `WorkObservation`. `ColumnTile` remains an earned physical derived layout (DEC-033), never one universal layout or width, and never authority.

### Column

A typed ordered view over one schema field for one bounded row domain, binding at least:

```text
schema identity
field identity
row-domain identity
source generation or source cut
layout identity
value sort and unit where applicable
```

A columnar layout is derived execution material unless an existing contract explicitly names an authoritative source representation. Column order corresponds to the source row domain: an optimized layout never silently reorders semantic rows.

### SelectionMask

Membership over one exact row domain, binding:

```text
row-domain identity
logical length
source cut or generation
mask representation profile
```

The physical representation is not frozen. It may be one machine word for a small domain, multiple words, or another safe qualified representation. Nothing here constitutionalizes `u64`, 64 rows, `AoSoA64`, or one tile width forever.

`AND`, `OR`, and `NOT` are legal only for masks over the same row domain, logical length, source cut, and compatible representation profile. `NOT` is bounded by the logical row length: unused high bits cannot select nonexistent rows. A mask is derived selection evidence, never durable event authority.

### ScanKernel

```text
consumes   typed Column inputs, typed predicate or admitted scan plan,
           source cut, row domain, work budget, numeric and comparison
           profiles, kernel identity

returns    SelectionMask, completeness posture, WorkObservation,
           proof or evidence posture required by the operation
```

The scalar `ScanKernel` is mandatory and is the reference behavior. No optimized kernel is ever the sole oracle. Approximate numeric inputs remain governed by DEC-069: a scan wrapper does not make raw `ApproximateBinary` comparison lawful.

### LateMaterialization

Gathers only the selected semantic rows or fields required by the result, preserving source row order, row identity, availability distinctions, proof and evidence bindings, and source-cut identity. It may decode a bounded physical block when a qualified layout requires it. It may not materialize an unbounded result and then claim the mask saved work. `WorkObservation` records the actual physical work, including bounded block decode.

### Scalar reference law

V1 requires a safe stable scalar reference implementation, reference mask algebra, reference scan behavior, reference late materialization, and `WorkObservation`. V1 does not require nightly Rust, unsafe target intrinsics, a portable-SIMD dependency, a vendor SIMD dependency, target-specific executable code, or mixed-kind AoSoA layout. Compiler auto-vectorization is an implementation detail, never a semantic dependency or public promise.

### Optimized kernel seam

A future optimized `ScanKernel` is admitted only after scalar reference equivalence, layout qualification, target-profile qualification, operation-count evidence, a representative adopter workload, small and interleaved data checks, no proof/availability/ordering regression, a routing threshold that justifies retaining the kernel, and a release-visible qualification receipt.

Wall-clock speed alone is insufficient. A faster kernel that performs more unbounded work, changes ordering, weakens proof, or loses rare-case behavior is not qualified. An unavailable optimized kernel falls back to scalar reference behavior. Deleting optimized kernels changes performance only; none is required for semantic correctness.

### Work observation

`WorkObservation` exposes, where applicable: execution path, rows considered, columns touched, bytes or bounded blocks decoded, predicate operations, selected row count, materialized row or field count, kernel identity, fallback posture, specialization work, and execution work. No physical counter layout is frozen here; the frozen obligation is that work remains observable and comparable.

## Non-goals

```text
no universal ECS crate
no tile for every temporary
no ambient system registration
no game-loop ownership
no cache-friendly claim without access/work evidence
no custom column engine before a real adopter
```

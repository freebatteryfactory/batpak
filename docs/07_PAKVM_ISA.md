---
status: AUTHORITATIVE
contract_id: BP-PAKVM-ISA-1
authority_scope: PakVM instruction set, executable .vpak section semantics, validation, memory, bounds, and execution
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# PakVM ISA and Executable `.vpak` Sections

## Name separation

```text
PakVM        the typed interpreter and execution machine
.vpak        the immutable executable package format
ProgramImage one typed program
WorldImage   linked executable world
```

The machine name and package extension are not interchangeable.

## Design choice

PakVM uses typed SSA/dataflow with structured regions. It is not an untyped stack machine and not a general-purpose native ISA.

V1 excludes arbitrary backward jumps, general recursion, raw memory access, ambient callbacks, host syscalls, and unbounded loops. Iteration occurs through bounded operators.

## `.vpak` image structure

A `.vpak` package is immutable, sectioned, bounded, and content-addressed. Expected semantic sections include:

```text
header and section directory
string and constant pools
type/schema/codec tables
contract and program images
functions and structured regions
capability requirements
entrypoints and process contracts
world interface
origins and explanation dictionary
proof manifest and signatures
small embedded resources
```

Every offset, count, length, nesting depth, and decoded allocation is validated before executable admission.

BatPak owns the image contracts and canonical framing. `syncbat::pakvm` owns executable validation and interpretation.

## Type table

PakVM values use BatPak's closed value algebra. No generic host object or `Any` value crosses the boundary. Types include fixed primitives, bounded text/bytes, records, enums, options, lists, paths, IDs, decisions, availability, pages, receipts, and declared references.

## Memory model

Programs have no raw pointers or address arithmetic. Values live in validated frames, bounded arenas, and read-only/owned views. Tile access occurs through typed layouts and selection vectors.

Live capability slots are opaque, instance-scoped, generation-scoped, and non-serializable as authority.

## Three instruction algebras

### Formula and decision

```text
literal, binding, field, let, compare, all, any, not,
case, allow, deny, defer, require, margin, explain,
built-in kernel call
```

### Query and dataflow

```text
scan, seek, select, filter, project, fold, group,
aggregate, join, sequence match, sort, page, limit,
materialize tile
```

### Effects

```text
append, append batch, request effect, stage artifact,
emit result, advance checkpoint intent
```

A pure query image cannot contain Effect instructions. The validator rejects the image before execution.

## Built-in kernels

A kernel is content-identified, typed, bounded, deterministic where declared, and implemented in reviewed Rust. It cannot smuggle an ambient host callback. PakVM calls only kernels in the admitted WorldImage and capability closure.

## Suspension

A program may yield a typed `PortRequest`. The attempt retains a bounded suspended state. Bvisor mediates the request, validates the response identity/bounds, and resumes the same AttemptId.

The guest does not see whether a native host blocked, a browser awaited a promise, or an in-memory test returned immediately.

## Work accounting

Validation computes static maxima where possible. Bvisor reserves admissible budgets. PakVM counts actual instructions, rows, decoded bytes, tile bytes, groups, matches, outputs, artifacts, effects, and call depth.

Wall time is supporting evidence, not the sole budget unit.

## Reference and optimized paths

V1 ships a simple canonical interpreter first. Optimized tile-aware execution remains differential against the reference interpreter and simple TestPak models.

## Decision-circuit lowering

Eligible fixed-width decision fragments may lower to a bounded canonical circuit. That circuit is an independent executable lowering, not the whole ISA and not the sole evaluator. `Allow`, `Deny`, and `Defer` remain distinct in the typed decision contract.

## Validation phases

```text
bounded structural decode
→ section and identity validation
→ type and control-flow validation
→ capability/effect closure
→ work-bound validation
→ origin/proof manifest validation
→ ValidatedWorldImage
```

No program runs from merely parsed bytes.

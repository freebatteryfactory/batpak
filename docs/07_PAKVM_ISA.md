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

## ISA version

`PakVmIsaVersion` identifies the instruction-set and encoding revision. It is a distinct type from `.vpak` section identity, `ProgramImageVersion`, and `WorldImageVersion`; the complete version-identity law is owned by `16_IDENTITY_TIME_AND_NAVIGATION.md` (DEC-064). Exact opcode numbers and the ISA version value are G5 implementation constants.

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

### Kernel identity (DEC-062)

`FROM KERNEL @blake3:...` refers to a canonical `KernelManifest`, never a display name, a Rust path, a function pointer, or an arbitrary source-file digest. Four identities are distinct:

```text
KernelContractId          semantic operation and version
KernelInterfaceHash       canonical inputs, outputs, errors, determinism,
                          capabilities, bounds, and cost contract
KernelImplementationId    the exact qualified implementation/build manifest
KernelQualificationReceiptId  proof the implementation satisfies the contract
```

A canonical implementation manifest binds at least:

```text
source-tree or implementation artifact digest
toolchain
target
features
dependency-lock subset
generated-code inputs
build profile
resulting artifact digest where applicable
```

PakVM never resolves a kernel by string name alone. Kernel binding policy inside a WorldImage is owned by `10_WORLD_IMAGES_AND_PORTS.md`.

## Suspension

A program may yield a typed `PortRequest`. The attempt retains a bounded suspended state. Bvisor mediates the request, validates the response identity/bounds, and resumes the same AttemptId.

The guest does not see whether a native host blocked, a browser awaited a promise, or an in-memory test returned immediately.

## Work accounting

Validation computes static maxima where possible. Bvisor reserves admissible budgets. PakVM counts actual instructions, rows, decoded bytes, tile bytes, groups, matches, outputs, artifacts, effects, and call depth.

Wall time is supporting evidence, not the sole budget unit. A declared bound exceeded is `BudgetExceeded`; a failed but legal bounded arena allocation is `ResourceExhausted`; neither is `INVALID`, and neither publishes partial state. The typed resource-exhaustion law is owned by `08_SYNCBAT_RUNTIME.md` (DEC-066).

## Reference and optimized paths

V1 ships a simple canonical interpreter first. Optimized tile-aware execution remains differential against the reference interpreter and simple TestPak models.

## Adaptive residual specialization (DEC-073)

The reference interpreter is the executable semantic authority for admitted programs. It stays small, portable, obvious, complete, and non-optimized where clarity wins. The specializer is not a second interpreter authority, and the residual path is never the sole oracle.

```text
AdmittedProgram                          may execute directly through the reference interpreter
AdmittedProgram + valid SpecializationKey  may derive a SpecializedPlan
SpecializedPlan                          may execute only while every bound identity still matches
reference and specialized execution      produce the same ExecutedResult semantics
```

A `SpecializedPlan` is derived optimization material. It is never program authority, canonical source, a new `ProgramImage`, a replacement ISA, a second VM, or native executable authority. It does not erase or replace its originating `AdmittedProgram`.

### Phase law

```text
ParsedProgram      syntax has been recognized
TypedProgram       operand, result, unit, and sort relationships are valid
AdmittedProgram    capability, effect, bound, proof, and kernel requirements are admitted
SpecializedPlan    derived optimization bound to one AdmittedProgram and one complete
                   specialization key
ExecutedResult     semantic result plus the required orthogonal axes and receipts
```

Illegal transitions:

```text
ParsedProgram   -> ExecutedResult
TypedProgram    -> SpecializedPlan without admission
SpecializedPlan -> ProgramImage
SpecializedPlan -> authority fact
SpecializedPlan reused under a different key
```

### Specialization modes

V1 admits exactly two, and no others:

```text
compile-time residual specialization    may package a verified derived plan alongside
                                        the program profile; this does not make the
                                        plan authoritative

bounded first-use residual specialization
                                        only after parsing, typing, and admission;
                                        under an explicit work budget; under the
                                        absolute monotonic deadline; executing no
                                        application effects
```

Not admitted in V1: unbounded background optimization, ambient profile-guided recompilation, machine-code JIT, executable-memory allocation, self-modifying code, and runtime patching of authoritative `ProgramImage` bytes. A future native-code kernel profile requires its own explicit decision and qualification boundary.

### Specialization key

Every residual plan binds the complete static context that can affect its structure or behavior:

```text
ProgramImageId
WorldImageId
PakVM ISA version
specializer implementation or semantic version identity
residual-plan format identity
schema identities
layout identities
numeric profile identities
operator and qualified-kernel interface identities
qualified-kernel implementation identities where pinned
capability-envelope identity
target profile
qualified CPU-feature profile where relevant
```

A component may be marked not applicable only when it provably cannot influence that plan. It may never be silently omitted. A plan produced under one key cannot execute under another merely because the human-readable programs look similar.

The key must not capture dynamic facts as though they were static: wall-clock time, file descriptor numbers, socket state, thread identity, process-global registry state, ambient environment variables, uncommitted mutable cache state, ephemeral authorization tokens, secret bytes, external witness freshness results, or dynamic input values. A future explicitly qualified mechanism could give such a fact a stable identity and a lawful binding contract; absent that, binding it is forbidden.

### Safety boundary

Specialization may remove repeated structural work: program decoding, schema lookup, layout lookup, operator dispatch, kernel lookup, static path resolution, constant pure expressions, provably unreachable pure branches, and static capability-envelope closure.

It may not pre-execute durable effects, port effects, wall-clock reads, monotonic-deadline observations, external witness verification, secret access, randomness, dynamic availability resolution, dynamic proof verification, or Bvisor physical admission.

It may not eliminate or bypass Bvisor admission, dynamic capability checks, remaining work-budget checks, absolute deadline checks, receipt emission, effect ordering, publication fences, proof requirements, freshness checks, or runtime authority checks — unless the exact check is proven static under the complete key, recorded as discharged, and cannot vary for that plan. A check is not eliminated merely because the current implementation usually passes it.

```text
PakVM specialization cannot mint capabilities.
PakVM specialization cannot advance a durable checkpoint.
PakVM specialization cannot authorize a physical effect.
```

### Cache and fallback law

A `SpecializedPlan` cache is derived, content-addressed, generation and identity bound, replaceable, deletable, rebuildable, and non-authoritative. Lookup verifies key equality, plan format compatibility, content commitment, originating admitted-program identity, and all pinned profile and kernel identities. On mismatch it discards, refuses reuse, and falls back to reference execution, optionally rebuilding within budget. An old residual is never reinterpreted under a new key. A corrupt or partially published residual is ignored.

Deleting every specialization cache changes performance and `WorkObservation` only. It never alters value, availability, truth, decision, completeness, `ProofDisposition`, effects, effect ordering, authority meaning, or required receipts. A cache-publication failure cannot make correct execution unavailable, because reference execution remains complete.

### Bounded first use

First-use specialization carries an explicit work budget, an absolute monotonic deadline, a bounded memory posture, a cancellation posture, a `WorkObservation`, and a fallback path. If it exceeds budget, reaches its deadline, is cancelled, is refused, fails validation, or cannot publish its cache safely, the logical operation continues through the reference interpreter when its own remaining deadline and budget permit. **The specialization attempt never resets the logical operation deadline**, and one bounded request never becomes unbounded compilation work.

### Result parity

Reference and residual execution preserve semantic value, `Availability`, `Truth`, `Decision`, `Completeness`, `ProofDisposition`, `Freshness`, `TypedMargin`, effect declaration, effect ordering, authority classification, receipt requirements, and explanation meaning. Only performance metadata may differ. A faster path never emits less proof, explanation, or receipt material.

`WorkObservation` states which path ran:

```text
ReferenceInterpreter
CompileTimeResidual
FirstUseResidual
FallbackAfterSpecializationRefusal
```


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

## Numeric execution (DEC-069 / docs/37)

PakVM reference execution consumes the shared numeric sorts frozen in `04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md` and the numeric law in `37_NUMERIC_SEMANTICS_AND_AUTHORITY.md`: exact authority arithmetic, `Quantize` and `IntervalDecision` crossings, and typed numeric refusals for non-finite inputs. The reference interpreter is the numeric authority; residual specialization and optimized kernels are deferred to 5.5D and must prove equivalence to it.

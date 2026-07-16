---
status: AUTHORITATIVE
contract_id: BP-WORLD-PORTS-1
authority_scope: WorldImage composition, instances, namespaces, host ports, and portability
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# World Images and Ports

## WorldImage

A BatPak application is explicit data:

```text
ContractImages
+ ProgramImages
+ ProcessContracts
+ schemas/codecs
+ capability requirements
+ imports/exports
+ origins/proof manifest
→ WorldImage (.vpak)
```

No Rust closure, ambient registration, or hidden linker inventory becomes application authority.

## Ownership

BatPak owns language-neutral image types, IDs, canonical framing, schemas, and interface descriptors. MacBat and BatQL produce image inputs. SyncBat's `world` plane links/validates instances. PakVM executes programs. Bvisor admits instances and attempts.

## Image identities

```text
ContractImageId
ProgramImageId
WorldImageId
WorldInterfaceHash
```

WorldImageId commits to the complete canonical package. WorldInterfaceHash commits only to exported entrypoints, schemas, capabilities, results, and receipt contracts.

## Composition

Linking is explicit, deterministic, and collision-checked across contract IDs, schema/field IDs, event kinds, effects, capabilities, projections, process IDs, kernels, imports, exports, origins, and proof manifests.

Order-independent inputs produce the same canonical image where composition declares order irrelevant.

## Kernel binding (DEC-062)

A WorldImage binds each kernel through exactly one policy. Kernel identity types and the canonical manifest are owned by `07_PAKVM_ISA.md`.

```text
ExactImplementation
    pins KernelImplementationId

QualifiedInterface
    pins KernelContractId + KernelInterfaceHash,
    and requires an accepted KernelQualificationReceiptId
```

PakVM never resolves a kernel by display name. The `AttemptReceipt` records the exact `KernelImplementationId` actually used, even when the WorldImage allowed a qualified interface — so a portable qualified binding still yields an exact execution record.

This document owns the kernel vocabulary, the two binding modes, and the receipt rule. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere.

Required proof rows, projected from docs/24 (qualification target: `DEC-062`; canonical proof-row owner: docs/24 Gauntlet):

```text
kernel_cannot_resolve_by_display_name
kernel_contract_and_implementation_ids_are_not_interchangeable
qualified_interface_requires_matching_qualification_receipt
exact_kernel_binding_rejects_another_implementation
attempt_receipt_records_exact_kernel_implementation
```

## World instance

A live instance binds:

```text
WorldImageId
WorldInstanceId
WorldGeneration
Store/journal grants
capability table
runtime process state
budget state
attempt table
```

Live capability authority cannot be serialized out of the instance.

## Invocation classes

`docs/10` owns the two — and only two — ways a caller reaches PakVM execution. Both execute canonical ProgramImages; they differ in deployment authority, lifetime, effect permission, and capability origin, never in interpreter semantics.

### `InvokeEntrypoint` (DEC-049)

A deployed, persistent entrypoint declared by an admitted WorldImage. Its ProgramImage is bound by WorldImage identity; a request cannot substitute a foreign ProgramImage while keeping the entrypoint's authority. An entrypoint is not a name mapped to whatever code the server currently holds. It may be ASK or DO; effects are permitted only when the entrypoint contract declares them. Capability comes from the WorldImage grant. Identity binds `WorldImageId`, `WorldInterfaceHash`, `EntrypointId`, `ProgramImageId`, `InvocationId`, `AttemptId`.

### `ExecuteQueryProgram` (DEC-050)

A client-supplied canonical, query-only ProgramImage, executed ephemerally under a server-issued read-only query capability envelope. Effects, process installation, and persistent lifecycle hooks are forbidden. The server validates the image independently — "the client compiled it" grants no trust. Allowed operations: bounded read, filter, fold, match, group, aggregate, project, explain, prove. Required bounds: source/selectors, event or row work, decoded bytes, groups, matches, result bytes/items, proof bytes/work. The response binds `ProgramImageId`, query authority digest, historical cut, source generation/frontier, completeness, proof disposition, actual work, result digest, and receipt. A query program may evaluate `ALLOW`/`DENY`/`DEFER` as data; it may never use the result to perform an effect.

### Canonical comparison

| Property             | `InvokeEntrypoint`           | `ExecuteQueryProgram`          |
| -------------------- | ---------------------------- | ------------------------------ |
| Source               | Admitted WorldImage          | Supplied ProgramImage          |
| Lifetime             | Persistent deployment        | Ephemeral request              |
| Profile              | Declared entrypoint          | Query-only                     |
| ASK                  | Yes                          | Yes                            |
| DO/effects           | When declared                | Never                          |
| Capability source    | WorldImage grant             | Restricted request envelope    |
| Process installation | Declared processes only      | Forbidden                      |
| Raw BatQL            | No                           | No                             |
| Identity             | World + entrypoint + program | Program + query grant          |
| Admission            | World and invocation         | Program, query profile, bounds |
| Receipt              | Invocation/attempt receipt   | Query execution receipt        |

> Both invocation classes execute canonical ProgramImages through PakVM and Bvisor. They differ in deployment authority, lifetime, effect permission, and capability origin, not in interpreter semantics.

### Compilation boundary (DEC-051)

BatQL source text is authoring input, never executable authority. PakVM executes validated ProgramImages, not source text. Ordinary transport permits `InvokeEntrypoint` and `ExecuteQueryProgram`; it never permits `CompileAndExecuteRawBatQL`, `EvalText`, or `ExecuteSource`.

Source compilation is a separately admitted host capability, `CompilerPort`: a development/admin profile only, not available to guest programs, not implied by NetBat access, and absent from the default server profile. It never compiles-and-silently-executes in one opaque action; it keeps the evidence chain visible:

```text
BatQL source
→ CompilationRequest
→ ProgramImage
→ CompilationReceipt
→ ordinary invocation/admission path
```

The compiled image still passes ordinary validation and admission. Source identity is provenance, not runtime authority — the compiler is never a tunnel around PakVM validation.

### Required proof rows

This document owns the boundary law, the invocation classes, the capability envelopes, and the transport posture. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere. The rows below span four obligations, each qualifying at its own typed gates.

Required proof rows, projected from docs/24 (qualification targets: `DEC-051` for the compilation boundary, `DEC-050` for query-program invocation, `DEC-049` for entrypoint invocation, `LEG-045` for transport authority; canonical proof-row owner: docs/24 Gauntlet):

```text
raw_batql_is_not_a_netbat_invocation
compiler_port_is_not_available_to_guest_programs
compiler_port_is_absent_from_default_server_profile
compiler_output_still_requires_program_validation
query_program_with_effect_instruction_is_rejected
query_program_cannot_install_process
query_program_cannot_request_write_capability
query_program_over_work_bound_is_denied_before_execution
query_program_result_binds_program_and_grant_identity
entrypoint_receipt_cannot_satisfy_query_program_execution
entrypoint_cannot_substitute_foreign_program_image
entrypoint_effect_must_be_declared_by_world_interface
query_program_receipt_cannot_satisfy_entrypoint_invocation
tls_does_not_upgrade_program_or_proof_authority
```

An effect instruction is not a name on a list. It is any node `spec/pakvm_isa.rs` admits with the `Effectful` effect posture, which admission binds to the Effect algebra in both directions. `query_program_with_effect_instruction_is_rejected` therefore covers a node from the day it is admitted; there is no enumeration here to fall out of date, and none anywhere else.

Equivalence oracle: the same query compiled into the same canonical ProgramImage must produce equivalent PakVM semantics whether invoked as a pure declared entrypoint or an ephemeral `ExecuteQueryProgram`, given the same source cut, parameters, selectors, bounds, and proof posture. The receipts differ (authority and invocation identities differ); the query value and structured derivation do not.

## World namespace

A filesystem-like namespace is an ergonomic projection, not a host filesystem:

```text
world:/journal
world:/state
world:/input
world:/output
world:/artifact
world:/secret
world:/effect
world:/receipt
world:/process
world:/system
```

Typed WorldPath values resolve to ResourceRef plus rights requirements. Knowing a path grants no authority.

## Port families

```text
JournalPort
ArtifactPort
ClockPort
EntropyPort
TransportPort
CompilerPort   development/TestPak only
```

Requests and responses bind world, generation, attempt, capability slot, limits, and schema identity.

## Absolute monotonic deadlines (LEG-066)

A port operation accepting a deadline accepts an absolute monotonic deadline (`MonotonicDeadline`, `16_IDENTITY_TIME_AND_NAVIGATION.md`) or an equivalent type whose semantics are absolute and monotonic. A relative inactivity timeout is not an overall operation deadline, and an adapter may not substitute one for the other.

The lowest mechanism capable of internally extending work enforces the remaining absolute deadline:

```text
raw socket reads and writes below TLS
retry loops below a client adapter
incremental decoders
poll loops
buffer refill loops
streaming request or response bodies
```

A higher wrapper cannot claim deadline compliance while a lower repeating mechanism continues indefinitely. Ten consecutive 30-second "inactivity timeouts" are not a 30-second deadline.

### Retries

```text
retry does not reset the overall deadline
attempt N cannot extend the logical operation beyond its absolute deadline
wall-clock adjustment cannot extend a monotonic deadline
```

Retries consume the same overall deadline unless the contract explicitly declares a per-attempt deadline, and an explicit per-attempt deadline remains bounded by the overall operation deadline.

Deadline expiry before admission and after admission preserve the attempt and commit distinctions in `09_BVISOR.md` and `05_STORAGE_FBAT_AND_TILES.md`.

## Sans-I/O execution

```text
PakVM step
→ NeedPort(request)
→ host driver performs mechanism
→ PortResponse
→ resume same attempt
```

A native driver may block synchronously. A browser driver may await OPFS, IndexedDB, Web Crypto, Fetch, or WebSocket and resume later. The guest and logical runtime do not acquire an async-runtime dependency.

## Minimal semantic profile

`batpak` and `syncbat` must compile their semantic profiles with `#![no_std]` plus `alloc`. PakVM instruction semantics, Bvisor admission state, logical runtime transitions, WorldImage values, and port protocols therefore cannot depend on host filesystem, process, socket, thread, wall-clock, or entropy APIs.

Host drivers live behind explicit `std`, browser, or embedded adapter profiles. A mechanism profile may suspend and resume execution differently, but it must preserve the same request identity, result type, receipt fields, bounds, and recovery disposition.

## Portability profiles

The same semantics target native embedded applications, self-hosted servers, container deployments, browsers/workers, and deterministic in-memory tests.

A browser storage adapter must qualify its own atomicity, ordering, visibility, durability, quota, crash/reload, ownership, and size behavior. It never claims POSIX semantics by analogy.

## Artifact plane

Large immutable inputs/outputs, captured streams, proof bundles, and transferable tile buffers live in a content-addressed artifact plane. `.fbat` records references, attempts, and dispositions. `.vpak` may embed small immutable resources or refer to artifacts by digest.

No public artifact bundle extension is frozen until a real packed-artifact adopter earns it.

## Host composition succession

Legacy module manifests, interface fingerprints, hooks, jobs, and handlers are replaced by ContractImage, ProgramImage, WorldImage, WorldInterfaceHash, lifecycle events, ProcessContract, and compiled programs. The old shell is dissolved by ownership rather than copied into SyncBat.

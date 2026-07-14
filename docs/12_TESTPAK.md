---
status: AUTHORITATIVE
contract_id: BP-TESTPAK-1
authority_scope: repository proof battery, Repo IR, role tree, mutation, gauntlet, and command authority
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# TestPak

## Identity

TestPak is the repository proof battery and development command authority. It is dev-only and may inspect every package. It is not product runtime.

## Role-based tree

```text
testpak/src/
├── fixture/       deterministic worlds, inputs, corruption, and negative cases
├── model/         deliberately simple reference semantics
├── oracle/        independent expected-result computation
├── harness/       production implementation drivers
├── schedule/      deterministic, hostile, crash, and concurrency schedules
├── bench/         workload definitions and measurement contracts
├── corpus/        persisted compatibility, fuzz, recovery, and mutation cases
├── repo/          Repo IR and architecture inspection
├── forge/         deterministic generated artifacts
├── gauntlet/      trust-boundary and proof-of-proof orchestration
├── receipt/       proof, build, mutation, and release receipts
├── command/       repository command catalog
└── muterprater/   mutation planning, execution, classification, explanation
```

Proof classes such as property, state-machine, differential, equivalence, fault-injection, crash-recovery, structural, and complexity are metadata on harnesses. They are not architecture folders.

## Repo IR

Repo IR is a typed column/fact plane for:

```text
semantic concepts and owners
contracts and generated lowerings
source files and public items
dependency roles and profiles
proof obligations and witnesses
fixtures and mutants
compatibility dispositions
document status and supersession
commands and receipts
```

One traversal may feed many fitness systems when lawful. The fact plane is not a second authority; facts point to canonical owners.

## Forge

Forge materializes generated tables, manifests, docs, SDKs, corpus indexes, and command schemas from signed facts. A generated view carries source contract ID and digest. Hand-maintained mirrors are refused after a generated successor exists.

## Independent models

The product PakVM interpreter executes real programs. TestPak retains deliberately simple evaluators for selected algebras: decision/K3, codec, path ordering, projection tree, schedule, delivery, and recovery. It does not maintain a second full VM religion.

## Muterprater scope

Muterprater owns only:

```text
mutation identity and operators
mutation planning
mutant materialization
execution requests
classification
survivor explanation
mutation-grounded test promotion
mutation receipts
```

It does not own Repo IR, all commands, release, benchmarks, LSP, documentation, or general proof.

## Mutation lanes

```text
Lane A: Contract, schema, BatQL, ProgramImage, and PakVM IR mutations
        zero Rust compiles

Lane B: generated selectable Rust mutation sites
        one compile per shard; runtime-selected site

Lane C: handwritten mechanism kernels
        narrowly scoped compiler-backed mutation
```

A mutation site must be observed as activated. A green test with a dormant mutant is not evidence.

## Audited denominator

Every planned proof unit ends as:

```text
Passed
Failed
Refused
Unsupported
SkippedWithAuthority
Expired
Superseded
```

Zero executed tests, missing fixture activation, stale corpus, or unqualified rule cannot silently count green.

## Gauntlet economy

Fast structural, model, and IR lanes run locally. Expensive fuzz, mutation, cross-platform, and performance envelopes run under named profiles with freshness receipts. Proof strength follows evidence, not where a badge appears in CI.

## Self-host without self-blessing

TestPak may express commands and proof plans as WorldImages and execute them through PakVM. Seedcheck and selected reference models remain independently implemented. No generated manifest may notarize itself as current solely because it can decode its own bytes.

## Command authority

TestPak owns repository commands: inspect, forge, test, mutate, fuzz, bench, prove, context, and seal. Product commands remain in `batpak-cli` over production library contracts.

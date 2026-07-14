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
├── fixture.rs +   loaders/classifiers/generators over the fixture assets
│   fixture/       (frozen bytes live in crates/testpak/fixtures/, not here)
├── model/         deliberately simple reference semantics
├── oracle/        independent expected-result computation
├── harness/       production implementation drivers
├── schedule/      deterministic, hostile, crash, and concurrency schedules
├── bench/         workload definitions and measurement contracts
├── corpus.rs +    load/index/minimize the persisted corpus cases
│   corpus/        (frozen bytes live in crates/testpak/corpus/, not here)
├── repo/          Repo IR and architecture inspection
├── forge/         deterministic generated artifacts
├── gauntlet/      trust-boundary and proof-of-proof orchestration
├── receipt/       proof, build, mutation, and release receipts
├── command/       repository command catalog
└── muterprater/   mutation planning, execution, classification, explanation
```

Proof classes such as property, state-machine, differential, equivalence, fault-injection, crash-recovery, structural, and complexity are metadata on harnesses. They are not architecture folders.

## Canonical asset homes

Frozen assets and Rust that operates on them are separate:

```text
crates/testpak/corpus/     frozen bytes: compatibility, fuzz, recovery, mutation
crates/testpak/fixtures/   hostile/negative fixtures and deterministic inputs
crates/testpak/src/        loaders, classifiers, generators, minimizers
```

These are the sole canonical homes (DEC-044). Root `corpus/` and root `fixtures/` are forbidden target paths. A package may keep a tiny local test fixture only when it is explicitly noncanonical and owned solely by that package's own test — never a canonical mirror.

## Binary target

TestPak ships its command door as a binary in the same package — no separate command crate (DEC-045):

```text
crates/testpak/src/lib.rs
crates/testpak/src/bin/testpak.rs   (publish = false)
```

The root `justfile` is a discoverability projection over `cargo run -p testpak -- <command>`. TestPak launches and checks `batpak-examples` through workspace metadata / Cargo commands, never through a Cargo dependency edge to the examples package.

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

## AST enforcement gate (DEC-068)

This gate does **not** exist yet. It is implemented at G3. This section freezes its exact obligation so implementation cannot quietly narrow it. Until then, the bootstrap lexical scan (DEC-067) provides defense in depth; the TestPak AST gate is the authoritative Rust classification.

TestPak's AST gate owns detection of:

```text
function-local and closure-local named domain types
unsafe blocks, functions, and impls
pointer and integer-pointer casts
narrowing numeric casts
lint suppressions (#[allow] / #[expect])
panic / unwrap / expect / dbg in production source
dependency-owned public types in ordinary public APIs
drawer modules (utils, common, helpers, manager, processor, misc)
stale architecture aliases (derived from spec/dispositions.rs)
canonical-order dependence on hash-map iteration
```

Required behavior:

```text
production violation           hard finding
test-only contextual expect    permitted
dangerous mechanism            requires an exact compiler-assumption ledger entry
bootstrap lexical result       defense in depth
TestPak AST result             authoritative Rust classification
```

Named hostile fixtures (implemented at G3):

```text
local_domain_type_inside_function_is_rejected
local_domain_type_inside_closure_is_rejected
test_local_fixture_type_is_classified_correctly
production_expect_is_rejected
test_expect_with_context_is_allowed
unledgered_unsafe_fn_is_rejected
unledgered_pointer_cast_is_rejected
public_flume_receiver_is_rejected
hash_map_iteration_in_canonical_encoder_is_rejected
drawer_module_name_requires_explicit_disposition
```

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

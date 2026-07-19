---
status: AUTHORITATIVE
contract_id: BP-COMMAND-PLANE-1
authority_scope: product and repository commands, generated interfaces, MCP, and receipts
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Command Plane

## Two domains

```text
batpak CLI
    product compilation, execution, query, serving, inspection, verification

testpak
    repository inspection, generation, testing, mutation, fuzzing,
    benchmarking, proof, context, and release sealing
```

The CLI adapter owns command parsing/composition only. Semantic commands are declared by the libraries they invoke.

## Product commands

Each entry carries its typed authority relation: `direct` cites the contract
owning the command-level operation; `composite` cites the orchestration
owner and the delegates whose semantic law it routes to.

<!-- PRODUCT-COMMANDS:BEGIN generated from spec/commands/types.rs by bootstrap/project.py; do not edit -->
```text
compile    composite  BP-COMMAND-PLANE-1           BP-MACBAT-1 BP-BATQL-ARCH-1 BP-WORLD-PORTS-1
run        direct     BP-WORLD-PORTS-1
query      direct     BP-WORLD-PORTS-1
serve      direct     BP-NETBAT-1
inspect    composite  BP-COMMAND-PLANE-1           BP-WORLD-PORTS-1 BP-SCHEMA-CODEC-1 BP-PAKVM-ISA-1 BP-BVISOR-1 BP-RECEIPTS-1
verify     composite  BP-COMMAND-PLANE-1           BP-WORLD-PORTS-1 BP-RECEIPTS-1 BP-GAUNTLET-1
repl       direct     BP-BATQL-LANGUAGE-1
```
<!-- PRODUCT-COMMANDS:END -->

Three command namespaces stay distinct (5.5E1): product commands above, BatQL
source modes (`ASK`, `DO` -- language law, owned by the companion, never
product commands), and the TestPak repository commands below. `batpak run`
invokes a declared WorldImage entrypoint whose program may be ASK or DO;
`DO` is not a top-level command.

## TestPak commands

<!-- TESTPAK-COMMANDS:BEGIN generated from spec/commands/types.rs by bootstrap/project.py; do not edit -->
```text
inspect    direct     BP-SELF-EXPLAINING-1
forge      direct     BP-TESTPAK-1
test       direct     BP-TESTPAK-1
mutate     direct     BP-TESTPAK-1
fuzz       direct     BP-GAUNTLET-1
bench      direct     BP-GAUNTLET-1
prove      direct     BP-GAUNTLET-1
context    direct     BP-SELF-EXPLAINING-1
seal       direct     BP-PUBLIC-API-CI-RELEASE-1
```
<!-- TESTPAK-COMMANDS:END -->

## Composite commands

`compile`, `inspect`, and `verify` are composition boundaries, not
single-sovereign operations, and the command plane owns their orchestration
and routing only. `compile` composes BatQL source compilation, MacBat
contract lowering, and WorldImage/`.vpak` assembly. `inspect` routes
read-only views across images, schemas, programs, worlds, capabilities,
results, and receipts. `verify` routes packages, results, receipts, and
proof bundles to their verifying contracts and reports which verifier
established which claim — never one generic "valid". Composition transfers
no ownership: every delegate retains its underlying semantic law.

## One declaration, many interfaces

A command contract declares stable ID, request/result schemas, capabilities, determinism, bounds, receipt contract, and help/narration projection. CLI, shell completion, REPL, MCP schema, docs, and admission requirements are generated views.

## Root `justfile`

The root file is a discoverability counter that invokes typed commands. It owns no policy, duplicate defaults, or hidden shell pipeline.

## Development effects

TestPak may request Rust compiler, Git state, repository read/write, qualified fuzzers, and benchmarks through typed development capabilities. Receipts bind tool identity, args, environment allowlist, inputs, outputs, duration, and terminal status.

## MCP and agents

MCP is a projection of the same command catalog. Agents receive typed schemas and stable IDs, not scraped help text.

## Context packet

`testpak context` emits commit/tree identity, active contracts, dirty state, latest proof receipts, affected obligations, open implementation constants, stale-document warnings, and next legal commands. It contains no private reasoning or hand-curated status story.

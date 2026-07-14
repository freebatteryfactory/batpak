---
status: AUTHORITATIVE
contract_id: BP-COMMAND-PLANE-1
authority_scope: product and repository commands, generated interfaces, MCP, and receipts
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
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

```text
compile   BatQL + contract inputs → .vpak
run       execute a world entrypoint
query     execute an ASK program
serve     expose declared NetBat entrypoints
inspect   inspect image, schema, program, world, capability, result, receipt
verify    verify package, result, receipt, proof bundle
```

## TestPak commands

```text
inspect
forge
test
mutate
fuzz
bench
prove
context
seal
```

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

---
status: AUTHORITATIVE
contract_id: BP-ROOT-README
authority_scope: entrypoint and reading order
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# BatPak Clean-Room Specification v1.0

This bundle is the final clean-room architecture for **BatPak**.

BatPak is a contract-compiled, sync-first event machine. It stores accepted local truth in `.fbat`, compiles Rust declarations through MacBat, compiles application questions and actions through BatQL, runs logical work through SyncBat, interprets sealed programs through PakVM, admits and supervises attempts through Bvisor, carries bounded calls through NetBat, and proves the repository through TestPak.

The one-line system law is:

> **MacBat declares. BatPak remembers. BatQL compiles. SyncBat runs. PakVM interprets. Bvisor contains. NetBat carries. TestPak attacks and proves.**

PakVM and Bvisor are not standalone packages. They are internal planes of the `syncbat` runtime crate, beside `runtime`, `world`, and `port`.

## Build qualification

The semantic profiles of `batpak` and `syncbat` are required to compile under `no_std + alloc`. Native and browser host mechanisms are explicit adapters and may not alter semantic traces, receipts, or recovery law.

## Frozen package map

The package inventory is a generated projection of the typed owner (`spec/architecture.rs`); membership, class, layer, path, and order live there, never in this document:

<!-- PACKAGE-INVENTORY:BEGIN generated from spec/architecture/inventory.rs; spec/architecture/types.rs by bootstrap/project.py; do not edit -->
| Package | Class | Layer | Workspace path | Role |
| --- | --- | --- | --- | --- |
| macbat-compiler | production | 0 | crates/macbat/compiler | pure Rust contract compiler |
| macbat | production | 1 | crates/macbat/macros | proc-macro front door |
| batpak | production | 2 | crates/batpak | semantic and durable core, including .fbat |
| syncbat | production | 3 | crates/syncbat | runtime crate containing runtime, PakVM, Bvisor, world, and port planes |
| batql | production | 3 | crates/batql | BatQL parser, type checker, planner, partial evaluator, and ProgramImage compiler |
| netbat | production | 4 | crates/netbat | bounded typed transport over declared SyncBat world entrypoints |
| testpak | dev-only | 6 | crates/testpak | repository proof, forge, gauntlet, benchmark, and mutation battery |
| batpak-cli | binary-adapter | 5 | apps/batpak-cli | thin product command adapter; owns no semantic law |
| batpak-examples | example | 5 | examples | public-surface witness; runnable demos over production APIs only; owns no semantic law and depends on no dev tooling |
<!-- PACKAGE-INVENTORY:END -->

PakVM and Bvisor are internal planes of `syncbat`, not Cargo packages. Muterprater is a module inside `testpak`, not a Cargo package. A module cannot enter the package inventory by appearing in this explanation.

`.vpak` remains the immutable executable package extension. `PakVM` is the machine. `ProgramImage` and `WorldImage` are semantic types.

## Authority order

```text
00 Constitution
→ typed facts in spec/
→ status and decision ledgers
→ numbered domain contracts
→ BatQL companion grammar
→ generated views
→ source and receipts
→ legacy evidence
```

When two documents disagree, do not average them. Apply [Status and Supersession](docs/29_STATUS_AND_SUPERSESSION.md), then report the contradiction if it survives.

## Reading order

1. [Constitution](docs/00_CONSTITUTION.md)
2. [System Model](docs/02_SYSTEM_MODEL.md)
3. [Repository and Packages](docs/03_REPOSITORY_AND_PACKAGES.md)
4. [Type System and Source Layout](docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md)
5. [Storage, `.fbat`, and Tiles](docs/05_STORAGE_FBAT_AND_TILES.md)
6. [Crypto and Secret Authority](docs/35_CRYPTO_AND_SECRET_AUTHORITY.md)
7. [MacBat](docs/06_MACBAT.md)
8. [PakVM ISA](docs/07_PAKVM_ISA.md)
9. [SyncBat Runtime](docs/08_SYNCBAT_RUNTIME.md)
10. [Bvisor](docs/09_BVISOR.md)
11. [World Images and Ports](docs/10_WORLD_IMAGES_AND_PORTS.md)
12. [BatQL Architectural Contract](docs/13_BATQL_CONTRACT.md)
13. [BatQL Language](companion/BATQL_LANGUAGE.md)
14. [Legacy Semantic Obligations](docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md)
15. [Legacy Invariant Coverage](docs/34_LEGACY_INVARIANT_COVERAGE.md)
16. [Decision and Rejection Ledger](docs/30_DECISION_AND_REJECTION_LEDGER.md)
17. [Public API, CI, and Release](docs/36_PUBLIC_API_CI_AND_RELEASE.md)
18. [Agent Finish-Line Checklist](docs/33_AGENT_FINISH_LINE_CHECKLIST.md)

The remaining numbered documents close the storage, schema, delivery, proof, security, migration, bootstrap, command, workflow, dynamic-verification and conformance, and sprouting-nursery and promotion details.

## Independent checks

The operational runbook — canonical command order, the `python -I` isolated-mode
law for every Python gate, and the spec-rlib build the Rust tools link against —
lives in [bootstrap/README.md](bootstrap/README.md). This root document is an
entrypoint, not a second runbook; the Python tools use only the standard
library, and the Rust tools link only the typed `spec` crate.

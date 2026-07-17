---
status: AUTHORITATIVE
contract_id: BP-CONSTITUTION-1
authority_scope: root architecture and non-negotiable law
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Constitution

## 0. Root sentence

> BatPak is a contract-compiled, sync-first event machine that turns declarations and BatQL programs into bounded execution, immutable `.fbat` history, logical turns, physical attempts, and independently verifiable receipts.

## 1. Root verbs

```text
MacBat declares.
BatPak remembers.
BatQL compiles.
SyncBat runs.
PakVM interprets.
Bvisor contains.
NetBat carries.
TestPak attacks and proves.
```

PakVM and Bvisor are internal planes of SyncBat. Their names describe responsibilities, not package borders.

## 2. Fundamental units

```text
Contract       semantic law
Event          immutable accepted truth
Representation canonical bytes or buffers
Tile           bounded physical materialization
Turn           logical execution over one frozen input frontier
Attempt        one physical execution
Receipt        structured evidence
World          linked contracts, programs, processes, capabilities, and entrypoints
```

No identifier or generic wrapper may collapse two units.

## 3. Ownership planes

### MacBat

Owns Rust declaration grammar, validation, typed Contract IR, origin, diagnostics, partial evaluation, and generated lowerings.

### BatPak

Owns shared semantic and durable law: contracts, schemas, codecs, values, events, `.fbat`, IDs, time/order types, coordinates, projections, tiles, effects, image contracts, receipts, compatibility, and storage-port law.

### SyncBat

Owns one runtime crate with five planes:

```text
runtime   logical process, turn, checkpoint, delivery, restart legality
pakvm     ProgramImage and WorldImage validation and interpretation
bvisor    admission, capabilities, budgets, AttemptId, supervision, reconciliation
world     linking, instantiation, entrypoints, generation, interface
port      typed host requests and responses
```

### BatQL

Owns the human language frontend and compilation into BatPak-owned ProgramImage and WorldImage inputs.

### NetBat

Owns bounded transport of declared world calls, pages, results, and receipts. It does not interpret domain semantics or proof strength.

### TestPak

Owns repository facts, fixtures, models, oracles, schedules, forge, fuzzing, benchmarks, gauntlet, proof receipts, commands, and mutation orchestration. Muterprater is its mutation-only module.

## 4. Semantic sovereignty

BatPak owns meaning, identity, schema, lifecycle, availability, error taxonomy, authority, and proof obligations.

Dependencies may supply difficult mechanisms behind BatPak-owned contracts or explicit interop profiles. Dependency count is not a quality metric. Semantic leakage is.

## 5. Source of truth

Accepted events in `.fbat` are source truth. Corrections are later events. Projections, indexes, tiles, caches, UI state, and generated summaries are rebuildable views unless explicitly classified as mutable or secret authority.

Damage to authority is never treated as absence.

## 6. Types are the state

Primary semantic types live in discoverable root concept files. Operations live in same-name module directories. Domain-significant types do not hide inside functions or unrelated algorithm files.

Core owns cross-package public, durable, wire-visible, proof-bearing types. Outer packages do not mint local passports for the same concept.

## 7. Effects are explicit

Every external interaction crosses a named capability or port, has declared bounds, and returns a typed outcome. A pure program cannot contain effect instructions. A dropped observer does not retroactively cancel admitted work.

## 8. Runtime law

The durable mailbox of a logical process is a selector plus cursor over journal truth. Wake mechanisms accelerate discovery but cannot become authority.

Runtime decides semantic restart legality. Bvisor performs attempt mechanics. PakVM reports program execution. No plane silently performs another plane's transition.

## 9. PakVM law

PakVM interprets only validated BatPak programs. The instruction set contains no syscall, raw pointer, host path, raw descriptor, ambient environment, ambient clock, ambient entropy, or arbitrary callback operation.

`.vpak` is the immutable package profile. It is not `.fbat` and neither format is forced through one universal container parser.

## 10. Proof law

No generated or optimized implementation is its own only oracle. Runtime fusion is encouraged. Proof-source fusion is not.

Every planned proof unit has an explicit terminal disposition and freshness-bound receipt.

## 11. K3 and bounded-circuit law

K3 truth is used only where unresolved facts affect predicates and decisions. Fixed-width eligible decision fragments may lower to bounded circuits. Neither claim is painted over replay, storage recovery, world lifetime, or external-effect reconciliation.

## 12. ECS law

Contract is the semantic primitive. Tile is the physical materialization primitive. Typed tables and explicit systems are implementation tools for repeated facts. BatPak does not ship a universal game-style ECS or dynamic type bag.

## 13. Package law

A new package must earn an independent semantic lifecycle, multiple meaningful consumers, a required compilation/target boundary, mechanism isolation that modules cannot prove, or an independent publication/proof surface. A name, file extension, or purity preference is insufficient.

## 14. Build-profile law

The `batpak` and `syncbat` packages each qualify a semantic profile under `no_std + alloc`. Native filesystem, mapping, thread, socket, clock, entropy, and browser mechanisms are explicit adapters behind profile gates. `no_std` proves absence of ambient `std`; TestPak still proves determinism, boundedness, authority, and semantic parity.

## 15. Big-bang law

Design may iterate. The signed architecture is atomic. The final tree contains one product model, not a legacy path plus an optional new path.

## 16. Status law

Filename age and prose confidence do not confer authority. Every document declares status and supersession. Rejected designs remain readable only through the decision ledger.

## Corpus reconciliation epoch

`ReconciliationEpoch` identifies which coherent semantic corpus a document
belongs to. The current epoch is `cleanroom-v1`. It is not a date, a
release, a tree digest, a proof-freshness claim, or a runtime
reconciliation posture: `last_reconciled` answers when a document was
reviewed, and the epoch answers which whole-corpus reconciliation it
belongs to — neither derives from the other. An evidence-only document
carrying the current epoch has been reconciled into the corpus as
evidence; its historical design does not thereby become current
authority. A new epoch is admitted only by a deliberate whole-corpus
reconciliation that moves the current selection.

<!-- CORPUS-RECONCILIATION-EPOCH:BEGIN generated from spec/corpus.rs by bootstrap/project.py; do not edit -->
```text
current epoch      cleanroom-v1
semantic owner     BP-CONSTITUTION-1
admission basis    SEED-DOC-STATUS
```
<!-- CORPUS-RECONCILIATION-EPOCH:END -->

## 17. Legacy law

The old repository is evidence. Every imported behavior requires a Legacy Semantic Obligation row naming the retained law, rejected mechanism, clean owner, gate, and witness. No old folder or dependency edge crosses the clean-room boundary by default.

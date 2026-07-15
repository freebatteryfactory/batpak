---
status: AUTHORITATIVE
contract_id: BP-GATES-1
authority_scope: dependency-ordered implementation gates, acceptance, and forbidden shortcuts
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Implementation Gates

## Destination order

```text
G0 Constitution and skeleton
G1 MacBat
G2 BatPak semantic and durable core
G3 TestPak seed
G4 BatQL compiler
G5 SyncBat world, ports, PakVM reference, Bvisor admission
G6 SyncBat logical runtime and recovery
G7 NetBat and product CLI
G8 Optimized tiles, codecs, and delivery succession
G9 Self-hosting and release seal
GJ Integrated final tree
```

Each gate is one review unit with savepoint commits inside it. A later gate may not redefine an earlier owner's meaning.

## G0: Constitution and skeleton

Deliver exact workspace members/edges, status metadata, seed tools, concept-file skeleton, no forbidden paths, no legacy source, declared qualification profiles, and a passing architecture audit. Materialize the skeleton with `bootstrap/materialize.rs`; then compile `batpak` and `syncbat` semantic profiles under `no_std + alloc` before G0 closes.

## G1: MacBat

Deliver pure compiler pipeline, first contract kinds, diagnostics/origins, snapshots, explicit composition facts, error generation, mutation facts, budgets, and independent snapshots/compile-fail witnesses.

No runtime package implements a hand-maintained mirror of a generated contract.

## G2: BatPak core

Deliver ID/time/navigation vocabulary, schema/codec IR, EventFrameV2 bytes/goldens, `.fbat` journal, storage-port conformance, durable idempotency, visibility, batch/recovery, receipts, projection/query/path contracts, authority generation, compatibility readers, SIDX reference scan, and the BatPak `no_std + alloc` semantic profile. Native filesystem and mapping implementations remain explicit `std` adapters.

This gate closes every `LEG-*` row assigned G2 or explicitly carries it forward with a signed reason. Any unsafe storage/codec mechanism has a `SAFETY-CONTRACT:` record, ledger entry, independent witness, and failure-policy proof before merge.

## G3: TestPak seed

Deliver role-based tree, fixture/model/oracle/harness/schedule foundations, Repo IR seed, forge, audited denominator, proof receipts, and Muterprater Lane A.

## G4: BatQL

Deliver grammar, parser, type/availability analysis, historical-frame law, formatter, speak/explain projections, partial evaluation, bounds/capability analysis, ProgramImage lowering, diagnostics, fuzz grammar, and canonical vectors.

The conceptual language remains frozen.

## G5: SyncBat world, ports, PakVM, Bvisor admission

Deliver the five required module planes, WorldImage linking, structural and executable validation, simple PakVM interpreter, port suspension/resume, capability table, budget reservation, AttemptId lifecycle, decision-circuit differential, deterministic simulation skeleton, and the real SyncBat `no_std + alloc` semantic profile. Native threaded and browser adapters must preserve the same semantic trace and receipt model.

No OS-backend matrix or arbitrary guest execution is permitted. PakVM cannot call host mechanisms directly. Any admitted unsafe adapter is isolated, ledgered, and tested against the typed port contract.

## G6: SyncBat runtime and recovery

Deliver ProcessContract, selector/cursor mailbox, TurnId, EffectBatch, checkpoint, delivery topologies, supervision, external effect recovery classes, Bvisor attempt reconciliation, cooperative/threaded parity, and runtime receipts.

## G7: NetBat and CLI

Deliver bounded transport, pages/cursors/proof carriage, native/browser adapters as qualified, generated client surfaces, thin product commands, package leak scan, and end-to-end world invocation.

## G8: Optimizations and mechanism succession

Deliver SIDX column tile, selective decode, fused folds, optimized PakVM path, native Completion/Permit/Broadcast candidates, and optional Mailbox candidate. Every optimization remains differential against reference semantics.

Dependencies leave only after proof, never because the gate name says “cleanup.”

## G9: Self-host and release

Deliver repository command WorldImages where useful, independent seedcheck parity, context packets, release seal, issue/legacy closure receipts, and exact published artifacts.

## Numeric gate ownership (DEC-069 / docs/37)

The numeric contract in `37_NUMERIC_SEMANTICS_AND_AUTHORITY.md` is owned across gates: G2 fixes the durable Fixed128 format constants and wire encoding (exact format tags and raw bits preserved, no normalization), G5 admits qualified numeric kernels and profiles, and TestPak realizes the numeric proof families (`24_GAUNTLET.md`). Bootstrap enforces only the numeric contract, never executed arithmetic.

## GJ: Integrated final tree

Refuse the gate if any of these remain:

```text
legacy architectural path
second semantic owner
unresolved contract surface
public dependency mechanism type
unclassified legacy obligation
unqualified target adapter
self-grading proof
stale/superseded doc treated as law
compatibility reader with no removal/retention receipt
```

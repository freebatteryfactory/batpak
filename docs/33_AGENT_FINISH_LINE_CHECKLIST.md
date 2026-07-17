---
status: AUTHORITATIVE
contract_id: BP-AGENT-FINISH-LINE-1
authority_scope: code-agent execution checklist and stale-architecture cheat sheet
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Agent Finish-Line Checklist

## Before touching code

- [ ] Read Constitution, Status, Decision Ledger, and the active gate.
- [ ] Run `bootstrap/audit.py`.
- [ ] Confirm package/edge facts in `spec/architecture.rs`.
- [ ] Locate every affected `LEG-*` obligation and legacy `INV-*` coverage row.
- [ ] Confirm no stale or rejected path is being recreated.
- [ ] Confirm the affected semantic profile still compiles under `no_std + alloc`, or that the file is an explicitly admitted host adapter.
- [ ] Write the before/after tree and exact owner map.

## Architecture cheat sheet

```text
BatPak       contracts, schemas, .fbat, images, durable truth, receipts
BatQL        source → ProgramImage
SyncBat      one runtime crate
  runtime    logical process/turn/checkpoint/restart legality
  pakvm      validated program semantics
  bvisor     capability/budget/AttemptId/physical evidence
  world      linking/instances/interfaces
  port       host requests/responses
NetBat       bounded transport
TestPak      repository proof
Muterprater  mutation only
```

## Immediate stale alarms

Stop before creating or depending on:

```text
standalone runtime VM/Bvisor/PakVM crate
separate storage-format package
empty product umbrella
universal type drawer
Host composition shell
OS-specific Bvisor backend matrix
raw Flume receiver in public API
Serde supertrait on event contract
ambient inventory/constructor as minimal composition
one generic cursor/timestamp/position
```

## Derived stale vocabulary (machine-checked)

The complete stale-alias set is DERIVED from `spec/dispositions.rs`; `audit.py` proves this projection matches it exactly. Do not edit by hand — change the owning decision's `stale_aliases` instead.

<!-- STALE-VOCAB:BEGIN generated from spec/dispositions.rs by bootstrap/project.py; do not edit -->
```text
FileBat
filebat package
file-bat crate
separate storage-format package
standalone Bat VM
bat-vm package
standalone runtime VM package
VPak machine
VPak as the machine name
universal type drawer
src/_types
_types drawer
universal layer-folder tree
HostBat
HostBat shell
Host composition shell
HostBat compatibility path
OS-backend Bvisor
OS backend matrix
platform sandbox backend matrix
Serde as event-contract authority
Serde supertrait on event contract
raw Flume receiver
public Flume receiver
empty product umbrella
meaning-free umbrella
```
<!-- STALE-VOCAB:END -->

## Per-file checks

- [ ] Primary semantic types are in root concept files.
- [ ] No named domain type is declared inside a function.
- [ ] Implementation file states owner/read/write/bounds/proof where critical.
- [ ] No drawer filename.
- [ ] No lint silencer, unresolved marker, or unimplemented production branch.
- [ ] Generated files name source contract and digest.
- [ ] Every unsafe occurrence is in an admitted kernel/adapter, carries `SAFETY-CONTRACT:`, and appears in the unsafe ledger.

## Per-public-API checks

- [ ] Type owner is below every consumer.
- [ ] No dependency mechanism leaks.
- [ ] Availability, completeness, freshness, and proof are not collapsed.
- [ ] Effect is declared, bounded, mediated, and receipted.
- [ ] Async/browser integration does not change semantic result type.
- [ ] `std` use appears only in an admitted native/browser adapter profile.
- [ ] Error has stable code/class/fields and generated mappings.

## Per-runtime-transition checks

- [ ] TurnId and AttemptId are not interchangeable.
- [ ] PakVM does not advance checkpoint.
- [ ] Bvisor does not decide retry legality.
- [ ] Runtime does not bypass Bvisor.
- [ ] Port response binds world/generation/attempt/request.
- [ ] Outcome unknown remains explicit.
- [ ] Replay produces stable idempotency.

## Per-storage-change checks

- [ ] Source truth versus authority/cache role is explicit.
- [ ] StoreId/generation/frontier/commitment binding is correct.
- [ ] Crash point and publication order are tested.
- [ ] Old bytes remain readable under a signed compatibility row.
- [ ] Cache damage cannot authenticate or invent events.

## Per-proof checks

- [ ] Independent oracle exists.
- [ ] Negative fixture proves the gate bites.
- [ ] Mutation site activation is observed.
- [ ] Denominator row terminates explicitly.
- [ ] Receipt binds exact source/tool/profile and freshness.

## Gate closure

- [ ] All assigned `LEG-*` rows close or carry an explicit signed disposition.
- [ ] All new dependencies have role records.
- [ ] No superseded file/path appears in docs or code.
- [ ] Seedcheck, audit, freeze, package, and gate suites pass.
- [ ] Review packet and deletion receipts are complete.

When any checkbox changes architecture rather than implementation, stop and file a structured finding. Do not improvise a seventh package while the compiler watches. 🦇

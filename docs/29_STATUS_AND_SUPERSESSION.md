---
status: AUTHORITATIVE
contract_id: BP-STATUS-1
authority_scope: document statuses, precedence, supersession, and stale-evidence behavior
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Status and Supersession

## Status vocabulary

```text
AUTHORITATIVE   current signed law
GENERATED       deterministic view of named authority
EVIDENCE-ONLY   historical/external evidence, not current law
SUPERSEDED      retained lineage with explicit successor
REJECTED        intentionally refused architecture or mechanism
```

`PROPOSED` is not used inside this final bundle. New proposals live outside the authoritative tree until reconciled.

## Required metadata

There is no one universal metadata shape; the corpus carries two, and neither may imitate the other.

Authored Markdown documents declare: status, contract ID, authority scope, supersession source or successor, last-reconciled date, and ReconciliationEpoch corpus membership.

A standalone generated document declares: GENERATED status, derived authority scope, generator, exact authority sources, do-not-edit posture, and source ReconciliationEpoch.

A generated document does not invent a contract ID or supersession claim merely to imitate authored frontmatter. An authored document cannot claim generated status to evade the authored fields.

## Precedence

```text
Constitution
→ spec/ typed facts and decision/legacy ledgers
→ numbered domain contracts
→ BatQL language companion
→ generated machine views
→ current implementation and receipts
→ evidence-only legacy source/issues/docs
```

Current code is factual evidence of behavior. It is not automatic target architecture.

## Contradiction behavior

When two current authorities appear inconsistent:

1. cite both exact contracts;
2. check typed facts and decision ledger;
3. do not blend them into a compromise;
4. stop implementation if owner, bytes, authority, or proof meaning changes;
5. record a finding for architectural ruling.

## Supersession record

A supersession names old claim, retained obligation, killed mechanism, new owner, successor contract ID, gate, and proof successor.

## Stale term cheat sheet

Inside normative implementation work, these are immediate warning signs unless quoted by the rejection/legacy ledgers:

```text
standalone VM/Bvisor package
separate storage-format product
universal type drawer
Host composition shell
platform sandbox backend matrix
VPak as the machine name
raw Flume receiver in public API
Serde trait as event-contract authority
```

Agents do not “support both just in case.” They apply the decision ledger.

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

---
status: AUTHORITATIVE
contract_id: BP-STATUS-1
authority_scope: document statuses, precedence, supersession, and stale-evidence behavior
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
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

Every Markdown document declares status, contract ID, authority scope, supersession source/successor, and reconciliation date.

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

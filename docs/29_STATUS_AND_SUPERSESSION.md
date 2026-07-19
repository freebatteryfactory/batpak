---
status: AUTHORITATIVE
contract_id: BP-STATUS-1
authority_scope: document statuses, precedence, supersession, and stale-evidence behavior
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
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

The complete stale-alias set is DERIVED from `spec/dispositions/inventory.rs`; `audit.py` proves this projection matches it exactly. Do not edit by hand — change the owning decision's `stale_aliases` instead.

<!-- STALE-VOCAB:BEGIN generated from spec/dispositions/inventory.rs by bootstrap/project.py; do not edit -->
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
root concept file
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

## Pass evidence registry

The clean-room reached v1 across four passes. Only the final pass is current
law; the earlier passes are retained as external evidence.

```text
pass       role               disposition
Pass 1     external evidence  retained conceptual base
Pass 2     external evidence  package map rejected, named ideas imported
Pass 3     external evidence  recovery ruling
Final v1   current authority  cleanroom-v1
```

Prior pass bundles remain external evidence, deliberately not copied into the
authoritative tree so stale architecture cannot masquerade as current law; git
history preserves the bytes.

### Pass chronology

Pass 1 found the correct machine-centered architecture; it remains the conceptual base. Pass 2 added useful status, availability, portability, delivery, schema, identity, and ECS work, but over-split the single machine into separate packages. That package map is rejected and holds no authority, and Pass 2 is not a normative predecessor: its retained ideas were rewritten into their current owners rather than inherited. Pass 3 issued the recovery ruling — it restored Pass 1 as the base and rejected the split product models: a separate product for the durable journal format, an empty top-level product shell, a universal type directory, the platform-shell product model, and the package-by-purity rule. Final v1 is the current authority: it places the runtime, PakVM, Bvisor, world, and port planes inside the one SyncBat machine and keeps the useful Pass 2 laws under that recovered machine.

### What "final" means

Architecture, ownership, dependency direction, package boundaries, language boundaries, and semantic laws are frozen. Exact constants — opcode numbers, byte offsets, benchmark thresholds, and adapter-specific physical guarantees — remain gate-scoped implementation selections owned by `docs/32_IMPLEMENTATION_CONSTANTS.md`; each is chosen only inside its named gate with goldens and conformance evidence, and none may reopen the frozen architecture.

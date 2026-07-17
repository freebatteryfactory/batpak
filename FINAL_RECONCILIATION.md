---
status: AUTHORITATIVE
contract_id: BP-FINAL-RECONCILIATION
authority_scope: pass history and final authority ruling
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Final Reconciliation

## Pass history

```text
Pass 1
    found the correct machine-centered architecture

Pass 2
    added useful status, availability, portability, delivery, schema,
    identity, and ECS work, but over-split the machine into packages

Pass 3
    restored Pass 1 as the base and rejected the storage split,
    empty umbrella, universal type drawer, and package-by-purity rule

Final v1
    places runtime, PakVM, Bvisor, world, and ports inside SyncBat;
    retains the useful Pass 2 laws under the recovered machine
```

Pass 2 is not a normative predecessor. Its retained ideas have been rewritten into current owners. Its rejected package map has no authority.

## Final machine

```text
MacBat declarations ─┐
                     ├→ BatPak contracts, schemas, ProgramImages, WorldImages
BatQL source ────────┘                     │
                                           ▼
                                     SyncBat world
                         ┌─────────────────┼─────────────────┐
                         ▼                 ▼                 ▼
                      runtime           PakVM             Bvisor
                         │                 │                 │
                         └────────── world + port ───────────┘
                                           │
                                           ▼
                                      .fbat truth
```

NetBat transports declared world calls. TestPak proves the entire graph.

## Locked corrections

- `.fbat` remains BatPak's journal format; no separate storage package exists.
- PakVM names the interpreter; `.vpak` names the executable package format.
- Bvisor and PakVM are internal SyncBat planes.
- The root concept-file spine replaces a universal type directory.
- HostBat is dissolved by ownership, not copied into SyncBat.
- The OS-specific Bvisor backend ladder is evidence, not product architecture.
- BatQL is an independently consumable compiler crate targeting BatPak-owned image contracts.
- TestPak owns repository proof; Muterprater owns mutation only.

## What “final” means

Architecture, ownership, dependency direction, package boundaries, language boundaries, and semantic laws are frozen.

Exact opcode numbers, exact byte offsets, benchmark thresholds, and adapter-specific physical guarantees remain implementation constants. They may be selected only inside their named gate with goldens and conformance evidence. They cannot reopen the architecture.

---
status: AUTHORITATIVE
contract_id: BP-DECISION-LEDGER-1
authority_scope: KEEP, LOCK, KILL, SUPERSEDE, DEMOTE, DEFER, and implementation dispositions
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Decision and Rejection Ledger

## Tags

```text
KEEP                 retained as current architecture
LOCK                 retained and frozen against casual reopening
KILL                 forbidden from the clean target
SUPERSEDE            old meaning survives through a named successor
DEMOTE                retained only for compatibility, oracle, or evidence
DEFER                 outside V1; requires a real adopter and new ruling
OPEN-IMPLEMENTATION  owner/law fixed; exact constant selected at named gate
RETAIN-AS-EVIDENCE   historical material may be consulted, never treated as law
```

## Architecture decisions

| ID | Tag | Subject | Final ruling |
| --- | --- | --- | --- |
| DEC-001 | LOCK | Pass 1 machine-centered architecture | Base of final v1 |
| DEC-002 | RETAIN-AS-EVIDENCE | Pass 2 package-stratified architecture | Useful ideas salvaged individually; package map rejected |
| DEC-003 | KILL | Separate storage-format package | `.fbat` remains owned by `batpak` |
| DEC-004 | KILL | Standalone runtime VM package | PakVM and Bvisor live inside SyncBat |
| DEC-005 | SUPERSEDE | VPak as machine name | PakVM is machine; `.vpak` is package extension |
| DEC-006 | LOCK | SyncBat internal shape | `runtime`, `pakvm`, `bvisor`, `world`, `port` |
| DEC-007 | KILL | Universal type directory | Root concept file + same-name directory |
| DEC-008 | KILL | Universal tables/systems/ports/kernels tree | Put mechanics under the concept they serve |
| DEC-009 | SUPERSEDE | HostBat | WorldImage, ProgramImage, lifecycle events, ProcessContract, NetBat |
| DEC-010 | SUPERSEDE | Bvisor OS backend product matrix | Capability/attempt membrane over PakVM |
| DEC-011 | KEEP | `.fbat` | BatPak append-oriented source-truth format |
| DEC-012 | KEEP | `.vpak` | Immutable WorldImage package format |
| DEC-013 | KEEP | BatQL compiler package | Independent frontend over BatPak image contracts |
| DEC-014 | LOCK | TestPak role tree | Role folders; proof classes are metadata |
| DEC-015 | LOCK | Muterprater scope | Mutation testing only inside TestPak |
| DEC-016 | KEEP | Status/supersession metadata | Required on all docs |
| DEC-017 | KEEP | Bounded push + durable pull | Core delivery doctrine |
| DEC-018 | KEEP | K3 | Truth/decision handling only where unresolved data matters |
| DEC-019 | KEEP | NC¹ circuit lowering | Eligible bounded decision fragments only |
| DEC-020 | DEMOTE | Serde/MessagePack | Interop and historical readers, not new semantic authority |
| DEC-021 | DEMOTE | Flume | Private reference mechanism during topology-by-topology succession |
| DEC-022 | DEMOTE | UUID crate | Differential oracle until owned generator closes |
| DEC-023 | DEFER | Arbitrary native/Wasm guest execution | Future external-effect adapter only after real adopter |
| DEC-024 | KEEP | Artifact/content plane | Concern and port; no public bundle extension yet |
| DEC-025 | KEEP | Thin CLI package | Binary adapter only, no semantic ownership |
| DEC-040 | LOCK | BatPak and SyncBat semantic profiles | `no_std + alloc`; host mechanisms stay in explicit std/browser adapters |
| DEC-041 | LOCK | Cargo package `batpak` | The real semantic and durable library, not an empty umbrella; repository and library share the product name |
| DEC-042 | DEFER | BatDenseRecord representation | Requires a sealed-image or measured-kernel adopter; not part of the initial native authority path |

## Behavior decisions

| ID | Tag | Subject | Final ruling |
| --- | --- | --- | --- |
| DEC-026 | LOCK | Immutable accepted history | Corrections are later events |
| DEC-027 | LOCK | Read-time schema evolution | Historical bytes remain unchanged |
| DEC-028 | LOCK | Turn versus Attempt | Logical and physical execution identities remain distinct |
| DEC-029 | LOCK | Runtime/Bvisor restart split | Runtime decides legality; Bvisor performs mechanics |
| DEC-030 | LOCK | Availability axes | Value state, truth, decision, completeness, freshness, proof are separate |
| DEC-031 | LOCK | HLC scope | Chronology/filtering aid, not durability or universal order |
| DEC-032 | KEEP | ECS/data orientation | Implementation algebra, not ontology |
| DEC-033 | KEEP | SIDX successor | First earned column tile |
| DEC-034 | KEEP | One evaluation, many projections | Value/explanation/evidence/margin/cost share typed evaluation |

## Implementation constants

| ID | Tag | Subject | Gate |
| --- | --- | --- | --- |
| DEC-035 | OPEN-IMPLEMENTATION | EventFrameV2 field IDs and exact bytes | G2 |
| DEC-036 | OPEN-IMPLEMENTATION | `.vpak` section and PakVM opcode numbers | G5 |
| DEC-037 | OPEN-IMPLEMENTATION | First persistent browser adapter | G5/G7 |
| DEC-038 | OPEN-IMPLEMENTATION | Performance/work thresholds | owning gate + TestPak |
| DEC-039 | DEFER | Public packed-artifact extension | real adopter required |

## Reopening rule

A locked or killed decision reopens only through a signed amendment containing new evidence, affected obligations, compatibility impact, proof changes, migration plan, and explicit supersession. A code agent cannot reopen it through implementation convenience.

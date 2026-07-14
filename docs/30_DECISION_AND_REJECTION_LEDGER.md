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
| DEC-043 | LOCK | Examples package batpak-examples | Public-surface witness at examples/; publish is false; owns no semantic law; no TestPak dependency; production APIs only; observable output; executed through TestPak orchestration |
| DEC-044 | LOCK | TestPak sole corpus and fixture home | crates/testpak/corpus and crates/testpak/fixtures own frozen assets; root corpus/ and fixtures/ are forbidden; no canonical package-local mirrors |
| DEC-045 | LOCK | TestPak binary target | The testpak package ships src/lib.rs and src/bin/testpak.rs; the root justfile projects cargo run -p testpak; no separate command crate |
| DEC-046 | LOCK | Workspace version train | 1.0.0-alpha.1 implementation train; publishable production packages ship lockstep to 1.0.0; versions below the train are rejected |
| DEC-047 | LOCK | Default feature behavior | batpak and syncbat default to std with useful native usability; no_std via default-features false; std must not activate the threaded browser encryption mapping or interop adapter zoo |
| DEC-048 | LOCK | Orphan clean-room cutover | cleanroom is orphan with zero parents; legacy/main owns the final legacy source lineage; legacy-pre-cleanroom is the immutable legacy tag; no source merge graft cherry-pick or subtree import crosses; behavior crosses only through LEG obligations and clean witnesses; legacy source 6c4a46f8; clean-room base ada284a0 |
| DEC-049 | LOCK | Declared WorldImage entrypoint invocation | InvokeEntrypoint executes a ProgramImage bound by an admitted WorldImage entrypoint; persistent deployment; ASK or DO; effects only when the entrypoint contract declares them; capability from the WorldImage grant; a request cannot substitute a foreign ProgramImage while keeping entrypoint authority; identity binds WorldImageId WorldInterfaceHash EntrypointId ProgramImageId InvocationId AttemptId |
| DEC-050 | LOCK | Ephemeral query ProgramImage invocation | ExecuteQueryProgram executes a client-supplied canonical query-only ProgramImage ephemerally under a server-issued read-only query capability envelope; effects process installation and lifecycle hooks forbidden; the server validates the image independently and client compilation grants no trust; response binds ProgramImageId query authority digest historical cut source generation completeness proof disposition actual work result digest and receipt; ALLOW DENY DEFER may be data but never drive an effect |
| DEC-051 | LOCK | Source compilation and raw-BatQL authority boundary | BatQL source text is authoring input never executable authority; PakVM executes validated ProgramImages not source; ordinary NetBat permits InvokeEntrypoint and ExecuteQueryProgram and rejects CompileAndExecuteRawBatQL EvalText and ExecuteSource; source compilation is a separately admitted dev and admin CompilerPort not available to guests not implied by NetBat access and absent from the default server profile; it emits ProgramImage plus CompilationReceipt through the ordinary validation and admission path so source identity is provenance not runtime authority |

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

---
status: AUTHORITATIVE
contract_id: BP-WORLD-PORTS-1
authority_scope: WorldImage composition, instances, namespaces, host ports, and portability
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# World Images and Ports

## WorldImage

A BatPak application is explicit data:

```text
ContractImages
+ ProgramImages
+ ProcessContracts
+ schemas/codecs
+ capability requirements
+ imports/exports
+ origins/proof manifest
→ WorldImage (.vpak)
```

No Rust closure, ambient registration, or hidden linker inventory becomes application authority.

## Ownership

BatPak owns language-neutral image types, IDs, canonical framing, schemas, and interface descriptors. MacBat and BatQL produce image inputs. SyncBat's `world` plane links/validates instances. PakVM executes programs. Bvisor admits instances and attempts.

## Image identities

```text
ContractImageId
ProgramImageId
WorldImageId
WorldInterfaceHash
```

WorldImageId commits to the complete canonical package. WorldInterfaceHash commits only to exported entrypoints, schemas, capabilities, results, and receipt contracts.

## Composition

Linking is explicit, deterministic, and collision-checked across contract IDs, schema/field IDs, event kinds, effects, capabilities, projections, process IDs, kernels, imports, exports, origins, and proof manifests.

Order-independent inputs produce the same canonical image where composition declares order irrelevant.

## World instance

A live instance binds:

```text
WorldImageId
WorldInstanceId
WorldGeneration
Store/journal grants
capability table
runtime process state
budget state
attempt table
```

Live capability authority cannot be serialized out of the instance.

## World namespace

A filesystem-like namespace is an ergonomic projection, not a host filesystem:

```text
world:/journal
world:/state
world:/input
world:/output
world:/artifact
world:/secret
world:/effect
world:/receipt
world:/process
world:/system
```

Typed WorldPath values resolve to ResourceRef plus rights requirements. Knowing a path grants no authority.

## Port families

```text
JournalPort
ArtifactPort
ClockPort
EntropyPort
TransportPort
CompilerPort   development/TestPak only
```

Requests and responses bind world, generation, attempt, capability slot, limits, and schema identity.

## Sans-I/O execution

```text
PakVM step
→ NeedPort(request)
→ host driver performs mechanism
→ PortResponse
→ resume same attempt
```

A native driver may block synchronously. A browser driver may await OPFS, IndexedDB, Web Crypto, Fetch, or WebSocket and resume later. The guest and logical runtime do not acquire an async-runtime dependency.

## Minimal semantic profile

`batpak` and `syncbat` must compile their semantic profiles with `#![no_std]` plus `alloc`. PakVM instruction semantics, Bvisor admission state, logical runtime transitions, WorldImage values, and port protocols therefore cannot depend on host filesystem, process, socket, thread, wall-clock, or entropy APIs.

Host drivers live behind explicit `std`, browser, or embedded adapter profiles. A mechanism profile may suspend and resume execution differently, but it must preserve the same request identity, result type, receipt fields, bounds, and recovery disposition.

## Portability profiles

The same semantics target native embedded applications, self-hosted servers, container deployments, browsers/workers, and deterministic in-memory tests.

A browser storage adapter must qualify its own atomicity, ordering, visibility, durability, quota, crash/reload, ownership, and size behavior. It never claims POSIX semantics by analogy.

## Artifact plane

Large immutable inputs/outputs, captured streams, proof bundles, and transferable tile buffers live in a content-addressed artifact plane. `.fbat` records references, attempts, and dispositions. `.vpak` may embed small immutable resources or refer to artifacts by digest.

No public artifact bundle extension is frozen until a real packed-artifact adopter earns it.

## Host composition succession

Legacy module manifests, interface fingerprints, hooks, jobs, and handlers are replaced by ContractImage, ProgramImage, WorldImage, WorldInterfaceHash, lifecycle events, ProcessContract, and compiled programs. The old shell is dissolved by ownership rather than copied into SyncBat.

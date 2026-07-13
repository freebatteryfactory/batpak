[![crates.io](https://img.shields.io/crates/v/batpak.svg)](https://crates.io/crates/batpak)
[![docs.rs](https://docs.rs/batpak/badge.svg)](https://docs.rs/batpak)
[![CI](https://github.com/freebatteryfactory/batpak/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/freebatteryfactory/batpak/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/batpak.svg)](#license)

# batpak

**batpak is an embedded, sync-first event store for Rust: an append-only,
hash-chained journal with typed events, verifiable receipts, deterministic
replay, and derived projections — no server, no async runtime.** Events append
to a per-entity log, each hash-bound to its ancestor with Blake3; every
accepted write returns a verifiable (optionally Ed25519-signed) receipt; state
is rebuilt by deterministic replay into projections. Use it for tamper-evident
audit trails, local-first app logs, compliance evidence, and event-sourced
application state.

Product overview: [freebatteryfactory.com/batpak/overview](https://freebatteryfactory.com/batpak/overview)
· Documentation map: [DOCS.md](DOCS.md)

## Quickstart — embedded event sourcing in Rust

```sh
cargo add batpak
```

```rust
use batpak::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, EventPayload)]
#[batpak(category = 0xF, type_id = 1)]
struct PlayerMoved { x: i32, y: i32 }

let store = Store::open(StoreConfig::new(dir.path()))?;
let coord = Coordinate::new("player:alice", "room:dungeon")?;
let receipt = store.append_typed(&coord, &PlayerMoved { x: 10, y: 20 })?;
let fetched = store.get(receipt.event_id)?; // immutable, verifiable
store.close()?;
```

The fully annotated version is [First Shape](#first-shape) below; run it for
real with `cargo run -p batpak-examples --bin quickstart`.

## The Factory Frame

The Free Battery Factory makes batteries for software boundaries.

> A battery does not own the machine. It powers one boundary.

**batpak** is the core battery. The **family** around it wires that journal into
larger hosts — `syncbat` for runtime dispatch, `netbat` for NETBAT/1 network
terminals, and `hostbat` for manifest-owned host contracts. Circuits connect
batteries without one owning another's state.

batpak is not a database server, queue, ORM, workflow engine, async runtime,
network framework, or agent framework. Callers own process model, disk
placement, runtime integration, network boundaries, and application authority.

## What Ships

| Battery / surface | Crate / package | Role |
| --- | --- | --- |
| Core journal | `batpak` | Append-only store, HLC frontier, receipts, replay, projections |
| Runtime dispatch | `syncbat` | Operation descriptors, handler registration, runtime receipts |
| Network terminal | `netbat` | NETBAT/1 frames, bounded request/response |
| Host contract | `hostbat` | Host composition manifest, H-interface fingerprints, subscription descriptors |
| Boundary supervisor | `bvisor` | Confined workload execution over a fail-closed boundary contract (Linux landlock/seccomp/cgroups, Wasm/WASI) |

See [04_BATTERIES.md](04_BATTERIES.md) for the full battery map and
[05_TERMINALS.md](05_TERMINALS.md) for the ten-op NETBAT profile.

## Two Doors

**Door A — Rust embedded.** Add the core crate and open a `Store` on a
directory you own:

```sh
cargo add batpak
```

**Door B — Networked host.** Add the runtime and wire crates, then prove the
NETBAT surface with integration tests:

```sh
cargo add syncbat netbat
cargo test -p netbat
```

The ten reference NETBAT terminals — `bank.commit`, `event.query`, `event.get`,
`receipt.verify`, `event.walk`, `system.heartbeat`, and the four `evidence.*` ops — are documented
in [05_TERMINALS.md](05_TERMINALS.md). The Rust `hostbat` crate derives the live
host contract from its registered parts as a `HostCompositionManifest` (per-module
`HostModuleManifest`s folded into one host fingerprint). The
`hostbat_supervised_jobs` witness under `bpk-lib/crates/batpak-examples/src/bin/`
declares and drives that surface end to end.

## First Shape

```rust
use batpak::prelude::*;

// One struct binds a Rust type to its event kind at compile time.
#[derive(serde::Serialize, serde::Deserialize, EventPayload)]
#[batpak(category = 0xF, type_id = 1)]
struct PlayerMoved {
    x: i32,
    y: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;

    // Open the Store: the battery. It owns this directory and nothing else.
    let store = Store::open(StoreConfig::new(dir.path()))?;

    // A Coordinate names where events belong: an entity within a scope.
    let coord = Coordinate::new("player:alice", "room:dungeon")?;

    // Append a cell of source truth. The receipt is verifiable evidence
    // of exactly what was accepted.
    let receipt = store.append_typed(&coord, &PlayerMoved { x: 10, y: 20 })?;

    // Read it back. Accepted events are immutable.
    let fetched = store.get(receipt.event_id)?;
    println!(
        "stored {} at sequence {} in scope {}",
        fetched.event.header.event_id,
        receipt.global_sequence,
        fetched.coordinate.scope()
    );

    store.close()?;
    Ok(())
}
```

Run it for real: `cargo run -p batpak-examples --bin quickstart` under `bpk-lib/crates/batpak-examples`.

The full beginner path is eight jobs — open, append, page commit order with
`query_entries_after`, point-read with `get`, walk hash-chain ancestry with
`walk_ancestors`, verify receipts, derive projections, close. See
`bpk-lib/crates/batpak-examples/src/bin/eight_jobs.rs` for the contract example and the
[cookbook](cookbook/README.md) for task-shaped recipes. The cookbook is the
task-shaped field guide: each recipe maps intent to API to proof surface, and
its index lives at [cookbook/README.md](cookbook/README.md).

## Scale And Composition

A **journal** is one `Store` on one `data_dir` with one exclusive writer. That
scope is the **local truth boundary** — truth is bounded to that journal, not
denied to distributed systems.

**Scale out** with multiple journals and explicit circuits: `netbat` routes,
cross-store observations, and host wiring documented in
[08_CIRCUITS.md](08_CIRCUITS.md) and [11_INTEGRATION.md](11_INTEGRATION.md). There is no
single `global_sequence` across separate store roots, and no in-core Raft over
one mutable directory.

**HLC** (hybrid logical clock) is the per-journal frontier inside one writer:
accepted → written → durable → visible → applied watermarks. `wait_for_durable`,
batch gates, and projection progress use those watermarks. HLC coordinates
durability and visibility inside a journal; it is not cross-machine consensus.

## Does batpak require an async runtime?

No. The core API is synchronous end-to-end: no tokio, no executor, no
`.await`. One exclusive writer thread owns each journal, and cross-thread
coordination happens over bounded channels inside the crate. Async
applications embed batpak by bridging at their own boundary (for example
`spawn_blocking` under tokio). The sync-first contract is part of the model —
see [02_MODEL.md](02_MODEL.md).

## How is batpak different from SQLite, sqlx, or EventStoreDB?

The decision-stage grid, for when you are weighing alternatives:

| Concern | batpak | `sqlx` on SQLite/Postgres | EventStoreDB client | Hand-rolled append log |
| --- | --- | --- | --- | --- |
| In-process, no server | ✅ embedded | ⚠️ file DB or DB server | ❌ dedicated server | ✅ |
| Needs an async runtime | ❌ sync API | ✅ async | ✅ | your choice |
| Tamper-evident hash chain | ✅ Blake3, per-entity | ❌ | ❌ | DIY |
| Verifiable receipts | ✅ optional Ed25519 | ❌ | ❌ | DIY |
| Deterministic replay → projections | ✅ built-in | DIY | ⚠️ server-side | DIY |
| Typed payloads at compile time | ✅ `#[derive(EventPayload)]` | manual serde + SQL | manual | DIY |

SQLite gives you durable rows. batpak gives you durable rows plus proof:

- Every event is hash-bound to its per-entity ancestor with Blake3, so
  tampering and reordering are detectable, not silent.
- Every accepted write returns a receipt you can verify later — against the
  committed store, and against an Ed25519 signature when keys are configured.
- Projections are derived views rebuilt from the log by construction, so
  read models can never silently drift from source truth.
- Canonical bytes are stable inside the Rust substrate and sealed by Rust
  golden vectors; non-Rust clients are intentionally deferred until after the
  Rust host/schema contracts settle.

When batpak is the wrong tool:

| Need | Reach for |
| --- | --- |
| Ad-hoc SQL over relational data | SQLite or Postgres |
| Many writers on one mutable directory with leader election | A database server or etcd — batpak is one writer per `data_dir` |
| Maximum write throughput over verifiable history | batpak serializes appends through a single writer on purpose |
| Automatic Raft replication inside the core crate | Compose multiple journals and explicit host circuits instead |

## How do I verify an append receipt?

Every accepted write returns an `AppendReceipt`. Check it against the
committed store with `Store::verify_append_receipt`, or — when you hold the
ack-shaped wire fields rather than the receipt struct — with
`Store::verify_append_receipt_wire_detailed`. A party holding only the
receipt fields, the event's chain metadata, and the verifying keys needs no
store at all: `verify_receipt_claim` (in `batpak::store`) returns the same
verdict from portable inputs, backed by the same implementation the store
delegates to. The verification vocabulary (`Signed`, `UnsignedAccepted`,
`Invalid(reason)`) and the signing model live in
[07_RECEIPTS.md](07_RECEIPTS.md).

## Can You Trust A 0.x Store?

Judge the evidence, not the version number:

- A deep test surface — integration, property, crash-recovery, and
  cold-start suites, not a thin unit-test layer.
- Deterministic concurrency proofs with `loom`, not just stress tests.
- Property-based tests over hash-chain integrity and canonical encoding.
- Chaos testing with fault injection, including disk-fault integration.
- Mutation testing on critical seams, so the tests are themselves tested.
- 115 named invariants traced to 159 concrete artifacts — the `docs-catalog`
  integrity gate re-validates these counts against `traceability/invariants.yaml`
  and `traceability/artifacts.yaml` on every run and fails CI on any orphaned or
  stale claim, so this headline cannot silently rot; see
  [03_INVARIANTS.md](03_INVARIANTS.md) and [12_CONFORMANCE.md](12_CONFORMANCE.md).

All of it runs from one command surface: `just verify`.

## The Mental Model

batpak ships with an opinionated mental model. You can use the API without
ever adopting it — the code above is the whole beginner story. But composition
gets much easier once you think in it, because every boundary question
("who owns this state? where may it cross?") already has a name.

The Rosetta table — factory words on the left, the precise engineering
surface on the right:

| Factory word | Rust surface | Plain engineering meaning |
| --- | --- | --- |
| Battery | `Store` | An embedded append log that owns one directory, one boundary. |
| Journal | one `data_dir` | One append-only store root; one exclusive writer. |
| Cell | `Event` | An immutable typed record; source truth. |
| — | `Coordinate` | Names where an event belongs: an entity within a scope. |
| Terminal | named API entry points | The only places where state or evidence crosses the boundary. |
| Receipt | `Receipt` | Verifiable evidence of what was accepted, denied, or replayed. |
| Discharge | `Replay` | Rebuild state from the cells. |
| Gauge | `Projection` | A derived view: disposable, rebuildable, never source truth. |
| Frontier | HLC watermarks | Per-journal accepted → durable → visible → applied progress inside one writer. |
| Gate | `Gate` | Caller-defined policy evaluated before commit. |
| Circuit | host wiring | Connects terminals across batteries without hiding ownership. |
| — | `Capability` | Explicit authority to perform an operation. |

Factory words explain the shape. Engineering names stay precise in the API,
and factory language never renames a Rust contract unless the type model
earns that name. Deeper factory identity lives in [01_FACTORY.md](01_FACTORY.md).

## Reading Paths

Pick the door that matches your intent — the docs are one model, but nobody
is required to read all of them:

- **Evaluating?** You have already read enough. Run the quickstart, skim
  the [cookbook](cookbook/README.md), decide.
- **Building on the store?** [02_MODEL.md](02_MODEL.md) →
  [06_EVENTS.md](06_EVENTS.md) → [07_RECEIPTS.md](07_RECEIPTS.md) →
  [09_REPLAY.md](09_REPLAY.md) → [10_PROJECTIONS.md](10_PROJECTIONS.md) →
  [cookbook](cookbook/README.md).
- **Composing batteries or operating a host?** [01_FACTORY.md](01_FACTORY.md) →
  [04_BATTERIES.md](04_BATTERIES.md) → [05_TERMINALS.md](05_TERMINALS.md) →
  [08_CIRCUITS.md](08_CIRCUITS.md) → [11_INTEGRATION.md](11_INTEGRATION.md).
- **Auditing the guarantees?** [03_INVARIANTS.md](03_INVARIANTS.md) →
  [12_CONFORMANCE.md](12_CONFORMANCE.md).

Machine law lives in `bpk-lib/traceability/` and `bpk-lib/tools/integrity/`.
These root docs describe the current system; they are not a decision archive.

## Command Authority

Use the root `justfile`.

```sh
just list
just inspect
just verify
just perf-gates
just loom
just seal
just ship dry
```

`just` is the command counter. `xtask` is the factory machinery. `ast-grep`
inspects structural doctrine. Tests inspect behavior. Receipts preserve
evidence. `perf-gates` and `loom` are manual release-confidence proofs, not
part of `just verify`.

Raw `cargo` is an implementation detail unless routed through the explicit
escape hatch:

```sh
just cargo -- <args>
```

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

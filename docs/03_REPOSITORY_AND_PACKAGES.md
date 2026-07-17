---
status: AUTHORITATIVE
contract_id: BP-REPOSITORY-PACKAGES-1
authority_scope: repository cutover, package graph, and source tree
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Repository and Packages

## Repository decision

Use the existing GitHub repository and preserve its issue database, links, stars, security archaeology, and downstream evidence.

Create an orphan clean-room branch with no source-history parent. Preserve the existing implementation under:

```text
branch: legacy/main
tag: legacy-pre-cleanroom
```

No Git merge crosses from legacy into the clean branch. Imported behavior passes through the Legacy Semantic Obligation ledger.

## Root tree

```text
BatPak/
├── README.md
├── AGENTS.md
├── FINAL_RECONCILIATION.md
├── Cargo.toml
├── rust-toolchain.toml
├── justfile
├── docs/
├── companion/
│   └── BATQL_LANGUAGE.md
├── spec/
│   ├── architecture.rs
│   ├── invariants.rs
│   ├── dispositions.rs
│   └── legacy_obligations.rs
├── bootstrap/
│   ├── audit.py
│   ├── freeze.py
│   ├── materialize.rs
│   └── seedcheck.rs
├── crates/
│   ├── batpak/
│   ├── macbat/
│   │   ├── compiler/
│   │   └── macros/
│   ├── syncbat/
│   ├── batql/
│   ├── netbat/
│   └── testpak/
│       ├── corpus/          frozen assets (sole canonical home)
│       ├── fixtures/        hostile/negative fixtures (sole canonical home)
│       └── src/
├── apps/
│   └── batpak-cli/
├── examples/                batpak-examples package (src/bin/)
└── .github/
```

Frozen corpora and fixtures live only under `crates/testpak/`. Root `corpus/`
and `fixtures/` are forbidden target paths; there are no canonical package-local
mirrors.

## Package roles

### `batpak`

The public semantic and durable substrate. It owns `.fbat`; event, schema, codec, query, projection, tile, effect, image, receipt, identity, navigation, storage-port, compatibility, and recovery law.

It is not an empty facade.

### `macbat-compiler` and `macbat`

A physically contiguous subsystem under `crates/macbat/`. The compiler is an ordinary pure library. The proc-macro package delegates immediately into it.

### `syncbat`

One runtime package with five internal planes:

```text
src/runtime.rs + runtime/
src/pakvm.rs   + pakvm/
src/bvisor.rs  + bvisor/
src/world.rs   + world/
src/port.rs    + port/
```

`runtime` owns logical legality. `pakvm` owns program semantics. `bvisor` owns attempt admission and physical evidence. `world` owns linking/instances. `port` owns explicit host requests and responses.

### `batql`

Independent language frontend. It depends on BatPak image/schema contracts, not on SyncBat's scheduler or host drivers.

### `netbat`

Thin bounded transport over BatPak values and SyncBat world entrypoints.

### `testpak`

Dev-only top layer. It may inspect every package. Muterprater is one module inside it and owns mutation only.

### `batpak-cli`

Thin binary adapter named `batpak`. It composes commands from the production libraries and owns no semantic type, codec, runtime law, or proof rule.

### `batpak-examples`

Non-publishable public-surface witness at `examples/` (`src/bin/`). It depends only on the production libraries' public APIs (`batpak`, `syncbat`, `batql`), owns no semantic law, and has no dev-tooling dependency. Every example emits observable output and is executed through TestPak orchestration — never through a Cargo dependency edge from `testpak`. It is a compatibility witness, not another battery, so the production semantic package count does not change.

## Dependency graph

```text
macbat-compiler
      ↑
    macbat
      ↑
    batpak
    ↑    ↑
syncbat batql
    ↑    │
  netbat │
      \  │
     batpak-cli

TestPak is dev-only over all semantic packages.
```

Exact edges, profile classes, and strict layer numbers live in `spec/architecture.rs`.

```text
L0  macbat-compiler
L1  macbat
L2  batpak
L3  syncbat, batql
L4  netbat
L5  batpak-cli, batpak-examples
L6  testpak (dev-only)
```

`batpak-examples` (L5) imports `batpak`, `syncbat`, and `batql`; nothing imports
it, and it never imports dev tooling. Every dependency points to a strictly lower layer. Same-layer packages are siblings and may communicate only through lower-owned types or top-level composition, never by adding a convenience edge.

## Crate boundary test

A new crate must satisfy at least two:

1. independent semantic lifecycle;
2. multiple meaningful consumers;
3. required compilation or target boundary;
4. mechanism isolation modules cannot prove;
5. independent publication, compatibility, or proof surface.

A format name, noun, or purity preference does not earn a Cargo package.

## Qualification profiles

Package ownership and build qualification are separate questions. The target matrix is:

```text
batpak semantic    no_std + alloc
syncbat semantic   no_std + alloc
batpak native      std adapters
syncbat native     std drivers/adapters
syncbat browser    wasm32 host adapters
```

The semantic profiles include contracts, typed values, canonical parsing/encoding over caller-provided buffers, image validation, PakVM reference interpretation, logical runtime transitions, Bvisor admission state, and port request/response protocols. They exclude native filesystem handles, threads, sockets, ambient clocks, entropy providers, and browser APIs.

Semantic qualification is distinct from Cargo feature defaults. Per DEC-047, the `batpak` and `syncbat` default profiles are usable native `std` (a real `cargo add batpak` works); a consumer reaches the `no_std + alloc` semantic profile with `default-features = false`. Enabling `std` by default grants ordinary native usability only — it must never pull in the threaded, browser, encryption, mapping, or interop adapters, which stay behind their own explicit opt-in profiles added at their owning gate.

The native and browser profiles may supply mechanisms but may not change semantic result, error, receipt, replay, or authority contracts. Exact profile facts live in `spec/architecture.rs`.

## no_std realization (DEC-065)

The `no_std + alloc` profiles are realized, not merely asserted. The required semantic surfaces:

```text
batpak semantic (no_std + alloc)
    IDs and branded identities
    contracts and schemas
    canonical codecs over caller buffers
    EventFrame values
    paths and navigation
    content digests and commitments
    receipts and pure verification
    storage request/response protocols
    pure journal and recovery state transitions

syncbat semantic (no_std + alloc)
    ProgramImage and WorldImage values
    PakVM validation and reference interpretation
    runtime transition state
    Bvisor admission and attempt state
    port request/response protocols
    deterministic in-memory execution
```

Std/browser adapters — and only adapters — own the mechanisms:

```text
filesystem and memory mapping
threads
sockets
wall and monotonic clock providers
entropy providers
OS keychain/KMS bridges
OPFS / IndexedDB / Web Crypto
```

`default-features = false` must be **compile-proven**; a green build with defaults is not by itself evidence of the no_std surface. The qualification matrix that CI must exercise:

```text
batpak no_std + alloc
syncbat no_std + alloc
native std default
threaded opt-in
browser/wasm host
encryption opt-in
interop opt-in
```

The collection and canonical-order law is owned by `04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md`.

Every profile in the matrix preserves program semantics (DEC-065). The same program over the same inputs yields the same canonical observables on native and browser alike: an adapter supplies mechanism and evidence, never semantic meaning (LEG-066). Profile-owned differences are confined to mechanism, performance, and available capability. A capability a profile does not offer is refused with its own typed disposition; it is never emulated into a different program-observable result.

This document owns the package inventory, the feature posture, and the qualification matrix. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere.

Required proof rows, projected from docs/24 (qualification target: `DEC-065`; canonical proof-row owner: docs/24 Gauntlet):

```text
no_std_batpak_has_no_std_dependency_route
no_std_syncbat_has_no_std_dependency_route
default_std_does_not_enable_threaded_or_browser_adapters
browser_and_native_profiles_preserve_program_semantics
hash_map_iteration_cannot_influence_canonical_observables
```

## Package-internal source grammar

Packages organize around concepts, not universal layer folders:

```text
src/event.rs
src/event/encode.rs
src/event/replay.rs

src/receipt.rs
src/receipt/verify.rs
```

Tables, systems, ports, and kernels live under the concept they serve. Generated output lives under `src/generated/` and declares its source contract.

## Forbidden target topology

The clean target contains no standalone runtime-machine, Bvisor, PakVM, Host composition, storage-format, legacy testkit, xtask, or integrity package. Their useful meanings have explicit successors in the current graph.

It also contains no root `corpus/` or `fixtures/` directory: those assets live only under `crates/testpak/`. `spec/architecture.rs` lists both as forbidden target paths.

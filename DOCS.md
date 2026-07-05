# Documentation Map

One screen to find the right door. batpak's reading surface is the
[README](README.md) plus twelve numbered guides; pick the path that matches
your intent — nobody is required to read all of them.

## The Twelve Guides

| Guide | What it covers |
| --- | --- |
| [01_FACTORY.md](01_FACTORY.md) | The factory identity: batteries, boundaries, and why the metaphor exists |
| [02_MODEL.md](02_MODEL.md) | The core model: journal, writer, sync-first contract, HLC frontier |
| [03_INVARIANTS.md](03_INVARIANTS.md) | The named invariant catalog and what enforces each one |
| [04_BATTERIES.md](04_BATTERIES.md) | The battery map: `batpak`, `syncbat`, `netbat`, `hostbat` |
| [05_TERMINALS.md](05_TERMINALS.md) | The ten-op NETBAT terminal profile |
| [06_EVENTS.md](06_EVENTS.md) | Events, typed payloads, coordinates, canonical encoding |
| [07_RECEIPTS.md](07_RECEIPTS.md) | Receipts, signing, and verification |
| [08_CIRCUITS.md](08_CIRCUITS.md) | Composing batteries and cross-store circuits |
| [09_REPLAY.md](09_REPLAY.md) | Deterministic replay and rebuild semantics |
| [10_PROJECTIONS.md](10_PROJECTIONS.md) | Derived views: projections, caches, freshness |
| [11_INTEGRATION.md](11_INTEGRATION.md) | Embedding and operating batpak in a larger host |
| [12_CONFORMANCE.md](12_CONFORMANCE.md) | The testing doctrine: how the guarantees are proven |

## Reading Paths

- **Evaluating?** Read the [README](README.md), run the quickstart, skim the
  [cookbook](cookbook/README.md), decide.
- **Building on the store?** [02_MODEL.md](02_MODEL.md) →
  [06_EVENTS.md](06_EVENTS.md) → [07_RECEIPTS.md](07_RECEIPTS.md) →
  [09_REPLAY.md](09_REPLAY.md) → [10_PROJECTIONS.md](10_PROJECTIONS.md) →
  [cookbook](cookbook/README.md).
- **Composing batteries or operating a host?** [01_FACTORY.md](01_FACTORY.md) →
  [04_BATTERIES.md](04_BATTERIES.md) → [05_TERMINALS.md](05_TERMINALS.md) →
  [08_CIRCUITS.md](08_CIRCUITS.md) → [11_INTEGRATION.md](11_INTEGRATION.md).
- **Auditing the guarantees?** [03_INVARIANTS.md](03_INVARIANTS.md) →
  [12_CONFORMANCE.md](12_CONFORMANCE.md).

## Everything Else

- [cookbook/README.md](cookbook/README.md) — task-shaped recipes: each maps
  intent to API to proof surface.
- API reference — [docs.rs/batpak](https://docs.rs/batpak).
- [CHANGELOG.md](CHANGELOG.md), [CONTRIBUTING.md](CONTRIBUTING.md),
  [ROADMAP.md](ROADMAP.md), [SUPPORT.md](SUPPORT.md).
- Machine law lives in `bpk-lib/traceability/` and `bpk-lib/tools/integrity/`;
  historical decisions in `archive/decisions/`. These root docs describe the
  current system; they are not a decision archive.

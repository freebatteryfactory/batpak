---
status: AUTHORITATIVE
contract_id: TP-ROOT-README-1
authority_scope: product entrypoint and active reading order
last_reconciled: 2026-07-22
---

# ThreadPak

ThreadPak is an opinionated library of semantic primitives. Its flagship
composition is an embedded, sync-first, event-native database and runtime.

It preserves the logical thread from intent through accepted facts, bounded
execution, effects, outcomes, and evidence using self-explaining programs,
immutable events, explicit authority, and independently qualified
implementations.

## Current authority

Read these two tracked documents first:

1. [Product and Behavior](docs/PRODUCT_AND_BEHAVIOR.md) — the machine, semantic
   ownership, public behavior, and explicitly open decisions.
2. [Implementation Plan](docs/IMPLEMENTATION_PLAN.md) — dependency order,
   packet readiness, tests and independent oracles, stop conditions, and the
   current authorization boundary.

The canonical-byte document will join this active surface when its owning
packet is authorized and its requirements and evidence are ready.

The rest of the current tree is qualified implementation evidence and migration
input. It does not outrank the two documents above and is not the ordinary
reading path.

## Current implementation state

The reset begins with one substantive `threadpak` product crate, one `macros`
procedural-macro crate, and one `testpak` expert development crate. Open language,
networking-package, command, storage-profile, checker, publication, and physical
format choices remain absent until their governing decisions are recorded.

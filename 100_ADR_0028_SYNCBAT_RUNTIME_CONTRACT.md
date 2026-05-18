# ADR-0028: Syncbat Runtime Contract

## Status

Accepted for the 0.7.6 correction cut.

## Context

R4 hardened `batpak` core into the substrate boundary: durable event storage,
visibility, receipts, evidence, clocks, and delivery canals are now owned by
core. After retiring the in-workspace Rust `clawbat` kit crate, `syncbat` is the
only runtime layer between application code and the substrate.

That makes `syncbat` a contract surface, not a sketch. It must stay small, sync,
and explicit: callers register operations, dispatch byte-oriented checkouts, and
optionally persist operation receipts through batpak-owned append surfaces.

## Decision

`syncbat` owns these runtime semantics:

- operation descriptor validation and stable operation-name grammar
- module mounting and duplicate operation rejection
- checkout dispatch from operation name plus input bytes to handler output bytes
- handler failure classification through `RuntimeError::Handler`
- optional receipt emission through a configured `ReceiptSink`
- durable operation-catalog rows via `StoreRegisterCatalog`
- deterministic catalog rebuild from batpak store sequence order

`syncbat` does not own:

- network framing or transport
- async runtimes
- application kit vocabulary
- PCP, MCP, browser, or agent semantics
- native batpak store internals beyond public API calls

## Receipt Contract

When a checkout resolves to a registered operation:

- successful handlers emit a completed receipt when a receipt sink is configured
- failed handlers emit a failed receipt when a receipt sink is configured
- receipt-sink failure is fail-closed and surfaces as `RuntimeError::ReceiptSink`
- unknown operations do not emit receipts

Receipt hashes are policy-controlled by `ReceiptHashPolicy`. The default
deferred policy leaves hashes empty; `RawBytes` hashes input/output bytes without
changing handler behavior.

## Catalog Contract

`StoreRegisterCatalog` persists `RegisterOperationRowV1` events at one caller
chosen coordinate. Rows fold in store sequence order.

Valid lifecycle actions are:

- `put`
- `update`
- `delete`
- `supersede`

The writer API preflights current catalog state before appending. Rebuild still
fails closed on malformed rows, invalid descriptors, invalid effects, unsupported
schema versions, and conflicting lifecycle transitions.

Delete and supersede tombstones are terminal. A name cannot be reused after a
tombstone. A supersession replacement may exist only if its descriptor is
identical to the incoming replacement descriptor.

## Consequences

- `syncbat` receives a checked public API baseline.
- `syncbat` tests cite catalog invariants directly.
- Future application kits must consume `syncbat`; they do not live inside
  `batpak`.
- `netbat` can expose `syncbat`, but it cannot reinterpret runtime semantics.

## References

- `002_SYNCBAT_RUNTIME.md`
- `bpk-lib/crates/syncbat/src/`
- `bpk-lib/crates/syncbat/tests/`
- `bpk-lib/traceability/public_api/syncbat.txt`

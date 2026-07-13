# Events

An event records a domain fact at a coordinate.

Events are the source of truth. Projections, indexes, caches, and reports may be rebuilt or rederived from accepted events and their artifacts.

## Valid Append Shape

The exact Rust constructor may change, but a valid append supplies:

- a coordinate
- a payload
- enough metadata to canonicalize and link the event
- policy input for gates where required

Callers append product kinds (category `>= 0x1`, e.g. `DATA`). The system
category `0x0` and the effect category `0xD` are substrate-reserved and are
rejected at the public append surface with `StoreError::ReservedKind` (see
`EventKind::is_reserved`); the substrate emits those reserved kinds itself.

## Coordinates

A coordinate names where an event belongs. Keep the engineering name `Coordinate`; factory prose may describe it as where a cell lands in the pack.

## Payloads

Payload bytes are part of the event contract. When structured payloads are hashed or linked, canonical encoding matters.

## Schema Evolution

Payloads change shape over time. batpak versions that shape and decode old bytes on read; it never rewrites stored bytes (that would break the hash chain and signatures).

Each `#[derive(EventPayload)]` type carries `PAYLOAD_VERSION` (default `1`, set via `#[batpak(version = N)]`). A typed append (`append_typed` and the typed `submit`/`reaction` family) stamps that version into `EventHeader.payload_version`. Untyped / batch / denial / lifecycle appends stamp `0`; `0` means "legacy or app-managed" and is decoded tolerantly as the current shape. The version field rides inside the frame but outside the hashed region: the content hash covers payload bytes only and the signature cover is `event_id + sequence + coord + kind + prev_hash + content_hash + extensions`, so adding or stamping it moves no existing hash or signature.

Decode happens at one seam (`event::decode`). For a stored version `v` and current version `c`:

- `v == c` or `v == 0` — decode as today.
- `v < c` — run the registered `Upcast` chain (`v -> v+1 -> ... -> c`) over the value, then decode.
- `v > c` — hard `FutureVersion` error, everywhere including replay and cold-start scan. There is no downcaster; upgrade the reader.

### Safe vs unsafe changes

| Change | Safe without a version bump? |
|---|---|
| Add a field with `#[serde(default)]` or `Option` | yes — serde fills it |
| Reorder fields | yes — named MessagePack |
| Remove a field | yes — unknown keys are ignored |
| Add a non-defaulted field | no — bump version + add an upcaster |
| Rename a field | no — bump version + add an upcaster (a defaulted rename silently loses data) |
| Change a field's type | no — bump version + add an upcaster (compatible reprs coerce silently to a wrong value) |

Do not add `deny_unknown_fields`: it breaks the safe forward direction. The structural frozen-fixture lint guards the unsafe directions instead.

### Adding an upcaster

When a change needs a version bump:

1. Bump the struct's `#[batpak(version = N)]`.
2. Write an `Upcast` for each new hop: `impl Upcast for MyV{N-1}ToV{N} { const KIND = ...; const FROM_VERSION = N-1; fn upcast(value: rmpv::Value) -> Result<rmpv::Value, UpcastError> { ... } }`, transforming a value of `FROM_VERSION` into one of `FROM_VERSION + 1`.
3. Register it: `register_upcast!(MyV{N-1}ToV{N});`.
4. Freeze a fixture for the new shape (next section).

### Freezing a fixture

Frozen fixtures are the payload MessagePack bytes (journal form) under `bpk-lib/crates/core/tests/golden/payloads/<category>_<type_id>__v<N>.hex`, decoded by `assert_frozen_decode::<T>` with the current decoder to prove old bytes still decode. Regeneration is append-only: `GOLDEN_UPDATE=I_KNOW_WHAT_IM_DOING cargo test --test schema_evolution` writes a missing fixture but never overwrites an existing one — proof-of-compat bytes must not mutate. A new version writes `__v<N+1>` alongside the old file. Every `#[derive(EventPayload)]` type should have a fixture; the warn-first structural lint tracks the backlog in `FROZEN_FIXTURE_DEBT`.

A wire version tag only helps frames written after it lands; pre-versioning frames are `0` forever, so a future upcaster cannot repair a breaking change to legacy data. Additive changes stay safe regardless.

### Durable idempotency

`AppendOptions::with_idempotency(key)` makes the key the event id; a duplicate keyed append returns the original receipt as a no-op. This dedup is durable beyond an event's retention window: a dedicated sidecar `index.idemp` (magic `FBATID`, versioned, crc32fast CRC, written atomically — same posture as the checkpoint) records the minimal tuple needed to reconstruct the original receipt and survives `Retention` compaction, cold-start, and snapshot independent of event eviction. The sidecar is restored unconditionally and early on open and is never rebuilt from a segment scan (segments may have evicted the events). The authority is scoped to REAL user obligations: internal lifecycle one-shot keys never create one, a store that never used user idempotency keys publishes no image at all (and reopens freely without one), and the recorded expectation retires when no user obligation remains. Once a user obligation exists it is an AUTHORITY, and damage is never absence. Admission is by generation token: every publication mints a random image id that `store.meta` authorizes BEFORE the image lands and finalizes after, and cold start accepts only an image carrying the currently authorized (or in-flight pending) token — frontier arithmetic never admits an image, because a sibling fork can be farther ahead, and per-entity hash chains cannot see divergence in other entities. A corrupt sidecar, a sidecar missing while `store.meta` records a finalized authorization, and any unauthorized image (an old self image, classified stale by the diagnostic anchor; another lineage's image; a diverged, farther-ahead, or anchor-colliding sibling's image, classified foreign) are each a distinct typed open refusal; a stored version newer than the reader is a hard error, mirroring the `FutureVersion` stance above. Compaction publishes the new image durably BEFORE deleting the source segments it supersedes, so no crash boundary can leave compacted-away frames behind an older image. The replacement segments themselves are staged under a distinct name and take their final segment names only after the authority is published — the final filename is a truthful publication fact (see 03_INVARIANTS.md).

Growth is bounded by `IdempotencyRetention` (config `with_idempotency_retention`). The default `Hybrid { keep_sequences, max_keys }` is window-priority: the window is the inviolable correctness guarantee — a key whose original commit is within `keep_sequences` of the frontier is never evicted by the `max_keys` soft cap, which may only ever trim out-of-window keys. If within-window keys alone exceed `max_keys` (a key-rate spike) the window wins: the store temporarily exceeds the cap (bounded by rate × window) with a loud diagnostic, and `OverflowPolicy` (default `Warn`) decides escalation. Net: a within-window keyed retry is always a no-op regardless of load.

Derive operation keys with `IdempotencyKey::for_operation(domain, components)` — a length-delimited blake3 over `(domain, components)` (so `["ab","c"] != ["a","bc"]`). This is OPERATION IDENTITY, not a payload content hash: it answers "is this the same operation?", not "are these the same bytes?". Do not use it as a content-addressing scheme.

The authority is also exportable out-of-band. `Store::export_idempotency_authority` returns the one canonical opaque authority image — exactly the durable `index.idemp` bytes up to their covered generation and frontier, wrapped in an opaque `IdempotencyAuthorityExport`; there is no parallel checkpoint format and no embedder-visible tuple schema, and a store with no user obligation exports nothing (`Ok(None)`). Restore is a single offline door: `Store::restore_idempotency_authority` operates on a CLOSED store directory, takes the store-dir lock, validates lineage and coverage, rejects rollback, refuses while a pending or unresolved compaction transition exists, then mints a fresh local authority image id and republishes through the canonical authorize-publish-finalize protocol — the restore call is itself the authorization ceremony, so a dead store never needs to have pre-authorized the incoming image. Corruption, a future format version, foreign lineage, stale rollback, sibling divergence, entries beyond the image's declared coverage, and an unresolved pending-compaction transition are each a distinct typed refusal (`StoreError::IdempotencyRestoreRefused`). Because the directory is closed and locked for the whole restore, no keyed append can race it. Lineage continuity is the embedder's obligation — `store.meta` must ride with the copied directory (a segments-only rebuild mints a fresh lineage and is refused as foreign) — and because every publication mints a new generation, the latest export supersedes older ones: an older export is restorable only until a newer generation lands, then fails closed.

## Fork And Import

`Store::fork_with_evidence(dest, options)` materializes a self-contained store
directory and returns deterministic evidence. It does not open the fork. Opening
is caller-owned so the clone stays pristine until the caller decides to mutate
it.

Fork is filesystem-level sharing with Rust-like aliasing discipline: sealed
segments are immutable and may be reflinked or hardlinked; the active segment,
durable idempotency sidecar, visibility ranges, pending compaction marker, and
`store.meta` (the lineage identity the copied idempotency authority is bound
to) are copied. Regenerable cold-start caches are excluded by default. Symlink leaf
destinations are rejected.

`Store::import_events(source, selector, options)` re-applies visible source
events into a destination store. It is not a merge. Destination event ids,
global sequences, and per-entity predecessors are local to the destination.
Payload bytes and content hashes are preserved; correlation is preserved as
opaque metadata; causation is cleared so no source event id is forged as a
destination-local edge. Each imported event carries provenance keyed by the
caller-supplied source namespace and source event id.

## Envelope Boundary

The substrate event kind is not the same as a domain receipt taxonomy. batpak may
store one registered envelope kind, while the payload inside that event carries
domain-owned strings such as `receipt_kind`.

That split is intentional:

- batpak owns numeric `EventKind`, canonical bytes, hashes, receipts, indexes,
  and replay traversal.
- application layers own envelope schemas, domain receipt families, and payload
  dispatch.
- `event.query` returns metadata summaries only; it does not return payload
  bytes, extension maps, or decoded domain fields.
- `event.query` orders those summaries by `global_sequence`; it is not a
  domain graph walk.
- the `evidence.*` ops return batpak's own evidence-report bodies (and their
  `body_hash` identity) only; traversal never returns decoded domain payloads.

Use `event.get` to fetch payload bytes after traversal, then decode the envelope
above batpak.

## Ordering

batpak records ordering information required by the substrate. Application-level ordering beyond that must be modeled by the application or by explicit higher-level batteries.

## Corrections

Accepted events are immutable. Corrective work is represented by later events, not by editing old ones.

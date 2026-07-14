---
status: AUTHORITATIVE
contract_id: BP-STORAGE-TILES-1
authority_scope: durable journal, native event framing, storage authority, tiles, and recovery
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Storage, `.fbat`, and Tiles

## `.fbat` authority

`.fbat` is BatPak's append-oriented journal container and source-truth plane.

It owns:

```text
StoreId and lineage
one physical commit order
EventFrame identity
batch boundaries
visibility and durability frontiers
content digests and commitments
recovery classification
receipt references
```

It does not imply a separate product or package. It does not own application domain meaning or arbitrary large artifact bytes.

## `.fbat` and `.vpak`

They are siblings with different physics:

```text
.fbat
    append, rotation, group commit, torn-tail recovery,
    mutable authority companions, long-lived journal

.vpak
    immutable complete package, section directory,
    whole-image commitment, bounded random access
```

They may share small BatPak-owned primitives such as version identities, bounded lengths, digest types, canonical integer rules, and refusal taxonomy. They do not share one forced generic parser.

## One coordinated V2 boundary

Event framing V2, event commitment V2, schema/codec identity, and repaired time/order identity land as one durable migration.

Illustrative semantic envelope:

```rust
pub struct EventFrameV2 {
    pub frame_version: FrameVersion,
    pub contract: ContractId,
    pub event_kind: EventKind,
    pub schema: SchemaRef,
    pub codec: CodecRef,
    pub store: StoreId,
    pub coordinate: Coordinate,
    pub event: EventId,
    pub commit_point: CommitPoint,
    pub stream_position: StreamPosition,
    pub dag_position: DagPosition,
    pub hlc: Hlc,
    pub correlation: CorrelationId,
    pub causation: Option<EventId>,
    pub content_digest: ContentDigest,
    pub previous_commitment: Option<EventCommitment>,
    pub event_commitment: EventCommitment,
    pub payload: EncodedPayload,
    pub extensions: ExtensionSet,
}
```

Exact bytes are selected at Gate 2 and frozen with language-neutral goldens. The identity separation is already law.

## Payload codecs

Initial profiles:

```text
LegacyMessagePackV1  historical compatibility reader
BatTaggedRecordV1    native evolution-friendly authority record
RawOpaqueV1          explicit app-managed bytes under a declared contract
```

Historical bytes are never rewritten to make a dependency graph look cleaner.

## Integrity hierarchy

```text
exact payload bytes
→ ContentDigest
→ EventCommitment
→ predecessor topology
→ optional signature
→ optional inclusion proof
→ optional external anchor
```

CRC may detect accidental corruption early. It is not authenticity.

## Artifact roles

Every durable artifact declares exactly one role:

```text
SourceTruth
MutableAuthority
DerivedCache
DiagnosticOnly
SecretAuthority
```

Examples:

```text
segment frame             SourceTruth
idempotency image         MutableAuthority
visibility ranges         MutableAuthority
SIDX successor tile       DerivedCache
open/recovery report      DiagnosticOnly
key material              SecretAuthority
```

A missing or corrupt mutable authority is a typed refusal, not an empty state.

## Store and authority generation

Authorities and caches bind StoreId, AuthorityGeneration, source frontier, source commitment, and format version.

A foreign or stale derived cache is discarded and rebuilt. A foreign or damaged authority is refused unless an explicit restore transition authorizes replacement.

## Storage ports

The store depends on a BatPak-owned handle/transaction interface that can be implemented by native filesystems, in-memory models, embedded flash adapters, or browser persistence adapters.

The conformance contract covers open/create, exact reads/writes, append, sync, atomic publication, enumeration, locking, rename/replace, quota/full behavior, crash/reload, and refusal behavior. An adapter advertises only guarantees it proves.

## Tile definition

A tile is a bounded materialization of one schema and one layout. Contract remains the semantic primitive.

```rust
pub struct TileDescriptor {
    pub tile: TileId,
    pub schema: SchemaRef,
    pub layout: LayoutRef,
    pub role: AuthorityRole,
    pub generation: MaterializationGeneration,
    pub source: SourceBinding,
    pub rows: RowRange,
}
```

Tile lifecycle, trust, durability, completeness, authority role, and proof disposition are orthogonal facts.

## First production tile: SIDX successor

The first column tile separates hot location/query fields from proof and lineage fields:

```text
Hot
    event_id, entity, scope, kind, global_sequence,
    stream_position, lane, visibility, frame_offset, frame_length

Proof/lineage
    content_digest, event_commitment, previous_commitment,
    correlation_id, causation_id

Variable
    dictionaries, offsets, extension arenas
```

It is a generation-bound DerivedCache. The simple authoritative frame scan remains an independent rebuild path until semantic parity and workload evidence close.

## Recovery laws

- acknowledged durable events are never silently lost under the declared adapter model;
- partial batches are invisible, rolled back, or canonically refused;
- source truth is never manufactured from a cache;
- a cache cannot authenticate itself;
- recovery ends in committed prefix, rollback, or typed refusal;
- durable idempotency returns original receipts after reopen;
- visibility cancellation remains durable and hidden;
- fork, snapshot, import, and restore preserve their distinct identity and authority rules.

---
status: AUTHORITATIVE
contract_id: BP-STORAGE-TILES-1
authority_scope: durable journal, native event framing, storage authority, tiles, and recovery
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
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

FrameVersion versions the EventFrame envelope. FbatFormatVersion versions the surrounding .fbat container. BatTaggedRecordVersion versions a tagged payload record inside EncodedPayload. A coordinated migration may move all three in one release while preserving three independent version identities.

Exact bytes are selected at Gate 2 and frozen with language-neutral goldens. The identity separation is already law.

## Payload codecs

Initial profiles:

```text
LegacyMessagePackV1  historical compatibility reader
BatTaggedRecordV1    native evolution-friendly authority record
RawOpaqueV1          explicit app-managed bytes under a declared contract
```

Historical bytes are never rewritten to make a dependency graph look cleaner.

## Compression posture (DEC-063)

V1 is uncompressed where identity and recovery depend on exact bytes:

```text
.fbat authority frames             uncompressed
.vpak canonical semantic sections  uncompressed
large payloads and outputs         ArtifactRef (content-addressed plane)
derived tiles and artifacts        may use an explicit versioned compression profile
```

Transparent compression is forbidden: it would defeat canonical bytes, bounded decode, random access, corruption isolation, selective field decode, crypto-shred semantics, and independent verification. No compression algorithm is selected now.

A future compressed profile requires a `CompressionId`, canonical parameters, bounded decompression, a maximum expansion ratio, a digest-coverage law, compatibility rows, hostile fixtures, and a measured adopter. It must state separately whether identity covers stored compressed bytes, canonical uncompressed bytes, or both through distinct typed digests.

`FbatFormatVersion` and `BatTaggedRecordVersion` are distinct typed identities (DEC-064); a future format version that a reader cannot open fails with its own typed disposition, never a silent misread.

This document owns the compression posture and the future-profile admission requirements. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere. The two obligations below qualify at their own typed gates; sharing this paragraph does not merge their gate schedules.

Required proof rows, projected from docs/24 (qualification targets: `DEC-063` for the V1 compression posture, `LEG-053` for canonical open or typed refusal; canonical proof-row owner: docs/24 Gauntlet):

```text
compressed_fbat_authority_frame_is_rejected_in_v1
compressed_vpak_semantic_section_is_rejected_in_v1
compression_profile_requires_expansion_bound
future_format_version_fails_with_its_own_typed_disposition
```

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

Authorities and caches bind StoreId, AuthorityGeneration, source frontier, source commitment, and format version. Each product is itself identified: a `MaterializationId` names one materialization, and an `AuthorityImageId` names one published authority image.

A foreign or stale derived cache is discarded and rebuilt. A foreign or damaged authority is refused unless an explicit restore transition authorizes replacement.

## Authenticated history at open and restore (DEC-071)

The store owns the durable material that authenticated history verifies. `spec/architecture.rs` owns the typed profile matrix; `19_SECURITY_MODEL.md` owns the threat and the claims; this document owns the storage semantics:

```text
segment seal            a per-segment authenticity commitment over accepted bytes
history commitment      a global append accumulator, or an equivalent whole-history
                        commitment, that a later generation extends monotonically
witness binding         an external anchor binds StoreId, AuthorityGeneration, and
                        the history commitment; an anchor for another lineage,
                        generation, or accumulator does not verify this store
```

Open and restore behave according to the selected `AuthenticatedHistoryProfile`:

```text
InternalConsistency        verify local commitments and authority-generation
                           relationships; claim no authorship or freshness

SignedHistory              additionally verify segment seals, the history
                           commitment, and signer identity

ExternallyAnchoredHistory  additionally verify an independent monotonic witness;
                           fail closed when it does not verify
```

A restored older generation may satisfy every local commitment and every signature and still not be the newest history ever acknowledged. Coherence is not freshness. The store never infers freshness from internal evidence alone.

No byte layout is frozen here. The seal and accumulator encodings, the signature algorithm, and the witness protocol are G2 implementation and G9 qualification constants, not spec bytes.

## Shared publication algebra

Every durable publication in the system crosses one irreversible boundary. This document owns that boundary and its outcome vocabulary; `08_SYNCBAT_RUNTIME.md`, `09_BVISOR.md`, `10_WORLD_IMAGES_AND_PORTS.md`, `14_RECEIPTS_AND_EXPLANATION.md`, and `35_CRYPTO_AND_SECRET_AUTHORITY.md` consume it rather than re-inventing it.

Three axes stay orthogonal. Collapsing them is what forces higher layers to build corrective exoskeletons around ambiguous commit outcomes:

```text
commit knowledge        KnownAbsent | KnownCommitted | Unknown
receipt completeness    Complete | Incomplete
reconciliation posture  NotRequired | Pending | ReconciledCommitted |
                        ReconciledNotCommitted
```

A publication is `KnownCommitted` with an `Incomplete` receipt and `Pending` reconciliation, all at once. No single status enum can say that, so none is used.

### Irreversible boundary law

Before the durable commit boundary an operation may be refused, fail, be cancelled, or expire before admission. After it:

```text
committed                        is NOT failed
committed, receipt incomplete    is NOT failed, and is NOT outcome unknown
outcome unknown                  is NOT proof of non-commit
cancelled after admission        is NOT proof of non-commit
```

A durable publication that crossed the boundary is never reported as an ordinary operation failure. Lost acknowledgement leaves the caller with `Unknown`, which is an absence of knowledge, not evidence of absence.

Later reconciliation **appends or references new evidence**. It never rewrites the original durable event or falsifies the original attempt observation. A reconciliation record that edited history would destroy the only evidence that the ambiguity existed. The durable order this document owns is the DurableOrderWitness coordinate of DEC-075's composition (`02_SYSTEM_MODEL.md`); this document owns the law, the composition only binds it.

### Purpose-specific commands

The algebra owns the boundary and the outcome vocabulary. It does not own a universal command envelope. There is no `GenericPublicationCommand { kind: String, payload: Vec<u8> }`, and no other untyped payload drawer.

These stay distinct typed commands sharing only boundary semantics:

```text
event append
typed effect batch
artifact publication
checkpoint intent
authority-image publication
secret-authority publication
```

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

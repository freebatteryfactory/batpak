---
status: AUTHORITATIVE
contract_id: BP-SCHEMA-CODEC-1
authority_scope: schema, codec, representations, migration, native encoding, and compatibility
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Schema, Codec, and Migration

## Root law

Schema describes meaning. Codec describes canonical bytes. Layout describes physical organization. Rust shape is an authoring surface, not durable identity.

## Schema contract

A schema has stable ID, monotonic version, field identities, value types, bounds, default policy, unknown-field policy, compatibility rules, and golden vectors.

Field IDs are permanent. Removed IDs remain reserved. Rust field names may change without changing field identity.

## Value algebra

V1 supports only bounded, canonical types such as booleans, fixed integers, fixed bytes, bounded bytes/text, options, bounded lists, records, enums, typed paths, IDs, decisions, and explicit references.

Maps require a declared canonical key order. Locale-sensitive comparison and implicit numeric-string coercion are absent.

## Native representations

### BatTaggedRecordV1

Evolution-friendly authority record with explicit field IDs and lengths. Fields are canonically sorted. Unknown-field handling follows schema law.

### BatDenseRecord candidate

A dense exact-shape representation is deferred until a sealed image section or measured kernel proves it removes meaningful work. It is not required for V1 and cannot become a second authority format by convenience.

### BatColumnTileV1

Derived column buffers with validity bitmaps, offsets, dictionaries, and child columns. It is for scans, projections, indexes, and snapshots, not the journal authority by default.

### LegacyMessagePackV1

Compatibility reader and differential oracle for historical bytes. It no longer defines new contract meaning.

## Native codec contracts

```text
encoded length calculation
direct bounded encode into ByteSink
borrowed decode view
materialize when ownership is required
canonical re-encode
selective field access where layout permits
stable typed errors and limits
```

MacBat generates direct implementations for eligible contracts.

## Migration

Preferred migration is typed and adjacent:

```text
V1View<'a> → V2Owned → V3Owned
```

A small bounded dynamic `BatValue` may exist only for legacy/migration tooling when a typed transition cannot represent the old data. It supports only the BatPak value algebra and never becomes a universal serializer.

Stored historical event bytes are not rewritten. Readers interpret old codecs and versions under explicit compatibility law.

## Compatibility classes

```text
CompatibleRead
CompatibleWrite
RequiresDefault
RequiresTypedMigration
LegacyReaderOnly
BreakingNewContract
FutureVersionRefusal
```

Compatibility is mechanical and versioned. “Serde currently accepts it” is not a compatibility class.

## Serde succession

Serde is removed as semantic authority and from event-contract supertraits. Explicit JSON/Serde adapters remain for interoperability, diagnostics, and legacy formats behind named profiles.

The target is not a generic serializer clone. It is a smaller bounded canonical codec system for BatPak's closed algebra.

## One durable migration

EventFrameV2, schema/codec identity, event commitments, and repaired clock/order fields share one coordinated persisted boundary. Nearby sequential header migrations are refused.

## Proof

Native codecs require byte goldens, canonical round-trip, malformed/canonicality rejection, structured fuzzing, mutation seams, language-neutral fixtures, and differential checks against independent legacy/reference implementations where parity is claimed.

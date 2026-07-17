---
status: AUTHORITATIVE
contract_id: BP-IMPLEMENTATION-CONSTANTS-1
authority_scope: exact values intentionally selected during implementation gates
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Implementation Constants

## Purpose

These items are intentionally unresolved numeric or physical constants. Their semantic owners and selection procedures are frozen. They are not permission to revisit package or ownership architecture.

## G2 storage constants

```text
EventFrameV2 field IDs and physical order
frame/header magic and version numbers
BatTaggedRecordV1 tags and length encoding
maximum frame/payload/schema bounds
sidecar/authority format version IDs
store metadata and generation encoding
legacy compatibility matrix rows
```

Selection requires language-neutral goldens, canonicality rules, hostile length tests, old-reader disposition, and one coordinated migration record.

## G2 crypto and secret constants

The security properties and typed vocabulary are frozen now in `35_CRYPTO_AND_SECRET_AUTHORITY.md` (DEC-052/053/054, LEG-081). Only these exact values are selected at G2, before any durable encrypted bytes exist (DEC-055):

```text
exact AEAD algorithm and version
nonce width and construction
key width
AAD field set and canonical byte ordering
domain-separation context strings
key-backend durable image bytes
rotation and rewrap wire schemas
signature algorithm and version
external-anchor encoding
legacy keyset compatibility bytes
```

Selection requires language-neutral goldens and hostile vectors. No password KDF is in scope (DEC-054): V1 accepts no password or passphrase as key material.

## G4 BatQL constants

```text
parser token limits
AST/type depth bounds
canonical formatter version
stable diagnostic code allocation
initial built-in function IDs
```

The conceptual grammar may not change while selecting them.

## G5 PakVM constants

```text
.vpak magic/version/section IDs
PakVM opcode numbers
frame/arena/call-depth limits
work-unit weights
canonical instruction encoding
proof/origin section encoding
```

Selection requires reference decoder/interpreter, cross-language or independent vectors, malformed-image corpus, and mutation seams.

## Port constants

```text
first browser persistence adapter and advertised guarantee profile
native and embedded storage transaction limits
default artifact inline/reference threshold
transport frame/page defaults
```

An adapter may advertise a weaker profile or refuse. It cannot inherit another platform's claim.

## Performance constants

Work and latency thresholds are selected from declared workloads with anti-vacuous planted regressions. No benchmark target may weaken semantic checks to recover a number.

## Deferred extension

A public artifact-bundle extension is deferred until a real adopter needs packed artifact transfer. The artifact contract and content references are already owned; no package boundary depends on the extension.

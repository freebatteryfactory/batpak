---
status: AUTHORITATIVE
contract_id: BP-CRYPTO-SECRET-1
authority_scope: integrity roles, secret authority, key lifecycle, shred durability, and anti-rollback
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-14
---

# Crypto and Secret Authority

This document freezes two related but distinct planes: the **integrity roles**
that distinguish accidental corruption from authenticity, and the **secret
authority** law that governs keys, encryption, deletion, and anti-rollback.

It is property-complete and algorithm-light. It freezes the security properties
and typed vocabulary so a later gate cannot improvise them away. It does not
choose exact AEAD layouts, nonce encodings, AAD bytes, or algorithm
discriminants — those are G2 implementation constants (see below).

## Integrity roles

Six roles are six distinct types. A value in one role never silently stands in
for another:

```text
Checksum32       accidental corruption and framing triage only
ContentDigest    public exact-byte identity
CommitmentDigest domain-separated semantic commitment
MacTag           keyed authenticity inside one trust domain
Signature        publicly verifiable authority
ExternalAnchor   independently retained rollback witness
```

Laws:

- A checksum may detect accidental corruption early; it never implies authority.
- An unkeyed digest may identify content; it never authenticates its own
  expected value. "The digest matched" is not "the authority was verified."
- `ContentDigest` (which bytes) and `CommitmentDigest` (which semantic claim)
  are separate identities; one is not derived by relabeling the other.
- `MacTag` authenticity holds only within its keyed trust domain; it is not a
  publicly verifiable `Signature`.

This is the same separation carried by `LEG-019` (a cache cannot authenticate
itself) and `LEG-023` (digest, commitment, predecessor topology, signature,
inclusion, and anchor are separate claims). This document names their types.

## Secret authority vocabulary

`batpak::secret` owns the semantic law and typed vocabulary:

```text
KeyScope           granularity a key protects
KeyId              stable key identity
KeyGeneration      monotonic authority generation of a key/keyset
KeyBackendId       identity/version of the owning key backend
KeyState           lifecycle state of key material
ShredTransition    the identity of a completed destruction
RotationId         a key rotation event
RewrapId           a data-encryption-key rewrap event
AntiRollbackPolicy the rule binding readability to generation
```

The storage/host adapter supplies the key-backend mechanism. `batpak::secret`
never invents a cryptographic primitive.

## Key backend contract

An owning key backend (default or external — OS keychain, HSM, separate
database, or service) must support:

```text
get
put
delete
enumerate
durability barrier
rotation / rewrap
generation identity
```

An external key backend must preserve the same deletion, durability,
generation, and anti-rollback semantics as the default backend (`LEG-081`).

## Shred durability and anti-rollback (LEG-081)

Deletion of secret authority is durable and rollback-resistant:

- A shred is acknowledged **only after** the owning key backend has durably
  destroyed the required key material. Deleting from live memory and returning
  success before durable deletion is a violation.
- Secret authority binds `StoreId`, `AuthorityGeneration`, `KeyGeneration`,
  key-scope granularity, `ShredTransition` identity, and external backend
  identity/version where applicable.
- A foreign, stale, or pre-shred keyset cannot restore readability to a newer
  store generation. Restoring old key authority cannot reverse a completed shred.

The concrete resurrection failures this forbids:

```text
delete key from memory → return success → crash before durable delete
    → reopen with old key → plaintext resurrects        (forbidden)

shred current keyset → restore a valid pre-shred keyset → accept as healthy
    → plaintext resurrects                              (forbidden)
```

## Distinct dispositions

`Shredded`, `Unavailable`, and `KeysetMissing` remain distinct and are never
collapsed. A missing keyset is not silently the same as an intentional shred,
and neither is silently "empty."

## Secrets never leak into ordinary artifacts

Raw key material never enters:

```text
.fbat
.vpak / WorldImage
ordinary receipts
proof manifests
content-addressed artifact bundles
logs, Debug, or Display
```

Snapshot, fork, WorldImage, and artifact bundles exclude keys by default. Keys
travel only through an explicit, independent ceremony.

## Encryption, entropy, and no homemade crypto

```text
encrypted payloads require a qualified AEAD
entropy enters only through EntropyPort
no homemade cryptographic primitives
```

Random data-encryption keys come from the qualified entropy implementation, not
a local invention.

## No password-derived keys in V1

V1 accepts no human password or passphrase as key material. Therefore **no
password KDF is part of the V1 runtime contract**. A KDF appears only if a
later, explicit password-derived-key feature earns one through the normal
amendment path (DEC-054). There is no "implementation may pick a KDF" opening.

## Frozen now vs selected at G2

Frozen by this contract (properties and types):

```text
the six distinct integrity roles
the secret-authority vocabulary
AEAD required for encrypted payloads
entropy only through EntropyPort
no homemade primitives; no V1 password KDF
shred acknowledged only after durable deletion
old key authority cannot reverse a completed shred
raw secrets never enter .fbat/.vpak/WorldImage/receipts/proofs/logs/Debug/Display
checksums never imply authority; unkeyed digests never self-authenticate
```

Selected at G2 as implementation constants (see `32_IMPLEMENTATION_CONSTANTS.md`),
before any durable encrypted bytes exist, then frozen with goldens and hostile
vectors:

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

G2 must select these exact cryptographic constants before implementing durable
encrypted bytes. Implementation does not get to leave them ambient.

## Authenticated history is owned elsewhere (DEC-071)

This document owns signing-key ownership, secret authority, and verification-key trust. It does not own whole-store authenticated history:

```text
spec/architecture.rs   the typed AuthenticatedHistoryProfile matrix and witness policy
19_SECURITY_MODEL.md   the whole-store rollback threat and the permitted claims
05_STORAGE_FBAT_AND_TILES.md  segment seals, history commitment, open/restore behavior
```

The anti-rollback law above is about **key material**: a completed shred cannot be reversed by restoring a pre-shred keyset. That is a different attack from **whole-store history rollback**, where an attacker restores an older complete generation whose signatures remain valid. A signing key proves authorship; it does not prove that a signed generation is the newest one ever acknowledged. Only an independent monotonic witness carries that claim.

Signing and verification keys used by `SignedHistory` and `ExternallyAnchoredHistory` obey the key-backend contract and secret-authority vocabulary here. The signature algorithm, key storage, witness provider, and network protocol remain unselected (G2 implementation, G9 qualification).

## Ownership and proof

```text
batpak::secret        semantic law and typed authority vocabulary
storage/host adapter  the key-backend mechanism
TestPak               independent crash, rollback, and transplant witnesses
```

Required witnesses (proof owner TestPak; gates G2/G3), also carried by `LEG-081`:

```text
shred_ack_waits_for_backend_durability
crash_before_durable_key_delete_does_not_report_shred_success
reopen_after_ack_cannot_recover_shredded_plaintext
pre_shred_keyset_restore_is_rejected
foreign_keyset_generation_is_rejected
shredded_and_keyset_missing_remain_distinct
snapshot_and_fork_exclude_keys_by_default
external_key_backend_preserves_shred_semantics
```

Related obligations: `LEG-015` (crypto shredding destroys plaintext authority
without rewriting history), `LEG-019` (a derived cache cannot authenticate
itself), `LEG-023` (integrity claims are separate), `LEG-066` (platform
adapters supply mechanism and evidence, never semantic meaning), and the locked
decisions DEC-052 through DEC-055.

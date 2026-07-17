---
status: AUTHORITATIVE
contract_id: BP-DEPENDENCY-1
authority_scope: dependency roles, retain/demote/replace, profiles, succession, and proof
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Dependency Sovereignty

## Root law

The goal is not dependency zero. The goal is semantic zero-leakage.

BatPak owns meaning, identity, schema, lifecycle, availability, errors, authority, and proof obligations. Dependencies supply mechanisms behind owned contracts or explicit interop profiles.

## Dispositions

```text
Retain
    difficult reviewed mechanism behind a BatPak contract

Demote
    compatibility, legacy, tooling, or interop backend only

Replace
    native mechanism wins semantic parity, proof, complexity,
    portability, and actual workload evidence

Erase
    architecture removes the need for the abstraction
```

## Required dependency record

Every dependency row names semantic role, owner, public/private status, profile, types exposed, mechanism contract, independent oracle, removal/retention condition, license/toolchain posture, and current proof receipt.

## Initial posture

```text
uuid                 replace generator; retain as differential oracle temporarily
serde                remove as semantic authority; retain explicit interop profiles
rmp-serde/rmpv       legacy codec/migration profiles
serde_json           human/API/diagnostic interop
flume                private reference; split completion/permit/broadcast/mailbox
blake3               retain mechanism; BatPak owns domains and branded digests
signature/AEAD       retain reviewed crypto mechanisms
getrandom            retain behind EntropySource
libc/platform APIs   retain only in narrow admitted host kernels
memmap2              retain behind mapping contract while earned
parking_lot/rayon    profile and benchmark; architecture may erase need
inventory/ctor       supersede with explicit deterministic composition
```

## Serde succession

Contract and codec IR become authority first. Native implementations run beside legacy adapters. Differential tests and goldens close. New eligible writes use native codecs. Event contracts lose Serde supertraits. Old bytes retain old readers.

Do not build a general serializer clone.

## Flume succession

Hide raw public types immediately. Freeze Mailbox, Completion, Broadcast, and Permit contracts. Replace role by role only after model and workload proof. The writer lane comes after semantics, not after ideology.

## UUID succession

Own IdSource and EntropySource, branded IDs, layout, ordering/regression policy, deterministic simulation, and goldens. Do not reimplement OS/browser entropy.

## Architecture may erase abstractions

Single ownership plus immutable generation publication may remove shared maps/locks. Declared tile execution may replace ambient parallel iterators. The best replacement is often deleting the old abstraction rather than cloning it.

## Proof to remove

A dependency leaves only after public type containment, semantic parity, compatibility, negative fixtures, differential oracle, mutation, fuzzing, target profiles, workload results, and deletion receipt close.

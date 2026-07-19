---
status: AUTHORITATIVE
contract_id: BP-TYPE-SYSTEM-1
authority_scope: type ownership, availability axes, typestate, and repository source grammar
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Type System and Source Layout

## Purpose

Types make state, authority, and legal transitions visible. A developer or agent should understand a package's semantic vocabulary by opening `src/`, not by reconstructing it from helper functions and error strings.

## Domain-directory source grammar

A package is a dependency, compilation, and release boundary. A directory is a semantic ownership boundary. `lib.rs` is the crate façade. Every top-level semantic domain is a directory, and its files carry named roles:

- `mod.rs` — the domain façade: the `//!` charter, private `mod` declarations, and explicit re-exports. Public items pass through the domain root; no glob re-export exists.
- `types.rs` — the domain's private canonical noun carrier. The placement test: a type belongs in `types.rs` when it defines what the domain IS, is consumed by multiple sibling files, crosses the domain boundary, or carries stable identity, authority, proof, durable, wire, or qualification meaning. Private single-algorithm state stays with its algorithm.
- `inventory.rs` — closed authored rows: denominators and catalogs. `registry.rs` exists only for a genuine identity-to-metadata registry.
- verb-named operation files (`encode.rs`, `admit.rs`, `verify.rs`, …) — semantic transformations and policy.
- `ports.rs`, or a precise capability-named file — effect capability traits, never placed in `types.rs`.
- `errors.rs` — only for a domain-wide error algebra.
- `tests.rs` — private tests beside the seams they attack.

```text
src/event/mod.rs      domain façade: charter, module declarations, explicit re-exports
src/event/types.rs    private canonical event nouns
src/event/encode.rs   encoding operations
src/event/decode.rs   decoding operations
src/event/replay.rs   replay operations
src/event/tests.rs    refusal and seam tests
```

Subdomains recursively adopt the same grammar when they earn it: a sub-concept starts as one focused file and graduates to a directory only with independent vocabulary, multiple operations, and its own proof or effect boundary. Size alone never earns a directory. Top level is the exception: every top-level domain is a directory, universally.

No global type drawer, generic helper drawer, fake microcrate, wildcard façade, or duplicate semantic owner exists. Forbidden module names without an explicit ruling: `_types`, `common`, `shared`, `helpers`, `utils`, `functions`, `logic`, `misc`. A crate-root or cross-domain type drawer, public exposure of a domain's internal types module, and domain-significant types hidden inside unrelated operation files are all forbidden.

This grammar replaces a universal type drawer and the earlier root-concept-file spine [STALE-REF: DEC-007]. House-style honesty: `mod.rs` is valid Rust but not the layout the Rust Reference encourages today, which prefers a sibling `foo.rs` beside `foo/`. BatPak chooses `mod.rs` deliberately, by ruling, because the checked-in tree is itself an agent- and human-facing interface: a domain directory stays fully self-contained, at the accepted cost of worse editor-tab identity. This is BatPak house style, never a claim about the universally idiomatic modern Rust layout.

## Type placement

```text
cross-package public, durable, wire-visible, proof-bearing
    → batpak

compile-time compiler state
    → macbat-compiler

runtime-private process/program/attempt state
    → syncbat concept modules

language frontend state
    → batql

repository and proof facts
    → testpak
```

A package may have private mechanism types, but they live inside the owning domain directory and remain module-scoped. They appear in the navigation index; they enter the normative semantic-owner ledger only when they cross a real boundary (docs/28, 5.5E1).

No domain-significant named type is declared inside a function.

## Identity types

Same bits do not imply same identity:

```text
EventId
StoreId
ProgramImageId
WorldImageId
WorldInstanceId
ProcessInstanceId
TurnId
AttemptId
ReceiptId
ContentDigest
```

No package boundary mints a second branded type for the same already-validated identity unless a real trust transition occurs.

## Semantic and diagnostic names

Stable field IDs, schema IDs, contract IDs, opcodes, and reason codes are authority. Rust paths and source names are navigation unless explicitly frozen by a contract.

## Value availability

The closed value-state vocabulary is:

```text
Known
Missing
Pending
Invalid
Unavailable
Shredded
NotApplicable
```

These are not truth values.

## Truth, decision, completeness, freshness, proof

```text
Truth:        True | False | Pending
Decision:     Allow | Deny | Defer
Completeness: Complete | Incomplete(reason)
Freshness:    Current | Stale(bound) | UnknownFreshness
Proof:        Verified | LegacyWeak | Unverified | ProofUnavailable | ProofInvalid
```

The axes remain independent. A result may be current but incomplete, complete but unverified, or verified at an older source cut. `LegacyWeak` is a distinct proof disposition, not a synonym for `Unverified` (see `14_RECEIPTS_AND_EXPLANATION.md` and companion 4.5).

## Shared semantic sorts (DEC-069)

The numeric and evaluation model shares one closed, orthogonal set of semantic sorts. These are semantic sorts, not necessarily one Rust enum variant each; the axes stay orthogonal and there is no universal status enum. The canonical numeric law is owned by `37_NUMERIC_SEMANTICS_AND_AUTHORITY.md`.

```text
ExactInteger
FixedDecimal
ExactRatio
WideExact
ApproximateBinary
FiniteDyadicObservation
Interval
Availability
Truth
Decision
Completeness
ProofDisposition
Freshness
TypedMargin
Evidence
WorkObservation
Explanation
```

No public universal `Category`, `Functor`, `Monad`, or `Lattice` trait is exposed. Capability sets may use lattice laws where those laws genuinely apply; there is no higher-kinded-type imitation, no unsafe coercion, and no lint suppression used as a design tool.

Executable phases stay distinct:

```text
ParsedProgram
TypedProgram
AdmittedProgram
SpecializedPlan
ExecutedResult
```

`SpecializedPlan` is a derived optimization phase bound to one `AdmittedProgram` and one complete specialization key (DEC-073). It is never authority, never a `ProgramImage`, and never reusable under a different key; its identity, lifecycle, safety boundary, and cache law are owned by `07_PAKVM_ISA.md`.

## Time and order

```text
ObservedWallTime
MonotonicDeadline
Hlc
GlobalSequence
CommitPoint
StreamPosition
DagPosition
```

There is no generic timestamp, cursor, or position type that launders their laws.

## Schema, codec, layout, materialization

```text
ContractId       semantic behavior
SchemaId         logical shape
CodecId          canonical byte grammar
LayoutId         physical organization
MaterializationId one occurrence
ContentDigest    exact bytes
Commitment       semantic claim
```

One schema may have multiple codecs and layouts.

## Typestate

Typestate is reserved for affine transitions where reuse is dangerous:

```text
UntrustedImage → ValidatedImage
WorldPlan → AdmittedWorldPlan
Attempt<Planned> → Attempt<Running> → Attempt<Terminal>
TileBuilder → SealedTile → RetiredTile
```

Orthogonal evidence uses branded witnesses rather than giant generic parameter lists.

## Effects and capabilities

A capability type states what may be requested. A request type states what was asked. A response or receipt states what happened. No marker trait alone proves a physical postcondition.

## Errors

The owning package declares its error enum. BatPak owns shared error vocabulary such as stable codes, handling classes, fields, and source/cause policy. MacBat generates mechanical implementations and facts.

MacBat is the pen. The owner owns the sentence.

## Collection and canonical-order law (DEC-065)

Collections are chosen so that canonical outputs never depend on nondeterministic iteration:

```text
Vec and bounded arenas       column and buffer storage
BTreeMap / BTreeSet          where an ordered map is semantically required
sorted Vec                   small fixed catalogs
hash maps                    derived mechanism caches only
```

Hash-map iteration order never influences canonical identity, formatting, diagnostics, execution order, or receipts. A hash map may accelerate a lookup; it may not decide any byte that a digest, receipt, or canonical encoding commits to.

## Closed machine enums

Instruction kinds, lifecycle states, authority roles, delivery topologies, and terminal outcomes are closed enums. Exhaustive matches are required. Open extension occurs through explicit composition of data contracts, not wildcard arms.

## Required proof rows

`spec/proof/` owns proof-row identity and membership. docs/24 owns proof-row meaning. This document owns the domain law being pressured:

<!-- PROOF-REQUIREMENTS:BEGIN generated from spec/proof/inventory.rs by bootstrap/project.py; do not edit -->
| Guarantee | Required proof rows |
| --- | --- |
| DEC-065 | hash_map_iteration_cannot_influence_canonical_observables |
<!-- PROOF-REQUIREMENTS:END -->

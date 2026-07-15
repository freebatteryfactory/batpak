---
status: AUTHORITATIVE
contract_id: BP-TYPE-SYSTEM-1
authority_scope: type ownership, availability axes, typestate, and repository source grammar
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Type System and Source Layout

## Purpose

Types make state, authority, and legal transitions visible. A developer or agent should understand a package's semantic vocabulary by opening `src/`, not by reconstructing it from helper functions and error strings.

## Root concept-file spine

A primary concept owns one root file and may own a same-name directory:

```text
src/event.rs          primary event types and module declaration
src/event/encode.rs   encoding operations
src/event/decode.rs   decoding operations
src/event/replay.rs   replay operations

src/turn.rs
src/turn/plan.rs
src/turn/apply.rs
```

This replaces a universal type drawer [STALE-REF: DEC-007]. It keeps types obvious without separating them from the concept whose laws they carry.

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

A package may have private mechanism types, but they live under the owning concept, remain module-scoped, and appear in the generated Type Ledger.

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
ExecutedResult
```

`SpecializedPlan` is a derived future phase whose exact identity and lifecycle are frozen in 5.5D; its mechanics are not defined here.

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

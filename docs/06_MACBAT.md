---
status: AUTHORITATIVE
contract_id: BP-MACBAT-1
authority_scope: MacBat compiler, Contract IR, lowerings, diagnostics, and generated proof facts
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# MacBat

## Identity

MacBat is the compile-time semantic compiler. The subsystem contains two physically adjacent Cargo packages:

```text
crates/macbat/compiler   package macbat-compiler
crates/macbat/macros     package macbat
```

The proc-macro crate is a doorway. All parsing, validation, IR, lowering, diagnostics, origin, snapshots, and budgets live in the ordinary compiler library.

## Compiler law

```text
tokens
→ declaration
→ validation
→ typed Contract IR
→ lowering plan
→ emitted items
→ normalized artifact
→ final tokens
```

The compiler is pure: no filesystem, environment, clock, randomness, network, Cargo metadata, or repository path knowledge.

MacBat knows what one declaration means locally. TestPak resolves global ownership, composition, collisions, proof closure, and repository location.

## Contract IR

Contract kinds are closed and exhaustive. A kind enters only when one declaration becomes canonical, a real adopter exists, and every lowering/proof surface is named.

Expected families include:

```text
Error and ErrorContract
Event
Schema and Codec
Projection
Subscription
Operation and Effect
StateMachine
Process
EvidenceBody
Module/Composition facts
```

Each contract carries local identity, version, schema references, capabilities, effects, lowering descriptors, origin map, proof obligations, and mutation facts.

## Generated lowerings

One declaration may produce:

```text
Rust implementations
schema and codec descriptors
compatibility tables
ProgramImage/WorldImage facts
state transition tables
process/effect descriptors
transport descriptors
diagnostics and documentation
fuzz grammars and mutation sites
TestPak Repo IR facts
```

Lowerings are demand-driven. MacBat never scans the repository from a proc macro to discover consumers.

## Generated typed effect routing (LEG-046)

One Contract IR declaration mechanically determines the closed typed routing relationship for an admitted effect family. Applications do not hand-author raw `kind integer + byte payload + switch` dispatch for a Contract IR-owned family.

MacBat generates:

```text
typed route identity
exhaustive type-id dispatch
typed decode binding
effect-capability binding
handler binding
receipt projection binding
unknown-type refusal
duplicate or collision refusal
```

The application owns:

```text
domain transition logic
coordinate derivation
business validation
domain-specific handler behavior
```

BatPak and MacBat do not absorb application domain decisions. Higher-layer pressure to strengthen these primitives is evidence the substrate was thin; it is not authority to import anyone's domain furniture.

Generated routing fails closed when a declared variant has no route, two variants claim the same route identity, a decoder does not match the declared type, an effect capability is missing, a receipt projection is missing, or an unknown route identity is observed.

Route discovery is never ambient linker registration, startup constructors, process-global mutable registries, or inventory-style hidden discovery. The generated router owns structure and glue; it never generates or owns storage, cryptographic, concurrency, or application algorithms.

## Partial evaluation

MacBat specializes runtime work where contracts close the choices:

```text
generic field lookup          → direct field access
dynamic schema lookup         → static descriptor
transition validation         → exhaustive table
effect authorization          → compact generated effect row
generic encoder               → direct bounded writes
query field extraction        → generated access view
```

Runtime work may fuse validation, encoding, digest update, index extraction, and length accounting. Independent proof does not derive expected results through the same fused traversal.

## Internal representation

MacBat keeps its typed pass pipeline and domain IRs. It does not become a generic ECS engine. It exports normalized relational facts for TestPak composition and semantic diff.

## Diagnostics

Every diagnostic has a stable code, contract kind, origin span, blame item, structured required action, and human projection. When the repository destination matters, TestPak enriches the local diagnostic with owner/path facts.

## Snapshots

Snapshots are span-free canonical projections of declaration, Contract IR, normalized expansion, human expansion, and origin facts. Syn `Debug` output is never authority.

## Budgets

Every surface declares bounds for tokens, IR rows, diagnostic count, emitted items, depth, and observed compile work. Exceeding a structural bound is a typed refusal.

## Mutation support

MacBat emits mutation facts and optional selectable generated alternatives under TestPak mutation builds. Production code contains no mutation selector.

## Explicit composition

Ambient inventories and constructors are not the minimal composition model. Deterministic composition takes explicit contract images/facts, rejects collisions, and produces canonical order-independent output where declared.

## Authored-code law

MacBat generates repetitive structure and glue. Reviewed algorithms remain ordinary Rust kernels. It must not generate a complex algorithm merely to hide it from review.

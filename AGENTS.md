---
status: AUTHORITATIVE
contract_id: BP-AGENT-GUIDE
authority_scope: agent authority, navigation, stop conditions, and implementation discipline
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Agent Guide

## Mission

Implement this specification once, in dependency order, without reviving a rejected architecture as a convenience layer.

The specification is not a bag of suggestions. It is the current architecture. A code agent may refine exact signatures and byte constants only inside the named implementation gate and only while preserving the owner, direction, lifecycle, and proof obligations already frozen.

## First five minutes

1. Read `README.md`.
2. Read `docs/00_CONSTITUTION.md`.
3. Read `docs/29_STATUS_AND_SUPERSESSION.md`.
4. Read `docs/30_DECISION_AND_REJECTION_LEDGER.md`.
5. Read the package contract for the gate you are implementing.
6. Run `python bootstrap/audit.py .`.
7. Inspect `spec/architecture.rs`, `spec/invariants.rs`, and `spec/legacy_obligations.rs`.

Do not begin by reading legacy source. Legacy code is evidence only and may be consulted only through a row in `docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md`.

## Frozen runtime shape

```text
syncbat crate
├── runtime
├── pakvm
├── bvisor
├── world
└── port
```

Do not create standalone packages for those planes. Do not reintroduce a generic host-composition layer. Do not split `.fbat` into a separate storage product.

## Source layout law

Primary concept types live in a root concept file, paired with a same-name implementation directory:

```text
src/event.rs
src/event/encode.rs
src/event/replay.rs

src/turn.rs
src/turn/plan.rs
src/turn/apply.rs
```

No universal type drawer [STALE-REF: DEC-007] exists. No domain-significant named type is declared inside a function. A mechanism-private module type is allowed only under the concept it implements and must appear in the generated Type Ledger.

## Non-negotiable code doctrine

1. No `#[allow]` or `#[expect]` in tracked source.
2. No production `todo!`, `unimplemented!`, casual `panic!`, `.unwrap()`, or unexplained `.expect()`.
3. No wildcard match arm over a closed machine enum.
4. No dependency-owned mechanism type in an ordinary public API.
5. No hidden thread, executor, filesystem access, clock, entropy, network call, or process launch.
6. No effect outside a named port/capability and typed outcome.
7. No generated semantic mirror beside a hand-maintained authority.
8. No deletion without a successor obligation and witness.
9. No implementation may be its own only oracle.
10. No placeholder architecture: unresolved markers, delegated semantic choices, and unnamed future cleanup are failures.
11. `batpak` and `syncbat` semantic code must remain `no_std + alloc` clean; `std` belongs only in explicitly named adapter profiles.
12. Unsafe Rust is permitted only in an explicitly owned kernel/adapter file with a `SAFETY-CONTRACT:` record, a ledger row, independent witness, and demonstrated need; global suppression is forbidden.

## Agent stop conditions

Stop and report a structured finding when any of these changes:

```text
durable or wire bytes
semantic owner
dependency direction
capability or authority boundary
availability/truth/completeness meaning
proof strength
legacy obligation disposition
deletion timing
```

Also stop after three unsuccessful repair attempts on the same gate. Do not chase compiler errors by widening public APIs, adding reverse dependencies, or moving types to a more convenient owner.

A stop report contains:

```text
finding ID
files and contracts involved
observed contradiction
which authority documents disagree
smallest lawful choices
recommended ruling
work already attempted
```

## No musical chairs

When a concept does not fit, do not move it among packages until compilation succeeds. Identify the concept, owner, consumers, lifecycle, authority role, and proof owner. Then change the contract intentionally or stop.

## Review packet per gate

Every gate closes with:

```text
before/after tree
public signature dossier
type and owner ledger
semantic obligation dispositions
dependency and feature graph
generated surfaces
independent oracles
negative fixtures
mutation seams
compatibility receipts
deletion receipts
work observations
```

## Legacy rule

The old repository is a quarry, not a parent tree. Import laws, fixtures, vectors, and independent algorithms. Do not import folders, build machinery, duplicate authorities, or dependency topology merely because they already exist.

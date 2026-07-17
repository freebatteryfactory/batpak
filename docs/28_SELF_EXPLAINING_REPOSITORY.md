---
status: AUTHORITATIVE
contract_id: BP-SELF-EXPLAINING-1
authority_scope: source navigation, headers, type ledger, dossiers, diagnostics, and context
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Self-Explaining Repository

## Goal

A developer or agent should answer “what exists, who owns it, what may it do, and how is it proven?” from repository structure and generated facts, without memorizing a project-specific ontology.

## Tree as grammar

Root concept files expose the vocabulary. Same-name directories expose operations. Examples:

```text
src/event.rs
src/event/encode.rs
src/event/replay.rs

src/attempt.rs
src/attempt/admit.rs
src/attempt/reconcile.rs
```

Established computer-science names are preferred. Branded names remain only where they identify real BatPak products or formats.

## File header contract

Structured authority headers are RISK-TRIGGERED, not universal (5.5E1
anti-ceremony ruling). They are required for semantic-owner modules, kernels,
ports/adapters, unsafe files, concurrency mechanisms, durable codecs, and
proof-policy implementation:

```text
OWNS
READS
WRITES
FORBIDS
BOUNDS
PROOF
```

An ordinary implementation file does not repeat an architecture questionnaire;
concise module documentation suffices. This is concise structured module
documentation, not a duplicate architecture essay.

## Generated Type Ledger

Two indexes with different authority (5.5E1 ruling). The SEMANTIC-OWNER index
is normative and covers types that cross at least one real boundary: public
package, durable or wire, trust or authority, proof or receipt, unsafe or
concurrency, cross-package lowering, or compatibility. Each such type carries
owner, identity class, consumers, durable/wire status, lifecycle, and proof
owner, and the index rejects duplicate semantic owners and inline domain
types. The NAVIGATION index may discover every type and file for search, but
its rows are non-normative and carry no per-row lifecycle, proof owner,
receipt, or denominator obligation — "every named type gets a proof owner" is
denominator inflation, and this contract refuses it.

## File dossiers

Dossiers are RISK-TRIGGERED, never size-triggered or universal: unsafe or
concurrency mechanisms, durable codecs, kernels, and trust boundaries earn
generated dossiers with concept, public surface, dependencies,
unsafe/allocator/locking assumptions, complexity, mutation seams, fixtures,
and receipts. An ordinary large file does not.

## Projection law (5.5E1)

A closed authority-bearing inventory with a typed owner is projected for
humans and agents; it is never independently re-authored in prose. This is
the standing law behind the generated operator, gate, plane, crossing, seal,
terminal, reconciliation, and classification blocks, and it applies to every
future closed inventory. Typing is warranted only when all three hold: the
vocabulary is closed or finite; it affects authority, admission,
compatibility, proof, generation, or release; and an invalid value should be
unconstructable. Speculative armor is debt.

## Typed diagnostics

Diagnostics answer the relevant 5W1H dimensions:

```text
WHO/WHAT failed
WHERE the owner/source lives
WHEN/source cut involved
HOW to reach legal state
WHY the rule exists
```

They carry stable codes and structured actions.

## Generated navigation

```text
testpak inspect owner <symbol>
testpak inspect file <path>
testpak inspect contract <id>
testpak inspect proof <id>
testpak inspect dependency <package>
testpak inspect legacy <LEG-id>
testpak inspect decision <DEC-id>
```

MCP exposes the same typed queries.

## No hidden coupling

TestPak rejects forbidden dependency direction, duplicate validators, stringly cross-package identities, unowned files/items, dead generated artifacts, docs pointing to retired paths, and public items without proof paths.

## Context packet

`testpak context` is generated from Repo IR and receipts. It includes active authority, stale evidence warnings, obligation status, dirty tree, and next legal actions. It does not ask the next agent to reconstruct project history from chat logs.

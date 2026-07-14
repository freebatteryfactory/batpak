---
status: AUTHORITATIVE
contract_id: BP-SELF-EXPLAINING-1
authority_scope: source navigation, headers, type ledger, dossiers, diagnostics, and context
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
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

Authored implementation files state:

```text
OWNS
READS
WRITES
FORBIDS
BOUNDS
PROOF
```

This is concise structured module documentation, not a duplicate architecture essay.

## Generated Type Ledger

TestPak generates every named type with owner, file, visibility, identity class, consumers, durable/wire status, lifecycle, and proof owner. It rejects duplicate semantic owners and inline domain types.

## File dossiers

Large or critical files have generated dossiers with concept, public surface, dependencies, unsafe/allocator/locking assumptions, complexity, mutation seams, fixtures, and receipts.

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

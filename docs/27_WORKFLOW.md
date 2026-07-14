---
status: AUTHORITATIVE
contract_id: BP-WORKFLOW-1
authority_scope: architecture, review, implementation, stop conditions, and code-agent workflow
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Workflow

## Operating style

```text
explore broadly
→ write the whole conceptual shape
→ smell-check
→ attack from independent angles
→ reconcile contradictions
→ freeze owners and interfaces
→ implement in dependency order
→ integrate atomically
→ prove the finished machine
```

The compiler is evidence, not the architect.

## Lockstep gate

Each gate has one owner and one review packet. Parallel agents may gather evidence or implement isolated signed clusters. Their outputs are reconciled before integration.

## Review packet

```text
before/after tree
file dossier
type/owner ledger
semantic obligation ledger
public signatures
feature/dependency graph
generated surfaces
independent oracles
negative fixtures
mutation seams
compatibility/deletion receipts
work observations
```

## No musical chairs

A compile error does not authorize moving a type, adding a reverse dependency, widening a public API, or creating an adapter crate. Resolve ownership first.

## Repair budget

An agent gets three focused repair attempts for one gate failure. After that it stops with a structured finding. Repeatedly changing architecture to satisfy the compiler is prohibited.

## Small-file discipline

Split when one file owns two concepts, phase transitions, or independently testable kernels. Do not split a coherent algorithm into forwarding confetti. Drawer names such as `utils`, `common`, `helpers`, `manager`, `processor`, and `misc` are refused.

## Generated code

Generated files declare source contract and digest and are never edited manually. A generated successor deletes the hand-maintained mirror in the same gate.

## Compatibility work

Compatibility readers are isolated, versioned, corpus-backed, and never become the default writer. “Temporary” code has an owner, disposition, and deletion or retention condition.

## Big bang

The architecture is one destination. Savepoint commits and incremental clusters are allowed. Dual product models and indefinite transition layers are not.

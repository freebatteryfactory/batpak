---
status: AUTHORITATIVE
contract_id: BP-MIGRATION-1
authority_scope: clean-room branch, legacy import, data compatibility, issue closure, and cutover
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Migration and Cutover

## Repository strategy

Keep the existing GitHub repository. Preserve legacy source under `legacy/main` and an immutable annotated tag. Create the clean implementation on an orphan branch with no parent commit.

The legacy branch is never merged into the clean branch.

## Import law

```text
legacy source pointer
→ Legacy Semantic Obligation
→ clean canonical owner
→ mechanism disposition
→ clean implementation
→ independent witness
→ deletion/compatibility receipt
```

Code copying is not default. A reviewed kernel may be imported only when the ledger classifies the mechanism itself as valuable and its dependencies/unsafe assumptions are requalified.

## What may cross

```text
byte goldens and compatibility corpora
hostile fixtures
independent reference models
semantic invariants
public behavior and error expectations
algorithmic kernels that earn requalification
security findings and issue acceptance criteria
```

## What does not cross by default

```text
crate graph
folder topology
build scripts and command wrappers
ambient registries
hand-maintained mirrors
public dependency types
legacy feature matrices
platform backend product identity
```

## Historical data

Exact deployed `.fbat`, authority sidecar, receipt, snapshot/export, and schema versions are enumerated before EventFrameV2 closes. Clean readers support only versions with a signed compatibility row and corpus.

Historical bytes are never rewritten merely to normalize the new architecture.

## No dual product

The clean branch does not ship old and new composition/runtime models side by side. Compatibility readers may coexist with native writers. Architectural authorities may not.

## Issue lifecycle

Legacy issues remain open through contract mapping. An issue closes as superseded only after it has a stable clean contract/obligation ID, implementation gate, and proof successor. Closing prose alone is not migration.

## Default-branch cutover

The orphan branch becomes default after:

```text
this specification is signed
seedcheck and audit pass under pinned tools
Gate 0 skeleton matches spec/architecture.rs
legacy branch/tag are protected
issue mapping and agent context point to clean authority
```

Do not wait for the full implementation; leaving legacy as default would keep teaching agents the wrong authority.

## Migration receipts

Every imported reader, vector, fixture, kernel, and retired legacy path receives a receipt naming source commit/path, clean owner, semantic obligation, hash, review, proof, and final disposition.

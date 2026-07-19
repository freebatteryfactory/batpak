---
status: AUTHORITATIVE
contract_id: BP-AGENT-GUIDE
authority_scope: agent discovery adapter — mission, authority order, first steps, and stop conditions
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Agent Guide

This file is a thin discovery adapter. It points at the authorities; it does not
restate them. Source layout, code doctrine, the review packet, and the legacy
rule live in their numbered owners (`docs/04`, `docs/12`, `docs/25`, `docs/27`,
`docs/33`), not here.

## Mission

Implement this specification once, in dependency order, without reviving a
rejected architecture as a convenience layer. The specification is the current
architecture, not a bag of suggestions: a code agent may refine exact signatures
and byte constants only inside a named implementation gate, and only while
preserving the owner, direction, lifecycle, and proof obligations already frozen.

## Authority order

```text
typed spec facts in spec/   >   numbered domain docs   >   generated projections
```

A stale document is evidence of past intent, never current law. When two
authorities disagree, do not average them: apply
`docs/29_STATUS_AND_SUPERSESSION.md`, and report the contradiction if it survives.

## First five minutes

1. Read `docs/00_CONSTITUTION.md`.
2. Read `docs/27_WORKFLOW.md`.
3. Read `docs/33_AGENT_FINISH_LINE_CHECKLIST.md`.
4. Run the bootstrap battery (`audit.py`, `selftest.py`, and the
   `seedcheck`/`receiptcheck` oracles; see `bootstrap/README.md`).

## Hard stop conditions

Stop and report a structured finding — do not local-patch, do not file a
"follow-up" — when any of these hold:

```text
two current authorities contradict each other
an enforced gate is red
a proof-policy change is unclassified
```

The workflow lives in `docs/27_WORKFLOW.md`; the finish-line checklist lives in
`docs/33_AGENT_FINISH_LINE_CHECKLIST.md`.

---
status: AUTHORITATIVE
contract_id: BP-FINAL-AUDIT-1
authority_scope: final cross-document contradiction and debt audit
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Final Contradiction Audit

## Result

The final v1 architecture has one owner and one dependency direction for every package-scale concept. No known contradiction remains between narrative documents and `spec/architecture.rs`.

## Pass 2 defects removed

```text
separate storage package
empty product umbrella
standalone runtime VM package
universal type drawer
universal layer-folder grammar
all runtime libraries flattened to sibling-only core dependencies
package boundaries chosen mainly for purity
review record disagreeing with architecture facts
```

## Retained Pass 2 repairs

```text
document status/supersession
unified availability vocabulary
bidirectional protocol versus dependency direction
bounded long-lived push plus durable pull
honest K3 and NC1 scope
web/server/embedded ports
no_std + alloc semantic qualification for BatPak and SyncBat
BatQL compiler versus runtime execution
role-based TestPak
artifact plane
schema/codec/layout separation
delivery topology split
identity/time/navigation taxonomy
WorldImage composition
```

Each now lives under the recovered SyncBat/BatPak architecture.

## Cross-owner checks

- BatPak owns ProgramImage/WorldImage contracts and `.vpak` framing; PakVM owns executable validation/interpretation.
- BatQL compiles images without depending on SyncBat.
- SyncBat runtime owns restart legality; Bvisor owns attempt mechanics.
- PakVM cannot access host mechanisms except through Bvisor-mediated ports.
- `.fbat` remains BatPak source truth; `.vpak` is immutable executable packaging.
- NetBat carries SyncBat entrypoints but cannot interpret proof or domain meaning.
- TestPak owns Repo IR and commands; Muterprater owns mutation only.
- MacBat owns compile-time generation; it does not own runtime semantic types.

## Debt traps explicitly checked

```text
no dual old/new product model
no compatibility writer for legacy format
no “temporary” crate with no deletion condition
no duplicate ID or error taxonomy
no raw dependency type in public law
no hidden async runtime
no ambient std dependency in semantic profiles
no unledgered or misplaced unsafe kernel
no cache as authority
no self-generated expected result as sole oracle
no unbounded push memory
no generic timestamp/cursor
no cross-attempt result attachment
no outcome-unknown laundering
no unresolved architecture marker in authoritative docs
no operator relationship defined in two places (spec/operators.rs is the one operator authority; grammar/precedence/formatting/spoken tables are generated and independently re-audited)
no undefined grammar nonterminal referenced by a normative fence (grammar 15/15.2/15.3 is closed)
no proof disposition collapsed on projection (the five states VERIFIED, LEGACY_WEAK, UNVERIFIED, PROOF_UNAVAILABLE, PROOF_INVALID pass through unchanged)
no approximate value granted silent numeric authority (crossings require a receipted Quantize or IntervalDecision; DEC-069, docs/37)
no numeric classification collapsed into availability (a known NaN is not Missing or Pending; the axes stay orthogonal)
no exact/approximate authority boundary flattened into one universal numeric type
no universal hand-authored guarantee ledger (SEED/LEG/DEC/architecture/qualification keep native authority; the Guarantee Graph is derived, DEC-070)
no guarantee lifetime conflated with deletion condition or active/closed status (DeletionCondition::Never never forces Permanent or Active)
no guarantee relation inferred from prose similarity, shared owner, or document section
no clean_owner text or gate name treated as a successor guarantee edge (UntilSuccessor requires a resolvable typed successor; a gate-closed active obligation is UntilGate)
no bare SEED, DEC, or architecture law Discharges or Closes an active obligation before its qualifying evidence or closure receipt exists at Gate 0
no second gate identity type (spec/gates.rs owns GateId, GateSpec, and the gate inventory; every gate-bearing fact family references &[GateId], never a gate name in prose or a string)
no gate reference unresolvable against the inventory, duplicated within one fact, or out of canonical order
no gate identity authored twice (the docs/25 gate inventory is a generated projection of spec/gates.rs and is independently re-audited; the surrounding gate doctrine stays authored)
```

## Remaining implementation work is not architecture ambiguity

Exact byte/opcode numbers, adapter physics, and measured thresholds are listed in `32_IMPLEMENTATION_CONSTANTS.md`. Their owners, gates, constraints, and proof requirements are fixed.

## Validation rule

If a future audit finds a contradiction, it opens a finding against this contract and the affected domain contract. It does not silently revise generated facts or choose the newer-looking file.

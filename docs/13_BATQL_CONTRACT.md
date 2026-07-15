---
status: AUTHORITATIVE
contract_id: BP-BATQL-ARCH-1
authority_scope: BatQL compiler boundary, frozen grammar concepts, lowering, analysis, and execution contract
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# BatQL Architectural Contract

## Identity

BatQL is a typed temporal query and action language over immutable BatPak history. It is not a policy language with an event reader attached and not a general-purpose programming language.

The full language is frozen in [BATQL_LANGUAGE.md](../companion/BATQL_LANGUAGE.md).

## Package boundary

The `batql` package owns:

```text
lexer and parser
name and contract resolution
type and availability checking
historical-frame checking
capability/effect analysis
bounds and cost analysis
normalization and canonical formatting
partial evaluation
query and tile access planning
structured explanation preparation
ProgramImage and WorldImage input lowering
origin and diagnostic mapping
```

It depends on BatPak-owned contracts and image types. It does not own PakVM execution, SyncBat scheduling, Bvisor admission, transport, or host adapters.

## Public grammar

```text
WHO   subject or partition
WHAT  result, formulas, decision, or declared action
WHERE source and source-event filters
WHEN  coherent historical cut and optional observation-time range
HOW   fold, match, group, order, bounds, completeness, freshness, proof posture
WHY   explanation, provenance, evidence, cost, and proof request
```

Canonical formatting orders clauses `WHO → WHAT → WHERE → WHEN → HOW → WHY`.

## Top-level modes

```text
ASK   pure query, no effects
DO    admitted transaction, declared effects, always receipted
```

A pure image containing effect instructions is a compile-time and image-validation error.

## Compiler pipeline

```text
source
→ parse
→ resolve contracts and fields
→ type/availability analysis
→ freeze historical-frame requirements
→ effect/capability closure
→ bounds and cost analysis
→ normalize
→ partial evaluation
→ access/tile plan
→ ProgramImage
→ deterministic WorldImage composition input
```

## Compilation authority

BatQL compilation produces a canonical ProgramImage. Source may be compiled locally or through an explicitly admitted host `CompilerPort`. PakVM never executes BatQL text directly. Remote ad hoc queries submit query-only ProgramImages (`ExecuteQueryProgram`); effectful programs must enter through declared WorldImage entrypoints (`InvokeEntrypoint`). The invocation classes and the compilation boundary are owned by `docs/10` (DEC-049, DEC-050, DEC-051).

## Semantic algebras

### Query/dataflow

Source, subject, snapshot, time range, filter, fold, match, partition, group, aggregate, join, order, page, project, proof request.

### Formula/decision

Literal, binding, field, record, call, compare, K3 logic, case, aggregate, window, availability test, require, allow, deny, defer, reason, evidence, typed margin.

### Effect

Append, append batch, request effect, stage artifact, emit result, and checkpoint intent.

## Availability and decision

BatQL uses the shared value states:

```text
Known, Missing, Pending, Invalid, Unavailable, Shredded, NotApplicable
```

Predicate truth is `True | False | Pending`. Decisions are `Allow | Deny | Defer`. Completeness, freshness, and proof disposition remain separate.

`Pending` is never silently treated as false. Known annihilators remain lawful under strong K3.

## Historical frame

Saved queries name an explicit source cut. Interactive omission is captured as `AS OF HEAD` before execution and included in result identity. Relative ranges are anchored. Commit order, HLC filtering, stream position, and causality do not collapse into one time keyword.

## Arithmetic and ordering

Typed arithmetic legality (same-unit add/subtract, dimensionless multiply, ratio-producing divide, rejected cross-currency and Money×Money, mandatory named rounding modes, typed `INVALID` on overflow/divide-by-zero) is frozen in the language companion (DEC-060). The concrete operator facts — spelling, arity, fixity, precedence, associativity, exactness, and the six canonical rounding modes with their surface aliases — are owned by `spec/operators.rs` (OperatorSpec) and projected into the companion's generated operator blocks; OperatorSpec is the one operator authority and does not own the function, MATCH, declaration, or lexical grammar. `ORDER BY HLC` is not a bare HLC sort: it lowers to a total key — `HLC, GlobalSequence` within a journal and `HLC, StoreId, GlobalSequence` across journals — with half-open `[start, end)` ranges; the full law and forbidden uses live in `16_IDENTITY_TIME_AND_NAVIGATION.md` (DEC-061).

## Bounded traversal lowering (LEG-028)

Three surface forms lower to three distinct bounded primitives. Collapsing any pair is a defect, not an optimization:

```text
GET           -> bounded seek
CHILDREN OF   -> bounded one-level traversal
SCAN UNDER    -> bounded subtree traversal
```

`GET` never lowers to a full scan. `CHILDREN OF` never lowers to an unbounded subtree walk. `SCAN UNDER` never lowers to a single level.

Internal ISA nodes may include seek, bounded one-level scan, and bounded subtree scan. These are internal: no storage implementation detail becomes public BatQL syntax, and this lowering adds no syntax.

### Limit before decode

The item limit and work budget constrain discovery **before** unbounded decode, allocation, or materialization. This is not bounded traversal:

```text
scan everything -> decode everything -> truncate the returned vector
```

The `WorkObservation` states enough to prove which resource boundary was consumed. No page claims completeness merely because its returned vector reached the requested size.

## Partial evaluation

The compiler folds constants, eliminates unreachable branches, resolves schemas/fields/capability closure, prunes selectors, determines required columns, calculates bounds, prepares explanation templates, and records tile eligibility.

It does not execute host effects or query live journal state during compilation.

## Explanation product

One typed evaluation produces value, structured explanation, evidence summary, typed margin, completeness, proof disposition, and work observations. `WHY` projects from that result and does not rerun the decision through a second evaluator.

## Built-in kernels

A pure built-in kernel is content-identified, typed, deterministic where declared, capability-declared, and costed. Arbitrary host callbacks and runtime code generation are not part of V1.

## Language change law

The conceptual grammar is locked. A syntax or semantic change requires a demonstrated parser/type/conformance defect, a compatibility disposition, updated canonical formatter, spoken projection, fuzz grammar, and goldens. Fashion is not a defect.

## Numeric typing (DEC-069 / docs/37)

BatQL values type into the shared numeric sorts (exact authority, `ApproximateBinary`, `Interval`) through the existing `type_name` mechanism; the additive numeric type-vocabulary is recorded in the companion. Qualified approximation never becomes authority except through a receipted `Quantize` or `IntervalDecision`. The canonical numeric law is owned by `37_NUMERIC_SEMANTICS_AND_AUTHORITY.md`.

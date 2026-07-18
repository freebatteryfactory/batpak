---
status: AUTHORITATIVE
contract_id: BP-SPROUTING-1
authority_scope: candidate sprouting, immutable nursery lineage, bounded search, evaluation-set roles, realization-preserving versus law-changing promotion, and generate-wide/qualify-deep frontier qualification
supersedes: candidate and campaign wording previously implied only through DEC-048, DEC-062, DEC-073, and DEC-075
last_reconciled: 2026-07-18
reconciliation_epoch: cleanroom-v1
---

# Sprouting, Nursery, and Promotion

This document is clean-room operating law: how a candidate is sprouted, where
its lineage is persisted, how a search is bounded, which evaluation roles keep
oracles independent, how a promotion is classified, and how authority advances
through a speculative tree without granting stale bytes squatters' rights. It
freezes the campaign vocabulary before the typed lifecycle hardens. It does not
implement the search loop, the materializer, or the judge runtime; those are
later gates (F2/F3/F4) named concretely below.

## 1. Candidate is not authority

A generated, machine-assisted, searched, transferred, repaired, or
human-authored candidate remains NONCANONICAL until exact qualification and
promotion. Candidate origin confers NO authority. This is
SEED-CANDIDATE-NOT-AUTHORITY (derives from DEC-079 and DEC-080, refines
SEED-ONE-OWNER).

Candidate origin is recorded TRUTHFULLY — including a machine-assisted-synthesis
origin and a repair-of-prior-candidate origin — precisely so the architecture
never lies about provenance. Recording a synthesizer confers no authority; that
is exactly what makes the trust sentence structurally true. A truthful synthesis
record and a zero-authority candidate are the same fact stated once.

The typed candidate lifecycle is authored in F3 `spec/sprouting.rs`. A
`RealizationPosture` with THREE states exists so a stub cannot pass for a
realization:

```text
Missing     no implementation material present
Scaffold    structurally present but unqualified (a compiling stub)
Candidate   qualification-eligible material under review
```

A compiling `fn f() -> Result<()> { Ok(()) }` stub is a Scaffold —
structurally present, unqualified — and counts in the UNREALIZED denominator. A
binary realized|unrealized graph would go green over a tree of stubs; the third
state is why it cannot. Candidate lineage reuses existing typed relation fields
and mints NO parallel `CandidateEvidenceRelation` edge vocabulary.

## 2. Immutable nursery: persistent records, disposable workspaces

The nursery is BOTH immutable and disposable, split by object. It is not a
purely disposable downstream tree, and it is not a persistent nursery of
whole-tree bytes. The ruled resolution divides by what is being stored.

The nursery persists IMMUTABLE, content-addressed lineage RECORDS:

```text
candidate lineage identity
patch / content commitment
source-frontier commitment
dependency commitments
lineage
generator / pilot evidence
qualification receipts
benchmark / holdout evidence
reuse key
```

Full speculative WORKSPACES — the materialized `crates/` tree — are DISPOSABLE
rematerializations, never persisted as authority. After a trusted-frontier
promotion the loop:

```text
recomputes transitive staleness
preserves unaffected candidate units
invalidates affected descendants
rematerializes the full workspace
```

This is how the campaign earns compound interest — persisted records plus
transfer reuse plus invalidation — WITHOUT granting stale whole-tree bytes
squatters' rights. Records accrue; workspaces are rebuilt.

## 3. The candidate lineage identity is orthogonal, never a generalization

The candidate lineage identity is orthogonal. It does NOT generalize or
replace the kernel identity quartet or the content and version identities:

```text
kernel quartet   KernelContractId, KernelImplementationId,
                 QualifiedKernelId, KernelQualificationReceiptId
content/version  ContentDigest, ProgramImageId, ReceiptId, git commit SHA
```

A specialized kernel candidate may carry BOTH a candidate lineage identity (its
lineage object) AND a `KernelImplementationId` (its implementation under the
kernel contract). The identity system is deliberately separated into object,
generation, binding, and version axes to prevent exactly this collapse:

```text
object axis      what a thing is (candidate lineage identity, kernel contract)
generation axis  which produced instance (implementation identity)
binding axis     what it was qualified against (qualified/receipt identity)
version axis      which snapshot of bytes (ContentDigest, commit SHA)
```

The candidate lineage identity is authored in F3 `spec/identities.rs` alongside
those kinds. A lineage object is never a shortcut around the identity it must
still earn.

## 4. Bounded search and evaluation-set roles

Every candidate search declares finite bounds and receipts the actual work
performed — SEED-BOUNDED-SEARCH (derives from DEC-081):

```text
candidate bound        finite maximum candidates considered
logical-work bound     finite maximum logical work units
memory bound           finite maximum working memory
monotonic deadline     a monotonic-clock deadline, never wall-clock
```

The search receipts the actual candidates, work, memory, and elapsed monotonic
time against those declared bounds; an unbounded or unreceipted search is
refused.

Search, qualification, holdout, and regression inputs carry explicit
evaluation-set roles. F2/F3 mints:

```text
EvaluationSetRole::Search
EvaluationSetRole::Qualification
EvaluationSetRole::Holdout
EvaluationSetRole::Regression
```

Holdout evidence for a campaign cannot reuse its search inputs —
SEED-HOLDOUT-INDEPENDENCE (derives from DEC-081, refines
SEED-INDEPENDENT-ORACLE). A holdout that overlaps the search set proves nothing
it was tuned against and is refused. Transfer reuse is keyed and requalified
against upstream churn: a reuse key that no longer matches the current upstream
frontier does not license reuse.

## 5. Realization-preserving versus law-changing promotion; repair authority

Promotion distinguishes a realization-preserving change from a law-changing one.
F3 mints:

```text
CandidateChangeClass::RealizationPreserving
CandidateChangeClass::LawChanging
```

A realization-preserving candidate may promote through a mechanical or bounded
lane. A law-changing candidate routes to the architect and the DEC-074
proof-policy process. Candidate `RepairAuthority` is:

```text
Mechanical         a deterministic mechanical fix
BoundedSearch      a bounded, receipted search repair
ArchitectRequired  escalated to the architect; no self-service lane
```

There is NO ordinary "judge-repair lane" a candidate may take on its own
courtroom. When the JUDGE itself is wrong — a drifted independent model, a
mis-specified fixture, a stale judge-root view — the campaign STOPS as
ArchitectRequired:

```text
campaign stops as ArchitectRequired
judge / proof-policy amendment proceeds SEPARATELY under DEC-074
affected evidence becomes stale
campaign resumes from the NEW frozen judge snapshot
```

A candidate never edits the court that judges it. ArchitectRequired has a
terminal disposition home so it can never silently drop from the denominator: an
escalated campaign is accounted for as escalated, not quietly absent. DEC-080
(Enforcement, Lock) owns the promotion distinction.

## 6. Generate-wide, qualify-deep

A complete speculative implementation tree may be generated BEFORE
implementation gates close, but AUTHORITY advances only through
dependency-ordered qualification — SEED-GENERATE-WIDE-QUALIFY-DEEP (derives from
DEC-082, refines DEC-048 and DEC-075):

```text
later green cannot bless an unresolved earlier frontier
the trusted frontier advances dependency-first
candidate work cannot modify its frozen judge or prior evidence
```

Generation is wide and speculative; qualification is deep and ordered. A green
leaf above an unresolved dependency is not authority — the frontier has not
reached it.

## 7. Campaign-root topology and judge integrity

The speculative `crates/` tree cannot live in the tracked checkout.
`FORBIDDEN_TARGET_PATHS` (`spec/architecture.rs`) refuses `crates/`, `corpus`,
and `fixtures` from ever existing in the tracked tree. The campaign lives in an
EXTERNAL campaign root with physically disjoint roots:

```text
judge/      physically disjoint, read-only where practical, content-addressed
candidate/  writable
evidence/   append-oriented
nursery/    immutable / content-addressed
```

Digest binding DETECTS and INVALIDATES judge mutation: a candidate that mutated
a judge produces evidence that cannot verify. It does NOT make filesystem writes
"unrepresentable." Types do not make arbitrary filesystem writes impossible.
Isolation comes from the COMBINATION:

```text
physically disjoint roots
write containment
entry-type-aware before/after snapshots
exact digest binding
root-containment checks
evidence invalidation on judge change
```

The judge root is a content-addressed `FrozenJudgeSnapshot`, and every evidence
artifact binds the exact judge digest it was produced under. DEC-082
(Architecture, Lock) owns the whole-tree topology. The campaign closure graph
must enter `GeneratedView::ALL` before it may land; that view is authored in F4,
not F1.

## Ownership

This document is clean-room operating law authoring SEED-CANDIDATE-NOT-AUTHORITY,
SEED-BOUNDED-SEARCH, SEED-HOLDOUT-INDEPENDENCE, and
SEED-GENERATE-WIDE-QUALIFY-DEEP, backed by decisions DEC-079, DEC-080, DEC-081,
and DEC-082. It introduces no new `LEG-*` row.

```text
DEC-079           candidate is noncanonical; origin confers no authority
DEC-080           realization-preserving versus law-changing promotion (Enforcement, Lock)
DEC-081           bounded search and holdout independence
DEC-082           generate-wide/qualify-deep whole-tree topology (Architecture, Lock)
DEC-074           proof-policy amendment process for a wrong judge
DEC-048 / DEC-075 dependency-ordered advancement refined here
spec/sprouting.rs   F3 candidate lifecycle and RealizationPosture
spec/identities.rs  F3 candidate lineage identity alongside kernel and content
spec/architecture.rs FORBIDDEN_TARGET_PATHS and campaign-root topology
GeneratedView::ALL  F4 closure view a campaign must enter before landing
```

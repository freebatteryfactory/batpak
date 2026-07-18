---
status: AUTHORITATIVE
contract_id: BP-DYNAMIC-VERIFICATION-1
authority_scope: layered dynamic verification, model/trace separation, runtime conformance, coverage strength, claim kinds, and the anti-self-grading discipline
supersedes: verification wording previously implied only through DEC-073, DEC-075, and SEED-INDEPENDENT-ORACLE
last_reconciled: 2026-07-18
reconciliation_epoch: cleanroom-v1
---

# Dynamic Verification and Conformance

This document is clean-room operating law: how BatPak verifies dynamic behavior,
what independence a verdict requires, and what runtime observation may and may
not do to durable law. It is the Phase 5.5F1 charter — F1 authors the law, later
phases land the typed bodies. Where a type is named as forthcoming, this section
is the law that type will enforce, not a placeholder.

This document authors SEED-LAYERED-DYNAMIC-VERIFICATION,
SEED-MODEL-TRACE-SEPARATION, and SEED-RUNTIME-CONFORMANCE-NO-REWRITE, and is
backed by decisions `DEC-077` (verification axes, coverage strength, and claim
kinds) and `DEC-078` (enforcement posture and runtime conformance lock). It
states law, not a new durable behavior obligation, so it introduces no `LEG-*`
row.

## 1. Layered axes, no assurance ladder

Verification keeps its concerns as SEPARATE typed axes. A verdict is not a rank;
it is a tuple whose fields never collapse into one another:

```text
basis                which of the four verification bases produced the evidence
method               how the evidence was obtained (exhaustive, sampled, replayed, observed)
claim                what property the evidence claims (section 5)
coverage             the strength of the covered space (section 4)
enforcement posture  refuse-before, append-only, advisory (section 6)
freshness            whether the evidence is bound to the current changed boundary
result               conformant, divergent, refused, or not-comparable
```

No aggregate "assurance level", "confidence tier", or "maturity" ladder may
substitute for these axes. A single ordered enum spanning method + coverage +
freshness + result is forbidden by shape: ordering it would let a sampled-but-
fresh row outrank an exhaustive-but-stale row, or let a strong method launder a
weak claim. Each axis is compared only against its own kind.

This is SEED-LAYERED-DYNAMIC-VERIFICATION (derives from `DEC-077`, refines
SEED-AVAILABILITY-AXES). The executable shape-based guard — the audit that
refuses any type presenting these concerns as one ordered scalar — is authored
in F2 against the concrete verification types. This section is the law that
guard enforces.

## 2. Model/trace separation

Four distinct verification bases exist and never collapse:

```text
ContractProjection   a projection generated from typed authority; drives inputs, never concludes
IndependentReference a separately owned model, evaluator, or relation authored against the law
DirectBoundary       a hostile probe of the real boundary under test
RuntimeObservation   evidence recorded from an executing system
```

A generated projection can never be the SOLE independent oracle: it is derived
from the same authority it would grade. A self-observation can never be the SOLE
independent oracle either: a system observing itself corroborates but does not
independently conclude. An independent verdict requires evidence whose basis is
distinct from the subject it grades.

This is SEED-MODEL-TRACE-SEPARATION (derives from `DEC-077`, refines
SEED-INDEPENDENT-ORACLE). The typed basis is a field ON the proof requirement,
not a comment beside it: a `ContractProjection` offered on a route that demands
independent evidence is a compile error, not a runtime warning. That basis field
is authored in F2 `spec/verification.rs`.

## 3. Projection is oracle-ineligible, but its completeness still matters

A `ContractProjection` generates test material and never serves as its own
oracle or a voting witness. This does NOT mean a projection's completeness is
unmeasured or irrelevant — that inference is wrong and the law refuses it.

Projection completeness still matters because the projection drives the space in
which every other basis operates:

```text
valid-action generation      what the tester is allowed to attempt
transition enumeration       which state moves are reachable in test
test-space coverage          the denominator other bases sample or exhaust
runtime monitor schemas      the shape a RuntimeObservation monitor recognizes
generated proof inputs       the material fed to an IndependentReference
conformance replay           the trace a replay witness re-executes
```

A nonvoting projection that omits one transition still prevents that transition
from ever being tested — no independent oracle can grade a case the projection
never generated. Completeness of the projection and independence of the oracle
are two different obligations, and neither excuses the other.

The clean split:

```text
projection completeness   exact typed-authority <-> generated-projection parity
independent evidence      a separately owned model, evaluator, hostile boundary, or relation
```

The projection cannot VOTE as an independent oracle, but it must still faithfully
PROJECT the law. A projection that projects the law completely and votes on
nothing is exactly correct; a projection that votes is a category error; a
projection that is incomplete silently shrinks every downstream verdict.

## 4. Coverage strength is not a ladder

Verification coverage strengths are DISTINCT axes, not an ordered ladder. An
exhaustive enumeration, a bounded exploration, a sampled fuzz, and a runtime
observation cover different kinds of space; none is a higher rung of the others,
and none converts into another by counting.

Coverage-of-a-declared-model is NOT coverage-of-its-law. An "exhaustive within
the declared model" pass is exhaustive only over the declared subset:

```text
exhaustive within the declared model   complete over the declared states and transitions
                                        and NOTHING outside them
law-completeness                        a claim over the actual law, which the declared
                                        model may under-approximate
```

An exhaustive-within-model pass carries no law-completeness claim. If the
declared model omits a transition (section 3), an "exhaustive" pass over that
model is silent about it. The model's declared scope is part of the verdict, not
erased by the word "exhaustive".

This law lives inside `DEC-077`. The green-counting method is renamed in F2 so a
sampled or runtime-observed row can never launder into denominator strength: a
row counts toward exhaustive-denominator strength only when its method and
coverage axes admit it, never merely because it is green. Today the seam is
`ProofUnitTerminal::counts_green` in `spec/proof.rs`; F2 renames and re-types it
so the method/coverage distinction is structural, not a naming convention.

## 5. Claim kinds, including NonOscillation

A verdict names WHICH property it claims. The typed claim kinds are distinct;
one is not a weaker or stronger form of another. Among them:

```text
NonOscillation    progress-per-cycle: each control or retry cycle must admit new evidence
Liveness          something good eventually happens; may terminate yet thrash first
BoundedResponse   each obligation meets its deadline; may keep cycling forever within budget
Convergence       the system reaches a bounded stable region; a limit-cycle satisfies this
```

The distinctions are load-bearing:

```text
a cycle that meets every deadline           satisfies BoundedResponse, may violate NonOscillation
a run that eventually terminates            satisfies Liveness, may have thrashed (violated NonOscillation)
a limit-cycle in a bounded stable region    satisfies Convergence while making no progress
a control/retry cycle admitting no new evidence   violates NonOscillation and is refused
```

NonOscillation / progress-per-cycle means: a control or retry cycle that admits
no new evidence makes no progress and is refused. A system may meet deadlines,
eventually terminate, and sit in a bounded region while still oscillating without
progress; NonOscillation is the claim that separates real progress from motion.

NonOscillation is a CLAIM KIND inside `DEC-077`, not a standalone decision. It
gives the docs/17 "no retry oscillation without new admitted evidence" prose a
typed proof-row carrier — the row that carries a NonOscillation claim and its
admitted-evidence witness is authored in F2.

## 6. Runtime conformance: refuse-or-append, never rewrite; independent replay

Runtime verification has exactly two lawful effects on the running system, and a
strict prohibition:

```text
REFUSE   halt before an admitted irreversible boundary is crossed
APPEND   record conformance evidence alongside durable history
NEVER    rewrite semantic law, durable history, or proof disposition
```

A runtime monitor may stop an effect before it commits, and it may append what
it observed. It may not edit the law it checks against, mutate durable history,
or flip a proof disposition after the fact. Observation is not authorship.

This is SEED-RUNTIME-CONFORMANCE-NO-REWRITE (derives from `DEC-078`, refines
`DEC-075`). `DEC-078` (Enforcement, Lock) owns this law.

A runtime monitor requires an INDEPENDENT replay witness. A monitor generated
from a `ContractProjection` corroborates but never alone concludes Conformant —
by section 2 it shares its basis with the authority it grades. Concluding
Conformant requires two distinct witnesses of distinct basis, reusing the
two-distinct-witness quorum proven in the E6c cross-run work
(`spec/tier0_cross_run.rs`): a run compared only against itself, or a monitor
corroborated only by its own generating projection, is refused for the same
reason a run compared against itself is refused there.

## 7. Independent-evidence route kind is forthcoming

Independence has kinds, and the kinds are named vocabulary, not free text. F2
mints `IndependentEvidenceRouteKind` — the single vocabulary describing WHICH
kind of independent route supplies independence (a separately owned model, an
independent evaluator, a hostile boundary probe, an independent replay relation,
and the other admitted routes F2 enumerates).

Two consumers depend on it:

```text
PromotionRequirement::IndependentEvidenceRoute   requires at least one admitted independent route exist
the anti-self-grading SEED law                   forbids a verdict resting only on the subject's own basis
```

Both consume `IndependentEvidenceRouteKind` in F2. Tier 0 cross-run independence
remains a separate bootstrap concern (`BP-PUBLIC-API-CI-RELEASE-1`,
`spec/tier0_cross_run.rs`): it proves two hosted runs describe one source
snapshot, which is a distinct bootstrap independence from the verification-route
independence this vocabulary names, and the two are not merged.

This section does not define the enum. `IndependentEvidenceRouteKind`,
`PromotionRequirement::IndependentEvidenceRoute`, and the anti-self-grading SEED
law are authored in F2; this section is the law they will carry.

## 8. Ownership

```text
DEC-077   verification axes, coverage strength, and claim kinds
DEC-078   enforcement posture and runtime conformance lock
DEC-073   verification independence wording (superseded here by explicit basis law)
DEC-075   durable-history and proof-disposition immutability (refined by section 6)
docs/17   retry/oscillation prose, now carried by the NonOscillation claim kind
docs/36   Tier 0 cross-run independence and the two-distinct-witness quorum
spec/verification.rs   F2 home of the typed basis and the layered axes
spec/proof.rs          counts_green seam renamed in F2 for method/coverage strength
spec/tier0_cross_run.rs   the quorum reused by section 6 runtime conformance
```

This document is clean-room operating law. It authors
SEED-LAYERED-DYNAMIC-VERIFICATION, SEED-MODEL-TRACE-SEPARATION, and
SEED-RUNTIME-CONFORMANCE-NO-REWRITE, and is backed by `DEC-077` and `DEC-078`.
It states law, not a new durable behavior obligation, and introduces no `LEG-*`
row.

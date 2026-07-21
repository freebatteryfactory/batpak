# Phase 6 Founding Intake

> **Status:** NON-AUTHORITATIVE review vehicle. This file lives on branch
> `phase6-founding-intake`, forked from the qualified foundation cut. It is not
> part of the authoritative corpus and does not change any frozen law. Its
> reconciled outputs flow into existing owners, gate packets, implementation
> issues, fixtures, proof plans, benchmark corpora, or explicit rejection.
>
> **Foundation cut (Phase 6 opening seed):** `8a92dcacafd6b1013c8131a7fe62c899d2b180e4`
> **Do not edit `cleanroom`.** A row that genuinely requires a new fundamental
> unit / ContractKind / package / changed BatQL semantics / changed receipt
> meaning / changed proof policy / changed authority boundary / new durable or
> wire identity stops as `ArchitectRequired`.
>
> **Naming lock:** the external context-interoperability standard is referred to
> ONLY through the neutral source key `SRC-CTXPROTO`. Its public name, wire
> namespaces, media types, endpoints, and archive digest are HELD for IP counsel
> and MUST NOT appear anywhere in this public repository.

**Planning statuses used in this packet (planning vocabulary only — never added
to the product specification):**
`BoundByExistingLaw` · `GateRequirement` · `ExternalConformanceTarget` ·
`BenchmarkOrWorkloadTarget` · `CandidateDesign` · `DeferredUntilAdopter` ·
`RejectedMechanism` · `ArchitectRequired` · `ClosedByEvidence`

**Completion state of this v0 draft:** §1, §2, §5, §7, §8, §9, §10 authored from
ratified session rulings. §3, §4, §6 are scaffolded and carry an explicit
`[VERIFY]` marker where a cross-referencing pass against the cleanroom is still
required before Gate-0 closure. No `[VERIFY]` row may remain open at Gate 0.

---

## Section 1 — North star and non-negotiable laws

BatPak's admitted law is the genome. MacBat and bounded generators produce
candidate realizations. TestPak is the arena. The dynamic-verification plane is
the governor. External research supplies behavioral targets, conformance
corpora, hostile examples, and benchmark families.

> **Runtime evidence may generate candidates. Independent qualification may
> promote implementations. Only explicit reconciliation may change semantic law.**

- **BatPak is a library of branded semantic primitives.** The event machine and
  embedded database are flagship *compositions* of those primitives, not the
  definition of every primitive.
- **Familiar external surfaces are never authority** merely because agents or
  humans find them convenient.
- **The eight fundamental units stay eight:** Contract, Event, Representation,
  Tile, Turn, Attempt, Receipt, World. No generic wrapper may collapse two of
  them.
- **The one firewall:** foreign material enters as a *claim*; only *admission*
  confers authority. Import, program-authoring, render, effect-ingress, and the
  model-output channel are five *shapes* of that single wall.
- **"Prove" is gladiator-arena, not formal-verification.** The ladder does not
  exist; verification is an orthogonal tuple (basis/method/claim/coverage/
  enforcement/lane/freshness/terminal), never a scalar rank.

Supporting evidence (evidence, not law): the Sprouting & Self-Hosting report
(bounded candidate generation, immutable lineage, hostile qualification,
receipt-backed promotion) and the Control-Theoretic Verification report
(explicit state, temporal properties, bounded exploration, runtime observation,
conformance). The cleanroom owners remain law.

---

## Section 2 — Research-source registry

| Source key | Kind | Authority relationship | Extracted behavior | Forbidden inference | Disposition |
|---|---|---|---|---|---|
| SRC-POCKETBASE | Product research | Competitor | Collection CRUD, realtime, generated surfaces | Its permissive authority model / that these are V1 scope | BoundByExistingLaw + BenchmarkOrWorkloadTarget |
| SRC-DUCKDB | Product research | Competitor | Columnar/vectorized analytics, pushdown, spill | SQL-as-the-soul; column store as source of truth | BenchmarkOrWorkloadTarget (G8) |
| SRC-CODEGRAPH | Product/tool research | Competitor/explanatory | Knowledge/relation images, context capsules, evidence-classed edges | That inferred edges are structural truth | DeferredUntilAdopter (TestPak Repo IR first) |
| SRC-DATABRICKS | Product research | Competitor | Active catalog, CDC, bitemporal, lineage, shadow replay, budgets | That BatPak must become a multimodel DBMS | CandidateDesign / BenchmarkOrWorkloadTarget |
| SRC-TIGERFS | Product research | Competitor (behavioral opponent) | Files as ergonomic surface; paths carry query intent; attributed shared workspaces | PostgreSQL-as-authority; filesystem-as-truth; fs errors as protocol | BoundByExistingLaw + CandidateDesign |
| SRC-OKF | Public standard (Google, Apache-2.0, 2026-06-12) | External standard (conform + namespaced-extend + upstream to Google) | Portable Markdown+frontmatter knowledge corpus | type/link/timestamp/path/citation as authority; body as instruction | ExternalConformanceTarget + CandidateDesign |
| SRC-CTXPROTO | Private external standard (name HELD for counsel) | Your standard (foundry + own the upstream) | Bounded context packet + lifecycle receipts + exact canonical codec | Packet-as-render; carrier-as-authority; a second source-truth/query-lang/receipt | ExternalConformanceTarget + CandidateDesign |
| SRC-CTRL-THEORY | Research report | Explanatory | Layered verification, temporal properties, runtime conformance | External tools as parallel authorities; an assurance ladder | BoundByExistingLaw (docs/38) + CandidateDesign (TracePropertySpec) |
| SRC-SPROUTING | Research report | Explanatory | Bounded candidate space, immutable lineage, hostile qualification, promotion | That the generator may grade itself; that generation changes law | BoundByExistingLaw (TestPak/campaign) |
| SRC-ISSUES | Live GitHub issue corpus | Internal | 45 open issues (see `phase6/open-issues.json`) | That "mentioned in docs" closes an issue | see §6 |

Exact archive/commit digests for SRC-CTXPROTO are recorded **privately**, not in
this public repository.

---

## Section 3 — Existing-law cross-reference `[VERIFY]`

Purpose: prove we are not inventing a second architecture. Each row asserts an
existing owner; `[VERIFY]` rows must be confirmed against the named cleanroom
sources before Gate 0.

| Research idea | Current BatPak owner | Status |
|---|---|---|
| Source history + disposable views | `.fbat`, projection, tile, authority-role law (docs/05) | BoundByExistingLaw `[VERIFY]` |
| Candidate nursery + promotion | TestPak / campaign receipts (docs/12, docs/39) | BoundByExistingLaw |
| Independent arena | TestPak, Gauntlet, docs/38 | BoundByExistingLaw |
| Layered verification / no ladder | docs/38 (DEC-077/078) | BoundByExistingLaw |
| Filesystem read surface | Projection + BatQL + Render | Composition intent |
| Filesystem mutation surface | OperationEffect + admission + EffectBatch (LEG-037) | Composition intent |
| Foreign artifact import/export | Representation + SchemaCodec + ArtifactRef | Candidate profile |
| Exact context packet codec | SchemaCodec + external conformance profile | Candidate profile |
| OKF tree import/export | ArtifactRef + candidate artifact-tree composition | Candidate profile |
| Temporal trace properties | Proof + verification + TestPak (docs/38, spec/proof) | CandidateDesign (G3) |
| Graph / context capsule | TestPak Repo IR + projections (#211) | DeferredUntilAdopter |
| Column analytics | Tile + ScanKernel + G8 | BenchmarkOrWorkloadTarget |

`[VERIFY]` sources for the full pass: docs/05 (storage/fbat/tiles), docs/12
(TestPak), docs/38 (verification), docs/39 (sprouting/promotion), docs/34 (LEG
coverage), docs/30 (decision & rejection ledger).

---

## Section 4 — Format & protocol inventory `[VERIFY]`

For every format: semantic owner · version identity · canonical bytes ·
authority role · producer · consumer · compatibility law · proof owner ·
embedding/reference rules · corruption rebuilds-or-refuses. This scaffold lists
the surfaces; the per-column detail (esp. exact version identities) is
`[VERIFY]` against the cleanroom.

| Format / surface | Authority role | Rebuild or refuse | Notes |
|---|---|---|---|
| BatQL source | authoring input | n/a | frozen 1.0 conceptual grammar `[VERIFY]` |
| Typed BatQL tree | compiler intermediate | n/a | the frozen thing, not concrete syntax |
| ProgramImage | canonical executable authority | refuse | `[VERIFY]` version id |
| WorldImage / `.vpak` | immutable deployed world | refuse | defers packed-artifact format until adopter `[VERIFY]` |
| EventFrame | accepted event identity | refuse | `[VERIFY]` v2 |
| BatTaggedRecord | native canonical payload | refuse | distinct from JCS JSON |
| `.fbat` | durable accepted history | refuse | source-truth plane |
| ArtifactRef | content-addressed blob | rebuild(if derived)/refuse(if authority) | |
| Artifact-tree snapshot | CANDIDATE | tbd | §8; OKF first adopter |
| Native receipt families | evidence | refuse | BatPak/SyncBat/Bvisor/NetBat/TestPak — never collapsed |
| NetBat frame | carrier (non-authoritative) | n/a | orthogonal to all four profiles |
| SRC-CTXPROTO packet bytes | ExternalConformanceTarget | codec (exact) | candidate interchange profile; distinct version id |
| OKF tree | ExternalConformanceTarget | codec (exact) | Google standard |
| Candidate manifests / CampaignEvidence / Tier-0 / E7 evidence | engineering evidence | refuse | not product wire |
| Release seal | release identity | refuse | declares supported profile versions once public |

**Load-bearing rule:** distinct version identities exist because these answer
different questions. "Everything has a hash" does not make them one format.

---

## Section 5 — Edge-profile role map

> These are Phase 6 **edge-profile responsibilities over the existing eight
> fundamental units**. They are NOT new fundamental units, packages,
> ContractKinds, or universal framework types. Classify **per artifact role**,
> not per product. Profiles compose (a render's output can feed an interchange
> encode).

- **ArtifactInterchangeProfile** — *bidirectional* foreign canonical bytes/tree ↔
  typed BatPak artifact representation. Byte-exact + conformance-testable. Exact
  export is the *encode half* — **there is no fifth crossing.**
- **ProgramAuthoringProfile** — one-way foreign notation → canonical BatQL input →
  ordinary compiler pipeline → ProgramImage. Cannot mint ProgramImage directly,
  bypass ASK/DO, or skip capability/bounds/source-cut.
- **RenderProfile** — one-way BatPak object → audience-shaped, possibly lossy
  surface. A view, never authority.
- **EffectIngressProfile** — one-way external mutation claim → typed
  OperationEffect candidate → admission → `EffectBatch` (LEG-037, the sole
  publication boundary; no parallel `CommitSet`). Lossy surfaces use an
  adapter-local `SurfaceTxn` staging envelope that proposes ONE EffectBatch.

| System / object | Role |
|---|---|
| OKF input tree | Artifact interchange decode |
| OKF exact output bundle | Artifact interchange encode |
| Generated `index.md` content | Render (then encoded into the interchange bundle) |
| Filesystem path query | Program authoring |
| Filesystem directory listing | Render |
| Filesystem write / rename | Effect ingress |
| SRC-CTXPROTO packet canonical bytes | Artifact interchange codec |
| SRC-CTXPROTO `plain_prompt` / messages | Render |
| SRC-CTXPROTO recall descriptor | External query claim → possibly mapped via program authoring |
| SRC-CTXPROTO capability | External claim, never a grant |
| Model answer requesting an action | Effect-ingress candidate, never authority |
| NetBat transport frame | Carrier, orthogonal to all four |

**One-firewall doctrine (also §6-security):** foreign content enters as a claim;
only admission confers authority. No render, import, model, carrier, or external
report may strengthen authority, capability, freshness, completeness, or proof
posture. A model's `Ignore previous instructions…` body is data; its resulting
output is an untrusted candidate command requiring ordinary admission.

---

## Section 6 — Open-issue disposition ledger `[VERIFY]`

Source of truth for exact text: `phase6/open-issues.json` (45 issues, digests +
timestamps, queried 2026-07-21). Categorical grouping below is confident; the
per-row `clean owner / LEG / DEC / proof target / gate / mechanism disposition /
compatibility disposition / required closure evidence` columns are `[VERIFY]`
against docs/34 (LEG coverage) and docs/30 (decision ledger).

Statuses: `LawAbsorbed` · `MechanismSuperseded` · `ImplementationPending` ·
`BenchmarkTarget` · `CompatibilityPending` · `ExternalDependency` ·
`ClosedByQualifiedSuccessor`.

**Group A — deep semantic laws, largely `LawAbsorbed` (GAUNT-\*):** #175, #177,
#186, #187, #188, #190, #191, #193, #194, #195, #196, #197, #198, #207, #208,
#211, #213, #214, #218 — GAUNT families mapped into cleanroom LEG/DEC rows.
`[VERIFY]` each against docs/34.

**Group B — product implementation, `ImplementationPending` (FAMILY-/MODEL-/core):**
#199, #200, #201, #202, #206, #209, #212, #229, #230, #233 — projections, path
collation, sparse tree, subscription family, atomic batch, paged discovery,
collision-free kinds. Mapped to G-gates in §9.

**Group C — performance/workload, `BenchmarkTarget`:** #176, #185, #203, #215,
#217, #228, #231, #232 — coordination/projection cliffs, workload envelope,
scan cost, buffered framing, active-tail concurrency, blocking client,
captured streams, launcher dispatch.

**Group D — rehomed, `MechanismSuperseded` (law survives, mechanism replaced):**
#180 (error contract generated into owning contract, not a testkit crate), #181
(`open_cooperative` semantics survive, exact name may not), #210 (`Outcome<T>`
superseded, distinctions survive as typed axes), #202 (HostBat is gone; behavior
rehomed to ContractImage/WorldImage/SyncBat/NetBat). `[VERIFY]`.

**Group E — hygiene/CI:** #224, #225 — Windows CI lane coverage, zero-warnings
gate. `ImplementationPending` (G0).

> **Closure rule:** no issue closes because its vocabulary appeared in docs. An
> issue closes only when a qualified successor exists and the closing comment can
> cite exact public API/mechanism, proof rows, compatibility disposition,
> benchmark/hostile evidence, release, and receipt.

---

## Section 7 — Primitive-composition map

Proves the "library of primitives" thesis: major product surfaces compose from
the existing eight units, no new kingdom.

```text
Event journal            Event + Representation + Receipt + Tile + storage port
Temporal query           Contract + ProgramImage + Tile + Receipt
Runtime world            World + Turn + Attempt + capability + Receipt
Context packet impl      Representation + ArtifactRef + Query claim + Budget +
                         capability claim + Receipt + canonical codec + Render
Filesystem surface       Projection + ProgramAuthoring + Render +
                         EffectIngress + EffectBatch
Knowledge / code graph   Artifact snapshot + relation projection + Tile +
                         query program + evidence claims
Self-growing impl        Contract + Candidate + proof target + Receipt + promotion
```

If a proposed product **cannot** be explained through the existing units, that —
and only that — is the moment to ask whether a semantic category is missing, and
to stop as `ArchitectRequired`.

---

## Section 8 — Candidate-design register

Each candidate names first adopter · existing owner · why current vocabulary may
be insufficient · entry gate · reference model · hostile corpus · compatibility
concern · proof target · promotion condition · rejection condition. Summary:

- **Trace-property artifact (`TracePropertySpec`-style)** — G3 TestPak proof
  artifact (NOT a public language). Expresses always / never / eventually-within-
  bound / until / at-most-once / after-X-before-Y / no-transition-without-
  evidence-Z, each with an explicit **ordering axis** (commit / causal / HLC /
  monotonic deadline / bounded transition count) and horizon. Owner: proof +
  verification + TestPak.
- **Artifact-tree snapshot** — G2/G3. Adopters: OKF, repository exports, proof
  bundles, SRC-CTXPROTO artifacts. Open question: composition over `ArtifactRef`
  vs. an independently-versioned tree-manifest wire format — *let the first
  implementation and hostile corpus answer, not prose.* Path/collision/traversal
  law independently specified, **informed by but not inheriting** freeze.py
  (the CL-12 parity lesson forbids sharing the enumerator). Shares a path-segment
  algebra with ProjectionPath but **not** its identity.
- **Declared program-authoring profile** — G4, only after a real non-BatQL
  surface exists. Build the compiler **socket**; do not freeze a public
  `ProgramSourceFrontend` type before an adopter earns it.
- **Model-facing trust-role vocabulary** — G3/G7/G9. Law already exists (foreign
  content ≠ instruction authority; model output ≠ effect authority). Exact role
  spellings unfrozen; needs a real model-context adopter + injection corpus.
- **Projection-status rendering** — generated *view* over existing source-cut /
  completeness / freshness / proof / loss / receipt facts. Do **not** mint a
  second status authority.
- **Native neutral context composition** — first adopter `testpak context`.
  SRC-CTXPROTO exact canonical bytes are a candidate interchange profile; public
  branding is a later profile admission at the boundary, never a rename of
  BatPak's neutral core. Name stays out of BatPak source until counsel.

---

## Section 9 — Gate map (G0–G9)

- **G0** — materialize exact workspace; verify package graph; compile semantic
  profiles; **finish this founding intake**; prove zero unresolved
  `ArchitectRequired`; produce the Gate-0 review packet. No TigerFS/OKF/context/
  graph/analytics features here.
- **G1** — Contract IR + generated surfaces; may prepare schema/codec generation,
  artifact-format declarations, receipt projections, profile metadata. No public
  interop profile without an adopter + conformance corpus.
- **G2** — durable core: `.fbat`, identities, formats, ArtifactRef, projections,
  source cuts, exact receipts, candidate artifact-tree composition. (#199–#202,
  #206, #229, #230, #233.)
- **G3** — arena: TracePropertySpec-style artifact; research/issue facts into
  Repo IR; conformance harnesses; hostile foreign-artifact corpora; SRC-CTXPROTO
  bundle comparison; OKF fixtures; model-context injection cases. **Migrate the
  hand-authored §3/§4/§6 tables into typed TestPak facts + generated views here.**
- **G4** — BatQL: build the internal authoring-profile socket **without** changing
  frozen 1.0 syntax.
- **G5/G6** — World, ports, Bvisor, runtime, retries, filesystem effect ingress,
  attempt-bound evidence, subscription/freshness. (#181, #185, #209, #212, #227.)
- **G7** — NetBat carriage, CLI, optional filesystem/context product surfaces
  after their owners exist. (#215, #228, #231, #232.)
- **G8** — column, graph, search, filesystem-perf, physical-image/serving
  optimizations — each differential against the reference path. (#176, #203,
  #217.)
- **G9** — self-hosting, external conformance profiles, release seal, issue
  closure receipts, public context/profile qualification, repository-as-world.

---

## Section 10 — Stop conditions

Gate 0 **cannot close** while any intake row is:

- unowned;
- ambiguous between two owners;
- missing a gate;
- missing an acceptance condition;
- claiming a new public format without a version identity;
- claiming a new authority crossing without proof;
- using the SRC-CTXPROTO private name in public BatPak source;
- silently changing BatQL grammar;
- creating a second transaction authority beside `EffectBatch`;
- creating a second source-truth plane;
- requiring a new package without satisfying package law;
- carrying an open `[VERIFY]` marker.

If any row genuinely requires a new fundamental unit / ContractKind / package /
changed BatQL semantics / changed receipt meaning / changed proof policy /
changed authority boundary / new durable-or-wire identity not owned by current
law → **stop as `ArchitectRequired`**, amend the real owner, rerun qualification.
Do not smuggle it into implementation.

---

## Row-liveness through implementation

Every gate review packet must include a **"Founding intake rows affected"**
section; each row ends in one of: `Realized` · `Qualified` · `DeferredWithTrigger`
· `RejectedWithEvidence` · `SupersededByOwner` · `ArchitectRequired`. At G3 the
durable facts migrate into TestPak Repo IR so `testpak context` generates the
current state instead of future agents reconstructing it from chat. At G9 no
founding row remains "interesting idea": it is shipped, qualified, deferred,
rejected, or blocked by a named architect finding.

**Model memory and chat summaries are navigation only. Git commits, issues,
owners, proof targets, fixtures, receipts, and gate packets are the durable
record.**

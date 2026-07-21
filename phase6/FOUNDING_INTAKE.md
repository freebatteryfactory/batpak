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

**Completion state.** The §3/§4/§6 cross-referencing pass is **COMPLETE** (three
read-only research agents, 2026-07-21; citations inline). No `[VERIFY]` markers
remain. Residual for architect ratification at Gate 0: of the 45 open issues, 22
carry an explicit issue-number citation in the authoritative legacy ledger and 23
carry **owner-by-inference** (no issue-number citation) — the inferred rows are
marked `(inf)` and low-confidence rows are flagged in §6. §1, §2, §5, §7, §8, §9,
§10 are authored from ratified session rulings.

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
  Tile, Turn, Attempt, Receipt, World (`docs/00_CONSTITUTION.md:31-42`). "No
  identifier or generic wrapper may collapse two units." (`docs/00:44`)
- **The one firewall:** foreign material enters as a *claim*; only *admission*
  confers authority. Import, program-authoring, render, effect-ingress, and the
  model-output channel are five *shapes* of that single wall.
- **"Prove" is gladiator-arena, not formal-verification.** The ladder does not
  exist; verification is an orthogonal tuple (basis/method/claim/coverage/
  enforcement/lane/freshness/terminal), never a scalar rank (`docs/38:25`).

Supporting evidence (evidence, not law): the Sprouting & Self-Hosting report and
the Control-Theoretic Verification report. The cleanroom owners remain law.

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

## Section 3 — Existing-law cross-reference (pass complete)

Verified against `status: AUTHORITATIVE` docs. Corrections from the v0 scaffold
are marked **[fixed]**.

| Research idea | Verified owner + citation | Status |
|---|---|---|
| Source history + disposable views | `.fbat` + Tile: `docs/05:12,253,269`. **[fixed]** Projection is NOT in docs/05 — the disposable-views law is `INV-CONTEXT-VIEWS-DERIVED-FROM-HISTORY` (`docs/34:70`) → LEG-017 (`docs/21:51`, `batpak::tile`) + LEG-026 (`docs/21:60`, `batpak::projection`) | BoundByExistingLaw |
| Candidate nursery + promotion | Nursery `docs/39:54`; promotion law `docs/12:273`, `AuditablePromotionReceipt` `docs/12:305,323`. ("campaign" is a docs/39 term; "promotion" a docs/12 term) | BoundByExistingLaw |
| Independent arena (informal term) | TestPak `docs/12:12-14`; oracle `docs/12:23`; gauntlet `docs/12:31`; `docs/24:1-4`. ("arena" is not a spec term — owners are correct) | BoundByExistingLaw |
| Layered verification / no ladder | `docs/38:25` (DEC-077/078), `docs/38:57` — confirmed exactly | BoundByExistingLaw |
| Filesystem mutation / atomic typed batch | OperationEffect + admission + **EffectBatch, LEG-037** (`docs/08:197`, `docs/21:71` `batpak::effect`, issue #229) — LEG-037 confirmed correct | BoundByExistingLaw (batch) + Composition intent (fs surface) |
| Foreign artifact import/export | Representation `docs/00:36`; SchemaCodec `docs/15:18,48`; ArtifactRef is cross-cutting (`docs/05:100`, `docs/07:267`) — **[fixed]** not defined in docs/15 | CandidateDesign (profile) |
| Temporal trace properties | `spec/proof/` + `spec/verification/` (both exist) + `docs/38` axis/claim law | CandidateDesign (G3) |
| Graph / context capsule | Repo IR `docs/12:62`; LEG-048 (`docs/21:82`, issues #207/#211/#213, `testpak::repo`) | DeferredUntilAdopter |
| Column analytics | `docs/18:95` (Column/SelectionMask/ScanKernel/LateMaterialization/WorkObservation), `docs/18:127`; G8 via DEC-073 (`docs/30:100`) | BenchmarkOrWorkloadTarget |

**Constitutional confirmations:** the eight fundamental units are exactly those
in §1 (`docs/00:31-44`). **`CommitSet` appears nowhere in authoritative spec** —
the only repo occurrence is the negative reference in this intake doc itself;
`EffectBatch` is the sole publication boundary (`docs/08:197`, `docs/02:36`). The
"no parallel CommitSet" ruling holds.

---

## Section 4 — Format & protocol inventory (pass complete)

| Format | Owner | Version identity | Authority role | Corruption |
|---|---|---|---|---|
| BatQL source | `docs/13:1-20` (BP-BATQL-ARCH-1); grammar in `companion/BATQL_LANGUAGE.md` | `BatQlLanguageVersion` (`docs/16:122`); no magic bytes in docs/spec | carrier (source is provenance, not authority — `docs/10:140`) | REFUSE (compile-time; `docs/13:59`) |
| typed BatQL tree / AST | `docs/04:143-153` phase `TypedProgram`; `docs/13:67` | **none** — internal, non-durable, not in version catalog | carrier (transient) | N/A (recompiled, never repaired) |
| ProgramImage | `docs/07:14-21` (BP-PAKVM-ISA-1) | `ProgramImageVersion` (`docs/16:123`); distinct from `PakVmIsaVersion` | source-truth (executable authority) | REFUSE (`docs/24:1292-1302`) |
| WorldImage / `.vpak` | `docs/10:12-25` (BP-WORLD-PORTS-1) | `WorldImageVersion` (`docs/16:124`); `.vpak` magic = G5 constant (`docs/32:65`) | source-truth (`docs/07:35`) | REFUSE (`docs/07:51`; LEG-053) |
| EventFrame | `docs/05:52-77` (`EventFrameV2`) | `FrameVersion` (`docs/16:128`); field IDs = G2 constant (`docs/32:19`) | source-truth (`docs/05:141-156`) | REFUSE (`docs/05:296`; LEG-023) |
| BatTaggedRecord | `docs/15:32-34` (`BatTaggedRecordV1`) | `BatTaggedRecordVersion` (`docs/16:127`); tags = G2 constant (`docs/32:21`) | source-truth (`docs/15:32-34`) | REFUSE (`FutureVersionRefusal`, `docs/15:83`) |
| `.fbat` | `docs/05:12-29` (BP-STORAGE-TILES-1) | `FbatFormatVersion` (`docs/16:126`); magic = G2 constant (`docs/32:20`) | source-truth (`docs/05:14`) | REFUSE (`docs/05:296`; `docs/00:88`) |
| ArtifactRef | `docs/10:236-240`; `docs/05:100`, `docs/07:265-267` | **none dedicated** — bound to `ContentDigest`/`Commitment` only | evidence/carrier (referenced artifact's own role varies) | REFUSE at mint (`docs/24:1056-1063`) |
| native receipt families | `docs/14:18-28` (BP-RECEIPTS-1) | `ReceiptSchemaVersion` (`docs/16:131`); `BATPAK-TIER0-QUALIFICATION/2` is a separate bootstrap format, explicitly NOT a substitute (`docs/14:16`) | evidence (`docs/14:13`) | REFUSE (canonicalized before hashing, `docs/14:236`) |
| — five families | `docs/14:20-26`: BatPak receipt, SyncBat receipt, **Bvisor _report_**, NetBat receipt, TestPak receipt | — distinct; "do not collapse into one universal generic receipt" (`docs/14:28`) | — | — |
| NetBat frame | `docs/11:1-14` (BP-NETBAT-1) | `NetBatProtocolVersion` (`docs/16:129`); envelope names `entrypoint.invoke.v1`, `query-program.execute.v1` (`docs/11:47-50`); wire bytes deferred (`docs/11:52`) | carrier ("thin by design", `docs/11:14-28`) | REFUSE (`docs/11:83,87`) |
| release seal | `docs/36:195-277` (DEC-058); `spec/release/types.rs` | **none dedicated** — 18 mandatory fields, no `ReleaseSealVersion` | evidence/mutable-authority | REFUSE (`docs/36:269-277`, `docs/25:87`) |

**Notes.** The only concrete magic-string values frozen anywhere are
`BATPAK-TIER0-QUALIFICATION/2` (`docs/14:16`, bootstrap-only) and
`BATPAK-CANDIDATE-MANIFEST/1|2` (`spec/identities/types.rs:102-114`) — neither on
the product-format list. Exact bytes for `.fbat`/`.vpak`/`FrameVersion`/
`BatTaggedRecordVersion` are deferred as G2/G5 implementation constants
(`docs/32:16-27,62-71`). **Load-bearing rule:** distinct version identities exist
because these answer different questions. "Everything has a hash" does not make
them one format (`docs/16:139-143` — a transport version never upgrades the ISA
or image).

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

**One-firewall doctrine:** foreign content enters as a claim; only admission
confers authority. No render, import, model, carrier, or external report may
strengthen authority, capability, freshness, completeness, or proof posture. A
model's `Ignore previous instructions…` body is data; its resulting output is an
untrusted candidate command requiring ordinary admission.

---

## Section 6 — Open-issue disposition ledger (pass complete)

Source of exact text/digests: `phase6/open-issues.json` (45 issues, queried
2026-07-21). Issue-number citations live in `docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md`
(LEG rows) and `docs/30` (DEC-023). **22 issues** carry an explicit issue-number
citation; **23** are owner-by-inference, marked `(inf)`. Statuses: `LawAbsorbed`
· `MechanismSuperseded` · `ImplementationPending` · `BenchmarkTarget` ·
`CompatibilityPending` · `ExternalDependency`.

| # | LEG/DEC (cited unless `(inf)`) | Owner | Disposition | Closure evidence |
|---|---|---|---|---|
| 175 | `(inf)` DEC-067/068 (`docs/30:94-95`) | docs/23/36 | ImplementationPending | G3 AST gate surfaces unsafe/cast/layout as ledgered PointerProvenance/UnsafeMemoryContract |
| 176 | `(inf)` LEG-020, DEC-038 | docs/24 / testpak::bench | BenchmarkTarget | Work/alloc receipt vs DEC-038 threshold |
| 177 | `(inf)` LEG-009/016 | docs/05 | LawAbsorbed | SimFs dir-entry crash matrix vs LEG-016 |
| 179 | **none anywhere (weakest)** `(inf)` LEG-016 | docs/05 | CompatibilityPending | DEC-056 baseline to publish StoreFs corpus + SimFs |
| 180 | `(inf)` LEG-047 | docs/06 | MechanismSuperseded | Generated StoreError→HandlingClass, published |
| 181 | `(inf)` LEG-035 | docs/08 | MechanismSuperseded | Cooperative/threaded parity + stable name via DEC-056 |
| 184 | **LEG-086** (`docs/21:120`) | docs/05 / projection | LawAbsorbed | `matched_kind_decode_failure_is_a_typed_terminal`; replay-agree witnesses |
| 185 | `(inf)` DEC-038, LEG-034 | docs/24 | BenchmarkTarget | Sustained-load/backpressure/recovery-tail receipt |
| 186 | `(inf)` DEC-049, LEG-042 | docs/09 / docs/14 | LawAbsorbed | Runtime receipt binds host composition + op identity |
| 187 | **LEG-042** (`docs/21:76`) | docs/09 Bvisor | LawAbsorbed | stale-report cross-attempt rejection witness |
| 188 | `(inf)` LEG-007 | docs/05 | CompatibilityPending | LEG-007 reopen/compaction/export-restore + DEC-056 |
| 190 | `(inf, strong)` DEC-052, LEG-023 | docs/35 | LawAbsorbed | DEC-052 six-type conformance + LEG-023 witnesses |
| 191 | `(inf, strong)` LEG-023 | docs/05 | LawAbsorbed | LEG-023 reorder/forged-index witnesses (INV-HASH-CHAIN) |
| 193 | **LEG-085** (`docs/21:119`) | docs/05 | LawAbsorbed | signed-compaction preserve/commit witnesses |
| 194 | **LEG-081** (`docs/21:115`) | docs/35 | LawAbsorbed | shred-ack durability, stale-keyset-restore-rejected |
| 195 | **LEG-081** (`docs/21:115`) | docs/35 | LawAbsorbed | shred-binding-mismatch, exports-exclude-raw-keys |
| 196 | `(inf)` DEC-071 | docs/19 / docs/14 | LawAbsorbed | Cross-subsystem receipt carrying DEC-071 witness matrix |
| 197 | **LEG-049** (`docs/21:83`) | docs/24 Gauntlet | LawAbsorbed | planted self-blessing/omission witness |
| 198 | `(inf)` LEG-023 + DEC-071/052 | docs/35 | LawAbsorbed | event-bytes→external-witness chain |
| 199 | **LEG-026** (`docs/21:60`) | docs/05 / docs/18 | ImplementationPending | coordinate-bearing replay/source-stamp witness |
| 200 | **LEG-027** (`docs/21:61`) | docs/16 | ImplementationPending + CompatibilityPending | comparator/encoder witness + frozen-identity compat |
| 201 | **LEG-028** (`docs/21:62`) | docs/05 / docs/18 | ImplementationPending | page-limit-bounds-discovery witness |
| 202 | **LEG-029** (`docs/21:63`) | docs/05/08/10/11 | ImplementationPending + MechanismSuperseded (HostBat gone) | cross-layer equivalence oracle witness |
| 203 | LEG-022 range only (`docs/21:56`) `(inf)` | docs/16 / docs/24 | BenchmarkTarget + ImplementationPending | work-count receipt: project() drops O(scope) |
| 206 | LEG-022 range only (`docs/21:56`) | docs/05/16 | ImplementationPending | cursor-transplant + ordering-oracle witness |
| 207 | **LEG-048** (`docs/21:82`) | docs/12 TestPak | LawAbsorbed | duplicate/omitted-fact fixture witness |
| 208 | **LEG-022/025/073** (`docs/21:56,59,107`) | docs/16 Time | LawAbsorbed | independent-algebra / deterministic-sim witness |
| 209 | **LEG-039** (`docs/21:73`) | docs/08/17 | MechanismSuperseded (Flume DEC-021 DEMOTE) | wait/poll/browser parity + Flume-retirement receipt |
| 210 | `(inf, strong)` INV-OUTCOME-FUNCTOR DEMOTE (`docs/34:130`) | docs/04 / docs/14 | MechanismSuperseded | DEC-056 deprecate-without-erasure migration off `Outcome<T>` |
| 211 | **LEG-048** (`docs/21:82`) | docs/12 TestPak | LawAbsorbed | LEG-048 witness |
| 212 | `(inf)` LEG-060 (INV-SUBSCRIPTION-STATE-MACHINE) | docs/08/17 | ImplementationPending | state-machine + disconnect/overrun corpus witness |
| 213 | **LEG-048** (`docs/21:82`) | docs/12 / docs/20 | LawAbsorbed | LEG-048 witness |
| 214 | **LEG-046** (`docs/21:80`) | docs/06 MacBat | LawAbsorbed | snapshots/origins/compile-fail/parity witness |
| 215 | `(inf)` LEG-045 (INV-NETBAT-BOUNDARY-THIN) | docs/11 | BenchmarkTarget | byte-at-a-time vs buffered differential over real TCP |
| 217 | `(inf)` LEG-017/018 | docs/05 Tile | BenchmarkTarget | concurrency differential vs DEC-033 ColumnTile |
| 218 | **LEG-043 + DEC-023** (`docs/21:77`, `docs/30:50`) | docs/09 Bvisor | LawAbsorbed (postcondition); DEC-023 DEFER (native-exec scope) | descriptor-postcondition + fcntl fail-closed witnesses |
| 224 | `(inf)` DEC-057 | docs/36 | ImplementationPending | TestPak lane-policy fix: Windows CI runs family-crate nextest |
| 225 | `(inf)` DEC-067/068 (INV-ZERO-WARNINGS) | docs/23/36 | ImplementationPending | AST gate names zero-warnings as a discoverable gate |
| 227 | `(inf)` LEG-074 (INV-IMPORT-CRASH-IDEMPOTENT); **flagged: also missing from §6 v0 groups** | docs/05 | LawAbsorbed (regression vs preserved law) | `close_reopen_reimport_returns_zero_new_events` re-proven |
| 228 | `(inf)` LEG-045 | docs/11 | BenchmarkTarget / ImplementationPending | bounded-decode + blocking-call witness |
| 229 | **LEG-037** (`docs/21:71`) | docs/08 effect | ImplementationPending | single/batch equivalence + failure-atomicity witness |
| 230 | `(inf)` LEG-028/029 | docs/05/18 | ImplementationPending | paged-evidence witness bounding discovery work |
| 231 | **DEC-023** (`docs/30:50`) | docs/09 Bvisor | **ExternalDependency (DEC-023 DEFER)** — corrects v0 "BenchmarkTarget" | DEC-023 reopening checklist + real adopter |
| 232 | **DEC-023** (`docs/30:50`) | docs/09 Bvisor | **ExternalDependency (DEC-023 DEFER)** — corrects v0 "BenchmarkTarget" | DEC-023 launcher-descriptor checklist + real adopter |
| 233 | `(inf)` LEG-084 | docs/05/09 | ImplementationPending | reserved-class rejection fixture → collision-free kinds |

**Flagged low-confidence (need architect ratification against the issue body at
Gate 0):** #179 (no citation anywhere), #227 (no citation; also absent from the
v0 grouping), #203/#206 (only inside the LEG-022 *range*, not singled out),
#190/#210 (strong textual inference, not citation). **Correction logged:**
#231/#232 move from the v0 "BenchmarkTarget" grouping to `ExternalDependency`
(DEC-023 classifies this territory as DEFER pending a real external adopter).

> **Closure rule:** no issue closes because its vocabulary appeared in docs. An
> issue closes only when a qualified successor exists and the closing comment can
> cite exact public API/mechanism, proof rows, compatibility disposition,
> benchmark/hostile evidence, release, and receipt.

---

## Section 7 — Primitive-composition map

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
  monotonic deadline / bounded transition count) and horizon. Owner: `spec/proof`
  + `spec/verification` + TestPak + docs/38.
- **Artifact-tree snapshot** — G2/G3. Adopters: OKF, repository exports, proof
  bundles, SRC-CTXPROTO artifacts. Open question: composition over `ArtifactRef`
  (which has **no dedicated version identity** — §4) vs. an independently-versioned
  tree-manifest wire format — *let the first implementation and hostile corpus
  answer, not prose.* Path/collision/traversal law independently specified,
  **informed by but not inheriting** freeze.py (the CL-12 parity lesson forbids
  sharing the enumerator). Shares a path-segment algebra with ProjectionPath but
  **not** its identity.
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
- **G1** — Contract IR + generated surfaces (#214). May prepare schema/codec
  generation, artifact-format declarations, receipt projections, profile
  metadata. No public interop profile without an adopter + conformance corpus.
- **G2** — durable core: `.fbat`, identities, formats, ArtifactRef, projections,
  source cuts, exact receipts, candidate artifact-tree composition. (#184, #188,
  #191, #193, #199–#202, #206, #227, #229, #230, #233.)
- **G3** — arena: TracePropertySpec-style artifact; research/issue facts into
  Repo IR (#207/#211/#213); conformance harnesses; hostile foreign-artifact
  corpora; SRC-CTXPROTO bundle comparison; OKF fixtures; model-context injection
  cases; #175/#197/#225. **Migrate the §3/§4/§6 tables into typed TestPak facts +
  generated views here.**
- **G4** — BatQL: build the internal authoring-profile socket **without** changing
  frozen 1.0 syntax.
- **G5/G6** — World, ports, Bvisor, runtime, retries, filesystem effect ingress,
  attempt-bound evidence, subscription/freshness. (#181, #185, #186, #187, #209,
  #212, #218.)
- **G7** — NetBat carriage, CLI, optional filesystem/context product surfaces
  after their owners exist. (#215, #228.)
- **G8** — column, graph, search, filesystem-perf, physical-image/serving
  optimizations — each differential against the reference path. (#176, #203,
  #217.)
- **G9** — self-hosting, external conformance profiles, release seal, issue
  closure receipts, public context/profile qualification, repository-as-world.
- **DEC-023 DEFER (ExternalDependency):** #231, #232 (and the native-exec scope of
  #218) stay parked until a real external adopter reopens the DEC-023 checklist.

---

## Section 10 — Stop conditions

Gate 0 **cannot close** while any intake row is:

- unowned;
- ambiguous between two owners;
- missing a gate;
- missing an acceptance condition;
- an `(inf)` owner-by-inference row not yet ratified by the architect against the
  issue body (esp. the flagged #179, #227, #203, #206, #190, #210);
- claiming a new public format without a version identity;
- claiming a new authority crossing without proof;
- using the SRC-CTXPROTO private name in public BatPak source;
- silently changing BatQL grammar;
- creating a second transaction authority beside `EffectBatch`;
- creating a second source-truth plane;
- requiring a new package without satisfying package law.

If any row genuinely requires a new fundamental unit / ContractKind / package /
changed BatQL semantics / changed receipt meaning / changed proof policy /
changed authority boundary / new durable-or-wire identity not owned by current
law → **stop as `ArchitectRequired`**, amend the real owner, rerun qualification.
Do not smuggle it into implementation. (No `ArchitectRequired` row surfaced in
this v0 pass; the eight-unit genome expressed every surveyed surface.)

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

# Roadmap — 0.9.0 → 1.0.0

This is a **living document**. It is re-audited every time a line item lands:
knock an item, re-read its section, promote/demote what the change revealed.
Items are owner-gated — nothing here merges on its own schedule. Version
strategy: 0.9.x is the pre-1.0 breaking-change budget; a 0.10.0 window is
reserved for API-shape changes driven by dogfood feedback before the surface
freezes at 1.0.0.

Provenance: the coupling items came from a three-agent audit (2026-07-02) of
the repo against Nygard's coupling taxonomy (operational / developmental /
semantic / functional / incidental) and the coupled-vs-feedback grid. The
posture verdict: batpak sits **deliberately** in the coupled + fast-feedback
quadrant (lockstep monorepo family + the gauntlet). That is the correct
posture for a solo pre-1.0 project and is kept until the exit conditions in
§4 are met. ~85% of enumerated tooling surfaces carry two-way anti-rot
guards; the items below are the measured remainder.

---

## 0. Now (first after the current heavy-validation pass)

- [ ] **Evidence-of-truncation sharpening (untrusted-recovery)** — lands with
  the fail-closed feature (recovery_manifest.rs). Today the ONLY positive
  loss signal is the SIDX manifest: case (a) → `FailClosedCorroboratedLoss`.
  When the manifest is forged / absent / unanchored (case (c)), recovery falls
  to policy on ABSENCE of proof alone. Add a second, footer-INDEPENDENT
  evidence channel: surface from the CRC-valid frame walk whether trailing
  un-parseable bytes exist past the recovery stop `P` (`file_len > P` = a torn
  in-progress write), and (i) refuse on that positive evidence in the
  strict/sealed path even with NO manifest — distinct from the generic
  `FailClosedUnprovableTail`; (ii) record it on the permissive tail path
  (`RecoverPrefix`) in the recovery receipt — "N torn bytes discarded at P" —
  instead of silently recovering a short prefix. Splits "unprovable tail"
  (couldn't verify) from "evidence of truncation" (detected an interrupted
  write): different operational facts, first-class receipt evidence. NOT
  optional — an owner-flagged gap (2026-07-03), not a maybe.
- [ ] **`StoreError` contract mirror gap** — 9 variants (all 0.9.0-era) are in
  neither `testkit::store_error_contract::classify()` nor
  `one_of_every_variant()`: `ProjectionStateContractUnspecified`,
  `ProjectionStateExtentUnavailable`, `ProjectionStateBoundExceeded`,
  `ChainVerificationFailed`, `KeysetCorrupt`, `PayloadSealFailed`,
  `PayloadShredded`, `PayloadDecryptFailed`, `ShredSelectorMismatch`.
  The panicking wildcard only fires on constructed variants, so CI is green
  with 9 uncontracted errors. Fix = classify all 9 + constructor coverage +
  a completeness guard so the next variant cannot ship unclassified.

## 1. Coupling debt — derive, don't enumerate

The in-repo precedent is `e2fa2a86` (production roots derived from cargo
metadata after the launcher-outside-the-size-gate incident): when a hand list
can be derived from source, derive it and delete both the list and its guard.

- [ ] **Seam single-source**: the 22 mutation-seam slugs live in 4 copies
  (`ci.yml` matrix, `lanes.rs`, `ci_parity.rs` literal list + its test copy)
  plus the 41-pair glob mirror in `assurance.rs` and `seam_registry.yaml`.
  Pick one source (seam_registry.yaml or lanes.rs), derive/generate the rest.
  One seam edit should touch one file.
- [ ] **AST-anchor the mutation exclusions**: 7 line-band/exact-line regex
  clusters in `lanes.rs` (mirrored byte-exact in the exclusion registry) have
  **cloud-only failure feedback** — code motion silently deadens them.
  `MUTATION_EXCLUSION_ANCHORS` (xtask ast-grep) already AST-asserts 6 of 26
  exclusions; extend to all, drop the line bands. Prerequisite: **promote
  `cargo xtask ast-grep` into `ci_fast`** (today it runs only via `just
  inspect`).
- [ ] **Class-C silent lists** (enumerated, no drift-guard — the #110 class):
  - GATES-completeness meta-check (a check wired into `main.rs` but absent
    from `GATES` is invisible to registry law; also assert
    `RECEIPT_REQUIRED_GATES ⊆ GATES`).
  - `CI_FAST_GATE_MARKERS` (meta_gate) vs `CI_FAST_REQUIRED_GATE_MARKERS`
    (ci_parity): divergent today; the missing lockstep test is confessed
    in-comment at meta_gate.rs:929. Write it.
  - `TESTED_CRATES` (invariant_bridge), the anchors path-prefix list, the
    dual `BACKENDS` lists (capability_snapshot vs qualification matrix),
    `CHAOS_ENTRYPOINTS`, `PERF_TEST_STEMS`, store_pub_fn_coverage ALLOWLIST,
    and the exclusion-regex extraction keyed on const *names* — each needs a
    derivation or a cross-check.
- [ ] **Delete `traceability/product_doctrine_audit.yaml`** — zero consumers
  (grep-verified). Pure rot.
- [ ] **Derive `fitness_functions.yaml`** from `default_oracles()` (today a
  verbatim hand mirror).
- [ ] **`=1.3.1` pin fan-out**: the rmp-serde/rmpv (and tempfile-band) pins
  span 6 manifests coordinated by comments only. Add a version-pin lockstep
  check (check-version-pins already exists for family versions — extend it).
- [ ] **macros→syncbat textual contract**: `batpak-macros` emits `::syncbat::`
  paths with no cargo edge — a syncbat rename breaks consumers silently at
  expansion. Gate it (expansion-compile test in a crate that has both deps).

## 2. Semantic hardening (small, sharp)

- [ ] `#[non_exhaustive]` on `TypedDecodeError` and `ReceiptVerificationError`
  (adding a variant is currently a semver break by construction).
- [ ] Public-api baselines for the 3 unguarded published crates
  (`batpak-macros-support`, `batpak-macros`, `batpak-bench-support`).
- [ ] syncbat exposes batpak types (`EventId`, `StoreError`, `Store<Open>`)
  without re-exporting batpak — decide: `pub use batpak;` or a doc'd
  version-lockstep guarantee. (Version-skew hazard for consumers.)
- [ ] One `hex_lower` helper (five independent hex encoders today) and one
  domain-separated-digest helper (syncbat hand-rolls six `blake3::Hasher`
  blocks that copy the pattern without citing it).
- [ ] **Family-wide examples**: `crates/batpak-examples` (the load-bearing
  API-usability canary) covers only core. Add syncbat/netbat/hostbat
  examples so the compile-gate watches the crates whose APIs churn most.
- [ ] Test polish: convert the 4 Display/Debug string-assertion kill tests to
  variant/code assertions; declare the sim mode-label strings FROZEN or stop
  pinning them; add a live Store append→get round-trip through the V1→V2
  upcast chain (today proven only at the decode seam against a golden).

## 3. 1.0.0 track — API stabilization & ergonomics

**Principle: keep every feature and guarantee; simplify the surface, not the
substance.** The API should *extend the user's existing mental model* (std,
serde, sqlite-like open/read/write lifecycles) rather than force ours — while
staying fully opinionated where the thesis lives: mechanisms-not-meanings,
fail-closed defaults, receipts as first-class evidence, sync-core/no-runtime.
Opinions stay in the semantics; ergonomics carry them.

- [ ] **API ergonomics audit** (#62, multi-agent, read-only): for each public
  surface measure (a) concepts-required-at-first-contact — how many types
  must a newcomer name to append one event and read it back; (b) naming
  against Rust idiom (std/serde vocabulary where meanings match); (c) builder
  defaults — is the safe path the short path everywhere (the 0.9.0 default
  flips did this for policy; do it for construction); (d) error ergonomics
  (variant granularity vs matchability); (e) every public item has a doc
  example that compiles.
- [ ] **Progressive disclosure**: a minimal entry path (prelude or top-level
  constructors) that needs the fewest concepts, with the full
  capability/policy surface reachable but not mandatory. No functionality
  removed — layered exposure.
- [ ] **Dogfood feedback loop (the ground truth)**: the owner's own builds
  (bvisor/hostbat are the first real consumers) generate friction notes;
  each friction note becomes a line item here. Live feedback shapes the
  surface — this is deliberately *not* designed in a vacuum.
- [ ] **0.10.0 breaking window (reserved)**: API-shape changes discovered by
  dogfooding land in one coordinated pre-1.0 window, not a drip.
- [ ] **1.0 exit criteria**: ergonomics audit findings resolved or explicitly
  waived; `#[non_exhaustive]` sweep complete; baselines green across ALL
  published crates; heavy matrix (24-shard mutation, fuzz, chaos, loom,
  Windows) green on the release candidate; docs current (per-change +
  repo-wide audit); CHANGELOG migration notes for every break since 0.9.0.

## 3b. CI cost policy (durable — the $422 lesson, 2026-07-02)

The **24-shard whole-repo mutation matrix** (`mutants-full`) is the single most
expensive CI job: 16-vCPU × ~5h × 24 ≈ 1,900 vCPU-hours per run. It drove a
two-day Blacksmith bill on its own. Standing rules, enforced by the workflow:

- **It never runs routinely.** `proof=heavy` runs every validation lane
  (verify, windows, chaos, loom, smoke) *except* the matrix. The matrix needs
  a second explicit opt-in: `gh workflow run ci.yml -f proof=heavy -f
  run_full_mutation_matrix=yes`. Default `no` makes casual triggering
  impossible (ci.yml `mutants-full` gate).
- **It is a release-candidate / scheduled-baseline tool**, run at most once per
  cut or on a deliberate `#49` baseline pass — *never* during PR iteration.
- **Per-PR mutation coverage is the diff-scoped smoke lanes** (they grade only
  changed lines — cheap, and what keeps new code at 100%). That is the everyday
  gate; the whale is not.
- **Kill superseded runs immediately.** A new push auto-cancels PR runs;
  dispatch runs must be cancelled by hand — do it reflexively.
- **Required merge gate = CI-fast + smoke lanes** (the cheap tier). Everything
  above it is opt-in signal you budget for.

## 3c. `#49` — first repo-wide mutation baseline (measured 2026-07-02)

The `mutants-full` matrix ran to (near) completion for the first time ever
(21/24 shards before a cost-cancel). This is the baseline measurement, NOT a
merge gate and NOT a regression — diff-scoped coverage on all recent changes is
100%; every survivor is on pre-existing, un-diffed whole-repo code.

**Measured:** all-features shards **84–95%**; no-default shards **72–94%**.
**323 distinct survivor sites, all in `core`.** The provisional 75% floor was
set before any measurement ("PROVISIONAL pending first cloud") — now there is
data. The honest program, cheapest-first (each step shrinks the number before
any curing):

- [ ] **1. Exclude cfg-phantoms** — payload-encryption / dangerous-test-hooks
  items uncatchable on the surface that compiles them out (`encrypted_replay`,
  the `read_api` encrypted read methods, etc.). Registry work we've done a
  dozen times; lifts the below-floor no-default shards with zero curing.
- [ ] **2. Decide test-infra grading** — `fault.rs` injectors + `alloc.rs`
  counting allocator (~34 survivors) are the *tooling*, not the product;
  exempt or separately-track them.
- [ ] **3. Recalibrate the floor** to the measured reality (per-shard or a
  single honest value), replacing the 75% guess. Steps 1–3 turn "2 red shards"
  green *by making the gate honest*, not by papering over.
- [ ] **4. Burn down genuine gaps** (~100–150 after 1–2: `open`, `segment`,
  `sidx/footer`, `signing`, `hidden_ranges`, `index/visibility`; ~70 trivial
  Display/getter fabrications) on a budgeted schedule — never per-PR. Full
  survivor list archived in the session scratchpad.
- [ ] **1 timeout mutant** (`cancel_visibility_fence` `<`→`<=` livelock) needs a
  bounded assertion catch, not tolerance.

## 4. Posture statement (and its exit conditions)

batpak is a lockstep-versioned monorepo family with aggressive fast feedback
(the gauntlet) — **coupled + fast-feedback, on purpose**. Strong coupling to
stable things (frozen wire contracts, the encoding chokepoint, SQL-grade
public surfaces) is accepted; coupling to uncertain things is loosened
(additive serde evolution, versioned formats, FutureVersion refusal).
Exit conditions to revisit the posture — not before: (a) 1.0 ships and
external consumers exist; (b) a crate needs an independent release cadence;
(c) team size > 1 makes lockstep releases a coordination tax. Until then,
independent crate versioning is complexity without a customer.

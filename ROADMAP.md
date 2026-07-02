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

# batpak 0.10.0 Hardening — PLAN & Portable Context

> Purpose: a durable, compaction-proof context window. It carries the decisions, doctrine,
> tacit constraints, and the checkbox work-plan so nothing is lost if the conversation compacts.
> Companion: `HARDENING_AUDIT.md` (10-agent consolidated findings, file:line evidence on both sides).

## STATUS SNAPSHOT
- **Release posture: 0.10.0 tag is PAUSED.** Owner decision: **everything blocks the tag** — every
  audit finding must reach a resolution state before cut (NOT unbounded new scope): each is
  COMPLETE / CONSOLIDATE / REMOVE-DEBT / KEEP-BY-DESIGN / OWNER-CUT-resolved.
- **Branches:**
  - `origin/main` @ `0b599273` — #172 merged (GH runners + #171 diagnostics-off-host).
  - **#173 `fix/wasm-doc-link`** @ `ee472acf` — PARKED. Holds: wasm doc-gate fix (`3fd2fe1a`),
    doc corrections (`929cd923`: bvisor in battery map, ROADMAP retitle, DOCS/README/core-README),
    shipped-README fixes (`788e34a4`: `receipt.global_sequence`, hostbat `let mut host`),
    root-README fix (`ee472acf`: front-door `receipt.global_sequence`). Blocks tag; not published.
  - **local `main` is 1 commit ahead of `origin/main`** (`3fd2fe1a`, also on #173) — stranded-commit
    artifact. Reconcile after #173 merges: `git checkout main && git reset --hard origin/main`
    (needs owner OK per branch rule).
  - **WP1 branch: `fix/unsafe-ledger-unsafe-fn`** (current), based on `3fd2fe1a`.
- **Dry-run (release.yml) last failed on DISK EXHAUSTION** ("No space left on device" compiling
  benches) on free `ubuntu-latest` — infra, not code. At cut time: trim what the dry-run compiles,
  or use a bigger-disk runner.
- **Publish token** (given as 24h burn) will lapse — mint a fresh crates.io token when ready.

## DOCTRINE (the spells — do not lose)
- **NON-NERF:** never resolve a spec↔code gap by weakening the spec. The implementation owes the
  spec, not vice versa. Resolution tags:
  - **COMPLETE** — build to the truthful end state the claim implies.
  - **CONSOLIDATE** — one source of truth, derive/gate the rest. No capability lost.
  - **REMOVE-DEBT** — delete legacy accommodation / dead code. Current capability unchanged.
  - **OWNER-CUT** — might reduce real capability → owner decides. Never auto-deleted.
  - **KEEP-BY-DESIGN** — examined + correct; stop re-litigating.
- **Release doctrine ladder:** don't trust docs over code; code over manifests; manifests over
  gates; **a green gate without a red fixture.** No 0.10.0 tag until gates can prove they go red.
- **Banned word "honest"** (nerf-euphemism). Use PROVEN / FAIL-CLOSED / INCOMPLETE / WAIVED /
  FAULT-INJECTED.
- **Factory vocab = VOICE, not a second technical vocabulary.** Engineering nouns are canonical
  (operation not "terminal", event not "cell", crate not "battery", projection not "gauge"). Brand
  stays (`batpak`/`syncbat`/`netbat`/`hostbat`/`bvisor`, "Free Battery Factory") + prose flavor. A
  factory noun may introduce a concept once but cannot be the ONLY technical name for an API /
  invariant / op / proof surface. Every technical claim names the code noun.
- **Potato constraints:** box OOMs on heavy compiles; NEVER run mutants/fuzz/coverage/full-suite
  locally. Cheap-verify only: `cargo check` / targeted `cargo test`, `-j1`/`-j2`,
  `CARGO_INCREMENTAL=0`. Heavy validation → free GH runners (cloud).
- **Git rules:** NEVER stash/switch/reset/rebase/force/clean or move work between branches without
  explicit per-op OK; surface git-state problems and WAIT. PRs required (main branch-protected →
  admin merge). Commit with `--no-verify` (nested-cargo pre-commit hook is broken here). Structural
  checks scan TRACKED working-tree content → `git add -A` before running a gate or it passes
  vacuously.
- **CI:** GH-hosted free runners (Blacksmith removed after a $500 bill). Triggers: `pull_request` +
  `workflow_dispatch` only (NO push-to-main). The rustdoc doc-gate (`docs` job) is `if: github.ref
  == 'refs/heads/main'` → never fires on a PR → doc breaks slip (WP4 wires a real PR doc-gate).

## OWNER DECISIONS (locked)
- **refbat: DELIBERATELY CUT (cancer). Do NOT rebuild.** Resolution = REMOVE-DEBT + CONSOLIDATE.
  Live witness = `batpak-examples` + current Store/host/netbat surfaces. Kill `ClientManifest`
  (phantom type). Rule: if a stale refbat claim points to real desired behavior → add it as an
  example/test around CURRENT surfaces; if only to removed architecture → delete/reframe. No
  refbat-shaped ceremony under a new name.
- **Sequencing: EVERYTHING blocks the tag** (every finding resolved; not unbounded new scope).
- **Parked capability: WIRE IT IN, bounded** — each capability gets ONE live consumer path OR one
  canonical example/test witness, not a new ceremony layer. Rationale: 0.x allows breaking changes
  (SemVer `0.y.z`; Cargo `^0.10` = `<0.11`; break freely pre-1.0; sole consumer). 1.0 not until
  stable. Once a version is PUBLISHED it is stone — change shape → cut `0.11.0`.
- **Versioning: LOCKSTEP** via `[workspace.package]`, gate against Cargo metadata.
- **AoSoA64: MAKE LOCALITY REAL** (inline `kind` lanes / contiguous scan); reword away from "SIMD"
  to the real tile-skip mechanism.

## WORK PACKAGES

### WP1 — Prove the gates bite (CROWN — do first; a green gate without a red fixture is superstition)
- [x] **C1** unsafe-ledger detector now visits `unsafe fn` bodies (`visit_item_fn` +
  `visit_impl_item_fn`), not only `unsafe {}` blocks. The 3 surfaced naked-FFI sites
  (`run_child`, `child_fail`, `apply_fd_placements`) are anchored + ledgered (23 sites ↔ 23
  entries; structural-check green). Red fixture `unanchored_unsafe_fn_body_fails_closed` bites
  (would pass-as-green under the old visitor); positive twin passes. Detector-hardening, no new
  waiver. `unsafe_ledger.rs` visitor + fixtures; `unsafe_ledger.yaml` +3 entries. DONE.
- [ ] **C2/LD1** family version-drift gate `ast_grep_family_version.rs` — match `block_mapping_pair`
  (not `plain_scalar`) so the regex can match; wire into CI; sync stale checklists (`syncbat`/
  `netbat` `0.8.2`, `bvisor` `0.9.0` → `0.10.0`). `traceability/public_api/*_semver_checklist.yaml`.
  Fix detector FIRST, then update YAML.
- [ ] **LD2** `crates/core/tests/compat_matrix/support.rs:32` — make `SUPPORTED_{MMAP,CHECKPOINT,
  IDEMP,VISIBILITY}_VERSION` `pub` + derive (stop shadowing the real consts). Restores
  `INV-ONDISK-FORWARD-COMPAT-CANONICAL` for 4 formats (currently passes `6==6` vacuously).
- [ ] **LD4** coverage crate list derived from a package-family oracle — **SEED the oracle here**
  (WP3 grows it). `coverage.rs:162` instruments only `-p batpak/syncbat/netbat`; add `hostbat`+
  `bvisor`. Do NOT lower the floor to compensate if they red.
- [ ] Remaining under-checking gates + a RED FIXTURE each: H1 FS-ratchet allowlist over-broad
  (`platform_boundary.rs:161`); `prove_gates_bite.rs:27` MIN_FIXTURES 5→registry-derived (23 exist);
  `harness_lints.rs:387` `--test T -- filter` skips existence checks; `meta_gate.rs:1132` malformed
  L4 manifest fails open; `release_status.rs:323` expired non-terminal WAIVED passes `--strict`;
  `invariant_bridge.rs:295` name-match waiver excuses witnessless; `store_pub_fn_coverage.rs:149`
  name-only Store-method match; `release_manifest.rs:18` git-error fails open; `msrv_check.rs:77`
  bails on hostbat/bvisor (can never pass).

### WP2 — Functional bugs (after WP1 crown bites)
- [ ] **C3** `store/write/writer/publish.rs:174` `.ok()` swallow → under `payload-encryption` every
  sealed event's envelope is dropped, in-process reactors get nothing, undisclosed. Log + either
  fail-closed or a typed "envelope unavailable (encrypted)" outcome subscribers/docs understand.
- [ ] **H2/H3** subscription failure contract: mint distinct wire codes (not all `cursor_invalid`
  for 9 categories), emit terminal `SUB_ERR` before teardown, increment `runtime_failures`.
  `netbat/transport/stream_tcp.rs:727,856`, `syncbat subscription_runtime/{projection_stream,
  operation_status_stream}.rs`.

### WP3 — Single source of truth
- [ ] **Package-family oracle** (grow from LD4 seed) → publish-chain, coverage, MSRV, public-API,
  release-manifest, SBOM, docs-tables, crate-dir map. Gate `PUBLISH_CRATES == publishable workspace
  members` (dynamic oracle exists at `architecture_ir.rs:225`, unused for this). New publishable
  crate must fail until every downstream surface includes it or records why not.
- [ ] **Lockstep `[workspace.package]` version**.
- [ ] `wire_header` module (7 on-disk formats: `MAGIC_LEN`/`VERSION_RANGE`/`CRC_RANGE`/`HEADER_LEN`
  + read/verify helper). One LE byte-cursor (4× dup, already drifted). One op-name manifest binding
  the ~11 doc restatements + gate added/renamed (not just missing). `seccompiler` pin gate (3×).
  `128`-byte limit const. keyscope scope-discriminants (2×). `FAMILY_CRATES` read-from-const in its
  own gate. crate-name→dir `package_dir()` (5×).

### WP4 — Doc truth + doc-gate
- [ ] **refbat/ClientManifest reframe** → current surfaces + `batpak-examples` witness (owner:
  REMOVE-DEBT/CONSOLIDATE, no rebuild). `07_RECEIPTS`, `09_REPLAY`, `11_INTEGRATION`,
  `12_CONFORMANCE`, `README:89-91`, `CHANGELOG:109`, `hostbat/src/schema.rs` comments. Keep the
  gated op-NAMES; kill the "reference host surfaces X over NETBAT / ClientManifest / `cargo test -p
  netbat` proves the ten ops" framing. Absent wire fields (`next_after_global_sequence`,
  `report_hex`, `receipt.evidence.*.v1`, `receipt.verify {valid,outcome,reason_code}`) → remove.
- [ ] "not yet / future work" doc rot: unsafe_ledger "launcher empty" (15 blocks exist),
  `reactor_typed` MultiKindDispatcher "T6 will add" (implemented), bvisor admission S10/"12-membrane"
  (S10 landed, 6 membranes), hostbat subscription "(not yet implemented)", `repo_ir.rs:344`,
  `index/mod.rs:564` begin_replay, `checkpoint.rs` module doc, bvisor `ids.rs:108` H_A claim.
- [ ] `AGENTS.md:263` Phase4/75% → "reported by `cargo xtask mutants policy`"; `AGENTS.md:296` stale
  wire.rs todo (already cured — remove/re-anchor); `README:226` "103 invariants/150 artifacts" →
  derive-or-gate; `AGENTS.md:254` detector names non-literal.
- [ ] **Wire CI doc-gate** (`cargo doc --workspace --no-deps` + `RUSTDOCFLAGS=-D warnings`) on
  rust-changed PRs + **docs-only escape hatch** (ci-fast fast-path when `rust==false`, guard FAILS
  if any `.rs`/`bpk-lib/**` changed). `ci.yml` — the `docs` job is main-ref-only (dead on PRs).
- [ ] **Three-bucket doc law**: every present-tense technical claim = generated-from-code OR
  gated OR explicitly marked design/history. Lore allowed in `archive/`, poison in command docs.
- [ ] Kill "honest" in design comments (`projection/flow`, `strategy.rs`, xtask `doctor.rs`,
  `mutation_debt.rs`). Fix dangling links (bvisor tests `recovery_oracle.rs`, `segment/id.rs`
  phantom `SegmentId::new`).

### WP5 — Macros
- [ ] Cheap wins: complete `define_entity_id!` (16 From/serde blocks, `id/mod.rs:132,235`);
  `Divergence`/`Violation` byte-identical shells (bvisor admission + `sim/recovery*`); `AllVariants`
  derive (`RequirementKind::ALL` 17-variant + 17-arm tripwire test = 3-way sync).
- [ ] **thiserror-lite error derive** in `batpak-macros` — tiny spec: `#[error("…")]`, `#[source]`,
  `#[from]`, `transparent`, and `#[from]`-opt-out for the ~4 routing conversions (e.g.
  `From<CoordinateError> for StoreError`, `error.rs:728`). Co-locate message with variant. Covers
  ~2000 LOC / 73 error types (`StoreError` Display 634 lines, `source()` 70-variant hand-mirror).
  NOT a macro cathedral. KEEP-EXPLICIT: frozen wire/digest token tables ("NEVER reorder"),
  per-matrix violation logic, error-constructor helpers, closed-set `wire.rs` u128 visitors.

### WP6 — Backcompat REMOVE-DEBT
- [ ] Write the **pre-1.0 support policy FIRST** ("0.10.0 does not guarantee reading stores written
  by pre-X batpak") in release notes + `SECURITY.md`/`SUPPORT.md`. Boundary statement, not a
  migration guide.
- [ ] Remove: SDX2 legacy segment footer (~45 refs, cold-start recovery hot path,
  `segment/sidx.rs:73`); checkpoint v2/v3/v4 decoders (`checkpoint/format.rs:23,181`); netbat legacy
  unversioned `CALL` frame (`transport/frame.rs:120`); projection-cache legacy 16-byte trailer
  (`projection/mod.rs:107`); `HEADER_DEBT_ALLOWLIST`→0 (`harness_lints.rs:43`); API duals
  (`#[deprecated] snapshot()` `lifecycle_api.rs:285`, `receiver()` `delivery/subscription.rs:68`,
  `canonical` alias `lib.rs:246`, fire-and-detach `ReactLoopHandle` `watch_api.rs:8`).
- [ ] KEEP-BY-DESIGN (do NOT touch): Upcast chain + `decode_versioned`, `FROZEN_FIXTURE_DEBT` +
  frozen-decode proofs, `>SUPPORTED→FutureVersion` refusals, `typed_waivers.yaml` (empty), registry
  deprecated STATE value, anti-grandfathering CI lints.

### WP7 — Wire-in parked capability + bloat removal (OWNER: wire-in bounded, break freely in 0.x)
- [ ] bvisor schedule/lowering model (~2700 LOC) → route the live Linux path THROUGH it (drop the
  parallel hand-built `LoweringWireV1`, `backend/linux/plan_build.rs:210`). Cleanest COMPLETE.
- [ ] outcome/saga algebra (~640) → real example/test witness (`batpak-examples`):
  Retry/Pending/Batch/WaitCondition/combine.
- [ ] sim simulator (~890) → wire as fast pre-check/oracle in the DST corpus.
- [ ] registry/reservation/transition batteries (~1400) → promote template consumers into examples.
- [ ] syncbat register catalog + netbat route layer → one example/conformance witness each.
- [ ] AoSoA64 → inline `kind` lanes (make locality real).
- [ ] Bloat REMOVE-DEBT: unused `flume` dep (`batpak-examples`); dead `CATEGORY_MIN`/`MAX`
  (`macros-support:195`); `interner.rs` 4 pub(crate) methods; `StoreFile::is_empty`;
  `ImportCeilingOutcome` write-only fields; hostbat `schema_registry`/`all_finished`/`spawn_with`;
  syncbat `clear_receipt_sink`/`with_title`. CONSOLIDATE: `FanoutList`/`FilteredSubscriberList`
  dedup; `DebtEntry` single-impl; xtask verb aliases; `project_timed` clock reads; stale
  cargo-machete ignore.
- [ ] Deps (tool-only levers): xtask `sha2 = "0.10"` (crypto-common dedup); refresh `deny.toml`
  hashbrown comment + core `getrandom` "unified" comment. INHERENT-KEEP: wasm WASI stack (cap-std
  IS bvisor's confinement mechanism).

### THEN — cut ceremony
- [ ] `just seal` → `release.yml` dry-run on an adequate-disk runner → mint fresh token → publish
  RELEASE_CHAIN (`batpak-macros-support → batpak-macros → batpak-bench-support → batpak → syncbat →
  netbat → hostbat → bvisor`) → tag `0.10.0` → flip `CUT-PUBLISH-TAG-0100` to PROVEN
  (`traceability/releases/0.10.0.yaml`).

## KEY CONVERGENCES (found independently by ≥2 agents / re-verified — highest confidence)
- `ClientManifest` phantom: 4 agents. Version drift: ceremony(LD1)+debt(C2), ✓verified.
  Coverage gap: ceremony(LD4)+debt, ✓verified. C1 unsafe-ledger blind: ✓verified.
  C3 `.ok()` swallow: ✓verified. AoSoA64 SIMD framing: debt+bloat.
- GPT-Pro independently re-verified C1/C2/LD2/LD4/C3 + version drift against GitHub — external
  confirmation the gate-holes are real, not agent hallucination.

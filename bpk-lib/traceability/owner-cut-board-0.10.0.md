# OWNER-CUT Decision Board — 0.10.0

> Evidence-based fate of "parked capability". 5 agents, whole-workspace consumer greps.
> Dispositions: WIRE-IN-NOW · KEEP-EXPERIMENTAL · EXAMPLE-WITNESS-ONLY · REMOVE-DEBT · CUT · KEEP(live).
> "already-live" / "already-witnessed" items were mis-classified as parked by the priors — evidence corrected them.

## Roll-up (recommended disposition per item)

| # | Item | LOC | Live consumer? | Recommended | Note |
|---|---|---|---|---|---|
| 1 | bvisor schedule/lowering model | ~2327 | **NONE** (live path hand-builds parallel `LoweringWireV1`) | **WIRE-IN-NOW** | two-truths; fallback KEEP-EXPERIMENTAL, never CUT. *shadow-proven, not SMT* |
| 2 | bvisor `backend-windows/macos` stub flags | ~240 | none (never constructed; not on Linux CI) | **KEEP-EXPERIMENTAL** | + add compile-smoke gate (they rot silently) |
| 3a | `outcome/` core (`Ok/Err/Cancelled/Retry` + combinators) | — | **LIVE** (`try_submit*` return type; Retry = backpressure) | **KEEP (live)** | brief premise "only ok/cancelled" was WRONG |
| 3b | `outcome/` saga reach (Pending/Batch/WaitCondition/Compensation/join/zip) | ~350 | none (tests/testkit only) | **EXAMPLE-WITNESS-ONLY** | published 0.9.0 API; add a saga demo |
| 4 | `store/sim/` model simulator | ~1297 | 1 test (`sim.rs`) | **KEEP-EXPERIMENTAL** | already `dangerous-test-hooks`+`doc(hidden)`, no public API. `ModelState` re-checks → optional REMOVE-DEBT |
| 5 | registry/reservation/transition batteries | 1416 | none prod; **3 CI-gated templates** | **EXAMPLE-WITNESS-ONLY (already satisfied)** | templates run clippy-D + test; keep |
| 6 | `Canal`/`CanalItem` | 292 | **LIVE** (ProjectionWatcher; **cross-crate: syncbat**) | **KEEP (live)** | GPT REMOVE overturned; optional: narrow to `pub(crate)` |
| 7a | `ProjectionCache::prefetch` | — | **LIVE** (capability-gated, test-proven `flow/mod.rs:471`) | **KEEP (live)** | GPT REMOVE overturned |
| 7b | `ProjectionCache::sync` | — | none (required method Store never calls) | **WIRE-IN-NOW** | make `Store::sync`/`close` call `cache.sync()` (latent trap for durable caches). alt: REMOVE-DEBT |
| 8 | AoSoA64 tiled index | ~200 | LIVE (opt-in `IndexTopology::tiled()`) | **WIRE-IN-NOW** (owner-decided) | 2-file change specified: `kinds: Vec<EventKind>` → `[EventKind; N]` + `kind.rs` UNINIT sentinel |
| 9 | `blake3 = []` feature | — | 0 cfg-sites, 0 in-workspace requesters | **REMOVE-DEBT** | delete + strip from `default` |
| 10 | hostbat `subscription.rs` (declaration layer) | ~392 | none (no listener ever stood up; netbat serving disjoint) | **KEEP-EXPERIMENTAL** | feature-gate the `H_interface` commit + witness |
| 11 | hostbat Supervisor/JobBody/`spawn_job` | ~189 | none (composition_tests only) | **KEEP-EXPERIMENTAL** | + example-witness |
| 12 | hostbat effect-backend/host-control/hooks | ~275 | none (**bvisor bypasses via direct `append_typed`**) | **WIRE-IN-NOW** target | HOLLOW SEAM; but seam is `append_event(kind,payload)` vs bvisor's coordinate-addressed `append_typed→receipt` = granularity gap. Fallback KEEP-EXPERIMENTAL + record honestly |
| 13 | hostbat `schema_registry()`/`all_finished`/`spawn_with` | — | none (dead even in tests) | **REMOVE-DEBT** | fields/behavior stay |
| 14 | syncbat `register_store/` catalog | 836 | none (tests only) | **KEEP-EXPERIMENTAL → WIRE-IN** | complete durable op-register story, unwired. wire `rebuild_register_from_store` into Core boot, OR feature-gate |
| 15a | netbat `route.rs` introspection | 487 | none (metadata-only, parallel to live `dispatch_frame`) | **EXAMPLE-WITNESS-ONLY** | move to examples or wire to future router |
| 15b | netbat `Route::into_module` | — | ZERO callers, dead bridge | **REMOVE-DEBT** | straight cut |
| — | hostbat `builder.rs` assembly (CompositeGuard, SchemaValidatingHandler, fingerprints) | — | **LIVE (bvisor)** | **KEEP — load-bearing** | not a cut row (GPT caution confirmed) |

## Shape of the board
- **Genuinely dead / REMOVE-DEBT (safe deletes, PR-3):** blake3 feature (9), hostbat dead accessors (13), netbat `into_module` (15b), optional `sim::ModelState` re-checks (4).
- **Already live — NOT parked (evidence corrected the priors):** outcome-core (3a), Canal (6), prefetch (7a), AoSoA64 reachable (8), batteries CI-witnessed (5).
- **Real WIRE-IN engineering forks (effort vs feature-gate):** bvisor schedule (1), hostbat effect-backend seam (12), syncbat catalog (14), ProjectionCache::sync (7b, cheap), AoSoA64 (8, 2-file).
- **EXAMPLE-WITNESS (add batpak-examples demos):** outcome saga reach (3b), netbat route (15a), hostbat subscription (10) + supervisor (11).
- **KEEP-EXPERIMENTAL (gate/keep, add teeth):** bvisor OS stubs (2, +compile gate), sim (4).

## Nothing here is a CUT
No item's evidence supports a straight CUT (real-capability removal). The board is WIRE-IN / KEEP / WITNESS / REMOVE-DEBT — the anti-nerf discipline held across all 5 agents.

## OWNER DECISION (2026-07-06) — RESOLVED
Owner chose the maximal non-nerf path: wire all four, witness all four, delete all four.

**WIRE-IN-NOW (→ PR-5, live path before 0.10.0):**
- bvisor schedule model — route live `plan_build` through `compile_schedule` via `LoweringWireV1::from_schedule`; kills the two-truths.
- `ProjectionCache::sync` — call `cache.sync()` from `Store::sync`/`Store::close`.
- AoSoA64 — inline `kinds: [EventKind; N]` + `kind.rs` `UNINIT` sentinel (2-file, no public-API change).
- hostbat effect-backend seam — grow a coordinate-carrying seam variant + route bvisor's persist through it (closes the hollow seam). **Caveat: granularity gap** (`append_event(kind,payload)` vs `append_typed→receipt`) — this one is real design work, not mechanical.

**EXAMPLE-WITNESS-ONLY (→ PR-7-adjacent, `batpak-examples` demos):**
- syncbat register catalog (demo the durable op-register: append → restart → rebuild).
- outcome saga reach (suspend-resume saga demo).
- hostbat subscription + supervised-jobs (declare a subscription; declare + spawn a job).
- netbat route introspection (inspect a Core / describe routes without serving).

**REMOVE-DEBT — delete all four (→ PR-3):**
- `blake3 = []` phantom feature (+ strip `"blake3"` from `default`).
- hostbat dead accessors: `schema_registry()`, `Supervisor::all_finished`, `HostBuilder::spawn_with`.
- netbat `Route::into_module` (zero callers).
- `sim::ModelState` safety re-checks (redundant with `recovery_matrix` real-Store proofs).

**KEEP (ratified, no change):** outcome-core, Canal (cross-crate syncbat), `ProjectionCache::prefetch`, batteries (CI-witnessed templates), hostbat `builder.rs` assembly.
**KEEP-EXPERIMENTAL (ratified):** bvisor OS-stub flags **+ add a compile-smoke gate**; `sim/` (already gated).

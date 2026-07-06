# 0.10.0 Hardening — File-Touch Manifest & Batch Plan

> Companion to `hardening-plan-0.10.0.md` (the WP checklist) and `HARDENING_AUDIT.md` (findings).
> Ordered **deletion-first** so we never harden/gate/macro/doc code we then cut.
> ✂ = supersession flag (code another batch deletes/cuts — do the deleter first).
> Paths are workspace-relative (under `bpk-lib/`). "scan" = enumerate exact sites when the batch starts.

## Ordering rule (compounding-refined 2026-07-06)
gates → OWNER-CUT decisions (RESOLVED) → deletions → **SoT infra** → **macros** →
functional bugs → wire-in survivors + witnesses → docs → cut.

Two composing principles:
- **Deletion-first** — don't polish/gate/macro/doc code we're about to delete.
- **Infra-first** — the force-multipliers (package-family oracle, error-derive, `wire_header`)
  land BEFORE the mechanical batches that ride on them, so each later PR is cheaper: new error
  types become free, new crates auto-derive across coverage/MSRV/public-API/release/docs, format
  changes are one-place.

Override: pull functional bugs (C3, H2/H3) up to right after deletions if you want the real
defects fixed sooner — the only compounding cost is H2/H3's new `stream_code` variant being
hand-rolled instead of derived (trivial). Recommendation: keep the oracle first regardless.

---

## PR #174 (OPEN) — WP1 C1/C2/LD4
Done. `tools/integrity/src/unsafe_ledger.rs`, `family_version.rs`, `structural.rs`, `main.rs`,
`architecture_lints/tooling_contract.rs`; `tools/xtask/src/coverage.rs`;
`crates/bvisor/{launcher/linux,src/backend/linux}/sys.rs`; `traceability/unsafe_ledger.yaml`,
`public_api/{bvisor,netbat,syncbat}_semver_checklist.yaml`.

## PR-2 — WP1 remaining gates cluster (all tooling; no product code)
Each: fix the under-check + add a red fixture proving it bites.
- `tools/integrity/src/architecture_lints/platform_boundary.rs` — H1: tighten `ALLOWED_DIRECT_FS_CONTACTS` counts to observed (or 0).
- `tools/xtask/src/commands/prove_gates_bite.rs` — MIN_FIXTURES 5 → registry-derived exact count.
- `tools/integrity/src/harness_lints.rs` — `--test T -- filter` form skips existence checks; fix.
- `tools/integrity/src/meta_gate.rs` — malformed L4 manifest `unwrap_or_default()` fails open.
- `tools/integrity/src/release_status.rs` — expired non-terminal WAIVED passes `--strict`.
- `tools/integrity/src/invariant_bridge.rs` — name-match waiver excuses witnessless citation.
- `tools/integrity/src/store_pub_fn_coverage.rs` — name-only Store-method match (no receiver type).
- `tools/xtask/src/commands/release_manifest.rs` — strict dirty-refusal fails open on git error.
- `tools/xtask/src/commands/msrv_check.rs` — bails on hostbat/bvisor (can never pass) + parity test.

## DECISION GATE — WP7 OWNER-CUT (no files; your call per item)
Each → keep-public / keep-experimental / wire-in / remove-debt / cut. Drives PR-3 vs PR-5.
- bvisor schedule/lowering model (~2700 LOC): `crates/bvisor/src/contract/{primitive,lowering}.rs`,
  `admission/schedule*.rs`
- core `outcome/` saga algebra (~640): `crates/core/src/outcome/{mod,wait,combine}.rs`
- core `store/sim/` model simulator (~890): `crates/core/src/store/sim/{mod,workload,fault_model,invariants,scheduler}.rs`
- `registry`/`reservation`/`transition` batteries (~1400): `crates/core/src/{registry,reservation,transition}.rs`
- hostbat generic-host subsystems: `crates/hostbat/src/{subscription,supervisor,host_control_backend,validating_effect_backend,event_payload_binding}.rs`
- syncbat register catalog: `crates/syncbat/src/register_store/*`
- netbat `route.rs`: `crates/netbat/src/route.rs`
- `Canal` (`store/delivery/canal.rs`), `ProjectionCache` prefetch/sync hooks (`store/projection/mod.rs`)
- AoSoA64 (`store/index/columnar/aosoa.rs`) — you already chose "make locality real" (→ PR-5)
- `blake3 = []` feature alias (`crates/core/Cargo.toml`); bvisor `backend-windows/macos` empty flags

## PR-3 — ALL DELETIONS (WP6 backcompat + WP7 dead-code + LD2)
**Write the pre-1.0 support policy FIRST** in `SECURITY.md` + `SUPPORT.md` + CHANGELOG
("0.10.0 does not guarantee reading stores written by pre-X batpak").

WP6 legacy readers (REMOVE-DEBT):
- ✂ SDX2 footer: `crates/core/src/store/segment/sidx.rs` (magic + footer), `segment/mod.rs`
  (`detect_sidx_boundary` SDX2 arm), `segment/scan/{full_scan,recovery}.rs`, `segment/recovery_manifest.rs`,
  + tests `manifest_recovery_tests.rs`, `segment_scan_hardening_frame_bounds.rs` (scan: ~45 refs)
- ✂ checkpoint v2/v3/v4: `crates/core/src/store/cold_start/checkpoint/format.rs` (structs + match arms)
- ✂ netbat legacy CALL frame: `crates/netbat/src/transport/frame.rs`, golden `tests/golden/request_call_legacy.hex`,
  `traceability/{artifacts,flows}.yaml` entries
- ✂ projection-cache 16-byte trailer: `crates/core/src/store/projection/mod.rs`
- HEADER_DEBT_ALLOWLIST → 0: `tools/integrity/src/harness_lints.rs`
- API duals: `store/lifecycle_api.rs` (`#[deprecated] snapshot()`), `store/delivery/subscription.rs`
  (`receiver()`), `crates/core/src/lib.rs` (`canonical` alias), `store/watch_api.rs` (fire-and-detach `ReactLoopHandle`)

LD2 (make consts pub + derive; PAIRED because it edits checkpoint/format.rs too):
- `store/cold_start/mmap/format.rs` (`MMAP_INDEX_VERSION` pub), `store/cold_start/checkpoint/format.rs`
  (`CHECKPOINT_VERSION` pub), `store/index/idemp.rs` (`IDEMP_VERSION` pub), `store/hidden_ranges.rs`
  (`VISIBILITY_RANGES_VERSION` pub) — likely `pub use` at `store/mod.rs`
- `crates/core/tests/compat_matrix/support.rs` (derive the 4 from the pub consts)
- `traceability/public_api/batpak.txt` (baseline update — the link-heavy bit; bless on CI)

WP7 dead-code (REMOVE-DEBT, confirmed):
- `crates/batpak-examples/Cargo.toml` (drop unused `flume`)
- `crates/macros-support/src/lib.rs` (dead `CATEGORY_MIN`/`CATEGORY_MAX`)
- `store/index/interner.rs` (4 dead `pub(crate)` fns), `store/platform/handle.rs` (`StoreFile::is_empty`)
- `store/sim/import_recovery.rs` (`ImportCeilingOutcome` write-only fields) ✂ *only if sim survives*
- `crates/hostbat/src/{host.rs (schema_registry), supervisor.rs (all_finished), builder.rs (spawn_with)}`
- `crates/syncbat/src/{builder.rs (clear_receipt_sink), operation.rs (with_title)}`

## PR-4 — WP2 functional bugs (independent; publish/subscription all stay)
- `crates/core/src/store/write/writer/publish.rs` — C3: replace `.ok()` swallow (log + fail-closed
  or typed "envelope-unavailable-encrypted" outcome).
- `crates/netbat/src/transport/stream_tcp.rs` — H2/H3: distinct wire codes, emit `SUB_ERR` before
  teardown, increment `runtime_failures`.
- `crates/syncbat/src/subscription_runtime/{projection_stream,operation_status_stream}.rs`,
  `crates/syncbat/src/error.rs` — H2: internal-failure `stream_code` variant.

## PR-5 — WP7 wire-in survivors (only the keeps from the decision gate)
- bvisor schedule model → route the live Linux path through it, delete parallel `LoweringWireV1`:
  `crates/bvisor/src/backend/linux/plan_build.rs` + `contract/{lowering,schedule}.rs`
- AoSoA64 make-locality-real: `store/index/columnar/aosoa.rs` (inline `kinds: [u16; N]`) + `columnar.rs`
- Example witnesses for surviving parked capability: new bins under `crates/batpak-examples/src/bin/`
  (outcome saga, registry/reservation/transition, register catalog, route layer) — one per survivor
- sim → wire as DST-corpus pre-check (if kept): `crates/core/tests/sim.rs` + corpus wiring
- `FanoutList`/`FilteredSubscriberList` dedup: `store/write/fanout.rs` (CONSOLIDATE)

## PR-6 — WP3 source of truth (over survivors)
- Package-family oracle: expand `tools/xtask/src/publish.rs` (one list) → derive in `coverage.rs`,
  `msrv_check.rs`, `public_api.rs`, `release_manifest.rs`, SBOM, docs-table gate; add
  `PUBLISH_CRATES == publishable-members` gate (`architecture_ir.rs` oracle already exists).
- Lockstep version: `crates/*/Cargo.toml` → `[workspace.package] version` inheritance; gate vs metadata.
- `wire_header` module (new, `store/`): `MAGIC_LEN`/`VERSION_RANGE`/`CRC_RANGE`/`HEADER_LEN` + helper;
  adopt in `checkpoint/format.rs`, `idemp.rs`, `keyscope/persist.rs`, `hidden_ranges.rs`,
  `mmap/{format,load}.rs`, `fork_report.rs` (7 formats).
- One LE byte-cursor: `mmap/format.rs`, `segment/sidx.rs` (dedup the 4× drifted macros).
- One op-name manifest: `docs_contract.rs` + bind the 11 doc restatements; gate added/renamed.
- `seccompiler` pin gate, `128`-limit const (hostbat/netbat), keyscope discriminants,
  crate-name→dir `package_dir()` (dedup 5 copies), `FAMILY_CRATES` read-from-const in its gate (B3).

## PR-7 — WP5 macros (over surviving error types)
- New error derive in `crates/macros/src/` (thiserror-lite: `#[error]`/`#[source]`/`#[from]`/opt-out).
- Apply across ~73 error types / ~40 files (scan): headline `store/error.rs` + `store/error/display.rs`
  (`StoreError` 634-line Display + `source()`); `event/decode.rs` (`TypedDecodeError`), `event/upcast.rs`,
  `coordinate/mod.rs`, `syncbat/operation_name.rs`, `bvisor/contract/plan.rs`, … (scan for `impl Display`
  + `impl Error` in `crates/*/src`). KEEP-EXPLICIT: frozen token tables, per-matrix violations,
  error-constructor helpers, `wire.rs` u128 visitors.
- Cheap wins: finish `define_entity_id!` (`store/id/mod.rs`, 16 blocks); `Divergence`/`Violation`
  shells (`bvisor/contract/admission/{shadow,schedule_circuit}.rs`, `core/store/sim/{recovery,recovery_matrix}.rs`);
  `AllVariants` derive (`bvisor/contract/support.rs` `RequirementKind::ALL`).

## PR-8 — WP4 doc truth + CI doc-gate (final state)
- refbat/ClientManifest reframe → current surfaces + `batpak-examples` witness (owner: no rebuild):
  `07_RECEIPTS.md`, `09_REPLAY.md`, `11_INTEGRATION.md`, `12_CONFORMANCE.md`, `README.md`,
  `CHANGELOG.md` (historical note), `crates/hostbat/src/schema.rs` comments. Keep gated op-names.
- "not-yet" doc-rot: `store/reactor_typed.rs`, `bvisor/src/contract/admission/*`,
  `crates/hostbat/src/subscription.rs`, `repo_ir.rs`, `store/index/mod.rs`, `store/cold_start/checkpoint.rs`.
- AGENTS.md hardcodes: `AGENTS.md` (Phase4/75% → "reported by xtask", wire.rs todo, detector names),
  `README.md` (103/150 → derive/gate), `crates/bvisor/src/contract/ids.rs` (H_A claim).
- Wire CI doc-gate + docs-only escape hatch: `.github/workflows/ci.yml` (rustdoc `-D warnings` on
  rust-changed PRs; ci-fast fast-path + guard when `rust==false`).
- Three-bucket doc law (generated/gated/marked-history). Kill "honest": `store/projection/flow`,
  `store/strategy.rs`, `tools/xtask/src/commands/doctor.rs`, `mutation_debt.rs`. Fix dangling links
  (`bvisor/tests/grid.rs` recovery_oracle, `store/segment/id.rs` SegmentId::new).

## Then — cut ceremony
`just seal` → dry-run on adequate-disk runner → mint fresh crates.io token → publish RELEASE_CHAIN →
tag `0.10.0` → flip `CUT-PUBLISH-TAG-0100`.

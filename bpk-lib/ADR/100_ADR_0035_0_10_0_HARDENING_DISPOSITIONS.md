# ADR-0035: 0.10.0 Hardening Dispositions

## Status

Accepted; 0.10.0 pre-1.0 breaking window. Records the WP7 OWNER-CUT board result,
the SIDX universal fail-closed guard, the EffectBackend typed-seam redesign, and
`define_entity_id!` opt-in serde.

## Context

The 0.10.0 sprint ran a parked-capability audit (five agents, whole-workspace
consumer greps) to decide the fate of every subsystem the priors had flagged as
unused. The audit corrected several priors — a number of "parked" surfaces were
already live and load-bearing — and reduced the genuinely-open work to a small,
evidence-backed set. No item's evidence supported a straight capability removal;
the anti-nerf discipline held across all five agents.

Three deeper dispositions landed alongside the board: a universal fail-closed
guard over segment-index footers (generalizing the SDX3 footer of ADR-0032), a
redesign of the hollow hostbat effect-backend seam onto the typed store write
path, and an opt-in serde contract for `define_entity_id!`.

This ADR is the durable record of those decisions; the sprint's scratch planning
notes were transient and are not retained.

## Decision

### 1. OWNER-CUT board — wire four, witness four, delete four

The owner chose the maximal non-nerf path over the feature-gate fallback.

**Wire-in (live path before 0.10.0):**

- **bvisor schedule / lowering** — route the live Linux `plan_build` through
  `compile_schedule` via `LoweringWireV1::from_schedule`, deleting the parallel
  hand-built lowering wire. This kills a two-truths hazard where the validated
  schedule model and the live path could disagree.
- **`ProjectionCache::sync`** — call `cache.sync()` from `Store::sync` and
  `Store::close`. The cache trait required the method but the store never invoked
  it, a latent durability trap for cache backends that buffer.
- **AoSoA64 tiled index** — inline the tiled index's `kinds: [EventKind; N]`
  array with a `kind.rs` `UNINIT` sentinel (a two-file change, no public-API
  change), making the tiled topology's locality real rather than nominal.
- **hostbat effect-backend seam** — see disposition 3 (its own design work).

**Example-witness (one `batpak-examples` demo per survivor):** the syncbat
durable op-register catalog (append → restart → rebuild), the `outcome` saga
suspend/resume reach, hostbat subscription plus a supervised job, and netbat
route introspection.

**Remove-debt (straight deletes):** the `blake3 = []` phantom feature (and its
entry stripped from `default`), the hostbat dead accessors `schema_registry()`,
`Supervisor::all_finished`, and `HostBuilder::spawn_with`, the netbat
`Route::into_module` bridge (zero callers), and the `sim::ModelState` safety
re-checks (redundant with the real-`Store` `recovery_matrix` proofs).

### 2. SIDX universal fail-closed guard

Every unrecognized segment-index footer now routes through
`resolve_untrusted_frames_end`. A footer whose version the running build cannot
fully authenticate surfaces a typed `SidxFutureVersion` refusal instead of
falling through to a best-effort scan, and compaction fail-closes on a partial
recovery rather than compacting a truncated view. This generalizes the SDX3
integrity footer of [ADR-0032](100_ADR_0032_SIDX_SDX3_INTEGRITY_FOOTER.md) into a
universal boundary: any footer the current version cannot verify is refused, not
guessed.

### 3. EffectBackend typed seam redesign

The hostbat effect-backend seam was hollow: bvisor bypassed it entirely by
persisting through `Store::append_typed`, and the seam's `append_event(kind,
payload)` shape could not carry the coordinate-addressed, receipt-returning
granularity that path needs. The seam is regrown around the typed
`Store::append_typed` write path, so persistence is encryption- and
crypto-shred-safe by construction — the typed envelope carries the keyset
coordinate, so a shredded key fails closed on decode instead of exposing
plaintext. This was real design work, not a mechanical rewire: the granularity
gap is exactly why the seam had sat unused. It extends the store-backend seam
lineage of [ADR-0018](100_ADR_0018_STORE_PLATFORM_BACKEND.md) and
[ADR-0034](100_ADR_0034_STORE_FS_HANDLE_ABSTRACTION.md) up into the host layer.

### 4. `define_entity_id!` opt-in serde

`define_entity_id!` makes serde derivation explicit. The two-argument form
(`define_entity_id!(Name, "tag")`) emits no serde implementations; a caller opts
in with the three-argument `serde` form (`define_entity_id!(Name, "tag",
serde)`). A local-only identifier can no longer accidentally acquire a wire
representation, and compile-fail coverage pins the two-argument form to
no-serde.

### 5. Segment-scan recovery contract — fail closed on non-tail corruption

_Addendum, 2026-07-07. Records a release contract already implemented in code and pinned by tests; landed after ADR acceptance during the 0.10.0 end-CI drive._

A segment whose interior contains a corrupted frame followed by any CRC-valid frame, including a valid `SYSTEM_CLOSE_COMPLETED`, must refuse to open. Scan and recovery must not truncate to the valid prefix.

**Invariant.** Corruption is mid-stream, not tail, when a CRC-valid frame still exists after the poisoned position. The scan/recovery path routes undecodable or impossible frames through `resolve_untrusted_frames_end`, making non-tail classification fail closed by construction. Tail corruption remains governed by the existing tail-recovery rules. The `MAX_FRAME_PAYLOAD` boundedness guard is unchanged: pathological lengths stop the walk before allocation rather than driving an unbounded read.

**Rationale.** Recovering the prefix of a segment that has a cleanly written later frame silently truncates committed data and reports it as a successful open. That is a data-loss-as-recovery failure mode. A typed refusal is safer than a quiet partial recovery. This applies the SIDX guard's canonical-refusal posture from disposition 2 to the frame stream.

**Witness.** Commit `4a34a8c0` aligns the three pre-#25 scan tests with this contract, each verified as property-preserving or property-strengthening:

- `corruption_inside_committed_batch_fails_closed` now surfaces `CorruptSegment` with "mid-stream corruption" instead of the narrower `CrcMismatch`; the pinned property remains "never leak the valid prefix."
- `pathological_frame_length_is_bounded_not_panicking` flips recover → fail closed because a valid `SYSTEM_CLOSE_COMPLETED` follows the poison; the old path silently truncated a cleanly closed segment.
- `non_tail_pathological_frame_length_fails_closed_on_reopen` still refuses as `CorruptFrame`; the reason string changed to the uniform region-bounds path, while the `MAX_FRAME_PAYLOAD` cap remains pinned by scan tests.

**Watch item, not a promise.** A recover-with-loud-warning policy toggle may be explored after 0.10.0 only if a real operator use case appears. It is not the default, not a release blocker, and not planned work. Fail-closed is the 0.10.0 release contract.

## Consequences

- The three wire-in forks remove standing two-truths / latent-trap / hollow-seam
  hazards rather than carrying them into 1.0; the deletes remove dead surface the
  gates would otherwise have to keep exempting.
- The SIDX guard makes footer version handling uniformly fail-closed, so a
  forward-incompatible reader refuses with a typed error instead of silently
  degrading — consistent with the store's canonical-refusal posture.
- The typed effect seam ties host-level persistence to the same encryption and
  crypto-shred guarantees the store enforces on `append_typed`; there is one
  write path, not two with divergent safety.
- `define_entity_id!` serde is now a deliberate, reviewable choice at each id
  definition site.
- Segment-scan recovery fails closed on non-tail corruption: a cleanly closed
  segment with a poisoned interior frame refuses to open rather than reporting a
  truncated-prefix recovery as success — the frame-stream analogue of the SIDX
  guard's canonical refusal.

## Alternatives considered

- **Feature-gate the wire-in survivors instead of wiring them.** Rejected by the
  owner: gating preserves the two-truths and hollow-seam hazards behind a flag
  rather than resolving them, and 0.10.0 is unpublished, so the wire-in cost is
  paid once now instead of compounding toward 1.0.
- **Leave unrecognized SIDX footers on the best-effort scan path.** Rejected: a
  best-effort read of a footer the version cannot authenticate is precisely the
  silent-degradation failure the integrity footer exists to prevent.

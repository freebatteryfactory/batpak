# DEATH WARRANT — `macbat-registration`

**Status:** temporary phase-bridge crate. Scheduled for deletion in **crate 3**.

**Issued:** 2026-07-13 (crate-1 build).
**Executes:** crate 3 (Muterprater), once the successor lands.

## Why this crate exists

`macbat-registration` is a **verbatim-in-behavior move** of the runtime scan
types that previously lived in `batpak-macros-support/src/lib.rs`:

- `pub extern crate inventory` + the two `inventory::collect!` registries
- `EventPayloadRegistration`, `UpcastRegistration`
- `upcast_steps_for`
- `IncompleteUpcastChain`, `find_incomplete_upcast_chains`
- `EventKindCollision`, `find_kind_collisions`
- `assert_no_kind_collisions`, `scan_for_kind_collisions`

plus exactly **one additive helper** — `projection_decode_abort(kind, cause) -> !`
— the O-21 interim hard-signal that the crate-1 projection lowering emits in
place of a literal `panic!` token.

**Honesty note on `projection_decode_abort`:** it is a **temporary
panic-equivalent hard signal** — panic machinery behind a named seam, NOT a
non-panic implementation. It removes the literal `panic!` token from generated
code and from this crate's source, but its runtime behavior IS a panic (same
unwind/abort, same message shape as the pre-bridge generated `panic!`; an
assert/abort wearing a mustache is still panic machinery). Its own death is
dated **earlier than the crate's**: it is deleted immediately when **crate 2's
fallible `EventSourced` trait** lands, replacing the abort with a structured
decode-failure error carrying event kind, payload version, coordinate, and
cause. The registration crate as a whole dies at crate 3.

It **deliberately duplicates** `batpak-macros-support` behavior during the
bridge (ledgered `InternalImplementation` / `TemporaryLegacyMirror`). Both
crates coexist through crate 1; `batpak-macros-support` is untouched.

The reserved-range oracle (`validate_category`, `validate_type_id`,
`TYPE_ID_MAX`) is **not** in this crate — it moved to `macbat-compiler`'s
`grammar/kind_rule.rs` (checked-in generated projection + `PARITY_HASH`).
Registration never performed that validation; the derive did.

## Successor (what replaces it, and why the warrant can fire)

Deletion is authorized once crate 3 provides all of:

1. **Contract-IR composition proof** — the collision / upcast-completeness facts
   are proven statically over the Contract-IR at composition time, replacing the
   link-time `inventory` scans (`find_kind_collisions`,
   `find_incomplete_upcast_chains`) and the generated per-type
   `#[cfg(test)]` collision-check function.
2. **Muterprater collision / upcast facts** — the structural gate that emits the
   same collision and incomplete-upcast diagnostics from the IR, removing the
   need for a runtime registry.
3. **O-21 VersionedBreak** — crate 2's fallible `EventSourced::apply_event`
   returns a typed error `{ kind, payload_version, coordinate, cause }`, so the
   projection lowering no longer emits `projection_decode_abort`; this helper
   (and its interim hard-signal) is then dead and removed.

When those land, this crate — and the `inventory` / `rmpv` dependencies it
carries — are deleted whole. Do not build new surface on it.

# Idempotent Pass

Agent surface task: `idempotent_pass`.

Problem: make a keyed append a durable, re-runnable no-op so a retry (after a
crash, a redelivery, a replayed request) commits the operation exactly once —
and stays a no-op even after retention compaction has evicted the event or the
store has cold-started.

Correct API: `IdempotencyKey::for_operation`, `AppendOptions::with_idempotency`,
`StoreConfig::with_idempotency_retention`.

Minimal code is mirrored by `bpk-lib/crates/batpak-examples/src/bin/idempotent_pass.rs`.

Derive the key from OPERATION IDENTITY, not payload bytes:
`IdempotencyKey::for_operation("account.credit", &[account, request_id])` hashes
a length-delimited `(domain, components)` tuple, so `["ab","c"]` and `["a","bc"]`
never collide. Re-running the same operation recomputes the same key, and the
second append returns the original receipt without writing a duplicate.

Growth is bounded by the window-priority hybrid (`IdempotencyRetention::Hybrid`,
the default): the window is the inviolable guarantee — a key whose original
commit is within `keep_sequences` of the frontier is never evicted by the
`max_keys` soft cap. A within-window retry is always a no-op regardless of load.

Wrong tempting move: treating `for_operation` as a content hash (it certifies
the OPERATION, not the payload), assuming dedup only survives the event
retention window (the durable `index.idemp` sidecar outlives event eviction),
or hand-editing/deleting `index.idemp` to "reset" a store — the sidecar is an
AUTHORITY and the store fails closed on a corrupt, missing-while-expected,
stale, or foreign image rather than double-appending acknowledged retries
(restore it from a backup or a healthy replica instead).

Test command: `cargo test -p batpak --test idempotency_durable_store --test idempotency_window_priority --all-features`.

Invariant protected: INV-IDEMPOTENCY-DURABLE-WINDOW — a within-window keyed
retry is always a no-op across compaction, cold-start, and snapshot.

## Disaster recovery — out-of-band authority export/restore (#188)

The `index.idemp` authority lives INSIDE the store directory, so a backup that
copies only segment files loses it. To survive total directory loss, snapshot
the authority out-of-band alongside your segment copies:

```rust
// Belt-and-suspenders backup: capture the canonical authority image bytes.
if let Some(export) = store.export_idempotency_authority()? {
    persist_out_of_band("idemp-authority.bin", export.as_bytes());
}
```

`export_idempotency_authority()` returns `Ok(None)` when the store carries no
user idempotency obligations yet (nothing to protect) and `Ok(Some(export))`
otherwise. The `IdempotencyAuthorityExport` bytes are the exact canonical on-disk
image — an opaque blob, not an embedder-visible schema; store them as bytes.

Recovery is OFFLINE. The primary disaster — `index.idemp` lost while `store.meta`
survives — refuses to open (`StoreError::IdempotencyAuthorityMissing`), so there
is no live store to call; restore is a closed-directory associated fn that runs
against the unopenable directory before any `open`:

```rust
let export =
    IdempotencyAuthorityExport::from_bytes(read_out_of_band("idemp-authority.bin"));
// config-first associated fn: no Store handle exists yet.
Store::restore_idempotency_authority(&config, &export)?;
let store = Store::open(config)?; // now opens; within-window retries stay no-ops
```

The restore call IS the authorization ceremony: it takes the store-dir lock,
validates the export against the surviving `store.meta` (lineage + coverage),
rejects a rollback to an older frontier, MINTS a fresh local authority image,
republishes it through the canonical authority protocol, updates `store.meta`,
and leaves the directory openable. It never requires the dead store to have
pre-authorized the incoming image's token — the closed, locked directory is the
admission control.

Pitfalls:

- **Export is a publication, not a peek.** It runs the authorize→publish→finalize
  protocol and mints a generation before handing you the bytes; the newest export
  supersedes older ones. An older export is offline-restorable only until the next
  authority publication moves the frontier past it — after that it refuses as a
  stale rollback (`IdempotencyRestoreRefusal::StaleRollback`).
- **Lineage rides with the directory, not the blob.** `store.meta` carries the
  lineage identity restore checks against; restoring an export into an unrelated
  store refuses `IdempotencyRestoreRefusal::ForeignLineage`. Keep `store.meta` in
  the same backup set.
- **Restore refuses mid-transition.** A directory with an unresolved compaction
  marker refuses with `IdempotencyRestoreRefused { reason:
  IdempotencyRestoreRefusal::PendingCompactionUnresolved }`; resolve the pending
  compaction (reopen writable once) before restoring.
- **Never hand-edit or delete `store.meta` or `index.idemp` to "fix" a refusal.**
  Every refusal (`Corrupt`, `FutureVersion`, `ForeignLineage`, `StaleRollback`,
  `EntriesBeyondCoverage`, `PendingCompactionUnresolved`) is a fail-closed signal
  that the image and the directory disagree; the cure is a correct backup, never a
  forged authority. To triage a directory WITHOUT opening it, call
  `Store::inspect_recovery_state(&config)` first.

Invariant protected: INV-IDEMPOTENCY-EXPORT-RESTORE-FAIL-CLOSED — every corrupt,
future-format, foreign-lineage, stale, mid-transition, or beyond-coverage export
is a distinct typed refusal, and a restored authority preserves the original
receipt identity of every keyed retry.

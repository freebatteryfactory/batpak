# ADR-0034: StoreFs Handle Abstraction

## Status

Accepted; 0.10.0 pre-1.0 breaking window (#168; successor to ADR-0018).

## Context

ADR-0018 promoted `StoreFs` to the store's filesystem seam, but its method
signatures were anchored on concrete `std::fs::File` / `tempfile::NamedTempFile`
/ `std::fs::ReadDir` / `std::fs::Metadata` handles. That made the seam suitable
only for alternate *native* backends — a pure in-memory or non-POSIX (wasm,
Durable-Object) backend cannot mint real OS handles, so it could not implement
the trait.

Issue #168 (drive the store end-to-end on an in-memory filesystem to prove
wasm runtime viability) assumed `SimFs` was such an in-memory backend. It was
not: `SimFs` was a durability-fault injector over real tempfiles, and its
concrete `File` handles made it neither diskless nor wasm-portable. The
`File`-anchored signatures were the wall — no in-memory backend could exist
while the seam trafficked in OS types.

## Decision

`StoreFs` is retyped onto abstract handles; the seam no longer returns concrete
OS types.

- **`StoreFile`** — an open handle (`write_all` / `sync_data` / `sync_all` /
  `len` / positioned `read_at` / `read_exact_at`), minted by `create_new_file`
  and the new `open_file`. A native escape `as_std_file() -> Option<&File>`
  lets platform-only optimizations (sealed-segment mmap, the mmap index) admit
  themselves through the EXISTING evidence/admission machinery; a virtual
  handle returns `None` and every read takes the byte-identical positioned-read
  fallback.
- **`StagedFile`** — the staging half of the atomic publish (`write_all` /
  `sync_all` / `persist`); replaces `NamedTempFile` in signatures and moves the
  staging fsync onto the seam (fault-injectable).
- **`StoreDirLockGuard`** — the held store-directory lock; `try_lock_store_dir`
  replaces the OS-`File`-typed lock acquisition.
- Owned **`DirEntryInfo`** / **`FileKind`** / **`FileStat`** replace the
  `std::fs::ReadDir` / `Metadata` returns.

`MemFs` — a public, pure in-memory reference backend — implements the retyped
seam with no OS file, tempfile, or mmap (`as_std_file` returns `None`; the
store-directory lock is an in-process registry). `SimFs` becomes a fault
*layer* over any inner backend (default `RealFs`) rather than a real-file fork,
so the seeded fault schedules interpose abstract handles and are
backend-agnostic.

## Consequences

- Any backend can implement the seam: alternate native volumes, fault layers, a
  pure in-memory tree, or a non-POSIX host. The #168 architectural blocker is
  removed; a real `Store` runs open→append→read→reopen entirely over `MemFs`.
- One shared conformance corpus (`tests/common`) runs the identical contract
  over `RealFs` and `MemFs`; a planted-divergence red fixture proves it bites.
- `SimFs::crash` truncation still requires a real-file-backed inner (the fault
  schedules themselves do not); this is documented on the type.
- The mmap fast path is now conditional on `as_std_file`; virtual and
  non-mmap-admitting hosts take the owned whole-file read — byte-identical
  downstream, since every consumer slices `&bytes[..]`.

## Alternatives considered

- **Keep the `File`-anchored seam, add a second in-memory trait.** Rejected:
  two filesystem seams with divergent contracts is the exact fragmentation the
  single seam exists to avoid, and the store's crash/durability proofs would
  have to be duplicated.
- **Defer to 0.11.** Rejected: 0.10.0 is unpublished, so the retype costs one
  breaking window and one CI round now instead of two later.

# Projections

The log is the source of truth. A projection is a cached interpretation.

Factory prose may call projections gauges. The Rust API should keep the engineering name `Projection`.

## Projection Rules

- A projection is derived from events.
- A projection may be rebuilt.
- A projection cache is optional acceleration unless a surface explicitly says otherwise.
- A stale projection must be classified, not silently treated as fresh.
- A projection failure should produce typed outcome or evidence when user-visible freshness depends on it.
- Derived state persisted OUTSIDE the store (projection sidecars, external
  cursors) should anchor to `Store::identity()` — the store's lineage identity,
  minted once and persisted in `store.meta` — PLUS a history anchor (the
  frontier sequence it was built at and the event id observed at that
  sequence). The identity alone distinguishes unrelated store lineages; it is
  copied by snapshot and fork, so it does not by itself detect rewind or
  sibling-fork divergence — that is what the history anchor is for.

## Outside The Model

If a value cannot be rebuilt from the log, it is application state outside batpak's projection model.

That may be valid application design. It is not a batpak projection.


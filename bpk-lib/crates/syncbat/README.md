# syncbat

Sync-first runtime layer for batpak-family operation surfaces.

```text
sb runs.
```

`syncbat` owns operation descriptors, handler registration, checkout dispatch,
runtime receipts, and durable operation-catalog rows through batpak public APIs.
It does not own network framing, async runtimes, application kit vocabulary, or
batpak store internals.

Safety defaults fail closed: `CoreBuilder::build` refuses to build without a
receipt sink (opt out with `without_receipts`), receipt hashing defaults to
`ReceiptHashPolicy::Blake3` (opt out with `Deferred`), and capability tokens are
enforced at checkout (grant them with `grant_capability` / `grant_capabilities`).

The runtime contract is documented in repository ADR-0028.

Live terminals: [05_TERMINALS.md](../../../05_TERMINALS.md). Composition:
[11_INTEGRATION.md](../../../11_INTEGRATION.md).

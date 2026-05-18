# ADR-0029: Netbat Boundary Contract

## Status

Accepted for the 0.7.6 correction cut.

## Context

`netbat` is the thin boundary layer above `syncbat`. It exposes already-built
sync runtimes through route metadata and bounded line-protocol frames. It does
not own handler execution, operation meaning, durable records, or admission
policy.

R4 closed the immediate request-encoder asymmetry. This ADR locks the full
boundary contract so future protocol changes are explicit instead of appearing
as ad hoc test byte strings or transport shortcuts.

## Decision

`netbat` owns:

- route metadata and endpoint validation
- server/module introspection
- bounded request/response frame encoding
- `NETBAT/1 CALL <operation-name> <hex-input>` request decoding
- stable response frames: `OK <hex-output>` and `ERR <code> <hex-message>`
- sequential blocking TCP listener behavior
- stable error-code mapping for malformed, limit, protocol, and runtime failures

`netbat` does not own:

- handler execution semantics
- receipt emission
- durable writes
- application protocol envelopes
- async runtimes or thread-pool policy

## Protocol Contract

The versioned request frame is:

```text
NETBAT/1 CALL <operation-name> <hex-input>\n
```

The first-rung legacy frame remains accepted for 0.7.6 compatibility:

```text
CALL <operation-name> <hex-input>\n
```

Unsupported `NETBAT/*` versions fail with
`ERR unsupported_protocol_version <hex-message>`.

All request decoding is bounded by `Limits`:

- maximum line bytes
- maximum operation-name bytes
- maximum decoded input bytes
- maximum encoded output bytes

The TCP listener is sequential and sync-first. Listener owners choose process
model, TLS, worker pools, and shutdown policy outside `netbat`.

## Streaming

R4 added batpak's `Canal` trait, but `NETBAT/1` remains request/response only.
Streaming transport, if added, must be a new explicit protocol decision rather
than a hidden extension to `CALL`.

## Consequences

- `netbat` receives a checked public API baseline.
- protocol fixtures and response shapes become traceable artifacts.
- downstream clients can encode requests against one owner function.
- future streaming work has a named decision point instead of accidental drift.

## References

- `003_NETBAT_NETWORK.md`
- `bpk-lib/crates/netbat/src/`
- `bpk-lib/crates/netbat/tests/`
- `bpk-lib/traceability/public_api/netbat.txt`

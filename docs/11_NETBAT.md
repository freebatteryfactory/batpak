---
status: AUTHORITATIVE
contract_id: BP-NETBAT-1
authority_scope: bounded transport, sessions, paging, proof carriage, and adapters
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# NetBat

## Identity

NetBat is the bounded transport layer for declared BatPak world entrypoints. It is thin by design. Its wire contract evolves under its own `NetBatProtocolVersion`; negotiating a transport version never upgrades any image or ISA.

NetBat may know:

```text
WorldInterfaceHash
EntrypointId
request/result schema references
page and cursor envelopes
receipt/proof envelopes
transport limits
session and delivery sequence
```

It may not know application domain meaning, projection reducer semantics, capability policy, proof interpretation, or PakVM instruction meaning.

## Runtime composition

NetBat depends on BatPak contracts and SyncBat's public world invocation surface. It does not duplicate runtime request types or construct a second operation registry.

```text
network frame
→ bounded decode
→ WorldInterface lookup
→ SyncBat invocation
→ result/receipt envelope
→ bounded encode
```

## Invocation carriage

The two invocation classes are owned by `docs/10`. NetBat carries them as two distinct request envelopes and nothing else:

```text
entrypoint.invoke.v1
query-program.execute.v1
```

Exact wire spellings are selected at the owning gate; the semantic envelopes are frozen now.

Entrypoint request carries: world identity, entrypoint identity, interface identity, input schema/version, input bytes, caller authority, request bounds.

Query-program request carries: ProgramImage bytes or `ArtifactRef`, `ProgramImageId`, query capability envelope, historical cut, result/proof bounds, input parameters.

NetBat must: bound lengths before allocation; transport opaque canonical image bytes; preserve typed dispositions. NetBat must never: interpret query semantics; widen capabilities; convert raw source to a ProgramImage; let TLS imply ProgramImage trust. There is no `CompileAndExecuteRawBatQL`, `EvalText`, or `ExecuteSource` envelope; source compilation is the separately admitted `CompilerPort` (DEC-051), not ordinary transport.

## Transport profiles

Initial host adapters may include native stream sockets and browser-compatible HTTP/WebSocket profiles. The transport profile is explicit and cannot change message-level proof meaning.

Valid TLS does not upgrade an invalid receipt, foreign cursor, wrong WorldImage, or stale source generation.

## Bounds

Every frame and session declares maximum:

```text
header and body bytes
decoded allocation
entrypoint name/ID size
request and response bytes
page items
cursor bytes
proof references and inline proof bytes
in-flight requests
retained push window
connection/session lifetime policy
```

Oversize values are refused before attacker-controlled allocation.

## Paging and cursors

Cursors remain opaque on the wire and are validated by the producing semantic layer. NetBat preserves exact bytes, limits their size, and returns typed refusal codes. It does not reinterpret query, collation, generation, or source identity.

## Push and pull

Pull is bounded and continuation-based. Push is long-lived but memory-bounded and carries explicit lag/overrun disposition. A durable pull path remains the recovery truth for any push surface that can lose awareness.

## Proof carriage

NetBat transports proof disposition, completeness, freshness, source frontier, result digest, WorldImage/Interface identities, TurnId, AttemptId, and ReceiptId when present. It does not manufacture or strengthen them.

## Generated clients

WorldInterface declarations may generate clients, servers, CLI/MCP schemas, and docs from one source. Generated SDKs invoke stable entrypoint and schema contracts rather than copying business logic.

## Network effects

PakVM never opens sockets. Network access is a typed capability request mediated by Bvisor and host ports. NetBat is one possible host transport, not ambient guest authority.

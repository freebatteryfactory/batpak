---
status: AUTHORITATIVE
contract_id: BP-LEGACY-INVARIANT-COVERAGE-1
authority_scope: one-for-one disposition of the live legacy invariant catalog
supersedes: legacy invariant catalog as target architecture authority
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Legacy Invariant Coverage

## Purpose

This matrix prevents a clean-room rewrite from preserving only the memorable laws. It classifies every invariant in the live legacy catalog at commit `6c4a46f8994528f0aeddfeb192868d084a73d1d8` as preserved, superseded, demoted, killed, or target-requalified.

The detailed clean obligations live in [Legacy Semantic Obligations](21_LEGACY_SEMANTIC_OBLIGATIONS.md). This document closes the denominator between the old invariant catalog and those successors. Legacy identifiers remain evidence pointers, not target ownership.

## Source manifest reconciliation

The denominator is the full **115-declaration** legacy invariant catalog at `LEGACY_SOURCE_COMMIT`, frozen in `spec/legacy_invariant_coverage.rs::SOURCE_INVARIANT_IDS`. The count is the number of `- id:` *declarations*, not a raw `INV-` token scan: the source view carries a generated `INV-CATALOG` block marker that resembles an invariant name but is a structural key, not a declaration, so it is excluded. The audit enforces exact set parity between `SOURCE_INVARIANT_IDS` and the coverage rows, so a declared invariant cannot silently lose coverage and a coverage row cannot cite an undeclared invariant.

This closure raised the denominator from the earlier 107-row capability snapshot to the full 115. The eight newly reconciled invariants are:

```text
INV-COMPACTION-NAMESPACE-COMMIT            -> LEG-082  (recovered obligation)
INV-IDEMPOTENCY-EXPORT-RESTORE-FAIL-CLOSED -> LEG-083  (recovered obligation)
INV-STOREERROR-HANDLING-CLASS             -> LEG-047
INV-STOREFS-CONFORMANCE-REUSABLE          -> LEG-016
INV-SIMFS-NAMESPACE-TRUTH                 -> LEG-016 and BP-TESTPAK-1
INV-FUZZ-REGRESSION-SEMANTIC              -> LEG-049
INV-PROOF-RECEIPT-COMPLETE                -> LEG-049
INV-ZERO-WARNINGS                         -> DEC-067 and DEC-068
```

The two recovered obligations had a witnessed law and no successor; the remaining six already mapped cleanly onto existing successors and needed only an explicit coverage row. The former 107-row snapshot is retained as secondary historical evidence, not a competing denominator.

## Dispositions

```text
PRESERVE    semantic law survives through named successor
SUPERSEDE   law survives but old mechanism/API/catalog does not
DEMOTE      useful compatibility/reference evidence, not native authority
KILL        intentionally absent from the target
REQUALIFY   valid only under an explicit target/profile proof scope
```

| Legacy invariant | Disposition | Clean successor | Rationale |
| --- | --- | --- | --- |
| `INV-ALLOW-IS-DESIGN` | SUPERSEDE | `BP-AGENT-GUIDE and BP-GAUNTLET-1` | Keep the zero-silencer law; replace the legacy checker implementation. |
| `INV-BATCH-ATOMIC-VISIBILITY` | PRESERVE | `LEG-003` | Atomic visibility is durable journal law. |
| `INV-BATCH-CRASH-RECOVERY` | PRESERVE | `LEG-004` | Incomplete batch recovery remains mandatory. |
| `INV-BIDIRECTIONAL-SUBSTRATE-LANE` | PRESERVE | `LEG-063` | Every committing boundary retains bounded domain-neutral replay access. |
| `INV-BUILD-FAIL-FAST` | SUPERSEDE | `BP-BOOTSTRAP-1 and TestPak structural gates` | Fail-fast construction remains; the legacy build.rs mechanism does not. |
| `INV-CACHE-CAPABILITIES-EXPLICIT` | PRESERVE | `LEG-068` | Acceleration policy remains discoverable and explicit. |
| `INV-CANONICAL-CONTAINER-CI` | REQUALIFY | `BP-GAUNTLET-1 target profiles` | Canonical environments are qualified per target rather than universal Linux product law. |
| `INV-CANONICAL-PATCH-STABILITY` | PRESERVE | `LEG-040` | Versioned canonical evidence bytes and goldens remain stable. |
| `INV-CHAOS-LINUX-ONLY` | REQUALIFY | `BP-GAUNTLET-1 target-qualified durability evidence` | Linux chaos evidence remains useful but cannot claim cross-target proof. |
| `INV-CHECKPOINT-V2-INTERNED` | SUPERSEDE | `LEG-017 and LEG-018` | Derived-cache compactness is retained; exact v2 interner layout is legacy mechanism. |
| `INV-CLOCK-NOW-US-LIVE` | PRESERVE | `LEG-069` | The default clock adapter must be live and typed. |
| `INV-COLD-START-ARTIFACTS` | PRESERVE | `LEG-017, LEG-052, and LEG-053` | Open/reopen artifacts and authority distinctions remain explicit. |
| `INV-COLUMNAR-REPLACES-DASHMAP` | SUPERSEDE | `LEG-018 and DEC-033` | The SIDX successor earns column tiles; the old overlay topology is not frozen. |
| `INV-COMPACTION-NAMESPACE-COMMIT` | PRESERVE | `LEG-082` | Compaction namespace publication stays crash-atomic, decided only by agreement of the pending marker, store metadata commit record, and authority image. |
| `INV-COMPLEXITY-EXPONENT-BOUNDED` | PRESERVE | `LEG-020` | Asymptotic and work-count gates remain anti-vacuous. |
| `INV-CONCURRENCY-SCHEDULE-PROOF` | PRESERVE | `BP-TESTPAK-1 deterministic schedule plane` | Schedule exploration remains required across idempotency, CAS, supervision, and compaction. |
| `INV-CONTEXT-VIEWS-DERIVED-FROM-HISTORY` | PRESERVE | `LEG-017 and LEG-026` | Views remain disposable derivations of accepted history. |
| `INV-COORDINATE-IS-LOGICAL-STREAM` | PRESERVE | `LEG-021` | Coordinate keeps its logical-stream meaning. |
| `INV-CROSS-DIRECTORY-CONSISTENCY-PRODUCT-OWNED` | PRESERVE | `LEG-054` | Cross-journal consistency remains runtime/product composition. |
| `INV-CRYPTO-SHRED-SCOPE-DESTROYS-PLAINTEXT` | PRESERVE | `LEG-015` | Erasure destroys plaintext authority without rewriting history. |
| `INV-DANGEROUS-TEST-HOOKS-NONDEFAULT` | PRESERVE | `LEG-065` | Dangerous levers remain absent from production profiles. |
| `INV-DELIVERY-AT-LEAST-ONCE-WITNESS` | PRESERVE | `LEG-034` | Checkpoint-backed at-least-once evidence remains explicit. |
| `INV-DEPENDENCY-DIRECTION` | PRESERVE | `BP-REPOSITORY-PACKAGES-1 and spec/architecture.rs` | The graph remains acyclic and direction-checked. |
| `INV-DOCS-CATALOG-VIEW-CURRENT` | SUPERSEDE | `BP-STATUS-1 plus typed spec facts and audit.py` | Generated views remain current, but the old catalog implementation is replaced. |
| `INV-DST-CORPUS-CURRENCY` | SUPERSEDE | `BP-GAUNTLET-1 corpus and receipt freshness` | Corpus replay currency survives under TestPak receipts. |
| `INV-DST-RECOVERY-LEGAL` | PRESERVE | `LEG-010` | Recovery legality remains prefix, rollback, or canonical refusal. |
| `INV-EVENT-PAYLOAD-DECODE-BACKCOMPAT` | PRESERVE | `LEG-002` | Historical bytes remain readable through explicit migration. |
| `INV-EVENTKIND-PARSE-ROUNDTRIP` | PRESERVE | `LEG-024` | Event-kind parsing remains total and collision-checked. |
| `INV-EXAMPLES-OBSERVABLE-OUTPUT` | DEMOTE | `BP-AGENT-GUIDE example observability rule` | Observable examples remain desirable; the exact stdout macro policy is not semantic law. |
| `INV-EXTERNAL-REPLAY-NO-SIDECAR-TRUTH` | PRESERVE | `LEG-017 and LEG-019` | Caches may accelerate but never authenticate replay. |
| `INV-FAULT-INJECT-GATED` | PRESERVE | `LEG-065` | Fault controls remain test-profile only. |
| `INV-FENCE-CANCELLED-STAYS-HIDDEN` | PRESERVE | `LEG-006` | Cancelled visibility remains durable hidden authority. |
| `INV-FORK-CRASH-ATOMIC` | PRESERVE | `LEG-011` | Fork publication remains crash-atomic. |
| `INV-FORK-HOSTILE-FS-REFUSAL` | PRESERVE | `LEG-011` | Hostile destinations cannot publish a half-fork. |
| `INV-FORK-ISOLATION` | PRESERVE | `LEG-011` | Mutable authorities never alias across fork. |
| `INV-FRONTIER-APPEND-GATE-HONORED` | PRESERVE | `LEG-051` | Wait timeout reports an unmet gate, not rollback. |
| `INV-FRONTIER-APPLIED-MIN` | PRESERVE | `LEG-070` | Applied progress cannot outrun a lagging required projection. |
| `INV-FRONTIER-DURABLE-COVERS-RECOVERED` | PRESERVE | `LEG-071` | Recovered durability claims remain honest. |
| `INV-FRONTIER-FAULT-ORDINALS` | SUPERSEDE | `G2 fault schedule under BP-GAUNTLET-1` | Fault points remain ordered and observable; exact legacy names are not frozen. |
| `INV-FRONTIER-MONOTONIC` | PRESERVE | `LEG-005` | Every frontier remains monotonic. |
| `INV-FRONTIER-OPEN-MONOTONIC` | PRESERVE | `LEG-005 and LEG-052` | Open progress cannot move behind recovered or closed history. |
| `INV-FRONTIER-ORDERING` | PRESERVE | `LEG-005` | Accepted, written, durable, visible, applied, and emitted ordering remains explicit. |
| `INV-FRONTIER-TORN-FREE` | PRESERVE | `LEG-005 and LEG-056` | Published frontier views cannot expose torn multi-field state. |
| `INV-FRONTIER-WAIT-MONOTONIC` | PRESERVE | `LEG-072` | Spurious wakeups cannot satisfy progress. |
| `INV-FUZZ-REGRESSION-SEMANTIC` | PRESERVE | `LEG-049` | Every critical semantic fuzz target keeps a committed minimized regression fixture proven red against the pre-fix decision core; file presence is not a receipt. |
| `INV-GAUNTLET-FOLD-FUSION` | PRESERVE | `LEG-031 and LEG-048` | Proof-plane fusion must equal separate passes. |
| `INV-GENERATED-WITNESS-PIN` | PRESERVE | `LEG-024 and LEG-046` | Generated collision and ownership witnesses remain bound to declarations. |
| `INV-GROUP-COMMIT-IDEMPOTENCY` | PRESERVE | `LEG-057` | Grouped admission requires stable idempotency. |
| `INV-HASH-CHAIN-INTEGRITY` | PRESERVE | `LEG-023` | Reorder, deletion, and mutation remain detectable. |
| `INV-HLC-JOIN-SEMILATTICE` | PRESERVE | `LEG-073` | The causal HLC algebra remains lawful and separate from commit order. |
| `INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION` | PRESERVE | `LEG-007 and LEG-009` | Authority and namespace publication remain one recoverable transition. |
| `INV-IDEMPOTENCY-DURABLE-WINDOW` | PRESERVE | `LEG-007` | Within-window retries remain durable no-ops. |
| `INV-IDEMPOTENCY-EXPORT-RESTORE-FAIL-CLOSED` | PRESERVE | `LEG-083` | Idempotency-authority export and restore stay fail-closed, lineage- and generation-bound, and republish a fresh AuthorityImageId through the destination authority chain. |
| `INV-IMPORT-CONTENT-ISOMORPHISM` | PRESERVE | `LEG-013` | Import preserves content/provenance while assigning destination identity. |
| `INV-IMPORT-CRASH-IDEMPOTENT` | PRESERVE | `LEG-074` | Interrupted import remains safely retryable. |
| `INV-IMPORT-NO-RUNAWAY` | PRESERVE | `LEG-075` | Same-journal import freezes its source ceiling. |
| `INV-INDEX-FILTER-COMPOSES` | PRESERVE | `LEG-017 and LEG-079` | Optimized filters must equal authoritative scan semantics. |
| `INV-INJECTED-CLOCK-OWNS-TIMEKEEPING` | PRESERVE | `LEG-055 and LEG-069` | Injected time excludes ambient access. |
| `INV-INVARIANT-WITNESS-TEST` | PRESERVE | `BP-GAUNTLET-1 audited denominator` | Named witnesses must resolve to executable proof. |
| `INV-JOURNAL-SINGLE-LIVE-OWNER` | PRESERVE | `LEG-001` | One journal has one live owner. |
| `INV-JOURNAL-WRITER-SERIALIZES-COMMITS` | PRESERVE | `LEG-001` | All commits share one writer-owned order. |
| `INV-LANE-BRANCH-ISOLATION` | PRESERVE | `LEG-050` | Lane heads and CAS state remain isolated. |
| `INV-LINEARIZABILITY-SINGLE-WRITER` | PRESERVE | `LEG-067` | Visible history remains a dense single-writer linearization. |
| `INV-LITERAL-REGEX-UNWRAP-SAFE` | KILL | `No general exception in final code doctrine` | The legacy tool-specific unwrap exception does not become target architecture. |
| `INV-MACRO-BOUNDED-CAST` | PRESERVE | `LEG-076` | Compiler narrowing remains checked and diagnostic. |
| `INV-MMAP-SEALED-READS` | DEMOTE | `BP-STORAGE-TILES-1 native adapter option` | Memory mapping remains an optional mechanism subject to workload proof. |
| `INV-MULTI-VIEW-PUBLISH-AFTER-VIEW-SYNC` | PRESERVE | `LEG-056` | Visibility follows every active view update. |
| `INV-NATIVE-DELETE-IDEMPOTENT` | SUPERSEDE | `BP-STORAGE-TILES-1 cache/artifact deletion conformance` | Idempotent deletion survives if the successor exposes the operation; NativeCache does not. |
| `INV-NETBAT-BOUNDARY-THIN` | PRESERVE | `LEG-045` | Transport remains bounded and domain-thin. |
| `INV-NETBAT-LINE-PROTOCOL-STABLE` | DEMOTE | `BP-NETBAT-1 compatibility inventory at G7` | NETBAT/1 is legacy compatibility evidence, not the native target protocol. |
| `INV-NO-DEAD-CODE-SILENCERS` | PRESERVE | `BP-AGENT-GUIDE and BP-GAUNTLET-1` | Dead code is deleted, gated, or re-owned rather than silenced. |
| `INV-NO-TOKIO-PROD` | PRESERVE | `LEG-080` | No hidden executor enters the semantic runtime. |
| `INV-OBSERVABILITY-FAILURE-PATHS` | PRESERVE | `BP-RECEIPTS-1` | Success, degraded, causal, and failure paths remain distinguishable. |
| `INV-ONDISK-FORWARD-COMPAT-CANONICAL` | PRESERVE | `LEG-053` | Every version pair has one canonical success or refusal. |
| `INV-OPEN-REPORT-RECEIPT` | PRESERVE | `LEG-052 and LEG-059` | Open/recovery behavior remains receipted and read-only opens remain side-effect free. |
| `INV-OUTBOX-DROP-SAFETY` | PRESERVE | `LEG-058` | Unsubmitted buffers publish nothing. |
| `INV-OUTCOME-FUNCTOR-COMPOSITION` | DEMOTE | `Reference-law corpus if equivalent combinators are retained` | The algebra is useful evidence but does not force the legacy public combinator API. |
| `INV-PAYLOAD-LENGTH-EXACT` | PRESERVE | `LEG-064` | Encoded lengths remain exact and bounded before allocation. |
| `INV-PAYLOAD-VERSION-NONZERO` | SUPERSEDE | `LEG-002 and LEG-053 under EventFrameV2` | Typed/current and legacy frames remain distinguishable; exact sentinel layout is selected at G2. |
| `INV-PER-LANE-FRONTIER` | PRESERVE | `LEG-005` | One physical durability axis and lane-scoped logical progress remain separate. |
| `INV-PERFORMANCE-GATES-ENFORCED` | PRESERVE | `BP-COMMAND-PLANE-1 and BP-TESTPAK-1` | Hardware-sensitive gates remain explicit TestPak commands. |
| `INV-PLATFORM-EVIDENCE-NOT-MEANING` | PRESERVE | `LEG-066` | Adapters provide mechanism and evidence, never semantic law. |
| `INV-POSITION-HINT-PERSISTENCE` | PRESERVE | `LEG-077` | Hints cannot replace writer-owned positions and must survive qualified representations. |
| `INV-PREPARED-BATCH-STAGING-EQUIVALENCE` | PRESERVE | `LEG-078` | Prepared staging remains an optimization only. |
| `INV-PROJECTION-FUSION-EQUIVALENCE` | PRESERVE | `LEG-031` | Fused folds equal separate folds. |
| `INV-PROJECTION-FUSION-EQUIVALENT` | PRESERVE | `LEG-031 and LEG-079` | Runtime fused replay preserves per-projection filtering and frontiers. |
| `INV-PROOF-RECEIPT-COMPLETE` | PRESERVE | `LEG-049` | The gauntlet reds rather than certifies vacuously; a proof receipt is complete only with command, features, binary, executed and skipped counts, corpus identities, and terminal result. |
| `INV-RECEIPT-SEALED` | PRESERVE | `LEG-059` | Only the owning gate/runtime constructs receipts. |
| `INV-RECOVERY-ORACLE-LEGAL` | PRESERVE | `LEG-010 and LEG-071` | Crash recovery remains independently classified and deterministic. |
| `INV-REPLAY-LANE-SELECTION` | DEMOTE | `LegacyMessagePack compatibility reader under LEG-002` | The old raw-MessagePack lane is evidence; native codec/layout selection supersedes it. |
| `INV-SCHEMA-VERSION-ISOLATION` | PRESERVE | `LEG-017 and LEG-028` | Derived state remains schema/version/generation isolated. |
| `INV-SEMANTIC-DIFF-EQUIVALENCE` | PRESERVE | `LEG-079` | Equivalent profiles must expose identical observables. |
| `INV-SIDX-TIMESTAMP-US-APPROXIMATION` | KILL | `Explicit time precision in the SIDX successor` | The millisecond approximation is legacy evidence, not target law. |
| `INV-SIMFS-NAMESPACE-TRUTH` | PRESERVE | `LEG-016 and BP-TESTPAK-1` | The fault-injection simulation models namespace durability separately from byte durability so the recovery oracle can express impossible directory outcomes. |
| `INV-STORE-ERROR-TAXONOMY` | PRESERVE | `LEG-047` | Failures remain structured and generated from owned declarations. |
| `INV-STORE-ISOMORPHISM-LAWS` | PRESERVE | `LEG-064 and LEG-079` | Durable identity seams retain round-trip/isomorphism proof. |
| `INV-STORE-LIFECYCLE-HONESTY` | PRESERVE | `LEG-052` | Writer and shutdown failures cannot report success. |
| `INV-STORE-LINEAGE-IDENTITY` | PRESERVE | `LEG-008` | Lineage is minted once and mutable authority binds stronger generation/history evidence. |
| `INV-STORE-SYNC-ONLY` | PRESERVE | `LEG-080` | The semantic store/runtime API remains synchronous. |
| `INV-STOREERROR-HANDLING-CLASS` | PRESERVE | `LEG-047` | Every StoreError variant maps to exactly one HandlingClass through one exhaustive taxonomy with no wildcard arm. |
| `INV-STOREFS-CONFORMANCE-REUSABLE` | PRESERVE | `LEG-016` | The StoreFs conformance corpus is a reusable harness proving native, memory, and wrapped-simulation backends against one law. |
| `INV-SUBSCRIPTION-STATE-MACHINE` | PRESERVE | `LEG-060` | Subscription lifecycle and closed/overrun terminals remain explicit. |
| `INV-SUBSTRATE-TRAVERSAL-DOMAIN-NEUTRAL` | PRESERVE | `LEG-063` | Substrate traversal never leaks domain semantics. |
| `INV-SYNCBAT-CAPABILITY-GRANT-ENFORCEMENT` | PRESERVE | `LEG-061` | Capabilities are checked before execution. |
| `INV-SYNCBAT-DISPATCH-RECEIPTS` | PRESERVE | `LEG-059` | Resolved calls emit at most one terminal receipt; sink failure is fail-closed. |
| `INV-SYNCBAT-EFFECT-ROW-ENFORCEMENT` | PRESERVE | `LEG-036 and LEG-061` | Observed effects remain a subset of declaration and grants. |
| `INV-SYNCBAT-REGISTER-CATALOG-DETERMINISTIC` | PRESERVE | `LEG-062` | Runtime composition/lifecycle facts rebuild deterministically. |
| `INV-TEST-PANIC-AS-ASSERTION` | PRESERVE | `BP-AGENT-GUIDE and BP-TESTPAK-1` | Tests use explicit assertion semantics without lint suppression. |
| `INV-TRACEABILITY-COMPLETE` | PRESERVE | `BP-GAUNTLET-1 and BP-STATUS-1` | Every important law terminates in owned evidence and every artifact has a role. |
| `INV-TYPELEVEL-COMBINATOR-LAWS` | DEMOTE | `Reference-law corpus if successor combinators exist` | The laws remain reusable evidence but do not force legacy API shape. |
| `INV-TYPESTATE-OPEN-HAS-WRITER` | PRESERVE | `LEG-052` | Open typestate cannot exist without the required writer mechanism. |
| `INV-UNTRUSTED-MANIFEST-NO-AUTHORITY-ESCALATION` | PRESERVE | `LEG-019` | Untrusted cache/manifest rows cannot become recovery authority. |
| `INV-WIRE-ROUNDTRIP-TOTALITY` | PRESERVE | `LEG-064` | Wire surfaces remain total on valid values and reject malformed bytes atomically. |
| `INV-WORKSPACE-DAG-ACYCLIC` | PRESERVE | `spec/architecture.rs, seedcheck.rs, and audit.py` | The workspace graph remains independently derived and acyclic. |
| `INV-ZERO-WARNINGS` | SUPERSEDE | `DEC-067 and DEC-068` | The zero-warnings posture is mechanically derived from lint tables, clippy and rustdoc deny lanes, and the armed zero-allow tripwire; the legacy gate implementation does not survive. |

## Note: the two fusion invariants are distinct

`INV-PROJECTION-FUSION-EQUIVALENCE` and `INV-PROJECTION-FUSION-EQUIVALENT` are not a duplicate row. They are two genuinely different laws and both are preserved:

```text
INV-PROJECTION-FUSION-EQUIVALENCE
    generic banana-split fold algebra: fused folds equal separate folds

INV-PROJECTION-FUSION-EQUIVALENT
    concrete Store replay behavior: fused replay preserves per-projection
    filtering, frontiers, watermark, and cache semantics
```

Both keep unique IDs; the 115-row denominator stands. Do not merge or renumber them.

## Closure law

A code agent may not import behavior from an unmapped legacy invariant. A newly discovered invariant is first added here and, when semantic, receives a `LEG-*` obligation. Exact legacy mechanism names do not become target API merely because their law is retained.

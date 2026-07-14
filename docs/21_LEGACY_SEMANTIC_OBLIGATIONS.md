---
status: AUTHORITATIVE
contract_id: BP-LEGACY-OBLIGATIONS-1
authority_scope: retained legacy behavior, clean successors, witnesses, and deletion conditions
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Legacy Semantic Obligations

## Purpose

This is the anti-bathwater ledger. It preserves law, edge cases, fixtures, and independent algorithms without preserving the legacy package graph or technical debt.

A clean implementation may not delete a row until its successor contract, independent witness, compatibility disposition, and gate receipt close. The one-for-one disposition of the live legacy invariant catalog is recorded in [Legacy Invariant Coverage](34_LEGACY_INVARIANT_COVERAGE.md).

## Row grammar

```text
ID
legacy evidence pointer
retained semantic law
clean owner
mechanism disposition
required witness
gate
```

## Journal and durability

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-001 | One live writer and one physical commit order per journal | `03_INVARIANTS.md`, store writer runtime | `batpak::store` | Reimplement cleanly | concurrency model + duplicate-writer refusal | G2 |
| LEG-002 | Accepted event bytes are immutable; shape evolves on read | `03_INVARIANTS.md`, event migration tests | `batpak::event` | Preserve readers, replace codec authority | historical vectors + future-version refusal | G2 |
| LEG-003 | Batch append is atomically visible | batch invariants/tests | `batpak::store` | Reimplement | concurrent reader + mutation + crash model | G2 |
| LEG-004 | Incomplete batch is discarded or canonically refused after crash/rotation | cold-start and batch recovery tests | `batpak::store` | Reimplement | cross-segment crash matrix | G2 |
| LEG-005 | Physical durability and per-lane logical frontiers remain distinct | lane frontier model/invariants | `batpak::frontier` | Preserve semantics, redesign types | algebra + crash/reopen | G2 |
| LEG-006 | Visibility fences can leave durable events hidden and that state survives reopen/export | visibility fence code/invariants | `batpak::visibility` | Reimplement without copied topology | cancel/reopen/snapshot fixtures | G2 |
| LEG-007 | Durable idempotency returns original receipt after reopen within retention law | idempotency authority tests | `batpak::store` | Preserve authority, new representation allowed | reopen, compaction, export/restore, damage refusal | G2 |
| LEG-008 | Store lineage and authority generation reject foreign mutable state | store identity/generation issue work | `batpak::store` | Strengthen | transplant/fork/restore fixtures | G2 |
| LEG-009 | Compaction publication and namespace durability are separate from content writes | compaction ADRs/tests | `batpak::compaction` | Reimplement | phase fault matrix + directory durability | G2 |
| LEG-010 | Recovery terminal is committed prefix, rollback, or typed refusal | SimFs/ShadowFs recovery corpus | `batpak::recovery` | Preserve model; rewrite mechanism | independent crash oracle | G2/G3 |

## Lifecycle operations

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-011 | Fork may share immutable sealed data but never mutable authorities | fork invariants and hostile filesystem tests | `batpak::lifecycle` | Reimplement | parent/fork isolation + crash matrix | G2 |
| LEG-012 | Snapshot binds exact source cut and evidence, not later frontier | snapshot report ADR/tests | `batpak::lifecycle` | Preserve semantics | concurrent append/source-stamp oracle | G2 |
| LEG-013 | Import preserves source content while assigning destination-local identity/order | import recovery/model | `batpak::lifecycle` | Reimplement | content parity + identity distinction | G2 |
| LEG-014 | Restore distinguishes missing cache, missing authority, lineage mismatch, and corruption | recovery manifests/tests | `batpak::recovery` | Strengthen taxonomy | hostile artifact matrix | G2 |
| LEG-015 | Crypto shredding destroys plaintext authority without rewriting accepted ciphertext/history | keyscope and delivery tests | `batpak::secret` | Preserve law; new key representation allowed | sibling-scope, tamper, reopen, compaction | G2 |

## Storage abstraction and derived data

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-016 | Storage adapters satisfy one handle/transaction/crash conformance corpus | StoreFs conformance, ADR-0034 | `batpak::storage_port` | Rename/rebuild interface as needed | native + memory + hostile adapter corpus | G2/G3 |
| LEG-017 | Fast-start checkpoint/mmap/SIDX artifacts are derived and never source authority | cold-start ADRs/invariants | `batpak::tile` | Supersede with generation-bound tiles | delete/corrupt/transplant/rebuild | G2/G8 |
| LEG-018 | SIDX has explicit fixed layout and independent frame-scan fallback | `store/segment/sidx.rs` | `batpak::tile` | First ColumnTile adopter | simple scan equivalence + byte goldens | G8 |
| LEG-019 | Derived cache cannot authenticate itself | untrusted-manifest ADR/invariants | `batpak::integrity` | Keep as constitutional law | forged cache that grades itself | G2/G3 |
| LEG-020 | Complexity regressions require work/count evidence, not noisy stopwatch alone | complexity invariant/bench gates | `testpak::bench` | Strengthen | planted quadratic/reference work model | G3/G8 |

## Identity, navigation, and integrity

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-021 | Coordinate names a logical context stream, not a filesystem/network shard | model/invariants | `batpak::coordinate` | Keep name and law | normalization/bounds/round-trip | G2 |
| LEG-022 | Commit pagination, ancestry, delivery cursor, projection path, and world path are distinct axes | traversal invariant and issues #199-#206/#208 | `batpak::navigation` | Strengthen type separation | cursor transplant + ordering oracles | G2 |
| LEG-023 | Content digest, event commitment, predecessor topology, signature, inclusion, and anchor are separate claims | authenticated-store issue family | `batpak::integrity` | Strengthen | mutation and external vectors | G2/G3 |
| LEG-024 | Event kind allocation and parsing are total, bounded, and collision-checked | EventKind tests/registry | `batpak::event` | Supersede ambient registry with explicit composition | compile/composition collision fixtures | G1/G2 |
| LEG-025 | HLC chronology does not replace global commit order, stream position, or causality | clock/order issue #208 | `batpak::time` | Repair contradictory legacy naming | algebra + deterministic simulation | G2 |

## Projection, query, and delivery

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-026 | Entity-local aggregate and selector-backed region materialization are different contracts | issue #199 | `batpak::projection` | Implement cleanly | coordinate-bearing replay/source-stamp tests | G2 |
| LEG-027 | Typed path/collation identity is versioned, bounded, locale-independent, and non-coercing | issue #200 | `batpak::path` | Implement cleanly | independent comparator/encoder | G2 |
| LEG-028 | Sparse projection trees are disposable views with bounded get/children/scan and generation-bound cursors | issue #201 | `batpak::projection` | Implement cleanly | page equivalence + stale cursor | G2 |
| LEG-029 | Projection query authority binds contract, selector, shape, bounds, source stamp, completeness, and proof | issue #202 | `batpak::projection_query` | Implement end-to-end | cross-layer equivalence oracle | G2/G6/G7 |
| LEG-030 | Full and incremental projection application agree at the same cut | projection tests | `batpak::projection` | Preserve | differential and mutation | G2/G3 |
| LEG-031 | Fold fusion equals separate folds over one frozen traversal | projection fusion laws | `batpak::projection` | Optimize later | banana-split oracle | G8 |
| LEG-032 | Lossy push is awareness; durable cursor pull is at-least-once replay truth | ADR-0011 | `batpak::delivery` | Preserve distinction | lag/overrun and restart fixtures | G2/G6 |

## Runtime and effects

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-033 | Reaction output is typed, atomically staged, and dropped structurally on handler error | ReactionBatch and typed reactor tests | `syncbat::runtime` | Generalize into EffectBatch | error/no-partial and causation fixtures | G6 |
| LEG-034 | Durable cursor checkpoint, at-least-once witness, panic recovery, restart budget, and explicit join errors remain visible | reactor/cursor worker | `syncbat::runtime` | Reimplement as ProcessContract runtime | deterministic schedule/reopen | G6 |
| LEG-035 | Cooperative and threaded drivers preserve one logical transition protocol | writer/spawn seam | `syncbat::runtime` | Preserve, simplify | trace equivalence | G3/G6 |
| LEG-036 | Declared effect row must contain observed effects and receipt payload | current SyncBat | `syncbat::runtime` | Preserve, compile from contracts | denial-before-effect + omission mutants | G6 |
| LEG-037 | Atomic typed batch append is one runtime effect, not a bypass | issue #229 | `batpak::effect` | Implement | single/batch equivalence + failure atomicity | G2/G6 |
| LEG-038 | External effects record requested/attempted/completed/failed/outcome-unknown and recovery class | outbox/reactor/attempt work | `syncbat::runtime` | Generalize | crash between external call and receipt | G5/G6 |
| LEG-039 | Public async integration does not leak raw Flume semantics or require an executor | issue #209 | `syncbat::runtime` | Supersede public mechanism | wait/poll/browser adapter parity | G6 |

## Composition, execution, transport, and proof

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-040 | Schemas have language-neutral identity, versions, canonical bytes, and goldens; Rust paths are diagnostic | HostBat schema work | `batpak::schema` | Keep semantics, dissolve shell | composition/golden fixtures | G2/G5 |
| LEG-041 | Composition identity is deterministic and changes with interface authority | HostBat fingerprints | `batpak::world` | Supersede | order-independence and collision tests | G5 |
| LEG-042 | One AttemptId binds start, execution, report, and reconciliation | Bvisor issue #187 | `syncbat::bvisor` | Keep and strengthen | stale report cross-attempt rejection | G5 |
| LEG-043 | Reports state established postconditions, not planned mechanisms | Bvisor issue #218 | `syncbat::bvisor` | Keep general law; archive native-exec details | fault injection and transcript oracle | G5 |
| LEG-044 | Admission decision circuits are bounded, canonical, and independently cross-checked | existing AdmissionProgram | `syncbat::pakvm` | Demote to specialized lowering | typed evaluator vs circuit differential | G3/G5 |
| LEG-045 | Transport is bounded, domain-thin, and cannot launder proof | NetBat | `netbat` | Preserve, regenerate contracts | oversize/cursor/proof round-trip | G7 |
| LEG-046 | One declaration becomes one typed Contract IR and generated lowerings | issue #214/current MacBat | `macbat-compiler` | Keep, rewrite cleanly | snapshots, origins, compile-fail, parity | G1 |
| LEG-047 | Error handling class and public error shape do not drift into side tables | error census/issues | `macbat-compiler` | Generate from declaration | variant addition compile failure/mutant | G1/G2 |
| LEG-048 | Repo facts, dependency roles, proof obligations, and mutations have one normalized fact plane | issues #207/#211/#213 | `testpak::repo` | Implement cleanly | duplicate/omitted fact fixtures | G3 |
| LEG-049 | Proof inventories require hostile semantic closure, rule qualification, and freshness | issue #197 | `testpak::gauntlet` | Keep and strengthen | planted self-blessing/omission cases | G3/G9 |

## Additional preserved edge laws

| ID | Retained law | Legacy evidence | Clean owner | Mechanism disposition | Required witness | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| LEG-050 | Per-entity lane heads, clocks, compare-and-set state, predecessor links, and branch queries remain lane-isolated while unfiltered reads preserve the all-lane timeline | lane branch and CAS invariants | `batpak::event` | Preserve law, redesign representation | deterministic schedule + cold-start lane reconstruction | G2/G3 |
| LEG-051 | An append durability/visibility gate timeout reports the unmet wait guarantee without pretending the already-committed event was rolled back | frontier append-gate invariant | `batpak::store` | Preserve exact outcome taxonomy | timeout with committed/queryable event fixture | G2 |
| LEG-052 | Open typestate implies a live writer; sync/drop failures surface honestly; read-only open never mints or appends authority/lifecycle state | store lifecycle and open-report invariants | `batpak::store` | Reimplement cleanly | writer-start failure, close failure, read-only no-side-effect fixtures | G2/G3 |
| LEG-053 | Every persisted format/version pair has one canonical open or typed refusal; corrupt regenerable caches may rebuild while damaged authority fails closed | compatibility matrix and future-version invariants | `batpak::compatibility` | Replace matrix format, retain law | forged future/corrupt artifact corpus | G2/G3 |
| LEG-054 | Cross-journal or cross-directory consistency is product/runtime composition, never an implicit single-journal guarantee | cross-directory invariant | `syncbat::runtime` | Keep boundary explicit | two-journal failure/interleaving model | G2/G6 |
| LEG-055 | Supplying an injected clock makes ambient wall/monotonic time unreachable through open, append, sync, close, and runtime transitions | injected-clock invariant | `batpak::time` | Preserve and strengthen with no_std profile | trapping ambient-clock adapter + deterministic trace | G2/G3 |
| LEG-056 | Visibility publication occurs only after every active index/tile view for the sequence range is populated | multi-view publication invariant | `batpak::store` | Generalize beyond legacy indexes | concurrent reader + planted omitted-view mutant | G2/G8 |
| LEG-057 | Group-commit admission requires stable item idempotency so replay cannot duplicate a partially acknowledged group | group-commit idempotency invariant | `batpak::store` | Preserve semantics | missing-key refusal + replay/original-receipt test | G2/G6 |
| LEG-058 | Dropping an unsubmitted output/effect buffer publishes nothing; only explicit submit/commit crosses the durable boundary | outbox drop-safety invariant | `syncbat::runtime` | Generalize into EffectBatch | drop-without-submit and destructor-order fixtures | G6 |
| LEG-059 | Receipts are constructed only by the owning gate/runtime; unknown operations do not fabricate receipts; receipt-sink failure cannot report success | receipt sealing and SyncBat receipt invariants | `batpak::receipt` | Preserve, unify receipt constructors | forged-constructor, unknown-call, rejecting-sink fixtures | G2/G6 |
| LEG-060 | Subscription sessions have explicit open/receive/close/overrun terminals and never fabricate delivery after producer closure | subscription state-machine invariant | `syncbat::runtime` | Reimplement over bounded push/durable pull | state-machine model + disconnect/overrun corpus | G6/G7 |
| LEG-061 | Required capabilities are checked before program/handler execution and concrete observed effects remain a subset of declaration, world imports, instance grants, and budget | SyncBat capability/effect-row invariants | `syncbat::bvisor` | Preserve, compile from contracts | denial-before-execution + undeclared-effect mutants | G5/G6 |
| LEG-062 | Durable runtime registration/composition state folds in journal order, rejects malformed/conflicting transitions, and rebuilds identically after reopen | SyncBat register-catalog invariant | `syncbat::world` | Supersede catalog with WorldImage/process lifecycle facts | full/reopen equivalence + conflicting transition fixture | G6 |
| LEG-063 | Public traversal and transport terminals expose domain-neutral substrate metadata, never downstream workflow entities or decoded application semantics | substrate traversal invariant | `batpak::navigation` | Preserve at generated interface boundary | API surface scan + hostile schema fixture | G2/G7 |
| LEG-064 | Public wire, ID, event, cursor, and receipt values round-trip every valid case, reject malformed bytes without partial success, and enforce exact declared lengths | wire totality and payload-length invariants | `testpak::oracle` | Preserve as shared conformance law | generated round-trip + truncation/overlength corpus | G2/G7 |
| LEG-065 | Fault injection, poison levers, allocator faults, and dangerous frontier mutation are absent from default production profiles | dangerous-hook invariants | `testpak::repo` | Preserve profile containment | package/API scan under default and test profiles | G0/G3 |
| LEG-066 | Platform adapters may emit evidence and enforce mechanisms but cannot define durability, replay, visibility, admission, or proof meaning | platform-evidence invariant | `batpak::port` | Strengthen through semantic profiles | cross-adapter semantic trace equivalence | G2/G5/G7 |
| LEG-067 | Visible journal history is a dense single-writer linearization: no gap below the visible high-water, monotonic rereads, convergent readers, and no return-order inversion | single-writer linearizability invariant | `batpak::store` | Preserve and prove independently | seeded concurrent operation model + pure history checker | G2/G3 |


| LEG-068 | Cache prefetch, acceleration, and retained-window behavior is discoverable through an explicit capability or configuration surface; no hidden cache policy changes semantics | cache-capability invariant | `batpak::tile` | Preserve law, redesign surface | capability discovery + disabled/enabled trace equivalence | G2/G8 |
| LEG-069 | The default clock adapter produces positive advancing wall observations, while an injected clock owns every observation and makes ambient time unreachable | live-clock and injected-clock invariants | `batpak::time` | Preserve as two explicit adapter contracts | live-adapter progression + trapping injected adapter | G2/G3 |
| LEG-070 | Applied progress is the minimum reflected frontier across every required registered projection; one lagging projection prevents an overclaimed applied frontier | applied-frontier invariant | `batpak::frontier` | Preserve semantics with repaired time/order types | two-projection lag model + zero-projection bootstrap | G2/G3 |
| LEG-071 | After reopen, the durable frontier honestly covers every visible recovered event without claiming which unacknowledged in-flight events had to survive | recovered-durable-frontier invariant | `batpak::frontier` | Preserve | honest/lying durability fault matrix + recovered-prefix oracle | G2/G3 |
| LEG-072 | Frontier and checkpoint waits succeed only after observing the target on the same state transition that wakes them; spurious wakeups never satisfy progress | frontier-wait invariant | `batpak::frontier` | Preserve independent of synchronization mechanism | deterministic spurious-wakeup schedule + threshold model | G2/G6 |
| LEG-073 | HLC causal merge remains a bounded commutative, associative, idempotent join with an explicit bottom; it never substitutes for commit or stream order | HLC semilattice invariant and issue #208 | `batpak::time` | Preserve algebra, repair names and uses | independent algebra/property model | G2/G3 |
| LEG-074 | Import interrupted at any durability boundary is safely retryable through stable import identity and cannot duplicate already-applied source events | import crash-idempotency invariant | `batpak::lifecycle` | Preserve | crash/reopen/reimport matrix | G2/G3 |
| LEG-075 | Same-journal import freezes its source visible ceiling before appending and cannot ingest its own newly created output | import no-runaway invariant | `batpak::lifecycle` | Preserve | tiny-page/rotation self-import fixture | G2/G3 |
| LEG-076 | Compiler-generated integer narrowing is preceded by an explicit bound check and produces a source diagnostic instead of truncation or panic | macro bounded-cast invariant | `macbat-compiler` | Generalize across compiler lowerings | boundary compile-fail corpus + narrowing mutants | G1 |
| LEG-077 | Caller position hints may provide only contract-authorized branch hints; writer-owned commit point, HLC, stream position, and sequence remain authoritative and survive every qualified representation | position-hint invariant | `batpak::event` | Preserve with repaired position taxonomy | live/reopen/cache/full-rebuild equivalence | G2/G3 |
| LEG-078 | Prepared or cached batch staging is an optimization only; it must commit, recover, publish, and receipt identically to fresh construction | prepared-batch equivalence invariant | `batpak::store` | Preserve | fresh/prepared differential under crash and reopen | G2/G8 |
| LEG-079 | Any two profiles or representations claiming semantic equivalence produce identical visible observables at one frozen cut; disagreement is a hard finding, never a tolerated optimization delta | semantic-diff invariant | `testpak::oracle` | Strengthen across native and browser profiles | seeded cross-profile semantic differential | G3/G8 |
| LEG-080 | The public store/runtime surface remains sync-first with no hidden executor; async and browser hosts own suspension and resume outside the semantic API | sync-only and no-runtime invariants | `syncbat::port` | Preserve, expose explicit adapters only | dependency scan + native/browser trace parity | G0/G5/G7 |

## Closure rule

A code agent may discover additional legacy laws. It adds a new `LEG-*` row before importing the behavior. It may not silently widen an existing row to cover unrelated semantics.

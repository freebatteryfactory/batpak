---
status: AUTHORITATIVE
contract_id: TP-PRODUCT-BEHAVIOR-1
authority_scope: current ThreadPak product, semantic machine, and public behavior
supersedes: prior product and machine models
last_reconciled: 2026-07-22
source_digest: sha256:6522437355313820A1A9182C4837A5BC2DBAAFB7EC82BF9296644B50FC9470F5
---

# ThreadPak Product and Behavior

This document is the current authority for the machine ThreadPak is building.
It defines product behavior and semantic ownership. It does not select an
explicitly open name, package spelling, byte layout, storage profile, or
implementation mechanism.

## 1. Product identity

ThreadPak is an opinionated library of semantic primitives. Its flagship
composition is an embedded, sync-first, event-native database and runtime.

The machine is designed around:

```text
immutable accepted events
typed contracts
bounded deterministic computation
explicit effects
capability admission
logical replay
physical attempts
typed evidence
reconciliation
derived materialization
generated implementations
independent qualification
```

ThreadPak is not an operating system. Events are durable root facts at semantic
boundaries; transient intermediate calculations do not automatically become
events.

## 2. Logical threads

A logical thread is the durable line connecting:

```text
intent
accepted facts
bounded decisions
logical turns
physical attempts
effects
outcomes
receipts
replay
reconciliation
```

It is not an operating-system thread. It may advance on one or many physical
threads, suspend without occupying one, resume after restart, move to another
machine, and outlive its original caller, connection, worker, or attempt.

An `Event` is one immutable accepted fact. A logical thread is a typed causal or
process view over accepted events and runtime evidence. The event is not copied
to create the view. One event may participate in entity, correlation,
business-process, effect, reconciliation, subscription, and process histories.

### Turn and Attempt

```text
Turn
    one bounded logical transition over a frozen input range

Attempt
    one physical effort to realize a turn or external effect
```

A replay of a logical turn retains its `TurnId`. Every physical execution gets
a fresh `AttemptId`. A response, observation, or receipt from one attempt cannot
satisfy another attempt.

## 3. Current naming ledger

| Name | Current role |
|---|---|
| `ThreadPak` | whole system, repository, and substantive primary library |
| `<LANGUAGE_NAME>` | current placeholder for the application language |
| `macros` | procedural-macro surface and private compiler implementation |
| `runtime` | internal sync-first logical runtime |
| `Serve` | separate networking, protocol, client, streaming, and server-construction library |
| `TestPak` | reusable testing and qualification arena |
| `PakVM` | portable execution machine for validated ProgramImages |
| `Muterprater` | TestPak mutation-testing subsystem |
| `Bvisor` | boundary supervisor for attempts, capabilities, budgets, and ports |
| `DataBlock` | expert physical-data abstraction |
| `ProgramImage` | one self-explaining directly executable program |
| `ApplicationImage` | deterministic persistent multi-program composition |
| `EventFrame` | canonical bounded envelope for one accepted event |
| `EffectBatch` | bounded ordered local publication intent |
| `Receipt` | typed surviving evidence about a specific boundary or claim |

The following names and extensions remain open:

```text
<LANGUAGE_NAME>
<LANGUAGE_SOURCE_EXTENSION>
<IMAGE_FORMAT_NAME>
<IMAGE_FILE_EXTENSION>
<HISTORY_FORMAT_NAME>
<HISTORY_FILE_EXTENSION>
<CLI_NAME>
```

An open public name does not reopen the settled behavior beneath it.

## 4. Application language behavior

The application language supports:

```text
ASK
DO
DEFINE FUNCTION
DEFINE ASK
decisions
transactions
effects
bounded reusable definitions
future persistent-process declarations
future application entrypoints
```

Its six-question structure is:

```text
WHO    subject, actor, entity, correlation, or partition
WHAT   value, state, decision, transition, effect, or result
WHERE  journal, projection, saved query, artifact, or authority source
WHEN   historical cut, interval, order frame, generation, or deadline
HOW    fold, match, group, order, page, bound, admit, execute, recover, qualify
WHY    explanation, evidence, provenance, proof posture, work, or receipt request
```

The language obeys these laws:

- `ASK` is pure.
- `DO` is explicitly effectful and receipted.
- `DO` evaluates required evidence and decisions before attempting an effect.
- unresolved evidence never silently becomes false;
- `ALLOW`, `DENY`, and `DEFER` remain distinct;
- historical cuts are explicit;
- chronology and durable order are distinct;
- authority-bearing arithmetic is exact and typed;
- units, rounding, margins, intervals, completeness, freshness, proof, and
  availability remain typed;
- work and outputs are bounded; and
- one evaluation produces its value and structured explanation, evidence,
  completeness, and work projections.

## 5. Definitions and semantic compression

ThreadPak uses a small typed semantic kernel plus named reducible definitions:

```text
small typed semantic kernel
    -> named reducible definitions
    -> concise application vocabulary
    -> exact canonical semantic expansion
    -> deterministic human and machine explanations
```

The established terms are `primitive operation`, `definition`, `derived form`,
`definitional expansion`, `normal form`, and `semantic signature`.

Definitions support progressive explanation:

1. a concise description;
2. a typed signature covering inputs, outputs, reads, writes, effects,
   capabilities, bounds, and failure behavior;
3. a structured explanation of decisive conditions, values, margins, missing
   evidence, provenance, and work; and
4. recursive expansion into admitted primitive operations or qualified kernel
   contracts.

Private implementation helpers use ordinary Rust documentation. They do not
need public semantic explications.

## 6. Semantic lowering

The application language has one continuous semantic spine:

```text
Source Form
    -> Semantic Form
    -> Execution Form
    -> ProgramImage
    -> PakVM
    -> Physical Plan
```

### Source Form

Source Form preserves author-facing structure: operations, the six-question
shape, names, comments, admitted shorthand, source locations, formatting, and
narration origins. It supports parsing, formatting, diagnostics, language tools,
source maps, and origin tracking. Presentation trivia does not define semantic
identity.

### Semantic Form

Semantic Form is canonical typed meaning. It makes resolved contracts,
definitions, types, units, historical cuts, query and traversal semantics,
three-valued truth, decisions, events, effects, capabilities, bounds, work
formulas, failure behavior, uncertainty, and explanation structure explicit.

It is typed, syntax-independent, normalized, bounded, language-neutral,
hashable, and independently inspectable. Meaning does not live in parser
spelling, Rust declaration order, source layout, compiler process state,
physical optimization, or serialized offsets.

### Execution Form

Execution Form is PakVM's portable baseline form. It exposes dataflow,
structured control regions, reads, folds, matches, decision branches, result
construction, effect boundaries, suspension points, semantic-work charges,
memory and output bounds, and attempt crossings. It may be graph-shaped or
region-shaped rather than a linear tape.

### ProgramImage

A `ProgramImage` packages canonical semantic meaning and portable execution:

```text
Semantic section
Execution section
type and schema descriptors
definition and operator tables
constants
inputs and outputs
effect declarations
capability requirements
source-cut requirements
work, memory, result, and effect bounds
explanation structures
imports and immutable resources
entrypoints
version and compatibility data
optional origin/source maps
optional qualification and attestation references
```

Carrying both forms makes the artifact directly executable and self-explaining.
It also establishes an independent seam: a checker reads Semantic Form,
independently lowers it, recomputes baseline Execution Form, and compares the
result to the embedded form. A mismatch invalidates the image.

### Physical Plan

A Physical Plan is a derived runtime-specific realization. It may fuse
operators, use columnar scans, select specialized kernels, prepare indexes,
vectorize work, or use admitted native or portable execution mechanisms.

It is replaceable, deletable, rebuildable, identity-bound, and differentially
qualified. Removing every Physical Plan may change performance but cannot
change meaning.

## 7. ProgramImage and ApplicationImage

A standalone ProgramImage reader must be able to determine:

- what the program does;
- what it may read, append, or request;
- which effects it may request;
- which capabilities it requires;
- which historical cut it uses;
- which work, memory, output, and effect bounds apply;
- what may remain unresolved;
- how results are explained;
- which semantic and format versions are required; and
- whether a runtime can validate and execute it.

The reader cannot require original source, the compiler process, the Rust tree,
repository documentation, conversation history, an online registry, or ambient
linker registration.

A ProgramImage executes directly. An `ApplicationImage` is used only for real
persistent composition: multiple programs, persistent processes, shared
resources, exported entrypoints, scheduling, supervision, lifecycle, or
deployment generation. It contains or binds contracts, schemas, ProgramImages,
process declarations, resources, imports, exports, capability requirements,
entrypoints, and lifecycle declarations.

Program and Application profiles share one image-format family. Its physical
layout and extension remain open.

## 8. Identity and canonical bytes

Semantic equivalence and exact artifact identity are distinct:

```text
SemanticGraphDigest
    commitment to the normalized semantic graph

ImageDigest
    commitment to exact serialized image bytes
```

Images may share one semantic graph while differing in permitted non-semantic
sections such as source maps, debug names, attestations, qualification
references, or packaging metadata. Behavioral equivalence needs separate
evidence.

ThreadPak does not impose one universal physical encoding. The byte roles are:

```text
ProgramImage / ApplicationImage    self-explaining image family
EventFrame                         bounded event envelope
accepted event history             append-oriented source truth
evolving records                   schema-owned canonical codec
dense sealed records               schema/layout-bound representation
DataBlock                           workload-specific physical layouts
external interchange               exact named external profile
receipts and cursors                role-specific semantic formats
```

A hash commits to selected canonical bytes; it does not define framing,
ordering, lengths, numeric representation, unknown-field behavior, extension
preservation, or selective decoding. Exact encodings are selected through
frozen requirements, independent readers, hostile corpora, workloads, and a
recorded selection or no-selection result. No incumbent wins by familiarity.

## 9. Accepted event history

Executable images and accepted history are separate because immutable execution
artifacts and append/crash-recovery truth have different physical needs.

Accepted history remains understandable after source and compiler loss.
`EventFrame` and the history container contain or immutably bind event contract,
event kind, schema and codec identity/version, payload interpretation, durable
order, chronology, causation commitments, source/store identity, related effect
and evidence identities, and compatibility posture.

An index, projection, cache, or DataBlock is derived. It cannot prove that an
accepted event exists or does not exist.

## 10. Bounded execution

ThreadPak admits structurally bounded computation rather than attempting to
decide termination for arbitrary programs. Static closure excludes or bounds
general recursion, arbitrary backward jumps, loops, source selectors,
allocation, output, effects, and suspension.

Runtime additionally meters portable semantic work. Distinct limits exist for:

```text
semantic work
memory
decoded bytes
rows
groups
matches
effects
artifacts
result bytes
suspended state
absolute monotonic deadline
```

Optimization does not grant a larger semantic budget. Exhaustion before
admission publishes nothing. Exhaustion after durable admission cannot erase
committed work.

## 11. ASK, DO, REQUEST, and AWAIT

`ASK` performs pure bounded evaluation. It does not append an event or request
an external effect.

`DO` evaluates the required query, evidence, and decision first. Effects occur
only after required conditions pass conclusively, and the result is typed
evidence of execution or non-execution.

`REQUEST EFFECT` durably admits an asynchronous effect intent and returns an
effect handle and receipt. Completion is represented by later evidence.

`AWAIT EFFECT` admits the same durable intent, performs an immediate bounded
attempt, waits within explicit limits, and returns completed, failed, refused,
cancelled, or outcome-unknown evidence.

REQUEST and AWAIT share one durable effect identity but remain distinct public
semantics. Dropping a waiter, future, browser promise, or connection ends
observation; it does not cancel or roll back admitted work. Cancellation before
admission and cancellation after admission are different operations.

## 12. Atomicity and external effects

ThreadPak promises exactly-once intent admission only inside the declared local
atomic `EffectBatch` boundary. External effects follow the external protocol.
Recovery properties may include idempotence by key, queryable outcome,
compensation, at-least-once-only behavior, and non-replayability. The exact
public recovery type structure remains open.

Missing acknowledgement does not prove non-execution. Process death does not
authorize retry. `OutcomeUnknown` remains explicit. Reconciliation appends new
evidence, and compensation is a new admitted effect rather than history erasure.

## 13. runtime, PakVM, and Bvisor

The internal sync-first `runtime` owns logical threads, turn selection, process
state, checkpoint authority, effect causality, restart legality, reconciliation,
and delivery coordination.

PakVM validates and executes ProgramImage Execution Form. It has no ambient
filesystem, network, process, environment, wall clock, entropy, raw pointer, or
system-call vocabulary. Those capabilities exist only through declared ports.

Bvisor owns capability admission, budget reservation, `AttemptId`, physical
attempt lifecycle, port mediation, suspension/response binding, physical
observations, and terminal attempt classification. It does not own application
composition, logical process state, `TurnId`, retry legality, checkpoints, or
domain decisions.

The semantic core remains safe Rust. An optional process-isolated hardening
profile may be evaluated later without altering ProgramImage semantics.

## 14. Delivery and asynchronous adapters

ThreadPak owns four delivery contracts:

```text
Mailbox     bounded many-submitters to one logical consumer
Completion  one admitted operation to one eventual observation
Broadcast   one producer to multiple observers with explicit lag/overrun
Permit      bounded reusable capacity
```

A channel library may implement a contract privately but does not define the
public semantic model or durable truth. Blocking, cooperative poll, future,
browser-promise, and asynchronous-runtime adapters drive the same synchronous
semantics. Scheduling may vary; meaning, order, cancellation, evidence, and
recovery do not.

## 15. Serve

Serve is the current product name for the separate networking and
server-construction library. Its exact Cargo and registry spelling is open.

Serve provides a bounded deterministic protocol state machine, client,
listener/server construction, sessions, request/response values, framing,
streaming, flow control, resumption, delivery checkpoints, carrier adapters,
authentication adapters, and ThreadPak server adaptation.

Its semantic core is synchronous. Carrier and asynchronous integrations drive
that core. The intended first networking epoch covers native encrypted streams,
HTTP request/response across modern versions, streaming HTTP, server-sent events
with a reverse request lane, bidirectional web sockets, modern web transport
with fallback, and push notifications as wake-to-pull.

Push is bounded awareness and acceleration. Pull plus a durable acknowledged
checkpoint is recovery truth. Carrier identity, caller identity, proof of
possession, capability grant, and operation admission remain separate.

Dependency direction is one-way: Serve consumes ThreadPak semantic values;
ThreadPak does not import Serve.

## 16. Time, order, cursors, and checkpoints

Keep these roles distinct:

```text
HLC                     chronology evidence
GlobalSequence          exact local-writer order
CommitPoint             durable historical cut
Causation               explicit predecessor topology
DeliveryIndex           session-direction transport order
Cursor                  immutable source/operation-bound continuation
SubscriptionCheckpoint  durably acknowledged progress
MonotonicDeadline       timeout authority
```

HLC is not commit order, a cursor, checkpoint, retry authority, deadline, or a
global total order. For an excessively future remote HLC, preserve `SourceHlc`,
record the skew, refuse its merge into `AcceptedHlc`, and advance accepted
chronology only from lawful local state.

Best-effort delivery may use an ephemeral continuation. Resumable at-least-once
delivery requires a durable acknowledged checkpoint, permits redelivery, and
forbids skipping accepted history. Exact cursor types and bytes remain open.

## 17. Authentication and admission

Carrier encryption protects transport. Identity proves who is asking. Proof of
possession constrains credential use. `CapabilityGrant` defines what the caller
may do. ThreadPak admission decides whether this operation proceeds. Transport
authentication never substitutes for capability admission.

Exact authenticated-history, secret-authority, and storage profiles remain
localized decisions. No unsupported profile is implied here.

## 18. DataBlock

`DataBlock` is the expert physical-data abstraction: one bounded physical
occurrence under one schema, layout, source binding, generation, and authority
role. Ordinary users do not need it for the paved road.

Expert surfaces may expose layout, columns, selections, batches,
materialization, and scan kernels. Implementations may explore row, column,
hybrid, dictionary, bitmap, offset, selective-decoding, mapped, batched, and
vectorizable layouts. A simple scalar or row-wise implementation remains the
independent semantic reference. Removing all derived DataBlocks cannot change a
semantic result.

The term `Tile` is reserved for an actually tiled physical layout.

## 19. macros

`macros` is one procedural-macro crate with one private compiler core and
derive, attribute, function-like, and internal compiler entrypoints. Macro
shells own syntax capture and diagnostics; the compiler core owns deterministic
lowering.

Expansion must be inspectable, source-mapped, diffable, stable, and independently
testable. Closed contracts may be partially evaluated. Public and
inspection-critical generated source may be committed; bulk mechanical output
remains derived and reproducible. Generated output is not semantic authority
merely because a generator produced it.

The dependency direction between the macro crate and ThreadPak remains open and
must be selected as one acyclic edge when compiler work begins.

## 20. TestPak and Muterprater

TestPak is the reusable expert development and qualification arena.
Muterprater is its mutation-testing subsystem. Production packages never depend
on TestPak.

Application contracts should generate reference-model skeletons, valid and
invalid generators, stateful and temporal properties, mutation seams, replay
fixtures, codec round trips, effect/capability refusal tests, benchmark
skeletons, and conformance snapshots. Applications extend these with domain
properties, independent models, and hostile fixtures.

Generated tests are useful; generated self-approval is forbidden. Important
semantics need an independent route such as a simple reference evaluator,
algebraic model, differential implementation, metamorphic relation, hostile
boundary test, trace model, or holdout workload.

## 21. Self-generation

Semantic law and realization remain separate. Generators may explore bounded
typed candidate grammars for algorithms, data layouts, fusion, specialization,
source generation, tests, and mutations. Origin does not grant authority.

Runtime evidence may produce candidates, counterexamples, hostile fixtures,
workloads, and proof gaps. It cannot silently rewrite semantic law. Independent
qualification promotes an implementation. A candidate cannot edit or become
its only judge.

Formal proof may serve narrow high-value kernels. The broader machine earns
trust through reference behavior, properties, fuzzing, mutation, deterministic
simulation, crash/recovery exploration, cross-run comparison, workloads,
holdouts, and typed evidence.

## 22. Crates and modules

The default is one substantive `threadpak` product crate. Semantic owners begin
as modules inside it. A separate crate must earn a dependency, distribution,
procedural-macro, independent-consumer, target-profile, publication, or release
boundary.

Current concrete workspace roots are:

```text
crates/threadpak    substantive product library
crates/macros       procedural macros with private compiler
crates/testpak      expert development and qualification
```

The runtime, PakVM, and Bvisor begin as semantic modules inside ThreadPak.
Potential later package roots for the application language, Serve, command
adapter, checker, publication mechanism, and storage profiles remain absent
until their governing decisions are recorded.

## 23. Public mental model

The default user model is intentionally small:

```text
Store
Event
Query
Projection
Subscription
Program
Application
Receipt
```

Expert users may work directly with contracts, definitions, ProgramImage,
ApplicationImage, effects, capabilities, DataBlock, layout, PakVM, Bvisor, Serve
protocol values, TestPak, and Muterprater.

Users do not need compiler internals, physical-plan caches, evidence internals,
package history, or repository process metadata to append an event, query
history, or execute a program.

## 24. Repository and documentation

The active authored documentation consists of this product/behavior document,
the implementation plan, and—after its packet selects requirements and
profiles—a canonical-byte document. Typed owners generate detailed API,
operator, format, protocol, identity, error, command, test, and support
references.

A package is a real dependency or distribution boundary. A directory is a
semantic owner within a package. A file contains one cohesive model, operation,
or adapter. Historical material remains in Git and qualified evidence rather
than the normal reading path.

## 25. Explicitly open decisions

These remain unresolved and cannot be silently selected:

```text
application-language public name, extension, and exact package spelling
Serve Cargo and registry spelling
image/history public names and extensions
command name and package spelling
checker package and implementation
generated-publication mechanism
supported durable storage profiles
exact Semantic Form and Execution Form Rust type names
exact semantic-kernel node inventory
exact public Definition shape
exact effect-recovery type structure
exact cursor hierarchy
exact receipt/evidence families and physical envelopes
exact ProgramImage, ApplicationImage, EventFrame, history, payload,
    cursor, receipt, and interchange encodings
stable public HLC operation surface
stable WorkObservation fields and serialization
macro/ThreadPak dependency direction
support and release claims
```

Open details are localized. They do not reopen settled semantic behavior.

## 26. Concise machine model

```text
ThreadPak's application language
    expresses typed bounded queries, decisions, transactions, effects,
    and processes.

Source Form
    preserves author intent and origin.

Semantic Form
    freezes normalized typed meaning.

Execution Form
    freezes portable PakVM execution.

ProgramImage
    packages both forms into self-explaining canonical bytes.

ApplicationImage
    composes programs and persistent resources only when needed.

PakVM
    executes validated programs.

runtime
    advances durable logical threads through bounded turns.

Bvisor
    supervises physical capabilities, budgets, attempts, and ports.

Serve
    exposes the machine through bounded deterministic protocol semantics.

accepted event history
    records immutable facts and durable effect intent.

DataBlock
    accelerates reads and computation without becoming authority.

TestPak and Muterprater
    attack implementations until one earns promotion.
```

> ThreadPak preserves the logical thread from intent to accepted fact, bounded
> execution, effect, outcome, and evidence through self-explaining programs,
> immutable events, explicit authority, and independently qualified
> implementations.

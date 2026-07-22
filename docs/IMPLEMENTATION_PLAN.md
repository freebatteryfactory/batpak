---
status: AUTHORITATIVE
contract_id: TP-IMPLEMENTATION-PLAN-1
authority_scope: dependency order, packet readiness, blockers, validation, and stop conditions
supersedes: prior implementation workflow and packet routing
last_reconciled: 2026-07-22
product_authority: TP-PRODUCT-BEHAVIOR-1
source_digest: sha256:6522437355313820A1A9182C4837A5BC2DBAAFB7EC82BF9296644B50FC9470F5
---

# ThreadPak Implementation Plan

This document is the current implementation authority. It orders work without
selecting open names, types, formats, profiles, mechanisms, or support claims.
The product behavior it implements is owned by
[Product and Behavior](PRODUCT_AND_BEHAVIOR.md).

## 1. Plan law

Work proceeds in dependency order. An arrow means the upstream owner must be
stable enough for its consumer before the downstream public contract freezes.
A localized blocker stops only the affected field, path, command, or packet; it
does not authorize an implementation agent to invent the missing decision.

Every packet:

1. names exact paths and semantic owners;
2. writes or extracts tests and independent oracles before the implementation
   becomes its own only witness;
3. preserves the relevant hostile and refusal cases;
4. records prerequisites and downstream consumers;
5. states acceptance commands and when they become available;
6. stops on semantic ambiguity rather than moving ownership to satisfy a
   compiler; and
7. retires old material only after surviving behavior and evidence have a
   current owner.

No path is created merely because it appears in a destination manifest. It is
materialized only by its owning packet when it has executable behavior, tests,
and a real consumer.

## 2. Current authorization boundary

```text
authorized now
    P00   active ThreadPak authority and routing
    P01A  minimal real Cargo workspace

not authorized
    P01 and every later packet
```

P00 and P01A are separate commits. P01A starts only after P00 is committed and
green. No remote push is part of this authorization.

P00 changes only:

```text
docs/PRODUCT_AND_BEHAVIOR.md
docs/IMPLEMENTATION_PLAN.md
README.md
AGENTS.md
docs/00_CONSTITUTION.md
docs/27_WORKFLOW.md
docs/29_STATUS_AND_SUPERSESSION.md
docs/33_AGENT_FINISH_LINE_CHECKLIST.md
```

The four numbered paths are temporary route adapters. Every other existing path
assigned to P00 for extraction remains byte-identical until P11.

P01A creates exactly:

```text
Cargo.toml
Cargo.lock
deny.toml
.cargo/config.toml
crates/threadpak/Cargo.toml
crates/threadpak/src/lib.rs
crates/macros/Cargo.toml
crates/macros/src/lib.rs
crates/testpak/Cargo.toml
crates/testpak/src/lib.rs
```

The three library roots contain crate documentation and safety/policy attributes
only. They define no semantic item, module route, placeholder API, generated
code, or architectural TODO.

## 3. Dependency graph

```text
P00  active authority
  -> P01A  literal threadpak/macros/testpak workspace
     -> P01   semantic types and identities
        -> P01B  independent oracle and corpus roots
        -> P02   canonical-byte requirements and bakeoffs
        -> P03A  Semantic/Execution Forms and ProgramImage
           -> P04   runtime, PakVM, effects, delivery, Bvisor
           -> P06A  semantic store, EventFrame truth, DataBlock reference
           -> P07A  private macros compiler and scratch artifacts

        -> P03B  source-language frontend
           [after the language name and root are selected]

        -> P05A  Serve semantic protocol and reference model
           [after the Serve package spelling is selected]

P05A -> P05B  carriers and conformance
P05A -> P05C  authentication profiles
P06A -> P06B  admitted native durable profile
     -> P06C  additional admitted profiles
P07A -> P07B  generated publication transaction
P01B -> P08   TestPak and Muterprater

P01 through P08 plus required name/profile decisions
  -> P09  public surface, command adapter, support, CI, release
     -> P10  narrow repository tools and generated references
        -> P11  retirement and residue closure
```

## 4. Command readiness

Acceptance commands use these states:

| State | Meaning |
|---|---|
| `AVAILABLE_NOW` | exists and is safe at the current boundary |
| `CREATED_BY_PACKET` | the packet creates the named target before using it |
| `AVAILABLE_AFTER_PACKET` | a named prerequisite creates the command or target |
| `BAKEOFF_REQUIRED` | evidence comparison must finish before selecting a winner |
| `ARCHITECT_DECISION_REQUIRED` | an explicit decision must exist before the target or public claim exists |

The last two states are deferred obligations, never an invitation to make a
provisional choice. An agent does not repair a missing command by creating
undeclared packages, files, dependencies, features, or public APIs.

## 5. Packet register

### P00 — Active authority and routing

Create the product/behavior document and this implementation plan. Route README
and the agent guide to them. Reduce the four numbered route adapters to visible
transitional pointers. Do not modify any other extraction source. No deletion
or archival occurs.

Stop if faithful tracked authority would require a new decision or an open
detail to be selected.

### P01A — Minimal real workspace

Create a private Cargo workspace containing only `crates/threadpak`,
`crates/macros`, and `crates/testpak`. Use resolver 3 and Rust edition 2024.
Track the lockfile. Create real but policy-only library roots. The macros package
is a procedural-macro crate but exports no macro yet. Add no dependencies among
the three packages.

P01A does not freeze a minimum toolchain, target support, feature policy,
portable-core profile, version, release posture, language package, Serve
package, command package, storage profile, checker, publication mechanism, or
macro dependency direction.

Stop if Cargo would require another package, dependency, source target, or
symbolic root.

### P01 — Semantic types and identity domains

Build the typed semantic spine without collapsing object, generation, semantic,
exact-byte, chronology, order, cursor, turn, attempt, effect, or evidence
identities. Preserve direct ProgramImage execution, optional ApplicationImage
composition, EventFrame authority, DataBlock derivation, and REQUEST/AWAIT
separation.

The stable public HLC operations and WorkObservation fields remain localized
blockers. P01 is not currently authorized.

### P01B — Independent judgment and corpora

Extract hostile cases, models, vectors, and corpus manifests into independent
roots before the implementations they judge. Preserve source separation,
model/trace separation, honest denominators, holdout independence, mutation
accounting, and source/target/run binding.

The permanent checker implementation remains open. P01B is not currently
authorized.

### P02 — Canonical-byte workstreams

Freeze requirements, candidate registers, independent readers, hostile corpora,
workloads, and immutable selection or no-selection records separately for each
byte role. Complete the active authored documentation surface with
`docs/CANONICAL_BYTES.md` only when the packet is authorized.

No image, event, history, payload, receipt, cursor, identifier, compression, or
interchange format wins by familiarity. Receipt/evidence ownership, HLC result
shape, and WorkObservation serialization must exist before their bytes freeze.

P02 is not currently authorized.

### P03A — Semantic/execution spine

Inside ThreadPak, implement Definition, Semantic Form, Execution Form,
ProgramImage, validation interfaces, language-neutral fixtures, and the
independent re-lowering comparison. This packet does not require a public
language name or package.

Exact ProgramImage bytes remain gated by P02. P03A is not currently authorized.

### P03B — Source-language frontend

After the application-language name, extension, package boundary, and literal
root are recorded, implement Source Form, parser, diagnostics, formatting, and
lowering into ThreadPak-owned Semantic Form. The frontend cannot own runtime,
durable storage, or effect execution.

P03B is blocked by the open language root and is not authorized.

### P04 — runtime, PakVM, effects, delivery, and Bvisor

Implement bounded logical turns, physical attempts, effect admission and
recovery, PakVM validation/execution, boundary supervision, Mailbox, Completion,
Broadcast, Permit, and synchronous semantics with adapters.

PakVM has no ambient authority. Bvisor cannot decide retry legality or advance
logical checkpoints. runtime cannot bypass Bvisor. P04 is not authorized.

### P05A — Serve semantic protocol

After literal package spelling is selected, implement the synchronous protocol
state machine, message algebra, bounds, sessions, client, listener/server core,
delivery profiles, and scripted reference model. The dependency is Serve to
ThreadPak, never the reverse.

P05A is blocked by package spelling and is not authorized.

### P05B — Serve carriers

Implement carrier adapters only after the semantic core exists. Prove semantic
equivalence across admitted carriers. Library selection and carrier performance
remain evidence-selected. P05B is not authorized.

### P05C — Authentication profiles

Preserve the separation among encrypted carrier, caller identity, proof of
possession, capability grant, and operation admission. Implement only approved
profiles. P05C is blocked by profile decisions and is not authorized.

### P06A — Semantic store and DataBlock reference

Implement accepted event-history semantics, EventFrame truth, deterministic
in-memory reference storage, recovery/compaction transitions, DataBlock meaning,
and a simple independent layout reference. Derived state cannot authenticate or
invent accepted events.

Exact event/history bytes remain P02-gated. P06A is not authorized.

### P06B — Native durable profile

Implement only the selected native durable storage profile, including crash,
locking, publication, compaction, and conformance evidence. Its placement and
mechanisms follow the storage-profile decision and measurements. P06B is
blocked and not authorized.

### P06C — Additional storage profiles

Add browser-local, embedded, or other profiles only after each is explicitly
admitted with support and conformance requirements. P06C is blocked and not
authorized.

### P07A — Private macros compiler

Implement one deterministic private compiler core, frontend parity, source maps,
diagnostics, partial evaluation, artifact provenance, and non-authoritative
scratch output. Choose exactly one acyclic macro/ThreadPak dependency direction
inside this packet; never create a cycle or a second compiler package.

P07A is not authorized.

### P07B — Generated publication

Publish generated output only through an approved declared unit with staged
validation, atomic publication, complete output manifest, provenance, and
divergence behavior. Until that mechanism is decided, generated scratch output
has no authority. P07B is blocked and not authorized.

### P08 — TestPak and Muterprater

Implement the reusable qualification arena, honest requirement denominator,
independence tracking, mutation inventory, holdout separation, candidate
evaluation, and generated application test kit. Production packages cannot
depend on TestPak. Public command spelling remains later work.

P08 is not authorized.

### P09 — Public surface, support, and release

Freeze the curated ThreadPak surface, examples, command adapter, support
authority, CI matrix, compatibility/removal policy, and release evidence only
after lower owners and open names/profiles are resolved. One normalized support
authority may project multiple views; claims require current qualification.

P09 is blocked and not authorized.

### P10 — Repository tools and generated references

Replace broad bootstrap machinery with narrow tools for repository enumeration,
generation, verification, source/generated boundaries, and generated API,
language, byte, protocol, test, and support references. The permanent independent
checker implementation remains a localized decision.

P10 is not authorized.

### P11 — Retirement and residue closure

Retire extraction sources and route adapters only after every surviving
behavior, hostile fixture, workload, oracle, and proof technique has a current
owner and consumer. Git preserves history. No compatibility reader or facade is
kept without a real external obligation.

P11 is not authorized. No earlier packet may perform its deletions.

## 6. Tests and independent oracles first

An implementation cannot be its own only decoder, model, oracle, or judge.
Before or with each behavior, establish an independent route appropriate to the
risk:

```text
type-level separation
simple reference evaluator
algebraic model
state-transition model
differential implementation
metamorphic relation
independent decoder
hostile boundary corpus
deterministic scheduler or trace model
crash/recovery exploration
holdout workload
dependency/source-boundary checker
```

Every planned test remains in the denominator with an explicit result such as
pass, fail, not run, refused, unsupported, timed out, stale, or infrastructure
failure. A generator never supplies the sole verdict for its own output.

## 7. Extraction before retirement

Existing files have one of three migration roles:

```text
TARGET_FILE        survives at its current path
TRANSITIONAL_EDIT  changes only to route the migration safely
EXTRACTION_SOURCE  supplies behavior or evidence and remains unchanged until retirement
```

Extraction does not authorize renovation. Retirement waits until the replacement
owner, consumer, hostile cases, oracle, and residue checks exist. If a file
contains the only known witness for a behavior, it cannot be removed.

## 8. Localized blockers

The following block only their affected surfaces:

| Open decision | Blocks |
|---|---|
| receipt/evidence family ownership | exact types, identifiers, envelopes, and fields |
| generated publication | final publication unit and atomic transaction |
| support authority | public support projections, CI claims, release evidence |
| HLC public operations | stable join/observation signatures and result shape |
| storage profiles | durable adapters and public storage promises |
| WorkObservation | stable public fields and serialization |
| application-language name/root | source-language package and spelling |
| Serve package spelling | literal networking package paths |
| command name/root | command package and public commands |
| checker implementation | checker package, language, and lifecycle |
| physical-format bakeoffs | only the exact byte role being selected |
| macro dependency direction | the compiler edge selected in P07A |

None authorizes a temporary name or placeholder package.

## 9. Global stop conditions

Stop the affected work rather than improvising when:

- a required starting commit or branch differs;
- two current owners contradict each other;
- product behavior cannot remain faithful to its authority;
- an open name, format, type, profile, package, or mechanism must be selected;
- a new dependency or reverse edge appears necessary;
- a symbolic root would need provisional materialization;
- an implementation would grade itself;
- chronology is used as durable order or timeout authority;
- REQUEST and AWAIT collapse;
- ProgramImage requires application composition to execute;
- DataBlock becomes accepted-event authority;
- ThreadPak imports Serve;
- production code depends on TestPak;
- the macro dependency direction would be chosen outside P07A;
- an extraction source must change to make a gate pass; or
- deletion or archival would occur before P11.

Compiler errors do not authorize moving types, adding packages, reversing
dependencies, widening public APIs, or inventing adapters.

## 10. Completion standard

A packet is complete only when its exact path set, tests, independent evidence,
hostile cases, acceptance commands, residue checks, and stop-condition audit are
recorded and green. Commits remain packet-scoped. Downstream work starts only
after its prerequisites are complete and its own authorization exists.

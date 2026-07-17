---
status: AUTHORITATIVE
contract_id: BP-TESTPAK-1
authority_scope: repository proof battery, Repo IR, role tree, mutation, gauntlet, and command authority
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# TestPak

## Identity

TestPak is the repository proof battery and development command authority. It is dev-only and may inspect every package. It is not product runtime.

## Role-based tree

```text
testpak/src/
├── fixture.rs +   loaders/classifiers/generators over the fixture assets
│   fixture/       (frozen bytes live in crates/testpak/fixtures/, not here)
├── model/         deliberately simple reference semantics
├── oracle/        independent expected-result computation
├── harness/       production implementation drivers
├── schedule/      deterministic, hostile, crash, and concurrency schedules
├── bench/         workload definitions and measurement contracts
├── corpus.rs +    load/index/minimize the persisted corpus cases
│   corpus/        (frozen bytes live in crates/testpak/corpus/, not here)
├── repo/          Repo IR and architecture inspection
├── forge/         deterministic generated artifacts
├── gauntlet/      trust-boundary and proof-of-proof orchestration
├── receipt/       proof, build, mutation, and release receipts
├── command/       repository command catalog
└── muterprater/   mutation planning, execution, classification, explanation
```

Proof classes such as property, state-machine, differential, equivalence, fault-injection, crash-recovery, structural, and complexity are metadata on harnesses. They are not architecture folders.

## Canonical asset homes

Frozen assets and Rust that operates on them are separate:

```text
crates/testpak/corpus/     frozen bytes: compatibility, fuzz, recovery, mutation
crates/testpak/fixtures/   hostile/negative fixtures and deterministic inputs
crates/testpak/src/        loaders, classifiers, generators, minimizers
```

These are the sole canonical homes (DEC-044). Root `corpus/` and root `fixtures/` are forbidden target paths. A package may keep a tiny local test fixture only when it is explicitly noncanonical and owned solely by that package's own test — never a canonical mirror.

## Binary target

TestPak ships its command door as a binary in the same package — no separate command crate (DEC-045):

```text
crates/testpak/src/lib.rs
crates/testpak/src/bin/testpak.rs   (publish = false)
```

The root `justfile` is a discoverability projection over `cargo run -p testpak -- <command>`. TestPak launches and checks `batpak-examples` through workspace metadata / Cargo commands, never through a Cargo dependency edge to the examples package.

## Repo IR

Repo IR is a typed column/fact plane for:

```text
semantic concepts and owners
contracts and generated lowerings
source files and public items
dependency roles and profiles
proof obligations and witnesses
fixtures and mutants
compatibility dispositions
document status and supersession
commands and receipts
```

One traversal may feed many fitness systems when lawful. The fact plane is not a second authority; facts point to canonical owners.

## Forge

Forge materializes generated tables, manifests, docs, SDKs, corpus indexes, and command schemas from signed facts. A generated view carries source contract ID and digest. Hand-maintained mirrors are refused after a generated successor exists.

## AST enforcement gate (DEC-068)

This gate does **not** exist yet. It is implemented at G3. This section freezes its exact obligation so implementation cannot quietly narrow it. Until then, the bootstrap lexical scan (DEC-067) provides defense in depth; the TestPak AST gate is the authoritative Rust classification.

TestPak's AST gate owns detection of:

```text
function-local and closure-local named domain types
unsafe blocks, functions, and impls
pointer and integer-pointer casts
narrowing numeric casts
lint suppressions (#[allow] / #[expect])
panic / unwrap / expect / dbg in production source
dependency-owned public types in ordinary public APIs
drawer modules (utils, common, helpers, manager, processor, misc)
stale architecture aliases (derived from spec/dispositions.rs)
canonical-order dependence on hash-map iteration
```

Required behavior:

```text
production violation           hard finding
test-only contextual expect    permitted
dangerous mechanism            requires an exact compiler-assumption ledger entry
bootstrap lexical result       defense in depth
TestPak AST result             authoritative Rust classification
```

This document owns the detector inventory, the required-behavior table, and the defense-in-depth posture. It does not own executable proof-row identity or per-row meaning: those live in `docs/24_GAUNTLET.md`, and a meaning changes there or nowhere.

Required proof rows, projected from docs/24 (qualification target: `DEC-068`; canonical proof-row owner: docs/24 Gauntlet):

```text
local_domain_type_inside_function_is_rejected
local_domain_type_inside_closure_is_rejected
test_local_nonsemantic_fixture_type_is_allowed
production_expect_is_rejected
test_expect_with_context_is_allowed
unledgered_unsafe_fn_is_rejected
unledgered_pointer_cast_is_rejected
public_flume_receiver_is_rejected
hash_map_iteration_in_canonical_encoder_is_rejected
drawer_module_name_requires_explicit_disposition
```

## Independent models

The product PakVM interpreter executes real programs. TestPak retains deliberately simple evaluators for selected algebras: decision/K3, codec, path ordering, projection tree, schedule, delivery, and recovery. It does not maintain a second full VM religion.

## Mutation lanes and result algebra (DEC-015, DEC-074)

Muterprater is TestPak's mutation planning, activation, evidence, classification, selection, and receipt mechanism. It is not a standalone product package, a second semantic authority, a replacement compiler, a replacement test runner, a Rust interpreter pretending to replace rustc, or an agent that writes and approves its own tests.

### Three frozen lanes

```text
SemanticIr           mutates BatPak-owned semantic structures: Contract IR,
                     schema IR, typed BatQL IR, ProgramImage and PakVM semantic
                     nodes, decision formulas, projection plans, receipt
                     policies, numeric and interval laws, effect declarations.
                     No per-candidate Rust compile. The reference interpreter
                     executes the mutant; an independent evidence route judges it.

SelectableCompiled   MacBat may generate mutation slots into test-profile Rust.
                     One shard holds many candidate slots; one stable mutation
                     identity selects one slot per isolated process; the shard
                     compiles once for its declared candidate set; every slot
                     records whether it was actually activated. Slots exist only
                     in an explicit test or mutation profile and never enter an
                     ordinary production artifact. A release artifact carries no
                     ambient runtime mutation selector.

CompilerBacked       implementation-sensitive material whose truth depends on
                     real compiler and platform behavior: handwritten storage
                     and recovery, concurrency and atomics, unsafe boundaries,
                     cryptographic integration, codecs, FFI, OS adapters,
                     handwritten optimized kernels, layout-sensitive code.
                     Candidates execute under real rustc semantics.
```

Muterprater may use compiler-backed mutation tooling as a backend. It never claims a homegrown Rust evaluator equals rustc, and V1 promises no wholesale replacement of external mutation tools.

The executed lane facts are projected from the typed owner:

<!-- MUTATION-LANES:BEGIN generated from spec/mutation.rs by bootstrap/project.py; do not edit -->
```text
lane                 compile rustc   activ   slots   indep   gates
SemanticIr           false   false   true    false   true    G3
SelectableCompiled   false   true    true    false   true    G3
CompilerBacked       true    true    true    false   true    G3
```
<!-- MUTATION-LANES:END -->

### Runner and differential boundary

```text
Muterprater   mutation identity, discovery, typed plan, lane selection, test
              selection, activation evidence, result classification, freshness,
              receipts, promotion proposals
nextest       isolated test-process execution and scheduling
rustc         Rust compilation semantics
cargo-mutants or equivalent
              an independent differential or migration backend until Muterprater
              qualifies its retirement
```

Nextest executes; it is never semantic authority. No retirement deadline or automatic milestone exists. Retirement requires a later qualification receipt comparing, on an agreed corpus: candidate locations, mutation operators, buildability, activation, killed and survived outcomes, runtime, bugs detected, and blind spots. No coronation by architecture prose.

### Terminal result algebra

Eight categories, never collapsed into pass/fail, killed/not-killed, or success/error:

```text
Killed                  activated; baseline and selected-test posture qualified;
                        one or more qualified witnesses reject the mutant
Survived                activated; selected test set qualified for the target;
                        no selected witness rejected the mutant
NotActivated            not observed as reached or selected. Never Survived.
Refused                 admission or policy refused the candidate or run. Never Killed.
Unbuildable             could not produce the required executable or admitted
                        semantic artifact. Never Killed.
TimedOut                exceeded its declared deadline. Neither Killed nor Survived.
InfrastructureFailure   runner, compiler, environment, or proof mechanism failed.
                        Neither Killed nor Survived.
EquivalentCandidate     may be equivalent under the named semantic scope;
                        exclusion requires an independent equivalence witness
```

A `Survived` verdict without activation evidence is invalid. A `Killed` verdict without a qualified baseline and activation evidence is invalid. A failing test that also fails under the baseline does not kill the mutant. A timeout is not a kill. An unbuildable candidate is not a kill: a compiler error produced by an invalid mutation is not evidence that the test suite detected a semantic fault. `EquivalentCandidate` is a classification pending its witness, never equivalence proof.

The executed classification is projected from the typed owner:

<!-- MUTATION-RESULTS:BEGIN generated from spec/mutation.rs by bootstrap/project.py; do not edit -->
```text
result                 kill    surv    denominator
Killed                 true    false   true
Survived               false   true    true
NotActivated           false   false   true
Refused                false   false   true
Unbuildable            false   false   true
TimedOut               false   false   true
InfrastructureFailure  false   false   true
EquivalentCandidate    false   false   true
```
<!-- MUTATION-RESULTS:END -->

### Denominator truth

Every planned mutation reaches an explicit terminal disposition. Nothing silently leaves the denominator because of feature selection, cfg state, parser drift, generated-code blindness, unbuildability, timeout, backend limitation, test exclusion, lack of activation, infrastructure failure, or an equivalence claim.

Receipts preserve, where applicable: planned, materialized, selected, compiled, executed, activated, and the terminal `MutationResult`. A mutation score exposes the full result distribution. Publishing only `killed / (killed + survived)` without separately reporting every excluded or nonterminal category and its receipts is forbidden. No result disappears to make a score prettier.

### Activation evidence

Every candidate requiring execution carries evidence sufficient to distinguish: mutant selected, mutant compiled or materialized, mutant reached, mutant changed the executed semantic path, and selected witnesses executed. Activation is evidence, never an assumption.

The semantic receipt identifies at least: stable mutation identity, `MutationLane`, source or IR commitment, target owner, target guarantee or invariant, mutation operator or semantic transformation, selected test or proof-set identity, test-set qualification posture, activation posture, baseline posture, terminal `MutationResult`, toolchain/target/feature/platform posture where applicable, work/timeout/deadline posture, proof receipt identity, and freshness posture. No wire layout is frozen here.

### Independent semantic evidence

Every owned semantic operation intended for mutation qualification identifies at least one evidence route that is not merely the production implementation calling itself.

```text
direct reference evaluator
algebraic model
differential implementation
metamorphic relation
hostile boundary example
equivalence proof
```

```text
no independent evidence route          -> no semantic mutation qualification
passing production evaluator against itself -> not an independent oracle
```

This requires no second universal VM. A small evaluator for a numeric operator, a page-composition model, an interval property, or a state-machine reference is often more independent than another full interpreter sharing the same machinery. Building a second full PakVM merely to call it independent is not independence.

### Candidate output boundary

```text
target/muterprater/candidates/    disposable derived material, never durable proof truth
```

Muterprater may propose tests, fixtures, patches, mutation operators, and proof additions. It may not write candidates directly into `src/`, `tests/`, `spec/`, `docs/`, or `companion/`. There is no automatic commit path, and a generated candidate never edits the evidence that judges it. Generation and promotion are two different operations with two different receipts.

### Candidate promotion law

```text
No oracle, no promotion.
No invariant or named proof obligation, no promotion.
No killed mutant or equivalent hostile evidence, no promotion.
No proof receipt, no trust.
```

The promotion receipt records: candidate identity and content commitment, candidate origin and generator, target tracked path, named owner, named guarantee or proof gap, independent evidence route, mutation or hostile-evidence result, selected and executed proof units, review or mechanically predetermined admission posture, freshness, promotion decision, and resulting tracked-content commitment.

Promotion may be mechanically predetermined when the contract leaves no semantic choice; human review is not required as ceremony. Human or architect authority remains required for new semantic ownership, proof weakening, durable or wire changes, compatibility decisions, authority-boundary changes, and genuinely ambiguous concepts. A promoted candidate becomes ordinary tracked source and passes the same gates as hand-authored material.

## Proof-policy anti-weakening (DEC-074)

Owned surfaces: mutation thresholds, waiver logic, equivalent-mutant rules, hostile fixtures, candidate-promotion requirements, proof receipt schema, proof freshness, release-blocking rules, test-selection rules, assurance requirements, and gate qualification. A change declaration names the affected typed surfaces explicitly; the gate never infers a policy change from a filename or keyword.

Every change declares one class:

```text
Strengthening   adds or raises proof obligations, increases hostile coverage,
                reduces unjustified waiver scope, or improves freshness or
                denominator visibility, without silently removing a proof unit
                or terminal disposition

Neutral         preserves denominator, selection meaning, terminal dispositions,
                activation, freshness, blocking posture, promotion requirements,
                and documentary and receipt meaning. Requires parity evidence:
                "refactor" is not automatically Neutral.

Weakening       removes a proof unit, lowers a threshold, widens a waiver, makes
                a required proof optional, reduces hostile coverage, weakens
                freshness, reduces activation requirements, turns a blocking
                failure into a warning, shrinks selected tests without equivalent
                qualification, lets a candidate promote on less evidence, drops
                units from the denominator, or retires an independent oracle
```

An apparently stronger policy that narrows the tested domain or silently removes denominator units is Weakening. An unclassified proof-policy change is refused.

Every Weakening requires: the typed classification, a named DEC or explicit supersession authority, rationale, affected `GuaranteeRef`s or typed guarantee IDs, the old boundary, the new boundary, a replacement-proof or accepted-risk disposition, hostile qualification proving the new boundary is enforced, release-visible disclosure, an auditable change receipt, an effective version or compatibility posture, and rollback or restoration instructions where applicable.

A generic issue link is not decision authority. A commit message is not a weakening receipt. Strengthening and Neutral changes also receive receipts; they require no new DEC unless another architecture law does.

### Self-hostile law

The gate proves its own rule family is active. Weakening the mutation threshold, the activation requirement, the equivalent-candidate witness, the promotion requirements, proof freshness, the release-blocking posture, **or the anti-weakening gate itself** is rejected without the complete Weakening authority package. A proof-policy gate that can be disabled as unclassified cleanup is not an anti-weakening gate.

Bootstrap proves the contract and the detector rule. Future TestPak execution proves the full change-classification workflow. Bootstrap does not understand arbitrary semantic diffs and never claims to.

## Muterprater scope

Muterprater owns only:

```text
mutation identity and operators
mutation planning
mutant materialization
execution requests
classification
survivor explanation
mutation-grounded test promotion
mutation receipts
```

It does not own Repo IR, all commands, release, benchmarks, LSP, documentation, or general proof.

## Audited denominator

Every planned proof unit ends in one semantic terminal of the typed
vocabulary (`spec/proof.rs`, `ProofUnitTerminal`; this list is a generated
projection). Only `Passed` counts green. Attempt outcomes -- not executed,
timed out, infrastructure failed -- belong to the qualification-receipt
algebra and never collapse into these terminals: a row whose attempt died has
no semantic terminal and stays honestly unterminated in the denominator.

<!-- PROOF-TERMINALS:BEGIN generated from spec/proof.rs by bootstrap/project.py; do not edit -->
```text
Passed
Failed
Refused
Unsupported
SkippedWithAuthority
Expired
Superseded
```
<!-- PROOF-TERMINALS:END -->

Zero executed tests, missing fixture activation, stale corpus, or unqualified rule cannot silently count green.

## Gauntlet economy

Fast structural, model, and IR lanes run locally. Expensive fuzz, mutation, cross-platform, and performance envelopes run under named profiles with freshness receipts. Proof strength follows evidence, not where a badge appears in CI.

## Self-host without self-blessing

TestPak may express commands and proof plans as WorldImages and execute them through PakVM. Seedcheck and selected reference models remain independently implemented. No generated manifest may notarize itself as current solely because it can decode its own bytes.

## Command authority

TestPak owns repository commands: inspect, forge, test, mutate, fuzz, bench, prove, context, and seal. Product commands remain in `batpak-cli` over production library contracts.

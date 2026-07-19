---
status: AUTHORITATIVE
contract_id: BP-BOOTSTRAP-1
authority_scope: bootstrap braid, stage order, seedcheck, materialization, and self-hosting
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Bootstrap and Self-Hosting

## Goal

The repository should eventually describe, build, run, attack, and prove itself without becoming its own sole notary.

## Braided loop

```text
signed documents and typed seed facts
→ seedcheck/audit
→ MacBat compiler
→ BatPak contracts and .fbat
→ BatQL + ProgramImage
→ SyncBat PakVM/Bvisor runtime
→ TestPak Repo IR and gauntlet
→ BatPak proof receipts
→ independent seedcheck remains
```

## Stage 0: signed seed

The current bundle, `spec/lib.rs` with the domain-directory module trees under `spec/`, `audit.py`, `freeze.py`, `seedcheck.rs`, and `materialize.rs` are the starting authority. Seedcheck validates shape and stale architecture before production code exists. Materialize consumes the same package facts to publish an isolated Gate-0 workspace candidate outside the signed seed tree.

## Materialization law

`bootstrap/materialize.rs` is a one-time factory with two explicit roots: `materialize --seed <signed-seed-root> --output <candidate-output-root>`. The signed seed and the generated output are separate roots. The materializer reads and validates the seed; it writes only the output; the seed is never modified. It may publish only the exact workspace, package manifests, source doors, and SyncBat plane skeleton the typed owners declare, and the complete plan -- every path and every byte -- is constructed from `spec/bootstrap_output/`, `spec/architecture/`, and `spec/toolchain/` before any filesystem write. It never imports legacy source, invents a package, or writes semantic implementation.

Publication has exactly two successful dispositions: `Created` (the output was absent and a complete staged tree was renamed into place) and `Unchanged` (the output already carried exactly the planned tree and nothing was written). A divergent existing output is refused, never repaired in place. Qualification compares the published candidate against an independent output oracle that reconstructs the expected bytes from the same typed owners, and it proves the seed tree unchanged across the run.

The output is a CANDIDATE tree. Promotion of its exact bytes into tracked source is a later explicit reviewed integration action, owned by Phase 6; it is never a side effect of materialization or qualification. The materializer is not a permanent parallel build system.

The complete expanded plan:

<!-- GATE0-MATERIALIZATION-PLAN:BEGIN generated from spec/bootstrap_output/types.rs; spec/architecture/types.rs; spec/toolchain/types.rs by bootstrap/project.py; do not edit -->
| Path | Artifact family | Derivation |
| --- | --- | --- |
| Cargo.toml | Root / WorkspaceManifest | Gate0RootArtifact::WorkspaceManifest |
| rust-toolchain.toml | Root / RustToolchain | Gate0RootArtifact::RustToolchain |
| justfile | Root / Justfile | Gate0RootArtifact::Justfile |
| .cargo/config.toml | Root / CargoConfig | Gate0RootArtifact::CargoConfig |
| crates/macbat/compiler/Cargo.toml | Package / Manifest | PackageId::MacBatCompiler (Library) |
| crates/macbat/compiler/README.md | Package / Readme | PackageId::MacBatCompiler (Library) |
| crates/macbat/compiler/src/lib.rs | Package / SourceDoor | PackageId::MacBatCompiler (Library) |
| crates/macbat/macros/Cargo.toml | Package / Manifest | PackageId::MacBat (ProcMacroLibrary) |
| crates/macbat/macros/README.md | Package / Readme | PackageId::MacBat (ProcMacroLibrary) |
| crates/macbat/macros/src/lib.rs | Package / SourceDoor | PackageId::MacBat (ProcMacroLibrary) |
| crates/batpak/Cargo.toml | Package / Manifest | PackageId::BatPak (Library) |
| crates/batpak/README.md | Package / Readme | PackageId::BatPak (Library) |
| crates/batpak/src/lib.rs | Package / SourceDoor | PackageId::BatPak (Library) |
| crates/syncbat/Cargo.toml | Package / Manifest | PackageId::SyncBat (Library) |
| crates/syncbat/README.md | Package / Readme | PackageId::SyncBat (Library) |
| crates/syncbat/src/lib.rs | Package / SourceDoor | PackageId::SyncBat (Library) |
| crates/batql/Cargo.toml | Package / Manifest | PackageId::BatQl (Library) |
| crates/batql/README.md | Package / Readme | PackageId::BatQl (Library) |
| crates/batql/src/lib.rs | Package / SourceDoor | PackageId::BatQl (Library) |
| crates/netbat/Cargo.toml | Package / Manifest | PackageId::NetBat (Library) |
| crates/netbat/README.md | Package / Readme | PackageId::NetBat (Library) |
| crates/netbat/src/lib.rs | Package / SourceDoor | PackageId::NetBat (Library) |
| crates/testpak/Cargo.toml | Package / Manifest | PackageId::TestPak (Library) |
| crates/testpak/README.md | Package / Readme | PackageId::TestPak (Library) |
| crates/testpak/src/lib.rs | Package / SourceDoor | PackageId::TestPak (Library) |
| apps/batpak-cli/Cargo.toml | Package / Manifest | PackageId::BatPakCli (Binary) |
| apps/batpak-cli/README.md | Package / Readme | PackageId::BatPakCli (Binary) |
| apps/batpak-cli/src/main.rs | Package / SourceDoor | PackageId::BatPakCli (Binary) |
| examples/Cargo.toml | Package / Manifest | PackageId::BatPakExamples (ExampleBinary) |
| examples/README.md | Package / Readme | PackageId::BatPakExamples (ExampleBinary) |
| examples/src/bin/family_smoke.rs | Package / SourceDoor | PackageId::BatPakExamples (ExampleBinary) |
| crates/syncbat/src/runtime/mod.rs | SyncBat plane / ModuleDoor | SyncBatPlane::Runtime |
| crates/syncbat/src/runtime/types.rs | SyncBat plane / TypesCarrier | SyncBatPlane::Runtime |
| crates/syncbat/src/pakvm/mod.rs | SyncBat plane / ModuleDoor | SyncBatPlane::PakVm |
| crates/syncbat/src/pakvm/types.rs | SyncBat plane / TypesCarrier | SyncBatPlane::PakVm |
| crates/syncbat/src/bvisor/mod.rs | SyncBat plane / ModuleDoor | SyncBatPlane::Bvisor |
| crates/syncbat/src/bvisor/types.rs | SyncBat plane / TypesCarrier | SyncBatPlane::Bvisor |
| crates/syncbat/src/world/mod.rs | SyncBat plane / ModuleDoor | SyncBatPlane::World |
| crates/syncbat/src/world/types.rs | SyncBat plane / TypesCarrier | SyncBatPlane::World |
| crates/syncbat/src/port/mod.rs | SyncBat plane / ModuleDoor | SyncBatPlane::Port |
| crates/syncbat/src/port/types.rs | SyncBat plane / TypesCarrier | SyncBatPlane::Port |
<!-- GATE0-MATERIALIZATION-PLAN:END -->

## First implementation action

The first Gate-0 actions execute this plan in order:

1. Install the pinned Rust toolchain.
2. Compile and run `bootstrap/seedcheck.rs`.
3. Run `bootstrap/audit.py` and `bootstrap/freeze.py --check`.
4. Compile and run `bootstrap/materialize.rs` with explicit `--seed` and `--output` roots to materialize and qualify the isolated Gate-0 candidate. Do not write the skeleton into the signed seed tree.
5. Compile `batpak` and `syncbat` semantic profiles under `no_std + alloc`.
6. Do not copy legacy source.
7. Begin Gate 1 MacBat only after the Gate-0 review packet closes.

## Tier 0 receipt denominator

The required Tier 0 receipt denominator is a generated projection of the harness that enforces it:

<!-- TIER0-RECEIPT-DENOMINATOR:BEGIN generated from spec/bootstrap_qualification/types.rs by bootstrap/project.py; do not edit -->
| Receipt | Artifact policy |
| --- | --- |
| tier0-law-fixtures | FixtureSet |
| tier0-seedcheck | Executable |
| tier0-materialize | ExecutableAndOutputTree |
| tier0-seedcheck-tests | Executable |
| tier0-spec-tests | Executable |
| tier0-spec-rlib | Executable |
<!-- TIER0-RECEIPT-DENOMINATOR:END -->

Every listed receipt is required. A receipt qualifies only when availability, compilation, execution, passing disposition, physical target, and required artifact/source binding all hold.

Membership is enforced by evidence, not prose (5.5E6b). `bootstrap/selftest.py` produces a canonical `qualification.t0` artifact and an evidence bundle from a real gate run; `bootstrap/receiptcheck.rs` independently recomputes every digest and calls the sealed `spec::bootstrap_qualification::verify`, which enforces this exact denominator, per-kind artifact policy, single target, source posture, and the hosted-run requirement for the authoritative target. No Python predicate judges qualification.

This block declares the denominator. It does not claim that the latest run passed. Candidate and cleanroom qualification outcomes belong to exact-SHA, exact-target run receipts, not timeless prose.

## Stage 1: MacBat

Implement the pure compiler kernel and first manual contracts. Generate snapshots, origins, error facts, schema facts, and mutation facts. No repository scans from proc macros.

## Stage 2: BatPak

Implement identities, schemas/codecs, EventFrameV2, `.fbat`, storage ports, receipts, projection/query contracts, authority generations, and compatibility readers.

## Stage 3: TestPak seed

Stand up fixtures, simple models, harnesses, Repo IR, forge, and proof receipts early enough to qualify later runtime work. The Muterprater SemanticIr lane can begin once Contract/Image IR exists.

## Stage 4: BatQL

Implement parser, resolver, type/availability checker, canonical formatter, narration, partial evaluation, bounds/capability analysis, and ProgramImage lowering.

## Stage 5: SyncBat world and PakVM reference

Implement WorldImage linking/validation, port protocol, simple PakVM interpreter, work accounting, and Bvisor admission/attempt skeleton inside the SyncBat package.

## Stage 6: SyncBat runtime and Bvisor recovery

Implement ProcessContract scheduling, selector/cursor mailboxes, TurnId, EffectBatch, checkpoints, delivery contracts, supervision, external effect lifecycle, attempt reconciliation, cooperative and threaded drivers.

## Stage 7: NetBat and CLI

Expose bounded WorldInterface calls and compose product commands in the thin binary adapter.

## Stage 8: optimization and native succession

Add SIDX column tile, selective decode, fused folds, generated mutation shards, and purpose-specific delivery mechanisms only after reference behavior closes.

## Stage 9: repository as a world

Represent TestPak commands and bounded proof/mutation plans as WorldImages where useful. External compiler, fuzz, benchmark, Git, and repository operations remain typed development capabilities with receipts.

## Seedcheck permanence

Seedcheck is never deleted. TestPak may generate its expected manifest or supersede coverage, but an independently implemented small checker remains outside the self-hosted proof loop.

Seedcheck is unrelated to partial evaluation. It checks structure and authority; compilers specialize programs.

## No self-blessing

No component may generate both the claim and its sole expected result. Every bootstrap stage retains at least one independent algorithm, frozen vector, external tool receipt, or hostile fixture.

## Guarantee classification (generated)

Each SEED fact carries its own guarantee classification (DEC-070); `spec/invariants/inventory.rs` is authoritative. The block below is generated from it by `bootstrap/project.py` and independently re-audited; the full derived index across all fact families is the non-normative [Guarantee Graph](GUARANTEE_GRAPH.generated.md).

<!-- SEED-CLASSIFICATION:BEGIN generated from spec/invariants/inventory.rs by bootstrap/project.py; do not edit -->
| GuaranteeId | Kind | Lifetime | Owner | Gates | Witness | Relations |
| --- | --- | --- | --- | --- | --- | --- |
| SEED-ONE-OWNER | SemanticLaw | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py; BP-SELF-EXPLAINING-1 -- contract-id uniqueness scan; the self-explaining-repository law | - |
| SEED-SYNCBAT-ONE-HEARTBEAT | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | seedcheck; audit.py -- SyncBatPlane::ALL plane-file checks in both independent derivations | - |
| SEED-NO-STANDALONE-VM | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | seedcheck; audit.py -- PACKAGES inventory; forbidden-path scan | - |
| SEED-FBAT-CORE | ArchitectureConstraint | Permanent | docs/05_STORAGE_FBAT_AND_TILES.md | G2 | audit.py -- stale-vocabulary scan | - |
| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 | audit.py -- stale-vocabulary scan | - |
| SEED-NO-DUAL-PRODUCT | ArchitectureConstraint | Permanent | docs/02_SYSTEM_MODEL.md | G0 | audit.py; BP-FINAL-AUDIT-1 -- stale-vocabulary scan; final contradiction audit | - |
| SEED-NO-AMBIENT-AUTHORITY | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G5 | LEG-066; BP-GAUNTLET-1 -- capability enforcement | - |
| SEED-SEMANTIC-ZERO-LEAKAGE | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0 | seedcheck; DEC-068 -- production-token scan; the AST gate | DerivesFrom DEC-068 |
| SEED-SYNC-FIRST | ArchitectureConstraint | Permanent | docs/08_SYNCBAT_RUNTIME.md | G0/G5 | LEG-080; seedcheck -- no-tokio scan | Refines LEG-080 |
| SEED-NO-STD-SEMANTIC-PROFILES | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0/G5 | DEC-065 -- qualification matrix | DerivesFrom DEC-065 |
| SEED-CONCEPT-SPINE | ArchitectureConstraint | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G0 | DEC-068; seedcheck -- the AST gate | DerivesFrom DEC-068; DerivesFrom DEC-007 |
| SEED-NO-INLINE-DOMAIN-TYPES | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G1/G3 | DEC-068 -- the AST gate | DerivesFrom DEC-068 |
| SEED-EXPLICIT-EFFECTS | SemanticLaw | Permanent | docs/08_SYNCBAT_RUNTIME.md | G6 | LEG-036; LEG-061; BP-GAUNTLET-1 | - |
| SEED-INDEPENDENT-ORACLE | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G8 | LEG-079; LEG-049 | - |
| SEED-AUDITED-DENOMINATOR | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G9 | LEG-049; BP-LEGACY-INVARIANT-COVERAGE-1; audit.py -- coverage table; declaration parity | - |
| SEED-MUTERPRATER-SCOPE | ArchitectureConstraint | Permanent | docs/12_TESTPAK.md | G3 | DEC-015 | DerivesFrom DEC-015 |
| SEED-BOUNDED-PUSH | SemanticLaw | Permanent | docs/17_DELIVERY_AND_CONCURRENCY.md | G6 | LEG-032; LEG-060 | - |
| SEED-AVAILABILITY-AXES | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G2 | DEC-030; BP-NUMERIC-1 | DerivesFrom DEC-030 |
| SEED-TIME-AXES | SemanticLaw | Permanent | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | G2 | DEC-061; LEG-025; LEG-073 | DerivesFrom DEC-061 |
| SEED-DOC-STATUS | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py -- front-matter check | - |
| SEED-NO-PLACEHOLDER-LAW | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py -- placeholder scan | - |
| SEED-LEGACY-OBLIGATION | SemanticLaw | Permanent | docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md | G0/G9 | BP-LEGACY-INVARIANT-COVERAGE-1; audit.py -- coverage table; LEG declaration parity | - |
| SEED-ECS-NOT-ONTOLOGY | ArchitectureConstraint | Permanent | docs/18_DATA_ORIENTED_ECS.md | G8 | BP-ECS-1; BP-FINAL-AUDIT-1 | - |
| SEED-BVISOR-HONESTY | SemanticLaw | Permanent | docs/09_BVISOR.md | G5 | LEG-042; LEG-043 | - |
| SEED-BATQL-FROZEN | SemanticLaw | Permanent | companion/BATQL_LANGUAGE.md | G4 | BP-BATQL-LANGUAGE-1; DEC-060 -- section 13 language-change law | DerivesFrom DEC-060 |
| SEED-LAYERED-DYNAMIC-VERIFICATION | SemanticLaw | Permanent | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | G0/G3/G9 | DEC-077; BP-DYNAMIC-VERIFICATION-1 -- layered verification axes; no assurance ladder | DerivesFrom DEC-077; Refines SEED-AVAILABILITY-AXES |
| SEED-MODEL-TRACE-SEPARATION | SemanticLaw | Permanent | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | G3/G6/G9 | DEC-077; BP-DYNAMIC-VERIFICATION-1 -- model/trace separation; projection is not the sole oracle | DerivesFrom DEC-077; Refines SEED-INDEPENDENT-ORACLE |
| SEED-RUNTIME-CONFORMANCE-NO-REWRITE | SemanticLaw | Permanent | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | G3/G6/G9 | DEC-078; BP-DYNAMIC-VERIFICATION-1 -- refuse-or-append conformance; never rewrite law, history, or disposition | DerivesFrom DEC-078; Refines DEC-075 |
| SEED-CANDIDATE-NOT-AUTHORITY | SemanticLaw | Permanent | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | G3/G8/G9 | DEC-079; DEC-080; BP-SPROUTING-1 -- candidate noncanonical until qualified; origin confers no authority | DerivesFrom DEC-079; DerivesFrom DEC-080; Refines SEED-ONE-OWNER |
| SEED-BOUNDED-SEARCH | SemanticLaw | Permanent | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | G3/G8/G9 | DEC-081; BP-SPROUTING-1 -- declared search bounds; work receipts | DerivesFrom DEC-081 |
| SEED-HOLDOUT-INDEPENDENCE | SemanticLaw | Permanent | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | G3/G8/G9 | DEC-081; BP-SPROUTING-1 -- evaluation-set roles; holdout cannot reuse search inputs | DerivesFrom DEC-081; Refines SEED-INDEPENDENT-ORACLE |
| SEED-GENERATE-WIDE-QUALIFY-DEEP | SemanticLaw | Permanent | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | G0/G3/G9 | DEC-082; BP-SPROUTING-1 -- dependency-ordered frontier; later green cannot bless earlier red; frozen judge | DerivesFrom DEC-082; Refines DEC-048; Refines DEC-075 |
<!-- SEED-CLASSIFICATION:END -->

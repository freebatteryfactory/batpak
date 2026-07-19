---
status: AUTHORITATIVE
contract_id: BP-SELF-EXPLAINING-1
authority_scope: source navigation, headers, type ledger, dossiers, diagnostics, and context
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Self-Explaining Repository

## Goal

A developer or agent should answer “what exists, who owns it, what may it do, and how is it proven?” from repository structure and generated facts, without memorizing a project-specific ontology.

## Tree as grammar

Every top-level semantic domain is a directory. `mod.rs` is the domain façade and explicitly re-exports the public items; the private `types.rs` exposes the domain's canonical vocabulary; `inventory.rs` carries closed authored rows; verb-named files expose operations; `tests.rs` holds the refusal tests beside the seams they attack. Examples:

```text
src/event/mod.rs
src/event/types.rs
src/event/encode.rs
src/event/replay.rs

src/attempt/mod.rs
src/attempt/types.rs
src/attempt/admit.rs
src/attempt/reconcile.rs
```

The reader algorithm: open the domain directory; read `mod.rs` for the boundary and what is deliberately public; read `types.rs` for the domain vocabulary; read the verb file for the operation in question; read `tests.rs` for what the domain refuses. `mod.rs` is BatPak house style by ruling — directory self-containment for an agent-facing tree — not a claim of the universally idiomatic modern Rust layout.

Established computer-science names are preferred. Branded names remain only where they identify real BatPak products or formats.

## File header contract

Structured authority headers are RISK-TRIGGERED, not universal (5.5E1
anti-ceremony ruling). They are required for semantic-owner modules, kernels,
ports/adapters, unsafe files, concurrency mechanisms, durable codecs, and
proof-policy implementation:

```text
OWNS
READS
WRITES
FORBIDS
BOUNDS
PROOF
```

An ordinary implementation file does not repeat an architecture questionnaire;
concise module documentation suffices. This is concise structured module
documentation, not a duplicate architecture essay.

## Generated Type Ledger

Two indexes with different authority (5.5E1 ruling). The SEMANTIC-OWNER index
is normative and covers types that cross at least one real boundary: public
package, durable or wire, trust or authority, proof or receipt, unsafe or
concurrency, cross-package lowering, or compatibility. Each such type carries
owner, identity class, consumers, durable/wire status, lifecycle, and proof
owner, and the index rejects duplicate semantic owners and inline domain
types. The NAVIGATION index may discover every type and file for search, but
its rows are non-normative and carry no per-row lifecycle, proof owner,
receipt, or denominator obligation — "every named type gets a proof owner" is
denominator inflation, and this contract refuses it.

## File dossiers

Dossiers are RISK-TRIGGERED, never size-triggered or universal: unsafe or
concurrency mechanisms, durable codecs, kernels, and trust boundaries earn
generated dossiers with concept, public surface, dependencies,
unsafe/allocator/locking assumptions, complexity, mutation seams, fixtures,
and receipts. An ordinary large file does not.

## Projection law (5.5E1)

A closed authority-bearing inventory with a typed owner is projected for
humans and agents; it is never independently re-authored in prose. This is
the standing law behind the generated operator, gate, plane, crossing, seal,
terminal, reconciliation, and classification blocks, and it applies to every
future closed inventory. Typing is warranted only when all three hold: the
vocabulary is closed or finite; it affects authority, admission,
compatibility, proof, generation, or release; and an invalid value should be
unconstructable. Speculative armor is debt.

## Typed diagnostics

Diagnostics answer the relevant 5W1H dimensions:

```text
WHO/WHAT failed
WHERE the owner/source lives
WHEN/source cut involved
HOW to reach legal state
WHY the rule exists
```

They carry stable codes and structured actions.

## Generated navigation

```text
testpak inspect owner <symbol>
testpak inspect file <path>
testpak inspect contract <id>
testpak inspect proof <id>
testpak inspect dependency <package>
testpak inspect legacy <LEG-id>
testpak inspect decision <DEC-id>
```

MCP exposes the same typed queries.

## No hidden coupling

TestPak rejects forbidden dependency direction, duplicate validators, stringly cross-package identities, unowned files/items, dead generated artifacts, docs pointing to retired paths, and public items without proof paths.

## Context packet

`testpak context` is generated from Repo IR and receipts. It includes active authority, stale evidence warnings, obligation status, dirty tree, and next legal actions. It does not ask the next agent to reconstruct project history from chat logs.

## Generated-view registry

`spec/generated_views/registry.rs` owns which repository-generated views exist, their authority source, target selector, surface form, marker, and generator. The registry owns generation metadata only. It never owns or completes the semantic facts a view renders.

An embedded block inherits no authority from its containing document. A standalone generated file is a derived view. Corpus epoch frontmatter states corpus membership, not semantic truth. A future generated view must enter the registry before it may land.

<!-- GENERATED-VIEW-REGISTRY:BEGIN generated from spec/generated_views/registry.rs by bootstrap/project.py; do not edit -->
| View | Surface | Authority source(s) | Target | Marker | Generator |
| --- | --- | --- | --- | --- | --- |
| OperatorsCatalog | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | companion/BATQL_LANGUAGE.md | OPERATORS-CATALOG | bootstrap/project.py |
| OperatorsSurfaces | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | companion/BATQL_LANGUAGE.md | OPERATORS-SURFACES | bootstrap/project.py |
| OperatorsGrammar | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | companion/BATQL_LANGUAGE.md | OPERATORS-GRAMMAR | bootstrap/project.py |
| OperatorsProjection | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | companion/BATQL_LANGUAGE.md | OPERATORS-PROJECTION | bootstrap/project.py |
| OperatorsTyping | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | companion/BATQL_LANGUAGE.md | OPERATORS-TYPING | bootstrap/project.py |
| OperatorsNumeric | embedded-block | spec/operators/inventory.rs; spec/operators/types.rs | docs/37_NUMERIC_SEMANTICS_AND_AUTHORITY.md | OPERATORS-NUMERIC | bootstrap/project.py |
| SeedClassification | embedded-block | spec/invariants/inventory.rs | docs/23_BOOTSTRAP_AND_SELF_HOSTING.md | SEED-CLASSIFICATION | bootstrap/project.py |
| GuaranteeGraph | standalone-file | spec/invariants/inventory.rs; spec/legacy_obligations/inventory.rs; spec/dispositions/inventory.rs; spec/architecture/inventory.rs; spec/architecture/types.rs; spec/guarantees/policy.rs; spec/gates/inventory.rs | docs/GUARANTEE_GRAPH.generated.md | - | bootstrap/project.py |
| PakVmSemanticIsa | embedded-block | spec/pakvm_isa/nodes.rs; spec/pakvm_isa/policies.rs; spec/pakvm_isa/types.rs | docs/07_PAKVM_ISA.md | PAKVM-SEMANTIC-ISA | bootstrap/project.py |
| PakVmSignatures | embedded-block | spec/pakvm_isa/nodes.rs | docs/07_PAKVM_ISA.md | PAKVM-SIGNATURES | bootstrap/project.py |
| SyncBatPlaneOwnership | embedded-block | spec/architecture/types.rs | docs/08_SYNCBAT_RUNTIME.md | SYNCBAT-PLANE-OWNERSHIP | bootstrap/project.py |
| SyncBatAuthorities | embedded-block | spec/syncbat_firewall/types.rs | docs/08_SYNCBAT_RUNTIME.md | SYNCBAT-AUTHORITIES | bootstrap/project.py |
| SyncBatCrossings | embedded-block | spec/syncbat_firewall/inventory.rs | docs/08_SYNCBAT_RUNTIME.md | SYNCBAT-CROSSINGS | bootstrap/project.py |
| ReconciliationCoordinates | embedded-block | spec/reconciliation/inventory.rs | docs/02_SYSTEM_MODEL.md | RECONCILIATION-COORDINATES | bootstrap/project.py |
| ReconciliationRetry | embedded-block | spec/reconciliation/types.rs | docs/02_SYSTEM_MODEL.md | RECONCILIATION-RETRY | bootstrap/project.py |
| ReleaseSeal | embedded-block | spec/release/inventory.rs | docs/36_PUBLIC_API_CI_AND_RELEASE.md | RELEASE-SEAL | bootstrap/project.py |
| ProofTerminals | embedded-block | spec/proof/types.rs | docs/12_TESTPAK.md | PROOF-TERMINALS | bootstrap/project.py |
| StaleVocabulary | embedded-block | spec/dispositions/inventory.rs | docs/29_STATUS_AND_SUPERSESSION.md; docs/33_AGENT_FINISH_LINE_CHECKLIST.md | STALE-VOCAB | bootstrap/project.py |
| GateInventory | embedded-block | spec/gates/inventory.rs | docs/25_IMPLEMENTATION_GATES.md | GATE-INVENTORY | bootstrap/project.py |
| ContractKinds | embedded-block | spec/contracts/types.rs | docs/06_MACBAT.md | CONTRACT-KINDS | bootstrap/project.py |
| RustToolchain | standalone-file | spec/toolchain/types.rs | rust-toolchain.toml | - | bootstrap/project.py |
| IdentityCatalog | embedded-block | spec/identities/types.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | IDENTITY-CATALOG | bootstrap/project.py |
| GenerationCatalog | embedded-block | spec/identities/types.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | GENERATION-CATALOG | bootstrap/project.py |
| BindingCatalog | embedded-block | spec/identities/types.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | BINDING-CATALOG | bootstrap/project.py |
| VersionCatalog | embedded-block | spec/identities/types.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | VERSION-CATALOG | bootstrap/project.py |
| IdentityResidue | embedded-block | spec/identities/inventory.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | IDENTITY-RESIDUE | bootstrap/project.py |
| ProductCommands | embedded-block | spec/commands/types.rs | docs/26_COMMAND_PLANE.md | PRODUCT-COMMANDS | bootstrap/project.py |
| TestPakCommands | embedded-block | spec/commands/types.rs | docs/26_COMMAND_PLANE.md | TESTPAK-COMMANDS | bootstrap/project.py |
| BatQlSourceModes | embedded-block | spec/commands/types.rs | companion/BATQL_LANGUAGE.md | BATQL-SOURCE-MODES | bootstrap/project.py |
| MutationLanes | embedded-block | spec/mutation/types.rs | docs/12_TESTPAK.md | MUTATION-LANES | bootstrap/project.py |
| MutationResults | embedded-block | spec/mutation/types.rs | docs/12_TESTPAK.md | MUTATION-RESULTS | bootstrap/project.py |
| PromotionRequirements | embedded-block | spec/promotion/types.rs | docs/12_TESTPAK.md | PROMOTION-REQUIREMENTS | bootstrap/project.py |
| CompilerAssumptionKinds | embedded-block | spec/compiler_assumptions/types.rs | docs/19_SECURITY_MODEL.md | COMPILER-ASSUMPTION-KINDS | bootstrap/project.py |
| CorpusReconciliationEpoch | embedded-block | spec/corpus/types.rs | docs/00_CONSTITUTION.md | CORPUS-RECONCILIATION-EPOCH | bootstrap/project.py |
| CorpusEpochMembership | corpus-frontmatter | spec/corpus/types.rs | eligible markdown corpus | - | bootstrap/project.py |
| PackageInventory | embedded-block | spec/architecture/inventory.rs; spec/architecture/types.rs | README.md; docs/03_REPOSITORY_AND_PACKAGES.md | PACKAGE-INVENTORY | bootstrap/project.py |
| PackageEdges | embedded-block | spec/architecture/inventory.rs; spec/architecture/types.rs | docs/03_REPOSITORY_AND_PACKAGES.md | PACKAGE-EDGES | bootstrap/project.py |
| QualificationProfiles | embedded-block | spec/architecture/inventory.rs; spec/architecture/types.rs | docs/03_REPOSITORY_AND_PACKAGES.md | QUALIFICATION-PROFILES | bootstrap/project.py |
| BundleInventory | embedded-block | spec/architecture/inventory.rs; spec/invariants/inventory.rs; spec/dispositions/inventory.rs; spec/legacy_obligations/inventory.rs; spec/legacy_invariant_coverage/inventory.rs; spec/operators/inventory.rs; spec/generated_views/registry.rs | docs/28_SELF_EXPLAINING_REPOSITORY.md | BUNDLE-INVENTORY | bootstrap/project.py |
| Tier0ReceiptDenominator | embedded-block | spec/bootstrap_qualification/types.rs | docs/23_BOOTSTRAP_AND_SELF_HOSTING.md | TIER0-RECEIPT-DENOMINATOR | bootstrap/project.py |
| DecisionLedger | embedded-block | spec/dispositions/inventory.rs; spec/dispositions/types.rs | docs/30_DECISION_AND_REJECTION_LEDGER.md | DECISION-LEDGER | bootstrap/project.py |
| LegacyInvariantCoverage | embedded-block | spec/legacy_invariant_coverage/inventory.rs | docs/34_LEGACY_INVARIANT_COVERAGE.md | LEGACY-INVARIANT-COVERAGE | bootstrap/project.py |
| LegacyObligationLedger | embedded-block | spec/legacy_obligations/inventory.rs; spec/proof/inventory.rs | docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md | LEGACY-OBLIGATION-LEDGER | bootstrap/project.py |
| GauntletProofRelations | embedded-block | spec/proof/inventory.rs | docs/24_GAUNTLET.md | PROOF-RELATIONS | bootstrap/project.py |
| SystemModelProofRequirements | embedded-block | spec/proof/inventory.rs | docs/02_SYSTEM_MODEL.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| RepositoryProofRequirements | embedded-block | spec/proof/inventory.rs | docs/03_REPOSITORY_AND_PACKAGES.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| TypeSystemProofRequirements | embedded-block | spec/proof/inventory.rs | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| StorageProofRequirements | embedded-block | spec/proof/inventory.rs | docs/05_STORAGE_FBAT_AND_TILES.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| SyncBatProofRequirements | embedded-block | spec/proof/inventory.rs | docs/08_SYNCBAT_RUNTIME.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| BvisorProofRequirements | embedded-block | spec/proof/inventory.rs | docs/09_BVISOR.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| WorldPortsProofRequirements | embedded-block | spec/proof/inventory.rs | docs/10_WORLD_IMAGES_AND_PORTS.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| TestPakProofRequirements | embedded-block | spec/proof/inventory.rs | docs/12_TESTPAK.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| IdentityTimeProofRequirements | embedded-block | spec/proof/inventory.rs | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| MigrationProofRequirements | embedded-block | spec/proof/inventory.rs | docs/22_MIGRATION_AND_CUTOVER.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| SecretAuthorityProofRequirements | embedded-block | spec/proof/inventory.rs | docs/35_CRYPTO_AND_SECRET_AUTHORITY.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| Gate0MaterializationPlan | embedded-block | spec/bootstrap_output/types.rs; spec/architecture/types.rs; spec/toolchain/types.rs | docs/23_BOOTSTRAP_AND_SELF_HOSTING.md | GATE0-MATERIALIZATION-PLAN | bootstrap/project.py |
| VerificationPlans | embedded-block | spec/proof/inventory.rs; spec/verification/types.rs | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | VERIFICATION-PLANS | bootstrap/project.py |
| SproutingVocabulary | embedded-block | spec/sprouting/types.rs | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | SPROUTING-VOCABULARY | bootstrap/project.py |
| SpecializedPlanCandidatePolicy | embedded-block | spec/sprouting/inventory.rs | docs/07_PAKVM_ISA.md | SPECIALIZED-PLAN-CANDIDATE-POLICY | bootstrap/project.py |
| SproutingProofRequirements | embedded-block | spec/proof/inventory.rs | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | PROOF-REQUIREMENTS | bootstrap/project.py |
| DynamicVerificationVocabulary | embedded-block | spec/verification/types.rs | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | DYNAMIC-VERIFICATION-VOCABULARY | bootstrap/project.py |
| VerificationRuntimeMatrix | embedded-block | spec/verification/types.rs | docs/38_DYNAMIC_VERIFICATION_AND_CONFORMANCE.md | VERIFICATION-RUNTIME-MATRIX | bootstrap/project.py |
| SproutingPromotionMatrix | embedded-block | spec/promotion/types.rs; spec/sprouting/inventory.rs | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | SPROUTING-PROMOTION-MATRIX | bootstrap/project.py |
| ReleaseSealMatrix | embedded-block | spec/release/types.rs; spec/release/inventory.rs | docs/36_PUBLIC_API_CI_AND_RELEASE.md | RELEASE-SEAL-MATRIX | bootstrap/project.py |
| CampaignClosureGraph | embedded-block | spec/campaign/types.rs | docs/39_SPROUTING_NURSERY_AND_PROMOTION.md | CAMPAIGN-CLOSURE-GRAPH | bootstrap/project.py |
| GeneratedViewRegistry | embedded-block | spec/generated_views/registry.rs | docs/28_SELF_EXPLAINING_REPOSITORY.md | GENERATED-VIEW-REGISTRY | bootstrap/project.py |
| SpecModuleCatalog | embedded-block | spec/lib.rs | spec/README.md | SPEC-MODULE-CATALOG | bootstrap/project.py |
| BootstrapToolCatalog | embedded-block | spec/guarantees/types.rs | bootstrap/README.md | BOOTSTRAP-TOOL-CATALOG | bootstrap/project.py |
<!-- GENERATED-VIEW-REGISTRY:END -->

## Bundle inventory

The repository describes its own size the same way it describes everything else: by projection. Every row below is a generated derivation of a typed denominator or the current tracked tree — never an authored count — and the exact numbers that affect machine law are derived from `spec/`, so human prose can never override them.

<!-- BUNDLE-INVENTORY:BEGIN generated from spec/architecture/inventory.rs; spec/invariants/inventory.rs; spec/dispositions/inventory.rs; spec/legacy_obligations/inventory.rs; spec/legacy_invariant_coverage/inventory.rs; spec/operators/inventory.rs; spec/generated_views/registry.rs by bootstrap/project.py; do not edit -->
| Metric | Count | Derivation |
| --- | --- | --- |
| numbered architecture documents | 40 | current tracked docs matching docs/[0-9][0-9]_*.md |
| Markdown documents | 46 | current eligible Markdown corpus |
| Cargo packages | 9 | PackageId::ALL with PACKAGES parity |
| package edges | 19 | EDGES |
| qualification profiles | 6 | QUALIFICATION_PROFILES |
| SEED guarantees | 32 | spec/invariants/inventory.rs SEED inventory |
| decision rows | 82 | spec/dispositions/inventory.rs DECISIONS |
| legacy semantic obligations | 87 | spec/legacy_obligations/inventory.rs OBLIGATIONS |
| legacy invariant declarations | 115 | SOURCE_INVARIANT_IDS with COVERAGE parity |
| BatQL operators | 13 | OperatorId::ALL with OPERATORS parity |
| registered generated views | 68 | GeneratedView::ALL |
| static generated target instances | 69 | expansion of every Static registry target |
| corpus-frontmatter bindings | 46 | the eligible Markdown corpus reached by CorpusEpochMembership |
<!-- BUNDLE-INVENTORY:END -->

## Exact mirrors and semantic owners

Exact mirrors: when a typed owner already contains every field in a documentary table, the table is generated. An equivalence checker babysitting two editable copies is not convergence.

Complementary semantic ownership: an authored document may remain the canonical owner of meaning that does not exist in the typed row. A generator may not fill that missing meaning from conventions, neighbouring rows, or Python defaults.

Subset projections: a cross-document subset list is a generated view only after a typed relation says which rows belong to which target. Parsing prose headings is not an authority relation.

Historical narrative: historical counts and migration explanations may remain authored when they are explicitly historical, bind a frozen source, and cannot be read as the current inventory.

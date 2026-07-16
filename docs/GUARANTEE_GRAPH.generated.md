---
status: GENERATED
authority_scope: derived structural index only
generated_by: bootstrap/project.py
generated_from: typed SEED, LEG, DEC, architecture, and qualification fact families
do_not_edit: true
---

# Guarantee Graph (generated)

Derived structural index across the authored fact families (DEC-070). It is not
normative: every law resolves to its owning fact. Displayed law text is copied for
navigation only; the owning fact is authoritative. Do not edit.

## Source-family inventory

| Family | Nodes |
| --- | --- |
| SEED | 25 |
| LEG | 87 |
| DEC | 75 |
| ARCH | 9 |
| QUAL | 6 |

## Totals

```text
nodes: 202
edges: 9
unresolved references: 0
```

## Counts by kind

| GuaranteeKind | Nodes |
| --- | --- |
| ArchitectureConstraint | 19 |
| BootstrapAssertion | 2 |
| Decision | 75 |
| LegacyObligation | 87 |
| QualificationRequirement | 6 |
| SemanticLaw | 13 |

## Counts by lifetime

| GuaranteeLifetime | Nodes |
| --- | --- |
| HistoricalCoverageOnly | 4 |
| Permanent | 103 |
| UntilCompatibilityExpiry | 8 |
| UntilGate | 87 |

## Active versus closed

```text
active (gating): 198
closed evidence: 0
historical coverage only: 4
```

## Classified SEED

| GuaranteeId | Kind | Lifetime | Owner | Gates | Witness | Relations |
| --- | --- | --- | --- | --- | --- | --- |
| SEED-ONE-OWNER | SemanticLaw | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py; BP-SELF-EXPLAINING-1 -- contract-id uniqueness scan; the self-explaining-repository law | - |
| SEED-SYNCBAT-ONE-HEARTBEAT | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | seedcheck; audit.py -- SYNCBAT_REQUIRED_PLANES plane checks in both independent derivations | - |
| SEED-NO-STANDALONE-VM | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | seedcheck; audit.py -- PACKAGES inventory; forbidden-path scan | - |
| SEED-FBAT-CORE | ArchitectureConstraint | Permanent | docs/05_STORAGE_FBAT_AND_TILES.md | G2 | audit.py -- stale-vocabulary scan | - |
| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 | audit.py -- stale-vocabulary scan | - |
| SEED-NO-DUAL-PRODUCT | ArchitectureConstraint | Permanent | docs/02_SYSTEM_MODEL.md | G0 | audit.py; BP-FINAL-AUDIT-1 -- stale-vocabulary scan; final contradiction audit | - |
| SEED-NO-AMBIENT-AUTHORITY | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G5 | LEG-066; BP-GAUNTLET-1 -- capability enforcement | - |
| SEED-SEMANTIC-ZERO-LEAKAGE | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0 | seedcheck; DEC-068 -- production-token scan; the AST gate | DerivesFrom DEC-068 |
| SEED-SYNC-FIRST | ArchitectureConstraint | Permanent | docs/08_SYNCBAT_RUNTIME.md | G0/G5 | LEG-080; seedcheck -- no-tokio scan | Refines LEG-080 |
| SEED-NO-STD-SEMANTIC-PROFILES | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0/G5 | DEC-065 -- qualification matrix | DerivesFrom DEC-065 |
| SEED-CONCEPT-SPINE | ArchitectureConstraint | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G0 | DEC-068; seedcheck -- the AST gate | DerivesFrom DEC-068 |
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

## Nodes

Gate posture schedules qualification. Qualification target names the environment
a requirement is qualified against. They are different dimensions: a target never
appears under gate posture. `-` means the dimension does not apply to that family,
which is distinct from missing data -- a field with no authored source fails
admission and never reaches this table.

| GuaranteeId | Family | Kind | Lifetime | DecisionClass | Owner | GatePosture | QualificationTarget | WitnessPosture |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SEED-AUDITED-DENOMINATOR | SEED | SemanticLaw | Permanent | - | docs/24_GAUNTLET.md | G3/G9 | - | RowDeclared |
| SEED-AVAILABILITY-AXES | SEED | SemanticLaw | Permanent | - | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G2 | - | RowDeclared |
| SEED-BATQL-FROZEN | SEED | SemanticLaw | Permanent | - | companion/BATQL_LANGUAGE.md | G4 | - | RowDeclared |
| SEED-BOUNDED-PUSH | SEED | SemanticLaw | Permanent | - | docs/17_DELIVERY_AND_CONCURRENCY.md | G6 | - | RowDeclared |
| SEED-BVISOR-HONESTY | SEED | SemanticLaw | Permanent | - | docs/09_BVISOR.md | G5 | - | RowDeclared |
| SEED-CONCEPT-SPINE | SEED | ArchitectureConstraint | Permanent | - | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G0 | - | RowDeclared |
| SEED-DOC-STATUS | SEED | BootstrapAssertion | Permanent | - | docs/00_CONSTITUTION.md | G0 | - | RowDeclared |
| SEED-ECS-NOT-ONTOLOGY | SEED | ArchitectureConstraint | Permanent | - | docs/18_DATA_ORIENTED_ECS.md | G8 | - | RowDeclared |
| SEED-EXPLICIT-EFFECTS | SEED | SemanticLaw | Permanent | - | docs/08_SYNCBAT_RUNTIME.md | G6 | - | RowDeclared |
| SEED-FBAT-CORE | SEED | ArchitectureConstraint | Permanent | - | docs/05_STORAGE_FBAT_AND_TILES.md | G2 | - | RowDeclared |
| SEED-INDEPENDENT-ORACLE | SEED | SemanticLaw | Permanent | - | docs/24_GAUNTLET.md | G3/G8 | - | RowDeclared |
| SEED-LEGACY-OBLIGATION | SEED | SemanticLaw | Permanent | - | docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md | G0/G9 | - | RowDeclared |
| SEED-MUTERPRATER-SCOPE | SEED | ArchitectureConstraint | Permanent | - | docs/12_TESTPAK.md | G3 | - | RowDeclared |
| SEED-NO-AMBIENT-AUTHORITY | SEED | SemanticLaw | Permanent | - | docs/07_PAKVM_ISA.md | G5 | - | RowDeclared |
| SEED-NO-DUAL-PRODUCT | SEED | ArchitectureConstraint | Permanent | - | docs/02_SYSTEM_MODEL.md | G0 | - | RowDeclared |
| SEED-NO-INLINE-DOMAIN-TYPES | SEED | SemanticLaw | Permanent | - | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G1/G3 | - | RowDeclared |
| SEED-NO-PLACEHOLDER-LAW | SEED | BootstrapAssertion | Permanent | - | docs/00_CONSTITUTION.md | G0 | - | RowDeclared |
| SEED-NO-STANDALONE-VM | SEED | ArchitectureConstraint | Permanent | - | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | - | RowDeclared |
| SEED-NO-STD-SEMANTIC-PROFILES | SEED | ArchitectureConstraint | Permanent | - | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0/G5 | - | RowDeclared |
| SEED-ONE-OWNER | SEED | SemanticLaw | Permanent | - | docs/00_CONSTITUTION.md | G0 | - | RowDeclared |
| SEED-PAKVM-NAME | SEED | SemanticLaw | Permanent | - | docs/07_PAKVM_ISA.md | G0/G5 | - | RowDeclared |
| SEED-SEMANTIC-ZERO-LEAKAGE | SEED | ArchitectureConstraint | Permanent | - | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0 | - | RowDeclared |
| SEED-SYNC-FIRST | SEED | ArchitectureConstraint | Permanent | - | docs/08_SYNCBAT_RUNTIME.md | G0/G5 | - | RowDeclared |
| SEED-SYNCBAT-ONE-HEARTBEAT | SEED | ArchitectureConstraint | Permanent | - | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | - | RowDeclared |
| SEED-TIME-AXES | SEED | SemanticLaw | Permanent | - | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | G2 | - | RowDeclared |
| LEG-001 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-002 | LEG | LegacyObligation | UntilCompatibilityExpiry | - | batpak::event | G2 | - | NoFamilyWitness |
| LEG-003 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-004 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-005 | LEG | LegacyObligation | UntilGate | - | batpak::frontier | G2 | - | NoFamilyWitness |
| LEG-006 | LEG | LegacyObligation | UntilGate | - | batpak::visibility | G2 | - | NoFamilyWitness |
| LEG-007 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-008 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-009 | LEG | LegacyObligation | UntilGate | - | batpak::compaction | G2 | - | NoFamilyWitness |
| LEG-010 | LEG | LegacyObligation | UntilGate | - | batpak::recovery | G2/G3 | - | NoFamilyWitness |
| LEG-011 | LEG | LegacyObligation | UntilGate | - | batpak::lifecycle | G2 | - | NoFamilyWitness |
| LEG-012 | LEG | LegacyObligation | UntilGate | - | batpak::lifecycle | G2 | - | NoFamilyWitness |
| LEG-013 | LEG | LegacyObligation | UntilGate | - | batpak::lifecycle | G2 | - | NoFamilyWitness |
| LEG-014 | LEG | LegacyObligation | UntilGate | - | batpak::recovery | G2 | - | NoFamilyWitness |
| LEG-015 | LEG | LegacyObligation | UntilGate | - | batpak::secret | G2 | - | NoFamilyWitness |
| LEG-016 | LEG | LegacyObligation | UntilGate | - | batpak::storage_port | G2/G3 | - | NoFamilyWitness |
| LEG-017 | LEG | LegacyObligation | UntilGate | - | batpak::tile | G2/G8 | - | NoFamilyWitness |
| LEG-018 | LEG | LegacyObligation | UntilGate | - | batpak::tile | G8 | - | NoFamilyWitness |
| LEG-019 | LEG | LegacyObligation | UntilGate | - | batpak::integrity | G2/G3 | - | NoFamilyWitness |
| LEG-020 | LEG | LegacyObligation | UntilGate | - | testpak::bench | G3/G8 | - | NoFamilyWitness |
| LEG-021 | LEG | LegacyObligation | UntilGate | - | batpak::coordinate | G2 | - | NoFamilyWitness |
| LEG-022 | LEG | LegacyObligation | UntilGate | - | batpak::navigation | G2 | - | NoFamilyWitness |
| LEG-023 | LEG | LegacyObligation | UntilGate | - | batpak::integrity | G2/G3 | - | NoFamilyWitness |
| LEG-024 | LEG | LegacyObligation | UntilGate | - | batpak::event | G1/G2 | - | NoFamilyWitness |
| LEG-025 | LEG | LegacyObligation | UntilGate | - | batpak::time | G2 | - | NoFamilyWitness |
| LEG-026 | LEG | LegacyObligation | UntilGate | - | batpak::projection | G2 | - | NoFamilyWitness |
| LEG-027 | LEG | LegacyObligation | UntilCompatibilityExpiry | - | batpak::path | G2 | - | NoFamilyWitness |
| LEG-028 | LEG | LegacyObligation | UntilGate | - | batpak::projection | G2 | - | NoFamilyWitness |
| LEG-029 | LEG | LegacyObligation | UntilGate | - | batpak::projection_query | G2/G6/G7 | - | NoFamilyWitness |
| LEG-030 | LEG | LegacyObligation | UntilGate | - | batpak::projection | G2/G3 | - | NoFamilyWitness |
| LEG-031 | LEG | LegacyObligation | UntilGate | - | batpak::projection | G8 | - | NoFamilyWitness |
| LEG-032 | LEG | LegacyObligation | UntilGate | - | batpak::delivery | G2/G6 | - | NoFamilyWitness |
| LEG-033 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6 | - | NoFamilyWitness |
| LEG-034 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6 | - | NoFamilyWitness |
| LEG-035 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G3/G6 | - | NoFamilyWitness |
| LEG-036 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6 | - | NoFamilyWitness |
| LEG-037 | LEG | LegacyObligation | UntilGate | - | batpak::effect | G2/G6 | - | NoFamilyWitness |
| LEG-038 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G5/G6 | - | NoFamilyWitness |
| LEG-039 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6 | - | NoFamilyWitness |
| LEG-040 | LEG | LegacyObligation | UntilCompatibilityExpiry | - | batpak::schema | G2/G5 | - | NoFamilyWitness |
| LEG-041 | LEG | LegacyObligation | UntilGate | - | batpak::world | G5 | - | NoFamilyWitness |
| LEG-042 | LEG | LegacyObligation | UntilGate | - | syncbat::bvisor | G5 | - | NoFamilyWitness |
| LEG-043 | LEG | LegacyObligation | UntilGate | - | syncbat::bvisor | G5 | - | NoFamilyWitness |
| LEG-044 | LEG | LegacyObligation | UntilGate | - | syncbat::pakvm | G3/G5 | - | NoFamilyWitness |
| LEG-045 | LEG | LegacyObligation | UntilGate | - | netbat | G7 | - | NoFamilyWitness |
| LEG-046 | LEG | LegacyObligation | UntilGate | - | macbat-compiler | G1 | - | NoFamilyWitness |
| LEG-047 | LEG | LegacyObligation | UntilGate | - | macbat-compiler | G1/G2 | - | NoFamilyWitness |
| LEG-048 | LEG | LegacyObligation | UntilGate | - | testpak::repo | G3 | - | NoFamilyWitness |
| LEG-049 | LEG | LegacyObligation | UntilGate | - | testpak::gauntlet | G3/G9 | - | NoFamilyWitness |
| LEG-050 | LEG | LegacyObligation | UntilGate | - | batpak::event | G2/G3 | - | NoFamilyWitness |
| LEG-051 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-052 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G3 | - | NoFamilyWitness |
| LEG-053 | LEG | LegacyObligation | UntilCompatibilityExpiry | - | batpak::compatibility | G2/G3 | - | NoFamilyWitness |
| LEG-054 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G2/G6 | - | NoFamilyWitness |
| LEG-055 | LEG | LegacyObligation | UntilGate | - | batpak::time | G2/G3 | - | NoFamilyWitness |
| LEG-056 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G8 | - | NoFamilyWitness |
| LEG-057 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G6 | - | NoFamilyWitness |
| LEG-058 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6 | - | NoFamilyWitness |
| LEG-059 | LEG | LegacyObligation | UntilGate | - | batpak::receipt | G2/G6 | - | NoFamilyWitness |
| LEG-060 | LEG | LegacyObligation | UntilGate | - | syncbat::runtime | G6/G7 | - | NoFamilyWitness |
| LEG-061 | LEG | LegacyObligation | UntilGate | - | syncbat::bvisor | G5/G6 | - | NoFamilyWitness |
| LEG-062 | LEG | LegacyObligation | UntilGate | - | syncbat::world | G6 | - | NoFamilyWitness |
| LEG-063 | LEG | LegacyObligation | UntilGate | - | batpak::navigation | G2/G7 | - | NoFamilyWitness |
| LEG-064 | LEG | LegacyObligation | UntilCompatibilityExpiry | - | testpak::oracle | G2/G7 | - | NoFamilyWitness |
| LEG-065 | LEG | LegacyObligation | UntilGate | - | testpak::repo | G0/G3 | - | NoFamilyWitness |
| LEG-066 | LEG | LegacyObligation | UntilGate | - | batpak::port | G2/G5/G7 | - | NoFamilyWitness |
| LEG-067 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G3 | - | NoFamilyWitness |
| LEG-068 | LEG | LegacyObligation | UntilGate | - | batpak::tile | G2/G8 | - | NoFamilyWitness |
| LEG-069 | LEG | LegacyObligation | UntilGate | - | batpak::time | G2/G3 | - | NoFamilyWitness |
| LEG-070 | LEG | LegacyObligation | UntilGate | - | batpak::frontier | G2/G3 | - | NoFamilyWitness |
| LEG-071 | LEG | LegacyObligation | UntilGate | - | batpak::frontier | G2/G3 | - | NoFamilyWitness |
| LEG-072 | LEG | LegacyObligation | UntilGate | - | batpak::frontier | G2/G6 | - | NoFamilyWitness |
| LEG-073 | LEG | LegacyObligation | UntilGate | - | batpak::time | G2/G3 | - | NoFamilyWitness |
| LEG-074 | LEG | LegacyObligation | UntilGate | - | batpak::lifecycle | G2/G3 | - | NoFamilyWitness |
| LEG-075 | LEG | LegacyObligation | UntilGate | - | batpak::lifecycle | G2/G3 | - | NoFamilyWitness |
| LEG-076 | LEG | LegacyObligation | UntilGate | - | macbat-compiler | G1 | - | NoFamilyWitness |
| LEG-077 | LEG | LegacyObligation | UntilGate | - | batpak::event | G2/G3 | - | NoFamilyWitness |
| LEG-078 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G8 | - | NoFamilyWitness |
| LEG-079 | LEG | LegacyObligation | UntilGate | - | testpak::oracle | G3/G8 | - | NoFamilyWitness |
| LEG-080 | LEG | LegacyObligation | UntilGate | - | syncbat::port | G0/G5/G7 | - | NoFamilyWitness |
| LEG-081 | LEG | LegacyObligation | UntilGate | - | batpak::secret | G2/G3 | - | NoFamilyWitness |
| LEG-082 | LEG | LegacyObligation | UntilGate | - | batpak::compaction | G2 | - | NoFamilyWitness |
| LEG-083 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2 | - | NoFamilyWitness |
| LEG-084 | LEG | LegacyObligation | UntilGate | - | batpak::event | G1/G2 | - | NoFamilyWitness |
| LEG-085 | LEG | LegacyObligation | UntilGate | - | batpak::store | G2/G8 | - | NoFamilyWitness |
| LEG-086 | LEG | LegacyObligation | UntilGate | - | batpak::projection | G2/G3 | - | NoFamilyWitness |
| LEG-087 | LEG | LegacyObligation | UntilGate | - | batpak::storage_port | G2/G3 | - | NoFamilyWitness |
| DEC-001 | DEC | Decision | Permanent | HistoricalReceipt | docs/30_DECISION_AND_REJECTION_LEDGER.md | GateIndependent | - | NoFamilyWitness |
| DEC-002 | DEC | Decision | HistoricalCoverageOnly | HistoricalReceipt | docs/30_DECISION_AND_REJECTION_LEDGER.md | GateIndependent | - | NoFamilyWitness |
| DEC-003 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-004 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-005 | DEC | Decision | HistoricalCoverageOnly | Naming | docs/30_DECISION_AND_REJECTION_LEDGER.md | historical:G0 | - | NoFamilyWitness |
| DEC-006 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-007 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-008 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-009 | DEC | Decision | HistoricalCoverageOnly | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | historical:G0 | - | NoFamilyWitness |
| DEC-010 | DEC | Decision | HistoricalCoverageOnly | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | historical:G5 | - | NoFamilyWitness |
| DEC-011 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-012 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-013 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4 | - | NoFamilyWitness |
| DEC-014 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3 | - | NoFamilyWitness |
| DEC-015 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3 | - | NoFamilyWitness |
| DEC-016 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-017 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G6 | - | NoFamilyWitness |
| DEC-018 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-019 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4 | - | NoFamilyWitness |
| DEC-020 | DEC | Decision | UntilCompatibilityExpiry | Compatibility | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-021 | DEC | Decision | UntilCompatibilityExpiry | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G6 | - | NoFamilyWitness |
| DEC-022 | DEC | Decision | UntilCompatibilityExpiry | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-023 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-024 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-025 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G7 | - | NoFamilyWitness |
| DEC-026 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-027 | DEC | Decision | Permanent | Compatibility | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-028 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G6 | - | NoFamilyWitness |
| DEC-029 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5/G6 | - | NoFamilyWitness |
| DEC-030 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-031 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-032 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G8 | - | NoFamilyWitness |
| DEC-033 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G8 | - | NoFamilyWitness |
| DEC-034 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-035 | DEC | Decision | UntilGate | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-036 | DEC | Decision | UntilGate | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-037 | DEC | Decision | UntilGate | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5/G7 | - | NoFamilyWitness |
| DEC-038 | DEC | Decision | UntilGate | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3/G8 | - | NoFamilyWitness |
| DEC-039 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-040 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0/G5 | - | NoFamilyWitness |
| DEC-041 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-042 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-043 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-044 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3 | - | NoFamilyWitness |
| DEC-045 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3 | - | NoFamilyWitness |
| DEC-046 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-047 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-048 | DEC | Decision | Permanent | HistoricalReceipt | docs/30_DECISION_AND_REJECTION_LEDGER.md | GateIndependent | - | NoFamilyWitness |
| DEC-049 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5 | - | NoFamilyWitness |
| DEC-050 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4/G5 | - | NoFamilyWitness |
| DEC-051 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4 | - | NoFamilyWitness |
| DEC-052 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-053 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-054 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-055 | DEC | Decision | UntilGate | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-056 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3/G9 | - | NoFamilyWitness |
| DEC-057 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3 | - | NoFamilyWitness |
| DEC-058 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G9 | - | NoFamilyWitness |
| DEC-059 | DEC | Decision | Permanent | Compatibility | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-060 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4 | - | NoFamilyWitness |
| DEC-061 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-062 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4 | - | NoFamilyWitness |
| DEC-063 | DEC | Decision | Permanent | ImplementationPosture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2/G8 | - | NoFamilyWitness |
| DEC-064 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2 | - | NoFamilyWitness |
| DEC-065 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0/G5 | - | NoFamilyWitness |
| DEC-066 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G5/G6 | - | NoFamilyWitness |
| DEC-067 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-068 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G1/G3 | - | NoFamilyWitness |
| DEC-069 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2/G3/G5 | - | NoFamilyWitness |
| DEC-070 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0 | - | NoFamilyWitness |
| DEC-071 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2/G9 | - | NoFamilyWitness |
| DEC-072 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G0/G9 | - | NoFamilyWitness |
| DEC-073 | DEC | Decision | Permanent | Capability | docs/30_DECISION_AND_REJECTION_LEDGER.md | G4/G5/G8/G9 | - | NoFamilyWitness |
| DEC-074 | DEC | Decision | Permanent | Enforcement | docs/30_DECISION_AND_REJECTION_LEDGER.md | G3/G9 | - | NoFamilyWitness |
| DEC-075 | DEC | Decision | Permanent | Architecture | docs/30_DECISION_AND_REJECTION_LEDGER.md | G2/G5/G6 | - | NoFamilyWitness |
| ARCH-batpak | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-batpak-cli | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-batpak-examples | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-batql | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-macbat | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-macbat-compiler | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-netbat | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-syncbat | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| ARCH-testpak | ARCH | ArchitectureConstraint | Permanent | - | spec/architecture.rs | G0 | - | StructuralArchitecture |
| QUAL-batpak-browser-storage | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G2/G5/G7 | wasm32 host | QualificationReceipt |
| QUAL-batpak-native | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G0/G5 | std | QualificationReceipt |
| QUAL-batpak-semantic | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G0/G5 | no_std + alloc | QualificationReceipt |
| QUAL-syncbat-browser | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G0/G5 | wasm32 host | QualificationReceipt |
| QUAL-syncbat-native | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G0/G5 | std | QualificationReceipt |
| QUAL-syncbat-semantic | QUAL | QualificationRequirement | Permanent | - | spec/architecture.rs | G0/G5 | no_std + alloc | QualificationReceipt |

## Edges

| Source | Relation | Target |
| --- | --- | --- |
| SEED-AVAILABILITY-AXES | DerivesFrom | DEC-030 |
| SEED-BATQL-FROZEN | DerivesFrom | DEC-060 |
| SEED-CONCEPT-SPINE | DerivesFrom | DEC-068 |
| SEED-MUTERPRATER-SCOPE | DerivesFrom | DEC-015 |
| SEED-NO-INLINE-DOMAIN-TYPES | DerivesFrom | DEC-068 |
| SEED-NO-STD-SEMANTIC-PROFILES | DerivesFrom | DEC-065 |
| SEED-SEMANTIC-ZERO-LEAKAGE | DerivesFrom | DEC-068 |
| SEED-SYNC-FIRST | Refines | LEG-080 |
| SEED-TIME-AXES | DerivesFrom | DEC-061 |

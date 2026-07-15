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
| LEG | 84 |
| DEC | 70 |
| ARCH | 9 |
| QUAL | 5 |

## Totals

```text
nodes: 193
edges: 9
unresolved references: 0
```

## Counts by kind

| GuaranteeKind | Nodes |
| --- | --- |
| ArchitectureConstraint | 19 |
| BootstrapAssertion | 2 |
| Decision | 70 |
| LegacyObligation | 84 |
| QualificationRequirement | 5 |
| SemanticLaw | 13 |

## Counts by lifetime

| GuaranteeLifetime | Nodes |
| --- | --- |
| Permanent | 109 |
| UntilCompatibilityExpiry | 5 |
| UntilSuccessor | 79 |

## Active versus closed

```text
active (gating): 193
closed evidence: 0
historical coverage only: 0
```

## Classified SEED

| GuaranteeId | Kind | Lifetime | Owner | Gates | Witness | Relations |
| --- | --- | --- | --- | --- | --- | --- |
| SEED-ONE-OWNER | SemanticLaw | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py contract-id uniqueness; docs/28 self-explaining repository | - |
| SEED-SYNCBAT-ONE-HEARTBEAT | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | spec/architecture.rs SYNCBAT_REQUIRED_PLANES; audit.py | - |
| SEED-NO-STANDALONE-VM | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 | spec/architecture.rs PACKAGES; audit.py forbidden paths | - |
| SEED-FBAT-CORE | ArchitectureConstraint | Permanent | docs/05_STORAGE_FBAT_AND_TILES.md | G2 | stale-vocabulary scan; audit.py | - |
| SEED-PAKVM-NAME | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 | stale-vocabulary scan; audit.py | - |
| SEED-NO-DUAL-PRODUCT | ArchitectureConstraint | Permanent | docs/02_SYSTEM_MODEL.md | G0 | stale-vocabulary scan; docs/31 | - |
| SEED-NO-AMBIENT-AUTHORITY | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G5 | LEG-066; BP-GAUNTLET-1 capability enforcement | - |
| SEED-SEMANTIC-ZERO-LEAKAGE | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0 | seedcheck production-token scan; DEC-068 AST gate | DerivesFrom DEC-068 |
| SEED-SYNC-FIRST | ArchitectureConstraint | Permanent | docs/08_SYNCBAT_RUNTIME.md | G0/G5 | LEG-080; seedcheck no-tokio | Discharges LEG-080 |
| SEED-NO-STD-SEMANTIC-PROFILES | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0/G5 | DEC-065 qualification matrix | DerivesFrom DEC-065 |
| SEED-CONCEPT-SPINE | ArchitectureConstraint | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G0 | DEC-068 AST gate; seedcheck | DerivesFrom DEC-068 |
| SEED-NO-INLINE-DOMAIN-TYPES | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G1/G3 | DEC-068 AST gate | DerivesFrom DEC-068 |
| SEED-EXPLICIT-EFFECTS | SemanticLaw | Permanent | docs/08_SYNCBAT_RUNTIME.md | G6 | LEG-036; LEG-061; BP-GAUNTLET-1 | - |
| SEED-INDEPENDENT-ORACLE | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G8 | LEG-079; LEG-049 | - |
| SEED-AUDITED-DENOMINATOR | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G9 | LEG-049; docs/34 coverage; audit.py declaration parity | - |
| SEED-MUTERPRATER-SCOPE | ArchitectureConstraint | Permanent | docs/12_TESTPAK.md | G3 | DEC-015 | DerivesFrom DEC-015 |
| SEED-BOUNDED-PUSH | SemanticLaw | Permanent | docs/17_DELIVERY_AND_CONCURRENCY.md | G6 | LEG-032; LEG-060 | - |
| SEED-AVAILABILITY-AXES | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G2 | DEC-030; docs/37 | DerivesFrom DEC-030 |
| SEED-TIME-AXES | SemanticLaw | Permanent | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | G2 | DEC-061; LEG-025; LEG-073 | DerivesFrom DEC-061 |
| SEED-DOC-STATUS | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py front-matter check | - |
| SEED-NO-PLACEHOLDER-LAW | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 | audit.py placeholder scan | - |
| SEED-LEGACY-OBLIGATION | SemanticLaw | Permanent | docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md | G0/G9 | docs/34 coverage; audit.py LEG parity | - |
| SEED-ECS-NOT-ONTOLOGY | ArchitectureConstraint | Permanent | docs/18_DATA_ORIENTED_ECS.md | G8 | docs/18; docs/31 | - |
| SEED-BVISOR-HONESTY | SemanticLaw | Permanent | docs/09_BVISOR.md | G5 | LEG-042; LEG-043 | - |
| SEED-BATQL-FROZEN | SemanticLaw | Permanent | companion/BATQL_LANGUAGE.md | G4 | companion 13 language-change law; DEC-060 | DerivesFrom DEC-060 |

## Nodes

| GuaranteeId | Family | Kind | Lifetime | Owner | Gates |
| --- | --- | --- | --- | --- | --- |
| SEED-AUDITED-DENOMINATOR | SEED | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G9 |
| SEED-AVAILABILITY-AXES | SEED | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G2 |
| SEED-BATQL-FROZEN | SEED | SemanticLaw | Permanent | companion/BATQL_LANGUAGE.md | G4 |
| SEED-BOUNDED-PUSH | SEED | SemanticLaw | Permanent | docs/17_DELIVERY_AND_CONCURRENCY.md | G6 |
| SEED-BVISOR-HONESTY | SEED | SemanticLaw | Permanent | docs/09_BVISOR.md | G5 |
| SEED-CONCEPT-SPINE | SEED | ArchitectureConstraint | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G0 |
| SEED-DOC-STATUS | SEED | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 |
| SEED-ECS-NOT-ONTOLOGY | SEED | ArchitectureConstraint | Permanent | docs/18_DATA_ORIENTED_ECS.md | G8 |
| SEED-EXPLICIT-EFFECTS | SEED | SemanticLaw | Permanent | docs/08_SYNCBAT_RUNTIME.md | G6 |
| SEED-FBAT-CORE | SEED | ArchitectureConstraint | Permanent | docs/05_STORAGE_FBAT_AND_TILES.md | G2 |
| SEED-INDEPENDENT-ORACLE | SEED | SemanticLaw | Permanent | docs/24_GAUNTLET.md | G3/G8 |
| SEED-LEGACY-OBLIGATION | SEED | SemanticLaw | Permanent | docs/21_LEGACY_SEMANTIC_OBLIGATIONS.md | G0/G9 |
| SEED-MUTERPRATER-SCOPE | SEED | ArchitectureConstraint | Permanent | docs/12_TESTPAK.md | G3 |
| SEED-NO-AMBIENT-AUTHORITY | SEED | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G5 |
| SEED-NO-DUAL-PRODUCT | SEED | ArchitectureConstraint | Permanent | docs/02_SYSTEM_MODEL.md | G0 |
| SEED-NO-INLINE-DOMAIN-TYPES | SEED | SemanticLaw | Permanent | docs/04_TYPE_SYSTEM_AND_SOURCE_LAYOUT.md | G1/G3 |
| SEED-NO-PLACEHOLDER-LAW | SEED | BootstrapAssertion | Permanent | docs/00_CONSTITUTION.md | G0 |
| SEED-NO-STANDALONE-VM | SEED | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 |
| SEED-NO-STD-SEMANTIC-PROFILES | SEED | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0/G5 |
| SEED-ONE-OWNER | SEED | SemanticLaw | Permanent | docs/00_CONSTITUTION.md | G0 |
| SEED-PAKVM-NAME | SEED | SemanticLaw | Permanent | docs/07_PAKVM_ISA.md | G0/G5 |
| SEED-SEMANTIC-ZERO-LEAKAGE | SEED | ArchitectureConstraint | Permanent | docs/20_DEPENDENCY_SOVEREIGNTY.md | G0 |
| SEED-SYNC-FIRST | SEED | ArchitectureConstraint | Permanent | docs/08_SYNCBAT_RUNTIME.md | G0/G5 |
| SEED-SYNCBAT-ONE-HEARTBEAT | SEED | ArchitectureConstraint | Permanent | docs/03_REPOSITORY_AND_PACKAGES.md | G0 |
| SEED-TIME-AXES | SEED | SemanticLaw | Permanent | docs/16_IDENTITY_TIME_AND_NAVIGATION.md | G2 |
| LEG-001 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-002 | LEG | LegacyObligation | UntilCompatibilityExpiry | batpak::event | G2 |
| LEG-003 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-004 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-005 | LEG | LegacyObligation | UntilSuccessor | batpak::frontier | G2 |
| LEG-006 | LEG | LegacyObligation | UntilSuccessor | batpak::visibility | G2 |
| LEG-007 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-008 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-009 | LEG | LegacyObligation | UntilSuccessor | batpak::compaction | G2 |
| LEG-010 | LEG | LegacyObligation | UntilSuccessor | batpak::recovery | G2/G3 |
| LEG-011 | LEG | LegacyObligation | UntilSuccessor | batpak::lifecycle | G2 |
| LEG-012 | LEG | LegacyObligation | UntilSuccessor | batpak::lifecycle | G2 |
| LEG-013 | LEG | LegacyObligation | UntilSuccessor | batpak::lifecycle | G2 |
| LEG-014 | LEG | LegacyObligation | UntilSuccessor | batpak::recovery | G2 |
| LEG-015 | LEG | LegacyObligation | UntilSuccessor | batpak::secret | G2 |
| LEG-016 | LEG | LegacyObligation | UntilSuccessor | batpak::storage_port | G2/G3 |
| LEG-017 | LEG | LegacyObligation | UntilSuccessor | batpak::tile | G2/G8 |
| LEG-018 | LEG | LegacyObligation | UntilSuccessor | batpak::tile | G8 |
| LEG-019 | LEG | LegacyObligation | UntilSuccessor | batpak::integrity | G2/G3 |
| LEG-020 | LEG | LegacyObligation | UntilSuccessor | testpak::bench | G3/G8 |
| LEG-021 | LEG | LegacyObligation | UntilSuccessor | batpak::coordinate | G2 |
| LEG-022 | LEG | LegacyObligation | UntilSuccessor | batpak::navigation | G2 |
| LEG-023 | LEG | LegacyObligation | UntilSuccessor | batpak::integrity | G2/G3 |
| LEG-024 | LEG | LegacyObligation | UntilSuccessor | batpak::event | G1/G2 |
| LEG-025 | LEG | LegacyObligation | UntilSuccessor | batpak::time | G2 |
| LEG-026 | LEG | LegacyObligation | UntilSuccessor | batpak::projection | G2 |
| LEG-027 | LEG | LegacyObligation | UntilCompatibilityExpiry | batpak::path | G2 |
| LEG-028 | LEG | LegacyObligation | UntilSuccessor | batpak::projection | G2 |
| LEG-029 | LEG | LegacyObligation | UntilSuccessor | batpak::projection_query | G2/G6/G7 |
| LEG-030 | LEG | LegacyObligation | UntilSuccessor | batpak::projection | G2/G3 |
| LEG-031 | LEG | LegacyObligation | UntilSuccessor | batpak::projection | G8 |
| LEG-032 | LEG | LegacyObligation | UntilSuccessor | batpak::delivery | G2/G6 |
| LEG-033 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6 |
| LEG-034 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6 |
| LEG-035 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G3/G6 |
| LEG-036 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6 |
| LEG-037 | LEG | LegacyObligation | UntilSuccessor | batpak::effect | G2/G6 |
| LEG-038 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G5/G6 |
| LEG-039 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6 |
| LEG-040 | LEG | LegacyObligation | UntilCompatibilityExpiry | batpak::schema | G2/G5 |
| LEG-041 | LEG | LegacyObligation | UntilSuccessor | batpak::world | G5 |
| LEG-042 | LEG | LegacyObligation | UntilSuccessor | syncbat::bvisor | G5 |
| LEG-043 | LEG | LegacyObligation | UntilSuccessor | syncbat::bvisor | G5 |
| LEG-044 | LEG | LegacyObligation | UntilSuccessor | syncbat::pakvm | G3/G5 |
| LEG-045 | LEG | LegacyObligation | UntilSuccessor | netbat | G7 |
| LEG-046 | LEG | LegacyObligation | UntilSuccessor | macbat-compiler | G1 |
| LEG-047 | LEG | LegacyObligation | UntilSuccessor | macbat-compiler | G1/G2 |
| LEG-048 | LEG | LegacyObligation | UntilSuccessor | testpak::repo | G3 |
| LEG-049 | LEG | LegacyObligation | UntilSuccessor | testpak::gauntlet | G3/G9 |
| LEG-050 | LEG | LegacyObligation | UntilSuccessor | batpak::event | G2/G3 |
| LEG-051 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-052 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2/G3 |
| LEG-053 | LEG | LegacyObligation | UntilCompatibilityExpiry | batpak::compatibility | G2/G3 |
| LEG-054 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G2/G6 |
| LEG-055 | LEG | LegacyObligation | UntilSuccessor | batpak::time | G2/G3 |
| LEG-056 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2/G8 |
| LEG-057 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2/G6 |
| LEG-058 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6 |
| LEG-059 | LEG | LegacyObligation | UntilSuccessor | batpak::receipt | G2/G6 |
| LEG-060 | LEG | LegacyObligation | UntilSuccessor | syncbat::runtime | G6/G7 |
| LEG-061 | LEG | LegacyObligation | UntilSuccessor | syncbat::bvisor | G5/G6 |
| LEG-062 | LEG | LegacyObligation | UntilSuccessor | syncbat::world | G6 |
| LEG-063 | LEG | LegacyObligation | UntilSuccessor | batpak::navigation | G2/G7 |
| LEG-064 | LEG | LegacyObligation | UntilCompatibilityExpiry | testpak::oracle | G2/G7 |
| LEG-065 | LEG | LegacyObligation | UntilSuccessor | testpak::repo | G0/G3 |
| LEG-066 | LEG | LegacyObligation | UntilSuccessor | batpak::port | G2/G5/G7 |
| LEG-067 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2/G3 |
| LEG-068 | LEG | LegacyObligation | UntilSuccessor | batpak::tile | G2/G8 |
| LEG-069 | LEG | LegacyObligation | UntilSuccessor | batpak::time | G2/G3 |
| LEG-070 | LEG | LegacyObligation | UntilSuccessor | batpak::frontier | G2/G3 |
| LEG-071 | LEG | LegacyObligation | UntilSuccessor | batpak::frontier | G2/G3 |
| LEG-072 | LEG | LegacyObligation | UntilSuccessor | batpak::frontier | G2/G6 |
| LEG-073 | LEG | LegacyObligation | UntilSuccessor | batpak::time | G2/G3 |
| LEG-074 | LEG | LegacyObligation | UntilSuccessor | batpak::lifecycle | G2/G3 |
| LEG-075 | LEG | LegacyObligation | UntilSuccessor | batpak::lifecycle | G2/G3 |
| LEG-076 | LEG | LegacyObligation | UntilSuccessor | macbat-compiler | G1 |
| LEG-077 | LEG | LegacyObligation | UntilSuccessor | batpak::event | G2/G3 |
| LEG-078 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2/G8 |
| LEG-079 | LEG | LegacyObligation | UntilSuccessor | testpak::oracle | G3/G8 |
| LEG-080 | LEG | LegacyObligation | UntilSuccessor | syncbat::port | G0/G5/G7 |
| LEG-081 | LEG | LegacyObligation | UntilSuccessor | batpak::secret | G2/G3 |
| LEG-082 | LEG | LegacyObligation | UntilSuccessor | batpak::compaction | G2 |
| LEG-083 | LEG | LegacyObligation | UntilSuccessor | batpak::store | G2 |
| LEG-084 | LEG | LegacyObligation | UntilSuccessor | batpak::event | G1/G2 |
| DEC-001 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-002 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-003 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-004 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-005 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-006 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-007 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-008 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-009 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-010 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-011 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-012 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-013 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-014 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-015 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-016 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-017 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-018 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-019 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-020 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-021 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-022 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-023 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-024 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-025 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-026 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-027 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-028 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-029 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-030 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-031 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-032 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-033 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-034 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-035 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-036 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-037 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-038 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-039 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-040 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-041 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-042 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-043 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-044 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-045 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-046 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-047 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-048 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-049 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-050 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-051 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-052 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-053 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-054 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-055 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-056 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-057 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-058 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-059 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-060 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-061 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-062 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-063 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-064 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-065 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-066 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-067 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-068 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-069 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| DEC-070 | DEC | Decision | Permanent | docs/30_DECISION_AND_REJECTION_LEDGER.md |  |
| ARCH-batpak | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-batpak-cli | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-batpak-examples | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-batql | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-macbat | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-macbat-compiler | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-netbat | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-syncbat | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| ARCH-testpak | ARCH | ArchitectureConstraint | Permanent | spec/architecture.rs |  |
| QUAL-batpak-native | QUAL | QualificationRequirement | Permanent | spec/architecture.rs | std |
| QUAL-batpak-semantic | QUAL | QualificationRequirement | Permanent | spec/architecture.rs | no_std + alloc |
| QUAL-syncbat-browser | QUAL | QualificationRequirement | Permanent | spec/architecture.rs | wasm32 host |
| QUAL-syncbat-native | QUAL | QualificationRequirement | Permanent | spec/architecture.rs | std |
| QUAL-syncbat-semantic | QUAL | QualificationRequirement | Permanent | spec/architecture.rs | no_std + alloc |

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
| SEED-SYNC-FIRST | Discharges | LEG-080 |
| SEED-TIME-AXES | DerivesFrom | DEC-061 |

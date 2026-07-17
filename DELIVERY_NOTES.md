---
status: AUTHORITATIVE
contract_id: BP-DELIVERY-NOTES
authority_scope: bundle inventory, validation denominator, and implementation handoff
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Delivery Notes

This bundle is the implementation handoff for the final clean-room BatPak v1 architecture.

## Inventory

Every count below is a generated derivation of a typed denominator or the current tracked tree — never authored prose:

<!-- BUNDLE-INVENTORY:BEGIN generated from spec/architecture.rs; spec/invariants.rs; spec/dispositions.rs; spec/legacy_obligations.rs; spec/legacy_invariant_coverage.rs; spec/operators.rs; spec/generated_views.rs by bootstrap/project.py; do not edit -->
| Metric | Count | Derivation |
| --- | --- | --- |
| numbered architecture documents | 38 | current tracked docs matching docs/[0-9][0-9]_*.md |
| Markdown documents | 47 | current eligible Markdown corpus |
| Cargo packages | 9 | PackageId::ALL with PACKAGES parity |
| package edges | 19 | EDGES |
| qualification profiles | 6 | QUALIFICATION_PROFILES |
| SEED guarantees | 25 | spec/invariants.rs SEED inventory |
| decision rows | 75 | spec/dispositions.rs DECISIONS |
| legacy semantic obligations | 87 | spec/legacy_obligations.rs OBLIGATIONS |
| legacy invariant declarations | 115 | SOURCE_INVARIANT_IDS with COVERAGE parity |
| BatQL operators | 13 | OperatorId::ALL with OPERATORS parity |
| registered generated views | 41 | GeneratedView::ALL |
| static generated target instances | 42 | expansion of every Static registry target |
| corpus-frontmatter bindings | 47 | the eligible Markdown corpus reached by CorpusEpochMembership |
<!-- BUNDLE-INVENTORY:END -->

The bundle also carries, nonnumerically:

```text
canonical numeric semantics and authority contract (docs/37, DEC-069)
classified SEED guarantees plus one derived Guarantee Graph (DEC-070)
full BatQL frontend language companion
independent Python audit and freeze tools
registry-driven deterministic repository projection generator
independent Rust seed checker
non-overwriting Rust Gate-0 materializer
agent authority guide and finish-line checklist
```

The exact counts that affect machine law are derived from `spec/`. Human prose does not override those facts.

## Package topology

The package topology lives in `spec/architecture.rs` and its generated projections: the `PACKAGE-INVENTORY` blocks in the root README and docs/03, and the `PACKAGE-EDGES` block in docs/03. Delivery Notes does not carry a third package topology.

## First implementation action

1. Install the pinned Rust toolchain.
2. Compile and run `bootstrap/seedcheck.rs`.
3. Run `bootstrap/audit.py` and `bootstrap/freeze.py --check`.
4. Compile and run `bootstrap/materialize.rs` to create only the signed Gate-0 skeleton.
5. Compile `batpak` and `syncbat` semantic profiles under `no_std + alloc`.
6. Do not copy legacy source.
7. Begin Gate 1 MacBat only after the Gate-0 review packet closes.

## Validation denominator

The required Tier 0 receipt denominator is a generated projection of the harness that enforces it:

<!-- TIER0-RECEIPT-DENOMINATOR:BEGIN generated from bootstrap/selftest.py by bootstrap/project.py; do not edit -->
| Receipt | Artifact-bound |
| --- | --- |
| tier0-law-fixtures | no |
| tier0-seedcheck | yes |
| tier0-materialize | yes |
| tier0-seedcheck-tests | yes |
| tier0-spec-tests | yes |
<!-- TIER0-RECEIPT-DENOMINATOR:END -->

Every listed receipt is required. A receipt qualifies only when availability, compilation, execution, passing disposition, physical target, and required artifact/source binding all hold.

This block declares the denominator. It does not claim that the latest run passed. Candidate and cleanroom qualification outcomes belong to exact-SHA, exact-target run receipts, not timeless prose.

`spec/gates.rs` is the one gate identity: every gate-bearing fact carries `&'static [GateId]`, never a slash-delimited string.

## Integrity

`SPEC.sha256` freezes every bundle file except itself and generated archives. The deterministic ZIP and its SHA-256 checksum are delivered beside this folder. The archive is integrity-read after creation.

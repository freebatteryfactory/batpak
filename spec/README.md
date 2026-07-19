---
status: AUTHORITATIVE
contract_id: BP-SPEC-FACTS-README
authority_scope: machine-readable seed fact guidance
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Typed Seed Facts

The Rust files in this directory are the standard-library-only architecture facts
consumed by `seedcheck`, `materialize`, and `receiptcheck`, and later imported
into TestPak Repo IR. They are authoritative where their fronting documents say
so. Do not generate them from implementation source during bootstrap: they are
the seed against which implementation is checked. After self-hosting, TestPak may
generate equivalent views while `seedcheck` retains an independently reviewed
expected form.

## One authority, one public door

Every file in this directory owns exactly one authority and exposes it through
one public door. No fact has two owners; no owner has two doors. `spec/lib.rs`
is the crate's single public door: the whole directory compiles once as a real
rlib (5.5E2), every `pub` item below is API by construction, and each binary
links that one boundary instead of textually mounting modules. When a module
names a fact another module owns, it consumes it — it never restates it.

## Module census

The exact module set and shape, generated from `spec/lib.rs` — the census is
machine-derived and audited, never hand-counted here.

<!-- SPEC-MODULE-CATALOG:BEGIN generated from spec/lib.rs by bootstrap/project.py; do not edit -->
| Module | Shape |
| --- | --- |
| `architecture/` | domain directory (5 modules) |
| `bootstrap_output/` | domain directory (2 modules) |
| `bootstrap_qualification/` | domain directory (4 modules) |
| `commands/` | domain directory (2 modules) |
| `compiler_assumptions/` | domain directory (2 modules) |
| `contracts/` | domain directory (2 modules) |
| `corpus/` | domain directory (2 modules) |
| `dispositions/` | domain directory (3 modules) |
| `gates/` | domain directory (3 modules) |
| `generated_views/` | domain directory (3 modules) |
| `guarantees/` | domain directory (3 modules) |
| `identities/` | domain directory (3 modules) |
| `invariants/` | domain directory (3 modules) |
| `legacy_invariant_coverage/` | domain directory (3 modules) |
| `legacy_obligations/` | domain directory (3 modules) |
| `mutation/` | domain directory (2 modules) |
| `operators/` | domain directory (3 modules) |
| `promotion/` | domain directory (2 modules) |
| `pakvm_isa/` | domain directory (6 modules) |
| `proof/` | domain directory (4 modules) |
| `reconciliation/` | domain directory (3 modules) |
| `sprouting/` | domain directory (5 modules) |
| `syncbat_firewall/` | domain directory (4 modules) |
| `tier0_cross_run/` | domain directory (5 modules) |
| `toolchain/` | domain directory (2 modules) |
| `verification/` | domain directory (7 modules) |
| | 26 modules behind the one `lib.rs` boundary |
<!-- SPEC-MODULE-CATALOG:END -->

## Module catalog

Each row states what that file owns; the file's own top doc comment is the
binding claim, and this row must not widen it. This catalog is authored
meaning; the census above is the generated mirror of the declared module set.

| File | Owns |
| --- | --- |
| `lib.rs` | The single public door: one rlib boundary linking every module; every `pub` item is API by construction. |
| `architecture.rs` | Frozen repository architecture facts: packages, edges, profiles, required docs, forbidden paths. |
| `bootstrap_output.rs` | The SHAPE of the isolated Gate-0 workspace candidate the materializer publishes; derived from the closed inventories, never a hardcoded file table. |
| `bootstrap_qualification.rs` | The typed Tier 0 qualification receipt algebra: what a qualification receipt means and its required `ALL` denominator. |
| `commands.rs` | The three closed command namespaces: `ProductCommand`, `BatQlSourceMode`, `TestPakCommand`. |
| `compiler_assumptions.rs` | The admitted compiler-assumption kinds: dangerous mechanisms that have earned a ledger row, detector, and hostile witness. |
| `contracts.rs` | The admitted MacBat contract kinds at Gate 1 — the border crossing, not a brochure of families that might someday exist. |
| `corpus.rs` | The corpus reconciliation epoch: which whole-corpus reconciliation a document claims membership in (documentary, never runtime state). |
| `dispositions.rs` | Frozen architecture decision and disposition facts (the DEC ledger) and the derived stale-vocabulary source. |
| `gates.rs` | The one gate identity: `GateId`, `GateSpec`, and the gate inventory — no second gate-identity type. |
| `generated_views.rs` | The generated-view registry: the closed denominator of every repository-generated view and its authority source. |
| `guarantees.rs` | The shared guarantee-classification types (DEC-070); it owns no individual guarantee and generates no graph. |
| `identities.rs` | The identity, generation, binding, and version catalogs — four separate closed vocabularies. |
| `invariants.rs` | The SEED facts (bootstrap invariants) and their per-fact guarantee classification. |
| `legacy_invariant_coverage.rs` | One-for-one coverage of the live legacy invariant catalog against a frozen declaration manifest. |
| `legacy_obligations.rs` | The retained clean-room legacy semantic obligations (the LEG facts) and their gating status. |
| `mutation.rs` | The canonical mutation vocabulary: lane facts and result classifications as total functions on the enums. |
| `operators.rs` | The concrete BatQL V1 operator facts (`OperatorSpec`) and the operator-related grammar fragments and tables. |
| `promotion.rs` | The conjunctive candidate-promotion denominator: the four required requirements, no waiver or optional flag. |
| `pakvm_isa.rs` | The typed PakVM semantic ISA: the one authority for what a PakVM semantic node means. |
| `proof.rs` | The proof-unit vocabulary: the closed proof-denominator types the audited-denominator doctrine depends on. |
| `reconciliation.rs` | The DEC-075 composition: single-writer dual-axis reconciliation, binding coordinates it does not redeclare. |
| `sprouting.rs` | The typed candidate-sprouting axes and the specialized-plan admission policy (5.5F3). |
| `syncbat_firewall.rs` | The typed SyncBat authority firewall: the plane authority boundaries (D4c2). |
| `tier0_cross_run.rs` | Cross-run same-source comparison and promotion confirmation over verified Tier 0 qualifications (5.5E6c). |
| `toolchain.rs` | The typed toolchain owner: channel, edition, resolver, MSRV floor, and the tracked `rust-toolchain.toml` bytes. |
| `verification.rs` | The typed layered dynamic-verification plane (DEC-077/DEC-078, docs/38) with no aggregate assurance ladder. |

Each SEED fact in `invariants.rs` carries its own guarantee classification (kind,
lifetime, owner, gates, witness, relations). The derived cross-family index is
the non-normative `docs/GUARANTEE_GRAPH.generated.md`, generated by
`bootstrap/project.py` and independently re-audited; it never replaces an owning
fact.

## Anti-drift

The census block above is generated from `spec/lib.rs` by `bootstrap/project.py`
and held current by `project.py --check`; a new `pub mod` regenerates it in the
same commit or the gate refuses. The authored catalog rows carry meaning only —
when a module is added or removed, its authored row is corrected in the same
commit, and the freeze manifest `SPEC.sha256` binds the exact byte set every
tool sees. A module that is not in `spec/lib.rs` and the manifest is not part
of the seed.

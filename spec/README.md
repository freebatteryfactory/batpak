---
status: AUTHORITATIVE
contract_id: BP-SPEC-FACTS-README
authority_scope: machine-readable seed fact guidance
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-19
reconciliation_epoch: cleanroom-v1
---

# Typed Seed Facts

The Rust files in this directory are the dependency-free typed architecture
facts — compiled once as a `#![no_std]` + `alloc` rlib, no external crates —
consumed by `seedcheck`, `materialize`, and `receiptcheck`, and later imported
into TestPak Repo IR. They are authoritative where their fronting documents say
so. Do not generate them from implementation source during bootstrap: they are
the seed against which implementation is checked. After self-hosting, TestPak may
generate equivalent views while `seedcheck` retains an independently reviewed
expected form.

## One authority, one public door

Every stable semantic fact has one owner. Every owning domain has one public
door. Every tracked source file has one coherent role. Owners are concepts —
contracts, decisions, typed fact families — and files are the physical carriers
of an owner's bytes: a carrier never becomes a second sovereign owner of the
facts it carries. `spec/lib.rs` is the crate's single public door: the whole
directory compiles once as a real rlib (5.5E2), every `pub` item below is API
by construction, and each binary links that one boundary instead of textually
mounting modules. When a domain names a fact another domain owns, it consumes
it — it never restates it.

## Module census

The exact module set and shape, generated from `spec/lib.rs` — the census is
machine-derived and audited, never hand-counted here.

<!-- SPEC-MODULE-CATALOG:BEGIN generated from spec/lib.rs by bootstrap/project.py; do not edit -->
| Module | Shape |
| --- | --- |
| `architecture/` | domain directory (3 modules) |
| `authenticated_history/` | domain directory (2 modules) |
| `bootstrap_output/` | domain directory (2 modules) |
| `bootstrap_qualification/` | domain directory (4 modules) |
| `campaign/` | domain directory (4 modules) |
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
| `release/` | domain directory (3 modules) |
| `sprouting/` | domain directory (5 modules) |
| `syncbat_firewall/` | domain directory (4 modules) |
| `tier0_cross_run/` | domain directory (5 modules) |
| `toolchain/` | domain directory (2 modules) |
| `verification/` | domain directory (7 modules) |
| | 29 modules behind the one `lib.rs` boundary |
<!-- SPEC-MODULE-CATALOG:END -->

## Domain catalog

Each row states what that domain owns; the domain door's own top doc comment
(`mod.rs`) is the binding claim, and this row must not widen it. This catalog
is authored meaning at domain level — the generated census above carries the
exact module tree, so no filename ledger is hand-maintained here.

| Domain | Owns |
| --- | --- |
| `lib.rs` | The single public door: one rlib boundary linking every domain; every `pub` item is API by construction. |
| `architecture/` | Frozen repository architecture facts: packages, edges, profiles, required docs, forbidden paths. |
| `authenticated_history/` | The authenticated-history claim contract (DEC-071): profiles, witness policy and dispositions, and the four independent claim axes — a contract, not an implementation. |
| `bootstrap_output/` | The SHAPE of the isolated Gate-0 workspace candidate the materializer publishes; derived from the closed inventories, never a hardcoded file table. |
| `bootstrap_qualification/` | The typed Tier 0 qualification receipt algebra: what a qualification receipt means and its required `ALL` denominator. |
| `commands/` | The three closed command namespaces: `ProductCommand`, `BatQlSourceMode`, `TestPakCommand`. |
| `compiler_assumptions/` | The admitted compiler-assumption kinds: dangerous mechanisms that have earned a ledger row, detector, and hostile witness. |
| `contracts/` | The admitted MacBat contract kinds at Gate 1 — the border crossing, not a brochure of families that might someday exist. |
| `corpus/` | The corpus reconciliation epoch: which whole-corpus reconciliation a document claims membership in (documentary, never runtime state). |
| `dispositions/` | Frozen architecture decision and disposition facts (the DEC ledger) and the derived stale-vocabulary source. |
| `gates/` | The one gate identity: `GateId`, `GateSpec`, and the gate inventory — no second gate-identity type. |
| `generated_views/` | The generated-view registry: the closed denominator of every repository-generated view and its authority source. |
| `guarantees/` | The shared guarantee-classification types (DEC-070); it owns no individual guarantee and generates no graph. |
| `identities/` | The identity, generation, binding, and version catalogs — four separate closed vocabularies. |
| `invariants/` | The SEED facts (bootstrap invariants) and their per-fact guarantee classification. |
| `legacy_invariant_coverage/` | One-for-one coverage of the live legacy invariant catalog against a frozen declaration manifest. |
| `legacy_obligations/` | The retained clean-room legacy semantic obligations (the LEG facts) and their gating status. |
| `mutation/` | The canonical mutation vocabulary: lane facts and result classifications as total functions on the enums. |
| `operators/` | The concrete BatQL V1 operator facts (`OperatorSpec`) and the operator-related grammar fragments and tables. |
| `promotion/` | The conjunctive candidate-promotion denominator: the four required requirements, no waiver or optional flag. |
| `pakvm_isa/` | The typed PakVM semantic ISA: the one authority for what a PakVM semantic node means. |
| `proof/` | The proof-unit vocabulary: the closed proof-denominator types the audited-denominator doctrine depends on. |
| `reconciliation/` | The DEC-075 composition: single-writer dual-axis reconciliation, binding coordinates it does not redeclare. |
| `release/` | The release-seal vocabulary (DEC-058): the typed `ReleaseSealField` inventory of what a release receipt binds; every field mandatory even when its set is empty. |
| `sprouting/` | The typed candidate-sprouting axes and the specialized-plan admission policy (5.5F3). |
| `syncbat_firewall/` | The typed SyncBat authority firewall: the plane authority boundaries (D4c2). |
| `tier0_cross_run/` | Cross-run same-source comparison and promotion confirmation over verified Tier 0 qualifications (5.5E6c). |
| `toolchain/` | The typed toolchain owner: channel, edition, resolver, MSRV floor, and the tracked `rust-toolchain.toml` bytes. |
| `verification/` | The typed layered dynamic-verification plane (DEC-077/DEC-078, docs/38) with no aggregate assurance ladder. |

Each SEED fact in `invariants.rs` carries its own guarantee classification (kind,
lifetime, owner, gates, witness, relations). The derived cross-family index is
the non-normative `docs/GUARANTEE_GRAPH.generated.md`, generated by
`bootstrap/project.py` and independently re-audited; it never replaces an owning
fact.

## Anti-drift

The census block above is generated from `spec/lib.rs` by `bootstrap/project.py`
and held current by `project.py --check`; a new `pub mod` regenerates it in the
same commit or the gate refuses. The authored catalog rows carry meaning only —
when a domain is added or removed, its authored row is corrected in the same
commit, and the freeze manifest `SPEC.sha256` binds the exact byte set every
tool sees. A module that is not in `spec/lib.rs` and the manifest is not part
of the seed.

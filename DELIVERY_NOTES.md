---
status: AUTHORITATIVE
contract_id: BP-DELIVERY-NOTES
authority_scope: bundle inventory and validation record
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Delivery Notes

This bundle is the implementation handoff for the final clean-room BatPak v1 architecture.

## Inventory

```text
38 numbered architecture documents (00 through 37)
47 Markdown documents
31,000+ words of architecture and language specification
9 declared Cargo packages (8 + the non-publishable batpak-examples witness)
19 declared package edges
5 target/feature qualification profiles
25 bootstrap invariants
75 architectural decision/disposition rows
84 retained legacy semantic obligations
115 one-for-one legacy invariant dispositions
13 concrete BatQL operator facts (OperatorSpec)
canonical numeric semantics and authority contract (docs/37, DEC-069)
25 classified SEED guarantees plus one derived Guarantee Graph (DEC-070)
full BatQL frontend language companion
independent Python audit and freeze tools
deterministic Python operator-projection generator
independent Rust seed checker
non-overwriting Rust Gate-0 materializer
agent authority guide and finish-line checklist
```

The exact counts that affect machine law are derived from `spec/`. Human prose does not override those facts.

## Frozen package direction

```text
L0  macbat-compiler
      ↑
L1    macbat
      ↑
L2    batpak
      ↑        ↑
L3  syncbat   batql
      ↑
L4   netbat
      ↑
L5  batpak-cli   batpak-examples

L6  testpak is dev-only over the semantic packages.
```

`batpak-examples` (L5, non-publishable) imports the production public APIs and is a compatibility witness; the production semantic package count is unchanged.

`syncbat` contains the five internal planes `runtime`, `pakvm`, `bvisor`, `world`, and `port`. No standalone VM, PakVM, or Bvisor package exists.

The exact edges, edge classes, profile classes, and strict layer numbers are authoritative in `spec/architecture.rs`.

## First implementation action

1. Install the pinned Rust toolchain.
2. Compile and run `bootstrap/seedcheck.rs`.
3. Run `bootstrap/audit.py` and `bootstrap/freeze.py --check`.
4. Compile and run `bootstrap/materialize.rs` to create only the signed Gate-0 skeleton.
5. Compile `batpak` and `syncbat` semantic profiles under `no_std + alloc`.
6. Do not copy legacy source.
7. Begin Gate 1 MacBat only after the Gate-0 review packet closes.

## Validation completed in this delivery environment

```text
required-file and front-matter audit              PASS
unique contract/decision/obligation IDs           PASS
package/path uniqueness                           PASS
dependency graph direction and acyclicity          PASS
modeled Cargo manifest/TOML construction            PASS
SyncBat internal-plane declaration                 PASS
relative Markdown links                            PASS
stale architecture term scan                       PASS
normative unresolved-marker scan                   PASS
decision ledger ↔ typed seed equivalence           PASS
legacy obligation ledger ↔ typed seed equivalence  PASS
107-row legacy catalog coverage equivalence        PASS
Python bootstrap-tool syntax                       PASS
Rust source lexical/delimiter structural scan      PASS
SHA-256 manifest recomputation                      PASS
```

`spec/gates.rs` is the one gate identity. The 25 SEED and 84 LEG gate-bearing facts were migrated from
slash-delimited strings to `&'static [GateId]`; the migration was proven set-equal and byte-identical in
every rendered projection, so no gate was added, dropped, renamed, duplicated, or reordered.

The environment did not contain `rustc` or `cargo`. Therefore `bootstrap/seedcheck.rs` and `bootstrap/materialize.rs` were structurally inspected but not compiled here. Compiling and executing both under Rust 1.97.0 is Gate 0's first executable obligation, not a waived check.

## Integrity

`SPEC.sha256` freezes every bundle file except itself and generated archives. The deterministic ZIP and its SHA-256 checksum are delivered beside this folder. The archive is integrity-read after creation.

---
status: AUTHORITATIVE
contract_id: BP-BOOTSTRAP-TOOLS-README
authority_scope: independent bootstrap tool usage
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
reconciliation_epoch: cleanroom-v1
---

# Bootstrap Tools

```sh
python bootstrap/freeze.py .
python bootstrap/freeze.py . --check
python bootstrap/audit.py .
rustc bootstrap/seedcheck.rs -o target/seedcheck
./target/seedcheck .
rustc bootstrap/materialize.rs -o target/materialize
./target/materialize .
```

`freeze.py` writes the exact SHA-256 manifest.

`audit.py` checks front matter, unique contract IDs, relative links, stale target terms, normative placeholders, and exact freeze bytes.

`seedcheck.rs` independently checks required files, package graph acyclicity, qualification profiles, forbidden paths, typed IDs, exact decision/legacy/invariant-coverage ledgers, SyncBat's required internal planes when source exists, and early source-debt patterns.

`materialize.rs` reads the same typed architecture seed and creates the Gate-0 Cargo workspace, package manifests, package roots, and required SyncBat planes without overwriting nonmatching files. It is a one-way skeleton materializer, not a code generator or semantic owner.

These tools never inspect private reasoning and never confer semantic correctness merely because structure passes.

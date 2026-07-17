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
./target/materialize --seed . --output ../batpak-gate0-candidate
```

`freeze.py` writes the exact SHA-256 manifest.

`audit.py` checks front matter, unique contract IDs, relative links, stale target terms, normative placeholders, and exact freeze bytes.

`seedcheck.rs` independently checks required files, package graph acyclicity, qualification profiles, forbidden paths, typed IDs, exact decision/legacy/invariant-coverage ledgers, SyncBat's required internal planes when source exists, and early source-debt patterns.

`materialize.rs` reads the same typed architecture seed and publishes the Gate-0 Cargo workspace -- workspace manifest, package manifests, source doors, and required SyncBat planes -- as one isolated candidate tree at the explicit `--output` root. Both roots are required; there is no default and no positional form. It refuses a shared or nested seed/output pair, parent traversal in either path, a missing seed, and an unresolvable output parent. The only successful dispositions are `Created` (absent output, complete staged tree renamed into place) and `Unchanged` (the output already carries exactly the planned tree; zero writes); a divergent existing output is refused, never repaired. It is a one-time factory, not a code generator, synchronizer, or semantic owner.

Qualification of the candidate runs on a disposable copy so Cargo lockfiles and build artifacts never touch the qualified tree:

```sh
cargo metadata --no-deps --format-version 1
cargo check --workspace --all-targets
cargo check -p batpak --no-default-features
cargo check -p syncbat --no-default-features
cargo run -p batpak-examples --bin family_smoke
```

These tools never inspect private reasoning and never confer semantic correctness merely because structure passes.

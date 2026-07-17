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

`materialize.rs` reads the same typed architecture seed and publishes the Gate-0 Cargo workspace -- workspace manifest, package manifests, source doors, and required SyncBat planes -- as one isolated candidate tree at the explicit `--output` root. Both roots are required; there is no default and no positional form. It refuses a shared or nested seed/output pair, parent traversal in either path, a missing seed, and an unresolvable output parent. Every planned path obeys one portable relative-path grammar (no backslash, drive colon, `.`/`..` component, control character, trailing dot or space, Windows device name, or case-fold twin), and the binary is bound to the exact `SPEC.sha256` it was compiled against: a seed carrying a different manifest is refused before any plan is built. The only successful dispositions are `Created` (absent output, complete staged tree renamed into place) and `Unchanged` (the output already carries exactly the planned tree; zero writes); a divergent existing output is refused, never repaired. It is a one-time factory, not a code generator, synchronizer, or semantic owner.

Seed auditing and freezing are the signed seed repository's commands (`audit.py`, `freeze.py`, `seedcheck.rs` above); the candidate carries only its own justfile targets -- `check`, `check-no-std`, and `smoke`. Qualification of the candidate runs on a disposable copy so Cargo lockfiles and build artifacts never touch the qualified tree, and every target-sensitive command carries an explicit `--target` (the authoritative triple is `x86_64-pc-windows-msvc`):

```sh
cargo metadata --no-deps --format-version 1
cargo check --target x86_64-pc-windows-msvc --workspace --all-targets
cargo check --target x86_64-pc-windows-msvc -p batpak --no-default-features
cargo check --target x86_64-pc-windows-msvc -p syncbat --no-default-features
cargo run   --target x86_64-pc-windows-msvc -p batpak-examples --bin family_smoke
```

These tools never inspect private reasoning and never confer semantic correctness merely because structure passes.

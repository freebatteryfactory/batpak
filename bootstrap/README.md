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
python bootstrap/selftest.py --emit-evidence ../batpak-tier0
rustc bootstrap/receiptcheck.rs --extern spec=libspec.rlib -o target/receiptcheck
./target/receiptcheck verify ../batpak-tier0/tier0-evidence/qualification.t0 \
    --root . --evidence ../batpak-tier0/tier0-evidence
```

`seedcheck.rs`, `materialize.rs`, and `receiptcheck.rs` each link the real `spec` rlib (`rustc --edition 2024 --crate-type rlib --crate-name spec -o libspec.rlib spec/lib.rs`), the exact library boundary production uses; the lines above elide the rlib build for brevity.

`freeze.py` writes the exact SHA-256 manifest.

`audit.py` checks front matter, unique contract IDs, relative links, stale target terms, normative placeholders, and exact freeze bytes.

`seedcheck.rs` independently checks required files, package graph acyclicity, qualification profiles, forbidden paths, typed IDs, exact decision/legacy/invariant-coverage ledgers, SyncBat's required internal planes when source exists, and early source-debt patterns.

`materialize.rs` reads the same typed architecture seed and publishes the Gate-0 Cargo workspace -- workspace manifest, package manifests, source doors, and required SyncBat planes -- as one isolated candidate tree at the explicit `--output` root. Both roots are required; there is no default and no positional form. It refuses a shared or nested seed/output pair, parent traversal in either path, a missing seed, and an unresolvable output parent. Every planned path obeys one portable relative-path grammar (no backslash, drive colon, `.`/`..` component, control character, trailing dot or space, Windows device name, or case-fold twin), and the binary is bound to the exact `SPEC.sha256` it was compiled against: a seed carrying a different manifest is refused before any plan is built. The only successful dispositions are `Created` (absent output, complete staged tree renamed into place) and `Unchanged` (the output already carries exactly the planned tree; zero writes); a divergent existing output is refused, never repaired. It is a one-time factory, not a code generator, synchronizer, or semantic owner.

`receiptcheck.rs` is the independent Tier 0 qualification verifier (5.5E6b). `selftest.py` is the evidence PRODUCER: it runs every Tier 0 gate against a clean export, retains the concrete evidence (executables, the materialized candidate tree, a compile-fail law-fixture ledger), and writes the canonical line-oriented `qualification.t0` artifact (`Tier0QualificationArtifactVersion`, `BATPAK-TIER0-QUALIFICATION/1`). `receiptcheck.rs` then recomputes every digest from the bytes on disk, checks each against the artifact's claims under a strict ASCII/LF grammar, and only then calls the sealed `spec::bootstrap_qualification::verify`, which enforces the `Tier0ReceiptKind::ALL` denominator, per-kind artifact policy, single target, source posture, and hosted-run requirement. selftest holds no admission predicate of its own: a dishonest receipt is caught by the verifier, not by Python. The one denominator is the typed owner in `spec/bootstrap_qualification.rs`; the authoritative `x86_64-pc-windows-msvc` receipt is produced by the hosted CI lane, and a local run produces the supplemental `x86_64-pc-windows-gnu` frozen-export lane.

Seed auditing and freezing are the signed seed repository's commands (`audit.py`, `freeze.py`, `seedcheck.rs` above); the candidate carries only its own justfile targets -- `check`, `check-no-std`, and `smoke`. Qualification of the candidate runs on a disposable copy so Cargo lockfiles and build artifacts never touch the qualified tree, and every target-sensitive command carries an explicit `--target` (the authoritative triple is `x86_64-pc-windows-msvc`):

```sh
cargo metadata --no-deps --format-version 1
cargo check --target x86_64-pc-windows-msvc --workspace --all-targets
cargo check --target x86_64-pc-windows-msvc -p batpak --no-default-features
cargo check --target x86_64-pc-windows-msvc -p syncbat --no-default-features
cargo run   --target x86_64-pc-windows-msvc -p batpak-examples --bin family_smoke
```

These tools never inspect private reasoning and never confer semantic correctness merely because structure passes.

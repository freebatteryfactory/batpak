# MacBat — the macro lane

MacBat is the batpak family's declarative-contract macro subsystem, built as a
**nested Cargo workspace** quarantined behind its own `Cargo.lock` and
`target/`. It is excluded from the parent `bpk-lib` workspace
(`bpk-lib/Cargo.toml` `[workspace].exclude`) and is the one place a compiler runs
before the J1/J2 integration event — so it depends on **nothing in `batpak`
core**.

## The three crates

| Directory | Package | Role |
|---|---|---|
| `compiler/` | `macbat-compiler` | The pure semantic compiler. Owns `ContractIr`, the grammar, diagnostics, the eight passes, lowerings, snapshots, origins, budgets, and hygiene. Unconditionally depends on syn/quote/proc-macro2. |
| `macros/` | `macbat` | The proc-macro front doors (`proc-macro = true`). Thin adapters only — each forwards raw tokens to `macbat-compiler` and lowers the returned artifact. No logic. |
| `registration/` | `macbat-registration` | A **temporary** phase bridge (syn-free): a verbatim-in-behavior move of the runtime registration/collision types plus one additive hard-signal helper. Carries `DEATH_WARRANT.md`; deleted in crate 3. |

The nesting is not a Cargo namespace — the three members are listed explicitly in
`Cargo.toml`.

## Dependency law

- `macbat` (front doors) → `macbat-compiler`. Nothing depends back on `macbat`.
- `macbat-compiler` depends only on syn/quote/proc-macro2. It **never imports
  core**, does no I/O, and its `src/` tree is a **pure function**: no
  `std::fs`/`std::env`/`std::time`/`std::net`, no `rand`, no `SystemTime`, no
  thread-locals, and no `HashMap` in any emit/normalize path (`Vec`/`BTreeMap`/
  `BTreeSet` only, for deterministic output). Disk lives in `tests/`,
  `examples/`, and `specimens/`.
- `macbat-registration` is syn-free and duplicates the legacy `macros-support`
  behavior by design during the bridge.

## The pipeline

Eight typed passes, with cross-cutting concerns threaded through them:

```
tokens → declaration → validation → ir → lowering → emit → normalize → tokenize
         (identity / diagnostic / budget / metrics / trace / origin / grammar /
          hygiene / artifact cross-cut)
```

`macbat_compiler::compile_derive` and `compile_attribute` are the only entry
points; each runs passes 1..8 and folds any diagnostics into the returned
`ExpansionArtifact` (whose `outcome` makes "tokens AND diagnostics"
unrepresentable).

## Identity is local

A `ContractId` is **local to one expansion artifact** — derived from the
declaration's kind and ident only. A pure proc-macro pass cannot know the
caller's crate or module path, so it claims none. Global resolution (crate +
module path, a `ResolvedContractId`) is Muterprater / Repo-IR enrichment owned by
crate 3, not defined here. `Owner::caller_module()` is the symbolic default
provenance for the same reason.

## Lowering extension contract

Adding a contract kind later happens in **that kind's own crate's packet — never
here**. It is a pure addition:

1. add a `ContractKind` variant;
2. add a `ContractKindIr` variant + `ir/<kind>.rs` with `build`;
3. add `lowering/<kind>.rs` implementing `ContractLowering` + a new arm in
   `lowering::plan`'s exhaustive match + any new `LoweringRole` variants;
4. add one `AttributeGrammar` const in `grammar/catalog.rs` + a `grammar_for`
   arm;
5. add a specimen directory.

No existing arm changes semantics. Because wildcard/named catch-all arms over an
enum are denied repo-wide, a new `ContractKindIr` variant **mechanically forces**
a new `plan` arm or the build fails — that compiler-enforced non-exhaustiveness
*is* the extension contract.

The other reserved kinds — `Effect`, `StateMachine`, `Evidence`, `Codec`,
`IntegrityDomain`, `PlatformCapability`, `DependencyFacade`, `Module` — exist
**only** as this documented list: zero speculative structs, zero empty drawers,
until their owning packet lands.

## Building the lane

`xtask` depends on core and cannot run before integration, so the lane drives
**raw cargo** (from the repo root, via `just`):

```
just macro-lane-check              # cargo check --workspace
just macro-lane-test               # cargo test --workspace
just macro-lane-snapshot -- --update   # regenerate byte-exact snapshots
just macro-lane-expand <specimen>  # step one specimen through the passes
```

Or directly: `cd bpk-lib/crates/macbat && cargo check --workspace`. The lane is
edition 2024; the rest of the workspace is not.

## Warrants

`macbat-registration` is a `TemporaryLegacyMirror` / `InternalImplementation`
bridge and carries `registration/DEATH_WARRANT.md` (crate-3 deletion; successor:
the Contract-IR composition proof plus Muterprater collision/upcast facts). The
legacy `bpk-lib/crates/macros` + `macros-support` crates remain in tree, untouched,
during crate 1; their death warrant fires in crate 2 when core re-points its
`pub use` to `macbat`.

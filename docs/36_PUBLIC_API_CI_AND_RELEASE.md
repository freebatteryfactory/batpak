---
status: AUTHORITATIVE
contract_id: BP-PUBLIC-API-CI-RELEASE-1
authority_scope: public API ownership, semver, MSRV, CI lane responsibilities, release seal, and refusal
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-14
reconciliation_epoch: cleanroom-v1
---

# Public API, CI, and Release

This document is clean-room operating law: who owns the public surface, how it
changes, what must be proven, where proof runs, and what a release binds. It is
not a new durable behavior obligation, so it introduces no `LEG-*` row.

## Ownership split

```text
TestPak   owns WHAT must be proven
CI        chooses WHERE and WHEN those TestPak lanes execute
Release   consumes fresh, matching receipts
```

Exact GitHub Actions filenames and job layouts are a projection of TestPak's
command and lane policy, never the authority. A workflow file may be
reorganized freely as long as it invokes the same TestPak lanes.

## Public API (DEC-056)

TestPak owns public API baselines, semver classification, MSRV qualification,
and target/profile qualification. Every publishable package gets:

```text
generated public API baseline
dependency-type leakage scan
semver change classification
downstream compile fixture
deprecation/removal disposition
```

A public removal requires all of:

```text
successor
migration example
deprecation interval
downstream audit
decision receipt
baseline update
```

Delete-and-discover public API change is refused.

## Compatibility scope (DEC-059)

Compatibility applies to durable legacy data and intentionally preserved public contracts. It does not require keeping superseded internal crate topology, HostBat [STALE-REF: DEC-009], a standalone Bat VM [STALE-REF: DEC-004], FileBat [STALE-REF: DEC-003], an OS-backend Bvisor [STALE-REF: DEC-010] matrix, or dual execution models alive.

Semver is a contract for users of the current architecture, not a hostage
negotiator for dead architecture.

## CI lanes (DEC-057)

Lane responsibilities are frozen; their host placement is not.

```text
PR-fast
    seedcheck / audit / freeze
    format / clippy
    unit / property / model tests
    public API diff
    no_std semantic checks
    wasm compile check
    affected hostile fixtures

merge
    workspace feature/profile matrix
    integration and compatibility corpus
    bounded Loom/schedule exploration
    fuzz smoke
    affected mutation shards
    examples

scheduled
    full mutation
    semantic fuzz corpus
    browser/workerd runtime
    cross-platform adapters
    performance envelopes
    Linux chaos where applicable

release
    clean pinned container/toolchain
    full MSRV and target matrix
    public API + semver
    all compatibility readers/goldens
    package-content / license / SBOM
    fresh matching gauntlet receipts
    examples and downstream fixtures
    exact published artifacts
```

The TestPak AST enforcement gate (`12_TESTPAK.md`, DEC-068) runs in the PR-fast structural lane and again in the merge lane; an unresolved AST finding is a release-refusal condition. The bootstrap lexical scan (DEC-067) runs in every lane as defense in depth, but the AST gate is the authoritative Rust classification.

The compatibility surface is the real one — examples, fuzz, Loom/chaos, public
API/semver, WASM, Windows, and release/preflight parity — not merely a
compile-green toolchain bump. Release-grade proof is freshness-bound, hostile,
anti-vacuous, and tied to the exact changed trust boundary.

## Tier 0 qualification evidence (5.5E6b)

The bootstrap gate qualification is itself evidence, not a green checkmark. A run
produces a canonical, line-oriented `qualification.t0` artifact
(`Tier0QualificationArtifactVersion`, `BATPAK-TIER0-QUALIFICATION/1`, owned in
`spec/bootstrap_qualification.rs`) that binds, for one physical target: the
source posture (a real git checkout for the authoritative lane, or a frozen
`git archive` export for a supplemental local lane), the exact toolchain, the
hosted-run identity when the target is authoritative, and one receipt per
`Tier0ReceiptKind::ALL` carrying typed stage outcomes and concrete
artifact-evidence digests.

`bootstrap/selftest.py` is the evidence PRODUCER; `bootstrap/receiptcheck.rs` is
the independent VERIFIER. receiptcheck links the real `spec` rlib, recomputes
every digest from the bytes on disk, checks each against the artifact's claims
under a strict ASCII/LF grammar, and only then calls the sealed
`spec::bootstrap_qualification::verify`. The producer holds no admission
predicate: membership in the denominator, per-kind artifact policy, single
target, source posture, and the hosted-run requirement for the authoritative
`x86_64-pc-windows-msvc` target are all decided by the typed verifier. This lane
owns which hosted target and run posture may close a gate; the receipt algebra
(`BP-RECEIPTS-1`) owns what a receipt means, and the bootstrap denominator
(`BP-BOOTSTRAP-1`) owns which gates the qualification requires. A qualification
that names the authoritative target without a hosted-run identity, or that rides
a frozen export, is refused — a local machine produces only the supplemental
`x86_64-pc-windows-gnu` lane, never the authoritative receipt.

## Release seal and refusal (DEC-058)

A release receipt binds every field of the typed inventory
(`spec/architecture.rs`, `ReleaseSealField`) — this list is a generated
projection of it. `KernelQualificationSet` is mandatory even when empty: an
empty set states "no kernels admitted" (KernelImplementationId +
KernelQualificationReceiptId per admitted kernel), it never disappears from
the schema. An admitted row binds the `KernelContractId` it implements, the
`QualifiedKernelId` it earned, the `KernelInterfaceHash` of the interface it
was qualified against, and the `KernelManifestVersion` of the manifest that
carried it.

<!-- RELEASE-SEAL:BEGIN generated from spec/architecture.rs by bootstrap/project.py; do not edit -->
```text
SourceTree
Toolchain
DependencyGraph
GeneratedFacts
CompatibilityCorpus
TestDispositions
MutationDispositions
FuzzDispositions
BenchmarkDispositions
CompilerAssumptionLedger
DependencyLedger
KernelQualificationSet
PackageContents
PublicApi
Sbom
LicenseEvidence
ProofFreshness
```
<!-- RELEASE-SEAL:END -->

A release is refused when any of these holds:

```text
an unclassified denominator row
a stale or missing proof receipt for the changed boundary
an unresolved public API/semver classification
a compatibility reader with no removal/retention receipt
a package-content, license, or SBOM gap
```

## Publication and install names

```text
library crate:   batpak      (cargo add batpak)
product binary:  batpak-cli  (cargo install batpak-cli)
```

Do not claim `cargo install batpak` unless package naming is deliberately
reopened. `batpak-examples` and `testpak` are non-publishable and never ship as
release artifacts.

## MSRV and targets

MSRV is a qualified, tested floor, not a comment. The target/profile matrix
(the `no_std + alloc` semantic profiles and the native/browser adapter
profiles) is qualified per target; one platform's proof never blesses another.
Exact MSRV and target constants are selected at their owning gate with evidence.

## Generated-proof compatibility (DEC-070)

The Guarantee Graph (`docs/GUARANTEE_GRAPH.generated.md`) and the generated SEED classification and operator blocks are frozen, audited, derived artifacts, not public API surface. They carry no compatibility promise of their own; every guarantee resolves to its owning fact. Release qualification regenerates and re-audits them; a hand-edited generated artifact fails the gate.

## Numeric compatibility (DEC-069 / docs/37)

The additive BatQL V1 numeric type-vocabulary (`FixedDecimal`, `ExactRatio`, `ApproximateBinary`, `Interval`, and the `Binary32`/`Binary64` format identities) and the `FLOOR`/`CEILING` rounding spellings are recorded language changes with additive-only compatibility: no existing valid expression changes meaning. Public numeric surfaces preserve exact format tags and raw bits, approximate values never gain silent authority, and release qualification covers the numeric profile identities and the exact-versus-approximate crossing contracts.

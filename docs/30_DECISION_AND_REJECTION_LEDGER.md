---
status: AUTHORITATIVE
contract_id: BP-DECISION-LEDGER-1
authority_scope: KEEP, LOCK, KILL, SUPERSEDE, DEMOTE, DEFER, and implementation dispositions
supersedes: BatPak clean-room Pass 1 and selectively retained Pass 2 rulings
last_reconciled: 2026-07-13
---

# Decision and Rejection Ledger

## Tags

```text
KEEP                 retained as current architecture
LOCK                 retained and frozen against casual reopening
KILL                 forbidden from the clean target
SUPERSEDE            old meaning survives through a named successor
DEMOTE                retained only for compatibility, oracle, or evidence
DEFER                 outside V1; requires a real adopter and new ruling
OPEN-IMPLEMENTATION  owner/law fixed; exact constant selected at named gate
RETAIN-AS-EVIDENCE   historical material may be consulted, never treated as law
```

## Architecture decisions

| ID | Tag | Subject | Final ruling |
| --- | --- | --- | --- |
| DEC-001 | LOCK | Pass 1 machine-centered architecture | Base of final v1 |
| DEC-002 | RETAIN-AS-EVIDENCE | Pass 2 package-stratified architecture | Useful ideas salvaged individually; package map rejected |
| DEC-003 | KILL | Separate storage-format package | `.fbat` remains owned by `batpak` |
| DEC-004 | KILL | Standalone runtime VM package | PakVM and Bvisor live inside SyncBat |
| DEC-005 | SUPERSEDE | VPak as machine name | PakVM is machine; `.vpak` is package extension |
| DEC-006 | LOCK | SyncBat internal shape | `runtime`, `pakvm`, `bvisor`, `world`, `port` |
| DEC-007 | KILL | Universal type directory | Root concept file + same-name directory |
| DEC-008 | KILL | Universal tables/systems/ports/kernels tree | Put mechanics under the concept they serve |
| DEC-009 | SUPERSEDE | HostBat | WorldImage, ProgramImage, lifecycle events, ProcessContract, NetBat |
| DEC-010 | SUPERSEDE | Bvisor OS backend product matrix | Capability/attempt membrane over PakVM |
| DEC-011 | KEEP | `.fbat` | BatPak append-oriented source-truth format |
| DEC-012 | KEEP | `.vpak` | Immutable WorldImage package format |
| DEC-013 | KEEP | BatQL compiler package | Independent frontend over BatPak image contracts |
| DEC-014 | LOCK | TestPak role tree | Role folders; proof classes are metadata |
| DEC-015 | LOCK | Muterprater scope | Mutation testing only inside TestPak |
| DEC-016 | KEEP | Status/supersession metadata | Required on all docs |
| DEC-017 | KEEP | Bounded push + durable pull | Core delivery doctrine |
| DEC-018 | KEEP | K3 | Truth/decision handling only where unresolved data matters |
| DEC-019 | KEEP | NC¹ circuit lowering | Eligible bounded decision fragments only |
| DEC-020 | DEMOTE | Serde/MessagePack | Interop and historical readers, not new semantic authority |
| DEC-021 | DEMOTE | Flume | Private reference mechanism during topology-by-topology succession |
| DEC-022 | DEMOTE | UUID crate | Differential oracle until owned generator closes |
| DEC-023 | DEFER | Arbitrary native/Wasm guest execution | Future external-effect adapter only after real adopter |
| DEC-024 | KEEP | Artifact/content plane | Concern and port; no public bundle extension yet |
| DEC-025 | KEEP | Thin CLI package | Binary adapter only, no semantic ownership |
| DEC-040 | LOCK | BatPak and SyncBat semantic profiles | `no_std + alloc`; host mechanisms stay in explicit std/browser adapters |
| DEC-041 | LOCK | Cargo package `batpak` | The real semantic and durable library, not an empty umbrella; repository and library share the product name |
| DEC-042 | DEFER | BatDenseRecord representation | Requires a sealed-image or measured-kernel adopter; not part of the initial native authority path |
| DEC-043 | LOCK | Examples package batpak-examples | Public-surface witness at examples/; publish is false; owns no semantic law; no TestPak dependency; production APIs only; observable output; executed through TestPak orchestration |
| DEC-044 | LOCK | TestPak sole corpus and fixture home | crates/testpak/corpus and crates/testpak/fixtures own frozen assets; root corpus/ and fixtures/ are forbidden; no canonical package-local mirrors |
| DEC-045 | LOCK | TestPak binary target | The testpak package ships src/lib.rs and src/bin/testpak.rs; the root justfile projects cargo run -p testpak; no separate command crate |
| DEC-046 | LOCK | Workspace version train | 1.0.0-alpha.1 implementation train; publishable production packages ship lockstep to 1.0.0; versions below the train are rejected |
| DEC-047 | LOCK | Default feature behavior | batpak and syncbat default to std with useful native usability; no_std via default-features false; std must not activate the threaded browser encryption mapping or interop adapter zoo |
| DEC-048 | LOCK | Orphan clean-room cutover | cleanroom is orphan with zero parents; legacy/main owns the final legacy source lineage; legacy-pre-cleanroom is the immutable legacy tag; no source merge graft cherry-pick or subtree import crosses; behavior crosses only through LEG obligations and clean witnesses; legacy source 6c4a46f8; clean-room base ada284a0 |
| DEC-049 | LOCK | Declared WorldImage entrypoint invocation | InvokeEntrypoint executes a ProgramImage bound by an admitted WorldImage entrypoint; persistent deployment; ASK or DO; effects only when the entrypoint contract declares them; capability from the WorldImage grant; a request cannot substitute a foreign ProgramImage while keeping entrypoint authority; identity binds WorldImageId WorldInterfaceHash EntrypointId ProgramImageId InvocationId AttemptId |
| DEC-050 | LOCK | Ephemeral query ProgramImage invocation | ExecuteQueryProgram executes a client-supplied canonical query-only ProgramImage ephemerally under a server-issued read-only query capability envelope; effects process installation and lifecycle hooks forbidden; the server validates the image independently and client compilation grants no trust; response binds ProgramImageId query authority digest historical cut source generation completeness proof disposition actual work result digest and receipt; ALLOW DENY DEFER may be data but never drive an effect |
| DEC-051 | LOCK | Source compilation and raw-BatQL authority boundary | BatQL source text is authoring input never executable authority; PakVM executes validated ProgramImages not source; ordinary NetBat permits InvokeEntrypoint and ExecuteQueryProgram and rejects CompileAndExecuteRawBatQL EvalText and ExecuteSource; source compilation is a separately admitted dev and admin CompilerPort not available to guests not implied by NetBat access and absent from the default server profile; it emits ProgramImage plus CompilationReceipt through the ordinary validation and admission path so source identity is provenance not runtime authority |
| DEC-052 | LOCK | Integrity role separation | Checksum32 ContentDigest CommitmentDigest MacTag Signature and ExternalAnchor are six distinct types; a checksum never implies authority; an unkeyed digest never authenticates its own expected value |
| DEC-053 | LOCK | Secret authority vocabulary and no homemade crypto | batpak::secret owns KeyScope KeyId KeyGeneration KeyBackendId KeyState ShredTransition RotationId RewrapId and AntiRollbackPolicy; encrypted payloads require a qualified AEAD; entropy enters only through EntropyPort; no homemade cryptographic primitives; raw secrets never enter .fbat .vpak WorldImage ordinary receipts proof manifests logs Debug or Display |
| DEC-054 | DEFER | Password-derived key material | V1 accepts no human password or passphrase as key material; no password KDF is part of the V1 runtime contract; a KDF enters only if a later explicit password-derived-key feature earns one through the amendment path |
| DEC-056 | LOCK | Public API ownership and semver | TestPak owns public API baselines semver classification MSRV qualification and target profile qualification; every publishable package gets a generated baseline dependency-type leakage scan semver classification downstream compile fixture and deprecation disposition; a public removal requires successor migration example deprecation interval downstream audit decision receipt and baseline update; delete-and-discover public API change is refused |
| DEC-057 | LOCK | CI lanes versus proof ownership | TestPak owns what must be proven; CI chooses where and when the lanes execute; release consumes fresh matching receipts; exact GitHub Actions filenames and job layouts are a projection of TestPak lane policy not authority |
| DEC-058 | LOCK | Release seal and refusal | A release receipt binds source tree toolchain dependency graph generated facts compatibility corpus test mutation fuzz and benchmark dispositions unsafe and dependency ledgers package contents public API SBOM license and proof freshness; a release is refused on an unclassified denominator row a stale receipt an unresolved semver classification a compatibility reader without a removal receipt or a package-content license or SBOM gap; the library crate is batpak and the product binary is batpak-cli |
| DEC-059 | LOCK | Compatibility scope | Compatibility applies to durable legacy data and intentionally preserved public contracts; it does not require keeping superseded internal crate topology HostBat a standalone Bat VM a separate storage-format package an OS-backend Bvisor matrix or dual execution models alive; semver is not a hostage negotiator for dead architecture |
| DEC-060 | LOCK | Typed arithmetic and rounding law | BatQL arithmetic is dimensionally typed: same-unit addition and subtraction; multiplication requires a dimensionless operand; division of like dimensions yields Ratio; Money times Money and cross-currency operations are rejected; every scale-reducing conversion names a rounding mode HALF_EVEN HALF_UP DOWN or UP; overflow division-by-zero and unsupported scale yield INVALID; illegal units are a compile-time type error; currency conversion needs an explicit function and a receipted rate source |
| DEC-061 | LOCK | Deterministic HLC query ordering and range law | HLC is causal chronological evidence never journal progress authority; ORDER BY HLC lowers to a total key HLC then GlobalSequence within one journal and HLC then StoreId then GlobalSequence across journals; DURING HLC ranges are half-open; HLC is forbidden for durability coverage cursor resume frontier advancement commit identity deadline measurement and implicit causation; when causal HLC is absent the result is Unavailable and ObservedWallTime is never promoted into Hlc |
| DEC-062 | LOCK | Kernel identity and binding policy | FROM KERNEL refers to a canonical KernelManifest never a display name Rust path or function pointer; KernelContractId KernelInterfaceHash KernelImplementationId and KernelQualificationReceiptId are distinct; WorldImage binding is either ExactImplementation pinning KernelImplementationId or QualifiedInterface pinning KernelContractId plus KernelInterfaceHash with an accepted qualification receipt; PakVM never resolves a kernel by string name and the AttemptReceipt records the exact KernelImplementationId used |
| DEC-063 | LOCK | V1 compression posture | V1 .fbat authority frames and .vpak canonical sections are uncompressed; large payloads use ArtifactRef; derived tiles may use an explicit versioned compression profile; transparent compression is forbidden; a future compressed profile requires CompressionId canonical parameters bounded decompression a maximum expansion ratio a digest-coverage law compatibility rows hostile fixtures and a measured adopter and must state whether identity covers compressed bytes uncompressed bytes or both; no compression algorithm is selected now |
| DEC-064 | LOCK | Version identities remain distinct | BatQlLanguageVersion ProgramImageVersion WorldImageVersion PakVmIsaVersion FbatFormatVersion BatTaggedRecordVersion NetBatProtocolVersion KernelManifestVersion ReceiptSchemaVersion and SchemaVersion are distinct types; no generic Version crosses subsystem boundaries; canonical bytes carry their owning version; ProgramImageId commits to canonical bytes including version and is independent of compiler provenance so identical bytes from two qualified compilers share an id; NetBat protocol negotiation is independent of PakVM ISA and WorldImage versions |
| DEC-065 | LOCK | no_std and alloc realization matrix | batpak and syncbat realize their no_std plus alloc semantic surfaces not merely assert them; std and browser adapters own filesystem mapping threads sockets clock entropy keychain and web APIs; default-features false is compile-proven and a green default build is not sufficient evidence; collections use Vec bounded arenas BTreeMap ordered maps and sorted catalogs while hash maps are derived caches whose iteration never influences canonical identity formatting diagnostics execution order or receipts; the qualification matrix covers batpak no_std syncbat no_std native std default threaded browser encryption and interop |
| DEC-066 | LOCK | Typed resource exhaustion and no-partial-publication | BudgetExceeded and ResourceExhausted are distinct outcomes never collapsed into each other a string or INVALID; INVALID means a value violated its contract while resource exhaustion is an execution failure; every attacker-influenced allocation decodes the declared length checks arithmetic enforces the semantic bound try_reserves initializes and publishes only after completion; failure never panics aborts wraps or leaves a partial value event artifact or checkpoint; this covers .fbat .vpak BatQL PakVM arenas group and match state NetBat framing proof bundles artifact staging and tile materialization |
| DEC-067 | LOCK | Bootstrap lexical debt scan and generated lint posture | seedcheck lexically scans production source under crates apps and examples after stripping comments and strings for the panic unwrap expect and dbg tokens while test-owned paths tests benches fixtures corpus and fuzz are exempt; the generated workspace denies wildcard_enum_match_arm cast_possible_truncation cast_sign_loss missing_errors_doc large_enum_variant clone_on_ref_ptr and needless_pass_by_value but never globally denies expect_used and never blanket-bans as casts; pointer and layout casts belong to the compiler-assumption ledger and the TestPak AST gate |

## Behavior decisions

| ID | Tag | Subject | Final ruling |
| --- | --- | --- | --- |
| DEC-026 | LOCK | Immutable accepted history | Corrections are later events |
| DEC-027 | LOCK | Read-time schema evolution | Historical bytes remain unchanged |
| DEC-028 | LOCK | Turn versus Attempt | Logical and physical execution identities remain distinct |
| DEC-029 | LOCK | Runtime/Bvisor restart split | Runtime decides legality; Bvisor performs mechanics |
| DEC-030 | LOCK | Availability axes | Value state, truth, decision, completeness, freshness, proof are separate |
| DEC-031 | LOCK | HLC scope | Chronology/filtering aid, not durability or universal order |
| DEC-032 | KEEP | ECS/data orientation | Implementation algebra, not ontology |
| DEC-033 | KEEP | SIDX successor | First earned column tile |
| DEC-034 | KEEP | One evaluation, many projections | Value/explanation/evidence/margin/cost share typed evaluation |

## Implementation constants

| ID | Tag | Subject | Gate |
| --- | --- | --- | --- |
| DEC-035 | OPEN-IMPLEMENTATION | EventFrameV2 field IDs and exact bytes | G2 |
| DEC-036 | OPEN-IMPLEMENTATION | `.vpak` section and PakVM opcode numbers | G5 |
| DEC-037 | OPEN-IMPLEMENTATION | First persistent browser adapter | G5/G7 |
| DEC-038 | OPEN-IMPLEMENTATION | Performance/work thresholds | owning gate + TestPak |
| DEC-039 | DEFER | Public packed-artifact extension | real adopter required |
| DEC-055 | OPEN-IMPLEMENTATION | Exact cryptographic constants | G2 |

## Reopening rule

A locked or killed decision reopens only through a signed amendment containing new evidence, affected obligations, compatibility impact, proof changes, migration plan, and explicit supersession. A code agent cannot reopen it through implementation convenience.

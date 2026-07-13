# ADR-0037: Compaction Publication Is A Namespace Transaction

## Status

Accepted; 0.11.0 pre-1.0 breaking window. Records the GAUNT-NAMESPACE-DURABILITY
(#177) protocol repair, the branded generation/publication token law (#195 slice),
the public idempotency-authority export/restore seam (#188), and the
simulation/conformance consequences (#179). Companion to ADR-0036 (untrusted
manifests carry no authority) and the #189/#205 authority protocol in this same
cut; ADR-0005 (test-support surface) governs the new conformance feature boundary.

## Context

The pre-repair compaction retirement path used FINAL-NAME PRESENCE as a commit
signal. A replacement segment was materialized directly under (or renamed early to)
its final merged segment name — an id it SHARED with a source segment — and
recovery's roll-forward predicate in `cold_start/rebuild/topology.rs::segment_paths`
was "the final merged filename exists". The final pathname was therefore ambiguous
three ways: original source, partial replacement, or committed replacement. A crash
between materialization and authority publication could expose a final-named
replacement that recovery trusted while the idempotency authority, source
retirement, and parent-directory durability were not yet committed.

The byte simulation could not even represent the failure, because the fault-injecting
filesystem modeled `sync_parent_dir` as always durable — namespace truth was assumed
for free. The existing suite's byte-equality oracle therefore passed on the wrong
states (#197's false-green finding). The in-process order compounded it: the live
in-memory index was swapped BEFORE the authority image was published, so a
post-swap flush failure left `memory=new, disk=old` while the store kept serving
commands.

## Decision

### 1. Final filename = truthful publication fact

> A segment may occupy its final committed filename only when that name is a
> truthful publication fact: the frames are completely written, synced, and sealed;
> the file is parent-directory durable; the replacement is bound to a published
> idempotency authority image; and it no longer depends on any source file recovery
> would exclude. File existence alone never authorizes roll-forward.

No future optimization may weaken this sentence. The recovery predicate "final merged
filename exists → roll forward" survives ONLY because the writer protocol now makes
the implication true; any weakening requires revisiting this ADR.

### 2. The staged-name protocol

Compaction is one recoverable namespace transaction:

1. Mint the branded transaction token and write the pending marker (§3).
2. Rename the merged-id source away to `.compact-src` and parent-sync — from here
   until the commit rename the final name is ABSENT.
3. Materialize the replacement under the staged name `NNNNNN.fbat.compact-new`
   (`COMPACT_STAGED_EXTENSION = "compact-new"`), fully write, `sync_all`, seal, and
   parent-sync it — name-durable before anything is published.
4. Publish the idempotency authority image (the #189 authorize → publish → finalize
   token protocol), evicted against a SHADOW clone of the durable map marked against
   the fresh staged-view index — the live map keeps serving appends.
5. Authorize the commit in `store.meta` (§3): the finalize meta write and the
   compaction commit record are ONE atomic `store.meta` replace.
6. Publish the final namespace: atomically rename `.compact-new` → the final segment
   name, then parent-sync.
7. Swap the live in-memory index in ONE publication.
8. Retire the source artifacts (only now — after the committed generation is
   recoverable), then parent-sync.
9. Clear the pending marker LAST.

Parent directories sync at every publication point. A `CompactStaged`
`StoreFileKind` classifies the staged name so it is excluded from every copy/fork
set and cleared from destinations; a leftover `.compact-new` or `.compact-src`
WITHOUT a marker is inert and cleaned by the next compaction.

**The in-memory commit point.** Any failure AFTER the on-disk commit (steps 4–6)
but BEFORE the memory swap (step 7) POISONS the store: the writer refuses all
further commands with a typed error and reopen is required. No return path may leave
`memory=new, disk=old` or `memory=old, disk=new` while the store serves commands.
This is a deliberate rewrite obligation — the old code swapped memory before
authority publication.

### 3. The branded generation/publication token (#195 slice)

The transaction carries branded identity types (`store/generation_ids.rs`), never
raw `u128`s that could be compared across roles: `CompactionId`, `AuthorityImageId`,
and (where lineage was a raw id) `StoreLineage`. There is NO `From`/`Into` between
brands or with raw `u128`; comparison across brands is unrepresentable.

The token binds the staged replacement set, the source segment set, the authority
image, the store lineage, the transition marker, and the final publication state.
Three artifacts carry the SAME branded ids and all three must agree:

- **The marker** (`compaction.pending`) — a canonical versioned BINARY artifact:
  dedicated magic + `u16` version + CRC32, payload `{compaction_id, lineage,
  merged_segment_id, sorted source_segment_ids, expected authority_image_id}`,
  published atomically (temp → persist → parent sync). Corrupt or future-version
  bytes are typed `MarkerCorrupt` / `MarkerFutureVersion` refusals.
- **The store.meta commit record** — `last_compaction_commit: Option<CompactionCommit
  { compaction_id, authority_image_id, merged_segment_id, source_segment_ids }>`,
  required-present in the encoding (value may be `None`), stating the last durably
  AUTHORIZED transition. It is written in the SAME atomic replace as the idempotency
  finalize.
- **The on-disk authority image** — stamps the `CompactionId` INSIDE the image, not
  only in `store.meta`.

Recovery direction is decided by three-artifact agreement, never by file presence:
marker `compaction_id` matches `last_compaction_commit.compaction_id` AND the on-disk
image agrees → roll FORWARD; marker with a non-matching or absent commit → roll BACK;
any cross-artifact disagreement (right meta + wrong/older image, transplanted
lineage, …) → a typed `StoreError::CompactionRecoveryRefused { marker_path, kind }`.
No at-or-past / frontier admission rule is resurrected under a new name.

Explicit boundary: this is NOT the signed authority manifest. Cryptographic binding
and external anti-rollback witnesses (#190/#191/#193/#196) are deliberately NOT
built here.

Recovery is PHYSICAL and runs at writable open under the store-dir lock; a read-only
open with a pending marker refuses (`RepairRequiresWritableOpen`). Repair reopens as
exactly the old complete generation, the new complete generation, or a typed
refusal — never a mixed accepted generation, and never healed by a clean `close()`.
`Store::inspect_recovery_state(config)` reports the committed generation, marker
summary, and required writable action WITHOUT constructing `Store<Open>` or mutating
disk.

### 4. Namespace-aware simulation + conformance (#177/#179)

The fault-injecting simulation filesystem now models NAMESPACE truth separately from
BYTE truth: directory entries created, renamed, removed, and parent-dir-synced each
carry their own durability state, so `sync_parent_dir` is no longer always-durable.
A simulated crash independently produces a staged name present with the final absent,
a rename visible but not parent-durable, stale staged+final coexisting, and a dropped
parent sync at each transition. A deterministic crash fixture exists for every one of
the nine publication boundaries — `MarkerDurableNoSourceRename`,
`SourceRenamesPerformed`, `PartialReplacementMaterialization`,
`StagedReplacementCompleteAuthorityUnpublished`,
`AuthorityPublishedFinalRenameNotDurable`, `FinalReplacementDurableSourcesNotRetired`,
`SourcesPartiallyRetired`, `MarkerNotCleared`, `MarkerCleared` — each reopening as
old-complete / new-complete / typed refusal.

The `StoreFs` conformance corpus and the fault/namespace layer are published behind a
two-tier feature boundary. `conformance-harness` (the clean external surface) exposes
the `store::conformance` corpus + backend factories and the `ShadowFs` wrapper with
its `CrashOp`/`SimOp` vocabulary and crash/replay controls; `dangerous-test-hooks`
(internal only) implies it and additionally exposes the SimFs promotion, the `__sim`
namespace matrix, the fault hooks, and the poison levers. The native, memory, and
wrapped simulation backends are all proven against the SAME corpus, so an
out-of-tree backend is proven-conformant by the law that gates the in-tree ones.

### 5. Public authority export/restore (#188)

`Store::export_idempotency_authority(&self)` returns the ONE canonical opaque
authority image — bytes-equivalent to the durable `index.idemp` v2 image up to its
covered generation and frontier, preserving original receipt identity and
extensions, exposing no embedder-visible tuple schema and no parallel checkpoint
format — as an opaque `IdempotencyAuthorityExport`.

Restore is OFFLINE-PRIMARY: a single closed-directory associated fn
`Store::restore_idempotency_authority(config: &StoreConfig, export: &IdempotencyAuthorityExport)
-> Result<(), StoreError>` acquires the store-dir lock, refuses while a pending
compaction marker / unresolved transition exists, validates lineage and coverage,
rejects rollback, MINTS a fresh local `AuthorityImageId`, republishes through the
canonical authority protocol, updates `store.meta`, and leaves the directory
openable. The restore call IS the authorization ceremony — it never requires the dead
store to have pre-authorized the incoming image's token, so there is no circularity.
Keyed-admission blocking is structural: the directory is closed and locked, so no
keyed append can race a half-restored authority. The earlier live "adopt-and-mint"
door is DELETED.

Restore refuses with one `StoreError::IdempotencyRestoreRefused { reason:
IdempotencyRestoreRefusal }`, each reason a distinct typed variant: a corrupt image,
a future format version, a stale rollback (an image behind the live authorized
state), a foreign lineage, a diverged / farther-ahead sibling (token or anchor
mismatch — frontier arithmetic never admits), entries beyond the image's declared
coverage, and a pending compaction transaction that must be resolved before restore.

Why now: the image was already being redesigned for the #189/#177 authority work, so
finishing the out-of-band seam was cheapest here.

### 6. Test-oracle correction (#197 Phase 0)

Byte-equality of two directories was the wrong oracle — it certified the wrong
states green. It is replaced by state-machine assertions over the recovery law: every
injected-crash fixture reopens WITHOUT a clean `close()` (the mandate forbids
clean-close healing) and classifies as exactly old-complete, new-complete, or a typed
refusal, with no double append, no invented event, and no undead event. The pending
marker is now a BINARY artifact; a legacy JSON `compaction.pending.json` is parsed
for DIAGNOSTICS ONLY and refuses with `CompactionRecoveryRefusal::LegacyMarkerUnsupported`
— there is NO automatic v1 recovery, ever. The assurance machine additionally REDS on
vacuous proof (zero-executed feature-gated binaries, ceremonial regression files,
unfixtured L4 crash clauses, feature-set/test mismatch), and proof receipts record
command / features / binary / executed+skipped counts / corpus identities / SHA /
result.

## Consequences

- Machine law: `INV-COMPACTION-NAMESPACE-COMMIT` (L4; declares the nine crash
  boundaries), `INV-SIMFS-NAMESPACE-TRUTH`,
  `INV-IDEMPOTENCY-EXPORT-RESTORE-FAIL-CLOSED`, `INV-STOREFS-CONFORMANCE-REUSABLE`,
  `INV-PROOF-RECEIPT-COMPLETE`; `INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION`
  rewritten to the staged-name/token protocol.
- `store.meta` is v1 as the completed INITIAL schema before first release — there is
  NO supported earlier v1 reader. A body missing required v1 fields is CORRUPTION
  (fail-closed family), not a tolerated additive-draft body.
- Recovery decision moves fully to the three-artifact commit record; the ambiguous
  "final merged filename exists → roll forward" heuristic and the temp-substitution
  "recovery" (which produced an index whose `DiskPos` entries could not be read back)
  are DELETED.
- A post-disk-commit / pre-memory-swap failure poisons the writer (typed error,
  reopen required) rather than serving a split memory/disk view.
- New public surface (semver-tracked): `export_idempotency_authority`,
  `restore_idempotency_authority` (offline associated fn), `IdempotencyAuthorityExport`,
  `inspect_recovery_state`/`RecoveryInspection`, `handling_class`/`HandlingClass`, the
  `conformance-harness` feature and its items, `StoreError::CompactionRecoveryRefused`
  + `CompactionRecoveryRefusal`, and `StoreError::IdempotencyRestoreRefused` +
  `IdempotencyRestoreRefusal`. `StoreError` stays `#[non_exhaustive]` (no break).
- Leftover staged / `.compact-src` files after a crash under the new protocol are
  inert without a marker (classified, excluded, cleaned by the next compaction); a
  marker drives roll-forward / roll-back / typed refusal.
- No signed manifests, no external anti-rollback witness — deferred to #190–#196.

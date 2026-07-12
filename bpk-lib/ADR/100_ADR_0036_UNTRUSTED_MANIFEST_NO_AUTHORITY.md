# ADR-0036: Untrusted SIDX Manifest Carries No Recovery Authority

## Status

Accepted; 0.11.0 pre-1.0 breaking window. Records the GAUNT-SIDX-NO-SELF-AUTH
(#192) doctrine correction, the resulting recovery-decision rewrite, the
availability consequence for sealed segments with unauthenticated footers
(including legacy SDX2), and the operator posture ruling.

## Context

The untrusted-footer recovery path (ADR-0032's fallback for a CRC-failed SDX3 or
legacy SDX2 boundary) treated the SIDX entry table as a "self-authenticating
manifest": one entry whose `(offset, length, event_hash)` matched an
independently CRC-verified recovered frame was held to anchor the ENTIRE table,
after which sibling rows could attest that a missing trailing committed frame
existed (`FailClosedCorroboratedLoss`), and a table that "agreed the recovered
region was complete" could recover a sealed segment EVEN UNDER the strict
`FailClosed` posture.

That is not a valid authentication argument. BLAKE3 is public and unkeyed: a
party able to read or edit the segment can copy a real frame's stored
`event_hash` or recompute it. Matching one legitimate row proves only that one
row describes one recovered frame — it grants no authority over sibling rows,
`entry_count`, order, the footer boundary, or claims about nonexistent trailing
frames. The design therefore carried BOTH failure duals of the same escalation:

- **false LOSS** — a forged sibling row "proved" a committed frame was missing
  and refused recovery of an intact store (denial-of-service, any posture);
- **false SAFETY** — a truncated-but-agreeing table recovered under STRICT
  posture while a torn committed frame was silently dropped (the exact loss the
  strict posture exists to prevent).

## Decision

### 1. The safe law

> A row corroborated against independently verified frame bytes proves only that
> row's relationship to that frame. It does not authenticate any other row or
> any table-level metadata.

No future optimization may weaken this sentence without a keyed MAC, digital
signature, or independently trusted root covering the complete table (the
authenticated-store spine, #190/#191). Machine form:
`INV-UNTRUSTED-MANIFEST-NO-AUTHORITY-ESCALATION` (invariants.yaml), witnessed by
the forged-sibling fixture.

### 2. The decision rewrite (hint-only fallback)

Issue #192 Layer A, option 2. Under an untrusted footer, SIDX rows are consumed
ONLY as suspicion geometry (offsets claimed at/after the recovered prefix end);
recovery authority comes exclusively from the CRC-verified frame walk plus the
caller's tail posture:

- an EMPTY recovered prefix recovers under any posture (nothing to lose);
- PERMISSIVE (`RecoverTornTail`, granted only to the newest segment at cold
  start) recovers the CRC-valid prefix, recording truncation evidence when an
  untrusted claim (footer claimed end, or a row at/after the prefix end)
  reaches past it;
- STRICT (`FailClosed`, every other caller: sealed non-tail segments at cold
  start, compaction reads, full scans) refuses EVERY non-empty prefix —
  `FailClosedEvidenceOfTruncation` when suspicion is positive, else
  `FailClosedUnprovableTail`.

`FailClosedCorroboratedLoss` is retired — there is deliberately no "proven
loss" outcome, because no unauthenticated row can PROVE a committed frame
existed. The hash-corroboration machinery (the per-frame `event_hash` re-read
walk feeding the retired table-anchoring check) was DELETED rather than left as
unclaimed authority machinery; the untrusted fallback is strictly cheaper as a
result. `recovery_manifest.rs` and `sidx/footer.rs` joined the `segment-scan`
critical mutation seam (L4).

### 3. The availability consequence (accepted deliberately)

Removing the case-(b) "table agrees" recovery means two previously-auto-opening
shapes now refuse at cold start with a typed `StoreError::CorruptSegment`:

- a sealed NON-TAIL segment whose footer corrupted benignly (bit rot in the
  index region, all event frames intact);
- a sealed NON-TAIL segment carrying a legacy SDX2 footer — SDX2 has no CRC at
  all, so "this is just an old format" is exactly as unauthenticated as a
  corrupt SDX3. Pre-0.8.3 stores whose history was never re-sealed under SDX3
  therefore refuse to open under 0.11.0+ until migrated (open under
  batpak <= 0.10.0 and compact — compaction re-seals history under SDX3 — or
  restore from a backup).

Sound refusal wins over unsound availability: the retired recovery in both
shapes rested on forgeable evidence. The tail segment keeps its permissive
torn-tail grant, so crash recovery of the active segment is unchanged.

### 4. Operator posture (owner ruling, 2026-07-12)

The refusal carries the full operator evidence set in its detail: segment id
and canonical filename (`NNNNNN.fbat`), the last independently verified frame
boundary, the fact that no CRC-valid frame decodes past it, the untrusted claim
when one exists, the posture used, the explicit statement that no authenticated
evidence proves the missing region empty, and remediation guidance. The
operator runbook is `cookbook/200_SEALED_SEGMENT_REFUSAL.md`.

There is deliberately NO permissive `StoreConfig` knob for sealed-segment
recovery. A persistent config switch would become an ongoing alternate
consistency mode: open a suspicious historical prefix, resume normal writes,
and permanently build new history over possibly-missing committed history.
ADR-0035 §5's watch item is resolved AGAINST a sticky policy toggle. Any future
escape hatch must be a **one-shot salvage-to-new-directory operation** or a
**refusal-bound authorization token** — an explicit, non-persistent, per-refusal
act that cannot silently become the store's standing posture. Remediation
guidance must never suggest deleting or editing a segment in place.

## Consequences

- The forged-sibling DoS and the strict-posture silent-truncation hole are
  closed; the decision is provably table-blind (dual-run fuzz harness
  `sidx_manifest` compares `resolve_untrusted_frames_end` against the
  table-blind walk on semantically mutated tables).
- Benign sealed-footer corruption and un-migrated pre-0.8.3 SDX2 history now
  require operator action (runbook above) until the authenticated-store spine
  (#190/#191) restores sound automatic recovery.
- `cargo xtask bench --surface neutral --compare` guards the healthy-open fast
  path; the untrusted fallback lost its per-frame hash re-read and got cheaper.

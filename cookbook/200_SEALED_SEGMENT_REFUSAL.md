# Sealed Segment Refusal (Operator Runbook)

Problem: `Store::open` refuses with `StoreError::CorruptSegment` naming a sealed
segment file (`NNNNNN.fbat`) and an "unprovable tail" or "POSITIVE truncation
evidence" detail. This is the strict fail-closed cold-start posture
(GAUNT-SIDX-NO-SELF-AUTH, #192 / ADR-0036): the segment's SIDX footer failed
CRC authentication (or is a legacy un-CRC'd SDX2 footer), and nothing
unauthenticated can prove the frame region complete. Only the newest (tail)
segment gets the permissive torn-tail grant; every older sealed segment refuses
rather than guess.

## Read the refusal before acting

The detail string carries the full evidence set:

- the segment id AND canonical filename (`000007.fbat` lives in the store's
  `data_dir`);
- the last independently verified frame boundary (byte offset) — every frame
  before it decoded under its own CRC;
- the fact that no CRC-valid frame decodes past that boundary;
- the untrusted claim, when one exists (the footer trailer or a SIDX manifest
  row says frames extend further — a torn committed frame MAY sit in that gap);
- the posture used (strict `FailClosed`);
- the statement that no authenticated evidence proves the missing region empty.

An "unprovable tail" refusal means nothing even claims data is missing — the
footer simply cannot vouch for itself. A "POSITIVE truncation evidence" refusal
means an (untrusted) claim reaches past the verified boundary; treat it as a
possible torn committed frame until proven otherwise.

## Remediation, in order of preference

1. **Restore the named segment file from a backup** taken while the store was
   healthy (or restore the whole store directory). Reopen; the restored footer
   authenticates and the refusal clears.
2. **Inspect a COPY of the store directory offline.** Copy the entire
   `data_dir`, and examine the copy — compare the named segment against backups,
   check whether the verified boundary matches the file's expected frame region,
   diff sibling replicas if you have them. Never experiment on the live
   directory.
3. **Legacy SDX2 history** (pre-0.8.3 stores never re-sealed under SDX3): open
   the store under batpak <= 0.10.0 and run compaction there — compaction
   re-seals merged history under the CRC-authenticated SDX3 footer — then
   upgrade. Or restore from a backup taken after such a migration.
4. **Wait for authenticated-format recovery** (#190/#191, signed segment
   seals): a segment whose complete table is covered by a real signature can be
   recovered soundly even when a footer CRC fails. If the data is not urgent,
   parking the directory untouched is a legitimate choice.

## What NOT to do

- **Do not delete or edit the segment in place.** The refused file still holds
  every CRC-valid frame up to the verified boundary; destroying it converts a
  refusal into real loss.
- **Do not hex-edit the footer to make the CRC pass.** That manufactures the
  exact forged authority the refusal exists to reject.
- There is deliberately **no configuration switch** that opens a refused sealed
  segment permissively (ADR-0036 §4): a sticky knob would let new history be
  built over possibly-missing committed history. Any future escape hatch will
  be a one-shot salvage-to-new-directory operation, never a standing mode.

Invariant protected: INV-UNTRUSTED-MANIFEST-NO-AUTHORITY-ESCALATION — no
unauthenticated SIDX row or table can escalate into recovery authority.

Test command: `cargo test -p batpak --test segment_scan_hardening_frame_bounds`.

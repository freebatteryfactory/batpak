//! Fuzz-only decode entry points (GAUNT-FUZZ-1).
//!
//! This module exists **solely** so the workspace-excluded `batpak-fuzz`
//! cargo-fuzz crate (a path dependency built with
//! `--features dangerous-test-hooks`) can reach the **real** on-disk / untrusted
//! DECODE entry points of the store — with no copies. Every wrapper here calls
//! production code directly so a libFuzzer crash is a crash in real parse logic.
//!
//! The whole module is gated behind `#[cfg(feature = "dangerous-test-hooks")]`
//! and `#[doc(hidden)]`, so:
//!   * a default build never compiles it (no production API-surface change), and
//!   * even with the feature on it never appears in published docs.
//!
//! ## Contract for the fuzz target authors
//!
//! Each `__fuzz_*` function takes arbitrary `&[u8]` (plus a small scalar where the
//! decoder needs one) and **must never panic by construction beyond what the
//! decoder itself does** — it simply forwards to the real decoder and returns its
//! `Result`/`Option`/discriminant. The fuzz target asserts no-panic; these
//! wrappers add no assertions of their own. Some return types of the underlying
//! decoders are crate-private, so those wrappers collapse the success value to a
//! `bool` / `&'static str` discriminant — the fuzz contract is "does decoding this
//! arbitrary buffer panic", not "what did it decode to".
//!
//! File-path decoders (those that take a directory / open a file rather than a
//! `&[u8]`) get a wrapper that writes the bytes to a freshly-created `tempfile`
//! tree under the real on-disk filename, calls the real loader, and drops the
//! tempfile on return (RAII cleanup). `tempfile` is a normal `[dependencies]`
//! entry of this crate, so it is available in `src` without any feature plumbing.

use crate::store::StoreError;

// ---------------------------------------------------------------------------
// Direct `&[u8]` decoders.
// ---------------------------------------------------------------------------

/// `encoding::from_bytes::<SegmentHeader>(&[u8])`.
///
/// `encoding::from_bytes` and `store::segment::SegmentHeader` are both already
/// `pub`, so a fuzz target *could* call them directly; this wrapper pins the
/// concrete monomorphization (`SegmentHeader`) into one stable entry point and
/// collapses the decoded header to `()` so the target need not name the header
/// type. Returns the real `rmp_serde` decode error on failure.
#[doc(hidden)]
pub fn __fuzz_segment_header(bytes: &[u8]) -> Result<(), rmp_serde::decode::Error> {
    crate::encoding::from_bytes::<crate::store::segment::SegmentHeader>(bytes).map(|_| ())
}

/// `SidxEntry::decode_from(&[u8], segment_id)` (segment/sidx.rs).
///
/// `SidxEntry` and its decoder are `pub(crate)`; the decoded entry type cannot
/// appear in a `pub fn` signature, so this wrapper discards it and returns
/// `Result<(), StoreError>`. The required SIDX entry buffer length is a fixed
/// `ENTRY_SIZE` (162 bytes); a buffer of any other length is the canonical typed
/// `Err` path. `segment_id` is a free scalar fed straight through to the decoder
/// — the fuzz target can pass any `u64` (e.g. `0`).
#[doc(hidden)]
pub fn __fuzz_sidx_entry(buf: &[u8], segment_id: u64) -> Result<(), StoreError> {
    crate::store::segment::sidx::SidxEntry::decode_from(buf, segment_id).map(|_| ())
}

/// `decode_checkpoint_data(path, version, &[u8])` (cold_start/checkpoint).
///
/// Wraps the `pub(super)` decoder via the crate-visible
/// `checkpoint::__fuzz_decode_checkpoint_data` shim, which supplies a throwaway
/// `Path` internally. `version` selects the checkpoint body version; `body` is
/// the msgpack body. Returns `true` when the decoder produced `Some`, `false`
/// when it returned `None` (the typed "ignore corrupt checkpoint" path).
#[doc(hidden)]
pub fn __fuzz_checkpoint_data(version: u16, body: &[u8]) -> bool {
    crate::store::cold_start::checkpoint::__fuzz_decode_checkpoint_data(version, body)
}

/// `decode_checkpoint_snapshot_v6(path, &[u8])` (cold_start/checkpoint).
///
/// Wraps the `pub(super)` v6 snapshot decoder via the crate-visible shim. Returns
/// `true` for `Some`, `false` for `None`.
#[doc(hidden)]
pub fn __fuzz_checkpoint_snapshot_v6(body: &[u8]) -> bool {
    crate::store::cold_start::checkpoint::__fuzz_decode_checkpoint_snapshot_v6(body)
}

/// `MmapIndexEntry::decode_from(&[u8], version)` (cold_start/mmap).
///
/// Wraps the `pub(super)` fixed-width mmap entry decoder via the crate-visible
/// `mmap::__fuzz_decode_mmap_entry` shim. Returns `true` for a successful decode,
/// `false` for the typed `Err` path. `version` is fed straight through.
#[doc(hidden)]
pub fn __fuzz_mmap_entry(buf: &[u8], version: u16) -> bool {
    crate::store::cold_start::mmap::__fuzz_decode_mmap_entry(buf, version)
}

/// `CacheMeta::decode_from_bytes(&[u8])` (projection/mod.rs).
///
/// `CacheMeta` is `pub`, so the real `(remaining_bytes, CacheMeta)` tuple can be
/// returned directly. The decoder splits a small fixed-size meta header off the
/// front and returns the trailing state bytes alongside the parsed meta.
#[doc(hidden)]
pub fn __fuzz_cache_meta(
    bytes: &[u8],
) -> Result<(Vec<u8>, crate::store::projection::CacheMeta), StoreError> {
    crate::store::projection::CacheMeta::decode_from_bytes(bytes)
}

/// Representative concrete state for `decode_cached_state::<T>` fuzzing.
///
/// `decode_cached_state<T>` (projection/flow/mod.rs) is generic over
/// `T: serde::de::DeserializeOwned` and its body is exactly
/// `serde_json::from_slice::<T>(bytes)` — the decode path is identical for every
/// `T`. The crate's real projection-state types are all test-local (private), so
/// there is no public production state type to monomorphize on; this struct
/// stands in for a typical projection state (a couple of scalar counters plus a
/// map), exercising the same monomorphized `serde_json::from_slice` decode path
/// the production callers hit. Documented as the chosen `T` for the contract.
#[derive(Debug, serde::Deserialize)]
#[doc(hidden)]
pub struct FuzzProjectionState {
    /// A monotonic counter, the most common projection-state shape.
    pub count: u64,
    /// A signed accumulator, exercises numeric coercion paths.
    pub balance: i64,
    /// A per-key map, exercises the nested-collection decode path.
    pub by_key: std::collections::BTreeMap<String, u64>,
}

/// `decode_cached_state::<T>(entity, &[u8], warn)` (projection/flow/mod.rs),
/// monomorphized on [`FuzzProjectionState`].
///
/// `decode_cached_state` is a private free fn; this wrapper re-implements its
/// one-line body against the **same** representative `T` so the fuzz crate drives
/// the identical `serde_json::from_slice` decode path the production callers use.
/// Returns `true` when the JSON deserialized, `false` on the warn-and-`None`
/// path. (The underlying fn is private and cannot be re-exported; mirroring its
/// trivial body here keeps the fuzzed code path byte-for-byte identical.)
#[doc(hidden)]
pub fn __fuzz_projection_state(bytes: &[u8]) -> bool {
    serde_json::from_slice::<FuzzProjectionState>(bytes).is_ok()
}

// ---------------------------------------------------------------------------
// File-path decoders. The wrapper owns a `tempfile::TempDir`/`NamedTempFile`,
// writes the untrusted bytes to the real on-disk filename, calls the real
// loader, and drops the tempfile on return.
// ---------------------------------------------------------------------------

/// `load_cancelled_ranges(dir)` (hidden_ranges.rs).
///
/// Writes `data` to `<tmp>/visibility_ranges.fbv` (the real
/// `VISIBILITY_RANGES_FILENAME`) inside a throwaway `TempDir`, then calls the real
/// loader. The loader returns `Ok(Some(ranges))` on a valid file, `Ok(None)` when
/// absent (never here), or a typed `Err` on corruption. The success value is
/// collapsed to `Result<bool, StoreError>` (`bool` = "ranges present"). Returns
/// the I/O error as a `StoreError::Io` if the temp write itself fails (it should
/// not, but the wrapper never panics).
#[doc(hidden)]
pub fn __fuzz_hidden_ranges(data: &[u8]) -> Result<bool, StoreError> {
    let dir = tempfile::tempdir().map_err(StoreError::Io)?;
    let path = dir
        .path()
        .join(crate::store::hidden_ranges::VISIBILITY_RANGES_FILENAME);
    std::fs::write(&path, data).map_err(StoreError::Io)?;
    crate::store::hidden_ranges::load_cancelled_ranges(dir.path(), &crate::store::RealFs)
        .map(|ranges| ranges.is_some())
}

/// `load_mmap_index(dir, &clock)` (cold_start/mmap/load.rs).
///
/// Writes `data` to `<tmp>/index.fbati` (the real `MMAP_INDEX_FILENAME`) inside a
/// throwaway `TempDir`, then calls the real loader via the crate-visible
/// `mmap::__fuzz_load_mmap_index` shim (which supplies a real `SystemClock`). The
/// loader's private `FileLoad` outcome is returned as a stable `&'static str`
/// discriminant: `"missing" | "loaded" | "invalid" | "future_version"`. Returns
/// `"io_error"` if the temp write itself fails. Never panics.
#[doc(hidden)]
pub fn __fuzz_mmap_index_load(data: &[u8]) -> &'static str {
    let Ok(dir) = tempfile::tempdir() else {
        return "io_error";
    };
    let path = dir
        .path()
        .join(crate::store::cold_start::mmap::MMAP_INDEX_FILENAME);
    if std::fs::write(&path, data).is_err() {
        return "io_error";
    }
    crate::store::cold_start::mmap::__fuzz_load_mmap_index(dir.path())
}

/// `footer::read_layout(&mut Read+Seek, seg_id)` then
/// `read_entries_unauthenticated` (segment/sidx/footer.rs).
///
/// Writes `data` to a throwaway `NamedTempFile`, opens it `Read + Seek`, and
/// drives BOTH footer-parse paths on the same untrusted file:
///   * `authenticated_string_table_offset` — which internally calls
///     `footer::read_layout` (the private footer fns are reachable only through
///     these `pub(crate)` sidx-module wrappers), and
///   * `read_entries_unauthenticated`.
///
/// Both are seeked from the file start before each call. Returns
/// `Result<(bool, usize), StoreError>` = `(layout authenticated an offset,
/// number of unauthenticated entries parsed)`. `segment_id` is fed through to
/// both; the fuzz target can pass any `u64`.
#[doc(hidden)]
pub fn __fuzz_sidx_footer(data: &[u8], segment_id: u64) -> Result<(bool, usize), StoreError> {
    use std::io::{Seek, SeekFrom, Write};

    let mut file = tempfile::NamedTempFile::new().map_err(StoreError::Io)?;
    file.write_all(data).map_err(StoreError::Io)?;
    file.flush().map_err(StoreError::Io)?;
    let f = file.as_file_mut();

    f.seek(SeekFrom::Start(0)).map_err(StoreError::Io)?;
    let layout_authenticated =
        crate::store::segment::sidx::authenticated_string_table_offset(f, segment_id)?.is_some();

    f.seek(SeekFrom::Start(0)).map_err(StoreError::Io)?;
    let entries = crate::store::segment::sidx::read_entries_unauthenticated(f, segment_id)?;

    Ok((layout_authenticated, entries.len()))
}

/// `segment::detect_sidx_boundary(&mut Read+Seek, file_len, segment_id)`.
///
/// Drives the boundary detector directly over arbitrary file bytes, including
/// trailers whose final four bytes look like known, legacy, future, or unrelated
/// magic. The returned discriminant is only for the fuzz harness; the contract is
/// that malformed tails return `Ok("none")`, `Ok("untrusted")`, or a typed
/// [`StoreError`] such as [`StoreError::SidxFutureVersion`], never a panic.
#[doc(hidden)]
pub fn __fuzz_sidx_boundary(data: &[u8], segment_id: u64) -> Result<&'static str, StoreError> {
    use std::io::Cursor;

    let file_len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut cursor = Cursor::new(data);
    let boundary = crate::store::segment::detect_sidx_boundary(&mut cursor, file_len, segment_id)?;
    Ok(match boundary {
        Some(boundary) if boundary.trusted => "trusted",
        Some(_) => "untrusted",
        None => "none",
    })
}

/// Semantic SIDX-manifest mutation harness (GAUNT-SIDX-NO-SELF-AUTH, #192).
///
/// Unlike the pure decode wrappers above, this is a PROPERTY harness: it builds
/// a VALID `[frames][SDX3 footer]` pair, applies `script`-driven SEMANTIC
/// mutations to the entry table (forged offsets/lengths, removed rows,
/// duplicated forged siblings, a torn last frame, an overridden footer claim) —
/// keeping the table well-formed so every input reaches the recovery DECISION
/// core instead of dying in bounds checks — then asserts the
/// no-authority-escalation law against a ground truth computed by the
/// table-blind CRC walk itself:
///
/// * TABLE-BLINDNESS: the permissive posture recovers EXACTLY the walk's
///   verified prefix end, no matter what the table claims (a forged row can
///   neither brick recovery nor extend it past verified bytes);
/// * NO FALSE SAFETY: with a non-empty verified prefix the strict posture
///   ALWAYS refuses — no table content is a recovery ticket;
/// * EMPTY-PREFIX: with nothing recovered, both postures recover;
/// * FAIL-CLOSED SYMMETRY: when the walk itself fails closed (mid-stream
///   shape), both postures refuse.
///
/// The asserts live HERE (not in the target) because the ground truth needs
/// crate-internal entry points; a violated law panics, which libFuzzer reports
/// as a crash in real decision logic. Returns a classification discriminant for
/// corpus statistics.
#[doc(hidden)]
pub fn __fuzz_sidx_manifest(script: &[u8]) -> Result<&'static str, StoreError> {
    let mut it = script.iter().copied();
    let (mut bytes, frame_spans, mut entries) = build_manifest_fixture(&mut it)?;
    let (tear, claim_override) = apply_manifest_mutation_script(&mut it, &mut entries);
    let claim =
        assemble_untrusted_segment(&mut bytes, &frame_spans, entries, tear, claim_override)?;
    assert_no_authority_escalation(bytes, claim)
}

/// [`build_manifest_fixture`]'s output: the raw frame bytes, each frame's
/// `(offset, on_disk_len)` span, and the honest SIDX entries describing them.
type ManifestFixture = (
    Vec<u8>,
    Vec<(u64, u64)>,
    Vec<crate::store::segment::sidx::SidxEntry>,
);

/// Fixture half of [`__fuzz_sidx_manifest`]: build 1..=4 real CRC-valid frames
/// plus their honest SIDX entries. Returns `(frame_bytes, frame_spans, entries)`.
fn build_manifest_fixture(
    it: &mut impl Iterator<Item = u8>,
) -> Result<ManifestFixture, StoreError> {
    let frame_count = usize::from(it.next().unwrap_or(2) % 4) + 1;
    let mut bytes: Vec<u8> = Vec::new();
    let mut frame_spans: Vec<(u64, u64)> = Vec::new();
    let mut entries: Vec<crate::store::segment::sidx::SidxEntry> = Vec::new();
    for i in 0..frame_count {
        let frame_offset = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let frame = crate::store::segment::frame_encode(&(
            u32::try_from(i).unwrap_or(u32::MAX),
            "sidx-manifest-fuzz",
        ))?;
        let frame_length = u32::try_from(frame.len()).unwrap_or(u32::MAX);
        frame_spans.push((frame_offset, u64::from(frame_length)));
        bytes.extend_from_slice(&frame);
        entries.push(crate::store::segment::sidx::SidxEntry {
            event_id: i as u128 + 1,
            entity_idx: 0,
            scope_idx: 0,
            kind: crate::store::segment::sidx::kind_to_raw(crate::event::EventKind::custom(0x1, 1)),
            wall_ms: 1,
            clock: 1,
            dag_lane: 0,
            dag_depth: 0,
            prev_hash: [0; 32],
            event_hash: [7; 32],
            frame_offset,
            frame_length,
            global_sequence: i as u64 + 1,
            correlation_id: 1,
            causation_id: 0,
        });
    }
    Ok((bytes, frame_spans, entries))
}

/// Mutation half of [`__fuzz_sidx_manifest`]: apply the script's SEMANTIC
/// entry-table mutations in place. Returns `(tear, claim_override)` for the
/// assembly step.
fn apply_manifest_mutation_script(
    it: &mut impl Iterator<Item = u8>,
    entries: &mut Vec<crate::store::segment::sidx::SidxEntry>,
) -> (Option<u8>, Option<u64>) {
    let mut tear: Option<u8> = None;
    let mut claim_override: Option<u64> = None;
    while let Some(op) = it.next() {
        match op % 6 {
            0 => {
                // Forge an entry's claimed frame offset.
                if let (Some(i), Some(v)) = (pick_index(it, entries.len()), take_u64(it)) {
                    entries[i].frame_offset = v;
                }
            }
            1 => {
                // Forge an entry's claimed frame length.
                if let (Some(i), Some(v)) = (pick_index(it, entries.len()), take_u32(it)) {
                    entries[i].frame_length = v;
                }
            }
            2 => {
                // Truncate the table (remove a row) — the "agreeing table" dual.
                if let Some(i) = pick_index(it, entries.len()) {
                    entries.remove(i);
                }
            }
            3 => {
                // Duplicate a row as a forged sibling with a scripted offset.
                if entries.len() < 64 {
                    if let (Some(i), Some(v)) = (pick_index(it, entries.len()), take_u64(it)) {
                        let mut forged = entries[i].clone();
                        forged.frame_offset = v;
                        entries.push(forged);
                    }
                }
            }
            4 => {
                // Tear the last committed frame (at most once).
                if tear.is_none() {
                    tear = Some(it.next().unwrap_or(0));
                }
            }
            _ => {
                // Override the footer's claimed frames-end passed to resolve.
                if let Some(v) = take_u64(it) {
                    claim_override = Some(v);
                }
            }
        }
    }
    (tear, claim_override)
}

/// Assembly half of [`__fuzz_sidx_manifest`]: apply the tear, write a
/// well-formed footer over the (mutated) entries, then break its CRC so the
/// boundary is UNTRUSTED — the only path that reaches this decision. Returns
/// the footer claim to pass to `resolve_untrusted_frames_end`.
fn assemble_untrusted_segment(
    bytes: &mut Vec<u8>,
    frame_spans: &[(u64, u64)],
    entries: Vec<crate::store::segment::sidx::SidxEntry>,
    tear: Option<u8>,
    claim_override: Option<u64>,
) -> Result<Option<u64>, StoreError> {
    use crate::store::segment::sidx::SidxEntryCollector;
    use std::io::{Cursor, Seek, SeekFrom};

    // Apply the tear: keep the last frame's 8-byte header plus a partial payload
    // so it can never decode.
    if let (Some(k), Some(&(off, len))) = (tear, frame_spans.last()) {
        let payload = len.saturating_sub(8);
        let keep = if payload == 0 {
            0
        } else {
            u64::from(k) % payload
        };
        bytes.truncate(usize::try_from(off + 8 + keep).unwrap_or(usize::MAX));
    }

    let footer_start = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut collector = SidxEntryCollector::new();
    for entry in entries {
        collector.record(entry, "entity:fuzz", "scope:fuzz")?;
    }
    let mut cursor = Cursor::new(&mut *bytes);
    cursor.seek(SeekFrom::End(0)).map_err(StoreError::Io)?;
    collector.write_footer(&mut cursor, 7)?;
    if let Some(byte) = bytes.get_mut(usize::try_from(footer_start).unwrap_or(usize::MAX)) {
        *byte ^= 0xFF;
    }
    Ok(Some(claim_override.unwrap_or(footer_start)))
}

/// Verdict half of [`__fuzz_sidx_manifest`]: run the table-blind CRC walk as
/// ground truth, run `resolve_untrusted_frames_end` under both postures, and
/// assert the no-authority-escalation law (see the orchestrator doc).
fn assert_no_authority_escalation(
    bytes: Vec<u8>,
    claim: Option<u64>,
) -> Result<&'static str, StoreError> {
    use std::io::Cursor;

    let file_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);

    // Ground truth: the table-blind CRC walk.
    let walk = {
        let mut c = Cursor::new(bytes.clone());
        crate::store::segment::recovery_manifest::crc_valid_frames_end_counting(
            &mut c, 0, file_len, 7,
        )
    };
    let permissive = {
        let mut c = Cursor::new(bytes.clone());
        crate::store::segment::resolve_untrusted_frames_end(&mut c, 0, file_len, 7, claim, false)
    };
    let strict = {
        let mut c = Cursor::new(bytes);
        crate::store::segment::resolve_untrusted_frames_end(&mut c, 0, file_len, 7, claim, true)
    };

    let (walk_p, walk_count) = match walk {
        Ok(ground_truth) => ground_truth,
        Err(_mid_stream) => {
            // FAIL-CLOSED SYMMETRY: the walk itself refuses (mid-stream shape);
            // no table content may recover past it under either posture.
            assert!(
                permissive.is_err() && strict.is_err(),
                "fail-closed symmetry violated: the table-blind walk refused but a posture recovered"
            );
            return Ok("mid-stream-both-refuse");
        }
    };

    match permissive {
        Ok(resolved) => {
            // TABLE-BLINDNESS: recovery lands exactly on the verified prefix end.
            assert_eq!(
                resolved.frames_end, walk_p,
                "no-authority-escalation violated: permissive recovery diverged from the \
                 table-blind verified prefix end"
            );
            if walk_count > 0 {
                // NO FALSE SAFETY: strict refuses every non-empty prefix.
                assert!(
                    matches!(strict, Err(StoreError::CorruptSegment { .. })),
                    "no-false-safety violated: strict posture recovered a non-empty prefix \
                     under an untrusted footer"
                );
                Ok("permissive-recovered-strict-refused")
            } else {
                let strict_resolved = strict?;
                assert_eq!(
                    strict_resolved.frames_end, walk_p,
                    "empty-prefix law violated: strict recovery diverged from the walk"
                );
                Ok("empty-prefix-both-recover")
            }
        }
        Err(err) => {
            // The walk succeeded, so a permissive refusal is a FALSE LOSS — the
            // exact denial-of-service the safe law forbids.
            let refusal = format!("{err:?}");
            assert!(
                std::hint::black_box(false),
                "false-loss violated: permissive posture refused ({refusal}) although the \
                 table-blind walk recovered"
            );
            Ok("unreachable")
        }
    }
}

/// Consume one byte as an index into `0..len`. `None` when the script is
/// exhausted or the collection is empty.
fn pick_index(it: &mut impl Iterator<Item = u8>, len: usize) -> Option<usize> {
    let byte = it.next()?;
    if len == 0 {
        return None;
    }
    Some(usize::from(byte) % len)
}

/// Consume 8 script bytes as a little-endian `u64`.
fn take_u64(it: &mut impl Iterator<Item = u8>) -> Option<u64> {
    let mut buf = [0u8; 8];
    for slot in &mut buf {
        *slot = it.next()?;
    }
    Some(u64::from_le_bytes(buf))
}

/// Consume 4 script bytes as a little-endian `u32`.
fn take_u32(it: &mut impl Iterator<Item = u8>) -> Option<u32> {
    let mut buf = [0u8; 4];
    for slot in &mut buf {
        *slot = it.next()?;
    }
    Some(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod sidx_manifest_harness_tests {
    use super::__fuzz_sidx_manifest;

    #[test]
    fn seed_scripts_uphold_the_no_authority_escalation_law() {
        // Non-vacuity smoke for the property harness: the checked-in corpus
        // seed scripts (fuzz/corpus/sidx_manifest/) plus degenerate inputs run
        // the in-shim asserts under plain `cargo test`, so a law violation is
        // caught by CI even before any cargo-fuzz run. Each script drives a
        // different mutation shape; the assert payload lives inside the shim.
        let seeds: &[&[u8]] = &[
            // seed-clean: three intact frames, unmutated table.
            &[0x02],
            // seed-forged-sibling: duplicate row 0 as a forged trailing claim.
            &[
                0x02, 0x03, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            // seed-truncated-table-torn-tail: remove a row, then tear the tail.
            &[0x03, 0x02, 0x02, 0x04, 0x05],
            // seed-claim-override: forge a row length, override the footer claim.
            &[0x01, 0x05, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            // Degenerate: empty script (defaults), single-frame torn-to-empty.
            &[],
            &[0x00, 0x04, 0x00],
        ];
        for seed in seeds {
            let outcome = __fuzz_sidx_manifest(seed)
                .expect("seed scripts drive in-memory paths; only a real Io failure may error");
            assert!(
                matches!(
                    outcome,
                    "permissive-recovered-strict-refused"
                        | "empty-prefix-both-recover"
                        | "mid-stream-both-refuse"
                ),
                "unexpected harness classification: {outcome}"
            );
        }
    }
}

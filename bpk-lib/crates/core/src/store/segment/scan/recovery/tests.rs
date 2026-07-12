use super::*;
use crate::coordinate::DagPosition;
use crate::event::EventKind;
use crate::store::platform::fs::RealFs;
use crate::store::segment::scan::recovery::sidx_fast_path::SidxTailCoverage;
use crate::store::segment::sidx::{kind_to_raw, read_footer, SidxEntry, SidxEntryCollector};
use std::io::ErrorKind;
use std::io::{Cursor, Seek, SeekFrom, Write};
use tempfile::{NamedTempFile, TempDir};

fn sample_entry(frame_offset: u64, frame_length: u32) -> SidxEntry {
    SidxEntry {
        event_id: 1,
        entity_idx: 0,
        scope_idx: 0,
        kind: kind_to_raw(EventKind::custom(0x1, 1)),
        wall_ms: 1,
        clock: 1,
        dag_lane: 0,
        dag_depth: 0,
        prev_hash: [0; 32],
        event_hash: [1; 32],
        frame_offset,
        frame_length,
        global_sequence: 1,
        correlation_id: 1,
        causation_id: 0,
    }
}

fn footer_bytes(prefix_len: usize, entries: &[SidxEntry]) -> Vec<u8> {
    let mut bytes = vec![0xA5; prefix_len];
    let mut cursor = Cursor::new(&mut bytes);
    cursor.seek(SeekFrom::End(0)).expect("seek to end");

    let mut collector = SidxEntryCollector::new();
    for (idx, entry) in entries.iter().cloned().enumerate() {
        let entity = format!("entity:{idx}");
        collector
            .record(entry, &entity, "scope:test")
            .expect("intern test strings");
    }
    collector
        .write_footer(&mut cursor, 7)
        .expect("write footer");

    bytes
}

fn footer_file(prefix_len: usize, entries: &[SidxEntry]) -> NamedTempFile {
    let bytes = footer_bytes(prefix_len, entries);
    let mut tmp = NamedTempFile::new().expect("create temp file");
    tmp.write_all(&bytes).expect("write temp bytes");
    tmp.flush().expect("flush temp file");
    tmp
}

fn footer_segment_path(
    dir: &TempDir,
    segment_id: u64,
    prefix_len: usize,
    entries: &[SidxEntry],
) -> std::path::PathBuf {
    let path = dir
        .path()
        .join(crate::store::segment::segment_filename(segment_id));
    crate::store::platform::fs::write_derivative_file_atomically(
        dir.path(),
        &path,
        "segment footer fixture",
        &footer_bytes(prefix_len, entries),
    )
    .expect("write segment footer file");
    path
}

fn scanned_entry(event_id: u128) -> ScannedIndexEntry {
    ScannedIndexEntry {
        header: EventHeader::new(
            event_id,
            event_id,
            None,
            0,
            DagPosition::root(),
            0,
            EventKind::custom(0x1, 1),
        ),
        entity: "entity:test".to_owned(),
        scope: "scope:test".to_owned(),
        hash_chain: HashChain::default(),
        segment_id: 7,
        offset: u64::try_from(event_id).expect("test ids fit u64"),
        length: 16,
        receipt_extensions: std::collections::BTreeMap::new(),
        global_sequence: None,
    }
}

#[test]
fn batch_recovery_state_refuses_items_past_declared_count() {
    let mut state = BatchRecoveryState {
        remaining: 1,
        started_count: 1,
        in_batch: true,
        staged: Vec::new(),
    };

    assert!(
        state.stage_entry(scanned_entry(1)),
        "PROPERTY: the first item in a one-item recovered batch is accepted"
    );
    assert_eq!(
        state.remaining, 0,
        "PROPERTY: staging a recovered batch item decrements the remaining item budget"
    );
    assert!(
        !state.stage_entry(scanned_entry(2)),
        "PROPERTY: recovery must reject items beyond the BEGIN marker's declared count"
    );
    assert_eq!(
        state.staged.len(),
        1,
        "PROPERTY: rejected over-count items must not be silently staged"
    );

    state.discard_incomplete();
    assert!(
        !state.in_batch && state.staged.is_empty() && state.remaining == 0,
        "PROPERTY: corrupt over-count batches can be discarded without leaving pending state"
    );
}

#[test]
fn sidx_covers_segment_tail_requires_last_entry_to_reach_footer_start() {
    let tmp = footer_file(64, &[sample_entry(0, 64)]);
    let (entries, _) = read_footer(&RealFs, tmp.path())
        .expect("read footer")
        .expect("footer should be present");

    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, tmp.path(), &entries),
        SidxTailCoverage::Complete,
        "PROPERTY: SIDX coverage is complete only when the last indexed frame tail reaches the footer start"
    );
}

#[test]
fn sidx_covers_segment_tail_rejects_trailing_unindexed_bytes() {
    let tmp = footer_file(80, &[sample_entry(0, 64)]);
    let (entries, _) = read_footer(&RealFs, tmp.path())
        .expect("read footer")
        .expect("footer should be present");

    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, tmp.path(), &entries),
        SidxTailCoverage::Incomplete,
        "PROPERTY: trailing bytes between the last indexed frame and the footer force frame-scan fallback"
    );
}

#[test]
fn sidx_covers_segment_tail_treats_truly_empty_segment_as_covered() {
    let tmp = footer_file(0, &[]);
    let (entries, _) = read_footer(&RealFs, tmp.path())
        .expect("read footer")
        .expect("footer should be present");

    assert!(
        entries.is_empty(),
        "SANITY: empty-footer fixture should not produce SIDX entries"
    );
    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, tmp.path(), &entries),
        SidxTailCoverage::Complete,
        "PROPERTY: an empty segment with an empty SIDX footer is fully covered and must not be forced onto the frame-scan fallback"
    );
}

#[test]
fn sidx_covers_segment_tail_rejects_empty_footer_after_frames() {
    let tmp = footer_file(64, &[]);
    let (entries, _) = read_footer(&RealFs, tmp.path())
        .expect("read footer")
        .expect("footer should be present");

    assert!(
        entries.is_empty(),
        "SANITY: fixture should have frames before an empty SIDX footer"
    );
    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, tmp.path(), &entries),
        SidxTailCoverage::Incomplete,
        "PROPERTY: an empty SIDX footer does not cover preceding frame bytes and must force frame-scan fallback"
    );
}

#[test]
fn payload_read_unexpected_eof_respects_tail_policy() {
    assert_eq!(
        tail::classify_payload_read_error(
            7,
            std::io::Error::from(ErrorKind::UnexpectedEof),
            FrameScanTailPolicy::RecoverTornTail,
        )
        .expect("latest tail EOF should be recoverable"),
        PayloadReadFailure::RecoverTornTail,
        "PROPERTY: only the latest tail policy may turn payload EOF into torn-tail recovery"
    );

    let err = tail::classify_payload_read_error(
        7,
        std::io::Error::from(ErrorKind::UnexpectedEof),
        FrameScanTailPolicy::FailClosed,
    )
    .expect_err("non-tail payload EOF must fail closed");
    assert!(
        matches!(
            err,
            StoreError::CorruptSegment { ref detail, .. }
            if detail.contains("frame payload ended before requested length")
        ),
        "PROPERTY: non-tail payload EOF must surface as committed-frame corruption, got {err:?}"
    );
}

#[test]
fn payload_read_non_eof_error_is_never_torn_tail_recovery() {
    let err = tail::classify_payload_read_error(
        7,
        std::io::Error::from(ErrorKind::PermissionDenied),
        FrameScanTailPolicy::RecoverTornTail,
    )
    .expect_err("non-EOF read errors must remain I/O errors");
    assert!(
        matches!(err, StoreError::Io(_)),
        "PROPERTY: torn-tail recovery applies only to UnexpectedEof, got {err:?}"
    );
}

#[test]
fn scan_segment_index_into_uses_sidx_fast_path_for_sealed_segments() {
    let dir = TempDir::new().expect("create temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let segment_id = 7;
    let path = footer_segment_path(&dir, segment_id, 64, &[sample_entry(0, 64)]);
    reader.set_active_segment(segment_id + 1);

    let mut rows = Vec::new();
    reader
        .scan_segment_index_into_with_tail_policy(
            &path,
            None,
            FrameScanTailPolicy::FailClosed,
            |row| {
                rows.push(row);
                Ok(())
            },
        )
        .expect("sealed scan should succeed");

    assert_eq!(
            rows.len(),
            1,
            "PROPERTY: sealed segments with full SIDX tail coverage should use the footer fast path and emit indexed rows"
        );
}

#[test]
fn scan_segment_index_into_uses_sidx_fast_path_when_batch_state_is_idle() {
    let dir = TempDir::new().expect("create temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let segment_id = 7;
    let path = footer_segment_path(&dir, segment_id, 64, &[sample_entry(0, 64)]);
    reader.set_active_segment(segment_id + 1);
    let mut batch_state = BatchRecoveryState::default();

    let mut rows = Vec::new();
    reader
        .scan_segment_index_into_with_tail_policy(
            &path,
            Some(&mut batch_state),
            FrameScanTailPolicy::FailClosed,
            |row| {
                rows.push(row);
                Ok(())
            },
        )
        .expect("idle batch state should not disable the SIDX fast path");

    assert_eq!(
            rows.len(),
            1,
            "PROPERTY: an idle cross-segment batch state is equivalent to no batch state for SIDX fast-path admission"
        );
}

#[test]
fn scan_segment_index_into_rejects_sidx_fast_path_when_batch_is_pending() {
    let dir = TempDir::new().expect("create temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let segment_id = 7;
    let path = footer_segment_path(&dir, segment_id, 64, &[sample_entry(0, 64)]);
    reader.set_active_segment(segment_id + 1);
    let mut batch_state = BatchRecoveryState {
        in_batch: true,
        remaining: 1,
        started_count: 1,
        staged: Vec::new(),
    };

    let mut rows = Vec::new();
    let err = reader
        .scan_segment_index_into_with_tail_policy(
            &path,
            Some(&mut batch_state),
            FrameScanTailPolicy::FailClosed,
            |row| {
                rows.push(row);
                Ok(())
            },
        )
        .expect_err("pending batch state must force frame scan over synthetic footer bytes");

    assert!(
            matches!(err, StoreError::CorruptSegment { .. }),
            "PROPERTY: a pending cross-segment batch must not trust a SIDX footer until the batch is resolved; got {err:?}"
        );
    assert!(
        rows.is_empty(),
        "PROPERTY: rejecting the SIDX fast path while a batch is pending must not emit footer rows"
    );
}

#[test]
fn scan_segment_index_into_filters_batch_markers_from_sidx_fast_path() {
    let dir = TempDir::new().expect("create temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let segment_id = 7;
    let mut begin = sample_entry(0, 64);
    begin.event_id = 10;
    begin.kind = kind_to_raw(EventKind::SYSTEM_BATCH_BEGIN);
    begin.global_sequence = 10;
    let mut item = sample_entry(64, 64);
    item.event_id = 11;
    item.kind = kind_to_raw(EventKind::custom(0x1, 2));
    item.global_sequence = 11;
    let mut commit = sample_entry(128, 64);
    commit.event_id = 12;
    commit.kind = kind_to_raw(EventKind::SYSTEM_BATCH_COMMIT);
    commit.global_sequence = 12;
    let path = footer_segment_path(&dir, segment_id, 192, &[begin, item, commit]);
    reader.set_active_segment(segment_id + 1);

    let mut rows = Vec::new();
    reader
        .scan_segment_index_into_with_tail_policy(
            &path,
            None,
            FrameScanTailPolicy::FailClosed,
            |row| {
                rows.push(row);
                Ok(())
            },
        )
        .expect("sealed scan should succeed");

    assert_eq!(
            rows.len(),
            1,
            "PROPERTY: SIDX fast-path recovery must filter BEGIN/COMMIT markers and emit only logical user rows"
        );
    assert_eq!(
        rows[0].header.event_id,
        crate::id::EventId::from(11u128),
        "PROPERTY: filtering batch markers must preserve the real batch item"
    );
    assert_eq!(
        rows[0].header.event_kind,
        EventKind::custom(0x1, 2),
        "PROPERTY: filtering batch markers must not rewrite the surviving item kind"
    );
}

#[test]
fn sidx_covers_segment_tail_rejects_offset_length_overflow() {
    // R15 oracle: round-trip a SIDX entry whose frame_offset + frame_length
    // overflows u64 through the real writer (SidxEntryCollector::write_footer)
    // and reader (read_footer). The old saturating_add silently clamped this to
    // u64::MAX and (since MAX != footer start) fell through to Incomplete by
    // accident; checked_add now returns Incomplete deliberately on overflow,
    // never trusting a corrupt offset on the unverified fast path.
    let overflowing = sample_entry(u64::MAX - 16, 64);
    let tmp = footer_file(64, &[overflowing]);
    let (entries, _) = read_footer(&RealFs, tmp.path())
        .expect("read footer")
        .expect("footer should be present");

    assert_eq!(
        entries.len(),
        1,
        "SANITY: the overflowing entry must round-trip through the real writer/reader"
    );
    assert_eq!(
        entries[0].frame_offset,
        u64::MAX - 16,
        "SANITY: round-trip must preserve the near-MAX frame_offset that triggers overflow"
    );
    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, tmp.path(), &entries),
        SidxTailCoverage::Incomplete,
        "PROPERTY: a frame_offset + frame_length that overflows u64 is corruption and must degrade to the frame-scan fallback, not silently saturate"
    );
}

#[test]
fn scan_segment_index_into_ignores_sidx_footer_for_active_segments() {
    let dir = TempDir::new().expect("create temp dir");
    let reader = Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    );
    let segment_id = 7;
    let path = footer_segment_path(&dir, segment_id, 64, &[sample_entry(0, 64)]);
    reader.set_active_segment(segment_id);

    let mut rows = Vec::new();
    let err = reader
        .scan_segment_index_into_with_tail_policy(
            &path,
            None,
            FrameScanTailPolicy::FailClosed,
            |row| {
                rows.push(row);
                Ok(())
            },
        )
        .expect_err("active segment must not trust the synthetic SIDX footer fixture");

    assert!(
            matches!(err, StoreError::CorruptSegment { .. }),
            "PROPERTY: the active segment must refuse the SIDX fast path and fall back to frame scan even if a footer is present"
        );
    assert!(
            rows.is_empty(),
            "PROPERTY: falling back to frame scan must not synthesize rows from the active segment's footer bytes"
        );
}

fn recovery_reader(dir: &TempDir) -> Reader {
    Reader::new(
        dir.path().to_path_buf(),
        4,
        &(std::sync::Arc::new(crate::store::SystemClock::new())
            as std::sync::Arc<dyn crate::store::Clock>),
        std::sync::Arc::new(crate::store::platform::fs::RealFs),
    )
}

/// Build a real sealed segment file (magic + header + one CRC-valid frame per
/// payload) through the production `Segment` writer. Returns `(path, frames_start)`
/// where `frames_start == 8 + header_len` is the true start of the frame region.
fn segment_with_frames(
    dir: &TempDir,
    segment_id: u64,
    payloads: &[serde_json::Value],
) -> (std::path::PathBuf, u64) {
    let fs: std::sync::Arc<dyn crate::store::platform::fs::StoreFs> =
        std::sync::Arc::new(crate::store::platform::fs::RealFs);
    let mut active = segment::Segment::<segment::Active>::create_with_created_ns_on(
        dir.path(),
        segment_id,
        0,
        &fs,
    )
    .expect("create segment");
    let frames_start = active.written_bytes;
    for payload in payloads {
        let payload_bytes = crate::encoding::to_bytes(payload).expect("encode payload bytes");
        let event = crate::event::Event {
            header: EventHeader::new(
                1,
                1,
                None,
                1,
                DagPosition::root(),
                u32::try_from(payload_bytes.len()).expect("payload size fits u32"),
                EventKind::DATA,
            ),
            payload: payload_bytes,
            hash_chain: Some(HashChain::default()),
        };
        let frame = segment::FramePayloadRef {
            event: &event,
            entity: "entity:t",
            scope: "scope:t",
            receipt_extensions: &std::collections::BTreeMap::new(),
        };
        let frame_bytes = segment::frame_encode(&frame).expect("encode frame");
        active.write_frame(&frame_bytes).expect("write frame");
    }
    let path = active.path.clone();
    active
        .sync_with_mode(&crate::store::SyncMode::SyncAll)
        .expect("sync segment");
    let _sealed = active.seal();
    (path, frames_start)
}

/// Append a CRC-VALID SDX3 footer (zero entries) whose `string_table_offset` is the
/// caller-chosen `offset`, so `detect_sidx_boundary` reports the boundary TRUSTED
/// with that offset. The footer covers `[offset .. current_len]` (an empty entry
/// block), so the stored CRC is `crc32` of that span — exactly what `read_layout`
/// recomputes. Used to forge an authenticated offset BELOW frames_start (offset 0)
/// or exactly AT frames_start (empty region), both of which the strict path must
/// handle without recovering zero events silently.
fn append_trusted_sdx3_footer(path: &std::path::Path, offset: u64) {
    let mut bytes = crate::store::platform::fs::read(path).expect("read segment");
    let covered_start = usize::try_from(offset).expect("offset fits usize");
    let crc = crc32fast::hash(&bytes[covered_start..]);
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes()); // entry_count = 0
    bytes.extend_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    crate::store::platform::fs::write_derivative_file_atomically(
        path.parent().expect("segment file has a parent directory"),
        path,
        "trusted footer fixture",
        &bytes,
    )
    .expect("write segment with trusted footer");
}

/// Append a raw 16-byte SIDX trailer (no CRC-covered body), so the boundary is
/// recognized but UNAUTHENTICATED → UNTRUSTED (a legacy SDX2 magic can never
/// authenticate). Drives the untrusted-recovery scan path.
fn append_raw_trailer(path: &std::path::Path, offset: u64, count: u32, magic: &[u8; 4]) {
    let mut bytes = crate::store::platform::fs::read(path).expect("read segment");
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(magic);
    crate::store::platform::fs::write_derivative_file_atomically(
        path.parent().expect("segment file has a parent directory"),
        path,
        "raw trailer fixture",
        &bytes,
    )
    .expect("write segment with raw trailer");
}

#[test]
fn scan_segment_fails_closed_for_an_untrusted_low_offset_footer_with_no_corroboration() {
    let dir = TempDir::new().expect("tmpdir");
    let (path, _frames_start) = segment_with_frames(&dir, 3, &[serde_json::json!({"v": "one"})]);
    // A legacy SDX2 footer (never CRC-authenticated → UNTRUSTED) whose offset (0)
    // is below the frame region AND which carries ZERO entries (count = 0).
    // `scan_segment` is the compaction full read of SEALED segments and passes
    // `fallback_fail_closed = true` (strict). A NON-EMPTY recovered prefix under
    // an untrusted footer is an unprovable tail and must be REFUSED — the
    // untrusted offset that would bound the frame region is exactly what we do
    // not trust, so a torn/truncated further committed frame cannot be ruled
    // out; no entry table could vouch otherwise (rows are suspicion, never
    // authority — #192). (Before the strict flag had teeth this silently
    // recovered the prefix — the round-7 gap.)
    append_raw_trailer(
        &path,
        0,
        0,
        crate::store::segment::sidx::SIDX_MAGIC_LEGACY_SDX2,
    );

    let reader = recovery_reader(&dir);
    let result = reader.scan_segment(&path).map(|entries| entries.len());
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 3, ref detail })
            if detail.contains("unprovable tail")
        ),
        "PROPERTY: the strict compaction full-read must fail closed on an unprovable non-empty tail \
         under an untrusted footer with no corroborating manifest; got {result:?}"
    );
}

#[test]
fn scan_segment_rejects_a_trusted_offset_below_the_frame_region() {
    let dir = TempDir::new().expect("tmpdir");
    let (path, _frames_start) = segment_with_frames(&dir, 4, &[]); // header only, no frames
                                                                   // A CRC-AUTHENTICATED SDX3 offset of 0 falls BELOW frames_start (8 + header_len).
                                                                   // The lower-bound guard must reject it as CorruptSegment — recovering zero events
                                                                   // would silently lose data. The delete-`!` mutant flips the guard to fire only on
                                                                   // UNtrusted boundaries, so a trusted-but-too-low offset silently returns empty.
    append_trusted_sdx3_footer(&path, 0);

    let reader = recovery_reader(&dir);
    // Map Ok to the recovered count (ScannedEntry is not Debug) so both arms print.
    let result = reader.scan_segment(&path).map(|entries| entries.len());
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 4, ref detail })
            if detail.contains("below the frame region start")
        ),
        "a trusted offset below the frame region must be rejected as CorruptSegment, not silently \
         emptied; the lower-bound guard must fire; got {result:?}"
    );
}

#[test]
fn scan_index_rejects_a_trusted_offset_below_the_frame_region() {
    let dir = TempDir::new().expect("tmpdir");
    let (path, _frames_start) = segment_with_frames(&dir, 4, &[]);
    append_trusted_sdx3_footer(&path, 0);
    let reader = recovery_reader(&dir);
    reader.set_active_segment(5); // sealed → fast path consulted, then falls through

    let mut rows = 0usize;
    let err = {
        let mut sink = |_row: ScannedIndexEntry| -> Result<(), StoreError> {
            rows += 1;
            Ok(())
        };
        reader
            .scan_segment_index_into_with_tail_policy(
                &path,
                None,
                FrameScanTailPolicy::FailClosed,
                &mut sink,
            )
            .expect_err("a trusted offset below the frame region must be rejected")
    };
    assert!(
        matches!(
            err,
            StoreError::CorruptSegment { segment_id: 4, ref detail }
            if detail.contains("below the frame region start")
        ),
        "the index-scan lower-bound guard must fire for a trusted too-low offset; the `<`→`==` \
         mutant only fires at frames_end == cursor and silently empties this segment; got {err:?}"
    );
    assert_eq!(rows, 0, "a rejected segment must not emit any index rows");
}

#[test]
fn scan_index_accepts_a_trusted_empty_frame_region() {
    let dir = TempDir::new().expect("tmpdir");
    let (path, frames_start) = segment_with_frames(&dir, 6, &[]);
    // A CRC-valid SDX3 footer whose authenticated offset lands EXACTLY at
    // frames_start — a legitimately empty (but valid) frame region. frames_end ==
    // cursor must stay VALID (zero rows, no error). The `<`→`<=` mutant treats the
    // inclusive boundary as corrupt and rejects a legitimately empty sealed segment.
    append_trusted_sdx3_footer(&path, frames_start);
    let reader = recovery_reader(&dir);
    reader.set_active_segment(7);

    let mut rows = 0usize;
    {
        let mut sink = |_row: ScannedIndexEntry| -> Result<(), StoreError> {
            rows += 1;
            Ok(())
        };
        reader
            .scan_segment_index_into_with_tail_policy(
                &path,
                None,
                FrameScanTailPolicy::FailClosed,
                &mut sink,
            )
            .expect("a trusted empty frame region (frames_end == cursor) must scan cleanly");
    }
    assert_eq!(rows, 0, "an empty frame region yields zero index rows");
}

#[test]
fn scan_index_fails_closed_on_a_torn_tail_frame_under_failclosed_policy() {
    let dir = TempDir::new().expect("tmpdir");
    let (path, _frames_start) = segment_with_frames(&dir, 8, &[serde_json::json!({"v": "kept"})]);
    // Append an 8-byte frame HEADER claiming a 100-byte payload that is NOT present
    // (a torn last frame), with NO footer → frames_end == file_len. Under FailClosed,
    // a frame whose tail runs past the frame region is committed-frame corruption and
    // must error. The `&&`→`||` mutant lets `frames_end == file_len` alone trigger a
    // torn-tail BREAK, silently truncating instead of failing closed.
    {
        let mut bytes = crate::store::platform::fs::read(&path).expect("read segment");
        bytes.extend_from_slice(&100u32.to_be_bytes()); // claimed payload length = 100
        bytes.extend_from_slice(&0u32.to_be_bytes()); // crc (payload never reached)
        crate::store::platform::fs::write_derivative_file_atomically(
            dir.path(),
            &path,
            "torn tail fixture",
            &bytes,
        )
        .expect("write torn tail");
    }
    let reader = recovery_reader(&dir);
    reader.set_active_segment(9);

    let mut rows = 0usize;
    let err = {
        let mut sink = |_row: ScannedIndexEntry| -> Result<(), StoreError> {
            rows += 1;
            Ok(())
        };
        reader
            .scan_segment_index_into_with_tail_policy(
                &path,
                None,
                FrameScanTailPolicy::FailClosed,
                &mut sink,
            )
            .expect_err("a torn tail under FailClosed must surface corruption, not truncate")
    };
    assert!(
        matches!(
            err,
            StoreError::CorruptFrame { segment_id: 8, ref reason, .. }
            if reason.contains("extends past the frame region")
        ),
        "FailClosed must reject a frame whose payload runs past the region; the `&&`→`||` mutant \
         would break as a torn tail and return Ok; got {err:?}"
    );
}

#[test]
fn sidx_covers_segment_tail_admits_a_bare_sixteen_byte_footer() {
    // A file of EXACTLY 16 bytes is a bare SDX3 trailer over a zero-length frame
    // region: with zero entries, max_tail (0) == sidx_start (0) → Complete coverage.
    // The `file_len < 16`→`<= 16` mutant treats file_len == 16 as "too small" and
    // returns Incomplete, needlessly forcing the frame-scan fallback.
    let dir = TempDir::new().expect("tmpdir");
    let path = dir.path().join("bare16.fbat");
    let mut trailer = Vec::with_capacity(16);
    trailer.extend_from_slice(&0u64.to_le_bytes()); // string_table_offset = 0
    trailer.extend_from_slice(&0u32.to_le_bytes()); // entry_count = 0
    trailer.extend_from_slice(crate::store::segment::sidx::SIDX_MAGIC);
    crate::store::platform::fs::write_derivative_file_atomically(
        dir.path(),
        &path,
        "bare footer fixture",
        &trailer,
    )
    .expect("write 16-byte footer");

    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, &path, &[]),
        SidxTailCoverage::Complete,
        "a 16-byte bare footer (zero entries, offset 0) is fully covered: max_tail == sidx_start"
    );
}

#[test]
fn sidx_covers_segment_tail_treats_a_subtrailer_file_as_incomplete() {
    // A file too small to hold the 16-byte trailer cannot prove coverage from its
    // footer, so it degrades to Incomplete (frame-scan with CRC verification). The
    // `file_len < 16`→`== 16` mutant only short-circuits at file_len == 16, so a
    // 10-byte file falls through to `seek(End(-16))`, underflows, and returns
    // Unreadable instead of the Incomplete fall-back.
    let dir = TempDir::new().expect("tmpdir");
    let path = dir.path().join("tiny.fbat");
    crate::store::platform::fs::write_derivative_file_atomically(
        dir.path(),
        &path,
        "tiny file fixture",
        &[0u8; 10],
    )
    .expect("write 10-byte file");

    assert_eq!(
        Reader::sidx_covers_segment_tail(&RealFs, &path, &[]),
        SidxTailCoverage::Incomplete,
        "a sub-trailer file degrades to Incomplete (frame-scan), never Unreadable"
    );
}

#[test]
fn try_sidx_fast_path_declines_when_coverage_is_incomplete() {
    // A sealed segment whose SIDX footer does NOT cover the tail (trailing unindexed
    // bytes) must NOT use the fast path — it must frame-scan so no committed frame is
    // missed. try_sidx_fast_path returns Ok(false) and emits nothing. The match-guard
    // `... == Complete`→`true` mutant would take the fast path for ANY readable footer,
    // emitting the (incomplete) SIDX rows and returning Ok(true).
    let dir = TempDir::new().expect("tmpdir");
    let segment_id = 7;
    // prefix 80 with one entry covering [0, 64): max_tail 64 != sidx_start 80 → Incomplete.
    let path = footer_segment_path(&dir, segment_id, 80, &[sample_entry(0, 64)]);
    let reader = recovery_reader(&dir);
    reader.set_active_segment(segment_id + 1);

    let mut rows = 0usize;
    let used_fast_path = {
        let mut sink = |_row: ScannedIndexEntry| -> Result<(), StoreError> {
            rows += 1;
            Ok(())
        };
        reader
            .try_sidx_fast_path(&path, segment_id, false, &mut sink)
            .expect("try_sidx_fast_path must not error on a readable incomplete footer")
    };
    assert!(
        !used_fast_path,
        "an incomplete-coverage footer must decline the SIDX fast path; the guard→true mutant \
         forces it on"
    );
    assert_eq!(rows, 0, "declining the fast path must emit no rows");
}

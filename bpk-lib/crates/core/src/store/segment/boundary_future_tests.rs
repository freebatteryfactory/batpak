//! Forward-compatibility refusals for SIDX boundary detection.

use super::*;
use crate::coordinate::DagPosition;
use crate::event::{Event, EventHeader, EventKind, HashChain};
use crate::store::segment::scan::Reader;
use std::io::{Cursor, Seek, SeekFrom};
use std::sync::Arc;
use tempfile::TempDir;

fn frames_then_sdx3_footer(payloads: &[&str]) -> Vec<u8> {
    use crate::store::segment::sidx::{kind_to_raw, SidxEntry, SidxEntryCollector};

    let mut bytes = Vec::new();
    let mut collector = SidxEntryCollector::new();
    for (idx, payload) in payloads.iter().enumerate() {
        let frame_offset = bytes.len() as u64;
        let frame =
            frame_encode(&serde_json::json!({ "payload": payload })).expect("test frame encodes");
        let frame_length = u32::try_from(frame.len()).expect("frame length fits u32");
        bytes.extend_from_slice(&frame);
        collector
            .record(
                SidxEntry {
                    event_id: idx as u128 + 1,
                    entity_idx: 0,
                    scope_idx: 0,
                    kind: kind_to_raw(crate::event::EventKind::custom(0x1, 1)),
                    wall_ms: 1,
                    clock: 1,
                    dag_lane: 0,
                    dag_depth: 0,
                    prev_hash: [0; 32],
                    event_hash: [1; 32],
                    frame_offset,
                    frame_length,
                    global_sequence: idx as u64 + 1,
                    correlation_id: 1,
                    causation_id: 0,
                },
                "entity:test",
                "scope:test",
            )
            .expect("intern test strings");
    }
    let mut cursor = Cursor::new(&mut bytes);
    cursor.seek(SeekFrom::End(0)).expect("seek to footer start");
    collector
        .write_footer(&mut cursor, 7)
        .expect("write footer");
    bytes
}

fn segment_with_frames(
    dir: &TempDir,
    segment_id: u64,
    payloads: &[serde_json::Value],
) -> std::path::PathBuf {
    let fs: Arc<dyn crate::store::platform::fs::StoreFs> =
        Arc::new(crate::store::platform::fs::RealFs);
    let mut active = Segment::<Active>::create_with_created_ns_on(dir.path(), segment_id, 0, &fs)
        .expect("create segment");
    for payload in payloads {
        let payload_bytes = crate::encoding::to_bytes(payload).expect("encode payload bytes");
        let event = Event {
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
        let frame = FramePayloadRef {
            event: &event,
            entity: "entity:boundary",
            scope: "scope:boundary",
            receipt_extensions: &std::collections::BTreeMap::new(),
        };
        let frame_bytes = frame_encode(&frame).expect("encode frame");
        active.write_frame(&frame_bytes).expect("write frame");
    }
    let path = active.path.clone();
    active
        .sync_with_mode(&crate::store::SyncMode::SyncAll)
        .expect("sync segment");
    let _sealed = active.seal();
    path
}

fn scan_reader(dir: &TempDir) -> Reader {
    let clock: Arc<dyn crate::store::Clock> = Arc::new(crate::store::SystemClock::new());
    Reader::new(
        dir.path().to_path_buf(),
        4,
        &clock,
        Arc::new(crate::store::platform::fs::RealFs),
    )
}

fn append_non_sdxn_tail(path: &std::path::Path) {
    let mut bytes = crate::store::platform::fs::read(path).expect("read segment");
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(b"NOPE");
    crate::store::platform::fs::write_derivative_file_atomically(
        path.parent().expect("segment file has a parent directory"),
        path,
        "non-sdxn tail fixture",
        &bytes,
    )
    .expect("write segment with non-SDXn tail");
}

fn realfs() -> Arc<dyn crate::store::platform::fs::StoreFs> {
    Arc::new(crate::store::platform::fs::RealFs)
}

#[test]
fn detect_sidx_boundary_refuses_a_future_sidx_footer_version() {
    // A future SIDX-family footer (SDX4) is a versioned format this reader does
    // not understand. It must be a canonical typed refusal, not `Ok(None)`,
    // because `Ok(None)` makes callers scan through footer bytes as if frames
    // extended to EOF.
    let mut bytes = frames_then_sdx3_footer(&["a", "b", "c"]);
    let magic_start = bytes.len().checked_sub(4).expect("footer has magic");
    bytes[magic_start..].copy_from_slice(b"SDX4");
    let file_len = bytes.len() as u64;
    let mut cursor = Cursor::new(bytes);

    let err = detect_sidx_boundary(&mut cursor, file_len, 7)
        .expect_err("future SIDX footer version must refuse canonically");
    assert!(
        matches!(
            err,
            StoreError::SidxFutureVersion {
                found: 4,
                supported: 3,
            }
        ),
        "PROPERTY: an SDX4 footer must surface SidxFutureVersion, not fall through to \
         no-footer scanning or generic corruption; got {err:?}"
    );
}

#[test]
fn scan_segment_fails_closed_on_a_non_sdxn_tail() {
    // The tail's last four bytes are not SDXn and not a recognized footer magic.
    // The strict compaction full-scan must still CRC-walk to find that boundary,
    // but it must refuse to recover a partial prefix from a sealed segment.
    let dir = TempDir::new().expect("tmpdir");
    let path = segment_with_frames(
        &dir,
        11,
        &[
            serde_json::json!({"v": "one"}),
            serde_json::json!({"v": "two"}),
        ],
    );
    append_non_sdxn_tail(&path);

    let result = scan_reader(&dir)
        .scan_segment(&path)
        .map(|entries| entries.len());
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 11, ref detail })
            if detail.contains("strict full scan refuses")
        ),
        "PROPERTY: a strict full scan of a sealed segment must fail closed on a non-SDXn tail, \
         not silently recover a partial prefix; got {result:?}"
    );
}

#[test]
fn append_frames_from_segment_fails_closed_on_a_non_sdxn_tail() {
    // Compaction copy is also always strict: copying only the CRC-valid prefix of
    // a sealed source with a garbage tail would publish a merged segment that
    // silently dropped whatever came after the corruption point.
    let source_dir = TempDir::new().expect("source tmpdir");
    let source_path = segment_with_frames(
        &source_dir,
        13,
        &[
            serde_json::json!({"v": "one"}),
            serde_json::json!({"v": "two"}),
        ],
    );
    append_non_sdxn_tail(&source_path);

    let dest_dir = TempDir::new().expect("dest tmpdir");
    let fs = realfs();
    let mut dest = Segment::<Active>::create_with_created_ns_on(dest_dir.path(), 99, 0, &fs)
        .expect("create destination segment");
    let result = dest.append_frames_from_segment(&source_path);
    assert!(
        matches!(
            result,
            Err(StoreError::CorruptSegment { segment_id: 0, ref detail })
            if detail.contains("strict compaction copy refuses")
        ),
        "PROPERTY: compaction copy must fail closed on a non-SDXn tail, not copy a partial \
         prefix; got {result:?}"
    );
}

#[test]
fn scan_segment_without_a_footer_still_recovers_frames_to_eof() {
    // The same None branch also covers genuine no-footer segments: the CRC walk
    // reaches EOF and yields the same frame count as the old frames_end=file_len
    // path, now with the tail validated by construction.
    let dir = TempDir::new().expect("tmpdir");
    let path = segment_with_frames(
        &dir,
        12,
        &[
            serde_json::json!({"v": "one"}),
            serde_json::json!({"v": "two"}),
        ],
    );

    let entries = scan_reader(&dir)
        .scan_segment(&path)
        .expect("no-footer segment scans to EOF");
    assert_eq!(
        entries.len(),
        2,
        "PROPERTY: a legitimate no-footer segment still recovers every CRC-valid frame to EOF"
    );
}

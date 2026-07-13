//! Compaction pending-marker on-disk format + the segment-listing entry point.
//!
//! The marker (`compaction.pending`) is a canonical versioned BINARY artifact
//! (#177/#195, A9): `magic(6) | version(2 le) | crc32(4 le) | body(msgpack
//! PendingCompactionV2)` — the same fixed prefix every other store format uses
//! (`wire_header`). Its payload carries the transaction's BRANDED identity
//! (`compaction_id`, store `lineage`, `expected_authority_image_id`) so recovery
//! decides direction by three-artifact agreement across the marker, the durable
//! authority image, and `store.meta` — never by file existence (A8). The
//! decision + physical repair live in `compaction_recovery.rs`; this module only
//! reads/writes the marker and lists segments.
//!
//! A legacy JSON marker (`compaction.pending.json`, ≤0.10.x) is parsed for
//! DIAGNOSTICS ONLY and unconditionally refuses with
//! `CompactionRecoveryRefusal::LegacyMarkerUnsupported` — there is no automatic
//! v1 recovery, ever. Offline migration or a typed refusal are its only exits.

use crate::store::file_classification::{
    StoreFileKind, COMPACT_SOURCE_EXTENSION, COMPACT_STAGED_EXTENSION,
};
use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
use crate::store::platform::fs::StoreFs;
use crate::store::segment;
use crate::store::{CompactionRecoveryRefusal, StoreError};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The canonical (binary, v2+) pending-compaction marker filename.
pub(crate) const COMPACTION_MARKER_FILENAME: &str = "compaction.pending";

/// The legacy (JSON, ≤0.10.x) marker filename. Present only to detect and
/// refuse old bytes (A9); never written, never recovered.
const LEGACY_COMPACTION_MARKER_FILENAME: &str = "compaction.pending.json";

/// Marker magic (6 bytes, `wire_header` layout): distinct from every other
/// store format's magic so a mislabeled file cannot masquerade as a marker.
const COMPACTION_MARKER_MAGIC: &[u8; 6] = b"FBATCM";

/// The only marker schema version this binary reads or writes. A marker
/// declaring a higher version is a typed `MarkerFutureVersion` refusal.
const COMPACTION_MARKER_VERSION: u16 = 2;

/// v2 marker payload. The branded ids (A3/A4) bind this transaction to the
/// SAME `compaction_id`/`authority_image_id` stamped in the durable authority
/// image and `store.meta`, so recovery can require three-artifact agreement
/// (A8) rather than trusting a filename.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PendingCompactionV2 {
    pub(crate) compaction_id: CompactionId,
    pub(crate) lineage: StoreLineage,
    pub(crate) merged_id: u64,
    pub(crate) source_segment_ids: Vec<u64>,
    pub(crate) expected_authority_image_id: AuthorityImageId,
}

/// Legacy v1 marker — DIAGNOSTIC PARSE ONLY (A9). It never flows through the
/// recovery state machine; its presence is an unconditional
/// `LegacyMarkerUnsupported` refusal. Decoded solely to enrich the refusal's
/// observed-fields diagnostic.
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct PendingCompactionV1 {
    pub(crate) merged_id: u64,
    pub(crate) source_segment_ids: Vec<u64>,
}

fn pending_compaction_path(data_dir: &Path) -> PathBuf {
    data_dir.join(COMPACTION_MARKER_FILENAME)
}

fn legacy_pending_compaction_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LEGACY_COMPACTION_MARKER_FILENAME)
}

pub(crate) fn compaction_source_temp_path(data_dir: &Path, merged_id: u64) -> PathBuf {
    data_dir.join(format!(
        "{merged_id:06}.{}.{}",
        segment::SEGMENT_EXTENSION,
        COMPACT_SOURCE_EXTENSION
    ))
}

/// The staged replacement name (`NNNNNN.fbat.compact-new`): an in-flight
/// compaction replacement is materialized here and is renamed to the final
/// segment name only after the commit record is durable (A9/A11). An
/// incomplete replacement can therefore never occupy the final committed name.
pub(crate) fn compaction_staged_path(data_dir: &Path, merged_id: u64) -> PathBuf {
    data_dir.join(format!(
        "{merged_id:06}.{}.{}",
        segment::SEGMENT_EXTENSION,
        COMPACT_STAGED_EXTENSION
    ))
}

/// Load the pending-compaction marker.
///
/// * binary `compaction.pending` present → decode/validate → `Ok(Some(v2))` or
///   a typed `MarkerCorrupt`/`MarkerFutureVersion` refusal;
/// * binary absent, legacy JSON present → `Err(LegacyMarkerUnsupported)` (A9 —
///   old bytes always refuse, parsed only for the diagnostic);
/// * neither present → `Ok(None)`.
pub(crate) fn load_pending_compaction(
    data_dir: &Path,
    fs: &dyn StoreFs,
) -> Result<Option<PendingCompactionV2>, StoreError> {
    let path = pending_compaction_path(data_dir);
    let bytes = match fs.read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return refuse_any_legacy_marker(data_dir, fs);
        }
        Err(error) => return Err(StoreError::Io(error)),
    };
    parse_v2_marker(&path, &bytes).map(Some)
}

/// With no binary marker present, a legacy JSON marker is an unconditional
/// refusal; its absence is a clean `Ok(None)`.
fn refuse_any_legacy_marker(
    data_dir: &Path,
    fs: &dyn StoreFs,
) -> Result<Option<PendingCompactionV2>, StoreError> {
    let legacy_path = legacy_pending_compaction_path(data_dir);
    let bytes = match fs.read(&legacy_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StoreError::Io(error)),
    };
    // Parsed for the diagnostic ONLY — the presence of the file, not its
    // contents, is what refuses.
    let observed = match serde_json::from_slice::<PendingCompactionV1>(&bytes) {
        Ok(v1) => format!(
            "merged_id={}, source_segment_ids={:?}",
            v1.merged_id, v1.source_segment_ids
        ),
        Err(error) => format!("unparseable legacy marker: {error}"),
    };
    Err(StoreError::CompactionRecoveryRefused {
        marker_path: legacy_path.clone(),
        kind: CompactionRecoveryRefusal::LegacyMarkerUnsupported {
            path: legacy_path,
            observed,
        },
    })
}

fn parse_v2_marker(path: &Path, raw: &[u8]) -> Result<PendingCompactionV2, StoreError> {
    let prefix = match crate::store::wire_header::parse(raw, COMPACTION_MARKER_MAGIC) {
        Ok(prefix) => prefix,
        Err(crate::store::wire_header::PrefixError::TooShort { len }) => {
            return Err(marker_corrupt(
                path,
                format!(
                    "marker shorter than the {}-byte header (got {len} bytes)",
                    crate::store::wire_header::HEADER_LEN
                ),
            ));
        }
        Err(crate::store::wire_header::PrefixError::BadMagic) => {
            return Err(marker_corrupt(path, "marker magic mismatch".to_string()));
        }
    };
    if prefix.version > COMPACTION_MARKER_VERSION {
        return Err(StoreError::CompactionRecoveryRefused {
            marker_path: path.to_path_buf(),
            kind: CompactionRecoveryRefusal::MarkerFutureVersion {
                found: prefix.version,
                supported: COMPACTION_MARKER_VERSION,
            },
        });
    }
    if prefix.version != COMPACTION_MARKER_VERSION {
        return Err(marker_corrupt(
            path,
            format!(
                "unsupported marker version {} (this binary reads v{COMPACTION_MARKER_VERSION})",
                prefix.version
            ),
        ));
    }
    let computed_crc = crc32fast::hash(prefix.body);
    if prefix.stored_crc != computed_crc {
        return Err(marker_corrupt(
            path,
            format!(
                "crc mismatch: stored {:#010x}, computed {computed_crc:#010x}",
                prefix.stored_crc
            ),
        ));
    }
    match crate::encoding::from_bytes::<PendingCompactionV2>(prefix.body) {
        Ok(marker) => Ok(marker),
        Err(error) => Err(marker_corrupt(
            path,
            format!("marker body decode failed: {error}"),
        )),
    }
}

fn marker_corrupt(path: &Path, detail: String) -> StoreError {
    StoreError::CompactionRecoveryRefused {
        marker_path: path.to_path_buf(),
        kind: CompactionRecoveryRefusal::MarkerCorrupt { detail },
    }
}

pub(crate) fn write_pending_compaction(
    data_dir: &Path,
    marker: &PendingCompactionV2,
    fs: &dyn StoreFs,
) -> Result<(), StoreError> {
    let body = crate::encoding::to_bytes(marker)
        .map_err(|error| StoreError::ser_msg(&format!("encode compaction marker: {error}")))?;
    let crc = crc32fast::hash(&body);
    let final_path = pending_compaction_path(data_dir);
    crate::store::platform::fs::write_file_atomically_with_fs(
        data_dir,
        &final_path,
        "compaction marker",
        |file| {
            file.write_all(&crate::store::wire_header::encode(
                COMPACTION_MARKER_MAGIC,
                COMPACTION_MARKER_VERSION,
                crc,
            ))
            .map_err(StoreError::Io)?;
            file.write_all(&body).map_err(StoreError::Io)?;
            Ok(())
        },
        fs,
    )
}

pub(crate) fn clear_pending_compaction(data_dir: &Path, fs: &dyn StoreFs) -> Result<(), StoreError> {
    let path = pending_compaction_path(data_dir);
    match fs.remove_file(&path) {
        Ok(()) => {
            fs.sync_parent_dir(&path)?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(StoreError::Io(err)),
    }
}

/// List the durable segments in `data_dir`.
///
/// Fail-closed on a live marker: a pending compaction transaction MUST be
/// resolved by `resolve_pending_compaction` (writable repair) or refused
/// (read-only) BEFORE the index is rebuilt. Reaching a marker here is an
/// internal-invariant breach surfaced as a typed refusal (or the load's own
/// corrupt/legacy refusal), never a silent reinterpretation of file existence.
pub(super) fn segment_paths(
    data_dir: &Path,
    fs: &dyn StoreFs,
) -> Result<Vec<(u64, PathBuf)>, StoreError> {
    if load_pending_compaction(data_dir, fs)?.is_some() {
        return Err(StoreError::CompactionRecoveryRefused {
            marker_path: pending_compaction_path(data_dir),
            kind: CompactionRecoveryRefusal::PendingTransactionUnresolved,
        });
    }
    let mut entries = Vec::new();
    for entry in fs.read_dir(data_dir).map_err(StoreError::Io)? {
        let path = data_dir.join(&entry.name);
        let kind = StoreFileKind::from_path(&path);
        if let StoreFileKind::MalformedSegment(error) = &kind {
            tracing::warn!(
                path = %path.display(),
                %error,
                "skipping malformed segment filename"
            );
            continue;
        }
        // Every non-segment artifact (keyset, idempotency store, checkpoint,
        // staged/source compaction temporaries, …) reports no segment id and
        // is skipped.
        let Some(segment_id) = kind.segment_id() else {
            continue;
        };
        entries.push((segment_id.as_u64(), path));
    }
    entries.sort_by_key(|(segment_id, _)| *segment_id);
    Ok(entries)
}

// The unit-test island lives beside this module in `src` (R16 / plan-05 FILE 5)
// so it can drive the pub(crate)/pub(super) marker + recovery internals that
// `tests/` cannot reach.
#[cfg(test)]
#[path = "topology_crash_window_tests.rs"]
mod crash_window_tests;

#[cfg(test)]
mod tests {
    use super::{
        clear_pending_compaction, load_pending_compaction, pending_compaction_path,
        segment_paths, write_pending_compaction, PendingCompactionV2, COMPACTION_MARKER_FILENAME,
        COMPACTION_MARKER_MAGIC, COMPACTION_MARKER_VERSION,
    };
    use crate::store::generation_ids::{AuthorityImageId, CompactionId, StoreLineage};
    use crate::store::platform;
    use crate::store::platform::fs::RealFs;
    use crate::store::segment;
    use crate::store::{CompactionRecoveryRefusal, StoreError};

    fn sample_marker() -> PendingCompactionV2 {
        PendingCompactionV2 {
            compaction_id: CompactionId::from_u128(0x0102_0304_0506_0708),
            lineage: StoreLineage::from_u128(0x0a0b_0c0d_0e0f_1011),
            merged_id: 7,
            source_segment_ids: vec![1, 2, 7],
            expected_authority_image_id: AuthorityImageId::from_u128(0x00ff_00ff_00ff_00ff),
        }
    }

    #[test]
    fn clear_pending_compaction_propagates_non_not_found_errors() {
        // The match guard `err.kind() == NotFound` must let real I/O errors
        // through. Replacing it with `true` would swallow EVERY removal failure
        // as Ok(()). A DIRECTORY at the marker path makes `remove_file` fail
        // with a non-NotFound error (EISDIR/EPERM), which must surface as Err.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker = pending_compaction_path(dir.path());
        std::fs::create_dir(&marker).expect("create directory at marker path");

        let result = clear_pending_compaction(dir.path(), &RealFs);
        assert!(
            matches!(result, Err(StoreError::Io(_))),
            "a non-NotFound removal failure must propagate as Err(Io), not be \
             swallowed by the NotFound guard; got {result:?}"
        );
    }

    #[test]
    fn clear_pending_compaction_is_ok_when_marker_is_absent() {
        // The NotFound arm: an absent marker is a clean no-op success.
        let dir = tempfile::tempdir().expect("tempdir");
        clear_pending_compaction(dir.path(), &RealFs).expect("absent marker clears cleanly");
    }

    #[test]
    fn write_then_load_round_trips_a_v2_marker() {
        // The binary marker survives the magic+version+crc envelope intact,
        // branded ids and all — the three-artifact agreement (A8) depends on
        // these ids being byte-exact across reopen.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker = sample_marker();
        write_pending_compaction(dir.path(), &marker, &RealFs).expect("write v2 marker");

        let loaded = load_pending_compaction(dir.path(), &RealFs)
            .expect("load must succeed")
            .expect("PROPERTY: a just-written v2 marker is present");
        assert_eq!(
            loaded, marker,
            "PROPERTY: a v2 marker survives a binary write/load round-trip verbatim"
        );
    }

    #[test]
    fn segment_paths_refuses_while_a_pending_marker_exists() {
        // A live marker means recovery has not yet resolved the transaction;
        // segment listing must fail closed, never reinterpret file existence.
        let dir = tempfile::tempdir().expect("tempdir");
        write_pending_compaction(dir.path(), &sample_marker(), &RealFs).expect("write marker");

        let result = segment_paths(dir.path(), &RealFs);
        assert!(
            matches!(
                &result,
                Err(StoreError::CompactionRecoveryRefused {
                    kind: CompactionRecoveryRefusal::PendingTransactionUnresolved,
                    marker_path,
                }) if *marker_path == pending_compaction_path(dir.path())
            ),
            "PROPERTY: a live pending marker refuses segment listing with \
             PendingTransactionUnresolved; got {result:?}"
        );
    }

    #[test]
    fn segment_paths_lists_plain_segments_when_no_marker_exists() {
        // The Ok anchor: with no marker, listing is a plain id-sorted directory
        // scan of the segment files.
        let dir = tempfile::tempdir().expect("tempdir");
        let path_one = dir.path().join(segment::segment_filename(1));
        let path_two = dir.path().join(segment::segment_filename(2));
        drop(platform::fs::create_new_file(&path_one).expect("create segment 1"));
        drop(platform::fs::create_new_file(&path_two).expect("create segment 2"));

        let entries = segment_paths(dir.path(), &RealFs).expect("plain listing succeeds");
        assert_eq!(
            entries,
            vec![(1, path_one), (2, path_two)],
            "PROPERTY: with no marker, listing yields exactly the segment set in id order"
        );
    }

    #[test]
    fn load_refuses_a_legacy_json_marker_as_unsupported() {
        // A ≤0.10.x JSON marker is never recovered automatically (A9): its
        // presence alone is a typed LegacyMarkerUnsupported refusal.
        let dir = tempfile::tempdir().expect("tempdir");
        let legacy = dir.path().join("compaction.pending.json");
        std::fs::write(&legacy, br#"{"merged_id":1,"source_segment_ids":[1,2]}"#)
            .expect("write legacy marker");

        let result = load_pending_compaction(dir.path(), &RealFs);
        assert!(
            matches!(
                &result,
                Err(StoreError::CompactionRecoveryRefused {
                    kind: CompactionRecoveryRefusal::LegacyMarkerUnsupported { .. },
                    ..
                })
            ),
            "PROPERTY: a legacy JSON marker refuses as LegacyMarkerUnsupported; got {result:?}"
        );
    }

    #[test]
    fn load_refuses_a_future_marker_version() {
        // A marker declaring a version newer than this binary supports is a
        // hard, exact-variant refusal — the version check precedes the CRC.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(COMPACTION_MARKER_FILENAME);
        let mut bytes = crate::store::wire_header::encode(COMPACTION_MARKER_MAGIC, 3, 0).to_vec();
        bytes.extend_from_slice(b"body");
        std::fs::write(&path, &bytes).expect("write future marker");

        let result = load_pending_compaction(dir.path(), &RealFs);
        assert!(
            matches!(
                &result,
                Err(StoreError::CompactionRecoveryRefused {
                    kind: CompactionRecoveryRefusal::MarkerFutureVersion {
                        found: 3,
                        supported: 2,
                    },
                    ..
                })
            ),
            "PROPERTY: a future marker version refuses as \
             MarkerFutureVersion {{ found: 3, supported: 2 }}; got {result:?}"
        );
    }

    #[test]
    fn load_refuses_a_marker_with_a_crc_mismatch() {
        // Correct magic + supported version, but a CRC that does not cover the
        // body: the tampered/garbled marker refuses as MarkerCorrupt.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(COMPACTION_MARKER_FILENAME);
        let mut bytes = crate::store::wire_header::encode(
            COMPACTION_MARKER_MAGIC,
            COMPACTION_MARKER_VERSION,
            0,
        )
        .to_vec();
        bytes.extend_from_slice(b"tampered-body");
        std::fs::write(&path, &bytes).expect("write corrupt marker");

        let result = load_pending_compaction(dir.path(), &RealFs);
        assert!(
            matches!(
                &result,
                Err(StoreError::CompactionRecoveryRefused {
                    kind: CompactionRecoveryRefusal::MarkerCorrupt { .. },
                    ..
                })
            ),
            "PROPERTY: a marker whose CRC does not cover its body refuses as \
             MarkerCorrupt; got {result:?}"
        );
    }
}

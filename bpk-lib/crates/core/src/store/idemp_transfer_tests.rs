//! In-crate test island for the public idempotency-authority restore seam
//! (#188). These are the rejection-matrix rows the opaque public surface cannot
//! forge honestly: images built directly from crate-internal types
//! (`IdempImageV2` with its branded `StoreLineage`/`AuthorityImageId`,
//! `IdempEntry`, `wire_header::encode`, `crc32fast`) and fed through
//! `IdempotencyAuthorityExport::from_bytes` into the offline restore door.
//!
//! Restore is OFFLINE-PRIMARY (#226 A13): one closed-directory associated fn
//! `Store::restore_idempotency_authority(config, export)` that acquires the
//! store-dir lock, validates the export against this store's durable state, then
//! MINTS its own fresh generation. There is no live/adopt door and no token
//! pre-authorization check — so the caller-side guards proven here are format
//! integrity, coverage structure, lineage, and rollback. Sibling-divergence and
//! the real end-to-end doors are proven with real exports in the T1/T2 tests.

use super::IdempotencyAuthorityExport;
use crate::coordinate::Coordinate;
use crate::event::EventKind;
use crate::id::IdempotencyKey;
use crate::store::generation_ids::{AuthorityImageId, StoreLineage};
use crate::store::index::idemp::{IdempEntry, IdempImageV2, IDEMP_MAGIC, IDEMP_VERSION};
use crate::store::store_meta::{load_store_meta, IdempAuthorityAnchor};
use crate::store::{
    AppendOptions, IdempAuthorityCorruption, IdempotencyRestoreRefusal, Store, StoreConfig,
    StoreError,
};
use std::collections::BTreeMap;

// ── forge helpers (crate-internal; the point of the island) ─────────────────

/// A durable entry with the given kind so a test can place a USER
/// (non-reserved) or SYSTEM (reserved, coverage-exempt) key at a chosen
/// sequence. `recorded_global_sequence` and `global_sequence` are both set to
/// `seq` so the coverage check fires the same way regardless of which sequence
/// the validator inspects.
fn entry(key: u128, seq: u64, kind: EventKind) -> IdempEntry {
    IdempEntry {
        key,
        event_id: key,
        global_sequence: seq,
        disk_pos_segment: 0,
        disk_pos_offset: 0,
        disk_pos_length: 0,
        content_hash: [0u8; 32],
        prev_hash: [0u8; 32],
        entity: "e".to_owned(),
        scope: "s".to_owned(),
        kind,
        recorded_global_sequence: seq,
        event_evicted: false,
        receipt_extensions: BTreeMap::new(),
    }
}

/// A user-kind (`EventKind::DATA`, category 0x1 — `!is_reserved()`) entry.
fn user_entry(key: u128, seq: u64) -> IdempEntry {
    entry(key, seq, EventKind::DATA)
}

/// A system-kind (`EventKind::SYSTEM_CHECKPOINT`, category 0x0 — reserved,
/// coverage-exempt) entry.
fn system_entry(key: u128, seq: u64) -> IdempEntry {
    entry(key, seq, EventKind::SYSTEM_CHECKPOINT)
}

/// Encode a forged image into a canonical export at an ARBITRARY declared
/// header version (CRC covers the body only, so a version override stays a
/// valid parse — the header-version rows depend on this).
fn forge_export_at_version(image: &IdempImageV2, version: u16) -> IdempotencyAuthorityExport {
    let body = crate::encoding::to_bytes(image).expect("encode forged idemp image");
    let crc = crc32fast::hash(&body);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&crate::store::wire_header::encode(IDEMP_MAGIC, version, crc));
    bytes.extend_from_slice(&body);
    IdempotencyAuthorityExport::from_bytes(bytes)
}

/// Encode a forged image into a canonical current-version export.
fn forge_export(image: &IdempImageV2) -> IdempotencyAuthorityExport {
    forge_export_at_version(image, IDEMP_VERSION)
}

/// Open a store to mint `store.meta` (lineage), then close it so the directory
/// is unlocked for the offline restore door.
fn fresh_store_dir() -> (tempfile::TempDir, StoreConfig) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let config = StoreConfig::new(dir.path());
    let store = Store::open(config.clone()).expect("open store");
    store.close().expect("close store");
    (dir, config)
}

/// The lineage `store.meta` recorded, so a forged image can either match it
/// (reach the coverage/rollback guards) or deliberately diverge from it (foreign
/// lineage).
fn store_lineage(config: &StoreConfig) -> u128 {
    load_store_meta(config.data_dir(), config.fs().as_ref())
        .expect("load store.meta")
        .expect("store.meta present after open")
        .lineage
}

/// A well-formed v2 image bound to `lineage` with an anchor at `covered` and the
/// supplied entries. Branded ids are reconstructed via the internal `from_u128`
/// seam (no cross-brand conversion).
fn image_for(lineage: u128, covered: Option<u64>, entries: Vec<IdempEntry>) -> IdempImageV2 {
    IdempImageV2 {
        lineage: StoreLineage::from_u128(lineage),
        image_id: AuthorityImageId::from_u128(0x00A1),
        previous_image_id: None,
        compaction_id: None,
        anchor: covered.map(|covered_global_sequence| IdempAuthorityAnchor {
            covered_global_sequence,
            event_id_at: 0x11,
            chain_commitment: [0u8; 32],
        }),
        entries,
    }
}

// ── coverage-structure rows (matrix §0.4 row 6) ─────────────────────────────

#[test]
fn beyond_coverage_user_entry_is_rejected() {
    // A user entry recorded PAST the image's own declared anchor is a shape no
    // canonical publication produces — refused as EntriesBeyondCoverage with the
    // declared coverage carried verbatim.
    let (_dir, config) = fresh_store_dir();
    let lineage = store_lineage(&config);
    let image = image_for(lineage, Some(10), vec![user_entry(0x11, 11)]);
    let export = forge_export(&image);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: a user entry past the declared anchor must be refused");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::EntriesBeyondCoverage {
                    covered_global_sequence: 10,
                    first_offender_sequence: 11,
                }
            }
        ),
        "wrong refusal: {err:?}"
    );
}

#[test]
fn anchor_none_with_user_entries_is_beyond_coverage() {
    // An anchorless image that nonetheless carries a user obligation covers
    // sequence 0 by construction, so the user entry is beyond coverage.
    let (_dir, config) = fresh_store_dir();
    let lineage = store_lineage(&config);
    let image = image_for(lineage, None, vec![user_entry(0x22, 5)]);
    let export = forge_export(&image);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: an anchorless image with a user entry must be refused");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::EntriesBeyondCoverage {
                    covered_global_sequence: 0,
                    ..
                }
            }
        ),
        "wrong refusal: {err:?}"
    );
}

#[test]
fn system_entry_past_anchor_is_admitted() {
    // GREEN companion: a SYSTEM (reserved) entry recorded past the anchor is
    // EXEMPT — the canonical flush anchors at the max USER entry, so legal
    // exports routinely carry system lifecycle keys past coverage. This kills a
    // mutant that drops the `!is_reserved()` filter (which would wrongly refuse
    // this shape as EntriesBeyondCoverage).
    let (_dir, config) = fresh_store_dir();
    let lineage = store_lineage(&config);
    let image = image_for(lineage, Some(10), vec![system_entry(0x33, 20)]);
    let export = forge_export(&image);
    let result = Store::restore_idempotency_authority(&config, &export);
    assert!(
        !matches!(
            &result,
            Err(StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::EntriesBeyondCoverage { .. }
            })
        ),
        "PROPERTY: a system entry past the anchor must be exempt from the \
         beyond-coverage filter; got {result:?}"
    );
}

// ── format-integrity rows (matrix §0.4 row 1/2) ─────────────────────────────

#[test]
fn truncated_export_is_corrupt_too_short() {
    // Fewer than the 12-byte header cannot parse: TooShort, surfaced as a typed
    // restore Corrupt refusal (parse runs before any store-state guard).
    let (_dir, config) = fresh_store_dir();
    let export = IdempotencyAuthorityExport::from_bytes(vec![0u8; 3]);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: a 3-byte export cannot be a canonical image");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::Corrupt {
                    kind: IdempAuthorityCorruption::TooShort { .. }
                }
            }
        ),
        "wrong refusal: {err:?}"
    );
}

#[test]
fn bad_magic_export_is_corrupt() {
    // A header-length buffer whose leading bytes are not `FBATID` is BadMagic.
    let (_dir, config) = fresh_store_dir();
    let export = IdempotencyAuthorityExport::from_bytes(vec![0u8; 20]);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: a wrong-magic export cannot be a canonical image");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::Corrupt {
                    kind: IdempAuthorityCorruption::BadMagic
                }
            }
        ),
        "wrong refusal: {err:?}"
    );
}

// ── future-version row (matrix §0.4 row 2) ──────────────────────────────────

#[test]
fn future_version_export_is_rejected() {
    // A body-valid image stamped one version ahead: the CRC (body-only) stays
    // valid, so the header version is what refuses — FutureVersion with the
    // exact declared/supported pair.
    let (_dir, config) = fresh_store_dir();
    let lineage = store_lineage(&config);
    let image = image_for(lineage, Some(1), Vec::new());
    let export = forge_export_at_version(&image, IDEMP_VERSION + 1);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: a future-version export must be refused");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::FutureVersion { found, supported }
            } if found == IDEMP_VERSION + 1 && supported == IDEMP_VERSION
        ),
        "wrong refusal: {err:?}"
    );
}

// ── lineage row (matrix §0.4 row 4) ─────────────────────────────────────────

#[test]
fn foreign_lineage_export_is_rejected() {
    // An image bound to a different lineage is never restorable — the export
    // belongs to an unrelated store. `wrapping_add(1)` guarantees the forged
    // lineage differs from the store's minted one.
    let (_dir, config) = fresh_store_dir();
    let lineage = store_lineage(&config);
    let image = image_for(lineage.wrapping_add(1), Some(1), Vec::new());
    let export = forge_export(&image);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: a foreign-lineage export must be refused");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::ForeignLineage { .. }
            }
        ),
        "wrong refusal: {err:?}"
    );
}

// ── rollback row (matrix §0.4 row 3) ────────────────────────────────────────

#[test]
fn stale_rollback_below_recorded_authority_is_rejected() {
    // A store that has flushed a real user obligation holds authority through
    // some covered frontier S. An export anchored BELOW S would narrow the dedup
    // contract — refused as StaleRollback with the exact covered pair.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let config = StoreConfig::new(dir.path());
    {
        let store = Store::open(config.clone()).expect("open store");
        let coord = Coordinate::new("acct:stale", "scope:idemp-restore").expect("coord");
        let opts = AppendOptions::default().with_idempotency(IdempotencyKey::from(0x0A11CE_u128));
        store
            .append_with_options(&coord, EventKind::DATA, &serde_json::json!({ "n": 1 }), opts)
            .expect("keyed user append");
        store.close().expect("close store");
    }

    let meta = load_store_meta(config.data_dir(), config.fs().as_ref())
        .expect("load store.meta")
        .expect("store.meta present after keyed append");
    let lineage = meta.lineage;
    let expected_covered = meta
        .idemp_authority
        .expect("a user keyed append records a durable-authority expectation")
        .anchor
        .covered_global_sequence;
    assert!(
        expected_covered >= 1,
        "the keyed append advanced the covered frontier: {expected_covered}"
    );

    let image_covered = expected_covered - 1;
    let image = image_for(lineage, Some(image_covered), vec![user_entry(0x11, image_covered)]);
    let export = forge_export(&image);
    let err = Store::restore_idempotency_authority(&config, &export)
        .expect_err("PROPERTY: an export behind recorded authority must be refused");
    assert!(
        matches!(
            &err,
            StoreError::IdempotencyRestoreRefused {
                reason: IdempotencyRestoreRefusal::StaleRollback {
                    image_covered: found_covered,
                    expected_covered: found_expected,
                }
            } if *found_covered == image_covered && *found_expected == expected_covered
        ),
        "wrong refusal: {err:?}"
    );
}

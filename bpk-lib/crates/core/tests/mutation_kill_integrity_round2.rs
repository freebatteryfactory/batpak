//! PROVES: round-2 mutation kills for the chain-integrity / config surface —
//! the at-open `ChainVerification::Recompute` recompute actually bites (a
//! CRC-valid payload tamper is refused with the exact
//! `StoreError::ChainVerificationFailed` counts), `ChainVerificationReport::
//! is_intact` requires BOTH conjuncts, the batch size cap is inclusive at the
//! boundary, the snapshot destination pre-clear wipes only store-shaped
//! artifacts (never a foreign file, never the crypto-shred keyset), and
//! `with_fault_injector` carries the injector into the opened store's writer.
//! CATCHES: `run_open_chain_verification -> Ok(())` (open.rs:243), `&&`->`||`
//! in `is_intact` (read_api.rs:25), `>`->`>=` in `validate_batch`
//! (writer/batch.rs:65), `should_clear_from_snapshot_destination -> true`
//! (file_classification.rs:121), and `with_fault_injector -> Default::
//! default()` (config.rs:528) — plus `StoreConfig::chain_verification ->
//! Default::default()` (config.rs:493), which would silently disable the
//! configured Recompute refusal below.
//! SEEDED: deterministic — fixed payload needles, surgical byte flips with
//! re-stamped frame CRCs, no randomness, no timing.

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::id::EventId;
use batpak::store::segment::SEGMENT_EXTENSION;
use batpak::store::{
    AppendOptions, BatchAppendItem, CausationRef, ChainVerification, ChainVerificationReport,
    SnapshotFinding, Store, StoreConfig, StoreError,
};
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xE, 0x21);

/// The payload marker the tamper helper flips one byte of. ASCII, so the
/// mutated byte (XOR 0x01) stays ASCII and the frame still decodes cleanly.
const NEEDLE: &[u8] = b"TAMPER-EVIDENCE-NEEDLE-0123456789";

/// Force the frame-scan open path (no checkpoint / mmap fast path), so the
/// tampered frame is re-read from disk on every reopen.
fn scan_config(dir: &TempDir) -> StoreConfig {
    StoreConfig::new(dir.path())
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_sync_every_n_events(1)
}

/// Locate the segment holding the needle event (reopen cycles rotate to fresh
/// active segments, so the data dir legitimately holds several segments).
fn needle_segment_path(dir: &TempDir) -> std::path::PathBuf {
    let mut found = None;
    for entry in std::fs::read_dir(dir.path()).expect("read data dir") {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(SEGMENT_EXTENSION) {
            continue;
        }
        let bytes = std::fs::read(&path).expect("read segment bytes");
        if bytes.windows(NEEDLE.len()).any(|window| window == NEEDLE) {
            assert!(
                found.is_none(),
                "the needle must live in exactly one segment; also found it in {path:?}"
            );
            found = Some(path);
        }
    }
    found.expect("exactly one segment must hold the needle event")
}

/// Flip one payload byte inside the needle and RE-STAMP the enclosing frame's
/// CRC, producing the tamper class `ChainVerification::Recompute` exists for:
/// invisible to the per-frame CRC, visible only to a blake3 recompute against
/// the stored `event_hash`.
fn flip_payload_byte_and_fix_frame_crc(seg: &std::path::Path) {
    let mut bytes = std::fs::read(seg).expect("read segment bytes");
    let needle_at = bytes
        .windows(NEEDLE.len())
        .position(|window| window == NEEDLE)
        .expect("the needle payload must appear in the segment");

    // Segment layout: magic(4) + header_len(u32 BE) + header, then frames of
    // [len:u32 BE][crc32:u32 BE][msgpack].
    let header_len_bytes: [u8; 4] = bytes[4..8]
        .try_into()
        .expect("segment header length prefix");
    let mut cursor = 8_usize
        + usize::try_from(u32::from_be_bytes(header_len_bytes)).expect("header len fits usize");

    let mut tampered = false;
    while cursor + 8 <= bytes.len() {
        let frame_len_bytes: [u8; 4] = bytes[cursor..cursor + 4]
            .try_into()
            .expect("frame length prefix");
        let frame_len =
            usize::try_from(u32::from_be_bytes(frame_len_bytes)).expect("frame len fits usize");
        let msgpack_start = cursor + 8;
        let msgpack_end = msgpack_start + frame_len;
        assert!(
            msgpack_end <= bytes.len(),
            "frame walk must stay inside the segment (found the needle before the SIDX footer)"
        );
        if (msgpack_start..msgpack_end).contains(&needle_at) {
            bytes[needle_at] ^= 0x01;
            let crc = crc32fast::hash(&bytes[msgpack_start..msgpack_end]);
            bytes[cursor + 4..cursor + 8].copy_from_slice(&crc.to_be_bytes());
            tampered = true;
            break;
        }
        cursor = msgpack_end;
    }
    assert!(tampered, "the needle-bearing frame must be tampered");
    std::fs::write(seg, bytes).expect("write tampered segment");
}

/// KILLS open.rs:243 `run_open_chain_verification -> Ok(())` (and
/// config.rs:493 `chain_verification -> Default::default()`, which would read
/// the configured `Recompute` back as the default `Crc` and skip the check).
#[test]
fn recompute_at_open_refuses_a_crc_valid_payload_tamper_with_the_exact_error() {
    let dir = TempDir::new().expect("temp dir");
    {
        let store = Store::open(scan_config(&dir)).expect("open store for seeding");
        let tampered_coord = Coordinate::new("entity:tamper-target", "scope:mk2").expect("coord");
        let intact_coord = Coordinate::new("entity:intact", "scope:mk2").expect("coord");
        let _ = store
            .append(
                &tampered_coord,
                KIND,
                &serde_json::json!({ "secret": "TAMPER-EVIDENCE-NEEDLE-0123456789" }),
            )
            .expect("append the needle event");
        let _ = store
            .append(&intact_coord, KIND, &serde_json::json!({ "n": 1 }))
            .expect("append an intact event");
        store.close().expect("clean close");
    }

    // CONTROL 1: untampered, the Recompute open succeeds — the fixture itself
    // never trips verification.
    {
        let store =
            Store::open(scan_config(&dir).with_chain_verification(ChainVerification::Recompute))
                .expect("an untampered store opens under Recompute");
        store.close().expect("close untampered Recompute control");
    }

    flip_payload_byte_and_fix_frame_crc(&needle_segment_path(&dir));

    // CONTROL 2: the tamper is INVISIBLE to the default CRC-trusting open, so
    // the refusal below can only come from the at-open recompute.
    {
        let store = Store::open(scan_config(&dir)).expect("a CRC-only open cannot see the tamper");
        store.close().expect("close CRC control");
    }

    // THE KILL: `Recompute` must FAIL CLOSED with the exact refusal — exactly
    // the one tampered event, no dangling links. A neutered
    // `run_open_chain_verification` opens this store instead.
    let refused =
        Store::open(scan_config(&dir).with_chain_verification(ChainVerification::Recompute)).err();
    assert!(
        matches!(
            refused,
            Some(StoreError::ChainVerificationFailed {
                content_hash_mismatches: 1,
                dangling_links: 0,
            })
        ),
        "PROPERTY: Recompute at open must refuse a CRC-valid payload tamper with \
         ChainVerificationFailed {{ content_hash_mismatches: 1, dangling_links: 0 }}; got {refused:?}"
    );
}

/// KILLS read_api.rs:25 `&&` -> `||` in `ChainVerificationReport::is_intact`:
/// each conjunct is made false ALONE, so the OR-mutant reports intact where
/// the real conjunction must not.
#[test]
fn is_intact_requires_no_mismatches_and_no_dangling_links_independently() {
    let mismatch_only = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: vec![EventId::from(0xA2_u128)],
        dangling_links: Vec::new(),
    };
    assert!(
        !mismatch_only.is_intact(),
        "PROPERTY: a content-hash mismatch ALONE (no dangling links) must report NOT intact"
    );

    let dangling_only = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: Vec::new(),
        dangling_links: vec![EventId::from(0xD1_u128)],
    };
    assert!(
        !dangling_only.is_intact(),
        "PROPERTY: a dangling chain link ALONE (no content mismatch) must report NOT intact"
    );

    let clean = ChainVerificationReport {
        events_checked: 3,
        content_hash_mismatches: Vec::new(),
        dangling_links: Vec::new(),
    };
    assert!(
        clean.is_intact(),
        "PROPERTY: no mismatches and no dangling links must report intact"
    );
}

fn plain_batch_items(coord: &Coordinate, count: usize) -> Vec<BatchAppendItem> {
    (0..count)
        .map(|i| {
            BatchAppendItem::new(
                coord.clone(),
                KIND,
                &serde_json::json!({ "i": i }),
                AppendOptions::default(),
                CausationRef::None,
            )
            .expect("construct batch item")
        })
        .collect()
}

/// KILLS writer/batch.rs:65 `>` -> `>=` in `validate_batch`: BOTH sides of the
/// size boundary are pinned — exactly-at-limit accepted, one-over rejected
/// with the exact typed refusal.
#[test]
fn batch_size_cap_accepts_exactly_at_limit_and_rejects_one_over() {
    let dir = TempDir::new().expect("temp dir");
    let store =
        Store::open(StoreConfig::new(dir.path()).with_batch_max_size(2)).expect("open store");
    let coord = Coordinate::new("entity:batch-cap", "scope:mk2").expect("coord");

    let receipts = store
        .append_batch(plain_batch_items(&coord, 2))
        .expect("PROPERTY: a batch of exactly max_size items must be ACCEPTED (cap is inclusive)");
    assert_eq!(
        receipts.len(),
        2,
        "the exactly-at-limit batch must commit every item"
    );

    let err = store
        .append_batch(plain_batch_items(&coord, 3))
        .expect_err("PROPERTY: a batch one item over max_size must be REJECTED");
    assert!(
        matches!(
            &err,
            StoreError::BatchFailed { item_index: 0, source }
                if matches!(
                    source.as_ref(),
                    StoreError::Configuration(msg) if msg == "batch size 3 exceeds max 2"
                )
        ),
        "PROPERTY: the over-limit refusal must be BatchFailed {{ item_index: 0, \
         Configuration(\"batch size 3 exceeds max 2\") }}; got {err:?}"
    );

    store.close().expect("close");
}

/// KILLS file_classification.rs:121 `should_clear_from_snapshot_destination ->
/// true`: the snapshot pre-clear must wipe store-shaped artifacts (a stale
/// segment) while a foreign file AND the crypto-shred keyset survive —
/// deleting the keyset from a destination would crypto-shred every encrypted
/// payload a real store there depends on.
#[test]
fn snapshot_preclear_wipes_stale_segments_but_never_foreign_or_keyset_files() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open source store");
    let coord = Coordinate::new("entity:snap-preclear", "scope:mk2").expect("coord");
    for i in 0..3 {
        let _ = store
            .append(&coord, KIND, &serde_json::json!({ "i": i }))
            .expect("append");
    }

    let dest = TempDir::new().expect("dest dir");
    std::fs::write(
        dest.path().join("user-notes.txt"),
        b"caller-owned foreign file",
    )
    .expect("seed foreign file");
    std::fs::write(
        dest.path().join("keyset.fbatk"),
        b"resident crypto-shred keyset",
    )
    .expect("seed keyset file");
    std::fs::write(dest.path().join("000099.fbat"), b"stale segment bytes")
        .expect("seed stale segment");

    let report = store
        .snapshot_with_evidence(dest.path())
        .expect("snapshot into the pre-seeded destination");

    assert!(
        dest.path().join("user-notes.txt").exists(),
        "PROPERTY: a foreign (Other) file must SURVIVE the snapshot pre-clear"
    );
    assert!(
        dest.path().join("keyset.fbatk").exists(),
        "PROPERTY: the crypto-shred keyset must NEVER be cleared from a snapshot \
         destination — deleting it would shred every encrypted payload"
    );
    assert!(
        !dest.path().join("000099.fbat").exists(),
        "PROPERTY: a stale store segment in the destination must be cleared before copy"
    );
    assert!(
        report
            .body
            .findings
            .contains(&SnapshotFinding::DestinationCleared { artifact_count: 1 }),
        "PROPERTY: exactly ONE artifact (the stale segment) is cleared — a blanket \
         clear-everything would report 3; findings were {:?}",
        report.body.findings
    );

    store.close().expect("close");
}

/// KILLS config.rs:528 `with_fault_injector -> Default::default()`: the
/// builder must carry the injector into the opened store — the armed injector
/// FIRES on its matching append (exact `FaultInjected` refusal) while a
/// non-matching append proceeds, proving the filter (not a dead store) decides.
#[cfg(feature = "dangerous-test-hooks")]
#[test]
fn with_fault_injector_arms_the_injector_inside_the_opened_store() {
    use batpak::store::{CountdownAction, CountdownInjector, InjectionPoint};
    use std::sync::Arc;

    let dir = TempDir::new().expect("temp dir");
    let injector = CountdownInjector::new(1, CountdownAction::Fail("mutation-kill injected fault"))
        .with_filter(|point| {
            matches!(
                point,
                InjectionPoint::SingleAppendStart { entity } if entity == "entity:faulted"
            )
        });
    let store =
        Store::open(StoreConfig::new(dir.path()).with_fault_injector(Some(Arc::new(injector))))
            .expect("open with an armed, entity-filtered fault injector");

    let clean = Coordinate::new("entity:clean", "scope:mk2").expect("coord");
    let _ = store
        .append(&clean, KIND, &serde_json::json!({ "n": 1 }))
        .expect("a non-matching append proceeds (the filter is honored)");

    let faulted = Coordinate::new("entity:faulted", "scope:mk2").expect("coord");
    let err = store
        .append(&faulted, KIND, &serde_json::json!({ "n": 2 }))
        .expect_err("the armed injector must fail the matching append");
    assert!(
        matches!(
            &err,
            StoreError::FaultInjected(detail) if detail.starts_with("mutation-kill injected fault")
        ),
        "PROPERTY: the injected fault must surface as FaultInjected with the armed \
         message; got {err:?}"
    );

    store.close().expect("close");
}

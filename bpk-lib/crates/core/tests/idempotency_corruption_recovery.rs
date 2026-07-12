//! Corruption posture for the durable idempotency authority (`index.idemp`).
//!
//! PROVES: INV-IDEMPOTENCY-DURABLE-WINDOW (fail-closed authority posture,
//! GAUNT-IDEMPOTENCY-AUTHORITY #189). A corrupt `index.idemp` (bad CRC /
//! truncated / wrong magic / garbled version) is a TYPED OPEN REFUSAL — never
//! reinterpreted as an empty set. After retention compaction evicts an event's
//! frames, the sidecar is the only remaining proof that a keyed retry is not a
//! new command; the retired degrade-to-empty posture let an acknowledged retry
//! commit a second event. Restoring the healthy bytes clears the refusal and
//! the ORIGINAL key still deduplicates.
//! CATCHES: the `Invalid -> Missing` collapse (corruption downgraded to
//! absence), a wrong-variant refusal, and a "recovery" that loses the durable
//! dedup history it was refusing to protect.
//! SEEDED: fixed key, deterministic file mutation, healthy-bytes backup for
//! the restore leg.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::{EntityIdType, IdempotencyKey};
use batpak::store::{AppendOptions, IdempAuthorityCorruption, Store, StoreConfig, StoreError};
use std::io::Write;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 3);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:idem", "scope:corrupt").expect("valid coord")
}

fn config(dir: &TempDir) -> StoreConfig {
    StoreConfig::new(dir.path())
        .with_enable_checkpoint(false)
        .with_enable_mmap_index(false)
        .with_segment_max_bytes(512)
        .with_sync_every_n_events(1)
}

fn append_keyed(store: &Store, key: u128) -> batpak::store::AppendReceipt {
    let payload_tag = u64::try_from(key & u128::from(u64::MAX)).expect("low 64 bits fit u64");
    store
        .append_with_options(
            &coord(),
            KIND,
            &serde_json::json!({ "k": payload_tag }),
            AppendOptions::new().with_idempotency(IdempotencyKey::from(key)),
        )
        .expect("keyed append")
}

/// Count user events of `KIND` carrying the given key (a duplicate would push
/// this above 1).
fn key_event_count(store: &Store, key: u128) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_kind() == KIND && e.event_id().as_u128() == key)
        .count()
}

/// Run one corruption scenario: seed a keyed event, mutate the sidecar,
/// assert the typed fail-closed refusal, then RESTORE the healthy bytes and
/// prove the original key still deduplicates (the payoff of refusing).
/// Returns the corruption kind the refusal carried for exact-variant checks.
fn assert_fail_closed(
    mutate: impl FnOnce(&std::path::Path),
) -> Result<IdempAuthorityCorruption, Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let key = 0x9999_8888_7777_6666_5555_4444_3333_2222u128;

    let first_sequence;
    {
        let store = Store::open(config(&dir)).expect("open");
        let first = append_keyed(&store, key);
        first_sequence = first.global_sequence;
        assert_eq!(key_event_count(&store, key), 1, "one keyed event committed");
        store.close().expect("close");
    }

    let path = dir.path().join(IDEMP_FILENAME);
    let healthy = std::fs::read(&path)?;
    mutate(&path);

    // Damaged authority bytes are a typed refusal, never an empty map.
    let err =
        match Store::open(config(&dir)) {
            Ok(_) => return Err(std::io::Error::other(
                "PROPERTY (#189): corrupt index.idemp must refuse open — corruption is not absence",
            )
            .into()),
            Err(e) => e,
        };
    let StoreError::IdempotencyAuthorityCorrupt {
        path: err_path,
        kind,
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert_eq!(
        err_path.file_name().and_then(|name| name.to_str()),
        Some(IDEMP_FILENAME),
        "the refusal names the sidecar; got {}",
        err_path.display()
    );

    // Restoring the healthy image clears the refusal, and the ORIGINAL key
    // still deduplicates — the history the refusal protected is intact.
    std::fs::write(&path, &healthy)?;
    let store = Store::open(config(&dir)).expect("restored authority admits the store");
    let replay = append_keyed(&store, key);
    assert_eq!(
        replay.global_sequence, first_sequence,
        "PROPERTY (#189): after restore, retrying the ORIGINAL key is a no-op returning the \
         original sequence — no second event was ever possible"
    );
    assert_eq!(
        key_event_count(&store, key),
        1,
        "no duplicate keyed event exists after refusal + restore + retry"
    );
    store.close().expect("close");
    Ok(kind)
}

#[test]
fn corrupt_crc_refuses_open_and_restore_recovers() -> Result<(), Box<dyn std::error::Error>> {
    let kind = assert_fail_closed(|path| {
        let mut bytes = std::fs::read(path).expect("read idemp file");
        assert!(bytes.len() > 12, "idemp file should have a header + body");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(path, &bytes).expect("write corrupted idemp file");
    })?;
    assert!(
        matches!(kind, IdempAuthorityCorruption::CrcMismatch { .. }),
        "a flipped body byte is a CRC mismatch; got {kind:?}"
    );
    Ok(())
}

#[test]
fn wrong_magic_refuses_open_and_restore_recovers() -> Result<(), Box<dyn std::error::Error>> {
    let kind = assert_fail_closed(|path| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open idemp for clobber");
        file.write_all(b"XXXXXX").expect("clobber magic");
        file.sync_all().expect("sync");
    })?;
    assert!(
        matches!(kind, IdempAuthorityCorruption::BadMagic),
        "a clobbered magic is BadMagic; got {kind:?}"
    );
    Ok(())
}

#[test]
fn truncated_file_refuses_open_and_restore_recovers() -> Result<(), Box<dyn std::error::Error>> {
    let kind = assert_fail_closed(|path| {
        std::fs::write(path, b"FBA").expect("truncate idemp file");
    })?;
    assert!(
        matches!(kind, IdempAuthorityCorruption::TooShort { .. }),
        "a 3-byte file is TooShort; got {kind:?}"
    );
    Ok(())
}

#[test]
fn garbled_zero_version_refuses_open_and_restore_recovers() -> Result<(), Box<dyn std::error::Error>>
{
    // A corrupted `version = 0` with a CRC-VALID body must NOT load as any
    // version (the CRC excludes the header, so the body alone cannot catch a
    // flipped version field) — and must NOT degrade to absence either.
    let kind = assert_fail_closed(|path| {
        let mut bytes = std::fs::read(path).expect("read idemp file");
        assert!(bytes.len() >= 12, "idemp file should have a header + body");
        let bad_version: u16 = 0;
        bytes[6..8].copy_from_slice(&bad_version.to_le_bytes());
        std::fs::write(path, &bytes).expect("write version-0 idemp file");
    })?;
    assert!(
        matches!(
            kind,
            IdempAuthorityCorruption::UnsupportedVersion { observed: 0, .. }
        ),
        "a zero version is UnsupportedVersion; got {kind:?}"
    );
    Ok(())
}

#[test]
fn future_version_is_a_hard_error_at_cold_start() {
    // A future on-disk version must FAIL CLOSED (mirroring schema-evo
    // FutureVersion): a reader can never reconstruct a format it predates.
    let dir = TempDir::new().expect("tempdir");
    let key = 0x4242_4242_4242_4242_4242_4242_4242_4242u128;
    {
        let store = Store::open(config(&dir)).expect("open");
        let _ = append_keyed(&store, key);
        store.close().expect("close");
    }

    // Rewrite the version field (bytes 6..8, little-endian) to a future value;
    // the CRC covers only the body, so the version check fires first.
    let path = dir.path().join(IDEMP_FILENAME);
    let mut bytes = std::fs::read(&path).expect("read idemp file");
    assert!(bytes.len() >= 12);
    let future_version: u16 = 999;
    bytes[6..8].copy_from_slice(&future_version.to_le_bytes());
    std::fs::write(&path, &bytes).expect("write future-version idemp file");

    let err = Store::open(config(&dir))
        .err()
        .expect("future-version idemp must fail open closed");
    assert!(
        matches!(
            err,
            StoreError::IdempotencyFutureVersion {
                stored: 999,
                current: 2
            }
        ),
        "future-version sidecar is a hard error: {err:?}"
    );
}

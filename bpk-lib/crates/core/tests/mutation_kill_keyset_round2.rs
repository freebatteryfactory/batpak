//! PROVES: round-2 mutation kills for the durable crypto-shred keyset file —
//! the on-disk header geometry is exactly `magic(6) | version(2 le) |
//! crc(4 le) | body` (a shifted body offset breaks the load of a genuine
//! flush AND of an independently forged file), and the persisted scope
//! granularity is a real, validated discriminant: a NON-default granularity
//! round-trips, a mismatched granularity fails closed, and an unknown
//! discriminant is refused with the exact corruption reason.
//! CATCHES: `+` -> `*` in the keyset `HEADER_LEN` offset math
//! (keyscope/persist.rs:73) and `granularity_from_disc ->
//! Some(Default::default())` (keyscope/persist.rs:93) — the latter would
//! silently read every persisted granularity back as the default and either
//! reject a valid non-default keyset or accept an unknown discriminant.
//! SEEDED: deterministic — fixed nonces, fixed key bytes, forged files built
//! with the crate's own named-field MessagePack encoding.
#![cfg(feature = "payload-encryption")]

use batpak::coordinate::Coordinate;
use batpak::event::EventKind;
use batpak::id::EventId;
use batpak::store::{scope_for, KeyScope, KeyScopeGranularity, KeyStore, StoreError};
use serde::Serialize;

/// Name of the on-disk keyset artifact (kept in sync with
/// `store::file_classification::KEYSET_FILENAME`; not public API, so pinned here).
const KEYSET_FILENAME: &str = "keyset.fbatk";
const NONCE: [u8; 24] = [0x4B; 24];

fn scope(granularity: KeyScopeGranularity, entity: &str) -> KeyScope {
    let coord = Coordinate::new(entity, "scope:keyset-mk2").expect("coordinate");
    scope_for(
        granularity,
        &coord,
        EventKind::custom(0xF, 1),
        EventId::from(7_u128),
    )
}

/// Mirror of the private on-disk `KeysetWire` shape, encoded with the same
/// named-field MessagePack surface the store uses, so a forged keyset file is
/// byte-compatible with what `KeyStore::load` expects to decode.
#[derive(Serialize)]
struct ForgedKeysetWire {
    granularity: u8,
    entries: Vec<ForgedKeysetEntryWire>,
}

#[derive(Serialize)]
struct ForgedKeysetEntryWire {
    scope: Vec<u8>,
    key: [u8; 32],
}

/// Assemble a keyset file exactly per the documented layout:
/// `magic(6) | version(2 le) | crc32(body)(4 le) | body(msgpack)`.
fn forge_keyset_file(dir: &std::path::Path, granularity_disc: u8) {
    let wire = ForgedKeysetWire {
        granularity: granularity_disc,
        entries: vec![ForgedKeysetEntryWire {
            scope: vec![0x04, 0xAB],
            key: [0xCD; 32],
        }],
    };
    let body = rmp_serde::to_vec_named(&wire).expect("encode forged keyset body");
    let mut raw = Vec::with_capacity(12 + body.len());
    raw.extend_from_slice(b"FBATKS");
    raw.extend_from_slice(&1_u16.to_le_bytes());
    raw.extend_from_slice(&crc32fast::hash(&body).to_le_bytes());
    raw.extend_from_slice(&body);
    std::fs::write(dir.join(KEYSET_FILENAME), raw).expect("write forged keyset file");
}

/// KILLS keyscope/persist.rs:73 `+` -> `*` in `HEADER_LEN` (6 + 2 + 4): the
/// mutant shifts the body slice off byte 12, so the CRC check over the body
/// fails and a keyset that was JUST flushed refuses to load. The raw-byte
/// assertions additionally pin the write-side layout so read and write can
/// never drift apart silently.
#[test]
fn keyset_file_header_is_exactly_twelve_bytes_and_a_flushed_keyset_loads() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let gran = KeyScopeGranularity::PerEntity;
    let target = scope(gran, "entity:layout");

    let mut store = KeyStore::new(gran);
    let ciphertext = store
        .get_or_create(&target)
        .expect("mint key")
        .seal(&NONCE, b"aad", b"layout round-trip")
        .expect("seal");
    let _ = store
        .get_or_create(&scope(gran, "entity:layout-second"))
        .expect("mint second key");
    store.flush(dir.path()).expect("flush keyset");

    let raw = std::fs::read(dir.path().join(KEYSET_FILENAME)).expect("read keyset file");
    assert_eq!(&raw[0..6], b"FBATKS", "magic occupies bytes 0..6");
    assert_eq!(
        u16::from_le_bytes([raw[6], raw[7]]),
        1,
        "format version occupies bytes 6..8 (little-endian)"
    );
    assert_eq!(
        u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]),
        crc32fast::hash(&raw[12..]),
        "PROPERTY: the stored CRC covers EXACTLY the body starting at byte 12 \
         (magic 6 + version 2 + crc 4) — the documented keyset header geometry"
    );

    // The read side must accept its own write under the same 12-byte header —
    // a shifted HEADER_LEN turns this genuine file into a phantom CRC mismatch.
    let reloaded = KeyStore::load(dir.path(), gran)
        .expect("PROPERTY: a freshly flushed keyset must load (header offsets agree)");
    assert_eq!(reloaded.key_count(), 2, "both flushed keys recover");
    assert_eq!(
        reloaded
            .get(&target)
            .expect("target key recovered")
            .open(&NONCE, b"aad", &ciphertext)
            .expect("recovered key opens the pre-flush ciphertext")
            .as_slice(),
        b"layout round-trip",
    );
}

/// KILLS keyscope/persist.rs:93 `granularity_from_disc ->
/// Some(Default::default())`, direction 1: a persisted NON-default granularity
/// (`PerEvent`) must round-trip; the mutant reads the disc back as the default
/// (`PerEntity`), mismatches the configured `PerEvent`, and fails the load.
/// Direction 2: loading the same file under a DIFFERENT configured granularity
/// must fail closed — the mutant instead reads the disc as `PerEntity` and
/// accepts, silently re-keying every scope (an effective total shred).
#[test]
fn a_non_default_granularity_round_trips_and_a_mismatch_fails_closed() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let gran = KeyScopeGranularity::PerEvent;
    assert_ne!(
        gran,
        KeyScopeGranularity::default(),
        "fixture must exercise a NON-default granularity"
    );
    let target = scope(gran, "entity:per-event");

    let mut store = KeyStore::new(gran);
    let _ = store.get_or_create(&target).expect("mint per-event key");
    store.flush(dir.path()).expect("flush PerEvent keyset");

    let reloaded = KeyStore::load(dir.path(), gran)
        .expect("PROPERTY: a persisted NON-default granularity must round-trip");
    assert_eq!(reloaded.key_count(), 1, "the per-event key recovers");
    assert!(
        reloaded.get(&target).is_some(),
        "the recovered key is filed under its original per-event scope"
    );

    let mismatched = KeyStore::load(dir.path(), KeyScopeGranularity::PerEntity)
        .err()
        .expect("PROPERTY: a granularity mismatch must fail the load closed");
    assert!(
        matches!(
            &mismatched,
            StoreError::KeysetCorrupt { reason }
                if reason.contains("does not match persisted keyset granularity")
        ),
        "PROPERTY: the mismatch refusal must be KeysetCorrupt naming the \
         granularity disagreement; got {mismatched:?}"
    );
}

/// KILLS keyscope/persist.rs:93 `granularity_from_disc ->
/// Some(Default::default())`, direction 3: an UNKNOWN on-disk discriminant
/// must be refused with the exact corruption reason — the mutant maps it to
/// the default granularity and accepts the file. The disc-4 control proves the
/// forged encoding is genuine (a real discriminant in the same forged file
/// loads), so the 0xFF refusal can only come from the discriminant check.
#[test]
fn an_unknown_granularity_discriminant_fails_the_load_closed() {
    // CONTROL: the same forged file shape with a REAL discriminant (4 =
    // PerEvent) decodes and loads under the matching granularity.
    let control_dir = tempfile::tempdir().expect("tmpdir");
    forge_keyset_file(control_dir.path(), 4);
    let control = KeyStore::load(control_dir.path(), KeyScopeGranularity::PerEvent)
        .expect("a forged keyset with a real discriminant loads (encoding is genuine)");
    assert_eq!(control.key_count(), 1, "the forged entry rehydrates");

    // THE KILL: discriminant 0xFF names no granularity — fail closed, exactly.
    let dir = tempfile::tempdir().expect("tmpdir");
    forge_keyset_file(dir.path(), 0xFF);
    let refused = KeyStore::load(dir.path(), KeyScopeGranularity::default())
        .err()
        .expect("PROPERTY: an unknown granularity discriminant must refuse the load");
    assert!(
        matches!(
            &refused,
            StoreError::KeysetCorrupt { reason }
                if reason.contains("unknown key-scope granularity discriminant 255")
        ),
        "PROPERTY: the refusal must be KeysetCorrupt naming the unknown \
         discriminant 255; got {refused:?}"
    );
}

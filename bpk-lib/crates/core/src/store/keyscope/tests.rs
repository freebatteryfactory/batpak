//! Unit tests for the crypto-shred KeyScope / KeyStore foundation.
//!
//! Split into a child file-module (rather than an inline `mod tests`) to stay
//! under the structural inline-test-island cap; as a child of `keyscope` it can
//! still reach the module's private `PayloadKey` constructor and `generate`.

use super::*;

fn coord(entity: &str) -> Coordinate {
    Coordinate::new(entity, "scope:test").expect("coordinate")
}

fn fixed_key(byte: u8) -> PayloadKey {
    PayloadKey(Zeroizing::new([byte; KEY_LEN]))
}

#[test]
fn scope_for_is_deterministic_per_granularity() {
    let coordinate = coord("entity:a");
    let kind = EventKind::custom(0xF, 1);
    let id = EventId::from(7u128);
    for granularity in [
        KeyScopeGranularity::PerEntity,
        KeyScopeGranularity::PerCategory,
        KeyScopeGranularity::PerTypeId,
        KeyScopeGranularity::PerEvent,
    ] {
        let first = scope_for(granularity, &coordinate, kind, id);
        let second = scope_for(granularity, &coordinate, kind, id);
        assert_eq!(first, second, "scope_for must be deterministic");
    }
}

#[test]
fn scope_for_distinguishes_by_the_relevant_field_only() {
    let kind = EventKind::custom(0xF, 1);
    let id = EventId::from(7u128);

    // PerEntity keys off the entity, not the kind/id.
    let entity_a = scope_for(KeyScopeGranularity::PerEntity, &coord("a"), kind, id);
    let entity_b = scope_for(KeyScopeGranularity::PerEntity, &coord("b"), kind, id);
    assert_ne!(
        entity_a, entity_b,
        "distinct entities must not share a scope"
    );
    let entity_a_other_kind = scope_for(
        KeyScopeGranularity::PerEntity,
        &coord("a"),
        EventKind::custom(0xE, 2),
        EventId::from(99u128),
    );
    assert_eq!(
        entity_a, entity_a_other_kind,
        "PerEntity ignores kind and id"
    );

    // PerCategory collapses type ids within a category but splits categories.
    let cat_f1 = scope_for(KeyScopeGranularity::PerCategory, &coord("a"), kind, id);
    let cat_f2 = scope_for(
        KeyScopeGranularity::PerCategory,
        &coord("a"),
        EventKind::custom(0xF, 2),
        id,
    );
    let cat_e1 = scope_for(
        KeyScopeGranularity::PerCategory,
        &coord("a"),
        EventKind::custom(0xE, 1),
        id,
    );
    assert_eq!(cat_f1, cat_f2, "same category shares a scope");
    assert_ne!(cat_f1, cat_e1, "distinct categories split");

    // PerTypeId splits on the full kind; PerEvent splits on the id.
    let type_f1 = scope_for(KeyScopeGranularity::PerTypeId, &coord("a"), kind, id);
    let type_f2 = scope_for(
        KeyScopeGranularity::PerTypeId,
        &coord("a"),
        EventKind::custom(0xF, 2),
        id,
    );
    assert_ne!(type_f1, type_f2, "distinct kinds split under PerTypeId");
    let evt_1 = scope_for(KeyScopeGranularity::PerEvent, &coord("a"), kind, id);
    let evt_2 = scope_for(
        KeyScopeGranularity::PerEvent,
        &coord("a"),
        kind,
        EventId::from(8u128),
    );
    assert_ne!(evt_1, evt_2, "distinct ids split under PerEvent");

    // Different granularities never collide (distinct discriminants).
    assert_ne!(entity_a, cat_f1);
    assert_ne!(cat_f1, type_f1);
    assert_ne!(type_f1, evt_1);
}

#[test]
fn default_granularity_is_per_entity() {
    assert_eq!(
        KeyScopeGranularity::default(),
        KeyScopeGranularity::PerEntity
    );
}

#[test]
fn seal_then_open_round_trips() {
    let key = fixed_key(0x11);
    let nonce = [0x22u8; NONCE_LEN];
    let aad = b"associated";
    let plaintext = b"top secret payload";
    let ciphertext = key.seal(&nonce, aad, plaintext).expect("seal");
    assert_ne!(
        ciphertext.as_slice(),
        plaintext,
        "payload must be encrypted"
    );
    let recovered = key.open(&nonce, aad, &ciphertext).expect("open");
    assert_eq!(
        recovered.as_slice(),
        plaintext,
        "round-trip must recover plaintext"
    );
}

#[test]
fn open_fails_on_wrong_key_nonce_or_aad() {
    let key = fixed_key(0x11);
    let nonce = [0x22u8; NONCE_LEN];
    let aad = b"aad";
    let ciphertext = key.seal(&nonce, aad, b"payload").expect("seal");

    let wrong_key = fixed_key(0x33);
    assert_eq!(
        wrong_key.open(&nonce, aad, &ciphertext),
        Err(KeyStoreError::Open),
        "wrong key must fail"
    );

    let wrong_nonce = [0x44u8; NONCE_LEN];
    assert_eq!(
        key.open(&wrong_nonce, aad, &ciphertext),
        Err(KeyStoreError::Open),
        "wrong nonce must fail"
    );

    assert_eq!(
        key.open(&nonce, b"other", &ciphertext),
        Err(KeyStoreError::Open),
        "wrong aad must fail"
    );

    let mut tampered = ciphertext.clone();
    if let Some(first) = tampered.first_mut() {
        *first ^= 0xFF;
    }
    assert_eq!(
        key.open(&nonce, aad, &tampered),
        Err(KeyStoreError::Open),
        "tampered ciphertext must fail"
    );
}

#[test]
fn get_or_create_mints_once_and_returns_the_same_key() {
    let mut store = KeyStore::new(KeyScopeGranularity::PerEntity);
    let scope = scope_for(
        KeyScopeGranularity::PerEntity,
        &coord("entity:mint"),
        EventKind::custom(0xF, 1),
        EventId::from(1u128),
    );
    let nonce = [0x01u8; NONCE_LEN];

    let ciphertext = {
        let key = store.get_or_create(&scope).expect("mint");
        key.seal(&nonce, b"", b"same-key?").expect("seal")
    };
    // A second get_or_create must return the SAME key: opening succeeds.
    let recovered = {
        let key = store.get_or_create(&scope).expect("reuse");
        key.open(&nonce, b"", &ciphertext)
            .expect("open with reused key")
    };
    assert_eq!(recovered.as_slice(), b"same-key?");
}

#[test]
fn destroy_removes_key_and_shreds_prior_ciphertext() {
    let mut store = KeyStore::new(KeyScopeGranularity::PerEvent);
    let scope = scope_for(
        KeyScopeGranularity::PerEvent,
        &coord("entity:shred"),
        EventKind::custom(0xF, 1),
        EventId::from(5u128),
    );
    let nonce = [0x09u8; NONCE_LEN];

    let ciphertext = {
        let key = store.get_or_create(&scope).expect("mint");
        key.seal(&nonce, b"", b"shred me").expect("seal")
    };

    assert!(store.get(&scope).is_some(), "key exists before destroy");
    assert!(store.destroy(&scope), "destroy removes an existing key");
    assert!(store.get(&scope).is_none(), "get after destroy is None");
    assert!(
        !store.destroy(&scope),
        "destroying an absent scope is false"
    );

    // A freshly minted key for the same scope cannot open the old ciphertext.
    let fresh = store.get_or_create(&scope).expect("re-mint");
    assert_eq!(
        fresh.open(&nonce, b"", &ciphertext),
        Err(KeyStoreError::Open),
        "post-shred key must not recover the old payload"
    );
}

#[test]
fn generate_yields_distinct_keys() {
    let a = PayloadKey::generate().expect("key a");
    let b = PayloadKey::generate().expect("key b");
    let nonce = [0u8; NONCE_LEN];
    let ciphertext = a.seal(&nonce, b"", b"probe").expect("seal");
    // Two independently generated keys are (overwhelmingly) distinct: b
    // cannot open a's ciphertext.
    assert_eq!(
        b.open(&nonce, b"", &ciphertext),
        Err(KeyStoreError::Open),
        "independent keys must differ"
    );
}

#[test]
fn payload_key_debug_does_not_leak_bytes() {
    let key = fixed_key(0xAB);
    let rendered = format!("{key:?}");
    assert!(
        !rendered.contains("ab"),
        "debug must not print hex key bytes: {rendered}"
    );
    assert!(
        !rendered.contains("171"),
        "debug must not print decimal key bytes: {rendered}"
    );
    assert!(
        rendered.contains("PayloadKey"),
        "debug still names the type: {rendered}"
    );
}

#[test]
fn payload_aad_layout_is_versioned_length_prefixed_and_coordinate_bound() {
    // Pin the DOCUMENTED AAD encoding byte-for-byte: version 0x01, u32-le
    // length-prefixed entity then scope, kind u16 le, event id u128 be. The
    // write seam (writer/encrypt) and the read seam (read_api) rebuild this
    // independently, so the byte layout itself is the contract — a degenerate
    // AAD (`payload_aad -> vec![0]`) would still round-trip through both seams
    // and silently un-bind every ciphertext from its event identity. Kills
    // `payload_aad -> vec![0]` structurally (the relocation proof below kills
    // it behaviorally).
    let coordinate = coord("entity:aad-layout");
    let kind = EventKind::custom(0xF, 2);
    let aad = payload_aad(&coordinate, kind, EventId::from(0x0102_0304_u128));

    let entity = coordinate.entity().as_bytes();
    let scope = coordinate.scope().as_bytes();
    let mut expected = vec![0x01];
    expected.extend_from_slice(
        &u32::try_from(entity.len())
            .expect("entity length fits u32")
            .to_le_bytes(),
    );
    expected.extend_from_slice(entity);
    expected.extend_from_slice(
        &u32::try_from(scope.len())
            .expect("scope length fits u32")
            .to_le_bytes(),
    );
    expected.extend_from_slice(scope);
    expected.extend_from_slice(&kind.as_raw_u16().to_le_bytes());
    expected.extend_from_slice(&0x0102_0304_u128.to_be_bytes());
    assert_eq!(
        aad, expected,
        "AAD must follow the documented layout exactly (version, len-prefixed \
         entity/scope, kind le, id be)"
    );

    let other_coordinate = payload_aad(
        &coord("entity:aad-layout-other"),
        kind,
        EventId::from(0x0102_0304_u128),
    );
    assert_ne!(
        aad, other_coordinate,
        "AAD must vary with the coordinate — a constant AAD makes ciphertext relocatable"
    );
}

#[test]
fn payload_aad_binds_ciphertext_to_event_identity() {
    // The AAD binds coordinate + kind + event id, so a ciphertext sealed under
    // one event's identity cannot be opened under another's (relocation/tamper),
    // even with the SAME key and SAME nonce.
    let mut store = KeyStore::new(KeyScopeGranularity::PerEntity);
    let coordinate = coord("entity:aad");
    let kind = EventKind::custom(0xF, 1);
    let scope = scope_for(
        KeyScopeGranularity::PerEntity,
        &coordinate,
        kind,
        EventId::from(1u128),
    );
    let key = store.get_or_create(&scope).expect("mint");
    let nonce = [0x5u8; NONCE_LEN];

    let aad_event_1 = payload_aad(&coordinate, kind, EventId::from(1u128));
    let ciphertext = key
        .seal(&nonce, &aad_event_1, b"bound secret")
        .expect("seal");

    // A DIFFERENT event id → different AAD → authentication fails (tamper).
    let aad_event_2 = payload_aad(&coordinate, kind, EventId::from(2u128));
    assert_eq!(
        key.open(&nonce, &aad_event_2, &ciphertext),
        Err(KeyStoreError::Open),
        "relocating the ciphertext onto a different event id must fail to open"
    );
    // A DIFFERENT coordinate → different AAD → also fails.
    let other_coord = coord("entity:other");
    let aad_other = payload_aad(&other_coord, kind, EventId::from(1u128));
    assert_eq!(
        key.open(&nonce, &aad_other, &ciphertext),
        Err(KeyStoreError::Open),
        "relocating the ciphertext onto a different coordinate must fail to open"
    );
    // The correct identity still opens.
    assert_eq!(
        key.open(&nonce, &aad_event_1, &ciphertext).expect("open"),
        b"bound secret",
    );
}

#[test]
fn resolve_shred_scope_matches_the_configured_granularity_and_is_byte_identical_to_scope_for() {
    // PROPERTY: a `ShredScope` selector resolves to the SAME `KeyScope` a matching
    // append sealed under (so the erasure removes exactly the right key), but ONLY
    // when the selector addresses the store's configured granularity. Kills
    // `resolve_shred_scope -> None` (the matching selectors must return `Some`) and
    // pins each of the four match arms to `scope_for`'s builder.
    let coordinate = coord("entity:resolve");
    let kind = EventKind::custom(0xF, 0x2A);
    let id = EventId::from(0x99u128);

    let entity_sel = ShredScope::Entity(&coordinate);
    assert_eq!(
        KeyScopeGranularity::PerEntity.resolve_shred_scope(&entity_sel),
        Some(scope_for(
            KeyScopeGranularity::PerEntity,
            &coordinate,
            kind,
            id
        )),
        "PerEntity+Entity must resolve to the byte-identical per-entity scope"
    );
    let kind_sel = ShredScope::Kind(kind);
    assert_eq!(
        KeyScopeGranularity::PerCategory.resolve_shred_scope(&kind_sel),
        Some(scope_for(
            KeyScopeGranularity::PerCategory,
            &coordinate,
            kind,
            id
        )),
        "PerCategory+Kind must resolve to the per-category scope"
    );
    assert_eq!(
        KeyScopeGranularity::PerTypeId.resolve_shred_scope(&kind_sel),
        Some(scope_for(
            KeyScopeGranularity::PerTypeId,
            &coordinate,
            kind,
            id
        )),
        "PerTypeId+Kind must resolve to the per-type-id scope"
    );
    let event_sel = ShredScope::Event(id);
    assert_eq!(
        KeyScopeGranularity::PerEvent.resolve_shred_scope(&event_sel),
        Some(scope_for(
            KeyScopeGranularity::PerEvent,
            &coordinate,
            kind,
            id
        )),
        "PerEvent+Event must resolve to the per-event scope"
    );
}

#[test]
fn resolve_shred_scope_returns_none_on_a_selector_that_cannot_address_the_granularity() {
    // PROPERTY: a selector that does not address the configured granularity is a
    // typed mismatch (`None`), never silently reinterpreted as another
    // granularity's scope. Pins the `_ => None` catch-all: if a matching arm were
    // widened or the fallback changed, these mismatched pairs would resolve to
    // `Some`.
    let coordinate = coord("entity:mismatch");
    let kind = EventKind::custom(0xE, 3);
    let id = EventId::from(4u128);

    assert_eq!(
        KeyScopeGranularity::PerEntity.resolve_shred_scope(&ShredScope::Kind(kind)),
        None,
        "a Kind selector cannot address a PerEntity store"
    );
    assert_eq!(
        KeyScopeGranularity::PerEntity.resolve_shred_scope(&ShredScope::Event(id)),
        None,
        "an Event selector cannot address a PerEntity store"
    );
    assert_eq!(
        KeyScopeGranularity::PerCategory.resolve_shred_scope(&ShredScope::Entity(&coordinate)),
        None,
        "an Entity selector cannot address a PerCategory store"
    );
    assert_eq!(
        KeyScopeGranularity::PerEvent.resolve_shred_scope(&ShredScope::Kind(kind)),
        None,
        "a Kind selector cannot address a PerEvent store"
    );
}

#[test]
fn shred_scope_label_names_each_selector_variant() {
    // PROPERTY: `label()` renders the non-secret selector name used in the
    // `ShredSelectorMismatch` error. Kills `label -> ""` and any arm that returns
    // the wrong constant.
    let coordinate = coord("entity:label");
    assert_eq!(ShredScope::Entity(&coordinate).label(), "Entity");
    assert_eq!(ShredScope::Kind(EventKind::custom(0xF, 1)).label(), "Kind");
    assert_eq!(ShredScope::Event(EventId::from(1u128)).label(), "Event");
}

#[test]
fn scope_discriminant_bytes_are_stable_and_distinct_per_granularity() {
    // PROPERTY: every scope's first byte is its stable granularity discriminant
    // (0x01..0x04), so the on-disk/on-wire scope byte never silently tracks a
    // source-order change and two granularities never collide. Pins the four
    // `SCOPE_DISC_*` discriminants at the `as_bytes()` boundary.
    let coordinate = coord("entity:disc");
    let kind = EventKind::custom(0xF, 1);
    let id = EventId::from(1u128);
    assert_eq!(
        scope_for(KeyScopeGranularity::PerEntity, &coordinate, kind, id).as_bytes()[0],
        0x01,
        "PerEntity discriminant"
    );
    assert_eq!(
        scope_for(KeyScopeGranularity::PerCategory, &coordinate, kind, id).as_bytes()[0],
        0x02,
        "PerCategory discriminant"
    );
    assert_eq!(
        scope_for(KeyScopeGranularity::PerTypeId, &coordinate, kind, id).as_bytes()[0],
        0x03,
        "PerTypeId discriminant"
    );
    assert_eq!(
        scope_for(KeyScopeGranularity::PerEvent, &coordinate, kind, id).as_bytes()[0],
        0x04,
        "PerEvent discriminant"
    );
}

#[test]
fn granularity_accessor_echoes_the_configured_value_not_the_default() {
    // Kills `granularity -> Default::default()`: `KeyScopeGranularity::default()`
    // is `PerEntity`, so a store built with a NON-default granularity must not read
    // back as the default.
    let store = KeyStore::new(KeyScopeGranularity::PerEvent);
    assert_eq!(
        store.granularity(),
        KeyScopeGranularity::PerEvent,
        "granularity() must echo the configured granularity, not the PerEntity default"
    );
    assert_ne!(store.granularity(), KeyScopeGranularity::default());
}

#[test]
fn key_count_tracks_live_keys_and_is_not_a_constant_zero() {
    // Kills `key_count -> 0`: after minting two scope keys the count is 2, and it
    // drops to 1 after a destroy — a constant-zero body cannot track either.
    let mut store = KeyStore::new(KeyScopeGranularity::PerEvent);
    assert_eq!(store.key_count(), 0, "a fresh key store holds no keys");

    let scope_a = scope_for(
        KeyScopeGranularity::PerEvent,
        &coord("entity:count"),
        EventKind::custom(0xF, 1),
        EventId::from(1u128),
    );
    let scope_b = scope_for(
        KeyScopeGranularity::PerEvent,
        &coord("entity:count"),
        EventKind::custom(0xF, 1),
        EventId::from(2u128),
    );
    store.get_or_create(&scope_a).expect("mint a");
    store.get_or_create(&scope_b).expect("mint b");
    assert_eq!(
        store.key_count(),
        2,
        "two distinct scopes mint two live keys"
    );

    assert!(store.destroy(&scope_a), "destroy an existing key");
    assert_eq!(
        store.key_count(),
        1,
        "destroy drops the live-key count by one"
    );
}

#[test]
fn dirty_flag_starts_clean_and_only_mark_dirty_or_destroy_sets_it() {
    // Kills `is_dirty -> true`/`-> false`, `mark_dirty -> ()`, `destroy -> true`,
    // and the `if removed { self.dirty = true }` guard inside `destroy`.
    let mut store = KeyStore::new(KeyScopeGranularity::PerEntity);
    assert!(!store.is_dirty(), "a fresh key store is not dirty");

    let scope = scope_for(
        KeyScopeGranularity::PerEntity,
        &coord("entity:dirty"),
        EventKind::custom(0xF, 1),
        EventId::from(1u128),
    );
    // Minting alone does NOT flip the durability fence signal (the writer marks it
    // explicitly); so the store is still clean and destroy's effect is isolable.
    store.get_or_create(&scope).expect("mint");
    assert!(
        !store.is_dirty(),
        "get_or_create must not by itself mark the keyset dirty"
    );

    // Destroying a PRESENT key removes it AND flags the erasure not-yet-durable.
    assert!(
        store.destroy(&scope),
        "destroying a present key returns true"
    );
    assert!(
        store.is_dirty(),
        "destroy of a present key must flag the keyset dirty (erasure pending flush)"
    );

    // Destroying an ABSENT scope in a clean store returns false and does NOT dirty.
    let mut clean = KeyStore::new(KeyScopeGranularity::PerEntity);
    let absent = scope_for(
        KeyScopeGranularity::PerEntity,
        &coord("entity:absent"),
        EventKind::custom(0xF, 1),
        EventId::from(9u128),
    );
    assert!(
        !clean.destroy(&absent),
        "destroying an absent scope returns false"
    );
    assert!(
        !clean.is_dirty(),
        "a no-op destroy must not flag the keyset dirty"
    );

    // mark_dirty flips the clean store's signal (kills mark_dirty -> ()).
    clean.mark_dirty();
    assert!(store.is_dirty(), "the destroyed store is still dirty");
    assert!(clean.is_dirty(), "mark_dirty must set the dirty signal");
}

#[test]
fn key_store_error_display_is_the_exact_opaque_message_per_variant() {
    // Kills `KeyStoreError::Display -> Ok(())` (empty render) and any arm that
    // returns the wrong message. The messages are deliberately opaque (no oracle),
    // so they are the contract.
    assert_eq!(
        KeyStoreError::Rng.to_string(),
        "CSPRNG failed to produce key material"
    );
    assert_eq!(
        KeyStoreError::KeyInit.to_string(),
        "AEAD key initialization rejected the key length"
    );
    assert_eq!(
        KeyStoreError::Seal.to_string(),
        "authenticated encryption failed"
    );
    assert_eq!(
        KeyStoreError::Open.to_string(),
        "authenticated decryption failed"
    );
}

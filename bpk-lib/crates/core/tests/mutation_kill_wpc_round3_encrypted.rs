//! Round-3 mutation kills for the WP-C survivors — the `payload-encryption`
//! surface half (key-aware replay seam + key-aware ancestry).
//!
//! PROVES: (1) the value-lane plaintext-bytes replay seam
//! (`JsonValueInput::payload_from_plaintext_bytes`) decodes the in-band
//! SYSTEM_BATCH_BEGIN/COMMIT markers — which carry EMPTY payload bytes on
//! disk — to `Value::Null` exactly like the plaintext read seam, while every
//! other kind MessagePack-decodes; (2) the raw lane
//! (`RawMsgpackInput::payload_from_plaintext_bytes`) passes payload bytes
//! through VERBATIM, both at the seam and end-to-end through a raw-mode
//! projection over an encrypted store; (3) the key-aware ancestry step
//! returns the REAL chain: a ≥2-step walk yields the exact ancestor ids in
//! reverse append order with decrypted payloads, includes a crypto-shredded
//! ancestor (Null placeholder, flagged in `shredded`) and continues through
//! it to genesis.
//! CATCHES: replay_input.rs:66 deletion of the SYSTEM_BATCH_BEGIN|COMMIT
//! match arm (empty marker bytes would become a decode ERROR instead of
//! Null); replay_input.rs:98 `RawMsgpackInput::payload_from_plaintext_bytes
//! -> Ok(Default::default())` (decrypted payloads would silently become
//! empty byte strings); ancestry/mod.rs:293 fabrication of
//! `step_ancestor_key_aware`'s outcome (any fabricated step breaks the exact
//! id/payload/boundary/shredded pins).
//! SEEDED: deterministic — fixed payloads, fixed byte fixtures encoded with
//! the crate's own named-field MessagePack encoder; no randomness.
#![cfg(feature = "payload-encryption")]

use batpak::store::{
    AncestryBoundary, Freshness, KeyScopeGranularity, ReplayInput, ShredScope, Store, StoreConfig,
    StoreError,
};
use batpak_testkit::prelude::*;
use tempfile::TempDir;

// ─── C1: value lane — batch markers decode to Null, everything else decodes ─

#[test]
fn value_lane_plaintext_seam_maps_empty_batch_markers_to_null() {
    // Batch marker frames are written with EMPTY payload bytes
    // (`Event::new(header, Vec::new())` in the writer). The carve-out must
    // yield Null for BOTH markers; without the arm, empty bytes would be fed
    // to the MessagePack decoder and fail.
    for kind in [
        EventKind::SYSTEM_BATCH_BEGIN,
        EventKind::SYSTEM_BATCH_COMMIT,
    ] {
        let decoded =
            <JsonValueInput as ReplayInput>::payload_from_plaintext_bytes(Vec::new(), kind);
        assert!(
            matches!(&decoded, Ok(serde_json::Value::Null)),
            "PROPERTY: the value lane must decode an in-band batch marker ({kind:?}, empty \
             payload bytes) to Value::Null exactly like the plaintext read seam — a decode \
             error here would fail every key-aware replay that crosses a batch marker; got \
             {decoded:?}"
        );
    }
}

#[test]
fn value_lane_plaintext_seam_msgpack_decodes_every_non_marker_kind() {
    let payload = serde_json::json!({ "probe": 7, "tag": "value-lane" });
    let bytes = rmp_serde::to_vec_named(&payload).expect("encode probe payload");

    let decoded =
        <JsonValueInput as ReplayInput>::payload_from_plaintext_bytes(bytes, EventKind::DATA)
            .expect("non-marker kinds MessagePack-decode");
    assert_eq!(
        decoded, payload,
        "PROPERTY: a non-marker kind's plaintext bytes decode to the exact original value \
         (the marker carve-out must not swallow ordinary kinds)"
    );

    let garbage =
        <JsonValueInput as ReplayInput>::payload_from_plaintext_bytes(Vec::new(), EventKind::DATA);
    assert!(
        matches!(&garbage, Err(StoreError::Serialization(_))),
        "PROPERTY: empty bytes under a NON-marker kind are a Serialization error — only the \
         two batch markers get the Null carve-out; got {garbage:?}"
    );
}

// ─── C2: raw lane — plaintext bytes pass through verbatim ───────────────────

#[test]
fn raw_lane_plaintext_seam_returns_the_exact_bytes_verbatim() {
    let bytes = vec![0x93_u8, 0x01, 0xA3, b'r', b'a', b'w', 0x2A];
    for kind in [
        EventKind::DATA,
        EventKind::SYSTEM_BATCH_BEGIN,
        EventKind::SYSTEM_BATCH_COMMIT,
    ] {
        let out =
            <RawMsgpackInput as ReplayInput>::payload_from_plaintext_bytes(bytes.clone(), kind)
                .expect("raw lane never fails on plaintext bytes");
        assert_eq!(
            out, bytes,
            "PROPERTY: the raw lane returns plaintext payload bytes VERBATIM for {kind:?} — \
             an empty default here would silently erase every decrypted payload"
        );
    }
}

const RAW_KIND: EventKind = EventKind::custom(0xC, 0x55);

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct RawByteTap {
    payloads: Vec<Vec<u8>>,
}

impl EventSourced for RawByteTap {
    type Input = RawMsgpackInput;
    const STATE_CONTRACT: ProjectionStateContract =
        ProjectionStateContract::single_entity("mutation-kill-wpc-raw-byte-tap");

    fn from_events(events: &[Event<Vec<u8>>]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }
        let mut state = Self::default();
        for event in events {
            state.apply_event(event);
        }
        Some(state)
    }

    fn apply_event(&mut self, event: &Event<Vec<u8>>) {
        self.payloads.push(event.payload.clone());
    }

    fn relevant_event_kinds() -> &'static [EventKind] {
        &[RAW_KIND]
    }

    fn state_extent(&self) -> StateExtent {
        StateExtent::single_entity()
    }
}

#[test]
fn raw_projection_over_an_encrypted_store_recovers_the_exact_plaintext_bytes() {
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(
        StoreConfig::new(dir.path()).with_payload_encryption(KeyScopeGranularity::PerEntity),
    )
    .expect("open encrypted store");
    let entity = "entity:raw-byte-tap";
    let coord = Coordinate::new(entity, "scope:mutation").expect("coord");

    let first = serde_json::json!({ "n": 1, "tag": "first" });
    let second = serde_json::json!({ "n": 2, "tag": "second" });
    for payload in [&first, &second] {
        drop(
            store
                .append(&coord, RAW_KIND, payload)
                .expect("append encrypted event"),
        );
    }

    // The store encodes payloads with its named-field MessagePack encoder;
    // the key-aware raw replay must hand those exact recovered plaintext
    // bytes to the fold, byte-for-byte.
    let expected = vec![
        rmp_serde::to_vec_named(&first).expect("encode first payload"),
        rmp_serde::to_vec_named(&second).expect("encode second payload"),
    ];
    let tapped = store
        .project::<RawByteTap>(entity, &Freshness::Consistent)
        .expect("raw projection over encrypted store")
        .expect("two appended events fold to a state");
    assert_eq!(
        tapped.payloads, expected,
        "PROPERTY: raw-projection over an encrypted store delivers each event's decrypted \
         payload bytes VERBATIM (exact bytes, exact order) — empty or altered bytes mean the \
         raw plaintext seam fabricated its output"
    );
    store.close().expect("close store");
}

// ─── C10: key-aware ancestry walk — exact chain, shred-aware, to genesis ────

const CHAIN_KIND: EventKind = EventKind::custom(0xC, 0x56);

#[test]
fn key_aware_ancestry_walk_returns_the_exact_chain_and_marks_the_shredded_link() {
    // PerEvent keying so ONE mid-chain ancestor can be shredded while its
    // neighbours stay readable.
    let dir = TempDir::new().expect("temp dir");
    let store = Store::open(
        StoreConfig::new(dir.path()).with_payload_encryption(KeyScopeGranularity::PerEvent),
    )
    .expect("open encrypted store");
    let coord = Coordinate::new("entity:key-aware-walk", "scope:mutation").expect("coord");

    let ids: Vec<EventId> = (0..3u32)
        .map(|step| {
            store
                .append(&coord, CHAIN_KIND, &serde_json::json!({ "step": step }))
                .expect("append chain event")
                .event_id
        })
        .collect();
    let genesis = ids[0];
    let shredded_mid = ids[1];
    let anchor = ids[2];

    assert!(
        store
            .shred_scope(ShredScope::Event(shredded_mid))
            .expect("shred the mid-chain event"),
        "a live key existed for the mid-chain event and was destroyed"
    );

    let walk = store.walk_ancestors_outcome(anchor, 8);

    // Exact ≥2-step chain: every ancestor id, newest first — a fabricated
    // step outcome cannot reproduce these ids.
    let walked: Vec<EventId> = walk.ancestors.iter().map(|s| s.event.event_id()).collect();
    assert_eq!(
        walked,
        vec![anchor, shredded_mid, genesis],
        "PROPERTY: the key-aware walk returns the EXACT ancestor ids in reverse append order, \
         including the shredded link"
    );
    assert_eq!(
        walk.boundary,
        AncestryBoundary::ReachedGenesis,
        "PROPERTY: the walk continues THROUGH the shredded ancestor to genesis — never a \
         fabricated truncation"
    );

    // Shred marking: exactly the shredded id, Null placeholder payload; the
    // readable neighbours decrypt to their exact original plaintext.
    assert_eq!(
        walk.shredded_ancestors(),
        &[shredded_mid],
        "PROPERTY: exactly the shredded ancestor is flagged in the shredded set"
    );
    for stored in &walk.ancestors {
        let id = stored.event.event_id();
        if id == shredded_mid {
            assert_eq!(
                stored.event.payload,
                serde_json::Value::Null,
                "PROPERTY: the shredded ancestor carries the Null placeholder (plaintext gone)"
            );
        } else {
            let step = if id == genesis { 0 } else { 2 };
            assert_eq!(
                stored.event.payload,
                serde_json::json!({ "step": step }),
                "PROPERTY: a readable ancestor decrypts to its exact original plaintext"
            );
        }
    }
    store.close().expect("close store");
}

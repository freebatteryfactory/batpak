//! GAUNT-IDEMPOTENCY-AUTHORITY (#189): the durable idempotency window is an
//! AUTHORITY all the way through admission, cold start, and compaction commit.
//!
//! PROVES: INV-IDEMPOTENCY-DURABLE-WINDOW (fail-closed) and
//! INV-IDEMPOTENCY-AUTHORITY-ATOMIC-COMPACTION — after retention eviction a
//! corrupt sidecar can never turn a retry into a second append; a missing
//! sidecar on a keyed store is authority LOSS, not a fresh store; a CRC-valid
//! but STALE image is rejected through its coverage frontier; a FOREIGN image
//! (other lineage, or a diverged sibling fork at the same covered sequence) is
//! rejected through the compound anchor. (The compaction old-or-new commit
//! half lives in `idempotency_authority_compaction.rs`.)
//! CATCHES: the issue's named admission mutants — Invalid->Missing collapse,
//! removal of the expectation check, removal of the frontier comparison, and
//! accepting keyed traffic on unproven authority.
//! SEEDED: real stores with tiny segments; deterministic key constants; byte
//! transplants between sibling directories.

use batpak::coordinate::{Coordinate, Region};
use batpak::event::EventKind;
use batpak::id::{EntityIdType, IdempotencyKey};
use batpak::store::{
    AppendOptions, CompactionConfig, CompactionStrategy, IdempAuthorityForeignKind, Store,
    StoreConfig, StoreError,
};
use std::path::Path;
use tempfile::TempDir;

const KIND: EventKind = EventKind::custom(0xB, 4);
const IDEMP_FILENAME: &str = "index.idemp";

fn coord() -> Coordinate {
    Coordinate::new("entity:authority", "scope:fail-closed").expect("valid coord")
}

fn config(dir: &Path) -> StoreConfig {
    StoreConfig::new(dir)
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

fn key_event_count(store: &Store, key: u128) -> usize {
    store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_kind() == KIND && e.event_id().as_u128() == key)
        .count()
}

/// Retention strategy that evicts EVERY user event of `KIND` (keeps only the
/// batch/system markers), forcing keyed event frames out of the store.
fn evict_all_user_events() -> CompactionConfig {
    CompactionConfig {
        strategy: CompactionStrategy::Retention(Box::new(|stored| {
            stored.event.header.event_kind != KIND
        })),
        min_segments: 1,
    }
}

/// Force sealed segments then run the evicting retention compaction; assert
/// the keyed event frame is really gone from the live index.
fn evict_key_via_compaction(store: &Store, key: u128) {
    for i in 0..8 {
        let _ = store
            .append(&coord(), KIND, &serde_json::json!({ "filler": i }))
            .expect("append filler event");
    }
    let (_result, _report) = store
        .compact(&evict_all_user_events())
        .expect("retention compaction");
    let survivors = store
        .query(&Region::all())
        .into_iter()
        .filter(|e| e.event_id().as_u128() == key)
        .count();
    assert_eq!(
        survivors, 0,
        "PRECONDITION: retention compaction evicted the keyed event frame"
    );
}

// ---------------------------------------------------------------------------
// Fixture 1 (+6): corrupt authority after retention eviction — the retry can
// NEVER append a second event; keyed traffic stays blocked until the
// authority is restored, then the original receipt comes back.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_authority_after_retention_never_reappends_key() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = TempDir::new().expect("tempdir");
    let key = 0x1111_2222_3333_4444_5555_6666_7777_8888u128;

    let first = {
        let store = Store::open(config(dir.path())).expect("open");
        let first = append_keyed(&store, key);
        evict_key_via_compaction(&store, key);
        store.close().expect("close");
        first
    };

    // The durable map is now the ONLY replay authority for `key`. Corrupt it.
    let path = dir.path().join(IDEMP_FILENAME);
    let healthy = std::fs::read(&path)?;
    let mut corrupt = healthy.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0xFF;
    std::fs::write(&path, &corrupt)?;

    // Reopen refuses — no keyed append can be admitted on unproven authority.
    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): corrupt authority after eviction must refuse open — an \
                 admitted store would treat the retry as new and double-append",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityCorrupt { .. } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };

    // Restore the authority: the retry returns the ORIGINAL receipt identity.
    std::fs::write(&path, &healthy)?;
    let store = Store::open(config(dir.path())).expect("restored authority admits the store");
    let replay = append_keyed(&store, key);
    assert_eq!(
        replay.global_sequence, first.global_sequence,
        "retry after restore returns the original sequence"
    );
    assert_eq!(
        u128::from(replay.event_id),
        u128::from(first.event_id),
        "retry after restore returns the original event id"
    );
    assert_eq!(
        replay.content_hash, first.content_hash,
        "retry after restore returns the original content hash"
    );
    assert_eq!(
        key_event_count(&store, key),
        0,
        "the evicted frame stays evicted — the retry re-appended NOTHING"
    );
    store.close().expect("close");
    Ok(())
}

// ---------------------------------------------------------------------------
// Fixture 2: deleting an expected sidecar differs from a fresh store.
// ---------------------------------------------------------------------------
#[test]
fn missing_expected_authority_is_not_a_fresh_store() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0xAAAA_BBBB_CCCC_DDDD_EEEE_FFFF_0000_1111u128);
        store.close().expect("close");
    }
    std::fs::remove_file(dir.path().join(IDEMP_FILENAME))?;

    let err = match Store::open(config(dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a deleted sidecar on a keyed store is authority LOSS, not a \
                 fresh store — open must refuse",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityMissing { path } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some(IDEMP_FILENAME),
        "the refusal names the expected sidecar; got {}",
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Fixture 3: a CRC-valid but STALE image is rejected via its coverage
// frontier.
// ---------------------------------------------------------------------------
#[test]
fn stale_crc_valid_authority_image_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(IDEMP_FILENAME);

    {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x1000_0000_0000_0000_0000_0000_0000_0001u128);
        store.close().expect("close");
    }
    let old_image = std::fs::read(&path)?;

    {
        let store = Store::open(config(dir.path())).expect("reopen");
        let _ = append_keyed(&store, 0x2000_0000_0000_0000_0000_0000_0000_0002u128);
        store.close().expect("close");
    }

    // Roll the sidecar back to the older, still-CRC-valid image.
    std::fs::write(&path, &old_image)?;

    let err =
        match Store::open(config(dir.path())) {
            Ok(_) => return Err(std::io::Error::other(
                "PROPERTY (#189): a stale CRC-valid image must be rejected through its coverage \
                 frontier — keys committed after it would silently vanish from the contract",
            )
            .into()),
            Err(e) => e,
        };
    let StoreError::IdempotencyAuthorityStale {
        image_covered,
        expected_covered,
    } = err
    else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        image_covered < expected_covered,
        "the refusal reports the frontier regression ({image_covered} < {expected_covered})"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Foreign images: another store's image (lineage), and a diverged sibling
// fork's image at the same covered sequence (compound history anchor).
// ---------------------------------------------------------------------------
#[test]
fn foreign_lineage_image_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let a_dir = TempDir::new().expect("tempdir a");
    let b_dir = TempDir::new().expect("tempdir b");
    for dir in [&a_dir, &b_dir] {
        let store = Store::open(config(dir.path())).expect("open");
        let _ = append_keyed(&store, 0x3000_0000_0000_0000_0000_0000_0000_0003u128);
        store.close().expect("close");
    }
    // Transplant B's (CRC-valid) image into A.
    std::fs::copy(
        b_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): another lineage's authority image must be rejected",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::Lineage),
        "a transplanted other-store image is a LINEAGE mismatch; got {kind:?}"
    );
    Ok(())
}

#[test]
fn diverged_sibling_fork_image_is_rejected_via_history_anchor(
) -> Result<(), Box<dyn std::error::Error>> {
    // Sibling forks SHARE the lineage id (owner ruling, #205), so lineage +
    // numeric frontier alone would admit a transplanted sibling image. The
    // compound anchor (event id + chain commitment at the covered sequence)
    // is what catches the divergence.
    let a_dir = TempDir::new().expect("tempdir a");
    let f_dir = TempDir::new().expect("fork dir");

    {
        let store = Store::open(config(a_dir.path())).expect("open");
        let _ = append_keyed(&store, 0x4000_0000_0000_0000_0000_0000_0000_0004u128);
        store.fork(f_dir.path()).expect("fork");
        store.close().expect("close");
    }

    // Diverge the siblings, then TOP UP the laggard with filler until both
    // frontiers are EQUAL, so the transplant below cannot be caught by the
    // frontier comparison — only the compound anchor can refuse it.
    let a = Store::open(config(a_dir.path())).expect("reopen a");
    let f = Store::open(config(f_dir.path())).expect("open fork");
    let _ = append_keyed(&a, 0x5000_0000_0000_0000_0000_0000_0000_0005u128);
    let _ = append_keyed(&f, 0x6000_0000_0000_0000_0000_0000_0000_0006u128);
    for _ in 0..64 {
        let (ga, gf) = (a.stats().global_sequence, f.stats().global_sequence);
        if ga == gf {
            break;
        }
        let lagging = if ga < gf { &a } else { &f };
        let _ = lagging
            .append(&coord(), KIND, &serde_json::json!({"filler": "top-up"}))
            .expect("top-up filler append");
    }
    let covered = (a.stats().global_sequence, f.stats().global_sequence);
    assert_eq!(
        covered.0, covered.1,
        "PRECONDITION: both siblings sit at the same covered sequence — the transplant below \
         must be caught by the anchor, not the frontier"
    );
    a.close().expect("close a");
    f.close().expect("close fork");

    // Transplant the fork's image into A.
    std::fs::copy(
        f_dir.path().join(IDEMP_FILENAME),
        a_dir.path().join(IDEMP_FILENAME),
    )?;

    let err = match Store::open(config(a_dir.path())) {
        Ok(_) => {
            return Err(std::io::Error::other(
                "PROPERTY (#189): a diverged sibling fork's image must be rejected via the \
                 compound history anchor",
            )
            .into())
        }
        Err(e) => e,
    };
    let StoreError::IdempotencyAuthorityForeign { kind } = err else {
        return Err(std::io::Error::other(format!("wrong variant: {err:?}")).into());
    };
    assert!(
        matches!(kind, IdempAuthorityForeignKind::HistoryAnchor),
        "a same-frontier sibling transplant is a HISTORY-ANCHOR divergence; got {kind:?}"
    );
    Ok(())
}
